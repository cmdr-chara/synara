//! The Rust port of `packages/shared/src/shell.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value};

const MARKER_NAME: &str = "SYNARA_PATH_HYDRATED";
const MARKER_VALUE: &str = "1";

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\u{b}'
            | '\u{c}'
            | '\r'
            | '\u{a0}'
            | '\u{feff}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

fn js_trim(s: &str) -> String {
    s.trim_matches(is_js_space).to_string()
}

fn nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

fn read_member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(member(v, key))
}

fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => js::number_to_string(js::as_number(v).unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(_) => "[object Array]".into(),
        Value::Object(_) => "[object Object]".into(),
    }
}

/// `value?.trim()` where `value` may be any JS value: nullish → undefined,
/// string → trimmed, anything else → "value?.trim is not a function".
fn trim_non_empty(v: &Value) -> Result<Option<String>, String> {
    if nullish(v) {
        return Ok(None);
    }
    match v {
        Value::String(s) => {
            let t = js_trim(s);
            Ok((!t.is_empty()).then_some(t))
        }
        _ => Err("value?.trim is not a function".into()),
    }
}

fn read_user_login_shell() -> Option<String> {
    use users::os::unix::UserExt;
    let user = users::get_user_by_uid(users::get_current_uid())?;
    let shell = user.shell().to_string_lossy().to_string();
    let t = js_trim(&shell);
    (!t.is_empty()).then_some(t)
}

fn platform_is(v: &Value, name: &str) -> bool {
    matches!(v, Value::String(s) if s == name)
}

fn is_valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

fn env_capture_start(name: &str) -> String {
    format!("__SYNARA_ENV_{name}_START__")
}
fn env_capture_end(name: &str) -> String {
    format!("__SYNARA_ENV_{name}_END__")
}

fn is_nushell(shell: &str) -> bool {
    let lower = shell.to_lowercase();
    for suffix in ["nu", "nu.exe"] {
        if lower.ends_with(suffix) {
            let rest = &lower[..lower.len() - suffix.len()];
            if rest.is_empty() || rest.ends_with('/') || rest.ends_with('\\') {
                return true;
            }
        }
    }
    false
}

fn build_capture_command(names: &[Value], nushell: bool) -> Result<String, String> {
    let mut parts = Vec::new();
    for name in names {
        let n = js_string(name);
        if !is_valid_env_name(&n) {
            return Err(format!("Unsupported environment variable name: {n}"));
        }
        if nushell {
            parts.push(format!(
                "print '{}'; do {{ ^printenv {n} }} | complete | get stdout | print --no-newline; print '{}'",
                env_capture_start(&n),
                env_capture_end(&n)
            ));
        } else {
            parts.push(format!(
                "printf '%s\\n' '{}'; printenv {n} || true; printf '%s\\n' '{}'",
                env_capture_start(&n),
                env_capture_end(&n)
            ));
        }
    }
    Ok(parts.join("; "))
}

/// Entries of a JS object source in `Object.entries` order: canonical integer
/// index keys ascending first, then the rest in insertion order.
fn object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
    if nullish(v) {
        return Err("Cannot convert undefined or null to object".into());
    }
    let entries: Vec<(String, Value)> = match v {
        Value::Object(o) => o.iter().map(|(k, x)| (k.clone(), x.clone())).collect(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect(),
        Value::String(s) => crate::utf16::units(s)
            .iter()
            .enumerate()
            .map(|(i, u)| (i.to_string(), js::utf16_string(&[*u])))
            .collect(),
        _ => Vec::new(),
    };
    Ok(js_key_order(entries))
}

/// JS property order: canonical array-index keys first (ascending), then insertion order.
fn js_key_order(entries: Vec<(String, Value)>) -> Vec<(String, Value)> {
    let mut ints: Vec<(u64, String, Value)> = Vec::new();
    let mut rest: Vec<(String, Value)> = Vec::new();
    for (k, v) in entries {
        if let Some(n) = canonical_index(&k) {
            ints.push((n, k, v));
        } else {
            rest.push((k, v));
        }
    }
    ints.sort_by_key(|(n, _, _)| *n);
    ints.into_iter()
        .map(|(_, k, v)| (k, v))
        .chain(rest)
        .collect()
}

