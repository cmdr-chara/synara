//! The Rust port of `apps/desktop/src/desktopAppIcon.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

fn member<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
        Value::Number(_) => js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Number(_), _) | (_, Value::Number(_)) => {
            match (js::as_number(a), js::as_number(b)) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            }
        }
        // Symbols/unknown sentinels never strictly equal anything else; objects
        // and arrays compare by identity and are always different here.
        _ => false,
    }
}

/// The property key JS prints inside `Cannot read properties of undefined (reading '<key>')`.
fn prop_key(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(_) => js::number_to_string(js::as_number(v).unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(items) => items
            .iter()
            .map(|i| match i {
                Value::Null => String::new(),
                other => prop_key(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// The `Received {repr}` tail of a Node `data`/`path` type error.
fn js_repr(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(_) => format!(
            "type number ({})",
            js::number_to_string(js::as_number(v).unwrap_or(f64::NAN))
        ),
        Value::String(s) => format!("type string ('{s}')"),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(o) => {
            if js::is_undefined(&Value::Object(o.clone())) {
                "undefined".to_string()
            } else if o.contains_key("$bytes") {
                "an instance of Uint8Array".to_string()
            } else {
                "an instance of Object".to_string()
            }
        }
    }
}

/// `path.posix.dirname` for the subset of inputs Node accepts here.
fn posix_dirname(p: &str) -> String {
    let trimmed = p.trim_end_matches('/');
    match trimmed.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
    }
}

fn e_msg(err: &std::io::Error, path: &str) -> String {
    use std::io::ErrorKind;
    match err.kind() {
        ErrorKind::IsADirectory => {
            format!("EISDIR: illegal operation on a directory, open '{path}'")
        }
        ErrorKind::NotFound => format!("ENOENT: no such file or directory, open '{path}'"),
        ErrorKind::PermissionDenied => format!("EACCES: permission denied, open '{path}'"),
        _ => match err.raw_os_error() {
            Some(36) => format!("ENAMETOOLONG: name too long, open '{path}'"),
            Some(20) => format!("ENOTDIR: not a directory, open '{path}'"),
            Some(13) => format!("EACCES: permission denied, open '{path}'"),
            Some(2) => format!("ENOENT: no such file or directory, open '{path}'"),
            _ => format!("{}: open '{path}'", err),
        },
    }
}

/// Port of `writeDesktopAppIconPreference` (`apps/desktop/src/desktopAppIcon.ts:45`).
pub fn write_desktop_app_icon_preference(
    file_path: String,
    icon: serde_json::Value,
) -> Result<(), String> {
    // FS.mkdirSync(Path.dirname(filePath)) runs before FS.writeFileSync, so the
    // directory is created even when the write then fails its data type check.
    let dir = posix_dirname(&file_path);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(e_msg(&e, &file_path));
    }
    match &icon {
        Value::String(_) => {}
        Value::Object(o) if o.contains_key("$bytes") => {}
        _ => {
            return Err(format!(
                "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received {}",
                js_repr(&icon)
            ));
        }
    }
    let write = match &icon {
        Value::String(s) => std::fs::write(&file_path, s.as_bytes()),
        Value::Object(o) => {
            let bytes = o
                .get("$bytes")
                .and_then(|b| b.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|n| n.as_u64().map(|v| v as u8))
                        .collect::<Vec<u8>>()
                })
                .unwrap_or_default();
            std::fs::write(&file_path, &bytes)
        }
        _ => Ok(()),
    };
    match write {
        Ok(()) => Ok(()),
        Err(e) => Err(e_msg(&e, &file_path)),
    }
}

/// Port of `usesMacBundleAppIcon` (`apps/desktop/src/desktopAppIcon.ts:89`).
pub fn uses_mac_bundle_app_icon(input: serde_json::Value) -> Result<bool, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'platform')".to_string());
    }
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'platform')".to_string());
    }
    let platform = member(&input, "platform");
    if platform != Some(&Value::String("darwin".to_string())) {
        return Ok(false);
    }
    if member(&input, "usesLegacyDockIcon").is_some_and(|v| !falsy(v)) {
        return Ok(false);
    }
    let icon = member(&input, "icon");
    let is_default = icon == Some(&Value::String("default".to_string()));
    let is_beta = icon == Some(&Value::String("beta".to_string()));
    let beta_flavor = member(&input, "isBetaFlavor").is_some_and(|v| !falsy(v));
    Ok((is_default && !beta_flavor) || (is_beta && beta_flavor))
}

/// Port of `shouldUpdateDesktopAppIcon` (`apps/desktop/src/desktopAppIcon.ts:97`).
pub fn should_update_desktop_app_icon(
    current_icon: serde_json::Value,
    requested_icon: serde_json::Value,
) -> Result<bool, String> {
    Ok(!strict_eq(&current_icon, &requested_icon))
}

/// Port of `desktopAppIconResourceName` (`apps/desktop/src/desktopAppIcon.ts:104`).
pub fn desktop_app_icon_resource_name(input: serde_json::Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'isBetaFlavor')".to_string());
    }
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'isBetaFlavor')".to_string());
    }
    let icon = member(&input, "icon")
        .cloned()
        .unwrap_or_else(js::undefined);
    let platform = member(&input, "platform")
        .cloned()
        .unwrap_or_else(js::undefined);
    let beta = member(&input, "isBetaFlavor").is_some_and(|v| !falsy(v));
    let is_default = strict_eq(&icon, &Value::String("default".to_string()));
    let is_dark = strict_eq(&icon, &Value::String("dark".to_string()));
    let is_darwin = strict_eq(&platform, &Value::String("darwin".to_string()));
    if beta && (is_default || (is_dark && !is_darwin)) {
        if is_darwin {
            return Ok(Value::String("dock-icon.png".to_string()));
        }
        let is_linux = strict_eq(&platform, &Value::String("linux".to_string()));
        return Ok(Value::String(
            if is_linux {
                "app-icon-default-linux.png"
            } else {
                "app-icon-default-windows.ico"
            }
            .to_string(),
        ));
    }
    if is_darwin && is_default {
        let dark = member(&input, "isDarkAppearance").is_some_and(|v| !falsy(v));
        return Ok(Value::String(
            if dark {
                "dock-icon-dark.png"
            } else {
                "dock-icon.png"
            }
            .to_string(),
        ));
    }
    // APP_ICON_RESOURCE_NAMES[input.platform][input.icon]
    let table = match platform.as_str() {
        Some("darwin") => Some([
            ("default", "dock-icon.png"),
            ("icon", "app-icon-macos.png"),
            ("dark", "dock-icon-dark.png"),
            ("beta", "dock-icon-beta.png"),
        ]),
        Some("linux") => Some([
            ("default", "icon.png"),
            ("icon", "app-icon-linux.png"),
            ("dark", "icon.png"),
            ("beta", "app-icon-beta-linux.png"),
        ]),
        Some("win32") => Some([
            ("default", "icon.ico"),
            ("icon", "app-icon-windows.ico"),
            ("dark", "icon.ico"),
            ("beta", "app-icon-beta-windows.ico"),
        ]),
        _ => None,
    };
    match table {
        None => Err(format!(
            "Cannot read properties of undefined (reading '{}')",
            prop_key(&icon)
        )),
        Some(entries) => {
            for (key, name) in entries {
                if strict_eq(&icon, &Value::String(key.to_string())) {
                    return Ok(Value::String(name.to_string()));
                }
            }
            Ok(js::undefined())
        }
    }
}
