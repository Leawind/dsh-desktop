use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, SystemTime};

const LOCK_FILE: &str = "host.lock";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);

pub struct PrimaryInstance {
    pub requests: Receiver<HostRequest>,
    _lease: Lease,
}

pub enum HostRequest {
    OpenWindow,
}

pub enum Claim {
    Primary(PrimaryInstance),
    Forwarded,
}

struct Lease {
    path: PathBuf,
    running: Arc<AtomicBool>,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        let _ = fs::remove_file(&self.path);
    }
}

pub fn claim(config_directory: &Path) -> std::io::Result<Claim> {
    fs::create_dir_all(config_directory)?;
    let path = config_directory.join(LOCK_FILE);
    let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;
    loop {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => return start_primary(path, &mut file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if forward_to_primary(&path)? {
                    return Ok(Claim::Forwarded);
                }
                if std::time::Instant::now() >= deadline {
                    // A Host writes the lock only after its listener is bound, so a
                    // non-connectable lock beyond the startup grace period is stale.
                    let _ = fs::remove_file(&path);
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(error),
        }
    }
}

fn start_primary(path: PathBuf, file: &mut fs::File) -> std::io::Result<Claim> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let token = format!(
        "{:032x}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            ^ u128::from(std::process::id())
    );
    writeln!(file, "{port} {token}")?;
    file.sync_all()?;

    let running = Arc::new(AtomicBool::new(true));
    let listener_running = Arc::clone(&running);
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        while listener_running.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = String::new();
                    let _ = BufReader::new(&mut stream).read_line(&mut request);
                    let accepted = request.trim() == format!("open {token}")
                        && sender.send(HostRequest::OpenWindow).is_ok();
                    let _ = stream.write_all(if accepted { b"ok\n" } else { b"error\n" });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(_) => break,
            }
        }
    });

    Ok(Claim::Primary(PrimaryInstance {
        requests: receiver,
        _lease: Lease { path, running },
    }))
}

fn forward_to_primary(path: &Path) -> std::io::Result<bool> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let Some((port, token)) = contents.trim().split_once(' ') else {
        return Ok(false);
    };
    let Ok(port) = port.parse::<u16>() else {
        return Ok(false);
    };
    let mut stream = match TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .expect("valid loopback address"),
        Duration::from_millis(250),
    ) {
        Ok(stream) => stream,
        Err(_) => return Ok(false),
    };
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    stream.write_all(format!("open {token}\n").as_bytes())?;
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response)?;
    Ok(response.trim() == "ok")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwards_a_second_launch_to_the_primary_host() {
        let directory = std::env::temp_dir().join(format!(
            "dsh-desktop-single-instance-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        let primary = match claim(&directory).expect("claim primary instance") {
            Claim::Primary(primary) => primary,
            Claim::Forwarded => panic!("first claim must become primary"),
        };

        assert!(matches!(
            claim(&directory).expect("forward second launch"),
            Claim::Forwarded
        ));
        assert!(matches!(
            primary
                .requests
                .recv_timeout(Duration::from_secs(1))
                .expect("primary receives launch request"),
            HostRequest::OpenWindow
        ));

        drop(primary);
        assert!(!directory.join(LOCK_FILE).exists());
        let _ = fs::remove_dir_all(directory);
    }
}
