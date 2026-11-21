//! The Rust port of `apps/web/src/recentViews.logic.ts`.

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

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn js_str(v: &Value) -> String {
    if let Some(n) = crate::js::as_number(v) {
        return crate::js::number_to_string(n);
    }
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
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
        v if is_undefined(v) => "undefined".to_string(),
        _ => "[object Object]".to_string(),
    }
}

/// Optional-chaining member read: null/undefined → undefined, else like `v[key]`.
fn opt_member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    }
}

fn normalize_optional_id(v: &Value) -> Result<Value, String> {
    // value?.trim(); non-string non-nullish → trim is not a function.
    match v {
        Value::Null => Ok(crate::js::undefined()),
        x if is_undefined(x) => Ok(crate::js::undefined()),
        Value::String(s) => {
            let t = js_trim(s);
            Ok(if t.is_empty() {
                crate::js::undefined()
            } else {
                Value::String(t.to_string())
            })
        }
        _ => Err("value?.trim is not a function".to_string()),
    }
}

/// Port of `recentViewKey` (`apps/web/src/recentViews.logic.ts:69`).
pub fn recent_view_key(view: serde_json::Value) -> Result<serde_json::Value, String> {
    let kind = member(&view, "kind")?;
    match kind.as_str() {
        Some("thread") => {
            let thread_id = member(&view, "threadId")?;
            let split = member(&view, "splitViewId")?;
            if truthy(&split) {
                Ok(Value::String(format!(
                    "thread:{}:split:{}",
                    js_str(&thread_id),
                    js_str(&split)
                )))
            } else {
                Ok(Value::String(format!("thread:{}", js_str(&thread_id))))
            }
        }
        Some("settings") => {
            let section = member(&view, "section")?;
            if truthy(&section) {
                Ok(Value::String(format!("settings:{}", js_str(&section))))
            } else {
                Ok(Value::String("settings".to_string()))
            }
        }
        Some("plugins") => Ok(Value::String("plugins".to_string())),
        _ => Ok(crate::js::undefined()),
    }
}

/// Port of `deriveCurrentRecentView` (`apps/web/src/recentViews.logic.ts:80`).
pub fn derive_current_recent_view(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let split_view_id = normalize_optional_id(&member(&input, "splitViewId")?)?;
    let pathname = member(&input, "pathname")?;
    match pathname.as_str() {
        Some("/settings") => {
            let section = normalize_optional_id(&member(&input, "settingsSection")?)?;
            let mut m = Map::new();
            m.insert("kind".to_string(), Value::String("settings".to_string()));
            if !is_undefined(&section) {
                m.insert("section".to_string(), section);
            }
            Ok(Value::Object(m))
        }
        Some("/plugins") => {
            let mut m = Map::new();
            m.insert("kind".to_string(), Value::String("plugins".to_string()));
            Ok(Value::Object(m))
        }
        _ => {
            let route_thread_id = member(&input, "routeThreadId")?;
            if !truthy(&route_thread_id) {
                return Ok(Value::Null);
            }
            let active = member(&input, "activeThreadId")?;
            let thread_id = if active.is_null() || is_undefined(&active) {
                route_thread_id
            } else {
                active
            };
            let mut m = Map::new();
            m.insert("kind".to_string(), Value::String("thread".to_string()));
            m.insert("threadId".to_string(), thread_id);
            if !is_undefined(&split_view_id) {
                m.insert("splitViewId".to_string(), split_view_id);
            }
            Ok(Value::Object(m))
        }
    }
}

fn to_slice_limit(limit: &Value, default: i64, len: i64) -> i64 {
    if is_undefined(limit) {
        return default;
    }
    let n = match limit {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => crate::js::as_number(limit).unwrap_or(f64::NAN),
    };
    if n.is_nan() {
        return 0;
    }
    let n = n.trunc() as i64;
    if n < 0 { (len + n).max(0) } else { n.min(len) }
}

