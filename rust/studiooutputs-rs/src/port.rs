//! The Rust port of `apps/server/src/studioOutputs.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::sync::LazyLock;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

static UNDEF: LazyLock<Value> = LazyLock::new(crate::js::undefined);

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

fn is_record(v: &Value) -> bool {
    matches!(v, Value::Object(_)) && !is_undefined(v)
}

fn js_falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|x| x == 0.0 || x.is_nan()),
        Value::String(s) => s.is_empty(),
        Value::Object(_) => is_undefined(v),
        Value::Array(_) => false,
    }
}

/// `for...of` iterability: arrays, strings (chars) and `{$map:...}` iterate.
fn is_iterable(v: &Value) -> bool {
    matches!(v, Value::Array(_) | Value::String(_))
        || matches!(v, Value::Object(m) if m.contains_key("$map"))
}

const EXCLUDED_TOP_LEVEL_DIRECTORY_NAMES: &[&str] = &["tmp", "logs", "inbox", "context", "skills"];
const EXCLUDED_ROOT_FILE_NAMES: &[&str] = &["agents.md", "claude.md"];
const EXCLUDED_NESTED_DIRECTORY_NAMES: &[&str] = &["node_modules"];
const ACTIVITY_PATH_KEYS: &[&str] = &["path", "filePath", "file_path"];
const FAILED_FILE_CHANGE_STATUSES: &[&str] = &["failed", "declined"];
const MAX_THREAD_OUTPUT_FILES: usize = 500;

/// Port of `isStudioOutputRelativePath` (`apps/server/src/studioOutputs.ts:79`).
pub fn is_studio_output_relative_path(relative_path: String) -> Result<bool, String> {
    let segments: Vec<&str> = relative_path.split('/').collect();
    let Some(first) = segments.first() else {
        return Ok(false);
    };
    if segments.len() == 1 && EXCLUDED_ROOT_FILE_NAMES.contains(&first.to_lowercase().as_str()) {
        return Ok(false);
    }
    if segments.len() > 1
        && EXCLUDED_TOP_LEVEL_DIRECTORY_NAMES.contains(&first.to_lowercase().as_str())
    {
        return Ok(false);
    }
    Ok(segments.iter().all(|segment| {
        !segment.is_empty()
            && *segment != "."
            && *segment != ".."
            && !segment.starts_with('.')
            && !EXCLUDED_NESTED_DIRECTORY_NAMES.contains(&segment.to_lowercase().as_str())
    }))
}

fn collect_activity_path_values(value: &Value, paths: &mut Vec<String>, depth: usize) {
    if depth > 6 || value.is_null() || is_undefined(value) {
        return;
    }
    if let Value::Array(items) = value {
        for item in items {
            collect_activity_path_values(item, paths, depth + 1);
        }
        return;
    }
    if !is_record(value) {
        return;
    }
    if let Value::Object(map) = value {
        for (key, child) in map {
            if ACTIVITY_PATH_KEYS.contains(&key.as_str()) {
                if let Value::String(s) = child {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        paths.push(trimmed.to_string());
                    }
                    continue;
                }
            }
            collect_activity_path_values(child, paths, depth + 1);
        }
    }
}

/// Port of `collectFileChangeActivityPathCandidates` (`apps/server/src/studioOutputs.ts:121`).
pub fn collect_file_change_activity_path_candidates(
    payloads_newest_first: serde_json::Value,
) -> Result<Vec<String>, String> {
    if !is_iterable(&payloads_newest_first) {
        return Err("payloadsNewestFirst is not iterable".to_string());
    }
    let payloads: Vec<Value> = match &payloads_newest_first {
        Value::String(s) => s.chars().map(|c| json!(c.to_string())).collect(),
        _ => payloads_newest_first
            .as_array()
            .cloned()
            .unwrap_or_default(),
    };
    let mut paths: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for payload in &payloads {
        if !is_record(payload) {
            continue;
        }
        if let Value::String(status) = member(payload, "status") {
            if FAILED_FILE_CHANGE_STATUSES.contains(&status.to_lowercase().as_str()) {
                continue;
            }
        }
        let mut candidates = Vec::new();
        collect_activity_path_values(member(payload, "data"), &mut candidates, 0);
        for candidate in candidates {
            if seen.insert(candidate.clone()) {
                paths.push(candidate);
            }
        }
    }
    Ok(paths)
}

