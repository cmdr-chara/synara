//! The Rust port of `packages/shared/src/diagnosticsRedaction.ts`.

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

// JS regex atoms are inlined: `\s`/`\S` became the JS whitespace set (incl.
// \u{feff}, excl. \u{0085}), `\d` became `[0-9]` and `\w` `[A-Za-z0-9_]`.
const WS: &str =
    "\\t-\\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

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

/// JS `String(value)` coercion for the values regex `.exec` and templates see.
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

/// JS `ToNumber` for `>` comparisons (`maxLength` may be any value).
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

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `String.prototype.replace` with a `g` flag plus the JS boundary semantics the
/// `regex` crate lacks: `\b` at the pattern edges (adjacent chars here are
/// always word chars) becomes a predecessor/successor check, and `(?<!...)` /
/// `(?!...)` pass a wider disallowed-char predicate.
fn replace_all(
    text: &str,
    re: &Regex,
    prev_disallow: Option<fn(char) -> bool>,
    next_disallow: Option<fn(char) -> bool>,
    repl: impl Fn(&Captures) -> String,
) -> String {
    let mut out = String::with_capacity(text.len());
    let (mut last, mut pos) = (0usize, 0usize);
    while pos <= text.len() {
        let Some(caps) = re.captures_at(text, pos) else {
            break;
        };
        let m = caps.get(0).expect("group 0");
        let (s, e) = (m.start(), m.end());
        let p_ok = prev_disallow
            .map(|f| text[..s].chars().next_back().is_none_or(|c| !f(c)))
            .unwrap_or(true);
        let n_ok = next_disallow
            .map(|f| text[e..].chars().next().is_none_or(|c| !f(c)))
            .unwrap_or(true);
        if p_ok && n_ok {
            out.push_str(&text[last..s]);
            out.push_str(&repl(&caps));
            last = e;
            pos = if e > s {
                e
            } else {
                text.ceil_char_boundary(s + 1)
            };
        } else {
            pos = text.ceil_char_boundary(s + 1);
        }
    }
    out.push_str(&text[last..]);
    out
}

/// `exec`-style single match with optional trailing boundary check.
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

// Lone UTF-16 surrogates cannot live in a Rust `String`; they ride along as
// private-use stand-ins U+E000..U+E7FF so truncating mid-pair preserves the
// exact units a JS string would hold.
fn units_to_str(units: &[u16]) -> String {
    let mut s = String::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        if (0xD800..0xDC00).contains(&u)
            && i + 1 < units.len()
            && (0xDC00..0xE000).contains(&units[i + 1])
        {
            let cp = 0x10000 + (((u as u32 - 0xD800) << 10) | (units[i + 1] as u32 - 0xDC00));
            s.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
            i += 2;
        } else if (0xD800..0xE000).contains(&u) {
            s.push(char::from_u32(0xE000 + u32::from(u) - 0xD800).unwrap_or('\u{FFFD}'));
            i += 1;
        } else {
            s.push(char::from_u32(u32::from(u)).unwrap_or('\u{FFFD}'));
            i += 1;
        }
    }
    s
}

fn str_to_units(s: &str) -> Vec<u16> {
    let mut out = Vec::with_capacity(s.len());
    for ch in s.chars() {
        let cp = ch as u32;
        if (0xE000..0xE800).contains(&cp) {
            out.push((cp - 0xE000 + 0xD800) as u16);
        } else {
            let mut buf = [0u16; 2];
            out.extend_from_slice(ch.encode_utf16(&mut buf));
        }
    }
    out
}

/// Emit a JS string value: plain string when the units are valid UTF-16, the
/// `{"$utf16": ...}` marker when a lone surrogate survives.
fn emit_js_string(units: &[u16]) -> Value {
    match String::from_utf16(units) {
        Ok(s) => Value::String(s),
        Err(_) => crate::js::utf16_string(units),
    }
}

fn u16_len(s: &str) -> usize {
    str_to_units(s).len()
}

