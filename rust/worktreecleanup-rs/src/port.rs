//! Clean-room implementation of the worktree-cleanup helpers
//! (`apps/web/src/worktreeCleanup.ts`), derived exclusively from the
//! recorded input/output pairs in `cases.jsonl`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `value[key]` with JS semantics: null/undefined receivers raise the
/// canonical TypeError, non-objects yield undefined, a missing key on an
/// object yields the undefined marker.
fn get_prop(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

/// `===` on wire values: numbers through the `$num` marker, then
/// per-type equality, then two undefined markers are equal.
fn same_value(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => undefined(a) && undefined(b),
    }
}

/// `String.prototype.trim`: Unicode whitespace plus U+FEFF on both ends.
fn trim_ws(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// A path is usable when, after trimming, it is a non-empty string.
fn usable_path(v: &Value) -> Option<String> {
    let s = trim_ws(v.as_str()?);
    (!s.is_empty()).then(|| s.to_string())
}

/// Returns the worktree path of the thread whose `id` is `thread_id`,
/// provided no other thread claims the same path; `null` otherwise.
pub fn get_orphaned_worktree_path_for_thread(
    threads: Vec<serde_json::Value>,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut found: Option<&Value> = None;
    for element in &threads {
        let element_id = get_prop(element, "id")?;
        if same_value(&element_id, &thread_id) {
            found = Some(element);
            break;
        }
    }
    let Some(target) = found else {
        return Ok(Value::Null);
    };
    let Some(path) = usable_path(&get_prop(target, "worktreePath")?) else {
        return Ok(Value::Null);
    };
    let claimed_by_sibling = threads.iter().any(|element| {
        let is_target = get_prop(element, "id")
            .map(|i| same_value(&i, &thread_id))
            .unwrap_or(false);
        if is_target {
            return false;
        }
        usable_path(&get_prop(element, "worktreePath").unwrap_or(Value::Null))
            .is_some_and(|p| p == path)
    });
    Ok(if claimed_by_sibling {
        Value::Null
    } else {
        Value::String(path)
    })
}

/// Whether either `worktreePath` or `associatedWorktreePath` on the
/// thread, trimmed, equals the given path.
pub fn is_thread_associated_with_worktree(
    thread: serde_json::Value,
    worktree_path: String,
) -> Result<bool, String> {
    for key in ["worktreePath", "associatedWorktreePath"] {
        if let Some(p) = usable_path(&get_prop(&thread, key)?)
            && p == worktree_path
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Display form of a worktree path: trimmed, backslashes normalized to
/// `/`, reduced to the last non-empty `/`-separated segment; falls back
/// to the trimmed input when no usable segment remains.
pub fn format_worktree_path_for_display(worktree_path: String) -> Result<String, String> {
    let trimmed = trim_ws(&worktree_path);
    if trimmed.is_empty() {
        return Ok(worktree_path);
    }
    let mut unified = trimmed.replace('\\', "/");
    while unified.ends_with('/') {
        unified.pop();
    }
    let tail = trim_ws(unified.rsplit('/').next().unwrap_or(""));
    Ok(if tail.is_empty() {
        trimmed.to_string()
    } else {
        tail.to_string()
    })
}