/// Port of `collectThreadOutputRelativePaths` (`apps/server/src/studioOutputs.ts:155`).
pub fn collect_thread_output_relative_paths(
    checkpoints: serde_json::Value,
) -> Result<Vec<String>, String> {
    if checkpoints.is_null() {
        return Err("Cannot read properties of null (reading 'length')".to_string());
    }
    if is_undefined(&checkpoints) {
        return Err("Cannot read properties of undefined (reading 'length')".to_string());
    }
    // `checkpoints.length` — arrays and strings have one; anything else is NaN → skip loop.
    let len = match &checkpoints {
        Value::Array(a) => Some(a.len()),
        Value::String(s) => Some(s.chars().count()),
        _ => None,
    };
    let mut relative_paths: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let Some(len) = len else {
        return Ok(relative_paths);
    };
    for index in (0..len).rev() {
        let checkpoint = match &checkpoints {
            Value::Array(a) => &a[index],
            // Strings iterate by char in JS when indexed.
            Value::String(s) => {
                let c = s
                    .chars()
                    .nth(index)
                    .map(|c| c.to_string())
                    .unwrap_or_default();
                // char.checkpoint.files → undefined → not iterable
                let _ = c;
                return Err("checkpoint.files is not iterable".to_string());
            }
            _ => &UNDEF,
        };
        if js_falsy(checkpoint) {
            continue;
        }
        let files = member(checkpoint, "files");
        if !is_iterable(files) {
            return Err("checkpoint.files is not iterable".to_string());
        }
        let file_list: Vec<Value> = match files {
            Value::Array(a) => a.clone(),
            Value::String(s) => s.chars().map(|c| json!(c.to_string())).collect(),
            _ => Vec::new(),
        };
        for file in &file_list {
            if relative_paths.len() >= MAX_THREAD_OUTPUT_FILES {
                return Ok(relative_paths);
            }
            if file.is_null() {
                return Err("Cannot read properties of null (reading 'path')".to_string());
            }
            if is_undefined(file) {
                return Err("Cannot read properties of undefined (reading 'path')".to_string());
            }
            let path = member(file, "path");
            let Value::String(path) = path else {
                return Err(if path.is_null() {
                    "Cannot read properties of null (reading 'split')".to_string()
                } else if is_undefined(path) {
                    "Cannot read properties of undefined (reading 'split')".to_string()
                } else {
                    "relativePath.split is not a function".to_string()
                });
            };
            if is_studio_output_relative_path(path.clone())? && seen.insert(path.clone()) {
                relative_paths.push(path.clone());
            }
        }
    }
    Ok(relative_paths)
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (crate::js::as_number(a), crate::js::as_number(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

fn map_pairs(v: &Value) -> Vec<(Value, Value)> {
    match v {
        Value::Object(m) => match m.get("$map") {
            Some(Value::Array(pairs)) => pairs
                .iter()
                .filter_map(|p| p.as_array().and_then(|kv| kv.first().zip(kv.get(1))))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn js_value_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => "[object Object]".to_string(),
    }
}

/// Port of `diffStudioWorkspaceScans` (`apps/server/src/studioOutputs.ts:306`).
pub fn diff_studio_workspace_scans(
    before: serde_json::Value,
    after: serde_json::Value,
) -> Result<Vec<String>, String> {
    if !is_iterable(&after) {
        return Err("after is not iterable".to_string());
    }
    let before_pairs = map_pairs(&before);
    // `before.get(key)` — only a decoded $map has a `get` method. The member
    // read happens per yielded entry, so an empty `after` never touches it.
    let before_get = |key: &Value| -> Result<Option<&Value>, String> {
        if before.is_null() {
            return Err("Cannot read properties of null (reading 'get')".to_string());
        }
        if is_undefined(&before) {
            return Err("Cannot read properties of undefined (reading 'get')".to_string());
        }
        if !matches!(&before, Value::Object(m) if m.contains_key("$map")) {
            return Err("before.get is not a function".to_string());
        }
        Ok(before_pairs
            .iter()
            .find(|(k, _)| js_strict_eq(k, key))
            .map(|(_, v)| v))
    };
    // Entries `after` yields: $map pairs, array items destructured, string chars.
    let entries: Vec<(Value, Value)> = match &after {
        Value::Object(_) => map_pairs(&after),
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                match item {
                    Value::String(s) => {
                        let mut it = s.chars();
                        let k = it
                            .next()
                            .map(|c| json!(c.to_string()))
                            .unwrap_or(Value::Null);
                        let v = it
                            .next()
                            .map(|c| json!(c.to_string()))
                            .unwrap_or(Value::Null);
                        out.push((k, v));
                    }
                    Value::Array(kv) => {
                        let k = kv.first().cloned().unwrap_or(Value::Null);
                        let v = kv.get(1).cloned().unwrap_or(Value::Null);
                        out.push((k, v));
                    }
                    _ => return Err(format!("{} is not iterable", describe(item))),
                }
            }
            out
        }
        Value::String(s) => s
            .chars()
            .map(|c| (json!(c.to_string()), Value::Null))
            .collect(),
        _ => Vec::new(),
    };
    let mut changed: Vec<String> = Vec::new();
    for (relative_path, stat) in entries {
        let previous = before_get(&relative_path)?;
        let Some(previous) = previous else {
            changed.push(js_value_string(&relative_path));
            continue;
        };
        let p_mtime = member(previous, "mtimeMs");
        let p_size = member(previous, "size");
        let s_mtime = member(&stat, "mtimeMs");
        let s_size = member(&stat, "size");
        if !js_strict_eq(p_mtime, s_mtime) || !js_strict_eq(p_size, s_size) {
            changed.push(js_value_string(&relative_path));
        }
    }
    Ok(changed)
}

fn describe(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) => "".to_string(),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// Port of `studioOutputsCapturedActivityPayload` (`apps/server/src/studioOutputs.ts:325`).
pub fn studio_outputs_captured_activity_payload(
    relative_paths: Vec<String>,
    options: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let files: Vec<Value> = relative_paths
        .iter()
        .map(|path| json!({ "path": path }))
        .collect();
    let mut data = Map::new();
    data.insert("files".to_string(), Value::Array(files));
    if let Some(options) = &options {
        let generated = member(options, "generatedImage");
        if !js_falsy(generated) {
            data.insert("generatedImage".to_string(), generated.clone());
        }
    }
    Ok(json!({
        "itemType": "studio_outputs",
        "data": Value::Object(data),
    }))
}
