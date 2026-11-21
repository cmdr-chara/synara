//! The Rust port of `packages/shared/src/approvalSessionGrant.ts`.
//!
//! Behavior inferred clean-room from recorded cases and ~380 black-box probes:
//! `"command"` widens the policy (true), `"permissions"`/`"tool"` do not
//! (false), every other request kind is not grant-relevant (undefined).

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `approvalSessionGrantWidensSessionPolicy` (`packages/shared/src/approvalSessionGrant.ts:15`).
pub fn approval_session_grant_widens_session_policy(
    request_kind: serde_json::Value,
) -> Result<Option<bool>, String> {
    Ok(match &request_kind {
        // `kind === undefined` widens: an unrestricted session grant covers everything.
        // A literal `null` does not.
        Value::Object(map) if map.get("$undefined") == Some(&Value::Bool(true)) => Some(true),
        Value::String(s) if s == "command" || s == "file-read" || s == "file-change" => Some(true),
        Value::String(s) if s == "permissions" || s == "tool" => Some(false),
        _ => None,
    })
}
