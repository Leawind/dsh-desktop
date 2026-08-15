use std::fs;
use std::io::{Error, ErrorKind, Read};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::model::{BundledRuntimeSnapshot, DistributionSnapshot, DistributionVariant};

const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RuntimeManifest {
    schema_version: u32,
    runtime_id: String,
    target: String,
    node_version: String,
    dsh_version: String,
    pnpm_version: String,
    definition_sha256: String,
    node_executable: String,
    dsh_entrypoint: String,
    pnpm_entrypoint: String,
    archive: RuntimeFile,
    files: Vec<RuntimeFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct RuntimeFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone)]
pub struct InstalledRuntime {
    pub node_executable: PathBuf,
    pub dsh_entrypoint: PathBuf,
    pub executable_path: Vec<PathBuf>,
    pub dsh_version: String,
}

#[derive(Clone)]
pub struct RuntimeManager {
    variant: DistributionVariant,
    seed_directory: PathBuf,
    install_root: PathBuf,
}

impl DistributionVariant {
    pub fn current() -> Self {
        match env!("DSH_DESKTOP_VARIANT") {
            "bundled" => Self::Bundled,
            "slim" => Self::Slim,
            _ => unreachable!(),
        }
    }
}

impl RuntimeManager {
    pub fn new(resource_directory: PathBuf, data_directory: PathBuf) -> Self {
        let packaged_seed = resource_directory.join("runtime").join("bundled");
        let development_seed = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("runtime")
            .join("bundled");
        let seed_directory = if packaged_seed.join(MANIFEST_FILE).is_file() {
            packaged_seed
        } else {
            development_seed
        };
        Self {
            variant: DistributionVariant::current(),
            seed_directory,
            install_root: data_directory.join("runtimes").join("dsh"),
        }
    }

    #[cfg(test)]
    fn for_test(
        variant: DistributionVariant,
        seed_directory: PathBuf,
        install_root: PathBuf,
    ) -> Self {
        Self {
            variant,
            seed_directory,
            install_root,
        }
    }

    pub fn distribution_snapshot(&self) -> DistributionSnapshot {
        let built_in_runtime = (self.variant == DistributionVariant::Bundled)
            .then(|| self.read_manifest(&self.seed_directory).ok())
            .flatten()
            .map(|manifest| BundledRuntimeSnapshot {
                runtime_id: manifest.runtime_id.clone(),
                node_version: manifest.node_version.clone(),
                dsh_version: manifest.dsh_version.clone(),
                pnpm_version: manifest.pnpm_version.clone(),
                installed: self
                    .validate_installation(&self.install_root.join(&manifest.runtime_id), &manifest)
                    .is_ok(),
            });
        DistributionSnapshot {
            variant: self.variant,
            built_in_runtime,
        }
    }

