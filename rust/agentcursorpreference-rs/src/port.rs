//! The Rust port of `apps/desktop/src/agentCursorPreference.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

use crate::js;

const JS_WS: &[char] = &[
    '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
    '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
];

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c| JS_WS.contains(&c))
}

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
        _ => js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    }
}

fn normalize_channel(value: Option<&Value>) -> Option<String> {
    let Value::String(s) = value? else {
        return None;
    };
    let candidate = js_trim(s).to_lowercase();
    let b = candidate.as_bytes();
    if b.len() == 7
        && b[0] == b'#'
        && b[1..]
            .iter()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
    {
        Some(candidate)
    } else {
        None
    }
}

fn normalize_style(value: &Value) -> Option<Value> {
    if falsy(value) || !matches!(value, Value::Object(_)) || js::is_undefined(value) {
        return None;
    }
    let candidate = value.as_object()?;
    let fill = normalize_channel(candidate.get("fill"));
    let rim = normalize_channel(candidate.get("rim"));
    let shadow = normalize_channel(candidate.get("shadow"));
    if fill.is_none() && rim.is_none() && shadow.is_none() {
        return None;
    }
    let mut out = Map::new();
    if let Some(v) = fill {
        out.insert("fill".to_string(), json!(v));
    }
    if let Some(v) = rim {
        out.insert("rim".to_string(), json!(v));
    }
    if let Some(v) = shadow {
        out.insert("shadow".to_string(), json!(v));
    }
    Some(Value::Object(out))
}

/// Port of `normalizeAgentCursorStylePreference` (`apps/desktop/src/agentCursorPreference.ts:37`).
pub fn normalize_agent_cursor_style_preference(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    Ok(normalize_style(&value))
}

/// Port of `parseAgentCursorPreference` (`apps/desktop/src/agentCursorPreference.ts:53`).
pub fn parse_agent_cursor_preference(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if falsy(&value) || !matches!(value, Value::Object(_) | Value::Array(_)) {
        return Ok(None);
    }
    let Value::Object(candidate) = &value else {
        // Arrays: `version !== 1` already fails.
        return Ok(None);
    };
    if candidate.get("version").and_then(js::as_number) != Some(1.0) {
        return Ok(None);
    }
    if !candidate.contains_key("style") {
        return Ok(None);
    }
    let style = normalize_style(candidate.get("style").unwrap_or(&Value::Null));
    Ok(Some(
        json!({ "version": 1, "style": style.unwrap_or(Value::Null) }),
    ))
}

/// Port of `readAgentCursorPreference` (`apps/desktop/src/agentCursorPreference.ts:61`).
pub fn read_agent_cursor_preference(
    file_path: String,
) -> Result<Option<serde_json::Value>, String> {
    let Ok(text) = std::fs::read_to_string(&file_path) else {
        return Ok(None);
    };
    let Ok(parsed) = js::from_json::<Value>(&text) else {
        return Ok(None);
    };
    let Some(Value::Object(o)) = parse_agent_cursor_preference(parsed)? else {
        return Ok(None);
    };
    match o.get("style") {
        Some(v) if !v.is_null() => Ok(Some(v.clone())),
        _ => Ok(None),
    }
}

fn node_io_err(op: &str, e: &std::io::Error, path: &str) -> String {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT: no such file or directory",
        std::io::ErrorKind::PermissionDenied => "EACCES: permission denied",
        _ => match e.raw_os_error() {
            Some(21) => "EISDIR: illegal operation on a directory",
            Some(20) => "ENOTDIR: not a directory",
            _ => return e.to_string(),
        },
    };
    format!("{code}, {op} '{path}'")
}

/// Port of `writeAgentCursorPreference` (`apps/desktop/src/agentCursorPreference.ts:74`).
pub fn write_agent_cursor_preference(
    file_path: String,
    style: Option<serde_json::Value>,
) -> Result<(), String> {
    let normalized = style.as_ref().and_then(normalize_style);
    let Some(style) = normalized else {
        match std::fs::remove_file(&file_path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(node_io_err("rm", &e, &file_path)),
        }
        return Ok(());
    };
    let trimmed = file_path.trim_end_matches('/');
    let dir = match trimmed.rfind('/') {
        None => ".",
        Some(0) => "/",
        Some(i) => &trimmed[..i],
    };
    std::fs::create_dir_all(dir).map_err(|e| node_io_err("mkdir", &e, dir))?;
    let payload = json!({ "version": 1, "style": style });
    let text = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, format!("{text}\n")).map_err(|e| node_io_err("open", &e, &file_path))
}
