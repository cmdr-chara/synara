//! The Rust port of `apps/web/src/session-logic.ts`.
//!
//! Activity ordering comes from `apps/web/src/workLog.ts` (`orderedActivities` and
//! `compareActivitiesByOrder`) and background-task folding from
//! `packages/shared/src/backgroundTasks.ts`; both are inlined here so the crate is
//! self-contained. `localeCompare` uses the ranked order recorded in `corpus.rs`.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use serde_json::{Map, Value};

// ---------- JS value helpers ----------

fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `v.key` member access: throws on null/undefined receivers, returns
/// `undefined` for a missing property on anything else.
fn member<'a>(v: &'a Value, key: &str, expr: &str) -> Result<Option<&'a Value>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!("undefined is not an object (evaluating '{expr}')"));
    }
    Ok(match v {
        Value::Object(o) => o.get(key),
        _ => None,
    })
}

/// `v.key` when the receiver is known non-nullish.
fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) => o.get(key),
        _ => None,
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    if crate::js::is_undefined(a) || crate::js::is_undefined(b) {
        return crate::js::is_undefined(a) && crate::js::is_undefined(b);
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().unwrap_or(f64::NAN) == y.as_f64().unwrap_or(f64::NAN)
        }
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if is_nullish(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if o.contains_key("$undefined") {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// `String.prototype.localeCompare`, approximated by the ranked order of the
/// recorded corpus; unranked strings fall back to codepoint order.
fn locale_compare(a: &str, b: &str) -> i64 {
    match (crate::corpus::rank(a), crate::corpus::rank(b)) {
        (Some(x), Some(y)) => x - y,
        _ => a.cmp(b) as i64,
    }
}

/// `Math.max` with JS NaN semantics (Rust `max` drops NaN).
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64()
}

// ---------- `Date.parse` (JSC lenient parser, corpus coverage) ----------

/// The parse results Bun's `Date.parse` returns for the recorded corpus:
/// a string whose trailing run is `-<digits>` or bare `<digits>` parses as a
/// year (two digits map 0..=49 → 2000+n, 50..=99 → 1900+n, longer literals stay).
fn date_parse(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return f64::NAN;
    }
    // ISO 8601 forms: YYYY-MM-DD, optionally T HH:mm[:ss[.fff]][Z|±HH:mm].
    if let Some(ms) = parse_iso(t) {
        return ms;
    }
    // JSC's fallback: trailing digits, optionally after a '-', name a year.
    let digits_end = t.trim_end_matches(|c: char| c.is_whitespace());
    let run_start = digits_end
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    let digits = &digits_end[run_start..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return f64::NAN;
    }
    // The digit run must either follow '-' or be the whole string.
    let before = &digits_end[..run_start];
    if !before.is_empty() && !before.ends_with('-') {
        return f64::NAN;
    }
    let n: i64 = match digits.parse() {
        Ok(n) => n,
        Err(_) => return f64::NAN,
    };
    let year = if digits.len() <= 2 {
        if n <= 49 { 2000 + n } else { 1900 + n }
    } else {
        n
    };
    epoch_ms(year, 1, 1, 0, 0, 0, 0)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    // Howard Hinnant's days_from_civil, 1970-01-01 = 0.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn epoch_ms(y: i64, m: i64, d: i64, h: i64, min: i64, s: i64, ms: i64) -> f64 {
    (days_from_civil(y, m, d) * 86_400_000 + h * 3_600_000 + min * 60_000 + s * 1_000 + ms) as f64
}

