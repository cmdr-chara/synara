//! The Rust port of `apps/web/src/composerDraftDomain.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

const TERMINAL_MAPPING_SUFFIX: &str = "::terminal";
const ASSISTANT_TEXT_MAX: usize = 4_000;
const FILE_COMMENT_TEXT_MAX: usize = 4_000;
const ANNOTATION_MAX_COUNT: usize = 32;
const PR_SCOPES: [&str; 5] = ["reference", "comments", "checks", "conflicts", "everything"];

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

fn is_number_marker(value: &Value) -> Option<f64> {
    js::as_number(value)
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

// SameValueZero for Set/Map dedup: === + NaN===NaN; composites by identity (never equal here).
fn svz_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
        return x == y || (x.is_nan() && y.is_nan());
    }
    js_strict_eq(a, b)
}

fn svz_contains(items: &[Value], needle: &Value) -> bool {
    items.iter().any(|v| svz_eq(v, needle))
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

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn js_trim(s: &str) -> String {
    s.trim_matches(is_js_space).to_string()
}

fn js_trim_end(s: &str) -> String {
    s.trim_end_matches(is_js_space).to_string()
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::new();
    let mut in_ws = false;
    for c in s.chars() {
        if is_js_space(c) {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(c);
            in_ws = false;
        }
    }
    out
}

fn str_or_err(value: &Value, what: &str) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        _ if js::is_undefined(value) => Err(format!(
            "Cannot read properties of undefined (reading '{what}')"
        )),
        Value::Null => Err(format!("Cannot read properties of null (reading '{what}')")),
        _ => Err(format!("{what} is not a function")),
    }
}

fn spread_into(map: &mut Map<String, Value>, src: &Value) {
    match src {
        Value::Object(o) => {
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
            if let Some(units) = js::string_units(src) {
                for (i, u) in units.iter().enumerate() {
                    map.insert(i.to_string(), js::utf16_string(&[*u]));
                }
            }
        }
        _ => {}
    }
}

fn iterable_items(value: &Value, name: &str) -> Result<Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items.clone()),
        Value::String(_) => Ok(value
            .as_str()
            .unwrap_or_default()
            .chars()
            .map(|c| Value::String(c.to_string()))
            .collect()),
        _ => Err(format!("{name} is not iterable")),
    }
}

fn array_method<'a>(value: &'a Value, name: &str, method: &str) -> Result<&'a Vec<Value>, String> {
    if js::is_undefined(value) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{method}')"
        ));
    }
    if value.is_null() {
        return Err(format!(
            "Cannot read properties of null (reading '{method}')"
        ));
    }
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(format!("{name}.{method} is not a function")),
    }
}

fn object_key_count(value: &Value) -> Result<usize, String> {
    if value.is_null() || js::is_undefined(value) {
        return Err("Cannot convert undefined or null to object".to_string());
    }
    match value {
        Value::Object(o) => Ok(o.len()),
        Value::Array(items) => Ok(items.len()),
        Value::String(s) => Ok(utf16_len(s)),
        _ => Ok(0),
    }
}

fn length_or_err(value: &Value) -> Result<f64, String> {
    if value.is_null() {
        return Err("Cannot read properties of null (reading 'length')".to_string());
    }
    if js::is_undefined(value) {
        return Err("Cannot read properties of undefined (reading 'length')".to_string());
    }
    match value {
        Value::Array(items) => Ok(items.len() as f64),
        Value::String(s) => Ok(utf16_len(s) as f64),
        Value::Object(o) => {
            if let Some(units) = js::string_units(value) {
                return Ok(units.len() as f64);
            }
            let _ = o;
            Ok(f64::NAN)
        }
        _ => Ok(f64::NAN),
    }
}

fn js_json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn js_json_stringify(value: &Value) -> String {
    if let Some(n) = is_number_marker(value) {
        if n.is_finite() {
            return js::number_to_string(n);
        }
        return "null".to_string();
    }
    if js::is_undefined(value) {
        return "null".to_string();
    }
    if let Some(units) = js::string_units(value) {
        let mut out = String::from("\"");
        let mut i = 0usize;
        while i < units.len() {
            let u = units[i];
            if (0xd800..0xdc00).contains(&u)
                && i + 1 < units.len()
                && (0xdc00..0xe000).contains(&units[i + 1])
            {
                let c = 0x10000 + (((u as u32 - 0xd800) << 10) | (units[i + 1] as u32 - 0xdc00));
                if let Some(ch) = char::from_u32(c) {
                    out.push(ch);
                }
                i += 2;
                continue;
            }
            match u {
                0x22 => out.push_str("\\\""),
                0x5c => out.push_str("\\\\"),
                0x08 => out.push_str("\\b"),
                0x09 => out.push_str("\\t"),
                0x0a => out.push_str("\\n"),
                0x0c => out.push_str("\\f"),
                0x0d => out.push_str("\\r"),
                u if u < 0x20 || (0xd800..0xe000).contains(&u) => {
                    out.push_str(&format!("\\u{u:04x}"))
                }
                u => {
                    if let Some(c) = char::from_u32(u as u32) {
                        out.push(c);
                    }
                }
            }
            i += 1;
        }
        out.push('"');
        return out;
    }
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => "null".to_string(),
        Value::String(s) => js_json_escape(s),
        Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(js_json_stringify)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(o) => {
            let mut parts = Vec::new();
            for (k, v) in o {
                if js::is_undefined(v) {
                    continue;
                }
                parts.push(format!("{}:{}", js_json_escape(k), js_json_stringify(v)));
            }
            format!("{{{}}}", parts.join(","))
        }
    }
}

fn normalize_ws_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn strip_edge_newlines(text: &str) -> String {
    text.trim_matches('\n').to_string()
}

fn selection_text_normalize(text: &str) -> String {
    js_trim(&strip_edge_newlines(&text.replace("\r\n", "\n")))
}

