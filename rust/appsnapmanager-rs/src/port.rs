//! The Rust port of the bridgeable helpers in `apps/desktop/src/appSnapManager.ts`:
//! `desktopAppSnapPlatform`, `parseAppSnapHelperMessage`, `isPathInsideDirectory`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

// --- JS helpers --------------------------------------------------------------

fn js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("$utf16").and_then(|u| u.as_array()).map(|a| {
            String::from_utf16_lossy(
                &a.iter()
                    .map(|x| x.as_u64().unwrap_or(0) as u16)
                    .collect::<Vec<u16>>(),
            )
        }),
        _ => None,
    }
}

/// `typeof value === "string"` under the wire codec (String or `$utf16`).
fn is_string(v: &Value) -> bool {
    crate::js::string_units(v).is_some()
}

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn is_safe_integer(v: &Value) -> Option<f64> {
    crate::js::as_number(v)
        .filter(|n| n.is_finite() && n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0)
}

fn js_strict_eq_str(v: &Value, s: &str) -> bool {
    js_str(v).as_deref() == Some(s)
}

fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn get<'v>(o: &'v Value, k: &str) -> Option<&'v Value> {
    o.get(k).filter(|v| !is_undefined(v))
}

/// `typeof value[key] === "string"` → echo the raw wire value back.
fn opt_string(o: &mut serde_json::Map<String, Value>, v: &Value, key: &str) {
    if let Some(x) = get(v, key)
        && is_string(x)
    {
        o.insert(key.to_string(), x.clone());
    }
}

/// `typeof value[key] === "string" && value[key].length > 0` → echo raw.
fn opt_nonempty_string(o: &mut serde_json::Map<String, Value>, v: &Value, key: &str) {
    if let Some(x) = get(v, key)
        && crate::js::string_units(x).is_some_and(|u| !u.is_empty())
    {
        o.insert(key.to_string(), x.clone());
    }
}

/// `typeof value[key] === "string" || value[key] === null` → echo raw.
fn opt_string_or_null(o: &mut serde_json::Map<String, Value>, v: &Value, key: &str) {
    match get(v, key) {
        Some(Value::Null) => {
            o.insert(key.to_string(), Value::Null);
        }
        Some(x) if is_string(x) => {
            o.insert(key.to_string(), x.clone());
        }
        _ => {}
    }
}

/// `value.trim().slice(0, 512)` — the slice counts UTF-16 code units.
fn normalize_optional_text(v: &Value) -> Value {
    let Some(units) = crate::js::string_units(v) else {
        return Value::Null;
    };
    let s = String::from_utf16_lossy(&units);
    let trimmed = s.trim_matches(is_js_whitespace);
    if trimmed.is_empty() {
        return Value::Null;
    }
    let u: Vec<u16> = trimmed.encode_utf16().take(512).collect();
    crate::js::utf16_string(&u)
}

/// `/^data:image\/png;base64,[A-Za-z0-9+/]+={0,2}$/.test(value)` with the
/// `value.length > 256_000` guard (UTF-16 length).
fn normalize_app_icon_data_url(v: &Value) -> Value {
    let Some(units) = crate::js::string_units(v) else {
        return Value::Null;
    };
    if units.len() > 256_000 {
        return Value::Null;
    }
    let s = String::from_utf16_lossy(&units);
    let Some(rest) = s.strip_prefix("data:image/png;base64,") else {
        return Value::Null;
    };
    let eq = rest.len() - rest.trim_end_matches('=').len();
    let core = &rest[..rest.len() - eq];
    if eq > 2
        || core.is_empty()
        || !core
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/')
    {
        return Value::Null;
    }
    crate::js::utf16_string(&units)
}

fn is_permission(v: Option<&Value>) -> bool {
    v.is_some_and(|x| js_strict_eq_str(x, "granted") || js_strict_eq_str(x, "denied"))
}

/// `typeof value[key] === "string" && value[key].length > 0` → echo raw into `out`.
fn req_nonempty<'v>(v: &'v Value, key: &str) -> Option<&'v Value> {
    get(v, key).filter(|x| crate::js::string_units(x).is_some_and(|u| !u.is_empty()))
}

// --- desktopAppSnapPlatform ---------------------------------------------------

/// Port of `desktopAppSnapPlatform` (`apps/desktop/src/appSnapManager.ts:305`).
/// Args arrive over the wire as JSON, so non-strings (numbers, null, arrays)
/// must degrade to "other" rather than erroring at the boundary.
pub fn desktop_app_snap_platform(platform: serde_json::Value) -> Result<String, String> {
    Ok(match js_str(&platform).as_deref() {
        Some("darwin") => "macos",
        Some("win32") => "windows",
        Some("linux") => "linux",
        _ => "other",
    }
    .to_string())
}

