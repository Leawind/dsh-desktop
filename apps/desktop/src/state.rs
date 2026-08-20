use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::model::{
    EndpointOwnership, EndpointSnapshot, GlobalSettings, HostSnapshot, ServiceStatus,
    SystemColorScheme, WindowSnapshot,
};
use crate::process_supervisor::ProcessSupervisor;
use crate::runtime::RuntimeManager;
use crate::service::ManagedService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostLifecycle {
    Running,
    ShuttingDown,
}

#[derive(Clone)]
pub struct AppState {
    pub config_dir: PathBuf,
    pub settings: Arc<RwLock<GlobalSettings>>,
    pub host: Arc<Mutex<HostState>>,
    pub startup_lock: Arc<Mutex<()>>,
    pub runtime_manager: RuntimeManager,
    system_color_scheme: Arc<RwLock<Option<SystemColorScheme>>>,
    next_window_id: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
    process_supervisor: ProcessSupervisor,
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
    pub managed: bool,
    pub launch: Option<crate::service::ManagedLaunch>,
    pub logs: Vec<String>,
    pub idle_since: Option<Instant>,
}

impl EndpointRecord {
    pub fn external(status: ServiceStatus) -> Self {
        Self {
            status,
            process: None,
            runtime_version: None,
            last_error: None,
            last_successful_connection: None,
            managed: false,
            launch: None,
            logs: Vec::new(),
            idle_since: None,
        }
    }
}

impl AppState {
    pub fn new(
        config_dir: PathBuf,
        settings: GlobalSettings,
        runtime_manager: RuntimeManager,
    ) -> Self {
        Self {
            config_dir,
            settings: Arc::new(RwLock::new(settings)),
            host: Arc::new(Mutex::new(HostState {
                windows: HashMap::new(),
                endpoints: HashMap::new(),
                next_connection_order: 1,
            })),
            startup_lock: Arc::new(Mutex::new(())),
            runtime_manager,
            system_color_scheme: Arc::new(RwLock::new(None)),
            next_window_id: Arc::new(AtomicU64::new(1)),
            running: Arc::new(AtomicBool::new(true)),
            process_supervisor: ProcessSupervisor::default(),
        }
    }

    pub fn lifecycle(&self) -> HostLifecycle {
        if self.running.load(Ordering::Acquire) {
            HostLifecycle::Running
        } else {
            HostLifecycle::ShuttingDown
        }
    }

    pub fn is_running(&self) -> bool {
        self.lifecycle() == HostLifecycle::Running
    }

    pub fn ensure_running(&self) -> crate::error::AppResult<()> {
        self.is_running()
            .then_some(())
            .ok_or_else(|| crate::error::AppError::new("app.error.hostShuttingDown"))
    }

    pub fn begin_shutdown(&self) -> bool {
        self.running.swap(false, Ordering::AcqRel)
    }

    pub fn running_signal(&self) -> &AtomicBool {
        &self.running
    }

    pub fn process_supervisor(&self) -> &ProcessSupervisor {
        &self.process_supervisor
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
        let idle_timeout = self.managed_service_idle_timeout_seconds();
        if let Ok(mut host) = self.host.lock() {
            let removed_url = host.windows.remove(label).map(|window| window.url);
            if let Some(url) = removed_url {
                host.mark_idle_if_unused(&url);
                host.reap_idle_services(idle_timeout);
            }
        }
    }

    pub fn managed_service_idle_timeout_seconds(&self) -> u64 {
        self.settings
            .read()
            .map(|settings| settings.managed_service_idle_timeout_seconds)
            .unwrap_or_default()
    }

    pub fn reap_idle_services(&self) -> Option<HostSnapshot> {
        let idle_timeout = self.managed_service_idle_timeout_seconds();
        let mut host = self.host.lock().ok()?;
        host.reap_idle_services(idle_timeout);
        Some(snapshot_locked(&host))
    }

    pub fn should_exit_after_idle_reap(&self) -> bool {
        self.host
            .lock()
            .map(|host| host.windows.is_empty() && !host.has_managed_processes())
            .unwrap_or(false)
    }

    pub fn shutdown(&self) {
        self.begin_shutdown();
        if let Ok(mut host) = self.host.lock() {
            for endpoint in host.endpoints.values_mut() {
                if let Some(process) = endpoint.process.as_mut() {
                    process.stop();
                }
            }
        }
        self.process_supervisor.shutdown();
    }

    pub fn system_color_scheme(&self) -> Option<SystemColorScheme> {
        self.system_color_scheme
            .read()
            .ok()
            .and_then(|scheme| *scheme)
    }

    pub fn refresh_system_color_scheme(&self) {
        let Some(scheme) = crate::system_appearance::detect() else {
            return;
        };
        if let Ok(mut current) = self.system_color_scheme.write() {
            *current = Some(scheme);
        }
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

        let idle_timeout = self.managed_service_idle_timeout_seconds();
        let mut host = self.host.lock().expect("host state poisoned");
        for (url, status) in probes {
            let status = host
                .endpoints
                .get(&url)
                .filter(|endpoint| {
                    matches!(
                        endpoint.status,
                        ServiceStatus::Starting
                            | ServiceStatus::Stopping
                            | ServiceStatus::Restarting
                            | ServiceStatus::Updating
                            | ServiceStatus::Failed
                    )
                })
                .map(|endpoint| endpoint.status)
                .unwrap_or(status);
            if let Some(endpoint) = host.endpoints.get_mut(&url) {
                endpoint.status = status;
            }
            for window in host.windows.values_mut().filter(|window| window.url == url) {
                window.status = status;
            }
        }
        host.reap_idle_services(idle_timeout);
        snapshot_locked(&host)
    }
}

