use crate::endpoint::{dsh_url, normalize_dsh_url};
use crate::error::{AppError, AppResult};
use crate::model::{
    AppMetadataSnapshot, BootstrapPayload, GlobalSettings, GlobalSettingsPatch, HostSnapshot,
    LOCAL_DSH_HOST, RuntimeUpdateResult, RuntimeUpdateSnapshot, ServiceStatus,
    StartupAttemptFailure, WindowSnapshot, WindowStartupAttempt, WindowStartupResult,
};
use crate::service::{self, ProbeResult};
use crate::state::AppState;

pub fn initialize_window(state: &AppState, label: &str) -> AppResult<BootstrapPayload> {
    state.ensure_running()?;
    state.register_window(label);
    get_desktop_snapshot(state, label)
}

pub fn get_desktop_snapshot(state: &AppState, label: &str) -> AppResult<BootstrapPayload> {
    state.ensure_running()?;
    let app_metadata = AppMetadataSnapshot {
        name: "DSH Desktop".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        identifier: "io.github.leawind.dsh-desktop".to_owned(),
    };
    let window = state
        .window_snapshot(label)
        .ok_or_else(|| AppError::new("window.error.notFound"))?;
    let settings = state.settings_snapshot()?;
    let host = state.snapshot();
    Ok(BootstrapPayload {
        app: app_metadata,
        settings,
        distribution: state.runtime_manager().distribution_snapshot(),
        window,
        host,
        system_color_scheme: state.system_color_scheme(),
    })
}

pub fn set_window_target(
    window_label: &str,
    url: String,
    state: &AppState,
) -> AppResult<WindowSnapshot> {
    state.ensure_running()?;
    let url = normalize_dsh_url(&url)?;
    let probe = service::probe(&url);
    let status = match probe {
        ProbeResult::Dsh => ServiceStatus::Running,
        ProbeResult::Unreachable | ProbeResult::OtherHttp => ServiceStatus::Unreachable,
    };
    state.assign_external_endpoint(window_label, &url, status, probe == ProbeResult::Dsh)
}

pub fn start_window(state: &AppState, window_label: &str) -> AppResult<WindowStartupResult> {
    state.ensure_running()?;
    run_window_startup(state, window_label)
}

fn run_window_startup(state: &AppState, window_label: &str) -> AppResult<WindowStartupResult> {
    let settings = state.settings_snapshot()?;
    let _startup_guard = state.startup_guard()?;
    state.ensure_running()?;

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

    state.mark_window_startup_failed(window_label)?;
    startup_result(state, window_label, false, failures)
}

