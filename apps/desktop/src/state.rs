use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::model::{
    EndpointOwnership, EndpointSnapshot, GlobalSettings, HostSnapshot, ServiceStatus,
    WindowSnapshot,
};
use crate::service::ManagedService;

#[derive(Clone)]
pub struct AppState {
    pub config_dir: PathBuf,
    pub settings: Arc<RwLock<GlobalSettings>>,
    pub host: Arc<Mutex<HostState>>,
    pub startup_lock: Arc<Mutex<()>>,
    monitor_stopped: Arc<AtomicBool>,
    next_window_id: Arc<AtomicU64>,
}

pub struct HostState {
    pub windows: HashMap<String, WindowRecord>,
    pub endpoints: HashMap<String, EndpointRecord>,
    next_connection_order: u64,
}

pub struct WindowRecord {
    pub url: String,
    pub status: ServiceStatus,
}

pub struct EndpointRecord {
    pub status: ServiceStatus,
    pub process: Option<ManagedService>,
    pub runtime_version: Option<String>,
    pub last_error: Option<String>,
    pub last_successful_connection: Option<u64>,
}

impl EndpointRecord {
    pub fn external(status: ServiceStatus) -> Self {
        Self {
            status,
            process: None,
            runtime_version: None,
            last_error: None,
            last_successful_connection: None,
        }
    }
}

impl AppState {
    pub fn new(config_dir: PathBuf, settings: GlobalSettings) -> Self {
        Self {
            config_dir,
            settings: Arc::new(RwLock::new(settings)),
            host: Arc::new(Mutex::new(HostState {
                windows: HashMap::new(),
                endpoints: HashMap::new(),
                next_connection_order: 1,
            })),
            startup_lock: Arc::new(Mutex::new(())),
            monitor_stopped: Arc::new(AtomicBool::new(false)),
            next_window_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn next_window_label(&self) -> String {
        let id = self.next_window_id.fetch_add(1, Ordering::Relaxed);
        format!("dsh-{id}")
    }

    pub fn register_window(&self, label: &str) -> WindowSnapshot {
        let mut host = self.host.lock().expect("host state poisoned");
        let window = host
            .windows
            .entry(label.to_owned())
            .or_insert_with(|| WindowRecord {
                url: String::new(),
                status: ServiceStatus::Unreachable,
            });
        WindowSnapshot {
            label: label.to_owned(),
            url: window.url.clone(),
            status: window.status,
        }
    }

    pub fn snapshot(&self) -> HostSnapshot {
        let mut host = self.host.lock().expect("host state poisoned");
        refresh_processes(&mut host);
        snapshot_locked(&host)
    }

    pub fn remove_window(&self, label: &str) {
        if let Ok(mut host) = self.host.lock() {
            host.windows.remove(label);
        }
    }

    pub fn shutdown(&self) {
        self.monitor_stopped.store(true, Ordering::Relaxed);
        if let Ok(mut host) = self.host.lock() {
            for endpoint in host.endpoints.values_mut() {
                if let Some(process) = endpoint.process.as_mut() {
                    process.stop();
                }
            }
        }
    }

    pub fn monitor_stopped(&self) -> bool {
        self.monitor_stopped.load(Ordering::Relaxed)
    }

    pub fn refresh_endpoint_health(&self) -> HostSnapshot {
        let urls = {
            let mut host = self.host.lock().expect("host state poisoned");
            refresh_processes(&mut host);
            let mut urls = host
                .windows
                .values()
                .filter_map(|window| (!window.url.is_empty()).then_some(window.url.clone()))
                .collect::<Vec<_>>();
            urls.sort();
            urls.dedup();
            urls
        };

        let probes = urls
            .into_iter()
            .map(|url| {
                let status = if crate::service::probe(&url) == crate::service::ProbeResult::Dsh {
                    ServiceStatus::Running
                } else {
                    ServiceStatus::Unreachable
                };
                (url, status)
            })
            .collect::<Vec<_>>();

        let mut host = self.host.lock().expect("host state poisoned");
        for (url, status) in probes {
            let status = host
                .endpoints
                .get(&url)
                .filter(|endpoint| {
                    endpoint.process.is_some() && endpoint.status == ServiceStatus::Failed
                })
                .map(|_| ServiceStatus::Failed)
                .unwrap_or(status);
            if let Some(endpoint) = host.endpoints.get_mut(&url) {
                endpoint.status = status;
            }
            for window in host.windows.values_mut().filter(|window| window.url == url) {
                window.status = status;
            }
        }
        snapshot_locked(&host)
    }
}

impl HostState {
    pub fn known_endpoint_urls(&self) -> Vec<String> {
        let mut endpoints = self
            .endpoints
            .iter()
            .filter_map(|(url, endpoint)| {
                endpoint
                    .last_successful_connection
                    .map(|order| (order, url.clone()))
            })
            .collect::<Vec<_>>();
        endpoints.sort_by(|left, right| right.0.cmp(&left.0));
        endpoints.into_iter().map(|(_, url)| url).collect()
    }

    pub fn record_connection(&mut self, url: &str) {
        let order = self.next_connection_order;
        self.next_connection_order = self.next_connection_order.saturating_add(1);
        if let Some(endpoint) = self.endpoints.get_mut(url) {
            endpoint.status = ServiceStatus::Running;
            endpoint.last_error = None;
            endpoint.last_successful_connection = Some(order);
        }
    }

    pub fn assign_window(&mut self, label: &str, url: &str, status: ServiceStatus) -> bool {
        let Some(window) = self.windows.get_mut(label) else {
            return false;
        };
        window.url = url.to_owned();
        window.status = status;
        true
    }

    pub fn window_snapshot(&self, label: &str) -> Option<WindowSnapshot> {
        self.windows.get(label).map(|window| WindowSnapshot {
            label: label.to_owned(),
            url: window.url.clone(),
            status: window.status,
        })
    }
}

pub fn snapshot_locked(host: &HostState) -> HostSnapshot {
    let mut windows = host
        .windows
        .iter()
        .map(|(label, window)| WindowSnapshot {
            label: label.clone(),
            url: window.url.clone(),
            status: window.status,
        })
        .collect::<Vec<_>>();
    windows.sort_by(|left, right| left.label.cmp(&right.label));

    let mut endpoints = host
        .endpoints
        .iter()
        .map(|(url, endpoint)| {
            let connected_windows = host
                .windows
                .values()
                .filter(|window| window.url == *url)
                .count();
            (
                endpoint.last_successful_connection,
                EndpointSnapshot {
                    url: url.clone(),
                    status: endpoint.status,
                    ownership: if endpoint.process.is_some() {
                        EndpointOwnership::Managed
                    } else {
                        EndpointOwnership::External
                    },
                    connected_windows,
                    pid: endpoint.process.as_ref().map(ManagedService::pid),
                    runtime_version: endpoint.runtime_version.clone(),
                    last_error: endpoint.last_error.clone(),
                    known: endpoint.last_successful_connection.is_some(),
                },
            )
        })
        .collect::<Vec<_>>();
    endpoints.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.url.cmp(&right.1.url))
    });

    HostSnapshot {
        windows,
        endpoints: endpoints
            .into_iter()
            .map(|(_, endpoint)| endpoint)
            .collect(),
    }
}