fn canonical_index(k: &str) -> Option<u64> {
    if k.is_empty() || k.len() > 10 {
        return None;
    }
    let n: u64 = k.parse().ok()?;
    if n >= u32::MAX as u64 {
        return None;
    }
    if n.to_string() != k {
        return None;
    }
    Some(n)
}

/// Port of `isShellEnvironmentHydrated` (`packages/shared/src/shell.ts:38`).
pub fn is_shell_environment_hydrated(env: serde_json::Value) -> Result<bool, String> {
    let marker = read_member(&env, MARKER_NAME)?;
    if !matches!(&marker, Value::String(s) if s == MARKER_VALUE) {
        return Ok(false);
    }
    Ok(trim_non_empty(&read_member(&env, "PATH")?)?.is_some())
}

/// Port of `applyShellEnvironmentHydrationMarker` (`packages/shared/src/shell.ts:50`).
pub fn apply_shell_environment_hydration_marker(
    env: serde_json::Value,
    hydrated: bool,
) -> Result<serde_json::Value, String> {
    if env.is_null() {
        return if hydrated {
            Err(format!(
                "Cannot set properties of null (setting '{MARKER_NAME}')"
            ))
        } else {
            Err("Cannot convert undefined or null to object".into())
        };
    }
    if js::is_undefined(&env) {
        return if hydrated {
            Err(format!(
                "Cannot set properties of undefined (setting '{MARKER_NAME}')"
            ))
        } else {
            Err("Cannot convert undefined or null to object".into())
        };
    }
    let mut env = env;
    match &mut env {
        Value::Object(o) => {
            if hydrated {
                o.insert(MARKER_NAME.to_string(), Value::String(MARKER_VALUE.into()));
            } else {
                o.remove(MARKER_NAME);
            }
            Ok(env)
        }
        Value::Array(_) => Ok(env),
        _ => {
            if hydrated {
                let kind = match &env {
                    Value::Bool(_) => "boolean",
                    Value::Number(_) => "number",
                    _ => "string",
                };
                Err(format!(
                    "Cannot create property '{MARKER_NAME}' on {kind} '{}'",
                    js_string(&env)
                ))
            } else {
                Ok(env)
            }
        }
    }
}

