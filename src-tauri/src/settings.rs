use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::model::{GlobalSettings, GlobalSettingsPatch};

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

pub fn validate(patch: GlobalSettingsPatch) -> AppResult<GlobalSettings> {
    if patch.default_dsh_port == 0 {
        return Err(AppError::new("settings.error.invalidPort"));
    }

    let dsh_executable = patch
        .dsh_executable
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    Ok(GlobalSettings {
        default_dsh_port: patch.default_dsh_port,
        locale: patch.locale,
        dsh_executable,
    })
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
    fn rejects_port_zero() {
        let patch = GlobalSettingsPatch {
            default_dsh_port: 0,
            locale: None,
            dsh_executable: None,
        };
        assert!(validate(patch).is_err());
    }
}
