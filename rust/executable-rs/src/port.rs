//! The Rust port of `packages/shared/src/executable.ts`.
#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value};

const DEFAULT_WIN_EXTS: [&str; 4] = [".COM", ".EXE", ".BAT", ".CMD"];
const DEFAULT_POSIX_PATH: [&str; 2] = ["/usr/bin", "/bin"];

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
fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}
fn is_undefined(v: &Value) -> bool {
    js::is_undefined(v)
}
fn undefined() -> Value {
    js::undefined()
}
fn nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => !is_undefined(v),
    }
}
fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(undefined),
        _ => undefined(),
    }
}
/// `receiver.key` where null/undefined receivers throw.
fn read_member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undefined(v) {
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
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        _ => "undefined".into(),
    }
}
fn is_win32(v: &Value) -> bool {
    matches!(v, Value::String(s) if s == "win32")
}
fn proc_env() -> Value {
    let mut o = Map::new();
    for (k, v) in std::env::vars() {
        o.insert(k, Value::String(v));
    }
    Value::Object(o)
}

/// Node `path.posix.extname`.
fn posix_extname(path: &str) -> &str {
    let basename = path.rsplit('/').next().unwrap_or(path);
    let bytes = basename.as_bytes();
    let mut first_non_dot = 0;
    while first_non_dot < bytes.len() && bytes[first_non_dot] == b'.' {
        first_non_dot += 1;
    }
    let dot = basename.rfind('.');
    match dot {
        Some(i) if i >= first_non_dot && i > 0 => &basename[i..],
        _ => "",
    }
}
/// Node `path.posix.normalize`.
fn posix_normalize(path: &str) -> String {
    let abs = path.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    let mut trailing = false;
    for seg in path.split('/') {
        match seg {
            "" | "." => {
                trailing = seg == ".";
            }
            ".." => {
                if out.last().is_some_and(|s| *s != "..") {
                    out.pop();
                } else if !abs {
                    out.push("..");
                }
                trailing = true;
            }
            _ => {
                out.push(seg);
                trailing = false;
            }
        }
    }
    let mut r = String::new();
    if abs {
        r.push('/');
    }
    r.push_str(&out.join("/"));
    if r.is_empty() {
        return ".".into();
    }
    if trailing && !r.ends_with('/') {
        r.push('/');
    }
    r
}
/// Node `path.posix.join`.
fn posix_join(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    if joined.is_empty() {
        ".".into()
    } else {
        posix_normalize(&joined)
    }
}
fn posix_resolve(cwd: &str, p: &str) -> String {
    if p.starts_with('/') {
        posix_normalize(p)
    } else {
        posix_join(&[cwd, p])
    }
}

fn windows_path_exts(env: &Value) -> Result<Vec<String>, String> {
    let raw = read_member(env, "PATHEXT")?;
    let raw = if truthy(&raw) {
        match &raw {
            Value::String(s) => s.clone(),
            _ => return Err("env.PATHEXT.split is not a function".into()),
        }
    } else {
        return Ok(DEFAULT_WIN_EXTS.iter().map(|s| s.to_string()).collect());
    };
    let mut seen: Vec<String> = Vec::new();
    for entry in raw.split(';') {
        let e = js_trim(entry);
        if e.is_empty() {
            continue;
        }
        let up = e.to_uppercase();
        let full = if up.starts_with('.') {
            up
        } else {
            format!(".{up}")
        };
        if !seen.contains(&full) {
            seen.push(full);
        }
    }
    if seen.is_empty() {
        Ok(DEFAULT_WIN_EXTS.iter().map(|s| s.to_string()).collect())
    } else {
        Ok(seen)
    }
}