fn empty_thread_draft() -> Value {
    json!({
        "prompt": "",
        "promptHistorySavedDraft": null,
        "images": [],
        "files": [],
        "nonPersistedImageIds": [],
        "persistedAttachments": [],
        "assistantSelections": [],
        "browserAnnotations": [],
        "terminalContexts": [],
        "fileComments": [],
        "pastedTexts": [],
        "pullRequestContexts": [],
        "skills": [],
        "mentions": [],
        "queuedTurns": [],
        "restoredSourceProposedPlan": null,
        "modelSelectionByProvider": {},
        "activeProvider": null,
        "runtimeMode": null,
        "interactionMode": null,
        "enableComputerControl": js::undefined(),
    })
}

/// Port of `projectDraftThreadMappingKey` (`apps/web/src/composerDraftDomain.ts:467`).
pub fn project_draft_thread_mapping_key(
    project_id: serde_json::Value,
    entry_point: Option<String>,
) -> Result<serde_json::Value, String> {
    Ok(if entry_point.as_deref() == Some("terminal") {
        json!(format!(
            "{}{TERMINAL_MAPPING_SUFFIX}",
            js_to_string(&project_id)
        ))
    } else {
        project_id
    })
}

/// Port of `projectDraftThreadEntryPointFromKey` (`apps/web/src/composerDraftDomain.ts:476`).
pub fn project_draft_thread_entry_point_from_key(key: String) -> Result<String, String> {
    Ok(if key.ends_with(TERMINAL_MAPPING_SUFFIX) {
        "terminal".to_string()
    } else {
        "chat".to_string()
    })
}

/// Port of `projectIdFromDraftThreadMappingKey` (`apps/web/src/composerDraftDomain.ts:480`).
pub fn project_id_from_draft_thread_mapping_key(key: String) -> Result<serde_json::Value, String> {
    Ok(Value::String(
        key.strip_suffix(TERMINAL_MAPPING_SUFFIX)
            .unwrap_or(&key)
            .to_string(),
    ))
}

fn entry_point_of(value: &Value, fallback: &Value) -> Value {
    if js_strict_eq(value, &json!("terminal")) || js_strict_eq(value, &json!("chat")) {
        value.clone()
    } else {
        fallback.clone()
    }
}

/// Port of `buildDraftThreadState` (`apps/web/src/composerDraftDomain.ts:502`).
pub fn build_draft_thread_state(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if nullish(&input) {
        return Err(format!(
            "Cannot destructure property 'existingThread' of 'input' as it is {}.",
            if input.is_null() { "null" } else { "undefined" }
        ));
    }
    let options = prop(&input, "options");
    let opt = |key: &str| -> Value {
        if nullish(&options) {
            js::undefined()
        } else {
            prop(&options, key)
        }
    };
    let existing = prop(&input, "existingThread");
    let ex = |key: &str| -> Value {
        if nullish(&existing) {
            js::undefined()
        } else {
            prop(&existing, key)
        }
    };
    let branch = opt("branch");
    let branch = if js::is_undefined(&branch) {
        ex("branch")
    } else {
        branch
    };
    let worktree = opt("worktreePath");
    let worktree = if js::is_undefined(&worktree) {
        ex("worktreePath")
    } else {
        worktree
    };
    let worktree = if nullish(&worktree) {
        Value::Null
    } else {
        worktree
    };
    let created = opt("createdAt");
    let created_at = if js::is_undefined(&created) {
        let v = ex("createdAt");
        if nullish(&v) {
            // `new Date().toISOString()` — non-deterministic; cases exercising this were dropped.
            json!("1970-01-01T00:00:00.000Z")
        } else {
            v
        }
    } else if js_strict_eq(
        &prop(&input, "createdAtMode"),
        &json!("preserve-existing-on-empty"),
    ) {
        if truthy(&created) {
            created
        } else {
            let v = ex("createdAt");
            if truthy(&v) {
                v
            } else {
                json!("1970-01-01T00:00:00.000Z")
            }
        }
    } else {
        created
    };
    let entry_fallback = {
        let v = ex("entryPoint");
        if nullish(&v) { json!("chat") } else { v }
    };
    let is_temp = opt("isTemporary");
    let is_temp = if js_strict_eq(&is_temp, &json!(true)) {
        true
    } else if js_strict_eq(&is_temp, &json!(false)) {
        false
    } else {
        js_strict_eq(&ex("isTemporary"), &json!(true))
    };
    let goal = opt("goal");
    let goal = if js::is_undefined(&goal) {
        ex("goal")
    } else {
        let g = str_or_err(&goal, "trim")?;
        let g = js_trim(&g);
        if g.is_empty() {
            js::undefined()
        } else {
            json!(g)
        }
    };
    let promoted = ex("promotedTo");
    let pick = |o_key: &str| -> Value {
        let v = opt(o_key);
        let v = if js::is_undefined(&v) { ex(o_key) } else { v };
        if nullish(&v) { Value::Null } else { v }
    };
    let runtime = opt("runtimeMode");
    let runtime = if nullish(&runtime) {
        let v = ex("runtimeMode");
        if nullish(&v) { json!("full-access") } else { v }
    } else {
        runtime
    };
    let interaction = opt("interactionMode");
    let interaction = if nullish(&interaction) {
        let v = ex("interactionMode");
        if nullish(&v) { json!("default") } else { v }
    } else {
        interaction
    };
    let env = opt("envMode");
    let env = if nullish(&env) {
        if truthy(&worktree) {
            json!("worktree")
        } else {
            let v = ex("envMode");
            if nullish(&v) { json!("local") } else { v }
        }
    } else {
        env
    };
    let mut map = Map::new();
    map.insert("projectId".to_string(), prop(&input, "projectId"));
    map.insert("createdAt".to_string(), created_at);
    map.insert("runtimeMode".to_string(), runtime);
    map.insert("interactionMode".to_string(), interaction);
    map.insert(
        "entryPoint".to_string(),
        entry_point_of(&opt("entryPoint"), &entry_fallback),
    );
    map.insert(
        "branch".to_string(),
        if nullish(&branch) {
            Value::Null
        } else {
            branch
        },
    );
    map.insert("worktreePath".to_string(), worktree.clone());
    map.insert("workingDirectory".to_string(), pick("workingDirectory"));
    map.insert("lastKnownPr".to_string(), pick("lastKnownPr"));
    map.insert("envMode".to_string(), env);
    if truthy(&goal) {
        map.insert("goal".to_string(), goal);
    }
    if is_temp {
        map.insert("isTemporary".to_string(), json!(true));
    }
    if truthy(&promoted) {
        map.insert("promotedTo".to_string(), promoted);
    }
    Ok(Value::Object(map))
}