/// `s.slice(0, end)` in UTF-16 units; negative `end` counts back from the end.
fn slice_utf16(s: &str, end: f64) -> String {
    let units = str_to_units(s);
    let e = if end.is_sign_negative() && end < 0.0 {
        (units.len() as f64 + end).max(0.0)
    } else {
        end.min(units.len() as f64)
    };
    if e <= 0.0 || e.is_nan() {
        String::new()
    } else {
        units_to_str(&units[..e as usize])
    }
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

/// Port of `sanitizeDesktopUpdateFailureReason` (`packages/shared/src/diagnosticsRedaction.ts:15`).
pub fn sanitize_desktop_update_failure_reason(
    reason: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match as_js_str(&reason) {
        Some(s) if UPDATE_FAILURE_REASONS.contains(&s.as_str()) => Ok(reason),
        _ => Ok(crate::js::undefined()),
    }
}

regex!(
    stalled,
    r"^Update download stalled and could not resume \((idle-timeout|http-[45][0-9]{2}|request-error: net::ERR_[A-Z_]{1,64}|response-error|response-aborted|premature-end|request-abort); [0-9]{1,20}/(?:[0-9]{1,20}|\?) bytes after [0-9]{1,9} attempts\)\.$"
);
regex!(
    req_timeout,
    r"^Request timed out after [0-9]{1,9}ms of inactivity\.$"
);
regex!(
    dl_stalled,
    r"^Download stalled after [0-9]{1,9} seconds without progress\. Try again\.$"
);
regex!(
    err_timeout,
    r"^net::ERR_(?:TIMED_OUT|CONNECTION_TIMED_OUT)$"
);
regex!(
    err_network,
    r"^net::ERR_(?:INTERNET_DISCONNECTED|NAME_NOT_RESOLVED|NETWORK_CHANGED|CONNECTION_(?:CLOSED|RESET|REFUSED|ABORTED)|ADDRESS_UNREACHABLE|PROXY_CONNECTION_FAILED|TUNNEL_CONNECTION_FAILED|CERT_(?:AUTHORITY_INVALID|DATE_INVALID|COMMON_NAME_INVALID))$"
);
regex!(
    stalled_network,
    r"^(?:response-error|response-aborted|premature-end|request-abort)$"
);
regex!(http4_status, r#"^4[0-9]{2} [^\r\n]+\n(?:"|\{|Headers:)"#);
regex!(http5_status, r#"^5[0-9]{2} [^\r\n]+\n(?:"|\{|Headers:)"#);
regex!(http4_short, r"^Cannot download update: HTTP 4[0-9]{2}\.$");
regex!(http5_short, r"^Cannot download update: HTTP 5[0-9]{2}\.$");
regex!(http4_code, r"^http-4[0-9]{2}$");
regex!(http5_code, r"^http-5[0-9]{2}$");
regex!(
    manifest,
    r"^Cannot (?:find|parse update info from) (?:remote|beta|synara|latest)(?:-mac|-linux(?:-arm64)?)?\.yml in the latest release artifacts \("
);
regex!(
    checksum,
    r"^(?:sha256 |sha512 )?checksum mismatch(?:[ ,:(]|$)"
);
regex!(
    size_mismatch,
    r"^Update download size mismatch \([0-9]{1,20} != [0-9]{1,20} bytes\)\.$"
);
regex!(
    unsigned_version,
    r"^New version [0-9]{1,8}\.[0-9]{1,8}\.[0-9]{1,8}(?:-[0-9A-Za-z.-]{1,64})? is not signed by the application owner: "
);

/// Port of `classifyDesktopUpdateFailure` (`packages/shared/src/diagnosticsRedaction.ts:22`).
pub fn classify_desktop_update_failure(
    message: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Some(raw) = as_js_str(&message) else {
        return Ok(Value::String("unknown".to_string()));
    };
    let text = {
        let units: Vec<u16> = raw.encode_utf16().collect();
        crate::utf16::slice(&units, 0, 1024)
    };
    let text = text.as_str();
    let stalled_reason = stalled()
        .captures(text)
        .and_then(|c| c.get(1).map(|m| m.as_str()));
    let request_error = match stalled_reason.and_then(|s| s.strip_prefix("request-error: ")) {
        Some(rest) => rest,
        None => text,
    };
    if text == "Timed out while checking for updates. Try again."
        || text == "Request timed out"
        || text == "The update couldn’t be installed automatically."
        || req_timeout().is_match(text)
        || dl_stalled().is_match(text)
        || err_timeout().is_match(request_error)
        || stalled_reason == Some("idle-timeout")
    {
        return Ok(Value::String("timeout".to_string()));
    }
    if text == "cancelled" || request_error == "net::ERR_ABORTED" {
        return Ok(Value::String("cancelled".to_string()));
    }
    if err_network().is_match(request_error)
        || stalled_network().is_match(stalled_reason.unwrap_or(""))
    {
        return Ok(Value::String("network".to_string()));
    }
    if http4_status().is_match(text)
        || http4_short().is_match(text)
        || http4_code().is_match(stalled_reason.unwrap_or(""))
    {
        return Ok(Value::String("http-client".to_string()));
    }
    if http5_status().is_match(text)
        || http5_short().is_match(text)
        || http5_code().is_match(stalled_reason.unwrap_or(""))
    {
        return Ok(Value::String("http-server".to_string()));
    }
    if manifest().is_match(text) {
        return Ok(Value::String("manifest".to_string()));
    }
    if text == "No published versions on GitHub"
        || text.starts_with("Cannot parse releases feed: ")
        || text.starts_with("Unable to find latest version on GitHub (")
    {
        return Ok(Value::String("release-feed".to_string()));
    }
    if checksum().is_match(text)
        || text.starts_with("The downloaded update could not be verified: ")
        || size_mismatch().is_match(text)
        || text == "The downloaded update could not be reverified. Download it again."
        || text == "The downloaded update changed during install preparation. Download it again."
        || unsigned_version().is_match(text)
        || text == "The update download did not produce a verified update. Try again."
    {
        return Ok(Value::String("verification".to_string()));
    }
    Ok(Value::String("unknown".to_string()))
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
            return Ok((Value::Object(obj), input.clone()));
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
    Ok((Value::Object(obj), input.clone()))
}

/// Port of `sanitizeDiagnosticError` (`packages/shared/src/diagnosticsRedaction.ts:95`).
pub fn sanitize_diagnostic_error(input: serde_json::Value) -> Result<serde_json::Value, String> {
    sanitize_error(&input).map(|(v, _)| v)
}

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
    exit_reason,
    r"^code=(-?[0-9]{1,10}|null)(?: signal=(SIG[A-Z0-9]{1,12}|null))?$"
);

/// Port of `sanitizeDiagnosticCrash` (`packages/shared/src/diagnosticsRedaction.ts:192`).
pub fn sanitize_diagnostic_crash(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let process_type_raw = prop(&input, "processType")?;
    let reason_raw = prop(&input, "reason")?;
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
            let code = match exit
                .get(1)
                // INFALLIBLE: group 1 is a required alternative in exit_reason
                .expect("group 1")
                .as_str()
            {
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

/// Port of `diagnosticErrorFingerprint` (`packages/shared/src/diagnosticsRedaction.ts:221`).
pub fn diagnostic_error_fingerprint(input: serde_json::Value) -> Result<String, String> {
    let (safe, raw) = sanitize_error(&input)?;
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

/// Port of `diagnosticPayloadKind` (`packages/shared/src/diagnosticsRedaction.ts:237`).
pub fn diagnostic_payload_kind(event: String) -> Result<serde_json::Value, String> {
    let kind = match event.as_str() {
        "app.start" | "app.exit" => "lifecycle",
        "app.error" => "error",
        "app.renderer-crash" | "app.child-process-crash" => "crash",
        "update.check" | "update.available" | "update.downloaded" | "update.installed"
        | "update.error" => "update",
        "usage.daily" => "usage",
        "beta.installed" | "beta.left" => "beta",
        _ => return Ok(crate::js::undefined()),
    };
    Ok(Value::String(kind.to_string()))
}

regex!(
    pem_block,
    r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----"
);
regex!(
    local_scheme,
    r"(?i)^(?:file|synara(?:-[a-z]+)?|node|electron|devtools|chrome|chrome-extension)$"
);
regex!(http_scheme, r"(?i)^(?:https?|wss?)$");
regex!(
    sensitive_key,
    r"(?i)token|secret|password|api[_-]?key|auth|cookie|session|credential|key|sess|sid"
);
regex!(
    time_hhmmss,
    r"^(?:[01]?[0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$"
);

/// One `RULES` entry: pattern, edge-boundary emulation, replacement callback.
struct Rule {
    re: &'static Regex,
    prev_disallow: Option<fn(char) -> bool>,
    next_disallow: Option<fn(char) -> bool>,
    repl: fn(&Captures) -> String,
}

fn word(c: char) -> bool {
    is_word_char(c)
}
fn word_or_colon(c: char) -> bool {
    is_word_char(c) || c == ':'
}

fn host_safe(scheme: &str, host: &str) -> String {
    if http_scheme().is_match(scheme) {
        for prefix in ["127.0.0.1", "[::1]"] {
            if let Some(rest) = host.strip_prefix(prefix) {
                let ok = rest.is_empty()
                    || rest.strip_prefix(':').is_some_and(|p| {
                        !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit())
                    });
                if ok {
                    return format!("localhost{rest}");
                }
            }
        }
    }
    host.to_string()
}

fn rules() -> Vec<Rule> {
    let url_re = {
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| {
            Regex::new(
                &("(?i)([a-z][a-z0-9+.-]{0,31})://([^WS/?#@]*@)?([^WS/?#]+)(/[^WS?#]*)?(?:\\?[^WS#]*)?(?:#[^WS]*)?"
                    .replace("WS", WS)),
            )
            .expect("valid regex")
        })
    };
    vec![
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(&r#"ssh://git@[^WS'"]+"#.replace("WS", WS)).expect("re")
                })
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "<git-url>".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(&r#"git@[A-Za-z0-9.-]+:[^WS'"]+"#.replace("WS", WS)).expect("re")
                })
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "<git-url>".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(
                        &r#"/Users/[^/WS:'"]+|/home/[^/WS:'"]+|[A-Za-z]:\\Users\\[^\\:'"]+"#
                            .replace("WS", WS),
                    )
                    .expect("re")
                })
            },
            prev_disallow: None,
            next_disallow: None,
            repl: |_| "<user>".to_string(),
        },
        Rule {
            re: url_re,
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |caps| {
                let scheme = caps.get(1).expect("scheme").as_str();
                let userinfo = caps.get(2);
                let host = caps.get(3).expect("host").as_str();
                let path = caps.get(4).map(|m| m.as_str());
                let keep_path = local_scheme().is_match(scheme);
                let tail = match path {
                    Some(p) if p != "/" => {
                        if keep_path {
                            p.to_string()
                        } else {
                            "/…".to_string()
                        }
                    }
                    _ => String::new(),
                };
                format!(
                    "{}://{}{}{}",
                    scheme,
                    if userinfo.is_some() {
                        "<redacted>@"
                    } else {
                        ""
                    },
                    host_safe(scheme, host),
                    tail
                )
            },
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(r"[A-Za-z0-9._%+-]{1,64}@[A-Za-z0-9.-]{1,255}\.[A-Za-z]{2,63}")
                        .expect("re")
                })
            },
            prev_disallow: None,
            next_disallow: None,
            repl: |_| "<email>".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(
                        &r#"(~|<user>)[\\/](?:[^\r\n'"()\\/]+[\\/])*([^WS'"()\\/]+)"#
                            .replace("WS", WS),
                    )
                    .expect("re")
                })
            },
            prev_disallow: None,
            next_disallow: None,
            repl: |caps| {
                let basename = caps.get(2).expect("basename").as_str();
                format!("~/…/{basename}")
            },
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(&"(?i)Bearer[WS]+[^WS]+".replace("WS", WS)).expect("re")
                })
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "Bearer [redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(
                        &"(?i)Authorization[WS]*[:=][WS]*[^WS]+([WS]+[^WS]+)?".replace("WS", WS),
                    )
                    .expect("re")
                })
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "Authorization: [redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(&"(?i)(Set-Cookie|Cookie)[WS]*:[WS]*[^\r\n]*".replace("WS", WS))
                        .expect("re")
                })
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |caps| format!("{}: [redacted]", caps.get(1).expect("header").as_str()),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"sk-ant-[A-Za-z0-9_-]+").expect("re"))
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"sk-[A-Za-z0-9_-]{16,}").expect("re"))
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(r"(?:ghp|gho|ghs|ghu|ghr)_[A-Za-z0-9]{20,}").expect("re")
                })
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"github_pat_[A-Za-z0-9_]{20,}").expect("re"))
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"xox[abprs]-[A-Za-z0-9-]+").expect("re"))
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"AKIA[0-9A-Z]{16}").expect("re"))
            },
            prev_disallow: Some(word),
            next_disallow: Some(word),
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"AIza[0-9A-Za-z_-]{35}").expect("re"))
            },
            prev_disallow: Some(word),
            next_disallow: Some(word),
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").expect("re")
                })
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(
                        &r#"(["']?)([A-Za-z0-9_-]{1,64})(["']?)([WS]*[:=][WS]*)("[^"]*"|'[^']*'|[^WS,;&]+)"#
                            .replace("WS", WS),
                    )
                    .expect("re")
                })
            },
            prev_disallow: Some(word),
            next_disallow: None,
            repl: |caps| {
                let m = caps.get(0).expect("match").as_str();
                let key = caps.get(2).expect("key").as_str();
                if !sensitive_key().is_match(key) {
                    return m.to_string();
                }
                let v = caps.get(5).expect("value").as_str();
                let redacted = if v.starts_with('"') {
                    "\"[redacted]\"".to_string()
                } else if v.starts_with('\'') {
                    "'[redacted]'".to_string()
                } else {
                    "[redacted]".to_string()
                };
                format!(
                    "{}{}{}{}{}",
                    caps.get(1).expect("openQ").as_str(),
                    key,
                    caps.get(3).expect("closeQ").as_str(),
                    caps.get(4).expect("sep").as_str(),
                    redacted
                )
            },
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"(?:[0-9]{1,3}\.){3}[0-9]{1,3}").expect("re"))
            },
            prev_disallow: Some(word),
            next_disallow: Some(word),
            repl: |_| "<ip>".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| {
                    Regex::new(
                        r"(?:[0-9a-fA-F]{1,4}:){2,7}[0-9a-fA-F]{0,4}|[0-9a-fA-F:]*::[0-9a-fA-F:]*",
                    )
                    .expect("re")
                })
            },
            prev_disallow: Some(word_or_colon),
            next_disallow: Some(word_or_colon),
            repl: |caps| {
                let m = caps.get(0).expect("match").as_str();
                if time_hhmmss().is_match(m) {
                    m.to_string()
                } else {
                    "<ip>".to_string()
                }
            },
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"[A-Za-z0-9_-]{32,}").expect("re"))
            },
            prev_disallow: Some(word),
            next_disallow: Some(word),
            repl: |_| "[redacted]".to_string(),
        },
        Rule {
            re: {
                static R: OnceLock<Regex> = OnceLock::new();
                R.get_or_init(|| Regex::new(r"[A-Za-z0-9+/]{40,}={0,2}").expect("re"))
            },
            prev_disallow: None,
            next_disallow: None,
            repl: |caps| {
                let m = caps.get(0).expect("match").as_str();
                let keep = m.bytes().any(|b| b == b'+' || b == b'/')
                    && m.bytes().any(|b| b.is_ascii_digit())
                    && m.bytes().any(|b| b.is_ascii_lowercase())
                    && m.bytes().any(|b| b.is_ascii_uppercase());
                if keep {
                    "[redacted]".to_string()
                } else {
                    m.to_string()
                }
            },
        },
    ]
}

