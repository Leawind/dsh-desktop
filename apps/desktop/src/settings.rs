use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::model::{
    DistributionVariant, DshHome, DshSource, GlobalSettings, GlobalSettingsPatch,
    MAX_PAGE_SCALE_PERCENT, MIN_PAGE_SCALE_PERCENT, WindowStartupAttempt,
};

const SETTINGS_FILE: &str = "settings.json";

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SETTINGS_FILE)
}

pub fn load(config_dir: &Path, variant: DistributionVariant) -> GlobalSettings {
    let path = settings_path(config_dir);
    let Ok(contents) = fs::read_to_string(path) else {
        return GlobalSettings::default_for(variant);
    };
    serde_json::from_str(&contents).unwrap_or_else(|_| GlobalSettings::default_for(variant))
}

pub fn validate(
    mut patch: GlobalSettingsPatch,
    variant: DistributionVariant,
) -> AppResult<GlobalSettings> {
    if !(MIN_PAGE_SCALE_PERCENT..=MAX_PAGE_SCALE_PERCENT).contains(&patch.page_scale_percent) {
        return Err(AppError::new("settings.error.invalidPageScale"));
    }
    if patch.managed_service_idle_timeout_seconds > 7 * 24 * 60 * 60 {
        return Err(AppError::new("settings.error.invalidIdleTimeout"));
    }
    if patch.dsh_source == DshSource::BuiltIn && variant != DistributionVariant::Bundled {
        return Err(AppError::new("settings.error.unsupportedSource"));
    }
    if let DshSource::Custom { executable } = &mut patch.dsh_source {
        *executable = executable.trim().to_owned();
        if executable.is_empty() {
            return Err(AppError::new("settings.error.emptyExecutable"));
        }
    }
    if let DshSource::Npx { version } = &mut patch.dsh_source {
        *version = version.trim().to_owned();
        if !valid_npx_dsh_version(version) {
            return Err(AppError::new("settings.error.invalidNpxVersion"));
        }
    }
    if let DshHome::Custom { path } = &mut patch.dsh_home {
        *path = path.trim().to_owned();
        if path.is_empty() {
            return Err(AppError::new("settings.error.emptyDshHome"));
        }
        if !Path::new(path).is_absolute()
            && path != "~"
            && !path.starts_with("~/")
            && !path.starts_with("~\\")
        {
            return Err(AppError::new("settings.error.relativeDshHome"));
        }
    }

    for attempt in &mut patch.window_startup_attempts {
        match attempt {
            WindowStartupAttempt::KnownServices => {}
            WindowStartupAttempt::ConnectFixed { host, port }
            | WindowStartupAttempt::StartFixed { host, port } => {
                validate_host(host)?;
                validate_port(*port)?;
            }
            WindowStartupAttempt::StartRange {
                host,
                start_port,
                end_port,
            } => {
                validate_host(host)?;
                validate_port(*start_port)?;
                validate_port(*end_port)?;
                if start_port > end_port {
                    return Err(AppError::new("settings.error.invalidPortRange"));
                }
            }
        }
    }
    Ok(patch)
}

fn valid_npx_dsh_version(version: &str) -> bool {
    if version == "latest" {
        return true;
    }
    let (core, build) = version.split_once('+').unwrap_or((version, ""));
    if !build.is_empty()
        && !build.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
    {
        return false;
    }
    let (core, pre_release) = core.split_once('-').unwrap_or((core, ""));
    let valid_identifiers = |value: &str| {
        !value.is_empty()
            && value.split('.').all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '-')
            })
    };
    let mut parts = core.split('.');
    let valid_core = parts
        .by_ref()
        .map(|part| !part.is_empty() && part.chars().all(|character| character.is_ascii_digit()))
        .collect::<Vec<_>>();
    valid_core.len() == 3
        && valid_core.into_iter().all(|valid| valid)
        && (pre_release.is_empty() || valid_identifiers(pre_release))
}

fn validate_host(host: &mut String) -> AppResult<()> {
    *host = host.trim().to_owned();
    host.parse::<IpAddr>()
        .map(|_| ())
        .map_err(|_| AppError::new("settings.error.invalidIpAddress"))
}

fn validate_port(port: u16) -> AppResult<()> {
    if port == 0 {
        Err(AppError::new("settings.error.invalidPort"))
    } else {
        Ok(())
    }
}

pub fn save(config_dir: &Path, settings: &GlobalSettings) -> AppResult<()> {
    fs::create_dir_all(config_dir)
        .map_err(|error| AppError::new("settings.error.saveFailed").technical(error.to_string()))?;
    let path = settings_path(config_dir);
    let temporary_path = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(settings)
        .map_err(|error| AppError::new("settings.error.saveFailed").technical(error.to_string()))?;
    fs::write(&temporary_path, json)
        .map_err(|error| AppError::new("settings.error.saveFailed").technical(error.to_string()))?;
    fs::rename(&temporary_path, &path)
        .map_err(|error| AppError::new("settings.error.saveFailed").technical(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_port_range() {
        let settings = GlobalSettings {
            window_startup_attempts: vec![WindowStartupAttempt::StartRange {
                host: "127.0.0.1".to_owned(),
                start_port: 4000,
                end_port: 3000,
            }],
            ..GlobalSettings::default()
        };
        assert!(validate(settings, DistributionVariant::Slim).is_err());
    }

    #[test]
    fn rejects_page_scale_outside_the_supported_range() {
        let settings = GlobalSettings {
            page_scale_percent: 49.0,
            ..GlobalSettings::default()
        };
        assert!(validate(settings, DistributionVariant::Slim).is_err());

        let settings = GlobalSettings {
            page_scale_percent: 401.0,
            ..GlobalSettings::default()
        };
        assert!(validate(settings, DistributionVariant::Slim).is_err());
    }

    #[test]
    fn accepts_fractional_page_scale() {
        let settings = GlobalSettings {
            page_scale_percent: 172.8,
            ..GlobalSettings::default()
        };
        assert!(validate(settings, DistributionVariant::Slim).is_ok());
    }

    #[test]
    fn trims_custom_dsh_home() {
        let settings = GlobalSettings {
            dsh_home: DshHome::Custom {
                path: "  ~/.dsh-desktop  ".to_owned(),
            },
            ..GlobalSettings::default()
        };
        let validated = validate(settings, DistributionVariant::Slim).expect("valid settings");
        assert_eq!(
            validated.dsh_home,
            DshHome::Custom {
                path: "~/.dsh-desktop".to_owned()
            }
        );
    }

    #[test]
    fn rejects_relative_custom_dsh_home() {
        let settings = GlobalSettings {
            dsh_home: DshHome::Custom {
                path: "relative/dsh-home".to_owned(),
            },
            ..GlobalSettings::default()
        };
        assert!(validate(settings, DistributionVariant::Slim).is_err());
    }

    #[test]
    fn validates_and_trims_the_npx_dsh_version() {
        let settings = GlobalSettings {
            dsh_source: DshSource::Npx {
                version: " 0.1.0-rc.6 ".to_owned(),
            },
            ..GlobalSettings::default()
        };
        let validated = validate(settings, DistributionVariant::Slim).expect("valid settings");
        assert_eq!(
            validated.dsh_source,
            DshSource::Npx {
                version: "0.1.0-rc.6".to_owned()
            }
        );
    }

    #[test]
    fn rejects_non_version_npx_dsh_selectors() {
        let settings = GlobalSettings {
            dsh_source: DshSource::Npx {
                version: "next".to_owned(),
            },
            ..GlobalSettings::default()
        };
        assert!(validate(settings, DistributionVariant::Slim).is_err());
    }
}
