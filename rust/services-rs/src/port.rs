//! The Rust port of `apps/server/src/externalMcp/Services/`.
//!
//! Derived from `cases.jsonl` plus the declared unit interface (clean-room):
//! the transition matrix `planned -> [created, failed]`, `created -> [failed]`,
//! `failed -> []`; a `from` that is not a key throws
//! "Cannot read properties of undefined (reading 'includes')".

#![forbid(unsafe_code)]

use serde_json::Value;

fn transition_targets(from: &str) -> Option<&'static [&'static str]> {
    match from {
        "planned" => Some(&["created", "failed"]),
        "created" => Some(&["failed"]),
        "failed" => Some(&[]),
        _ => None,
    }
}

/// Port of `canTransitionExternalMcpTask` (`apps/server/src/externalMcp/Services/ExternalMcpRepository.ts:64`).
pub fn can_transition_external_mcp_task(from: Value, to: Value) -> Result<bool, String> {
    // `EXTERNAL_MCP_TASK_TRANSITIONS[from]` on a non-key coerces to a lookup on a
    // key the object does not have -> undefined -> `.includes` TypeError.
    let targets = match &from {
        Value::String(s) => transition_targets(s),
        _ => None,
    };
    let Some(targets) = targets else {
        return Err("Cannot read properties of undefined (reading 'includes')".to_string());
    };
    // `Array.prototype.includes` uses SameValueZero: only string equality can hit.
    Ok(matches!(&to, Value::String(s) if targets.contains(&s.as_str())))
}
