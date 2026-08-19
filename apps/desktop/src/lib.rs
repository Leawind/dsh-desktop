mod app_update;
mod bridge_server;
mod commands;
mod compatibility;
mod embedded_resources;
mod endpoint;
mod error;
mod model;
mod runtime;
mod service;
mod settings;
mod single_instance;
mod state;
mod system_appearance;
mod tray;
mod webui;
mod window_control;
mod window_registry;

use std::time::Duration;

use state::AppState;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray::TrayAction;
use window_control::WindowControlRegistry;
use window_registry::{WindowActivityRegistry, WindowRegistry};

const CLIENT_DISCONNECT_TIMEOUT_MILLIS: u64 = 1_000;
const INITIAL_WINDOW_CONNECTION_TIMEOUT_MILLIS: u64 = 10_000;
const HOST_MAINTENANCE_INTERVAL: Duration = Duration::from_millis(250);

enum HostEvent {
    Tray(tray_icon::menu::MenuEvent),
    Maintenance,
}

pub fn run() {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("dsh-desktop");
    let instance = match single_instance::claim(&config_dir) {
        Ok(single_instance::Claim::Primary(instance)) => instance,
        Ok(single_instance::Claim::Forwarded) => return,
        Err(error) => {
            eprintln!("failed to establish the DSH Desktop single instance: {error}");
            return;
        }
    };
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("dsh-desktop");
    let resources = embedded_resources::materialize(&data_dir)
        .expect("failed to materialize embedded application resources");
    let runtime_manager =
        runtime::RuntimeManager::new(resources.runtime_seed_directory.clone(), data_dir.clone());
    let settings = settings::load(&config_dir, model::DistributionVariant::current());
    let state = AppState::new(config_dir, data_dir.clone(), settings, runtime_manager);
    app_update::confirm_applied_update(&data_dir);
    let windows = WindowRegistry::default();
    let client_activity = WindowActivityRegistry::default();
    let window_controls = WindowControlRegistry::default();
    let bridge = bridge_server::start(
        state.clone(),
        resources.frontend_directory.clone(),
        windows.clone(),
        client_activity.clone(),
        window_controls.clone(),
    )
    .expect("failed to start the local DSH Desktop bridge");
    create_window(
        "main".to_owned(),
        &state,
        &resources.icon,
        &windows,
        &client_activity,
        &bridge,
    );
    let appearance_state = state.clone();
    std::thread::spawn(move || appearance_state.refresh_system_color_scheme());
    let event_loop = EventLoopBuilder::<HostEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(HostEvent::Tray(event));
    }));
    let maintenance_proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(HOST_MAINTENANCE_INTERVAL);
            if maintenance_proxy
                .send_event(HostEvent::Maintenance)
                .is_err()
            {
                break;
            }
        }
    });
    let tray = tray::Tray::create(&resources.icon, saved_locale(&state))
        .expect("failed to create the DSH Desktop tray icon");
    let mut last_locale = saved_locale(&state);
    let mut next_monitor_refresh = std::time::Instant::now() + Duration::from_secs(5);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(HostEvent::Tray(event)) => match tray::menu_action(&event) {
                Some(TrayAction::OpenNewWindow) => {
                    create_new_window(&state, &resources.icon, &windows, &client_activity, &bridge)
                }
                Some(TrayAction::Quit) => {
                    bridge.close_windows();
                    state.shutdown();
                    *control_flow = ControlFlow::Exit;
                }
                None => {}
            },
            Event::UserEvent(HostEvent::Maintenance) => {}
            Event::MainEventsCleared => {
                if state.app_update_shutdown_requested() {
                    bridge.close_windows();
                    state.shutdown();
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                while let Ok(single_instance::HostRequest::OpenWindow) =
                    instance.requests.try_recv()
                {
                    create_new_window(&state, &resources.icon, &windows, &client_activity, &bridge);
                }
                reap_closed_windows(&state, &windows, &client_activity, &window_controls);
                if std::time::Instant::now() >= next_monitor_refresh {
                    state.refresh_endpoint_health();
                    state.refresh_system_color_scheme();
                    next_monitor_refresh = std::time::Instant::now() + Duration::from_secs(5);
                }
                let locale = saved_locale(&state);
                if locale != last_locale {
                    tray.update_locale(locale);
                    last_locale = locale;
                }
                if state.should_exit_after_idle_reap() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });
}

pub fn run_update_helper_if_requested() -> bool {
    app_update::run_helper_if_requested()
}

fn create_new_window(
    state: &AppState,
    icon: &std::path::Path,
    windows: &WindowRegistry,
    client_activity: &WindowActivityRegistry,
    bridge: &bridge_server::BridgeServer,
) {
    create_window(
        state.next_window_label(),
        state,
        icon,
        windows,
        client_activity,
        bridge,
    );
}

fn create_window(
    label: String,
    state: &AppState,
    icon: &std::path::Path,
    windows: &WindowRegistry,
    client_activity: &WindowActivityRegistry,
    bridge: &bridge_server::BridgeServer,
) {
    let window = webui::Window::create(icon);
    let url =
        development_frontend_url(&bridge.token, &label).unwrap_or_else(|| bridge.url_for(&label));
    state.register_window(&label);
    windows.insert(label.clone(), window);
    client_activity.insert(label.clone(), unix_time_millis());
    if !window.show(&url) && !window.show_webview_fallback(&url) {
        eprintln!("WebUI could not launch an external browser or the native WebView fallback");
        windows.remove(&label);
        state.remove_window(&label);
        client_activity.remove(&label);
        return;
    }
}

fn reap_closed_windows(
    state: &AppState,
    windows: &WindowRegistry,
    client_activity: &WindowActivityRegistry,
    window_controls: &WindowControlRegistry,
) {
    let now = unix_time_millis();
    let stale = client_activity.stale_labels(
        now,
        CLIENT_DISCONNECT_TIMEOUT_MILLIS,
        INITIAL_WINDOW_CONNECTION_TIMEOUT_MILLIS,
    );
    for label in stale {
        client_activity.remove(&label);
        window_controls.remove(&label);
        if let Some(window) = windows.remove(&label) {
            window.close();
            state.remove_window(&label);
        }
    }
}

fn saved_locale(state: &AppState) -> Option<model::AppLocale> {
    state
        .settings
        .read()
        .ok()
        .and_then(|settings| settings.locale)
}

fn unix_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn development_frontend_url(token: &str, label: &str) -> Option<String> {
    let mut url = url::Url::parse(&std::env::var("DSH_DESKTOP_FRONTEND_URL").ok()?).ok()?;
    if url.scheme() != "http" || !matches!(url.host_str(), Some("127.0.0.1" | "localhost")) {
        return None;
    }
    url.set_path(&format!("/session/{label}/{token}/"));
    url.set_query(None);
    Some(url.into())
}
