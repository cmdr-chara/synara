//! The Rust port of `apps/web/src/openThreadTabs.logic.ts`.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use serde_json::{Map, Value, json};

fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) => o.get(key),
        _ => None,
    }
}

/// `v === other` — strict equality: different types or distinct objects/arrays
/// never match (heap identity in JS is untestable over the wire).
fn strict_eq(a: &Value, b: &Value) -> bool {
    if crate::js::is_undefined(a) || crate::js::is_undefined(b) {
        return crate::js::is_undefined(a) && crate::js::is_undefined(b);
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().unwrap_or(f64::NAN) == y.as_f64().unwrap_or(f64::NAN)
        }
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

/// SameValueZero — `strict_eq` plus `NaN === NaN`.
fn same_value_zero(a: &Value, b: &Value) -> bool {
    if let (Value::Number(x), Value::Number(y)) = (a, b)
        && x.as_f64().is_some_and(f64::is_nan)
        && y.as_f64().is_some_and(f64::is_nan)
    {
        return true;
    }
    strict_eq(a, b)
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a.iter().map(js_to_string).collect::<Vec<_>>().join(","),
        Value::Object(o) => {
            if crate::js::is_undefined(v) {
                "undefined".to_string()
            } else {
                let _ = o;
                "[object Object]".to_string()
            }
        }
    }
}

/// `x.y` on `undefined`/missing.
fn undefined_member_err(expr: &str) -> String {
    format!("undefined is not an object (evaluating '{expr}')")
}

/// `x.y` on `null`.
fn null_member_err(expr: &str) -> String {
    format!("null is not an object (evaluating '{expr}')")
}

/// `x.y` exists nowhere on a non-nullish value.
fn not_a_function_err(expr: &str, member: &str) -> String {
    format!("{member} is not a function. (In '{expr}', '{member}' is undefined)")
}

/// `.includes(value)` — exists on arrays (SameValueZero) and strings (substring).
enum Includes {
    Found,
    Missing,
    NotAvailable,
    Null,
    Undefined,
}

fn includes(container: &Value, needle: &Value) -> Includes {
    if container.is_null() {
        return Includes::Null;
    }
    if crate::js::is_undefined(container) {
        return Includes::Undefined;
    }
    match container {
        Value::Array(items) => {
            if items.iter().any(|item| same_value_zero(item, needle)) {
                Includes::Found
            } else {
                Includes::Missing
            }
        }
        Value::String(s) => {
            if s.contains(&js_to_string(needle)) {
                Includes::Found
            } else {
                Includes::Missing
            }
        }
        _ => Includes::NotAvailable,
    }
}

/// `[...x]` — arrays copy, strings spread UTF-16 units; other values throw.
fn spread(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(items) => items.clone(),
        Value::String(s) => s
            .encode_utf16()
            .map(|u| crate::js::utf16_string(&[u]))
            .collect(),
        _ => Vec::new(),
    }
}

/// Port of `addOpenThreadTab` (`apps/web/src/openThreadTabs.logic.ts:43`).
pub fn add_open_thread_tab(
    thread_ids: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match includes(&thread_ids, &thread_id) {
        Includes::Null => Err(null_member_err("threadIds.includes")),
        Includes::Undefined => Err(undefined_member_err("threadIds.includes")),
        Includes::NotAvailable => Err(not_a_function_err(
            "threadIds.includes(threadId)",
            "threadIds.includes",
        )),
        Includes::Found => Ok(thread_ids),
        Includes::Missing => {
            let mut out = spread(&thread_ids);
            out.push(thread_id);
            Ok(Value::Array(out))
        }
    }
}

/// Port of `removeOpenThreadTab` (`apps/web/src/openThreadTabs.logic.ts:50`).
pub fn remove_open_thread_tab(
    thread_ids: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match includes(&thread_ids, &thread_id) {
        Includes::Null => Err(null_member_err("threadIds.includes")),
        Includes::Undefined => Err(undefined_member_err("threadIds.includes")),
        Includes::NotAvailable => Err(not_a_function_err(
            "threadIds.includes(threadId)",
            "threadIds.includes",
        )),
        Includes::Missing => Ok(thread_ids),
        Includes::Found => match &thread_ids {
            Value::Array(items) => Ok(Value::Array(
                items
                    .iter()
                    .filter(|c| !strict_eq(c, &thread_id))
                    .cloned()
                    .collect(),
            )),
            _ => Err(not_a_function_err(
                "threadIds.filter((candidate) => candidate !== threadId)",
                "threadIds.filter",
            )),
        },
    }
}

fn index_of(container: &Value, needle: &Value) -> i64 {
    match container {
        Value::Array(items) => items
            .iter()
            .position(|item| strict_eq(item, needle))
            .map(|i| i as i64)
            .unwrap_or(-1),
        Value::String(s) => s
            .find(&js_to_string(needle))
            .map(|i| i as i64)
            .unwrap_or(-1),
        _ => -1,
    }
}

