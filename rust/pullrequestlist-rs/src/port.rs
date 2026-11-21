//! The Rust port of `packages/shared/src/pullRequestList.ts`.
#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn undef() -> Value {
    json!({"$undefined": true})
}

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
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        _ => match crate::js::as_number(v) {
            Some(x) => x != 0.0 && !x.is_nan(),
            None => true,
        },
    }
}

fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or_else(undef),
        _ => undef(),
    }
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if a.is_null() && b.is_null() {
        return true;
    }
    if crate::js::is_undefined(a) && crate::js::is_undefined(b) {
        return true;
    }
    if is_nullish(a) || is_nullish(b) {
        return false;
    }
    match (a, b) {
        (Value::Number(_), _) | (_, Value::Number(_)) => {
            match (crate::js::as_number(a), crate::js::as_number(b)) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            }
        }
        _ => a == b,
    }
}

const JS_WS: &[char] = &[
    '\t', '\n', '\u{b}', '\x0c', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
    '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
];

fn js_trim(s: &str) -> String {
    s.trim_matches(|c| JS_WS.contains(&c)).to_string()
}

fn js_lowercase(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if is_nullish(i) {
                    String::new()
                } else {
                    js_str(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn js_str_for_concat(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        "undefined".to_string()
    } else {
        js_str(v)
    }
}

fn read_project_contexts_field(entry: &Value) -> Result<Value, String> {
    if entry.is_null() {
        return Err("Cannot read properties of null (reading 'projectContexts')".to_string());
    }
    Ok(prop(entry, "projectContexts"))
}

fn default_context(entry: &Value) -> Value {
    let is_pinned = prop(entry, "isPinned");
    json!({
        "projectId": prop(entry, "projectId"),
        "projectTitle": js_str_for_concat(&prop(entry, "projectTitle")),
        "isPinned": if is_nullish(&is_pinned) { json!(false) } else { is_pinned },
    })
}

fn contexts_of(entry: &Value) -> Result<Value, String> {
    let field = read_project_contexts_field(entry)?;
    if is_nullish(&field) {
        Ok(json!([default_context(entry)]))
    } else {
        Ok(field)
    }
}

fn iter_contexts<'a>(ctxs: &'a Value, method: &str) -> Result<&'a Vec<Value>, String> {
    match ctxs {
        Value::Array(items) => Ok(items),
        _ => Err(format!("entry.projectContexts.{method} is not a function")),
    }
}

/// Port of `pullRequestListRepositoryIdentity` (`packages/shared/src/pullRequestList.ts:14`).
pub fn pull_request_list_repository_identity(entry: serde_json::Value) -> Result<String, String> {
    if entry.is_null() {
        return Err("Cannot read properties of null (reading 'repository')".to_string());
    }
    let repository = prop(&entry, "repository");
    if crate::js::is_undefined(&repository) {
        return Err("Cannot read properties of undefined (reading 'trim')".to_string());
    }
    if repository.is_null() {
        return Err("Cannot read properties of null (reading 'trim')".to_string());
    }
    let repo_str = match &repository {
        Value::String(s) => s.clone(),
        _ => return Err("entry.repository.trim is not a function".to_string()),
    };
    let number = prop(&entry, "number");
    Ok(format!(
        "{}#{}",
        js_lowercase(&js_trim(&repo_str)),
        js_str_for_concat(&number)
    ))
}

/// Port of `pullRequestListProjectContexts` (`packages/shared/src/pullRequestList.ts:21`).
pub fn pull_request_list_project_contexts(
    entry: serde_json::Value,
) -> Result<serde_json::Value, String> {
    contexts_of(&entry)
}

/// Port of `pullRequestListEntryHasProject` (`packages/shared/src/pullRequestList.ts:36`).
pub fn pull_request_list_entry_has_project(
    entry: serde_json::Value,
    project_id: serde_json::Value,
) -> Result<bool, String> {
    let ctxs = contexts_of(&entry)?;
    let items = iter_contexts(&ctxs, "some")?;
    Ok(items
        .iter()
        .any(|c| js_strict_eq(&prop(c, "projectId"), &project_id)))
}