/// Port of `listLoginShellCandidates` (`packages/shared/src/shell.ts:70`).
pub fn list_login_shell_candidates(
    platform: serde_json::Value,
    shell: Option<String>,
    user_shell: serde_json::Value,
) -> Result<Vec<String>, String> {
    let fallback: Option<&str> = if platform_is(&platform, "darwin") {
        Some("/bin/zsh")
    } else if platform_is(&platform, "linux") {
        Some("/bin/bash")
    } else {
        None
    };
    let user_shell_trimmed = if js::is_undefined(&user_shell) {
        read_user_login_shell()
    } else {
        trim_non_empty(&user_shell)?
    };
    let shell_trimmed = match &shell {
        Some(s) => {
            let t = js_trim(s);
            (!t.is_empty()).then_some(t)
        }
        None => None,
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in [
        shell_trimmed,
        user_shell_trimmed,
        fallback.map(String::from),
    ] {
        if let Some(c) = c
            && seen.insert(c.clone())
        {
            out.push(c);
        }
    }
    Ok(out)
}

/// Port of `resolveLoginShell` (`packages/shared/src/shell.ts:91`).
pub fn resolve_login_shell(
    platform: serde_json::Value,
    shell: Option<String>,
) -> Result<serde_json::Value, String> {
    let candidates = list_login_shell_candidates(platform, shell, js::undefined().clone())?;
    Ok(match candidates.into_iter().next() {
        Some(s) => Value::String(s),
        None => js::undefined().clone(),
    })
}

/// Port of `mergePathEntries` (`packages/shared/src/shell.ts:121`).
pub fn merge_path_entries(
    preferred_path: Option<String>,
    inherited_path: Option<String>,
    platform: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let windows = platform_is(&platform, "win32");
    let delimiter = if windows { ";" } else { ":" };
    let mut merged: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for path_value in [preferred_path, inherited_path] {
        let Some(path_value) = path_value else {
            continue;
        };
        if path_value.is_empty() {
            continue;
        }
        for entry in path_value.split(delimiter) {
            let trimmed = js_trim(entry);
            if trimmed.is_empty() {
                continue;
            }
            let key = if windows {
                trimmed
                    .to_lowercase()
                    .trim_end_matches(['\\', '/'])
                    .to_string()
            } else {
                trimmed.clone()
            };
            if seen.insert(key) {
                merged.push(trimmed);
            }
        }
    }
    Ok(if merged.is_empty() {
        js::undefined().clone()
    } else {
        Value::String(merged.join(delimiter))
    })
}

/// Port of `readEnvironmentFromLoginShell` (`packages/shared/src/shell.ts:219`).
pub fn read_environment_from_login_shell(
    shell: serde_json::Value,
    names: serde_json::Value,
    exec_file: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let names_len: Option<usize> = match &names {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'length')".into());
        }
        _ if js::is_undefined(&names) => {
            return Err("Cannot read properties of undefined (reading 'length')".into());
        }
        Value::Array(a) => Some(a.len()),
        Value::String(s) => Some(s.chars().count()),
        _ => None,
    };
    if names_len == Some(0) {
        return Ok(Value::Object(Map::new()));
    }
    let Value::Array(names) = names else {
        return Err("names.map is not a function".into());
    };
    let shell_str = js_string(&shell);
    let nushell = is_nushell(&shell_str);
    let command = build_capture_command(&names, nushell)?;
    if !js::is_undefined(&exec_file) {
        return Err("execFile is not a function".into());
    }
    let file = match &shell {
        Value::String(s) => s.clone(),
        Value::Array(_) => {
            return Err(
                "The \"file\" argument must be of type string. Received an instance of Array"
                    .into(),
            );
        }
        other => {
            let kind = match other {
                Value::Object(_) => "Object",
                Value::Null => "null",
                Value::Bool(_) => "boolean",
                Value::Number(_) => "number",
                _ => "Object",
            };
            return Err(format!(
                "The \"file\" argument must be of type string. Received an instance of {kind}"
            ));
        }
    };
    let args = if nushell {
        vec![
            "--login".to_string(),
            "--interactive".to_string(),
            "--commands".to_string(),
            command,
        ]
    } else {
        vec!["-ilc".to_string(), command]
    };
    let output = std::process::Command::new(&file)
        .args(&args)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("spawnSync {file} ENOENT")
            } else {
                format!("spawnSync {file} {e}")
            }
        })?;
    if !output.status.success() {
        return Err(format!("Command failed: {file} {}", args.join(" ")));
    }
    let output = String::from_utf8_lossy(&output.stdout).to_string();
    let mut env = Map::new();
    for name in &names {
        let n = js_string(name);
        if let Some(v) = extract_environment_value(&output, &n) {
            env.insert(n, Value::String(v));
        }
    }
    Ok(Value::Object(env))
}

fn extract_environment_value(output: &str, name: &str) -> Option<String> {
    let start = env_capture_start(name);
    let end = env_capture_end(name);
    let start_index = utf16_index_of(output, &start)?;
    let value_start = start_index + start.chars().count();
    let end_index = utf16_index_of(&output[byte_offset(output, value_start)..], &end)
        .map(|i| i + value_start)?;
    let mut value = utf16_slice(output, value_start, end_index);
    if let Some(rest) = value.strip_prefix('\n') {
        value = rest.to_string();
    }
    if let Some(rest) = value.strip_suffix('\n') {
        value = rest.to_string();
    }
    (!value.is_empty()).then_some(value)
}