/// Port of `moveOpenThreadTab` (`apps/web/src/openThreadTabs.logic.ts:63`).
pub fn move_open_thread_tab(
    thread_ids: serde_json::Value,
    thread_id: serde_json::Value,
    over_thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if thread_ids.is_null() {
        return Err(null_member_err("threadIds.indexOf"));
    }
    if crate::js::is_undefined(&thread_ids) {
        return Err(undefined_member_err("threadIds.indexOf"));
    }
    if !matches!(thread_ids, Value::Array(_) | Value::String(_)) {
        return Err(not_a_function_err(
            "threadIds.indexOf(threadId)",
            "threadIds.indexOf",
        ));
    }
    let from = index_of(&thread_ids, &thread_id);
    let to = index_of(&thread_ids, &over_thread_id);
    if from < 0 || to < 0 || from == to {
        return Ok(thread_ids);
    }
    let mut items = spread(&thread_ids);
    let moved = items.remove(from as usize);
    items.insert(to as usize, moved);
    Ok(Value::Array(items))
}

/// Port of `normalizeOpenThreadTabIds` (`apps/web/src/openThreadTabs.logic.ts:83`).
pub fn normalize_open_thread_tab_ids(
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let Some(items) = input.as_array() else {
        return Ok(Vec::new());
    };
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for candidate in items {
        let Some(s) = candidate.as_str() else {
            continue;
        };
        let trimmed = s.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        out.push(Value::String(trimmed.to_string()));
    }
    Ok(out)
}

fn is_sidechat_thread(thread: &Value) -> bool {
    get(thread, "sidechatSourceThreadId").is_some_and(truthy)
        || get(thread, "sidechatContext").is_some_and(|v| !is_nullish(v))
}

fn can_keep(summary: Option<&Value>, has_draft: bool) -> bool {
    match summary {
        Some(s) if truthy(s) => {
            get(s, "archivedAt").is_none_or(is_nullish) && !is_sidechat_thread(s)
        }
        _ => has_draft,
    }
}

/// Port of `canKeepOpenThreadTab` (`apps/web/src/openThreadTabs.logic.ts:103`).
pub fn can_keep_open_thread_tab(
    summary: Option<serde_json::Value>,
    has_draft: bool,
) -> Result<bool, String> {
    Ok(can_keep(summary.as_ref(), has_draft))
}

