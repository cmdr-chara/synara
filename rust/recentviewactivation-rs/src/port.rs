//! The Rust port of `apps/web/src/recentViewActivation.logic.ts`.

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

fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if e.is_null() || is_undefined(e) {
                    String::new()
                } else {
                    js_str(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) if is_undefined(v) => "undefined".to_string(),
        Value::Object(m) => crate::js::as_number(v)
            .map(crate::js::number_to_string)
            .unwrap_or_else(|| {
                let _ = m;
                "[object Object]".to_string()
            }),
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

/// Port of `resolveRecentThreadSplitActivation`
/// (`apps/web/src/recentViewActivation.logic.ts:19`).
pub fn resolve_recent_thread_split_activation(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let view = member(&input, "view")?;
    let kind = member(&view, "kind")?;
    if kind != Value::String("thread".to_string()) {
        return Ok(None);
    }
    let split_view_id = member(&view, "splitViewId")?;
    if !truthy(&split_view_id) {
        return Ok(None);
    }
    let by_id = member(&input, "splitViewsById")?;
    let key = js_str(&split_view_id);
    let split_view = match &by_id {
        Value::Null => return Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(&by_id) => {
            return Err(format!(
                "Cannot read properties of undefined (reading '{key}')"
            ));
        }
        Value::Object(m) => m.get(&key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    };
    if !truthy(&split_view) {
        return Ok(None);
    }
    let thread_id = member(&view, "threadId")?;
    if !truthy(&thread_id) {
        return Ok(None);
    }
    let root = member(&split_view, "root")?;
    for leaf in collect_leaves(&root) {
        let leaf_thread = member(&leaf, "threadId").unwrap_or_else(|_| crate::js::undefined());
        if strict_eq(&leaf_thread, &thread_id) {
            let mut out = Map::new();
            out.insert("splitViewId".to_string(), split_view_id.clone());
            out.insert(
                "paneId".to_string(),
                member(&leaf, "id").unwrap_or_else(|_| crate::js::undefined()),
            );
            return Ok(Some(Value::Object(out)));
        }
    }
    Ok(None)
}