/// Port of `pullRequestListProjectPin` (`packages/shared/src/pullRequestList.ts:43`).
pub fn pull_request_list_project_pin(
    entry: serde_json::Value,
    project_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let ctxs = contexts_of(&entry)?;
    let items = iter_contexts(&ctxs, "find")?;
    for c in items {
        if js_strict_eq(&prop(c, "projectId"), &project_id) {
            let pin = prop(c, "isPinned");
            return Ok(if is_nullish(&pin) { Value::Null } else { pin });
        }
    }
    Ok(Value::Null)
}

/// Port of `coalescePullRequestListEntries` (`packages/shared/src/pullRequestList.ts:103`).
pub fn coalesce_pull_request_list_entries(
    entries: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let items: Vec<Value> = match &entries {
        Value::Array(items) => items.clone(),
        Value::String(s) => s.chars().map(|c| json!(c.to_string())).collect(),
        _ => return Err("entries is not iterable".to_string()),
    };
    let preferred = options
        .as_ref()
        .and_then(|o| o.get("preferredProjectId"))
        .cloned()
        .unwrap_or_else(undef);

    let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
    for entry in items {
        let key = pull_request_list_repository_identity(entry.clone())?;
        if let Some((_, group)) = groups.iter_mut().find(|(k, _)| *k == key) {
            group.push(entry);
        } else {
            groups.push((key, vec![entry]));
        }
    }

    let mut out = Vec::new();
    for (_, group) in groups {
        let canonical_idx = group
            .iter()
            .position(|e| js_strict_eq(&prop(e, "projectTitle"), &prop(e, "headBranch")))
            .unwrap_or(0);
        let rep_idx = if is_nullish(&preferred) {
            canonical_idx
        } else {
            group
                .iter()
                .position(|e| js_strict_eq(&prop(e, "projectId"), &preferred))
                .unwrap_or(canonical_idx)
        };
        let mut merged_contexts: Vec<Value> = Vec::new();
        for (i, e) in group.iter().enumerate() {
            if i == canonical_idx {
                let ctxs = contexts_of(e)?;
                if let Value::Array(list) = ctxs {
                    merged_contexts.extend(list);
                }
            }
        }
        for (i, e) in group.iter().enumerate() {
            if i != canonical_idx {
                let ctxs = contexts_of(e)?;
                if let Value::Array(list) = ctxs {
                    merged_contexts.extend(list);
                }
            }
        }
        let mut rep = group[rep_idx].clone();
        let is_pinned = group.iter().any(|e| truthy(&prop(e, "isPinned")));
        if let Value::Object(m) = &mut rep {
            m.insert("isPinned".to_string(), json!(is_pinned));
            m.insert("projectContexts".to_string(), json!(merged_contexts));
        }
        out.push(rep);
    }
    Ok(out)
}

/// Port of `updatePullRequestListEntryProjectPin` (`packages/shared/src/pullRequestList.ts:133`).
pub fn update_pull_request_list_entry_project_pin(
    entry: serde_json::Value,
    project_id: serde_json::Value,
    is_pinned: bool,
) -> Result<serde_json::Value, String> {
    let field = read_project_contexts_field(&entry)?;
    if is_nullish(&field) {
        return Ok(entry);
    }
    let items = match &field {
        Value::Array(items) => items.clone(),
        _ => return Err("entry.projectContexts.map is not a function".to_string()),
    };
    let mut matched = false;
    let new_ctxs: Vec<Value> = items
        .iter()
        .map(|c| {
            if js_strict_eq(&prop(c, "projectId"), &project_id) {
                matched = true;
                let mut nc = c.clone();
                if let Value::Object(m) = &mut nc {
                    m.insert("isPinned".to_string(), json!(is_pinned));
                }
                nc
            } else {
                c.clone()
            }
        })
        .collect();
    let mut out = entry.clone();
    if let Value::Object(m) = &mut out {
        if matched {
            m.insert(
                "isPinned".to_string(),
                json!(new_ctxs.iter().any(|c| truthy(&prop(c, "isPinned")))),
            );
        }
        m.insert("projectContexts".to_string(), json!(new_ctxs));
    }
    Ok(out)
}
