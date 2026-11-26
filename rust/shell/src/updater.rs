//! Desktop updater admission, staged-state transitions, and payload identity.
//! This is the Rust port of the upstream updater policy; transfer and install
//! orchestration plug into these pure transitions rather than owning policy.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateStatus {
    #[default]
    Disabled,
    Idle,
    Checking,
    Available,
    UpToDate,
    Downloading,
    Downloaded,
    Error,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateFlavor {
    #[default]
    Production,
    Beta,
    Remote,
    Canary,
    Cua,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub enabled: bool,
    pub status: UpdateStatus,
    pub current_version: String,
    pub host_arch: String,
    pub app_arch: String,
    pub running_under_arm64_translation: bool,
    pub available_version: Option<String>,
    pub downloaded_version: Option<String>,
    pub download_percent: Option<f64>,
    pub checked_at: Option<String>,
    pub message: Option<String>,
    pub error_context: Option<String>,
    pub can_retry: bool,
    pub install_failure_count: u32,
    pub flavor: UpdateFlavor,
    pub release_url: Option<String>,
}

impl UpdateState {
    pub fn initial(current_version: impl Into<String>, host_arch: impl Into<String>, app_arch: impl Into<String>, translated: bool, flavor: UpdateFlavor) -> Self {
        Self {
            enabled: false,
            status: UpdateStatus::Disabled,
            current_version: current_version.into(),
            host_arch: host_arch.into(),
            app_arch: app_arch.into(),
            running_under_arm64_translation: translated,
            available_version: None,
            downloaded_version: None,
            download_percent: None,
            checked_at: None,
            message: None,
            error_context: None,
            can_retry: false,
            install_failure_count: 0,
            flavor,
            release_url: None,
        }
    }

    pub fn check_start(mut self, checked_at: impl Into<String>) -> Self {
        self.status = UpdateStatus::Checking;
        self.checked_at = Some(checked_at.into());
        self.message = None;
        self.download_percent = None;
        self.error_context = None;
        self.can_retry = false;
        self
    }

    pub fn check_failure(mut self, message: impl Into<String>, checked_at: impl Into<String>) -> Self {
        let retained = self.downloaded_version.is_some();
        self.status = if retained { UpdateStatus::Downloaded } else { UpdateStatus::Error };
        self.message = Some(message.into());
        self.checked_at = Some(checked_at.into());
        self.download_percent = retained.then_some(100.0);
        self.error_context = Some("check".into());
        self.can_retry = true;
        self
    }

    pub fn update_available(mut self, version: impl Into<String>, checked_at: impl Into<String>) -> Self {
        let version = version.into();
        if self.downloaded_version.as_deref().is_some_and(|downloaded| !version_newer(downloaded, &version)) {
            self.status = UpdateStatus::Downloaded;
            self.checked_at = Some(checked_at.into());
            self.download_percent = Some(100.0);
            self.can_retry = true;
            return self;
        }
        let install_failure_count = if self.available_version.as_deref() == Some(version.as_str()) { self.install_failure_count } else { 0 };
        self.status = UpdateStatus::Available;
        self.available_version = Some(version);
        self.downloaded_version = None;
        self.download_percent = None;
        self.checked_at = Some(checked_at.into());
        self.message = None;
        self.error_context = None;
        self.can_retry = false;
        self.install_failure_count = install_failure_count;
        self
    }

    pub fn no_update(mut self, checked_at: impl Into<String>) -> Self {
        if self.downloaded_version.as_deref().is_some_and(|downloaded| {
            version_newer(&self.current_version, downloaded) && version_allowed_for_flavor(downloaded, self.flavor)
        }) {
            self.status = UpdateStatus::Downloaded;
            self.download_percent = Some(100.0);
            self.checked_at = Some(checked_at.into());
            self.message = None;
            self.error_context = None;
            self.can_retry = true;
            return self;
        }
        self.status = UpdateStatus::UpToDate;
        self.available_version = None;
        self.downloaded_version = None;
        self.download_percent = None;
        self.checked_at = Some(checked_at.into());
        self.message = None;
        self.error_context = None;
        self.can_retry = false;
        self.install_failure_count = 0;
        self
    }

