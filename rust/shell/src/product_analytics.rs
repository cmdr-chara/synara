//! Consent-gated product analytics sender.
//!
//! Ports the durable installation boundary of apps/desktop/src/productAnalytics.ts:
//! no identity or queue exists while sharing is off, stored events are rebuilt
//! through the allowlist before upload, host-usage windows carry only a
//! day-scoped hash, and acknowledgements remove local data.

use hostusagerelay_rs::port::HostUsageRelay;
use serde_json::{Map, Value, json};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const ENDPOINT: &str = "https://synara-beta-diagnostics.synara-orgs.workers.dev/v1/product-events";
const IDENTITY_VERSION: &str = "2";
const MAX_QUEUE: usize = 500;
const MAX_BATCH_EVENTS: usize = 50;
const MAX_BATCH_BYTES: usize = 64 * 1024;
const MAX_EVENT_AGE_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const MAX_COUNTER: u64 = 1_000_000_000_000;
const MAX_DURATION_MS: u64 = 86_400_000;
const FLUSH_INTERVAL_MS: i64 = 30_000;
const MAX_BACKOFF_MS: i64 = 60_000;

const EVENTS: [&str; 10] = [
    "app.open",
    "app.active",
    "feature.used",
    "command.used",
    "connection.pair",
    "connection.connect",
    "connection.reconnect",
    "chat.request",
    "turn.completed",
    "performance.startup",
];
const ANONYMOUS_EVENTS: [&str; 5] = [
    "remote.pairing-approved",
    "remote.connection-succeeded",
    "remote.reconnection-succeeded",
    "remote.connection-timeout",
    "app.crash",
];
const OUTCOMES: [&str; 4] = ["started", "succeeded", "failed", "cancelled"];
const FEATURES: [&str; 9] = [
    "chat",
    "connections",
    "inbox",
    "tasks",
    "hubs",
    "browser",
    "settings",
    "search",
    "project",
];
const MODES: [&str; 2] = ["local", "remote"];
const PROVIDERS: [&str; 3] = ["codex", "claude", "other"];
const REASONS: [&str; 10] = [
    "unreachable",
    "not_accepting",
    "account_unavailable",
    "host_unlinked",
    "pairing_required",
    "incompatible",
    "timeout",
    "invalid_code",
    "rejected",
    "other",
];
const TURN_FAILURES: [&str; 9] = [
    "provider_startup",
    "provider_crash",
    "provider_timeout",
    "permission_denied",
    "rate_limited",
    "usage_limit",
    "network",
    "cancelled",
    "other",
];

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn cohort_event_id(install_id: &str, day: &str, stage: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("synara-cohort-v1:{}:{day}:{stage}", install_id.to_lowercase()));
    let mut bytes = hasher.finalize();
    bytes[6] = (bytes[6] & 15) | 128;
    bytes[8] = (bytes[8] & 63) | 128;
    let hex: String = bytes[..16].iter().map(|byte| format!("{byte:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

fn parse_day(day: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CohortState {
    day: String,
    entry_ts: String,
    completed: Vec<String>,
}

fn platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        "linux" => "linux",
        _ => "other",
    }
}

fn valid_semver(value: &str) -> bool {
    fn numeric(part: &str) -> bool {
        !part.is_empty()
            && part.bytes().all(|b| b.is_ascii_digit())
            && (part == "0" || !part.starts_with('0'))
    }
    let mut core = value.split('.');
    value.len() <= 64
        && core.next().is_some_and(numeric)
        && core.next().is_some_and(numeric)
        && core.next().is_some_and(|part| {
            let patch = part.split(['-', '+']).next().unwrap_or(part);
            numeric(patch)
        })
}

fn valid_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok()
}

fn remove_file(path: &Path) {
    if std::fs::symlink_metadata(path)
        .is_ok_and(|meta| meta.file_type().is_file() || meta.file_type().is_symlink())
    {
        let _ = std::fs::remove_file(path);
    }
}

