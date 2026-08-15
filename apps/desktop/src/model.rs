use serde::{Deserialize, Serialize};

pub const DEFAULT_DSH_PORT: u16 = 3080;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub default_dsh_port: u16,
    pub locale: Option<AppLocale>,
    pub dsh_executable: Option<String>,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            default_dsh_port: DEFAULT_DSH_PORT,
            locale: None,
            dsh_executable: None,
        }
    }
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettingsPatch {
    pub default_dsh_port: u16,
    pub locale: Option<AppLocale>,
    pub dsh_executable: Option<String>,
}
