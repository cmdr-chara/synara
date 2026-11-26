//! Desktop updater admission, staged-state transitions, and payload identity.
//! This is the Rust port of the upstream updater policy; transfer and install
//! orchestration plug into these pure transitions rather than owning policy.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use sha2::{Digest, Sha512};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

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

pub const FOREGROUND_RECHECK_MIN_BACKGROUND_MS: u64 = 30_000;
pub const FOREGROUND_RECHECK_MIN_INTERVAL_MS: i64 = 5 * 60 * 1000;

pub fn foreground_recheck_needed(
    checked_at: Option<&str>,
    backgrounded_for_ms: u64,
    now: i64,
) -> bool {
    if backgrounded_for_ms < FOREGROUND_RECHECK_MIN_BACKGROUND_MS {
        return false;
    }
    match checked_at {
        None => true,
        Some(value) => match iso_ms(value) {
            None => true,
            Some(checked) => now - checked >= FOREGROUND_RECHECK_MIN_INTERVAL_MS,
        },
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopUpdateButtonAction {
    Check,
    Download,
    Install,
    None,
}

pub fn resolve_desktop_update_button_action(state: &UpdateState) -> DesktopUpdateButtonAction {
    if state.status == UpdateStatus::Error && state.error_context.as_deref() == Some("install") && !state.can_retry {
        return DesktopUpdateButtonAction::None;
    }
    if matches!(state.status, UpdateStatus::Idle | UpdateStatus::Checking | UpdateStatus::UpToDate)
        || (state.status == UpdateStatus::Error && state.error_context.as_deref() == Some("check"))
    {
        return DesktopUpdateButtonAction::Check;
    }
    if state.status == UpdateStatus::Available { return DesktopUpdateButtonAction::Download; }
    if state.status == UpdateStatus::Downloaded { return DesktopUpdateButtonAction::Install; }
    if state.status == UpdateStatus::Error {
        if state.error_context.as_deref() == Some("install") && state.downloaded_version.is_none() && state.available_version.is_some() {
            return DesktopUpdateButtonAction::Download;
        }
        if state.downloaded_version.is_some() && matches!(state.error_context.as_deref(), Some("install") | None) {
            return DesktopUpdateButtonAction::Install;
        }
        if state.available_version.is_some() && matches!(state.error_context.as_deref(), Some("download") | None) {
            return DesktopUpdateButtonAction::Download;
        }
    }
    DesktopUpdateButtonAction::None
}

pub fn should_show_desktop_update_button(state: Option<&UpdateState>) -> bool {
    let Some(state) = state.filter(|state| state.enabled) else { return false };
    let action = resolve_desktop_update_button_action(state);
    matches!(state.status, UpdateStatus::Available | UpdateStatus::Downloading | UpdateStatus::Downloaded)
        || (state.status == UpdateStatus::Error
            && state.error_context.as_deref() != Some("check")
            && (action != DesktopUpdateButtonAction::None
                || (state.error_context.as_deref() == Some("install") && state.downloaded_version.is_some())))
}

pub fn desktop_update_button_disabled(state: Option<&UpdateState>) -> bool {
    let Some(state) = state else { return true };
    (state.status == UpdateStatus::Error && state.error_context.as_deref() == Some("install") && !state.can_retry)
        || matches!(state.status, UpdateStatus::Downloading | UpdateStatus::Checking)
        || (state.status == UpdateStatus::Available && state.error_context.as_deref() != Some("download"))
}

pub fn desktop_update_download_percent(state: &UpdateState) -> Option<u32> {
    if state.status != UpdateStatus::Downloading { return None; }
    state.download_percent.filter(|percent| percent.is_finite()).map(|percent| percent.clamp(0.0, 100.0).floor() as u32)
}

pub fn desktop_update_button_label(state: &UpdateState, installing: bool) -> String {
    if installing { return "Applying update...".into(); }
    if state.status == UpdateStatus::Error && state.error_context.as_deref() == Some("install") && !state.can_retry {
        return state.message.clone().unwrap_or_else(|| "Fully quit and reopen Synara before retrying the update.".into());
    }
    match state.status {
        UpdateStatus::Idle => "Check for updates".into(),
        UpdateStatus::Checking => "Checking for updates...".into(),
        UpdateStatus::UpToDate => format!("You're up to date on {}. Click to check again.", state.current_version),
        UpdateStatus::Available => format!("Preparing update {}", state.available_version.clone().unwrap_or_default()).trim().to_string(),
        UpdateStatus::Downloading => {
            let progress = state.download_percent.map(|percent| format!(" ({}%)", percent.floor() as u32)).unwrap_or_default();
            format!("Preparing update{progress}")
        }
        UpdateStatus::Downloaded => format!(
            "Update {} is ready. Click to restart and install.",
            state.downloaded_version.clone().or_else(|| state.available_version.clone()).unwrap_or_else(|| "ready".into())
        ),
        UpdateStatus::Error => {
            if state.error_context.as_deref() == Some("install") && state.downloaded_version.is_none() && state.available_version.is_some() {
                return format!("Synara restarted, but update {} was not installed. Click to try again.", state.available_version.clone().unwrap_or_default());
            }
            if state.error_context.as_deref() == Some("download") && state.available_version.is_some() {
                return format!("Could not prepare update {}. Click to retry.", state.available_version.clone().unwrap_or_default());
            }
            if state.error_context.as_deref() == Some("install") {
                if let Some(version) = state.downloaded_version.clone().or_else(|| state.available_version.clone()) {
                    return format!("Could not install update {version}. Click to retry.");
                }
            }
            if state.error_context.as_deref() == Some("check") {
                return state.message.clone().map(|message| format!("{message}. Click to check again.")).unwrap_or_else(|| "Update check failed. Click to try again.".into());
            }
            state.message.clone().unwrap_or_else(|| "Update failed".into())
        }
        UpdateStatus::Disabled => "Updates are unavailable".into(),
    }
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


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallMarker {
    pub schema_version: u32,
    pub attempt_id: String,
    pub from_version: String,
    pub to_version: String,
    pub requested_at: String,
    pub handoff_at: Option<String>,
    pub phase: String,
    pub consecutive_failures: u32,
    pub last_failure_at: Option<String>,
    pub artifact: ArtifactIdentity,
}

impl InstallMarker {
    pub fn create(from_version: impl Into<String>, to_version: impl Into<String>, requested_at: impl Into<String>, artifact: ArtifactIdentity) -> Self {
        Self {
            schema_version: 2,
            attempt_id: uuid::Uuid::new_v4().to_string(),
            from_version: from_version.into(),
            to_version: to_version.into(),
            requested_at: requested_at.into(),
            handoff_at: None,
            phase: "requested".into(),
            consecutive_failures: 0,
            last_failure_at: None,
            artifact,
        }
    }

    pub fn valid(&self) -> bool {
        self.schema_version == 2
            && !self.attempt_id.trim().is_empty()
            && !self.from_version.trim().is_empty()
            && !self.to_version.trim().is_empty()
            && iso_ms(&self.requested_at).is_some()
            && self.handoff_at.as_deref().is_none_or(|value| iso_ms(value).is_some())
            && matches!(self.phase.as_str(), "requested" | "handoff" | "failed")
            && self.artifact.size > 0
            && self.artifact.sha512.len() == 128
            && self.artifact.sha512.bytes().all(|byte| byte.is_ascii_hexdigit())
    }
}

fn iso_ms(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value).ok().map(|time| time.timestamp_millis())
}

pub fn read_install_marker(path: &Path) -> Result<Option<InstallMarker>, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let marker: InstallMarker = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    if !marker.valid() {
        return Err("marker does not match schema version 2".into());
    }
    Ok(Some(marker))
}

pub fn write_install_marker(path: &Path, marker: &InstallMarker) -> Result<(), String> {
    if !marker.valid() {
        return Err("cannot write an invalid update install marker".into());
    }
    let directory = path.parent().ok_or("install marker path has no parent")?;
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let temporary = directory.join(format!(".{}.{}.tmp", path.file_name().and_then(|name| name.to_str()).unwrap_or("install-marker"), uuid::Uuid::new_v4()));
    std::fs::write(&temporary, format!("{}\n", serde_json::to_string_pretty(marker).map_err(|error| error.to_string())?)).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}

pub fn mark_install_handoff(path: &Path, attempt_id: &str, artifact: &ArtifactIdentity, now_iso: &str) -> Result<Option<InstallMarker>, String> {
    let Some(mut marker) = read_install_marker(path)? else { return Ok(None) };
    if marker.attempt_id != attempt_id || marker.artifact != *artifact || marker.phase == "failed" {
        return Ok(None);
    }
    if marker.handoff_at.is_some() {
        return Ok((marker.phase == "handoff").then_some(marker));
    }
    marker.phase = "handoff".into();
    marker.handoff_at = Some(now_iso.into());
    write_install_marker(path, &marker)?;
    Ok(Some(marker))
}

pub fn record_install_marker_failure(path: &Path, attempt_id: &str, artifact: &ArtifactIdentity, now_iso: &str) -> Result<Option<InstallMarker>, String> {
    let Some(mut marker) = read_install_marker(path)? else { return Ok(None) };
    if marker.attempt_id != attempt_id || marker.artifact != *artifact {
        return Ok(None);
    }
    if marker.phase != "failed" {
        marker.phase = "failed".into();
        marker.consecutive_failures += 1;
        marker.last_failure_at = Some(now_iso.into());
        write_install_marker(path, &marker)?;
    }
    Ok(Some(marker))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    Success,
    Failure,
    AlreadyFailed,
    Stale,
    Invalid,
}

pub fn install_marker_outcome(marker: &InstallMarker, current_version: &str, now_iso: &str) -> InstallOutcome {
    let (Some(now), Some(requested)) = (iso_ms(now_iso), iso_ms(&marker.requested_at)) else { return InstallOutcome::Invalid };
    if !marker.valid() { return InstallOutcome::Invalid; }
    if !version_newer(current_version, &marker.to_version) { return InstallOutcome::Success; }
    if now - requested > 7 * 24 * 60 * 60 * 1000 { return InstallOutcome::Stale; }
    if marker.phase == "failed" { InstallOutcome::AlreadyFailed } else { InstallOutcome::Failure }
}

pub fn updater_cache_dir(cache_name: &str, platform: &str, home: &Path, local_app_data: Option<&Path>, xdg_cache_home: Option<&Path>) -> Option<PathBuf> {
    if cache_name.trim().is_empty() { return None; }
    let base = match platform {
        "win32" => local_app_data.map(Path::to_path_buf).unwrap_or_else(|| home.join("AppData").join("Local")),
        "macos" | "darwin" => home.join("Library").join("Caches"),
        _ => xdg_cache_home.map(Path::to_path_buf).unwrap_or_else(|| home.join(".cache")),
    };
    Some(base.join(cache_name))
}

#[derive(Default)]
pub struct InstallPreparationCoordinator {
    active: std::sync::Mutex<Option<u64>>,
    next: std::sync::atomic::AtomicU64,
}

impl InstallPreparationCoordinator {
    pub fn begin(&self) -> Option<u64> {
        let mut active = self.active.lock().ok()?;
        if active.is_some() { return None; }
        let token = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        *active = Some(token);
        Some(token)
    }

    pub fn cancel(&self) -> bool {
        self.active.lock().map(|mut active| active.take().is_some()).unwrap_or(false)
    }

    pub fn require_active(&self, token: u64) -> Result<(), String> {
        if self.active.lock().map_err(|_| "install preparation lock poisoned")?.is_some_and(|active| active == token) {
            Ok(())
        } else {
            Err("Update install preparation was cancelled.".into())
        }
    }

    pub fn release(&self, token: u64) -> Result<(), String> {
        self.require_active(token)?;
        if let Ok(mut active) = self.active.lock() { *active = None; }
        Ok(())
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResumableDownloadConfig {
    pub idle_timeout_ms: u64,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub max_consecutive_stall_retries: u32,
    pub max_total_attempts: u32,
    pub overall_timeout_ms: u64,
    pub progress_throttle_ms: u64,
    pub max_redirects: u32,
}

impl Default for ResumableDownloadConfig {
    fn default() -> Self {
        Self {
            idle_timeout_ms: 15_000,
            retry_base_delay_ms: 500,
            retry_max_delay_ms: 5_000,
            max_consecutive_stall_retries: 6,
            max_total_attempts: 100,
            overall_timeout_ms: 600_000,
            progress_throttle_ms: 500,
            max_redirects: 10,
        }
    }
}

pub fn parse_content_range_total(value: Option<&str>) -> Option<u64> {
    value?.rsplit('/').next()?.trim().parse().ok().filter(|total| *total > 0)
}

pub fn select_sha512_encoding(value: &str) -> &'static str {
    if value.len() == 128 && !value.contains('+') && !value.contains('Z') && !value.contains('=') { "hex" } else { "base64" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadResponseAction {
    Append { total: Option<u64> },
    FromStart { total: Option<u64> },
    Complete,
    Retryable { status: u16 },
    Fatal { status: u16 },
}

pub fn classify_download_response(status: u16, content_range: Option<&str>, content_length: Option<u64>, already: u64) -> DownloadResponseAction {
    match status {
        206 => DownloadResponseAction::Append { total: parse_content_range_total(content_range).or(content_length.map(|length| already + length)) },
        200 => DownloadResponseAction::FromStart { total: content_length },
        416 => DownloadResponseAction::Complete,
        429 | 500..=599 => DownloadResponseAction::Retryable { status },
        _ => DownloadResponseAction::Fatal { status },
    }
}

pub fn retry_delay_ms(consecutive_stalls: u32, config: &ResumableDownloadConfig) -> u64 {
    if consecutive_stalls <= 1 { return 0; }
    let shift = consecutive_stalls.saturating_sub(2).min(20);
    (config.retry_base_delay_ms.saturating_mul(1u64 << shift)).min(config.retry_max_delay_ms)
}

pub fn download_should_give_up(consecutive_stalls: u32, attempts: u32, elapsed_ms: u64, config: &ResumableDownloadConfig) -> bool {
    consecutive_stalls > config.max_consecutive_stall_retries || attempts > config.max_total_attempts || elapsed_ms > config.overall_timeout_ms
}

pub fn download_headers(start_offset: u64, attach_auth: bool, headers: &[(String, String)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = headers.iter().filter(|(key, _)| {
        let key = key.to_ascii_lowercase();
        attach_auth || (key != "authorization" && key != "proxy-authorization")
    }).cloned().collect();
    if !out.iter().any(|(key, _)| key.eq_ignore_ascii_case("user-agent")) {
        out.push(("User-Agent".into(), "synara-updater".into()));
    }
    out.push(("Cache-Control".into(), "no-cache".into()));
    if start_offset > 0 { out.push(("Range".into(), format!("bytes={start_offset}-"))); }
    out
}


fn download_file_resumable(url: &str, target: &Path, expected_size: Option<u64>, mut progress: impl FnMut(u64, Option<u64>)) -> Result<u64, String> {
    let config = ResumableDownloadConfig::default();
    let temporary = target.with_extension("download");
    let started = std::time::Instant::now();
    let mut attempts = 0u32;
    let mut stalls = 0u32;
    let mut received = std::fs::metadata(&temporary).map(|metadata| metadata.len()).unwrap_or(0);
    loop {
        attempts += 1;
        if download_should_give_up(stalls, attempts, started.elapsed().as_millis() as u64, &config) {
            return Err("The update download stalled repeatedly. Check the connection and try again.".into());
        }
        let mut request = ureq::get(url)
            .set("User-Agent", "synara-updater")
            .set("Cache-Control", "no-cache")
            .timeout(std::time::Duration::from_millis(config.idle_timeout_ms));
        if received > 0 {
            request = request.set("Range", &format!("bytes={received}-"));
        }
        let response = match request.call() {
            Ok(response) => response,
            Err(ureq::Error::Status(status, response)) => {
                let action = classify_download_response(status, response.header("Content-Range"), response.header("Content-Length").and_then(|value| value.parse().ok()), received);
                if matches!(action, DownloadResponseAction::Complete) && expected_size.is_some_and(|size| size == received) {
                    break;
                }
                if matches!(action, DownloadResponseAction::Retryable { .. }) {
                    stalls += 1;
                    std::thread::sleep(std::time::Duration::from_millis(retry_delay_ms(stalls, &config)));
                    continue;
                }
                return Err(format!("Could not download update: server returned HTTP {status}"));
            }
            Err(error) => {
                stalls += 1;
                if download_should_give_up(stalls, attempts, started.elapsed().as_millis() as u64, &config) {
                    return Err(format!("Could not download update: {error}"));
                }
                std::thread::sleep(std::time::Duration::from_millis(retry_delay_ms(stalls, &config)));
                continue;
            }
        };
        let status = response.status();
        let action = classify_download_response(status, response.header("Content-Range"), response.header("Content-Length").and_then(|value| value.parse().ok()), received);
        let mut append = true;
        let mut total = expected_size;
        match action {
            DownloadResponseAction::Append { total: response_total } => {
                if received == 0 { append = false; }
                total = response_total.or(total);
            }
            DownloadResponseAction::FromStart { total: response_total } => {
                append = false;
                total = response_total.or(total);
            }
            DownloadResponseAction::Complete => {
                if expected_size.is_some_and(|size| size == received) { break; }
                stalls += 1;
                continue;
            }
            DownloadResponseAction::Retryable { .. } => {
                stalls += 1;
                std::thread::sleep(std::time::Duration::from_millis(retry_delay_ms(stalls, &config)));
                continue;
            }
            DownloadResponseAction::Fatal { status } => {
                return Err(format!("Could not download update: server returned HTTP {status}"));
            }
        }
        if !append { received = 0; }
        let mut output = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(&temporary)
            .map_err(|error| format!("Could not create update download: {error}"))?;
        let mut reader = response.into_reader();
        let mut buffer = [0u8; 128 * 1024];
        let mut interrupted = None;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    if total.is_some_and(|total| received < total) {
                        interrupted = Some("connection ended before the update payload was complete".to_string());
                    }
                    break;
                }
                Ok(count) => {
                    if let Err(error) = std::io::Write::write_all(&mut output, &buffer[..count]) {
                        return Err(format!("Could not write update download: {error}"));
                    }
                    received += count as u64;
                    progress(received, total);
                }
                Err(error) => {
                    interrupted = Some(error.to_string());
                    break;
                }
            }
        }
        drop(output);
        if let Some(error) = interrupted {
            stalls += 1;
            if download_should_give_up(stalls, attempts, started.elapsed().as_millis() as u64, &config) {
                return Err(format!("Could not download update: {error}"));
            }
            std::thread::sleep(std::time::Duration::from_millis(retry_delay_ms(stalls, &config)));
            continue;
        }
        break;
    }
    if let Some(expected_size) = expected_size {
        if expected_size != received {
            return Err("The update download ended before the expected payload size.".into());
        }
    }
    std::fs::rename(&temporary, target).map_err(|error| format!("Could not stage update download: {error}"))?;
    Ok(received)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateFeedFile {
    pub url: String,
    pub sha512: String,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateManifest {
    pub version: String,
    pub files: Vec<UpdateFeedFile>,
}

fn yaml_scalar(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        trimmed[1..trimmed.len()-1].replace("''", "'")
    } else if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len()-1].into()
    } else { trimmed.into() }
}

pub fn parse_update_manifest(text: &str) -> Result<UpdateManifest, String> {
    let mut version = None;
    let mut files = Vec::new();
    let mut current: Option<usize> = None;
    let mut in_files = false;
    for raw in text.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        if !raw.starts_with(' ') && !raw.starts_with('\t') && !raw.starts_with('-') {
            in_files = false;
            current = None;
            if let Some((key, value)) = trimmed.split_once(':') {
                if key.trim() == "files" { in_files = true; }
                if key.trim() == "version" { version = Some(yaml_scalar(value)); }
            }
            continue;
        }
        if !in_files { continue; }
        if let Some(rest) = trimmed.strip_prefix("- ") {
            files.push(UpdateFeedFile { url: String::new(), sha512: String::new(), size: None });
            current = Some(files.len() - 1);
            if let Some((key, value)) = rest.split_once(':') {
                assign_manifest_field(&mut files[current.unwrap()], key.trim(), value);
            }
        } else if let (Some(index), Some((key, value))) = (current, trimmed.split_once(':')) {
            assign_manifest_field(&mut files[index], key.trim(), value);
        }
    }
    files.retain(|file| !file.url.is_empty() && !file.sha512.is_empty());
    match (version.filter(|value: &String| !value.is_empty()), files.is_empty()) {
        (Some(version), false) => Ok(UpdateManifest { version, files }),
        _ => Err("could not parse update manifest: missing version or files".into()),
    }
}