impl HostState {
    pub fn has_managed_processes(&self) -> bool {
        self.endpoints
            .values()
            .any(|endpoint| endpoint.process.is_some())
    }

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
            endpoint.idle_since = None;
        }
    }

    pub fn assign_window(&mut self, label: &str, url: &str, status: ServiceStatus) -> bool {
        let Some(previous_url) = self.windows.get(label).map(|window| window.url.clone()) else {
            return false;
        };
        let window = self.windows.get_mut(label).expect("window disappeared");
        window.url = url.to_owned();
        window.status = status;
        if previous_url != url {
            self.mark_idle_if_unused(&previous_url);
        }
        if let Some(endpoint) = self.endpoints.get_mut(url) {
            endpoint.idle_since = None;
        }
        true
    }

    fn mark_idle_if_unused(&mut self, url: &str) {
        let used = self.windows.values().any(|window| window.url == url);
        if !used {
            if let Some(endpoint) = self.endpoints.get_mut(url) {
                if endpoint.managed {
                    endpoint.idle_since.get_or_insert_with(Instant::now);
                }
            }
        }
    }

    pub fn reap_idle_services(&mut self, timeout_seconds: u64) {
        reap_idle_services(self, timeout_seconds);
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
                    ownership: if endpoint.managed {
                        EndpointOwnership::Managed
                    } else {
                        EndpointOwnership::External
                    },
                    connected_windows,
                    pid: endpoint.process.as_ref().map(ManagedService::pid),
                    runtime_version: endpoint.runtime_version.clone(),
                    last_error: endpoint.last_error.clone(),
                    known: endpoint.last_successful_connection.is_some(),
                    can_stop: endpoint.process.is_some()
                        && matches!(endpoint.status, ServiceStatus::Running),
                    can_restart: endpoint.managed
                        && endpoint.launch.is_some()
                        && !matches!(
                            endpoint.status,
                            ServiceStatus::Starting
                                | ServiceStatus::Stopping
                                | ServiceStatus::Restarting
                                | ServiceStatus::Updating
                        ),
                    logs: endpoint
                        .process
                        .as_ref()
                        .map(ManagedService::log_lines)
                        .unwrap_or_else(|| endpoint.logs.clone()),
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
        match process.try_wait() {
            Ok(Some(status)) => {
                endpoint.logs = process.log_lines();
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
                process.stop();
                endpoint.status = ServiceStatus::Failed;
                endpoint.last_error = Some(error.to_string());
            }
        }
    }
}

fn reap_idle_services(host: &mut HostState, timeout_seconds: u64) {
    let timeout = Duration::from_secs(timeout_seconds);
    for endpoint in host.endpoints.values_mut().filter(|endpoint| {
        endpoint
            .idle_since
            .is_some_and(|idle_since| idle_since.elapsed() >= timeout)
    }) {
        if let Some(mut process) = endpoint.process.take() {
            endpoint.logs = process.log_lines();
            process.stop();
        }
        endpoint.status = ServiceStatus::Unreachable;
        endpoint.last_error = None;
        endpoint.idle_since = None;
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

    #[test]
    fn zero_idle_timeout_reaps_an_idle_endpoint_immediately() {
        let mut endpoint = EndpointRecord::external(ServiceStatus::Running);
        endpoint.managed = true;
        endpoint.idle_since = Some(Instant::now());
        let mut host = HostState {
            windows: HashMap::new(),
            endpoints: HashMap::from([("http://127.0.0.1:3080".to_owned(), endpoint)]),
            next_connection_order: 1,
        };

        host.reap_idle_services(0);

        let endpoint = host
            .endpoints
            .get("http://127.0.0.1:3080")
            .expect("endpoint remains registered");
        assert_eq!(endpoint.status, ServiceStatus::Unreachable);
        assert!(endpoint.idle_since.is_none());
    }

    #[test]
    fn does_not_count_a_reaped_managed_endpoint_as_a_running_service() {
        let mut endpoint = EndpointRecord::external(ServiceStatus::Unreachable);
        endpoint.managed = true;
        let host = HostState {
            windows: HashMap::new(),
            endpoints: HashMap::from([("http://127.0.0.1:3080".to_owned(), endpoint)]),
            next_connection_order: 1,
        };

        assert!(!host.has_managed_processes());
    }

    #[test]
    fn shutdown_transitions_the_host_lifecycle_once() {
        let state = AppState::new(
            PathBuf::from("config"),
            GlobalSettings::default(),
            RuntimeManager::new(PathBuf::from("seed"), PathBuf::from("data")),
        );

        assert_eq!(state.lifecycle(), HostLifecycle::Running);
        assert!(state.begin_shutdown());
        assert_eq!(state.lifecycle(), HostLifecycle::ShuttingDown);
        assert!(!state.begin_shutdown());
        assert_eq!(
            state
                .ensure_running()
                .expect_err("shutdown rejects commands")
                .code,
            "app.error.hostShuttingDown"
        );
    }
}
