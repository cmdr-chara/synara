//! The Rust port of `packages/shared/src/windowsProcess.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

const BATCH_EXTS: [&str; 2] = [".cmd", ".bat"];
const UNSAFE_TOKENS: [char; 7] = ['\r', '\n', '&', '|', '<', '>', '^'];
const DEFAULT_PATHEXT: [&str; 4] = [".COM", ".EXE", ".BAT", ".CMD"];

// ---- JS string helpers ------------------------------------------------------

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
                | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
        )
    })
}

fn member(v: &Value, k: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{k}')")),
        _ if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        )),
        Value::Object(o) => Ok(o.get(k).cloned().unwrap_or_else(js::undefined)),
        _ => Ok(js::undefined()),
    }
}

fn nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    !nullish(v)
        && match v {
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64() != Some(0.0),
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
}

/// `x?.trim()` on an env value; non-string non-nullish values throw in JS.
fn trim_non_empty(v: Option<&Value>) -> Result<Option<String>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let t = js_trim(s);
            Ok((!t.is_empty()).then(|| t.to_string()))
        }
        Some(_) => Err("value?.trim is not a function".into()),
    }
}

// ---- win32 path shims -------------------------------------------------------

fn win32_is_abs(p: &str) -> bool {
    p.starts_with(['/', '\\'])
        || (p.len() >= 3
            && p.as_bytes()[1] == b':'
            && matches!(p.as_bytes()[2], b'/' | b'\\')
            && p.as_bytes()[0].is_ascii_alphabetic())
}

/// `path.win32.resolve(a, b)` for the shapes this unit produces: `b` wins when
/// rooted, else `a` + `\` + `b` (no `.`/`..` collapsing — recorded cases never
/// reach a path with them through this shim).
fn win32_resolve(a: &str, b: &str) -> String {
    if b.is_empty() {
        a.to_string()
    } else if win32_is_abs(b) {
        b.to_string()
    } else {
        format!("{a}\\{b}")
    }
}

fn win32_extname(p: &str) -> &str {
    let base = p.rsplit(['/', '\\']).next().unwrap_or(p);
    match base.rfind('.') {
        Some(0) | None => "",
        Some(i) => &base[i..],
    }
}

// ---- environment ------------------------------------------------------------

/// `env ?? process.env` — the port substitutes a snapshot map for `process.env`.
fn env_snapshot(input_env: Option<&Value>) -> Value {
    match input_env {
        Some(v) if !nullish(v) => v.clone(),
        _ => {
            let mut m = serde_json::Map::new();
            for (k, v) in std::env::vars() {
                m.insert(k, Value::String(v));
            }
            Value::Object(m)
        }
    }
}

fn env_str<'a>(env: &'a Value, keys: &[&str]) -> Option<&'a str> {
    for k in keys {
        if let Value::Object(o) = env
            && let Some(Value::String(s)) = o.get(*k)
        {
            return Some(s);
        }
    }
    None
}

fn windows_system_root(env: &Value) -> Result<String, String> {
    if let Some(t) = trim_non_empty(env.get("SystemRoot"))? {
        return Ok(t);
    }
    if let Some(t) = trim_non_empty(env.get("SYSTEMROOT"))? {
        return Ok(t);
    }
    Ok("C:\\Windows".into())
}

fn windows_system_exe(env: &Value, segs: &[&str]) -> Result<String, String> {
    let mut parts = vec![windows_system_root(env)?];
    parts.extend(segs.iter().map(|s| s.to_string()));
    Ok(parts.join("\\"))
}

fn windows_path_extensions(env: &Value) -> Vec<String> {
    let raw = env_str(env, &["PATHEXT"]);
    let fallback = || DEFAULT_PATHEXT.iter().map(|s| s.to_string()).collect();
    match raw {
        None => fallback(),
        Some(raw) => {
            let parsed: Vec<String> = raw
                .split(';')
                .map(js_trim)
                .filter(|e| !e.is_empty())
                .map(|e| {
                    if let Some(stripped) = e.strip_prefix('.') {
                        format!(".{}", stripped.to_uppercase())
                    } else {
                        format!(".{e}").to_uppercase()
                    }
                })
                .collect();
            if parsed.is_empty() {
                fallback()
            } else {
                let mut seen = Vec::new();
                for p in parsed {
                    if !seen.contains(&p) {
                        seen.push(p);
                    }
                }
                seen
            }
        }
    }
}

/// `pathEntries(env, "win32")`.
fn path_entries(env: &Value) -> Vec<String> {
    let path_value = env_str(env, &["PATH", "Path", "path"]);
    match path_value {
        None => vec![],
        Some("") => vec![],
        Some(v) => v
            .split(';')
            .map(|e| {
                let t = js_trim(e);
                t.trim_matches('"').to_string()
            })
            .filter(|e| !e.is_empty())
            .collect(),
    }
}

