//! The Rust port of `apps/server/src/codexAppServerTransport.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

/// Port of `isFatalCodexLineError` (`apps/server/src/codexAppServerTransport.ts:41`).
pub fn is_fatal_codex_line_error(error: Value) -> Result<bool, String> {
    if error.is_null() {
        return Err("Cannot read properties of null (reading 'reason')".to_string());
    }
    if js::is_undefined(&error) {
        return Err("Cannot read properties of undefined (reading 'reason')".to_string());
    }
    Ok(error.get("reason").and_then(Value::as_str) == Some("frame-too-large"))
}
