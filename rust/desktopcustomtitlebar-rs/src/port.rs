//! The Rust port of `apps/desktop/src/desktopCustomTitleBar.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
        _ => js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    }
}

fn supports(platform: &Value) -> bool {
    matches!(platform, Value::String(s) if s == "win32" || s == "linux")
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
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

/// Port of `parseCustomTitleBarPreference` (`apps/desktop/src/desktopCustomTitleBar.ts:20`).
pub fn parse_custom_title_bar_preference(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if falsy(&value) || !matches!(value, Value::Object(_) | Value::Array(_)) {
        return Ok(None);
    }
    let Value::Object(o) = &value else {
        return Ok(None);
    };
    if o.get("version").and_then(js::as_number) != Some(1.0) {
        return Ok(None);
    }
    let Some(Value::Bool(enabled)) = o.get("enabled") else {
        return Ok(None);
    };
    Ok(Some(json!({ "version": 1, "enabled": enabled })))
}

/// Port of `readCustomTitleBarPreference` (`apps/desktop/src/desktopCustomTitleBar.ts:33`).
pub fn read_custom_title_bar_preference(file_path: String) -> Result<Option<bool>, String> {
    let Ok(text) = std::fs::read_to_string(&file_path) else {
        return Ok(None);
    };
    let Ok(parsed) = js::from_json::<Value>(&text) else {
        return Ok(None);
    };
    let Some(Value::Object(o)) = parse_custom_title_bar_preference(parsed)? else {
        return Ok(None);
    };
    Ok(o.get("enabled").and_then(|v| v.as_bool()))
}

/// Port of `writeCustomTitleBarPreference` (`apps/desktop/src/desktopCustomTitleBar.ts:42`).
pub fn write_custom_title_bar_preference(file_path: String, enabled: bool) -> Result<(), String> {
    let trimmed = file_path.trim_end_matches('/');
    let dir = match trimmed.rfind('/') {
        None => ".",
        Some(0) => "/",
        Some(i) => &trimmed[..i],
    };
    std::fs::create_dir_all(dir).map_err(|e| node_io_err("mkdir", &e, dir))?;
    let payload = json!({ "version": 1, "enabled": enabled });
    let text = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, format!("{text}\n")).map_err(|e| node_io_err("open", &e, &file_path))
}

/// Port of `resolveDesktopCustomTitleBarState` (`apps/desktop/src/desktopCustomTitleBar.ts:48`).
pub fn resolve_desktop_custom_title_bar_state(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() || js::is_undefined(&input) {
        let kind = if input.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "Cannot read properties of {kind} (reading 'platform')"
        ));
    }
    let platform = get(&input, "platform").cloned().unwrap_or(Value::Null);
    let supported = supports(&platform);
    let pref_v = get(&input, "preference").cloned().unwrap_or(Value::Null);
    // `preference ?? default` treats null and undefined as missing.
    let preference: Value = if supported {
        if pref_v.is_null() || js::is_undefined(&pref_v) {
            Value::Bool(true)
        } else {
            pref_v
        }
    } else {
        Value::Bool(false)
    };
    let active: Value = if supported {
        get(&input, "active").cloned().unwrap_or_else(js::undefined)
    } else {
        Value::Bool(false)
    };
    // `preference !== active`: strict compare; booleans/others differ by type anyway.
    let restart_required = supported && !strict_eq(&preference, &active);
    Ok(json!({
        "supported": supported,
        "preference": preference,
        "active": active,
        "restartRequired": restart_required,
    }))
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        (Value::String(x), Value::String(y)) => x == y,
        _ => match (js::as_number(a), js::as_number(b)) {
            (Some(x), Some(y)) => x == y,
            _ => a == b,
        },
    }
}

/// Port of `resolveDesktopTitleBarFrameOptions` (`apps/desktop/src/desktopCustomTitleBar.ts:71`).
pub fn resolve_desktop_title_bar_frame_options(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() || js::is_undefined(&input) {
        let kind = if input.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "Cannot read properties of {kind} (reading 'platform')"
        ));
    }
    let platform = get(&input, "platform").cloned().unwrap_or(Value::Null);
    let preference = get(&input, "preference").cloned().unwrap_or(Value::Null);
    // resolveCustomTitleBarActive: `preference === null` only — undefined returns undefined.
    let active = if !supports(&platform) {
        Value::Bool(false)
    } else if preference.is_null() {
        Value::Bool(true)
    } else {
        preference
    };
    if falsy(&active) {
        Ok(json!({}))
    } else {
        Ok(json!({ "frame": false }))
    }
}