fn utf16_index_of(hay: &str, needle: &str) -> Option<usize> {
    hay.find(needle).map(|b| hay[..b].chars().count())
}
fn byte_offset(hay: &str, char_index: usize) -> usize {
    hay.char_indices()
        .nth(char_index)
        .map(|(b, _)| b)
        .unwrap_or(hay.len())
}
fn utf16_slice(s: &str, from: usize, to: usize) -> String {
    s.chars().skip(from).take(to - from).collect()
}

/// Port of `isPathName` (`packages/shared/src/shell.ts:252`).
pub fn is_path_name(name: String) -> Result<bool, String> {
    Ok(name.to_uppercase() == "PATH")
}

/// Port of `mergeWindowsScopes` (`packages/shared/src/shell.ts:258`).
pub fn merge_windows_scopes(
    machine: serde_json::Value,
    user: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let machine_entries = object_entries(&machine)?;
    let user_entries = object_entries(&user)?;
    let mut merged: Vec<(String, Value)> = Vec::new();
    let mut key_by_lower: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for entries in [machine_entries, user_entries] {
        for (name, value) in entries {
            if is_path_name(name.clone())? {
                continue;
            }
            let Some(trimmed) = trim_non_empty(&value)? else {
                continue;
            };
            let lower = name.to_lowercase();
            if let Some(existing) = key_by_lower.get(&lower)
                && existing != &name
            {
                merged.retain(|(k, _)| k != existing);
            }
            if let Some(slot) = merged.iter_mut().find(|(k, _)| *k == name) {
                slot.1 = Value::String(trimmed);
            } else {
                merged.push((name.clone(), Value::String(trimmed)));
            }
            key_by_lower.insert(lower, name);
        }
    }
    let scope_path = |entries: &[(String, Value)]| -> Result<Option<String>, String> {
        for (name, value) in entries {
            if is_path_name(name.clone())? {
                return trim_non_empty(value);
            }
        }
        Ok(None)
    };
    let machine_entries = object_entries(&machine)?;
    let user_entries = object_entries(&user)?;
    let machine_path = scope_path(&machine_entries)?;
    let user_path = scope_path(&user_entries)?;
    let combined: Vec<String> = [machine_path, user_path].into_iter().flatten().collect();
    if !combined.is_empty() {
        if let Some(slot) = merged.iter_mut().find(|(k, _)| k == "PATH") {
            slot.1 = Value::String(combined.join(";"));
        } else {
            merged.push(("PATH".into(), Value::String(combined.join(";"))));
        }
    }
    let mut out = Map::new();
    for (k, v) in js_key_order(merged) {
        out.insert(k, v);
    }
    Ok(Value::Object(out))
}

/// Port of `readWindowsPersistentEnvironment` (`packages/shared/src/shell.ts:324`).
pub fn read_windows_persistent_environment(
    exec_file: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !js::is_undefined(&exec_file) {
        return Err("execFile is not a function".into());
    }
    let system_root = trim_non_empty(&Value::String(
        std::env::var("SystemRoot").unwrap_or_default(),
    ))?
    .unwrap_or_else(|| "C:\\Windows".into());
    let file = format!("{system_root}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    let script = concat!(
        "$ErrorActionPreference='Stop';",
        "function dump($s){$m=[ordered]@{};$v=[Environment]::GetEnvironmentVariables($s);",
        "foreach($k in $v.Keys){$m[[string]$k]=[Environment]::ExpandEnvironmentVariables([string]$v[$k])};$m}",
        "$o=[ordered]@{machine=(dump 'Machine');user=(dump 'User')};",
        "[Console]::Out.Write(($o|ConvertTo-Json -Compress -Depth 3))"
    );
    let output = std::process::Command::new(&file)
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("spawnSync {file} ENOENT")
            } else {
                format!("spawnSync {file} {e}")
            }
        })?;
    if !output.status.success() {
        return Err(format!("Command failed: {file}"));
    }
    let text = js_trim(&String::from_utf8_lossy(&output.stdout));
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(Value::Object(Map::new())),
    };
    let machine = member(&parsed, "machine").clone();
    let user = member(&parsed, "user").clone();
    merge_windows_scopes(machine, user)
}
