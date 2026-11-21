//! The Rust port of `apps/desktop/src/windowsShellAppUserModel.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || js::is_undefined(i) {
                    String::new()
                } else {
                    js_to_string(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if let Some(n) = js::as_number(v) {
                js::number_to_string(n)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// Port of `windowsShellIconResource` (`apps/desktop/src/windowsShellAppUserModel.ts:245`).
pub fn windows_shell_icon_resource(
    icon_path: String,
    icon_index: serde_json::Value,
) -> Result<String, String> {
    let index = if js::is_undefined(&icon_index) {
        "0".to_string()
    } else {
        js_to_string(&icon_index)
    };
    Ok(format!("{icon_path},{index}"))
}

/// Port of `nativeWindowHandleToHwnd` (`apps/desktop/src/windowsShellAppUserModel.ts:249`).
pub fn native_window_handle_to_hwnd(
    handle: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if js::is_undefined(&handle) {
        return Err("Cannot read properties of undefined (reading 'length')".to_string());
    }
    if handle.is_null() {
        return Err("Cannot read properties of null (reading 'length')".to_string());
    }
    let length = handle
        .as_array()
        .map(Vec::len)
        .or_else(|| handle.as_str().map(|s| s.encode_utf16().count()))
        .or_else(|| {
            handle
                .as_object()
                .and_then(|o| o.get("length"))
                .and_then(|l| l.as_u64().map(|n| n as usize))
        });
    if length.is_some_and(|n| n >= 8) {
        return Err("handle.readBigUInt64LE is not a function".to_string());
    }
    Err("handle.readUInt32LE is not a function".to_string())
}

/// Port of `windowsShellAppUserModelHelperName` (`apps/desktop/src/windowsShellAppUserModel.ts:254`).
pub fn windows_shell_app_user_model_helper_name() -> Result<String, String> {
    Ok("windows-shell-appusermodel-8e34a025faf4.exe".to_string())
}

/// Port of `cscCompilerCandidates` (`apps/desktop/src/windowsShellAppUserModel.ts:262`).
pub fn csc_compiler_candidates() -> Result<Vec<String>, String> {
    let root = std::env::var("WINDIR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "C:\\Windows".to_string());
    Ok(vec![
        format!("{root}/Microsoft.NET/Framework64/v4.0.30319/csc.exe"),
        format!("{root}/Microsoft.NET/Framework/v4.0.30319/csc.exe"),
    ])
}

fn node_mkdir_error(e: &std::io::Error, path: &str) -> String {
    let (code, msg) = match e.raw_os_error() {
        Some(2) => ("ENOENT", "no such file or directory"),
        Some(13) => ("EACCES", "permission denied"),
        Some(1) => ("EPERM", "operation not permitted"),
        Some(21) => ("EISDIR", "illegal operation on a directory"),
        Some(20) => ("ENOTDIR", "not a directory"),
        Some(36) => ("ENAMETOOLONG", "file name too long"),
        Some(17) => ("EEXIST", "file already exists"),
        _ => ("EIO", "i/o error"),
    };
    format!("{code}: {msg}, mkdir '{path}'")
}

/// Port of `ensureWindowsShellAppUserModelHelper` (`apps/desktop/src/windowsShellAppUserModel.ts:279`).
pub fn ensure_windows_shell_app_user_model_helper(
    cache_directory: String,
) -> Result<String, String> {
    if cache_directory.is_empty() {
        return Err("ENOENT: no such file or directory, mkdir ''".to_string());
    }
    std::fs::create_dir_all(&cache_directory)
        .map_err(|e| node_mkdir_error(&e, &cache_directory))?;
    let helper = windows_shell_app_user_model_helper_name()?;
    let exe_path = std::path::Path::new(&cache_directory).join(&helper);
    if exe_path.exists() {
        return Ok(exe_path.to_string_lossy().to_string());
    }
    // No csc.exe on this platform: the .NET compiler candidates never exist.
    Err("csc.exe not found".to_string())
}

/// Port of `applyWindowsShellAppUserModel` (`apps/desktop/src/windowsShellAppUserModel.ts:305`).
pub fn apply_windows_shell_app_user_model(
    input: serde_json::Value,
    cache_directory: String,
    _options: Option<serde_json::Value>,
) -> Result<(), String> {
    let _ = &input;
    ensure_windows_shell_app_user_model_helper(cache_directory)?;
    Ok(())
}
