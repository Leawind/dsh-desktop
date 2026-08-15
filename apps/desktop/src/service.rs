use std::collections::VecDeque;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;

use crate::error::{AppError, AppResult};
use crate::model::DshSource;
use crate::runtime::RuntimeManager;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const SHELL_PATH_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_LOG_LINES: usize = 400;
const MAX_PROBE_BODY_BYTES: u64 = 1024 * 1024;
const PATH_MARKER: &str = "__DSH_DESKTOP_PATH__";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeResult {
    Unreachable,
    Dsh,
    OtherHttp,
}

pub struct ManagedService {
    pub child: Child,
    pub executable: PathBuf,
    pub runtime_version: String,
    pub logs: Arc<Mutex<VecDeque<String>>>,
    pub launch: ManagedLaunch,
}

#[derive(Clone)]
pub struct ResolvedDshRuntime {
    executable: PathBuf,
    prefix_args: Vec<OsString>,
    runtime_path: Option<OsString>,
    runtime_version: String,
}

#[derive(Clone)]
pub struct ManagedLaunch {
    runtime: ResolvedDshRuntime,
    port: u16,
    dsh_home: PathBuf,
}

impl ManagedService {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
    if !response.status().is_success() {
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
    if body.contains("<title>DeepSeek Harness</title>") {
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
) -> AppResult<ResolvedDshRuntime> {
    let runtime_path = effective_path();
    let (executable, prefix_args, runtime_path, known_version) = match source {
        DshSource::None => return Err(AppError::new("service.error.sourceDisabled")),
        DshSource::BuiltIn => {
            let runtime = runtime_manager.resolve_built_in()?;
            let runtime_path = prepend_paths(&runtime.executable_path, runtime_path.as_deref());
            (
                runtime.node_executable,
                vec![runtime.dsh_entrypoint.into_os_string()],
                runtime_path,
                Some(runtime.dsh_version),
            )
        }
        DshSource::System => (
            resolve_executable(None, runtime_path.as_deref())?,
            Vec::new(),
            runtime_path,
            None,
        ),
        DshSource::Custom { executable } => (
            resolve_executable(Some(executable), runtime_path.as_deref())?,
            Vec::new(),
            runtime_path,
            None,
        ),
    };
    let runtime_version = read_version(&executable, &prefix_args, runtime_path.as_deref())?;
    if known_version
        .as_ref()
        .is_some_and(|expected| expected != &runtime_version)
    {
        return Err(AppError::new("runtime.error.versionMismatch")
            .arg("expected", known_version)
            .arg("actual", runtime_version));
    }
    Ok(ResolvedDshRuntime {
        executable,
        prefix_args,
        runtime_path,
        runtime_version,
    })
}

pub fn start(
    runtime: &ResolvedDshRuntime,
    port: u16,
    dsh_home: PathBuf,
) -> AppResult<ManagedService> {
    start_launch(&ManagedLaunch {
        runtime: runtime.clone(),
        port,
        dsh_home,
    })
}

pub fn start_launch(launch: &ManagedLaunch) -> AppResult<ManagedService> {
    let runtime = &launch.runtime;
    let port = launch.port;
    let executable = &runtime.executable;
    std::fs::create_dir_all(&launch.dsh_home).map_err(|error| {
        AppError::new("service.error.homeUnavailable").technical(error.to_string())
    })?;
    let mut command = Command::new(&executable);
    command
        .args(&runtime.prefix_args)
        .args(["web", "--host", "127.0.0.1", "--port", &port.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env("DSH_HOME", &launch.dsh_home);
    command.env("PNPM_HOME", launch.dsh_home.join("pnpm"));
    command.env(
        "npm_config_store_dir",
        launch.dsh_home.join("pnpm").join("store"),
    );
    #[cfg(target_os = "linux")]
    crate::direct_network::restore_child_proxy_environment(&mut command);
    if let Some(runtime_path) = runtime.runtime_path.as_ref() {
        command.env("PATH", runtime_path);
    }
    let mut child = command.spawn().map_err(|error| {
        AppError::new("service.error.startFailed")
            .arg("executable", executable.display().to_string())
            .technical(error.to_string())
    })?;

    let logs = Arc::new(Mutex::new(VecDeque::new()));
    if let Some(stdout) = child.stdout.take() {
        capture_output(stdout, "stdout", Arc::clone(&logs));
    }
    if let Some(stderr) = child.stderr.take() {
        capture_output(stderr, "stderr", Arc::clone(&logs));
    }

    let url = format!("http://127.0.0.1:{port}");
    let started_at = Instant::now();
    loop {
        match probe(&url) {
            ProbeResult::Dsh => break,
            ProbeResult::OtherHttp => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::new("service.error.portOccupied").arg("port", port));
            }
            ProbeResult::Unreachable => {}
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(AppError::new("service.error.processExited")
                    .arg("status", status.to_string())
                    .technical(recent_logs(&logs)));
            }
            Ok(None) => {}
            Err(error) => {
                return Err(AppError::new("service.error.startFailed").technical(error.to_string()));
            }
        }

        if started_at.elapsed() >= STARTUP_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new("service.error.startTimeout")
                .arg("port", port)
                .technical(recent_logs(&logs)));
        }
        thread::sleep(Duration::from_millis(200));
    }

    Ok(ManagedService {
        child,
        executable: executable.clone(),
        runtime_version: runtime.runtime_version.clone(),
        logs,
        launch: launch.clone(),
    })
}

fn capture_output(
    output: impl std::io::Read + Send + 'static,
    stream: &'static str,
    logs: Arc<Mutex<VecDeque<String>>>,
) {
    thread::spawn(move || {
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            if let Ok(mut lines) = logs.lock() {
                lines.push_back(format!("[{stream}] {line}"));
                while lines.len() > MAX_LOG_LINES {
                    lines.pop_front();
                }
            }
        }
    });
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
) -> AppResult<String> {
    let mut command = Command::new(executable);
    command
        .args(prefix_args)
        .arg("--version")
        .stdin(Stdio::null());
    if let Some(runtime_path) = runtime_path {
        command.env("PATH", runtime_path);
    }
    let output = command.output().map_err(|error| {
        AppError::new("service.error.invalidExecutable").technical(error.to_string())
    })?;
    if !output.status.success() {
        return Err(AppError::new("service.error.invalidExecutable")
            .technical(String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
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

fn effective_path() -> Option<OsString> {
    shell_path().or_else(|| env::var_os("PATH"))
}

#[cfg(windows)]
fn shell_path() -> Option<OsString> {
    None
}

#[cfg(unix)]
fn shell_path() -> Option<OsString> {
    let shell = env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));
    let mut child = Command::new(shell)
        .args(["-ilc", "printf '\\n__DSH_DESKTOP_PATH__%s\\n' \"$PATH\""])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started_at = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if started_at.elapsed() < SHELL_PATH_TIMEOUT => {
                thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    extract_marked_path(&output).map(OsString::from)
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
    use super::extract_marked_path;

    #[test]
    fn extracts_path_after_shell_startup_output() {
        let output = "shell banner\n__DSH_DESKTOP_PATH__/usr/local/bin:/usr/bin\n";
        assert_eq!(extract_marked_path(output), Some("/usr/local/bin:/usr/bin"));
    }
}
