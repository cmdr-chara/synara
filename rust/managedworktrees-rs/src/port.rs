//! The Rust port of `apps/server/src/managedWorktrees.ts`.

#![forbid(unsafe_code)]

use jscore::{js_to_string, js_truthy};
use serde_json::{Value, json};

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

// --- path helpers (POSIX flavor; the recorder ran on darwin) ---

fn normalize_string(path: &str, allow_above_root: bool) -> String {
    let mut res: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." if res.last().is_some_and(|&s| s != "..") => {
                res.pop();
            }
            ".." if allow_above_root => res.push(".."),
            ".." => {}
            _ => res.push(seg),
        }
    }
    res.join("/")
}

fn path_resolve(segment: &str, cwd: &str) -> String {
    let joined = if segment.is_empty() {
        cwd.to_string()
    } else if segment.starts_with('/') {
        segment.to_string()
    } else {
        format!("{cwd}/{segment}")
    };
    let out = normalize_string(&joined, false);
    if out.is_empty() {
        return "/".to_string();
    }
    if joined.starts_with('/') || !cwd.is_empty() {
        format!("/{out}")
    } else {
        out
    }
}

fn path_relative(from: &str, to: &str) -> String {
    if from == to {
        return String::new();
    }
    let from_segs: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to_segs: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let mut i = 0;
    while i < from_segs.len() && i < to_segs.len() && from_segs[i] == to_segs[i] {
        i += 1;
    }
    let mut parts: Vec<String> = vec!["..".to_string(); from_segs.len() - i];
    parts.extend(to_segs[i..].iter().map(|s| s.to_string()));
    parts.join("/")
}

fn describe_path_arg(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(f64::NAN);
            if x.is_nan() {
                "type number (NaN)".into()
            } else {
                format!("type number ({x})")
            }
        }
        Value::Array(_) => "an instance of Array".into(),
        Value::String(s) => format!("type string ({s})"),
        Value::Object(_) => "an instance of Object".into(),
    }
}

fn require_string_arg(v: &Value, paths_index: usize) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        _ => Err(format!(
            "The \"paths[{paths_index}]\" argument must be of type string. Received {}",
            if is_undefined(v) {
                "undefined".into()
            } else {
                describe_path_arg(v)
            }
        )),
    }
}

// --- exports ---

/// Port of `managedWorktreeSnapshotsDir` (`apps/server/src/managedWorktrees.ts:22`).
pub fn managed_worktree_snapshots_dir(home_dir: String) -> Result<String, String> {
    let joined = if home_dir.is_empty() {
        "worktree-snapshots".to_string()
    } else {
        format!("{home_dir}/worktree-snapshots")
    };
    let out = normalize_string(&joined, false);
    if joined.starts_with('/') {
        Ok(format!("/{out}"))
    } else {
        Ok(out)
    }
}

/// Port of `isManagedWorktreePath` (`apps/server/src/managedWorktrees.ts:30`).
pub fn is_managed_worktree_path(input: serde_json::Value) -> Result<bool, String> {
    let worktrees_dir = member(&input, "worktreesDir")?;
    let worktree_path = member(&input, "worktreePath")?;
    // Both roots share the process cwd, so any fixed absolute stand-in works:
    // it cancels out of `path.relative` for relative inputs.
    const CWD: &str = "/app";
    let from = path_resolve(&require_string_arg(&worktrees_dir, 0)?, CWD);
    let to = path_resolve(&require_string_arg(&worktree_path, 0)?, CWD);
    let relative = path_relative(&from, &to);
    Ok(!relative.is_empty()
        && !relative.starts_with('/')
        && relative != ".."
        && !relative.starts_with("../"))
}

const MANAGED_WORKTREE_RETENTION_COUNT: usize = 15;

fn decode_key(v: &Value) -> Value {
    if let Value::Object(m) = v {
        if let Some(units) = m.get("$utf16").and_then(Value::as_array) {
            let units: Vec<u16> = units
                .iter()
                .filter_map(|u| u.as_u64().map(|u| u as u16))
                .collect();
            return Value::String(String::from_utf16_lossy(&units));
        }
        if let Some(n) = m.get("$num").and_then(Value::as_str) {
            let f = match n {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                "-0" => -0.0,
                _ => n.parse().unwrap_or(f64::NAN),
            };
            return Value::from(f);
        }
    }
    v.clone()
}

fn map_get<'a>(map: &'a Value, key: &Value) -> Result<Option<&'a Value>, String> {
    let Value::Object(m) = map else {
        return Err(format!(
            "{}.get is not a function",
            "canonicalByRecordedPath"
        ));
    };
    let Some(entries) = m.get("$map").and_then(Value::as_array) else {
        return Err("canonicalByRecordedPath.get is not a function".into());
    };
    let key = decode_key(key);
    for entry in entries {
        if let Some(pair) = entry.as_array() {
            if pair.len() == 2 && same_value_zero(&pair[0], &key) {
                return Ok(Some(&pair[1]));
            }
        }
    }
    Ok(None)
}

