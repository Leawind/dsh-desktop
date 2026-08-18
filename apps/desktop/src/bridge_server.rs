use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use serde::Deserialize;
use serde_json::{Value, json};
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::commands;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::webui::Window;

const WINDOW_LABEL: &str = "main";

#[derive(Deserialize)]
struct CommandRequest {
    name: String,
    #[serde(default)]
    args: Value,
}

pub fn start(state: AppState, window: Window, frontend: PathBuf) -> AppResult<String> {
    let server = Server::http("127.0.0.1:0").map_err(|error| {
        AppError::new("app.error.bridgeUnavailable").technical(error.to_string())
    })?;
    let address = server.server_addr();
    let token = format!(
        "{:032x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            ^ u128::from(std::process::id())
    );
    let state = Arc::new(state);
    let server_token = token.clone();
    thread::spawn(move || {
        for request in server.incoming_requests() {
            let state = Arc::clone(&state);
            let frontend = frontend.clone();
            let token = server_token.clone();
            thread::spawn(move || respond(request, state, window, &frontend, &token));
        }
    });
    Ok(format!("http://{address}/?token={token}"))
}

fn respond(
    mut request: tiny_http::Request,
    state: Arc<AppState>,
    window: Window,
    frontend: &Path,
    token: &str,
) {
    let response = if request.url().starts_with("/api/") {
        if !authorized(&request, token) {
            json_response(
                StatusCode(403),
                json!({ "error": AppError::new("app.error.unauthorized") }),
            )
        } else if request.method() != &Method::Post || request.url() != "/api/command" {
            json_response(
                StatusCode(404),
                json!({ "error": AppError::new("app.error.notFound") }),
            )
        } else {
            let mut body = String::new();
            let result = std::io::Read::read_to_string(&mut request.as_reader(), &mut body)
                .map_err(|error| {
                    AppError::new("app.error.invalidRequest").technical(error.to_string())
                })
                .and_then(|_| {
                    serde_json::from_str(&body).map_err(|error| {
                        AppError::new("app.error.invalidRequest").technical(error.to_string())
                    })
                })
                .and_then(|request: CommandRequest| dispatch(&state, window, request));
            match result {
                Ok(value) => json_response(StatusCode(200), json!({ "value": value })),
                Err(error) => json_response(StatusCode(400), json!({ "error": error })),
            }
        }
    } else {
        static_response(frontend, request.url())
    };
    let _ = request.respond(response);
}

fn authorized(request: &tiny_http::Request, token: &str) -> bool {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv("X-DSH-Desktop-Token"))
        .is_some_and(|header| header.value.as_str() == token)
}

fn dispatch(state: &AppState, window: Window, request: CommandRequest) -> AppResult<Value> {
    let args = request.args;
    match request.name.as_str() {
        "initialize_window" => value(commands::initialize_window(state, WINDOW_LABEL)?),
        "get_host_snapshot" => value(commands::get_host_snapshot(state)),
        "set_window_target" => value(commands::set_window_target(
            WINDOW_LABEL,
            string(&args, "url")?,
            state,
        )?),
        "start_window" => value(commands::start_window(state, WINDOW_LABEL)?),
        "stop_service" => value(commands::stop_service(state, string(&args, "url")?)?),
        "restart_service" => value(commands::restart_service(state, string(&args, "url")?)?),
        "check_built_in_runtime_update" => value(commands::check_built_in_runtime_update(state)?),
        "update_built_in_runtime" => value(commands::update_built_in_runtime(state)?),
        "update_global_settings" => value(commands::update_global_settings(
            serde_json::from_value(args.get("patch").cloned().unwrap_or(Value::Null)).map_err(
                |error| AppError::new("app.error.invalidRequest").technical(error.to_string()),
            )?,
            state,
        )?),
        "window_minimize" => {
            window.minimize();
            Ok(Value::Null)
        }
        "window_maximize" => {
            window.maximize();
            Ok(Value::Null)
        }
        "window_close" | "restart_app" => {
            window.close();
            Ok(Value::Null)
        }
        "focus_app_window" | "close_app_window" => Ok(Value::Null),
        _ => Err(AppError::new("app.error.unknownCommand")),
    }
}

fn string(args: &Value, name: &str) -> AppResult<String> {
    args.get(name)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::new("app.error.invalidRequest"))
}

fn value(value: impl serde::Serialize) -> AppResult<Value> {
    serde_json::to_value(value).map_err(|error| {
        AppError::new("app.error.serializationFailed").technical(error.to_string())
    })
}

fn json_response(status: StatusCode, body: Value) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(serde_json::to_vec(&body).expect("JSON serializes"))
        .with_status_code(status)
        .with_header(
            Header::from_bytes("Content-Type", "application/json; charset=utf-8")
                .expect("valid header"),
        )
}

fn static_response(frontend: &Path, url: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let path = url.split('?').next().unwrap_or("/").trim_start_matches('/');
    let requested = frontend.join(if path.is_empty() { "index.html" } else { path });
    let file = requested
        .is_file()
        .then_some(requested)
        .unwrap_or_else(|| frontend.join("index.html"));
    match fs::read(&file) {
        Ok(body) => Response::from_data(body)
            .with_header(Header::from_bytes("Content-Type", mime(&file)).expect("valid header")),
        Err(_) => Response::from_string("DSH Desktop frontend is unavailable")
            .with_status_code(StatusCode(500)),
    }
}

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("json") => "application/json; charset=utf-8",
        _ => "text/html; charset=utf-8",
    }
}
