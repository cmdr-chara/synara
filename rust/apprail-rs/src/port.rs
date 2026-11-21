//! The Rust port of `apps/web/src/appRail.logic.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

const RAIL_ORDERABLE_ITEM_IDS: [&str; 8] = [
    "home",
    "inbox",
    "spaces",
    "kanban",
    "tasks",
    "pullRequests",
    "automations",
    "studio",
];

const SPACE_SHORTCUT_PREFIX: &str = "space:";
const PROJECT_SHORTCUT_PREFIX: &str = "project:";
const VOID_SPACE_KEY: &str = "void";

fn truthy(v: &Value) -> bool {
    if js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_key_str(v: &Value) -> String {
    if js::is_undefined(v) {
        return "undefined".to_string();
    }
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(items) => items
            .iter()
            .map(|i| match i {
                Value::Null => String::new(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
                Value::String(s) => s.clone(),
                Value::Array(_) | Value::Object(_) => "[object Object]".to_string(),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn is_orderable(value: &Value) -> bool {
    matches!(value, Value::String(s) if RAIL_ORDERABLE_ITEM_IDS.contains(&s.as_str()))
}

/// Port of `isRailOrderableItemId` (`apps/web/src/appRail.logic.ts:53`).
pub fn is_rail_orderable_item_id(value: String) -> Result<bool, String> {
    Ok(RAIL_ORDERABLE_ITEM_IDS.contains(&value.as_str()))
}

/// Port of `railItemCanHide` (`apps/web/src/appRail.logic.ts:58`).
pub fn rail_item_can_hide(id: serde_json::Value) -> Result<bool, String> {
    Ok(!matches!(&id, Value::String(s) if s == "home"))
}

fn normalize_known_ids(ids: &[String]) -> Vec<String> {
    let mut seen = Vec::new();
    for c in ids {
        if is_orderable(&json!(c)) && !seen.contains(c) {
            seen.push(c.clone());
        }
    }
    seen
}

/// Port of `normalizeRailItemOrder` (`apps/web/src/appRail.logic.ts:62`).
pub fn normalize_rail_item_order(order: Vec<String>) -> Result<Vec<serde_json::Value>, String> {
    let mut normalized = normalize_known_ids(&order);
    let seen = normalized.clone();
    for id in RAIL_ORDERABLE_ITEM_IDS {
        if !seen.contains(&id.to_string()) {
            normalized.push(id.to_string());
        }
    }
    // placeNewIdAfter(normalized, order, "inbox", "home")
    if !order.iter().any(|e| e == "inbox") {
        normalized.retain(|e| e != "inbox");
        let pos = normalized
            .iter()
            .position(|e| e == "home")
            .map(|p| p + 1)
            .unwrap_or(0);
        normalized.insert(pos, "inbox".to_string());
    }
    Ok(normalized.iter().map(|s| json!(s)).collect())
}

/// Port of `normalizeHiddenRailItems` (`apps/web/src/appRail.logic.ts:68`).
pub fn normalize_hidden_rail_items(hidden: Vec<String>) -> Result<Vec<serde_json::Value>, String> {
    Ok(normalize_known_ids(&hidden)
        .into_iter()
        .filter(|id| id != "home")
        .map(|s| json!(s))
        .collect())
}

/// Port of `isRailItemAvailable` (`apps/web/src/appRail.logic.ts:80`).
pub fn is_rail_item_available(
    id: serde_json::Value,
    availability: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match &id {
        Value::String(s) if s == "studio" => {
            if availability.is_null() {
                return Err(
                    "null is not an object (evaluating 'availability.studioAvailable')".to_string(),
                );
            }
            Ok(json!(truthy(
                availability.get("studioAvailable").unwrap_or(&Value::Null)
            )))
        }
        Value::String(s) if s == "inbox" => {
            if availability.is_null() {
                return Err(
                    "null is not an object (evaluating 'availability.inboxAvailable')".to_string(),
                );
            }
            Ok(json!(truthy(
                availability.get("inboxAvailable").unwrap_or(&Value::Null)
            )))
        }
        _ => Ok(json!(true)),
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => {
            matches!((x.as_f64(), y.as_f64()), (Some(p), Some(q)) if p == q)
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(_), Value::Array(_)) => a == b,
        (Value::Object(_), Value::Object(_)) => a == b,
        _ => false,
    }
}

/// Port of `buildRailItemOrder` (`apps/web/src/appRail.logic.ts:90`).
pub fn build_rail_item_order(input: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    if input.is_null() {
        return Err("null is not an object (evaluating 'input.order')".to_string());
    }
    let order = match input.get("order") {
        None => {
            return Err("undefined is not an object (evaluating 'input.order.filter')".to_string());
        }
        Some(Value::Null) => {
            return Err("null is not an object (evaluating 'input.order.filter')".to_string());
        }
        Some(v) if js::is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'input.order.filter')".to_string());
        }
        Some(Value::Array(a)) => a.clone(),
        Some(_) => {
            return Err("input.order.filter is not a function".to_string());
        }
    };
    let hidden = input.get("hidden").cloned();
    let active = input.get("activeItem").cloned().unwrap_or(Value::Null);
    let mut out = Vec::new();
    for id in order {
        let available = is_rail_item_available(id.clone(), input.clone())?;
        if !truthy(&available) {
            continue;
        }
        let is_hidden = match &hidden {
            Some(Value::Array(list)) => list.iter().any(|h| strict_eq(h, &id)),
            Some(Value::Object(_)) | Some(Value::Null) | None => {
                return Err("input.hidden.has is not a function".to_string());
            }
            Some(v) if js::is_undefined(v) => {
                return Err("input.hidden.has is not a function".to_string());
            }
            Some(_) => {
                return Err("input.hidden.has is not a function".to_string());
            }
        };
        if !is_hidden || strict_eq(&id, &active) {
            out.push(id);
        }
    }
    Ok(out)
}

/// Port of `railSpaceShortcutKey` (`apps/web/src/appRail.logic.ts:110`).
pub fn rail_space_shortcut_key(space_id: Option<serde_json::Value>) -> Result<String, String> {
    let key = match &space_id {
        None | Some(Value::Null) => VOID_SPACE_KEY.to_string(),
        Some(v) if js::is_undefined(v) => VOID_SPACE_KEY.to_string(),
        Some(v) => js_key_str(v),
    };
    Ok(format!("{SPACE_SHORTCUT_PREFIX}{key}"))
}

/// Port of `railProjectShortcutKey` (`apps/web/src/appRail.logic.ts:114`).
pub fn rail_project_shortcut_key(project_id: serde_json::Value) -> Result<String, String> {
    Ok(format!(
        "{PROJECT_SHORTCUT_PREFIX}{}",
        js_key_str(&project_id)
    ))
}

/// Port of `resolveRailShortcuts` (`apps/web/src/appRail.logic.ts:122`).
pub fn resolve_rail_shortcuts(input: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    if input.is_null() {
        return Err("null is not an object (evaluating 'input.keys')".to_string());
    }
    let keys = match input.get("keys") {
        Some(Value::Array(a)) => a.clone(),
        _ if input.is_array() => {
            // `[].keys` is the Array.prototype.keys function: iterating it fails.
            return Err("{} is not iterable".to_string());
        }
        _ => return Err("undefined is not an object (evaluating 'key of input.keys')".to_string()),
    };
    let space_ids = input.get("spaceIds").cloned().unwrap_or(Value::Null);
    let project_ids = input.get("projectIds").cloned().unwrap_or(Value::Null);
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for key_v in keys {
        let key = match &key_v {
            Value::String(s) => s.clone(),
            _ => {
                return Err("key.startsWith is not a function".to_string());
            }
        };
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        if let Some(id) = key.strip_prefix(SPACE_SHORTCUT_PREFIX) {
            if id == VOID_SPACE_KEY {
                out.push(json!({"kind": "space", "key": key, "spaceId": Value::Null}));
            } else {
                match &space_ids {
                    Value::Array(list) => {
                        if list
                            .iter()
                            .any(|s| matches!(s, Value::String(x) if x == id))
                        {
                            out.push(json!({"kind": "space", "key": key, "spaceId": id}));
                        }
                    }
                    Value::Null => return Err("input.spaceIds.has is not a function".to_string()),
                    _ => return Err("input.spaceIds.has is not a function".to_string()),
                }
            }
        } else if let Some(id) = key.strip_prefix(PROJECT_SHORTCUT_PREFIX) {
            match &project_ids {
                Value::Array(list) => {
                    if list
                        .iter()
                        .any(|s| matches!(s, Value::String(x) if x == id))
                    {
                        out.push(json!({"kind": "project", "key": key, "projectId": id}));
                    }
                }
                _ => return Err("input.projectIds.has is not a function".to_string()),
            }
        }
    }
    Ok(out)
}

/// Port of `toggleRailShortcutKey` (`apps/web/src/appRail.logic.ts:148`).
pub fn toggle_rail_shortcut_key(keys: Vec<String>, key: String) -> Result<Vec<String>, String> {
    if keys.contains(&key) {
        Ok(keys.into_iter().filter(|k| *k != key).collect())
    } else {
        let mut out = keys;
        out.push(key);
        Ok(out)
    }
}

/// Port of `resolveActiveRailShortcutKey` (`apps/web/src/appRail.logic.ts:156`).
pub fn resolve_active_rail_shortcut_key(
    input: serde_json::Value,
) -> Result<Option<String>, String> {
    if input.is_null() {
        return Err("null is not an object (evaluating 'input.shortcuts')".to_string());
    }
    let shortcuts = match input.get("shortcuts") {
        Some(Value::Array(a)) => a.clone(),
        _ => {
            return Err(
                "undefined is not an object (evaluating 'shortcut of input.shortcuts')".to_string(),
            );
        }
    };
    let active_item = input.get("activeItem").cloned().unwrap_or(Value::Null);
    let active_space = input.get("activeSpaceId").cloned().unwrap_or(Value::Null);
    let spaces_project = input.get("spacesProjectId").cloned().unwrap_or(Value::Null);
    for shortcut in shortcuts {
        let kind = shortcut.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        if kind == "space"
            && matches!(&active_item, Value::String(s) if s == "home")
            && strict_eq(
                shortcut.get("spaceId").unwrap_or(&Value::Null),
                &active_space,
            )
        {
            return Ok(shortcut
                .get("key")
                .and_then(|k| k.as_str())
                .map(|s| s.to_string()));
        }
        if kind == "project"
            && matches!(&active_item, Value::String(s) if s == "spaces")
            && strict_eq(
                shortcut.get("projectId").unwrap_or(&Value::Null),
                &spaces_project,
            )
        {
            return Ok(shortcut
                .get("key")
                .and_then(|k| k.as_str())
                .map(|s| s.to_string()));
        }
    }
    Ok(None)
}

/// Port of `railItemShowsPanel` (`apps/web/src/appRail.logic.ts:187`).
pub fn rail_item_shows_panel(id: serde_json::Value) -> Result<bool, String> {
    Ok(
        !matches!(&id, Value::String(s) if s == "kanban" || s == "tasks" || s == "pullRequests" || s == "inbox"),
    )
}

fn matches_route(pathname: &str, route: &str) -> bool {
    pathname == route || pathname.starts_with(&format!("{route}/"))
}

fn rail_item_for_pathname_inner(pathname: &str) -> Option<&'static str> {
    if matches_route(pathname, "/inbox") {
        return Some("inbox");
    }
    if matches_route(pathname, "/kanban") {
        return Some("kanban");
    }
    if matches_route(pathname, "/tasks") {
        return Some("tasks");
    }
    if matches_route(pathname, "/pull-requests") {
        return Some("pullRequests");
    }
    if matches_route(pathname, "/automations") {
        return Some("automations");
    }
    if matches_route(pathname, "/hubs")
        || matches_route(pathname, "/groups")
        || matches_route(pathname, "/studio")
    {
        return Some("studio");
    }
    if matches_route(pathname, "/settings") {
        return Some("settings");
    }
    None
}

/// Port of `railItemForPathname` (`apps/web/src/appRail.logic.ts:196`).
pub fn rail_item_for_pathname(pathname: String) -> Result<Option<serde_json::Value>, String> {
    Ok(rail_item_for_pathname_inner(&pathname).map(|s| json!(s)))
}

/// Port of `reconcileActiveRailItem` (`apps/web/src/appRail.logic.ts:219`).
pub fn reconcile_active_rail_item(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("null is not an object (evaluating 'input.pathname')".to_string());
    }
    let pathname = input.get("pathname").cloned().unwrap_or(Value::Null);
    let route_item = match &pathname {
        Value::String(s) => rail_item_for_pathname_inner(s).map(|s| json!(s)),
        Value::Null => {
            return Err(
                "undefined is not an object (evaluating 'pathname.startsWith')".to_string(),
            );
        }
        v if js::is_undefined(v) => {
            return Err(
                "undefined is not an object (evaluating 'pathname.startsWith')".to_string(),
            );
        }
        _ => {
            return Err("pathname.startsWith is not a function".to_string());
        }
    };
    if let Some(item) = route_item {
        return Ok(item);
    }
    if truthy(input.get("onStudioSurface").unwrap_or(&Value::Null)) {
        return Ok(json!("studio"));
    }
    Ok(input.get("panelView").cloned().unwrap_or(Value::Null))
}