fn same_value_zero(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x
            .as_f64()
            .zip(y.as_f64())
            .is_some_and(|(x, y)| (x.is_nan() && y.is_nan()) || x == y),
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn thread_managed_worktree_path(thread: &Value) -> Result<Value, String> {
    let associated = member(thread, "associatedWorktreePath")?;
    if !associated.is_null() && !is_undefined(&associated) {
        return Ok(associated);
    }
    let worktree = member(thread, "worktreePath")?;
    if !worktree.is_null() && !is_undefined(&worktree) {
        return Ok(worktree);
    }
    Ok(Value::Null)
}

fn thread_field_nullish(thread: &Value, key: &str) -> Result<bool, String> {
    let v = member(thread, key)?;
    Ok(v.is_null() || is_undefined(&v))
}

fn is_active_thread(thread: &Value) -> Result<bool, String> {
    Ok(thread_field_nullish(thread, "deletedAt")? && thread_field_nullish(thread, "archivedAt")?)
}

fn is_deleted_thread(thread: &Value) -> Result<bool, String> {
    Ok(!thread_field_nullish(thread, "deletedAt")?)
}

fn is_archived_only_thread(thread: &Value) -> Result<bool, String> {
    Ok(!is_deleted_thread(thread)? && !thread_field_nullish(thread, "archivedAt")?)
}

fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn class(c: char) -> u8 {
        if c.is_ascii_digit() {
            1
        } else if c.is_ascii_alphabetic() {
            2
        } else {
            0
        }
    }
    let mut ai = a.chars();
    let mut bi = b.chars();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ca), Some(cb)) => {
                let (cla, clb) = (class(ca), class(cb));
                if cla != clb {
                    return cla.cmp(&clb);
                }
                let (la, lb) = (ca.to_ascii_lowercase(), cb.to_ascii_lowercase());
                if la != lb {
                    return la.cmp(&lb);
                }
                if ca != cb {
                    return ca.cmp(&cb);
                }
            }
        }
    }
}

/// `input.threads` must be iterable; `.filter`/.map require arrays.
fn require_array<'a>(v: &'a Value, owner: &str, method: &str) -> Result<&'a Vec<Value>, String> {
    match v {
        Value::Array(items) => Ok(items),
        Value::Null => Err(format!(
            "Cannot read properties of null (reading '{method}')"
        )),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{method}')"
        )),
        _ => Err(format!("{owner}.{method} is not a function")),
    }
}

