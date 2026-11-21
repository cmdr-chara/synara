//! The Rust port of `apps/server/src/wsOwnerAuthorization.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// Port of `requireUsageAnalyticsAccess` (`apps/server/src/wsOwnerAuthorization.ts:16`).
///
/// The source returns an Effect, which the bridge serializes by its `args` slot:
/// the owner-session generator for "mac", `undefined` for `Effect.succeed(undefined)`.
pub fn require_usage_analytics_access(
    scope: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let is_mac = matches!(&scope, Some(Value::String(s)) if s == "mac");
    Ok(if is_mac {
        json!({"~effect/Effect/args": {"$function": ""}})
    } else {
        json!({"~effect/Effect/args": {"$undefined": true}})
    })
}
