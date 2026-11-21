//! The Rust port of `apps/web/src/splitViewRoute.ts`.

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

fn focused_thread_id(split_view: &Value) -> Result<Value, String> {
    let focused_pane_id = member(split_view, "focusedPaneId")?;
    let root = member(split_view, "root")?;
    let leaves = collect_leaves(&root);
    if let Some(leaf) = leaves.iter().find(|l| {
        member(l, "id")
            .map(|id| strict_eq(&id, &focused_pane_id))
            .unwrap_or(false)
    }) {
        let t = member(leaf, "threadId")?;
        if truthy(&t) {
            return Ok(t);
        }
    }
    for leaf in &leaves {
        let t = member(leaf, "threadId").unwrap_or_else(|_| crate::js::undefined());
        if truthy(&t) {
            return Ok(t);
        }
    }
    Ok(Value::Null)
}

/// Port of `resolveActiveSplitView` (`apps/web/src/splitViewRoute.ts:14`).
pub fn resolve_active_split_view(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err(
            "Cannot destructure property 'routeThreadId' of 'input' as it is null.".to_string(),
        );
    }
    let route_thread_id = member(&input, "routeThreadId")?;
    let split_view = member(&input, "splitView")?;
    let mut out = Map::new();
    if !truthy(&split_view) {
        out.insert("splitView".to_string(), Value::Null);
        out.insert("focusedThreadId".to_string(), route_thread_id);
        out.insert("routePaneId".to_string(), Value::Null);
        return Ok(Value::Object(out));
    }
    let focused = focused_thread_id(&split_view)?;
    let root = member(&split_view, "root")?;
    let leaves = collect_leaves(&root);
    let pane = if truthy(&route_thread_id) {
        leaves
            .iter()
            .find(|l| {
                member(l, "threadId")
                    .map(|t| strict_eq(&t, &route_thread_id))
                    .unwrap_or(false)
            })
            .and_then(|l| member(l, "id").ok())
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    out.insert("splitView".to_string(), split_view);
    out.insert("focusedThreadId".to_string(), focused);
    out.insert("routePaneId".to_string(), pane);
    Ok(Value::Object(out))
}

/// Port of `isSplitRoute` (`apps/web/src/splitViewRoute.ts:39`).
pub fn is_split_route(search: serde_json::Value) -> Result<bool, String> {
    let v = member(&search, "splitViewId")?;
    Ok(matches!(&v, Value::String(s) if !s.is_empty()))
}
