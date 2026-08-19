use std::fs;
use std::io::{Error, ErrorKind, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::model::{BundledRuntimeSnapshot, DistributionSnapshot, DistributionVariant};

const MANIFEST_FILE: &str = "manifest.json";
const ACTIVE_RUNTIME_FILE: &str = "active.json";
const PAYLOAD_DIRECTORY: &str = "payload";

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
    #[serde(default)]
    archive: Option<RuntimeFile>,
    files: Vec<RuntimeFile>,
    #[serde(default)]
    dsh_package_integrity: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct RuntimeFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone)]
pub struct InstalledRuntime {
    pub runtime_id: String,
    pub node_executable: PathBuf,
    pub dsh_entrypoint: PathBuf,
    pub pnpm_entrypoint: PathBuf,
    pub executable_path: Vec<PathBuf>,
    pub node_version: String,
    pub pnpm_version: String,
    pub dsh_version: String,
}

pub struct PreparedRuntime {
    pub runtime: InstalledRuntime,
    staging_directory: PathBuf,
}

impl PreparedRuntime {
    pub fn verification_home(&self) -> PathBuf {
        self.staging_directory.join("verification-home")
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ActiveRuntime {
    runtime_id: String,
}

#[derive(Clone)]
pub struct RuntimeManager {
    variant: DistributionVariant,
    seed_directory: PathBuf,
    install_root: PathBuf,
    seed_materialization_lock: Arc<Mutex<()>>,
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
    pub fn new(seed_directory: PathBuf, data_directory: PathBuf) -> Self {
        Self {
            variant: DistributionVariant::current(),
            seed_directory,
            install_root: data_directory.join("runtimes").join("dsh"),
            seed_materialization_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn distribution_variant(&self) -> DistributionVariant {
        self.variant
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
            seed_materialization_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn distribution_snapshot(&self) -> DistributionSnapshot {
        let built_in_runtime = (self.variant == DistributionVariant::Bundled)
            .then(|| self.snapshot_manifest())
            .flatten()
            .map(|(manifest, installed)| BundledRuntimeSnapshot {
                runtime_id: manifest.runtime_id,
                node_version: manifest.node_version,
                dsh_version: manifest.dsh_version,
                pnpm_version: manifest.pnpm_version,
                installed,
            });
        DistributionSnapshot {
            variant: self.variant,
            built_in_runtime,
        }
    }

    fn snapshot_manifest(&self) -> Option<(RuntimeManifest, bool)> {
        let seed = self.read_manifest(&self.seed_directory).ok()?;
        let active = self.read_active_runtime().and_then(|active| {
            let installation = self.install_root.join(active.runtime_id);
            let manifest = self.read_manifest(&installation).ok()?;
            self.validate_installation(&installation, &manifest)
                .ok()
                .map(|()| (manifest, true))
        });
        active.or_else(|| {
            let installed = self
                .validate_installation(&self.install_root.join(&seed.runtime_id), &seed)
                .is_ok();
            Some((seed, installed))
        })
    }

    pub fn resolve_built_in(&self) -> AppResult<InstalledRuntime> {
        if self.variant != DistributionVariant::Bundled {
            return Err(AppError::new("service.error.builtInUnavailable"));
        }
        self.ensure_runtime_seed()?;
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
        let installation = self.active_installation(&manifest)?;
        let installed_manifest = self.read_manifest(&installation)?;
        self.validate_installation(&installation, &installed_manifest)?;

        self.installed_runtime(installation, installed_manifest)
    }

    fn ensure_runtime_seed(&self) -> AppResult<()> {
        let _guard = self
            .seed_materialization_lock
            .lock()
            .map_err(|_| AppError::new("runtime.error.installFailed"))?;
        crate::embedded_resources::materialize_runtime_seed(&self.seed_directory)
    }

    pub fn prepare_update(
        &self,
        version: &str,
        package_integrity: &str,
    ) -> AppResult<PreparedRuntime> {
        if self.variant != DistributionVariant::Bundled {
            return Err(AppError::new("service.error.builtInUnavailable"));
        }
        let current = self.resolve_built_in()?;
        let staging = self
            .install_root
            .join(format!(".dsh-{version}.staging-{}", std::process::id()));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(runtime_install_error)?;
        }
        fs::create_dir_all(&staging).map_err(runtime_install_error)?;
        let result = self.prepare_update_at(&current, version, package_integrity, &staging);
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result.map(|runtime| PreparedRuntime {
            runtime,
            staging_directory: staging,
        })
    }

    pub fn commit_prepared(&self, prepared: PreparedRuntime) -> AppResult<String> {
        let runtime_id = prepared.runtime.runtime_id.clone();
        let installation = self.install_root.join(&runtime_id);
        if installation.exists() {
            let manifest = self.read_manifest(&installation)?;
            self.validate_installation(&installation, &manifest)?;
            fs::remove_dir_all(&prepared.staging_directory).map_err(runtime_install_error)?;
        } else {
            fs::rename(&prepared.staging_directory, &installation)
                .map_err(runtime_install_error)?;
        }
        self.set_active_runtime(&runtime_id)?;
        Ok(runtime_id)
    }

    pub fn discard_prepared(&self, prepared: PreparedRuntime) {
        let _ = fs::remove_dir_all(prepared.staging_directory);
    }

    pub fn set_active_runtime(&self, runtime_id: &str) -> AppResult<()> {
        let installation = self.install_root.join(runtime_id);
        let manifest = self.read_manifest(&installation)?;
        self.validate_installation(&installation, &manifest)?;
        fs::create_dir_all(&self.install_root).map_err(runtime_install_error)?;
        let temporary = self
            .install_root
            .join(format!(".{ACTIVE_RUNTIME_FILE}.tmp"));
        fs::write(
            &temporary,
            serde_json::to_vec(&ActiveRuntime {
                runtime_id: runtime_id.to_owned(),
            })
            .map_err(|error| {
                AppError::new("runtime.error.installFailed").technical(error.to_string())
            })?,
        )
        .map_err(runtime_install_error)?;
        fs::rename(temporary, self.install_root.join(ACTIVE_RUNTIME_FILE))
            .map_err(runtime_install_error)
    }

    pub fn cleanup(&self, retained_runtime_ids: &[String]) -> AppResult<()> {
        if !self.install_root.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.install_root).map_err(runtime_install_error)? {
            let entry = entry.map_err(runtime_install_error)?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.contains(".staging-") {
                fs::remove_dir_all(path).map_err(runtime_install_error)?;
            } else if path.is_dir()
                && name != "compatibility"
                && !retained_runtime_ids.contains(&name)
            {
                fs::remove_dir_all(path).map_err(runtime_install_error)?;
            }
        }
        Ok(())
    }

    pub fn compatibility_cache_directory(&self) -> PathBuf {
        self.install_root.join("compatibility")
    }

    fn active_installation(&self, seed_manifest: &RuntimeManifest) -> AppResult<PathBuf> {
        let active = self
            .read_active_runtime()
            .map(|active| self.install_root.join(active.runtime_id));
        if let Some(installation) = active {
            if let Ok(active_manifest) = self.read_manifest(&installation) {
                if self
                    .validate_installation(&installation, &active_manifest)
                    .is_ok()
                {
                    return Ok(installation);
                }
            }
        }
        let installation = self.install_root.join(&seed_manifest.runtime_id);
        if self
            .validate_installation(&installation, seed_manifest)
            .is_err()
        {
            self.install(seed_manifest, &installation)?;
        }
        self.validate_installation(&installation, seed_manifest)?;
        self.set_active_runtime(&seed_manifest.runtime_id)?;
        Ok(installation)
    }

    fn read_active_runtime(&self) -> Option<ActiveRuntime> {
        serde_json::from_slice(&fs::read(self.install_root.join(ACTIVE_RUNTIME_FILE)).ok()?).ok()
    }

    fn installed_runtime(
        &self,
        installation: PathBuf,
        manifest: RuntimeManifest,
    ) -> AppResult<InstalledRuntime> {
        let node_executable = resolve_manifest_path(&installation, &manifest.node_executable)?;
        let dsh_entrypoint = resolve_manifest_path(&installation, &manifest.dsh_entrypoint)?;
        let pnpm_entrypoint = resolve_manifest_path(&installation, &manifest.pnpm_entrypoint)?;
        let app_directory = dsh_entrypoint
            .ancestors()
            .find(|path| path.file_name().is_some_and(|name| name == "app"))
            .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))?;
        let node_directory = node_executable
            .parent()
            .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))?;
        let executable_path = vec![app_directory.join("bin"), node_directory.to_owned()];
        Ok(InstalledRuntime {
            runtime_id: manifest.runtime_id,
            node_executable,
            dsh_entrypoint,
            pnpm_entrypoint,
            executable_path,
            node_version: manifest.node_version,
            pnpm_version: manifest.pnpm_version,
            dsh_version: manifest.dsh_version,
        })
    }

    fn prepare_update_at(
        &self,
        current: &InstalledRuntime,
        version: &str,
        package_integrity: &str,
        staging: &Path,
    ) -> AppResult<InstalledRuntime> {
        let payload = staging.join(PAYLOAD_DIRECTORY);
        fs::create_dir_all(&payload).map_err(runtime_install_error)?;
        copy_directory(
            current
                .node_executable
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))?,
            &payload.join("node"),
        )?;
        let app = payload.join("app");
        fs::create_dir_all(&app).map_err(runtime_install_error)?;
        fs::write(
            app.join("package.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "name": "dsh-desktop-runtime",
                "private": true,
                "dependencies": {
                    "@deepseek-ai/dsh": version,
                    "pnpm": current.pnpm_version,
                }
            }))
            .map_err(|error| {
                AppError::new("runtime.error.installFailed").technical(error.to_string())
            })?,
        )
        .map_err(runtime_install_error)?;
        run_pnpm_install(current, &app)?;
        verify_locked_integrity(&app, package_integrity)?;

        let dsh_entrypoint = "payload/app/node_modules/@deepseek-ai/dsh/lib/bin.js".to_owned();
        let pnpm_entrypoint = "payload/app/node_modules/pnpm/bin/pnpm.cjs".to_owned();
        let node_executable = relative_node_executable(&current.node_executable)?;
        let files = [
            node_executable.clone(),
            dsh_entrypoint.clone(),
            pnpm_entrypoint.clone(),
        ]
        .into_iter()
        .map(|path| {
            Ok(RuntimeFile {
                sha256: file_sha256(&resolve_manifest_path(staging, &path)?).map_err(|error| {
                    AppError::new("runtime.error.installFailed").technical(error.to_string())
                })?,
                path,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
        let manifest = RuntimeManifest {
            schema_version: 1,
            runtime_id: format!(
                "dsh-{version}-node-{}-{}",
                current.node_version,
                env!("DSH_DESKTOP_TARGET")
            ),
            target: env!("DSH_DESKTOP_TARGET").to_owned(),
            node_version: current.node_version.clone(),
            dsh_version: version.to_owned(),
            pnpm_version: current.pnpm_version.clone(),
            definition_sha256: "runtime-update".to_owned(),
            node_executable,
            dsh_entrypoint,
            pnpm_entrypoint,
            archive: None,
            files,
            dsh_package_integrity: Some(package_integrity.to_owned()),
        };
        fs::write(
            staging.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).map_err(|error| {
                AppError::new("runtime.error.installFailed").technical(error.to_string())
            })?,
        )
        .map_err(runtime_install_error)?;
        self.validate_installation(staging, &manifest)?;
        self.installed_runtime(staging.to_owned(), manifest)
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
        let archive = manifest
            .archive
            .as_ref()
            .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))?;
        let archive = resolve_manifest_path(&self.seed_directory, &archive.path)?;
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
        let archive = actual
            .archive
            .as_ref()
            .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))?;
        let archive_path = resolve_manifest_path(&self.seed_directory, &archive.path)?;
        let hash = file_sha256(&archive_path).map_err(|error| {
            AppError::new("runtime.error.integrityFailed")
                .arg("path", archive.path.clone())
                .technical(error.to_string())
        })?;
        if hash != archive.sha256 {
            return Err(
                AppError::new("runtime.error.integrityFailed").arg("path", archive.path.clone())
            );
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

fn run_pnpm_install(current: &InstalledRuntime, app_directory: &Path) -> AppResult<()> {
    let mut command = Command::new(&current.node_executable);
    command
        .arg(&current.pnpm_entrypoint)
        .args(["install", "--prod", "--config.node-linker=hoisted", "--dir"])
        .arg(app_directory)
        .stdin(Stdio::null());
    let mut paths = current.executable_path.clone();
    if let Some(inherited) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&inherited));
    }
    if let Ok(path) = std::env::join_paths(paths) {
        command.env("PATH", path);
    }
    let output = command.output().map_err(|error| {
        AppError::new("runtime.error.installFailed").technical(error.to_string())
    })?;
    if output.status.success() {
        return Ok(());
    }
    let diagnostics = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Err(AppError::new("runtime.error.installFailed").technical(diagnostics))
}

