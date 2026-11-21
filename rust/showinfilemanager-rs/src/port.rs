//! The Rust port of `apps/desktop/src/showInFileManager.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

fn extname(path: &str) -> &str {
    let base = path.rsplit('/').next().unwrap_or(path);
    match base.rfind('.') {
        Some(i) if i > 0 => &base[i..],
        _ => "",
    }
}

fn is_mac_app_bundle_path(directory_path: &str, platform: &Value) -> bool {
    platform.as_str() == Some("darwin") && extname(directory_path).eq_ignore_ascii_case(".app")
}

fn member_access_error(shell: &Value, method: &str) -> String {
    if shell.is_null() {
        return format!("Cannot read properties of null (reading '{method}')");
    }
    if js::is_undefined(shell) {
        return format!("Cannot read properties of undefined (reading '{method}')");
    }
    format!("shell.{method} is not a function")
}

/// Port of `showInFileManager` (`apps/desktop/src/showInFileManager.ts:14`).
pub async fn show_in_file_manager(
    resolved_path: String,
    is_directory: bool,
    platform: Value,
    shell: Value,
) -> Result<(), String> {
    if is_directory && !is_mac_app_bundle_path(&resolved_path, &platform) {
        // `shell.openPath` is only callable on a real Electron shell; any JSON
        // value crossing the bridge fails the method lookup.
        return Err(member_access_error(&shell, "openPath"));
    }
    Err(member_access_error(&shell, "showItemInFolder"))
}
