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
    Some(format!(
        "{protocol}://{host}/{owner}/{repo}/releases/latest/download/{}",
        update_manifest_name(flavor)
    ))
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
    std::env::var("SYNARA_DESKTOP_VERSION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

fn configured_flavor() -> UpdateFlavor {
    match std::env::var("SYNARA_UPDATE_FLAVOR").ok().as_deref() {
        Some("beta") => UpdateFlavor::Beta,
        Some("canary") => UpdateFlavor::Canary,
        Some("cua") => UpdateFlavor::Cua,
        Some("remote") => UpdateFlavor::Remote,
        _ => UpdateFlavor::Production,
    }
}

impl DesktopUpdateService {
    fn new() -> Self {
        let arch = runtime_arch();
        let home = crate::backend::configured_home_dir();
        let cache = updater_cache_dir(
            "synara-updater",
            std::env::consts::OS,
            &home,
            std::env::var_os("LOCALAPPDATA").as_deref().map(Path::new),
            std::env::var_os("XDG_CACHE_HOME").as_deref().map(Path::new),
        )
        .unwrap_or_else(|| home.join("updater"));
        let mut state = UpdateState::initial(
            current_desktop_version(),
            arch,
            arch,
            false,
            configured_flavor(),
        );
        state.enabled = configured_feed_url().is_some() && desktop_update_supported();
        state.status = if state.enabled { UpdateStatus::Idle } else { UpdateStatus::Disabled };
        state.release_url = configured_release_url();
        let restored = restore_pending_update(&cache, &state.current_version, state.flavor);
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

    fn request_check(self: &Arc<Self>) -> bool {
        let Some(state) = self.current_state() else { return false };
        if !self.supported() || !can_check(&state) {
            return false;
        }
        self.set_state(|state| state.check_start(now_iso()));
        let service = Arc::clone(self);
        std::thread::spawn(move || service.perform_check());
        true
    }

    fn perform_check(self: &Arc<Self>) {
        let result = (|| -> Result<(UpdateManifest, UpdateFeedFile), String> {
            let feed = self.feed_url.clone().ok_or("update feed is not configured")?;
            let response = ureq::get(&feed)
                .set("User-Agent", "synara-updater")
                .set("Cache-Control", "no-cache")
                .call()
                .map_err(|error| format!("Could not check for updates: {error}"))?;
            if response.status() != 200 {
                return Err(format!("Could not check for updates: update feed returned HTTP {}", response.status()));
            }
            let text = response.into_string().map_err(|error| format!("Could not read update feed: {error}"))?;
            let manifest = parse_update_manifest(&text)?;
            let extension = if cfg!(target_os = "macos") { ".zip" } else if cfg!(target_os = "windows") { ".exe" } else { ".AppImage" };
            let file = select_update_download_file(&manifest.files, runtime_arch(), extension)?.clone();
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
                }
            }
            Err(message) => self.set_state(|state| state.check_failure(message, now_iso())),
        }
    }

    fn request_download(self: &Arc<Self>) -> bool {
        let Some(state) = self.current_state() else { return false };
        if !self.supported() || state.status != UpdateStatus::Available || state.available_version.is_none() {
            return false;
        }
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
            let temporary = target.with_extension(format!("download-{}", uuid::Uuid::new_v4()));
            let response = ureq::get(&url)
                .set("User-Agent", "synara-updater")
                .set("Cache-Control", "no-cache")
                .call()
                .map_err(|error| format!("Could not download update: {error}"))?;
            if response.status() != 200 {
                return Err(format!("Could not download update: server returned HTTP {}", response.status()));
            }
            let total = response.header("Content-Length").and_then(|value| value.parse::<u64>().ok()).or(selected.size);
            let mut reader = response.into_reader();
            let mut output = File::create(&temporary).map_err(|error| format!("Could not create update download: {error}"))?;
            let mut received = 0u64;
            let mut buffer = [0u8; 128 * 1024];
            loop {
                let count = reader.read(&mut buffer).map_err(|error| format!("Could not read update download: {error}"))?;
                if count == 0 { break; }
                std::io::Write::write_all(&mut output, &buffer[..count]).map_err(|error| format!("Could not write update download: {error}"))?;
                received += count as u64;
                if let Some(total) = total.filter(|total| *total > 0) {
                    let percent = (received as f64 / total as f64) * 100.0;
                    self.set_state(|state| state.download_progress(percent));
                }
            }
            drop(output);
            if let Some(expected_size) = selected.size {
                if expected_size != received {
                    let _ = std::fs::remove_file(&temporary);
                    return Err("The update download ended before the expected payload size.".into());
                }
            }
            std::fs::rename(&temporary, &target).map_err(|error| format!("Could not stage update download: {error}"))?;
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
        if let Err(message) = prepare_macos_install(&artifact, &self.cache_dir, expected) {
            self.set_state(|state| state.install_failure(&message));
            return (false, None, Some(message));
        }
        if let Ok(mut ticket) = self.ticket.lock() { *ticket = None; }
        self.committed.store(true, std::sync::atomic::Ordering::SeqCst);
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(700));
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
        assert!(beta.contains("/beta-"), "{beta}");
    }

    #[test]
    fn update_manifest_selects_matching_arch_and_rejects_foreign_arch() {
        let manifest = parse_update_manifest("version: 0.9.1-beta.1\nfiles:\n  - url: Synara-0.9.1-beta.1-arm64.zip\n    sha512: abc\n    size: 10\n  - url: Synara-0.9.1-beta.1-x64.zip\n    sha512: def\n").unwrap();
        assert_eq!(manifest.version, "0.9.1-beta.1");
        assert_eq!(select_update_download_file(&manifest.files, "arm64", ".zip").unwrap().sha512, "abc");
        assert!(select_update_download_file(&manifest.files, "x86_64", ".zip").is_err());
    }

}
