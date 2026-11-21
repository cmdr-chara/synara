//! The Rust port of `apps/desktop/src/betaDiagnostics.ts`.
//!
//! The technical serializers (`sanitizeDiagnosticError`, `sanitizeDiagnosticCrash`,
//! `diagnosticErrorFingerprint`, `sanitizeDesktopUpdateFailureReason`) are re-derived
//! from `packages/shared/src/diagnosticsRedaction.ts` here, the same derivation the
//! `diagnosticsredaction-rs` crate carries; there is no shared Rust dependency yet.

#![forbid(unsafe_code)]

use regex::{Captures, Regex};
use serde_json::{Map, Value};
use std::sync::OnceLock;

macro_rules! regex {
    ($name:ident, $pat:expr) => {
        fn $name() -> &'static Regex {
            static R: OnceLock<Regex> = OnceLock::new();
            R.get_or_init(|| Regex::new($pat).expect("valid regex"))
        }
    };
}

// JS `\s` inside regexes: the JS whitespace set (incl. \u{feff}, excl. \u{0085}).
const WS: &str =
    "\\t-\\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

const BETA_DIAGNOSTICS_ENDPOINT: &str = "https://synara-beta-diagnostics.synara-orgs.workers.dev";
const BETA_DIAGNOSTICS_ENDPOINT_ENV: &str = "SYNARA_BETA_DIAGNOSTICS_URL";
const USAGE_COUNT_MAX: f64 = 100_000.0;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `v.key` with JS property-access semantics: TypeError on null/undefined,
/// `undefined` on every other primitive, the member on objects.
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

/// `typeof v === "string"` view: plain strings plus `{"$utf16": ...}` encoded ones.
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

/// JS `String(value)` coercion for the values regex `.exec` sees.
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

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `exec`-style single match with an optional trailing `\b` check.
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

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\x0c' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    })
}

fn is_finite_non_negative(v: &Value) -> bool {
    match v {
        Value::Number(n) => n.as_f64().is_some_and(|x| x.is_finite() && x >= 0.0),
        _ => false,
    }
}

/// `Math.floor` for numbers, NaN otherwise; clamps to [0, USAGE_COUNT_MAX].
fn clamp_usage_count(v: &Value) -> f64 {
    let count = match v {
        Value::Number(n) => n.as_f64().map(f64::floor).unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    if !count.is_finite() || count < 0.0 {
        0.0
    } else {
        count.min(USAGE_COUNT_MAX)
    }
}

/// Major.minor only: "15.3.1" -> "15.3", "10.0.22621" -> "10.0".
fn sanitize_os_version(v: &Value) -> Option<String> {
    let s = as_js_str(v)?;
    let caps = Regex::new(r"^(\d+)(?:\.(\d+))?")
        .expect("valid regex")
        .captures(&s)?;
    let major = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
    match caps.get(2).map(|m| m.as_str()) {
        Some(minor) => Some(format!("{major}.{minor}")),
        None => Some(major.to_string()),
    }
}

/// Primary language subtag ("en-US" -> "en"), 2-3 lowercase letters only.
fn sanitize_locale(v: &Value) -> Option<String> {
    let s = as_js_str(v)?;
    let primary = s.split('-').next().unwrap_or_default().to_lowercase();
    let re = Regex::new(r"^[a-z]{2,3}$").expect("valid regex");
    (re.is_match(&primary)).then_some(primary)
}

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

fn sanitize_desktop_update_failure_reason(reason: &Value) -> Value {
    match as_js_str(reason) {
        Some(s) if UPDATE_FAILURE_REASONS.contains(&s.as_str()) => reason.clone(),
        _ => crate::js::undefined(),
    }
}

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

regex!(
    handled_issue,
    r"^Handled issue: ([a-z0-9.-]+) \(([A-Za-z0-9._-]+)\)$"
);
regex!(duration_ms, r"^durationMs=([0-9]{1,9})(?:\n|$)");
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
regex!(
    target_version,
    r"^[0-9]+\.[0-9]+\.[0-9]+(?:-(?:beta|remote|canary)\.[0-9]+)?$"
);

/// Shared serializer (`sanitizeDiagnosticError`): returns the technical payload.
fn sanitize_error(input: &Value) -> Result<Value, String> {
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
            if let Some(d) = duration_ms().captures(&stack_for_duration) {
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
            return Ok(Value::Object(obj));
        }
    }
    // `(input.stack ?? "").slice(...)` throws when the raw value is not a string.
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
    Ok(Value::Object(obj))
}