/// Port of `upsertRecentView` (`apps/web/src/recentViews.logic.ts:106`).
pub fn upsert_recent_view(
    recent_views: serde_json::Value,
    view: serde_json::Value,
    limit: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let key = recent_view_key(view.clone())?;
    let entries = match &recent_views {
        Value::Null => return Err("Cannot read properties of null (reading 'filter')".to_string()),
        v if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'filter')".to_string());
        }
        Value::Array(a) => a.clone(),
        _ => return Err("recentViews.filter is not a function".to_string()),
    };
    let mut out = vec![view];
    for entry in entries {
        let k = recent_view_key(entry.clone())?;
        if !strict_values(&k, &key) {
            out.push(entry);
        }
    }
    let n = to_slice_limit(&limit, 5, out.len() as i64) as usize;
    out.truncate(n);
    Ok(out)
}

fn strict_values(a: &Value, b: &Value) -> bool {
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

fn set_has(container: &Value, _item: &Value, expr: &str) -> Result<bool, String> {
    // `container.has(item)`: only Set-like values have it. JSON arrays don't,
    // so any non-null value errors like in JS.
    match container {
        Value::Null => Err("Cannot read properties of null (reading 'has')".to_string()),
        v if is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'has')".to_string())
        }
        _ => Err(format!("{expr} is not a function")),
    }
}

fn normalize_available_view(view: &Value, availability: &Value) -> Result<Value, String> {
    let kind = member(view, "kind")?;
    match kind.as_str() {
        Some("settings") | Some("plugins") => return Ok(view.clone()),
        Some("thread") => {}
        _ => return Ok(Value::Null),
    }
    if kind.as_str() == Some("thread") {
        let thread_id = member(view, "threadId")?;
        let available = member(availability, "availableThreadIds")?;
        if !set_has(
            &available,
            &thread_id,
            "availability.availableThreadIds.has",
        )? {
            return Ok(Value::Null);
        }
        let split_id = member(view, "splitViewId")?;
        if truthy(&split_id) {
            let by_split = member(availability, "threadIdsBySplitViewId")?;
            let split_thread_ids = match &by_split {
                Value::Null | Value::Object(_) if by_split.is_null() || is_undefined(&by_split) => {
                    crate::js::undefined()
                }
                _ => {
                    // `.get` is a Map method; non-Map → not a function.
                    return Err(
                        "availability.threadIdsBySplitViewId?.get is not a function".to_string()
                    );
                }
            };
            let split_contains = if is_undefined(&split_thread_ids) {
                true
            } else {
                set_has(&split_thread_ids, &thread_id, "splitThreadIds.has")?
            };
            let split_available = member(availability, "availableSplitViewIds")?;
            let in_set = set_has(
                &split_available,
                &split_id,
                "availability.availableSplitViewIds.has",
            )?;
            if !in_set || !split_contains {
                let mut m = Map::new();
                m.insert("kind".to_string(), Value::String("thread".to_string()));
                m.insert("threadId".to_string(), thread_id);
                return Ok(Value::Object(m));
            }
        }
        return Ok(view.clone());
    }
    Ok(Value::Null)
}

/// Port of `pruneRecentViews` (`apps/web/src/recentViews.logic.ts:116`).
pub fn prune_recent_views(
    recent_views: serde_json::Value,
    availability: serde_json::Value,
    limit: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let entries: Vec<Value> = match &recent_views {
        Value::Array(a) => a.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("recentViews is not iterable".to_string()),
    };
    let limit_n = to_slice_limit(&limit, 5, i64::MAX);
    let mut out: Vec<Value> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for view in entries {
        let normalized = normalize_available_view(&view, &availability)?;
        if normalized.is_null() {
            continue;
        }
        let key = recent_view_key(normalized.clone())?;
        let key_str = js_str(&key);
        if seen.contains(&key_str) {
            continue;
        }
        seen.push(key_str);
        out.push(normalized);
        if out.len() as i64 >= limit_n {
            break;
        }
    }
    Ok(out)
}