fn parse_iso(t: &str) -> Option<f64> {
    // Full ISO only; anything else falls through to JSC's lenient rules.
    let b = t.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (t.get(0..4)?, t.get(5..7)?, t.get(8..10)?);
    let (Ok(y), Ok(m), Ok(d)) = (y.parse::<i64>(), m.parse::<i64>(), d.parse::<i64>()) else {
        return None;
    };
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    if b.len() == 10 {
        // Date-only is UTC per spec.
        return Some(epoch_ms(y, m, d, 0, 0, 0, 0));
    }
    if b[10] != b'T' && b[10] != b' ' {
        return None;
    }
    let rest = t.get(11..)?;
    let (time_part, tz) = match rest.find(['Z', '+', '-']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let segs: Vec<&str> = time_part.split(':').collect();
    if segs.len() < 2 || segs.len() > 3 {
        return None;
    }
    let (Ok(h), Ok(min)) = (segs[0].parse::<i64>(), segs[1].parse::<i64>()) else {
        return None;
    };
    let (s, ms) = match segs.get(2) {
        None => (0, 0),
        Some(x) => {
            let (ss, f) = x.split_once('.').unwrap_or((*x, "0"));
            let mut f3 = f.chars().take(3).collect::<String>();
            while f3.len() < 3 {
                f3.push('0');
            }
            (ss.parse::<i64>().ok()?, f3.parse::<i64>().ok()?)
        }
    };
    let mut offset_ms: i64 = 0;
    if tz == "Z" {
        offset_ms = 0;
    } else if !tz.is_empty() {
        let sign: i64 = if tz.starts_with('-') { -1 } else { 1 };
        let body = &tz[1..];
        let (hh, mm) = body.split_once(':').unwrap_or((body, "0"));
        offset_ms = sign * (hh.parse::<i64>().ok()? * 3_600_000 + mm.parse::<i64>().ok()? * 60_000);
    }
    Some(epoch_ms(y, m, d, h, min, s, ms) - offset_ms as f64)
}

// ---------- orderedActivities (workLog.ts) ----------

fn lifecycle_rank(kind: &Value) -> f64 {
    let Some(k) = kind.as_str() else {
        return 1.0;
    };
    if k.ends_with(".started") || k == "tool.started" {
        0.0
    } else if k.ends_with(".progress") || k.ends_with(".updated") {
        1.0
    } else if k.ends_with(".completed") || k.ends_with(".resolved") {
        2.0
    } else {
        1.0
    }
}

fn compaction_rank(summary: Option<&Value>) -> f64 {
    match summary {
        Some(Value::String(s))
            if s == "Compacting context" || s == "Compacting conversation..." =>
        {
            0.0
        }
        _ => 1.0,
    }
}

/// `left.<field>.localeCompare(right.<field>)` — errors on nullish/non-string
/// left values exactly as JSC reports them.
fn locale_compare_field(
    left: &Value,
    right: &Value,
    field: &str,
    lname: &str,
    rname: &str,
) -> Result<i64, String> {
    let member_expr = format!("{lname}.{field}");
    let lv = member(left, field, &member_expr)?;
    let lv = lv.unwrap_or(&Value::Null);
    let is_missing = lv.is_null() || crate::js::is_undefined(lv);
    let call_expr = format!("{lname}.{field}.localeCompare({rname}.{field})");
    if is_missing && get(left, field).is_none() || crate::js::is_undefined(lv) {
        return Err(format!(
            "undefined is not an object (evaluating '{member_expr}.localeCompare')"
        ));
    }
    if lv.is_null() {
        return Err(format!(
            "null is not an object (evaluating '{member_expr}.localeCompare')"
        ));
    }
    let Some(ls) = lv.as_str() else {
        return Err(format!(
            "{member_expr}.localeCompare is not a function. (In '{call_expr}', \
             '{member_expr}.localeCompare' is undefined)"
        ));
    };
    let rv = get(right, field);
    let rs = rv
        .map(js_to_string)
        .unwrap_or_else(|| "undefined".to_string());
    Ok(locale_compare(ls, &rs))
}

fn compare_activities(a: &Value, b: &Value) -> Result<f64, String> {
    // `.sequence` on primitives yields undefined; on nullish elements it throws.
    let ls = member(a, "sequence", "left.sequence")?;
    let rs = member(b, "sequence", "right.sequence")?;
    let l_def = ls.is_some_and(|v| !crate::js::is_undefined(v));
    let r_def = rs.is_some_and(|v| !crate::js::is_undefined(v));
    if l_def && r_def {
        let (x, y) = (ls.unwrap(), rs.unwrap());
        if !strict_eq(x, y) {
            return Ok(num(x).unwrap_or(f64::NAN) - num(y).unwrap_or(f64::NAN));
        }
    } else if l_def {
        return Ok(1.0);
    } else if r_def {
        return Ok(-1.0);
    }
    let created = locale_compare_field(a, b, "createdAt", "left", "right")?;
    if created != 0 {
        return Ok(created as f64);
    }
    let rank = lifecycle_rank(get(a, "kind").unwrap_or(&Value::Null))
        - lifecycle_rank(get(b, "kind").unwrap_or(&Value::Null));
    if rank != 0.0 {
        return Ok(rank);
    }
    if get(a, "kind").is_some_and(|k| k.as_str() == Some("context-compaction"))
        && get(b, "kind").is_some_and(|k| k.as_str() == Some("context-compaction"))
    {
        let cr = compaction_rank(get(a, "summary")) - compaction_rank(get(b, "summary"));
        if cr != 0.0 {
            return Ok(cr);
        }
    }
    Ok(locale_compare_field(a, b, "id", "left", "right")? as f64)
}

/// `orderedActivities` — including the WeakMap cache semantics: `get` never
/// throws, `set` throws for primitive keys after a stable (or sorted) pass.
fn ordered_activities(activities: &Value) -> Result<Value, String> {
    // orderedActivitiesCache.get(activities) -> undefined; then length loop.
    match activities {
        Value::Null => {
            return Err("null is not an object (evaluating 'activities.length')".to_string());
        }
        v if crate::js::is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'activities.length')".to_string());
        }
        _ => {}
    }
    let ordered = match activities {
        Value::Array(items) => {
            let mut out = items.clone();
            let stable = {
                let mut ok = true;
                for i in 1..out.len() {
                    if compare_activities(&out[i - 1], &out[i])? > 0.0 {
                        ok = false;
                        break;
                    }
                }
                ok
            };
            if !stable {
                let mut first_err: Option<String> = None;
                crate::js::sort(&mut out, |a, b| {
                    if first_err.is_some() {
                        return 0.0;
                    }
                    match compare_activities(a, b) {
                        Ok(n) => n,
                        Err(e) => {
                            first_err = Some(e);
                            0.0
                        }
                    }
                });
                if let Some(e) = first_err {
                    return Err(e);
                }
            }
            Value::Array(out)
        }
        Value::String(s) => {
            // String indexing yields char strings for compare.
            let chars: Vec<Value> = s
                .encode_utf16()
                .map(|u| crate::js::utf16_string(&[u]))
                .collect();
            for i in 1..chars.len() {
                compare_activities(&chars[i - 1], &chars[i])?;
            }
            return Err("WeakMap keys must be objects or non-registered symbols".to_string());
        }
        _ => activities.clone(),
    };
    if matches!(ordered, Value::Array(_) | Value::Object(_)) {
        // WeakMap.set(object) succeeds; callers handle the result.
        return Ok(ordered);
    }
    // Numbers and booleans reach orderedActivitiesCache.set and throw.
    Err("WeakMap keys must be objects or non-registered symbols".to_string())
}

/// Iterate a value as `for..of` does: arrays (and strings, though unreachable
/// here) iterate; anything else reports `X is not iterable`.
fn iterable(v: &Value) -> Result<&[Value], String> {
    match v {
        Value::Array(items) => Ok(items),
        other => Err(format!("{} is not iterable", js_to_string(other))),
    }
}

/// `[...x]`: arrays copy, strings spread UTF-16 chars, nullish/other values throw.
fn spread(v: &Value) -> Result<Vec<Value>, String> {
    if v.is_null() || crate::js::is_undefined(v) {
        return Err("Spread syntax requires ...iterable not be null or undefined".to_string());
    }
    match v {
        Value::Array(items) => Ok(items.clone()),
        Value::String(s) => Ok(s
            .encode_utf16()
            .map(|u| crate::js::utf16_string(&[u]))
            .collect()),
        _ => {
            Err("Spread syntax requires ...iterable[Symbol.iterator] to be a function".to_string())
        }
    }
}

// ---------- backgroundTasks.ts ----------

const BACKGROUND_TASKS_CHANGED: &str = "background_tasks_changed";

fn as_record(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(o) => Some(o),
        _ => None,
    }
}

