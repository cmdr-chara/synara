//! The Rust port of `apps/server/src/threadRetention.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::sync::LazyLock;

const THREAD_RETENTION_UNUSED_MS: f64 = 7.0 * 24.0 * 60.0 * 60.0 * 1000.0;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

static UNDEF: LazyLock<Value> = LazyLock::new(crate::js::undefined);

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn js_falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|x| x == 0.0 || x.is_nan()),
        Value::String(s) => s.is_empty(),
        Value::Object(_) => is_undefined(v) || crate::js::as_number(v).is_some_and(|x| x.is_nan()),
        Value::Array(_) => false,
    }
}

fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.trim().parse().unwrap_or(f64::NAN),
        Value::Object(_) => {
            if is_undefined(v) {
                f64::NAN
            } else {
                crate::js::as_number(v).unwrap_or(f64::NAN)
            }
        }
        Value::Array(_) => f64::NAN,
    }
}

/// What `for...of x` yields on the wire: arrays → items, strings → chars,
/// `{$map}` → [k,v] pairs, `{$set}` → items.
fn iter_items(v: &Value) -> Option<Vec<Value>> {
    match v {
        Value::Array(a) => Some(a.clone()),
        Value::String(s) => Some(s.chars().map(|c| json!(c.to_string())).collect()),
        Value::Object(m) => {
            if is_undefined(v) {
                return None;
            }
            if let Some(Value::Array(pairs)) = m.get("$map") {
                return Some(pairs.clone());
            }
            if let Some(Value::Array(items)) = m.get("$set") {
                return Some(items.clone());
            }
            None
        }
        _ => None,
    }
}

fn js_same_value(a: &Value, b: &Value) -> bool {
    match (crate::js::as_number(a), crate::js::as_number(b)) {
        (Some(x), Some(y)) => x == y || (x.is_nan() && y.is_nan()),
        _ => a == b,
    }
}

fn set_contains(set: &[Value], key: &Value) -> bool {
    set.iter().any(|k| js_same_value(k, key))
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}

/// `Date.parse` ISO subset (the recorded corpus never exercises lenient parses here).
fn iso_date_parse(s: &str) -> f64 {
    let s = s.trim();
    let b = s.as_bytes();
    let num_at = |i: usize, n: usize| -> Option<i64> {
        b.get(i..i + n)
            .filter(|w| w.iter().all(|c| c.is_ascii_digit()))
            .and_then(|w| std::str::from_utf8(w).ok()?.parse().ok())
    };
    let (y, mo, d) = match (num_at(0, 4), b.get(4), num_at(5, 2), b.get(7), num_at(8, 2)) {
        (Some(y), Some(b'-'), Some(mo), Some(b'-'), Some(d)) if (1..=12).contains(&mo) => {
            (y, mo, d)
        }
        _ => return f64::NAN,
    };
    if !(1..=31).contains(&d) || d > days_in_month(y, mo) {
        return f64::NAN;
    }
    if b.len() == 10 {
        return (days_from_civil(y, mo, d) * 86_400_000) as f64;
    }
    if !matches!(b.get(10), Some(b'T') | Some(b' ')) {
        return f64::NAN;
    }
    let (hh, mm) = match (num_at(11, 2), b.get(13), num_at(14, 2)) {
        (Some(h), Some(b':'), Some(m)) if h <= 24 && m <= 59 => (h, m),
        _ => return f64::NAN,
    };
    let mut idx = 16;
    let mut ss = 0i64;
    let mut frac = 0.0f64;
    if b.get(idx) == Some(&b':') {
        match num_at(idx + 1, 2) {
            Some(s2) if s2 <= 60 => ss = s2,
            _ => return f64::NAN,
        }
        idx += 3;
        if b.get(idx) == Some(&b'.') {
            let start = idx + 1;
            let mut end = start;
            while b.get(end).is_some_and(|c| c.is_ascii_digit()) {
                end += 1;
            }
            match std::str::from_utf8(&b[start..end])
                .ok()
                .filter(|t| !t.is_empty())
                .and_then(|t| t[..t.len().min(3)].parse::<f64>().ok())
            {
                Some(f) => frac = f / 10f64.powi(end.min(start + 3) as i32 - start as i32),
                None => return f64::NAN,
            }
            idx = end;
        }
    }
    let offset_ms = match b.get(idx) {
        None => 0.0,
        Some(b'Z') if idx == b.len() - 1 => 0.0,
        Some(sign @ (b'+' | b'-')) => {
            let (oh, om) = match (num_at(idx + 1, 2), b.get(idx + 3), num_at(idx + 4, 2)) {
                (Some(h), Some(b':'), Some(m)) => (h, m),
                (Some(h), None, _) if idx + 3 == b.len() => (h, 0),
                _ => return f64::NAN,
            };
            let total = (oh * 60 + om) as f64 * 60_000.0;
            if *sign == b'-' { total } else { -total }
        }
        _ => return f64::NAN,
    };
    (days_from_civil(y, mo, d) * 86_400_000 + hh * 3_600_000 + mm * 60_000 + ss * 1_000) as f64
        + frac
        + offset_ms
}

