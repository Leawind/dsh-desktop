use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};
use crate::model::{
    DistributionVariant, DshHome, EndpointOwnership, EndpointSnapshot, GlobalSettings,
    GlobalSettingsPatch, HostSnapshot, ServiceStatus, SystemColorScheme, WindowSnapshot,
};
use crate::process_supervisor::ProcessSupervisor;
use crate::runtime::RuntimeManager;
use crate::service::{ManagedLaunch, ManagedService, ResolvedDshRuntime};
use crate::settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostLifecycle {
    Running,
    ShuttingDown,
}

#[derive(Clone)]
pub struct AppState {
    config_dir: PathBuf,
    settings: Arc<RwLock<GlobalSettings>>,
    host: Arc<Mutex<HostState>>,
    startup_lock: Arc<Mutex<()>>,
    runtime_manager: RuntimeManager,
    system_color_scheme: Arc<RwLock<Option<SystemColorScheme>>>,
    next_window_id: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
    process_supervisor: ProcessSupervisor,
}

struct HostState {
    windows: HashMap<String, WindowRecord>,
    endpoints: HashMap<String, EndpointRecord>,
    next_connection_order: u64,
}

struct WindowRecord {
    url: String,
    status: ServiceStatus,
}

struct EndpointRecord {
    status: ServiceStatus,
    process: Option<ManagedService>,
    runtime_version: Option<String>,
    last_error: Option<String>,
    last_successful_connection: Option<u64>,
    managed: bool,
    launch: Option<crate::service::ManagedLaunch>,
    logs: Vec<String>,
    idle_since: Option<Instant>,
}

pub(crate) struct BuiltInRuntimeUpdateTarget {
    url: String,
    launch: ManagedLaunch,
    had_running_process: bool,
    process: Option<ManagedService>,
}

impl BuiltInRuntimeUpdateTarget {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn had_running_process(&self) -> bool {
        self.had_running_process
    }

    pub fn launch_with_runtime(&self, runtime: ResolvedDshRuntime) -> ManagedLaunch {
        self.launch.clone().with_runtime(runtime)
    }

    pub fn stop(&mut self) {
        if let Some(process) = self.process.as_mut() {
            process.stop();
        }
    }
}

impl EndpointRecord {
    fn external(status: ServiceStatus) -> Self {
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

    fn host_state(&self) -> AppResult<MutexGuard<'_, HostState>> {
        self.host
            .lock()
            .map_err(|_| AppError::new("app.error.stateUnavailable"))
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

    pub fn runtime_manager(&self) -> &RuntimeManager {
        &self.runtime_manager
    }

    pub fn settings_snapshot(&self) -> AppResult<GlobalSettings> {
        self.settings
            .read()
            .map_err(|_| AppError::new("app.error.stateUnavailable"))
            .map(|settings| settings.clone())
    }

    pub fn startup_guard(&self) -> AppResult<MutexGuard<'_, ()>> {
        self.startup_lock
            .lock()
            .map_err(|_| AppError::new("app.error.stateUnavailable"))
    }

    pub fn saved_locale(&self) -> Option<crate::model::AppLocale> {
        self.settings
            .read()
            .ok()
            .and_then(|settings| settings.locale)
    }

