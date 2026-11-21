//! The Rust port of `apps/desktop/src/keepAwake.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn default_preference() -> Value {
    json!({"version": 1, "enabled": false, "remoteAccessAllowed": false})
}

fn parse_preference(value: &Value) -> Option<Value> {
    let o = value.as_object()?;
    let enabled = o.get("enabled")?.as_bool()?;
    let remote = o.get("remoteAccessAllowed")?.as_bool()?;
    if o.get("version")?.as_i64() != Some(1) {
        return None;
    }
    Some(json!({"version": 1, "enabled": enabled, "remoteAccessAllowed": remote}))
}

fn node_fs_error(e: &std::io::Error, path: &str) -> String {
    let (code, msg) = match e.raw_os_error() {
        Some(2) => ("ENOENT", "no such file or directory"),
        Some(13) => ("EACCES", "permission denied"),
        Some(1) => ("EPERM", "operation not permitted"),
        Some(21) => ("EISDIR", "illegal operation on a directory"),
        Some(20) => ("ENOTDIR", "not a directory"),
        Some(36) => ("ENAMETOOLONG", "file name too long"),
        _ => ("EIO", "i/o error"),
    };
    format!("{code}: {msg}, open '{path}'")
}

/// Port of `parseKeepAwakePreference` (`apps/desktop/src/keepAwake.ts:28`).
pub fn parse_keep_awake_preference(value: Value) -> Result<Value, String> {
    Ok(parse_preference(&value).unwrap_or(Value::Null))
}

/// Port of `readKeepAwakePreference` (`apps/desktop/src/keepAwake.ts:44`).
pub fn read_keep_awake_preference(file_path: String) -> Result<Value, String> {
    let parsed = std::fs::read_to_string(&file_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| parse_preference(&v));
    Ok(parsed.unwrap_or_else(default_preference))
}

/// Port of `writeKeepAwakePreference` (`apps/desktop/src/keepAwake.ts:54`).
pub fn write_keep_awake_preference(file_path: String, preference: Value) -> Result<(), String> {
    let dir = std::path::Path::new(&file_path)
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                std::path::Path::new(".")
            } else {
                p
            }
        })
        .unwrap_or_else(|| std::path::Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| node_fs_error(&e, &file_path))?;
    let mut body = serde_json::to_string_pretty(&preference).unwrap_or_else(|_| "null".to_string());
    body.push('\n');
    std::fs::write(&file_path, body).map_err(|e| node_fs_error(&e, &file_path))
}
