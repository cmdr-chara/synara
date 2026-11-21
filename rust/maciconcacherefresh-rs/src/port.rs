//! The Rust port of `apps/desktop/src/macIconCacheRefresh.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn posix_normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|p| *p != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            s => parts.push(s),
        }
    }
    let joined = parts.join("/");
    match (absolute, joined.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{joined}"),
        (false, true) => ".".to_string(),
        (false, false) => joined,
    }
}

/// `path.join` for posix segments.
fn posix_join(parts: &[String]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("/");
    posix_normalize(&joined)
}

/// `path.resolve(execPath, "..", "..", "..")` — resolve right-to-left, then normalize.
fn posix_resolve3(exec_path: &str) -> String {
    // resolve applies "..","..",".." onto dirname-style resolution of execPath's dir.
    let mut resolved = exec_path.to_string();
    if !resolved.starts_with('/') {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string());
        resolved = format!("{cwd}/{resolved}");
    }
    for _ in 0..3 {
        resolved = format!("{resolved}/..");
    }
    posix_normalize(&resolved)
}

/// Port of `resolveLaunchVersionRecordPath` (`apps/desktop/src/macIconCacheRefresh.ts:14`).
pub fn resolve_launch_version_record_path(user_data_path: String) -> Result<String, String> {
    Ok(posix_join(&[
        user_data_path,
        "last-launch-version.json".to_string(),
    ]))
}

/// Port of `parseLastLaunchVersion` (`apps/desktop/src/macIconCacheRefresh.ts:19`).
pub fn parse_last_launch_version(raw_contents: Option<String>) -> Result<Value, String> {
    let raw = match raw_contents {
        None => return Ok(Value::Null),
        Some(r) => r,
    };
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Ok(Value::Null),
    };
    match parsed {
        Value::Object(o) => match o.get("version") {
            Some(Value::String(v)) => Ok(Value::String(v.clone())),
            _ => Ok(Value::Null),
        },
        _ => Ok(Value::Null),
    }
}

/// Port of `serializeLaunchVersionRecord` (`apps/desktop/src/macIconCacheRefresh.ts:33`).
pub fn serialize_launch_version_record(version: String) -> Result<String, String> {
    let mut map = serde_json::Map::new();
    map.insert("version".to_string(), Value::String(version));
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default()
    ))
}

/// Port of `shouldRefreshIconCache` (`apps/desktop/src/macIconCacheRefresh.ts:41`).
pub fn should_refresh_icon_cache(
    previous_version: Option<String>,
    current_version: String,
) -> Result<bool, String> {
    Ok(previous_version.as_deref() != Some(current_version.as_str()))
}

/// Port of `resolveMacAppBundlePath` (`apps/desktop/src/macIconCacheRefresh.ts:56`).
pub fn resolve_mac_app_bundle_path(
    exec_path: String,
    platform: serde_json::Value,
) -> Result<Option<String>, String> {
    if platform.as_str() != Some("darwin") {
        return Ok(None);
    }
    let bundle_path = posix_resolve3(&exec_path);
    Ok(if bundle_path.ends_with(".app") {
        Some(bundle_path)
    } else {
        None
    })
}
