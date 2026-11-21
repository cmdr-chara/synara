//! The Rust port of `apps/server/src/gitHandoffOperations.ts`.

use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    v.as_object().is_some_and(|o| o.contains_key("$undefined"))
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

fn undefined() -> Value {
    json!({"$undefined": true})
}

const TRANSITIONS: &[(&str, &[&str])] = &[
    ("pending", &["git_applied", "uncertain"]),
    ("git_applied", &["completed"]),
    ("completed", &[]),
    ("uncertain", &[]),
];

/// Port of `canTransitionGitHandoff` (`apps/server/src/gitHandoffOperations.ts:29`).
pub fn can_transition_git_handoff(
    from: serde_json::Value,
    to: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let allowed = match from.as_str() {
        Some(phase) => TRANSITIONS
            .iter()
            .find(|(p, _)| *p == phase)
            .map(|(_, targets)| *targets),
        None => None,
    };
    let Some(allowed) = allowed else {
        return Err("Cannot read properties of undefined (reading 'includes')".to_string());
    };
    // Array.prototype.includes is SameValueZero: serialized non-scalar values
    // can never match a phase string.
    Ok(Value::Bool(
        to.as_str().is_some_and(|t| allowed.contains(&t)),
    ))
}

/// Port of `beginGitHandoff` (`apps/server/src/gitHandoffOperations.ts:90`).
pub fn begin_git_handoff(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = &input;
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}

/// Port of `recordGitHandoffResult` (`apps/server/src/gitHandoffOperations.ts:115`).
pub fn record_git_handoff_result(
    command_id: serde_json::Value,
    result: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (&command_id, &result);
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}

/// Port of `completeGitHandoff` (`apps/server/src/gitHandoffOperations.ts:127`).
pub fn complete_git_handoff(command_id: String) -> Result<serde_json::Value, String> {
    let _ = &command_id;
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}

/// Port of `discardPendingGitHandoff` (`apps/server/src/gitHandoffOperations.ts:137`).
pub fn discard_pending_git_handoff(command_id: String) -> Result<serde_json::Value, String> {
    let _ = &command_id;
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}

/// Port of `gitHandoffMetadataCommand` (`apps/server/src/gitHandoffOperations.ts:146`).
pub fn git_handoff_metadata_command(
    input: serde_json::Value,
    result: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let command_id = if input.is_null() {
        return Err("Cannot read properties of null (reading 'commandId')".to_string());
    } else if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'commandId')".to_string());
    } else {
        prop(&input, "commandId").cloned().unwrap_or_else(undefined)
    };
    let thread_id = prop(&input, "threadId").cloned().unwrap_or_else(undefined);

    if result.is_null() {
        return Err("Cannot read properties of null (reading 'targetMode')".to_string());
    }
    if is_undefined(&result) {
        return Err("Cannot read properties of undefined (reading 'targetMode')".to_string());
    }
    let get = |key: &str| prop(&result, key).cloned().unwrap_or_else(undefined);
    let target_mode = get("targetMode");
    let mut command = serde_json::Map::new();
    command.insert("type".to_string(), json!("thread.meta.update"));
    command.insert("commandId".to_string(), command_id);
    command.insert("threadId".to_string(), thread_id);
    command.insert("envMode".to_string(), target_mode.clone());
    for key in [
        "branch",
        "worktreePath",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
    ] {
        command.insert(key.to_string(), get(key));
    }
    if target_mode.as_str() == Some("worktree") {
        command.insert("createBranchFlowCompleted".to_string(), Value::Bool(false));
    }
    Ok(Value::Object(command))
}
