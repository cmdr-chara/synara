//! The Rust port of `apps/desktop/src/desktopProcessErrors.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `isBrokenPipeError` (`apps/desktop/src/desktopProcessErrors.ts:5`).
pub fn is_broken_pipe_error(error: serde_json::Value) -> Result<bool, String> {
    if !matches!(error, Value::Object(_) | Value::Array(_)) {
        return Ok(false);
    }
    Ok(error.get("code").and_then(Value::as_str) == Some("EPIPE"))
}

/// Port of `handleDesktopStdioError` (`apps/desktop/src/desktopProcessErrors.ts:13`).
pub fn handle_desktop_stdio_error(error: serde_json::Value) -> Result<serde_json::Value, String> {
    if !is_broken_pipe_error(error.clone())? {
        return Err("error".to_string());
    }
    Ok(crate::js::undefined())
}
