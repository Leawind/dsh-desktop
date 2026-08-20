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
    pub candidate_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMetadataSnapshot {
    pub name: String,
    pub version: String,
    pub identifier: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SystemColorScheme {
    Light,
    Dark,
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
    pub system_color_scheme: Option<SystemColorScheme>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlobalSettingsPatch {
    #[serde(default, skip_serializing_if = "LocalePatch::is_unchanged")]
    pub locale: LocalePatch,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dsh_source: Option<DshSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dsh_home: Option<DshHome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_startup_attempts: Option<Vec<WindowStartupAttempt>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_service_idle_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LocalePatch {
    #[default]
    Unchanged,
    Set(Option<AppLocale>),
}

impl LocalePatch {
    fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }
}

impl Serialize for LocalePatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Unchanged => serializer.serialize_unit(),
            Self::Set(locale) => locale.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for LocalePatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::Set(Option::<AppLocale>::deserialize(deserializer)?))
    }
}

impl GlobalSettingsPatch {
    pub fn apply_to(self, mut settings: GlobalSettings) -> GlobalSettings {
        if let LocalePatch::Set(locale) = self.locale {
            settings.locale = locale;
        }
        if let Some(dsh_source) = self.dsh_source {
            settings.dsh_source = dsh_source;
        }
        if let Some(dsh_home) = self.dsh_home {
            settings.dsh_home = dsh_home;
        }
        if let Some(window_startup_attempts) = self.window_startup_attempts {
            settings.window_startup_attempts = window_startup_attempts;
        }
        if let Some(managed_service_idle_timeout_seconds) =
            self.managed_service_idle_timeout_seconds
        {
            settings.managed_service_idle_timeout_seconds = managed_service_idle_timeout_seconds;
        }
        settings
    }
}

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

    #[test]
    fn applies_only_the_settings_fields_present_in_a_patch() {
        let patch = serde_json::from_value::<GlobalSettingsPatch>(serde_json::json!({
            "locale": null,
            "managedServiceIdleTimeoutSeconds": 120,
        }))
        .expect("deserialize settings patch");

        assert_eq!(patch.locale, LocalePatch::Set(None));
        assert_eq!(patch.dsh_source, None);

        let absent = serde_json::from_value::<GlobalSettingsPatch>(serde_json::json!({}))
            .expect("deserialize empty settings patch");
        assert_eq!(absent.locale, LocalePatch::Unchanged);

        assert_eq!(
            serde_json::to_value(GlobalSettingsPatch::default()).expect("serialize empty patch"),
            serde_json::json!({})
        );
        assert_eq!(
            serde_json::to_value(GlobalSettingsPatch {
                locale: LocalePatch::Set(None),
                ..GlobalSettingsPatch::default()
            })
            .expect("serialize locale reset"),
            serde_json::json!({ "locale": null })
        );

        let settings = patch.apply_to(GlobalSettings {
            locale: Some(AppLocale::EnUs),
            dsh_source: DshSource::Npx {
                version: "0.1.0".to_owned(),
            },
            ..GlobalSettings::default()
        });

        assert_eq!(settings.locale, None);
        assert_eq!(
            settings.dsh_source,
            DshSource::Npx {
                version: "0.1.0".to_owned()
            }
        );
        assert_eq!(settings.managed_service_idle_timeout_seconds, 120);
    }

    #[test]
    fn serializes_the_desktop_snapshot_wire_contract() {
        let payload = BootstrapPayload {
            app: AppMetadataSnapshot {
                name: "DSH Desktop".to_owned(),
                version: "0.2.3".to_owned(),
                identifier: "io.github.leawind.dsh-desktop".to_owned(),
            },
            settings: GlobalSettings {
                locale: Some(AppLocale::ZhCn),
                dsh_source: DshSource::Npx {
                    version: "0.1.0".to_owned(),
                },
                dsh_home: DshHome::Custom {
                    path: "/tmp/dsh".to_owned(),
                },
                window_startup_attempts: vec![WindowStartupAttempt::StartRange {
                    host: "127.0.0.1".to_owned(),
                    start_port: 3080,
                    end_port: 3090,
                }],
                managed_service_idle_timeout_seconds: 120,
            },
            distribution: DistributionSnapshot {
                variant: DistributionVariant::Bundled,
                built_in_runtime: Some(BundledRuntimeSnapshot {
                    runtime_id: "runtime-id".to_owned(),
                    node_version: "24.18.1".to_owned(),
                    dsh_version: "0.1.0".to_owned(),
                    pnpm_version: "11.7.0".to_owned(),
                    installed: true,
                }),
            },
            window: WindowSnapshot {
                label: "main".to_owned(),
                url: "http://127.0.0.1:3080".to_owned(),
                status: ServiceStatus::Running,
            },
            host: HostSnapshot {
                windows: Vec::new(),
                endpoints: vec![EndpointSnapshot {
                    url: "http://127.0.0.1:3080".to_owned(),
                    status: ServiceStatus::Running,
                    ownership: EndpointOwnership::Managed,
                    connected_windows: 1,
                    pid: Some(42),
                    runtime_version: Some("0.1.0".to_owned()),
                    last_error: None,
                    known: true,
                    can_stop: true,
                    can_restart: true,
                    logs: vec!["started".to_owned()],
                }],
            },
            system_color_scheme: Some(SystemColorScheme::Dark),
        };

        assert_eq!(
            serde_json::to_value(payload).expect("serialize desktop snapshot"),
            serde_json::json!({
                "app": {
                    "name": "DSH Desktop",
                    "version": "0.2.3",
                    "identifier": "io.github.leawind.dsh-desktop",
                },
                "settings": {
                    "locale": "zh-CN",
                    "dshSource": { "type": "npx", "version": "0.1.0" },
                    "dshHome": { "type": "custom", "path": "/tmp/dsh" },
                    "windowStartupAttempts": [{
                        "type": "start-range",
                        "host": "127.0.0.1",
                        "startPort": 3080,
                        "endPort": 3090,
                    }],
                    "managedServiceIdleTimeoutSeconds": 120,
                },
                "distribution": {
                    "variant": "bundled",
                    "builtInRuntime": {
                        "runtimeId": "runtime-id",
                        "nodeVersion": "24.18.1",
                        "dshVersion": "0.1.0",
                        "pnpmVersion": "11.7.0",
                        "installed": true,
                    },
                },
                "window": {
                    "label": "main",
                    "url": "http://127.0.0.1:3080",
                    "status": "running",
                },
                "host": {
                    "windows": [],
                    "endpoints": [{
                        "url": "http://127.0.0.1:3080",
                        "status": "running",
                        "ownership": "managed",
                        "connectedWindows": 1,
                        "pid": 42,
                        "runtimeVersion": "0.1.0",
                        "lastError": null,
                        "known": true,
                        "canStop": true,
                        "canRestart": true,
                        "logs": ["started"],
                    }],
                },
                "systemColorScheme": "dark",
            })
        );
    }
}