/// Port of `resolveRecentViewNavigationIndex`
/// (`apps/web/src/recentViews.logic.ts:174`).
pub fn resolve_recent_view_navigation_index(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err(
            "Cannot destructure property 'recentViews' of 'input' as it is null.".to_string(),
        );
    }
    let recent_views = member(&input, "recentViews")?;
    let len = match &recent_views {
        Value::Array(a) => a.len(),
        Value::String(s) => s.chars().count(),
        Value::Null => return Err("Cannot read properties of null (reading 'length')".to_string()),
        v if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'length')".to_string());
        }
        _ => 0,
    };
    if len < 2 {
        return Ok(Value::Null);
    }
    let direction = member(&input, "direction")?;
    let delta: i64 = if direction.as_str() == Some("next") {
        1
    } else {
        -1
    };
    let current_view = member(&input, "currentView")?;
    let selected_key = member(&input, "selectedKey")?;
    let preferred_key = if !selected_key.is_null() && !is_undefined(&selected_key) {
        selected_key
    } else if !current_view.is_null() && !is_undefined(&current_view) {
        recent_view_key(current_view)?
    } else {
        Value::Null
    };
    let preferred_index: i64 = if preferred_key.is_null() {
        -1
    } else {
        let mut idx = -1i64;
        if let Value::Array(a) = &recent_views {
            for (i, v) in a.iter().enumerate() {
                let k = recent_view_key(v.clone())?;
                if strict_values(&k, &preferred_key) {
                    idx = i as i64;
                    break;
                }
            }
        }
        idx
    };
    let start = if preferred_index >= 0 {
        preferred_index
    } else {
        0
    };
    let len = len as i64;
    Ok(Value::from(((start + delta) % len + len) % len))
}

