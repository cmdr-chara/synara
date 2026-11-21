//! The Rust port of `apps/web/src/projectScripts.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const MAX_SCRIPT_ID_LENGTH: usize = 24;

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
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
    })
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

fn utf16_slice(s: &str, n: usize) -> &str {
    if utf16_len(s) <= n {
        return s;
    }
    let mut taken = 0;
    let mut end = s.len();
    for (i, c) in s.char_indices() {
        if taken + c.len_utf16() > n {
            end = i;
            break;
        }
        taken += c.len_utf16();
    }
    &s[..end]
}

fn normalize_script_id(value: &str) -> String {
    let lower = js_trim(value).to_lowercase();
    let mut cleaned = String::with_capacity(lower.len());
    let mut in_dash = false;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            cleaned.push(c);
            in_dash = false;
        } else if !in_dash {
            cleaned.push('-');
            in_dash = true;
        }
    }
    let cleaned = cleaned.trim_matches('-');
    if cleaned.is_empty() {
        return "script".to_string();
    }
    if utf16_len(cleaned) <= MAX_SCRIPT_ID_LENGTH {
        return cleaned.to_string();
    }
    let truncated = utf16_slice(cleaned, MAX_SCRIPT_ID_LENGTH).trim_end_matches('-');
    if truncated.is_empty() {
        "script".to_string()
    } else {
        truncated.to_string()
    }
}

fn valid_script_id(id: &str) -> Result<(), String> {
    let json = serde_json::to_string(id).unwrap_or_else(|_| "\"\"".to_string());
    if id.is_empty() {
        return Err(format!(
            "Expected a value with a length of at least 1, got {json}\n  at [1]"
        ));
    }
    if utf16_len(id) > MAX_SCRIPT_ID_LENGTH {
        return Err(format!(
            "Expected a value with a length of at most {MAX_SCRIPT_ID_LENGTH}, got {json}\n  at [1]"
        ));
    }
    let mut chars = id.chars();
    let id_char = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    let first_ok = chars.next().is_some_and(id_char);
    let rest_ok = chars.all(|c| id_char(c) || c == '-');
    if !(first_ok && rest_ok) {
        return Err(format!(
            "Expected a string matching the RegExp ^[a-z0-9][a-z0-9-]*$, got {json}\n  at [1]"
        ));
    }
    Ok(())
}

/// Port of `commandForProjectScript` (`apps/web/src/projectScripts.ts:25`).
pub fn command_for_project_script(script_id: String) -> Result<serde_json::Value, String> {
    valid_script_id(&script_id)?;
    Ok(Value::String(format!("script.{script_id}.run")))
}

/// Port of `projectScriptIdFromCommand` (`apps/web/src/projectScripts.ts:28`).
pub fn project_script_id_from_command(command: String) -> Result<Option<String>, String> {
    let trimmed = js_trim(&command);
    if let Some(inner) = trimmed
        .strip_prefix("script.")
        .and_then(|s| s.strip_suffix(".run"))
        && valid_script_id(inner).is_ok()
    {
        return Ok(Some(inner.to_string()));
    }
    Ok(None)
}

/// `Array.from(existingIds)` over the bridged iterable value.
fn array_from(value: &Value) -> Result<Vec<Value>, String> {
    match value {
        Value::Null => Err(
            "object null is not iterable (cannot read property Symbol(Symbol.iterator))"
                .to_string(),
        ),
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        Value::Object(m) if crate::js::is_undefined(value) => {
            let _ = m;
            Err(
                "object undefined is not iterable (cannot read property Symbol(Symbol.iterator))"
                    .to_string(),
            )
        }
        Value::Object(m) => {
            // Array-like fallback: only numeric `length` produces elements.
            match m.get("length").and_then(crate::js::as_number) {
                Some(n) if n > 0.0 => Ok((0..(n.trunc() as u64).min(1_000_000))
                    .map(|i| {
                        m.get(i.to_string().as_str())
                            .cloned()
                            .unwrap_or_else(crate::js::undefined)
                    })
                    .collect()),
                _ => Ok(Vec::new()),
            }
        }
        _ => Ok(Vec::new()),
    }
}