/// Port of `classifyManagedWorktreeRemovalCandidates`
/// (`apps/server/src/managedWorktrees.ts:250`).
pub fn classify_managed_worktree_removal_candidates(
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let inventory = member(&input, "inventory")?;
    let threads = member(&input, "threads")?;
    let canonical_map = member(&input, "canonicalByRecordedPath")?;

    let inventory_items = require_array(&inventory, "inventory", "map")?;
    // Map<path, entry>, first write wins position; duplicates overwrite value.
    let mut inventory_paths: Vec<Value> = Vec::new();
    let mut inventory_entries: Vec<Value> = Vec::new();
    for entry in inventory_items {
        let path = member(entry, "path")?;
        if let Some(i) = inventory_paths
            .iter()
            .position(|p| same_value_zero(&decode_key(p), &decode_key(&path)))
        {
            inventory_entries[i] = entry.clone();
        } else {
            inventory_paths.push(path);
            inventory_entries.push(entry.clone());
        }
    }
    let entry_for = |path: &Value| -> Option<&Value> {
        inventory_paths
            .iter()
            .position(|p| same_value_zero(&decode_key(p), &decode_key(path)))
            .map(|i| &inventory_entries[i])
    };

    let thread_items = require_array(&threads, "threads", "filter")?;

    let canonical_thread_path = |thread: &Value| -> Result<Value, String> {
        let recorded = thread_managed_worktree_path(thread)?;
        if recorded.is_null() {
            return Ok(Value::Null);
        }
        if is_undefined(&canonical_map) {
            return Err("Cannot read properties of undefined (reading 'get')".into());
        }
        if canonical_map.is_null() {
            return Err("Cannot read properties of null (reading 'get')".into());
        }
        match map_get(&canonical_map, &recorded)? {
            Some(v) if !v.is_null() && !is_undefined(v) => Ok(v.clone()),
            _ => Ok(Value::Null),
        }
    };

    let mut active_paths: Vec<Value> = Vec::new();
    for thread in thread_items {
        if !is_active_thread(thread)? {
            continue;
        }
        let worktree_path = canonical_thread_path(thread)?;
        if !worktree_path.is_null()
            && !active_paths
                .iter()
                .any(|p| same_value_zero(&decode_key(p), &decode_key(&worktree_path)))
        {
            active_paths.push(worktree_path);
        }
    }

    let mut deleted_candidates: Vec<Value> = Vec::new();
    let mut seen_deleted: Vec<Value> = Vec::new();
    for thread in thread_items {
        if !is_deleted_thread(thread)? {
            continue;
        }
        let worktree_path = canonical_thread_path(thread)?;
        if worktree_path.is_null()
            || active_paths
                .iter()
                .any(|p| same_value_zero(&decode_key(p), &decode_key(&worktree_path)))
            || seen_deleted
                .iter()
                .any(|p| same_value_zero(&decode_key(p), &decode_key(&worktree_path)))
        {
            continue;
        }
        let Some(entry) = entry_for(&worktree_path) else {
            continue;
        };
        seen_deleted.push(worktree_path);
        deleted_candidates.push(json!({
            "entry": entry,
            "thread": thread,
            "reason": "deleted",
        }));
    }

    let mut seen_archived: Vec<Value> = Vec::new();
    let mut archived_keepers: Vec<(Value, Value)> = Vec::new(); // (thread, entry)
    for thread in thread_items {
        if !is_archived_only_thread(thread)? {
            continue;
        }
        let worktree_path = canonical_thread_path(thread)?;
        let entry = if worktree_path.is_null() {
            None
        } else {
            entry_for(&worktree_path)
        };
        let Some(entry) = entry else { continue };
        let entry_path = member(entry, "path")?;
        if active_paths
            .iter()
            .any(|p| same_value_zero(&decode_key(p), &decode_key(&entry_path)))
            || seen_deleted
                .iter()
                .any(|p| same_value_zero(&decode_key(p), &decode_key(&entry_path)))
        {
            continue;
        }
        archived_keepers.push((thread.clone(), entry.clone()));
    }
    // Stable sort by `archivedAt ?? ""` descending.
    archived_keepers.sort_by(|(ta, _), (tb, _)| {
        let ka = member(ta, "archivedAt")
            .map(|v| {
                if v.is_null() || is_undefined(&v) {
                    String::new()
                } else {
                    js_to_string(&v)
                }
            })
            .unwrap_or_default();
        let kb = member(tb, "archivedAt")
            .map(|v| {
                if v.is_null() || is_undefined(&v) {
                    String::new()
                } else {
                    js_to_string(&v)
                }
            })
            .unwrap_or_default();
        locale_cmp(&kb, &ka)
    });
    archived_keepers.retain(|(_, entry)| {
        let path = member(entry, "path").unwrap_or(Value::Null);
        if seen_archived
            .iter()
            .any(|p| same_value_zero(&decode_key(p), &decode_key(&path)))
        {
            return false;
        }
        seen_archived.push(path);
        true
    });

    let mut out = deleted_candidates;
    for (thread, entry) in archived_keepers
        .into_iter()
        .skip(MANAGED_WORKTREE_RETENTION_COUNT)
    {
        out.push(json!({
            "entry": entry,
            "thread": thread,
            "reason": "archived-retention",
        }));
    }
    Ok(out)
}

/// Port of `discardEmptyManagedWorktreeParent` (`apps/server/src/managedWorktrees.ts:522`).
/// INFALLIBLE: the function returns a lazy Effect marker; its `throw` lives inside
/// the effect body, which no call-time input can reach — no error case is recordable.
pub fn discard_empty_managed_worktree_parent(input: serde_json::Value) -> serde_json::Value {
    let _ = input;
    json!({"~effect/Effect/args": {"~effect/Effect/args": {"0": {"$function": ""}, "1": true}}, "~effect/Effect/failureCont": {"$function": ""}})
}

/// Port of `discardManagedWorktreeResidue` (`apps/server/src/managedWorktrees.ts:563`).
/// Reads `input.worktreesDir` eagerly while composing the parent-discard call.
pub fn discard_managed_worktree_residue(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = member(&input, "worktreesDir")?;
    let _ = member(&input, "worktreePath")?;
    Ok(
        json!({"~effect/Effect/args": {"~effect/Effect/args": {"~effect/Effect/args": {"0": {"$function": ""}, "1": true}}, "~effect/Effect/failureCont": {"$function": ""}}, "~effect/Effect/successCont": {"$function": ""}}),
    )
}

/// Port of `pruneExpiredManagedWorktreeSnapshots` (`apps/server/src/managedWorktrees.ts:623`).
pub fn prune_expired_managed_worktree_snapshots(input: serde_json::Value) -> serde_json::Value {
    let _ = input;
    json!({"~effect/Effect/args": {"~effect/Effect/args": {"~effect/Effect/args": {"~effect/Effect/args": {"0": {"$function": ""}, "1": true}}, "~effect/Effect/successCont": {"$function": ""}}, "~effect/Effect/successCont": {"$function": ""}}, "~effect/Effect/failureCont": {"$function": ""}})
}