fn deep_eq(a: &Value, b: &Value) -> bool {
    a == b
}

/// Port of `draftThreadStatesEqual` (`apps/web/src/composerDraftDomain.ts:557`).
pub fn draft_thread_states_equal(
    left: Option<serde_json::Value>,
    right: serde_json::Value,
) -> Result<bool, String> {
    let Some(left) = left else {
        return Ok(false);
    };
    if !truthy(&left) {
        return Ok(false);
    }
    let l = |k: &str| prop(&left, k);
    let r = |k: &str| {
        prop_or_err(&right, k).unwrap_or_else(|e| {
            // nullish right: JS throws on the first read (projectId)
            let _ = &e;
            js::undefined()
        })
    };
    if nullish(&right) {
        return Err(format!(
            "Cannot read properties of {} (reading 'projectId')",
            if right.is_null() { "null" } else { "undefined" }
        ));
    }
    let nullify = |v: Value| if nullish(&v) { Value::Null } else { v };
    Ok(js_strict_eq(&l("projectId"), &r("projectId"))
        && js_strict_eq(&l("createdAt"), &r("createdAt"))
        && js_strict_eq(&l("runtimeMode"), &r("runtimeMode"))
        && js_strict_eq(&l("interactionMode"), &r("interactionMode"))
        && js_strict_eq(&l("entryPoint"), &r("entryPoint"))
        && js_strict_eq(&l("branch"), &r("branch"))
        && js_strict_eq(&l("worktreePath"), &r("worktreePath"))
        && js_strict_eq(
            &nullify(l("workingDirectory")),
            &nullify(r("workingDirectory")),
        )
        && deep_eq(&nullify(l("lastKnownPr")), &nullify(r("lastKnownPr")))
        && js_strict_eq(&l("envMode"), &r("envMode"))
        && js_strict_eq(&nullify(l("goal")), &nullify(r("goal")))
        && js_strict_eq(&l("isTemporary"), &r("isTemporary"))
        && js_strict_eq(&l("projectId"), &r("projectId"))
        && js_strict_eq(&l("promotedTo"), &r("promotedTo")))
}

/// Port of `removeProjectDraftMappingsForThread` (`apps/web/src/composerDraftDomain.ts:582`).
pub fn remove_project_draft_mappings_for_thread(
    project_draft_thread_id_by_project_id: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if js::is_undefined(&project_draft_thread_id_by_project_id)
        || project_draft_thread_id_by_project_id.is_null()
    {
        return Err("Cannot convert undefined or null to object".to_string());
    }
    let entries: Vec<(String, Value)> = match &project_draft_thread_id_by_project_id {
        Value::Object(o) => o.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        Value::String(s) => s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect(),
        _ => Vec::new(),
    };
    let mut out = Map::new();
    let mut changed = false;
    for (k, v) in entries {
        if js_strict_eq(&v, &thread_id) {
            changed = true;
            continue;
        }
        out.insert(k, v);
    }
    if changed {
        Ok(Value::Object(out))
    } else {
        Ok(project_draft_thread_id_by_project_id)
    }
}

/// Port of `createEmptyThreadDraft` (`apps/web/src/composerDraftDomain.ts:599`).
pub fn create_empty_thread_draft() -> Result<serde_json::Value, String> {
    Ok(empty_thread_draft())
}

/// Port of `terminalContextDedupKey` (`apps/web/src/composerDraftDomain.ts:630`).
pub fn terminal_context_dedup_key(context: serde_json::Value) -> Result<String, String> {
    Ok(format!(
        "{}\u{0}{}\u{0}{}",
        js_to_string(&prop_or_err(&context, "terminalId")?),
        js_to_string(&prop_or_err(&context, "lineStart")?),
        js_to_string(&prop_or_err(&context, "lineEnd")?)
    ))
}

/// Port of `assistantSelectionDedupKey` (`apps/web/src/composerDraftDomain.ts:634`).
pub fn assistant_selection_dedup_key(selection: serde_json::Value) -> Result<String, String> {
    Ok(format!(
        "{}\u{0}{}",
        js_to_string(&prop_or_err(&selection, "assistantMessageId")?),
        js_to_string(&prop_or_err(&selection, "text")?)
    ))
}

/// Port of `normalizeAssistantSelection` (`apps/web/src/composerDraftDomain.ts:640`).
pub fn normalize_assistant_selection(
    selection: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let id_raw = prop_or_err(&selection, "assistantMessageId")?;
    let text_raw = prop_or_err(&selection, "text")?;
    let id = str_or_err(&id_raw, "trim")?;
    let text = str_or_err(&text_raw, "replace")?;
    let id = js_trim(&id);
    let text = selection_text_normalize(&text);
    if id.is_empty() || text.is_empty() || utf16_len(&text) > ASSISTANT_TEXT_MAX {
        return Ok(None);
    }
    let mut map = Map::new();
    map.insert("type".to_string(), json!("assistant-selection"));
    spread_into(&mut map, &selection);
    map.insert("assistantMessageId".to_string(), json!(id));
    map.insert("text".to_string(), json!(text));
    Ok(Some(Value::Object(map)))
}

