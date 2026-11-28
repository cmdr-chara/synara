//! Beta/Remote diagnostics spool and uploader.
//!
//! Privacy boundary: stable/canary/cua never construct this reporter; every
//! payload is rebuilt through the generated upstream allowlist sanitizer before
//! disk retention or upload. There is intentionally no free-form metadata bag.

use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const ENDPOINT: &str = "https://synara-beta-diagnostics.synara-orgs.workers.dev";
const ENDPOINT_ENV: &str = "SYNARA_BETA_DIAGNOSTICS_URL";
const QUEUE_MAX_BYTES: u64 = 1024 * 1024;
const QUEUE_TRIM_TARGET_BYTES: u64 = 512 * 1024;
const FLUSH_BATCH_MAX_EVENTS: usize = 200;
const FLUSH_BATCH_MAX_BYTES: usize = 256 * 1024;
const FLUSH_INTERVAL: Duration = Duration::from_secs(5 * 60);
const ERROR_FINGERPRINT_WINDOW_MS: u64 = 10 * 60 * 1000;
const ERROR_HOURLY_CAP: usize = 30;
const ISSUE_MAX_DURATION_MS: u64 = 604_800_000;
const CONTEXT_MAX_ENTRIES: usize = 24;
const CONTEXT_MAX_AGE_MS: u64 = 10 * 60 * 1000;
const MEMORY_SAMPLE_INTERVAL: Duration = Duration::from_secs(30);
const DIAGNOSTICS_STACK_MAX_LENGTH: usize = 8 * 1024;
const DIAGNOSTICS_LOG_TAIL_MAX_LENGTH: usize = 16 * 1024;

const EVENTS: [&str; 13] = [
    "app.start",
    "app.exit",
    "app.error",
    "app.renderer-crash",
    "app.child-process-crash",
    "update.check",
    "update.available",
    "update.downloaded",
    "update.installed",
    "update.error",
    "usage.daily",
    "beta.installed",
    "beta.left",
];

fn platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        "linux" => "linux",
        _ => "unknown",
    }
}

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        "x86" => "ia32",
        "arm" => "arm",
        _ => "unknown",
    }
}

fn read_launch_version(home_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(home_dir.join("last-launch-version.json")).ok()?;
    serde_json::from_str::<Value>(&text)
        .ok()?
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn write_launch_version(home_dir: &Path, version: &str) {
    let body = format!(
        "{}\n",
        serde_json::to_string(&json!({"version": version})).unwrap_or_else(|_| "{}".into())
    );
    let _ = write_private_atomic(&home_dir.join("last-launch-version.json"), body.as_bytes());
}
fn os_version() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|value| !value.is_empty())
    }
    #[cfg(target_os = "linux")]
    {
        let release = std::fs::read_to_string("/etc/os-release").ok()?;
        release.lines().find_map(|line| {
            line.strip_prefix("VERSION_ID=")
                .map(|value| value.trim_matches('"').to_string())
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        std::env::var("OS").ok().filter(|value| !value.is_empty())
    }
}

fn locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .map(|value| value.split('.').next().unwrap_or(&value).replace('_', "-"))
        .filter(|value| !value.is_empty() && value != "C" && value != "POSIX")
}

fn valid_version(value: &str) -> bool {
    let (core, suffix) = value.split_once('-').unwrap_or((value, ""));
    let numeric = |part: &str| {
        !part.is_empty() && part.len() <= 8 && part.bytes().all(|b| b.is_ascii_digit())
    };
    let mut parts = core.split('.');
    let valid = parts.next().is_some_and(numeric)
        && parts.next().is_some_and(numeric)
        && parts.next().is_some_and(numeric)
        && parts.next().is_none();
    valid
        && (suffix.is_empty() && !value.contains('-')
            || suffix.split_once('.').is_some_and(|(lane, n)| {
                matches!(lane, "beta" | "remote" | "canary") && numeric(n)
            }))
}

fn resolve_endpoint() -> String {
    let Ok(raw) = std::env::var(ENDPOINT_ENV) else {
        return ENDPOINT.into();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return ENDPOINT.into();
    }
    if let Ok(url) = url::Url::parse(trimmed) {
        if url.scheme() == "https" {
            return trimmed.into();
        }
        if url.scheme() == "http"
            && matches!(
                url.host_str(),
                Some("127.0.0.1") | Some("localhost") | Some("[::1]")
            )
        {
            return trimmed.into();
        }
    }
    ENDPOINT.into()
}

fn write_private_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err("diagnostics path has no parent".into());
    };
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        use std::io::Write as _;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        std::fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn load_install_id(home_dir: &Path) -> String {
    let diagnostics = home_dir.join("diagnostics");
    let path = diagnostics.join("install-id");
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let existing = existing.trim();
        if uuid::Uuid::parse_str(existing).is_ok() {
            return existing.into();
        }
    }
    let generated = uuid::Uuid::new_v4().to_string();
    let _ = write_private_atomic(&path, format!("{generated}\n").as_bytes());
    let _ = write_private_atomic(
        &diagnostics.join("install-pending"),
        format!("{}\n", chrono::Utc::now().to_rfc3339()).as_bytes(),
    );
    generated
}

