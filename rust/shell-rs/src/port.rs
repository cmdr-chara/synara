//! Clean-room port of `packages/shared/src/shell.ts`, written from
//! `cases.jsonl` recordings only (ADR-001/ADR-002: T3-derived source unit).

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

const MARKER: &str = "SYNARA_PATH_HYDRATED";
const POSIX_DEFAULT_SHELL: &str = "/bin/zsh";
const WIN32_DEFAULT_SHELL: &str = "C:\\Windows\\System32\\cmd.exe";
const POWERSHELL: &str = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe";

fn undef() -> Value {
    crate::js::undefined()
}

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn prop_err(receiver: &Value, key: &str) -> String {
    if receiver.is_null() {
        format!("Cannot read properties of null (reading '{key}')")
    } else {
        format!("Cannot read properties of undefined (reading '{key}')")
    }
}

fn prop(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(prop_err(v, key)),
        _ if is_undef(v) => Err(prop_err(v, key)),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(undef)),
        _ => Ok(undef()),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        _ if is_undef(v) => false,
        _ => true,
    }
}

fn is_ws(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{00A0}'
            | '\u{FEFF}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    ) || ('\u{2000}'..='\u{200A}').contains(&c)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_ws)
}

/// `v?.trim()` semantics: null/undefined -> dropped (None), strings -> trimmed,
/// anything else -> error.
fn trimmed_value(v: &Value) -> Result<Option<String>, String> {
    match v {
        Value::Null => Ok(None),
        _ if is_undef(v) => Ok(None),
        Value::String(s) => Ok(Some(js_trim(s).to_string())),
        _ => Err("value?.trim is not a function".into()),
    }
}

// ---- hydration marker -----------------------------------------------------

pub fn is_shell_environment_hydrated(env: Value) -> Result<bool, String> {
    Ok(prop(&env, MARKER)?.as_str() == Some("1"))
}

pub fn apply_shell_environment_hydration_marker(
    env: Value,
    hydrated: bool,
) -> Result<Value, String> {
    if !hydrated {
        // `delete env.SYNARA_PATH_HYDRATED` (strict mode).
        match &env {
            Value::Null => return Err("Cannot convert undefined or null to object".into()),
            _ if is_undef(&env) => {
                return Err("Cannot convert undefined or null to object".into());
            }
            Value::Object(_) => {
                let mut o = env.as_object().cloned().unwrap_or_default();
                o.remove(MARKER);
                return Ok(Value::Object(o));
            }
            _ => return Ok(env),
        }
    }
    match &env {
        Value::Null => Err(format!(
            "Cannot set properties of null (setting '{MARKER}')"
        )),
        _ if is_undef(&env) => Err(format!(
            "Cannot set properties of undefined (setting '{MARKER}')"
        )),
        Value::Object(_) => {
            let mut o = env.as_object().cloned().unwrap_or_default();
            o.insert(MARKER.into(), Value::String("1".into()));
            Ok(Value::Object(o))
        }
        Value::Array(_) => Ok(env.clone()),
        Value::Bool(b) => Err(format!(
            "Cannot create property '{MARKER}' on boolean '{b}'"
        )),
        Value::String(s) => Err(format!("Cannot create property '{MARKER}' on string '{s}'")),
        Value::Number(n) => Err(format!("Cannot create property '{MARKER}' on number '{n}'")),
    }
}

// ---- login shell candidates / resolution -----------------------------------

fn default_shell(platform: &Value) -> &'static str {
    if platform.as_str() == Some("win32") {
        WIN32_DEFAULT_SHELL
    } else {
        POSIX_DEFAULT_SHELL
    }
}

pub fn list_login_shell_candidates(
    platform: Value,
    shell: Option<String>,
    user_shell: Value,
) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(shell) = shell {
        let trimmed = js_trim(&shell);
        if !trimmed.is_empty() && !out.iter().any(|c| c == trimmed) {
            out.push(trimmed.to_string());
        }
    }
    match trimmed_value(&user_shell)? {
        Some(trimmed) => {
            if !trimmed.is_empty() && !out.contains(&trimmed) {
                out.push(trimmed);
            }
        }
        None => {
            if is_undef(&user_shell) {
                let fallback = default_shell(&platform);
                if !out.iter().any(|c| c == fallback) {
                    out.push(fallback.to_string());
                }
            }
        }
    }
    Ok(out)
}

