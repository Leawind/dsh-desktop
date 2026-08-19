use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::model::{AppUpdateCandidate, AppUpdateSnapshot, DistributionVariant};

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/Leawind/dsh-desktop/releases/latest";
const RELEASE_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const ASSET_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const REPLACE_RETRY_DELAY: Duration = Duration::from_millis(250);
const REPLACE_RETRIES: usize = 120;
const UPDATE_DIRECTORY: &str = "updates";
const APPLIED_UPDATE_FILE: &str = "applied-update.json";

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    body: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    state: String,
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Debug, Clone)]
struct UpdateAsset {
    version: Version,
    notes: Option<String>,
    url: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppliedUpdate {
    version: String,
    target: PathBuf,
    backup: PathBuf,
}

pub fn check(variant: DistributionVariant) -> AppResult<AppUpdateSnapshot> {
    let current = current_version()?;
    let release = latest_release()?;
    let candidate = select_asset(&release, variant, &current)?;
    Ok(AppUpdateSnapshot {
        current_version: current.to_string(),
        candidate: candidate.map(|asset| AppUpdateCandidate {
            version: asset.version.to_string(),
            notes: asset.notes,
        }),
    })
}

pub fn install(data_directory: &Path, variant: DistributionVariant) -> AppResult<String> {
    let current = current_version()?;
    let release = latest_release()?;
    let asset = select_asset(&release, variant, &current)?
        .ok_or_else(|| AppError::new("appUpdate.error.alreadyUpToDate"))?;
    let target = std::env::current_exe().map_err(|error| {
        AppError::new("appUpdate.error.installUnavailable").technical(error.to_string())
    })?;
    ensure_target_directory_writable(&target)?;

    let update_directory = update_directory(data_directory);
    fs::create_dir_all(&update_directory).map_err(update_install_error)?;
    let staged = download_asset(&asset, &update_directory)?;
    let helper = copy_update_helper(&target, &update_directory)?;
    let current_directory = std::env::current_dir().map_err(|error| {
        AppError::new("appUpdate.error.installUnavailable").technical(error.to_string())
    })?;

    Command::new(&helper)
        .arg("--apply-update")
        .arg("--source")
        .arg(&staged)
        .arg("--target")
        .arg(&target)
        .arg("--current-directory")
        .arg(&current_directory)
        .arg("--data-directory")
        .arg(data_directory)
        .arg("--version")
        .arg(asset.version.to_string())
        .spawn()
        .map_err(|error| {
            AppError::new("appUpdate.error.installFailed").technical(error.to_string())
        })?;

    Ok(asset.version.to_string())
}

pub fn run_helper_if_requested() -> bool {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    if arguments.next().as_deref() != Some("--apply-update".as_ref()) {
        return false;
    }

    let result = parse_helper_arguments(arguments).and_then(|arguments| apply_update(arguments));
    if let Err(error) = result {
        eprintln!("failed to apply DSH Desktop update: {error}");
    }
    true
}

pub fn confirm_applied_update(data_directory: &Path) {
    let marker = update_directory(data_directory).join(APPLIED_UPDATE_FILE);
    let Ok(contents) = fs::read(&marker) else {
        return;
    };
    let Ok(update) = serde_json::from_slice::<AppliedUpdate>(&contents) else {
        return;
    };
    if update.version != env!("CARGO_PKG_VERSION") {
        return;
    }
    let Ok(current) = std::env::current_exe().and_then(fs::canonicalize) else {
        return;
    };
    let Ok(target) = fs::canonicalize(&update.target) else {
        return;
    };
    if current != target {
        return;
    }
    let _ = fs::remove_file(update.backup);
    let _ = fs::remove_file(marker);
}

fn latest_release() -> AppResult<GithubRelease> {
    let client = Client::builder()
        .timeout(RELEASE_REQUEST_TIMEOUT)
        .build()
        .map_err(release_error)?;
    client
        .get(LATEST_RELEASE_URL)
        .header("User-Agent", "dsh-desktop")
        .header("Accept", "application/vnd.github+json")
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(release_error)
        .and_then(|contents| {
            serde_json::from_str(&contents).map_err(|error| {
                AppError::new("appUpdate.error.invalidRelease").technical(error.to_string())
            })
        })
}

fn select_asset(
    release: &GithubRelease,
    variant: DistributionVariant,
    current: &Version,
) -> AppResult<Option<UpdateAsset>> {
    if release.draft || release.prerelease {
        return Err(AppError::new("appUpdate.error.invalidRelease"));
    }
    let version = parse_tag_version(&release.tag_name)?;
    if version <= *current {
        return Ok(None);
    }
    let name = asset_name(variant)?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == name && asset.state == "uploaded")
        .ok_or_else(|| AppError::new("appUpdate.error.assetUnavailable").arg("asset", name))?;
    let sha256 = parse_sha256(asset.digest.as_deref())?;
    if asset.size == 0 || asset.browser_download_url.is_empty() {
        return Err(AppError::new("appUpdate.error.invalidRelease"));
    }
    let notes = (!release.body.trim().is_empty()).then(|| release.body.clone());
    Ok(Some(UpdateAsset {
        version,
        notes,
        url: asset.browser_download_url.clone(),
        size: asset.size,
        sha256,
    }))
}

