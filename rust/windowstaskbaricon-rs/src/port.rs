//! The Rust port of `apps/desktop/src/windowsTaskbarIcon.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

static ICON_STATE: Mutex<(u64, Option<String>)> = Mutex::new((0, None));
static REFRESH_PENDING: Mutex<bool> = Mutex::new(false);

fn js_repr(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!(
            "type number ({})",
            js::number_to_string(n.as_f64().unwrap_or(0.0))
        ),
        Value::String(s) => format!("type string ('{s}')"),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".to_string()
            } else {
                "an instance of Object".to_string()
            }
        }
    }
}

/// POSIX path.join + normalize.
fn pjoin(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    let absolute = joined.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => continue,
            ".." => {
                if out.last().is_some_and(|s| *s != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let body = out.join("/");
    match (absolute, body.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{body}"),
        (false, true) => ".".to_string(),
        (false, false) => body,
    }
}

fn pbasename(p: &str) -> String {
    let trimmed = p.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(i) => trimmed[i + 1..].to_string(),
        None => trimmed.to_string(),
    }
}

fn pdirname(p: &str) -> String {
    let trimmed = p.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
        None => ".".to_string(),
    }
}

fn fold_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{ff01}'..='\u{ff5e}').contains(&c) {
                char::from_u32(c as u32 - 0xfee0).unwrap_or(c)
            } else {
                c
            }
        })
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Port of `windowsShellIconCachePath` (`apps/desktop/src/windowsTaskbarIcon.ts:34`).
pub fn windows_shell_icon_cache_path(
    cache_directory: String,
    icon_key: String,
) -> Result<String, String> {
    Ok(pjoin(&[
        &cache_directory,
        &format!("taskbar-{icon_key}.ico"),
    ]))
}

/// Port of `windowsShellIconContentKey` (`apps/desktop/src/windowsTaskbarIcon.ts:38`).
pub fn windows_shell_icon_content_key(
    icon_key: String,
    ico: serde_json::Value,
) -> Result<String, String> {
    let Some(text) = ico.as_str() else {
        return Err(format!(
            "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received {}",
            js_repr(&ico)
        ));
    };
    let digest = Sha256::digest(text.as_bytes());
    let hex: String = digest.iter().take(6).map(|b| format!("{b:02x}")).collect();
    Ok(format!("{icon_key}-{hex}"))
}

fn path_type_error(v: &Value) -> String {
    format!(
        "The \"path\" argument must be of type string. Received {}",
        js_repr(v)
    )
}

/// Port of `resolveWindowsShellIconCacheDirectory` (`apps/desktop/src/windowsTaskbarIcon.ts:43`).
pub fn resolve_windows_shell_icon_cache_directory(
    input: serde_json::Value,
) -> Result<String, String> {
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'executablePath')".to_string());
    }
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'executablePath')".to_string());
    }
    let exec = input
        .as_object()
        .and_then(|o| o.get("executablePath"))
        .cloned()
        .unwrap_or_else(js::undefined);
    let Some(exec_path) = exec.as_str() else {
        return Err(path_type_error(&exec));
    };
    let base = pbasename(exec_path);
    let folded = fold_case(&base);
    if folded == "electron" || folded == "electron.exe" {
        let fallback = input
            .as_object()
            .and_then(|o| o.get("fallbackDirectory"))
            .cloned()
            .unwrap_or(Value::Null);
        return match fallback.as_str() {
            Some(s) => Ok(s.to_string()),
            None => Err(path_type_error(&fallback)),
        };
    }
    Ok(pdirname(exec_path))
}

/// Port of `nextWindowsShellIconCacheKey` (`apps/desktop/src/windowsTaskbarIcon.ts:59`).
pub fn next_windows_shell_icon_cache_key(icon_key: String) -> Result<String, String> {
    let mut state = ICON_STATE.lock().map_err(|e| e.to_string())?;
    if state.1.as_deref() != Some(icon_key.as_str()) {
        state.0 += 1;
        state.1 = Some(icon_key.clone());
    }
    Ok(format!("{icon_key}-{}", state.0))
}

