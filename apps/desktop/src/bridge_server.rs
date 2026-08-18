use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use serde::Deserialize;
use serde_json::{Value, json};
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::commands;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::window_registry::WindowRegistry;

#[derive(Deserialize)]
struct CommandRequest {
    name: String,
    #[serde(default)]
    args: Value,
}

pub struct BridgeServer {
    base_url: String,
    pub token: String,
}

impl BridgeServer {
    pub fn url_for(&self, label: &str) -> String {
        format!("{}/session/{label}/{}/", self.base_url, self.token)
    }
}

pub fn start(
    state: AppState,
    frontend: PathBuf,
    windows: WindowRegistry,
    running: Arc<AtomicBool>,
    client_activity: Arc<Mutex<std::collections::HashMap<String, u64>>>,
) -> AppResult<BridgeServer> {
    let bind_address = std::env::var("DSH_DESKTOP_BRIDGE_PORT")
        .ok()
        .map(|port| format!("127.0.0.1:{port}"))
        .unwrap_or_else(|| "127.0.0.1:0".to_owned());
    let server = Server::http(&bind_address).map_err(|error| {
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
            let windows = windows.clone();
            let running = Arc::clone(&running);
            let client_activity = Arc::clone(&client_activity);
            thread::spawn(move || {
                respond(
                    request,
                    state,
                    &frontend,
                    &windows,
                    &token,
                    &running,
                    &client_activity,
                )
            });
        }
    });
    Ok(BridgeServer {
        base_url: format!("http://{address}"),
        token,
    })
}

fn respond(
    mut request: tiny_http::Request,
    state: Arc<AppState>,
    frontend: &Path,
    windows: &WindowRegistry,
    token: &str,
    running: &AtomicBool,
    client_activity: &Mutex<std::collections::HashMap<String, u64>>,
) {
    let request_path = path(request.url());
    let response = if request_path == "/api/session" {
        let Some(label) = window_label(request.url()) else {
            return respond_unauthorized(request);
        };
        if request.method() != &Method::Get
            || session_token(request.url()).as_deref() != Some(token)
            || windows.get(&label).is_none()
        {
            json_response(
                StatusCode(403),
                json!({ "error": AppError::new("app.error.unauthorized") }),
            )
        } else {
            session_response(frontend, token)
        }
    } else if let Some((label, session_token)) = session_path(request_path) {
        if session_token != token || windows.get(label).is_none() {
            json_response(
                StatusCode(403),
                json!({ "error": AppError::new("app.error.unauthorized") }),
            )
        } else {
            session_response(frontend, token)
        }
    } else if request_path.starts_with("/api/") {
        let Some(label) = window_label(request.url()) else {
            return respond_unauthorized(request);
        };
        if !authorized(&request, token) {
            json_response(
                StatusCode(403),
                json!({ "error": AppError::new("app.error.unauthorized") }),
            )
        } else if request.method() != &Method::Post || request_path != "/api/command" {
            json_response(
                StatusCode(404),
                json!({ "error": AppError::new("app.error.notFound") }),
            )
        } else {
            if windows.get(&label).is_none() {
                return respond_not_found(request);
            }
            client_activity
                .lock()
                .expect("client activity state poisoned")
                .insert(label.clone(), unix_time_millis());
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
                .and_then(|request: CommandRequest| {
                    dispatch(&state, windows, running, &label, request)
                });
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

fn respond_unauthorized(request: tiny_http::Request) {
    let _ = request.respond(json_response(
        StatusCode(403),
        json!({ "error": AppError::new("app.error.unauthorized") }),
    ));
}

fn respond_not_found(request: tiny_http::Request) {
    let _ = request.respond(json_response(
        StatusCode(404),
        json!({ "error": AppError::new("app.error.notFound") }),
    ));
}

fn unix_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn authorized(request: &tiny_http::Request, token: &str) -> bool {
    let cookie_authorized = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Cookie"))
        .is_some_and(|header| {
            header
                .value
                .as_str()
                .split(';')
                .map(str::trim)
                .any(|cookie| cookie == format!("dsh_desktop_token={token}"))
        });
    let header_authorized = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("X-DSH-Desktop-Token"))
        .is_some_and(|header| header.value.as_str() == token);
    cookie_authorized || header_authorized
}

fn path(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

fn window_label(url: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    url::form_urlencoded::parse(query.as_bytes())
        .find_map(|(name, value)| (name == "window").then(|| value.into_owned()))
        .filter(|label| !label.is_empty())
}

fn session_token(url: &str) -> Option<String> {
    url.split_once('?').and_then(|(_, query)| {
        url::form_urlencoded::parse(query.as_bytes())
            .find_map(|(name, value)| (name == "token").then(|| value.into_owned()))
    })
}

fn session_path(path: &str) -> Option<(&str, &str)> {
    let mut parts = path.strip_prefix("/session/")?.trim_matches('/').split('/');
    let label = parts.next()?;
    let token = parts.next()?;
    parts.next().is_none().then_some((label, token))
}

fn dispatch(
    state: &AppState,
    windows: &WindowRegistry,
    running: &AtomicBool,
    label: &str,
    request: CommandRequest,
) -> AppResult<Value> {
    let args = request.args;
    match request.name.as_str() {
        "initialize_window" => value(commands::initialize_window(state, label)?),
        "get_host_snapshot" => value(commands::get_host_snapshot(state)),
        "set_window_target" => value(commands::set_window_target(
            label,
            string(&args, "url")?,
            state,
        )?),
        "start_window" => value(commands::start_window(state, label)?),
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
            windows.get(label).expect("window was checked").minimize();
            Ok(Value::Null)
        }
        "window_maximize" => {
            windows.get(label).expect("window was checked").maximize();
            Ok(Value::Null)
        }
        "window_close" | "close_app_window" => {
            let target = args.get("label").and_then(Value::as_str).unwrap_or(label);
            if let Some(window) = windows.remove(target) {
                window.close();
                state.remove_window(target);
            }
            if windows.is_empty() {
                running.store(false, Ordering::Release);
            }
            Ok(Value::Null)
        }
        "focus_app_window" => {
            let target = args.get("label").and_then(Value::as_str).unwrap_or(label);
            windows
                .get(target)
                .ok_or_else(|| AppError::new("window.error.notFound"))?
                .focus();
            Ok(Value::Null)
        }
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

fn session_response(frontend: &Path, token: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let index = frontend.join("index.html");
    match fs::read(&index) {
        Ok(body) => Response::from_data(body)
            .with_header(Header::from_bytes("Content-Type", mime(&index)).expect("valid header"))
            .with_header(
                Header::from_bytes(
                    "Set-Cookie",
                    format!("dsh_desktop_token={token}; Path=/api; HttpOnly; SameSite=Strict"),
                )
                .expect("valid cookie header"),
            ),
        Err(_) => Response::from_string("DSH Desktop frontend is unavailable")
            .with_status_code(StatusCode(500)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_window_session_path() {
        assert_eq!(
            session_path("/session/dsh-1/a1b2/"),
            Some(("dsh-1", "a1b2"))
        );
        assert_eq!(session_path("/session/dsh-1/"), None);
    }

    #[test]
    fn extracts_the_bootstrap_token_without_retaining_it() {
        assert_eq!(
            session_token("/api/session?window=dsh-1&token=a1b2").as_deref(),
            Some("a1b2")
        );
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