fn payload_kind(event: &str) -> Option<String> {
    match event {
        "app.start" | "app.exit" => Some("lifecycle".into()),
        "app.error" => Some("error".into()),
        "app.renderer-crash" | "app.child-process-crash" => Some("crash".into()),
        "update.check" | "update.available" | "update.downloaded" | "update.installed"
        | "update.error" => Some("update".into()),
        "usage.daily" => Some("usage".into()),
        "beta.installed" | "beta.left" => Some("beta".into()),
        _ => None,
    }
}

fn sanitize_payload(payload: Value, home_dir: &Path, event: &str) -> Result<Value, String> {
    betadiagnostics_rs::port::sanitize_beta_diagnostics_payload(
        payload,
        Some(home_dir.to_string_lossy().into_owned()),
        Some(event.to_string()),
    )
}

fn valid_record(record: &Value, flavor: &str, install_id: &str, home_dir: &Path) -> Option<String> {
    let event = record.get("event").and_then(Value::as_str)?;
    if !EVENTS.contains(&event) {
        return None;
    }
    let kind = payload_kind(event)?;
    if record.get("v").and_then(Value::as_u64) != Some(1)
        || record
            .get("payload")
            .and_then(Value::as_object)?
            .get("kind")
            .and_then(Value::as_str)
            != Some(kind.as_str())
        || record.get("flavor").and_then(Value::as_str) != Some(flavor)
        || record.get("installId").and_then(Value::as_str) != Some(install_id)
        || record
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .is_none()
    {
        return None;
    }
    let version = record.get("appVersion").and_then(Value::as_str)?;
    if !valid_version(version) {
        return None;
    }
    let ts = record.get("ts").and_then(Value::as_str)?;
    let parsed = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
    let payload = sanitize_payload(record.get("payload")?.clone(), home_dir, event).ok()?;
    if payload.get("kind").and_then(Value::as_str) == Some("beta")
        && payload.get("outcome").is_none()
    {
        return None;
    }
    let platform = match record.get("platform").and_then(Value::as_str) {
        Some("darwin" | "win32" | "linux") => record
            .get("platform")
            .and_then(Value::as_str)
            .unwrap()
            .to_string(),
        _ => "unknown".into(),
    };
    let arch = match record.get("arch").and_then(Value::as_str) {
        Some("arm64" | "x64" | "ia32" | "arm") => record
            .get("arch")
            .and_then(Value::as_str)
            .unwrap()
            .to_string(),
        _ => "unknown".into(),
    };
    serde_json::to_string(&json!({
        "v": 1,
        "id": record.get("id"),
        "ts": parsed.to_utc().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "installId": install_id,
        "appVersion": version,
        "flavor": flavor,
        "platform": platform,
        "arch": arch,
        "event": event,
        "payload": payload,
    }))
    .ok()
}

const ISSUE_CODES: [&str; 19] = [
    "server.event-loop.stall",
    "git.request.failed",
    "git.branch.failed",
    "git.commit.failed",
    "git.push.failed",
    "git.pr.failed",
    "voice.record.failed",
    "voice.transcribe.failed",
    "claude.compaction.request-failed",
    "claude.compaction.uncertain",
    "claude.cache.request-failed",
    "claude.cache.failed",
    "claude.cache.uncertain",
    "startup.database-locked",
    "startup.migration-recovery-required",
    "startup.migration-divergence-consent-required",
    "startup.migration-runtime-identity-mismatch",
    "startup.migration-schema-too-new",
    "startup.migration-startup-block-invalid",
];
const ISSUE_REASONS: [&str; 7] = [
    "unknown",
    "auth",
    "invalid-response",
    "timeout",
    "output-limit",
    "live-owner",
    "unknown-owner",
];

