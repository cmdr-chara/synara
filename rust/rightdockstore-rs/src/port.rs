//! The Rust port of `apps/web/src/rightDockStore.logic.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

const PANE_KINDS: [&str; 11] = [
    "browser",
    "device",
    "diff",
    "explorer",
    "file",
    "terminal",
    "sidechat",
    "git",
    "pullRequest",
    "subagents",
    "backgroundTasks",
];

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

fn nullish(value: &Value) -> bool {
    value.is_null() || js::is_undefined(value)
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Object(_), Value::Object(_)) => js::is_undefined(a) && js::is_undefined(b),
        _ => false,
    }
}

fn js_to_string(value: &Value) -> String {
    if let Some(n) = js::as_number(value) {
        return js::number_to_string(n);
    }
    if js::is_undefined(value) {
        return "undefined".to_string();
    }
    if let Some(units) = js::string_units(value) {
        return String::from_utf16_lossy(&units);
    }
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                if item.is_null() || js::is_undefined(item) {
                    String::new()
                } else {
                    js_to_string(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn is_plain_object(value: &Value) -> bool {
    match value {
        Value::Object(o) => !(o.len() == 1 && o.keys().next().is_some_and(|k| k.starts_with('$'))),
        _ => false,
    }
}

fn is_singleton_kind(kind: &Value) -> bool {
    let Value::String(s) = kind else {
        return false;
    };
    s != "file" && s != "terminal" && PANE_KINDS.contains(&s.as_str())
}

fn kind_is(value: &Value, name: &str) -> bool {
    matches!(prop(value, "kind"), Value::String(k) if k == name)
}

fn panes_of(state: &Value) -> Result<Vec<Value>, String> {
    let panes = prop_or_err(state, "panes")?;
    match panes {
        Value::Array(items) => Ok(items),
        _ => Err("state.panes is not iterable".to_string()),
    }
}

fn state_with(state: &Value, extra: Vec<(&str, Value)>) -> Value {
    let mut map = Map::new();
    match state {
        Value::Object(o) if is_plain_object(state) => {
            for (k, v) in o {
                map.insert(k.clone(), v.clone());
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                map.insert(i.to_string(), v.clone());
            }
        }
        Value::String(_) => {
            if let Some(units) = js::string_units(state) {
                for (i, u) in units.iter().enumerate() {
                    map.insert(i.to_string(), js::utf16_string(&[*u]));
                }
            }
        }
        _ => {}
    }
    for (k, v) in extra {
        map.insert(k.to_string(), v);
    }
    Value::Object(map)
}

// `{...a, ...b}`: keys with undefined values end up absent after JSON.stringify.
fn spread_merge(base: &Value, patch: &Value) -> Value {
    let mut map = Map::new();
    for src in [base, patch] {
        match src {
            Value::Object(o) => {
                for (k, v) in o {
                    if js::is_undefined(v) {
                        map.shift_remove(k);
                    } else {
                        map.insert(k.clone(), v.clone());
                    }
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    map.insert(i.to_string(), v.clone());
                }
            }
            Value::String(_) => {
                if let Some(units) = js::string_units(src) {
                    for (i, u) in units.iter().enumerate() {
                        map.insert(i.to_string(), js::utf16_string(&[*u]));
                    }
                }
            }
            _ => {}
        }
    }
    Value::Object(map)
}

fn find_singleton(panes: &[Value], kind: &Value) -> Option<usize> {
    panes
        .iter()
        .position(|p| js_strict_eq(&prop(p, "kind"), kind))
}

fn default_state() -> Value {
    json!({"open": false, "panes": [], "activePaneId": null})
}

/// Port of `isSingletonPaneKind` (`apps/web/src/rightDockStore.logic.ts:72`).
pub fn is_singleton_pane_kind(kind: serde_json::Value) -> Result<bool, String> {
    Ok(is_singleton_kind(&kind))
}

/// Port of `createDefaultRightDockState` (`apps/web/src/rightDockStore.logic.ts:76`).
pub fn create_default_right_dock_state() -> Result<serde_json::Value, String> {
    Ok(default_state())
}

/// Port of `isRightDockPaneKind` (`apps/web/src/rightDockStore.logic.ts:84`).
pub fn is_right_dock_pane_kind(value: serde_json::Value) -> Result<bool, String> {
    Ok(matches!(&value, Value::String(s) if PANE_KINDS.contains(&s.as_str())))
}

fn sanitize_pane(value: &Value) -> Option<Value> {
    if !is_plain_object(value) {
        return None;
    }
    let id = prop(value, "id");
    let kind = prop(value, "kind");
    if !matches!(id, Value::String(_)) || !is_right_dock_pane_kind(kind.clone()).ok()? {
        return None;
    }
    let pick_str = |key: &str| -> Value {
        match prop(value, key) {
            Value::String(s) => Value::String(s),
            _ => Value::Null,
        }
    };
    let number = prop(value, "pullRequestNumber");
    let number = match js::as_number(&number) {
        Some(n) if n.fract() == 0.0 && n > 0.0 => number.clone(),
        _ => Value::Null,
    };
    let tab = prop(value, "pullRequestInitialTab");
    let tab = match &tab {
        Value::String(s) if s == "summary" || s == "timeline" || s == "code" => tab.clone(),
        _ => Value::Null,
    };
    Some(json!({
        "id": id,
        "kind": kind,
        "threadId": pick_str("threadId"),
        "diffTurnId": pick_str("diffTurnId"),
        "diffFilePath": pick_str("diffFilePath"),
        "filePath": pick_str("filePath"),
        "pullRequestProjectId": pick_str("pullRequestProjectId"),
        "pullRequestRepository": pick_str("pullRequestRepository"),
        "pullRequestNumber": number,
        "pullRequestInitialTab": tab,
    }))
}

/// Port of `sanitizeRightDockThreadState` (`apps/web/src/rightDockStore.logic.ts:128`).
pub fn sanitize_right_dock_thread_state(
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !is_plain_object(&value) {
        return Ok(default_state());
    }
    let mut sanitized: Vec<Value> = Vec::new();
    if let Value::Array(items) = prop(&value, "panes") {
        for item in &items {
            if let Some(pane) = sanitize_pane(item) {
                sanitized.push(pane);
            }
        }
    }
    let persisted_active = prop(&value, "activePaneId");
    let persisted_active = if matches!(persisted_active, Value::String(_)) {
        persisted_active
    } else {
        Value::Null
    };
    let mut kept: Vec<(String, String)> = Vec::new();
    for pane in &sanitized {
        let kind = prop(pane, "kind");
        let id = prop(pane, "id");
        if is_singleton_kind(&kind) {
            let kind_s = kind.as_str().unwrap_or_default().to_string();
            let id_s = id.as_str().unwrap_or_default().to_string();
            if js_strict_eq(&id, &persisted_active) || !kept.iter().any(|(k, _)| *k == kind_s) {
                if let Some(entry) = kept.iter_mut().find(|(k, _)| *k == kind_s) {
                    entry.1 = id_s;
                } else {
                    kept.push((kind_s, id_s));
                }
            }
        }
    }
    let panes: Vec<Value> = sanitized
        .into_iter()
        .filter(|pane| {
            let kind = prop(pane, "kind");
            if !is_singleton_kind(&kind) {
                return true;
            }
            let kind_s = kind.as_str().unwrap_or_default();
            let id = prop(pane, "id");
            kept.iter()
                .find(|(k, _)| k == kind_s)
                .is_some_and(|(_, v)| js_strict_eq(&id, &json!(v)))
        })
        .collect();
    let active = if !persisted_active.is_null()
        && panes
            .iter()
            .any(|p| js_strict_eq(&prop(p, "id"), &persisted_active))
    {
        persisted_active
    } else {
        panes.first().map(|p| prop(p, "id")).unwrap_or(Value::Null)
    };
    Ok(json!({
        "open": js_strict_eq(&prop(&value, "open"), &json!(true)),
        "panes": panes,
        "activePaneId": active,
    }))
}

/// Port of `sanitizeRightDockStateByThreadId` (`apps/web/src/rightDockStore.logic.ts:164`).
pub fn sanitize_right_dock_state_by_thread_id(
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    if let Value::Object(o) = &value {
        for (k, v) in o {
            if k == "__proto__" || k == "constructor" || k == "prototype" {
                continue;
            }
            if !js::is_undefined(v) {
                out.insert(k.clone(), sanitize_right_dock_thread_state(v.clone())?);
            }
        }
    }
    Ok(Value::Object(out))
}

fn create_pane(input: &Value) -> Value {
    let opt = |key: &str| -> Value {
        let v = prop(input, key);
        if nullish(&v) { Value::Null } else { v }
    };
    json!({
        "id": prop(input, "paneId"),
        "kind": prop(input, "kind"),
        "threadId": opt("threadId"),
        "diffTurnId": opt("diffTurnId"),
        "diffFilePath": opt("diffFilePath"),
        "filePath": opt("filePath"),
        "pullRequestProjectId": opt("pullRequestProjectId"),
        "pullRequestRepository": opt("pullRequestRepository"),
        "pullRequestNumber": opt("pullRequestNumber"),
        "pullRequestInitialTab": opt("pullRequestInitialTab"),
    })
}

fn reopen_patch(input: &Value) -> Option<Value> {
    let kind = prop(input, "kind");
    let has = |key: &str| !js::is_undefined(&prop(input, key));
    let opt = |key: &str| -> Value {
        let v = prop(input, key);
        if nullish(&v) { Value::Null } else { v }
    };
    if js_strict_eq(&kind, &json!("sidechat")) && has("threadId") {
        return Some(json!({"threadId": opt("threadId")}));
    }
    if js_strict_eq(&kind, &json!("diff")) && (has("diffTurnId") || has("diffFilePath")) {
        return Some(json!({"diffTurnId": opt("diffTurnId"), "diffFilePath": opt("diffFilePath")}));
    }
    if js_strict_eq(&kind, &json!("pullRequest"))
        && (has("pullRequestProjectId")
            || has("pullRequestRepository")
            || has("pullRequestNumber")
            || has("pullRequestInitialTab"))
    {
        return Some(json!({
            "pullRequestProjectId": opt("pullRequestProjectId"),
            "pullRequestRepository": opt("pullRequestRepository"),
            "pullRequestNumber": opt("pullRequestNumber"),
            "pullRequestInitialTab": opt("pullRequestInitialTab"),
        }));
    }
    None
}

/// Port of `openPaneInState` (`apps/web/src/rightDockStore.logic.ts:253`).
pub fn open_pane_in_state(
    state: serde_json::Value,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let panes = panes_of(&state)?;
    let kind = prop_or_err(&input, "kind")?;
    if is_singleton_kind(&kind) {
        if let Some(i) = find_singleton(&panes, &kind) {
            let existing_id = prop(&panes[i], "id");
            let next_panes = if let Some(patch) = reopen_patch(&input) {
                panes
                    .iter()
                    .map(|p| {
                        if js_strict_eq(&prop(p, "id"), &existing_id) {
                            spread_merge(p, &patch)
                        } else {
                            p.clone()
                        }
                    })
                    .collect()
            } else {
                panes
            };
            return Ok(json!({
                "open": true,
                "panes": next_panes,
                "activePaneId": existing_id,
            }));
        }
    } else if js_strict_eq(&kind, &json!("file")) {
        let file_path = {
            let v = prop(&input, "filePath");
            if nullish(&v) { Value::Null } else { v }
        };
        if let Some(existing) = panes
            .iter()
            .find(|p| kind_is(p, "file") && js_strict_eq(&prop(p, "filePath"), &file_path))
        {
            return Ok(json!({
                "open": true,
                "panes": panes,
                "activePaneId": prop(existing, "id"),
            }));
        }
    }
    let pane = create_pane(&input);
    let active = prop(&pane, "id");
    let mut next = panes;
    next.push(pane);
    Ok(json!({"open": true, "panes": next, "activePaneId": active}))
}

/// Port of `closePaneInState` (`apps/web/src/rightDockStore.logic.ts:293`).
pub fn close_pane_in_state(
    state: serde_json::Value,
    pane_id: String,
) -> Result<serde_json::Value, String> {
    let panes = panes_of(&state)?;
    let target = json!(pane_id);
    let removed = panes
        .iter()
        .position(|p| js_strict_eq(&prop(p, "id"), &target));
    let Some(removed_index) = removed else {
        return Ok(state);
    };
    let next_panes: Vec<Value> = panes
        .into_iter()
        .filter(|p| !js_strict_eq(&prop(p, "id"), &target))
        .collect();
    let prev_active = prop(&state, "activePaneId");
    let next_active = if !js_strict_eq(&prev_active, &target) {
        prev_active
    } else if next_panes.is_empty() {
        Value::Null
    } else {
        let idx = removed_index.min(next_panes.len() - 1);
        prop(&next_panes[idx], "id")
    };
    Ok(state_with(
        &state,
        vec![
            ("open", prop(&state, "open")),
            ("panes", json!(next_panes)),
            ("activePaneId", next_active),
        ],
    ))
}

/// Port of `movePaneInState` (`apps/web/src/rightDockStore.logic.ts:318`).
pub fn move_pane_in_state(
    state: serde_json::Value,
    pane_id: String,
    over_pane_id: String,
) -> Result<serde_json::Value, String> {
    let panes = panes_of(&state)?;
    let from = panes
        .iter()
        .position(|p| js_strict_eq(&prop(p, "id"), &json!(pane_id)));
    let to = panes
        .iter()
        .position(|p| js_strict_eq(&prop(p, "id"), &json!(over_pane_id)));
    let (Some(from), Some(to)) = (from, to) else {
        return Ok(state);
    };
    if from == to {
        return Ok(state);
    }
    let mut next = panes;
    let item = next.remove(from);
    next.insert(to, item);
    Ok(state_with(&state, vec![("panes", json!(next))]))
}

/// Port of `setActivePaneInState` (`apps/web/src/rightDockStore.logic.ts:330`).
pub fn set_active_pane_in_state(
    state: serde_json::Value,
    pane_id: String,
) -> Result<serde_json::Value, String> {
    let panes = panes_of(&state)?;
    let target = json!(pane_id);
    if !panes.iter().any(|p| js_strict_eq(&prop(p, "id"), &target)) {
        return Ok(state);
    }
    Ok(state_with(
        &state,
        vec![("open", json!(true)), ("activePaneId", target)],
    ))
}

/// Port of `setDockOpenInState` (`apps/web/src/rightDockStore.logic.ts:340`).
pub fn set_dock_open_in_state(
    state: serde_json::Value,
    open: bool,
) -> Result<serde_json::Value, String> {
    let current = prop_or_err(&state, "open")?;
    if js_strict_eq(&current, &json!(open)) {
        return Ok(state);
    }
    Ok(state_with(&state, vec![("open", json!(open))]))
}

const PATCHED_KEYS: [&str; 8] = [
    "diffTurnId",
    "diffFilePath",
    "filePath",
    "threadId",
    "pullRequestProjectId",
    "pullRequestRepository",
    "pullRequestNumber",
    "pullRequestInitialTab",
];

/// Port of `updatePaneInState` (`apps/web/src/rightDockStore.logic.ts:350`).
pub fn update_pane_in_state(
    state: serde_json::Value,
    pane_id: String,
    patch: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let panes = panes_of(&state)?;
    let target = json!(pane_id);
    let mut changed = false;
    let mut next_panes = Vec::with_capacity(panes.len());
    for pane in &panes {
        if !js_strict_eq(&prop(pane, "id"), &target) {
            next_panes.push(pane.clone());
            continue;
        }
        let mut next_map = match pane {
            Value::Object(o) => o.clone(),
            _ => Map::new(),
        };
        if let Value::Object(patch_obj) = &patch {
            for key in PATCHED_KEYS {
                if let Some(v) = patch_obj.get(key) {
                    if js::is_undefined(v) {
                        next_map.shift_remove(key);
                    } else {
                        next_map.insert(key.to_string(), v.clone());
                    }
                }
            }
        }
        let next = Value::Object(next_map);
        if PATCHED_KEYS
            .iter()
            .any(|k| !js_strict_eq(&prop(&next, k), &prop(pane, k)))
        {
            changed = true;
            next_panes.push(next);
        } else {
            next_panes.push(pane.clone());
        }
    }
    if changed {
        Ok(state_with(&state, vec![("panes", json!(next_panes))]))
    } else {
        Ok(state)
    }
}

/// Port of `setSidechatPaneThreadInState` (`apps/web/src/rightDockStore.logic.ts:394`).
pub fn set_sidechat_pane_thread_in_state(
    state: serde_json::Value,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let panes = panes_of(&state)?;
    let existing = find_singleton(&panes, &json!("sidechat"));
    let thread_id = prop_or_err(&input, "threadId")?;
    if thread_id.is_null() {
        return match existing {
            Some(i) => {
                let id = prop(&panes[i], "id");
                let id_s = match &id {
                    Value::String(s) => s.clone(),
                    other => js_to_string(other),
                };
                close_pane_in_state(state, id_s)
            }
            None => Ok(state),
        };
    }
    if let Some(i) = existing {
        let id = prop(&panes[i], "id");
        let id_s = match &id {
            Value::String(s) => s.clone(),
            other => js_to_string(other),
        };
        return update_pane_in_state(state, id_s, json!({"threadId": thread_id}));
    }
    let pane = create_pane(&json!({
        "paneId": prop(&input, "paneId"),
        "kind": "sidechat",
        "threadId": thread_id,
    }));
    let active = prop(&pane, "id");
    let mut next = panes;
    next.push(pane);
    Ok(state_with(
        &state,
        vec![
            ("open", prop(&state, "open")),
            ("panes", json!(next)),
            ("activePaneId", active),
        ],
    ))
}

/// Port of `toggleSingletonPaneInState` (`apps/web/src/rightDockStore.logic.ts:412`).
pub fn toggle_singleton_pane_in_state(
    state: serde_json::Value,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let panes = panes_of(&state)?;
    let kind = prop(&input, "kind");
    if let Some(i) = find_singleton(&panes, &kind)
        && truthy(&prop(&state, "open"))
        && js_strict_eq(&prop(&state, "activePaneId"), &prop(&panes[i], "id"))
    {
        return Ok(state_with(&state, vec![("open", json!(false))]));
    }
    open_pane_in_state(state, input)
}

/// Port of `resolveActivePane` (`apps/web/src/rightDockStore.logic.ts:423`).
pub fn resolve_active_pane(state: serde_json::Value) -> Result<Option<serde_json::Value>, String> {
    if !truthy(&prop_or_err(&state, "open")?) || prop(&state, "activePaneId").is_null() {
        return Ok(None);
    }
    let active = prop(&state, "activePaneId");
    let panes = panes_of(&state)?;
    Ok(panes
        .into_iter()
        .find(|p| js_strict_eq(&prop(p, "id"), &active)))
}

/// Port of `findMissingSidechatPaneIds` (`apps/web/src/rightDockStore.logic.ts:430`).
pub fn find_missing_sidechat_pane_ids(
    state: serde_json::Value,
    existing_thread_ids: serde_json::Value,
) -> Result<Vec<String>, String> {
    let panes = panes_of(&state)?;
    let mut out = Vec::new();
    for pane in &panes {
        if !kind_is(pane, "sidechat") {
            continue;
        }
        let thread = prop(pane, "threadId");
        if !truthy(&thread) {
            continue;
        }
        let present = match &existing_thread_ids {
            Value::Array(items) => items.iter().any(|v| js_strict_eq(v, &thread)),
            Value::Object(o) => {
                if let Some(set) = o.get("$set") {
                    set.as_array()
                        .map(|items| items.iter().any(|v| js_strict_eq(v, &thread)))
                        .unwrap_or(false)
                } else {
                    return Err("existingThreadIds.has is not a function".to_string());
                }
            }
            _ => return Err("existingThreadIds.has is not a function".to_string()),
        };
        if !present {
            if let Value::String(id) = prop(pane, "id") {
                out.push(id);
            } else {
                out.push(js_to_string(&prop(pane, "id")));
            }
        }
    }
    Ok(out)
}

/// Port of `resolveVisibleDockSidechatThreadIds` (`apps/web/src/rightDockStore.logic.ts:444`).
pub fn resolve_visible_dock_sidechat_thread_ids(
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    if !truthy(&prop_or_err(&input, "dockRendered")?) {
        return Ok(Vec::new());
    }
    let mut out: Vec<Value> = Vec::new();
    let mut seen: Vec<Value> = Vec::new();
    let hosts = match prop(&input, "hostThreadIds") {
        Value::Array(items) => items,
        _ => Vec::new(),
    };
    for host in &hosts {
        seen.push(host.clone());
    }
    for host in &hosts {
        let key = js_to_string(host);
        let dock = prop(&input, "dockStateByThreadId")
            .get(&key)
            .cloned()
            .unwrap_or_else(js::undefined);
        if !truthy(&dock) {
            continue;
        }
        if !truthy(&prop(&dock, "open")) || prop(&dock, "activePaneId").is_null() {
            continue;
        }
        let active = prop(&dock, "activePaneId");
        let panes = panes_of(&dock)?;
        let pane = panes.iter().find(|p| js_strict_eq(&prop(p, "id"), &active));
        let Some(pane) = pane else { continue };
        if !kind_is(pane, "sidechat") {
            continue;
        }
        let thread = prop(pane, "threadId");
        if !truthy(&thread) {
            continue;
        }
        if !seen.iter().any(|v| js_strict_eq(v, &thread)) {
            seen.push(thread.clone());
            out.push(thread);
        }
    }
    Ok(out)
}
