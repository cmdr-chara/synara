//! Consent-gated reliability event spool and uploader.
//!
//! Mirrors the desktop reporter boundary: strict allowlisted codes, anonymous
//! per-event receipts, durable 0600 spool, acknowledgement-driven removal,
//! bounded retention, and no collection before an explicit sharing choice.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const ENDPOINT: &str = "https://synara-beta-diagnostics.synara-orgs.workers.dev/v1/reliability-events";
const MAX_RECORDS: usize = 500;
const MAX_EVENTS_PER_POST: usize = 50;
const MAX_BODY_BYTES: usize = 64 * 1024;
const MAX_RECORD_AGE_DAYS: i64 = 7;
const FLUSH_INTERVAL: Duration = Duration::from_secs(30);

const BETA_HOST_CODES: [&str; 6] = [
    "remote.admission_refused",
    "remote.authorization_failed",
    "remote.authorization_denied",
    "remote.authorization_subscription_required",
    "remote.tunnel_provision_failed",
    "remote.connector_exited",
];

const CODES: [&str; 29] = [
    "backend.exit_before_ready",
    "backend.exit_after_ready",
    "backend.spawn_failed",
    "backend.event_loop_stall",
    "app.renderer_crashed",
    "app.main_unhandled",
    "recovery.crash_resume_accepted",
    "recovery.crash_resume_skipped",
    "recovery.crash_resume_failed",
    "recovery.shutdown_event_timeout",
    "recovery.restart_reconciliation_failed",
    "chat.settlement_unavailable",
    "account.refresh_rate_limited",
    "account.refresh_rejected",
    "account.refresh_uncertain",
    "remote.device_revoked",
    "remote.ingress_socket_error",
    "remote.tunnel_socket_error",
    "remote.admission_refused",
    "remote.authorization_failed",
    "remote.authorization_denied",
    "remote.authorization_subscription_required",
    "remote.tunnel_provision_failed",
    "remote.connector_exited",
    "billing.redirect_failed",
    "migration.startup_blocked",
    "migration.recovery_failed",
    "provider.event_retry",
    "provider.event_quarantined",
];

// The upstream contract has 30 entries; provider.turn_start_failed is kept
// separate because the array above is also used for direct membership tests.
const EXTRA_CODES: [&str; 1] = ["provider.turn_start_failed"];

fn valid_code(code: &str) -> bool {
    CODES.contains(&code) || EXTRA_CODES.contains(&code)
}

fn code_allowed_for_channel(code: &str, channel: &str) -> bool {
    channel == "beta" || !BETA_HOST_CODES.contains(&code)
}

fn valid_version(value: &str) -> bool {
    fn numeric(part: &str) -> bool {
        !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()) && (part == "0" || !part.starts_with('0'))
    }
    let (core, suffix) = value.split_once('-').unwrap_or((value, ""));
    let mut parts = core.split('.');
    let valid_core = parts.next().is_some_and(numeric)
        && parts.next().is_some_and(numeric)
        && parts.next().is_some_and(numeric)
        && parts.next().is_none();
    if !valid_core || value.len() > 64 {
        return false;
    }
    if suffix.is_empty() {
        return !value.contains('-');
    }
    let Some((lane, number)) = suffix.split_once('.') else { return false };
    matches!(lane, "beta" | "remote" | "alpha" | "canary" | "rc") && numeric(number)
}