/// FNV-1a over UTF-16 code units, the upstream handled-issue fingerprint.
fn diagnostic_error_fingerprint(message: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for unit in message.encode_utf16() {
        hash = (hash ^ u64::from(unit)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

const ACTIVITIES: [&str; 17] = [
    "chat.send",
    "chat.stop",
    "chat.open",
    "chat.unblock",
    "workspace.change",
    "project.create",
    "project.import",
    "project.import.preview",
    "project.import.catalog",
    "workspace.search",
    "workspace.read",
    "browser.open",
    "browser.navigate",
    "browser.resize",
    "window.resize",
    "transport.reconnect",
    "renderer.ready",
];

#[derive(Clone, Copy, Debug, Default)]
struct MemoryCounters {
    main_rss_mb: f64,
    renderer_rss_mb: f64,
    gpu_rss_mb: f64,
    utility_rss_mb: f64,
}

#[derive(Clone, Copy, Debug)]
struct MemorySample {
    at_ms: u64,
    counters: MemoryCounters,
}

#[derive(Clone, Debug)]
struct Breadcrumb {
    activity: String,
    phase: String,
    at_ms: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn iso_at(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".into())
}

/// Best-effort shell RSS. GPUI has no Electron renderer/GPU/utility process
/// split, so those upstream categories stay zero rather than inventing data.
fn sample_memory_counters() -> MemoryCounters {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output();
    let rss_kb = output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| text.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0);
    MemoryCounters {
        main_rss_mb: rss_kb / 1024.0,
        renderer_rss_mb: 0.0,
        gpu_rss_mb: 0.0,
        utility_rss_mb: 0.0,
    }
}

struct FlushReset<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for FlushReset<'_> {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

pub struct BetaDiagnosticsReporter {
    home_dir: PathBuf,
    queue_path: PathBuf,
    endpoint: String,
    install_id: String,
    flavor: String,
    app_version: String,
    enabled: Arc<Mutex<bool>>,
    error_sent_timestamps: Mutex<VecDeque<u64>>,
    error_fingerprint_sent_at: Mutex<HashMap<String, u64>>,
    issue_reports: Mutex<HashMap<String, (String, u64)>>,
    report_statuses: Mutex<HashMap<String, &'static str>>,
    recent_activity: Mutex<Vec<Breadcrumb>>,
    memory_samples: Mutex<Vec<MemorySample>>,
    disposed: std::sync::atomic::AtomicBool,
    flushing: std::sync::atomic::AtomicBool,
}

impl BetaDiagnosticsReporter {
    /// Only Beta and Remote have diagnostics. Remote starts conservatively and
    /// requires the product-analytics sharing switch; Beta follows the upstream
    /// diagnostics-informed build consent.
    pub fn create(home_dir: &Path, flavor: &str, app_version: &str) -> Option<Arc<Self>> {
        if !matches!(flavor, "beta" | "remote") {
            return None;
        }
        // Remote follows the live product-sharing gate dynamically; a saved opt-out
        // is already reflected there. Do not freeze the startup consent snapshot.
        let enabled = true;
        Some(Arc::new(Self {
            home_dir: home_dir.to_path_buf(),
            queue_path: home_dir.join("diagnostics").join("events.jsonl"),
            endpoint: resolve_endpoint(),
            install_id: load_install_id(home_dir),
            flavor: flavor.into(),
            app_version: if valid_version(app_version) {
                app_version.into()
            } else {
                "0.0.0".into()
            },
            enabled: Arc::new(Mutex::new(enabled)),
            error_sent_timestamps: Mutex::new(VecDeque::new()),
            error_fingerprint_sent_at: Mutex::new(HashMap::new()),
            issue_reports: Mutex::new(HashMap::new()),
            report_statuses: Mutex::new(HashMap::new()),
            recent_activity: Mutex::new(Vec::new()),
            memory_samples: Mutex::new(Vec::new()),
            disposed: std::sync::atomic::AtomicBool::new(false),
            flushing: std::sync::atomic::AtomicBool::new(false),
        }))
    }

    fn is_enabled(&self) -> bool {
        if self.disposed.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }
        let local = self.enabled.lock().map(|enabled| *enabled).unwrap_or(false);
        if self.flavor == "remote" {
            local && crate::product_analytics::enabled()
        } else {
            local
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut current) = self.enabled.lock() {
            *current = enabled;
        }
        if !enabled {
            let _ = std::fs::remove_file(&self.queue_path);
        }
    }

    pub fn track(&self, event: &str, payload: Value) -> Option<String> {
        if !self.is_enabled() || !EVENTS.contains(&event) {
            return None;
        }
        let kind = payload_kind(event)?;
        if payload.get("kind").and_then(Value::as_str) != Some(kind.as_str()) {
            return None;
        }
        let payload = match payload.get("kind").and_then(Value::as_str) {
            Some("crash") => {
                let mut payload = payload;
                if let Some(log_tail) = payload.get("logTail").and_then(Value::as_str) {
                    payload["logTail"] = json!(self.attach_context(
                        Some(log_tail),
                        DIAGNOSTICS_LOG_TAIL_MAX_LENGTH,
                        true
                    ));
                } else if let Some(context) =
                    self.attach_context(None, DIAGNOSTICS_LOG_TAIL_MAX_LENGTH, true)
                {
                    payload["logTail"] = json!(context);
                }
                payload
            }
            Some("error") => {
                let mut payload = payload;
                if let Some(stack) = payload.get("stack").and_then(Value::as_str) {
                    payload["stack"] = json!(self.attach_context(
                        Some(stack),
                        DIAGNOSTICS_STACK_MAX_LENGTH,
                        false
                    ));
                } else if let Some(context) =
                    self.attach_context(None, DIAGNOSTICS_STACK_MAX_LENGTH, false)
                {
                    payload["stack"] = json!(context);
                }
                payload
            }
            _ => payload,
        };
        let payload = sanitize_payload(payload, &self.home_dir, event).ok()?;
        if payload.get("kind").and_then(Value::as_str) == Some("beta")
            && payload.get("outcome").is_none()
        {
            return None;
        }
        let id = uuid::Uuid::new_v4().to_string();
        let record = json!({
            "v": 1,
            "id": id,
            "ts": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "installId": self.install_id,
            "appVersion": self.app_version,
            "flavor": self.flavor,
            "platform": platform(),
            "arch": arch(),
            "event": event,
            "payload": payload,
        });
        let line = serde_json::to_string(&record).ok()?;
        if let Some(parent) = self.queue_path.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        let mut options = std::fs::OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        use std::io::Write as _;
        let mut file = options.open(&self.queue_path).ok()?;
        writeln!(file, "{line}").ok()?;
        self.trim_queue();
        Some(id)
    }

    fn scrubbed_lines(&self) -> Vec<String> {
        let Ok(raw) = std::fs::read_to_string(&self.queue_path) else {
            return Vec::new();
        };
        raw.lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                serde_json::from_str::<Value>(line).ok().and_then(|record| {
                    valid_record(&record, &self.flavor, &self.install_id, &self.home_dir)
                })
            })
            .collect()
    }

    fn rewrite_queue(&self, lines: &[String]) {
        if lines.is_empty() {
            let _ = std::fs::remove_file(&self.queue_path);
        } else {
            let body = format!("{}\n", lines.join("\n"));
            let _ = write_private_atomic(&self.queue_path, body.as_bytes());
        }
    }

    fn trim_queue(&self) {
        let Ok(metadata) = std::fs::metadata(&self.queue_path) else {
            return;
        };
        if metadata.len() <= QUEUE_MAX_BYTES {
            return;
        }
        let lines = self.scrubbed_lines();
        let mut kept = Vec::new();
        let mut bytes = 0usize;
        for line in lines.iter().rev() {
            if bytes + line.len() + 1 > QUEUE_TRIM_TARGET_BYTES as usize && !kept.is_empty() {
                break;
            }
            bytes += line.len() + 1;
            kept.push(line.clone());
        }
        kept.reverse();
        self.rewrite_queue(&kept);
        let kept_ids = kept
            .iter()
            .filter_map(|line| {
                serde_json::from_str::<Value>(line)
                    .ok()?
                    .get("id")?
                    .as_str()
                    .map(str::to_string)
            })
            .collect::<std::collections::HashSet<_>>();
        if let Ok(mut statuses) = self.report_statuses.lock() {
            for (id, status) in statuses.iter_mut() {
                if *status == "queued" && !kept_ids.contains(id) {
                    *status = "unavailable";
                }
            }
        }
    }

    /// Synchronously upload the oldest valid batch. Successfully accepted ids
    /// are removed by id so concurrent appends and head trims cannot be lost.
    pub fn flush(&self) {
        if !self.is_enabled() {
            return;
        }
        if self
            .flushing
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            return;
        }
        let _reset = FlushReset(&self.flushing);
        let lines = self.scrubbed_lines();
        self.rewrite_queue(&lines);
        let mut batch = Vec::new();
        let mut bytes = 0usize;
        for line in lines.into_iter().take(FLUSH_BATCH_MAX_EVENTS) {
            if bytes + line.len() + 1 > FLUSH_BATCH_MAX_BYTES && !batch.is_empty() {
                break;
            }
            bytes += line.len() + 1;
            batch.push(line);
        }
        if batch.is_empty() {
            return;
        }
        let body = format!("{}\n", batch.join("\n"));
        let response = ureq::post(&format!("{}/v1/events", self.endpoint))
            .timeout(Duration::from_secs(15))
            .set("content-type", "application/x-ndjson")
            .send_string(&body);
        if response.is_err() {
            return;
        }
        let sent: std::collections::HashSet<String> = batch
            .iter()
            .filter_map(|line| {
                serde_json::from_str::<Value>(line)
                    .ok()?
                    .get("id")?
                    .as_str()
                    .map(str::to_string)
            })
            .collect();
        if let Ok(mut statuses) = self.report_statuses.lock() {
            for id in &sent {
                if statuses.contains_key(id) {
                    statuses.insert(id.clone(), "sent");
                }
            }
        }
        let rest = self
            .scrubbed_lines()
            .into_iter()
            .filter(|line| {
                serde_json::from_str::<Value>(line)
                    .ok()
                    .and_then(|record| record.get("id")?.as_str().map(str::to_string))
                    .is_none_or(|id| !sent.contains(&id))
            })
            .collect::<Vec<_>>();
        self.rewrite_queue(&rest);
    }

    fn allow_error(&self, fingerprint: &str, record: bool, now_ms: u64) -> bool {
        let Ok(mut timestamps) = self.error_sent_timestamps.lock() else {
            return false;
        };
        while timestamps
            .front()
            .is_some_and(|at| now_ms.saturating_sub(*at) > 60 * 60 * 1000)
        {
            timestamps.pop_front();
        }
        if timestamps.len() >= ERROR_HOURLY_CAP {
            return false;
        }
        let Ok(mut sent_at) = self.error_fingerprint_sent_at.lock() else {
            return false;
        };
        if sent_at
            .get(fingerprint)
            .is_some_and(|at| now_ms.saturating_sub(*at) < ERROR_FINGERPRINT_WINDOW_MS)
        {
            return false;
        }
        if !record {
            return true;
        }
        sent_at.insert(fingerprint.into(), now_ms);
        if sent_at.len() > 256 {
            sent_at.retain(|_, at| now_ms.saturating_sub(*at) <= ERROR_FINGERPRINT_WINDOW_MS);
        }
        timestamps.push_back(now_ms);
        true
    }

    /// Fixed-category renderer/application breadcrumb ring. Inputs are
    /// reconstructed field-by-field so payloads never carry free-form data.
    pub fn record_activity(&self, input: &Value) {
        if !self.is_enabled() {
            return;
        }
        let Some(activity) = input.get("activity").and_then(Value::as_str) else {
            return;
        };
        let Some(phase) = input.get("phase").and_then(Value::as_str) else {
            return;
        };
        if !ACTIVITIES.contains(&activity) || !matches!(phase, "started" | "succeeded" | "failed") {
            return;
        }
        let at_ms = now_ms();
        let Ok(mut activity_log) = self.recent_activity.lock() else {
            return;
        };
        if activity_log.last().is_some_and(|last| {
            last.activity == activity
                && last.phase == phase
                && at_ms.saturating_sub(last.at_ms) < 1000
        }) {
            return;
        }
        activity_log.push(Breadcrumb {
            activity: activity.into(),
            phase: phase.into(),
            at_ms,
        });
        if activity_log.len() > CONTEXT_MAX_ENTRIES {
            let overflow = activity_log.len() - CONTEXT_MAX_ENTRIES;
            activity_log.drain(..overflow);
        }
    }

    fn capture_memory_sample(&self) {
        if !self.is_enabled() {
            return;
        }
        let counters = sample_memory_counters();
        if ![
            counters.main_rss_mb,
            counters.renderer_rss_mb,
            counters.gpu_rss_mb,
            counters.utility_rss_mb,
        ]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0 && *value <= 16_777_216.0)
        {
            return;
        }
        if let Ok(mut samples) = self.memory_samples.lock() {
            samples.push(MemorySample {
                at_ms: now_ms(),
                counters,
            });
            while samples.len() > 4 {
                samples.remove(0);
            }
        }
    }

    fn truncate_chars(text: &str, limit: usize, keep_tail: bool) -> String {
        if text.chars().count() <= limit {
            return text.to_string();
        }
        if keep_tail {
            text.chars()
                .rev()
                .take(limit)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        } else {
            text.chars().take(limit).collect()
        }
    }

    fn attach_context(&self, text: Option<&str>, limit: usize, keep_tail: bool) -> Option<String> {
        let cutoff = now_ms().saturating_sub(CONTEXT_MAX_AGE_MS);
        let mut lines = Vec::new();
        if let Ok(samples) = self.memory_samples.lock() {
            for sample in samples.iter().filter(|sample| sample.at_ms >= cutoff) {
                lines.push(format!(
                    "{} memory MiB main={} renderer={} gpu={} utility={}",
                    iso_at(sample.at_ms),
                    sample.counters.main_rss_mb.round() as u64,
                    sample.counters.renderer_rss_mb.round() as u64,
                    sample.counters.gpu_rss_mb.round() as u64,
                    sample.counters.utility_rss_mb.round() as u64,
                ));
            }
        }
        if let Ok(activity_log) = self.recent_activity.lock() {
            for entry in activity_log.iter().filter(|entry| entry.at_ms >= cutoff) {
                lines.push(format!(
                    "{} {} {}",
                    iso_at(entry.at_ms),
                    entry.activity,
                    entry.phase
                ));
            }
        }
        if lines.is_empty() {
            return text.map(str::to_string);
        }
        let context = format!(
            "
[Beta diagnostic context]
{}",
            lines.join(
                "
"
            )
        );
        let available = limit.saturating_sub(context.chars().count());
        let original = Self::truncate_chars(text.unwrap_or(""), available, keep_tail);
        Some(format!("{original}{context}"))
    }

    pub fn report_status(&self, id: &str) -> &'static str {
        self.report_statuses
            .lock()
            .ok()
            .and_then(|statuses| statuses.get(id).copied())
            .unwrap_or("unavailable")
    }

    /// Reuse the bounded app.error envelope for handled operational issues.
    /// Input is untrusted backend output, so only exact fixed categories pass.
    pub fn track_issue(&self, source: &str, input: &Value) -> Option<String> {
        if source != "main" && source != "renderer" {
            return None;
        }
        let code = input.get("code").and_then(Value::as_str)?;
        if !ISSUE_CODES.contains(&code) {
            return None;
        }
        let reason = input.get("reason").and_then(Value::as_str);
        if reason.is_some_and(|value| !ISSUE_REASONS.contains(&value)) {
            return None;
        }
        let duration = input.get("durationMs").and_then(Value::as_u64);
        if input.get("durationMs").is_some() && duration.is_none() {
            return None;
        }
        if duration.is_some_and(|value| value > ISSUE_MAX_DURATION_MS) {
            return None;
        }
        let message = format!("Handled issue: {code} ({})", reason.unwrap_or("unknown"));
        let fingerprint = diagnostic_error_fingerprint(&message);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as u64;
        if let Ok(reports) = self.issue_reports.lock() {
            if let Some((id, at)) = reports.get(&fingerprint) {
                if now_ms.saturating_sub(*at) < ERROR_FINGERPRINT_WINDOW_MS
                    && self.report_status(id) != "unavailable"
                {
                    return Some(id.clone());
                }
            }
        }
        if !self.allow_error(&fingerprint, false, now_ms) {
            return None;
        }
        let mut payload = json!({
            "kind":"error",
            "source":source,
            "message":message,
            "fingerprint":fingerprint,
        });
        if let Some(duration) = duration {
            payload["stack"] = json!(format!("durationMs={duration}"));
        }
        let id = self.track("app.error", payload)?;
        let _ = self.allow_error(&fingerprint, true, now_ms);
        if let Ok(mut statuses) = self.report_statuses.lock() {
            statuses.insert(id.clone(), "queued");
            if statuses.len() > 128 {
                if let Some(oldest) = statuses.keys().next().cloned() {
                    statuses.remove(&oldest);
                }
            }
        }
        if let Ok(mut reports) = self.issue_reports.lock() {
            reports.insert(fingerprint, (id.clone(), now_ms));
            if reports.len() > 128 {
                let cutoff = now_ms.saturating_sub(ERROR_FINGERPRINT_WINDOW_MS);
                reports.retain(|_, (_, at)| *at >= cutoff);
            }
        }
        Some(id)
    }

    fn resume_sharing(&self) {
        let directory = self.home_dir.join("diagnostics");
        if std::fs::create_dir_all(&directory).is_ok() {
            let _ = write_private_atomic(
                &directory.join("sharing-resumed-at"),
                format!(
                    "{}
",
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                )
                .as_bytes(),
            );
        }
    }

    fn discard_queue(&self) {
        for path in [
            self.queue_path.clone(),
            self.home_dir
                .join("diagnostics")
                .join("usage-snapshot.json"),
        ] {
            let _ = std::fs::remove_file(path);
        }
        if let Ok(mut reports) = self.issue_reports.lock() {
            reports.clear();
        }
        if let Ok(mut statuses) = self.report_statuses.lock() {
            statuses.clear();
        }
        if let Ok(mut activity_log) = self.recent_activity.lock() {
            activity_log.clear();
        }
        if let Ok(mut samples) = self.memory_samples.lock() {
            samples.clear();
        }
        if let Ok(mut timestamps) = self.error_sent_timestamps.lock() {
            timestamps.clear();
        }
        if let Ok(mut fingerprints) = self.error_fingerprint_sent_at.lock() {
            fingerprints.clear();
        }
    }

    fn track_daily_usage(&self) {
        let path = self
            .home_dir
            .join("diagnostics")
            .join("usage-snapshot.json");
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return;
        };
        let Some(generated) = value.get("generatedAt").and_then(Value::as_str) else {
            return;
        };
        let Ok(generated_at) = chrono::DateTime::parse_from_rfc3339(generated) else {
            return;
        };
        let age =
            chrono::Utc::now().signed_duration_since(generated_at.with_timezone(&chrono::Utc));
        if age < chrono::Duration::zero() || age > chrono::Duration::hours(12) {
            return;
        }
        let resumed_path = self.home_dir.join("diagnostics").join("sharing-resumed-at");
        if let Ok(resumed_text) = std::fs::read_to_string(&resumed_path) {
            if let Ok(resumed_at) = chrono::DateTime::parse_from_rfc3339(resumed_text.trim()) {
                if generated_at.with_timezone(&chrono::Utc) - chrono::Duration::hours(24)
                    < resumed_at.with_timezone(&chrono::Utc)
                {
                    return;
                }
            } else {
                return;
            }
        }
        let day = generated_at.format("%Y-%m-%d").to_string();
        let marker = self.home_dir.join("diagnostics").join("usage-last-day");
        if std::fs::read_to_string(&marker)
            .ok()
            .is_some_and(|last| last.trim() == day)
        {
            return;
        }
        let payload = json!({
            "kind":"usage",
            "projects": value.get("projects").and_then(Value::as_u64).unwrap_or(0),
            "activeThreads": value.get("activeThreads").and_then(Value::as_u64).unwrap_or(0),
        });
        if self.track("usage.daily", payload).is_some() {
            let _ = write_private_atomic(
                &marker,
                format!(
                    "{day}
"
                )
                .as_bytes(),
            );
        }
    }

    pub fn start(self: &Arc<Self>) {
        self.capture_memory_sample();
        let memory_reporter = Arc::clone(self);
        std::thread::spawn(move || loop {
            std::thread::sleep(MEMORY_SAMPLE_INTERVAL);
            memory_reporter.capture_memory_sample();
        });
        let reporter = Arc::clone(self);
        std::thread::spawn(move || loop {
            std::thread::sleep(FLUSH_INTERVAL);
            reporter.track_daily_usage();
            reporter.flush();
        });
    }

    /// Best-effort final flush bounded for application shutdown. The blocking
    /// ureq call runs on a helper thread so a hung endpoint cannot stall quit.
    pub fn dispose(self: &Arc<Self>, timeout: Duration) {
        if self.disposed.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        self.track_daily_usage();
        let (tx, rx) = std::sync::mpsc::channel();
        let reporter = Arc::clone(self);
        std::thread::spawn(move || {
            reporter.flush();
            let _ = tx.send(());
        });
        let _ = rx.recv_timeout(timeout);
        self.disposed
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn has_install_pending(&self) -> bool {
        self.home_dir
            .join("diagnostics")
            .join("install-pending")
            .is_file()
    }

    pub fn clear_install_pending(&self) {
        let _ = std::fs::remove_file(self.home_dir.join("diagnostics").join("install-pending"));
    }

    fn track_beta_installed(&self) {
        if !self.has_install_pending() {
            return;
        }
        let outcome = std::fs::read_to_string(self.home_dir.join("import-result.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map(|result| {
                if result.get("ok").and_then(Value::as_bool) == Some(true) {
                    "imported"
                } else {
                    "import-failed"
                }
            })
            .unwrap_or("fresh");
        if self
            .track("beta.installed", json!({"kind":"beta","outcome": outcome}))
            .is_some()
        {
            self.clear_install_pending();
        }
    }

    pub fn track_lifecycle(&self, event: &str, duration_ms: Option<u64>) -> Option<String> {
        let mut payload = json!({"kind":"lifecycle"});
        if let Some(duration) = duration_ms {
            payload["durationMs"] = json!(duration);
        }
        if event == "app.start" {
            payload["osVersion"] = json!(os_version_major_minor());
            if let Some(locale) = std::env::var("LANG")
                .ok()
                .and_then(|lang| lang.split(['.', '_', '-']).next().map(str::to_string))
                .filter(|lang| !lang.is_empty())
            {
                payload["locale"] = json!(locale);
            }
        }
        self.track(event, payload)
    }

    pub fn track_update(
        &self,
        event: &str,
        outcome: &str,
        target_version: Option<&str>,
        error_context: Option<&str>,
    ) -> Option<String> {
        let mut payload =
            json!({"kind":"update","outcome": if outcome == "error" { "error" } else { "ok" }});
        if let Some(version) = target_version {
            payload["targetVersion"] = json!(version);
        }
        if let Some(context) = error_context {
            payload["errorContext"] = json!(context);
        }
        self.track(event, payload)
    }
}