/// Port of `resetWindowsShellIconGenerationForTests` (`apps/desktop/src/windowsTaskbarIcon.ts:67`).
pub fn reset_windows_shell_icon_generation_for_tests() -> Result<(), String> {
    let mut state = ICON_STATE.lock().map_err(|e| e.to_string())?;
    *state = (0, None);
    Ok(())
}

/// Port of `isWindowsTaskbarIconRefreshPending` (`apps/desktop/src/windowsTaskbarIcon.ts:72`).
pub fn is_windows_taskbar_icon_refresh_pending() -> Result<bool, String> {
    REFRESH_PENDING
        .lock()
        .map(|p| *p)
        .map_err(|e| e.to_string())
}

/// Port of `clearWindowsTaskbarIconRefresh` (`apps/desktop/src/windowsTaskbarIcon.ts:154`).
pub fn clear_windows_taskbar_icon_refresh() -> Result<(), String> {
    let mut pending = REFRESH_PENDING.lock().map_err(|e| e.to_string())?;
    *pending = false;
    Ok(())
}

/// Port of `windowsTaskbarIconPropertyUpdates` (`apps/desktop/src/windowsTaskbarIcon.ts:160`).
pub fn windows_taskbar_icon_property_updates(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'iconPath')".to_string());
    }
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'iconPath')".to_string());
    }
    let icon_path = input
        .as_object()
        .and_then(|o| o.get("iconPath"))
        .cloned()
        .unwrap_or(Value::Null);
    let identity = input.as_object().and_then(|o| o.get("identity"));
    let ident_err = |k: &str| -> String {
        if js::is_undefined(identity.unwrap_or(&Value::Null)) || identity.is_none() {
            format!("Cannot read properties of undefined (reading '{k}')")
        } else {
            format!("Cannot read properties of null (reading '{k}')")
        }
    };
    let relaunch_command = identity
        .and_then(|i| i.as_object())
        .and_then(|o| o.get("relaunchCommand"));
    let Some(relaunch_command) = relaunch_command else {
        return Err(ident_err("relaunchCommand"));
    };
    let relaunch_display = identity
        .and_then(|i| i.as_object())
        .and_then(|o| o.get("relaunchDisplayName"));
    let Some(relaunch_display) = relaunch_display else {
        return Err(ident_err("relaunchDisplayName"));
    };
    let app_id = identity
        .and_then(|i| i.as_object())
        .and_then(|o| o.get("appId"))
        .cloned()
        .unwrap_or(Value::Null);

    let mut icon_only = Map::new();
    icon_only.insert("appIconPath".to_string(), icon_path);
    icon_only.insert("appIconIndex".to_string(), json!(0));
    icon_only.insert("relaunchCommand".to_string(), relaunch_command.clone());
    icon_only.insert("relaunchDisplayName".to_string(), relaunch_display.clone());
    let mut with_app_id = Map::new();
    with_app_id.insert("appId".to_string(), app_id);
    for (k, v) in &icon_only {
        with_app_id.insert(k.clone(), v.clone());
    }
    let mut out = Map::new();
    out.insert("iconOnly".to_string(), Value::Object(icon_only));
    out.insert("withAppId".to_string(), Value::Object(with_app_id));
    Ok(Value::Object(out))
}

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|x| x == 0.0),
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
    }
}

/// Port of `applyWindowsTaskbarIcon` (`apps/desktop/src/windowsTaskbarIcon.ts:193`).
pub fn apply_windows_taskbar_icon(input: serde_json::Value) -> Result<(), String> {
    if input.is_null() {
        return Err("Cannot destructure property 'window' of 'input' as it is null.".to_string());
    }
    if js::is_undefined(&input) {
        return Err(
            "Cannot destructure property 'window' of 'input' as it is undefined.".to_string(),
        );
    }
    let window = input.as_object().and_then(|o| o.get("window"));
    if window.is_none() || falsy(window.unwrap_or(&Value::Null)) {
        return Ok(());
    }
    Err("window.isDestroyed is not a function".to_string())
}