/// Port of `nextProjectScriptId` (`apps/web/src/projectScripts.ts:37`).
pub fn next_project_script_id(
    name: String,
    existing_ids: serde_json::Value,
) -> Result<String, String> {
    let taken: std::collections::HashSet<String> = array_from(&existing_ids)?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    let base_id = normalize_script_id(&name);
    if !taken.contains(&base_id) {
        return Ok(base_id);
    }
    for suffix in 2u64..10_000 {
        let suffix_s = suffix.to_string();
        let candidate = format!("{base_id}-{suffix_s}");
        let safe_candidate = if utf16_len(&candidate) <= MAX_SCRIPT_ID_LENGTH {
            candidate
        } else {
            let keep = MAX_SCRIPT_ID_LENGTH
                .saturating_sub(suffix_s.len() + 1)
                .max(1);
            format!("{}-{suffix_s}", utf16_slice(&base_id, keep))
        };
        if !taken.contains(&safe_candidate) {
            return Ok(safe_candidate);
        }
    }
    Ok(utf16_slice(&format!("{base_id}-9999999999999"), MAX_SCRIPT_ID_LENGTH).to_string())
}

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member<'a>(v: &'a Value, key: &str) -> Result<&'a Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).unwrap_or(&Value::Null)),
        _ => Ok(&Value::Null),
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
        // Plain numbers were already handled by `as_number` above.
        Value::Number(_) => false,
    }
}

/// Port of `projectScriptRuntimeEnv` (`apps/web/src/projectScripts.ts:81`).
pub fn project_script_runtime_env(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let project = member(&input, "project")?;
    let cwd = member(project, "cwd")?.clone();
    let mut entries: Vec<(String, Value)> = vec![("SYNARA_PROJECT_ROOT".to_string(), cwd)];
    let worktree = member(&input, "worktreePath")?;
    if truthy(worktree) {
        entries.push(("SYNARA_WORKTREE_PATH".to_string(), worktree.clone()));
    }
    let extra = member(&input, "extraEnv")?;
    if truthy(extra)
        && let Value::Object(m) = extra
    {
        for (k, v) in m {
            if let Some(pos) = entries.iter().position(|(ek, _)| ek == k) {
                entries[pos] = (k.clone(), v.clone());
            } else {
                entries.push((k.clone(), v.clone()));
            }
        }
    }
    let mut ints: Vec<(u64, String, Value)> = Vec::new();
    let mut rest: Vec<(String, Value)> = Vec::new();
    for (k, v) in entries {
        match canonical_index(&k) {
            Some(n) => ints.push((n, k, v)),
            None => rest.push((k, v)),
        }
    }
    ints.sort_by_key(|(n, _, _)| *n);
    let ordered: Vec<(String, Value)> = ints
        .into_iter()
        .map(|(_, k, v)| (k, v))
        .chain(rest)
        .collect();
    Ok(Value::Object(ordered.into_iter().collect()))
}

fn canonical_index(k: &str) -> Option<u64> {
    if k.is_empty() || k.len() > 10 {
        return None;
    }
    let n: u64 = k.parse().ok()?;
    if n >= u32::MAX as u64 || n.to_string() != k {
        return None;
    }
    Some(n)
}

/// Port of `primaryProjectScript` (`apps/web/src/projectScripts.ts:96`).
pub fn primary_project_script(
    scripts: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    match &scripts {
        Value::Null => Err("Cannot read properties of null (reading 'find')".to_string()),
        _ if is_undefined(&scripts) => {
            Err("Cannot read properties of undefined (reading 'find')".to_string())
        }
        Value::Array(items) => {
            for s in items {
                if !truthy(member(s, "runOnWorktreeCreate")?) {
                    return Ok(Some(s.clone()));
                }
            }
            Ok(items.first().cloned())
        }
        _ => Err("scripts.find is not a function".to_string()),
    }
}

/// Port of `setupProjectScript` (`apps/web/src/projectScripts.ts:101`).
pub fn setup_project_script(
    scripts: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    match &scripts {
        Value::Null => Err("Cannot read properties of null (reading 'find')".to_string()),
        _ if is_undefined(&scripts) => {
            Err("Cannot read properties of undefined (reading 'find')".to_string())
        }
        Value::Array(items) => {
            for s in items {
                if truthy(member(s, "runOnWorktreeCreate")?) {
                    return Ok(Some(s.clone()));
                }
            }
            Ok(None)
        }
        _ => Err("scripts.find is not a function".to_string()),
    }
}