fn current_version() -> AppResult<Version> {
    Version::parse(env!("CARGO_PKG_VERSION")).map_err(|error| {
        AppError::new("appUpdate.error.invalidRelease").technical(error.to_string())
    })
}

fn parse_tag_version(tag: &str) -> AppResult<Version> {
    let version = tag
        .strip_prefix('v')
        .ok_or_else(|| AppError::new("appUpdate.error.invalidRelease"))?;
    Version::parse(version).map_err(|error| {
        AppError::new("appUpdate.error.invalidRelease").technical(error.to_string())
    })
}

fn asset_name(variant: DistributionVariant) -> AppResult<String> {
    let variant = match variant {
        DistributionVariant::Bundled => "bundled",
        DistributionVariant::Slim => "slim",
    };
    let platform = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => return Err(AppError::new("appUpdate.error.unsupportedTarget")),
    };
    let architecture = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return Err(AppError::new("appUpdate.error.unsupportedTarget")),
    };
    let extension = (platform == "windows")
        .then_some(".exe")
        .unwrap_or_default();
    Ok(format!(
        "dsh-desktop-{variant}-{platform}-{architecture}{extension}"
    ))
}

fn parse_sha256(value: Option<&str>) -> AppResult<String> {
    let value = value
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| AppError::new("appUpdate.error.invalidRelease"))?;
    Ok(value.to_ascii_lowercase())
}

fn download_asset(asset: &UpdateAsset, update_directory: &Path) -> AppResult<PathBuf> {
    let client = Client::builder()
        .timeout(ASSET_DOWNLOAD_TIMEOUT)
        .build()
        .map_err(download_error)?;
    let mut response = client
        .get(&asset.url)
        .header("User-Agent", "dsh-desktop")
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(download_error)?;
    let temporary = update_directory.join(format!("{}.part", update_file_stem(&asset.version)));
    let staged = update_directory.join(update_file_stem(&asset.version));
    let mut file = fs::File::create(&temporary).map_err(update_install_error)?;
    let mut digest = Sha256::new();
    let mut downloaded = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = response.read(&mut buffer).map_err(download_error)?;
        if count == 0 {
            break;
        }
        downloaded = downloaded.saturating_add(count as u64);
        if downloaded > asset.size {
            let _ = fs::remove_file(&temporary);
            return Err(AppError::new("appUpdate.error.integrityFailed"));
        }
        file.write_all(&buffer[..count])
            .map_err(update_install_error)?;
        digest.update(&buffer[..count]);
    }
    file.sync_all().map_err(update_install_error)?;
    let actual = format!("{:x}", digest.finalize());
    if downloaded != asset.size || actual != asset.sha256 {
        let _ = fs::remove_file(&temporary);
        return Err(AppError::new("appUpdate.error.integrityFailed"));
    }
    if staged.exists() {
        fs::remove_file(&staged).map_err(update_install_error)?;
    }
    fs::rename(temporary, &staged).map_err(update_install_error)?;
    Ok(staged)
}