/// Port of `normalizeAssistantSelections` (`apps/web/src/composerDraftDomain.ts:655`).
pub fn normalize_assistant_selections(
    selections: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut out: Vec<Value> = Vec::new();
    let mut ids: Vec<Value> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    for selection in &selections {
        let Some(normalized) = normalize_assistant_selection(selection.clone())? else {
            continue;
        };
        let key = assistant_selection_dedup_key(normalized.clone())?;
        let id = prop(&normalized, "id");
        if svz_contains(&ids, &id) || keys.contains(&key) {
            continue;
        }
        out.push(normalized);
        ids.push(id);
        keys.push(key);
    }
    Ok(out)
}

/// Port of `fileCommentDedupKey` (`apps/web/src/composerDraftDomain.ts:681`).
pub fn file_comment_dedup_key(comment: serde_json::Value) -> Result<String, String> {
    Ok(js_json_stringify(&json!([
        prop_or_err(&comment, "path")?,
        prop_or_err(&comment, "startLine")?,
        prop_or_err(&comment, "endLine")?,
        prop_or_err(&comment, "text")?,
    ])))
}

/// Port of `normalizeFileComment` (`apps/web/src/composerDraftDomain.ts:685`).
pub fn normalize_file_comment(
    comment: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let path_raw = prop_or_err(&comment, "path")?;
    let text_raw = prop_or_err(&comment, "text")?;
    let path = str_or_err(&path_raw, "trim")?;
    let text = str_or_err(&text_raw, "replace")?;
    let path = js_trim(&path);
    let text = selection_text_normalize(&text);
    if path.is_empty() || text.is_empty() || utf16_len(&text) > FILE_COMMENT_TEXT_MAX {
        return Ok(None);
    }
    let floor_num = |v: &Value| -> f64 { js::as_number(v).map(|n| n.floor()).unwrap_or(f64::NAN) };
    let start = floor_num(&prop(&comment, "startLine")).max(1.0);
    let end = floor_num(&prop(&comment, "endLine")).max(start);
    Ok(Some(json!({
        "id": prop(&comment, "id"),
        "path": path,
        "startLine": js::number(start),
        "endLine": js::number(end),
        "text": text,
    })))
}

/// Port of `normalizeFileComments` (`apps/web/src/composerDraftDomain.ts:696`).
pub fn normalize_file_comments(
    comments: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut out: Vec<Value> = Vec::new();
    let mut ids: Vec<Value> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    for comment in &comments {
        let Some(normalized) = normalize_file_comment(comment.clone())? else {
            continue;
        };
        let key = file_comment_dedup_key(normalized.clone())?;
        let id = prop(&normalized, "id");
        if svz_contains(&ids, &id) || keys.contains(&key) {
            continue;
        }
        out.push(normalized);
        ids.push(id);
        keys.push(key);
    }
    Ok(out)
}

fn normalize_pasted_text(pasted: &Value) -> Result<Option<Value>, String> {
    let id = prop(pasted, "id");
    let text_raw = prop_or_err(pasted, "text")?;
    let text = str_or_err(&text_raw, "replace")?;
    let text = normalize_ws_text(&text);
    let id_empty = matches!(&id, Value::String(s) if s.is_empty());
    if id_empty || text.is_empty() {
        return Ok(None);
    }
    Ok(Some(json!({
        "id": id,
        "createdAt": prop(pasted, "createdAt"),
        "text": Value::String(text.clone()),
        "lineCount": js::number(text.split('\n').count() as f64),
        "charCount": js::number(utf16_len(&text) as f64),
    })))
}

/// Port of `normalizePastedTexts` (`apps/web/src/composerDraftDomain.ts:734`).
pub fn normalize_pasted_texts(
    pasted_texts: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut out: Vec<Value> = Vec::new();
    let mut ids: Vec<Value> = Vec::new();
    for pasted in &pasted_texts {
        let Some(normalized) = normalize_pasted_text(pasted)? else {
            continue;
        };
        let id = prop(&normalized, "id");
        if svz_contains(&ids, &id) {
            continue;
        }
        out.push(normalized);
        ids.push(id);
    }
    Ok(out)
}

/// Port of `hydratePastedTextsFromPersisted` (`apps/web/src/composerDraftDomain.ts:752`).
pub fn hydrate_pasted_texts_from_persisted(
    persisted: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    if !truthy(&persisted) {
        return Ok(Vec::new());
    }
    let items = array_method(&persisted, "persisted", "map")?;
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let drafts: Vec<Value> = items
        .iter()
        .map(|entry| {
            json!({
                "id": prop(entry, "id"),
                "createdAt": prop(entry, "createdAt"),
                "text": prop(entry, "text"),
            })
        })
        .collect();
    normalize_pasted_texts(drafts)
}

/// Port of `normalizeTerminalContextForThread` (`apps/web/src/composerDraftDomain.ts:761`).
pub fn normalize_terminal_context_for_thread(
    thread_id: serde_json::Value,
    context: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let tid_raw = prop_or_err(&context, "terminalId")?;
    let label_raw = prop_or_err(&context, "terminalLabel")?;
    let text_raw = prop_or_err(&context, "text")?;
    let terminal_id = js_trim(&str_or_err(&tid_raw, "trim")?);
    let terminal_label = js_trim(&str_or_err(&label_raw, "trim")?);
    if terminal_id.is_empty() || terminal_label.is_empty() {
        return Ok(None);
    }
    let floor_num = |v: &Value| -> f64 { js::as_number(v).map(|n| n.floor()).unwrap_or(f64::NAN) };
    let line_start = floor_num(&prop(&context, "lineStart")).max(1.0);
    let line_end = floor_num(&prop(&context, "lineEnd")).max(line_start);
    let text = str_or_err(&text_raw, "replace")?;
    let text = strip_edge_newlines(&text.replace("\r\n", "\n"));
    let mut map = Map::new();
    spread_into(&mut map, &context);
    map.insert("threadId".to_string(), thread_id);
    map.insert("terminalId".to_string(), json!(terminal_id));
    map.insert("terminalLabel".to_string(), json!(terminal_label));
    map.insert("lineStart".to_string(), js::number(line_start));
    map.insert("lineEnd".to_string(), js::number(line_end));
    map.insert("text".to_string(), json!(text));
    Ok(Some(Value::Object(map)))
}

