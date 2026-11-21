//! The Rust port of `packages/shared/src/threadEnvironment.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `input.field` — `Err` for null/undefined receivers, `None` for non-objects or absent keys.
fn field<'a>(input: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    match input {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(input) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(o) => Ok(o.get(key)),
        _ => Ok(None),
    }
}

/// JS truthiness.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN) != 0.0,
        Value::String(s) => !s.is_empty(),
        _ if is_undefined(v) => false,
        _ if crate::js::as_number(v).is_some() => {
            crate::js::as_number(v).unwrap_or(f64::NAN) != 0.0
        }
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            !crate::js::string_units(v).is_none_or(|u| u.is_empty())
        }
        _ => true,
    }
}

/// `a ?? b`: null/undefined (or absent) selects the fallback; everything else passes through.
fn nullish(v: Option<&Value>) -> Option<&Value> {
    match v {
        Some(Value::Null) | None => None,
        Some(x) if is_undefined(x) => None,
        other => other,
    }
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

/// `isLikelyWindowsWorkspaceRoot(value, platform)` with `platform` always absent here.
fn is_likely_windows_root(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() >= 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b.len() == 2 || b[2] == b'\\' || b[2] == b'/')
    {
        return true;
    }
    value.starts_with("\\\\") || value.starts_with("//")
}

/// `normalizeWorkspaceRootForComparison(value)` with no options.
fn normalize_root(value: &str) -> String {
    let trimmed = value.trim_matches(is_js_space);
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd = trimmed.replace('\\', "/");
    let bytes = fwd.as_bytes();
    let is_win_drive_root = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && fwd[2..].chars().all(|c| c == '/');
    let has_unc = fwd.starts_with("//");
    let prefix = if has_unc {
        "//"
    } else if fwd.starts_with('/') {
        "/"
    } else {
        ""
    };
    let mut body = String::with_capacity(fwd.len());
    let mut last_slash = false;
    for c in fwd[prefix.len()..].chars() {
        if c == '/' {
            if !last_slash {
                body.push(c);
            }
            last_slash = true;
        } else {
            body.push(c);
            last_slash = false;
        }
    }
    let body = body.trim_end_matches('/');
    let normalized = if prefix.is_empty() {
        body.to_string()
    } else {
        format!("{prefix}{body}")
    };
    let final_value = if is_win_drive_root {
        format!("{}/", &fwd[..2])
    } else if !normalized.is_empty() {
        normalized
    } else {
        prefix.to_string()
    };
    if is_likely_windows_root(trimmed) {
        final_value.to_lowercase()
    } else {
        final_value
    }
}

/// `isWorkspaceRootWithin(candidate, ancestorRoot)` (no options).
fn root_within(candidate: &str, ancestor: &str) -> bool {
    let nc = normalize_root(candidate);
    let na = normalize_root(ancestor);
    if nc.is_empty() || na.is_empty() {
        return false;
    }
    if nc == na {
        return true;
    }
    let prefix = if na.ends_with('/') {
        na
    } else {
        format!("{na}/")
    };
    nc.starts_with(&prefix)
}

/// The `input.worktreePath` truthiness gate shared by the mode/state helpers.
fn worktree_truthy(input: &Value) -> Result<bool, String> {
    Ok(field(input, "worktreePath")?.is_some_and(truthy))
}

fn mode_of(input: &Value) -> Result<Option<Value>, String> {
    if worktree_truthy(input)? {
        return Ok(Some(Value::String("worktree".to_string())));
    }
    Ok(match nullish(field(input, "envMode")?) {
        Some(v) => Some(v.clone()),
        None => Some(Value::String("local".to_string())),
    })
}

fn mode_is_local(mode: &Value) -> bool {
    matches!(mode, Value::String(s) if s == "local")
}

/// Port of `resolveThreadEnvironmentMode` (`packages/shared/src/threadEnvironment.ts:5`).
pub fn resolve_thread_environment_mode(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(mode_of(&input)?.unwrap_or(Value::Null))
}

/// Port of `resolveThreadWorkspaceState` (`packages/shared/src/threadEnvironment.ts:14`).
pub fn resolve_thread_workspace_state(input: serde_json::Value) -> Result<String, String> {
    let mode = mode_of(&input)?.unwrap_or(Value::Null);
    if mode_is_local(&mode) {
        return Ok("local".to_string());
    }
    Ok(if worktree_truthy(&input)? {
        "worktree-ready"
    } else {
        "worktree-pending"
    }
    .to_string())
}

/// Port of `isPendingThreadWorktree` (`packages/shared/src/threadEnvironment.ts:24`).
pub fn is_pending_thread_worktree(input: serde_json::Value) -> Result<bool, String> {
    Ok(resolve_thread_workspace_state(input)? == "worktree-pending")
}

/// Port of `resolveThreadWorkspaceCwd` (`packages/shared/src/threadEnvironment.ts:32`).
pub fn resolve_thread_workspace_cwd(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let mode = mode_of(&input)?.unwrap_or(Value::Null);
    if !mode_is_local(&mode) {
        let worktree = field(&input, "worktreePath")?;
        let wd = nullish(field(&input, "workingDirectory")?);
        if let (Some(wt), Some(dir)) = (worktree.filter(|w| truthy(w)), wd.filter(|d| truthy(d)))
            && let (Some(wt_s), Some(dir_s)) = (as_str(wt), as_str(dir))
        {
            let no_dotdot = !dir_s.replace('\\', "/").split('/').any(|seg| seg == "..");
            if no_dotdot && root_within(&dir_s, &wt_s) {
                return Ok(Some(dir.clone()));
            }
        }
        return Ok(worktree.cloned().or(Some(Value::Null)));
    }
    Ok(nullish(field(&input, "workingDirectory")?)
        .or_else(|| nullish(field(&input, "projectCwd").ok().flatten()))
        .cloned()
        .or(Some(Value::Null)))
}

fn as_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            crate::js::string_units(v).and_then(|u| String::from_utf16(&u).ok())
        }
        _ => None,
    }
}

/// Port of `resolveThreadBranchSourceCwd` (`packages/shared/src/threadEnvironment.ts:58`).
pub fn resolve_thread_branch_source_cwd(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    Ok(nullish(field(&input, "worktreePath")?)
        .or_else(|| nullish(field(&input, "projectCwd").ok().flatten()))
        .cloned()
        .or(Some(Value::Null)))
}