// --- parseAppSnapHelperMessage ------------------------------------------------

/// Port of `parseAppSnapHelperMessage` (`apps/desktop/src/appSnapManager.ts:312`).
pub fn parse_app_snap_helper_message(line: String) -> Result<serde_json::Value, String> {
    let parsed: Value = match serde_json::from_str(&line) {
        Ok(v) => v,
        Err(_) => return Ok(Value::Null),
    };
    let Value::Object(_) = parsed else {
        return Ok(Value::Null);
    };
    let value = parsed;

    let ty = get(&value, "type").and_then(js_str);
    match ty.as_deref() {
        Some("permissions") => {
            let mut out = serde_json::Map::new();
            out.insert("type".into(), json!("permissions"));
            for key in ["accessibility", "inputMonitoring", "screenRecording"] {
                if is_permission(get(&value, key)) {
                    out.insert(key.into(), get(&value, key).unwrap().clone());
                }
            }
            if out.len() > 1 {
                Ok(Value::Object(out))
            } else {
                Ok(Value::Null)
            }
        }
        Some("ready") => Ok(json!({"type": "ready"})),
        Some("triggered") => match req_nonempty(&value, "id") {
            Some(id) => {
                let mut out = serde_json::Map::new();
                out.insert("type".into(), json!("triggered"));
                out.insert("id".into(), id.clone());
                opt_string(&mut out, &value, "capturedAt");
                Ok(Value::Object(out))
            }
            None => Ok(Value::Null),
        },
        Some("escape") => {
            let mut out = serde_json::Map::new();
            out.insert("type".into(), json!("escape"));
            opt_string(&mut out, &value, "capturedAt");
            Ok(Value::Object(out))
        }
        Some("escape-monitor-state") => {
            if let Some(b @ Value::Bool(_)) = get(&value, "armed") {
                let mut out = serde_json::Map::new();
                out.insert("type".into(), json!("escape-monitor-state"));
                out.insert("armed".into(), b.clone());
                opt_string(&mut out, &value, "capturedAt");
                Ok(Value::Object(out))
            } else {
                Ok(Value::Null)
            }
        }
        Some("physical-input") => {
            let kind = get(&value, "kind");
            if !kind
                .is_some_and(|k| js_strict_eq_str(k, "keyboard") || js_strict_eq_str(k, "pointer"))
            {
                return Ok(Value::Null);
            }
            let event_type = get(&value, "eventType")
                .and_then(is_safe_integer)
                .filter(|n| [1.0, 3.0, 10.0, 12.0, 22.0, 25.0].contains(n));
            let mut out = serde_json::Map::new();
            out.insert("type".into(), json!("physical-input"));
            out.insert("kind".into(), kind.unwrap().clone());
            if let Some(pid) = get(&value, "pid").and_then(is_safe_integer)
                && pid > 0.0
                && pid <= 2_147_483_647.0
            {
                out.insert("pid".into(), crate::js::number(pid));
            }
            if let Some(wid) = get(&value, "windowId").and_then(is_safe_integer)
                && wid > 0.0
                && wid <= 4_294_967_295.0
            {
                out.insert("windowId".into(), crate::js::number(wid));
            }
            if let Some(et) = event_type {
                out.insert("eventType".into(), crate::js::number(et));
            }
            if let Some(spid) = get(&value, "sourcePid").and_then(is_safe_integer)
                && (0.0..=2_147_483_647.0).contains(&spid)
            {
                out.insert("sourcePid".into(), crate::js::number(spid));
            }
            if let Some(ssid) = get(&value, "sourceStateId").and_then(crate::js::as_number)
                && [-1.0, 0.0, 1.0].contains(&ssid)
            {
                out.insert("sourceStateId".into(), crate::js::number(ssid));
            }
            if let Some(b @ Value::Bool(_)) = get(&value, "sourceTagged") {
                out.insert("sourceTagged".into(), b.clone());
            }
            if event_type == Some(12.0)
                && let Some(b @ Value::Bool(_)) = get(&value, "modifierKey")
            {
                out.insert("modifierKey".into(), b.clone());
            }
            opt_string(&mut out, &value, "capturedAt");
            Ok(Value::Object(out))
        }
        Some("captured") => {
            let id = req_nonempty(&value, "id");
            let path = req_nonempty(&value, "path");
            let name = get(&value, "name").filter(|x| is_string(x));
            match (id, path, name) {
                (Some(id), Some(path), Some(name)) => {
                    let mut out = serde_json::Map::new();
                    out.insert("type".into(), json!("captured"));
                    out.insert("id".into(), id.clone());
                    out.insert("path".into(), path.clone());
                    out.insert("name".into(), name.clone());
                    opt_string(&mut out, &value, "capturedAt");
                    for k in [
                        "sourceAppName",
                        "sourceBundleIdentifier",
                        "sourceAppIconDataUrl",
                        "sourceWindowTitle",
                    ] {
                        opt_string_or_null(&mut out, &value, k);
                    }
                    Ok(Value::Object(out))
                }
                _ => Ok(Value::Null),
            }
        }
        Some("error") => match (
            req_nonempty(&value, "code"),
            req_nonempty(&value, "message"),
        ) {
            (Some(code), Some(message)) => {
                let mut out = serde_json::Map::new();
                out.insert("type".into(), json!("error"));
                out.insert("code".into(), code.clone());
                out.insert("message".into(), message.clone());
                opt_nonempty_string(&mut out, &value, "id");
                opt_string(&mut out, &value, "capturedAt");
                opt_nonempty_string(&mut out, &value, "requestId");
                Ok(Value::Object(out))
            }
            _ => Ok(Value::Null),
        },
        Some("windows") => match (get(&value, "requestId"), get(&value, "windows")) {
            (Some(rid), Some(Value::Array(items))) if is_string(rid) => {
                let mut list = Vec::new();
                for entry in items {
                    let Some(wid) = get(entry, "windowId").and_then(is_safe_integer) else {
                        continue;
                    };
                    if wid <= 0.0 || wid > 4_294_967_295.0 {
                        continue;
                    }
                    let mut e = serde_json::Map::new();
                    e.insert("windowId".into(), crate::js::number(wid));
                    e.insert(
                        "appName".into(),
                        normalize_optional_text(get(entry, "appName").unwrap_or(&Value::Null)),
                    );
                    e.insert(
                        "bundleIdentifier".into(),
                        normalize_optional_text(
                            get(entry, "bundleIdentifier").unwrap_or(&Value::Null),
                        ),
                    );
                    e.insert(
                        "windowTitle".into(),
                        normalize_optional_text(get(entry, "windowTitle").unwrap_or(&Value::Null)),
                    );
                    e.insert(
                        "appIconDataUrl".into(),
                        normalize_app_icon_data_url(
                            get(entry, "appIconDataUrl").unwrap_or(&Value::Null),
                        ),
                    );
                    list.push(Value::Object(e));
                }
                Ok(json!({"type": "windows", "requestId": rid.clone(), "windows": list}))
            }
            _ => Ok(Value::Null),
        },
        Some("permission-guide") => match get(&value, "state") {
            Some(s) if js_strict_eq_str(s, "closed") || js_strict_eq_str(s, "granted") => {
                Ok(json!({"type": "permission-guide", "state": s.clone()}))
            }
            _ => Ok(Value::Null),
        },
        _ => Ok(Value::Null),
    }
}

