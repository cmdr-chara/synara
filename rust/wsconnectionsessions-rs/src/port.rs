//! The Rust port of `apps/server/src/wsConnectionSessions.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

/// Port of `withoutWsConnectionSessionHeaders` (`apps/server/src/wsConnectionSessions.ts:44`).
pub fn without_ws_connection_session_headers(
    serialization: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    match &serialization {
        Value::Object(m) => {
            for (k, v) in m {
                out.insert(k.clone(), v.clone());
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                out.insert(i.to_string(), v.clone());
            }
        }
        Value::String(s) => {
            for (i, ch) in s.chars().enumerate() {
                out.insert(i.to_string(), Value::String(ch.to_string()));
            }
        }
        _ => {}
    }
    out.insert(
        "makeUnsafe".to_string(),
        serde_json::json!({ "$function": "makeUnsafe" }),
    );
    Ok(Value::Object(out))
}
