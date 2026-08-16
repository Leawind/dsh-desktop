use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

use crate::endpoint::{dsh_url, normalize_dsh_url};
use crate::error::{AppError, AppResult};
use crate::model::{
    AppMetadataSnapshot, BootstrapPayload, GlobalSettings, GlobalSettingsPatch, HostSnapshot,
    LOCAL_DSH_HOST, RuntimeUpdateResult, RuntimeUpdateSnapshot, ServiceStatus,
    StartupAttemptFailure, WindowSnapshot, WindowStartupAttempt, WindowStartupResult,
};
use crate::service::{self, ProbeResult};
use crate::settings;
use crate::state::{AppState, EndpointRecord, snapshot_locked};

#[tauri::command]
pub fn initialize_window(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<BootstrapPayload> {
    let package_info = window.app_handle().package_info();
    let app_metadata = AppMetadataSnapshot {
        name: package_info.name.clone(),
        version: package_info.version.to_string(),
        identifier: window.app_handle().config().identifier.clone(),
    };
    let window = state.register_window(window.label());
    let settings = state
        .settings
        .read()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?
        .clone();
    let host = state.snapshot();
    Ok(BootstrapPayload {
        app: app_metadata,
        settings,
        distribution: state.runtime_manager.distribution_snapshot(),
        window,
        host,
    })
}

#[tauri::command]
pub fn focus_app_window(app: AppHandle, label: String) -> AppResult<()> {
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| AppError::new("window.error.notFound"))?;
    window
        .unminimize()
        .and_then(|_| window.show())
        .and_then(|_| window.set_focus())
        .map_err(|error| AppError::new("window.error.focusFailed").technical(error.to_string()))
}

#[tauri::command]
pub fn close_app_window(app: AppHandle, label: String) -> AppResult<()> {
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| AppError::new("window.error.notFound"))?;
    window
        .close()
        .map_err(|error| AppError::new("window.error.closeFailed").technical(error.to_string()))
}

