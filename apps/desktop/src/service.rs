use std::collections::VecDeque;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;

use crate::error::{AppError, AppResult};
use crate::model::{DshHome, DshSource};
use crate::process_supervisor::{ProcessLease, ProcessSupervisor, terminate_child_tree};
use crate::runtime::{InstalledRuntime, RuntimeManager};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const SHELL_PATH_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_LOG_LINES: usize = 400;
const MAX_PROBE_BODY_BYTES: u64 = 1024 * 1024;
const PATH_MARKER: &str = "__DSH_DESKTOP_PATH__";
const DSH_IDENTITY_TITLE: &str = "<title>DeepSeek Harness</title>";
const DSH_AUTH_CHALLENGE: &str = "dsh web authentication required";
const LAUNCH_URL_LINE_PREFIX: &str = "dsh web: ";
const LAUNCH_URL_GRACE: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeResult {
    Unreachable,
    Dsh,
    OtherHttp,
}

#[derive(Default)]
struct LaunchUrl {
    seen: bool,
    token: Option<String>,
}

fn parse_launch_url_line(line: &str) -> Option<LaunchUrl> {
    let url_text = line.strip_prefix(LAUNCH_URL_LINE_PREFIX)?.trim();
    let url_text = url_text.split_whitespace().next()?;
    let url = url::Url::parse(url_text).ok()?;
    Some(LaunchUrl {
        seen: true,
        token: url
            .query_pairs()
            .find(|(key, _)| key == "token")
            .map(|(_, value)| value.into_owned()),
    })
}

pub struct ManagedService {
    child: Child,
    process_lease: ProcessLease,
    pub executable: PathBuf,
    pub runtime_version: String,
    pub logs: Arc<Mutex<VecDeque<String>>>,
    launch_url: Arc<Mutex<LaunchUrl>>,
    pub launch: ManagedLaunch,
}

#[derive(Clone)]
pub struct ResolvedDshRuntime {
    executable: PathBuf,
    prefix_args: Vec<OsString>,
    runtime_path: Option<OsString>,
    runtime_version: String,
    supports_no_open: bool,
    built_in_runtime_id: Option<String>,
}

#[derive(Clone)]
pub struct ManagedLaunch {
    runtime: ResolvedDshRuntime,
    port: u16,
    dsh_home: DshHome,
}

impl ManagedLaunch {
    pub fn with_dsh_home(mut self, dsh_home: DshHome) -> Self {
        self.dsh_home = dsh_home;
        self
    }

    pub fn with_runtime(mut self, runtime: ResolvedDshRuntime) -> Self {
        self.runtime = runtime;
        self
    }

    pub fn uses_built_in_runtime(&self) -> bool {
        self.runtime.built_in_runtime_id.is_some()
    }
}

impl ResolvedDshRuntime {
    pub fn version(&self) -> &str {
        &self.runtime_version
    }

    /// Complete PATH value that makes this runtime's `dsh` and its pnpm
    /// launcher resolve ahead of the system environment.
    pub fn path_env(&self) -> Option<&OsStr> {
        self.runtime_path.as_deref()
    }
}

impl ManagedService {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn stop(&mut self) {
        terminate_child_tree(&mut self.child);
        self.process_lease.release();
    }

    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.process_lease.release();
        }
        Ok(status)
    }

    pub fn diagnostic(&self) -> String {
        let logs = recent_logs(&self.logs);
        if logs.is_empty() {
            format!("executable: {}", self.executable.display())
        } else {
            format!("executable: {}\n{logs}", self.executable.display())
        }
    }

    pub fn log_lines(&self) -> Vec<String> {
        self.logs
            .lock()
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Launch token printed by `dsh web` once the Web UI is ready.
    ///
    /// Newer DSH versions guard the Web UI with browser token
    /// authentication; the token is only distributed through this stdout
    /// URL line and is required to authenticate the first frame load.
    pub fn launch_token(&self) -> Option<String> {
        self.launch_url.lock().ok()?.token.clone()
    }
}