fn os_version_major_minor() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()?;
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let mut parts = version.split('.');
        let major = parts
            .next()?
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>();
        let minor = parts
            .next()
            .unwrap_or("0")
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>();
        if major.is_empty() {
            None
        } else {
            Some(format!(
                "{major}.{}",
                if minor.is_empty() { "0".into() } else { minor }
            ))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

static REPORTER: std::sync::OnceLock<Option<Arc<BetaDiagnosticsReporter>>> =
    std::sync::OnceLock::new();

pub fn initialize(
    home_dir: &Path,
    flavor: &str,
    app_version: &str,
) -> Option<Arc<BetaDiagnosticsReporter>> {
    let reporter = REPORTER
        .get_or_init(|| BetaDiagnosticsReporter::create(home_dir, flavor, app_version))
        .clone()?;
    let previous_launch_version = read_launch_version(home_dir);
    let mut start_payload = json!({"kind":"lifecycle"});
    if let Some(version) = os_version() {
        start_payload["osVersion"] = json!(version);
    }
    if let Some(locale) = locale() {
        start_payload["locale"] = json!(locale);
    }
    reporter.track("app.start", start_payload);
    if previous_launch_version
        .as_deref()
        .is_some_and(|version| version != reporter.app_version)
    {
        reporter.track(
            "update.installed",
            json!({"kind":"update","outcome":"ok","targetVersion":reporter.app_version}),
        );
    }
    write_launch_version(home_dir, &reporter.app_version);
    reporter.start();
    Some(reporter)
}

pub fn sharing_changed(enabled: bool) {
    if let Some(reporter) = REPORTER.get().and_then(Option::as_ref) {
        if enabled {
            reporter.resume_sharing();
        } else {
            reporter.discard_queue();
        }
    }
}

pub fn track(event: &str, payload: Value) -> Option<String> {
    REPORTER
        .get()
        .and_then(Option::as_ref)
        .and_then(|reporter| reporter.track(event, payload))
}

pub fn track_issue(source: &str, input: &Value) -> Option<String> {
    REPORTER
        .get()
        .and_then(Option::as_ref)
        .and_then(|reporter| reporter.track_issue(source, input))
}

pub fn record_activity(input: &Value) {
    if let Some(Some(reporter)) = REPORTER.get() {
        reporter.record_activity(input);
    }
}

pub fn report_status(id: &str) -> &'static str {
    REPORTER
        .get()
        .and_then(Option::as_ref)
        .map(|reporter| reporter.report_status(id))
        .unwrap_or("unavailable")
}

