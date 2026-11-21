//! The Rust port of `packages/shared/src/todo.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn undefined() -> Value {
    serde_json::json!({ "$undefined": true })
}

fn nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !is_undefined(v),
    }
}

/// JS object spread of `src` into `out`: plain objects copy fields, arrays copy
/// index keys, strings copy per-character index keys, other values copy nothing.
fn spread(out: &mut Map<String, Value>, src: &Value) {
    match src {
        Value::Object(o) => {
            for (k, v) in o {
                out.insert(k.clone(), v.clone());
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                out.insert(i.to_string(), v.clone());
            }
        }
        Value::String(s) => {
            for (i, c) in s.chars().enumerate() {
                out.insert(i.to_string(), Value::String(c.to_string()));
            }
        }
        _ => {}
    }
}

/// Port of `applyTodoPatch` (`packages/shared/src/todo.ts:12`).
pub fn apply_todo_patch(
    todo: serde_json::Value,
    patch: serde_json::Value,
    updated_at: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if todo.is_null() {
        return Err("Cannot read properties of null (reading 'completedAt')".to_string());
    }
    if is_undefined(&todo) {
        return Err("Cannot read properties of undefined (reading 'completedAt')".to_string());
    }
    let todo_completed_at = todo.get("completedAt").cloned().unwrap_or_else(undefined);
    if patch.is_null() {
        return Err("Cannot read properties of null (reading 'completed')".to_string());
    }
    if is_undefined(&patch) {
        return Err("Cannot read properties of undefined (reading 'completed')".to_string());
    }
    let completed = patch.get("completed").cloned().unwrap_or_else(undefined);

    let mut out = Map::new();
    spread(&mut out, &todo);

    for key in ["title", "notes", "priority", "projectId", "dueDate"] {
        if let Some(v) = patch.get(key).filter(|v| !is_undefined(v)) {
            out.insert(key.to_string(), v.clone());
        }
    }

    if let Some(thread_id) = patch.get("threadId").filter(|v| !is_undefined(v)) {
        out.insert("threadId".to_string(), thread_id.clone());
        out.insert(
            "linkedAt".to_string(),
            if thread_id.is_null() {
                Value::Null
            } else {
                updated_at.clone()
            },
        );
        out.insert(
            "delegationBaseTurnId".to_string(),
            patch
                .get("delegationBaseTurnId")
                .filter(|v| !nullish(v))
                .cloned()
                .unwrap_or(Value::Null),
        );
    } else if let Some(v) = patch
        .get("delegationBaseTurnId")
        .filter(|v| !is_undefined(v))
    {
        out.insert("delegationBaseTurnId".to_string(), v.clone());
    }

    out.insert(
        "completedAt".to_string(),
        if is_undefined(&completed) {
            todo_completed_at
        } else if truthy(&completed) {
            if nullish(&todo_completed_at) {
                updated_at.clone()
            } else {
                todo_completed_at
            }
        } else {
            Value::Null
        },
    );
    out.insert("updatedAt".to_string(), updated_at);
    Ok(Value::Object(out))
}