fn fold_outstanding_background_task_ids(ordered: &[Value]) -> Vec<String> {
    let mut outstanding: Vec<String> = Vec::new();
    for activity in ordered {
        let payload = get(activity, "payload").and_then(as_record);
        if get(activity, "kind").and_then(Value::as_str) == Some("runtime.warning") {
            if payload
                .and_then(|p| p.get("nativeEventType"))
                .and_then(Value::as_str)
                != Some(BACKGROUND_TASKS_CHANGED)
            {
                continue;
            }
            let tasks = payload
                .and_then(|p| p.get("data"))
                .and_then(as_record)
                .and_then(|d| d.get("tasks"))
                .and_then(Value::as_array);
            let Some(tasks) = tasks else { continue };
            for task in tasks {
                if let Some(id) = as_record(task)
                    .and_then(|t| t.get("task_id"))
                    .and_then(Value::as_str)
                {
                    let id = id.to_string();
                    if !outstanding.contains(&id) {
                        outstanding.push(id);
                    }
                }
            }
            continue;
        }
        let task_id = match payload
            .and_then(|p| p.get("taskId"))
            .and_then(Value::as_str)
        {
            Some(s) => s.to_string(),
            None => continue,
        };
        match get(activity, "kind").and_then(Value::as_str) {
            Some("task.completed") => {
                outstanding.retain(|x| x != &task_id);
            }
            Some("task.updated") => {
                let status = payload
                    .and_then(|p| p.get("status"))
                    .and_then(Value::as_str);
                let bg = payload.and_then(|p| p.get("isBackgrounded"));
                if matches!(
                    status,
                    Some("completed") | Some("failed") | Some("killed") | Some("paused")
                ) || bg == Some(&Value::Bool(false))
                {
                    outstanding.retain(|x| x != &task_id);
                } else if bg == Some(&Value::Bool(true)) && !outstanding.contains(&task_id) {
                    outstanding.push(task_id);
                }
            }
            _ => {}
        }
    }
    outstanding
}

fn can_session_finish_background_tasks(status: Option<&Value>) -> bool {
    match status {
        Some(v) if !v.is_null() && !crate::js::is_undefined(v) => {
            v.as_str() != Some("stopped") && v.as_str() != Some("error")
        }
        _ => false,
    }
}

// ---------- foldActiveTasks (session-logic.ts) ----------

#[derive(Clone, Default)]
struct FoldedTask {
    task_type: Option<String>,
    is_backgrounded: bool,
    tool_use_id: Option<String>,
    subagent_type: Option<String>,
    workflow_task_id: Option<String>,
    turn_id: Value,
}

struct TaskSet {
    order: Vec<String>,
    tasks: Vec<(String, FoldedTask)>,
}

impl TaskSet {
    fn new() -> Self {
        Self {
            order: Vec::new(),
            tasks: Vec::new(),
        }
    }
    fn get(&self, id: &str) -> Option<&FoldedTask> {
        self.tasks.iter().find(|(k, _)| k == id).map(|(_, t)| t)
    }
    fn set(&mut self, id: &str, task: FoldedTask) {
        if let Some(slot) = self.tasks.iter_mut().find(|(k, _)| k == id) {
            slot.1 = task;
        } else {
            self.order.push(id.to_string());
            self.tasks.push((id.to_string(), task));
        }
    }
    fn delete(&mut self, id: &str) {
        self.order.retain(|k| k != id);
        self.tasks.retain(|(k, _)| k != id);
    }
    fn entries(&self) -> impl Iterator<Item = (&String, &FoldedTask)> {
        self.tasks.iter().map(|(k, t)| (k, t))
    }
}

