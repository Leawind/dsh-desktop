use std::collections::VecDeque;
use std::env;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;

use crate::error::{AppError, AppResult};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_LOG_LINES: usize = 400;

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
}

pub fn probe(url: &str) -> ProbeResult {
    let client = match Client::builder().timeout(PROBE_TIMEOUT).build() {
        Ok(client) => client,
        Err(_) => return ProbeResult::Unreachable,
    };
    let response = match client.get(url).send() {
        Ok(response) => response,
        Err(_) => return ProbeResult::Unreachable,
    };
    if !response.status().is_success() {
        return ProbeResult::OtherHttp;
    }
    let body = match response.text() {
        Ok(body) => body,
        Err(_) => return ProbeResult::OtherHttp,
    };
    if body.contains("<title>DeepSeek Harness</title>") {
        ProbeResult::Dsh
    } else {
        ProbeResult::OtherHttp
    }
}

pub fn start(executable_setting: Option<&str>, port: u16) -> AppResult<ManagedService> {
    let executable = resolve_executable(executable_setting)?;
    let runtime_version = read_version(&executable)?;
    let mut child = Command::new(&executable)
        .args(["web", "--host", "127.0.0.1", "--port", &port.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
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
        executable,
        runtime_version,
        logs,
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

fn read_version(executable: &Path) -> AppResult<String> {
    let output = Command::new(executable)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            AppError::new("service.error.invalidExecutable").technical(error.to_string())
        })?;
    if !output.status.success() {
        return Err(AppError::new("service.error.invalidExecutable")
            .technical(String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn resolve_executable(setting: Option<&str>) -> AppResult<PathBuf> {
    if let Some(setting) = setting {
        let path = PathBuf::from(setting);
        if path.is_file() {
            return Ok(path.canonicalize().unwrap_or(path));
        }
        return Err(
            AppError::new("service.error.executableNotFound").arg("executable", setting.to_owned())
        );
    }

    find_on_path("dsh").ok_or_else(|| AppError::new("service.error.executableNotFound"))
}

fn find_on_path(command: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let candidates = executable_names(command);
    env::split_paths(&path)
        .flat_map(|directory| candidates.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
        .map(|path| path.canonicalize().unwrap_or(path))
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
