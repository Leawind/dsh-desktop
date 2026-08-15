use tauri::{AppHandle, Emitter, State, WebviewWindow};

use crate::endpoint::{dsh_url, normalize_dsh_url};
use crate::error::{AppError, AppResult};
use crate::model::{
    BootstrapPayload, GlobalSettings, GlobalSettingsPatch, HostSnapshot, LOCAL_DSH_HOST,
    ServiceStatus, StartupAttemptFailure, WindowSnapshot, WindowStartupAttempt,
    WindowStartupResult,
};
use crate::service::{self, ProbeResult};
use crate::settings;
use crate::state::{AppState, EndpointRecord, snapshot_locked};

#[tauri::command]
pub fn initialize_window(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<BootstrapPayload> {
    let window = state.register_window(window.label());
    let settings = state
        .settings
        .read()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))?
        .clone();
    let host = state.snapshot();
    Ok(BootstrapPayload {
        settings,
        window,
        host,
    })
}

#[tauri::command]
pub fn create_app_window(app: AppHandle) -> AppResult<String> {
    crate::windows::create(&app)
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
    match probe {
        ProbeResult::Dsh => {
            host.record_connection(url);
            Ok(())
        }
        ProbeResult::Unreachable => Err(AppError::new("service.error.unreachable").arg("url", url)),
        ProbeResult::OtherHttp => Err(AppError::new("service.error.notDsh").arg("url", url)),
    }
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
    let runtime = resolve_runtime_once(resolved_runtime, settings)?;
    let managed = service::start(&runtime, port)?;
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
    let runtime = resolve_runtime_once(resolved_runtime, settings)?;
    for port in start_port..=end_port {
        let url = dsh_url(host, port);
        if service::probe(&url) != ProbeResult::Unreachable {
            continue;
        }
        let managed = service::start(&runtime, port)?;
        return register_managed_service(state, window_label, url, managed);
    }
    Err(AppError::new("service.error.noFreePort")
        .arg("startPort", start_port)
        .arg("endPort", end_port))
}

fn resolve_runtime_once<'a>(
    resolved_runtime: &'a mut Option<AppResult<service::ResolvedDshRuntime>>,
    settings: &GlobalSettings,
) -> AppResult<&'a service::ResolvedDshRuntime> {
    if resolved_runtime.is_none() {
        *resolved_runtime = Some(service::resolve_runtime(&settings.dsh_source));
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
        },
    );
    host.record_connection(&url);
    Ok(())
}

#[tauri::command]
pub fn update_global_settings(
    app: AppHandle,
    patch: GlobalSettingsPatch,
    state: State<'_, AppState>,
) -> AppResult<GlobalSettings> {
    let settings = settings::validate(patch)?;
    settings::save(&state.config_dir, &settings)?;
    *state
        .settings
        .write()
        .map_err(|_| AppError::new("app.error.stateUnavailable"))? = settings.clone();
    let _ = app.emit("global-settings-changed", &settings);
    Ok(settings)
}
