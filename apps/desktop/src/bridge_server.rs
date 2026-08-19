use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::commands;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::window_control::WindowControlRegistry;
use crate::window_registry::{WindowActivityRegistry, WindowRegistry};

#[derive(Deserialize)]
struct CommandRequest {
    name: String,
    #[serde(default)]
    args: Value,
}

pub struct BridgeServer {
    base_url: String,
    pub token: String,
    window_controls: WindowControlRegistry,
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
    client_activity: WindowActivityRegistry,
    window_controls: WindowControlRegistry,
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
    let server_window_controls = window_controls.clone();
    thread::spawn(move || {
        for request in server.incoming_requests() {
            let state = Arc::clone(&state);
            let frontend = frontend.clone();
            let token = server_token.clone();
            let windows = windows.clone();
            let client_activity = client_activity.clone();
            let window_controls = server_window_controls.clone();
            thread::spawn(move || {
                respond(
                    request,
                    state,
                    &frontend,
                    &windows,
                    &token,
                    &client_activity,
                    &window_controls,
                )
            });
        }
    });
    Ok(BridgeServer {
        base_url: format!("http://{address}"),
        token,
        window_controls,
    })
}

impl BridgeServer {
    pub fn close_windows(&self) {
        self.window_controls
            .send_to_all(|stream| write_websocket_text(stream, r#"{"type":"close"}"#));
    }
}

fn respond(
    mut request: tiny_http::Request,
    state: Arc<AppState>,
    frontend: &Path,
    windows: &WindowRegistry,
    token: &str,
    client_activity: &WindowActivityRegistry,
    window_controls: &WindowControlRegistry,
) {
    let request_path = path(request.url());
    if request_path == "/api/window-control" {
        return respond_window_control(request, windows, token, window_controls);
    }
    let response = if let Some((label, session_token)) = session_path(request_path) {
        if session_token != token || windows.get(label).is_none() {
            json_response(
                StatusCode(403),
                json!({ "error": AppError::new("app.error.unauthorized") }),
            )
        } else {
            static_response(frontend, request.url())
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
            client_activity.record_seen(&label, unix_time_millis());
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
                .and_then(|request: CommandRequest| dispatch(&state, &label, request));
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

fn respond_window_control(
    request: tiny_http::Request,
    windows: &WindowRegistry,
    token: &str,
    window_controls: &WindowControlRegistry,
) {
    let Some(label) = window_label(request.url()) else {
        return respond_unauthorized(request);
    };
    if query_value(request.url(), "token").as_deref() != Some(token) {
        return respond_unauthorized(request);
    }
    if windows.get(&label).is_none() {
        return respond_not_found(request);
    }
    let Some(key) = websocket_key(&request) else {
        let _ = request.respond(json_response(
            StatusCode(400),
            json!({ "error": AppError::new("app.error.invalidRequest") }),
        ));
        return;
    };

    let response = Response::empty(StatusCode(101)).with_header(
        Header::from_bytes("Sec-WebSocket-Accept", websocket_accept(&key))
            .expect("valid websocket accept header"),
    );
    let stream = request.upgrade("websocket", response);
    window_controls.connect(label, stream);
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
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv("X-DSH-Desktop-Token"))
        .is_some_and(|header| header.value.as_str() == token)
}

fn path(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

fn window_label(url: &str) -> Option<String> {
    query_value(url, "window").filter(|label| !label.is_empty())
}

fn query_value(url: &str, expected_name: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    url::form_urlencoded::parse(query.as_bytes())
        .find_map(|(name, value)| (name == expected_name).then(|| value.into_owned()))
}

fn websocket_key(request: &tiny_http::Request) -> Option<String> {
    (request.method() == &Method::Get)
        .then_some(())
        .filter(|_| {
            request.headers().iter().any(|header| {
                header.field.equiv("Upgrade")
                    && header.value.as_str().eq_ignore_ascii_case("websocket")
            })
        })
        .filter(|_| {
            request.headers().iter().any(|header| {
                header.field.equiv("Connection")
                    && header
                        .value
                        .as_str()
                        .split(',')
                        .any(|value| value.trim().eq_ignore_ascii_case("upgrade"))
            })
        })
        .filter(|_| {
            request.headers().iter().any(|header| {
                header.field.equiv("Sec-WebSocket-Version") && header.value.as_str() == "13"
            })
        })
        .and_then(|_| {
            request.headers().iter().find_map(|header| {
                header
                    .field
                    .equiv("Sec-WebSocket-Key")
                    .then(|| header.value.as_str().to_owned())
            })
        })
}

fn websocket_accept(key: &str) -> String {
    let mut digest = Sha1::new();
    digest.update(key.as_bytes());
    digest.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64::engine::general_purpose::STANDARD.encode(digest.finalize())
}

fn write_websocket_text(stream: &mut dyn Write, text: &str) -> std::io::Result<()> {
    let payload = text.as_bytes();
    let length =
        u8::try_from(payload.len()).expect("control messages fit in a short websocket frame");
    stream.write_all(&[0x81, length])?;
    stream.write_all(payload)?;
    stream.flush()
}

fn session_path(path: &str) -> Option<(&str, &str)> {
    let mut parts = path.strip_prefix("/session/")?.trim_matches('/').split('/');
    let label = parts.next()?;
    let token = parts.next()?;
    parts.next().is_none().then_some((label, token))
}

fn dispatch(state: &AppState, label: &str, request: CommandRequest) -> AppResult<Value> {
    let args = request.args;
    match request.name.as_str() {
        "heartbeat" => value(()),
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
        "check_app_update" => value(commands::check_app_update(state)?),
        "install_app_update" => value(commands::install_app_update(state)?),
        "update_global_settings" => value(commands::update_global_settings(
            serde_json::from_value(args.get("patch").cloned().unwrap_or(Value::Null)).map_err(
                |error| AppError::new("app.error.invalidRequest").technical(error.to_string()),
            )?,
            state,
        )?),
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
    fn accepts_websocket_keys_using_the_rfc_handshake() {
        assert_eq!(
            websocket_accept("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn reads_control_credentials_from_the_query() {
        let url = "/api/window-control?window=main%20window&token=secret";
        assert_eq!(window_label(url).as_deref(), Some("main window"));
        assert_eq!(query_value(url, "token").as_deref(), Some("secret"));
    }

    #[test]
    fn writes_the_close_event_as_a_short_unmasked_text_frame() {
        let mut bytes = Vec::new();
        write_websocket_text(&mut bytes, r#"{"type":"close"}"#).unwrap();
        assert_eq!(bytes, b"\x81\x10{\"type\":\"close\"}");
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