fn assign_manifest_field(file: &mut UpdateFeedFile, key: &str, value: &str) {
    let value = yaml_scalar(value);
    match key {
        "url" => file.url = value,
        "sha512" => file.sha512 = value,
        "size" => file.size = value.parse().ok(),
        _ => {}
    }
}

pub fn select_update_download_file<'a>(files: &'a [UpdateFeedFile], arch: &str, extension: &str) -> Result<&'a UpdateFeedFile, String> {
    let matches: Vec<&UpdateFeedFile> = files.iter().filter(|file| file.url.ends_with(extension)).collect();
    let unmarked = (matches.len() == 1).then_some(matches[0]).filter(|file| !file.url.contains("-arm64-") && !file.url.contains("-x64-") && !file.url.ends_with("-arm64.zip") && !file.url.ends_with("-x64.zip"));
    matches.iter().copied().find(|file| file.url.contains(&format!("-{arch}-")) || file.url.ends_with(&format!("-{arch}{extension}"))).or(unmarked)
        .ok_or_else(|| format!("no update download for {arch} in manifest"))
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 { return None; }
    let mut out = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

fn artifact_matches_feed_file(identity: &ArtifactIdentity, expected: &UpdateFeedFile) -> bool {
    let Some(digest) = decode_hex(&identity.sha512) else { return false };
    identity.sha512.eq_ignore_ascii_case(&expected.sha512)
        || base64::Engine::encode(&base64::engine::general_purpose::STANDARD, digest) == expected.sha512
}