fn payload_string(payload: Option<&Map<String, Value>>, key: &str) -> Option<String> {
    payload
        .and_then(|p| p.get(key))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `foldActiveTasks` — insertion-ordered like a JS `Map`.
fn fold_active_tasks(
    activities: &Value,
    latest_turn_id: Option<&Value>,
) -> Result<TaskSet, String> {
    let ordered = ordered_activities(activities)?;
    let ordered = match &ordered {
        Value::Array(items) => items.clone(),
        other => return Err(format!("{} is not iterable", js_to_string(other))),
    };
    let mut active = TaskSet::new();
    for activity in &ordered {
        let kind = get(activity, "kind").and_then(Value::as_str);
        let act_turn = member(activity, "turnId", "activity.turnId")?;
        if let (Some(tid), Some(at)) = (latest_turn_id, act_turn)
            && truthy(tid)
            && truthy(at)
            && !strict_eq(at, tid)
            && kind != Some("task.completed")
            && kind != Some("task.updated")
        {
            continue;
        }
        if !matches!(
            kind,
            Some("task.started")
                | Some("task.progress")
                | Some("task.updated")
                | Some("task.completed")
        ) {
            continue;
        }
        let payload = get(activity, "payload").and_then(as_record);
        let task_id = match payload
            .and_then(|p| p.get("taskId"))
            .and_then(Value::as_str)
        {
            Some(s) => s.to_string(),
            None => continue,
        };
        if kind == Some("task.completed") {
            active.delete(&task_id);
            continue;
        }
        if kind == Some("task.updated") {
            let status = payload
                .and_then(|p| p.get("status"))
                .and_then(Value::as_str);
            if matches!(
                status,
                Some("completed") | Some("failed") | Some("killed") | Some("paused")
            ) {
                active.delete(&task_id);
                continue;
            }
            let previous = active.get(&task_id).cloned();
            let is_backgrounded = match payload.and_then(|p| p.get("isBackgrounded")) {
                Some(Value::Bool(b)) => *b,
                _ => previous.as_ref().is_some_and(|p| p.is_backgrounded),
            };
            let in_turn = latest_turn_id.is_none_or(|t| !truthy(t))
                || act_turn.is_none_or(|v| !truthy(v))
                || act_turn.is_some_and(|v| strict_eq(v, latest_turn_id.unwrap()));
            if previous.is_some() || (is_backgrounded && in_turn) {
                let prev = previous.clone().unwrap_or_default();
                active.set(
                    &task_id,
                    FoldedTask {
                        is_backgrounded,
                        tool_use_id: prev
                            .tool_use_id
                            .or_else(|| payload_string(payload, "toolUseId")),
                        workflow_task_id: prev
                            .workflow_task_id
                            .or_else(|| payload_string(payload, "workflowTaskId")),
                        turn_id: match prev.turn_id {
                            ref t if !is_nullish(t) => t.clone(),
                            _ => act_turn.cloned().unwrap_or(Value::Null),
                        },
                        ..prev
                    },
                );
            }
            continue;
        }
        // task.started / task.progress
        let previous = active.get(&task_id).cloned().unwrap_or_default();
        active.set(
            &task_id,
            FoldedTask {
                task_type: payload_string(payload, "taskType").or(previous.task_type),
                is_backgrounded: previous.is_backgrounded,
                tool_use_id: previous
                    .tool_use_id
                    .or_else(|| payload_string(payload, "toolUseId")),
                subagent_type: previous
                    .subagent_type
                    .or_else(|| payload_string(payload, "subagentType")),
                workflow_task_id: previous
                    .workflow_task_id
                    .or_else(|| payload_string(payload, "workflowTaskId")),
                turn_id: match previous.turn_id {
                    ref t if !is_nullish(t) => t.clone(),
                    _ => act_turn.cloned().unwrap_or(Value::Null),
                },
            },
        );
    }
    Ok(active)
}

// ---------- turn / session predicates ----------

fn latest_started(latest_turn: Option<&Value>) -> bool {
    latest_turn
        .and_then(|t| get(t, "startedAt"))
        .is_some_and(truthy)
}

fn is_latest_turn_settled_inner(latest_turn: Option<&Value>, session: Option<&Value>) -> bool {
    if !latest_started(latest_turn) {
        return false;
    }
    if !latest_turn
        .and_then(|t| get(t, "completedAt"))
        .is_some_and(truthy)
    {
        return false;
    }
    if get(latest_turn.unwrap(), "state").and_then(Value::as_str) == Some("interrupted")
        || get(latest_turn.unwrap(), "state").and_then(Value::as_str) == Some("error")
    {
        return true;
    }
    let Some(session) = session else {
        return true;
    };
    get(session, "orchestrationStatus").and_then(Value::as_str) != Some("running")
}

fn running_active_turn(session: Option<&Value>) -> Value {
    match session {
        Some(s) if get(s, "orchestrationStatus").and_then(Value::as_str) == Some("running") => {
            get(s, "activeTurnId").cloned().unwrap_or(Value::Null)
        }
        _ => Value::Null,
    }
}

// ---------- exported ports ----------

/// Port of `formatClockDuration` (`apps/web/src/session-logic.ts:105`).
pub fn format_clock_duration(duration_ms: f64) -> Result<String, String> {
    let secs = js_max(0.0, (duration_ms / 1_000.0).floor());
    if secs < 60.0 {
        return Ok(format!("{}s", crate::js::number_to_string(secs)));
    }
    let days = (secs / 86_400.0).floor();
    let hours = ((secs % 86_400.0) / 3_600.0).floor();
    if days > 0.0 {
        return Ok(if hours > 0.0 {
            format!(
                "{}d {}h",
                crate::js::number_to_string(days),
                crate::js::number_to_string(hours)
            )
        } else {
            format!("{}d", crate::js::number_to_string(days))
        });
    }
    let minutes = ((secs % 3_600.0) / 60.0).floor();
    let seconds = secs % 60.0;
    if hours > 0.0 {
        return Ok(if minutes > 0.0 {
            format!(
                "{}h {}m",
                crate::js::number_to_string(hours),
                crate::js::number_to_string(minutes)
            )
        } else {
            format!("{}h", crate::js::number_to_string(hours))
        });
    }
    Ok(if seconds > 0.0 {
        format!(
            "{}m {}s",
            crate::js::number_to_string(minutes),
            crate::js::number_to_string(seconds)
        )
    } else {
        format!("{}m", crate::js::number_to_string(minutes))
    })
}

/// `formatDuration` (internal): settled-time formatting.
fn format_duration(duration_ms: f64) -> String {
    if !duration_ms.is_finite() || duration_ms < 0.0 {
        return "0ms".to_string();
    }
    if duration_ms < 1_000.0 {
        return format!("{}ms", js_max(1.0, duration_ms.round()));
    }
    if duration_ms < 9_950.0 {
        return format!("{:.1}s", duration_ms / 1_000.0);
    }
    if duration_ms < 59_500.0 {
        return format!("{}s", (duration_ms / 1_000.0).round());
    }
    format_clock_duration((duration_ms / 1_000.0).round() * 1_000.0)
        .unwrap_or_else(|_| "0ms".to_string())
}

/// Port of `formatClockElapsed` (`apps/web/src/session-logic.ts:119`).
pub fn format_clock_elapsed(
    start_iso: String,
    end_iso: Option<String>,
) -> Result<Option<String>, String> {
    let Some(end_iso) = end_iso else {
        return Ok(None);
    };
    let (started, ended) = (date_parse(&start_iso), date_parse(&end_iso));
    if started.is_nan() || ended.is_nan() || ended < started {
        return Ok(None);
    }
    Ok(Some(format_clock_duration(ended - started)?))
}

/// Port of `formatElapsed` (`apps/web/src/session-logic.ts:129`).
pub fn format_elapsed(
    start_iso: String,
    end_iso: Option<String>,
) -> Result<Option<String>, String> {
    let Some(end_iso) = end_iso else {
        return Ok(None);
    };
    let (started, ended) = (date_parse(&start_iso), date_parse(&end_iso));
    if started.is_nan() || ended.is_nan() || ended < started {
        return Ok(None);
    }
    Ok(Some(format_duration(ended - started)))
}

/// Port of `isSessionOrLatestTurnRunning` (`apps/web/src/session-logic.ts:153`).
pub fn is_session_or_latest_turn_running(
    orchestration_status: Option<serde_json::Value>,
    latest_turn_state: Option<serde_json::Value>,
) -> Result<bool, String> {
    Ok(
        orchestration_status.as_ref().and_then(Value::as_str) == Some("running")
            || latest_turn_state.as_ref().and_then(Value::as_str) == Some("running"),
    )
}

/// Port of `isSessionOrLatestTurnStartingOrRunning` (`apps/web/src/session-logic.ts:161`).
pub fn is_session_or_latest_turn_starting_or_running(
    orchestration_status: Option<serde_json::Value>,
    latest_turn_state: Option<serde_json::Value>,
) -> Result<bool, String> {
    Ok(
        orchestration_status.as_ref().and_then(Value::as_str) == Some("starting")
            || is_session_or_latest_turn_running(orchestration_status, latest_turn_state)?,
    )
}

/// Port of `isSessionOpenOrLatestTurnRunning` (`apps/web/src/session-logic.ts:175`).
pub fn is_session_open_or_latest_turn_running(
    orchestration_status: Option<serde_json::Value>,
    latest_turn_state: Option<serde_json::Value>,
) -> Result<bool, String> {
    let open = match &orchestration_status {
        Some(v) if !v.is_null() && !crate::js::is_undefined(v) => {
            v.as_str() != Some("idle") && v.as_str() != Some("stopped")
        }
        _ => false,
    };
    Ok(open || latest_turn_state.as_ref().and_then(Value::as_str) == Some("running"))
}

/// Port of `runningSessionActiveTurnId` (`apps/web/src/session-logic.ts:188`).
pub fn running_session_active_turn_id(
    session: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    Ok(match running_active_turn(session.as_ref()) {
        Value::Null => None,
        v => Some(v),
    })
}

/// Port of `isLatestTurnSettled` (`apps/web/src/session-logic.ts:194`).
pub fn is_latest_turn_settled(
    latest_turn: Option<serde_json::Value>,
    session: Option<serde_json::Value>,
) -> Result<bool, String> {
    Ok(is_latest_turn_settled_inner(
        latest_turn.as_ref(),
        session.as_ref(),
    ))
}

/// Port of `hasLiveLatestTurn` (`apps/web/src/session-logic.ts:208`).
pub fn has_live_latest_turn(
    latest_turn: Option<serde_json::Value>,
    session: Option<serde_json::Value>,
) -> Result<bool, String> {
    if !latest_started(latest_turn.as_ref()) {
        return Ok(false);
    }
    Ok(!is_latest_turn_settled_inner(
        latest_turn.as_ref(),
        session.as_ref(),
    ))
}

/// Port of `canSessionAnswerPendingRequests` (`apps/web/src/session-logic.ts:226`).
pub fn can_session_answer_pending_requests(
    session: Option<serde_json::Value>,
) -> Result<bool, String> {
    let Some(session) = session.as_ref().filter(|s| truthy(s)) else {
        return Ok(true);
    };
    Ok(
        get(session, "status").and_then(Value::as_str) != Some("closed")
            && get(session, "status").and_then(Value::as_str) != Some("error"),
    )
}

/// Port of `isSessionRuntimeRetired` (`apps/web/src/session-logic.ts:240`).
pub fn is_session_runtime_retired(status: Option<serde_json::Value>) -> Result<bool, String> {
    Ok(status.as_ref().and_then(Value::as_str) == Some("stopped"))
}

/// Port of `isSessionAliveForSubagents` (`apps/web/src/session-logic.ts:247`).
pub fn is_session_alive_for_subagents(status: Option<serde_json::Value>) -> Result<bool, String> {
    Ok(match status.as_ref() {
        Some(v) if !v.is_null() && !crate::js::is_undefined(v) => {
            !is_session_runtime_retired(Some(v.clone()))?
        }
        _ => false,
    })
}

/// Port of `isSessionRunningTurn` (`apps/web/src/session-logic.ts:272`).
pub fn is_session_running_turn(session: Option<serde_json::Value>) -> Result<bool, String> {
    let Some(session) = session.as_ref() else {
        return Ok(false);
    };
    Ok(
        get(session, "status").and_then(Value::as_str) == Some("running")
            && get(session, "activeTurnId").is_some_and(|v| !is_nullish(v)),
    )
}

/// Port of `deriveActiveWorkStartedAt` (`apps/web/src/session-logic.ts:278`).
/// Returns the resolved start marker: `undefined` serializes as `{"$undefined":true}`.
pub fn derive_active_work_started_at(
    latest_turn: Option<serde_json::Value>,
    session: Option<serde_json::Value>,
    send_started_at: Option<String>,
) -> Result<serde_json::Value, String> {
    let send = || {
        send_started_at
            .map(Value::String)
            .unwrap_or_else(crate::js::undefined)
    };
    let running_id = running_active_turn(session.as_ref());
    let lt = latest_turn.as_ref();
    if !running_id.is_null() {
        let lt_turn = lt.and_then(|t| get(t, "turnId"));
        if lt_turn.is_some_and(|t| strict_eq(t, &running_id)) {
            let started = lt.and_then(|t| get(t, "startedAt"));
            return Ok(match started {
                Some(s) if !is_nullish(s) => s.clone(),
                _ => send(),
            });
        }
        return Ok(send());
    }
    if !is_latest_turn_settled_inner(lt, session.as_ref()) {
        let started = lt.and_then(|t| get(t, "startedAt"));
        return Ok(match started {
            Some(s) if !is_nullish(s) => s.clone(),
            _ => send(),
        });
    }
    Ok(send())
}

// ---------- task-list / background-task derivations ----------

fn parse_task_list_tasks(payload: &Value) -> Option<Vec<Value>> {
    let record = as_record(payload)?;
    let raw = record.get("tasks")?.as_array()?;
    let mut tasks = Vec::new();
    for entry in raw {
        let e = as_record(entry)?;
        let task = e.get("task")?.as_str()?;
        let status = match e.get("status").and_then(Value::as_str) {
            Some("completed") => "completed",
            Some("inProgress") => "inProgress",
            _ => "pending",
        };
        tasks.push(Value::Object(Map::from_iter([
            ("task".to_string(), Value::String(task.to_string())),
            ("status".to_string(), Value::String(status.to_string())),
        ])));
    }
    if !raw.is_empty() && tasks.is_empty() {
        return None;
    }
    Some(tasks)
}

fn to_active_task_list_state(activity: &Value) -> Option<Value> {
    let payload = get(activity, "payload")
        .filter(|p| truthy(p))
        .and_then(as_record);
    let tasks = payload.and_then(|p| parse_task_list_tasks(&Value::Object(p.clone())))?;
    let mut out = Map::new();
    out.insert(
        "createdAt".to_string(),
        get(activity, "createdAt")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
    );
    out.insert(
        "turnId".to_string(),
        get(activity, "turnId")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
    );
    if let Some(p) = payload
        && p.contains_key("explanation")
    {
        out.insert(
            "explanation".to_string(),
            p.get("explanation").cloned().unwrap_or(Value::Null),
        );
    }
    out.insert("tasks".to_string(), Value::Array(tasks));
    Some(Value::Object(out))
}

/// Port of `deriveActiveTaskListState` (`apps/web/src/session-logic.ts:315`).
pub fn derive_active_task_list_state(
    activities: serde_json::Value,
    latest_turn_id: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let ordered = ordered_activities(&activities)?;
    let Value::Array(ordered) = ordered else {
        return Err(
            "orderedActivities(activities).filter is not a function. (In \
             'orderedActivities(activities).filter((activity) => activity.kind === \
             \"turn.tasks.updated\")', 'orderedActivities(activities).filter' is undefined)"
                .to_string(),
        );
    };
    let all: Vec<&Value> = ordered
        .iter()
        .filter(|a| get(a, "kind").and_then(Value::as_str) == Some("turn.tasks.updated"))
        .collect();
    let latest_turn_id = latest_turn_id.filter(truthy);
    if let Some(tid) = &latest_turn_id {
        let current = all
            .iter()
            .filter(|a| get(a, "turnId").is_some_and(|t| strict_eq(t, tid)))
            .filter_map(|a| to_active_task_list_state(a))
            .next_back();
        if let Some(state) = current {
            let empty = get(&state, "tasks")
                .and_then(Value::as_array)
                .is_some_and(|t| t.is_empty());
            return Ok(if empty { None } else { Some(state) });
        }
    }
    let latest_prior = all
        .iter()
        .filter_map(|a| to_active_task_list_state(a))
        .next_back();
    let Some(state) = latest_prior else {
        return Ok(None);
    };
    let tasks = get(&state, "tasks").and_then(Value::as_array);
    let Some(tasks) = tasks else { return Ok(None) };
    if tasks.is_empty() {
        return Ok(None);
    }
    let unfinished = tasks
        .iter()
        .any(|t| get(t, "status").and_then(Value::as_str) != Some("completed"));
    Ok(if unfinished { Some(state) } else { None })
}

/// Port of `deriveActiveBackgroundTasksState` (`apps/web/src/session-logic.ts:462`).
pub fn derive_active_background_tasks_state(
    activities: serde_json::Value,
    latest_turn_id: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let folded = fold_active_tasks(&activities, latest_turn_id.as_ref())?;
    let ids: Vec<String> = folded
        .entries()
        .filter(|(_, t)| t.task_type.as_deref() != Some("plan"))
        .map(|(id, _)| id.clone())
        .collect();
    if ids.is_empty() {
        return Ok(None);
    }
    Ok(Some(Value::Object(Map::from_iter([
        (
            "activeCount".to_string(),
            crate::js::number(ids.len() as f64),
        ),
        (
            "taskIds".to_string(),
            Value::Array(ids.iter().cloned().map(Value::String).collect()),
        ),
    ]))))
}

/// Port of `derivePendingBackgroundWork` (`apps/web/src/session-logic.ts:481`).
pub fn derive_pending_background_work(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let latest_turn = member(&input, "latestTurn", "input.latestTurn")?;
    let latest_turn_id = latest_turn
        .and_then(|t| get(t, "turnId"))
        .filter(|v| truthy(v));
    let Some(latest_turn_id) = latest_turn_id else {
        return Ok(None);
    };
    let session_status =
        member(&input, "session", "input.session")?.and_then(|s| get(s, "orchestrationStatus"));
    if session_status.is_none()
        || session_status.is_some_and(crate::js::is_undefined)
        || session_status.and_then(Value::as_str) == Some("stopped")
        || session_status.and_then(Value::as_str) == Some("error")
    {
        return Ok(None);
    }
    let activities = member(&input, "activities", "input.activities")?;
    let activities = activities.unwrap_or(&Value::Null);
    let folded = fold_active_tasks(activities, Some(latest_turn_id))?;
    let ids: Vec<String> = folded
        .entries()
        .filter(|(_, t)| t.is_backgrounded)
        .map(|(id, _)| id.clone())
        .collect();
    if ids.is_empty() {
        return Ok(None);
    }
    Ok(Some(Value::Object(Map::from_iter([
        ("count".to_string(), crate::js::number(ids.len() as f64)),
        (
            "taskIds".to_string(),
            Value::Array(ids.iter().cloned().map(Value::String).collect()),
        ),
    ]))))
}

/// Port of `deriveOutstandingBackgroundTaskIds` (`apps/web/src/session-logic.ts:511`).
pub fn derive_outstanding_background_task_ids(
    activities: serde_json::Value,
) -> Result<Vec<String>, String> {
    let ordered = ordered_activities(&activities)?;
    let items = iterable(&ordered)?;
    Ok(fold_outstanding_background_task_ids(items))
}

/// Port of `countOutstandingBackgroundWork` (`apps/web/src/session-logic.ts:519`).
pub fn count_outstanding_background_work(input: serde_json::Value) -> Result<f64, String> {
    let session = member(&input, "session", "input.session")?;
    let status = session.and_then(|s| get(s, "orchestrationStatus"));
    if !can_session_finish_background_tasks(status) {
        return Ok(0.0);
    }
    let activities = member(&input, "activities", "input.activities")?;
    let activities = activities.unwrap_or(&Value::Null);
    let ordered = ordered_activities(activities)?;
    let items = iterable(&ordered)?;
    Ok(fold_outstanding_background_task_ids(items).len() as f64)
}

fn is_agent_task(t: &FoldedTask) -> bool {
    if t.workflow_task_id.is_some() {
        return false;
    }
    if t.task_type.as_deref() == Some("subagent") {
        return true;
    }
    t.tool_use_id.is_some()
        && (t.task_type.as_deref() == Some("local_agent") || t.subagent_type.is_some())
}

/// Port of `deriveRunningSubagentTasks` (`apps/web/src/session-logic.ts:570`).
pub fn derive_running_subagent_tasks(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let no_tasks = || {
        Value::Object(Map::from_iter([
            ("running".to_string(), Value::Array(Vec::new())),
            (
                "trackedKeys".to_string(),
                Value::Object(Map::from_iter([(
                    "$set".to_string(),
                    Value::Array(Vec::new()),
                )])),
            ),
        ]))
    };
    let session_status = member(&input, "sessionStatus", "input.sessionStatus")?;
    if !can_session_finish_background_tasks(session_status) {
        return Ok(no_tasks());
    }
    let activities = member(&input, "activities", "input.activities")?;
    let activities = activities.cloned().unwrap_or_else(crate::js::undefined);
    let ordered = ordered_activities(&activities)?;
    let items = iterable(&ordered)?.to_vec();

    let mut tracked: Vec<String> = Vec::new();
    let mut key_by_task: Vec<(String, String)> = Vec::new();
    let mut ended: HashSet<String> = HashSet::new();
    let mut resumed: HashSet<String> = HashSet::new();
    for activity in &items {
        let payload = get(activity, "payload").and_then(as_record);
        let task_id = match payload
            .and_then(|p| p.get("taskId"))
            .and_then(Value::as_str)
        {
            Some(s) => s.to_string(),
            None => continue,
        };
        if get(activity, "kind").and_then(Value::as_str) == Some("task.started") {
            let task = FoldedTask {
                task_type: payload_string(payload, "taskType"),
                is_backgrounded: false,
                tool_use_id: payload_string(payload, "toolUseId"),
                subagent_type: payload_string(payload, "subagentType"),
                workflow_task_id: payload_string(payload, "workflowTaskId"),
                turn_id: Value::Null,
            };
            if !is_agent_task(&task) {
                continue;
            }
            let key = task.tool_use_id.clone().unwrap_or_else(|| task_id.clone());
            if !tracked.contains(&key) {
                tracked.push(key.clone());
            }
            if !key_by_task.iter().any(|(k, _)| k == &task_id) {
                key_by_task.push((task_id.clone(), key.clone()));
            }
            if ended.contains(&key) {
                resumed.insert(key);
            }
            continue;
        }
        let status = payload
            .and_then(|p| p.get("status"))
            .and_then(Value::as_str);
        let ended_now = get(activity, "kind").and_then(Value::as_str) == Some("task.completed")
            || (get(activity, "kind").and_then(Value::as_str) == Some("task.updated")
                && matches!(status, Some("completed") | Some("failed") | Some("killed")));
        if let Some((_, key)) = key_by_task.iter().find(|(k, _)| k == &task_id)
            && ended_now
        {
            ended.insert(key.clone());
            resumed.remove(key);
        }
    }
    if tracked.is_empty() {
        return Ok(no_tasks());
    }
    let live_turn = member(&input, "liveTurnId", "input.liveTurnId")?;
    let folded = fold_active_tasks(&activities, None)?;
    let mut running: Vec<(String, Value)> = Vec::new();
    for (task_id, task) in folded.entries() {
        if !is_agent_task(task) {
            continue;
        }
        let counts = task.tool_use_id.is_some()
            || task.is_backgrounded
            || live_turn.is_some_and(|t| !is_nullish(t) && strict_eq(&task.turn_id, t));
        if !counts {
            continue;
        }
        let key = task.tool_use_id.clone().unwrap_or_else(|| task_id.clone());
        if running.iter().any(|(k, _)| k == &key) {
            continue;
        }
        running.push((
            key.clone(),
            Value::Object(Map::from_iter([
                ("taskId".to_string(), Value::String(task_id.clone())),
                (
                    "toolUseId".to_string(),
                    task.tool_use_id
                        .clone()
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                ),
                ("resumed".to_string(), Value::Bool(resumed.contains(&key))),
            ])),
        ));
    }
    Ok(Value::Object(Map::from_iter([
        (
            "running".to_string(),
            Value::Array(running.into_iter().map(|(_, v)| v).collect()),
        ),
        (
            "trackedKeys".to_string(),
            Value::Object(Map::from_iter([(
                "$set".to_string(),
                Value::Array(tracked.into_iter().map(Value::String).collect()),
            )])),
        ),
    ])))
}

/// Port of `hasLiveTurnTailWork` (`apps/web/src/session-logic.ts:639`).
pub fn has_live_turn_tail_work(input: serde_json::Value) -> Result<bool, String> {
    let latest_turn = member(&input, "latestTurn", "input.latestTurn")?;
    let latest_turn_id = latest_turn
        .and_then(|t| get(t, "turnId"))
        .filter(|v| truthy(v));
    let Some(latest_turn_id) = latest_turn_id else {
        return Ok(false);
    };
    let messages = member(&input, "messages", "input.messages")?;
    let messages = match messages {
        Some(Value::Array(m)) => m,
        Some(v) if v.is_null() => {
            return Err("null is not an object (evaluating 'input.messages.some')".into());
        }
        Some(v) if crate::js::is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'input.messages.some')".into());
        }
        Some(_) => {
            return Err(
                "input.messages.some is not a function. (In 'input.messages.some((message) \
                 => message.role === \"assistant\" && message.turnId === latestTurnId && \
                 message.streaming)', 'input.messages.some' is undefined)"
                    .into(),
            );
        }
        None => {
            return Err("undefined is not an object (evaluating 'input.messages.some')".into());
        }
    };
    let streaming = messages.iter().any(|m| {
        get(m, "role").and_then(Value::as_str) == Some("assistant")
            && get(m, "turnId").is_some_and(|t| strict_eq(t, latest_turn_id))
            && get(m, "streaming").is_some_and(truthy)
    });
    if streaming {
        let completed = latest_turn.and_then(|t| get(t, "completedAt"));
        return Ok(completed.is_none_or(is_nullish));
    }
    let session = get(&input, "session");
    if session
        .and_then(|s| get(s, "orchestrationStatus"))
        .and_then(Value::as_str)
        != Some("running")
    {
        return Ok(false);
    }
    let activities = get(&input, "activities")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let derived = derive_active_background_tasks_state(activities, Some(latest_turn_id.clone()))?;
    Ok(derived.is_some())
}