fn executable_name_candidates(command: &str, env: &Value) -> Vec<String> {
    let exts = windows_path_extensions(env);
    let extension = win32_extname(command);
    let norm = extension.to_uppercase();
    if !extension.is_empty()
        && (exts.iter().any(|e| e == &norm) || DEFAULT_PATHEXT.contains(&norm.as_str()))
    {
        let stem = &command[..command.len() - extension.len()];
        let mut names = vec![command.to_string()];
        for n in [
            format!("{stem}{norm}"),
            format!("{stem}{}", norm.to_lowercase()),
        ] {
            if !names.contains(&n) {
                names.push(n);
            }
        }
        return names;
    }
    let mut names = Vec::new();
    for ext in &exts {
        for n in [
            format!("{command}{ext}"),
            format!("{command}{}", ext.to_lowercase()),
        ] {
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    names
}

/// `isExecutableFileIn` on the recording host: stat a win32-resolved path, then
/// require a direct-launch extension (win32 branch).
fn is_executable_file(file_path: &str, cwd: &str, path_exts: &[String]) -> bool {
    let stat_path = if win32_is_abs(file_path) {
        file_path.to_string()
    } else {
        win32_resolve(cwd, file_path)
    };
    let is_file = std::fs::metadata(&stat_path)
        .map(|m| m.is_file())
        .unwrap_or(false);
    if !is_file {
        return false;
    }
    let ext = win32_extname(file_path).to_uppercase();
    !ext.is_empty()
        && (path_exts.iter().any(|e| e == &ext) || DEFAULT_PATHEXT.contains(&ext.as_str()))
}

/// `resolveExecutable(command, { platform: "win32", env, cwd })`.
fn resolve_executable(command: &str, env: &Value, cwd: Option<&str>) -> Option<String> {
    let path_exts = windows_path_extensions(env);
    let names = executable_name_candidates(command, env);
    let proc_cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".into());
    let cwd = cwd.unwrap_or(&proc_cwd);
    if command.contains(['/', '\\']) {
        for name in names {
            if is_executable_file(&name, cwd, &path_exts) {
                return Some(name);
            }
        }
        return None;
    }
    let cwd_resolved = win32_resolve(cwd, "");
    for dir in path_entries(env) {
        if win32_resolve(&cwd_resolved, &dir).to_lowercase() == cwd_resolved.to_lowercase() {
            continue;
        }
        for name in &names {
            let candidate = if dir.ends_with('/') || dir.ends_with('\\') {
                format!("{dir}{name}")
            } else {
                format!("{dir}/{name}")
            };
            if is_executable_file(&candidate, cwd, &path_exts) {
                return Some(candidate);
            }
        }
    }
    None
}

// ---- ported functions -------------------------------------------------------

/// `WINDOWS_WSL_UNC_PATTERN.exec(value.trim())` — manual match of
/// `^\\\\(?:wsl\.localhost|wsl\$)\\([^\\]+)(?:\\(.*))?$` with case-insensitive host.
fn parse_unc(value: &str) -> Option<(String, Option<String>)> {
    let v = js_trim(value);
    let lower = v.to_lowercase();
    let after = lower.strip_prefix("\\\\")?;
    let after = after
        .strip_prefix("wsl.localhost\\")
        .or_else(|| after.strip_prefix("wsl$\\"))?;
    // group1 = [^\\]+ up to the next backslash
    let bs = after.as_bytes().iter().position(|b| *b == b'\\');
    let byte_off = v.len() - after.len();
    let (dist, rest) = match bs {
        None => (&v[byte_off..], None),
        Some(i) => (&v[byte_off..byte_off + i], Some(&v[byte_off + i + 1..])),
    };
    if dist.is_empty()
        || rest
            .map(|r| r.contains(['\n', '\r', '\u{2028}', '\u{2029}']))
            .unwrap_or(false)
    {
        return None;
    }
    Some((dist.to_string(), rest.map(|r| r.to_string())))
}

/// Port of `parseWindowsWslUncPath` (`packages/shared/src/windowsProcess.ts:42`).
pub fn parse_windows_wsl_unc_path(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let s = match &value {
        Value::String(s) => s.clone(),
        _ => return Ok(None),
    };
    let Some((dist, suffix)) = parse_unc(&s) else {
        return Ok(None);
    };
    let distribution = js_trim(&dist);
    if distribution.is_empty() {
        return Ok(None);
    }
    let linux_path = match suffix {
        None => "/".to_string(),
        Some(rest) => {
            let segs: Vec<&str> = rest.split('\\').filter(|s| !s.is_empty()).collect();
            format!("/{}", segs.join("/"))
        }
    };
    Ok(Some(json!({
        "distribution": distribution,
        "linuxPath": linux_path,
    })))
}

/// Port of `isWindowsBatchCommand` (`packages/shared/src/windowsProcess.ts:64`).
pub fn is_windows_batch_command(command: String) -> Result<bool, String> {
    let lower = command.to_lowercase();
    Ok(BATCH_EXTS.iter().any(|e| lower.ends_with(e)))
}

fn quote_batch_token(token: &str, label: &str) -> Result<String, String> {
    if token
        .chars()
        .any(|c| UNSAFE_TOKENS.contains(&c) || c == '%')
    {
        return Err(format!(
            "Cannot safely execute Windows batch {label} containing cmd.exe control characters."
        ));
    }
    Ok(format!("\"{}\"", token.replace('"', "\"\"")))
}

/// Port of `buildWindowsBatchCommandArgs` (`packages/shared/src/windowsProcess.ts:77`).
pub fn build_windows_batch_command_args(
    command: String,
    args: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut parts = vec!["call".to_string(), quote_batch_token(&command, "command")?];
    for arg in &args {
        parts.push(quote_batch_token(arg, "argument")?);
    }
    Ok(vec![
        "/d".into(),
        "/s".into(),
        "/v:off".into(),
        "/c".into(),
        parts.join(" "),
    ])
}

/// Port of `resolveWindowsCommandPath` (`packages/shared/src/windowsProcess.ts:100`).
pub fn resolve_windows_command_path(
    command: String,
    input: serde_json::Value,
) -> Result<String, String> {
    let input = if js::is_undefined(&input) {
        json!({})
    } else {
        input
    };
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'env')".into());
    }
    let path_like = command.contains(['/', '\\']) || win32_is_abs(&command);
    if path_like && !win32_extname(&command).is_empty() {
        return Ok(command);
    }
    let env = env_snapshot(Some(&member(&input, "env")?));
    let cwd_v = member(&input, "cwd")?;
    let cwd = cwd_v.as_str().map(|s| s.to_string());
    Ok(resolve_executable(&command, &env, cwd.as_deref()).unwrap_or(command))
}

