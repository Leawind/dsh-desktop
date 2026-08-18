mod bridge_server;
mod commands;
mod compatibility;
mod direct_network;
mod endpoint;
mod error;
mod model;
mod runtime;
mod service;
mod settings;
mod state;
mod system_appearance;
mod webui;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use state::AppState;

const CLIENT_DISCONNECT_TIMEOUT_MILLIS: u64 = 3_000;

pub fn run() {
    direct_network::configure_process();
    let config_dir = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("dsh-desktop");
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("dsh-desktop");
    let resource_dir = resource_directory();
    let runtime_manager = runtime::RuntimeManager::new(resource_dir, data_dir);
    let settings = settings::load(&config_dir, model::DistributionVariant::current());
    let state = AppState::new(config_dir, settings, runtime_manager);
    state.register_window("main");
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

    let icon = resource_directory().join("icons/icon.png");
    let window = webui::Window::create(&icon);
    let frontend = frontend_directory();
    let running = Arc::new(AtomicBool::new(true));
    let last_client_activity = Arc::new(AtomicU64::new(0));
    let url = bridge_server::start(
        state.clone(),
        window,
        frontend,
        Arc::clone(&running),
        Arc::clone(&last_client_activity),
    )
    .expect("failed to start the local DSH Desktop bridge");
    if !window.show(&url) && !window.show_webview_fallback(&url) {
        eprintln!("WebUI could not launch an external browser or the native WebView fallback");
        state.shutdown();
        webui::clean();
        return;
    }
    while running.load(Ordering::Acquire) {
        let last_activity = last_client_activity.load(Ordering::Acquire);
        if last_activity != 0
            && unix_time_millis().saturating_sub(last_activity) >= CLIENT_DISCONNECT_TIMEOUT_MILLIS
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    state.shutdown();
    webui::clean();
}

fn unix_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn resource_directory() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn frontend_directory() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("DSH_DESKTOP_FRONTEND_DIST") {
        return path.into();
    }
    let packaged = resource_directory().join("frontend");
    if packaged.is_dir() {
        return packaged;
    }
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/dist")
}