pub fn dispose(timeout: Duration) {
    if let Some(Some(reporter)) = REPORTER.get() {
        reporter.dispose(timeout);
    }
}

pub fn track_beta_installed_once() {
    static EMITTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if EMITTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    if let Some(reporter) = REPORTER.get().and_then(Option::as_ref) {
        reporter.track_beta_installed();
    }
}

pub fn track_exit(duration_ms: Option<u64>) {
    if let Some(reporter) = REPORTER.get().and_then(Option::as_ref) {
        reporter.track_lifecycle("app.exit", duration_ms);
    }
}

#[cfg(test)]
impl BetaDiagnosticsReporter {
    fn with_endpoint(
        home_dir: &Path,
        flavor: &str,
        app_version: &str,
        endpoint: &str,
    ) -> Arc<Self> {
        let reporter = BetaDiagnosticsReporter {
            home_dir: home_dir.to_path_buf(),
            queue_path: home_dir.join("diagnostics").join("events.jsonl"),
            endpoint: endpoint.into(),
            install_id: load_install_id(home_dir),
            flavor: flavor.into(),
            app_version: if valid_version(app_version) {
                app_version.into()
            } else {
                "0.0.0".into()
            },
            enabled: Arc::new(Mutex::new(true)),
            error_sent_timestamps: Mutex::new(VecDeque::new()),
            error_fingerprint_sent_at: Mutex::new(HashMap::new()),
            issue_reports: Mutex::new(HashMap::new()),
            report_statuses: Mutex::new(HashMap::new()),
            recent_activity: Mutex::new(Vec::new()),
            memory_samples: Mutex::new(Vec::new()),
            disposed: std::sync::atomic::AtomicBool::new(false),
            flushing: std::sync::atomic::AtomicBool::new(false),
        };
        Arc::new(reporter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};

    #[test]
    fn beta_reporter_sanitizes_and_durable_queuestarts_only_for_beta_remote() {
        let stable_home =
            std::env::temp_dir().join(format!("diag-stable-{}", uuid::Uuid::new_v4()));
        assert!(BetaDiagnosticsReporter::create(&stable_home, "production", "1.2.3").is_none());
        let home = std::env::temp_dir().join(format!("diag-beta-{}", uuid::Uuid::new_v4()));
        let reporter = BetaDiagnosticsReporter::with_endpoint(
            &home,
            "beta",
            "1.2.3-beta.1",
            "http://127.0.0.1:1",
        );
        let id = reporter
            .track(
                "app.start",
                json!({"kind":"lifecycle","durationMs":12,"osVersion":"15.3","locale":"en"}),
            )
            .unwrap();
        assert!(uuid::Uuid::parse_str(&id).is_ok());
        assert_eq!(reporter.scrubbed_lines().len(), 1);
        assert!(reporter
            .track("chat.content", json!({"kind":"lifecycle"}))
            .is_none());
        let _ = std::fs::remove_dir_all(stable_home);
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn usage_snapshot_relays_once_per_generated_utc_day() {
        let home = std::env::temp_dir().join(format!("diag-usage-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(home.join("diagnostics")).unwrap();
        let snapshot = serde_json::to_string(&json!({
            "v": 1,
            "generatedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "projects": 3,
            "activeThreads": 2,
        }))
        .unwrap();
        std::fs::write(
            home.join("diagnostics").join("usage-snapshot.json"),
            format!("{snapshot}\n"),
        )
        .unwrap();
        let reporter = BetaDiagnosticsReporter::with_endpoint(
            &home,
            "beta",
            "1.2.3-beta.1",
            "http://127.0.0.1:1",
        );
        reporter.track_daily_usage();
        reporter.track_daily_usage();
        let lines = reporter.scrubbed_lines();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("usage.daily") && lines[0].contains("\"projects\":3"));
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn successful_flush_removes_only_acknowledged_generation_shape() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 8192];
            loop {
                let size = stream.read(&mut buffer).unwrap();
                assert!(size > 0);
                request.extend_from_slice(&buffer[..size]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let text = String::from_utf8_lossy(&request);
                    if text
                        .split("\r\n\r\n")
                        .nth(1)
                        .is_some_and(|body| body.ends_with('\n'))
                    {
                        break;
                    }
                }
            }
            let response = "HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).unwrap();
        });
        let home = std::env::temp_dir().join(format!("diag-upload-{}", uuid::Uuid::new_v4()));
        let reporter = BetaDiagnosticsReporter::with_endpoint(
            &home,
            "beta",
            "1.2.3-beta.1",
            &format!("http://127.0.0.1:{port}"),
        );
        let issue = json!({"code":"server.event-loop.stall","durationMs":1200});
        let id = reporter.track_issue("main", &issue).unwrap();
        assert_eq!(reporter.report_status(&id), "queued");
        reporter.flush();
        assert_eq!(reporter.report_status(&id), "sent");
        assert!(reporter.scrubbed_lines().is_empty());
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn diagnostic_context_is_bounded_and_fixed_shape() {
        let home = std::env::temp_dir().join(format!("diag-context-{}", uuid::Uuid::new_v4()));
        let reporter = BetaDiagnosticsReporter::with_endpoint(
            &home,
            "beta",
            "1.2.3-beta.1",
            "http://127.0.0.1:1",
        );
        reporter.capture_memory_sample();
        reporter.record_activity(&json!({"activity":"chat.send","phase":"started"}));
        reporter.record_activity(&json!({"activity":"chat.send","phase":"started"}));
        reporter.record_activity(&json!({"activity":"chat.content","phase":"started"}));
        let id = reporter
            .track(
                "app.error",
                json!({
                    "kind":"error",
                    "source":"main",
                    "message":"bounded failure",
                    "fingerprint":"abc123",
                    "stack":"safe stack"
                }),
            )
            .unwrap();
        assert!(uuid::Uuid::parse_str(&id).is_ok());
        let line = reporter.scrubbed_lines().pop().unwrap();
        // Context is local-only diagnostic evidence: the upstream allowlist
        // strips it before retention/upload, exactly as it strips free-form stacks.
        assert!(!line.contains("safe stack"));
        assert!(!line.contains("[Beta diagnostic context]"));
        assert!(!line.contains("chat.send started"));
        assert!(!line.contains("chat.content"));
        assert_eq!(reporter.recent_activity.lock().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn handled_issue_tracking_is_fixed_shape_and_deduplicated() {
        let home = std::env::temp_dir().join(format!("diag-issue-{}", uuid::Uuid::new_v4()));
        let reporter = BetaDiagnosticsReporter::with_endpoint(
            &home,
            "beta",
            "1.2.3-beta.1",
            "http://127.0.0.1:1",
        );
        let issue = json!({"code":"server.event-loop.stall","durationMs":2400});
        let first = reporter.track_issue("main", &issue).unwrap();
        assert_eq!(
            reporter.track_issue("main", &issue).as_deref(),
            Some(first.as_str())
        );
        assert!(reporter
            .track_issue("main", &json!({"code":"chat.content"}))
            .is_none());
        assert!(reporter
            .track_issue(
                "main",
                &json!({"code":"server.event-loop.stall","reason":"private detail"})
            )
            .is_none());
        let line = reporter.scrubbed_lines().pop().unwrap();
        assert!(line.contains("Handled issue: server.event-loop.stall (unknown)"));
        assert!(line.contains("durationMs=2400"));
    }
}