/// Port of `redactDiagnosticText` (`packages/shared/src/diagnosticsRedaction.ts:408`).
pub fn redact_diagnostic_text(
    text: String,
    opts: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let max_length = prop(&opts, "maxLength")?.map_or(f64::NAN, |v| {
        if is_undefined(v) {
            f64::NAN
        } else {
            js_to_num(v)
        }
    });
    let mut out = replace_all(&text, pem_block(), None, None, |_| {
        "[redacted private key]".to_string()
    });
    let input_truncated = (u16_len(&out) as f64) > max_length;
    if input_truncated {
        out = slice_utf16(&out, max_length);
    }
    let home_dir = match prop(&opts, "homeDir")? {
        None => None,
        Some(v) if is_undefined(v) || v.is_null() => None,
        Some(v) => match as_js_str(v) {
            Some(s) => Some(js_trim(&s).to_string()),
            None => return Err("opts.homeDir.trim is not a function".to_string()),
        },
    };
    if let Some(home) = home_dir
        && !home.is_empty()
        && home != "/"
    {
        out = out.replace(&home, "~");
    }
    for rule in rules() {
        out = replace_all(
            &out,
            rule.re,
            rule.prev_disallow,
            rule.next_disallow,
            rule.repl,
        );
    }
    if input_truncated || (u16_len(&out) as f64) > max_length {
        let end = (max_length - 1.0).max(0.0);
        let mut units = str_to_units(&slice_utf16(&out, end));
        units.push(0x2026);
        return Ok(emit_js_string(&units));
    }
    Ok(emit_js_string(&str_to_units(&out)))
}