fn platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        "linux" => "linux",
        _ => "other",
    }
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn valid_day(day: &str) -> bool {
    chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .ok()
        .is_some_and(|date| date.format("%Y-%m-%d").to_string() == day)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ReliabilityRecord {
    v: u32,
    id: String,
    day: String,
    code: String,
    channel: String,
    #[serde(rename = "appVersion")]
    app_version: String,
    platform: String,
}

impl ReliabilityRecord {
    fn valid(&self) -> bool {
        self.v == 1
            && uuid::Uuid::parse_str(&self.id).is_ok()
            && valid_day(&self.day)
            && valid_code(&self.code)
            && matches!(self.channel.as_str(), "stable" | "beta" | "remote")
            && code_allowed_for_channel(&self.code, &self.channel)
            && valid_version(&self.app_version)
            && matches!(self.platform.as_str(), "darwin" | "win32" | "linux" | "other")
    }
}

fn current_record(day: &str) -> bool {
    let Ok(record_day) = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") else { return false };
    let today = chrono::Utc::now().date_naive();
    let age = (today - record_day).num_days();
    (0..=MAX_RECORD_AGE_DAYS).contains(&age)
}

#[derive(Default)]
struct ReporterState {
    enabled: bool,
    queue: Vec<ReliabilityRecord>,
    failures: u32,
    next_retry_ms: Option<i64>,
    next_flush_ms: Option<i64>,
    needs_persistence: bool,
}

pub struct ReliabilityReporter {
    path: PathBuf,
    endpoint: String,
    channel: String,
    app_version: String,
    state: Arc<Mutex<ReporterState>>,
}

impl ReliabilityReporter {
    pub fn new(home_dir: &Path, channel: &str, app_version: &str) -> Arc<Self> {
        Self::with_endpoint(
            home_dir,
            channel,
            app_version,
            ENDPOINT,
        )
    }

    fn with_endpoint(home_dir: &Path, channel: &str, app_version: &str, endpoint: &str) -> Arc<Self> {
        let channel = match channel {
            "beta" => "beta",
            "remote" => "remote",
            _ => "stable",
        };
        let app_version = if valid_version(app_version) { app_version } else { "0.0.0" };
        Arc::new(Self {
            path: home_dir.join("product-analytics").join("reliability.json"),
            endpoint: endpoint.to_string(),
            channel: channel.into(),
            app_version: app_version.into(),
            state: Arc::new(Mutex::new(ReporterState::default())),
        })
    }

    /// Product analytics uses an explicit consent boundary for reliability:
    /// default-on collection does not retroactively start these receipts.
    pub fn consent_is_explicitly_enabled(home_dir: &Path) -> bool {
        std::fs::read_to_string(home_dir.join("product-analytics").join("consent.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|value| value.get("enabled").and_then(Value::as_bool))
            == Some(true)
    }

    pub fn set_enabled(self: &Arc<Self>, enabled: bool, restore_pending: bool) {
        {
            let Ok(mut state) = self.state.lock() else { return };
            state.enabled = false;
            state.queue.clear();
            state.failures = 0;
            state.next_retry_ms = None;
            state.next_flush_ms = None;
            state.needs_persistence = false;
        }
        self.clear_temporary_files();
        if !enabled {
            let _ = remove_regular_file(&self.path);
            return;
        }
        if restore_pending {
            self.restore_pending();
        } else {
            let _ = self.persist(&[]);
        }
        if let Ok(mut state) = self.state.lock() {
            state.enabled = true;
        }
    }

    fn restore_pending(&self) {
        let mut restored = Vec::new();
        if let Ok(metadata) = std::fs::symlink_metadata(&self.path) {
            if metadata.is_file() && metadata.len() <= 1024 * 1024 {
                if let Ok(text) = std::fs::read_to_string(&self.path) {
                    if let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&text) {
                        for row in rows.into_iter().rev().take(MAX_RECORDS).rev() {
                            if let Ok(record) = serde_json::from_value::<ReliabilityRecord>(row) {
                                if record.valid()
                                    && record.channel == self.channel
                                    && current_record(&record.day)
                                {
                                    restored.push(record);
                                }
                            }
                        }
                    }
                }
            }
        }
        let persisted = self.persist(&restored).is_ok();
        if let Ok(mut state) = self.state.lock() {
            state.needs_persistence = !persisted;
            state.queue = restored;
        }
    }

    pub fn track(&self, code: &str) {
        if !valid_code(code) || !code_allowed_for_channel(code, &self.channel) {
            return;
        }
        let record = ReliabilityRecord {
            v: 1,
            id: uuid::Uuid::new_v4().to_string(),
            day: today(),
            code: code.into(),
            channel: self.channel.clone(),
            app_version: self.app_version.clone(),
            platform: platform().into(),
        };
        let mut queue_snapshot = Vec::new();
        if let Ok(mut state) = self.state.lock() {
            if !state.enabled {
                return;
            }
            state.queue.retain(|record| current_record(&record.day));
            state.queue.push(record);
            if state.queue.len() > MAX_RECORDS {
                let overflow = state.queue.len() - MAX_RECORDS;
                state.queue.drain(..overflow);
            }
            queue_snapshot = state.queue.clone();
        }
        let persisted = self.persist(&queue_snapshot).is_ok();
        if let Ok(mut state) = self.state.lock() {
            state.needs_persistence = !persisted;
        }
    }

    fn persist(&self, queue: &[ReliabilityRecord]) -> Result<(), String> {
        let Some(parent) = self.path.parent() else { return Err("spool path has no parent".into()) };
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let temporary = self.path.with_extension(format!("json.tmp-{}", uuid::Uuid::new_v4()));
        let result = (|| -> Result<(), String> {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary).map_err(|error| error.to_string())?;
            use std::io::Write;
            file.write_all(serde_json::to_string(queue).map_err(|error| error.to_string())?.as_bytes())
                .map_err(|error| error.to_string())?;
            file.sync_all().map_err(|error| error.to_string())?;
            std::fs::rename(&temporary, &self.path).map_err(|error| error.to_string())?;
            Ok(())
        })();
        let _ = remove_regular_file(&temporary);
        result
    }

    fn clear_temporary_files(&self) {
        let Some(parent) = self.path.parent() else { return };
        let Ok(entries) = std::fs::read_dir(parent) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with("reliability.json.tmp-") {
                let _ = remove_regular_file(&entry.path());
            }
        }
    }

    fn batch(&self) -> Vec<ReliabilityRecord> {
        let pending_snapshot = {
            let Ok(mut state) = self.state.lock() else { return Vec::new() };
            if !state.enabled {
                return Vec::new();
            }
            state.queue.retain(|record| current_record(&record.day));
            state.needs_persistence.then(|| state.queue.clone())
        };
        if let Some(snapshot) = pending_snapshot {
            if self.persist(&snapshot).is_ok() {
                if let Ok(mut state) = self.state.lock() {
                    state.needs_persistence = false;
                }
            }
        }
        let Ok(mut state) = self.state.lock() else { return Vec::new() };
        if !state.enabled {
            return Vec::new();
        }
        let mut batch = Vec::new();
        for record in &state.queue {
            if batch.len() >= MAX_EVENTS_PER_POST {
                break;
            }
            let mut candidate = batch.clone();
            candidate.push(record.clone());
            if serde_json::to_string(&json!({"events": candidate})).map(|text| text.len()).unwrap_or(usize::MAX) > MAX_BODY_BYTES {
                break;
            }
            batch.push(record.clone());
        }
        batch
    }

    fn flush_once(&self) -> Result<(), String> {
        let batch = self.batch();
        if batch.is_empty() {
            return Ok(());
        }
        let response = ureq::post(&self.endpoint)
            .timeout(Duration::from_secs(8))
            .set("content-type", "application/json")
            .send_json(json!({"events": batch}))
            .map_err(|error| format!("reliability report was not acknowledged: {error}"))?;
        if response.status() < 200 || response.status() >= 300 {
            return Err("reliability report was not acknowledged".into());
        }
        let body: Value = response.into_json().map_err(|error| error.to_string())?;
        if body.get("stored").and_then(Value::as_bool) != Some(true) {
            return Err("reliability receipt missing".into());
        }
        let sent: std::collections::HashSet<_> = batch.iter().map(|record| record.id.as_str()).collect();
        let acknowledged: std::collections::HashSet<String> = body
            .get("acknowledged")
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(Value::as_str)
                    .filter(|id| sent.contains(id))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if acknowledged.is_empty() {
            return Err("reliability receipt missing".into());
        }
        let mut queue_snapshot = Vec::new();
        if let Ok(mut state) = self.state.lock() {
            if !state.enabled {
                return Ok(());
            }
            state.queue.retain(|record| !acknowledged.contains(&record.id));
            state.failures = 0;
            state.next_retry_ms = if acknowledged.len() < batch.len() {
                Some(now_ms() + 1_000)
            } else {
                None
            };
            queue_snapshot = state.queue.clone();
        }
        let persisted = self.persist(&queue_snapshot).is_ok();
        if let Ok(mut state) = self.state.lock() {
            state.needs_persistence = !persisted;
        }
        Ok(())
    }

    fn schedule_failure(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.failures = state.failures.saturating_add(1);
            let exponent = state.failures.saturating_sub(1).min(6);
            let delay_ms = 1_000i64.saturating_mul(1i64 << exponent).min(60_000);
            state.next_retry_ms = Some(now_ms() + delay_ms);
        }
    }

    pub fn start(self: &Arc<Self>) {
        let reporter = Arc::clone(self);
        std::thread::spawn(move || loop {
            let should_flush = reporter
                .state
                .lock()
                .ok()
                .map(|state| {
                    if !state.enabled {
                        return false;
                    }
                    let now = now_ms();
                    state.next_retry_ms.is_some_and(|retry| retry <= now)
                        || state.next_flush_ms.is_none_or(|flush| flush <= now)
                })
                .unwrap_or(false);
            if should_flush {
                if reporter.flush_once().is_err() {
                    reporter.schedule_failure();
                } else if let Ok(mut state) = reporter.state.lock() {
                    let retry = state.next_retry_ms.unwrap_or(i64::MAX);
                    state.next_flush_ms = Some((now_ms() + 30_000).min(retry));
                }
            }
            std::thread::sleep(FLUSH_INTERVAL.min(Duration::from_millis(500)));
        });
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn remove_regular_file(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() || metadata.is_symlink() => {
            std::fs::remove_file(path).map_err(|error| error.to_string())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

static REPORTER: OnceLock<Arc<ReliabilityReporter>> = OnceLock::new();

fn reporter() -> Option<&'static Arc<ReliabilityReporter>> {
    REPORTER.get()
}

pub fn initialize() {
    let home = crate::backend::configured_home_dir();
    let flavor = crate::desktop_identity::runtime_flavor();
    let channel = match flavor {
        "beta" => "beta",
        "remote" => "remote",
        _ => "stable",
    };
    let version = std::env::var("SYNARA_DESKTOP_VERSION").unwrap_or_else(|_| "0.1.0".into());
    let reporter = REPORTER.get_or_init(|| ReliabilityReporter::new(&home, channel, &version));
    reporter.set_enabled(ReliabilityReporter::consent_is_explicitly_enabled(&home), true);
    reporter.start();
}

pub fn track(code: &str) {
    if let Some(reporter) = reporter() {
        reporter.track(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};

    #[test]
    fn validates_versions_and_channel_codes() {
        assert!(valid_version("1.2.3"));
        assert!(valid_version("1.2.3-beta.4"));
        assert!(!valid_version("01.2.3"));
        assert!(code_allowed_for_channel("remote.connector_exited", "beta"));
        assert!(!code_allowed_for_channel("remote.connector_exited", "stable"));
        assert!(valid_code("provider.turn_start_failed"));
    }

    #[test]
    fn spool_round_trips_only_valid_current_records() {
        let home = std::env::temp_dir().join(format!("synara-reliability-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        let reporter = ReliabilityReporter::with_endpoint(&home, "beta", "1.2.3", "http://127.0.0.1:1");
        reporter.set_enabled(true, false);
        reporter.track("backend.spawn_failed");
        reporter.set_enabled(true, true);
        let state = reporter.state.lock().unwrap();
        assert_eq!(state.queue.len(), 1);
        assert_eq!(state.queue[0].code, "backend.spawn_failed");
        drop(state);
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn acknowledged_upload_removes_records() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request_bytes = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let size = stream.read(&mut buffer).unwrap();
                assert!(size > 0, "connection closed before the request completed");
                request_bytes.extend_from_slice(&buffer[..size]);
                if let Some(header_end) = request_bytes
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&request_bytes[..header_end]);
                    let content_length = headers
                        .split("\r\n")
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            (name.trim().eq_ignore_ascii_case("content-length"))
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    if request_bytes.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
            }
            let request = String::from_utf8_lossy(&request_bytes);
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("{}");
            let parsed: Value = serde_json::from_str(body).unwrap();
            let id = parsed["events"][0]["id"].as_str().unwrap().to_string();
            let payload = serde_json::to_string(&json!({
                "stored": true,
                "acknowledged": [id],
            })).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                payload.len(),
                payload
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let home = std::env::temp_dir().join(format!("synara-reliability-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        let reporter = ReliabilityReporter::with_endpoint(
            &home,
            "stable",
            "1.2.3",
            &format!("http://127.0.0.1:{port}"),
        );
        reporter.set_enabled(true, false);
        reporter.track("backend.spawn_failed");
        reporter.flush_once().unwrap();
        assert!(reporter.state.lock().unwrap().queue.is_empty());
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(home);
    }

}