/// Port of `buildRecentViewDisplayEntries`
/// (`apps/web/src/recentViews.logic.ts:197`).
pub fn build_recent_view_display_entries(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let current_view = member(&input, "currentView")?;
    let current_key = if !current_view.is_null() && !is_undefined(&current_view) {
        recent_view_key(current_view)?
    } else {
        Value::Null
    };
    let projects = member(&input, "projects")?;
    let project_list: Vec<Value> = match &projects {
        Value::Null => return Err("Cannot read properties of null (reading 'map')".to_string()),
        v if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'map')".to_string());
        }
        Value::Array(a) => a.clone(),
        _ => return Err("input.projects.map is not a function".to_string()),
    };
    let mut project_names: Vec<(String, Value)> = Vec::new();
    for p in &project_list {
        let id = opt_member(p, "id");
        let name = opt_member(p, "name");
        project_names.push((js_str(&id), name));
    }
    let pinned_thread_ids = member(&input, "pinnedThreadIds")?;
    let pinned_list: Vec<String> = match &pinned_thread_ids {
        Value::Array(a) => a.iter().map(js_str).collect(),
        Value::Null => return Err("input.pinnedThreadIds is not iterable".to_string()),
        v if is_undefined(v) => return Err("input.pinnedThreadIds is not iterable".to_string()),
        Value::String(s) => s.chars().map(|c| c.to_string()).collect(),
        _ => return Err("input.pinnedThreadIds is not iterable".to_string()),
    };
    let threads_by_id = member(&input, "threadsById")?;
    let draft_by_id = member(&input, "draftThreadsById")?;
    let terminal_map = member(&input, "terminalVisualIdentityByThreadId")?;
    let recent_views = member(&input, "recentViews")?;
    let views: Vec<Value> = match &recent_views {
        Value::Null => return Err("Cannot read properties of null (reading 'map')".to_string()),
        v if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'map')".to_string());
        }
        Value::Array(a) => a.clone(),
        _ => return Err("input.recentViews.map is not a function".to_string()),
    };
    let settings_labels = [
        ("general", "General"),
        ("appearance", "Appearance"),
        ("providers", "Providers"),
        ("integrations", "Integrations"),
        ("keybindings", "Keybindings"),
    ];
    let mut out: Vec<Value> = Vec::new();
    for view in views {
        let key = recent_view_key(view.clone())?;
        let kind = member(&view, "kind")?;
        let terminal_identity = if kind.as_str() == Some("thread") {
            // `terminalVisualIdentityByThreadId?.get(...)`: only Maps have get.
            match &terminal_map {
                Value::Null => crate::js::undefined(),
                v if is_undefined(v) => crate::js::undefined(),
                _ => {
                    return Err(
                        "input.terminalVisualIdentityByThreadId?.get is not a function".to_string(),
                    );
                }
            }
        } else {
            Value::Null
        };
        let split_id = member(&view, "splitViewId")?;
        let is_split = kind.as_str() == Some("thread") && truthy(&split_id);
        let is_terminal = truthy(&terminal_identity);
        let mut base = Map::new();
        base.insert("key".to_string(), key.clone());
        base.insert("view".to_string(), view.clone());
        base.insert("kind".to_string(), kind.clone());
        base.insert(
            "isCurrent".to_string(),
            Value::Bool(strict_values(&key, &current_key)),
        );
        base.insert("isPinned".to_string(), Value::Bool(false));
        base.insert("isSplit".to_string(), Value::Bool(is_split));
        base.insert("isTerminal".to_string(), Value::Bool(is_terminal));
        if !is_undefined(&terminal_identity) && !terminal_identity.is_null() {
            base.insert(
                "terminalVisualIdentity".to_string(),
                terminal_identity.clone(),
            );
        }
        match kind.as_str() {
            Some("thread") => {
                let thread_id = member(&view, "threadId")?;
                let tid = js_str(&thread_id);
                let summary = match &threads_by_id {
                    Value::Object(m) => m.get(&tid).cloned().unwrap_or(Value::Null),
                    _ => Value::Null,
                };
                let draft = match &draft_by_id {
                    Value::Object(m) => m.get(&tid).cloned().unwrap_or(Value::Null),
                    _ => Value::Null,
                };
                let thread = if is_undefined(&summary) || summary.is_null() {
                    draft
                } else {
                    summary.clone()
                };
                let project_name = {
                    let pid = opt_member(&thread, "projectId");
                    if is_undefined(&pid) || pid.is_null() {
                        crate::js::undefined()
                    } else {
                        let pid_str = js_str(&pid);
                        project_names
                            .iter()
                            .find(|(k, _)| *k == pid_str)
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(crate::js::undefined)
                    }
                };
                let provider = opt_member(&opt_member(&summary, "modelSelection"), "provider");
                let title = match normalize_optional_id(&opt_member(&thread, "title"))? {
                    v if is_undefined(&v) => Value::String("New chat".to_string()),
                    v => v,
                };
                let mut subtitle_parts: Vec<String> = Vec::new();
                subtitle_parts.push(match &project_name {
                    v if is_undefined(v) || v.is_null() => "Chat".to_string(),
                    v => js_str(v),
                });
                subtitle_parts.push(if is_terminal {
                    "Terminal".to_string()
                } else {
                    "Chat".to_string()
                });
                if is_split {
                    subtitle_parts.push("Split".to_string());
                }
                let is_pinned =
                    pinned_list.contains(&tid) || truthy(&opt_member(&thread, "isPinned"));
                let icon = if is_terminal {
                    let mut m = Map::new();
                    m.insert("kind".to_string(), Value::String("terminal".to_string()));
                    m.insert(
                        "iconKey".to_string(),
                        opt_member(&terminal_identity, "iconKey"),
                    );
                    Value::Object(m)
                } else if truthy(&provider) {
                    let mut m = Map::new();
                    m.insert("kind".to_string(), Value::String("provider".to_string()));
                    m.insert("provider".to_string(), provider.clone());
                    Value::Object(m)
                } else {
                    let mut m = Map::new();
                    m.insert("kind".to_string(), Value::String("chat".to_string()));
                    Value::Object(m)
                };
                base.insert("icon".to_string(), icon);
                base.insert("provider".to_string(), provider);
                base.insert("title".to_string(), title);
                base.insert(
                    "subtitle".to_string(),
                    Value::String(subtitle_parts.join(" · ")),
                );
                base.insert("isPinned".to_string(), Value::Bool(is_pinned));
            }
            Some("settings") => {
                let mut m = Map::new();
                m.insert("kind".to_string(), Value::String("settings".to_string()));
                base.insert("icon".to_string(), Value::Object(m));
                base.insert("title".to_string(), Value::String("Settings".to_string()));
                let section = member(&view, "section")?;
                let subtitle = if truthy(&section) {
                    let s = js_str(&section);
                    settings_labels
                        .iter()
                        .find(|(k, _)| *k == s)
                        .map(|(_, v)| v.to_string())
                        .unwrap_or(s)
                } else {
                    "App settings".to_string()
                };
                base.insert("subtitle".to_string(), Value::String(subtitle));
            }
            Some("plugins") => {
                let mut m = Map::new();
                m.insert("kind".to_string(), Value::String("plugins".to_string()));
                base.insert("icon".to_string(), Value::Object(m));
                base.insert("title".to_string(), Value::String("Plugins".to_string()));
                base.insert(
                    "subtitle".to_string(),
                    Value::String("Extensions and integrations".to_string()),
                );
            }
            _ => {}
        }
        out.push(Value::Object(base));
    }
    Ok(Value::Array(out))
}