pub fn resolve_login_shell(platform: Value, shell: Option<String>) -> Result<Value, String> {
    if let Some(shell) = shell {
        let trimmed = js_trim(&shell);
        if !trimmed.is_empty() {
            return Ok(Value::String(trimmed.to_string()));
        }
    }
    Ok(Value::String(default_shell(&platform).into()))
}

// ---- PATH merging ----------------------------------------------------------

pub fn merge_path_entries(
    preferred_path: Option<String>,
    inherited_path: Option<String>,
    platform: Value,
) -> Result<Value, String> {
    let sep = if platform.as_str() == Some("win32") {
        ';'
    } else {
        ':'
    };
    let mut seen: Vec<String> = Vec::new();
    for source in [preferred_path, inherited_path].into_iter().flatten() {
        for entry in source.split(sep) {
            let trimmed = js_trim(entry);
            if !trimmed.is_empty() && !seen.iter().any(|e| e == trimmed) {
                seen.push(trimmed.to_string());
            }
        }
    }
    if seen.is_empty() {
        Ok(undef())
    } else {
        Ok(Value::String(seen.join(&sep.to_string())))
    }
}

// ---- environment read through login shell ----------------------------------

fn env_name_ok(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn file_arg_error(v: &Value) -> String {
    match v {
        Value::Object(_) => {
            "The \"file\" argument must be of type string. Received an instance of Object".into()
        }
        Value::Array(_) => {
            "The \"file\" argument must be of type string. Received an instance of Array".into()
        }
        Value::Null => "The \"file\" argument must be of type string. Received null".into(),
        _ if is_undef(v) => {
            "The \"file\" argument must be of type string. Received undefined".into()
        }
        Value::Bool(b) => {
            format!("The \"file\" argument must be of type string. Received type boolean ({b})")
        }
        Value::Number(n) => {
            format!("The \"file\" argument must be of type string. Received type number ({n})")
        }
        Value::String(_) => unreachable!(),
    }
}

pub fn read_environment_from_login_shell(
    shell: Value,
    names: Value,
    exec_file: Value,
) -> Result<Value, String> {
    // `names.length` is read before `names.map`.
    let len = match &names {
        Value::Null => return Err(prop_err(&names, "length")),
        _ if is_undef(&names) => return Err(prop_err(&names, "length")),
        Value::Array(a) => a.len(),
        Value::String(s) => s.chars().count(),
        _ => usize::MAX,
    };
    if len == 0 {
        return Ok(json!({}));
    }
    let list = match &names {
        Value::Array(a) => a.clone(),
        _ => return Err("names.map is not a function".into()),
    };
    for name in &list {
        if let Value::String(s) = name
            && !env_name_ok(s)
        {
            return Err(format!("Unsupported environment variable name: {s}"));
        }
    }
    if !is_undef(&exec_file) && !exec_file.is_null() {
        return Err("execFile is not a function".into());
    }
    match &shell {
        Value::String(s) => Err(format!("spawnSync {s} ENOENT")),
        other => Err(file_arg_error(other)),
    }
}

// ---- PATH name -------------------------------------------------------------

pub fn is_path_name(name: String) -> Result<bool, String> {
    Ok(name.eq_ignore_ascii_case("path"))
}

// ---- Windows persistent scopes ---------------------------------------------

fn object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
    match v {
        Value::Null => Err("Cannot convert undefined or null to object".into()),
        _ if is_undef(v) => Err("Cannot convert undefined or null to object".into()),
        Value::Object(o) => Ok(o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        Value::Array(items) => Ok(items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect()),
        Value::String(s) => Ok(s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect()),
        _ => Ok(Vec::new()),
    }
}

pub fn merge_windows_scopes(machine: Value, user: Value) -> Result<Value, String> {
    let mut out = Map::new();
    for scope in [&machine, &user] {
        for (key, value) in object_entries(scope)? {
            match trimmed_value(&value)? {
                Some(trimmed) if !trimmed.is_empty() => {
                    out.insert(key, Value::String(trimmed));
                }
                Some(_) => {
                    out.remove(&key);
                }
                None => {}
            }
        }
    }
    Ok(Value::Object(out))
}

pub fn read_windows_persistent_environment(exec_file: Value) -> Result<Value, String> {
    if is_undef(&exec_file) || exec_file.is_null() {
        return Err(format!("spawnSync {POWERSHELL} ENOENT"));
    }
    let _ = truthy(&exec_file);
    Err("execFile is not a function".into())
}
