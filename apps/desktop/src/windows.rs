use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn create(app: &AppHandle) -> AppResult<String> {
    let state = app.state::<AppState>();
    let label = state.next_window_label();
    let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("Deepseek Harness Desktop")
        .inner_size(1180.0, 780.0)
        .min_inner_size(720.0, 520.0)
        .resizable(true);
    #[cfg(target_os = "macos")]
    let builder = builder
        .decorations(true)
        .hidden_title(true)
        .title_bar_style(TitleBarStyle::Overlay);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.decorations(false);
    #[cfg(target_os = "windows")]
    let builder = builder.additional_browser_args(crate::direct_network::WINDOWS_BROWSER_ARGS);
    builder
        .build()
        .map_err(|error| AppError::new("window.error.createFailed").technical(error.to_string()))?;
    state.register_window(&label);
    Ok(label)
}