fn is_executable_in(
    file_path: &str,
    platform: &Value,
    env_exts: &[String],
    cwd: Option<&str>,
) -> bool {
    let stat_path = if file_path.starts_with('/') {
        file_path.to_string()
    } else {
        let base = cwd
            .map(|c| c.to_string())
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.to_str().map(String::from))
            })
            .unwrap_or_else(|| ".".into());
        posix_resolve(&base, file_path)
    };
    let meta = match std::fs::metadata(&stat_path) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if !meta.is_file() {
        return false;
    }
    if is_win32(platform) {
        let ext = posix_extname(file_path).to_uppercase();
        return !ext.is_empty()
            && (env_exts.iter().any(|e| e == &ext) || DEFAULT_WIN_EXTS.contains(&ext.as_str()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return meta.permissions().mode() & 0o111 != 0;
    }
    #[allow(unreachable_code)]
    true
}

/// Port of `envPathKeyFor` (`packages/shared/src/executable.ts:41`).
pub fn env_path_key_for(
    env: serde_json::Value,
    platform: Option<serde_json::Value>,
) -> Result<String, String> {
    let check = |key: &str| -> Result<bool, String> {
        match &env {
            Value::Object(o) => Ok(o.contains_key(key)),
            Value::Array(_) => Ok(false),
            other => Err(format!(
                "Cannot use 'in' operator to search for '{key}' in {}",
                if is_undefined(other) {
                    "undefined".into()
                } else {
                    js_string(other)
                }
            )),
        }
    };
    if check("PATH")? {
        return Ok("PATH".into());
    }
    if check("Path")? {
        return Ok("Path".into());
    }
    if check("path")? {
        let win = platform.as_ref().is_some_and(is_win32);
        return Ok(if win { "path" } else { "PATH" }.into());
    }
    Ok("PATH".into())
}

/// Port of `hasPathSeparator` (`packages/shared/src/executable.ts:56`).
pub fn has_path_separator(command: String) -> Result<bool, String> {
    Ok(command.contains('/') || command.contains('\\'))
}

/// Port of `windowsPathExtensions` (`packages/shared/src/executable.ts:60`).
pub fn windows_path_extensions(env: serde_json::Value) -> Result<Vec<String>, String> {
    windows_path_exts(&env)
}

/// Port of `pathEntries` (`packages/shared/src/executable.ts:73`).
pub fn path_entries(
    env: serde_json::Value,
    platform: serde_json::Value,
) -> Result<Vec<String>, String> {
    let mut path_value = read_member(&env, "PATH")?;
    if nullish(&path_value) {
        path_value = read_member(&env, "Path")?;
    }
    if nullish(&path_value) {
        path_value = read_member(&env, "path")?;
    }
    let win = is_win32(&platform);
    if nullish(&path_value) {
        return Ok(if win {
            Vec::new()
        } else {
            DEFAULT_POSIX_PATH.iter().map(|s| s.to_string()).collect()
        });
    }
    let s = match &path_value {
        Value::String(s) => s.clone(),
        _ => return Err("pathValue.split is not a function".into()),
    };
    if s.is_empty() {
        return Ok(Vec::new());
    }
    Ok(s.split(if win { ';' } else { ':' })
        .map(|e| js_trim(e).trim_matches('"'))
        .filter(|e| !e.is_empty())
        .map(String::from)
        .collect())
}

/// Port of `executableNameCandidates` (`packages/shared/src/executable.ts:86`).
pub fn executable_name_candidates(
    command: String,
    platform: serde_json::Value,
    env: serde_json::Value,
    allow_extensionless: Option<bool>,
) -> Result<Vec<String>, String> {
    if !is_win32(&platform) {
        return Ok(vec![command]);
    }
    let path_exts = windows_path_exts(&env)?;
    let extension = posix_extname(&command);
    let normalized = extension.to_uppercase();
    let mut candidates: Vec<String> = Vec::new();
    if !extension.is_empty()
        && (path_exts.contains(&normalized) || DEFAULT_WIN_EXTS.contains(&normalized.as_str()))
    {
        let stem = &command[..command.len() - extension.len()];
        for c in [
            command.clone(),
            format!("{stem}{normalized}"),
            format!("{stem}{}", normalized.to_lowercase()),
        ] {
            if !candidates.contains(&c) {
                candidates.push(c);
            }
        }
        return Ok(candidates);
    }
    if allow_extensionless.unwrap_or(false) {
        candidates.push(command.clone());
    }
    for ext in &path_exts {
        for c in [
            format!("{command}{ext}"),
            format!("{command}{}", ext.to_lowercase()),
        ] {
            if !candidates.contains(&c) {
                candidates.push(c);
            }
        }
    }
    Ok(candidates)
}

fn lookup_opts(options: &Value) -> Result<(Value, Value, Option<String>), String> {
    let opts = if is_undefined(options) {
        Value::Object(Map::new())
    } else {
        options.clone()
    };
    let platform = read_member(&opts, "platform")?;
    let env = read_member(&opts, "env")?;
    let cwd = match read_member(&opts, "cwd")? {
        Value::String(s) => Some(s),
        _ => None,
    };
    Ok((
        if nullish(&platform) {
            Value::Null
        } else {
            platform
        },
        if nullish(&env) { proc_env() } else { env },
        cwd,
    ))
}

/// Port of `isExecutableFile` (`packages/shared/src/executable.ts:225`).
pub fn is_executable_file(file_path: String, options: serde_json::Value) -> Result<bool, String> {
    let (platform, env, cwd) = lookup_opts(&options)?;
    let exts = if is_win32(&platform) {
        windows_path_exts(&env).unwrap_or_default()
    } else {
        Vec::new()
    };
    Ok(is_executable_in(
        &file_path,
        &platform,
        &exts,
        cwd.as_deref(),
    ))
}

/// Port of `resolveExecutable` (`packages/shared/src/executable.ts:230`).
pub fn resolve_executable(
    command: String,
    options: serde_json::Value,
) -> Result<Option<String>, String> {
    let opts = if is_undefined(&options) {
        Value::Object(Map::new())
    } else {
        options.clone()
    };
    let platform = read_member(&opts, "platform")?;
    let env = read_member(&opts, "env")?;
    let platform = if nullish(&platform) {
        Value::Null
    } else {
        platform
    };
    let env = if nullish(&env) { proc_env() } else { env };
    let cwd = match read_member(&opts, "cwd")? {
        Value::String(s) => Some(s),
        _ => None,
    };
    let allow_ext = truthy(&read_member(&opts, "allowExtensionlessOnWindows")?);
    let names = executable_name_candidates(
        command.clone(),
        platform.clone(),
        env.clone(),
        Some(allow_ext),
    )?;
    let exts = if is_win32(&platform) {
        windows_path_exts(&env).unwrap_or_default()
    } else {
        Vec::new()
    };
    if has_path_separator(command.clone())? {
        for name in names {
            if is_executable_in(&name, &platform, &exts, cwd.as_deref()) {
                return Ok(Some(name));
            }
        }
        return Ok(None);
    }
    for dir in path_entries(env, platform.clone())? {
        for name in &names {
            let candidate = posix_join(&[&dir, name]);
            if is_executable_in(&candidate, &platform, &exts, cwd.as_deref()) {
                return Ok(Some(candidate));
            }
        }
    }
    Ok(None)
}

/// Port of `executableIdentity` (`packages/shared/src/executable.ts:322`).
pub fn executable_identity(file_path: String) -> Result<Option<String>, String> {
    match std::fs::metadata(&file_path) {
        Ok(m) => {
            let size = m.len();
            let ms = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64() * 1000.0)
                .unwrap_or(f64::NAN);
            Ok(Some(format!("{}:{}", size, js::number_to_string(ms))))
        }
        Err(_) => Ok(None),
    }
}
