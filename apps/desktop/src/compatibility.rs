use std::fs;
use std::path::Path;
use std::time::Duration;

use base64::Engine;
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha512};

use crate::error::{AppError, AppResult};

const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/Leawind/dsh-desktop/main/runtime/compatibility.json";
const EMBEDDED_MANIFEST: &[u8] = include_bytes!("../../../runtime/compatibility.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityManifest {
    schema_version: u32,
    apps: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NpmPackageMetadata {
    pub version: String,
    pub dist: NpmDistribution,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NpmDistribution {
    pub integrity: String,
    pub tarball: String,
}

pub fn load_for_app(cache_directory: &Path, app_version: &str) -> AppResult<String> {
    for source in [
        remote_manifest(cache_directory),
        cached_manifest(cache_directory),
        embedded_manifest(),
    ] {
        if let Ok(manifest) = source {
            if let Some(app) = manifest.apps.get(app_version) {
                return Ok(app.clone());
            }
        }
    }
    Err(AppError::new("runtime.error.compatibilityUnavailable").arg("appVersion", app_version))
}

pub fn select_update(
    candidate_version: &str,
    current_dsh_version: &str,
) -> AppResult<Option<String>> {
    let current = parse_version(current_dsh_version, "runtime.error.invalidRuntimeVersion")?;
    let candidate = parse_version(candidate_version, "runtime.error.invalidCompatibility")?;
    if candidate <= current {
        return Ok(None);
    }
    Ok(Some(candidate.to_string()))
}

pub fn fetch_dsh_package(version: &str) -> AppResult<NpmPackageMetadata> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| {
            AppError::new("runtime.error.packageDownloadFailed").technical(error.to_string())
        })?;
    let contents = client
        .get(format!(
            "https://registry.npmjs.org/%40deepseek-ai%2Fdsh/{version}"
        ))
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(|error| {
            AppError::new("runtime.error.packageDownloadFailed").technical(error.to_string())
        })?;
    let metadata = serde_json::from_str::<NpmPackageMetadata>(&contents).map_err(|error| {
        AppError::new("runtime.error.packageMetadataInvalid").technical(error.to_string())
    })?;
    if metadata.version != version
        || metadata.dist.integrity.is_empty()
        || metadata.dist.tarball.is_empty()
    {
        return Err(AppError::new("runtime.error.packageMetadataInvalid"));
    }
    Ok(metadata)
}

pub fn verify_package_integrity(package: &NpmPackageMetadata) -> AppResult<()> {
    let expected = package
        .dist
        .integrity
        .strip_prefix("sha512-")
        .ok_or_else(|| AppError::new("runtime.error.packageMetadataInvalid"))?;
    let expected = base64::engine::general_purpose::STANDARD
        .decode(expected)
        .map_err(|error| {
            AppError::new("runtime.error.packageMetadataInvalid").technical(error.to_string())
        })?;
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| {
            AppError::new("runtime.error.packageDownloadFailed").technical(error.to_string())
        })?;
    let archive = client
        .get(&package.dist.tarball)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.bytes())
        .map_err(|error| {
            AppError::new("runtime.error.packageDownloadFailed").technical(error.to_string())
        })?;
    verify_archive_integrity(&expected, &archive)
}

fn verify_archive_integrity(expected: &[u8], archive: &[u8]) -> AppResult<()> {
    let actual = Sha512::digest(archive);
    if actual.as_slice() != expected {
        return Err(AppError::new("runtime.error.packageIntegrityFailed"));
    }
    Ok(())
}

fn remote_manifest(cache_directory: &Path) -> AppResult<CompatibilityManifest> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| {
            AppError::new("runtime.error.compatibilityDownloadFailed").technical(error.to_string())
        })?;
    let manifest = client
        .get(MANIFEST_URL)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.bytes())
        .map_err(|error| {
            AppError::new("runtime.error.compatibilityDownloadFailed").technical(error.to_string())
        })?;
    let parsed = parse_manifest(&manifest)?;
    fs::create_dir_all(cache_directory).map_err(cache_error)?;
    fs::write(cache_directory.join("compatibility.json"), &manifest).map_err(cache_error)?;
    Ok(parsed)
}

fn cached_manifest(cache_directory: &Path) -> AppResult<CompatibilityManifest> {
    let manifest = fs::read(cache_directory.join("compatibility.json")).map_err(cache_error)?;
    parse_manifest(&manifest)
}

fn embedded_manifest() -> AppResult<CompatibilityManifest> {
    parse_manifest(EMBEDDED_MANIFEST)
}

fn parse_manifest(contents: &[u8]) -> AppResult<CompatibilityManifest> {
    let manifest: CompatibilityManifest = serde_json::from_slice(contents).map_err(|error| {
        AppError::new("runtime.error.invalidCompatibility").technical(error.to_string())
    })?;
    if manifest.schema_version != 2 {
        return Err(AppError::new("runtime.error.invalidCompatibility"));
    }
    Ok(manifest)
}

fn parse_version(value: &str, error: &str) -> AppResult<Version> {
    Version::parse(value).map_err(|cause| AppError::new(error).technical(cause.to_string()))
}

fn cache_error(error: std::io::Error) -> AppError {
    AppError::new("runtime.error.compatibilityCacheFailed").technical(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_manifest_selects_no_update_for_its_baseline() {
        let manifest = embedded_manifest().expect("embedded manifest must parse");
        let app = manifest
            .apps
            .get(env!("CARGO_PKG_VERSION"))
            .expect("app compatibility for the current app version");
        assert!(
            select_update(app, "0.1.0-rc.7")
                .expect("select update")
                .is_none()
        );
    }

    #[test]
    fn selects_a_newer_verified_version() {
        assert_eq!(
            select_update("0.2.0-rc.1", "0.1.0-rc.6").expect("select update"),
            Some("0.2.0-rc.1".to_owned())
        );
    }

    #[test]
    fn rejects_an_archive_with_a_different_sha512_digest() {
        let expected = Sha512::digest(b"verified archive");
        assert!(verify_archive_integrity(&expected, b"verified archive").is_ok());
        assert_eq!(
            verify_archive_integrity(&expected, b"modified archive")
                .expect_err("tampered archive must be rejected")
                .code,
            "runtime.error.packageIntegrityFailed"
        );
    }
}
