//! The Rust port of `apps/web/src/threadActivation.logic.ts`.
//!
//! `Err` mirrors a thrown JS error (property reads on null/undefined).

#![forbid(unsafe_code)]

use serde_json::Value;

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key)
}

fn is_undef(v: Option<&Value>) -> bool {
    v.is_none() || matches!(v, Some(Value::Object(m)) if m.get("$undefined").is_some())
}

/// JS truthiness for a possibly-missing property.
fn truthy_opt(v: Option<&Value>) -> bool {
    match v {
        None => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(true),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Object(m)) if m.get("$undefined").is_some() => false,
        Some(_) => true,
    }
}

/// Depth-first leaf collection over a pane tree (`collectLeaves`).
fn collect_leaves<'a>(pane: &'a Value, out: &mut Vec<&'a Value>) {
    if get(pane, "kind") == Some(&Value::String("leaf".to_string())) {
        out.push(pane);
        return;
    }
    if let Some(first) = get(pane, "first") {
        collect_leaves(first, out);
    }
    if let Some(second) = get(pane, "second") {
        collect_leaves(second, out);
    }
}

/// `resolveSplitViewPaneIdForThread`: first leaf whose threadId matches.
fn pane_id_for_thread<'a>(split_view: &'a Value, thread_id: &Value) -> Option<&'a Value> {
    let mut leaves = Vec::new();
    if let Some(root) = get(split_view, "root") {
        collect_leaves(root, &mut leaves);
    }
    leaves
        .into_iter()
        .find(|leaf| get(leaf, "threadId") == Some(thread_id))
        .and_then(|leaf| get(leaf, "id"))
}

/// Port of `resolveThreadCommandActivation` (`apps/web/src/threadActivation.logic.ts:24`).
pub fn resolve_thread_command_activation(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'threadExists')".to_string());
    }
    let thread_exists = get(&input, "threadExists");
    if !truthy_opt(thread_exists) {
        return Ok(serde_json::json!({ "kind": "ignore" }));
    }

    let preferred = get(&input, "preferredSplitViewId");
    let pane = get(&input, "splitPaneId");
    if let (Some(split_id), Some(pane_id)) = (preferred, pane)
        && truthy_opt(Some(split_id))
        && truthy_opt(Some(pane_id))
    {
        return Ok(serde_json::json!({
            "kind": "split",
            "threadId": input["threadId"],
            "splitViewId": split_id,
            "paneId": pane_id,
        }));
    }

    if get(&input, "threadId") == get(&input, "activeSidebarThreadId") {
        return Ok(serde_json::json!({ "kind": "ignore" }));
    }
    Ok(serde_json::json!({ "kind": "single", "threadId": input["threadId"] }))
}

/// Port of `resolvePreferredSplitForCommand` (`apps/web/src/threadActivation.logic.ts:58`).
pub fn resolve_preferred_split_for_command(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'activeSplitView')".to_string());
    }
    let thread_id = get(&input, "threadId").cloned().unwrap_or(Value::Null);
    if let Some(active) = get(&input, "activeSplitView")
        && !active.is_null()
        && !is_undef(Some(active))
        && let Some(pane_id) = pane_id_for_thread(active, &thread_id)
    {
        let split_id = get(active, "id").cloned().unwrap_or(Value::Null);
        return Ok(Some(serde_json::json!({
            "splitViewId": split_id,
            "paneId": pane_id,
        })));
    }

    let by_id = match get(&input, "splitViewsById") {
        Some(Value::Object(m)) => m,
        _ => {
            return Err("Cannot convert undefined or null to object".to_string());
        }
    };

    let mut matching: Vec<(&Value, &Value)> = Vec::new();
    for (_k, sv) in by_id {
        if sv.is_null() {
            // null is not filtered out by `!== undefined` — it crashes on .root read.
            return Err("Cannot read properties of null (reading 'root')".to_string());
        }
        if is_undef(Some(sv)) {
            continue;
        }
        if let Some(pane_id) = pane_id_for_thread(sv, &thread_id) {
            matching.push((sv, pane_id));
        }
    }

    let source = matching
        .iter()
        .find(|(sv, _)| get(sv, "sourceThreadId") == Some(&thread_id));
    let chosen: Option<(&Value, &Value)> = match source {
        Some(m) => Some(*m),
        None if matching.len() == 1 => Some(matching[0]),
        _ => None,
    };
    Ok(chosen.map(|(sv, pane_id)| {
        serde_json::json!({
            "splitViewId": get(sv, "id").cloned().unwrap_or(Value::Null),
            "paneId": pane_id,
        })
    }))
}