// ---------- proposed-plan resolution ----------

fn to_latest_plan_state(plan: &Value) -> Value {
    let mut out = Map::new();
    for key in [
        "id",
        "createdAt",
        "updatedAt",
        "turnId",
        "planMarkdown",
        "implementedAt",
        "implementationThreadId",
    ] {
        out.insert(
            key.to_string(),
            get(plan, key).cloned().unwrap_or_else(crate::js::undefined),
        );
    }
    Value::Object(out)
}

/// Port of `findLatestProposedPlan` (`apps/web/src/session-logic.ts:674`).
pub fn find_latest_proposed_plan(
    proposed_plans: serde_json::Value,
    latest_turn_id: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let plans = spread(&proposed_plans)?;
    let sort_plans = |items: &[Value]| -> Result<Vec<Value>, String> {
        let mut out = items.to_vec();
        let mut first_err: Option<String> = None;
        crate::js::sort(&mut out, |l, r| {
            if first_err.is_some() {
                return 0.0;
            }
            let res = (|| {
                let u = locale_compare_field(l, r, "updatedAt", "left", "right")?;
                if u != 0 {
                    return Ok(u);
                }
                locale_compare_field(l, r, "id", "left", "right")
            })();
            match res {
                Ok(n) => n as f64,
                Err(e) => {
                    first_err = Some(e);
                    0.0
                }
            }
        });
        if let Some(e) = first_err {
            return Err(e);
        }
        Ok(out)
    };
    if truthy(&latest_turn_id) {
        let matching: Vec<Value> = plans
            .iter()
            .filter(|p| get(p, "turnId").is_some_and(|t| strict_eq(t, &latest_turn_id)))
            .cloned()
            .collect();
        let sorted = sort_plans(&matching)?;
        if let Some(plan) = sorted.last() {
            return Ok(Some(to_latest_plan_state(plan)));
        }
    }
    let sorted = sort_plans(&plans)?;
    Ok(sorted.last().map(to_latest_plan_state))
}

