//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the JavaScript function would throw. Use `crate::js` helpers for JS coercions
//! (utf16 strings, numbers, undefined) so every exported signature stays
//! (safe JSON inputs) -> Result<safe JSON output, String>.

use serde_json::{Value, json};

/// `options?.maxCount` as a raw value (`None` = absent / undefined).
fn opt_max_count(options: Option<&Value>) -> Option<Value> {
    match options {
        Some(o) => match crate::js::prop(o, "maxCount") {
            Ok(v) => v.cloned(),
            Err(_) => None,
        },
        None => None,
    }
}

/// `Math.max(0, options?.maxCount ?? Infinity)` as the stop limit.
fn normalize_limit(options: Option<&Value>) -> f64 {
    let raw = opt_max_count(options).unwrap_or_else(|| crate::js::number(f64::INFINITY));
    let n = crate::js::to_number(&raw);
    // Math.max propagates NaN.
    if n.is_nan() { f64::NAN } else { n.max(0.0) }
}

fn normalize(ids: &[String], limit: f64) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for id in ids {
        if out.len() as f64 >= limit {
            break;
        }
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        out.push(id.clone());
    }
    out
}

pub fn normalize_pinned_ids(
    ids: Vec<String>,
    options: Option<Value>,
) -> Result<Vec<String>, String> {
    Ok(normalize(&ids, normalize_limit(options.as_ref())))
}

pub fn pin_id(ids: Vec<String>, id: String, options: Option<Value>) -> Result<Value, String> {
    let normalized = normalize(&ids, normalize_limit(options.as_ref()));
    if id.is_empty() {
        return Ok(json!({
            "pinnedIds": normalized,
            "changed": normalized.len() != ids.len(),
            "rejected": true,
        }));
    }
    if normalized.contains(&id) {
        return Ok(json!({
            "pinnedIds": normalized,
            "changed": normalized.len() != ids.len(),
            "rejected": false,
        }));
    }
    // `options?.maxCount !== undefined && normalized.length >= options.maxCount`
    if let Some(mc) = opt_max_count(options.as_ref())
        && (normalized.len() as f64) >= crate::js::to_number(&mc)
    {
        return Ok(json!({
            "pinnedIds": normalized,
            "changed": false,
            "rejected": true,
        }));
    }
    let mut pinned = vec![id];
    pinned.extend(normalized);
    Ok(json!({
        "pinnedIds": pinned,
        "changed": true,
        "rejected": false,
    }))
}

pub fn unpin_id(ids: Vec<String>, id: String) -> Result<Value, String> {
    let normalized = normalize(&ids, f64::INFINITY);
    let pinned: Vec<String> = normalized.iter().filter(|c| **c != id).cloned().collect();
    Ok(json!({
        "pinnedIds": pinned,
        "changed": pinned.len() != normalized.len() || normalized.len() != ids.len(),
        "rejected": false,
    }))
}

pub fn prune_pinned_ids(ids: Vec<String>, allowed_ids: Vec<String>) -> Result<Vec<String>, String> {
    let allowed: std::collections::HashSet<&String> = allowed_ids.iter().collect();
    Ok(normalize(&ids, f64::INFINITY)
        .into_iter()
        .filter(|id| allowed.contains(id))
        .collect())
}

/// `input.optimisticPinnedStateById.get(id)` — a value that crossed the wire
/// is never a Map, so every reachable `.get` throws.
fn optimistic_get(map: &Value, _id: &Value) -> Result<Value, String> {
    if map.is_null() {
        return Err("Cannot read properties of null (reading 'get')".into());
    }
    if crate::js::is_undefined(map) {
        return Err("Cannot read properties of undefined (reading 'get')".into());
    }
    Err("input.optimisticPinnedStateById.get is not a function".into())
}

/// `value === false` on a wire value.
fn is_false(v: &Value) -> bool {
    matches!(v, Value::Bool(false))
}

/// `value === true` on a wire value.
fn is_true(v: &Value) -> bool {
    matches!(v, Value::Bool(true))
}