// --- isPathInsideDirectory ----------------------------------------------------

/// `Path.resolve` for the POSIX recorder environment: join onto cwd, then drop
/// `.`/`..` segments (`..` at root is a no-op).
fn resolve(path: &str, cwd: &str) -> Vec<String> {
    let joined = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{cwd}/{path}")
    };
    let mut out: Vec<String> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            _ => out.push(seg.to_string()),
        }
    }
    out
}

/// `Path.relative` over already-resolved POSIX segments.
fn relative(from: &[String], to: &[String]) -> String {
    let common = from
        .iter()
        .zip(to.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<&str> = Vec::new();
    parts.extend(std::iter::repeat_n("..", from.len() - common));
    parts.extend(to[common..].iter().map(String::as_str));
    parts.join("/")
}

/// Port of `isPathInsideDirectory` (`apps/desktop/src/appSnapManager.ts:480`).
pub fn is_path_inside_directory(directory: String, candidate: String) -> Result<bool, String> {
    let cwd = std::env::var("PWD").unwrap_or_else(|_| "/".to_string());
    let from = resolve(&directory, &cwd);
    let to = resolve(&candidate, &cwd);
    let rel = relative(&from, &to);
    Ok(!rel.is_empty() && !rel.starts_with("../") && rel != "..")
}
