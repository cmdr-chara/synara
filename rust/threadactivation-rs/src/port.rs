//! The Rust port of `apps/web/src/threadActivation.logic.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::String(s) => !s.is_empty(),
        Value::Object(_) => !is_undefined(v),
        Value::Array(_) => true,
        Value::Number(_) => false,
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => is_undefined(a) && is_undefined(b),
    }
}

fn collect_leaves(root: &Value) -> Vec<Value> {
    match member(root, "kind") {
        Ok(Value::String(k)) if k == "leaf" => vec![root.clone()],
        Ok(_) => {
            let mut out = Vec::new();
            if let Ok(first) = member(root, "first") {
                out.extend(collect_leaves(&first));
            }
            if let Ok(second) = member(root, "second") {
                out.extend(collect_leaves(&second));
            }
            out
        }
        Err(_) => Vec::new(),
    }
}

fn pane_id_for_thread(split_view: &Value, thread_id: &Value) -> Value {
    if !truthy(thread_id) {
        return Value::Null;
    }
    match member(split_view, "root") {
        Ok(root) => {
            for leaf in collect_leaves(&root) {
                if member(&leaf, "threadId")
                    .map(|t| strict_eq(&t, thread_id))
                    .unwrap_or(false)
                {
                    return member(&leaf, "id").unwrap_or(Value::Null);
                }
            }
            Value::Null
        }
        Err(_) => Value::Null,
    }
}

fn kind(name: &str) -> Value {
    let mut m = Map::new();
    m.insert("kind".to_string(), Value::String(name.to_string()));
    Value::Object(m)
}

/// Port of `resolveThreadCommandActivation`
/// (`apps/web/src/threadActivation.logic.ts:22`).
pub fn resolve_thread_command_activation(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !truthy(&member(&input, "threadExists")?) {
        return Ok(kind("ignore"));
    }
    let thread_id = member(&input, "threadId")?;
    let preferred_split_view_id = member(&input, "preferredSplitViewId")?;
    let split_pane_id = member(&input, "splitPaneId")?;
    if truthy(&preferred_split_view_id) && truthy(&split_pane_id) {
        let mut m = Map::new();
        m.insert("kind".to_string(), Value::String("split".to_string()));
        m.insert("threadId".to_string(), thread_id);
        m.insert("splitViewId".to_string(), preferred_split_view_id);
        m.insert("paneId".to_string(), split_pane_id);
        return Ok(Value::Object(m));
    }
    if strict_eq(&thread_id, &member(&input, "activeSidebarThreadId")?) {
        return Ok(kind("ignore"));
    }
    let mut m = Map::new();
    m.insert("kind".to_string(), Value::String("single".to_string()));
    m.insert("threadId".to_string(), thread_id);
    Ok(Value::Object(m))
}

/// Port of `resolvePreferredSplitForCommand`
/// (`apps/web/src/threadActivation.logic.ts:58`).
pub fn resolve_preferred_split_for_command(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let active = member(&input, "activeSplitView")?;
    let thread_id = member(&input, "threadId")?;
    if truthy(&active) {
        let pane_id = pane_id_for_thread(&active, &thread_id);
        if truthy(&pane_id) {
            let mut m = Map::new();
            m.insert("splitViewId".to_string(), member(&active, "id")?);
            m.insert("paneId".to_string(), pane_id);
            return Ok(Some(Value::Object(m)));
        }
    }
    let by_id = member(&input, "splitViewsById")?;
    if is_undefined(&by_id) || by_id.is_null() {
        return Err("Cannot convert undefined or null to object".to_string());
    }
    let views: Vec<Value> = match &by_id {
        Value::Object(m) => m.values().filter(|v| !is_undefined(v)).cloned().collect(),
        Value::Array(a) => a.iter().filter(|v| !is_undefined(v)).cloned().collect(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        Value::Bool(_) | Value::Number(_) => Vec::new(),
        _ => return Err("Cannot convert undefined or null to object".to_string()),
    };
    let mut matching: Vec<(Value, Value)> = Vec::new();
    for sv in views {
        let pane_id = pane_id_for_thread(&sv, &thread_id);
        if !pane_id.is_null() {
            matching.push((sv, pane_id));
        }
    }
    let chosen = matching
        .iter()
        .find(|(sv, _)| {
            member(sv, "sourceThreadId")
                .map(|s| strict_eq(&s, &thread_id))
                .unwrap_or(false)
        })
        .or(if matching.len() == 1 {
            matching.first()
        } else {
            None
        });
    match chosen {
        Some((sv, pane_id)) => {
            let mut m = Map::new();
            m.insert("splitViewId".to_string(), member(sv, "id")?);
            m.insert("paneId".to_string(), pane_id.clone());
            Ok(Some(Value::Object(m)))
        }
        None => Ok(None),
    }
}