impl Drop for ManagedService {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn probe(url: &str) -> ProbeResult {
    let client = match Client::builder().no_proxy().timeout(PROBE_TIMEOUT).build() {
        Ok(client) => client,
        Err(_) => return ProbeResult::Unreachable,
    };
    let response = match client.get(url).send() {
        Ok(response) => response,
        Err(_) if tcp_port_is_reachable(url) => return ProbeResult::OtherHttp,
        Err(_) => return ProbeResult::Unreachable,
    };
    let status = response.status();
    // Newer DSH versions answer anonymous probes with 401 and a stable
    // challenge text instead of serving the identity page.
    if !(status.is_success() || status.as_u16() == 401) {
        return ProbeResult::OtherHttp;
    }
    let mut body = String::new();
    if response
        .take(MAX_PROBE_BODY_BYTES)
        .read_to_string(&mut body)
        .is_err()
    {
        return ProbeResult::OtherHttp;
    }
    classify_probe_response(status.as_u16(), &body)
}

fn classify_probe_response(status: u16, body: &str) -> ProbeResult {
    if body.contains(DSH_IDENTITY_TITLE) || (status == 401 && body.contains(DSH_AUTH_CHALLENGE)) {
        ProbeResult::Dsh
    } else {
        ProbeResult::OtherHttp
    }
}

fn tcp_port_is_reachable(url: &str) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    let (Some(host), Some(port)) = (url.host_str(), url.port_or_known_default()) else {
        return false;
    };
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return false;
    };
    addresses
        .into_iter()
        .any(|address| TcpStream::connect_timeout(&address, PROBE_TIMEOUT).is_ok())
}

pub fn resolve_runtime(
    source: &DshSource,
    runtime_manager: &RuntimeManager,
    process_supervisor: &ProcessSupervisor,
) -> AppResult<ResolvedDshRuntime> {
    let runtime_path = effective_path(process_supervisor);
    let (executable, prefix_args, runtime_path, known_version, invalid_error): (
        PathBuf,
        Vec<OsString>,
        Option<OsString>,
        Option<String>,
        &str,
    ) = match source {
        DshSource::None => return Err(AppError::new("service.error.sourceDisabled")),
        DshSource::BuiltIn => {
            return resolve_built_in_runtime(
                runtime_manager.resolve_built_in()?,
                process_supervisor,
            );
        }
        DshSource::System => (
            resolve_executable(None, runtime_path.as_deref())?,
            Vec::new(),
            runtime_path,
            None,
            "service.error.invalidExecutable",
        ),
        DshSource::Custom { executable } => (
            resolve_executable(Some(executable), runtime_path.as_deref())?,
            Vec::new(),
            runtime_path,
            None,
            "service.error.invalidExecutable",
        ),
        DshSource::Npx { version } => (
            runtime_path
                .as_deref()
                .and_then(|path| find_on_path("npx", path))
                .ok_or_else(|| AppError::new("service.error.npxNotFound"))?,
            vec![OsString::from("--yes"), npx_package_argument(version)],
            runtime_path,
            None,
            "service.error.invalidNpx",
        ),
    };
    let runtime_version = read_version(
        &executable,
        &prefix_args,
        runtime_path.as_deref(),
        invalid_error,
        process_supervisor,
    )?;
    if known_version
        .as_ref()
        .is_some_and(|expected| expected != &runtime_version)
    {
        return Err(AppError::new("runtime.error.versionMismatch")
            .arg("expected", known_version)
            .arg("actual", runtime_version));
    }
    let supports_no_open = detects_no_open_support(
        &executable,
        &prefix_args,
        runtime_path.as_deref(),
        process_supervisor,
    );
    Ok(ResolvedDshRuntime {
        executable,
        prefix_args,
        runtime_path,
        runtime_version,
        supports_no_open,
        built_in_runtime_id: None,
    })
}