// ---------------------------------------------------------------------------
// Runtime service and private desktop bridge
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct RestartTicket {
    token: String,
    authority: String,
    version: String,
    expires_at_ms: i64,
}

pub struct DesktopUpdateService {
    state: std::sync::Mutex<UpdateState>,
    artifact: std::sync::Mutex<Option<ArtifactIdentity>>,
    selected_file: std::sync::Mutex<Option<UpdateFeedFile>>,
    ticket: std::sync::Mutex<Option<RestartTicket>>,
    committed: std::sync::atomic::AtomicBool,
    automatic_checks_suppressed: std::sync::atomic::AtomicBool,
    feed_url: Option<String>,
    cache_dir: PathBuf,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingDownloadedUpdate {
    version: String,
    file: UpdateFeedFile,
    artifact_path: PathBuf,
    artifact_size: u64,
    artifact_sha512: String,
}

fn pending_update_path(cache_dir: &Path, version: &str) -> PathBuf {
    cache_dir.join(version).join("pending-update.json")
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let tmp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    std::fs::rename(&tmp, path).map_err(|error| error.to_string())
}

fn persist_pending_update(cache_dir: &Path, version: &str, file: &UpdateFeedFile, artifact: &ArtifactIdentity) -> Result<(), String> {
    write_json_atomic(&pending_update_path(cache_dir, version), &PendingDownloadedUpdate {
        version: version.to_string(),
        file: file.clone(),
        artifact_path: artifact.path.clone(),
        artifact_size: artifact.size,
        artifact_sha512: artifact.sha512.clone(),
    })
}

fn validate_pending_update(cache_dir: &Path, version: &str, flavor: UpdateFlavor) -> Option<(String, UpdateFeedFile, ArtifactIdentity)> {
    let path = pending_update_path(cache_dir, version);
    let pending: PendingDownloadedUpdate = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    if pending.version != version || !version_allowed_for_flavor(&pending.version, flavor) {
        return None;
    }
    let artifact = fingerprint_artifact(&pending.artifact_path).ok()?;
    if artifact.size != pending.artifact_size || artifact.sha512 != pending.artifact_sha512 {
        return None;
    }
    if !artifact_matches_feed_file(&artifact, &pending.file) { return None; }
    Some((pending.version, pending.file, artifact))
}

fn restore_pending_update(cache_dir: &Path, current_version: &str, flavor: UpdateFlavor) -> Option<(String, UpdateFeedFile, ArtifactIdentity)> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(cache_dir).ok()? {
        let Ok(entry) = entry else { continue };
        let Some(version) = entry.file_name().to_str().map(str::to_string) else { continue };
        if version_newer(current_version, &version) {
            if let Some(candidate) = validate_pending_update(cache_dir, &version, flavor) {
                candidates.push(candidate);
            }
        }
    }
    candidates.sort_by(|left, right| {
        if version_newer(&right.0, &left.0) { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
    });
    candidates.into_iter().next()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn update_install_marker_path() -> PathBuf {
    crate::backend::configured_home_dir()
        .join("userdata")
        .join("pending-update-install.json")
}

fn install_restart_failure_state(state: &mut UpdateState, version: &str, failures: u32) {
    state.status = UpdateStatus::Error;
    state.available_version = Some(version.to_string());
    state.downloaded_version = None;
    state.download_percent = None;
    state.message = Some(format!(
        "Synara restarted, but update {version} was not installed. Click to try again."
    ));
    state.error_context = Some("install".into());
    state.can_retry = true;
    state.install_failure_count = failures;
}

fn cleanup_successful_install_cache(cache_dir: &Path, marker: &InstallMarker) {
    let version = Path::new(&marker.to_version);
    let mut components = version.components();
    let Some(std::path::Component::Normal(_)) = components.next() else { return };
    if components.next().is_some() { return; }
    let canonical_cache = cache_dir.canonicalize().unwrap_or_else(|_| cache_dir.to_path_buf());
    let version_dir = canonical_cache.join(&marker.to_version);
    if marker.artifact.path.starts_with(&version_dir) {
        let _ = std::fs::remove_dir_all(version_dir);
    }
}

fn apply_install_marker_startup(state: &mut UpdateState, path: &Path) -> bool {
    let marker = match read_install_marker(path) {
        Ok(Some(marker)) => marker,
        Ok(None) => return false,
        Err(_) => {
            let _ = std::fs::remove_file(path);
            return false;
        }
    };
    match install_marker_outcome(&marker, &state.current_version, &now_iso()) {
        InstallOutcome::Success | InstallOutcome::Stale | InstallOutcome::Invalid => {
            let _ = std::fs::remove_file(path);
            false
        }
        InstallOutcome::Failure | InstallOutcome::AlreadyFailed => {
            let mut marker = marker;
            if marker.phase != "failed" {
                marker.phase = "failed".into();
                marker.consecutive_failures += 1;
                marker.last_failure_at = Some(now_iso());
                let _ = write_install_marker(path, &marker);
            }
            install_restart_failure_state(
                state,
                &marker.to_version,
                marker.consecutive_failures.max(1),
            );
            true
        }
    }
}

fn runtime_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        _ => "other",
    }
}