fn parse_iso_ms(value: &Value) -> Option<f64> {
    if js_falsy(value) {
        return None;
    }
    let s = match value {
        Value::String(s) => s.clone(),
        _ => return None,
    };
    let ms = iso_date_parse(&s);
    if ms.is_finite() { Some(ms) } else { None }
}

fn get_thread_last_activity_ms(thread: &Value) -> Option<f64> {
    let mut last: Option<f64> = None;
    for key in ["latestUserMessageAt", "updatedAt", "createdAt"] {
        let Some(ms) = parse_iso_ms(member(thread, key)) else {
            continue;
        };
        if last.is_none_or(|l| ms > l) {
            last = Some(ms);
        }
    }
    last
}

fn is_thread_archived(thread: &Value) -> bool {
    // `"archivedAt" in thread && (thread.archivedAt ?? null) !== null`
    match thread {
        Value::Object(m) => {
            !is_undefined(thread)
                && m.contains_key("archivedAt")
                && m.get("archivedAt").is_some_and(|v| !is_nullish(v))
        }
        _ => false,
    }
}

fn is_thread_deleted(thread: &Value) -> bool {
    // `"deletedAt" in thread && thread.deletedAt !== null` — an explicit
    // `undefined` still counts as deleted (`undefined !== null` in JS).
    match thread {
        Value::Object(m) => {
            !is_undefined(thread)
                && m.contains_key("deletedAt")
                && m.get("deletedAt").is_some_and(|v| !v.is_null())
        }
        _ => false,
    }
}

fn is_thread_busy(thread: &Value) -> bool {
    let session = member(thread, "session");
    if matches!(member(session, "status"), Value::String(s) if s == "starting" || s == "running") {
        return true;
    }
    let active_turn = member(session, "activeTurnId");
    if !active_turn.is_null() && !is_undefined(active_turn) {
        return true;
    }
    if matches!(member(member(thread, "latestTurn"), "state"), Value::String(s) if s == "running") {
        return true;
    }
    if matches!(member(thread, "hasPendingApprovals"), Value::Bool(true))
        || matches!(member(thread, "hasPendingUserInput"), Value::Bool(true))
    {
        return true;
    }
    false
}

fn is_thread_eligible(thread: &Value, cutoff_ms: f64, protected_ids: &[Value]) -> bool {
    let id = member(thread, "id");
    if set_contains(protected_ids, id) {
        return false;
    }
    if matches!(member(thread, "isPinned"), Value::Bool(true)) {
        return false;
    }
    // `(thread.snoozedUntil ?? null) !== null || thread.usageLimitRecovery != null`
    if !is_nullish(member(thread, "snoozedUntil"))
        || !is_nullish(member(thread, "usageLimitRecovery"))
    {
        return false;
    }
    if is_thread_busy(thread) {
        return false;
    }
    match get_thread_last_activity_ms(thread) {
        Some(ms) => ms <= cutoff_ms,
        None => false,
    }
}

