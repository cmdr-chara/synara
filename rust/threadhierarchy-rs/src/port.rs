//! The Rust port of `packages/shared/src/threadHierarchy.ts`.
//!
//! Reconstructed clean-room from cases.jsonl (202 cases, 96% line coverage) and the
//! exported signature: breadth-first collection of every thread reachable through
//! `parentThreadId` links from `rootThreadId`, deduplicated and cycle-safe via a
//! seen-set of thread ids.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::VecDeque;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

/// JS `===` for the JSON surface: `undefined === undefined`, `NaN !== NaN`.
fn js_eq(a: &Value, b: &Value) -> bool {
    if is_undefined(a) && is_undefined(b) {
        return true;
    }
    if is_undefined(a) || is_undefined(b) {
        return false;
    }
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().map(|v| !v.is_nan()) == Some(true) && x.as_f64() == y.as_f64()
        }
        _ => a == b,
    }
}

/// Object property read; JS `undefined` for missing keys and non-objects.
fn field<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) if !is_undefined(v) => m.get(name),
        _ => None,
    }
}

/// Port of `collectSubagentDescendants` (`packages/shared/src/threadHierarchy.ts:13`).
pub fn collect_subagent_descendants(
    threads: serde_json::Value,
    root_thread_id: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    // for..of accepts any iterable: arrays yield elements, strings yield chars
    // (none can hold a matching `parentThreadId`, so the result is empty).
    let items: Vec<Value> = match &threads {
        Value::Array(items) => items.clone(),
        Value::String(_) => Vec::new(),
        _ => return Err("threads is not iterable".into()),
    };
    let items = &items;

    let has_parent = |t: &Value, id: &Value| {
        field(t, "parentThreadId").is_some_and(|p| !p.is_null() && !is_undefined(p) && js_eq(p, id))
    };

    let mut seen: Vec<Value> = vec![root_thread_id.clone()];
    let mut queue: VecDeque<&Value> = items
        .iter()
        .filter(|t| has_parent(t, &root_thread_id))
        .collect();
    let mut out: Vec<Value> = Vec::new();

    while let Some(t) = queue.pop_front() {
        let id = field(t, "id").cloned().unwrap_or(Value::Null);
        let id = if is_undefined(&id) { Value::Null } else { id };
        if seen.iter().any(|s| js_eq(s, &id)) {
            continue;
        }
        seen.push(id.clone());
        out.push(t.clone());
        for child in items.iter().filter(|c| has_parent(c, &id)) {
            queue.push_back(child);
        }
    }

    Ok(out)
}
