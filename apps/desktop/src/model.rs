use serde::{Deserialize, Serialize};

pub const DEFAULT_DSH_PORT: u16 = 3080;
pub const DEFAULT_DSH_PORT_RANGE_END: u16 = 3090;
pub const LOCAL_DSH_HOST: &str = "127.0.0.1";
pub const DEFAULT_SERVICE_IDLE_TIMEOUT_SECONDS: u64 = 0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DistributionVariant {
    Bundled,
    Slim,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledRuntimeSnapshot {
    pub runtime_id: String,
    pub node_version: String,
    pub dsh_version: String,
    pub pnpm_version: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionSnapshot {
    pub variant: DistributionVariant,
    pub built_in_runtime: Option<BundledRuntimeSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeUpdateSnapshot {
    pub current_version: String,
    pub candidate_version: Option<String>,
    pub automatic_rollback_supported: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMetadataSnapshot {
    pub name: String,
    pub version: String,
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub locale: Option<AppLocale>,
    pub dsh_source: DshSource,
    pub dsh_home: DshHome,
    pub window_startup_attempts: Vec<WindowStartupAttempt>,
    pub managed_service_idle_timeout_seconds: u64,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            locale: None,
            dsh_source: DshSource::System,
            dsh_home: DshHome::Environment,
            window_startup_attempts: default_window_startup_attempts(),
            managed_service_idle_timeout_seconds: DEFAULT_SERVICE_IDLE_TIMEOUT_SECONDS,
        }
    }
}

impl GlobalSettings {
    pub fn default_for(variant: DistributionVariant) -> Self {
        Self {
            dsh_source: match variant {
                DistributionVariant::Bundled => DshSource::BuiltIn,
                DistributionVariant::Slim => DshSource::System,
            },
            ..Self::default()
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
        WindowStartupAttempt::StartRange {
            host: LOCAL_DSH_HOST.to_owned(),
            start_port: DEFAULT_DSH_PORT,
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
    Npx { version: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DshHome {
    Environment,
    Custom { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
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
    Stopping,
    Restarting,
    Updating,
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
    pub can_stop: bool,
    pub can_restart: bool,
    pub logs: Vec<String>,
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
pub struct RuntimeUpdateResult {
    pub distribution: DistributionSnapshot,
    pub host: HostSnapshot,
    pub updated_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub app: AppMetadataSnapshot,
    pub settings: GlobalSettings,
    pub distribution: DistributionSnapshot,
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
    pub distribution: DistributionSnapshot,
    pub window: WindowSnapshot,
    pub host: HostSnapshot,
    pub failures: Vec<StartupAttemptFailure>,
}

pub type GlobalSettingsPatch = GlobalSettings;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_use_the_standard_startup_sequence_and_immediate_cleanup() {
        assert_eq!(
            default_window_startup_attempts(),
            vec![
                WindowStartupAttempt::KnownServices,
                WindowStartupAttempt::ConnectFixed {
                    host: "127.0.0.1".to_owned(),
                    port: 3080,
                },
                WindowStartupAttempt::StartRange {
                    host: "127.0.0.1".to_owned(),
                    start_port: 3080,
                    end_port: 3090,
                },
            ]
        );
        assert_eq!(
            GlobalSettings::default().managed_service_idle_timeout_seconds,
            0
        );
    }

    #[test]
    fn serializes_startup_attempt_fields_as_camel_case() {
        let attempt = WindowStartupAttempt::StartRange {
            host: "127.0.0.1".to_owned(),
            start_port: 3081,
            end_port: 3090,
        };

        let value = serde_json::to_value(&attempt).expect("serialize startup attempt");
        assert_eq!(value["startPort"], 3081);
        assert_eq!(value["endPort"], 3090);
        assert!(value.get("start_port").is_none());
        assert!(value.get("end_port").is_none());
        assert_eq!(
            serde_json::from_value::<WindowStartupAttempt>(value)
                .expect("deserialize startup attempt"),
            attempt
        );
    }

    #[test]
    fn serializes_npx_source_with_a_version() {
        let source = DshSource::Npx {
            version: "0.1.0-rc.6".to_owned(),
        };

        assert_eq!(
            serde_json::to_value(&source).expect("serialize npx source"),
            serde_json::json!({ "type": "npx", "version": "0.1.0-rc.6" })
        );
    }
}
