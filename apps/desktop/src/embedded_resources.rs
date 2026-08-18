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
    if !is_complete(&root) {
        let staging = data_directory
            .join("embedded-resources")
            .join(format!(".{RESOURCE_ID}.staging-{}", std::process::id()));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(resource_error)?;
        }
        fs::create_dir_all(&staging).map_err(resource_error)?;
        let result = write_resources(&staging);
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
        icon: root.join("icons/icon.png"),
        runtime_seed_directory: root.join("runtime/bundled"),
    })
}

fn is_complete(root: &Path) -> bool {
    root.join("frontend/index.html").is_file()
        && root.join("icons/icon.png").is_file()
        && (RUNTIME_SEED_FILES.is_empty() || root.join("runtime/bundled/manifest.json").is_file())
}

fn write_resources(root: &Path) -> AppResult<()> {
    write_group(root, "frontend", FRONTEND_FILES)?;
    write_group(root, "icons", ICON_FILES)?;
    write_group(root, "runtime/bundled", RUNTIME_SEED_FILES)
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
        assert!(resources.frontend_directory.join("index.html").is_file());
        assert!(resources.icon.is_file());
        let _ = fs::remove_dir_all(directory);
    }
}
