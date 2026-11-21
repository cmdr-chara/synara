//! The Rust port of `apps/desktop/src/backendNodeOptions.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

const MB: f64 = 1024.0 * 1024.0;
const DEFAULT_BACKEND_HEAP_FRACTION: f64 = 0.25;
const MIN_BACKEND_OLD_SPACE_MB: f64 = 3072.0;
const MAX_BACKEND_OLD_SPACE_MB: f64 = 8192.0;
const MIN_CONFIGURED_OLD_SPACE_MB: f64 = 1024.0;
const MAX_CONFIGURED_OLD_SPACE_MB: f64 = 32768.0;

const JS_WS: &[char] = &[
    '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
    '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
];

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c| JS_WS.contains(&c))
}

/// `/(?:^|\s)--max[-_]old[-_]space[-_]size(?:=|\s|$)/` on an already trimmed string.
fn has_old_space_flag(s: &str) -> bool {
    let bytes = s.as_bytes();
    for start in 0..bytes.len() {
        let ok_start = start == 0 || bytes[start - 1].is_ascii_whitespace();
        if !ok_start {
            continue;
        }
        let rest = &s[start..];
        if !rest.starts_with("--max") {
            continue;
        }
        let tail = &rest[5..];
        let mut i = 0;
        let mut matched = true;
        for (token, dash) in [("old", true), ("space", true), ("size", false)] {
            let sep_ok = matches!(tail.as_bytes().get(i), Some(b'-') | Some(b'_'));
            if !sep_ok || !tail[i + 1..].starts_with(token) {
                matched = false;
                break;
            }
            i += 1 + token.len();
            if dash {
                continue;
            }
            // after "size": = or whitespace or end
            let end_ok = match tail.as_bytes().get(i) {
                None => true,
                Some(b'=') => true,
                Some(c) => c.is_ascii_whitespace(),
            };
            if !end_ok {
                matched = false;
            }
            break;
        }
        if matched {
            return true;
        }
    }
    false
}

/// `Number.parseInt(s, 10)` on a trimmed string: optional sign then leading digits.
fn js_parse_int(s: &str) -> f64 {
    let t = s.trim_start();
    let (sign, digits) = match t.strip_prefix('-') {
        Some(r) => (-1.0, r),
        None => (1.0, t.strip_prefix('+').unwrap_or(t)),
    };
    let run: String = digits.chars().take_while(|c| c.is_ascii_digit()).collect();
    if run.is_empty() {
        return f64::NAN;
    }
    run.parse::<f64>().unwrap_or(f64::NAN) * sign
}

fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

/// Port of `resolveBackendMaxOldSpaceMb` (`apps/desktop/src/backendNodeOptions.ts:28`).
pub fn resolve_backend_max_old_space_mb(input: serde_json::Value) -> Result<f64, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'configuredMb')".to_string());
    }
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'configuredMb')".to_string());
    }
    let configured = member(&input, "configuredMb");
    if let Value::String(s) = &configured {
        let parsed = js_parse_int(js_trim(s));
        if parsed.is_finite() && parsed > 0.0 {
            return Ok(parsed.clamp(MIN_CONFIGURED_OLD_SPACE_MB, MAX_CONFIGURED_OLD_SPACE_MB));
        }
    }
    let total_bytes = js::as_number(&member(&input, "totalMemoryBytes")).unwrap_or(f64::NAN);
    // Math.max(0, NaN) is NaN; Math.min/max propagate NaN through the clamps.
    let total_mb = jmax(0.0, (total_bytes / MB).floor());
    let target_mb = (total_mb * DEFAULT_BACKEND_HEAP_FRACTION).floor();
    Ok(jmin(
        MAX_BACKEND_OLD_SPACE_MB,
        jmax(MIN_BACKEND_OLD_SPACE_MB, target_mb),
    ))
}

fn jmax(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn jmin(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// Port of `withBackendHeapLimitArg` (`apps/desktop/src/backendNodeOptions.ts:44`).
pub fn with_backend_heap_limit_arg(
    node_options: Option<String>,
    max_old_space_mb: f64,
) -> Result<Vec<String>, String> {
    let existing = node_options
        .map(|s| js_trim(&s).to_string())
        .unwrap_or_default();
    if has_old_space_flag(&existing) {
        return Ok(Vec::new());
    }
    Ok(vec![format!(
        "--max-old-space-size={}",
        js::number_to_string(max_old_space_mb)
    )])
}

/// Port of `resolveBackendNodeArgs` (`apps/desktop/src/backendNodeOptions.ts:55`).
pub fn resolve_backend_node_args(input: serde_json::Value) -> Result<Vec<String>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'existingNodeOptions')".to_string());
    }
    if js::is_undefined(&input) {
        return Err(
            "Cannot read properties of undefined (reading 'existingNodeOptions')".to_string(),
        );
    }
    let existing = member(&input, "existingNodeOptions");
    let node_options = match &existing {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        v if js::is_undefined(v) => None,
        _ => return Err("input.existingNodeOptions.trim is not a function".to_string()),
    };
    let mb = resolve_backend_max_old_space_mb(json!({
        "configuredMb": member(&input, "configuredMaxOldSpaceMb"),
        "totalMemoryBytes": member(&input, "totalMemoryBytes"),
    }))?;
    with_backend_heap_limit_arg(node_options, mb)
}

use serde_json::json;
