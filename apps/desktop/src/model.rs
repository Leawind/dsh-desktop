use serde::{Deserialize, Serialize};

pub const DEFAULT_DSH_PORT: u16 = 3080;
pub const DEFAULT_DSH_PORT_RANGE_END: u16 = 3090;
pub const LOCAL_DSH_HOST: &str = "127.0.0.1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub locale: Option<AppLocale>,
    pub dsh_source: DshSource,
    pub window_startup_attempts: Vec<WindowStartupAttempt>,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            locale: None,
            dsh_source: DshSource::System,
            window_startup_attempts: default_window_startup_attempts(),
        }
    }
}

pub fn default_window_startup_attempts() -> Vec<WindowStartupAttempt> {
    vec![
        WindowStartupAttempt::KnownServices,
        WindowStartupAttempt::ConnectFixed {
            host: LOCAL_DSH_HOST.to_owned(),
            port: DEFAULT_DSH_PORT,
        },
        WindowStartupAttempt::StartFixed {
            host: LOCAL_DSH_HOST.to_owned(),
            port: DEFAULT_DSH_PORT,
        },
        WindowStartupAttempt::StartRange {
            host: LOCAL_DSH_HOST.to_owned(),
            start_port: DEFAULT_DSH_PORT + 1,
            end_port: DEFAULT_DSH_PORT_RANGE_END,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DshSource {
    None,
    BuiltIn,
    System,
    Custom { executable: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum WindowStartupAttempt {
    KnownServices,
    ConnectFixed {
        host: String,
        port: u16,
    },
    StartFixed {
        host: String,
        port: u16,
    },
    StartRange {
        host: String,
        start_port: u16,
        end_port: u16,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppLocale {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ServiceStatus {
    Unreachable,
    Starting,
    Running,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSnapshot {
    pub label: String,
    pub url: String,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointSnapshot {
    pub url: String,
    pub status: ServiceStatus,
    pub ownership: EndpointOwnership,
    pub connected_windows: usize,
    pub pid: Option<u32>,
    pub runtime_version: Option<String>,
    pub last_error: Option<String>,
    pub known: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EndpointOwnership {
    External,
    Managed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSnapshot {
    pub windows: Vec<WindowSnapshot>,
    pub endpoints: Vec<EndpointSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub settings: GlobalSettings,
    pub window: WindowSnapshot,
    pub host: HostSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupAttemptFailure {
    pub attempt: WindowStartupAttempt,
    pub error: crate::error::AppError,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStartupResult {
    pub connected: bool,
    pub window: WindowSnapshot,
    pub host: HostSnapshot,
    pub failures: Vec<StartupAttemptFailure>,
}

pub type GlobalSettingsPatch = GlobalSettings;