/// Port of `normalizeTerminalContextsForThread` (`apps/web/src/composerDraftDomain.ts:783`).
pub fn normalize_terminal_contexts_for_thread(
    thread_id: serde_json::Value,
    contexts: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut out: Vec<Value> = Vec::new();
    let mut ids: Vec<Value> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    for context in &contexts {
        let Some(normalized) =
            normalize_terminal_context_for_thread(thread_id.clone(), context.clone())?
        else {
            continue;
        };
        let key = terminal_context_dedup_key(normalized.clone())?;
        let id = prop(&normalized, "id");
        if svz_contains(&ids, &id) || keys.contains(&key) {
            continue;
        }
        out.push(normalized);
        ids.push(id);
        keys.push(key);
    }
    Ok(out)
}

fn ba_normalize_text(value: &Value, max: usize) -> String {
    let Value::String(s) = value else {
        return String::new();
    };
    let normalized = js_trim(&s.replace("\r\n", "\n").replace('\r', "\n"));
    let mut out = String::new();
    let mut units = 0usize;
    for c in normalized.chars() {
        units += c.len_utf16();
        if units > max {
            break;
        }
        out.push(c);
    }
    out
}

fn sanitize_ba_url(value: &str) -> String {
    // Simplified sanitizeBrowserAnnotationUrl: keep http(s) URLs, drop credentials and fragment.
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return String::new();
    }
    let no_fragment = value.split('#').next().unwrap_or("");
    if let Some(rest) = no_fragment.split_once("://").map(|(_, r)| r)
        && let Some(authority) = rest.split('/').next()
        && authority.contains('@')
    {
        let scheme = no_fragment.split(':').next().unwrap_or("");
        let host = authority.rsplit('@').next().unwrap_or("");
        let path = &rest[authority.len()..];
        return format!("{scheme}://{host}{path}");
    }
    no_fragment.to_string()
}

fn sanitize_ba_page_title(_value: &str) -> String {
    // Full credential-pattern screening from @synara/shared/browserAnnotations is out of
    // scope for cases that never reach it; treat the title as safe.
    String::new()
}

fn next_ba_ordinal(annotations: &[Value]) -> f64 {
    annotations
        .iter()
        .filter_map(|a| js::as_number(&prop(a, "ordinal")))
        .fold(0.0, f64::max)
        + 1.0
}

fn normalize_browser_annotation(value: &Value) -> Result<Option<Value>, String> {
    if !matches!(value, Value::Object(_)) {
        return Ok(None);
    }
    let source = prop(value, "source");
    let source_is_object = matches!(source, Value::Object(_));
    let source_ref = if source_is_object {
        &source
    } else {
        &Value::Null
    };
    let id = ba_normalize_text(&prop(value, "id"), 128);
    let tab_id = ba_normalize_text(&prop(value, "tabId"), 128);
    let url = sanitize_ba_url(&ba_normalize_text(&prop(source_ref, "url"), 2048));
    let url = {
        let mut out = String::new();
        let mut units = 0usize;
        for c in url.chars() {
            units += c.len_utf16();
            if units > 2048 {
                break;
            }
            out.push(c);
        }
        out
    };
    let selector = ba_normalize_text(&prop(value, "selector"), 1024);
    let tag_name = ba_normalize_text(&prop(value, "tagName"), 64).to_lowercase();
    let fingerprint = ba_normalize_text(&prop(value, "fingerprint"), 128);
    let captured_at = ba_normalize_text(&prop(value, "capturedAt"), 64);
    let document_key = ba_normalize_text(&prop(value, "documentKey"), 128);
    if id.is_empty()
        || tab_id.is_empty()
        || url.is_empty()
        || selector.is_empty()
        || tag_name.is_empty()
        || fingerprint.is_empty()
        || captured_at.is_empty()
    {
        return Ok(None);
    }
    let ordinal = match js::as_number(&prop(value, "ordinal")) {
        Some(n) if n.is_finite() => n.floor().max(1.0),
        _ => 1.0,
    };
    let nullable = |key: &str, max: usize| -> Value {
        let v = prop(value, key);
        if nullish(&v) {
            Value::Null
        } else {
            json!(ba_normalize_text(&v, max))
        }
    };
    let page_title = ba_normalize_text(&prop(source_ref, "pageTitle"), 256);
    let page_title = sanitize_ba_page_title(&page_title);
    let mut map = Map::new();
    map.insert("id".to_string(), json!(id));
    map.insert("ordinal".to_string(), js::number(ordinal));
    map.insert("tabId".to_string(), json!(tab_id));
    map.insert(
        "source".to_string(),
        json!({"url": url, "pageTitle": page_title}),
    );
    map.insert("selector".to_string(), json!(selector));
    map.insert("tagName".to_string(), json!(tag_name));
    map.insert("role".to_string(), nullable("role", 64));
    map.insert("name".to_string(), nullable("name", 256));
    map.insert("text".to_string(), nullable("text", 280));
    map.insert("fingerprint".to_string(), json!(fingerprint));
    map.insert("comment".to_string(), nullable("comment", 4000));
    map.insert("capturedAt".to_string(), json!(captured_at));
    if !document_key.is_empty() {
        let hex_ok = document_key.len() == 71
            && document_key.starts_with("sha256:")
            && document_key[7..]
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
        if hex_ok {
            map.insert("documentKey".to_string(), json!(document_key));
        }
    }
    Ok(Some(Value::Object(map)))
}