fn ensure_target_directory_writable(target: &Path) -> AppResult<()> {
    let directory = target
        .parent()
        .ok_or_else(|| AppError::new("appUpdate.error.installUnavailable"))?;
    let probe = directory.join(format!(".dsh-desktop-update-{}", unique_suffix()));
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|error| {
            AppError::new("appUpdate.error.installUnavailable").technical(error.to_string())
        })?;
    fs::remove_file(probe).map_err(update_install_error)
}

fn copy_update_helper(target: &Path, update_directory: &Path) -> AppResult<PathBuf> {
    let extension = target
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    let suffix = if extension.is_empty() {
        String::new()
    } else {
        format!(".{extension}")
    };
    let helper = update_directory.join(format!("helper-{}{}", unique_suffix(), suffix));
    fs::copy(target, &helper).map_err(update_install_error)?;
    Ok(helper)
}

fn update_directory(data_directory: &Path) -> PathBuf {
    data_directory.join(UPDATE_DIRECTORY)
}

fn update_file_stem(version: &Version) -> String {
    let extension = if cfg!(windows) { ".exe" } else { "" };
    format!("dsh-desktop-{version}-{}{extension}", unique_suffix())
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        ^ u128::from(std::process::id())
}

struct HelperArguments {
    source: PathBuf,
    target: PathBuf,
    current_directory: PathBuf,
    data_directory: PathBuf,
    version: String,
}

fn parse_helper_arguments(
    mut arguments: impl Iterator<Item = std::ffi::OsString>,
) -> AppResult<HelperArguments> {
    let mut source = None;
    let mut target = None;
    let mut current_directory = None;
    let mut data_directory = None;
    let mut version = None;
    while let Some(argument) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| AppError::new("appUpdate.error.installFailed"))?;
        match argument.to_string_lossy().as_ref() {
            "--source" => source = Some(PathBuf::from(value)),
            "--target" => target = Some(PathBuf::from(value)),
            "--current-directory" => current_directory = Some(PathBuf::from(value)),
            "--data-directory" => data_directory = Some(PathBuf::from(value)),
            "--version" => version = Some(value.to_string_lossy().into_owned()),
            _ => return Err(AppError::new("appUpdate.error.installFailed")),
        }
    }
    Ok(HelperArguments {
        source: source.ok_or_else(|| AppError::new("appUpdate.error.installFailed"))?,
        target: target.ok_or_else(|| AppError::new("appUpdate.error.installFailed"))?,
        current_directory: current_directory
            .ok_or_else(|| AppError::new("appUpdate.error.installFailed"))?,
        data_directory: data_directory
            .ok_or_else(|| AppError::new("appUpdate.error.installFailed"))?,
        version: version.ok_or_else(|| AppError::new("appUpdate.error.installFailed"))?,
    })
}

fn apply_update(arguments: HelperArguments) -> AppResult<()> {
    let backup = replace_target(&arguments.source, &arguments.target)?;

    let marker = update_directory(&arguments.data_directory).join(APPLIED_UPDATE_FILE);
    let marker_data = serde_json::to_vec(&AppliedUpdate {
        version: arguments.version,
        target: arguments.target.clone(),
        backup: backup.clone(),
    })
    .map_err(|error| AppError::new("appUpdate.error.installFailed").technical(error.to_string()))?;
    fs::write(&marker, marker_data).map_err(update_install_error)?;

    if let Err(error) = Command::new(&arguments.target)
        .current_dir(&arguments.current_directory)
        .spawn()
    {
        let _ = fs::remove_file(&arguments.target);
        let _ = fs::rename(&backup, &arguments.target);
        let _ = fs::remove_file(&marker);
        return Err(AppError::new("appUpdate.error.installFailed").technical(error.to_string()));
    }
    let _ = fs::remove_file(arguments.source);
    Ok(())
}

