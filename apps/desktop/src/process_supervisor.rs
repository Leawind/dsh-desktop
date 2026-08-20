use std::collections::HashSet;
use std::io::Read;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[derive(Clone, Default)]
pub struct ProcessSupervisor {
    tracked: Arc<Mutex<HashSet<u32>>>,
    shutting_down: Arc<AtomicBool>,
}

impl ProcessSupervisor {
    pub fn spawn(&self, command: &mut Command) -> std::io::Result<(Child, ProcessLease)> {
        if self.shutting_down.load(Ordering::Acquire) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "DSH Desktop Host is shutting down",
            ));
        }
        configure_process_tree(command);
        let mut child = command.spawn()?;
        let mut lease = self.track(child.id());
        if self.shutting_down.load(Ordering::Acquire) {
            terminate_child_tree(&mut child);
            lease.release();
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "DSH Desktop Host is shutting down",
            ));
        }
        Ok((child, lease))
    }

    pub fn output(&self, command: &mut Command) -> std::io::Result<Output> {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let (child, _lease) = self.spawn(command)?;
        child.wait_with_output()
    }

    pub fn output_with_timeout(
        &self,
        command: &mut Command,
        timeout: Duration,
    ) -> std::io::Result<Option<Output>> {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let (mut child, mut lease) = self.spawn(command)?;
        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Ok(None) => {
                    terminate_child_tree(&mut child);
                    return Ok(None);
                }
                Err(error) => {
                    terminate_child_tree(&mut child);
                    return Err(error);
                }
            }
        };
        lease.release();

        let mut stdout = Vec::new();
        if let Some(mut output) = child.stdout.take() {
            output.read_to_end(&mut stdout)?;
        }
        let mut stderr = Vec::new();
        if let Some(mut output) = child.stderr.take() {
            output.read_to_end(&mut stderr)?;
        }
        Ok(Some(Output {
            status,
            stdout,
            stderr,
        }))
    }

    pub fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::Release);
        let pids = self
            .tracked
            .lock()
            .map(|tracked| tracked.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        for pid in pids {
            terminate_process_tree(pid);
        }
    }

    fn track(&self, pid: u32) -> ProcessLease {
        if let Ok(mut tracked) = self.tracked.lock() {
            tracked.insert(pid);
        }
        ProcessLease {
            supervisor: self.clone(),
            pid,
            active: true,
        }
    }

    fn untrack(&self, pid: u32) {
        if let Ok(mut tracked) = self.tracked.lock() {
            tracked.remove(&pid);
        }
    }

    #[cfg(test)]
    fn tracked_count(&self) -> usize {
        self.tracked
            .lock()
            .map(|tracked| tracked.len())
            .unwrap_or_default()
    }
}

pub struct ProcessLease {
    supervisor: ProcessSupervisor,
    pid: u32,
    active: bool,
}

impl ProcessLease {
    pub fn release(&mut self) {
        if self.active {
            self.supervisor.untrack(self.pid);
            self.active = false;
        }
    }
}

impl Drop for ProcessLease {
    fn drop(&mut self) {
        self.release();
    }
}

pub fn terminate_child_tree(child: &mut Child) {
    match child.try_wait() {
        Ok(Some(_)) => return,
        Ok(None) | Err(_) => {}
    }
    terminate_process_tree(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

fn configure_process_tree(command: &mut Command) {
    #[cfg(unix)]
    command.process_group(0);

    #[cfg(windows)]
    hide_console_window(command);

    #[cfg(not(any(unix, windows)))]
    let _ = command;
}

fn terminate_process_tree(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    #[cfg(windows)]
    {
        let pid = pid.to_string();
        let mut taskkill = Command::new("taskkill");
        taskkill
            .args(["/PID", &pid, "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_console_window(&mut taskkill);
        let _ = taskkill.status();
    }

    #[cfg(not(any(unix, windows)))]
    let _ = pid;
}

#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releases_processes_when_their_lease_is_dropped() {
        let supervisor = ProcessSupervisor::default();
        let lease = supervisor.track(42);
        assert_eq!(supervisor.tracked_count(), 1);
        drop(lease);
        assert_eq!(supervisor.tracked_count(), 0);
    }

    #[test]
    fn refuses_to_spawn_after_shutdown_begins() {
        let supervisor = ProcessSupervisor::default();
        supervisor.shutdown();

        let error = match supervisor.spawn(&mut Command::new("unreachable-command")) {
            Ok(_) => panic!("shutdown prevents a spawn"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
    }
}