#[tauri::command]
pub fn get_host_snapshot(state: State<'_, AppState>) -> HostSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn set_window_target(
    app: AppHandle,
    window: WebviewWindow,
    url: String,
    state: State<'_, AppState>,
) -> AppResult<WindowSnapshot> {
    let url = normalize_dsh_url(&url)?;
    let probe = service::probe(&url);
    let status = match probe {
        ProbeResult::Dsh => ServiceStatus::Running,
        ProbeResult::Unreachable | ProbeResult::OtherHttp => ServiceStatus::Unreachable,
    };
    let idle_timeout = state.managed_service_idle_timeout_seconds();
    let mut host = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    if !host.assign_window(window.label(), &url, status) {
        return Err(AppError::new("window.error.notFound"));
    }
    let endpoint = host
        .endpoints
        .entry(url.clone())
        .or_insert_with(|| EndpointRecord::external(status));
    endpoint.status = status;
    if probe == ProbeResult::Dsh {
        host.record_connection(&url);
    }
    host.reap_idle_services(idle_timeout);
    let snapshot = host
        .window_snapshot(window.label())
        .ok_or_else(|| AppError::new("window.error.notFound"))?;
    let host_snapshot = snapshot_locked(&host);
    drop(host);
    let _ = app.emit("host-snapshot-changed", host_snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn start_window(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<WindowStartupResult> {
    let state = state.inner().clone();
    let window_label = window.label().to_owned();
    let result =
        tauri::async_runtime::spawn_blocking(move || run_window_startup(&state, &window_label))
            .await
            .map_err(|error| {
                AppError::new("app.error.taskFailed").technical(error.to_string())
            })??;

    let _ = app.emit("host-snapshot-changed", &result.host);
    Ok(result)
}

fn run_window_startup(state: &AppState, window_label: &str) -> AppResult<WindowStartupResult> {
    let settings = state
        .settings
        .read()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?
        .clone();
    let _startup_guard = state
        .startup_lock
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;

    let mut failures = Vec::new();
    let mut resolved_runtime = None;
    for attempt in &settings.window_startup_attempts {
        match run_attempt(
            state,
            window_label,
            &settings,
            attempt,
            &mut resolved_runtime,
        ) {
            Ok(()) => return startup_result(state, window_label, true, failures),
            Err(error) => failures.push(StartupAttemptFailure {
                attempt: attempt.clone(),
                error,
            }),
        }
    }

    {
        let mut host = state
            .host
            .lock()
            .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
        let current_url = host
            .windows
            .get(window_label)
            .map(|window| window.url.clone())
            .ok_or_else(|| AppError::new("window.error.notFound"))?;
        host.assign_window(window_label, &current_url, ServiceStatus::Failed);
    }
    startup_result(state, window_label, false, failures)
}

fn startup_result(
    state: &AppState,
    window_label: &str,
    connected: bool,
    failures: Vec<StartupAttemptFailure>,
) -> AppResult<WindowStartupResult> {
    let host = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    let window = host
        .window_snapshot(window_label)
        .ok_or_else(|| AppError::new("window.error.notFound"))?;
    Ok(WindowStartupResult {
        connected,
        distribution: state.runtime_manager.distribution_snapshot(),
        window,
        host: snapshot_locked(&host),
        failures,
    })
}

fn run_attempt(
    state: &AppState,
    window_label: &str,
    settings: &GlobalSettings,
    attempt: &WindowStartupAttempt,
    resolved_runtime: &mut Option<AppResult<service::ResolvedDshRuntime>>,
) -> AppResult<()> {
    match attempt {
        WindowStartupAttempt::KnownServices => connect_known_service(state, window_label),
        WindowStartupAttempt::ConnectFixed { host, port } => {
            connect_endpoint(state, window_label, &dsh_url(host, *port))
        }
        WindowStartupAttempt::StartFixed { host, port } => {
            start_fixed(state, window_label, settings, host, *port, resolved_runtime)
        }
        WindowStartupAttempt::StartRange {
            host,
            start_port,
            end_port,
        } => start_range(
            state,
            window_label,
            settings,
            host,
            *start_port,
            *end_port,
            resolved_runtime,
        ),
    }
}

fn connect_known_service(state: &AppState, window_label: &str) -> AppResult<()> {
    let urls = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?
        .known_endpoint_urls();
    if urls.is_empty() {
        return Err(AppError::new("service.error.noKnownServices"));
    }

    let mut errors = Vec::new();
    for url in urls {
        match connect_endpoint(state, window_label, &url) {
            Ok(()) => return Ok(()),
            Err(error) => errors.push(format!("{url}: {}", error.code)),
        }
    }
    Err(AppError::new("service.error.knownServicesUnavailable").technical(errors.join("\n")))
}

fn connect_endpoint(state: &AppState, window_label: &str, url: &str) -> AppResult<()> {
    let probe = service::probe(url);
    let idle_timeout = state.managed_service_idle_timeout_seconds();
    let mut host = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    let status = if probe == ProbeResult::Dsh {
        ServiceStatus::Running
    } else {
        ServiceStatus::Unreachable
    };
    if !host.assign_window(window_label, url, status) {
        return Err(AppError::new("window.error.notFound"));
    }
    host.endpoints
        .entry(url.to_owned())
        .or_insert_with(|| EndpointRecord::external(status))
        .status = status;
    let result = match probe {
        ProbeResult::Dsh => {
            host.record_connection(url);
            Ok(())
        }
        ProbeResult::Unreachable => Err(AppError::new("service.error.unreachable").arg("url", url)),
        ProbeResult::OtherHttp => Err(AppError::new("service.error.notDsh").arg("url", url)),
    };
    host.reap_idle_services(idle_timeout);
    result
}

fn start_fixed(
    state: &AppState,
    window_label: &str,
    settings: &GlobalSettings,
    host: &str,
    port: u16,
    resolved_runtime: &mut Option<AppResult<service::ResolvedDshRuntime>>,
) -> AppResult<()> {
    validate_start_host(host)?;
    let url = dsh_url(host, port);
    match service::probe(&url) {
        ProbeResult::Dsh | ProbeResult::OtherHttp => {
            return Err(AppError::new("service.error.portOccupied").arg("port", port));
        }
        ProbeResult::Unreachable => {}
    }
    let runtime = resolve_runtime_once(resolved_runtime, settings, &state.runtime_manager)?;
    let managed = service::start(&runtime, port, settings.dsh_home.clone())?;
    register_managed_service(state, window_label, url, managed)
}

fn start_range(
    state: &AppState,
    window_label: &str,
    settings: &GlobalSettings,
    host: &str,
    start_port: u16,
    end_port: u16,
    resolved_runtime: &mut Option<AppResult<service::ResolvedDshRuntime>>,
) -> AppResult<()> {
    validate_start_host(host)?;
    let runtime = resolve_runtime_once(resolved_runtime, settings, &state.runtime_manager)?;
    for port in start_port..=end_port {
        let url = dsh_url(host, port);
        if service::probe(&url) != ProbeResult::Unreachable {
            continue;
        }
        let managed = service::start(&runtime, port, settings.dsh_home.clone())?;
        return register_managed_service(state, window_label, url, managed);
    }
    Err(AppError::new("service.error.noFreePort")
        .arg("startPort", start_port)
        .arg("endPort", end_port))
}

fn resolve_runtime_once<'a>(
    resolved_runtime: &'a mut Option<AppResult<service::ResolvedDshRuntime>>,
    settings: &GlobalSettings,
    runtime_manager: &crate::runtime::RuntimeManager,
) -> AppResult<&'a service::ResolvedDshRuntime> {
    if resolved_runtime.is_none() {
        *resolved_runtime = Some(service::resolve_runtime(
            &settings.dsh_source,
            runtime_manager,
        ));
    }
    resolved_runtime
        .as_ref()
        .expect("runtime result initialized")
        .as_ref()
        .map_err(Clone::clone)
}

fn validate_start_host(host: &str) -> AppResult<()> {
    if host == LOCAL_DSH_HOST {
        Ok(())
    } else {
        Err(AppError::new("service.error.unsupportedBindAddress").arg("host", host))
    }
}

fn register_managed_service(
    state: &AppState,
    window_label: &str,
    url: String,
    managed: service::ManagedService,
) -> AppResult<()> {
    let runtime_version = managed.runtime_version.clone();
    let launch = managed.launch.clone();
    let idle_timeout = state.managed_service_idle_timeout_seconds();
    let mut host = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
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

#[tauri::command]
pub async fn stop_service(
    app: AppHandle,
    url: String,
    state: State<'_, AppState>,
) -> AppResult<HostSnapshot> {
    let url = normalize_dsh_url(&url)?;
    let state = state.inner().clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || stop_managed(&state, &url))
        .await
        .map_err(|error| AppError::new("app.error.taskFailed").technical(error.to_string()))??;
    let _ = app.emit("host-snapshot-changed", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn restart_service(
    app: AppHandle,
    url: String,
    state: State<'_, AppState>,
) -> AppResult<HostSnapshot> {
    let url = normalize_dsh_url(&url)?;
    let state = state.inner().clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || restart_managed(&state, &url))
        .await
        .map_err(|error| AppError::new("app.error.taskFailed").technical(error.to_string()))??;
    let _ = app.emit("host-snapshot-changed", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn check_built_in_runtime_update(
    state: State<'_, AppState>,
) -> AppResult<RuntimeUpdateSnapshot> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || available_built_in_update(&state))
        .await
        .map_err(|error| AppError::new("app.error.taskFailed").technical(error.to_string()))?
}

#[tauri::command]
pub async fn update_built_in_runtime(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<RuntimeUpdateResult> {
    let state = state.inner().clone();
    let result =
        tauri::async_runtime::spawn_blocking(move || apply_built_in_runtime_update(&state))
            .await
            .map_err(|error| {
                AppError::new("app.error.taskFailed").technical(error.to_string())
            })??;
    let _ = app.emit("host-snapshot-changed", &result.host);
    let _ = app.emit("runtime-distribution-changed", &result.distribution);
    let _ = app.emit("built-in-runtime-updated", &result.updated_urls);
    Ok(result)
}

fn available_built_in_update(state: &AppState) -> AppResult<RuntimeUpdateSnapshot> {
    let settings = state
        .settings
        .read()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?
        .clone();
    if !matches!(settings.dsh_source, crate::model::DshSource::BuiltIn) {
        return Err(AppError::new("runtime.error.builtInSourceRequired"));
    }
    let current = state.runtime_manager.resolve_built_in()?;
    let compatibility = crate::compatibility::load_for_app(
        &state.runtime_manager.compatibility_cache_directory(),
        env!("CARGO_PKG_VERSION"),
    )?;
    let update = crate::compatibility::select_update(
        &compatibility,
        &current.dsh_version,
        &current.node_version,
    )?;
    Ok(RuntimeUpdateSnapshot {
        current_version: current.dsh_version,
        candidate_version: update.as_ref().map(|update| update.version.clone()),
        automatic_rollback_supported: update
            .as_ref()
            .is_some_and(|update| update.automatic_rollback_supported),
    })
}

struct UpdateTarget {
    url: String,
    launch: service::ManagedLaunch,
    had_running_process: bool,
    process: Option<service::ManagedService>,
}

fn apply_built_in_runtime_update(state: &AppState) -> AppResult<RuntimeUpdateResult> {
    let _startup_guard = state
        .startup_lock
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    let update = available_built_in_update(state)?;
    let candidate = update
        .candidate_version
        .ok_or_else(|| AppError::new("runtime.error.alreadyUpToDate"))?;
    if !update.automatic_rollback_supported {
        return Err(AppError::new("runtime.error.rollbackNotSupported").arg("version", candidate));
    }
    let package = crate::compatibility::fetch_dsh_package(&candidate)?;
    crate::compatibility::verify_package_integrity(&package)?;
    let current = state.runtime_manager.resolve_built_in()?;
    let prepared = state
        .runtime_manager
        .prepare_update(&candidate, &package.dist.integrity)?;
    let replacement_runtime = match service::resolve_built_in_runtime(prepared.runtime.clone()) {
        Ok(runtime) => runtime,
        Err(error) => {
            state.runtime_manager.discard_prepared(prepared);
            return Err(error);
        }
    };
    if let Err(error) = verify_staged_runtime(&replacement_runtime, prepared.verification_home()) {
        state.runtime_manager.discard_prepared(prepared);
        return Err(error);
    }

    let mut targets = take_built_in_targets(state)?;
    for target in targets
        .iter_mut()
        .filter(|target| target.had_running_process)
    {
        if let Some(process) = target.process.as_mut() {
            process.stop();
        }
    }

    let new_runtime_id = match state.runtime_manager.commit_prepared(prepared) {
        Ok(runtime_id) => runtime_id,
        Err(error) => return rollback_built_in_update(state, current, targets, error),
    };
    let mut started = Vec::new();
    for target in &targets {
        if !target.had_running_process {
            continue;
        }
        let launch = target
            .launch
            .clone()
            .with_runtime(replacement_runtime.clone());
        match service::start_launch(&launch) {
            Ok(process) => started.push((target.url.clone(), process, launch)),
            Err(error) => {
                for (_, mut process, _) in started {
                    process.stop();
                }
                return rollback_built_in_update(state, current, targets, error);
            }
        }
    }

    let mut host = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    for target in &targets {
        if let Some(endpoint) = host.endpoints.get_mut(&target.url) {
            endpoint.launch = Some(
                target
                    .launch
                    .clone()
                    .with_runtime(replacement_runtime.clone()),
            );
            endpoint.runtime_version = Some(candidate.clone());
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
        set_window_status(&mut host, &url, ServiceStatus::Running);
        host.record_connection(&url);
    }
    let updated_urls = targets
        .iter()
        .filter(|target| target.had_running_process)
        .map(|target| target.url.clone())
        .collect::<Vec<_>>();
    let host_snapshot = snapshot_locked(&host);
    drop(host);
    state
        .runtime_manager
        .cleanup(&[new_runtime_id, current.runtime_id])?;
    Ok(RuntimeUpdateResult {
        distribution: state.runtime_manager.distribution_snapshot(),
        host: host_snapshot,
        updated_urls,
    })
}

fn take_built_in_targets(state: &AppState) -> AppResult<Vec<UpdateTarget>> {
    let mut host = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
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
        targets.push(UpdateTarget {
            url: url.clone(),
            launch,
            had_running_process,
            process,
        });
    }
    for url in updating_urls {
        set_window_status(&mut host, &url, ServiceStatus::Updating);
    }
    Ok(targets)
}

fn rollback_built_in_update(
    state: &AppState,
    previous: crate::runtime::InstalledRuntime,
    targets: Vec<UpdateTarget>,
    update_error: AppError,
) -> AppResult<RuntimeUpdateResult> {
    state
        .runtime_manager
        .set_active_runtime(&previous.runtime_id)?;
    let previous_runtime = service::resolve_built_in_runtime(previous)?;
    let mut recovered = Vec::new();
    for target in &targets {
        if target.had_running_process {
            match service::start_launch(
                &target.launch.clone().with_runtime(previous_runtime.clone()),
            ) {
                Ok(process) => recovered.push((target.url.clone(), process)),
                Err(error) => {
                    for (_, mut process) in recovered {
                        process.stop();
                    }
                    return Err(AppError::new("runtime.error.rollbackFailed")
                        .technical(format!("update: {update_error}\nrollback: {error}")));
                }
            }
        }
    }
    let mut host = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    for target in targets {
        if let Some(endpoint) = host.endpoints.get_mut(&target.url) {
            endpoint.launch = Some(target.launch);
            endpoint.runtime_version = Some(previous_runtime.version().to_owned());
            endpoint.last_error = Some(update_error.code.clone());
            endpoint.status = if target.had_running_process {
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
        set_window_status(&mut host, &url, ServiceStatus::Running);
        host.record_connection(&url);
    }
    Err(AppError::new("runtime.error.updateRolledBack").technical(update_error.to_string()))
}

fn verify_staged_runtime(
    runtime: &service::ResolvedDshRuntime,
    verification_home: std::path::PathBuf,
) -> AppResult<()> {
    service::verify_web_help(runtime)?;
    let listener = std::net::TcpListener::bind((LOCAL_DSH_HOST, 0)).map_err(|error| {
        AppError::new("runtime.error.verificationFailed").technical(error.to_string())
    })?;
    let port = listener
        .local_addr()
        .map_err(|error| {
            AppError::new("runtime.error.verificationFailed").technical(error.to_string())
        })?
        .port();
    drop(listener);
    let mut process = service::start(
        runtime,
        port,
        crate::model::DshHome::Custom {
            path: verification_home.display().to_string(),
        },
    )?;
    process.stop();
    let _ = std::fs::remove_dir_all(verification_home);
    Ok(())
}

fn stop_managed(state: &AppState, url: &str) -> AppResult<HostSnapshot> {
    let _startup_guard = state
        .startup_lock
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    let mut process = {
        let mut host = state
            .host
            .lock()
            .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
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
        set_window_status(&mut host, url, ServiceStatus::Stopping);
        process
    };
    process.stop();
    let mut host = state
        .host
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    if let Some(endpoint) = host.endpoints.get_mut(url) {
        endpoint.logs = process.log_lines();
        endpoint.status = ServiceStatus::Unreachable;
        endpoint.last_error = None;
    }
    set_window_status(&mut host, url, ServiceStatus::Unreachable);
    Ok(snapshot_locked(&host))
}

fn restart_managed(state: &AppState, url: &str) -> AppResult<HostSnapshot> {
    let _startup_guard = state
        .startup_lock
        .lock()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
    let dsh_home = state
        .settings
        .read()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?
        .dsh_home
        .clone();
    let (mut process, launch) = {
        let mut host = state
            .host
            .lock()
            .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
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
        endpoint.status = ServiceStatus::Restarting;
        set_window_status(&mut host, url, ServiceStatus::Restarting);
        (process, launch)
    };
    if let Some(process) = process.as_mut() {
        if let Ok(mut host) = state.host.lock() {
            if let Some(endpoint) = host.endpoints.get_mut(url) {
                endpoint.logs = process.log_lines();
            }
        }
        process.stop();
    }

    match service::start_launch(&launch) {
        Ok(process) => {
            let runtime_version = process.runtime_version.clone();
            let launch = process.launch.clone();
            let mut host = state
                .host
                .lock()
                .map_err(|_| AppError::new("app.error.stateUnavailable"))?;
            if let Some(endpoint) = host.endpoints.get_mut(url) {
                endpoint.process = Some(process);
                endpoint.launch = Some(launch);
                endpoint.runtime_version = Some(runtime_version);
                endpoint.status = ServiceStatus::Running;
                endpoint.last_error = None;
                endpoint.logs.clear();
                endpoint.idle_since = None;
            }
            set_window_status(&mut host, url, ServiceStatus::Running);
            host.record_connection(url);
            Ok(snapshot_locked(&host))
        }
        Err(error) => {
            if let Ok(mut host) = state.host.lock() {
                if let Some(endpoint) = host.endpoints.get_mut(url) {
                    endpoint.status = ServiceStatus::Failed;
                    endpoint.last_error = Some(error.code.clone());
                }
                set_window_status(&mut host, url, ServiceStatus::Failed);
            }
            Err(error)
        }
    }
}

fn set_window_status(host: &mut crate::state::HostState, url: &str, status: ServiceStatus) {
    for window in host.windows.values_mut().filter(|window| window.url == url) {
        window.status = status;
    }
}

#[tauri::command]
pub fn update_global_settings(
    app: AppHandle,
    patch: GlobalSettingsPatch,
    state: State<'_, AppState>,
) -> AppResult<GlobalSettings> {
    let settings = settings::validate(patch, crate::model::DistributionVariant::current())?;
    settings::save(&state.config_dir, &settings)?;
    *state
        .settings
        .write()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))? = settings.clone();
    let _ = app.emit("global-settings-changed", &settings);
    if let Some(snapshot) = state.reap_idle_services() {
        let _ = app.emit("host-snapshot-changed", snapshot);
    }
    Ok(settings)
}
