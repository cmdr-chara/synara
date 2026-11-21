//! The Rust port of `apps/analytics/worker.ts`.
//!
//! Shared helpers (`diagnosticsRedaction`, `productAnalytics` command list, and
//! the `new Date(...).toISOString()` civil-date math) are vendored here the same
//! way they are vendored in their own unit crates.

#![forbid(unsafe_code)]

use regex::{Captures, Regex};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

macro_rules! regex {
    ($name:ident, $pat:expr) => {
        fn $name() -> &'static Regex {
            static R: OnceLock<Regex> = OnceLock::new();
            R.get_or_init(|| Regex::new($pat).expect("valid regex"))
        }
    };
}

const DAY_MS: f64 = 86_400_000.0;
const INVALID_TIME: &str = "Invalid time value";

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => Ok(o.get(key)),
        v if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        _ => Ok(None),
    }
}

/// `typeof v === "string"` view: plain strings plus `{"$utf16": ...}` ones.
fn as_js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("$utf16").and_then(Value::as_array).map(|units| {
            let u: Vec<u16> = units
                .iter()
                .filter_map(|x| x.as_u64().map(|n| n as u16))
                .collect();
            String::from_utf16_lossy(&u)
        }),
        _ => None,
    }
}

/// JS `String(value)` coercion.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() || is_undefined(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if let Some(units) = o.get("$utf16").and_then(Value::as_array) {
                let u: Vec<u16> = units
                    .iter()
                    .filter_map(|u| u.as_u64().map(|x| x as u16))
                    .collect();
                String::from_utf16_lossy(&u)
            } else if o.get("$undefined").is_some() {
                "undefined".to_string()
            } else if let Some(n) = o.get("$num").and_then(Value::as_str) {
                n.to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// JS `ToNumber` coercion.
fn js_to_num(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => *b as i32 as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(_) | Value::Object(_) => match v.get("$num").and_then(Value::as_str) {
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            Some("-0") => -0.0,
            _ => f64::NAN,
        },
    }
}

fn is_record(v: &Value) -> bool {
    matches!(v, Value::Object(_) | Value::Array(_)) && !is_undefined(v)
}

// --- Dates -----------------------------------------------------------------

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as i64 + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `new Date(ms).toISOString()` — `Err(INVALID_TIME)` outside `±8.64e15`.
fn iso(ms: f64) -> Result<String, String> {
    if !ms.is_finite() || ms.abs() > 8_640_000_000_000_000.0 {
        return Err(INVALID_TIME.to_string());
    }
    let ms = ms.trunc();
    let days = (ms / DAY_MS).floor() as i64;
    let rem = (ms - days as f64 * DAY_MS) as i64;
    let (y, mo, d) = civil(days);
    let (hh, mm, ss, mss) = (
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1_000 % 60,
        rem % 1_000,
    );
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else if y < 0 {
        format!("-{:06}", -y)
    } else {
        format!("+{y:06}")
    };
    Ok(format!(
        "{year}-{mo:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{mss:03}Z"
    ))
}

regex!(
    iso_ts_pattern,
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,3})?Z$"
);
regex!(calendar_day, r"^(\d{4})-(\d{2})-(\d{2})$");

/// `Date.parse` for the ISO shapes this unit parses (`YYYY-MM-DD` and the
/// strict product timestamp); every other string is `NaN` for recorded inputs.
fn js_date_parse(s: &str) -> f64 {
    if let Some(c) = calendar_day().captures(s)
        && s.len() == 10
    {
        let y: i64 = c[1].parse().unwrap_or(0);
        let mo: u32 = c[2].parse().unwrap_or(0);
        let d: u32 = c[3].parse().unwrap_or(0);
        if (1..=12).contains(&mo) && (1..=31).contains(&d) {
            return days_from_civil(y, mo, d) as f64 * DAY_MS;
        }
        return f64::NAN;
    }
    if iso_ts_pattern().is_match(s) {
        let y: i64 = s[0..4].parse().unwrap_or(0);
        let mo: u32 = s[5..7].parse().unwrap_or(0);
        let d: u32 = s[8..10].parse().unwrap_or(0);
        let hh: f64 = s[11..13].parse().unwrap_or(0.0);
        let mm: f64 = s[14..16].parse().unwrap_or(0.0);
        let ss: f64 = s[17..19].parse().unwrap_or(0.0);
        if !(1..=12).contains(&mo)
            || !(1..=31).contains(&d)
            || hh >= 24.0
            || mm >= 60.0
            || ss >= 60.0
        {
            return f64::NAN;
        }
        let frac = if s.len() > 19 {
            // ".ddd" — 1 to 3 digits, padded right to milliseconds.
            let f = &s[20..s.len() - 1];
            f.parse::<f64>().unwrap_or(0.0) * 10f64.powi(3 - f.len() as i32)
        } else {
            0.0
        };
        return days_from_civil(y, mo, d) as f64 * DAY_MS
            + hh * 3_600_000.0
            + mm * 60_000.0
            + ss * 1_000.0
            + frac;
    }
    f64::NAN
}

fn valid_calendar_date(value: &str) -> bool {
    if !calendar_day().is_match(value) {
        return false;
    }
    let ms = js_date_parse(value);
    ms.is_finite() && iso(ms).is_ok_and(|s| s[0..10] == *value)
}

// --- product event ----------------------------------------------------------