fn replace_target(source: &Path, target: &Path) -> AppResult<PathBuf> {
    let target_parent = target
        .parent()
        .ok_or_else(|| AppError::new("appUpdate.error.installFailed"))?;
    let target_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::new("appUpdate.error.installFailed"))?;
    let suffix = unique_suffix();
    let incoming = target_parent.join(format!(".{target_name}.incoming-{suffix}"));
    let backup = target_parent.join(format!(".{target_name}.previous-{suffix}"));
    let permissions = fs::metadata(target)
        .map_err(update_install_error)?
        .permissions();
    fs::copy(source, &incoming).map_err(update_install_error)?;
    fs::set_permissions(&incoming, permissions).map_err(update_install_error)?;
    fs::File::open(&incoming)
        .and_then(|file| file.sync_all())
        .map_err(update_install_error)?;

    for _ in 0..REPLACE_RETRIES {
        match fs::rename(target, &backup) {
            Ok(()) => {
                fs::rename(&incoming, target).map_err(|error| {
                    let _ = fs::rename(&backup, target);
                    AppError::new("appUpdate.error.installFailed").technical(error.to_string())
                })?;
                return Ok(backup);
            }
            Err(_) => thread::sleep(REPLACE_RETRY_DELAY),
        }
    }
    let _ = fs::remove_file(&incoming);
    Err(AppError::new("appUpdate.error.installFailed"))
}

fn release_error(error: reqwest::Error) -> AppError {
    AppError::new("appUpdate.error.releaseUnavailable").technical(error.to_string())
}

fn download_error(error: impl std::fmt::Display) -> AppError {
    AppError::new("appUpdate.error.downloadFailed").technical(error.to_string())
}

fn update_install_error(error: impl std::fmt::Display) -> AppError {
    AppError::new("appUpdate.error.installFailed").technical(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, asset: GithubReleaseAsset) -> GithubRelease {
        GithubRelease {
            tag_name: tag.to_owned(),
            body: "Release notes".to_owned(),
            draft: false,
            prerelease: false,
            assets: vec![asset],
        }
    }

    fn asset(name: &str) -> GithubReleaseAsset {
        GithubReleaseAsset {
            name: name.to_owned(),
            state: "uploaded".to_owned(),
            size: 1,
            browser_download_url: "https://example.invalid/dsh-desktop".to_owned(),
            digest: Some(format!("sha256:{}", "a".repeat(64))),
        }
    }

    #[test]
    fn ignores_the_latest_release_when_it_is_not_newer() {
        let current = Version::parse("0.2.0").expect("version");
        let release = release("v0.2.0", asset("unused"));
        assert!(
            select_asset(&release, DistributionVariant::Slim, &current)
                .expect("current release is valid")
                .is_none()
        );
    }

    #[test]
    fn selects_only_the_current_distribution_asset() {
        let current = Version::parse("0.2.0").expect("version");
        let name = asset_name(DistributionVariant::Slim).expect("supported target");
        let selected = select_asset(
            &release("v0.2.1", asset(&name)),
            DistributionVariant::Slim,
            &current,
        )
        .expect("update is valid")
        .expect("newer update is selected");
        assert_eq!(selected.version, Version::parse("0.2.1").expect("version"));
        assert_eq!(selected.notes.as_deref(), Some("Release notes"));
    }

    #[test]
    fn rejects_an_asset_without_a_sha256_digest() {
        let current = Version::parse("0.2.0").expect("version");
        let name = asset_name(DistributionVariant::Slim).expect("supported target");
        let mut asset = asset(&name);
        asset.digest = None;
        assert_eq!(
            select_asset(
                &release("v0.2.1", asset),
                DistributionVariant::Slim,
                &current
            )
            .expect_err("digest is required")
            .code,
            "appUpdate.error.invalidRelease"
        );
    }

    #[test]
    fn replaces_a_staged_file_and_preserves_the_previous_binary() {
        let directory =
            std::env::temp_dir().join(format!("dsh-desktop-app-update-{}", unique_suffix()));
        fs::create_dir_all(&directory).expect("create update directory");
        let target = directory.join("dsh-desktop");
        let source = directory.join("staged");
        fs::write(&target, b"old executable").expect("write target");
        fs::write(&source, b"new executable").expect("write source");

        let backup = replace_target(&source, &target).expect("replace target");
        assert_eq!(fs::read(&target).expect("read target"), b"new executable");
        assert_eq!(fs::read(backup).expect("read backup"), b"old executable");

        let _ = fs::remove_dir_all(directory);
    }
}