pub fn resolve_built_in_runtime(
    runtime: InstalledRuntime,
    process_supervisor: &ProcessSupervisor,
) -> AppResult<ResolvedDshRuntime> {
    let runtime_path = prepend_paths(
        &runtime.executable_path,
        effective_path(process_supervisor).as_deref(),
    );
    let runtime_version = read_version(
        &runtime.node_executable,
        &[runtime.dsh_entrypoint.clone().into_os_string()],
        runtime_path.as_deref(),
        "service.error.invalidExecutable",
        process_supervisor,
    )?;
    if runtime_version != runtime.dsh_version {
        return Err(AppError::new("runtime.error.versionMismatch")
            .arg("expected", runtime.dsh_version)
            .arg("actual", runtime_version));
    }
    let prefix_args = vec![runtime.dsh_entrypoint.into_os_string()];
    let supports_no_open = detects_no_open_support(
        &runtime.node_executable,
        &prefix_args,
        runtime_path.as_deref(),
        process_supervisor,
    );
    Ok(ResolvedDshRuntime {
        executable: runtime.node_executable,
        prefix_args,
        runtime_path,
        runtime_version,
        supports_no_open,
        built_in_runtime_id: Some(runtime.runtime_id),
    })
}

pub fn verify_web_help(
    runtime: &ResolvedDshRuntime,
    process_supervisor: &ProcessSupervisor,
) -> AppResult<()> {
    let output = web_help_output(
        &runtime.executable,
        &runtime.prefix_args,
        runtime.runtime_path.as_deref(),
        process_supervisor,
    )
    .map_err(|error| {
        AppError::new("runtime.error.verificationFailed").technical(error.to_string())
    })?;
    if output.status.success() {
        return Ok(());
    }
    Err(AppError::new("runtime.error.verificationFailed")
        .technical(String::from_utf8_lossy(&output.stderr)))
}

pub fn start(
    runtime: &ResolvedDshRuntime,
    port: u16,
    dsh_home: DshHome,
    process_supervisor: &ProcessSupervisor,
    running: &AtomicBool,
) -> AppResult<ManagedService> {
    start_launch(
        &ManagedLaunch {
            runtime: runtime.clone(),
            port,
            dsh_home,
        },
        process_supervisor,
        running,
    )
}

pub fn start_launch(
    launch: &ManagedLaunch,
    process_supervisor: &ProcessSupervisor,
    running: &AtomicBool,
) -> AppResult<ManagedService> {
    if !running.load(Ordering::Acquire) {
        return Err(host_shutting_down_error());
    }
    let runtime = &launch.runtime;
    let port = launch.port;
    let executable = &runtime.executable;
    let mut command = Command::new(&executable);
    add_managed_web_arguments(
        &mut command,
        &runtime.prefix_args,
        port,
        runtime.supports_no_open,
    )
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    apply_dsh_home(&mut command, &launch.dsh_home);
    if let Some(runtime_path) = runtime.runtime_path.as_ref() {
        command.env("PATH", runtime_path);
    }
    let (mut child, mut process_lease) =
        process_supervisor.spawn(&mut command).map_err(|error| {
            AppError::new("service.error.startFailed")
                .arg("executable", executable.display().to_string())
                .technical(error.to_string())
        })?;
    if !running.load(Ordering::Acquire) {
        terminate_child_tree(&mut child);
        process_lease.release();
        return Err(host_shutting_down_error());
    }

    let logs = Arc::new(Mutex::new(VecDeque::new()));
    let launch_url = Arc::new(Mutex::new(LaunchUrl::default()));
    if let Some(stdout) = child.stdout.take() {
        capture_output(
            stdout,
            "stdout",
            Arc::clone(&logs),
            Some(Arc::clone(&launch_url)),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        capture_output(stderr, "stderr", Arc::clone(&logs), None);
    }

    let url = format!("http://127.0.0.1:{port}");
    let started_at = Instant::now();
    let mut ready_at: Option<Instant> = None;
    loop {
        if !running.load(Ordering::Acquire) {
            terminate_child_tree(&mut child);
            process_lease.release();
            return Err(host_shutting_down_error());
        }
        match probe(&url) {
            ProbeResult::Dsh => {
                // Wait briefly for the launch URL line so the first frame
                // load can already carry the authentication token. Older
                // DSH versions never print a token and are covered by the
                // grace period.
                let line_seen = launch_url.lock().map(|launch| launch.seen).unwrap_or(false);
                let waited = ready_at.get_or_insert_with(Instant::now).elapsed();
                if line_seen || waited >= LAUNCH_URL_GRACE {
                    break;
                }
            }
            ProbeResult::OtherHttp => {
                terminate_child_tree(&mut child);
                process_lease.release();
                return Err(AppError::new("service.error.portOccupied").arg("port", port));
            }
            ProbeResult::Unreachable => {
                ready_at = None;
            }
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                process_lease.release();
                return Err(AppError::new("service.error.processExited")
                    .arg("status", status.to_string())
                    .technical(recent_logs(&logs)));
            }
            Ok(None) => {}
            Err(error) => {
                terminate_child_tree(&mut child);
                process_lease.release();
                return Err(AppError::new("service.error.startFailed").technical(error.to_string()));
            }
        }

        if started_at.elapsed() >= STARTUP_TIMEOUT {
            terminate_child_tree(&mut child);
            process_lease.release();
            return Err(AppError::new("service.error.startTimeout")
                .arg("port", port)
                .technical(recent_logs(&logs)));
        }
        thread::sleep(Duration::from_millis(200));
    }

    Ok(ManagedService {
        child,
        process_lease,
        executable: executable.clone(),
        runtime_version: runtime.runtime_version.clone(),
        logs,
        launch_url,
        launch: launch.clone(),
    })
}