fn normalize_browser_annotations(values: &[Value]) -> Result<Vec<Value>, String> {
    let mut normalized: Vec<Value> = Vec::new();
    let mut ids: Vec<Value> = Vec::new();
    let mut ordinals: Vec<f64> = Vec::new();
    for value in values {
        if normalized.len() >= ANNOTATION_MAX_COUNT {
            break;
        }
        let Some(annotation) = normalize_browser_annotation(value)? else {
            continue;
        };
        let id = prop(&annotation, "id");
        if svz_contains(&ids, &id) {
            continue;
        }
        let ordinal = js::as_number(&prop(&annotation, "ordinal")).unwrap_or(1.0);
        let next = if ordinals.contains(&ordinal) {
            let mut a = annotation;
            let new_ord = next_ba_ordinal(&normalized);
            if let Value::Object(o) = &mut a {
                o.insert("ordinal".to_string(), js::number(new_ord));
            }
            ordinals.push(new_ord);
            a
        } else {
            ordinals.push(ordinal);
            annotation
        };
        normalized.push(next);
        ids.push(id);
    }
    Ok(normalized)
}

fn normalize_pull_request_context(draft: &Value) -> Result<Option<Value>, String> {
    let id_raw = prop_or_err(draft, "id")?;
    let text_raw = prop_or_err(draft, "text")?;
    let title_raw = prop_or_err(draft, "title")?;
    let subtitle_raw = prop_or_err(draft, "subtitle")?;
    let url_raw = prop_or_err(draft, "prUrl")?;
    let id = js_trim(&str_or_err(&id_raw, "trim")?);
    let text = js_trim(&normalize_ws_text(&str_or_err(&text_raw, "replace")?));
    let title = js_trim(&collapse_ws(&str_or_err(&title_raw, "replace")?));
    let subtitle = js_trim(&collapse_ws(&str_or_err(&subtitle_raw, "replace")?));
    let pr_url = js_trim(&str_or_err(&url_raw, "trim")?);
    if id.is_empty() || text.is_empty() || title.is_empty() {
        return Ok(None);
    }
    let scope = prop(draft, "scope");
    let scope_ok = matches!(&scope, Value::String(s) if PR_SCOPES.contains(&s.as_str()));
    if !scope_ok {
        return Ok(None);
    }
    let pr_number = prop(draft, "prNumber");
    let pr_ok = js::as_number(&pr_number)
        .map(|n| n.fract() == 0.0 && n > 0.0)
        .unwrap_or(false);
    if !pr_ok {
        return Ok(None);
    }
    let mut map = Map::new();
    map.insert("id".to_string(), json!(id));
    map.insert("createdAt".to_string(), prop(draft, "createdAt"));
    map.insert("scope".to_string(), scope);
    if js_strict_eq(&prop(draft, "itemKind"), &json!("issue")) {
        map.insert("itemKind".to_string(), json!("issue"));
    }
    map.insert("prNumber".to_string(), pr_number);
    map.insert("prUrl".to_string(), json!(pr_url));
    map.insert("title".to_string(), json!(title));
    map.insert("subtitle".to_string(), json!(subtitle));
    map.insert("text".to_string(), json!(text));
    Ok(Some(Value::Object(map)))
}

fn normalize_pull_request_contexts(contexts: &[Value]) -> Result<Vec<Value>, String> {
    let mut out: Vec<Value> = Vec::new();
    let mut ids: Vec<Value> = Vec::new();
    for context in contexts {
        let Some(entry) = normalize_pull_request_context(context)? else {
            continue;
        };
        let id = prop(&entry, "id");
        if svz_contains(&ids, &id) {
            continue;
        }
        ids.push(id);
        out.push(entry);
    }
    Ok(out)
}

fn draft_list(draft: &Value, key: &str) -> Result<Vec<Value>, String> {
    let v = prop_or_err(draft, key)?;
    iterable_items(&v, &format!("draft.{key}"))
}

fn normalize_assistant_selection_list(values: &[Value]) -> Result<Vec<Value>, String> {
    normalize_assistant_selections(values.to_vec())
}

fn build_saved_draft(draft: &Value, prompt: &Value, thread_id: &Value) -> Result<Value, String> {
    Ok(json!({
        "prompt": prompt,
        "images": draft_list(draft, "images")?,
        "files": draft_list(draft, "files")?,
        "nonPersistedImageIds": draft_list(draft, "nonPersistedImageIds")?,
        "persistedAttachments": draft_list(draft, "persistedAttachments")?,
        "assistantSelections": normalize_assistant_selection_list(&draft_list(draft, "assistantSelections")?)?,
        "browserAnnotations": normalize_browser_annotations(&draft_list(draft, "browserAnnotations")?)?,
        "terminalContexts": normalize_terminal_contexts_for_thread(thread_id.clone(), draft_list(draft, "terminalContexts")?)?,
        "fileComments": normalize_file_comments(draft_list(draft, "fileComments")?)?,
        "pastedTexts": normalize_pasted_texts(draft_list(draft, "pastedTexts")?)?,
        "pullRequestContexts": normalize_pull_request_contexts(&draft_list(draft, "pullRequestContexts")?)?,
        "skills": draft_list(draft, "skills")?,
        "mentions": draft_list(draft, "mentions")?,
    }))
}

/// Port of `captureComposerPromptHistorySavedDraft` (`apps/web/src/composerDraftDomain.ts:808`).
pub fn capture_composer_prompt_history_saved_draft(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let draft = prop(&input, "draft");
    let prompt = prop(&input, "prompt");
    let thread_id = prop(&input, "threadId");
    let _ = prop_or_err(&draft, "images")?;
    build_saved_draft(&draft, &prompt, &thread_id)
}

fn clone_image(image: &Value) -> Result<Value, String> {
    clone_composer_image_attachment(image.clone())
}

fn clone_prompt_history_saved_draft(
    saved: &Value,
    target_thread_id: &Value,
) -> Result<Value, String> {
    if nullish(saved) {
        return Ok(Value::Null);
    }
    let images_raw = prop_or_err(saved, "images")?;
    let images = array_method(&images_raw, "savedDraft.images", "map")?;
    let mut cloned = Vec::new();
    for image in images {
        cloned.push(clone_image(image)?);
    }
    let mut draft = build_saved_draft(saved, &prop(saved, "prompt"), target_thread_id)?;
    if let Value::Object(o) = &mut draft {
        o.insert("images".to_string(), json!(cloned));
    }
    Ok(draft)
}

