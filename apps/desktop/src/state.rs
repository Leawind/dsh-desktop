use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::endpoint::default_dsh_url;
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
    next_window_id: Arc<AtomicU64>,
}

pub struct HostState {
    pub windows: HashMap<String, WindowRecord>,
    pub endpoints: HashMap<String, EndpointRecord>,
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
}

impl AppState {
    pub fn new(config_dir: PathBuf, settings: GlobalSettings) -> Self {
        Self {
            config_dir,
            settings: Arc::new(RwLock::new(settings)),
            host: Arc::new(Mutex::new(HostState {
                windows: HashMap::new(),
                endpoints: HashMap::new(),
            })),
            next_window_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn next_window_label(&self) -> String {
        let id = self.next_window_id.fetch_add(1, Ordering::Relaxed);
        format!("dsh-{id}")
    }

    pub fn default_url(&self) -> String {
        let port = self
            .settings
            .read()
            .map(|settings| settings.default_dsh_port)
            .unwrap_or(crate::model::DEFAULT_DSH_PORT);
        default_dsh_url(port)
    }

    pub fn register_window(&self, label: &str) -> WindowSnapshot {
        let default_url = self.default_url();
        let mut host = self.host.lock().expect("host state poisoned");
        let window = host
            .windows
            .entry(label.to_owned())
            .or_insert_with(|| WindowRecord {
                url: default_url,
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
        if let Ok(mut host) = self.host.lock() {
            for endpoint in host.endpoints.values_mut() {
                if let Some(process) = endpoint.process.as_mut() {
                    process.stop();
                }
            }
        }
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
            }
        })
        .collect::<Vec<_>>();
    endpoints.sort_by(|left, right| left.url.cmp(&right.url));

    HostSnapshot { windows, endpoints }
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