fn add_managed_web_arguments<'a>(
    command: &'a mut Command,
    prefix_args: &[OsString],
    port: u16,
    supports_no_open: bool,
) -> &'a mut Command {
    let command =
        command
            .args(prefix_args)
            .args(["web", "--host", "127.0.0.1", "--port", &port.to_string()]);
    if supports_no_open {
        command.arg("--no-open");
    }
    command
}

fn detects_no_open_support(
    executable: &Path,
    prefix_args: &[OsString],
    runtime_path: Option<&OsStr>,
    process_supervisor: &ProcessSupervisor,
) -> bool {
    matches!(
        web_help_output(executable, prefix_args, runtime_path, process_supervisor),
        Ok(output) if output.status.success() && supports_no_open_flag(&output.stdout)
    )
}

fn web_help_output(
    executable: &Path,
    prefix_args: &[OsString],
    runtime_path: Option<&OsStr>,
    process_supervisor: &ProcessSupervisor,
) -> std::io::Result<std::process::Output> {
    let mut command = Command::new(executable);
    command
        .args(prefix_args)
        .args(["web", "--help"])
        .stdin(Stdio::null());
    hide_console_window(&mut command);
    if let Some(runtime_path) = runtime_path {
        command.env("PATH", runtime_path);
    }
    process_supervisor.output(&mut command)
}

fn supports_no_open_flag(help: &[u8]) -> bool {
    String::from_utf8_lossy(help).contains("--no-open")
}

fn capture_output(
    output: impl std::io::Read + Send + 'static,
    stream: &'static str,
    logs: Arc<Mutex<VecDeque<String>>>,
    launch_url: Option<Arc<Mutex<LaunchUrl>>>,
) {
    thread::spawn(move || {
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            if let Some(launch_url) = launch_url.as_ref() {
                if let Some(launch) = parse_launch_url_line(&line) {
                    if let Ok(mut slot) = launch_url.lock() {
                        *slot = launch;
                    }
                }
            }
            if let Ok(mut lines) = logs.lock() {
                lines.push_back(format!("[{stream}] {line}"));
                while lines.len() > MAX_LOG_LINES {
                    lines.pop_front();
                }
            }
        }
    });
}

fn apply_dsh_home(command: &mut Command, dsh_home: &DshHome) {
    if let DshHome::Custom { path } = dsh_home {
        command.env("DSH_HOME", path);
    }
}

fn recent_logs(logs: &Arc<Mutex<VecDeque<String>>>) -> String {
    logs.lock()
        .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("\n"))
        .unwrap_or_default()
}