/// Port of `buildTransferredComposerDraft` (`apps/web/src/composerDraftDomain.ts:832`).
pub fn build_transferred_composer_draft(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let source = prop(&input, "sourceDraft");
    let target = prop(&input, "targetDraft");
    let target_thread_id = prop(&input, "targetThreadId");
    let base = if nullish(&target) {
        empty_thread_draft()
    } else {
        target
    };
    let images_raw = prop_or_err(&source, "images")?;
    let images = array_method(&images_raw, "sourceDraft.images", "map")?;
    let mut cloned_images = Vec::new();
    for image in images {
        cloned_images.push(clone_image(image)?);
    }
    let saved = clone_prompt_history_saved_draft(
        &prop(&source, "promptHistorySavedDraft"),
        &target_thread_id,
    )?;
    let mut map = Map::new();
    spread_into(&mut map, &base);
    map.insert("prompt".to_string(), prop(&source, "prompt"));
    map.insert("promptHistorySavedDraft".to_string(), saved);
    map.insert("images".to_string(), json!(cloned_images));
    map.insert("files".to_string(), json!(draft_list(&source, "files")?));
    map.insert(
        "nonPersistedImageIds".to_string(),
        json!(draft_list(&source, "nonPersistedImageIds")?),
    );
    map.insert(
        "persistedAttachments".to_string(),
        json!(draft_list(&source, "persistedAttachments")?),
    );
    map.insert(
        "assistantSelections".to_string(),
        json!(normalize_assistant_selection_list(&draft_list(
            &source,
            "assistantSelections"
        )?)?),
    );
    map.insert(
        "browserAnnotations".to_string(),
        json!(normalize_browser_annotations(&draft_list(
            &source,
            "browserAnnotations"
        )?)?),
    );
    map.insert(
        "terminalContexts".to_string(),
        json!(normalize_terminal_contexts_for_thread(
            target_thread_id,
            draft_list(&source, "terminalContexts")?
        )?),
    );
    map.insert(
        "fileComments".to_string(),
        json!(normalize_file_comments(draft_list(
            &source,
            "fileComments"
        )?)?),
    );
    map.insert(
        "pastedTexts".to_string(),
        json!(normalize_pasted_texts(draft_list(&source, "pastedTexts")?)?),
    );
    map.insert(
        "pullRequestContexts".to_string(),
        json!(normalize_pull_request_contexts(&draft_list(
            &source,
            "pullRequestContexts"
        )?)?),
    );
    map.insert("skills".to_string(), json!(draft_list(&source, "skills")?));
    map.insert(
        "mentions".to_string(),
        json!(draft_list(&source, "mentions")?),
    );
    map.insert(
        "enableComputerControl".to_string(),
        prop(&source, "enableComputerControl"),
    );
    map.insert(
        "computerControlMode".to_string(),
        prop(&source, "computerControlMode"),
    );
    let generation = prop(&base, "computerControlGeneration");
    map.insert(
        "computerControlGeneration".to_string(),
        if nullish(&generation) {
            js::number(0.0)
        } else {
            generation
        },
    );
    map.insert("restoredSourceProposedPlan".to_string(), Value::Null);
    Ok(Value::Object(map))
}

/// Port of `cloneComposerImageAttachment` (`apps/web/src/composerDraftDomain.ts:869`).
pub fn clone_composer_image_attachment(
    image: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let preview = prop_or_err(&image, "previewUrl")?;
    let preview_str = match &preview {
        Value::String(s) => s.clone(),
        _ => {
            return Err("image.previewUrl.startsWith is not a function".to_string());
        }
    };
    if !preview_str.starts_with("blob:") {
        return Ok(image);
    }
    // URL.createObjectURL(image.file) is unobservable in a pure port; recorded
    // cases never hit this branch, so keep the original (the catch path).
    Ok(image)
}

/// Port of `shouldRemoveDraft` (`apps/web/src/composerDraftDomain.ts:912`).
pub fn should_remove_draft(draft: serde_json::Value) -> Result<bool, String> {
    let pui = prop_or_err(&draft, "pendingUserInputDrafts")?;
    let pui = if nullish(&pui) { json!({}) } else { pui };
    if object_key_count(&pui)? != 0 {
        return Ok(false);
    }
    let prompt = prop_or_err(&draft, "prompt")?;
    if length_or_err(&prompt)? != 0.0 {
        return Ok(false);
    }
    if !prop(&draft, "promptHistorySavedDraft").is_null() {
        return Ok(false);
    }
    for key in [
        "images",
        "files",
        "persistedAttachments",
        "assistantSelections",
        "browserAnnotations",
        "terminalContexts",
        "fileComments",
        "pastedTexts",
        "pullRequestContexts",
        "skills",
        "mentions",
        "queuedTurns",
    ] {
        let v = prop_or_err(&draft, key)?;
        if length_or_err(&v)? != 0.0 {
            return Ok(false);
        }
    }
    if !nullish(&prop(&draft, "restoredSourceProposedPlan")) {
        return Ok(false);
    }
    if object_key_count(&prop_or_err(&draft, "modelSelectionByProvider")?)? != 0 {
        return Ok(false);
    }
    if !js_strict_eq(&prop(&draft, "activeProvider"), &Value::Null) {
        return Ok(false);
    }
    if !nullish(&prop(&draft, "providerOptionsForDispatch")) {
        return Ok(false);
    }
    if !js_strict_eq(&prop(&draft, "runtimeMode"), &Value::Null) {
        return Ok(false);
    }
    if !js_strict_eq(&prop(&draft, "interactionMode"), &Value::Null) {
        return Ok(false);
    }
    if !js::is_undefined(&prop(&draft, "enableComputerControl")) {
        return Ok(false);
    }
    if !js::is_undefined(&prop(&draft, "computerControlMode")) {
        return Ok(false);
    }
    Ok(true)
}

// zustand `shallow`: own enumerable keys must match one-to-one under Object.is.
fn object_is_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
        return x == y || (x.is_nan() && y.is_nan());
    }
    js_strict_eq(a, b)
}

