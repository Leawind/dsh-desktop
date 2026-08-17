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
mod windows;

use std::time::Duration;

use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    direct_network::configure_process();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Err(error) = windows::create(app) {
                eprintln!("failed to create window for second launch: {error}");
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_local_data_dir()?;
            let resource_dir = app.path().resource_dir()?;
            let runtime_manager = runtime::RuntimeManager::new(resource_dir, data_dir.clone());
            let settings = settings::load(&config_dir, model::DistributionVariant::current());
            let state = AppState::new(config_dir, settings, runtime_manager);
            state.register_window("main");
            app.manage(state.clone());
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                while !state.monitor_stopped() {
                    std::thread::sleep(Duration::from_secs(5));
                    if state.monitor_stopped() {
                        break;
                    }
                    let snapshot = state.refresh_endpoint_health();
                    let _ = app_handle.emit("host-snapshot-changed", snapshot);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::initialize_window,
            commands::focus_app_window,
            commands::close_app_window,
            commands::get_host_snapshot,
            commands::set_window_target,
            commands::start_window,
            commands::stop_service,
            commands::restart_service,
            commands::check_built_in_runtime_update,
            commands::update_built_in_runtime,
            commands::restart_app,
            commands::update_global_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building DSH Desktop");

    app.run(|app, event| match event {
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::Destroyed,
            ..
        } => app.state::<AppState>().remove_window(&label),
        RunEvent::Exit => app.state::<AppState>().shutdown(),
        _ => {}
    });
}