fn read_version(
    executable: &Path,
    prefix_args: &[OsString],
    runtime_path: Option<&OsStr>,
    invalid_error: &str,
    process_supervisor: &ProcessSupervisor,
) -> AppResult<String> {
    let mut command = Command::new(executable);
    command
        .args(prefix_args)
        .arg("--version")
        .stdin(Stdio::null());
    hide_console_window(&mut command);
    if let Some(runtime_path) = runtime_path {
        command.env("PATH", runtime_path);
    }
    let output = process_supervisor
        .output(&mut command)
        .map_err(|error| AppError::new(invalid_error).technical(error.to_string()))?;
    if !output.status.success() {
        return Err(AppError::new(invalid_error).technical(String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console_window(_: &mut Command) {}

fn npx_package_argument(version: &str) -> OsString {
    OsString::from(format!("@deepseek-ai/dsh@{version}"))
}

fn prepend_paths(paths: &[PathBuf], inherited: Option<&OsStr>) -> Option<OsString> {
    let mut paths = paths.to_vec();
    if let Some(inherited) = inherited {
        paths.extend(env::split_paths(inherited));
    }
    env::join_paths(paths).ok()
}

fn resolve_executable(setting: Option<&str>, runtime_path: Option<&OsStr>) -> AppResult<PathBuf> {
    if let Some(setting) = setting {
        let path = PathBuf::from(setting);
        if path.is_file() {
            return Ok(path.canonicalize().unwrap_or(path));
        }
        return Err(
            AppError::new("service.error.executableNotFound").arg("executable", setting.to_owned())
        );
    }

    runtime_path
        .and_then(|path| find_on_path("dsh", path))
        .ok_or_else(|| AppError::new("service.error.executableNotFound"))
}

fn find_on_path(command: &str, path: &OsStr) -> Option<PathBuf> {
    let candidates = executable_names(command);
    env::split_paths(path)
        .flat_map(|directory| candidates.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
        .map(|path| path.canonicalize().unwrap_or(path))
}

fn effective_path(process_supervisor: &ProcessSupervisor) -> Option<OsString> {
    shell_path(process_supervisor).or_else(|| env::var_os("PATH"))
}

#[cfg(windows)]
fn shell_path(_: &ProcessSupervisor) -> Option<OsString> {
    None
}

#[cfg(unix)]
fn shell_path(process_supervisor: &ProcessSupervisor) -> Option<OsString> {
    let shell = env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));
    let mut command = Command::new(shell);
    command
        .args(["-ilc", "printf '\\n__DSH_DESKTOP_PATH__%s\\n' \"$PATH\""])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let (mut child, mut process_lease) = process_supervisor.spawn(&mut command).ok()?;
    let started_at = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                process_lease.release();
                break;
            }
            Ok(Some(_)) => {
                process_lease.release();
                return None;
            }
            Err(_) => {
                terminate_child_tree(&mut child);
                process_lease.release();
                return None;
            }
            Ok(None) if started_at.elapsed() < SHELL_PATH_TIMEOUT => {
                thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                terminate_child_tree(&mut child);
                process_lease.release();
                return None;
            }
        }
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    extract_marked_path(&output).map(OsString::from)
}

fn host_shutting_down_error() -> AppError {
    AppError::new("app.error.hostShuttingDown")
}

fn extract_marked_path(output: &str) -> Option<&str> {
    output
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(PATH_MARKER))
        .filter(|path| !path.is_empty())
}

#[cfg(windows)]
fn executable_names(command: &str) -> Vec<String> {
    let extensions = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned());
    extensions
        .split(';')
        .map(|extension| format!("{command}{extension}"))
        .collect()
}