fn refresh_processes(host: &mut HostState) {
    for (url, endpoint) in &mut host.endpoints {
        let Some(process) = endpoint.process.as_mut() else {
            continue;
        };
        match process.child.try_wait() {
            Ok(Some(status)) => {
                endpoint.status = ServiceStatus::Failed;
                endpoint.last_error = Some(format!(
                    "process exited with {status}\n{}",
                    process.diagnostic()
                ));
                for window in host
                    .windows
                    .values_mut()
                    .filter(|window| window.url == *url)
                {
                    window.status = ServiceStatus::Failed;
                }
            }
            Ok(None) => {}
            Err(error) => {
                endpoint.status = ServiceStatus::Failed;
                endpoint.last_error = Some(error.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_endpoints_follow_most_recent_connection_order() {
        let mut host = HostState {
            windows: HashMap::new(),
            endpoints: HashMap::from([
                (
                    "http://127.0.0.1:3080".to_owned(),
                    EndpointRecord::external(ServiceStatus::Running),
                ),
                (
                    "http://127.0.0.1:3081".to_owned(),
                    EndpointRecord::external(ServiceStatus::Running),
                ),
            ]),
            next_connection_order: 1,
        };
        host.record_connection("http://127.0.0.1:3080");
        host.record_connection("http://127.0.0.1:3081");
        assert_eq!(
            host.known_endpoint_urls(),
            ["http://127.0.0.1:3081", "http://127.0.0.1:3080"]
        );
    }
}