fn verify_locked_integrity(app_directory: &Path, expected_integrity: &str) -> AppResult<()> {
    let lockfile = fs::read_to_string(app_directory.join("pnpm-lock.yaml")).map_err(|error| {
        AppError::new("runtime.error.packageIntegrityFailed").technical(error.to_string())
    })?;
    if lockfile.contains(expected_integrity) {
        Ok(())
    } else {
        Err(AppError::new("runtime.error.packageIntegrityFailed"))
    }
}

fn relative_node_executable(node_executable: &Path) -> AppResult<String> {
    let components = node_executable.components().collect::<Vec<_>>();
    let payload_index = components
        .iter()
        .position(|component| component.as_os_str() == PAYLOAD_DIRECTORY)
        .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))?;
    let relative = components[payload_index..]
        .iter()
        .map(|component| component.as_os_str())
        .collect::<PathBuf>();
    relative
        .to_str()
        .map(|path| path.replace('\\', "/"))
        .ok_or_else(|| AppError::new("runtime.error.invalidManifest"))
}

fn copy_directory(source: &Path, destination: &Path) -> AppResult<()> {
    fs::create_dir_all(destination).map_err(runtime_install_error)?;
    for entry in fs::read_dir(source).map_err(runtime_install_error)? {
        let entry = entry.map_err(runtime_install_error)?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).map_err(runtime_install_error)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path).map_err(runtime_install_error)?;
        }
    }
    Ok(())
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
            archive: Some(RuntimeFile {
                path: "payload.tar.gz".to_owned(),
                sha256: file_sha256(&archive_path).expect("hash archive"),
            }),
            files: vec![RuntimeFile {
                path: "payload/node/bin/node".to_owned(),
                sha256: file_sha256(&seed.join("payload/node/bin/node")).expect("hash fixture"),
            }],
            dsh_package_integrity: None,
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
