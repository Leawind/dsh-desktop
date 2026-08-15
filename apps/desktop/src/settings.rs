use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::model::{DshSource, GlobalSettings, GlobalSettingsPatch, WindowStartupAttempt};

const SETTINGS_FILE: &str = "settings.json";

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SETTINGS_FILE)
}

pub fn load(config_dir: &Path) -> GlobalSettings {
    let path = settings_path(config_dir);
    let Ok(contents) = fs::read_to_string(path) else {
        return GlobalSettings::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn validate(mut patch: GlobalSettingsPatch) -> AppResult<GlobalSettings> {
    if let DshSource::Custom { executable } = &mut patch.dsh_source {
        *executable = executable.trim().to_owned();
        if executable.is_empty() {
            return Err(AppError::new("settings.error.emptyExecutable"));
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
        assert!(validate(settings).is_err());
    }
}