#[cfg(not(windows))]
fn executable_names(command: &str) -> Vec<String> {
    vec![command.to_owned()]
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use crate::model::DshHome;

    use super::{
        ProbeResult, add_managed_web_arguments, apply_dsh_home, classify_probe_response,
        extract_marked_path, npx_package_argument, parse_launch_url_line, supports_no_open_flag,
    };

    #[test]
    fn classifies_the_legacy_identity_page_as_dsh() {
        assert_eq!(
            classify_probe_response(
                200,
                "<html><head><title>DeepSeek Harness</title></head></html>",
            ),
            ProbeResult::Dsh
        );
    }

    #[test]
    fn classifies_the_authentication_challenge_as_dsh() {
        assert_eq!(
            classify_probe_response(
                401,
                "dsh web authentication required; reopen the URL printed by dsh web.\n",
            ),
            ProbeResult::Dsh
        );
    }

    #[test]
    fn classifies_foreign_http_responses_as_other_http() {
        assert_eq!(
            classify_probe_response(200, "<html><head><title>Router</title></head></html>"),
            ProbeResult::OtherHttp
        );
        assert_eq!(
            classify_probe_response(401, "unauthorized"),
            ProbeResult::OtherHttp
        );
        assert_eq!(classify_probe_response(403, ""), ProbeResult::OtherHttp);
        assert_eq!(
            classify_probe_response(503, "dsh web authentication required"),
            ProbeResult::OtherHttp
        );
    }

    #[test]
    fn extracts_the_launch_token_from_the_url_line() {
        let launch = parse_launch_url_line(
            "dsh web: http://127.0.0.1:3080/?token=abc123 (LAN: http://192.168.0.2:3080/?token=abc123)",
        )
        .expect("recognizes the launch url line");
        assert!(launch.seen);
        assert_eq!(launch.token.as_deref(), Some("abc123"));
    }

    #[test]
    fn recognizes_a_launch_url_line_without_a_token() {
        let launch = parse_launch_url_line("dsh web: http://127.0.0.1:3080/")
            .expect("recognizes the launch url line");
        assert!(launch.seen);
        assert_eq!(launch.token, None);
    }

    #[test]
    fn ignores_lines_that_are_not_the_launch_url() {
        assert!(parse_launch_url_line("dsh: starting profile web").is_none());
        assert!(parse_launch_url_line("[stderr] dsh web: not a real url").is_none());
    }

    #[test]
    fn extracts_path_after_shell_startup_output() {
        let output = "shell banner\n__DSH_DESKTOP_PATH__/usr/local/bin:/usr/bin\n";
        assert_eq!(extract_marked_path(output), Some("/usr/local/bin:/usr/bin"));
    }

    #[test]
    fn custom_home_sets_dsh_home_for_child_only() {
        let mut command = Command::new("dsh");
        apply_dsh_home(
            &mut command,
            &DshHome::Custom {
                path: "/tmp/dsh-home".to_owned(),
            },
        );
        assert!(command.get_envs().any(|(name, value)| {
            name == "DSH_HOME" && value.is_some_and(|value| value == "/tmp/dsh-home")
        }));
    }

    #[test]
    fn environment_home_does_not_override_child_environment() {
        let mut command = Command::new("dsh");
        apply_dsh_home(&mut command, &DshHome::Environment);
        assert!(command.get_envs().all(|(name, _)| name != "DSH_HOME"));
    }

    #[test]
    fn npx_source_always_targets_the_official_dsh_package() {
        assert_eq!(
            npx_package_argument("0.1.0-rc.6"),
            "@deepseek-ai/dsh@0.1.0-rc.6"
        );
    }

    #[test]
    fn managed_web_processes_disable_dsh_browser_handoff_when_supported() {
        let mut command = Command::new("dsh");
        add_managed_web_arguments(&mut command, &[], 3080, true);

        let arguments = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            arguments,
            ["web", "--host", "127.0.0.1", "--port", "3080", "--no-open"]
        );
    }

    #[test]
    fn managed_web_processes_remain_compatible_with_older_dsh() {
        let mut command = Command::new("dsh");
        add_managed_web_arguments(&mut command, &[], 3080, false);

        let arguments = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(arguments, ["web", "--host", "127.0.0.1", "--port", "3080"]);
    }

    #[test]
    fn recognizes_the_dsh_no_open_flag_in_web_help() {
        assert!(supports_no_open_flag(b"Usage: dsh web --no-open"));
        assert!(!supports_no_open_flag(b"Usage: dsh web"));
    }
}