fn write_atomic(path: &Path, contents: &str, private: bool) -> Result<(), String> {
    let parent = path.parent().ok_or("path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
    }
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("data"),
        uuid::Uuid::new_v4()
    ));
    let result = (|| {
        use std::io::Write as _;
        let mut options = std::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        if private {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
        file.write_all(contents.as_bytes())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        remove_file(&temporary);
    }
    result
}

fn object(value: &Value) -> Option<&Map<String, Value>> {
    value.as_object()
}

fn allowed(value: &Value, values: &[&str]) -> Option<String> {
    let text = value.as_str()?;
    values.contains(&text).then(|| text.to_string())
}

fn count(value: &Value, max: u64) -> Option<u64> {
    value.as_u64().filter(|n| *n <= max)
}

fn valid_stored_timestamp(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

#[derive(Default)]
struct AnalyticsState {
    initialized: bool,
    enabled: bool,
    has_explicit_consent: bool,
    install_id: Option<String>,
    queue: Vec<Value>,
    active_day: Option<String>,
    cohort: Option<CohortState>,
    cohort_supported: bool,
    failures: u32,
    next_retry_ms: Option<i64>,
    next_flush_ms: Option<i64>,
    flushing: bool,
}

pub struct ProductAnalytics {
    root: PathBuf,
    endpoint: String,
    channel: String,
    app_version: String,
    platform: String,
    default_enabled: bool,
    state: Arc<Mutex<AnalyticsState>>,
}

impl ProductAnalytics {
    pub fn new(
        home_dir: &Path,
        channel: &str,
        app_version: &str,
        default_enabled: bool,
    ) -> Arc<Self> {
        Self::with_endpoint(home_dir, channel, app_version, default_enabled, ENDPOINT)
    }

    fn with_endpoint(
        home_dir: &Path,
        channel: &str,
        app_version: &str,
        default_enabled: bool,
        endpoint: &str,
    ) -> Arc<Self> {
        Arc::new(Self {
            root: home_dir.join("product-analytics"),
            endpoint: endpoint.to_string(),
            channel: match channel {
                "beta" => "beta",
                "remote" => "remote",
                _ => "stable",
            }
            .to_string(),
            app_version: if valid_semver(app_version) {
                app_version
            } else {
                "0.0.0"
            }
            .to_string(),
            platform: platform().to_string(),
            default_enabled,
            state: Arc::new(Mutex::new(AnalyticsState::default())),
        })
    }

    fn consent_path(&self) -> PathBuf {
        self.root.join("consent.json")
    }

    fn queue_path(&self) -> PathBuf {
        self.root.join("events.json")
    }

    fn install_id_path(&self) -> PathBuf {
        self.root.join("install-id")
    }

    fn cohort_path(&self) -> PathBuf {
        self.root.join("cohort.json")
    }

    fn identity_version_path(&self) -> PathBuf {
        self.root.join("identity-version")
    }

    fn read_consent(&self) -> (bool, bool) {
        let text = match std::fs::read_to_string(self.consent_path()) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return (self.default_enabled, false);
            }
            Err(_) => return (false, false),
        };
        let enabled = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|value| value.get("enabled").and_then(Value::as_bool))
            == Some(true);
        (enabled, enabled)
    }

    fn persist_consent(&self, enabled: bool) -> Result<(), String> {
        write_atomic(
            &self.consent_path(),
            &json!({"enabled": enabled}).to_string(),
            true,
        )
    }

    fn clear_persistent_data(&self, state: &mut AnalyticsState) -> Result<(), String> {
        let mut relay = HostUsageRelay::new(
            self.root
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_string_lossy()
                .into_owned(),
            self.channel.clone(),
        )?;
        relay.clear()?;
        for path in [
            self.install_id_path(),
            self.identity_version_path(),
            self.root.join("account-hash"),
            self.queue_path(),
            self.root.join("cohort.json"),
        ] {
            remove_file(&path);
        }
        state.install_id = None;
        state.queue.clear();
        state.active_day = None;
        Ok(())
    }

    fn read_or_create_install_id(&self) -> Result<String, String> {
        if let Ok(existing) = std::fs::read_to_string(self.install_id_path()) {
            let existing = existing.trim();
            if valid_uuid(existing) {
                return Ok(existing.to_string());
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        write_atomic(&self.install_id_path(), &format!("{id}\n"), true)?;
        Ok(id)
    }

    fn persist_cohort(&self, install_id: &str, cohort: &CohortState) -> Result<(), String> {
        write_atomic(
            &self.cohort_path(),
            &json!({"v":1,"installId":install_id,"day":cohort.day,"entryTs":cohort.entry_ts,"completed":cohort.completed}).to_string(),
            true,
        )
    }

    fn read_cohort(&self, install_id: &str, queue: &[Value]) -> Option<CohortState> {
        let mut state = std::fs::read_to_string(self.cohort_path())
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|raw| {
                let day = raw.get("day").and_then(Value::as_str)?.to_string();
                let entry_ts = raw.get("entryTs").and_then(Value::as_str)?.to_string();
                let completed = raw.get("completed").and_then(Value::as_array)?.iter()
                    .filter_map(Value::as_str)
                    .filter(|stage| ["new", "week1", "month1"].contains(stage))
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                (raw.get("v").and_then(Value::as_u64) == Some(1)
                    && raw.get("installId").and_then(Value::as_str) == Some(install_id)
                    && parse_day(&day).is_some()
                    && entry_ts.starts_with(&day)
                    && valid_stored_timestamp(&entry_ts)
                    && chrono::DateTime::parse_from_rfc3339(&entry_ts).ok()?.timestamp_millis() <= now_ms() + 300_000)
                    .then(|| CohortState { day, entry_ts, completed })
            });
        let mut queued_entry: Option<(i64, String, String)> = None;
        for event in queue {
            if event.get("installId").is_some()
                && event.get("cohortStage").and_then(Value::as_str) == Some("new")
            {
                let Some(ts) = event.get("ts").and_then(Value::as_str) else { continue };
                let Some(day) = event.get("cohortDay").and_then(Value::as_str) else { continue };
                let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(ts) else { continue };
                if queued_entry.as_ref().is_none_or(|(time, _, _)| parsed.timestamp_millis() > *time) {
                    queued_entry = Some((parsed.timestamp_millis(), ts.to_string(), day.to_string()));
                }
            }
        }
        if let Some((time, ts, day)) = queued_entry {
            let current_time = state.as_ref().and_then(|cohort| {
                chrono::DateTime::parse_from_rfc3339(&cohort.entry_ts).ok().map(|entry| entry.timestamp_millis())
            });
            if current_time.is_none_or(|current| time > current) {
                state = Some(CohortState { day, entry_ts: ts, completed: Vec::new() });
            }
        }
        state
    }

    fn build_envelope(&self, input: &Value, install_id: &str) -> Option<Value> {
        let input = object(input)?;
        let event = allowed(input.get("event")?, &EVENTS)?;
        let outcome = allowed(input.get("outcome")?, &OUTCOMES)?;
        let mut fields = Map::new();
        if let Some(value) = input.get("feature") {
            fields.insert("feature".into(), Value::String(allowed(value, &FEATURES)?));
        }
        if let Some(value) = input.get("mode") {
            fields.insert("mode".into(), Value::String(allowed(value, &MODES)?));
        }
        if let Some(value) = input.get("provider") {
            fields.insert(
                "provider".into(),
                Value::String(allowed(value, &PROVIDERS)?),
            );
        }
        if let Some(value) = input.get("reason") {
            fields.insert("reason".into(), Value::String(allowed(value, &REASONS)?));
        }
        if let Some(value) = input.get("failure") {
            fields.insert(
                "failure".into(),
                Value::String(allowed(value, &TURN_FAILURES)?),
            );
        }
        if let Some(value) = input.get("httpStatus") {
            let status = count(value, 599)?;
            if status < 100 {
                return None;
            }
            fields.insert("httpStatus".into(), Value::from(status));
        }
        if let Some(value) = input.get("command") {
            let valid =
                productanalytics_rs::port::is_product_analytics_command(value.clone()).ok()?;
            if !valid {
                return None;
            }
            fields.insert("command".into(), value.clone());
        }
        if let Some(value) = input.get("durationMs") {
            fields.insert(
                "durationMs".into(),
                Value::from(count(value, MAX_DURATION_MS)?),
            );
        }
        for key in ["inputTokens", "outputTokens", "cachedInputTokens"] {
            if let Some(value) = input.get(key) {
                fields.insert(key.into(), Value::from(count(value, MAX_COUNTER)?));
            }
        }

        let mut allowed_keys = vec!["event", "outcome"];
        match event.as_str() {
            "feature.used" => allowed_keys.extend(["feature", "mode"]),
            "connection.pair" | "connection.connect" | "connection.reconnect" | "chat.request" => {
                allowed_keys.extend(["mode", "provider", "durationMs"]);
                if outcome == "failed" {
                    allowed_keys.push("reason");
                }
            }
            "turn.completed" => {
                allowed_keys.extend([
                    "mode",
                    "provider",
                    "durationMs",
                    "inputTokens",
                    "outputTokens",
                    "cachedInputTokens",
                ]);
                if matches!(outcome.as_str(), "failed" | "cancelled") {
                    allowed_keys.push("failure");
                }
                if fields.contains_key("failure") {
                    allowed_keys.push("httpStatus");
                }
            }
            "performance.startup" => allowed_keys.push("durationMs"),
            "command.used" => allowed_keys.push("command"),
            _ => {}
        }
        if fields
            .keys()
            .any(|key| !allowed_keys.contains(&key.as_str()))
        {
            return None;
        }
        if event == "feature.used" && !fields.contains_key("feature") {
            return None;
        }
        if event == "command.used" && !fields.contains_key("command") {
            return None;
        }

        let mut envelope = Map::new();
        envelope.insert("v".into(), Value::from(1));
        envelope.insert("id".into(), Value::String(uuid::Uuid::new_v4().to_string()));
        envelope.insert("ts".into(), Value::String(now_iso()));
        envelope.insert("installId".into(), Value::String(install_id.to_string()));
        envelope.insert("channel".into(), Value::String(self.channel.clone()));
        envelope.insert("surface".into(), Value::String("desktop".into()));
        envelope.insert("platform".into(), Value::String(self.platform.clone()));
        envelope.insert("appVersion".into(), Value::String(self.app_version.clone()));
        envelope.insert("event".into(), Value::String(event));
        envelope.insert("outcome".into(), Value::String(outcome));
        envelope.extend(fields);
        Some(Value::Object(envelope))
    }

    fn build_anonymous(&self, input: &Value) -> Option<Value> {
        let input = object(input)?;
        let event = allowed(input.get("event")?, &ANONYMOUS_EVENTS)?;
        let expected = if matches!(event.as_str(), "app.crash" | "remote.connection-timeout") {
            "failed"
        } else {
            "succeeded"
        };
        if input.get("outcome").and_then(Value::as_str) != Some(expected) {
            return None;
        }
        Some(json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "ts": today(),
            "event": event,
            "channel": self.channel,
        }))
    }

    fn valid_stored_event(&self, value: &Value, install_id: &str) -> Option<Value> {
        let row = object(value)?;
        if let Some(event) = row.get("event").and_then(Value::as_str) {
            if ANONYMOUS_EVENTS.contains(&event) {
                let id = row.get("id").and_then(Value::as_str)?;
                let ts = row.get("ts").and_then(Value::as_str)?;
                if !valid_uuid(id)
                    || chrono::NaiveDate::parse_from_str(ts, "%Y-%m-%d").is_err()
                    || row.get("channel").and_then(Value::as_str) != Some(self.channel.as_str())
                {
                    return None;
                }
                return Some(json!({"id": id, "ts": ts, "event": event, "channel": self.channel}));
            }
        }
        if row.get("v").and_then(Value::as_u64) != Some(1)
            || row
                .get("id")
                .and_then(Value::as_str)
                .is_none_or(|id| !valid_uuid(id))
            || row
                .get("ts")
                .and_then(Value::as_str)
                .is_none_or(|ts| !valid_stored_timestamp(ts))
            || row.get("installId").and_then(Value::as_str) != Some(install_id)
            || row.get("channel").and_then(Value::as_str) != Some(self.channel.as_str())
            || row.get("surface").and_then(Value::as_str) != Some("desktop")
            || row.get("platform").and_then(Value::as_str) != Some(self.platform.as_str())
            || row
                .get("appVersion")
                .and_then(Value::as_str)
                .is_none_or(|v| !valid_semver(v))
        {
            return None;
        }
        let mut input = Map::new();
        for key in [
            "event",
            "outcome",
            "feature",
            "mode",
            "provider",
            "reason",
            "failure",
            "httpStatus",
            "command",
            "durationMs",
            "inputTokens",
            "outputTokens",
            "cachedInputTokens",
        ] {
            if let Some(value) = row.get(key) {
                input.insert(key.into(), value.clone());
            }
        }
        let mut rebuilt = self.build_envelope(&Value::Object(input), install_id)?;
        if let Value::Object(ref mut rebuilt) = rebuilt {
            rebuilt.insert("id".into(), row.get("id")?.clone());
            rebuilt.insert("ts".into(), row.get("ts")?.clone());
            if row.get("cohortDay").is_some() || row.get("cohortStage").is_some() {
                let cohort_day = row.get("cohortDay").and_then(Value::as_str)?;
                let stage = row.get("cohortStage").and_then(Value::as_str)?;
                let id = row.get("id").and_then(Value::as_str)?;
                let event_day = parse_day(&row.get("ts").and_then(Value::as_str)?[..10])?;
                let cohort_date = parse_day(cohort_day)?;
                let age = (event_day - cohort_date).num_days();
                let valid_age = match stage {
                    "new" => age == 0,
                    "week1" => (7..=13).contains(&age),
                    "month1" => (30..=36).contains(&age),
                    _ => false,
                };
                if row.get("event").and_then(Value::as_str) != Some("app.active")
                    || row.get("outcome").and_then(Value::as_str) != Some("succeeded")
                    || !valid_age
                    || id != cohort_event_id(install_id, cohort_day, stage)
                {
                    return None;
                }
                rebuilt.insert("cohortDay".into(), Value::String(cohort_day.into()));
                rebuilt.insert("cohortStage".into(), Value::String(stage.into()));
            }
        }
        Some(rebuilt)
    }

    fn read_queue(&self, install_id: &str) -> Vec<Value> {
        let Ok(text) = std::fs::read_to_string(self.queue_path()) else {
            return Vec::new();
        };
        let Ok(Value::Array(values)) = serde_json::from_str::<Value>(&text) else {
            return Vec::new();
        };
        let mut valid: Vec<Value> = values
            .iter()
            .filter_map(|value| self.valid_stored_event(value, install_id))
            .collect();
        if valid.len() > MAX_QUEUE {
            let overflow = valid.len() - MAX_QUEUE;
            valid.drain(..overflow);
        }
        valid
    }

    fn persist_queue(&self, queue: &[Value]) -> Result<(), String> {
        write_atomic(
            &self.queue_path(),
            &serde_json::to_string(queue).map_err(|e| e.to_string())?,
            true,
        )
    }

    fn prune_expired(queue: &mut Vec<Value>) {
        let now = now_ms();
        let cutoff = now - MAX_EVENT_AGE_MS;
        let oldest_anonymous = (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();
        let today = today();
        queue.retain(|event| {
            let Some(row) = object(event) else {
                return false;
            };
            let Some(ts) = row.get("ts").and_then(Value::as_str) else {
                return false;
            };
            if row.get("installId").is_none() {
                return ts >= oldest_anonymous.as_str() && ts <= today.as_str();
            }
            chrono::DateTime::parse_from_rfc3339(ts)
                .ok()
                .is_some_and(|time| {
                    let ms = time.timestamp_millis();
                    ms >= cutoff && ms <= now + 300_000
                })
        });
    }

    pub fn initialize(self: &Arc<Self>) {
        let (enabled, explicit) = self.read_consent();
        let mut relay = match HostUsageRelay::new(
            self.root
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_string_lossy()
                .into_owned(),
            self.channel.clone(),
        ) {
            Ok(relay) => relay,
            Err(_) => return,
        };
        let mut persisted_queue = Vec::new();
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.initialized {
                return;
            }
            state.initialized = true;
            state.enabled = enabled;
            state.has_explicit_consent = explicit;
            if enabled {
                let identity_ok = std::fs::read_to_string(self.identity_version_path())
                    .map(|value| value.trim() == IDENTITY_VERSION)
                    .unwrap_or(false);
                if !identity_ok {
                    let _ = self.clear_persistent_data(&mut state);
                    let _ = write_atomic(
                        &self.identity_version_path(),
                        &format!("{IDENTITY_VERSION}\n"),
                        true,
                    );
                }
                if let Ok(install_id) = self.read_or_create_install_id() {
                    state.install_id = Some(install_id.clone());
                    state.queue = self.read_queue(&install_id);
                    state.cohort = self.read_cohort(&install_id, &state.queue);
                    persisted_queue = state.queue.clone();
                    let _ = relay.enable();
                } else {
                    state.enabled = false;
                }
            } else {
                let _ = self.clear_persistent_data(&mut state);
            }
        }
        if enabled {
            let _ = self.persist_queue(&persisted_queue);
        }
    }

    pub fn enabled(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.enabled)
            .unwrap_or(false)
    }

    pub fn set_enabled(self: &Arc<Self>, enabled: bool) -> bool {
        let mut relay = match HostUsageRelay::new(
            self.root
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_string_lossy()
                .into_owned(),
            self.channel.clone(),
        ) {
            Ok(relay) => relay,
            Err(_) => return self.enabled(),
        };
        let mut queue_snapshot = Vec::new();
        {
            let Ok(mut state) = self.state.lock() else {
                return false;
            };
            if !state.initialized {
                return state.enabled;
            }
            if enabled && state.enabled && !state.has_explicit_consent {
                if self.persist_consent(true).is_ok() {
                    state.has_explicit_consent = true;
                    crate::reliability::set_enabled(true);
                }
                return state.enabled;
            }
            if enabled == state.enabled && (enabled || !state.has_explicit_consent) {
                return state.enabled;
            }
            if !enabled {
                state.enabled = false;
                state.install_id = None;
                state.queue.clear();
                state.active_day = None;
                let _ = self.clear_persistent_data(&mut state);
                let _ = self.persist_consent(false);
                crate::reliability::set_enabled(false);
                return false;
            }
            let _ = self.clear_persistent_data(&mut state);
            if write_atomic(
                &self.identity_version_path(),
                &format!("{IDENTITY_VERSION}\n"),
                true,
            )
            .is_err()
            {
                return state.enabled;
            }
            let Ok(install_id) = self.read_or_create_install_id() else {
                return state.enabled;
            };
            state.install_id = Some(install_id);
            state.queue.clear();
            state.enabled = relay.enable().is_ok() && self.persist_consent(true).is_ok();
            state.has_explicit_consent = state.enabled;
            if !state.enabled {
                let _ = self.clear_persistent_data(&mut state);
            }
            queue_snapshot = state.queue.clone();
        }
        let _ = self.persist_queue(&queue_snapshot);
        crate::reliability::set_enabled(self.enabled());
        self.enabled()
    }

    pub fn track(self: &Arc<Self>, input: Value) {
        let (event, queue_snapshot) = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if !state.enabled || state.install_id.is_none() {
                return;
            }
            let install_id = state.install_id.clone().unwrap_or_default();
            let built = if input
                .get("event")
                .and_then(Value::as_str)
                .is_some_and(|event| ANONYMOUS_EVENTS.contains(&event))
            {
                self.build_anonymous(&input)
            } else {
                self.build_envelope(&input, &install_id)
            };
            let Some(envelope) = built else { return };
            state.queue.push(envelope);
            Self::prune_expired(&mut state.queue);
            if state.queue.len() > MAX_QUEUE {
                let overflow = state.queue.len() - MAX_QUEUE;
                state.queue.drain(..overflow);
            }
            let event = state.queue.last().cloned();
            (event, state.queue.clone())
        };
        if event.is_some() {
            let _ = self.persist_queue(&queue_snapshot);
        }
    }

    fn queue_cohort_stage(self: &Arc<Self>) {
        let (snapshot, cohort_write) = {
            let Ok(mut state) = self.state.lock() else { return };
            if !state.cohort_supported || !state.enabled || state.install_id.is_none() { return; }
            let install_id = state.install_id.clone().unwrap_or_default();
            let now = now_ms();
            let day = today();
            let mut cohort = state.cohort.clone().unwrap_or(CohortState {
                day: day.clone(),
                entry_ts: now_iso(),
                completed: Vec::new(),
            });
            if !cohort.completed.iter().any(|stage| stage == "new")
                && chrono::DateTime::parse_from_rfc3339(&cohort.entry_ts)
                    .ok()
                    .is_some_and(|entry| entry.timestamp_millis() < now - MAX_EVENT_AGE_MS)
            {
                cohort = CohortState { day: day.clone(), entry_ts: now_iso(), completed: Vec::new() };
            }
            let age = parse_day(&day)
                .and_then(|today| parse_day(&cohort.day).map(|entry| (today - entry).num_days()))
                .unwrap_or(i64::MAX);
            let stage = if !cohort.completed.iter().any(|stage| stage == "new") {
                Some("new")
            } else if (7..=13).contains(&age) && !cohort.completed.iter().any(|stage| stage == "week1") {
                Some("week1")
            } else if (30..=36).contains(&age) && !cohort.completed.iter().any(|stage| stage == "month1") {
                Some("month1")
            } else {
                None
            };
            let Some(stage) = stage else { return };
            if state.queue.iter().any(|event| {
                event.get("cohortDay").and_then(Value::as_str) == Some(cohort.day.as_str())
                    && event.get("cohortStage").and_then(Value::as_str) == Some(stage)
            }) {
                return;
            }
            let Some(mut envelope) = self.build_envelope(&json!({"event":"app.active","outcome":"succeeded"}), &install_id) else { return };
            if let Value::Object(ref mut row) = envelope {
                row.insert("id".into(), Value::String(cohort_event_id(&install_id, &cohort.day, stage)));
                if stage != "new" {
                    row.insert("ts".into(), Value::String(now_iso()));
                } else {
                    row.insert("ts".into(), Value::String(cohort.entry_ts.clone()));
                }
                row.insert("cohortDay".into(), Value::String(cohort.day.clone()));
                row.insert("cohortStage".into(), Value::String(stage.into()));
            }
            state.queue.push(envelope);
            Self::prune_expired(&mut state.queue);
            if state.queue.len() > MAX_QUEUE {
                let overflow = state.queue.len() - MAX_QUEUE;
                state.queue.drain(..overflow);
            }
            state.cohort = Some(cohort.clone());
            (state.queue.clone(), Some((install_id, cohort)))
        };
        // Durable queue first, then cohort state; a crash can only replay the UUID.
        let _ = self.persist_queue(&snapshot);
        if let Some((install_id, cohort)) = cohort_write {
            let _ = self.persist_cohort(&install_id, &cohort);
        }
    }

    fn track_active_day(self: &Arc<Self>) {
        self.queue_cohort_stage();
        let day = today();
        let should_track = self
            .state
            .lock()
            .map(|mut state| {
                if !state.enabled || state.active_day.as_deref() == Some(day.as_str()) {
                    false
                } else {
                    state.active_day = Some(day);
                    true
                }
            })
            .unwrap_or(false);
        if should_track {
            self.track(json!({"event":"app.active","outcome":"succeeded"}));
        }
    }

    fn batch(&self) -> (Vec<Value>, bool) {
        let Ok(mut state) = self.state.lock() else {
            return (Vec::new(), false);
        };
        if !state.enabled {
            return (Vec::new(), false);
        }
        Self::prune_expired(&mut state.queue);
        let mut batch: Vec<Value> = Vec::new();
        for event in &state.queue {
            if batch.len() >= MAX_BATCH_EVENTS {
                break;
            }
            let anonymous = event.get("installId").is_none();
            if let Some(first) = batch.first() {
                if (first.get("installId").is_none()) != anonymous {
                    break;
                }
            }
            let mut candidate = batch.clone();
            candidate.push(event.clone());
            let body = if anonymous {
                json!({"anonymousEvents": candidate})
            } else {
                json!({"events": candidate})
            };
            if serde_json::to_string(&body)
                .map(|body| body.len())
                .unwrap_or(usize::MAX)
                > MAX_BATCH_BYTES
            {
                break;
            }
            batch.push(event.clone());
        }
        let anonymous = batch
            .first()
            .is_some_and(|event| event.get("installId").is_none());
        (batch, anonymous)
    }

    fn host_window(&self) -> Option<Value> {
        let (home, install_id) = {
            let state = self.state.lock().ok()?;
            (
                self.root.parent()?.to_string_lossy().into_owned(),
                state.install_id.clone()?,
            )
        };
        let mut relay = HostUsageRelay::new(home, self.channel.clone()).ok()?;
        relay.enable().ok()?;
        relay.peek(install_id).ok().flatten()
    }

    fn flush_once(self: &Arc<Self>) -> Result<(), String> {
        {
            let Ok(mut state) = self.state.lock() else {
                return Err("state lock".into());
            };
            if !state.enabled || state.flushing {
                return Ok(());
            }
            state.flushing = true;
        }
        let result = self.flush_inner();
        if let Ok(mut state) = self.state.lock() {
            state.flushing = false;
            if result.is_err() {
                state.failures = state.failures.saturating_add(1);
                let exponent = state.failures.saturating_sub(1).min(6);
                let delay = 1_000i64
                    .saturating_mul(1i64 << exponent)
                    .min(MAX_BACKOFF_MS);
                state.next_retry_ms = Some(now_ms() + delay);
            } else {
                state.failures = 0;
                state.next_retry_ms = None;
                state.next_flush_ms = Some(now_ms() + FLUSH_INTERVAL_MS);
            }
        }
        result
    }

    fn flush_inner(self: &Arc<Self>) -> Result<(), String> {
        let (batch, anonymous) = self.batch();
        let host_window = if batch.is_empty() {
            self.host_window()
        } else {
            None
        };
        if batch.is_empty() && host_window.is_none() {
            return Ok(());
        }
        let (body, host_mode) = if let Some(window) = &host_window {
            (json!({"hostUsage": window}), true)
        } else if anonymous {
            (json!({"anonymousEvents": batch}), false)
        } else {
            (json!({"events": batch}), false)
        };
        let mut response = Some(
            ureq::post(&self.endpoint)
                .timeout(Duration::from_secs(8))
                .set("content-type", "application/json")
                .send_json(body)
                .map_err(|error| format!("product analytics request failed: {error}"))?,
        );
        let status = response.as_ref().map(|response| response.status()).unwrap_or(500);
        let cohorts_supported = response
            .as_ref()
            .and_then(|response| response.header("x-synara-product-cohorts"))
            == Some("1");
        if !(200..300).contains(&status) {
            if !host_mode && matches!(status, 400 | 413 | 422) {
                let rejected: std::collections::HashSet<String> = batch
                    .iter()
                    .filter_map(|event| event.get("id").and_then(Value::as_str).map(str::to_string))
                    .collect();
                let snapshot = {
                    let Ok(mut state) = self.state.lock() else {
                        return Ok(());
                    };
                    state.queue.retain(|event| {
                        event
                            .get("id")
                            .and_then(Value::as_str)
                            .is_none_or(|id| !rejected.contains(id))
                    });
                    state.queue.clone()
                };
                let _ = self.persist_queue(&snapshot);
                return Ok(());
            }
            return Err("product analytics request was not accepted".into());
        }
        if !self.enabled() {
            return Ok(());
        }
        if let Some(window) = host_window {
            let receipt: Value = response.take().ok_or("product analytics response consumed")?.into_json().map_err(|e| e.to_string())?;
            if receipt.get("stored").and_then(Value::as_bool) != Some(true)
                || receipt.get("windowId") != window.get("windowId")
            {
                return Err("host usage receipt missing".into());
            }
            let home = self
                .root
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_string_lossy()
                .into_owned();
            let mut relay =
                HostUsageRelay::new(home, self.channel.clone()).map_err(|e| e.to_string())?;
            relay.enable()?;
            relay.acknowledge(window)?;
            return Ok(());
        }
        if anonymous {
            let receipt: Value = response.take().ok_or("product analytics response consumed")?.into_json().map_err(|e| e.to_string())?;
            if receipt.get("stored").and_then(Value::as_bool) != Some(true) {
                return Err("anonymous product receipt missing".into());
            }
        }
        let mut cohort_acknowledged = std::collections::HashSet::new();
        if !anonymous {
            if cohorts_supported && batch.iter().any(|event| event.get("cohortStage").is_some()) {
                let receipt: Value = response.take().ok_or("product analytics response consumed")?.into_json().map_err(|e| e.to_string())?;
                if let Some(ids) = receipt.get("cohortAcknowledged").and_then(Value::as_array) {
                    for id in ids.iter().filter_map(Value::as_str) {
                        cohort_acknowledged.insert(id.to_string());
                    }
                }
            }
        }
        let sent: std::collections::HashSet<String> = batch
            .iter()
            .filter(|event| {
                event.get("cohortStage").is_none()
                    || event
                        .get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| cohort_acknowledged.contains(id))
            })
            .filter_map(|event| event.get("id").and_then(Value::as_str).map(str::to_string))
            .collect();
        let (snapshot, cohort_write) = {
            let Ok(mut state) = self.state.lock() else {
                return Ok(());
            };
            if !state.enabled {
                return Ok(());
            }
            state.cohort_supported = cohorts_supported;
            let mut cohort_write = None;
            if let Some(cohort) = &state.cohort {
                let mut completed = cohort.completed.clone();
                for event in &batch {
                    if event.get("cohortDay").and_then(Value::as_str) == Some(cohort.day.as_str())
                        && event
                            .get("id")
                            .and_then(Value::as_str)
                            .is_some_and(|id| cohort_acknowledged.contains(id))
                    {
                        if let Some(stage) = event.get("cohortStage").and_then(Value::as_str) {
                            if !completed.iter().any(|existing| existing == stage) {
                                completed.push(stage.to_string());
                            }
                        }
                    }
                }
                if completed != cohort.completed {
                    let updated = CohortState { completed, ..cohort.clone() };
                    state.cohort = Some(updated.clone());
                    if let Some(install_id) = state.install_id.clone() {
                        cohort_write = Some((install_id, updated));
                    }
                }
            }
            state.queue.retain(|event| {
                event
                    .get("id")
                    .and_then(Value::as_str)
                    .is_none_or(|id| !sent.contains(id))
            });
            (state.queue.clone(), cohort_write)
        };
        // Completion is durable before the envelope leaves the retry queue.
        if let Some((install_id, cohort)) = cohort_write {
            self.persist_cohort(&install_id, &cohort)?;
        }
        let result = self.persist_queue(&snapshot);
        if result.is_ok() {
            self.queue_cohort_stage();
        }
        result
    }

    pub fn start(self: &Arc<Self>) {
        self.track_active_day();
        let analytics = Arc::clone(self);
        std::thread::spawn(move || {
            loop {
                analytics.track_active_day();
                let due = analytics
                    .state
                    .lock()
                    .map(|state| {
                        state.enabled
                            && !state.flushing
                            && (state.next_retry_ms.is_some_and(|retry| retry <= now_ms())
                                || state.next_flush_ms.is_none_or(|flush| flush <= now_ms()))
                    })
                    .unwrap_or(false);
                if due {
                    let _ = analytics.flush_once();
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        });
    }
}

static ANALYTICS: OnceLock<Arc<ProductAnalytics>> = OnceLock::new();

pub fn initialize() {
    let home = crate::backend::configured_home_dir();
    let flavor = crate::desktop_identity::runtime_flavor();
    let channel = match flavor {
        "beta" => "beta",
        "remote" => "remote",
        _ => "stable",
    };
    let version = std::env::var("SYNARA_DESKTOP_VERSION").unwrap_or_else(|_| "0.1.0".into());
    let default_enabled = matches!(flavor, "production" | "remote");
    let analytics =
        ANALYTICS.get_or_init(|| ProductAnalytics::new(&home, channel, &version, default_enabled));
    analytics.initialize();
    analytics.start();
    analytics.track(json!({"event":"app.open","outcome":"succeeded"}));
}

pub fn enabled() -> bool {
    ANALYTICS
        .get()
        .map(|analytics| analytics.enabled())
        .unwrap_or(false)
}

pub fn set_enabled(enabled: bool) -> bool {
    ANALYTICS
        .get()
        .map(|analytics| analytics.set_enabled(enabled))
        .unwrap_or(false)
}

pub fn track(input: Value) {
    if let Some(analytics) = ANALYTICS.get() {
        analytics.track(input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};

    #[test]
    fn envelope_rebuild_discards_unlisted_fields() {
        let home =
            std::env::temp_dir().join(format!("synara-product-test-{}", uuid::Uuid::new_v4()));
        let analytics =
            ProductAnalytics::with_endpoint(&home, "stable", "1.2.3", false, "http://127.0.0.1:1");
        let install = uuid::Uuid::new_v4().to_string();
        let event = analytics
            .build_envelope(
                &json!({"event":"feature.used","outcome":"succeeded","feature":"settings","projectName":"private","prompt":"secret"}),
                &install,
            )
            .unwrap();
        assert_eq!(event["feature"], "settings");
        assert!(event.get("projectName").is_none());
        assert!(event.get("prompt").is_none());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn enable_writes_policy_and_disable_removes_identity() {
        let home =
            std::env::temp_dir().join(format!("synara-product-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        let analytics =
            ProductAnalytics::with_endpoint(&home, "stable", "1.2.3", false, "http://127.0.0.1:1");
        analytics.initialize();
        assert!(analytics.set_enabled(true));
        assert!(home.join("product-analytics/install-id").exists());
        assert!(
            home.join("product-analytics/host-usage-policy.json")
                .exists()
        );
        assert!(!analytics.set_enabled(false));
        assert!(!home.join("product-analytics/install-id").exists());
        assert!(
            !home
                .join("product-analytics/host-usage-policy.json")
                .exists()
        );
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn cohort_stage_is_deterministic_and_durable() {
        let home = std::env::temp_dir().join(format!("synara-product-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        let analytics = ProductAnalytics::with_endpoint(&home, "stable", "1.2.3", false, "http://127.0.0.1:1");
        analytics.initialize();
        assert!(analytics.set_enabled(true));
        {
            let mut state = analytics.state.lock().unwrap();
            state.cohort_supported = true;
        }
        analytics.queue_cohort_stage();
        let (install, event) = {
            let state = analytics.state.lock().unwrap();
            let event = state
                .queue
                .iter()
                .find(|event| event.get("cohortStage").and_then(Value::as_str) == Some("new"))
                .cloned()
                .unwrap();
            (state.install_id.clone().unwrap(), event)
        };
        let day = event["cohortDay"].as_str().unwrap();
        assert_eq!(event["id"].as_str().unwrap(), cohort_event_id(&install, day, "new"));
        assert_eq!(event["event"], "app.active");
        assert!(home.join("product-analytics/cohort.json").exists());
        assert!(!analytics.set_enabled(false));
        assert!(!home.join("product-analytics/cohort.json").exists());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn acknowledged_host_usage_removes_spool() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0_u8; 8192];
            loop {
                let size = stream.read(&mut buffer).unwrap();
                bytes.extend_from_slice(&buffer[..size]);
                if bytes.windows(4).any(|w| w == b"\r\n\r\n")
                    && String::from_utf8_lossy(&bytes).contains("windowId")
                {
                    break;
                }
                if size == 0 {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&bytes);
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("{}");
            let parsed: Value =
                serde_json::from_str(body.split('\0').next().unwrap_or(body)).unwrap_or(json!({}));
            let window_id = parsed
                .pointer("/hostUsage/windowId")
                .and_then(Value::as_str)
                .unwrap_or("x");
            let payload = json!({"stored":true,"windowId":window_id}).to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                payload.len(),
                payload
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let home =
            std::env::temp_dir().join(format!("synara-product-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        let analytics = ProductAnalytics::with_endpoint(
            &home,
            "stable",
            "1.2.3",
            false,
            &format!("http://127.0.0.1:{port}"),
        );
        analytics.initialize();
        assert!(analytics.set_enabled(true));
        let install = analytics.state.lock().unwrap().install_id.clone().unwrap();
        let mut relay =
            HostUsageRelay::new(home.to_string_lossy().into_owned(), "stable".into()).unwrap();
        relay.enable().unwrap();
        let spool = home.join("product-analytics/host-usage");
        std::fs::create_dir_all(&spool).unwrap();
        let policy_path = home.join("product-analytics/host-usage-policy.json");
        let mut policy: Value =
            serde_json::from_str(&std::fs::read_to_string(&policy_path).unwrap()).unwrap();
        let day = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        policy["since"] = json!(format!("{day}T00:00:00.000Z"));
        std::fs::write(&policy_path, policy.to_string()).unwrap();
        let window_start = format!("{day}T00:00:00.000Z");
        let window_end = format!("{day}T00:00:00.000Z");
        let _ = window_end;
        let start = chrono::NaiveDate::parse_from_str(&day, "%Y-%m-%d")
            .unwrap()
            .and_hms_milli_opt(0, 0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let end = iso_from_ms(start + 86_400_000);
        std::fs::write(
            spool.join(format!("{}-{day}.json", policy["policyId"].as_str().unwrap())),
            json!({
                "v":1,"policyId":policy["policyId"],"windowStart":window_start,"windowEnd":end,
                "coveredFrom":window_start,"generatedAt":now_iso(),"eligibleTurns":0,"includedTurns":0,
                "missingCompletionTimes":0,"tokenTruncatedTurns":0,"cappedMeasurements":0,"rows":[]
            }).to_string(),
        ).unwrap();
        assert!(
            analytics.host_window().is_some(),
            "host window should be pending for {install}"
        );
        analytics.flush_once().unwrap();
        assert!(
            std::fs::read_dir(&spool)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false)
        );
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(home);
    }

    fn iso_from_ms(ms: i64) -> String {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
}