pub fn derive_pinned_ids(input: Value) -> Result<Vec<String>, String> {
    // `new Set(input.items.map(item => item.id))`
    let items = match crate::js::prop(&input, "items")? {
        None => return Err("Cannot read properties of undefined (reading 'map')".into()),
        Some(Value::Null) => return Err("Cannot read properties of null (reading 'map')".into()),
        Some(v) => v,
    };
    let items = match items.as_array() {
        Some(a) => a.clone(),
        None => return Err("items.map is not a function".into()),
    };
    let mut item_ids: Vec<Value> = Vec::new();
    for item in &items {
        item_ids.push(match item {
            Value::Null => return Err("Cannot read properties of null (reading 'id')".into()),
            v => v
                .as_object()
                .and_then(|o| o.get("id"))
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        });
    }

    let optimistic = match crate::js::prop(&input, "optimisticPinnedStateById")? {
        Some(v) => v.clone(),
        None => crate::js::undefined(),
    };
    let max_count = crate::js::prop(&input, "maxCount")?.cloned();

    let mut pinned_ids: Vec<Value> = Vec::new();
    let add_pinned = |id: &Value, pinned_ids: &mut Vec<Value>| -> bool {
        // `input.maxCount !== undefined && pinnedIds.length >= input.maxCount`
        if let Some(mc) = &max_count
            && (pinned_ids.len() as f64) >= crate::js::to_number(mc)
        {
            return false;
        }
        if item_ids.iter().any(|i| crate::js::svz_eq(i, id))
            && !pinned_ids.iter().any(|p| crate::js::svz_eq(p, id))
        {
            pinned_ids.push(id.clone());
        }
        true
    };

    // `for (const id of input.persistedPinnedIds)`
    let persisted = match crate::js::prop(&input, "persistedPinnedIds")? {
        Some(v) => v.clone(),
        None => crate::js::undefined(),
    };
    for id in crate::js::iterate(&persisted, "input.persistedPinnedIds")? {
        if is_false(&optimistic_get(&optimistic, &id)?) {
            continue;
        }
        add_pinned(&id, &mut pinned_ids);
    }

    // `for (const item of input.items)`
    for item in &items {
        let item_id = item
            .as_object()
            .and_then(|o| o.get("id"))
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        let optimistic_pinned = optimistic_get(&optimistic, &item_id)?;
        let pinned = if crate::js::is_undefined(&optimistic_pinned) || optimistic_pinned.is_null() {
            match item {
                Value::Null => {
                    return Err("Cannot read properties of null (reading 'isPinned')".into());
                }
                v => is_true(
                    v.as_object()
                        .and_then(|o| o.get("isPinned"))
                        .unwrap_or(&Value::Null),
                ),
            }
        } else {
            matches!(optimistic_pinned, Value::Bool(true))
        };
        if pinned {
            add_pinned(&item_id, &mut pinned_ids);
        }
    }

    Ok(pinned_ids
        .iter()
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => crate::js::to_json(other).unwrap_or_default(),
        })
        .collect())
}

/// `new Map(items.map(i => [i.id, i])).get(id)` under SameValueZero.
fn item_for_id<'a>(items: &'a [Value], id: &str) -> Option<&'a Value> {
    // Later sets win in Map construction, so scan in reverse.
    items.iter().rev().find(|item| {
        crate::js::svz_eq(
            item.as_object()
                .and_then(|o| o.get("id"))
                .unwrap_or(&Value::Null),
            &Value::String(id.to_string()),
        )
    })
}

pub fn get_pinned_items(items: Vec<Value>, pinned_ids: Vec<String>) -> Result<Vec<Value>, String> {
    let mut pinned = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in &pinned_ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(item) = item_for_id(&items, id) {
            pinned.push(item.clone());
        }
    }
    Ok(pinned)
}

pub fn order_pinned_items_first(
    items: Vec<Value>,
    pinned_ids: Vec<String>,
) -> Result<Vec<Value>, String> {
    if pinned_ids.is_empty() {
        return Ok(items);
    }
    let pinned = get_pinned_items(items.clone(), pinned_ids.clone())?;
    let id_set: std::collections::HashSet<String> = pinned_ids.into_iter().collect();
    let rest = items.into_iter().filter(|item| {
        !matches!(
            item.as_object().and_then(|o| o.get("id")),
            Some(Value::String(s)) if id_set.contains(s)
        )
    });
    Ok(pinned.into_iter().chain(rest).collect())
}