    pub fn update_global_settings(&self, patch: GlobalSettingsPatch) -> AppResult<GlobalSettings> {
        self.ensure_running()?;
        let updated_settings = {
            let mut current = self
                .settings
                .write()
                .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
            let candidate = settings::apply_patch(&current, patch, DistributionVariant::current())?;
            settings::save(&self.config_dir, &candidate)?;
            *current = candidate.clone();
            candidate
        };
        let _ = self.reap_idle_services();
        Ok(updated_settings)
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

    pub fn window_snapshot(&self, label: &str) -> Option<WindowSnapshot> {
        self.host
            .lock()
            .ok()
            .and_then(|host| host.window_snapshot(label))
    }

    pub fn snapshot(&self) -> HostSnapshot {
        let host = self.host.lock().expect("host state poisoned");
        snapshot_locked(&host)
    }

    pub fn known_endpoint_urls(&self) -> AppResult<Vec<String>> {
        Ok(self.host_state()?.known_endpoint_urls())
    }

    pub fn assign_external_endpoint(
        &self,
        window_label: &str,
        url: &str,
        status: ServiceStatus,
        record_connection: bool,
    ) -> AppResult<WindowSnapshot> {
        let idle_timeout = self.managed_service_idle_timeout_seconds();
        let mut host = self.host_state()?;
        if !host.assign_window(window_label, url, status) {
            return Err(AppError::new("window.error.notFound"));
        }
        let endpoint = host
            .endpoints
            .entry(url.to_owned())
            .or_insert_with(|| EndpointRecord::external(status));
        endpoint.status = status;
        if record_connection {
            host.record_connection(url);
        }
        host.reap_idle_services(idle_timeout);
        host.window_snapshot(window_label)
            .ok_or_else(|| AppError::new("window.error.notFound"))
    }

    pub fn mark_window_startup_failed(&self, window_label: &str) -> AppResult<()> {
        let mut host = self.host_state()?;
        host.set_window_status(window_label, ServiceStatus::Failed)
            .then_some(())
            .ok_or_else(|| AppError::new("window.error.notFound"))
    }

    pub fn startup_snapshot(
        &self,
        window_label: &str,
    ) -> AppResult<(WindowSnapshot, HostSnapshot)> {
        let host = self.host_state()?;
        let window = host
            .window_snapshot(window_label)
            .ok_or_else(|| AppError::new("window.error.notFound"))?;
        Ok((window, snapshot_locked(&host)))
    }

    pub fn register_managed_service(
        &self,
        window_label: &str,
        url: String,
        managed: ManagedService,
    ) -> AppResult<()> {
        let runtime_version = managed.runtime_version.clone();
        let launch = managed.launch.clone();
        let idle_timeout = self.managed_service_idle_timeout_seconds();
        let mut host = self.host_state()?;
        if !host.assign_window(window_label, &url, ServiceStatus::Running) {
            return Err(AppError::new("window.error.notFound"));
        }
        host.endpoints.insert(
            url.clone(),
            EndpointRecord {
                status: ServiceStatus::Running,
                process: Some(managed),
                runtime_version: Some(runtime_version),
                last_error: None,
                last_successful_connection: None,
                managed: true,
                launch: Some(launch),
                logs: Vec::new(),
                idle_since: None,
            },
        );
        host.record_connection(&url);
        host.reap_idle_services(idle_timeout);
        Ok(())
    }

    pub fn begin_stop_managed(&self, url: &str) -> AppResult<ManagedService> {
        let mut host = self.host_state()?;
        let endpoint = host
            .endpoints
            .get_mut(url)
            .filter(|endpoint| endpoint.managed)
            .ok_or_else(|| AppError::new("service.error.notManaged").arg("url", url))?;
        let process = endpoint
            .process
            .take()
            .ok_or_else(|| AppError::new("service.error.notRunning").arg("url", url))?;
        endpoint.status = ServiceStatus::Stopping;
        host.set_endpoint_window_status(url, ServiceStatus::Stopping);
        Ok(process)
    }

    pub fn complete_stop_managed(
        &self,
        url: &str,
        process: &ManagedService,
    ) -> AppResult<HostSnapshot> {
        let mut host = self.host_state()?;
        if let Some(endpoint) = host.endpoints.get_mut(url) {
            endpoint.logs = process.log_lines();
            endpoint.status = ServiceStatus::Unreachable;
            endpoint.last_error = None;
        }
        host.set_endpoint_window_status(url, ServiceStatus::Unreachable);
        Ok(snapshot_locked(&host))
    }

    pub fn begin_restart_managed(
        &self,
        url: &str,
        dsh_home: DshHome,
    ) -> AppResult<(Option<ManagedService>, ManagedLaunch)> {
        let mut host = self.host_state()?;
        let endpoint = host
            .endpoints
            .get_mut(url)
            .filter(|endpoint| endpoint.managed)
            .ok_or_else(|| AppError::new("service.error.notManaged").arg("url", url))?;
        let launch = endpoint
            .launch
            .clone()
            .ok_or_else(|| AppError::new("service.error.restartUnavailable").arg("url", url))?
            .with_dsh_home(dsh_home);
        let process = endpoint.process.take();
        if let Some(process) = process.as_ref() {
            endpoint.logs = process.log_lines();
        }
        endpoint.status = ServiceStatus::Restarting;
        host.set_endpoint_window_status(url, ServiceStatus::Restarting);
        Ok((process, launch))
    }

    pub fn complete_restart_managed(
        &self,
        url: &str,
        process: ManagedService,
    ) -> AppResult<HostSnapshot> {
        let runtime_version = process.runtime_version.clone();
        let launch = process.launch.clone();
        let mut host = self.host_state()?;
        if let Some(endpoint) = host.endpoints.get_mut(url) {
            endpoint.process = Some(process);
            endpoint.launch = Some(launch);
            endpoint.runtime_version = Some(runtime_version);
            endpoint.status = ServiceStatus::Running;
            endpoint.last_error = None;
            endpoint.logs.clear();
            endpoint.idle_since = None;
        }
        host.set_endpoint_window_status(url, ServiceStatus::Running);
        host.record_connection(url);
        Ok(snapshot_locked(&host))
    }

    pub fn fail_restart_managed(&self, url: &str, error: &AppError) -> AppResult<()> {
        let mut host = self.host_state()?;
        if let Some(endpoint) = host.endpoints.get_mut(url) {
            endpoint.status = ServiceStatus::Failed;
            endpoint.last_error = Some(error.code.clone());
        }
        host.set_endpoint_window_status(url, ServiceStatus::Failed);
        Ok(())
    }

    pub(crate) fn begin_built_in_runtime_update(
        &self,
    ) -> AppResult<Vec<BuiltInRuntimeUpdateTarget>> {
        let mut host = self.host_state()?;
        let mut targets = Vec::new();
        let mut updating_urls = Vec::new();
        for (url, endpoint) in &mut host.endpoints {
            let Some(launch) = endpoint
                .launch
                .clone()
                .filter(|launch| launch.uses_built_in_runtime())
            else {
                continue;
            };
            let process = endpoint.process.take();
            let had_running_process = process.is_some();
            if had_running_process {
                endpoint.status = ServiceStatus::Updating;
                updating_urls.push(url.clone());
            }
            targets.push(BuiltInRuntimeUpdateTarget {
                url: url.clone(),
                launch,
                had_running_process,
                process,
            });
        }
        for url in updating_urls {
            host.set_endpoint_window_status(&url, ServiceStatus::Updating);
        }
        Ok(targets)
    }

    pub(crate) fn complete_built_in_runtime_update(
        &self,
        targets: &[BuiltInRuntimeUpdateTarget],
        replacement_runtime: &ResolvedDshRuntime,
        candidate: &str,
        started: Vec<(String, ManagedService, ManagedLaunch)>,
    ) -> AppResult<HostSnapshot> {
        let mut host = self.host_state()?;
        for target in targets {
            if let Some(endpoint) = host.endpoints.get_mut(target.url()) {
                endpoint.launch = Some(target.launch_with_runtime(replacement_runtime.clone()));
                endpoint.runtime_version = Some(candidate.to_owned());
                endpoint.last_error = None;
            }
        }
        for (url, process, launch) in started {
            if let Some(endpoint) = host.endpoints.get_mut(&url) {
                endpoint.process = Some(process);
                endpoint.launch = Some(launch);
                endpoint.status = ServiceStatus::Running;
                endpoint.idle_since = None;
            }
            host.set_endpoint_window_status(&url, ServiceStatus::Running);
            host.record_connection(&url);
        }
        Ok(snapshot_locked(&host))
    }

    pub(crate) fn complete_built_in_runtime_rollback(
        &self,
        targets: &[BuiltInRuntimeUpdateTarget],
        previous_runtime: &ResolvedDshRuntime,
        recovered: Vec<(String, ManagedService)>,
        update_error: &AppError,
    ) -> AppResult<()> {
        let mut host = self.host_state()?;
        for target in targets {
            if let Some(endpoint) = host.endpoints.get_mut(target.url()) {
                endpoint.launch = Some(target.launch.clone());
                endpoint.runtime_version = Some(previous_runtime.version().to_owned());
                endpoint.last_error = Some(update_error.code.clone());
                endpoint.status = if target.had_running_process() {
                    ServiceStatus::Running
                } else {
                    ServiceStatus::Unreachable
                };
            }
        }
        for (url, process) in recovered {
            if let Some(endpoint) = host.endpoints.get_mut(&url) {
                endpoint.process = Some(process);
            }
            host.set_endpoint_window_status(&url, ServiceStatus::Running);
            host.record_connection(&url);
        }
        Ok(())
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
        let Some(scheme) = crate::system_appearance::detect(self.process_supervisor()) else {
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
    fn has_managed_processes(&self) -> bool {
        self.endpoints
            .values()
            .any(|endpoint| endpoint.process.is_some())
    }

    fn known_endpoint_urls(&self) -> Vec<String> {
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

    fn record_connection(&mut self, url: &str) {
        let order = self.next_connection_order;
        self.next_connection_order = self.next_connection_order.saturating_add(1);
        if let Some(endpoint) = self.endpoints.get_mut(url) {
            endpoint.status = ServiceStatus::Running;
            endpoint.last_error = None;
            endpoint.last_successful_connection = Some(order);
            endpoint.idle_since = None;
        }
    }

    fn assign_window(&mut self, label: &str, url: &str, status: ServiceStatus) -> bool {
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

    fn set_window_status(&mut self, label: &str, status: ServiceStatus) -> bool {
        let Some(window) = self.windows.get_mut(label) else {
            return false;
        };
        window.status = status;
        true
    }

    fn set_endpoint_window_status(&mut self, url: &str, status: ServiceStatus) {
        for window in self.windows.values_mut().filter(|window| window.url == url) {
            window.status = status;
        }
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

    fn reap_idle_services(&mut self, timeout_seconds: u64) {
        reap_idle_services(self, timeout_seconds);
    }

    fn window_snapshot(&self, label: &str) -> Option<WindowSnapshot> {
        self.windows.get(label).map(|window| WindowSnapshot {
            label: label.to_owned(),
            url: window.url.clone(),
            status: window.status,
        })
    }
}

fn snapshot_locked(host: &HostState) -> HostSnapshot {
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
    fn assigning_an_external_endpoint_updates_its_window_and_snapshot_together() {
        let state = AppState::new(
            PathBuf::from("config"),
            GlobalSettings::default(),
            RuntimeManager::new(PathBuf::from("seed"), PathBuf::from("data")),
        );
        state.register_window("main");

        let window = state
            .assign_external_endpoint(
                "main",
                "http://127.0.0.1:3080",
                ServiceStatus::Running,
                true,
            )
            .expect("registered window accepts an endpoint");

        assert_eq!(window.status, ServiceStatus::Running);
        let snapshot = state.snapshot();
        assert_eq!(snapshot.windows.len(), 1);
        assert_eq!(snapshot.windows[0].label, window.label);
        assert_eq!(snapshot.windows[0].url, window.url);
        assert_eq!(snapshot.windows[0].status, window.status);
        assert_eq!(snapshot.endpoints.len(), 1);
        assert_eq!(snapshot.endpoints[0].status, ServiceStatus::Running);
        assert!(snapshot.endpoints[0].known);
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
