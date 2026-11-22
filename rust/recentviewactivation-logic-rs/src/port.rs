//! The Rust port of `apps/web/src/recentViewActivation.logic.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `v.key` — `None` mirrors a missing member; errors carry the member name.
fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undef(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(v.as_object().and_then(|o| o.get(key)))
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    Ok(prop(v, key)?.cloned().unwrap_or_else(crate::js::undefined))
}

/// `a === b`: strict equality; distinct structured objects never equal.
fn strict_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (crate::js::string_units(a), crate::js::string_units(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Object(_), Value::Object(_)) => is_undef(a) && is_undef(b),
        _ => false,
    }
}

/// JS truthiness for `!value`.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undef(v) {
                return false;
            }
            if let Some(n) = crate::js::as_number(v) {
                return n != 0.0;
            }
            if let Some(units) = crate::js::string_units(v) {
                return !units.is_empty();
            }
            true
        }
    }
}

/// `collectLeaves` from `./splitViewStore` — the same DFS over first/second.
fn leaves_of(root: &Value, out: &mut Vec<Value>) -> Result<(), String> {
    let kind = member(root, "kind")?;
    if kind.as_str() == Some("leaf") {
        out.push(root.clone());
        return Ok(());
    }
    leaves_of(&member(root, "first")?, out)?;
    leaves_of(&member(root, "second")?, out)
}

/// `resolveSplitViewPaneIdForThread` from `./splitViewStore`.
fn resolve_pane_id(split_view: &Value, thread_id: &Value) -> Result<Value, String> {
    if !truthy(thread_id) {
        return Ok(Value::Null);
    }
    let mut leaves = Vec::new();
    leaves_of(&member(split_view, "root")?, &mut leaves)?;
    for leaf in &leaves {
        if strict_eq(&member(leaf, "threadId")?, thread_id) {
            return member(leaf, "id");
        }
    }
    Ok(Value::Null)
}

/// `obj[key]` for a string key: member semantics on objects, `undefined`
/// elsewhere, with the null/undefined read errors.
fn index<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    prop(v, key)
}

/// Port of `resolveRecentThreadSplitActivation` (`apps/web/src/recentViewActivation.logic.ts:19`).
pub fn resolve_recent_thread_split_activation(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let view = member(&input, "view")?;
    let kind = member(&view, "kind")?;
    if kind.as_str() != Some("thread") {
        return Ok(None);
    }
    let split_view_id = member(&view, "splitViewId")?;
    if !truthy(&split_view_id) {
        return Ok(None);
    }
    let by_id = member(&input, "splitViewsById")?;
    // `byId[input.view.splitViewId]` — the key is the raw value's string form.
    let key = match &split_view_id {
        Value::String(s) => s.clone(),
        _ => String::new(),
    };
    let split_view = index(&by_id, &key)?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    if !truthy(&split_view) {
        return Ok(None);
    }
    let thread_id = member(&view, "threadId")?;
    let pane_id = resolve_pane_id(&split_view, &thread_id)?;
    if !truthy(&pane_id) {
        return Ok(None);
    }
    Ok(Some(
        json!({ "splitViewId": split_view_id, "paneId": pane_id }),
    ))
}
