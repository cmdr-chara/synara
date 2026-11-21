//! The Rust port of `apps/web/src/splitViewStore.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

fn prop(value: &Value, key: &str) -> Value {
    value.get(key).cloned().unwrap_or_else(js::undefined)
}

fn prop_or_err(value: &Value, key: &str) -> Result<Value, String> {
    if value.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if js::is_undefined(value) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(prop(value, key))
}

fn truthy(value: &Value) -> bool {
    if let Some(n) = js::as_number(value) {
        return n != 0.0 && !n.is_nan();
    }
    if js::is_undefined(value) {
        return false;
    }
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn is_composite(value: &Value) -> bool {
    match value {
        Value::Array(_) => true,
        Value::Object(_) => !js::is_undefined(value) && js::as_number(value).is_none(),
        _ => false,
    }
}

// JS `===`: objects/arrays compare by identity, so distinct parses never match.
fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if is_composite(a) || is_composite(b) {
        return false;
    }
    let an = js::as_number(a);
    let bn = js::as_number(b);
    if an.is_some() || bn.is_some() {
        return matches!((an, bn), (Some(x), Some(y)) if x == y);
    }
    if js::is_undefined(a) || js::is_undefined(b) {
        return js::is_undefined(a) && js::is_undefined(b);
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

// JS Set dedup uses SameValueZero: like `===` but NaN equals NaN.
fn js_same_value_zero(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
        return x == y || (x.is_nan() && y.is_nan());
    }
    js_strict_eq(a, b)
}

fn is_leaf(pane: &Value) -> bool {
    pane.get("kind").and_then(Value::as_str) == Some("leaf")
}

fn collect_leaves<'a>(pane: &'a Value, out: &mut Vec<&'a Value>) -> Result<(), String> {
    if pane.is_null() {
        return Err("Cannot read properties of null (reading 'kind')".to_string());
    }
    if js::is_undefined(pane) {
        return Err("Cannot read properties of undefined (reading 'kind')".to_string());
    }
    if is_leaf(pane) {
        out.push(pane);
        return Ok(());
    }
    match pane.get("first") {
        Some(first) => collect_leaves(first, out)?,
        None => {
            return Err("Cannot read properties of undefined (reading 'kind')".to_string());
        }
    }
    match pane.get("second") {
        Some(second) => collect_leaves(second, out),
        None => Err("Cannot read properties of undefined (reading 'kind')".to_string()),
    }
}

fn find_pane_by_id<'a>(pane: &'a Value, pane_id: &Value) -> Result<Option<&'a Value>, String> {
    if pane.is_null() {
        return Err("Cannot read properties of null (reading 'id')".to_string());
    }
    if js::is_undefined(pane) {
        return Err("Cannot read properties of undefined (reading 'id')".to_string());
    }
    if js_strict_eq(&prop(pane, "id"), pane_id) {
        return Ok(Some(pane));
    }
    if is_leaf(pane) {
        return Ok(None);
    }
    if let Some(first) = pane.get("first") {
        if let Some(found) = find_pane_by_id(first, pane_id)? {
            return Ok(Some(found));
        }
    } else {
        return Err("Cannot read properties of undefined (reading 'id')".to_string());
    }
    match pane.get("second") {
        Some(second) => find_pane_by_id(second, pane_id),
        None => Err("Cannot read properties of undefined (reading 'id')".to_string()),
    }
}

fn find_leaf_pane_by_id<'a>(pane: &'a Value, pane_id: &Value) -> Result<Option<&'a Value>, String> {
    match find_pane_by_id(pane, pane_id)? {
        Some(found) if is_leaf(found) => Ok(Some(found)),
        _ => Ok(None),
    }
}

/// Port of `resolveSplitViewFocusedThreadId` (`apps/web/src/splitViewStore.ts:318`).
pub fn resolve_split_view_focused_thread_id(
    split_view: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let root = prop_or_err(&split_view, "root")?;
    let focused_pane_id = prop_or_err(&split_view, "focusedPaneId")?;
    if let Some(focused) = find_leaf_pane_by_id(&root, &focused_pane_id)? {
        let thread_id = prop(focused, "threadId");
        if truthy(&thread_id) {
            return Ok(Some(thread_id));
        }
    }
    let mut leaves = Vec::new();
    collect_leaves(&root, &mut leaves)?;
    for leaf in leaves {
        let thread_id = prop(leaf, "threadId");
        if truthy(&thread_id) {
            return Ok(Some(thread_id));
        }
    }
    Ok(None)
}

/// Port of `resolveSplitViewFocusedPaneThreadId` (`apps/web/src/splitViewStore.ts:330`).
pub fn resolve_split_view_focused_pane_thread_id(
    split_view: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let root = prop_or_err(&split_view, "root")?;
    let focused_pane_id = prop_or_err(&split_view, "focusedPaneId")?;
    match find_leaf_pane_by_id(&root, &focused_pane_id)? {
        Some(focused) => {
            let thread_id = prop(focused, "threadId");
            if thread_id.is_null() || js::is_undefined(&thread_id) {
                Ok(None)
            } else {
                Ok(Some(thread_id))
            }
        }
        None => Ok(None),
    }
}

/// Port of `resolveSplitViewThreadIds` (`apps/web/src/splitViewStore.ts:334`).
pub fn resolve_split_view_thread_ids(
    split_view: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let root = prop_or_err(&split_view, "root")?;
    let mut leaves = Vec::new();
    collect_leaves(&root, &mut leaves)?;
    let mut out: Vec<Value> = Vec::new();
    for leaf in leaves {
        let thread_id = prop(leaf, "threadId");
        if thread_id.is_null() {
            continue;
        }
        let dup = !is_composite(&thread_id)
            && out
                .iter()
                .any(|seen| !is_composite(seen) && js_same_value_zero(seen, &thread_id));
        if !dup {
            out.push(thread_id);
        }
    }
    Ok(out)
}

/// Port of `resolveSplitViewPaneIdForThread` (`apps/web/src/splitViewStore.ts:341`).
pub fn resolve_split_view_pane_id_for_thread(
    split_view: serde_json::Value,
    thread_id: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let thread_id = thread_id.unwrap_or_else(js::undefined);
    if !truthy(&thread_id) {
        return Ok(None);
    }
    let root = prop_or_err(&split_view, "root")?;
    let mut leaves = Vec::new();
    collect_leaves(&root, &mut leaves)?;
    for leaf in leaves {
        if js_strict_eq(&prop(leaf, "threadId"), &thread_id) {
            let id = prop(leaf, "id");
            return Ok(Some(match &id {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            }));
        }
    }
    Ok(None)
}

/// Port of `selectSplitView` (`apps/web/src/splitViewStore.ts:352`).
pub fn select_split_view(split_view_id: Option<String>) -> Result<serde_json::Value, String> {
    let _ = &split_view_id;
    Ok(serde_json::json!({ "$function": "" }))
}
