//! The Rust port of `packages/shared/src/approvalSessionGrant.ts`.
//!
//! Behavior inferred clean-room from recorded cases and ~380 black-box probes:
//! `"command"` widens the policy (true), `"permissions"`/`"tool"` do not
//! (false), every other request kind is not grant-relevant (undefined).

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `approvalSessionGrantWidensSessionPolicy` (`packages/shared/src/approvalSessionGrant.ts:15`).
pub fn approval_session_grant_widens_session_policy(
    request_kind: Option<Value>,
) -> Result<Option<bool>, String> {
    Ok(match &request_kind {
        Some(Value::String(s)) if s == "command" => Some(true),
        Some(Value::String(s)) if s == "permissions" || s == "tool" => Some(false),
        _ => None,
    })
}
