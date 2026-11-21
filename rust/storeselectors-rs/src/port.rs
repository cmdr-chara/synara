//! The Rust port of `apps/web/src/storeSelectors.ts`
//! (`collectSnoozedThreadIds`, `isSidebarThreadVisible`).

#![forbid(unsafe_code)]

use serde_json::Value;

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
    v.as_object().and_then(|o| o.get(key))
}

/// `x != null` in JS — false for null and undefined, true for every other value
/// (including `0`, `false`, `""`, `[]`).
fn not_nullish(v: Option<&Value>) -> bool {
    v.is_some_and(|v| !is_nullish(v))
}

/// `collectSubagentDescendants(threads, rootThreadId)` from
/// `packages/shared/src/threadHierarchy.ts`: breadth-first through
/// `parentThreadId` links, visited-tracking against cyclic linkage.
fn collect_descendant_ids(threads: &[Value], root: &Value) -> Vec<Value> {
    use std::collections::{HashMap, HashSet, VecDeque};
    let mut children: HashMap<String, Vec<&Value>> = HashMap::new();
    for thread in threads {
        let parent = get(thread, "parentThreadId");
        if let Some(p) = parent.and_then(|p| p.as_str()) {
            children.entry(p.to_string()).or_default().push(thread);
        }
    }
    let mut out: Vec<Value> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    if let Some(r) = root.as_str() {
        visited.insert(r.to_string());
    }
    let mut queue: VecDeque<&Value> = VecDeque::from([root]);
    while let Some(parent) = queue.pop_front() {
        let Some(pid) = parent.as_str() else { continue };
        let Some(kids) = children.get(pid) else {
            continue;
        };
        for child in kids {
            let Some(id) = get(child, "id") else {
                continue;
            };
            let key = id.as_str().unwrap_or("").to_string();
            if !visited.insert(key) {
                continue;
            }
            out.push((*child).clone());
            queue.push_back(id);
        }
    }
    out
}

/// Port of `collectSnoozedThreadIds` (`apps/web/src/storeSelectors.ts:550`). A
/// snoozed task also hides its subagent subtree; the recorder serializes the
/// returned `ReadonlySet` as `{"$set": [...]}`.
pub fn collect_snoozed_thread_ids(threads: serde_json::Value) -> Result<serde_json::Value, String> {
    if threads.is_null() {
        return Err("null is not an object (evaluating 'threads.filter')".to_string());
    }
    let items = match &threads {
        Value::Array(items) => items,
        _ => {
            return Err("threads.filter is not a function. (In 'threads.filter((thread) => thread.snoozedUntil != null)', 'threads.filter' is undefined)".to_string());
        }
    };
    let mut snoozed: Vec<&Value> = Vec::new();
    for thread in items {
        if thread.is_null() {
            return Err("null is not an object (evaluating 'thread.snoozedUntil')".to_string());
        }
        if crate::js::is_undefined(thread) {
            return Err(
                "undefined is not an object (evaluating 'thread.snoozedUntil')".to_string(),
            );
        }
        if not_nullish(get(thread, "snoozedUntil")) {
            snoozed.push(thread);
        }
    }
    let mut ids: Vec<Value> = Vec::new();
    for thread in &snoozed {
        ids.push(
            get(thread, "id")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        );
    }
    for thread in &snoozed {
        let root = get(thread, "id")
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        for descendant in collect_descendant_ids(items, &root) {
            ids.push(
                get(&descendant, "id")
                    .cloned()
                    .unwrap_or_else(crate::js::undefined),
            );
        }
    }
    // JS `Set` dedupes by SameValueZero; the recorded corpus only ever yields
    // the empty set, but keep the dedupe for fidelity.
    let mut seen: Vec<String> = Vec::new();
    let mut unique: Vec<Value> = Vec::new();
    for id in ids {
        let key = serde_json::to_string(&id).unwrap_or_default();
        if !seen.contains(&key) {
            seen.push(key);
            unique.push(id);
        }
    }
    Ok(serde_json::json!({"$set": unique}))
}

/// Port of `isSidebarThreadVisible` (`apps/web/src/storeSelectors.ts:568`).
/// `options?.flag` short-circuits on nullish options, so only a null/undefined
/// `thread` can throw — on the first property read inside `isSidechatThread`.
pub fn is_sidebar_thread_visible(
    thread: serde_json::Value,
    options: serde_json::Value,
) -> Result<bool, String> {
    if thread.is_null() {
        return Err(
            "null is not an object (evaluating 'thread.sidechatSourceThreadId')".to_string(),
        );
    }
    if crate::js::is_undefined(&thread) {
        return Err(
            "undefined is not an object (evaluating 'thread.sidechatSourceThreadId')".to_string(),
        );
    }
    // `isSidechatThread`: truthy sidechatSourceThreadId or non-nullish sidechatContext.
    let sidechat_source = get(&thread, "sidechatSourceThreadId");
    let sidechat_context = get(&thread, "sidechatContext");
    if sidechat_source.is_some_and(truthy) || not_nullish(sidechat_context) {
        return Ok(false);
    }
    let opt_flag = |name: &str| get(&options, name).is_some_and(truthy);
    if get(&thread, "parentThreadId").is_some_and(truthy) && !opt_flag("includeSubagents") {
        return Ok(false);
    }
    if not_nullish(get(&thread, "snoozedUntil")) && !opt_flag("includeSnoozed") {
        return Ok(false);
    }
    if !opt_flag("hideAutomationRunThreads") {
        return Ok(true);
    }
    if get(&thread, "isPinned").is_some_and(truthy) {
        return Ok(true);
    }
    Ok(get(&thread, "creationSource").and_then(Value::as_str) != Some("automation_run"))
}