    pub fn resolve_built_in(&self) -> AppResult<InstalledRuntime> {
        if self.variant != DistributionVariant::Bundled {
            return Err(AppError::new("service.error.builtInUnavailable"));
        }
        let manifest = self.read_manifest(&self.seed_directory)?;
        if manifest.schema_version != 1 {
            return Err(AppError::new("runtime.error.unsupportedManifest")
                .arg("schemaVersion", manifest.schema_version));
        }
        if manifest.target != env!("DSH_DESKTOP_TARGET") {
            return Err(AppError::new("runtime.error.targetMismatch")
                .arg("expected", env!("DSH_DESKTOP_TARGET"))
                .arg("actual", &manifest.target));
        }
        self.validate_seed(&manifest)?;
        let installation = self.install_root.join(&manifest.runtime_id);
        if self
            .validate_installation(&installation, &manifest)
            .is_err()
        {
            self.install(&manifest, &installation)?;
        }
        self.validate_installation(&installation, &manifest)?;

        let node_executable = resolve_manifest_path(&installation, &manifest.node_executable)?;
        let dsh_entrypoint = resolve_manifest_path(&installation, &manifest.dsh_entrypoint)?;
        let app_directory = dsh_entrypoint
            .ancestors()
            .find(|path| path.file_name().is_some_and(|name| name == "app"))
            .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))?;
        let node_directory = node_executable
            .parent()
            .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))?;
        let executable_path = vec![app_directory.join("bin"), node_directory.to_owned()];
        Ok(InstalledRuntime {
            node_executable,
            dsh_entrypoint,
            executable_path,
            dsh_version: manifest.dsh_version,
        })
    }

    fn install(&self, manifest: &RuntimeManifest, installation: &Path) -> AppResult<()> {
        fs::create_dir_all(&self.install_root).map_err(runtime_install_error)?;
        let staging = self.install_root.join(format!(
            ".{}.staging-{}",
            manifest.runtime_id,
            std::process::id()
        ));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(runtime_install_error)?;
        }
        fs::create_dir_all(&staging).map_err(runtime_install_error)?;
        fs::copy(
            self.seed_directory.join(MANIFEST_FILE),
            staging.join(MANIFEST_FILE),
        )
        .map_err(runtime_install_error)?;
        let notices = self.seed_directory.join("THIRD_PARTY_NOTICES.md");
        if notices.is_file() {
            fs::copy(notices, staging.join("THIRD_PARTY_NOTICES.md"))
                .map_err(runtime_install_error)?;
        }
        let archive = resolve_manifest_path(&self.seed_directory, &manifest.archive.path)?;
        extract_payload(&archive, &staging).map_err(runtime_install_error)?;
        self.validate_installation(&staging, manifest)?;
        if installation.exists() {
            fs::remove_dir_all(installation).map_err(runtime_install_error)?;
        }
        fs::rename(&staging, installation).map_err(runtime_install_error)
    }

    fn read_manifest(&self, directory: &Path) -> AppResult<RuntimeManifest> {
        let contents = fs::read_to_string(directory.join(MANIFEST_FILE)).map_err(|error| {
            AppError::new("runtime.error.manifestMissing").technical(error.to_string())
        })?;
        serde_json::from_str(&contents).map_err(|error| {
            AppError::new("runtime.error.invalidManifest").technical(error.to_string())
        })
    }

    fn validate_seed(&self, expected: &RuntimeManifest) -> AppResult<()> {
        let actual = self.read_manifest(&self.seed_directory)?;
        if actual != *expected {
            return Err(AppError::new("runtime.error.invalidManifest"));
        }
        let archive = resolve_manifest_path(&self.seed_directory, &actual.archive.path)?;
        let hash = file_sha256(&archive).map_err(|error| {
            AppError::new("runtime.error.integrityFailed")
                .arg("path", actual.archive.path.clone())
                .technical(error.to_string())
        })?;
        if hash != actual.archive.sha256 {
            return Err(AppError::new("runtime.error.integrityFailed")
                .arg("path", actual.archive.path.clone()));
        }
        Ok(())
    }

    fn validate_installation(&self, directory: &Path, expected: &RuntimeManifest) -> AppResult<()> {
        let actual = self.read_manifest(directory)?;
        if actual != *expected {
            return Err(AppError::new("runtime.error.invalidManifest"));
        }
        for file in &actual.files {
            let path = resolve_manifest_path(directory, &file.path)?;
            let hash = file_sha256(&path).map_err(|error| {
                AppError::new("runtime.error.integrityFailed")
                    .arg("path", file.path.clone())
                    .technical(error.to_string())
            })?;
            if hash != file.sha256 {
                return Err(
                    AppError::new("runtime.error.integrityFailed").arg("path", file.path.clone())
                );
            }
        }
        Ok(())
    }
}

fn runtime_install_error(error: std::io::Error) -> AppError {
    AppError::new("runtime.error.installFailed").technical(error.to_string())
}