/// Port of `findSidebarProposedPlan` (`apps/web/src/session-logic.ts:704`).
pub fn find_sidebar_proposed_plan(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let threads = member(&input, "threads", "input.threads")?;
    let thread_id = get(&input, "threadId");
    let find_thread = |by: &dyn Fn(&Value) -> bool| -> Result<Option<Value>, String> {
        match threads {
            None => Err("undefined is not an object (evaluating 'input.threads.find')".to_string()),
            Some(v) if v.is_null() => {
                Err("null is not an object (evaluating 'input.threads.find')".to_string())
            }
            Some(v) if crate::js::is_undefined(v) => {
                Err("undefined is not an object (evaluating 'input.threads.find')".to_string())
            }
            Some(Value::Array(items)) => {
                Ok(items.iter().find(|t| get(t, "id").is_some_and(by)).cloned())
            }
            Some(_) => Err(
                "input.threads.find is not a function. (In 'input.threads.find((thread) => \
                 thread.id === input.threadId)', 'input.threads.find' is undefined)"
                    .to_string(),
            ),
        }
    };
    let active_plans = find_thread(&|id| thread_id.is_some_and(|t| strict_eq(id, t)))?
        .and_then(|t| get(&t, "proposedPlans").cloned())
        .unwrap_or_else(|| Value::Array(Vec::new()));

    let settled = get(&input, "latestTurnSettled").is_some_and(truthy);
    if !settled {
        let latest_turn = get(&input, "latestTurn");
        let source = latest_turn.and_then(|t| get(t, "sourceProposedPlan"));
        if let Some(source) = source.filter(|s| truthy(s)) {
            let src_thread_id = get(source, "threadId");
            let src_plan_id = get(source, "planId");
            let thread = find_thread(&|id| src_thread_id.is_some_and(|t| strict_eq(id, t)))?;
            if let Some(thread) = thread {
                let plans = get(&thread, "proposedPlans");
                if let Some(Value::Array(items)) = plans
                    && let Some(plan) = items.iter().find(|p| {
                        get(p, "id").is_some_and(|id| src_plan_id.is_some_and(|s| strict_eq(id, s)))
                    })
                {
                    return Ok(Some(to_latest_plan_state(plan)));
                }
            }
        }
    }
    let unimplemented: Vec<Value> = iterable(&active_plans)
        .unwrap_or(&[])
        .iter()
        .filter(|p| get(p, "implementedAt").is_some_and(|v| v.is_null()))
        .cloned()
        .collect();
    let latest_turn_id = get(&input, "latestTurn")
        .and_then(|t| get(t, "turnId"))
        .cloned()
        .unwrap_or(Value::Null);
    find_latest_proposed_plan(Value::Array(unimplemented), latest_turn_id)
}

