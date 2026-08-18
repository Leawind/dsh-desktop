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
mod webui;
mod window_registry;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use state::AppState;
use window_registry::WindowRegistry;

const CLIENT_DISCONNECT_TIMEOUT_MILLIS: u64 = 3_000;

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
        runtime::RuntimeManager::new(resources.runtime_seed_directory.clone(), data_dir);
    let settings = settings::load(&config_dir, model::DistributionVariant::current());
    let state = AppState::new(config_dir, settings, runtime_manager);
    let monitor_state = state.clone();
    std::thread::spawn(move || {
        while !monitor_state.monitor_stopped() {
            std::thread::sleep(Duration::from_secs(5));
            if !monitor_state.monitor_stopped() {
                monitor_state.refresh_endpoint_health();
                monitor_state.refresh_system_color_scheme();
            }
        }
    });

    let running = Arc::new(AtomicBool::new(true));
    let windows = WindowRegistry::default();
    let client_activity = Arc::new(Mutex::new(HashMap::new()));
    let bridge = bridge_server::start(
        state.clone(),
        resources.frontend_directory.clone(),
        windows.clone(),
        Arc::clone(&running),
        Arc::clone(&client_activity),
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
    while running.load(Ordering::Acquire) {
        match instance.requests.recv_timeout(Duration::from_millis(250)) {
            Ok(single_instance::HostRequest::OpenWindow) => {
                let label = state.next_window_label();
                create_window(
                    label,
                    &state,
                    &resources.icon,
                    &windows,
                    &client_activity,
                    &bridge,
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
        reap_closed_windows(&state, &windows, &client_activity);
        if windows.is_empty() {
            break;
        }
    }
    state.shutdown();
    webui::clean();
}

fn create_window(
    label: String,
    state: &AppState,
    icon: &std::path::Path,
    windows: &WindowRegistry,
    client_activity: &Mutex<HashMap<String, u64>>,
    bridge: &bridge_server::BridgeServer,
) {
    let window = webui::Window::create(icon);
    let url =
        development_frontend_url(&bridge.token, &label).unwrap_or_else(|| bridge.url_for(&label));
    state.register_window(&label);
    windows.insert(label.clone(), window);
    client_activity
        .lock()
        .expect("client activity state poisoned")
        .insert(label.clone(), unix_time_millis());
    if !window.show(&url) && !window.show_webview_fallback(&url) {
        eprintln!("WebUI could not launch an external browser or the native WebView fallback");
        windows.remove(&label);
        state.remove_window(&label);
        client_activity
            .lock()
            .expect("client activity state poisoned")
            .remove(&label);
        return;
    }
}

fn reap_closed_windows(
    state: &AppState,
    windows: &WindowRegistry,
    client_activity: &Mutex<HashMap<String, u64>>,
) {
    let now = unix_time_millis();
    let stale = client_activity
        .lock()
        .expect("client activity state poisoned")
        .iter()
        .filter_map(|(label, last)| {
            (now.saturating_sub(*last) >= CLIENT_DISCONNECT_TIMEOUT_MILLIS).then_some(label.clone())
        })
        .collect::<Vec<_>>();
    for label in stale {
        client_activity
            .lock()
            .expect("client activity state poisoned")
            .remove(&label);
        if let Some(window) = windows.remove(&label) {
            window.close();
            state.remove_window(&label);
        }
    }
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
    url.query_pairs_mut()
        .append_pair("token", token)
        .append_pair("window", label);
    Some(url.into())
}
