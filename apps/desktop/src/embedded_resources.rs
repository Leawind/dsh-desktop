use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::{AppError, AppResult};

pub struct EmbeddedFile {
    pub path: &'static str,
    pub data: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

pub struct EmbeddedResources {
    pub frontend_directory: PathBuf,
    pub icon: PathBuf,
    pub runtime_seed_directory: PathBuf,
}

pub fn materialize(data_directory: &Path) -> AppResult<EmbeddedResources> {
    let root = data_directory.join("embedded-resources").join(RESOURCE_ID);
    if !is_bootstrap_complete(&root) {
        let staging = data_directory
            .join("embedded-resources")
            .join(format!(".{RESOURCE_ID}.staging-{}", std::process::id()));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(resource_error)?;
        }
        fs::create_dir_all(&staging).map_err(resource_error)?;
        let result = write_bootstrap_resources(&staging);
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        if root.exists() {
            fs::remove_dir_all(&root).map_err(resource_error)?;
        }
        fs::rename(&staging, &root).map_err(resource_error)?;
    }
    Ok(EmbeddedResources {
        frontend_directory: root.join("frontend"),
        icon: root.join("icons/app-icon.png"),
        runtime_seed_directory: root.join("runtime/bundled"),
    })
}

pub fn materialize_runtime_seed(seed_directory: &Path) -> AppResult<()> {
    if RUNTIME_SEED_FILES.is_empty() || runtime_seed_is_complete(seed_directory) {
        return Ok(());
    }
    let parent = seed_directory
        .parent()
        .ok_or_else(|| AppError::new("runtime.error.installFailed"))?;
    fs::create_dir_all(parent).map_err(resource_error)?;
    let staging = parent.join(format!(".bundled.staging-{}", std::process::id()));
    let previous = parent.join(format!(".bundled.previous-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(resource_error)?;
    }
    if previous.exists() {
        fs::remove_dir_all(&previous).map_err(resource_error)?;
    }
    fs::create_dir_all(&staging).map_err(resource_error)?;
    if let Err(error) = write_group(&staging, "", RUNTIME_SEED_FILES) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if seed_directory.exists() {
        fs::rename(seed_directory, &previous).map_err(resource_error)?;
    }
    if let Err(error) = fs::rename(&staging, seed_directory) {
        if previous.exists() {
            let _ = fs::rename(&previous, seed_directory);
        }
        let _ = fs::remove_dir_all(&staging);
        return Err(resource_error(error));
    }
    if previous.exists() {
        fs::remove_dir_all(previous).map_err(resource_error)?;
    }
    Ok(())
}

fn is_bootstrap_complete(root: &Path) -> bool {
    (FRONTEND_FILES.is_empty() || root.join("frontend/index.html").is_file())
        && root.join("icons/app-icon.png").is_file()
        && (RUNTIME_SEED_FILES.is_empty() || root.join("runtime/bundled/manifest.json").is_file())
}

fn runtime_seed_is_complete(seed_directory: &Path) -> bool {
    RUNTIME_SEED_FILES
        .iter()
        .all(|file| seed_directory.join(file.path).is_file())
}

fn write_bootstrap_resources(root: &Path) -> AppResult<()> {
    write_group(root, "frontend", FRONTEND_FILES)?;
    write_group(root, "icons", ICON_FILES)?;
    let manifest = RUNTIME_SEED_FILES
        .iter()
        .find(|file| file.path == "manifest.json");
    if let Some(manifest) = manifest {
        write_group(root, "runtime/bundled", std::slice::from_ref(manifest))?;
    }
    Ok(())
}

fn write_group(root: &Path, prefix: &str, files: &[EmbeddedFile]) -> AppResult<()> {
    for file in files {
        let relative = Path::new(file.path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(AppError::new("runtime.error.installFailed"));
        }
        let destination = root.join(prefix).join(relative);
        let parent = destination
            .parent()
            .ok_or_else(|| AppError::new("runtime.error.installFailed"))?;
        fs::create_dir_all(parent).map_err(resource_error)?;
        fs::write(destination, file.data).map_err(resource_error)?;
    }
    Ok(())
}

fn resource_error(error: std::io::Error) -> AppError {
    AppError::new("runtime.error.installFailed").technical(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materializes_frontend_and_icon_from_the_executable() {
        let directory = std::env::temp_dir().join(format!(
            "dsh-desktop-embedded-resources-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        let resources = materialize(&directory).expect("materialize resources");
        if !FRONTEND_FILES.is_empty() {
            assert!(resources.frontend_directory.join("index.html").is_file());
        }
        assert!(resources.icon.is_file());
        if !RUNTIME_SEED_FILES.is_empty() {
            assert!(
                resources
                    .runtime_seed_directory
                    .join("manifest.json")
                    .is_file()
            );
            assert!(!runtime_seed_is_complete(&resources.runtime_seed_directory));
            materialize_runtime_seed(&resources.runtime_seed_directory)
                .expect("materialize deferred runtime seed");
            assert!(runtime_seed_is_complete(&resources.runtime_seed_directory));
        }
        let _ = fs::remove_dir_all(directory);
    }
}