/// Port of `hasActionableProposedPlan` (`apps/web/src/session-logic.ts:731`).
pub fn has_actionable_proposed_plan(proposed_plan: serde_json::Value) -> Result<bool, String> {
    if proposed_plan.is_null() {
        return Ok(false);
    }
    Ok(get(&proposed_plan, "implementedAt").is_some_and(|v| v.is_null()))
}

/// Port of `buildSourceProposedPlanReference` (`apps/web/src/session-logic.ts:737`).
pub fn build_source_proposed_plan_reference(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let plan = member(&input, "proposedPlan", "input.proposedPlan")?;
    let Some(plan) = plan.filter(|v| truthy(v)) else {
        return Ok(crate::js::undefined());
    };
    Ok(Value::Object(Map::from_iter([
        (
            "threadId".to_string(),
            get(&input, "threadId")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        ),
        (
            "planId".to_string(),
            get(plan, "id")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        ),
    ])))
}

/// Port of `inferCheckpointTurnCountByTurnId` (`apps/web/src/session-logic.ts:762`).
pub fn infer_checkpoint_turn_count_by_turn_id(
    summaries: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut sorted = spread(&summaries)?;
    let mut first_err: Option<String> = None;
    crate::js::sort(&mut sorted, |a, b| {
        if first_err.is_some() {
            return 0.0;
        }
        match locale_compare_field(a, b, "completedAt", "a", "b") {
            Ok(n) => n as f64,
            Err(e) => {
                first_err = Some(e);
                0.0
            }
        }
    });
    if let Some(e) = first_err {
        return Err(e);
    }
    let mut result = Map::new();
    for (index, summary) in sorted.iter().enumerate() {
        let turn = member(summary, "turnId", "summary.turnId")?;
        let key = turn
            .map(js_to_string)
            .unwrap_or_else(|| "undefined".to_string());
        result.insert(key, crate::js::number((index + 1) as f64));
    }
    Ok(Value::Object(result))
}

/// Port of `derivePhase` (`apps/web/src/session-logic.ts:775`).
pub fn derive_phase(session: Option<serde_json::Value>) -> Result<String, String> {
    let Some(session) = session.as_ref().filter(|s| truthy(s)) else {
        return Ok("disconnected".to_string());
    };
    let status = get(session, "status").and_then(Value::as_str);
    Ok(match status {
        Some("closed") => "disconnected",
        Some("connecting") => "connecting",
        Some("running") => "running",
        _ => "ready",
    }
    .to_string())
}
