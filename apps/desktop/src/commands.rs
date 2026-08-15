use tauri::{AppHandle, Emitter, State, WebviewWindow};

use crate::endpoint::{default_dsh_url, normalize_dsh_url};
use crate::error::{AppError, AppResult};
use crate::model::{
    BootstrapPayload, GlobalSettings, GlobalSettingsPatch, HostSnapshot, ServiceStatus,
    WindowSnapshot,
};
use crate::service::{self, ProbeResult};
use crate::settings;
use crate::state::{AppState, EndpointRecord};

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
    let record = host
        .windows
        .get_mut(window.label())
        .ok_or_else(|| AppError::new("window.error.notFound"))?;
    record.url.clone_from(&url);
    record.status = status;
    if probe == ProbeResult::Dsh {
        host.endpoints.entry(url.clone()).or_insert(EndpointRecord {
            status: ServiceStatus::Running,
            process: None,
            runtime_version: None,
            last_error: None,
        });
    }
    Ok(WindowSnapshot {
        label: window.label().to_owned(),
        url,
        status,
    })
}

#[tauri::command]
pub async fn ensure_default_service(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<HostSnapshot> {
    let state = state.inner().clone();
    let window_label = window.label().to_owned();
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        let settings = state
            .settings
            .read()
            .map_err(|_| AppError::new("app.error.stateUnavailable"))?
            .clone();
        let url = default_dsh_url(settings.default_dsh_port);
        let mut host = state
            .host
            .lock()
            .map_err(|_| AppError::new("app.error.stateUnavailable"))?;

        let window = host
            .windows
            .get_mut(&window_label)
            .ok_or_else(|| AppError::new("window.error.notFound"))?;
        window.url.clone_from(&url);
        window.status = ServiceStatus::Starting;

        match service::probe(&url) {
            ProbeResult::Dsh => {
                window.status = ServiceStatus::Running;
                host.endpoints.entry(url.clone()).or_insert(EndpointRecord {
                    status: ServiceStatus::Running,
                    process: None,
                    runtime_version: None,
                    last_error: None,
                });
            }
            ProbeResult::OtherHttp => {
                window.status = ServiceStatus::Failed;
                return Err(AppError::new("service.error.portOccupied")
                    .arg("port", settings.default_dsh_port));
            }
            ProbeResult::Unreachable => {
                let managed = service::start(
                    settings.dsh_executable.as_deref(),
                    settings.default_dsh_port,
                )?;
                let runtime_version = managed.runtime_version.clone();
                window.status = ServiceStatus::Running;
                host.endpoints.insert(
                    url,
                    EndpointRecord {
                        status: ServiceStatus::Running,
                        process: Some(managed),
                        runtime_version: Some(runtime_version),
                        last_error: None,
                    },
                );
            }
        }

        Ok(crate::state::snapshot_locked(&host))
    })
    .await
    .map_err(|error| AppError::new("app.error.taskFailed").technical(error.to_string()))??;

    let _ = app.emit("host-snapshot-changed", &snapshot);
    Ok(snapshot)
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
