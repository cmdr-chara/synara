//! The Rust port of `packages/shared/src/worktreeHandoff.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn undefined() -> Value {
    json!({ "$undefined": true })
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        _ => !crate::js::is_undefined(v),
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn get<'a>(input: &'a Value, key: &str) -> &'a Value {
    input.get(key).unwrap_or(&Value::Null)
}

fn get_or_undefined(input: &Value, key: &str) -> Value {
    input.get(key).cloned().unwrap_or_else(undefined)
}

fn err(input: &Value, key: &str) -> Result<(), String> {
    if input.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if crate::js::is_undefined(input) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(())
}

/// `input.a ?? input.b ?? input.c` then JS truthiness of the picked value.
fn pick_associated(input: &Value) -> &Value {
    for k in [
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
    ] {
        let v = get(input, k);
        if !is_nullish(v) {
            return v;
        }
    }
    &Value::Null
}

/// Port of `resolveWorktreeHandoffWorkspaceMetadata` (`packages/shared/src/worktreeHandoff.ts:25`).
pub fn resolve_worktree_handoff_workspace_metadata(input: Value) -> Result<Value, String> {
    err(&input, "targetMode")?;
    let is_worktree = input.get("targetMode") == Some(&Value::String("worktree".into()));
    let mut out = json!({
        "envMode": get_or_undefined(&input, "targetMode"),
        "branch": get_or_undefined(&input, "branch"),
        "worktreePath": get_or_undefined(&input, "worktreePath"),
        "associatedWorktreePath": get_or_undefined(&input, "associatedWorktreePath"),
        "associatedWorktreeBranch": get_or_undefined(&input, "associatedWorktreeBranch"),
        "associatedWorktreeRef": get_or_undefined(&input, "associatedWorktreeRef"),
    });
    if is_worktree {
        out["createBranchFlowCompleted"] = json!(false);
    }
    Ok(out)
}

/// Port of `hasAssociatedWorktree` (`packages/shared/src/worktreeHandoff.ts:44`).
pub fn has_associated_worktree(input: Value) -> Result<bool, String> {
    err(&input, "associatedWorktreePath")?;
    Ok(truthy(pick_associated(&input)))
}

/// Port of `resolveWorktreeHandoffIntent` (`packages/shared/src/worktreeHandoff.ts:54`).
pub fn resolve_worktree_handoff_intent(input: Value) -> Result<Option<Value>, String> {
    err(&input, "preferredNewWorktreeName")?;
    let name = get(&input, "preferredNewWorktreeName");
    if !is_nullish(name) {
        let Some(raw) = name.as_str() else {
            return Err("input.preferredNewWorktreeName?.trim is not a function".to_string());
        };
        let trimmed = js_trim(raw);
        if !trimmed.is_empty() {
            return Ok(Some(json!({
                "kind": "create-new",
                "worktreeName": trimmed,
                "baseBranch": base_branch(&input),
            })));
        }
    }
    if truthy(pick_associated(&input)) {
        return Ok(Some(json!({
            "kind": "reuse-associated",
            "associatedWorktreePath": get(&input, "associatedWorktreePath"),
            "associatedWorktreeBranch": get(&input, "associatedWorktreeBranch"),
            "associatedWorktreeRef": get(&input, "associatedWorktreeRef"),
            "baseBranch": base_branch(&input),
        })));
    }
    Ok(None)
}

fn base_branch(input: &Value) -> Value {
    let preferred = get(input, "preferredWorktreeBaseBranch");
    if !is_nullish(preferred) {
        return preferred.clone();
    }
    let current = get(input, "currentBranch");
    if !is_nullish(current) {
        return current.clone();
    }
    Value::Null
}