fn resolve_manifest_path(root: &Path, relative: &str) -> AppResult<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::new("runtime.error.invalidManifest"));
    }
    Ok(root.join(relative))
}

fn file_sha256(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn extract_payload(source: &Path, destination: &Path) -> std::io::Result<()> {
    let mut archive = tar::Archive::new(GzDecoder::new(fs::File::open(source)?));
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            continue;
        }
        if !entry_type.is_dir() && !entry_type.is_file() {
            continue;
        }
        let path = entry.path()?.into_owned();
        if path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
            || !path.starts_with("payload")
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "invalid runtime archive path",
            ));
        }
        if !entry.unpack_in(destination)? {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "runtime archive path escaped installation directory",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dsh-desktop-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos()
        ))
    }

    #[test]
    fn slim_distribution_rejects_built_in_runtime() {
        let manager = RuntimeManager::for_test(
            DistributionVariant::Slim,
            PathBuf::from("missing"),
            PathBuf::from("missing"),
        );
        assert_eq!(
            manager
                .resolve_built_in()
                .expect_err("slim must reject")
                .code,
            "service.error.builtInUnavailable"
        );
    }

    #[test]
    fn rejects_parent_components_in_manifest_paths() {
        assert!(resolve_manifest_path(Path::new("runtime"), "../outside").is_err());
    }

    #[test]
    fn extracts_and_validates_archived_runtime() {
        let root = temporary_directory("runtime");
        let seed = root.join("seed");
        let installs = root.join("installs");
        fs::create_dir_all(seed.join("payload/app/node_modules/@deepseek-ai/dsh/lib"))
            .expect("create fixture");
        fs::create_dir_all(seed.join("payload/node/bin")).expect("create fixture");
        fs::write(seed.join("payload/node/bin/node"), b"node").expect("write fixture");
        fs::write(
            seed.join("payload/app/node_modules/@deepseek-ai/dsh/lib/bin.js"),
            b"dsh",
        )
        .expect("write fixture");
        let archive_path = seed.join("payload.tar.gz");
        let encoder = flate2::write::GzEncoder::new(
            fs::File::create(&archive_path).expect("create archive"),
            flate2::Compression::default(),
        );
        let mut archive = tar::Builder::new(encoder);
        archive
            .append_dir_all("payload", seed.join("payload"))
            .expect("append fixture");
        archive
            .into_inner()
            .expect("finish tar archive")
            .finish()
            .expect("finish gzip stream");
        let manifest = RuntimeManifest {
            schema_version: 1,
            runtime_id: "fixture".to_owned(),
            target: env!("DSH_DESKTOP_TARGET").to_owned(),
            node_version: "24.0.0".to_owned(),
            dsh_version: "0.1.0".to_owned(),
            pnpm_version: "11.0.0".to_owned(),
            definition_sha256: "fixture".to_owned(),
            node_executable: "payload/node/bin/node".to_owned(),
            dsh_entrypoint: "payload/app/node_modules/@deepseek-ai/dsh/lib/bin.js".to_owned(),
            pnpm_entrypoint: "payload/app/node_modules/pnpm/bin/pnpm.cjs".to_owned(),
            archive: RuntimeFile {
                path: "payload.tar.gz".to_owned(),
                sha256: file_sha256(&archive_path).expect("hash archive"),
            },
            files: vec![RuntimeFile {
                path: "payload/node/bin/node".to_owned(),
                sha256: file_sha256(&seed.join("payload/node/bin/node")).expect("hash fixture"),
            }],
        };
        fs::write(
            seed.join(MANIFEST_FILE),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");
        fs::remove_dir_all(seed.join("payload")).expect("remove unpacked fixture");
        let manager = RuntimeManager::for_test(DistributionVariant::Bundled, seed, installs);
        let runtime = manager.resolve_built_in().expect("install runtime");
        assert_eq!(runtime.dsh_version, "0.1.0");
        let _ = fs::remove_dir_all(root);
    }
}