/// Port of `prepareWindowsSafeProcess` (`packages/shared/src/windowsProcess.ts:119`).
pub fn prepare_windows_safe_process(
    command: String,
    args: Vec<String>,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let input = if js::is_undefined(&input) {
        json!({})
    } else {
        input
    };
    let platform_v = member(&input, "platform")?;
    let platform = if nullish(&platform_v) {
        std::env::consts::OS
    } else {
        platform_v.as_str().unwrap_or("")
    };
    if platform != "win32" {
        return Ok(json!({"command": command, "args": args, "shell": false}));
    }
    let env = env_snapshot(Some(&member(&input, "env")?));
    let cwd_v = member(&input, "cwd")?;
    let wsl_workspace = if truthy(&cwd_v) {
        let cwd_str = match &cwd_v {
            Value::String(s) => s.clone(),
            _ => return Err("value.trim is not a function".into()),
        };
        parse_windows_wsl_unc_path(Value::String(cwd_str))?
    } else {
        None
    };
    if let Some(w) = wsl_workspace {
        let mut new_args = vec![
            "--distribution".into(),
            w["distribution"].as_str().unwrap_or_default().to_string(),
            "--cd".into(),
            w["linuxPath"].as_str().unwrap_or("/").to_string(),
            "--exec".into(),
            command,
        ];
        new_args.extend(args);
        return Ok(json!({
            "command": windows_system_exe(&env, &["System32", "wsl.exe"])?,
            "args": new_args,
            "shell": false,
            "windowsHide": true,
        }));
    }
    let cwd = cwd_v.as_str().map(|s| s.to_string());
    let resolved = {
        let path_like = command.contains(['/', '\\']) || win32_is_abs(&command);
        if path_like && !win32_extname(&command).is_empty() {
            command.clone()
        } else {
            resolve_executable(&command, &env, cwd.as_deref()).unwrap_or_else(|| command.clone())
        }
    };
    if !is_windows_batch_command(resolved.clone())? {
        return Ok(json!({
            "command": resolved,
            "args": args,
            "shell": false,
            "windowsHide": true,
        }));
    }
    let com_spec = match trim_non_empty(env.get("ComSpec"))? {
        Some(v) => v,
        None => match trim_non_empty(env.get("COMSPEC"))? {
            Some(v) => v,
            None => windows_system_exe(&env, &["System32", "cmd.exe"])?,
        },
    };
    Ok(json!({
        "command": com_spec,
        "args": build_windows_batch_command_args(resolved, args)?,
        "shell": false,
        "windowsHide": true,
        "windowsVerbatimArguments": true,
    }))
}