/// `sanitizeDiagnosticCrash` from the shared redaction module.
fn sanitize_crash(input: &Value) -> Result<Value, String> {
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
            let code = match exit.get(1).map(|m| m.as_str()) {
                Some("null") => "null".to_string(),
                Some(digits) => {
                    let n: f64 = digits.parse().unwrap_or(0.0);
                    crate::js::number_to_string(n)
                }
                None => String::new(),
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

/// `diagnosticErrorFingerprint` over the sanitized payload.
fn error_fingerprint(input: &Value) -> Result<String, String> {
    let safe = sanitize_error(input)?;
    let safe_message = prop(&safe, "message")?
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let key = if safe_message.starts_with("Handled issue: ") {
        safe_message
    } else {
        let source = prop(input, "source")?
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

fn spread_obj(into: &mut Map<String, Value>, v: Value) {
    if let Value::Object(o) = v {
        for (k, x) in o {
            into.insert(k, x);
        }
    }
}

/// Port of `sanitizeBetaDiagnosticsPayload` (`apps/desktop/src/betaDiagnostics.ts:210`).
pub fn sanitize_beta_diagnostics_payload(
    payload: serde_json::Value,
    _home_dir: Option<String>,
    event: Option<String>,
) -> Result<serde_json::Value, String> {
    let kind = prop(&payload, "kind")?;
    let kind_str = kind.and_then(|v| v.as_str()).unwrap_or_default();
    if kind_str == "lifecycle" {
        let os_version = sanitize_os_version(prop(&payload, "osVersion")?.unwrap_or(&Value::Null));
        let locale = sanitize_locale(prop(&payload, "locale")?.unwrap_or(&Value::Null));
        let mut out = Map::new();
        out.insert("kind".to_string(), Value::String("lifecycle".to_string()));
        let duration = prop(&payload, "durationMs")?;
        if duration.is_some_and(is_finite_non_negative) {
            out.insert(
                "durationMs".to_string(),
                duration.cloned().unwrap_or(Value::Null),
            );
        }
        if let Some(v) = os_version {
            out.insert("osVersion".to_string(), Value::String(v));
        }
        if let Some(v) = locale {
            out.insert("locale".to_string(), Value::String(v));
        }
        return Ok(Value::Object(out));
    }
    if kind_str == "crash" {
        let technical = sanitize_crash(&payload)?;
        let mut out = Map::new();
        out.insert("kind".to_string(), Value::String("crash".to_string()));
        spread_obj(&mut out, technical);
        return Ok(Value::Object(out));
    }
    if kind_str == "error" {
        let source = prop(&payload, "source")?
            .and_then(as_js_str)
            .unwrap_or_default();
        let source = if source == "renderer" {
            "renderer"
        } else {
            "main"
        };
        let mut input = Map::new();
        input.insert("source".to_string(), Value::String(source.to_string()));
        let message = prop(&payload, "message")?;
        input.insert(
            "message".to_string(),
            Value::String(message.and_then(as_js_str).unwrap_or_default()),
        );
        let stack = prop(&payload, "stack")?;
        if let Some(s) = stack.and_then(as_js_str) {
            input.insert("stack".to_string(), Value::String(s));
        }
        let input = Value::Object(input);
        let technical = sanitize_error(&input)?;
        let mut out = Map::new();
        out.insert("kind".to_string(), Value::String("error".to_string()));
        out.insert("source".to_string(), Value::String(source.to_string()));
        spread_obj(&mut out, technical);
        out.insert(
            "fingerprint".to_string(),
            Value::String(error_fingerprint(&input)?),
        );
        return Ok(Value::Object(out));
    }
    if kind_str == "update" {
        let reason = sanitize_desktop_update_failure_reason(
            prop(&payload, "reason")?.unwrap_or(&Value::Null),
        );
        let outcome_raw = prop(&payload, "outcome")?.and_then(as_js_str);
        let outcome = if outcome_raw.as_deref() == Some("error") {
            "error"
        } else {
            "ok"
        };
        let mut out = Map::new();
        out.insert("kind".to_string(), Value::String("update".to_string()));
        out.insert("outcome".to_string(), Value::String(outcome.to_string()));
        let duration = prop(&payload, "durationMs")?;
        if duration.is_some_and(is_finite_non_negative) {
            out.insert(
                "durationMs".to_string(),
                duration.cloned().unwrap_or(Value::Null),
            );
        }
        let target = prop(&payload, "targetVersion")?;
        if let Some(t) = target.and_then(as_js_str)
            && target_version().is_match(&t)
        {
            out.insert("targetVersion".to_string(), Value::String(t));
        }
        let ctx = prop(&payload, "errorContext")?.and_then(as_js_str);
        if matches!(
            ctx.as_deref(),
            Some("check") | Some("download") | Some("install")
        ) {
            out.insert(
                "errorContext".to_string(),
                Value::String(ctx.unwrap_or_default().to_string()),
            );
        }
        if event.as_deref() == Some("update.error")
            && outcome == "error"
            && !(is_undefined(&reason) || reason.is_null())
        {
            out.insert("reason".to_string(), reason);
        }
        return Ok(Value::Object(out));
    }
    if kind_str == "usage" {
        let mut out = Map::new();
        out.insert("kind".to_string(), Value::String("usage".to_string()));
        out.insert(
            "projects".to_string(),
            crate::js::number(clamp_usage_count(
                prop(&payload, "projects")?.unwrap_or(&Value::Null),
            )),
        );
        out.insert(
            "activeThreads".to_string(),
            crate::js::number(clamp_usage_count(
                prop(&payload, "activeThreads")?.unwrap_or(&Value::Null),
            )),
        );
        return Ok(Value::Object(out));
    }
    // kind === "beta" (and every other value): fixed outcome categories only.
    let allowed: Option<&[&str]> = match event.as_deref() {
        Some("beta.installed") => Some(&["imported", "import-failed", "fresh"]),
        Some("beta.left") => Some(&["trash", "keep"]),
        _ => None,
    };
    let mut out = Map::new();
    out.insert("kind".to_string(), Value::String("beta".to_string()));
    if let Some(set) = allowed {
        let outcome = prop(&payload, "outcome")?.and_then(as_js_str);
        if outcome.is_some_and(|o| set.contains(&o.as_str())) {
            out.insert(
                "outcome".to_string(),
                prop(&payload, "outcome")?.cloned().unwrap_or(Value::Null),
            );
        }
    }
    Ok(Value::Object(out))
}

/// Port of `resolveBetaDiagnosticsEndpoint` (`apps/desktop/src/betaDiagnostics.ts:273`).
pub fn resolve_beta_diagnostics_endpoint(env: serde_json::Value) -> Result<String, String> {
    let raw = prop(&env, BETA_DIAGNOSTICS_ENDPOINT_ENV)?;
    let Some(raw) = raw else {
        return Ok(BETA_DIAGNOSTICS_ENDPOINT.to_string());
    };
    if raw.is_null() || is_undefined(raw) {
        return Ok(BETA_DIAGNOSTICS_ENDPOINT.to_string());
    }
    let trimmed = match as_js_str(raw) {
        Some(s) => js_trim(&s).to_string(),
        None => {
            return Err(format!(
                "env[{BETA_DIAGNOSTICS_ENDPOINT_ENV}].trim is not a function"
            ));
        }
    };
    if trimmed.is_empty() {
        return Ok(BETA_DIAGNOSTICS_ENDPOINT.to_string());
    }
    if let Ok(u) = url::Url::parse(&trimmed) {
        if u.scheme() == "https" {
            return Ok(trimmed);
        }
        if u.scheme() == "http" {
            let host = u.host_str().unwrap_or_default();
            if host == "127.0.0.1" || host == "localhost" || host == "[::1]" {
                return Ok(trimmed);
            }
        }
    }
    Ok(BETA_DIAGNOSTICS_ENDPOINT.to_string())
}