/// Port of `buildOpenThreadTabs` (`apps/web/src/openThreadTabs.logic.ts:146`).
/// The recorded corpus only exercises the `input.sources` access ladder; the
/// per-source tab assembly mirrors the source shape.
pub fn build_open_thread_tabs(input: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    if input.is_null() {
        return Err(null_member_err("input.sources"));
    }
    if crate::js::is_undefined(&input) {
        return Err(undefined_member_err("input.sources"));
    }
    let sources = get(&input, "sources");
    let Some(Value::Array(sources)) = sources else {
        return Err(match sources {
            Some(v) if v.is_null() => null_member_err("input.sources.flatMap"),
            Some(v) if !v.is_array() && !crate::js::is_undefined(v) => not_a_function_err(
                "input.sources.flatMap((source) => {",
                "input.sources.flatMap",
            ),
            _ => undefined_member_err("input.sources.flatMap"),
        });
    };
    let active = get(&input, "activeThreadId");
    let project = get(&input, "projectId");
    let mut tabs = Vec::new();
    for source in sources {
        let source_tid = get(source, "threadId");
        let same_as_active = match (source_tid, active) {
            (Some(a), Some(b)) => strict_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        let summary = get(source, "summary");
        if !same_as_active && !can_keep(summary, false) {
            continue;
        }
        let Some(summary) = summary.filter(|s| truthy(s)) else {
            continue;
        };
        // `summary.modelSelection.provider` throws on a null selection.
        let provider = get(summary, "session")
            .filter(|s| !is_nullish(s))
            .and_then(|s| get(s, "provider"));
        let provider = match provider {
            Some(p) => Some(p.clone()),
            None => match get(summary, "modelSelection") {
                None => {
                    return Err(undefined_member_err("summary.modelSelection.provider"));
                }
                Some(v) if v.is_null() => {
                    return Err(null_member_err("summary.modelSelection.provider"));
                }
                Some(v) if crate::js::is_undefined(v) => {
                    return Err(undefined_member_err("summary.modelSelection.provider"));
                }
                Some(v) => get(v, "provider").cloned(),
            },
        };
        let mut tab = Map::new();
        tab.insert(
            "threadId".to_string(),
            source_tid.cloned().unwrap_or_else(crate::js::undefined),
        );
        tab.insert(
            "projectId".to_string(),
            get(summary, "projectId")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        );
        tab.insert(
            "title".to_string(),
            get(summary, "title")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        );
        tab.insert(
            "provider".to_string(),
            provider.unwrap_or_else(crate::js::undefined),
        );
        tab.insert(
            "isTerminal".to_string(),
            get(source, "terminalEntryPoint")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        );
        tab.insert("isRunning".to_string(), Value::Bool(false));
        if let Some(project) = project
            && truthy(project)
            && tab.get("projectId").is_some_and(|p| !strict_eq(p, project))
        {
            continue;
        }
        tabs.push(Value::Object(tab));
    }
    Ok(tabs)
}

/// Port of `resolveOpenThreadTabCloseTarget` (`apps/web/src/openThreadTabs.logic.ts:168`).
pub fn resolve_open_thread_tab_close_target(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if input.is_null() {
        return Err(null_member_err("input.closedThreadId"));
    }
    if crate::js::is_undefined(&input) {
        return Err(undefined_member_err("input.closedThreadId"));
    }
    let closed = get(&input, "closedThreadId");
    let active = get(&input, "activeThreadId");
    let same = match (closed, active) {
        (Some(a), Some(b)) => strict_eq(a, b),
        (None, None) => true,
        _ => false,
    };
    if !same {
        return Ok(None);
    }
    let tabs = get(&input, "tabs");
    let Some(Value::Array(tabs)) = tabs else {
        return Err(match tabs {
            Some(v) if v.is_null() => null_member_err("input.tabs.findIndex"),
            Some(v) if !v.is_array() && !crate::js::is_undefined(v) => not_a_function_err(
                "input.tabs.findIndex((tab) => tab.threadId === input.closedThreadId)",
                "input.tabs.findIndex",
            ),
            _ => undefined_member_err("input.tabs.findIndex"),
        });
    };
    let mut closed_index: i64 = -1;
    for (i, tab) in tabs.iter().enumerate() {
        if tab.is_null() {
            return Err(null_member_err("tab.threadId"));
        }
        if crate::js::is_undefined(tab) {
            return Err(undefined_member_err("tab.threadId"));
        }
        if let (Some(t), Some(c)) = (get(tab, "threadId"), closed)
            && strict_eq(t, c)
        {
            closed_index = i as i64;
            break;
        }
    }
    let remaining: Vec<&Value> = tabs
        .iter()
        .filter(|tab| match (get(tab, "threadId"), closed) {
            (Some(t), Some(c)) => !strict_eq(t, c),
            _ => true,
        })
        .collect();
    let thread_id = if remaining.is_empty() {
        Value::Null
    } else {
        let idx = closed_index.max(0).min(remaining.len() as i64 - 1) as usize;
        get(remaining[idx], "threadId")
            .cloned()
            .unwrap_or(Value::Null)
    };
    Ok(Some(json!({ "threadId": thread_id })))
}

/// Port of `resolveOpenThreadTabsInCloseScope` (`apps/web/src/openThreadTabs.logic.ts:261`).
pub fn resolve_open_thread_tabs_in_close_scope(
    tabs: serde_json::Value,
    anchor_thread_id: serde_json::Value,
    scope: String,
) -> Result<Vec<serde_json::Value>, String> {
    if tabs.is_null() {
        return Err(null_member_err("tabs.findIndex"));
    }
    if crate::js::is_undefined(&tabs) {
        return Err(undefined_member_err("tabs.findIndex"));
    }
    let Value::Array(items) = &tabs else {
        return Err(not_a_function_err(
            "tabs.findIndex((tab) => tab.threadId === anchorThreadId)",
            "tabs.findIndex",
        ));
    };
    let mut anchor_index: i64 = -1;
    for (i, tab) in items.iter().enumerate() {
        if tab.is_null() {
            return Err(null_member_err("tab.threadId"));
        }
        if crate::js::is_undefined(tab) {
            return Err(undefined_member_err("tab.threadId"));
        }
        if get(tab, "threadId").is_some_and(|t| strict_eq(t, &anchor_thread_id)) {
            anchor_index = i as i64;
            break;
        }
    }
    if anchor_index < 0 {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (i, tab) in items.iter().enumerate() {
        let i = i as i64;
        let keep = if scope == "left" {
            i < anchor_index
        } else if scope == "right" {
            i > anchor_index
        } else {
            i != anchor_index
        };
        if !keep {
            continue;
        }
        if tab.is_null() {
            return Err(null_member_err("tab.threadId"));
        }
        if crate::js::is_undefined(tab) {
            return Err(undefined_member_err("tab.threadId"));
        }
        out.push(
            get(tab, "threadId")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        );
    }
    Ok(out)
}
