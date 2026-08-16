use std::fs;
use std::path::Path;
use std::time::Duration;

use base64::Engine;
use reqwest::blocking::Client;
use ring::signature::{ED25519, UnparsedPublicKey};
use semver::{Version, VersionReq};
use serde::Deserialize;
use sha2::{Digest, Sha512};

use crate::error::{AppError, AppResult};

const MANIFEST_URL: &str = "https://leawind.github.io/dsh-desktop/compatibility.json";
const SIGNATURE_URL: &str = "https://leawind.github.io/dsh-desktop/compatibility.json.sig";
const PUBLIC_KEY_BASE64: &str = "R2Ot1Jp/FXC8s5u8UHlrI8Zlxq/CN/2yPDvRa6EydyM=";
const EMBEDDED_MANIFEST: &[u8] = include_bytes!("../../../runtime/compatibility.json");
const EMBEDDED_SIGNATURE: &str = include_str!("../../../runtime/compatibility.json.sig");

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityManifest {
    schema_version: u32,
    apps: std::collections::BTreeMap<String, AppCompatibility>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCompatibility {
    pub dsh: DshCompatibility,
    pub node: VersionCompatibility,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DshCompatibility {
    pub allowed_ranges: Vec<String>,
    pub recommended: String,
    #[serde(default)]
    pub rollback_compatible_ranges: Vec<String>,
    #[serde(default)]
    pub blocked: Vec<BlockedVersion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockedVersion {
    pub version: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCompatibility {
    pub allowed_ranges: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompatibleUpdate {
    pub version: String,
    pub automatic_rollback_supported: bool,
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

pub fn load_for_app(cache_directory: &Path, app_version: &str) -> AppResult<AppCompatibility> {
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
    compatibility: &AppCompatibility,
    current_dsh_version: &str,
    node_version: &str,
) -> AppResult<Option<CompatibleUpdate>> {
    let current = parse_version(current_dsh_version, "runtime.error.invalidRuntimeVersion")?;
    let candidate = parse_version(
        &compatibility.dsh.recommended,
        "runtime.error.invalidCompatibility",
    )?;
    let node = parse_version(node_version, "runtime.error.invalidRuntimeVersion")?;

    if !matches_any(&candidate, &compatibility.dsh.allowed_ranges)?
        || !matches_any(&node, &compatibility.node.allowed_ranges)?
    {
        return Err(AppError::new("runtime.error.invalidCompatibility"));
    }
    if compatibility
        .dsh
        .blocked
        .iter()
        .any(|blocked| blocked.version == candidate.to_string())
    {
        return Err(AppError::new("runtime.error.versionBlocked")
            .arg("version", candidate.to_string())
            .technical(
                compatibility
                    .dsh
                    .blocked
                    .iter()
                    .find(|blocked| blocked.version == candidate.to_string())
                    .map(|blocked| blocked.reason.clone())
                    .unwrap_or_default(),
            ));
    }
    if candidate <= current {
        return Ok(None);
    }
    if !same_automatic_update_line(&current, &candidate) {
        return Err(AppError::new("runtime.error.updateLineBlocked")
            .arg("current", current.to_string())
            .arg("candidate", candidate.to_string()));
    }

    Ok(Some(CompatibleUpdate {
        version: candidate.to_string(),
        automatic_rollback_supported: matches_any(
            &current,
            &compatibility.dsh.rollback_compatible_ranges,
        )?,
    }))
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
    let actual = Sha512::digest(&archive);
    if actual.as_slice() != expected.as_slice() {
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
    let signature = client
        .get(SIGNATURE_URL)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(|error| {
            AppError::new("runtime.error.compatibilityDownloadFailed").technical(error.to_string())
        })?;
    let parsed = parse_verified(&manifest, &signature)?;
    fs::create_dir_all(cache_directory).map_err(cache_error)?;
    fs::write(cache_directory.join("compatibility.json"), &manifest).map_err(cache_error)?;
    fs::write(cache_directory.join("compatibility.json.sig"), signature).map_err(cache_error)?;
    Ok(parsed)
}

fn cached_manifest(cache_directory: &Path) -> AppResult<CompatibilityManifest> {
    let manifest = fs::read(cache_directory.join("compatibility.json")).map_err(cache_error)?;
    let signature =
        fs::read_to_string(cache_directory.join("compatibility.json.sig")).map_err(cache_error)?;
    parse_verified(&manifest, &signature)
}

fn embedded_manifest() -> AppResult<CompatibilityManifest> {
    parse_verified(EMBEDDED_MANIFEST, EMBEDDED_SIGNATURE)
}

fn parse_verified(contents: &[u8], signature: &str) -> AppResult<CompatibilityManifest> {
    let public_key = base64::engine::general_purpose::STANDARD
        .decode(PUBLIC_KEY_BASE64)
        .map_err(|error| {
            AppError::new("runtime.error.invalidCompatibility").technical(error.to_string())
        })?;
    let signature = base64::engine::general_purpose::STANDARD
        .decode(signature.trim())
        .map_err(|error| {
            AppError::new("runtime.error.invalidCompatibility").technical(error.to_string())
        })?;
    UnparsedPublicKey::new(&ED25519, public_key)
        .verify(contents, &signature)
        .map_err(|_| AppError::new("runtime.error.compatibilitySignatureInvalid"))?;
    let manifest: CompatibilityManifest = serde_json::from_slice(contents).map_err(|error| {
        AppError::new("runtime.error.invalidCompatibility").technical(error.to_string())
    })?;
    if manifest.schema_version != 1 {
        return Err(AppError::new("runtime.error.invalidCompatibility"));
    }
    Ok(manifest)
}

fn parse_version(value: &str, error: &str) -> AppResult<Version> {
    Version::parse(value).map_err(|cause| AppError::new(error).technical(cause.to_string()))
}

fn matches_any(version: &Version, ranges: &[String]) -> AppResult<bool> {
    ranges.iter().try_fold(false, |matches, range| {
        let range = VersionReq::parse(range).map_err(|error| {
            AppError::new("runtime.error.invalidCompatibility").technical(error.to_string())
        })?;
        Ok(matches || range.matches(version))
    })
}

fn same_automatic_update_line(current: &Version, candidate: &Version) -> bool {
    if current.major > 0 {
        current.major == candidate.major
    } else {
        current.major == candidate.major && current.minor == candidate.minor
    }
}

fn cache_error(error: std::io::Error) -> AppError {
    AppError::new("runtime.error.compatibilityCacheFailed").technical(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_manifest_is_signed_and_selects_no_update_for_its_baseline() {
        let manifest = embedded_manifest().expect("embedded manifest must verify");
        let app = manifest.apps.get("0.1.0-rc.1").expect("app compatibility");
        assert!(
            select_update(app, "0.1.0-rc.6", "24.18.1")
                .expect("select update")
                .is_none()
        );
    }

    #[test]
    fn rejects_automatic_update_across_zero_minor_versions() {
        let compatibility = AppCompatibility {
            dsh: DshCompatibility {
                allowed_ranges: vec!["=0.2.0-rc.1".to_owned()],
                recommended: "0.2.0-rc.1".to_owned(),
                rollback_compatible_ranges: vec!["=0.1.0-rc.6".to_owned()],
                blocked: Vec::new(),
            },
            node: VersionCompatibility {
                allowed_ranges: vec![">=24.0.0, <25.0.0".to_owned()],
            },
        };
        assert_eq!(
            select_update(&compatibility, "0.1.0-rc.6", "24.18.1")
                .expect_err("minor update must be blocked")
                .code,
            "runtime.error.updateLineBlocked"
        );
    }
}