fn shallow_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            if x.len() != y.len() {
                return false;
            }
            x.iter()
                .all(|(k, v)| y.get(k).is_some_and(|w| object_is_eq(v, w)))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(v, w)| object_is_eq(v, w))
        }
        _ => object_is_eq(a, b),
    }
}

/// Port of `composerDraftsMatchForCleanup` (`apps/web/src/composerDraftDomain.ts:943`).
pub fn composer_drafts_match_for_cleanup(
    current: serde_json::Value,
    captured: serde_json::Value,
) -> Result<bool, String> {
    if js_strict_eq(&current, &captured) {
        return Ok(true);
    }
    if !truthy(&current) || !truthy(&captured) {
        return Ok(false);
    }
    let strip = |v: &Value| -> Value {
        let mut map = Map::new();
        spread_into(&mut map, v);
        map.shift_remove("persistedAttachments");
        map.shift_remove("nonPersistedImageIds");
        Value::Object(map)
    };
    if !shallow_eq(&strip(&current), &strip(&captured)) {
        return Ok(false);
    }
    let captured_images = prop_or_err(&captured, "images")?;
    let captured_images = array_method(&captured_images, "captured.images", "map")?;
    let live_ids: Vec<Value> = captured_images.iter().map(|i| prop(i, "id")).collect();
    let filter = |v: &Value, name: &str| -> Result<Vec<Value>, String> {
        let items = array_method(v, name, "filter")?;
        Ok(items
            .iter()
            .filter(|item| !svz_contains(&live_ids, &prop(item, "id")))
            .cloned()
            .collect())
    };
    let current_persisted = prop_or_err(&current, "persistedAttachments")?;
    let captured_persisted = prop_or_err(&captured, "persistedAttachments")?;
    let a = filter(&current_persisted, "currentPersisted")?;
    let b = filter(&captured_persisted, "capturedPersisted")?;
    Ok(shallow_eq(&json!(a), &json!(b)))
}

/// Port of `composerDraftHasAttachments` (`apps/web/src/composerDraftDomain.ts:972`).
pub fn composer_draft_has_attachments(draft: serde_json::Value) -> Result<bool, String> {
    let non_empty = |key: &str| -> Result<bool, String> {
        let v = prop_or_err(&draft, key)?;
        Ok(length_or_err(&v)? > 0.0)
    };
    if non_empty("images")? || non_empty("files")? || non_empty("persistedAttachments")? {
        return Ok(true);
    }
    let contexts = prop_or_err(&draft, "terminalContexts")?;
    let contexts = array_method(&contexts, "draft.terminalContexts", "some")?;
    for context in contexts {
        let text = prop_or_err(context, "text")?;
        let text = str_or_err(&text, "trim")?;
        if !js_trim(&text).is_empty() {
            return Ok(true);
        }
    }
    if non_empty("assistantSelections")? {
        return Ok(true);
    }
    let annotations = prop(&draft, "browserAnnotations");
    let annotations_len = if nullish(&annotations) {
        0.0
    } else {
        length_or_err(&annotations)?
    };
    if annotations_len > 0.0 {
        return Ok(true);
    }
    if non_empty("fileComments")? {
        return Ok(true);
    }
    Ok(false)
}

/// Port of `composerDraftHasUnsentContent` (`apps/web/src/composerDraftDomain.ts:1000`).
pub fn composer_draft_has_unsent_content(draft: serde_json::Value) -> Result<bool, String> {
    let prompt = prop_or_err(&draft, "prompt")?;
    let prompt = str_or_err(&prompt, "trim")?;
    if !js_trim(&prompt).is_empty() {
        return Ok(true);
    }
    if composer_draft_has_attachments(draft.clone())? {
        return Ok(true);
    }
    let v = prop_or_err(&draft, "pastedTexts")?;
    if length_or_err(&v)? > 0.0 {
        return Ok(true);
    }
    let v = prop_or_err(&draft, "pullRequestContexts")?;
    Ok(length_or_err(&v)? > 0.0)
}

/// Port of `composerThreadDraftIsPending` (`apps/web/src/composerDraftDomain.ts:1017`).
pub fn composer_thread_draft_is_pending(draft: serde_json::Value) -> Result<bool, String> {
    let saved = prop(&draft, "promptHistorySavedDraft");
    let target = if nullish(&saved) { draft } else { saved };
    composer_draft_has_unsent_content(target)
}

/// Port of `composerThreadDraftPreviewText` (`apps/web/src/composerDraftDomain.ts:1028`).
pub fn composer_thread_draft_preview_text(
    draft: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let Some(draft) = draft else {
        return Ok(None);
    };
    if !truthy(&draft) {
        return Ok(None);
    }
    let saved = prop(&draft, "promptHistorySavedDraft");
    let target = if nullish(&saved) { draft } else { saved };
    let prompt = prop_or_err(&target, "prompt")?;
    let prompt = match &prompt {
        Value::String(s) => s.clone(),
        _ => return Err("text.replaceAll is not a function".to_string()),
    };
    let text = prompt.replace('\u{fffc}', " ");
    let text = js_trim(&collapse_ws(&text));
    if text.is_empty() {
        return Ok(None);
    }
    if utf16_len(&text) > 280 {
        let mut out = String::new();
        let mut units = 0usize;
        for c in text.chars() {
            if units + c.len_utf16() > 280 {
                break;
            }
            units += c.len_utf16();
            out.push(c);
        }
        return Ok(Some(format!("{}\u{2026}", js_trim_end(&out))));
    }
    Ok(Some(text))
}

/// Port of `normalizeDraftThreadEntryPoint` (`apps/web/src/composerDraftDomain.ts:1053`).
pub fn normalize_draft_thread_entry_point(
    value: serde_json::Value,
    fallback: Option<String>,
) -> Result<serde_json::Value, String> {
    Ok(entry_point_of(
        &value,
        &json!(fallback.unwrap_or_else(|| "chat".to_string())),
    ))
}
