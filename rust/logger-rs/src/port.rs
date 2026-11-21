//! The Rust port of `apps/server/src/logger.ts`.

#![forbid(unsafe_code)]

use serde_json::json;

/// Port of `createLogger` (`apps/server/src/logger.ts:88`).
/// Returns the logger object; functions serialize as `{"$function": name}`.
pub fn create_logger(scope: String) -> Result<serde_json::Value, String> {
    let _ = scope;
    Ok(json!({
        "info": {"$function": "info"},
        "warn": {"$function": "warn"},
        "error": {"$function": "error"},
        "event": {"$function": "event"},
    }))
}
