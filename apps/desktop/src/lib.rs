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
mod webui;

use std::time::Duration;

use state::AppState;

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
            }
        }
    });

    let icon = resource_directory().join("icons/icon.png");
    let window = webui::Window::create(&icon);
    let frontend = frontend_directory();
    let url = bridge_server::start(state.clone(), window, frontend)
        .expect("failed to start the local DSH Desktop bridge");
    if !window.show(&url) && !window.show_webview_fallback(&url) {
        eprintln!("WebUI could not launch an external browser or the native WebView fallback");
        state.shutdown();
        webui::clean();
        return;
    }
    webui::wait();
    state.shutdown();
    webui::clean();
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