/// Port of `getRetentionArchiveRootIds` (`apps/server/src/threadRetention.ts:176`).
pub fn get_retention_archive_root_ids(
    read_model: serde_json::Value,
    now_ms: Option<serde_json::Value>,
    protected_thread_ids: Option<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    if read_model.is_null() {
        return Err("Cannot read properties of null (reading 'threads')".to_string());
    }
    if is_undefined(&read_model) {
        return Err("Cannot read properties of undefined (reading 'threads')".to_string());
    }
    let threads = member(&read_model, "threads");
    let Some(thread_items) = iter_items(threads) else {
        return Err("readModel.threads is not iterable".to_string());
    };

    // `nowMs = Date.now()` default applies for absent or explicit `undefined`.
    let now = match &now_ms {
        Some(v) if !is_undefined(v) => js_to_number(v),
        _ => std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0),
    };
    let cutoff_ms = now - THREAD_RETENTION_UNUSED_MS;

    // `protectedThreadIds = new Set()` default; wire `{$set:...}` decodes to a set.
    let protected: Vec<Value> = match &protected_thread_ids {
        Some(Value::Object(m)) => match m.get("$set") {
            Some(Value::Array(items)) => items.clone(),
            _ => Vec::new(),
        },
        Some(Value::Array(items)) => items.clone(),
        _ => Vec::new(),
    };

    // Insertion-ordered Map<ThreadId, RetentionThread> of non-deleted, non-archived.
    let mut active_threads: Vec<(Value, Value)> = Vec::new();
    for thread in &thread_items {
        if is_thread_deleted(thread) || is_thread_archived(thread) {
            continue;
        }
        let id = member(thread, "id").clone();
        if let Some((_, slot)) = active_threads
            .iter_mut()
            .find(|(k, _)| js_same_value(k, &id))
        {
            *slot = thread.clone();
        } else {
            active_threads.push((id, thread.clone()));
        }
    }

    let mut eligible: Vec<Value> = Vec::new();
    let mut children_by_parent: Vec<(Value, Vec<Value>)> = Vec::new();
    for (_, thread) in &active_threads {
        let id = member(thread, "id").clone();
        if is_thread_eligible(thread, cutoff_ms, &protected) && !set_contains(&eligible, &id) {
            eligible.push(id.clone());
        }
        let parent = member(thread, "parentThreadId");
        let parent_is_active =
            !parent.is_null() && active_threads.iter().any(|(k, _)| js_same_value(k, parent));
        if parent.is_null() || !parent_is_active {
            continue;
        }
        if let Some((_, kids)) = children_by_parent
            .iter_mut()
            .find(|(k, _)| js_same_value(k, parent))
        {
            kids.push(id.clone());
        } else {
            children_by_parent.push((parent.clone(), vec![id.clone()]));
        }
    }

    fn can_archive(
        id: &Value,
        eligible: &[Value],
        children_by_parent: &[(Value, Vec<Value>)],
        visiting: &mut Vec<Value>,
        memo: &mut Vec<(Value, bool)>,
    ) -> bool {
        if let Some((_, cached)) = memo.iter().find(|(k, _)| js_same_value(k, id)) {
            return *cached;
        }
        if !set_contains(eligible, id) || set_contains(visiting, id) {
            return false;
        }
        visiting.push(id.clone());
        let kids: Vec<Value> = children_by_parent
            .iter()
            .find(|(k, _)| js_same_value(k, id))
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let can = kids
            .iter()
            .all(|kid| can_archive(kid, eligible, children_by_parent, visiting, memo));
        visiting.retain(|v| !js_same_value(v, id));
        memo.push((id.clone(), can));
        can
    }

    let mut visiting: Vec<Value> = Vec::new();
    let mut memo: Vec<(Value, bool)> = Vec::new();
    let mut out: Vec<Value> = Vec::new();
    for (id, thread) in &active_threads {
        let parent = member(thread, "parentThreadId");
        let is_root =
            parent.is_null() || !active_threads.iter().any(|(k, _)| js_same_value(k, parent));
        if is_root && can_archive(id, &eligible, &children_by_parent, &mut visiting, &mut memo) {
            out.push(id.clone());
        }
    }
    Ok(out)
}