fn packaged_mac_app() -> bool {
    std::env::current_exe().is_ok_and(|exe| {
        let text = exe.to_string_lossy();
        text.contains(".app/Contents/MacOS/")
    })
}

fn desktop_update_supported() -> bool {
    std::env::var("SYNARA_UPDATE_ENABLE").ok().as_deref() == Some("1")
        || (cfg!(target_os = "macos") && packaged_mac_app())
}

fn parse_app_update_yml(text: &str) -> Option<std::collections::HashMap<String, String>> {
    let mut entries = std::collections::HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        let Some((key, value)) = trimmed.split_once(':') else { continue };
        if key.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
            entries.insert(key.trim().to_string(), yaml_scalar(value));
        }
    }
    entries.contains_key("provider").then_some(entries)
}

fn packaged_app_update_yml() -> Option<std::collections::HashMap<String, String>> {
    let exe = std::env::current_exe().ok()?;
    let macos_dir = exe.parent()?;
    if !macos_dir.ends_with("MacOS") { return None; }
    let contents = macos_dir.parent()?;
    let path = contents.join("Resources").join("app-update.yml");
    parse_app_update_yml(&std::fs::read_to_string(path).ok()?)
}

fn update_channel_for_flavor(flavor: UpdateFlavor) -> &'static str {
    match flavor {
        UpdateFlavor::Beta => "beta",
        UpdateFlavor::Remote => "remote",
        _ => "synara",
    }
}

fn update_manifest_name(flavor: UpdateFlavor) -> String {
    let channel = update_channel_for_flavor(flavor);
    if cfg!(target_os = "macos") {
        format!("{channel}-mac.yml")
    } else if cfg!(target_os = "windows") {
        format!("{channel}.yml")
    } else if runtime_arch() == "arm64" {
        format!("{channel}-linux-arm64.yml")
    } else {
        format!("{channel}-linux.yml")
    }
}

fn github_update_feed_url(entries: &std::collections::HashMap<String, String>, flavor: UpdateFlavor) -> Option<String> {
    if entries.get("provider").map(String::as_str) != Some("github") { return None; }
    let owner = entries.get("owner")?.trim();
    let repo = entries.get("repo")?.trim();
    if owner.is_empty() || repo.is_empty() { return None; }
    let host = entries.get("host").map(String::as_str).filter(|host| !host.trim().is_empty()).unwrap_or("github.com");
    let protocol = if entries.get("protocol").map(String::as_str) == Some("http") { "http" } else { "https" };
    let base = format!("{protocol}://{host}/{owner}/{repo}");
    if matches!(flavor, UpdateFlavor::Beta | UpdateFlavor::Remote) {
        // Prerelease lanes are never GitHub Latest. Like electron-updater,
        // discover their newest matching tag through the releases atom feed.
        Some(format!("{base}/releases.atom"))
    } else {
        Some(format!(
            "{base}/releases/latest/download/{}",
            update_manifest_name(flavor)
        ))
    }
}


fn plist_string_value(text: &str, key: &str) -> Option<String> {
    let needle = format!("<key>{key}</key>");
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.trim() == needle {
            for next in lines.by_ref() {
                let trimmed = next.trim();
                if trimmed.is_empty() { continue; }
                return trimmed
                    .strip_prefix("<string>")
                    .and_then(|value| value.strip_suffix("</string>"))
                    .map(str::to_string);
            }
        }
    }
    None
}

fn plist_value(path: &Path, key: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("/usr/libexec/PlistBuddy").arg("-c").arg(format!("Print:{key}")).arg(path).output() {
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !value.is_empty() { return Some(value); }
            }
        }
    }
    plist_string_value(&std::fs::read_to_string(path).ok()?, key)
}

fn current_app_bundle_path() -> Result<PathBuf, String> {
    if !cfg!(target_os = "macos") {
        return Err("Packaged install handoff is currently implemented for macOS only.".into());
    }
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    for ancestor in exe.ancestors() {
        if ancestor.extension().and_then(|value| value.to_str()) == Some("app") {
            return Ok(ancestor.to_path_buf());
        }
    }
    Err("This Synara build is not running from a packaged .app bundle.".into())
}

