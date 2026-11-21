//! The Rust port of `apps/web/src/worktreeCleanup.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => is_undefined(a) && is_undefined(b),
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn normalize_worktree_path(v: &Value) -> Option<String> {
    let s = js_trim(v.as_str()?);
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Port of `getOrphanedWorktreePathForThread`
/// (`apps/web/src/worktreeCleanup.ts:11`).
pub fn get_orphaned_worktree_path_for_thread(
    threads: Vec<serde_json::Value>,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut target: Option<&Value> = None;
    for t in &threads {
        let id = member(t, "id")?;
        if strict_eq(&id, &thread_id) {
            target = Some(t);
            break;
        }
    }
    let Some(target_thread) = target else {
        return Ok(Value::Null);
    };
    let target_path = match normalize_worktree_path(&member(target_thread, "worktreePath")?) {
        Some(p) => p,
        None => return Ok(Value::Null),
    };
    let shared = threads.iter().any(|t| {
        if member(t, "id")
            .map(|i| strict_eq(&i, &thread_id))
            .unwrap_or(false)
        {
            return false;
        }
        normalize_worktree_path(&member(t, "worktreePath").unwrap_or(Value::Null))
            .map(|p| p == target_path)
            .unwrap_or(false)
    });
    Ok(if shared {
        Value::Null
    } else {
        Value::String(target_path)
    })
}

/// Port of `isThreadAssociatedWithWorktree`
/// (`apps/web/src/worktreeCleanup.ts:41`).
pub fn is_thread_associated_with_worktree(
    thread: serde_json::Value,
    worktree_path: String,
) -> Result<bool, String> {
    for key in ["worktreePath", "associatedWorktreePath"] {
        if let Some(p) = normalize_worktree_path(&member(&thread, key)?)
            && p == worktree_path
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `formatWorktreePathForDisplay` (`apps/web/src/worktreeCleanup.ts:51`).
pub fn format_worktree_path_for_display(worktree_path: String) -> Result<String, String> {
    let trimmed = js_trim(&worktree_path);
    if trimmed.is_empty() {
        return Ok(worktree_path);
    }
    let normalized: String = trimmed.replace('\\', "/");
    let normalized = normalized.trim_end_matches('/');
    let last = js_trim(normalized.rsplit('/').next().unwrap_or(""));
    Ok(if last.is_empty() {
        trimmed.to_string()
    } else {
        last.to_string()
    })
}