    pub fn download_start(mut self) -> Self {
        self.status = UpdateStatus::Downloading;
        self.download_percent = None;
        self.message = None;
        self.error_context = None;
        self.can_retry = false;
        self
    }

    pub fn download_failure(mut self, message: impl Into<String>) -> Self {
        self.can_retry = self.available_version.is_some();
        self.status = if self.available_version.is_some() { UpdateStatus::Available } else { UpdateStatus::Error };
        self.downloaded_version = None;
        self.message = Some(message.into());
        self.download_percent = None;
        self.error_context = Some("download".into());
        self
    }

    pub fn download_progress(mut self, percent: f64) -> Self {
        self.status = UpdateStatus::Downloading;
        self.download_percent = Some(percent.clamp(0.0, 100.0));
        self.message = None;
        self.error_context = None;
        self.can_retry = false;
        self
    }

    pub fn download_complete(mut self, version: &str) -> Self {
        if self.status != UpdateStatus::Downloading || self.available_version.as_deref() != Some(version) {
            return self;
        }
        self.status = UpdateStatus::Downloaded;
        self.downloaded_version = Some(version.into());
        self.download_percent = Some(100.0);
        self.message = None;
        self.error_context = None;
        self.can_retry = true;
        self
    }

    pub fn install_failure(mut self, message: impl Into<String>) -> Self {
        self.status = UpdateStatus::Downloaded;
        self.message = Some(message.into());
        self.error_context = Some("install".into());
        self.can_retry = true;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Option<String>,
}

fn parse_version(input: &str) -> Option<Version> {
    let input = input.trim().trim_start_matches('v');
    let (core, prerelease) = input.split_once('-').map_or((input, None), |(core, pre)| (core, Some(pre.split('+').next().unwrap_or(pre).to_string())));
    let core = core.split('+').next().unwrap_or(core);
    let mut parts = core.split('.');
    let (major, minor, patch) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    if parts.next().is_some() { return None; }
    Some(Version { major, minor, patch, prerelease })
}

pub fn version_newer(current: &str, candidate: &str) -> bool {
    let (Some(current_version), Some(candidate_version)) = (parse_version(current), parse_version(candidate)) else {
        return candidate.trim() != current.trim();
    };
    for (left, right) in [
        (candidate_version.major, current_version.major),
        (candidate_version.minor, current_version.minor),
        (candidate_version.patch, current_version.patch),
    ] {
        if left != right { return left > right; }
    }
    match (&current_version.prerelease, &candidate_version.prerelease) {
        (Some(current_pre), Some(candidate_pre)) => compare_prerelease(candidate_pre, current_pre),
        (Some(_), None) => true,
        _ => false,
    }
}

fn compare_prerelease(candidate: &str, current: &str) -> bool {
    let candidate_parts: Vec<&str> = candidate.split('.').collect();
    let current_parts: Vec<&str> = current.split('.').collect();
    for index in 0..candidate_parts.len().max(current_parts.len()) {
        let (Some(candidate), Some(current)) = (candidate_parts.get(index), current_parts.get(index)) else {
            return candidate_parts.get(index).is_some();
        };
        let cn = candidate.parse::<u64>();
        let cur = current.parse::<u64>();
        match (cn, cur) {
            (Ok(c), Ok(r)) if c != r => return c > r,
            (Ok(_), Err(_)) => return false,
            (Err(_), Ok(_)) => return true,
            _ if candidate != current => return candidate > current,
            _ => {}
        }
    }
    false
}

pub fn version_allowed_for_flavor(candidate: &str, flavor: UpdateFlavor) -> bool {
    let Some(parsed) = parse_version(candidate) else {
        return !matches!(flavor, UpdateFlavor::Beta | UpdateFlavor::Remote | UpdateFlavor::Production);
    };
    match flavor {
        UpdateFlavor::Beta | UpdateFlavor::Remote => parsed.prerelease.as_deref().is_some_and(|pre| {
            let lane = if flavor == UpdateFlavor::Beta { "beta" } else { "remote" };
            pre == lane || pre.starts_with(&format!("{lane}."))
        }),
        UpdateFlavor::Production => parsed.prerelease.is_none(),
        UpdateFlavor::Canary | UpdateFlavor::Cua => true,
    }
}

pub fn can_check(state: &UpdateState) -> bool {
    !matches!(state.status, UpdateStatus::Checking | UpdateStatus::Downloading)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactIdentity {
    pub path: PathBuf,
    pub size: u64,
    pub sha512: String,
}

pub fn fingerprint_artifact(path: &Path) -> Result<ArtifactIdentity, String> {
    let path = path.canonicalize().map_err(|error| format!("downloaded update payload unavailable: {error}"))?;
    let metadata = std::fs::symlink_metadata(&path).map_err(|error| format!("downloaded update payload unavailable: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("downloaded update payload must be a regular, non-symlink file".into());
    }
    let before = std::fs::metadata(&path).map_err(|error| error.to_string())?;
    let mut file = File::open(&path).map_err(|error| format!("downloaded update payload unreadable: {error}"))?;
    let mut hash = Sha512::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| format!("downloaded update payload unreadable: {error}"))?;
        if count == 0 { break; }
        hash.update(&buffer[..count]);
    }
    let after = std::fs::metadata(&path).map_err(|error| error.to_string())?;
    if after.len() != before.len() || after.modified().ok() != before.modified().ok() {
        return Err("downloaded update payload changed while it was fingerprinted".into());
    }
    if after.len() == 0 {
        return Err("downloaded update payload is empty".into());
    }
    Ok(ArtifactIdentity { path, size: after.len(), sha512: format!("{:x}", hash.finalize()) })
}

pub fn verify_artifact_identity(expected: &ArtifactIdentity) -> bool {
    fingerprint_artifact(&expected.path).is_ok_and(|actual| actual.size == expected.size && actual.sha512.eq_ignore_ascii_case(&expected.sha512))
}

pub fn github_releases_page(owner: &str, repo: &str, host: Option<&str>, tag: Option<&str>) -> String {
    let host = host.filter(|host| !host.trim().is_empty()).unwrap_or("github.com");
    match tag.filter(|tag| !tag.trim().is_empty()) {
        Some(tag) => format!("https://{host}/{owner}/{repo}/releases/tag/{}", tag.trim()),
        None => format!("https://{host}/{owner}/{repo}/releases/latest"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn state() -> UpdateState {
        UpdateState::initial("0.9.0", "arm64", "arm64", false, UpdateFlavor::Production)
    }

    #[test]
    fn semantic_versions_advance_stable_and_prerelease_lanes() {
        assert!(version_newer("0.9.0", "0.9.1"));
        assert!(version_newer("0.9.1-beta.1", "0.9.1-beta.2"));
        assert!(version_newer("0.9.1-beta.2", "0.9.1"));
        assert!(!version_newer("0.9.1", "0.9.1-beta.3"));
        assert!(!version_newer("0.9.1", "0.9.1"));
    }

    #[test]
    fn flavor_lanes_fail_closed() {
        assert!(version_allowed_for_flavor("0.9.1", UpdateFlavor::Production));
        assert!(!version_allowed_for_flavor("0.9.1-beta.1", UpdateFlavor::Production));
        assert!(version_allowed_for_flavor("0.9.1-beta.1", UpdateFlavor::Beta));
        assert!(!version_allowed_for_flavor("0.9.1", UpdateFlavor::Beta));
    }

    #[test]
    fn downloaded_payload_survives_later_empty_or_failed_checks() {
        let staged = state()
            .check_start("now")
            .update_available("0.9.1", "now")
            .download_start()
            .download_progress(50.0)
            .download_complete("0.9.1");
        assert_eq!(staged.status, UpdateStatus::Downloaded);
        assert_eq!(staged.clone().no_update("later").status, UpdateStatus::Downloaded);
        assert_eq!(staged.check_failure("offline", "later").status, UpdateStatus::Downloaded);
    }

    #[test]
    fn fingerprint_detects_replacement() {
        let root = std::env::temp_dir().join(format!("synara-updater-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("payload.bin");
        File::create(&path).unwrap().write_all(b"payload").unwrap();
        let identity = fingerprint_artifact(&path).unwrap();
        assert!(verify_artifact_identity(&identity));
        File::create(&path).unwrap().write_all(b"changed").unwrap();
        assert!(!verify_artifact_identity(&identity));
        std::fs::remove_dir_all(root).unwrap();
    }
}