fn validate_bundle_architecture(app: &Path) -> Result<(), String> {
    let plist = app.join("Contents").join("Info.plist");
    let executable = plist_value(&plist, "CFBundleExecutable")
        .ok_or("The update bundle does not declare an executable.")?;
    if executable.contains('/') || executable.contains("..") {
        return Err("The update bundle declares an invalid executable.".into());
    }
    let binary = app.join("Contents").join("MacOS").join(executable);
    if !binary.is_file() {
        return Err("The update bundle executable is missing.".into());
    }
    let output = Command::new("lipo").arg("-info").arg(&binary).output()
        .map_err(|error| format!("Could not inspect update bundle architecture: {error}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let wanted = match runtime_arch() { "arm64" => "arm64", "x64" => "x86_64", other => other };
    if !output.status.success() || !text.contains(wanted) {
        return Err(format!("The update bundle does not contain a {wanted} executable."));
    }
    Ok(())
}


fn bundle_team_identifier(app: &Path) -> Result<Option<String>, String> {
    let output = Command::new("codesign")
        .arg("-dv")
        .arg("--verbose=4")
        .arg(app)
        .output()
        .map_err(|error| format!("Could not inspect the update bundle signer: {error}"))?;
    if !output.status.success() {
        return Err("Could not inspect the update bundle signer.".into());
    }
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    for line in text.lines() {
        if let Some(value) = line.trim().strip_prefix("TeamIdentifier=") {
            let value = value.trim();
            return Ok((value != "not set").then(|| value.to_string()));
        }
    }
    Ok(None)
}

fn verify_bundle_signature(app: &Path) -> Result<(), String> {
    let output = Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(app)
        .output()
        .map_err(|error| format!("Could not verify the update bundle signature: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "The update bundle signature is invalid: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn validate_staged_bundle(staged: &Path, current: &Path) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(staged).map_err(|error| format!("Could not read staged update: {error}"))?;
    let mut apps = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("app"));
    let Some(app) = apps.next() else { return Err("The update payload does not contain a .app bundle.".into()) };
    if apps.next().is_some() {
        return Err("The update payload contains multiple .app bundles.".into());
    }
    if app.is_symlink() || !app.is_dir() {
        return Err("The update bundle must be a real application directory.".into());
    }
    let current_plist = current.join("Contents").join("Info.plist");
    let staged_plist = app.join("Contents").join("Info.plist");
    let current_id = plist_value(&current_plist, "CFBundleIdentifier")
        .ok_or("The running app does not declare a bundle identifier.")?;
    let staged_id = plist_value(&staged_plist, "CFBundleIdentifier")
        .ok_or("The update bundle does not declare a bundle identifier.")?;
    if staged_id != current_id {
        return Err("The update bundle identity does not match the running Synara app.".into());
    }
    validate_bundle_architecture(&app)?;
    verify_bundle_signature(&app)?;
    if bundle_team_identifier(&app)? != bundle_team_identifier(current)? {
        return Err("The update bundle signer does not match the running Synara app.".into());
    }
    Ok(app)
}

fn extract_update_payload(payload: &Path, cache_dir: &Path, version: &str) -> Result<PathBuf, String> {
    let staging = cache_dir.join(version).join("staged");
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|error| format!("Could not clear the staged update: {error}"))?;
    }
    std::fs::create_dir_all(&staging).map_err(|error| format!("Could not create the staged update directory: {error}"))?;
    let output = Command::new("ditto").arg("-x").arg("-k").arg(payload).arg(&staging).output()
        .map_err(|error| format!("Could not extract the update payload: {error}"))?;
    if !output.status.success() {
        return Err(format!("Could not extract the update payload: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    validate_staged_bundle(&staging, &current_app_bundle_path()?)
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn spawn_macos_install_helper(current: &Path, staged: &Path, cache_dir: &Path) -> Result<(), String> {
    let parent = current.parent().ok_or("The running app bundle has no install directory.")?;
    let probe = parent.join(format!(".synara-update-write-test-{}", std::process::id()));
    std::fs::write(&probe, b"").map_err(|_| "Synara cannot write beside the installed app. Move it to a writable install location.".to_string())?;
    let _ = std::fs::remove_file(&probe);
    let backup = parent.join(format!(
        ".{}-update-backup-{}",
        current.file_name().and_then(|value| value.to_str()).unwrap_or("Synara.app"),
        uuid::Uuid::new_v4()
    ));
    let log_path = cache_dir.join("install.log");
    let script_path = cache_dir.join("install-update.sh");
    let shell_pid = std::process::id();
    let script = format!(
        "#!/bin/sh\nset -u\nparent_pid={shell_pid}\nself_pid=$$\nchild_pids=$(pgrep -P \"$parent_pid\" 2>/dev/null || true)\nwhile kill -0 \"$parent_pid\" 2>/dev/null; do sleep 0.1; done\nfor child in $child_pids; do [ \"$child\" = \"$self_pid\" ] && continue; kill \"$child\" 2>/dev/null || true; done\nsleep 0.3\ncurrent={current}\nstaged={staged}\nbackup={backup}\nlog={log}\nrm -rf \"$backup\"\nif ! mv \"$current\" \"$backup\" 2>>\"$log\"; then echo 'could not move running app aside' >>\"$log\"; exit 1; fi\nif ! mv \"$staged\" \"$current\" 2>>\"$log\"; then echo 'could not move update into place' >>\"$log\"; mv \"$backup\" \"$current\" 2>/dev/null; exit 1; fi\nxattr -dr com.apple.quarantine \"$current\" >/dev/null 2>&1 || true\nopen -n \"$current\" >>\"$log\" 2>&1\nrm -rf \"$backup\"\n",
        current = shell_single_quote(&current.to_string_lossy()),
        staged = shell_single_quote(&staged.to_string_lossy()),
        backup = shell_single_quote(&backup.to_string_lossy()),
        log = shell_single_quote(&log_path.to_string_lossy()),
    );
    std::fs::write(&script_path, script).map_err(|error| format!("Could not write the update installer: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700)).map_err(|error| error.to_string())?;
    }
    Command::new("/bin/sh")
        .arg(&script_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start the update installer: {error}"))?;
    Ok(())
}

fn prepare_macos_install(artifact: &ArtifactIdentity, cache_dir: &Path, version: &str) -> Result<(), String> {
    let staged = extract_update_payload(&artifact.path, cache_dir, version)?;
    let current = current_app_bundle_path()?;
    spawn_macos_install_helper(&current, &staged, cache_dir)
}

pub fn github_release_tags_from_atom(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for entry in text.split("<entry>").skip(1) {
        let mut rest = entry;
        while let Some(position) = rest.find("/releases/tag/") {
            let after = &rest[position + "/releases/tag/".len()..];
            let end = after.find(|ch| matches!(ch, '"' | '\'' | '<' | '?' | '#')).unwrap_or(after.len());
            let tag = after[..end].trim();
            if !tag.is_empty() && tag.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+')) {
                let tag = tag.strip_prefix('v').unwrap_or(tag).to_string();
                if !tags.contains(&tag) { tags.push(tag); }
                break;
            }
            rest = &after[end.min(after.len())..];
        }
    }
    tags
}

fn atom_tag_manifest_url(atom_feed: &str, tag: &str, flavor: UpdateFlavor) -> Option<String> {
    let base = atom_feed.strip_suffix("/releases.atom")?;
    Some(format!("{base}/releases/download/v{tag}/{}", update_manifest_name(flavor)))
}

fn fetch_update_text(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .set("User-Agent", "synara-updater")
        .set("Cache-Control", "no-cache")
        .call()
        .map_err(|error| format!("Could not check for updates: {error}"))?;
    if response.status() != 200 {
        return Err(format!("Could not check for updates: update feed returned HTTP {}", response.status()));
    }
    response.into_string().map_err(|error| format!("Could not read update feed: {error}"))
}

enum ResolvedUpdateFeed {
    Manifest { url: String, text: String },
    NoUpdate,
}

fn resolve_update_feed(feed: &str, current_version: &str, flavor: UpdateFlavor) -> Result<ResolvedUpdateFeed, String> {
    if !feed.ends_with("/releases.atom") {
        return fetch_update_text(feed).map(|text| ResolvedUpdateFeed::Manifest { url: feed.to_string(), text });
    }
    let atom = fetch_update_text(feed)?;
    for tag in github_release_tags_from_atom(&atom) {
        if version_newer(current_version, &tag) && version_allowed_for_flavor(&tag, flavor) {
            let url = atom_tag_manifest_url(feed, &tag, flavor).ok_or("update atom feed is invalid")?;
            return fetch_update_text(&url).map(|text| ResolvedUpdateFeed::Manifest { url, text });
        }
    }
    Ok(ResolvedUpdateFeed::NoUpdate)
}

fn configured_feed_url() -> Option<String> {
    if let Some(value) = std::env::var("SYNARA_UPDATE_FEED_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| value.starts_with("https://") || value.starts_with("http://"))
    {
        return Some(value);
    }
    packaged_app_update_yml().as_ref().and_then(|entries| github_update_feed_url(entries, configured_flavor()))
}

fn configured_release_url() -> Option<String> {
    if let Some(value) = std::env::var("SYNARA_UPDATE_RELEASE_URL").ok().filter(|v| !v.trim().is_empty()) {
        return Some(value);
    }
    let entries = packaged_app_update_yml()?;
    let owner = entries.get("owner")?;
    let repo = entries.get("repo")?;
    let host = entries.get("host").map(String::as_str).filter(|host| !host.trim().is_empty());
    Some(github_releases_page(owner, repo, host, None))
}

fn current_desktop_version() -> String {
    if let Some(value) = std::env::var("SYNARA_DESKTOP_VERSION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return value;
    }
    if let Ok(app) = current_app_bundle_path() {
        if let Some(version) = plist_value(&app.join("Contents").join("Info.plist"), "CFBundleShortVersionString") {
            if !version.trim().is_empty() { return version; }
        }
    }
    env!("CARGO_PKG_VERSION").to_string()
}

fn configured_flavor() -> UpdateFlavor {
    // Packaged bundle identity wins over process environment. The update
    // override remains useful in unpackaged smoke fixtures, where the shell is
    // not running from a flavor-stamped .app bundle.
    let flavor_override = std::env::var("SYNARA_UPDATE_FLAVOR").ok();
    let unpackaged = !std::env::current_exe()
        .is_ok_and(|exe| exe.to_string_lossy().contains(".app/Contents/MacOS/"));
    let flavor = if unpackaged {
        flavor_override.unwrap_or_else(|| crate::desktop_identity::runtime_flavor().to_string())
    } else {
        crate::desktop_identity::runtime_flavor().to_string()
    };
    match flavor.as_str() {
        "beta" => UpdateFlavor::Beta,
        "canary" => UpdateFlavor::Canary,
        "cua" => UpdateFlavor::Cua,
        "remote" => UpdateFlavor::Remote,
        _ => UpdateFlavor::Production,
    }
}

fn updater_cache_name(flavor: UpdateFlavor) -> &'static str {
    match flavor {
        UpdateFlavor::Beta => "synara-desktop-beta-updater",
        UpdateFlavor::Remote => "synara-desktop-remote-updater",
        UpdateFlavor::Canary => "synara-desktop-canary-updater",
        UpdateFlavor::Cua => "synara-desktop-cua-updater",
        UpdateFlavor::Production => "synara-updater",
    }
}

impl DesktopUpdateService {
    fn new() -> Self {
        let arch = runtime_arch();
        let home = crate::backend::configured_home_dir();
        // electron-updater caches are OS-user caches, not Synara data: a custom
        // SYNARA_HOME must not relocate downloads into the migrated database home.
        let os_home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.clone());
        let flavor = configured_flavor();
        let cache = updater_cache_dir(
            updater_cache_name(flavor),
            std::env::consts::OS,
            &os_home,
            std::env::var_os("LOCALAPPDATA").as_deref().map(Path::new),
            std::env::var_os("XDG_CACHE_HOME").as_deref().map(Path::new),
        )
        .unwrap_or_else(|| home.join("updater"));
        let mut state = UpdateState::initial(
            current_desktop_version(),
            arch,
            arch,
            false,
            flavor,
        );
        state.enabled = configured_feed_url().is_some() && desktop_update_supported();
        state.status = if state.enabled { UpdateStatus::Idle } else { UpdateStatus::Disabled };
        state.release_url = configured_release_url();
        let install_marker_path = update_install_marker_path();
        let completed_marker = read_install_marker(&install_marker_path)
            .ok()
            .flatten()
            .filter(|marker| {
                install_marker_outcome(marker, &state.current_version, &now_iso())
                    == InstallOutcome::Success
            });
        let automatic_checks_suppressed = apply_install_marker_startup(
            &mut state,
            &install_marker_path,
        );
        if let Some(marker) = completed_marker {
            cleanup_successful_install_cache(&cache, &marker);
        }
        let restored = (!automatic_checks_suppressed)
            .then(|| restore_pending_update(&cache, &state.current_version, state.flavor))
            .flatten();
        let mut restored_artifact = None;
        let mut restored_file = None;
        if state.enabled {
            if let Some((version, file, artifact)) = restored {
                state.status = UpdateStatus::Downloaded;
                state.available_version = Some(version.clone());
                state.downloaded_version = Some(version);
                state.download_percent = Some(100.0);
                state.can_retry = true;
                restored_file = Some(file);
                restored_artifact = Some(artifact);
            }
        }
        if !state.enabled {
            state.message = Some("Automatic updates are not available because no update feed is configured.".into());
        }
        Self {
            state: std::sync::Mutex::new(state),
            artifact: std::sync::Mutex::new(restored_artifact),
            selected_file: std::sync::Mutex::new(restored_file),
            ticket: std::sync::Mutex::new(None),
            committed: std::sync::atomic::AtomicBool::new(false),
            automatic_checks_suppressed: std::sync::atomic::AtomicBool::new(automatic_checks_suppressed),
            feed_url: configured_feed_url(),
            cache_dir: cache,
        }
    }

    fn supported(&self) -> bool {
        self.feed_url.is_some() && desktop_update_supported()
    }

    fn state_json(&self) -> Value {
        let state = self.state.lock().map(|state| state.clone()).unwrap_or_else(|_| {
            UpdateState::initial(current_desktop_version(), runtime_arch(), runtime_arch(), false, configured_flavor())
        });
        serde_json::to_value(state).unwrap_or(Value::Null)
    }

    fn status_json(&self, message: Option<String>) -> Value {
        let supported = self.supported();
        json!({
            "supported": supported,
            "state": if supported { self.state_json() } else { Value::Null },
            "busy": self.committed.load(std::sync::atomic::Ordering::SeqCst),
            "message": message.or_else(|| (!supported).then(|| "Updates require a packaged Synara Desktop app with an update feed configured.".to_string())),
        })
    }

    fn set_state(&self, update: impl FnOnce(UpdateState) -> UpdateState) {
        if let Ok(mut state) = self.state.lock() {
            let next = update(state.clone());
            *state = next;
        }
    }

    fn current_state(&self) -> Option<UpdateState> {
        self.state.lock().ok().map(|state| state.clone())
    }

    pub fn snapshot_state(&self) -> Option<UpdateState> {
        self.current_state()
    }

    fn request_foreground_check(self: &Arc<Self>, backgrounded_for: std::time::Duration) -> bool {
        let Some(state) = self.current_state() else { return false };
        if !foreground_recheck_needed(
            state.checked_at.as_deref(),
            backgrounded_for.as_millis() as u64,
            now_ms(),
        ) {
            return false;
        }
        self.request_automatic_check()
    }

    fn request_automatic_check(self: &Arc<Self>) -> bool {
        if self.automatic_checks_suppressed.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }
        self.request_check()
    }

    fn request_check(self: &Arc<Self>) -> bool {
        self.automatic_checks_suppressed.store(false, std::sync::atomic::Ordering::SeqCst);
        let Some(state) = self.current_state() else { return false };
        if !self.supported() || !can_check(&state) {
            return false;
        }
        if let Ok(mut ticket) = self.ticket.lock() { *ticket = None; }
        self.set_state(|state| state.check_start(now_iso()));
        let service = Arc::clone(self);
        std::thread::spawn(move || service.perform_check());
        true
    }

    fn perform_check(self: &Arc<Self>) {
        let result = (|| -> Result<(UpdateManifest, UpdateFeedFile), String> {
            let feed = self.feed_url.clone().ok_or("update feed is not configured")?;
            let current = self.current_state().map(|state| (state.current_version, state.flavor)).ok_or("update state unavailable")?;
            let (manifest_url, text) = match resolve_update_feed(&feed, &current.0, current.1)? {
                ResolvedUpdateFeed::Manifest { url, text } => (url, text),
                ResolvedUpdateFeed::NoUpdate => return Err("__synara_no_lane_update__".into()),
            };
            let manifest = parse_update_manifest(&text)?;
            let extension = if cfg!(target_os = "macos") { ".zip" } else if cfg!(target_os = "windows") { ".exe" } else { ".AppImage" };
            let mut file = select_update_download_file(&manifest.files, runtime_arch(), extension)?.clone();
            if !file.url.starts_with("https://") && !file.url.starts_with("http://") {
                let base = manifest_url.rsplit_once('/').map(|(base, _)| base).unwrap_or(&manifest_url);
                file.url = format!("{base}/{}", file.url.trim_start_matches('/'));
            }
            Ok((manifest, file))
        })();
        match result {
            Ok((manifest, file)) => {
                let state = self.current_state().unwrap_or_else(|| UpdateState::initial(current_desktop_version(), runtime_arch(), runtime_arch(), false, configured_flavor()));
                if !version_newer(&state.current_version, &manifest.version)
                    || !version_allowed_for_flavor(&manifest.version, state.flavor)
                {
                    if let Ok(mut selected) = self.selected_file.lock() { *selected = None; }
                    self.set_state(|state| state.no_update(now_iso()));
                } else {
                    if let Ok(mut selected) = self.selected_file.lock() { *selected = Some(file); }
                    self.set_state(|state| state.update_available(manifest.version, now_iso()));
                    // electron-updater starts preparing an available payload in
                    // the background; the rail's disabled "Preparing" state is
                    // parity with that flow, not a dead manual step.
                    let service = Arc::clone(self);
                    std::thread::spawn(move || {
                        let _ = service.request_download();
                    });
                }
            }
            Err(message) if message == "__synara_no_lane_update__" => {
                if let Ok(mut selected) = self.selected_file.lock() { *selected = None; }
                self.set_state(|state| state.no_update(now_iso()));
            }
            Err(message) => self.set_state(|state| state.check_failure(message, now_iso())),
        }
    }

    fn request_download(self: &Arc<Self>) -> bool {
        let Some(state) = self.current_state() else { return false };
        if !self.supported() || state.status != UpdateStatus::Available || state.available_version.is_none() {
            return false;
        }
        if let Ok(mut ticket) = self.ticket.lock() { *ticket = None; }
        self.set_state(|state| state.download_start());
        let service = Arc::clone(self);
        std::thread::spawn(move || service.perform_download());
        true
    }

    fn artifact_matches(&self, expected: &UpdateFeedFile, identity: &ArtifactIdentity) -> bool {
        artifact_matches_feed_file(identity, expected)
    }

    fn perform_download(self: &Arc<Self>) {
        let outcome = (|| -> Result<(String, ArtifactIdentity), String> {
            let state = self.current_state().ok_or("update state unavailable")?;
            let version = state.available_version.clone().ok_or("no update is available")?;
            let selected = self.selected_file.lock().ok().and_then(|file| file.clone()).ok_or("update payload is no longer selected")?;
            let feed = self.feed_url.clone().ok_or("update feed is not configured")?;
            let url = if selected.url.starts_with("https://") || selected.url.starts_with("http://") {
                selected.url.clone()
            } else {
                let base = feed.rsplit_once('/').map(|(base, _)| base).unwrap_or(&feed);
                format!("{base}/{}", selected.url.trim_start_matches('/'))
            };
            std::fs::create_dir_all(self.cache_dir.join(&version)).map_err(|error| error.to_string())?;
            let file_name = selected.url.rsplit('/').next().filter(|name| !name.is_empty()).unwrap_or("update.payload");
            let target = self.cache_dir.join(&version).join(file_name);
            download_file_resumable(&url, &target, selected.size, |received, total| {
                if let Some(total) = total.filter(|total| *total > 0) {
                    let percent = (received as f64 / total as f64) * 100.0;
                    self.set_state(|state| state.download_progress(percent));
                }
            })?;
            let identity = fingerprint_artifact(&target)?;
            if !self.artifact_matches(&selected, &identity) {
                let _ = std::fs::remove_file(&target);
                return Err("The downloaded update did not match its signed checksum.".into());
            }
            persist_pending_update(&self.cache_dir, &version, &selected, &identity)?;
            if let Ok(mut artifact) = self.artifact.lock() { *artifact = Some(identity.clone()); }
            Ok((version, identity))
        })();
        match outcome {
            Ok((version, _identity)) => self.set_state(|state| state.download_complete(&version)),
            Err(message) => self.set_state(|state| state.download_failure(message)),
        }
    }

    fn request_install(self: &Arc<Self>, authority: &str, expected: Option<&str>, restart_token: Option<&str>) -> (bool, Option<String>, Option<String>) {
        let Some(state) = self.current_state() else {
            return (false, None, Some("Update state unavailable.".into()));
        };
        let expected = expected.unwrap_or_default();
        if state.status != UpdateStatus::Downloaded || state.downloaded_version.as_deref() != Some(expected) {
            return (false, None, Some("The downloaded update changed. Refresh its status and confirm again.".into()));
        }
        if restart_token.is_none() {
            let ticket = RestartTicket {
                token: uuid::Uuid::new_v4().to_string(),
                authority: authority.to_string(),
                version: expected.to_string(),
                expires_at_ms: now_ms() + 60_000,
            };
            let token = ticket.token.clone();
            if let Ok(mut active) = self.ticket.lock() { *active = Some(ticket); }
            return (true, Some(token), Some("Synara will restart to install the update. This connection will disconnect.".into()));
        }
        let valid = self.ticket.lock().ok().and_then(|ticket| ticket.clone()).is_some_and(|ticket| {
            Some(ticket.token.as_str()) == restart_token
                && ticket.authority == authority
                && ticket.version == expected
                && ticket.expires_at_ms >= now_ms()
        });
        if !valid {
            return (false, None, Some("The restart confirmation expired. Refresh its status and confirm again.".into()));
        }
        let artifact = self.artifact.lock().ok().and_then(|artifact| artifact.clone());
        let Some(artifact) = artifact else {
            return (false, None, Some("The verified update payload is no longer staged. Download it again.".into()));
        };
        if !verify_artifact_identity(&artifact) {
            return (false, None, Some("The downloaded update could not be reverified. Download it again.".into()));
        }
        let marker_path = update_install_marker_path();
        let existing = read_install_marker(&marker_path)
            .ok()
            .flatten()
            .filter(|marker| marker.to_version == expected);
        let mut marker = InstallMarker::create(
            state.current_version.clone(),
            expected,
            now_iso(),
            artifact.clone(),
        );
        if let Some(existing) = existing {
            marker.consecutive_failures = existing.consecutive_failures;
            marker.last_failure_at = existing.last_failure_at;
        }
        if let Err(message) = write_install_marker(&marker_path, &marker) {
            return (false, None, Some(format!("Could not prepare a durable update install handoff: {message}")));
        }
        let install_result = prepare_macos_install(&artifact, &self.cache_dir, expected)
            .and_then(|_| {
                mark_install_handoff(&marker_path, &marker.attempt_id, &artifact, &now_iso())
                    .and_then(|marked| marked.ok_or_else(|| "Durable update install handoff no longer matches the active attempt.".to_string()))
                    .map(|_| ())
            });
        if let Err(message) = install_result {
            let _ = record_install_marker_failure(&marker_path, &marker.attempt_id, &artifact, &now_iso());
            self.set_state(|state| state.install_failure(&message));
            return (false, None, Some(message));
        }
        if let Ok(mut ticket) = self.ticket.lock() { *ticket = None; }
        self.committed.store(true, std::sync::atomic::Ordering::SeqCst);
        std::thread::spawn(|| {
            // Let the loopback bridge flush its accepted response first, then
            // terminate the owned backend before the bundle is swapped. Normal
            // process exit skips Rust destructors, so this explicit signal is
            // what prevents the sidecar from surviving the install.
            std::thread::sleep(std::time::Duration::from_millis(100));
            crate::backend::shutdown_managed_backend();
            std::thread::sleep(std::time::Duration::from_millis(600));
            std::process::exit(0);
        });
        (true, None, Some("Synara is restarting to install the update.".into()))
    }

    fn handle(self: &Arc<Self>, request: &Value) -> Value {
        let action = request.get("action").and_then(Value::as_str).unwrap_or("status");
        let authority = request.get("authority").and_then(Value::as_str).unwrap_or("owner:unknown");
        if action == "status" {
            return json!({"accepted": true, "status": self.status_json(None)});
        }
        if !self.supported() {
            return json!({"accepted": false, "status": self.status_json(Some("Updates require a packaged Synara Desktop app with an update feed configured.".into()))});
        }
        if self.committed.load(std::sync::atomic::Ordering::SeqCst) {
            return json!({"accepted": false, "status": self.status_json(Some("Synara is already preparing to restart.".into()))});
        }
        match action {
            "check" => {
                let accepted = self.request_check();
                json!({"accepted": accepted, "status": self.status_json((!accepted).then(|| "An update operation is already in progress.".to_string()))})
            }
            "download" => {
                let accepted = self.request_download();
                json!({"accepted": accepted, "status": self.status_json((!accepted).then(|| "No available update can be downloaded right now.".to_string()))})
            }
            "install" => {
                let (accepted, restart_token, message) = self.request_install(
                    authority,
                    request.get("expectedVersion").and_then(Value::as_str),
                    request.get("restartToken").and_then(Value::as_str),
                );
                let mut out = json!({"accepted": accepted, "status": self.status_json(message)});
                if let Some(token) = restart_token { out["restartToken"] = json!(token); }
                out
            }
            _ => json!({"accepted": false, "status": self.status_json(Some("Unknown desktop update action.".into()))}),
        }
    }
}

pub fn note_desktop_foregrounded(backgrounded_for: std::time::Duration) {
    let _ = desktop_update_service().request_foreground_check(backgrounded_for);
}

static UPDATE_SERVICE: std::sync::OnceLock<Arc<DesktopUpdateService>> = std::sync::OnceLock::new();

pub fn desktop_update_service() -> Arc<DesktopUpdateService> {
    UPDATE_SERVICE.get_or_init(|| Arc::new(DesktopUpdateService::new())).clone()
}

fn bridge_directory() -> PathBuf {
    crate::backend::configured_home_dir().join("userdata")
}

fn write_bridge_file(port: u16, token: &str) -> Result<PathBuf, String> {
    let dir = bridge_directory();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join("desktop-update-bridge.json");
    let tmp = dir.join(format!(".desktop-update-bridge.{}.tmp", uuid::Uuid::new_v4()));
    let body = json!({"version": 1, "port": port, "token": token, "pid": std::process::id()});
    std::fs::write(&tmp, serde_json::to_string(&body).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &path).map_err(|error| error.to_string())?;
    Ok(path)
}

pub fn start_desktop_update_bridge() -> Result<(), String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    listener.set_nonblocking(false).map_err(|error| error.to_string())?;
    let port = listener.local_addr().map_err(|error| error.to_string())?.port();
    let token = uuid::Uuid::new_v4().to_string();
    let path = write_bridge_file(port, &token)?;
    let service = desktop_update_service();
    let automatic = Arc::clone(&service);
    std::thread::spawn(move || {
        // Upstream desktop behavior: a startup check after 15s, then a quiet
        // poll every four hours. A newer payload downloads in the background;
        // installation still requires an explicit owner/approved-remote action.
        std::thread::sleep(std::time::Duration::from_secs(15));
        loop {
            let _ = automatic.request_automatic_check();
            std::thread::sleep(std::time::Duration::from_secs(4 * 60 * 60));
        }
    });
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let service = Arc::clone(&service);
            let token = token.clone();
            std::thread::spawn(move || {
                use std::io::{BufRead as _, BufReader, Write as _};
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(12)));
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(12)));
                let mut reader = BufReader::new(match stream.try_clone() { Ok(s) => s, Err(_) => return });
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() || line.len() > 32 * 1024 { return; }
                let Ok(request) = serde_json::from_str::<Value>(&line) else { return };
                if request.get("token").and_then(Value::as_str) != Some(token.as_str()) { return; }
                let response = service.handle(&request);
                let _ = stream.write_all(format!("{}\n", serde_json::to_string(&response).unwrap_or_else(|_| "{}".into())).as_bytes());
            });
        }
    });
    eprintln!("desktop updater bridge: listening on 127.0.0.1:{port} ({})", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn state() -> UpdateState {
        UpdateState::initial("0.9.0", "arm64", "arm64", false, UpdateFlavor::Production)
    }

    #[test]
    fn rail_button_action_and_visibility_match_upstream_policy() {
        let mut update = state();
        assert!(!should_show_desktop_update_button(Some(&update)));
        update.enabled = true;
        update = update.check_start("now").update_available("0.9.1", "now");
        assert!(should_show_desktop_update_button(Some(&update)));
        assert_eq!(resolve_desktop_update_button_action(&update), DesktopUpdateButtonAction::Download);
        assert!(desktop_update_button_disabled(Some(&update)));
        update.error_context = Some("download".into());
        assert!(!desktop_update_button_disabled(Some(&update)));
        update = update.download_start().download_progress(42.8).download_complete("0.9.1");
        assert_eq!(resolve_desktop_update_button_action(&update), DesktopUpdateButtonAction::Install);
        assert!(desktop_update_button_label(&update, false).contains("0.9.1"));
    }

    #[test]
    fn foreground_recheck_matches_upstream_cooldowns() {
        let now = 1_000_000;
        assert!(!foreground_recheck_needed(None, 29_999, now));
        assert!(foreground_recheck_needed(None, 30_000, now));
        assert!(foreground_recheck_needed(Some("not-a-date"), 30_000, now));
        let recent = chrono::DateTime::from_timestamp_millis(now - FOREGROUND_RECHECK_MIN_INTERVAL_MS + 1)
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        assert!(!foreground_recheck_needed(Some(&recent), 30_000, now));
        let stale = chrono::DateTime::from_timestamp_millis(now - FOREGROUND_RECHECK_MIN_INTERVAL_MS)
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        assert!(foreground_recheck_needed(Some(&stale), 30_000, now));
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
    #[test]
    fn resumable_download_recovers_from_interrupted_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = b"abcdefghijklmnopqrstuvwxyz0123456789".to_vec();
        let server_body = body.clone();
        let server = std::thread::spawn(move || {
            for index in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while !request.windows(4).any(|tail| tail == b"\r\n\r\n") {
                    if std::io::Read::read(&mut stream, &mut byte).unwrap() == 0 { break; }
                    request.push(byte[0]);
                }
                let request = String::from_utf8_lossy(&request);
                if index == 0 {
                    let part = &server_body[..12];
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", server_body.len());
                    std::io::Write::write_all(&mut stream, response.as_bytes()).unwrap();
                    std::io::Write::write_all(&mut stream, part).unwrap();
                } else {
                    assert!(request.contains("Range: bytes=12-"), "{request}");
                    let part = &server_body[12..];
                    let response = format!("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 12-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", server_body.len() - 1, server_body.len(), part.len());
                    std::io::Write::write_all(&mut stream, response.as_bytes()).unwrap();
                    std::io::Write::write_all(&mut stream, part).unwrap();
                }
            }
        });
        let root = std::env::temp_dir().join(format!("synara-resume-download-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("payload.bin");
        let mut progresses = 0;
        let received = download_file_resumable(&format!("http://127.0.0.1:{port}/payload"), &target, Some(body.len() as u64), |_, _| progresses += 1).unwrap();
        server.join().unwrap();
        assert_eq!(received as usize, body.len());
        assert_eq!(std::fs::read(&target).unwrap(), body);
        assert!(progresses > 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pending_download_restores_newest_verified_payload() {
        let root = std::env::temp_dir().join(format!("synara-pending-update-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let mut file = UpdateFeedFile { url: "Synara-0.9.2-arm64.zip".into(), sha512: String::new(), size: Some(7) };
        for version in ["0.9.1", "0.9.2"] {
            let dir = root.join(version);
            std::fs::create_dir_all(&dir).unwrap();
            let payload = dir.join(&file.url);
            File::create(&payload).unwrap().write_all(b"payload").unwrap();
            let artifact = fingerprint_artifact(&payload).unwrap();
            file.sha512 = artifact.sha512.clone();
            persist_pending_update(&root, version, &file, &artifact).unwrap();
        }
        let restored = restore_pending_update(&root, "0.9.0", UpdateFlavor::Production).unwrap();
        assert_eq!(restored.0, "0.9.2");
        assert_eq!(restored.2.size, 7);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_marker_is_atomic_and_reports_outcomes() {
        let root = std::env::temp_dir().join(format!("synara-marker-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let payload = root.join("payload.bin");
        std::fs::write(&payload, b"payload").unwrap();
        let artifact = fingerprint_artifact(&payload).unwrap();
        let path = root.join("install.json");
        let marker = InstallMarker::create("0.9.0", "0.9.1", "2026-01-01T00:00:00.000Z", artifact.clone());
        write_install_marker(&path, &marker).unwrap();
        let handed = mark_install_handoff(&path, &marker.attempt_id, &artifact, "2026-01-01T00:01:00.000Z").unwrap().unwrap();
        assert_eq!(handed.phase, "handoff");
        assert_eq!(install_marker_outcome(&handed, "0.9.0", "2026-01-01T00:02:00.000Z"), InstallOutcome::Failure);
        assert_eq!(install_marker_outcome(&handed, "0.9.1", "2026-01-01T00:02:00.000Z"), InstallOutcome::Success);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn startup_marker_failure_suppresses_automatic_checks_until_retry() {
        let root = std::env::temp_dir().join(format!("synara-startup-marker-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let payload = root.join("payload.bin");
        std::fs::write(&payload, b"payload").unwrap();
        let artifact = fingerprint_artifact(&payload).unwrap();
        let path = root.join("install.json");
        let mut marker = InstallMarker::create("0.9.0", "0.9.1", now_iso(), artifact);
        marker.phase = "handoff".into();
        write_install_marker(&path, &marker).unwrap();

        let mut update = state();
        assert!(apply_install_marker_startup(&mut update, &path));
        assert_eq!(update.status, UpdateStatus::Error);
        assert_eq!(update.available_version.as_deref(), Some("0.9.1"));
        assert_eq!(update.downloaded_version, None);
        assert_eq!(update.error_context.as_deref(), Some("install"));
        assert_eq!(update.install_failure_count, 1);

        let persisted = read_install_marker(&path).unwrap().unwrap();
        assert_eq!(persisted.phase, "failed");
        assert_eq!(persisted.consecutive_failures, 1);

        let mut repeated = state();
        assert!(apply_install_marker_startup(&mut repeated, &path));
        assert_eq!(repeated.install_failure_count, 1);
        let persisted = read_install_marker(&path).unwrap().unwrap();
        assert_eq!(persisted.consecutive_failures, 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn startup_success_marker_is_cleared() {
        let root = std::env::temp_dir().join(format!("synara-startup-success-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let payload = root.join("payload.bin");
        std::fs::write(&payload, b"payload").unwrap();
        let artifact = fingerprint_artifact(&payload).unwrap();
        let path = root.join("install.json");
        let marker = InstallMarker::create("0.9.0", "0.9.0", now_iso(), artifact);
        write_install_marker(&path, &marker).unwrap();
        let mut update = state();
        assert!(!apply_install_marker_startup(&mut update, &path));
        assert!(!path.exists());
        assert_eq!(update.status, UpdateStatus::Disabled);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_install_cleans_only_its_version_cache() {
        let root = std::env::temp_dir().join(format!("synara-update-cache-clean-test-{}", uuid::Uuid::new_v4()));
        let cache = root.join("cache");
        let version_dir = cache.join("0.9.1");
        let other_dir = cache.join("0.9.2");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::create_dir_all(&other_dir).unwrap();
        let payload = version_dir.join("update.zip");
        std::fs::write(&payload, b"payload").unwrap();
        std::fs::write(other_dir.join("update.zip"), b"other").unwrap();
        let artifact = fingerprint_artifact(&payload).unwrap();
        let marker = InstallMarker::create("0.9.0", "0.9.1", now_iso(), artifact);

        cleanup_successful_install_cache(&cache, &marker);

        assert!(!version_dir.exists());
        assert!(other_dir.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_preparation_allows_one_active_attempt() {
        let coordinator = InstallPreparationCoordinator::default();
        let first = coordinator.begin().unwrap();
        assert!(coordinator.begin().is_none());
        coordinator.require_active(first).unwrap();
        coordinator.release(first).unwrap();
        assert!(coordinator.begin().is_some());
    }

    #[test]
    fn resumable_policy_classifies_ranges_and_retries() {
        let config = ResumableDownloadConfig::default();
        assert_eq!(parse_content_range_total(Some("bytes 200-1000/1001")), Some(1001));
        assert_eq!(classify_download_response(206, Some("bytes 1-2/3"), None, 1), DownloadResponseAction::Append { total: Some(3) });
        assert_eq!(classify_download_response(429, None, None, 0), DownloadResponseAction::Retryable { status: 429 });
        assert_eq!(retry_delay_ms(1, &config), 0);
        assert_eq!(retry_delay_ms(3, &config), 1000);
    }

    #[test]
    fn plist_string_value_reads_bundle_identity() {
        let plist = "<dict>\n<key>CFBundleIdentifier</key>\n<string>com.emanueledipietro.synara</string>\n<key>CFBundleExecutable</key>\n<string>synara-shell</string>\n</dict>";
        assert_eq!(plist_string_value(plist, "CFBundleIdentifier").as_deref(), Some("com.emanueledipietro.synara"));
        assert_eq!(plist_string_value(plist, "CFBundleExecutable").as_deref(), Some("synara-shell"));
    }

    #[test]
    fn shell_single_quote_escapes_paths() {
        assert_eq!(shell_single_quote("/tmp/Synara's app"), "'/tmp/Synara'\\''s app'");
    }

    #[test]
    fn packaged_feed_config_derives_channel_manifest_url() {
        let entries = parse_app_update_yml("provider: github\nowner: cmdr-chara\nrepo: synara\n").unwrap();
        let url = github_update_feed_url(&entries, UpdateFlavor::Production).unwrap();
        if cfg!(target_os = "macos") {
            assert_eq!(url, "https://github.com/cmdr-chara/synara/releases/latest/download/synara-mac.yml");
        }
        let beta = github_update_feed_url(&entries, UpdateFlavor::Beta).unwrap();
        assert!(beta.ends_with("/releases.atom"), "{beta}");
    }

    #[test]
    fn github_atom_discovers_newest_lane_tags_in_feed_order() {
        let atom = r#"<feed><entry><link href="https://github.com/o/r/releases/tag/v0.9.3-beta.2" /></entry><entry><link href="https://github.com/o/r/releases/tag/v0.9.2" /></entry><entry><link href="https://github.com/o/r/releases/tag/v0.9.3-beta.1" /></entry></feed>"#;
        assert_eq!(github_release_tags_from_atom(atom), vec!["0.9.3-beta.2", "0.9.2", "0.9.3-beta.1"]);
        assert_eq!(
            atom_tag_manifest_url("https://github.com/o/r/releases.atom", "0.9.3-beta.2", UpdateFlavor::Beta).as_deref(),
            Some("https://github.com/o/r/releases/download/v0.9.3-beta.2/beta-mac.yml")
        );
    }

    #[test]
    fn update_manifest_selects_matching_arch_and_rejects_foreign_arch() {
        let manifest = parse_update_manifest("version: 0.9.1-beta.1\nfiles:\n  - url: Synara-0.9.1-beta.1-arm64.zip\n    sha512: abc\n    size: 10\n  - url: Synara-0.9.1-beta.1-x64.zip\n    sha512: def\n").unwrap();
        assert_eq!(manifest.version, "0.9.1-beta.1");
        assert_eq!(select_update_download_file(&manifest.files, "arm64", ".zip").unwrap().sha512, "abc");
        assert!(select_update_download_file(&manifest.files, "x86_64", ".zip").is_err());
    }

}
