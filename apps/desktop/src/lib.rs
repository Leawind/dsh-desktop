mod commands;
mod direct_network;
mod endpoint;
mod error;
mod model;
mod service;
mod settings;
mod state;
mod windows;

use tauri::{Manager, RunEvent, WindowEvent};

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
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings = settings::load(&config_dir);
            let state = AppState::new(config_dir, settings);
            state.register_window("main");
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::initialize_window,
            commands::create_app_window,
            commands::get_host_snapshot,
            commands::set_window_target,
            commands::ensure_default_service,
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