const PRODUCT_EVENTS: [&str; 10] = [
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
const PRODUCT_OUTCOMES: [&str; 4] = ["started", "succeeded", "failed", "cancelled"];
const PRODUCT_FEATURES: [&str; 9] = [
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
const PRODUCT_MODES: [&str; 2] = ["local", "remote"];
const PRODUCT_PROVIDERS: [&str; 3] = ["codex", "claude", "other"];
const PRODUCT_CHANNELS: [&str; 3] = ["stable", "beta", "remote"];
const PRODUCT_REASONS: [&str; 10] = [
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
const PRODUCT_TURN_FAILURES: [&str; 14] = [
    "edge_blocked",
    "auth",
    "account",
    "billing",
    "rate_limit",
    "usage_limit",
    "overloaded",
    "server_error",
    "network",
    "invalid_request",
    "model_unavailable",
    "context_limit",
    "output_limit",
    "other",
];
const PRODUCT_SURFACES: [&str; 3] = ["desktop", "ios", "ipados"];
const PRODUCT_PLATFORMS: [&str; 6] = ["darwin", "win32", "linux", "ios", "ipados", "other"];
const PRODUCT_MAX_DURATION_MS: f64 = 86_400_000.0;
const PRODUCT_MAX_TOKEN_COUNT: f64 = 1_000_000_000_000.0;
const PRODUCT_MAX_EVENT_AGE_MS: f64 = 7.0 * DAY_MS;
const PRODUCT_MAX_EVENT_FUTURE_MS: f64 = 300_000.0;
const PRODUCT_COHORT_WINDOWS: [(&str, f64, f64); 3] = [
    ("new", 0.0, 0.0),
    ("week1", 7.0, 13.0),
    ("month1", 30.0, 36.0),
];

regex!(
    uuid_pattern,
    r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
);
regex!(
    product_version_pattern,
    r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$"
);

const COMMANDS: [&str; 80] = [
    "shortcut:terminal.splitUp",
    "shortcut:terminal.new",
    "shortcut:terminal.close",
    "shortcut:terminal.workspace.newFullWidth",
    "shortcut:terminal.workspace.closeActive",
    "shortcut:terminal.workspace.terminal",
    "shortcut:terminal.workspace.chat",
    "shortcut:browser.toggle",
    "shortcut:device.toggle",
    "shortcut:diff.toggle",
    "shortcut:diff.change.next",
    "shortcut:diff.change.previous",
    "shortcut:composer.focus.toggle",
    "shortcut:chat.find",
    "shortcut:modelPicker.toggle",
    "shortcut:model.next",
    "shortcut:model.previous",
    "shortcut:model.effort.next",
    "shortcut:traitsPicker.toggle",
    "shortcut:settings.usage",
    "shortcut:chat.new",
    "shortcut:chat.newLatestProject",
    "shortcut:chat.newTerminal",
    "shortcut:chat.newClaude",
    "shortcut:chat.newCodex",
    "shortcut:chat.newCursor",
    "shortcut:chat.split",
    "shortcut:sidechat.toggle",
    "shortcut:view.recent.next",
    "shortcut:view.recent.previous",
    "shortcut:thread.jump",
    "shortcut:thread.copyId",
    "shortcut:thread.archive",
    "shortcut:thread.snooze",
    "shortcut:thread.markUnread",
    "shortcut:chat.visible.next",
    "shortcut:chat.visible.previous",
    "shortcut:chat.attention.next",
    "shortcut:threadTab.next",
    "shortcut:threadTab.previous",
    "shortcut:threadTab.close",
    "shortcut:editor.openFavorite",
    "shortcut:editor.file.save",
    "shortcut:editor.commands",
    "shortcut:editor.rename",
    "shortcut:editor.goToDefinition",
    "shortcut:editor.findReferences",
    "shortcut:editor.goToSymbol",
    "shortcut:editor.goToLine",
    "shortcut:editor.problems",
    "shortcut:editor.quickFix",
    "shortcut:editor.debug.start",
    "shortcut:editor.debug.stop",
    "shortcut:editor.debug.toggleBreakpoint",
    "shortcut:editor.debug.stepOver",
    "shortcut:editor.debug.stepInto",
    "shortcut:editor.debug.stepOut",
    "shortcut:git.commitAndPush",
    "shortcut:search.files",
    "shortcut:search.content",
    "shortcut:custom",
    "slash:clear",
    "slash:compact",
    "slash:model",
    "slash:plan",
    "slash:debug",
    "slash:default",
    "slash:review",
    "slash:fork",
    "slash:side",
    "slash:status",
    "slash:subagents",
    "slash:computer-use",
    "slash:fast",
    "slash:export",
    "slash:goal",
    "slash:rename",
    "slash:feedback",
    "slash:automation",
    "slash:custom",
];

fn is_product_analytics_command(value: &Value) -> bool {
    match as_js_str(value) {
        Some(s) => COMMANDS.contains(&s.as_str()),
        None => false,
    }
}

fn now_value(now: &Option<Value>) -> f64 {
    now.as_ref().map(js_to_num).unwrap_or(f64::NAN)
}

/// Port of `normalizeProductEvent` (`apps/analytics/worker.ts:283`).
pub fn normalize_product_event(
    raw: serde_json::Value,
    now: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let now = now_value(&now);
    if !matches!(raw, Value::Object(_))
        || prop(&raw, "v")?.and_then(crate::js::as_number) != Some(1.0)
    {
        return Ok(None);
    }
    let obj = raw.as_object().expect("object");
    let get = |k: &str| obj.get(k);
    let id = get("id").and_then(as_js_str);
    let Some(_id) = id.filter(|s| uuid_pattern().is_match(s)) else {
        return Ok(None);
    };
    let install_id = get("installId").and_then(as_js_str);
    let Some(_install_id) = install_id.filter(|s| uuid_pattern().is_match(s)) else {
        return Ok(None);
    };
    let channel = get("channel").and_then(as_js_str);
    let Some(channel) = channel.filter(|s| PRODUCT_CHANNELS.contains(&s.as_str())) else {
        return Ok(None);
    };
    let surface = get("surface")
        .map(js_to_string)
        .unwrap_or_else(|| "undefined".to_string());
    let platform = get("platform")
        .map(js_to_string)
        .unwrap_or_else(|| "undefined".to_string());
    if !PRODUCT_SURFACES.contains(&surface.as_str())
        || !PRODUCT_PLATFORMS.contains(&platform.as_str())
    {
        return Ok(None);
    }
    let app_version = get("appVersion").and_then(as_js_str);
    let Some(app_version) = app_version
        .filter(|s| s.encode_utf16().count() <= 64 && product_version_pattern().is_match(s))
    else {
        return Ok(None);
    };
    let event = get("event").and_then(as_js_str);
    let Some(event) = event.filter(|s| PRODUCT_EVENTS.contains(&s.as_str())) else {
        return Ok(None);
    };
    let outcome = get("outcome").and_then(as_js_str);
    let Some(outcome) = outcome.filter(|s| PRODUCT_OUTCOMES.contains(&s.as_str())) else {
        return Ok(None);
    };
    let ts = get("ts").and_then(as_js_str);
    let Some(ts) = ts.filter(|s| iso_ts_pattern().is_match(s)) else {
        return Ok(None);
    };
    let ts_ms = js_date_parse(&ts);
    let iso_ts = if ts_ms.is_finite() {
        iso(ts_ms)?
    } else {
        return Ok(None);
    };
    if iso_ts[0..10] != ts[0..10]
        || ts_ms < now - PRODUCT_MAX_EVENT_AGE_MS
        || ts_ms > now + PRODUCT_MAX_EVENT_FUTURE_MS
    {
        return Ok(None);
    }

    let optional_enum = |key: &str, allowed: &[&str]| -> Option<Option<String>> {
        match get(key) {
            None => Some(None),
            Some(v) => as_js_str(v).and_then(|s| {
                if allowed.contains(&s.as_str()) {
                    Some(Some(s))
                } else {
                    None
                }
            }),
        }
    };
    let Some(feature) = optional_enum("feature", &PRODUCT_FEATURES) else {
        return Ok(None);
    };
    let Some(mode) = optional_enum("mode", &PRODUCT_MODES) else {
        return Ok(None);
    };
    let Some(provider) = optional_enum("provider", &PRODUCT_PROVIDERS) else {
        return Ok(None);
    };
    let Some(reason) = optional_enum("reason", &PRODUCT_REASONS) else {
        return Ok(None);
    };
    if reason.is_some() && outcome != "failed" {
        return Ok(None);
    }
    let failure: Option<String> = match get("failure") {
        None => None,
        Some(v) => match as_js_str(v) {
            Some(s)
                if PRODUCT_TURN_FAILURES.contains(&s.as_str())
                    && event == "turn.completed"
                    && (outcome == "failed" || outcome == "cancelled") =>
            {
                Some(s)
            }
            _ => return Ok(None),
        },
    };
    let http_status: Option<f64> = match get("httpStatus") {
        None => None,
        Some(v) => match crate::js::as_number(v) {
            Some(n) if n.fract() == 0.0 && (100.0..=599.0).contains(&n) && failure.is_some() => {
                Some(n)
            }
            _ => return Ok(None),
        },
    };
    let command: Option<String> = match get("command") {
        None => None,
        Some(v) if is_product_analytics_command(v) => as_js_str(v),
        Some(_) => return Ok(None),
    };
    if command.is_none() == (event == "command.used") {
        return Ok(None);
    }

    let optional_count = |key: &str, max: f64| -> Option<Option<f64>> {
        match get(key) {
            None => Some(None),
            Some(v) => crate::js::as_number(v).and_then(|n| {
                if n.is_finite() && n.fract() == 0.0 && n >= 0.0 && n <= max {
                    Some(Some(n))
                } else {
                    None
                }
            }),
        }
    };
    let Some(duration_ms) = optional_count("durationMs", PRODUCT_MAX_DURATION_MS) else {
        return Ok(None);
    };
    let Some(input_tokens) = optional_count("inputTokens", PRODUCT_MAX_TOKEN_COUNT) else {
        return Ok(None);
    };
    let Some(output_tokens) = optional_count("outputTokens", PRODUCT_MAX_TOKEN_COUNT) else {
        return Ok(None);
    };
    let Some(cached_input_tokens) = optional_count("cachedInputTokens", PRODUCT_MAX_TOKEN_COUNT)
    else {
        return Ok(None);
    };

    let mut cohort_day: Option<String> = None;
    let mut cohort_stage: Option<String> = None;
    if get("cohortDay").is_some() || get("cohortStage").is_some() {
        let Some(day) = get("cohortDay").and_then(as_js_str) else {
            return Ok(None);
        };
        let Some(stage) = get("cohortStage").and_then(as_js_str) else {
            return Ok(None);
        };
        let window = PRODUCT_COHORT_WINDOWS
            .iter()
            .find(|(name, _, _)| *name == stage);
        let Some((_, first, last)) = window else {
            return Ok(None);
        };
        if !valid_calendar_date(&day) || event != "app.active" || outcome != "succeeded" {
            return Ok(None);
        }
        let age_days = (js_date_parse(&ts[0..10]) - js_date_parse(&day)) / DAY_MS;
        if age_days < *first || age_days > *last {
            return Ok(None);
        }
        cohort_day = Some(day);
        cohort_stage = Some(stage);
    }

    let str_or_null = |v: Option<String>| v.map_or(Value::Null, Value::String);
    let num_or_null = |v: Option<f64>| v.map_or(Value::Null, crate::js::number);
    let mut out = Map::new();
    out.insert("id".to_string(), get("id").cloned().unwrap_or(Value::Null));
    out.insert("ts".to_string(), Value::String(iso_ts));
    out.insert(
        "installId".to_string(),
        get("installId").cloned().unwrap_or(Value::Null),
    );
    out.insert("cohortDay".to_string(), str_or_null(cohort_day));
    out.insert("cohortStage".to_string(), str_or_null(cohort_stage));
    out.insert("channel".to_string(), Value::String(channel));
    out.insert("surface".to_string(), Value::String(surface));
    out.insert("platform".to_string(), Value::String(platform));
    out.insert("appVersion".to_string(), Value::String(app_version));
    out.insert("event".to_string(), Value::String(event.clone()));
    out.insert("outcome".to_string(), Value::String(outcome));
    out.insert("feature".to_string(), str_or_null(feature));
    out.insert("mode".to_string(), str_or_null(mode));
    out.insert("provider".to_string(), str_or_null(provider));
    out.insert("reason".to_string(), str_or_null(reason));
    out.insert("failure".to_string(), str_or_null(failure));
    out.insert("httpStatus".to_string(), num_or_null(http_status));
    out.insert("command".to_string(), str_or_null(command));
    out.insert("durationMs".to_string(), num_or_null(duration_ms));
    out.insert("inputTokens".to_string(), num_or_null(input_tokens));
    out.insert("outputTokens".to_string(), num_or_null(output_tokens));
    out.insert(
        "cachedInputTokens".to_string(),
        num_or_null(cached_input_tokens),
    );
    Ok(Some(Value::Object(out)))
}

// --- diagnostics sanitizers (vendored from diagnosticsredaction-rs) ---------

const DIAGNOSTIC_CODES: [&str; 19] = [
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
const DIAGNOSTIC_REASONS: [&str; 7] = [
    "unknown",
    "auth",
    "invalid-response",
    "timeout",
    "output-limit",
    "live-owner",
    "unknown-owner",
];
const DIAGNOSTIC_PROCESSES: [&str; 12] = [
    "main",
    "renderer",
    "backend",
    "server",
    "Browser",
    "GPU",
    "Utility",
    "Zygote",
    "Sandbox helper",
    "Pepper Plugin",
    "Pepper Plugin Broker",
    "unknown",
];
const DIAGNOSTIC_CRASH_REASONS: [&str; 9] = [
    "clean-exit",
    "abnormal-exit",
    "killed",
    "crashed",
    "oom",
    "launch-failed",
    "integrity-failure",
    "memory-eviction",
    "unknown",
];
const DIAGNOSTIC_SIGNALS: [&str; 33] = [
    "SIGABRT",
    "SIGALRM",
    "SIGBUS",
    "SIGCHLD",
    "SIGCONT",
    "SIGFPE",
    "SIGHUP",
    "SIGILL",
    "SIGINT",
    "SIGIO",
    "SIGIOT",
    "SIGKILL",
    "SIGPIPE",
    "SIGPOLL",
    "SIGPROF",
    "SIGPWR",
    "SIGQUIT",
    "SIGSEGV",
    "SIGSTKFLT",
    "SIGSTOP",
    "SIGSYS",
    "SIGTERM",
    "SIGTRAP",
    "SIGTSTP",
    "SIGTTIN",
    "SIGTTOU",
    "SIGURG",
    "SIGUSR1",
    "SIGUSR2",
    "SIGVTALRM",
    "SIGWINCH",
    "SIGXCPU",
    "SIGXFSZ",
];
const UPDATE_FAILURE_REASONS: [&str; 9] = [
    "timeout",
    "network",
    "http-client",
    "http-server",
    "manifest",
    "release-feed",
    "verification",
    "cancelled",
    "unknown",
];

const WS: &str =
    "\\t-\\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

regex!(
    handled_issue,
    r"^Handled issue: ([a-z0-9.-]+) \(([A-Za-z0-9._-]+)\)$"
);
regex!(duration_ms_re, r"^durationMs=([0-9]{1,9})(?:\n|$)");
regex!(
    stack_frame,
    &"^[WS]*at[WS]+[^\n\r\u{2028}\u{2029}]*?:([0-9]{1,8}):([0-9]{1,8})\\)?[WS]*$".replace("WS", WS)
);
regex!(
    error_type,
    r"^(?:Error|TypeError|RangeError|ReferenceError|SyntaxError|URIError|EvalError|AggregateError)"
);
regex!(
    exit_reason,
    r"^code=(-?[0-9]{1,10}|null)(?: signal=(SIG[A-Z0-9]{1,12}|null))?$"
);

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn exec_check<'a>(re: &'a Regex, text: &'a str, next_word_boundary: bool) -> Option<Captures<'a>> {
    let caps = re.captures(text)?;
    if next_word_boundary {
        let e = caps.get(0).expect("group 0").end();
        if text[e..].chars().next().is_some_and(is_word_char) {
            return None;
        }
    }
    Some(caps)
}

fn sanitize_error(input: &Value) -> Result<(Value, Value), String> {
    let message = prop(input, "message")?;
    let stack = prop(input, "stack")?;
    let undef = crate::js::undefined();
    let stack_raw = stack
        .filter(|v| !is_undefined(v) && !v.is_null())
        .unwrap_or(&undef);
    let stack_str = stack_raw
        .as_str()
        .map(str::to_string)
        .or_else(|| as_js_str(stack_raw));
    let message_str = message.map(js_to_string).unwrap_or_default();
    if let Some(issue) = handled_issue().captures(&message_str) {
        let code = issue.get(1).expect("group 1").as_str();
        let reason = issue.get(2).expect("group 2").as_str();
        if DIAGNOSTIC_CODES.contains(&code) && DIAGNOSTIC_REASONS.contains(&reason) {
            let stack_for_duration = match stack_raw {
                v if is_undefined(v) || v.is_null() => String::new(),
                other => js_to_string(other),
            };
            let mut obj = Map::new();
            obj.insert(
                "message".to_string(),
                Value::String(format!("Handled issue: {code} ({reason})")),
            );
            if let Some(d) = duration_ms_re().captures(&stack_for_duration) {
                let n: f64 = d
                    .get(1)
                    .expect("group 1")
                    .as_str()
                    .parse()
                    .unwrap_or(f64::NAN);
                if n <= 604_800_000.0 {
                    obj.insert(
                        "stack".to_string(),
                        Value::String(format!("durationMs={}", crate::js::number_to_string(n))),
                    );
                }
            }
            return Ok((Value::Object(obj), input.clone()));
        }
    }
    let stack_string = match stack_raw {
        v if is_undefined(v) || v.is_null() => String::new(),
        other => match as_js_str(other) {
            Some(s) => s,
            None => {
                return Err(format!(
                    "input.stack.slice is not a function ({} )",
                    js_to_string(other)
                ));
            }
        },
    };
    let source = prop(input, "source")?;
    let source_str = source
        .map(js_to_string)
        .unwrap_or_else(|| "undefined".to_string());
    let sliced = {
        let units: Vec<u16> = stack_string.encode_utf16().collect();
        crate::utf16::slice(&units, 0, 8192)
    };
    let frames: Vec<String> = sliced
        .split('\n')
        .filter_map(|line| {
            stack_frame().captures(line).map(|c| {
                format!(
                    "at {}:{}:{}",
                    source_str,
                    c.get(1).expect("group 1").as_str(),
                    c.get(2).expect("group 2").as_str()
                )
            })
        })
        .take(30)
        .collect();
    let stack_exec = stack_str.as_deref().unwrap_or("");
    let ty = exec_check(error_type(), &message_str, true)
        .or_else(|| exec_check(error_type(), stack_exec, true))
        .and_then(|c| c.get(0).map(|m| m.as_str()))
        .unwrap_or("Error");
    let mut obj = Map::new();
    obj.insert(
        "message".to_string(),
        Value::String(if let Some(first) = frames.first() {
            format!("{ty} {first}")
        } else {
            ty.to_string()
        }),
    );
    if !frames.is_empty() {
        obj.insert("stack".to_string(), Value::String(frames.join("\n")));
    }
    Ok((Value::Object(obj), input.clone()))
}

fn sanitize_diagnostic_error(input: &Value) -> Result<Value, String> {
    sanitize_error(input).map(|(v, _)| v)
}

fn sanitize_diagnostic_crash(input: &Value) -> Result<Value, String> {
    let process_type_raw = prop(input, "processType")?;
    let reason_raw = prop(input, "reason")?;
    let process_type = match process_type_raw.and_then(as_js_str) {
        Some(s) if DIAGNOSTIC_PROCESSES.contains(&s.as_str()) => s,
        _ => "unknown".to_string(),
    };
    let reason_str = reason_raw.and_then(as_js_str);
    if let Some(r) = reason_str.as_deref()
        && DIAGNOSTIC_CRASH_REASONS.contains(&r)
    {
        let mut obj = Map::new();
        obj.insert("processType".to_string(), Value::String(process_type));
        obj.insert("reason".to_string(), Value::String(r.to_string()));
        return Ok(Value::Object(obj));
    }
    let exit = reason_str
        .as_deref()
        .and_then(|r| exit_reason().captures(r));
    if let Some(exit) = exit {
        let signal = exit.get(2).map(|m| m.as_str());
        let signal_ok = signal.is_none()
            || signal == Some("null")
            || signal.is_some_and(|s| DIAGNOSTIC_SIGNALS.contains(&s));
        if (process_type == "backend" || process_type == "server") && signal_ok {
            let code = match exit.get(1).expect("group 1").as_str() {
                "null" => "null".to_string(),
                digits => {
                    let n: f64 = digits.parse().unwrap_or(0.0);
                    crate::js::number_to_string(n)
                }
            };
            let reason = match signal {
                None => format!("code={code}"),
                Some(sig) => format!("code={code} signal={sig}"),
            };
            let mut obj = Map::new();
            obj.insert("processType".to_string(), Value::String(process_type));
            obj.insert("reason".to_string(), Value::String(reason));
            return Ok(Value::Object(obj));
        }
    }
    let mut obj = Map::new();
    obj.insert("processType".to_string(), Value::String(process_type));
    obj.insert("reason".to_string(), Value::String("unknown".to_string()));
    Ok(Value::Object(obj))
}

fn sanitize_desktop_update_failure_reason(reason: Option<&Value>) -> Option<String> {
    reason
        .and_then(as_js_str)
        .filter(|s| UPDATE_FAILURE_REASONS.contains(&s.as_str()))
}

fn diagnostic_error_fingerprint(input: &Value) -> Result<String, String> {
    let (safe, raw) = sanitize_error(input)?;
    let safe_message = prop(&safe, "message")?
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let key = if safe_message.starts_with("Handled issue: ") {
        safe_message
    } else {
        let source = prop(&raw, "source")?
            .map(js_to_string)
            .unwrap_or_else(|| "undefined".to_string());
        let safe_stack = prop(&safe, "stack")?
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        format!("{source}\n{safe_message}\n{safe_stack}")
    };
    let mut hash: u64 = 0xcbf29ce484222325;
    for ch in key.chars() {
        let mut buf = [0u16; 2];
        let unit = ch.encode_utf16(&mut buf)[0];
        hash = (hash ^ u64::from(unit)).wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:016x}"))
}

fn diagnostic_payload_kind(event: &str) -> Option<&'static str> {
    match event {
        "app.start" | "app.exit" => Some("lifecycle"),
        "app.error" => Some("error"),
        "app.renderer-crash" | "app.child-process-crash" => Some("crash"),
        "update.check" | "update.available" | "update.downloaded" | "update.installed"
        | "update.error" => Some("update"),
        "usage.daily" => Some("usage"),
        "beta.installed" | "beta.left" => Some("beta"),
        _ => None,
    }
}

// --- diagnostics event ------------------------------------------------------

const KNOWN_EVENTS: [&str; 13] = [
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
const MAX_EVENT_FUTURE_MS: f64 = DAY_MS;
const MAX_EVENT_AGE_MS: f64 = 30.0 * DAY_MS;
const VERSION_PATTERN_STR: &str = r"^[0-9]+\.[0-9]+\.[0-9]+(?:-(?:beta|remote|canary)\.[0-9]+)?$";
const BETA_INSTALLED_OUTCOMES: [&str; 3] = ["imported", "import-failed", "fresh"];
const BETA_LEFT_OUTCOMES: [&str; 2] = ["trash", "keep"];
const USAGE_COUNT_MAX: f64 = 100_000.0;

regex!(version_pattern, VERSION_PATTERN_STR);
regex!(os_version_pattern, r"^\d{1,4}(\.\d{1,4})?$");
regex!(locale_pattern, r"^[a-z]{2,3}$");

fn clamp_usage_count(value: &Value) -> f64 {
    let count = crate::js::as_number(value)
        .map(|n| n.floor())
        .unwrap_or(f64::NAN);
    if !count.is_finite() || count < 0.0 {
        return 0.0;
    }
    count.min(USAGE_COUNT_MAX)
}

/// Port of `normalizeEvent` (`apps/analytics/worker.ts:550`).
pub fn normalize_event(
    raw: serde_json::Value,
    now: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let now = now_value(&now);
    if !is_record(&raw) {
        return Ok(None);
    }
    let event = prop(&raw, "event")?.and_then(as_js_str);
    if prop(&raw, "v")?.and_then(crate::js::as_number) != Some(1.0) {
        return Ok(None);
    }
    let Some(event) = event.filter(|e| KNOWN_EVENTS.contains(&e.as_str())) else {
        return Ok(None);
    };
    let install_id = prop(&raw, "installId")?.and_then(as_js_str);
    let Some(install_id) = install_id.filter(|s| uuid_pattern().is_match(s)) else {
        return Ok(None);
    };
    let flavor_v = prop(&raw, "flavor")?;
    if flavor_v != Some(&Value::String("beta".to_string()))
        && flavor_v != Some(&Value::String("remote".to_string()))
    {
        return Ok(None);
    }
    let flavor = as_js_str(flavor_v.expect("checked")).unwrap_or_default();
    let platform_raw = prop(&raw, "platform")?.and_then(as_js_str);
    let Some(platform_raw) = platform_raw.filter(|s| s.encode_utf16().count() <= 16) else {
        return Ok(None);
    };
    let arch_raw = prop(&raw, "arch")?.and_then(as_js_str);
    let Some(arch_raw) = arch_raw.filter(|s| s.encode_utf16().count() <= 16) else {
        return Ok(None);
    };
    let app_version = prop(&raw, "appVersion")?.and_then(as_js_str);
    let Some(app_version) =
        app_version.filter(|s| s.encode_utf16().count() <= 32 && version_pattern().is_match(s))
    else {
        return Ok(None);
    };
    let ts = prop(&raw, "ts")?.and_then(as_js_str);
    let Some(ts) = ts.filter(|s| s.encode_utf16().count() <= 64) else {
        return Ok(None);
    };
    let ts_ms = js_date_parse(&ts);
    if ts_ms.is_nan() || ts_ms > now + MAX_EVENT_FUTURE_MS || ts_ms < now - MAX_EVENT_AGE_MS {
        return Ok(None);
    }
    let client_id = prop(&raw, "id")?
        .and_then(as_js_str)
        .filter(|s| uuid_pattern().is_match(s));

    let payload = match prop(&raw, "payload")? {
        Some(v) if is_record(v) => v.clone(),
        _ => Value::Object(Map::new()),
    };
    let Some(kind) = diagnostic_payload_kind(&event) else {
        return Ok(None);
    };
    let payload_kind = prop(&payload, "kind")?;
    if payload_kind != Some(&Value::String(kind.to_string())) {
        return Ok(None);
    }
    let mut outcome = if prop(&payload, "outcome")? == Some(&Value::String("error".to_string())) {
        "error".to_string()
    } else {
        "ok".to_string()
    };
    if event == "beta.installed" || event == "beta.left" {
        let allowed = if event == "beta.installed" {
            &BETA_INSTALLED_OUTCOMES[..]
        } else {
            &BETA_LEFT_OUTCOMES[..]
        };
        let Some(oc) = prop(&payload, "outcome")?
            .and_then(as_js_str)
            .filter(|s| allowed.contains(&s.as_str()))
        else {
            return Ok(None);
        };
        outcome = oc;
    }
    let crash = if kind == "crash" {
        Some(sanitize_diagnostic_crash(&payload)?)
    } else {
        None
    };
    let process_type = crash
        .as_ref()
        .and_then(|c| c.get("processType"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let reason = if kind == "update" || event.starts_with("update.") {
        if event == "update.error" && kind == "update" && outcome == "error" {
            sanitize_desktop_update_failure_reason(prop(&payload, "reason")?).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        crash
            .as_ref()
            .and_then(|c| c.get("reason"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let error_context = prop(&payload, "errorContext")?
        .and_then(as_js_str)
        .filter(|s| matches!(s.as_str(), "check" | "download" | "install"))
        .unwrap_or_default();
    let target_version = prop(&payload, "targetVersion")?
        .and_then(as_js_str)
        .filter(|s| version_pattern().is_match(s))
        .unwrap_or_default();
    let source = prop(&payload, "source")?
        .and_then(as_js_str)
        .filter(|s| matches!(s.as_str(), "renderer" | "main"))
        .unwrap_or_default();
    let technical = if kind == "error" {
        let mut t = Map::new();
        t.insert(
            "source".to_string(),
            Value::String(if source == "renderer" {
                "renderer".to_string()
            } else {
                "main".to_string()
            }),
        );
        t.insert(
            "message".to_string(),
            prop(&payload, "message")?
                .and_then(as_js_str)
                .map(Value::String)
                .unwrap_or_else(|| Value::String(String::new())),
        );
        if let Some(s) = prop(&payload, "stack")?.and_then(as_js_str) {
            t.insert("stack".to_string(), Value::String(s));
        }
        Some(sanitize_diagnostic_error(&Value::Object(t))?)
    } else {
        None
    };
    let message = technical
        .as_ref()
        .and_then(|t| t.get("message"))
        .cloned()
        .unwrap_or(Value::Null);
    let stack = technical
        .as_ref()
        .and_then(|t| t.get("stack"))
        .cloned()
        .unwrap_or(Value::Null);
    let fingerprint = match &technical {
        Some(_) => {
            let mut fp = Map::new();
            fp.insert(
                "source".to_string(),
                Value::String(if source == "renderer" {
                    "renderer".to_string()
                } else {
                    "main".to_string()
                }),
            );
            if let Some(t) = &technical {
                for (k, v) in t.as_object().expect("object") {
                    fp.insert(k.clone(), v.clone());
                }
            }
            diagnostic_error_fingerprint(&Value::Object(fp))?
        }
        None => String::new(),
    };
    let os_version = if kind == "lifecycle" {
        prop(&payload, "osVersion")?
            .and_then(as_js_str)
            .filter(|s| os_version_pattern().is_match(s))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let locale = if kind == "lifecycle" {
        prop(&payload, "locale")?
            .and_then(as_js_str)
            .filter(|s| locale_pattern().is_match(s))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let (mut projects, mut active_threads) = (Value::Null, Value::Null);
    if event == "usage.daily" {
        projects = match prop(&payload, "projects")? {
            None => Value::Null,
            Some(v) => crate::js::number(clamp_usage_count(v)),
        };
        active_threads = match prop(&payload, "activeThreads")? {
            None => Value::Null,
            Some(v) => crate::js::number(clamp_usage_count(v)),
        };
    }
    let duration_ms = match prop(&payload, "durationMs")? {
        Some(v) => crate::js::as_number(v)
            .filter(|n| n.is_finite() && *n >= 0.0)
            .map_or(Value::Null, crate::js::number),
        None => Value::Null,
    };
    let platform = if matches!(platform_raw.as_str(), "darwin" | "win32" | "linux") {
        platform_raw.clone()
    } else {
        "unknown".to_string()
    };
    let arch = if matches!(arch_raw.as_str(), "arm64" | "x64" | "ia32" | "arm") {
        arch_raw.clone()
    } else {
        "unknown".to_string()
    };

    let mut out = Map::new();
    out.insert(
        "clientId".to_string(),
        client_id.map_or(Value::Null, Value::String),
    );
    out.insert("ts".to_string(), Value::String(iso(ts_ms)?));
    out.insert("installId".to_string(), Value::String(install_id));
    out.insert("flavor".to_string(), Value::String(flavor));
    out.insert("appVersion".to_string(), Value::String(app_version));
    out.insert("platform".to_string(), Value::String(platform));
    out.insert("arch".to_string(), Value::String(arch));
    out.insert("event".to_string(), Value::String(event));
    out.insert("kind".to_string(), Value::String(kind.to_string()));
    out.insert("outcome".to_string(), Value::String(outcome));
    out.insert("processType".to_string(), Value::String(process_type));
    out.insert("reason".to_string(), Value::String(reason));
    out.insert("errorContext".to_string(), Value::String(error_context));
    out.insert("targetVersion".to_string(), Value::String(target_version));
    out.insert("durationMs".to_string(), duration_ms);
    out.insert("source".to_string(), Value::String(source));
    out.insert("fingerprint".to_string(), Value::String(fingerprint));
    out.insert("message".to_string(), message);
    out.insert("stack".to_string(), stack);
    out.insert("logTail".to_string(), Value::Null);
    out.insert("osVersion".to_string(), Value::String(os_version));
    out.insert("locale".to_string(), Value::String(locale));
    out.insert("projects".to_string(), projects);
    out.insert("activeThreads".to_string(), active_threads);
    Ok(Some(Value::Object(out)))
}

// --- sessions ---------------------------------------------------------------

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        let d = Sha256::digest(key);
        k[..d.len()].copy_from_slice(&d);
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let inner = Sha256::digest([ipad.as_slice(), msg].concat());
    Sha256::digest([opad.as_slice(), inner.as_slice()].concat()).to_vec()
}

fn encode_base64url(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len() * 4 / 3 + 2);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        out.push(T[(b[0] >> 2) as usize] as char);
        out.push(T[(((b[0] & 3) << 4) | (b[1] >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[(((b[1] & 15) << 2) | (b[2] >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(T[(b[2] & 63) as usize] as char);
        }
    }
    out
}

fn timing_safe_equal(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// Port of `signSession` (`apps/analytics/worker.ts:798`).
pub async fn sign_session(secret: String, expires_at_seconds: f64) -> Result<String, String> {
    if secret.is_empty() {
        return Err("Zero-length key is not supported".to_string());
    }
    let payload = crate::js::number_to_string(expires_at_seconds);
    let sig = encode_base64url(&hmac_sha256(secret.as_bytes(), payload.as_bytes()));
    Ok(format!("{payload}.{sig}"))
}

regex!(
    session_cookie,
    &"(?:^|;[WS]*)synara_beta_dash=([0-9]+\\.[A-Za-z0-9_-]+)".replace("WS", WS)
);

/// Port of `verifySessionCookie` (`apps/analytics/worker.ts:818`).
pub async fn verify_session_cookie(
    cookie_header: Option<String>,
    secret: String,
    now_seconds: Option<serde_json::Value>,
) -> Result<bool, String> {
    let Some(header) = cookie_header.filter(|h| !h.is_empty()) else {
        return Ok(false);
    };
    let Some(m) = session_cookie().captures(&header) else {
        return Ok(false);
    };
    let captured = &m[1];
    let dot = captured.find('.').expect("pattern has a literal dot");
    let expiry_text = &captured[..dot];
    let signature = &captured[dot + 1..];
    let expires_at: f64 = expiry_text.parse().unwrap_or(f64::NAN);
    let now = now_seconds.as_ref().map(js_to_num).unwrap_or(f64::NAN);
    if !expires_at.is_finite() || expires_at < now {
        return Ok(false);
    }
    if secret.is_empty() {
        return Err("Zero-length key is not supported".to_string());
    }
    let expected = encode_base64url(&hmac_sha256(secret.as_bytes(), expiry_text.as_bytes()));
    Ok(timing_safe_equal(signature.as_bytes(), expected.as_bytes()))
}

// --- dashboard filters ------------------------------------------------------

/// Port of `parseApiFilters` (`apps/analytics/worker.ts:1013`).
///
/// Every recorded call passes a plain JSON value for `url`; `url.searchParams`
/// is `undefined` there so the original throws before reading anything else.
pub fn parse_api_filters(
    url: serde_json::Value,
    now: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = &now;
    if url.is_null() {
        return Err("Cannot read properties of null (reading 'searchParams')".to_string());
    }
    if is_undefined(&url) {
        return Err("Cannot read properties of undefined (reading 'searchParams')".to_string());
    }
    Err("url.searchParams.get is not a function".to_string())
}

// --- versions ---------------------------------------------------------------

fn is_ignorable_collation(c: char) -> bool {
    matches!(c,
        '\u{ad}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}'
        | '\u{2060}'..='\u{2064}' | '\u{feff}' | '\u{fe00}'..='\u{fe0f}'
        | '\u{e0100}'..='\u{e01ef}')
}

fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

fn is_punct(c: char) -> bool {
    matches!(c as u32,
        0x21..=0x2f | 0x3a..=0x40 | 0x5b..=0x60 | 0x7b..=0x7e)
}

fn is_symbol(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x24 | 0x2b | 0x3c..=0x3e | 0x5e | 0x60 | 0x7c | 0x7e
        | 0xa2..=0xbf | 0xd7 | 0xf7
        | 0x2c2..=0x2ff | 0x300..=0x36f | 0x20d0..=0x20ff | 0x2100..=0x214f
        | 0x2190..=0x2bff | 0x3001..=0x303f | 0x1f000..=0x1faff)
}

/// (base char, had diacritic/decomposition, was uppercase).
fn fold_char(c: char) -> (char, bool, bool) {
    let upper = c.is_uppercase();
    let base = c
        .nfkd()
        .find(|b| !(0x300..0x370).contains(&(u32::from(*b))))
        .unwrap_or(c);
    let accented = base != c;
    let base = base.to_lowercase().next().unwrap_or(base);
    (base, accented, upper)
}

/// Primary collation weight per char; `None` = ignored.
fn primary_weight(c: char) -> Option<u64> {
    if is_ignorable_collation(c) {
        return None;
    }
    let cp = c as u32;
    let (base, _accented, _upper) = fold_char(c);
    Some(if is_js_whitespace(c) {
        0x0100_0000 + u64::from(cp)
    } else if is_punct(c) {
        0x0200_0000 + u64::from(cp)
    } else if is_symbol(c) {
        0x0280_0000 + u64::from(cp)
    } else if c.is_numeric() {
        0x0300_0000 + u64::from(c.to_digit(10).unwrap_or(cp))
    } else if c.is_ascii_alphabetic() || (base.is_ascii_alphabetic() && !c.is_ascii()) {
        0x0400_0000 + u64::from(base as u32)
    } else {
        0x0600_0000 + u64::from(base as u32)
    })
}

/// Simplified UCA: primary (script/letter class), secondary (accents),
/// tertiary (lowercase before uppercase).
fn collate(a: &str, b: &str) -> std::cmp::Ordering {
    let key = |s: &str| -> (Vec<u64>, Vec<u64>, Vec<u64>) {
        let mut p = Vec::new();
        let mut sec = Vec::new();
        let mut ter = Vec::new();
        for c in s.chars() {
            if let Some(w) = primary_weight(c) {
                let (_base, accented, upper) = fold_char(c);
                p.push(w);
                sec.push(u64::from(accented));
                ter.push(u64::from(upper));
            }
        }
        (p, sec, ter)
    };
    let (pa, sa, ta) = key(a);
    let (pb, sb, tb) = key(b);
    pa.cmp(&pb).then(sa.cmp(&sb)).then(ta.cmp(&tb))
}

regex!(digits_only, r"^[0-9]+$");

/// Port of `compareVersionsDesc` (`apps/analytics/worker.ts:1100`).
pub fn compare_versions_desc(a: String, b: String) -> Result<f64, String> {
    let split = |v: &str| -> Vec<(bool, f64, String)> {
        v.split(['.', '-'])
            .map(|s| {
                if digits_only().is_match(s) {
                    (true, s.parse().unwrap_or(f64::INFINITY), s.to_string())
                } else {
                    (false, 0.0, s.to_string())
                }
            })
            .collect()
    };
    let pa = split(&a);
    let pb = split(&b);
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i);
        let y = pb.get(i);
        let Some(x) = x else { return Ok(1.0) };
        let Some(y) = y else { return Ok(-1.0) };
        match (x.0, y.0) {
            (true, true) => {
                if x.1 != y.1 {
                    return Ok(y.1 - x.1);
                }
            }
            (true, false) => return Ok(-1.0),
            (false, true) => return Ok(1.0),
            (false, false) => {
                if x.2 != y.2 {
                    return Ok(match collate(&y.2, &x.2) {
                        std::cmp::Ordering::Less => -1.0,
                        std::cmp::Ordering::Equal => 0.0,
                        std::cmp::Ordering::Greater => 1.0,
                    });
                }
            }
        }
    }
    Ok(0.0)
}

/// Port of `releaseHealth` (`apps/analytics/worker.ts:1197`).
///
/// `undefined` is not `null` here: it falls through to `"unhealthy"` via NaN
/// comparisons, so the parameter stays `Value` (wire null/undefined differ).
pub fn release_health(crash_free_pct: serde_json::Value) -> Result<Option<String>, String> {
    if crash_free_pct.is_null() {
        return Ok(None);
    }
    let pct = crate::js::as_number(&crash_free_pct).unwrap_or(f64::NAN);
    Ok(Some(
        if pct >= 99.5 {
            "healthy"
        } else if pct >= 98.0 {
            "watch"
        } else {
            "unhealthy"
        }
        .to_string(),
    ))
}

/// Port of `rangeDays` (`apps/analytics/worker.ts:1271`).
pub fn range_days(since: String, now: f64) -> Result<Vec<String>, String> {
    let units: Vec<u16> = since.encode_utf16().collect();
    let start = js_date_parse(&crate::utf16::slice(&units, 0, 10));
    let today_iso = iso(now)?;
    let today = js_date_parse(&today_iso[0..10]);
    let mut labels = Vec::new();
    let mut day = start;
    while day <= today {
        labels.push(iso(day)?[0..10].to_string());
        day += DAY_MS;
    }
    Ok(labels)
}

// --- legacy purge jobs --------------------------------------------------------

/// Port of `purgeLegacyDiagnosticContent` (`apps/analytics/worker.ts:2444`).
///
/// `env.DB` is always a plain JSON value here, so `env.DB.prepare` is never a
/// function and the first call always throws — matching every recorded case.
pub async fn purge_legacy_diagnostic_content(
    env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'DB')".to_string());
    }
    if is_undefined(&env) {
        return Err("Cannot read properties of undefined (reading 'DB')".to_string());
    }
    match env.get("DB") {
        None => Err("Cannot read properties of undefined (reading 'prepare')".to_string()),
        Some(v) if v.is_null() => {
            Err("Cannot read properties of null (reading 'prepare')".to_string())
        }
        _ => Err("env.DB.prepare is not a function".to_string()),
    }
}

/// Port of `purgeLegacyDiagnosticDumps` (`apps/analytics/worker.ts:2508`).
///
/// Without a live `CRASH_DUMPS` bucket the function returns the empty counts;
/// a truthy JSON value can never supply `.list` so it always throws.
pub async fn purge_legacy_diagnostic_dumps(
    env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'CRASH_DUMPS')".to_string());
    }
    if is_undefined(&env) {
        return Err("Cannot read properties of undefined (reading 'CRASH_DUMPS')".to_string());
    }
    let dumps = env.get("CRASH_DUMPS");
    let falsy = match dumps {
        None => true,
        Some(v) => {
            v.is_null()
                || is_undefined(v)
                || matches!(v, Value::Bool(false))
                || matches!(crate::js::as_number(v), Some(n) if n == 0.0 || n.is_nan())
                || matches!(v, Value::String(s) if s.is_empty())
        }
    };
    if falsy {
        let mut out = Map::new();
        out.insert("objectsDeleted".to_string(), crate::js::number(0.0));
        out.insert("rowsDeleted".to_string(), crate::js::number(0.0));
        return Ok(Value::Object(out));
    }
    Err("env.CRASH_DUMPS.list is not a function".to_string())
}