fn startup_result(
    state: &AppState,
    window_label: &str,
    connected: bool,
    failures: Vec<StartupAttemptFailure>,
) -> AppResult<WindowStartupResult> {
    let (window, host) = state.startup_snapshot(window_label)?;
    Ok(WindowStartupResult {
        connected,
        distribution: state.runtime_manager().distribution_snapshot(),
        window,
        host,
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
    let urls = state.known_endpoint_urls()?;
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
    let status = if probe == ProbeResult::Dsh {
        ServiceStatus::Running
    } else {
        ServiceStatus::Unreachable
    };
    state.assign_external_endpoint(window_label, url, status, probe == ProbeResult::Dsh)?;
    let result = match probe {
        ProbeResult::Dsh => Ok(()),
        ProbeResult::Unreachable => Err(AppError::new("service.error.unreachable").arg("url", url)),
        ProbeResult::OtherHttp => Err(AppError::new("service.error.notDsh").arg("url", url)),
    };
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
    let runtime = resolve_runtime_once(resolved_runtime, settings, state)?;
    let managed = service::start(
        &runtime,
        port,
        settings.dsh_home.clone(),
        state.process_supervisor(),
        state.running_signal(),
    )?;
    state.register_managed_service(window_label, url, managed)
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
    let runtime = resolve_runtime_once(resolved_runtime, settings, state)?;
    for port in start_port..=end_port {
        let url = dsh_url(host, port);
        if service::probe(&url) != ProbeResult::Unreachable {
            continue;
        }
        let managed = service::start(
            &runtime,
            port,
            settings.dsh_home.clone(),
            state.process_supervisor(),
            state.running_signal(),
        )?;
        return state.register_managed_service(window_label, url, managed);
    }
    Err(AppError::new("service.error.noFreePort")
        .arg("startPort", start_port)
        .arg("endPort", end_port))
}

fn resolve_runtime_once<'a>(
    resolved_runtime: &'a mut Option<AppResult<service::ResolvedDshRuntime>>,
    settings: &GlobalSettings,
    state: &AppState,
) -> AppResult<&'a service::ResolvedDshRuntime> {
    if resolved_runtime.is_none() {
        *resolved_runtime = Some(service::resolve_runtime(
            &settings.dsh_source,
            state.runtime_manager(),
            state.process_supervisor(),
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

pub fn stop_service(state: &AppState, url: String) -> AppResult<HostSnapshot> {
    state.ensure_running()?;
    let url = normalize_dsh_url(&url)?;
    stop_managed(state, &url)
}

pub fn restart_service(state: &AppState, url: String) -> AppResult<HostSnapshot> {
    state.ensure_running()?;
    let url = normalize_dsh_url(&url)?;
    restart_managed(state, &url)
}

pub fn check_built_in_runtime_update(state: &AppState) -> AppResult<RuntimeUpdateSnapshot> {
    state.ensure_running()?;
    available_built_in_update(state)
}

pub fn update_built_in_runtime(state: &AppState) -> AppResult<RuntimeUpdateResult> {
    state.ensure_running()?;
    apply_built_in_runtime_update(state)
}

fn available_built_in_update(state: &AppState) -> AppResult<RuntimeUpdateSnapshot> {
    let current = state.runtime_manager().resolve_built_in()?;
    let compatibility = crate::compatibility::load_for_app(
        &state.runtime_manager().compatibility_cache_directory(),
        env!("CARGO_PKG_VERSION"),
    )?;
    let update = crate::compatibility::select_update(&compatibility, &current.dsh_version)?;
    Ok(RuntimeUpdateSnapshot {
        candidate_version: update,
    })
}

fn apply_built_in_runtime_update(state: &AppState) -> AppResult<RuntimeUpdateResult> {
    let _startup_guard = state.startup_guard()?;
    state.ensure_running()?;
    let update = available_built_in_update(state)?;
    let candidate = update
        .candidate_version
        .ok_or_else(|| AppError::new("runtime.error.alreadyUpToDate"))?;
    let package = crate::compatibility::fetch_dsh_package(&candidate)?;
    crate::compatibility::verify_package_integrity(&package)?;
    let current = state.runtime_manager().resolve_built_in()?;
    let prepared = state.runtime_manager().prepare_update(
        &candidate,
        &package.dist.integrity,
        state.process_supervisor(),
    )?;
    if let Err(error) = state.ensure_running() {
        state.runtime_manager().discard_prepared(prepared);
        return Err(error);
    }
    let replacement_runtime = match service::resolve_built_in_runtime(
        prepared.runtime.clone(),
        state.process_supervisor(),
    ) {
        Ok(runtime) => runtime,
        Err(error) => {
            state.runtime_manager().discard_prepared(prepared);
            return Err(error);
        }
    };
    if let Err(error) =
        verify_staged_runtime(state, &replacement_runtime, prepared.verification_home())
    {
        state.runtime_manager().discard_prepared(prepared);
        return Err(error);
    }
    if let Err(error) = state.ensure_running() {
        state.runtime_manager().discard_prepared(prepared);
        return Err(error);
    }

    let mut targets = state.begin_built_in_runtime_update()?;
    for target in targets
        .iter_mut()
        .filter(|target| target.had_running_process())
    {
        target.stop();
    }
    if !state.is_running() {
        return Err(AppError::new("app.error.hostShuttingDown"));
    }

    let new_runtime_id = match state.runtime_manager().commit_prepared(prepared) {
        Ok(runtime_id) => runtime_id,
        Err(error) => return rollback_built_in_update(state, current, &targets, error),
    };
    let mut started = Vec::new();
    for target in &targets {
        if !target.had_running_process() {
            continue;
        }
        let launch = target.launch_with_runtime(replacement_runtime.clone());
        match service::start_launch(&launch, state.process_supervisor(), state.running_signal()) {
            Ok(process) => started.push((target.url().to_owned(), process, launch)),
            Err(error) => {
                for (_, mut process, _) in started {
                    process.stop();
                }
                if !state.is_running() {
                    return Err(error);
                }
                return rollback_built_in_update(state, current, &targets, error);
            }
        }
    }

    let updated_urls = targets
        .iter()
        .filter(|target| target.had_running_process())
        .map(|target| target.url().to_owned())
        .collect::<Vec<_>>();
    let host_snapshot = state.complete_built_in_runtime_update(
        &targets,
        &replacement_runtime,
        &candidate,
        started,
    )?;
    state
        .runtime_manager()
        .cleanup(&[new_runtime_id, current.runtime_id])?;
    Ok(RuntimeUpdateResult {
        distribution: state.runtime_manager().distribution_snapshot(),
        host: host_snapshot,
        updated_urls,
    })
}

fn rollback_built_in_update(
    state: &AppState,
    previous: crate::runtime::InstalledRuntime,
    targets: &[crate::state::BuiltInRuntimeUpdateTarget],
    update_error: AppError,
) -> AppResult<RuntimeUpdateResult> {
    state.ensure_running()?;
    state
        .runtime_manager()
        .set_active_runtime(&previous.runtime_id)?;
    let previous_runtime = service::resolve_built_in_runtime(previous, state.process_supervisor())?;
    let mut recovered = Vec::new();
    for target in targets {
        if target.had_running_process() {
            match service::start_launch(
                &target.launch_with_runtime(previous_runtime.clone()),
                state.process_supervisor(),
                state.running_signal(),
            ) {
                Ok(process) => recovered.push((target.url().to_owned(), process)),
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
    state.complete_built_in_runtime_rollback(
        targets,
        &previous_runtime,
        recovered,
        &update_error,
    )?;
    Err(AppError::new("runtime.error.updateRolledBack").technical(update_error.to_string()))
}

fn verify_staged_runtime(
    state: &AppState,
    runtime: &service::ResolvedDshRuntime,
    verification_home: std::path::PathBuf,
) -> AppResult<()> {
    service::verify_web_help(runtime, state.process_supervisor())?;
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
        state.process_supervisor(),
        state.running_signal(),
    )?;
    process.stop();
    let _ = std::fs::remove_dir_all(verification_home);
    Ok(())
}

fn stop_managed(state: &AppState, url: &str) -> AppResult<HostSnapshot> {
    let _startup_guard = state.startup_guard()?;
    state.ensure_running()?;
    let mut process = state.begin_stop_managed(url)?;
    process.stop();
    state.complete_stop_managed(url, &process)
}

fn restart_managed(state: &AppState, url: &str) -> AppResult<HostSnapshot> {
    let _startup_guard = state.startup_guard()?;
    state.ensure_running()?;
    let dsh_home = state.settings_snapshot()?.dsh_home;
    let (mut process, launch) = state.begin_restart_managed(url, dsh_home)?;
    if let Some(process) = process.as_mut() {
        process.stop();
    }

    match service::start_launch(&launch, state.process_supervisor(), state.running_signal()) {
        Ok(process) => state.complete_restart_managed(url, process),
        Err(error) => {
            let _ = state.fail_restart_managed(url, &error);
            Err(error)
        }
    }
}

pub fn update_global_settings(
    patch: GlobalSettingsPatch,
    state: &AppState,
) -> AppResult<GlobalSettings> {
    state.update_global_settings(patch)
}
