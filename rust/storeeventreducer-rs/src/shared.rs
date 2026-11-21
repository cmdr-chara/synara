//! Shared helpers for the `storeEventReducer.ts` port: the shared-package and
//! cross-module functions the reducer calls, inlined so the crate is
//! self-contained.
//!
//! Provenance (each section copies a proven sibling port, adapted to `&Value`
//! inputs and JavaScriptCore error strings):
//! - `rust/session-logic-rs/src/port.rs` — activity ordering (`workLog.ts`
//!   `orderedActivities`), `backgroundTasks.ts` folds, and the session-logic
//!   derivations (`apps/web/src/session-logic.ts`).
//! - `rust/threadsummary-rs/src/port.rs` — `packages/shared/src/threadSummary.ts`.
//! - `rust/pendinginteractions-rs/src/port.rs` +
//!   `packages/shared/src/pendingInteractions.ts` — pending-interaction policy.
//! - `rust/pinnedmessages-rs/src/port.rs` — `packages/shared/src/pinnedMessages.ts`.
//! - `rust/asyncuserinput-rs/src/port.rs` — `packages/shared/src/asyncUserInput.ts`.
//! - `rust/sidechatthread-rs/src/port.rs` — `packages/shared/src/sidechatThread.ts`.
//! - `rust/turnstopactivity-rs/src/port.rs` — `packages/shared/src/turnStopActivity.ts`.
//! - `rust/messagetextsegments-rs/src/port.rs` — `apps/web/src/messageTextSegments.ts`.
//! - `rust/model-rs/src/port.rs` — `normalizeModelSlug` (`packages/shared/src/model.ts`).
//! - `rust/git-rs/src/port.rs` — `resolveThreadBranchRegressionGuard` (`packages/shared/src/git.ts`).
//! - `rust/threadpullrequestlinks-rs/src/port.rs` —
//!   `pullRequestLinksFromEventPayload` (`packages/shared/src/threadPullRequestLinks.ts`).
//! - `rust/threadworkspace-rs/src/port.rs` —
//!   `normalizeWorkspaceRootForComparison` (`packages/shared/src/threadWorkspace.ts`).

#![forbid(unsafe_code)]
#![allow(dead_code)]

use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

use crate::jsv;

// ---------- shared small helpers ----------

/// The JS `WhiteSpace` + `LineTerminator` + U+FEFF set `String.prototype.trim` removes.
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'
            | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `for (const x of v)`: arrays by element, strings (plain or `$utf16`-marked)
/// by UTF-16 code unit, everything else raises `X is not iterable`.
fn for_of(v: &Value) -> Result<Vec<Value>, String> {
    if let Some(units) = crate::js::string_units(v) {
        return Ok(units
            .iter()
            .map(|u| crate::js::utf16_string(&[*u]))
            .collect());
    }
    match v {
        Value::Array(items) => Ok(items.clone()),
        other => Err(format!("{} is not iterable", js_text(other))),
    }
}

/// Decode a `{"$utf16":[...]}` marker to a Rust string (lossy).
fn utf16_decode(v: &Value) -> String {
    let units: Vec<u16> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_u64().and_then(|u| u16::try_from(u).ok()))
                .collect()
        })
        .unwrap_or_default();
    String::from_utf16_lossy(&units)
}

/// `String(v)` coercion, including the `$utf16`/`$num` transport markers
/// (`jsv::js_to_string` alone renders them "[object Object]").
fn js_text(v: &Value) -> String {
    match v {
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => utf16_decode(&o["$utf16"]),
        _ => crate::js::as_number(v)
            .map(crate::js::number_to_string)
            .unwrap_or_else(|| jsv::js_to_string(v)),
    }
}

/// `+v`/`Number(v)` as `f64`; unconvertible values are NaN (including markers —
/// `jsv::to_number` decodes plain JSON, `js::as_number` decodes `$num`).
fn js_num(v: &Value) -> f64 {
    jsv::to_number(v)
        .or_else(|| crate::js::as_number(v))
        .unwrap_or(f64::NAN)
}

/// `a > b` (relational): UTF-16 code-unit order when both operands are strings,
/// else numeric (any NaN operand makes it false).
fn js_gt(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::string_units(a), crate::js::string_units(b)) {
        return x > y;
    }
    js_num(a) > js_num(b)
}

/// `a >= b` (relational), same operand rules as [`js_gt`].
fn js_ge(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::string_units(a), crate::js::string_units(b)) {
        return x >= y;
    }
    js_num(a) >= js_num(b)
}

/// `a <= b` (relational): `undefined` on either side is false, then the same
/// string/numeric rules as [`js_gt`].
fn js_le(a: &Value, b: &Value) -> bool {
    if crate::js::is_undefined(a) || crate::js::is_undefined(b) {
        return false;
    }
    if let (Some(x), Some(y)) = (crate::js::string_units(a), crate::js::string_units(b)) {
        return x <= y;
    }
    js_num(a) <= js_num(b)
}

/// `v === s` for a string literal `s` (plain or `$utf16`-marked strings).
fn str_is(v: &Value, s: &str) -> bool {
    crate::js::string_units(v).is_some_and(|u| u == crate::utf16::units(s))
}

// ---------- 1. orderedActivities (apps/web/src/workLog.ts) ----------

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
/// left values exactly as JSCore reports them.
fn locale_compare_field(
    left: &Value,
    right: &Value,
    field: &str,
    lname: &str,
    rname: &str,
) -> Result<i64, String> {
    let member_expr = format!("{lname}.{field}");
    let lv = jsv::member(left, field, &member_expr)?;
    let lv = lv.unwrap_or(&Value::Null);
    let is_missing = lv.is_null() || crate::js::is_undefined(lv);
    let call_expr = format!("{lname}.{field}.localeCompare({rname}.{field})");
    if is_missing && jsv::get(left, field).is_none() || crate::js::is_undefined(lv) {
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
    let rv = jsv::get(right, field);
    let rs = rv.map(js_text).unwrap_or_else(|| "undefined".to_string());
    Ok(jsv::locale_compare(ls, &rs))
}

fn compare_activities(a: &Value, b: &Value) -> Result<f64, String> {
    // `.sequence` on primitives yields undefined; on nullish elements it throws.
    let ls = jsv::member(a, "sequence", "left.sequence")?;
    let rs = jsv::member(b, "sequence", "right.sequence")?;
    let l_def = ls.is_some_and(|v| !crate::js::is_undefined(v));
    let r_def = rs.is_some_and(|v| !crate::js::is_undefined(v));
    if l_def && r_def {
        if let (Some(x), Some(y)) = (ls, rs)
            && !jsv::strict_eq(x, y)
        {
            // `left.sequence - right.sequence` — numeric coercion (null→0,
            // bool→1, string→parse, object→NaN).
            return Ok(js_num(x) - js_num(y));
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
    let rank = lifecycle_rank(jsv::get(a, "kind").unwrap_or(&Value::Null))
        - lifecycle_rank(jsv::get(b, "kind").unwrap_or(&Value::Null));
    if rank != 0.0 {
        return Ok(rank);
    }
    if jsv::get(a, "kind").is_some_and(|k| k.as_str() == Some("context-compaction"))
        && jsv::get(b, "kind").is_some_and(|k| k.as_str() == Some("context-compaction"))
    {
        let cr = compaction_rank(jsv::get(a, "summary")) - compaction_rank(jsv::get(b, "summary"));
        if cr != 0.0 {
            return Ok(cr);
        }
    }
    Ok(locale_compare_field(a, b, "id", "left", "right")? as f64)
}

/// `orderedActivities` — including the WeakMap cache semantics: `get` never
/// throws, `set` throws for primitive keys after a stable (or sorted) pass.
pub(crate) fn ordered_activities(activities: &Value) -> Result<Value, String> {
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
        Value::Object(_) => {
            // `isActivityOrderStable` reads `activities.length` then indexes —
            // an object with a numeric length still runs the comparator.
            let len = jsv::get(activities, "length").map(js_num);
            if let Some(len) = len {
                let mut index = 1usize;
                while (index as f64) < len {
                    let left = jsv::opt(activities, &(index - 1).to_string());
                    let right = jsv::opt(activities, &index.to_string());
                    compare_activities(&left, &right)?;
                    index += 1;
                }
            }
            activities.clone()
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

/// Iterate a value as `for..of` does: arrays iterate; anything else reports
/// `X is not iterable`.
fn iterable(v: &Value) -> Result<&[Value], String> {
    match v {
        Value::Array(items) => Ok(items),
        other => Err(format!("{} is not iterable", jsv::js_to_string(other))),
    }
}

/// `[...x]`: arrays copy, strings spread UTF-16 units, nullish/other values throw.
fn spread(v: &Value) -> Result<Vec<Value>, String> {
    if v.is_null() || crate::js::is_undefined(v) {
        return Err("Spread syntax requires ...iterable not be null or undefined".to_string());
    }
    if let Some(units) = crate::js::string_units(v) {
        return Ok(units
            .iter()
            .map(|u| crate::js::utf16_string(&[*u]))
            .collect());
    }
    match v {
        Value::Array(items) => Ok(items.clone()),
        _ => {
            Err("Spread syntax requires ...iterable[Symbol.iterator] to be a function".to_string())
        }
    }
}

// ---------- 2. session-logic (apps/web/src/session-logic.ts + packages/shared/src/backgroundTasks.ts) ----------

const BACKGROUND_TASKS_CHANGED: &str = "background_tasks_changed";

fn as_record(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(o) => Some(o),
        _ => None,
    }
}

/// `foldOutstandingBackgroundTaskIds` — `asRecord(activity.payload)` is the
/// first member read on each element, so a nullish element throws at
/// `activity.payload`.
fn fold_outstanding_background_task_ids(ordered: &[Value]) -> Result<Vec<String>, String> {
    let mut outstanding: Vec<String> = Vec::new();
    for activity in ordered {
        let payload = jsv::member(activity, "payload", "activity.payload")?.and_then(as_record);
        if jsv::get(activity, "kind").and_then(Value::as_str) == Some("runtime.warning") {
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
        match jsv::get(activity, "kind").and_then(Value::as_str) {
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
    Ok(outstanding)
}

fn can_session_finish_background_tasks(status: Option<&Value>) -> bool {
    match status {
        Some(v) if !v.is_null() && !crate::js::is_undefined(v) => {
            v.as_str() != Some("stopped") && v.as_str() != Some("error")
        }
        _ => false,
    }
}

/// `isBackgroundTaskActivity` (`packages/shared/src/backgroundTasks.ts`), aliased
/// `isBackgroundWorkActivity` in session-logic. TS reads `activity.kind`, which
/// throws on a nullish activity; this `bool` signature maps that to `false`.
pub(crate) fn is_background_work_activity(activity: &Value) -> bool {
    match jsv::get(activity, "kind").and_then(Value::as_str) {
        Some("task.updated") | Some("task.completed") => true,
        Some("runtime.warning") => {
            jsv::get(activity, "payload")
                .and_then(as_record)
                .and_then(|p| p.get("nativeEventType"))
                .and_then(Value::as_str)
                == Some(BACKGROUND_TASKS_CHANGED)
        }
        _ => false,
    }
}

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
    let ordered = iterable(&ordered)?.to_vec();
    let mut active = TaskSet::new();
    for activity in &ordered {
        // First evaluated member read on the element decides the error a
        // nullish activity raises: `activity.turnId` when `latestTurnId` is
        // truthy, else `activity.kind`.
        let act_turn = match latest_turn_id {
            Some(tid) if jsv::truthy(tid) => jsv::member(activity, "turnId", "activity.turnId")?,
            _ => jsv::get(activity, "turnId"),
        };
        let kind = jsv::member(activity, "kind", "activity.kind")?.and_then(Value::as_str);
        if let (Some(tid), Some(at)) = (latest_turn_id, act_turn)
            && jsv::truthy(tid)
            && jsv::truthy(at)
            && !jsv::strict_eq(at, tid)
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
        let payload = jsv::get(activity, "payload").and_then(as_record);
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
            // `!latestTurnId || !activity.turnId || activity.turnId === latestTurnId`
            let in_turn = match latest_turn_id {
                Some(tid) if jsv::truthy(tid) => match act_turn {
                    Some(at) if jsv::truthy(at) => jsv::strict_eq(at, tid),
                    _ => true,
                },
                _ => true,
            };
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
                            ref t if !jsv::is_nullish(t) => t.clone(),
                            // `activity.turnId ?? null`
                            _ => act_turn
                                .cloned()
                                .filter(|v| !jsv::is_nullish(v))
                                .unwrap_or(Value::Null),
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
                    ref t if !jsv::is_nullish(t) => t.clone(),
                    // `activity.turnId ?? null`
                    _ => act_turn
                        .cloned()
                        .filter(|v| !jsv::is_nullish(v))
                        .unwrap_or(Value::Null),
                },
            },
        );
    }
    Ok(active)
}

/// `deriveActiveBackgroundTasksState` (`session-logic.ts:462`).
fn derive_active_background_tasks_state(
    activities: &Value,
    latest_turn_id: Option<&Value>,
) -> Result<Option<Value>, String> {
    let folded = fold_active_tasks(activities, latest_turn_id)?;
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

/// `deriveOutstandingBackgroundTaskIds` (`session-logic.ts:511`).
fn derive_outstanding_background_task_ids(activities: &Value) -> Result<Vec<String>, String> {
    let ordered = ordered_activities(activities)?;
    let items = iterable(&ordered)?;
    fold_outstanding_background_task_ids(items)
}

/// `isSessionRunningTurn` (`session-logic.ts:272`): `session != null &&
/// session.status === "running" && session.activeTurnId != null`. Non-object
/// sessions read members as `undefined`, so this is `false` for them.
pub(crate) fn is_session_running_turn(session: &Value) -> bool {
    jsv::get(session, "status").and_then(Value::as_str) == Some("running")
        && jsv::get(session, "activeTurnId").is_some_and(|v| !jsv::is_nullish(v))
}

/// `hasLiveTurnTailWork` (`session-logic.ts:639`). NOTE: the TS signature is a
/// single input object `{ latestTurn, messages, activities, session? }` — the
/// multi-parameter shape from the spec draft does not exist in the source.
pub(crate) fn has_live_turn_tail_work(input: &Value) -> Result<bool, String> {
    let latest_turn = jsv::member(input, "latestTurn", "input.latestTurn")?;
    let latest_turn_id = latest_turn
        .and_then(|t| jsv::get(t, "turnId"))
        .filter(|v| jsv::truthy(v));
    let Some(latest_turn_id) = latest_turn_id else {
        return Ok(false);
    };
    let messages = jsv::member(input, "messages", "input.messages")?;
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
    // `input.messages.some((message) => message.role === "assistant" && ...)` —
    // `message.role` is the first member read on each element.
    let mut streaming = false;
    for m in messages {
        let role = jsv::member(m, "role", "message.role")?;
        if role.and_then(Value::as_str) == Some("assistant")
            && jsv::get(m, "turnId").is_some_and(|t| jsv::strict_eq(t, latest_turn_id))
            && jsv::get(m, "streaming").is_some_and(jsv::truthy)
        {
            streaming = true;
            break;
        }
    }
    if streaming {
        let completed = latest_turn.and_then(|t| jsv::get(t, "completedAt"));
        return Ok(completed.is_none_or(jsv::is_nullish));
    }
    let session = jsv::get(input, "session");
    if session
        .and_then(|s| jsv::get(s, "orchestrationStatus"))
        .and_then(Value::as_str)
        != Some("running")
    {
        return Ok(false);
    }
    let activities = jsv::get(input, "activities")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let derived = derive_active_background_tasks_state(&activities, Some(latest_turn_id))?;
    Ok(derived.is_some())
}

/// `derivePendingBackgroundWork` (`session-logic.ts:481`). NOTE: the TS
/// signature is a single input object `{ activities, latestTurn, session }`;
/// the spec draft's `(activities, sessionStatus, tasksChangedAt)` does not exist
/// in the source. `null` return maps to `Value::Null`.
pub(crate) fn derive_pending_background_work(input: &Value) -> Result<Value, String> {
    let latest_turn = jsv::member(input, "latestTurn", "input.latestTurn")?;
    let latest_turn_id = latest_turn
        .and_then(|t| jsv::get(t, "turnId"))
        .filter(|v| jsv::truthy(v));
    let Some(latest_turn_id) = latest_turn_id else {
        return Ok(Value::Null);
    };
    let session_status = jsv::member(input, "session", "input.session")?
        .and_then(|s| jsv::get(s, "orchestrationStatus"));
    if session_status.is_none()
        || session_status.is_some_and(crate::js::is_undefined)
        || session_status.and_then(Value::as_str) == Some("stopped")
        || session_status.and_then(Value::as_str) == Some("error")
    {
        return Ok(Value::Null);
    }
    let activities = jsv::member(input, "activities", "input.activities")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let folded = fold_active_tasks(&activities, Some(latest_turn_id))?;
    let ids: Vec<String> = folded
        .entries()
        .filter(|(_, t)| t.is_backgrounded)
        .map(|(id, _)| id.clone())
        .collect();
    if ids.is_empty() {
        return Ok(Value::Null);
    }
    Ok(Value::Object(Map::from_iter([
        ("count".to_string(), crate::js::number(ids.len() as f64)),
        (
            "taskIds".to_string(),
            Value::Array(ids.iter().cloned().map(Value::String).collect()),
        ),
    ])))
}

/// `countOutstandingBackgroundWork` (`session-logic.ts:519`). NOTE: the TS
/// signature is a single input object `{ activities, session }`, not the
/// `(activities, sessionStatus)` pair from the spec draft.
pub(crate) fn count_outstanding_background_work(input: &Value) -> Result<Value, String> {
    let session = jsv::member(input, "session", "input.session")?;
    let status = session.and_then(|s| jsv::get(s, "orchestrationStatus"));
    if !can_session_finish_background_tasks(status) {
        return Ok(crate::js::number(0.0));
    }
    let activities = jsv::member(input, "activities", "input.activities")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let ids = derive_outstanding_background_task_ids(&activities)?;
    Ok(crate::js::number(ids.len() as f64))
}

// ---------- `Date.parse` (JSCore lenient parser, from session-logic-rs) ----------

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

// ---------- turn/session predicates kept for the reducer's session-logic use ----------

fn latest_started(latest_turn: Option<&Value>) -> bool {
    latest_turn
        .and_then(|t| jsv::get(t, "startedAt"))
        .is_some_and(jsv::truthy)
}

fn is_latest_turn_settled_inner(latest_turn: Option<&Value>, session: Option<&Value>) -> bool {
    if !latest_started(latest_turn) {
        return false;
    }
    if !latest_turn
        .and_then(|t| jsv::get(t, "completedAt"))
        .is_some_and(jsv::truthy)
    {
        return false;
    }
    let Some(turn) = latest_turn else {
        return true;
    };
    if jsv::get(turn, "state").and_then(Value::as_str) == Some("interrupted")
        || jsv::get(turn, "state").and_then(Value::as_str) == Some("error")
    {
        return true;
    }
    let Some(session) = session else {
        return true;
    };
    jsv::get(session, "orchestrationStatus").and_then(Value::as_str) != Some("running")
}

fn running_active_turn(session: Option<&Value>) -> Value {
    match session {
        Some(s)
            if jsv::get(s, "orchestrationStatus").and_then(Value::as_str) == Some("running") =>
        {
            // `session.activeTurnId ?? null`
            jsv::get(s, "activeTurnId")
                .cloned()
                .filter(|v| !jsv::is_nullish(v))
                .unwrap_or(Value::Null)
        }
        _ => Value::Null,
    }
}

// ---------- 3. threadSummary (packages/shared/src/threadSummary.ts) ----------

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// `pendingRequestInstanceKey`: `${requestId}\u0000${lifecycleGeneration ?? "legacy"}`.
/// The spec signature takes `&Value` args; a nullish generation coalesces to
/// "legacy" exactly as `??` does.
pub(crate) fn pending_request_instance_key(
    request_id: &Value,
    lifecycle_generation: Option<&Value>,
) -> String {
    let generation = match lifecycle_generation {
        Some(g) if !jsv::is_nullish(g) => js_text(g),
        _ => "legacy".to_string(),
    };
    format!("{}\u{0}{generation}", js_text(request_id))
}

/// `left > right` on transported values: string pairs compare lexicographically,
/// else numerically (a NaN or non-comparable operand makes it false → `right`).
fn max_iso(left: &Value, right: &Value) -> Value {
    if left.is_null() {
        return right.clone();
    }
    let gt = match (
        crate::js::string_units(left),
        crate::js::string_units(right),
    ) {
        (Some(l), Some(r)) => l > r,
        _ => match (crate::js::as_number(left), crate::js::as_number(right)) {
            (Some(l), Some(r)) => l > r,
            _ => false,
        },
    };
    if gt { left.clone() } else { right.clone() }
}

/// threadSummary's own `compareActivitiesByOrder`: `left.sequence ?? MAX_SAFE`,
/// then `createdAt`/`id` localeCompare. Distinct from workLog's rank-aware
/// [`compare_activities`].
fn ts_compare_activities(left: &Value, right: &Value) -> Result<i64, String> {
    let ls = jsv::member(left, "sequence", "left.sequence")?;
    let rs = jsv::member(right, "sequence", "right.sequence")?;
    // `?? Number.MAX_SAFE_INTEGER`: nullish coalesces; other values subtract
    // with `Number()` coercion.
    let seq = |v: Option<&Value>| -> f64 {
        match v {
            Some(v) if !jsv::is_nullish(v) => js_num(v),
            _ => MAX_SAFE_INTEGER,
        }
    };
    let diff = seq(ls) - seq(rs);
    // `diff || ...`: NaN is falsy and falls through to localeCompare.
    if diff != 0.0 && !diff.is_nan() {
        return Ok(diff as i64);
    }
    let c = locale_compare_field(left, right, "createdAt", "left", "right")?;
    if c != 0 {
        return Ok(c);
    }
    locale_compare_field(left, right, "id", "left", "right")
}

/// threadSummary's `orderedActivities`: stable-precheck, `.toSorted` fallback,
/// same WeakMap-cache throwing surface as [`ordered_activities`].
fn ts_ordered_activities(activities: &Value) -> Result<Value, String> {
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
                    if ts_compare_activities(&out[i - 1], &out[i])? > 0 {
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
                    match ts_compare_activities(a, b) {
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
            }
            Value::Array(out)
        }
        Value::String(s) => {
            let chars: Vec<Value> = s
                .encode_utf16()
                .map(|u| crate::js::utf16_string(&[u]))
                .collect();
            for i in 1..chars.len() {
                ts_compare_activities(&chars[i - 1], &chars[i])?;
            }
            return Err("WeakMap keys must be objects or non-registered symbols".to_string());
        }
        Value::Object(_) => {
            let len = jsv::get(activities, "length").map(js_num);
            if let Some(len) = len {
                let mut index = 1usize;
                while (index as f64) < len {
                    let left = jsv::opt(activities, &(index - 1).to_string());
                    let right = jsv::opt(activities, &index.to_string());
                    ts_compare_activities(&left, &right)?;
                    index += 1;
                }
            }
            activities.clone()
        }
        _ => activities.clone(),
    };
    if matches!(ordered, Value::Array(_) | Value::Object(_)) {
        return Ok(ordered);
    }
    Err("WeakMap keys must be objects or non-registered symbols".to_string())
}

/// `approvalRequestKindFromRequestType`: `unknown` → `ApprovalRequestKind | null`.
fn approval_request_kind_from_request_type(request_type: &Value) -> Option<String> {
    let kind = match request_type.as_str() {
        Some("command_execution_approval") | Some("exec_command_approval") => "command",
        Some("file_read_approval") => "file-read",
        Some("file_change_approval") | Some("apply_patch_approval") => "file-change",
        Some("permissions_approval") => "permissions",
        // `tool_approval` is canonical; `dynamic_tool_call` is the legacy value.
        Some("tool_approval") | Some("dynamic_tool_call") => "tool",
        _ => return None,
    };
    Some(kind.to_string())
}

/// `isStalePendingRequestFailureDetail` (`threadSummary.ts:119`). The spec
/// signature takes `Option<&str>` (the TS `detail: string | undefined`).
pub(crate) fn is_stale_pending_request_failure_detail(detail: Option<&str>) -> bool {
    let Some(detail) = detail else {
        return false;
    };
    if detail.is_empty() {
        return false;
    }
    let normalized = detail.to_lowercase();
    [
        "stale pending approval request",
        "stale pending user-input request",
        "unknown pending approval request",
        "unknown pending permission request",
        "unknown pending user-input request",
        "stale pending user input request",
        "unknown pending user input request",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

/// `buildStalePendingRequestFailureDetail` (`threadSummary.ts:165`).
fn build_stale_pending_request_failure_detail(request_kind: &str, request_id: &str) -> String {
    format!(
        "Stale pending {request_kind} request: {request_id}. Provider callback state does not survive app restarts or recovered sessions. Restart the turn to continue."
    )
}

fn lifecycle_generation_from_payload(payload: Option<&Map<String, Value>>) -> Option<Value> {
    payload
        .and_then(|p| p.get("lifecycleGeneration"))
        .and_then(Value::as_str)
        .filter(|g| !g.is_empty())
        .map(|g| Value::String(g.to_string()))
}

// `deleteOpenRequest`: with a generation only that instance key is removed;
// without one every open request carrying this id goes.
fn delete_open_request(
    open: &mut Vec<(String, Value)>,
    request_id: &Value,
    generation: Option<&Value>,
) {
    if generation.is_some() {
        let key = pending_request_instance_key(request_id, generation);
        open.retain(|(k, _)| k != &key);
        return;
    }
    open.retain(|(_, v)| v != request_id);
}

fn replace_open_request(
    open: &mut Vec<(String, Value)>,
    request_id: &Value,
    generation: Option<&Value>,
) {
    delete_open_request(open, request_id, None);
    open.push((
        pending_request_instance_key(request_id, generation),
        request_id.clone(),
    ));
}

fn has_structured_user_input_questions(payload: Option<&Map<String, Value>>) -> bool {
    let Some(questions) = payload
        .and_then(|p| p.get("questions"))
        .and_then(Value::as_array)
    else {
        return false;
    };
    questions.iter().any(|entry| {
        let Some(question) = entry.as_object() else {
            return false;
        };
        let has_strings = ["id", "header", "question"]
            .iter()
            .all(|k| question.get(*k).is_some_and(Value::is_string));
        has_strings
            && question
                .get("options")
                .and_then(Value::as_array)
                .is_some_and(|options| {
                    options.iter().any(|option| {
                        option.as_object().is_some_and(|o| {
                            o.get("label").is_some_and(Value::is_string)
                                && o.get("description").is_some_and(Value::is_string)
                        })
                    })
                })
    })
}

fn request_ids_payload(approvals: Vec<(String, Value)>, inputs: Vec<(String, Value)>) -> Value {
    let mut out = Map::new();
    out.insert(
        "approvalRequestIds".to_string(),
        Value::Array(approvals.into_iter().map(|(_, v)| v).collect()),
    );
    out.insert(
        "userInputRequestIds".to_string(),
        Value::Array(inputs.into_iter().map(|(_, v)| v).collect()),
    );
    Value::Object(out)
}

/// `derivePendingThreadRequestIds` (`threadSummary.ts:231`).
fn derive_pending_thread_request_ids(input: &Value) -> Result<Value, String> {
    let pending = jsv::member(input, "pendingInteractions", "input.pendingInteractions")?;
    if let Some(pending) = pending.filter(|p| !crate::js::is_undefined(p)) {
        let mut approvals: Vec<(String, Value)> = Vec::new();
        let mut inputs: Vec<(String, Value)> = Vec::new();
        // `input.pendingInteractions ?? []`: nullish yields an empty projection;
        // other values must be iterable.
        let interactions: Vec<Value> = match pending {
            v if jsv::is_nullish(v) => Vec::new(),
            other => for_of(other)?,
        };
        for interaction in &interactions {
            let is_approval = jsv::member(
                interaction,
                "interactionKind",
                "interaction.interactionKind",
            )?
            .and_then(Value::as_str)
                == Some("approval");
            let status = jsv::get(interaction, "status").and_then(Value::as_str);
            if status != Some("pending") && status != Some("retryable") {
                continue;
            }
            let request_id = jsv::get(interaction, "requestId")
                .cloned()
                .unwrap_or_else(crate::js::undefined);
            // `interaction.lifecycleGeneration ?? undefined`: nullish drops to None.
            let generation =
                jsv::get(interaction, "lifecycleGeneration").filter(|g| !jsv::is_nullish(g));
            let key = pending_request_instance_key(&request_id, generation);
            let open = if is_approval {
                &mut approvals
            } else {
                &mut inputs
            };
            open.retain(|(k, _)| k != &key);
            open.push((key, request_id));
        }
        return Ok(request_ids_payload(approvals, inputs));
    }

    let activities = jsv::member(input, "activities", "input.activities")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let ordered = ts_ordered_activities(&activities)?;
    let mut open_approvals: Vec<(String, Value)> = Vec::new();
    let mut open_inputs: Vec<(String, Value)> = Vec::new();
    for activity in for_of(&ordered)? {
        // `toPayloadRecord(activity.payload)` — the payload member read itself
        // throws on a nullish activity.
        let payload = jsv::member(&activity, "payload", "activity.payload")?;
        let payload = payload.and_then(as_record);
        let request_id = payload
            .and_then(|p| p.get("requestId"))
            .filter(|v| v.is_string())
            .cloned();
        let detail = payload
            .and_then(|p| p.get("detail"))
            .and_then(Value::as_str);
        let generation = lifecycle_generation_from_payload(payload);
        let Some(request_id) = request_id else {
            continue;
        };
        match jsv::get(&activity, "kind")
            .and_then(Value::as_str)
            .unwrap_or("")
        {
            "approval.requested" => {
                let request_kind = payload
                    .and_then(|p| p.get("requestKind"))
                    .and_then(Value::as_str)
                    .filter(|k| {
                        ["command", "file-read", "file-change", "permissions", "tool"].contains(k)
                    })
                    .map(str::to_string)
                    .or_else(|| {
                        approval_request_kind_from_request_type(
                            payload
                                .and_then(|p| p.get("requestType"))
                                .unwrap_or(&Value::Null),
                        )
                    });
                if request_kind.is_some() {
                    replace_open_request(&mut open_approvals, &request_id, generation.as_ref());
                }
            }
            "approval.resolved" => {
                delete_open_request(&mut open_approvals, &request_id, generation.as_ref());
            }
            "provider.approval.respond.failed"
                if is_stale_pending_request_failure_detail(detail) =>
            {
                delete_open_request(&mut open_approvals, &request_id, generation.as_ref());
            }
            "user-input.requested" => {
                if has_structured_user_input_questions(payload) {
                    replace_open_request(&mut open_inputs, &request_id, generation.as_ref());
                }
            }
            "user-input.resolved" => {
                delete_open_request(&mut open_inputs, &request_id, generation.as_ref());
            }
            "provider.user-input.respond.failed"
                if is_stale_pending_request_failure_detail(detail) =>
            {
                delete_open_request(&mut open_inputs, &request_id, generation.as_ref());
            }
            _ => {}
        }
    }
    Ok(request_ids_payload(open_approvals, open_inputs))
}

/// `resolveHumanMessageAt` (`threadSummary.ts:338`). TS reads `message.role`
/// first, so a null/undefined message raises here (it does not return `null`).
pub(crate) fn resolve_human_message_at(message: &Value) -> Result<Value, String> {
    let role = jsv::member(message, "role", "message.role")?;
    if role.and_then(Value::as_str) != Some("user") {
        return Ok(Value::Null);
    }
    // `dispatchOrigin != null && dispatchOrigin !== "user"` (loose `!=`).
    match jsv::get(message, "dispatchOrigin") {
        Some(origin)
            if !origin.is_null()
                && !crate::js::is_undefined(origin)
                && origin.as_str() != Some("user") =>
        {
            return Ok(Value::Null);
        }
        _ => {}
    }
    let created = jsv::opt(message, "createdAt");
    let right = jsv::get(message, "updatedAt")
        .filter(|u| !jsv::is_nullish(u))
        .cloned()
        .unwrap_or_else(|| created.clone());
    Ok(max_iso(&created, &right))
}

fn iter_messages(input: &Value) -> Result<Vec<Value>, String> {
    let messages = jsv::member(input, "messages", "input.messages")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    for_of(&messages)
}

fn resolve_latest_proposed_plan(input: &Value) -> Result<Option<Value>, String> {
    // TS reads `input.latestTurn?.turnId` before `[...input.proposedPlans]`.
    let turn_id =
        jsv::member(input, "latestTurn", "input.latestTurn")?.and_then(|t| jsv::get(t, "turnId"));
    let plans = jsv::member(input, "proposedPlans", "input.proposedPlans")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let plans = spread(&plans)?;
    let by_order = |a: &Value, b: &Value| -> Result<i64, String> {
        let c = locale_compare_field(a, b, "updatedAt", "left", "right")?;
        if c != 0 {
            return Ok(c);
        }
        locale_compare_field(a, b, "id", "left", "right")
    };
    let sort_plans = |items: Vec<Value>| -> Result<Vec<Value>, String> {
        let mut out = items;
        let mut first_err: Option<String> = None;
        crate::js::sort(&mut out, |l, r| {
            if first_err.is_some() {
                return 0.0;
            }
            match by_order(l, r) {
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
    if let Some(turn_id) = turn_id.filter(|t| jsv::truthy(t)) {
        // `plan.turnId === input.latestTurn?.turnId` — the member read throws
        // on a nullish plan element.
        let mut matching: Vec<Value> = Vec::new();
        for p in &plans {
            if jsv::member(p, "turnId", "plan.turnId")?.is_some_and(|t| jsv::strict_eq(t, turn_id))
            {
                matching.push(p.clone());
            }
        }
        let sorted = sort_plans(matching)?;
        if let Some(plan) = sorted.last() {
            return Ok(Some(plan.clone()));
        }
    }
    let sorted = sort_plans(plans)?;
    Ok(sorted.last().cloned())
}

fn derive_state(input: &Value) -> Result<Value, String> {
    let mut latest_user_at = Value::Null;
    let mut latest_human_at = Value::Null;
    for message in iter_messages(input)? {
        let role = jsv::member(&message, "role", "message.role")?;
        if role.and_then(Value::as_str) != Some("user") {
            continue;
        }
        latest_user_at = max_iso(&latest_user_at, &jsv::opt(&message, "createdAt"));
        let human_at = resolve_human_message_at(&message)?;
        if !human_at.is_null() {
            latest_human_at = max_iso(&latest_human_at, &human_at);
        }
    }
    let pending = derive_pending_thread_request_ids(input)?;
    let approvals = pending
        .get("approvalRequestIds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let inputs = pending
        .get("userInputRequestIds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let plan = resolve_latest_proposed_plan(input)?;
    // `latestProposedPlan?.implementedAt === null` — strict null, not undefined.
    let actionable = plan
        .as_ref()
        .is_some_and(|p| jsv::get(p, "implementedAt").is_some_and(Value::is_null));
    let mut out = Map::new();
    out.insert("latestUserMessageAt".to_string(), latest_user_at);
    out.insert("latestHumanMessageAt".to_string(), latest_human_at);
    out.insert(
        "pendingApprovalCount".to_string(),
        crate::js::number(approvals.len() as f64),
    );
    out.insert(
        "pendingUserInputCount".to_string(),
        crate::js::number(inputs.len() as f64),
    );
    out.insert(
        "hasPendingApprovals".to_string(),
        Value::Bool(!approvals.is_empty()),
    );
    out.insert(
        "hasPendingUserInput".to_string(),
        Value::Bool(!inputs.is_empty()),
    );
    out.insert(
        "hasActionableProposedPlan".to_string(),
        Value::Bool(actionable),
    );
    Ok(Value::Object(out))
}

/// `deriveThreadSummaryMetadata` (`threadSummary.ts:398`).
pub(crate) fn derive_thread_summary_metadata(input: &Value) -> Result<Value, String> {
    let state = derive_state(input)?;
    let mut out = Map::new();
    for key in [
        "latestUserMessageAt",
        "latestHumanMessageAt",
        "hasPendingApprovals",
        "hasPendingUserInput",
        "hasActionableProposedPlan",
    ] {
        out.insert(
            key.to_string(),
            state.get(key).cloned().unwrap_or(Value::Null),
        );
    }
    Ok(Value::Object(out))
}

// ---------- 4. pendingInteractions (packages/shared/src/pendingInteractions.ts) ----------

const GRACE_MS: f64 = 30_000.0;
const MS_PER_DAY: f64 = 86_400_000.0;

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    (
        y + if mp < 10 { 0 } else { 1 },
        if mp < 10 { mp + 3 } else { mp - 9 },
        doy - (153 * mp + 2) / 5 + 1,
    )
}

/// `new Date(ms).toISOString()`.
fn ms_to_iso(t: f64) -> String {
    let days = (t / MS_PER_DAY).floor() as i64;
    let rem = (t - days as f64 * MS_PER_DAY) as i64;
    let (h, mi, s, ms) = (
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000,
    );
    let (y, m, d) = civil_from_days(days);
    let ys = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else if y < 0 {
        format!("-{abs:06}", abs = -y)
    } else {
        format!("+{y:06}")
    };
    format!("{ys}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}Z")
}

/// `Date.parse(v)` — strings decode directly, other values coerce then parse.
/// NOTE: this crate's corpus is Bun/JSCore, so this shares the lenient JSC
/// [`date_parse`] from the session-logic port rather than the V8-strict parser
/// `pendinginteractions-rs` used for its own corpus.
fn pi_parse_arg(v: &Value) -> Option<f64> {
    let s = match crate::js::string_units(v) {
        Some(units) => String::from_utf16_lossy(&units),
        None => js_text(v),
    };
    let ms = date_parse(&s);
    if ms.is_nan() { None } else { Some(ms) }
}

fn reclaim(at: &Value, delta: f64) -> Value {
    match pi_parse_arg(at) {
        Some(ms) => Value::String(ms_to_iso(ms + delta)),
        None => at.clone(),
    }
}

/// `respondingInteractionReclaimCutoff` (`pendingInteractions.ts:91`).
fn responding_interaction_reclaim_cutoff(requested_at: &Value) -> Value {
    reclaim(requested_at, -GRACE_MS)
}

/// `isPendingInteractionResponseClaimable` (`pendingInteractions.ts:115`). The
/// spec signature takes the single `{ status, responseRequestedAt, requestedAt }`
/// input object, matching the TS.
pub(crate) fn is_pending_interaction_response_claimable(input: &Value) -> Result<bool, String> {
    let status = jsv::member(input, "status", "input.status")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    if str_is(&status, "pending") || str_is(&status, "retryable") || str_is(&status, "uncertain") {
        return Ok(true);
    }
    if !str_is(&status, "responding") {
        return Ok(false);
    }
    let rra = jsv::member(input, "responseRequestedAt", "input.responseRequestedAt")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    if rra.is_null() {
        return Ok(true);
    }
    let requested_at = jsv::member(input, "requestedAt", "input.requestedAt")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let cutoff = responding_interaction_reclaim_cutoff(&requested_at);
    Ok(js_le(&rra, &cutoff))
}

/// Fused `createStalePendingInteractionMatcher(activities)` +
/// `isStale(interaction)`: builds the matcher's stale-instance set from
/// `activities` and evaluates it against `interaction` in one call
/// (`packages/shared/src/pendingInteractions.ts:29`). NOTE: the current TS
/// matcher takes only `activities`; `last_pending_seen_at` is accepted for the
/// required call shape but is not read (no such parameter exists in the source).
pub(crate) fn is_stale_pending_interaction(
    interaction: &Value,
    activities: &Value,
    last_pending_seen_at: &Value,
) -> Result<bool, String> {
    let _ = last_pending_seen_at;
    let mut stale_at_by_instance: HashMap<String, Value> = HashMap::new();
    for activity in for_of(activities)? {
        let kind_label =
            match jsv::member(&activity, "kind", "activity.kind")?.and_then(Value::as_str) {
                Some("provider.approval.respond.failed") => "approval",
                Some("provider.user-input.respond.failed") => "userInput",
                _ => continue,
            };
        // `activity.payload === null || typeof activity.payload !== "object"`:
        // only objects carry the fields read below (arrays reach the same skip
        // through the `requestId` string check).
        let payload = match jsv::get(&activity, "payload") {
            Some(Value::Object(o)) => o,
            _ => continue,
        };
        let Some(request_id) = payload.get("requestId").and_then(Value::as_str) else {
            continue;
        };
        let detail = payload.get("detail").and_then(Value::as_str);
        if !is_stale_pending_request_failure_detail(detail) {
            continue;
        }
        let generation = payload
            .get("lifecycleGeneration")
            .and_then(Value::as_str)
            .filter(|g| !g.is_empty());
        let key = format!(
            "{kind_label}\u{0}{request_id}\u{0}{}",
            generation.unwrap_or("legacy")
        );
        let created_at = jsv::get(&activity, "createdAt")
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        // `previous === undefined || activity.createdAt > previous`
        let replace = match stale_at_by_instance.get(&key) {
            None => true,
            Some(previous) => js_gt(&created_at, previous),
        };
        if replace {
            stale_at_by_instance.insert(key, created_at);
        }
    }

    // The returned closure evaluated against `interaction`.
    let kind_key = || {
        format!(
            "{}\u{0}",
            js_text(&jsv::opt(interaction, "interactionKind"))
        )
    };
    let request_key = |generation: Option<&Value>| {
        format!(
            "{}{}",
            kind_key(),
            pending_request_instance_key(&jsv::opt(interaction, "requestId"), generation)
        )
    };
    let generation = jsv::member(
        interaction,
        "lifecycleGeneration",
        "interaction.lifecycleGeneration",
    )?;
    if let Some(generation) = generation.filter(|g| !jsv::is_nullish(g))
        && stale_at_by_instance.contains_key(&request_key(Some(generation)))
    {
        return Ok(true);
    }
    let Some(legacy_stale_at) = stale_at_by_instance.get(&request_key(None)) else {
        return Ok(false);
    };
    let created_at = jsv::get(interaction, "createdAt")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    Ok(js_ge(legacy_stale_at, &created_at))
}

// ---------- 5. pinnedMessages (packages/shared/src/pinnedMessages.ts) ----------

/// `existingPins.map(f)` where `existingPins = pins ?? []`; a non-array,
/// non-nullish `pins` reports the `.map` TypeError.
fn pins_map(
    pins: &Value,
    f: impl Fn(&Value) -> Result<Value, String>,
) -> Result<Vec<Value>, String> {
    match pins {
        Value::Array(items) => items.iter().map(f).collect(),
        _ if jsv::is_nullish(pins) => Ok(vec![]),
        _ => Err(
            "existingPins.map is not a function. (In 'existingPins.map(...)', \
             'existingPins.map' is undefined)"
                .to_string(),
        ),
    }
}

/// `isMessagePinned` (`pinnedMessages.ts:17`): `pins?.some(...) ?? false`.
pub(crate) fn is_message_pinned(pins: &Value, message_id: &Value) -> Result<bool, String> {
    let list = match pins {
        Value::Array(items) => items,
        _ if jsv::is_nullish(pins) => return Ok(false),
        _ => {
            return Err("pins.some is not a function. (In 'pins?.some((pin) => \
                 pin.messageId === messageId)', 'pins.some' is undefined)"
                .to_string());
        }
    };
    for pin in list {
        if jsv::strict_eq(
            jsv::member(pin, "messageId", "pin.messageId")?.unwrap_or(&Value::Null),
            message_id,
        ) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `addPinnedMessage` (`pinnedMessages.ts:24`).
pub(crate) fn add_pinned_message(pins: &Value, pin: &Value) -> Result<Value, String> {
    let message_id = jsv::member(pin, "messageId", "pin.messageId")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let existing = match pins {
        Value::Array(items) => items.clone(),
        _ if jsv::is_nullish(pins) => vec![],
        _ => {
            return Err("pins.some is not a function. (In 'pins?.some((pin) => \
                 pin.messageId === messageId)', 'pins.some' is undefined)"
                .to_string());
        }
    };
    if existing
        .iter()
        .any(|p| jsv::strict_eq(&jsv::opt(p, "messageId"), &message_id))
    {
        return Ok(Value::Array(existing));
    }
    let mut out = existing;
    out.push(pin.clone());
    Ok(Value::Array(out))
}

/// `removePinnedMessage` (`pinnedMessages.ts:34`).
pub(crate) fn remove_pinned_message(pins: &Value, message_id: &Value) -> Result<Value, String> {
    match pins {
        Value::Array(items) => Ok(Value::Array(
            items
                .iter()
                .filter(|p| !jsv::strict_eq(&jsv::opt(p, "messageId"), message_id))
                .cloned()
                .collect(),
        )),
        _ if jsv::is_nullish(pins) => Ok(Value::Array(vec![])),
        _ => Err(
            "existingPins.filter is not a function. (In 'existingPins.filter((pin) => \
             pin.messageId !== messageId)', 'existingPins.filter' is undefined)"
                .to_string(),
        ),
    }
}

/// `setPinnedMessageDone` (`pinnedMessages.ts:43`). `done` is taken as a `Value`
/// so a non-boolean argument is stored verbatim, as the TS object spread would.
pub(crate) fn set_pinned_message_done(
    pins: &Value,
    message_id: &Value,
    done: &Value,
) -> Result<Value, String> {
    Ok(Value::Array(pins_map(pins, |p| {
        if jsv::strict_eq(&jsv::opt(p, "messageId"), message_id) {
            let mut o = jsv::spread(p);
            o.insert("done".to_string(), done.clone());
            Ok(Value::Object(o))
        } else {
            Ok(p.clone())
        }
    })?))
}

fn utf16_truncate(s: &str, max_units: usize) -> String {
    let units = crate::utf16::units(s);
    if units.len() <= max_units {
        return s.to_string();
    }
    crate::utf16::slice(&units, 0, max_units)
}

/// `normalizePinLabel` (`pinnedMessages.ts:76`): `label?.trim() ?? ""` — nullish
/// normalizes to `null`; a non-string, non-nullish label throws the `.trim`
/// TypeError the optional call raises.
fn normalize_pin_label(label: &Value) -> Result<Option<String>, String> {
    if jsv::is_nullish(label) {
        return Ok(None);
    }
    let Some(s) = label.as_str() else {
        return Err(
            "label.trim is not a function. (In 'label?.trim()', 'label.trim' is undefined)"
                .to_string(),
        );
    };
    let trimmed = js_trim(s);
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(utf16_truncate(trimmed, 60)))
}

/// `setPinnedMessageLabel` (`pinnedMessages.ts:86`).
pub(crate) fn set_pinned_message_label(
    pins: &Value,
    message_id: &Value,
    label: &Value,
) -> Result<Value, String> {
    let normalized = normalize_pin_label(label)?;
    let label_value = match normalized {
        Some(s) => Value::String(s),
        None => Value::Null,
    };
    Ok(Value::Array(pins_map(pins, |p| {
        if jsv::strict_eq(&jsv::opt(p, "messageId"), message_id) {
            let mut o = jsv::spread(p);
            o.insert("label".to_string(), label_value.clone());
            Ok(Value::Object(o))
        } else {
            Ok(p.clone())
        }
    })?))
}

// ---------- 6. asyncUserInput (packages/shared/src/asyncUserInput.ts) ----------

/// `mergeAsyncUserInput` (`asyncUserInput.ts:14`): falsy previous yields
/// incoming, falsy incoming yields previous, otherwise the higher
/// `responseSequence` wins, and on a tie an already-accepted `previous.response`
/// is retained. NOTE: the proven port recorded only
/// `truthy(prev) && !truthy(next) ? prev : next`; this implements the full TS.
pub(crate) fn merge_async_user_input(previous: &Value, incoming: &Value) -> Result<Value, String> {
    if !jsv::truthy(previous) {
        return Ok(incoming.clone());
    }
    if !jsv::truthy(incoming) {
        return Ok(previous.clone());
    }
    // `x.responseSequence ?? 0`: member read on a truthy receiver never throws.
    let seq = |v: &Value| -> f64 {
        let s = jsv::opt(v, "responseSequence");
        if jsv::is_nullish(&s) { 0.0 } else { js_num(&s) }
    };
    let previous_sequence = seq(previous);
    let incoming_sequence = seq(incoming);
    if previous_sequence > incoming_sequence {
        return Ok(previous.clone());
    }
    if incoming_sequence > previous_sequence {
        return Ok(incoming.clone());
    }
    // Legacy snapshots have no sequence; retain an already accepted answer.
    if jsv::truthy(&jsv::opt(previous, "response")) {
        Ok(previous.clone())
    } else {
        Ok(incoming.clone())
    }
}

/// `clearRemovedAsyncUserInputResponses` (`asyncUserInput.ts:29`). Unlike the
/// proven port (which took a `{"$set":[...]}` marker), `retained_message_ids`
/// is a real set of message-id strings: `retainedMessageIds.has(nonString)`
/// is `false`, as `Set.prototype.has` does.
pub(crate) fn clear_removed_async_user_input_responses(
    messages: &Value,
    retained_message_ids: &HashSet<String>,
    sequence: &Value,
) -> Result<Value, String> {
    let list = match messages {
        Value::Null => {
            return Err("null is not an object (evaluating 'messages.map')".to_string());
        }
        v if crate::js::is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'messages.map')".to_string());
        }
        Value::Array(items) => items,
        _ => {
            return Err("messages.map is not a function. (In 'messages.map(...)', \
                 'messages.map' is undefined)"
                .to_string());
        }
    };
    let mut out = Vec::with_capacity(list.len());
    for message in list {
        let aui = jsv::member(message, "asyncUserInput", "message.asyncUserInput")?
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        // `input?.response` — nullish `input` yields undefined here.
        let response = jsv::opt(&aui, "response");
        if !jsv::truthy(&response) {
            out.push(message.clone());
            continue;
        }
        let retained = jsv::opt(&response, "messageId")
            .as_str()
            .is_some_and(|id| retained_message_ids.contains(id));
        if retained {
            out.push(message.clone());
            continue;
        }
        let mut next = jsv::spread(message);
        let mut aui_out = Map::new();
        aui_out.insert("questions".to_string(), jsv::opt(&aui, "questions"));
        aui_out.insert("responseSequence".to_string(), sequence.clone());
        next.insert("asyncUserInput".to_string(), Value::Object(aui_out));
        out.push(Value::Object(next));
    }
    Ok(Value::Array(out))
}

/// `answers[index]`: arrays index by position, strings index UTF-16 units,
/// objects read the numeric key, primitives yield `undefined`; nullish throws.
fn at_index(v: &Value, index: usize, expr: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!(
            "null is not an object (evaluating '{expr}[index]')"
        ));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "undefined is not an object (evaluating '{expr}[index]')"
        ));
    }
    if let Some(units) = crate::js::string_units(v) {
        return Ok(units
            .get(index)
            .map(|u| crate::js::utf16_string(&[*u]))
            .unwrap_or_else(crate::js::undefined));
    }
    Ok(match v {
        Value::Array(a) => a.get(index).cloned().unwrap_or_else(crate::js::undefined),
        Value::Object(o) => o
            .get(&index.to_string())
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    })
}

/// `formatAsyncUserInputResponse` (`asyncUserInput.ts:42`):
/// `questions.map((q, i) => `${q.title}\n${answers[i] ?? ""}`).join("\n\n")`.
/// Returns a Rust `String` like the proven port; `String()` coercions decode
/// `$utf16`/`$num` markers via [`js_text`].
pub(crate) fn format_async_user_input_response(
    questions: &Value,
    answers: &Value,
) -> Result<String, String> {
    let list = match questions {
        Value::Null => {
            return Err("null is not an object (evaluating 'questions.map')".to_string());
        }
        v if crate::js::is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'questions.map')".to_string());
        }
        Value::Array(items) => items,
        _ => {
            return Err(
                "questions.map is not a function. (In 'questions.map(...)', \
                 'questions.map' is undefined)"
                    .to_string(),
            );
        }
    };
    let mut parts = Vec::with_capacity(list.len());
    for (index, question) in list.iter().enumerate() {
        // `${question.title}` — the member read throws on a nullish element.
        let title = jsv::member(question, "title", "question.title")?
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        let answer = at_index(answers, index, "answers")?;
        let answer = if jsv::is_nullish(&answer) {
            String::new()
        } else {
            js_text(&answer)
        };
        parts.push(format!("{}\n{}", js_text(&title), answer));
    }
    Ok(parts.join("\n\n"))
}

// ---------- 7. sidechatThread (packages/shared/src/sidechatThread.ts) ----------

/// `isSidechatThread` (`sidechatThread.ts:5`): truthy `sidechatSourceThreadId`
/// or a non-nullish `sidechatContext`.
pub(crate) fn is_sidechat_thread(thread: &Value) -> Result<bool, String> {
    let source = jsv::member(
        thread,
        "sidechatSourceThreadId",
        "thread.sidechatSourceThreadId",
    )?;
    Ok(source.is_some_and(jsv::truthy)
        || jsv::get(thread, "sidechatContext").is_some_and(|v| !jsv::is_nullish(v)))
}

// ---------- 8. turnStopActivity (packages/shared/src/turnStopActivity.ts) ----------

/// `deriveTurnStopActivity` (`turnStopActivity.ts:5`). The proven port recorded
/// `id`/`sequence` as always `undefined`; the TS reads `event.eventId` and
/// `event.sequence`, so they are member-read here (still `undefined` when absent).
pub(crate) fn derive_turn_stop_activity(
    event: &Value,
    active_turn_id: &Value,
) -> Result<Value, String> {
    let payload = jsv::member(event, "payload", "event.payload")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let requested_by = jsv::member(&payload, "requestedBy", "event.payload.requestedBy")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    if !str_is(&requested_by, "user") {
        return Ok(Value::Null);
    }
    let payload_turn_id = jsv::get(&payload, "turnId")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    // `event.payload.turnId ?? activeTurnId` — nullish coalescing keeps 0/false.
    let turn_id = if jsv::is_nullish(&payload_turn_id) {
        active_turn_id.clone()
    } else {
        payload_turn_id
    };
    if matches!(turn_id, Value::Null) {
        return Ok(Value::Null);
    }
    let created_at = jsv::get(&payload, "createdAt")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    Ok(json!({
        "id": jsv::get(event, "eventId").cloned().unwrap_or_else(crate::js::undefined),
        "turnId": turn_id,
        "createdAt": created_at,
        "tone": "info",
        "kind": "turn.stop-requested",
        "summary": "Stop requested",
        "payload": { "requestedBy": "user" },
        "sequence": jsv::get(event, "sequence").cloned().unwrap_or_else(crate::js::undefined),
        "sequenceSource": "orchestration",
    }))
}

// ---------- 9. messageTextSegments (apps/web/src/messageTextSegments.ts) ----------

/// `v[k]` member read: nullish receivers throw, absent/`undefined` properties
/// map to `None` (the proven port's `prop`).
fn seg_prop<'a>(v: &'a Value, k: &str, expr: &str) -> Result<Option<&'a Value>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}.{k}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "undefined is not an object (evaluating '{expr}.{k}')"
        ));
    }
    Ok(v.as_object()
        .and_then(|o| o.get(k))
        .filter(|x| !crate::js::is_undefined(x)))
}

/// `v[k]` cloned, absent → `undefined` (the proven port's `opt_prop`).
fn seg_opt_prop(v: &Value, k: &str, expr: &str) -> Result<Value, String> {
    Ok(seg_prop(v, k, expr)?
        .cloned()
        .unwrap_or_else(crate::js::undefined))
}

/// `value.length` — arrays, strings (UTF-16 units), or an own `length` number.
fn seg_length(v: &Value) -> Option<f64> {
    crate::js::string_units(v)
        .map(|u| u.len() as f64)
        .or_else(|| match v {
            Value::Array(a) => Some(a.len() as f64),
            Value::Object(o) => o.get("length").and_then(crate::js::as_number),
            _ => None,
        })
}

/// `a === b` on primitives, `Option`-wrapped (the proven port's `js_eq`).
fn seg_js_eq(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            (x.is_null() && y.is_null())
                || (crate::js::string_units(x).is_some()
                    && crate::js::string_units(x) == crate::js::string_units(y))
                || (crate::js::as_number(x).is_some()
                    && crate::js::as_number(x) == crate::js::as_number(y))
                || matches!((x, y), (Value::Bool(u), Value::Bool(v)) if u == v)
        }
        _ => false,
    }
}

/// `v.at(-1)` — arrays and strings; other receivers raise the `.at` TypeError.
fn at_minus_one(v: &Value) -> Result<Option<Value>, String> {
    if let Some(u) = crate::js::string_units(v) {
        return Ok(u.last().map(|u| crate::js::utf16_string(&[*u])));
    }
    if let Value::Array(a) = v {
        Ok(a.last().cloned())
    } else {
        Err(
            "previous.at is not a function. (In 'previous.at(-1)', 'previous.at' \
             is undefined)"
                .into(),
        )
    }
}

/// `v.slice(0, -1)` — arrays; other iterables yield all but the last item.
fn slice_minus_one(v: &Value) -> Result<Vec<Value>, String> {
    if let Value::Array(a) = v {
        return Ok(a[..a.len().saturating_sub(1)].to_vec());
    }
    let mut it = for_of(v).map_err(|_| {
        "previous.slice is not a function. (In 'previous.slice(0, -1)', \
         'previous.slice' is undefined)"
            .to_string()
    })?;
    it.pop();
    Ok(it)
}

/// `v[k]` coerced with `String()` — absent reads as "undefined".
fn text_of(v: &Value, k: &str, expr: &str) -> Result<String, String> {
    Ok(seg_prop(v, k, expr)?
        .map(js_text)
        .unwrap_or_else(|| "undefined".into()))
}

/// Concatenated `segment.text` of every iterated segment.
fn collate(segments: &Value) -> Result<String, String> {
    let mut out = String::new();
    for segment in for_of(segments)? {
        out += &text_of(&segment, "text", "segment")?;
    }
    Ok(out)
}

/// `collated === next`; iteration (and its throws) precedes the comparison.
fn covers(segments: &Value, next: &Value) -> Result<bool, String> {
    let collated = collate(segments)?;
    Ok(crate::js::string_units(next).is_some_and(|u| collated == String::from_utf16_lossy(&u)))
}

/// `[...previous.slice(0, -1), { ...tail, ...patch }]`.
fn merge_tail(previous: &Value, tail: &Value, patch: Vec<(&str, Value)>) -> Result<Value, String> {
    let mut merged = jsv::spread(tail);
    for (k, v) in patch {
        merged.insert(k.into(), v);
    }
    let mut out = slice_minus_one(previous)?;
    out.push(Value::Object(merged));
    Ok(Value::Array(out))
}

/// `textSegmentsCoverText` (`messageTextSegments.ts:14`).
pub(crate) fn text_segments_cover_text(segments: &Value, text: &Value) -> Result<bool, String> {
    covers(segments, text)
}

/// `advanceMessageTextSegments` (`messageTextSegments.ts:34`).
pub(crate) fn advance_message_text_segments(
    previous: &Value,
    input: &Value,
) -> Result<Value, String> {
    if crate::js::is_undefined(previous) {
        return Ok(crate::js::undefined());
    }
    if previous.is_null() {
        return Err("null is not an object (evaluating 'previous.length')".into());
    }
    if seg_length(previous) == Some(0.0) {
        return Ok(crate::js::undefined());
    }
    if !seg_prop(input, "streaming", "input")?.is_some_and(jsv::truthy) {
        if seg_length(previous).is_some_and(|l| l < 2.0)
            || !covers(previous, &seg_opt_prop(input, "nextText", "input")?)?
        {
            return Ok(crate::js::undefined());
        }
        let tail = at_minus_one(previous)?.unwrap_or(Value::Null);
        return merge_tail(
            previous,
            &tail,
            vec![("endedAt", seg_opt_prop(input, "updatedAt", "input")?)],
        );
    }
    if let Some(started_at) = seg_prop(input, "segmentStartedAt", "input")? {
        let Value::Array(prev) = previous else {
            return Err(
                "previous.findIndex is not a function. (In 'previous.findIndex((segment) \
                 => segment.sequence === input.segmentSequence)', 'previous.findIndex' \
                 is undefined)"
                    .into(),
            );
        };
        let sequence = seg_prop(input, "segmentSequence", "input")?;
        let opened = Value::Object(Map::from_iter([
            (
                "sequence".into(),
                seg_opt_prop(input, "segmentSequence", "input")?,
            ),
            ("startedAt".into(), started_at.clone()),
            ("endedAt".into(), seg_opt_prop(input, "updatedAt", "input")?),
            ("text".into(), seg_opt_prop(input, "deltaText", "input")?),
        ]));
        let mut out = prev.clone();
        match prev
            .iter()
            .position(|s| seg_js_eq(seg_prop(s, "sequence", "segment").ok().flatten(), sequence))
        {
            Some(i) => out[i] = opened,
            None => out.push(opened),
        }
        return Ok(Value::Array(out));
    }
    let tail = at_minus_one(previous)?.unwrap_or(Value::Null);
    let text = format!(
        "{}{}",
        text_of(&tail, "text", "tail")?,
        text_of(input, "deltaText", "input")?
    );
    merge_tail(
        previous,
        &tail,
        vec![
            ("text", Value::String(text)),
            ("endedAt", seg_opt_prop(input, "updatedAt", "input")?),
        ],
    )
}

// ---------- 10. model slug (packages/shared/src/model.ts normalizeModelSlug) ----------

/// `MODEL_SLUG_ALIASES_BY_PROVIDER` (`packages/contracts/src/model.ts`), the same
/// dump `model-rs` carries in `model_data.json`.
const MODEL_SLUG_ALIASES_JSON: &str = include_str!("shared_model_aliases.json");

fn model_aliases() -> &'static Map<String, Value> {
    static ALIASES: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    ALIASES.get_or_init(|| {
        serde_json::from_str(MODEL_SLUG_ALIASES_JSON).unwrap_or_else(|_| Map::new())
    })
}

/// `/\[([^\]]+)\]$/u` — the trailing `[…]` context-window suffix, lowercased.
fn claude_suffix(model: &str) -> Option<String> {
    if !model.ends_with(']') {
        return None;
    }
    let open = model[..model.len() - 1].rfind('[')?;
    Some(model[open + 1..model.len() - 1].to_lowercase())
}

/// `stripClaudeContextWindowSuffix`.
fn strip_claude_context_window_suffix(model: &str) -> String {
    if !model.ends_with(']') {
        return model.to_string();
    }
    if let Some(open) = model[..model.len() - 1].rfind('[') {
        return model[..open].to_string();
    }
    model.to_string()
}

fn normalize_slug_internal(model: &str, provider: &Value) -> Result<Option<Value>, String> {
    let trimmed = js_trim(model);
    if trimmed.is_empty() {
        return Ok(None);
    }
    let provider = if crate::js::is_undefined(provider) {
        "codex"
    } else {
        provider.as_str().unwrap_or("")
    };
    let scoped: String = if provider == "claudeAgent" {
        strip_claude_context_window_suffix(trimmed)
    } else if provider == "devin"
        && trimmed == trimmed.to_lowercase()
        && trimmed.ends_with("-medium")
    {
        trimmed[..trimmed.len() - "-medium".len()].to_string()
    } else {
        trimmed.to_string()
    };
    let Some(aliases) = model_aliases().get(provider) else {
        return Err("Cannot convert undefined or null to object".into());
    };
    let alias_key = scoped.to_lowercase();
    let normalized = aliases
        .get(&alias_key)
        .and_then(Value::as_str)
        .unwrap_or(&scoped)
        .to_string();
    if provider == "claudeAgent" && claude_suffix(trimmed).as_deref() == Some("1m") {
        return Ok(Some(json!(format!("{normalized}[1m]"))));
    }
    Ok(Some(json!(normalized)))
}

/// `normalizeModelSlug` (`model.ts:634`). NOTE: the TS signature is
/// `(model: string | null | undefined, provider: ProviderKind = "codex")` —
/// two arguments, not the spec draft's single `slug`. A non-string model
/// returns `null`.
pub(crate) fn normalize_model_slug(model: &Value, provider: &Value) -> Result<Value, String> {
    match model {
        Value::String(s) => Ok(normalize_slug_internal(s, provider)?.unwrap_or(Value::Null)),
        _ => Ok(Value::Null),
    }
}

// ---------- 11. git (packages/shared/src/git.ts resolveThreadBranchRegressionGuard) ----------

const WORKTREE_BRANCH_PREFIX: &str = "synara";
const PRE_CUTOVER_WORKTREE_NAMESPACE_HASHES: [u64; 2] = [0x8559131d2c062cf0, 0xd53b4ab95395e345];

/// FNV-1a 64 over UTF-16 code units (`charCodeAt`), with `asUintN(64)` wraparound.
fn hash_worktree_namespace(value: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for unit in crate::utf16::units(value) {
        hash ^= u64::from(unit);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn is_pre_cutover_worktree_namespace(value: &str) -> bool {
    PRE_CUTOVER_WORKTREE_NAMESPACE_HASHES.contains(&hash_worktree_namespace(value))
}

/// `/^([a-z0-9][a-z0-9-]*)\/[0-9a-f]{8}$/` on the trimmed, lowercased branch —
/// returns the namespace when it matches.
fn temp_branch_namespace(branch: &str) -> Option<String> {
    let s = js_trim(branch).to_lowercase();
    let (ns, rest) = s.split_once('/')?;
    if ns.is_empty()
        || !ns.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-'))
        || !ns.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }
    if rest.len() != 8 || !rest.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
        return None;
    }
    Some(ns.to_string())
}

/// `isTemporaryWorktreeBranch` on a known string.
fn is_temporary_worktree_branch(branch: &str) -> bool {
    let Some(namespace) = temp_branch_namespace(branch) else {
        return false;
    };
    namespace == WORKTREE_BRANCH_PREFIX || is_pre_cutover_worktree_namespace(&namespace)
}

/// `resolveThreadBranchRegressionGuard` (`git.ts:152`), input
/// `{ currentBranch, nextBranch }`. Returns `input.nextBranch` verbatim when the
/// guard does not hold (the spec's `Value` return covers strings, `null`, and
/// `undefined`).
pub(crate) fn resolve_thread_branch_regression_guard(input: &Value) -> Result<Value, String> {
    let current = jsv::member(input, "currentBranch", "input.currentBranch")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let next = jsv::member(input, "nextBranch", "input.nextBranch")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);

    // `isTemporaryWorktreeBranch(v)`: `v.trim()` throws on undefined and on
    // non-strings; `currentBranch !== null` keeps `undefined` inside the guard.
    fn temp_check(v: &Value) -> Result<bool, String> {
        match v {
            Value::String(s) => Ok(is_temporary_worktree_branch(s)),
            Value::Null => Ok(false), // guarded out by `!== null` in JS
            _ if crate::js::is_undefined(v) => {
                Err("undefined is not an object (evaluating 'branch.trim')".into())
            }
            _ => Err(
                "branch.trim is not a function. (In 'branch.trim()', 'branch.trim' \
                 is undefined)"
                    .into(),
            ),
        }
    }

    // `!isTemporaryWorktreeBranch(current) && isTemporaryWorktreeBranch(next)`
    // evaluates the next-branch check only when the current is not temporary.
    let guard = !current.is_null()
        && !next.is_null()
        && match temp_check(&current)? {
            true => false,
            false => temp_check(&next)?,
        };
    if guard { Ok(current) } else { Ok(next) }
}

// ---------- 12. threadPullRequestLinks (packages/shared/src/threadPullRequestLinks.ts) ----------

fn pr_obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.into(), v);
    }
    Value::Object(m)
}

/// `legacyPullRequestLinks` (`threadPullRequestLinks.ts:129`).
fn legacy_pull_request_links(pull_request: &Value, linked_at: &Value) -> Vec<Value> {
    if jsv::truthy(pull_request) {
        vec![pr_obj(vec![
            ("pullRequest", pull_request.clone()),
            ("source", Value::from("legacy")),
            ("linkedAt", linked_at.clone()),
        ])]
    } else {
        Vec::new()
    }
}

/// `pullRequestLinksFromEventPayload` (`threadPullRequestLinks.ts:141`): the
/// event's `pullRequests` (spread into a fresh array), else a `lastKnownPr`
/// wrapped as one legacy link, else `undefined`.
pub(crate) fn pull_request_links_from_event_payload(
    payload: &Value,
    updated_at: &Value,
) -> Result<Value, String> {
    let pull_requests = jsv::member(payload, "pullRequests", "payload.pullRequests")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    // `payload.pullRequests !== undefined` → `[...payload.pullRequests]`.
    if !crate::js::is_undefined(&pull_requests) {
        return Ok(Value::Array(spread(&pull_requests)?));
    }
    let last_known = jsv::member(payload, "lastKnownPr", "payload.lastKnownPr")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    if !crate::js::is_undefined(&last_known) {
        return Ok(Value::Array(legacy_pull_request_links(
            &last_known,
            updated_at,
        )));
    }
    Ok(crate::js::undefined())
}

// ---------- 13. threadWorkspace (packages/shared/src/threadWorkspace.ts) ----------

/// `/^[a-z]:([\\/]|$)/i` on `value`, `\\` or `//` prefix.
fn is_likely_windows_root(value: &str, platform: &Value) -> bool {
    if platform.as_str() == Some("win32") {
        return true;
    }
    if jsv::truthy(platform) && platform.as_str() != Some("win32") {
        return false;
    }
    let b = value.as_bytes();
    (b.len() >= 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b.len() == 2 || b[2] == b'\\' || b[2] == b'/'))
        || value.starts_with("\\\\")
        || value.starts_with("//")
}

fn normalize_root(value: &str, platform: &Value) -> String {
    let trimmed = js_trim(value);
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd = trimmed.replace('\\', "/");
    // `/^[a-z]:\/+$/i`
    let fb = fwd.as_bytes();
    let is_drive_root = fb.len() >= 3
        && fb[0].is_ascii_alphabetic()
        && fb[1] == b':'
        && fb[2..].iter().all(|&c| c == b'/');
    let has_unc = fwd.starts_with("//");
    let prefix = if has_unc {
        "//"
    } else if fwd.starts_with('/') {
        "/"
    } else {
        ""
    };
    let mut body = String::with_capacity(fwd.len());
    let mut prev_slash = false;
    for c in fwd[prefix.len()..].chars() {
        if c == '/' {
            if !prev_slash {
                body.push(c);
            }
            prev_slash = true;
        } else {
            body.push(c);
            prev_slash = false;
        }
    }
    let body = body.trim_end_matches('/');
    let normalized = if !prefix.is_empty() {
        format!("{prefix}{body}")
    } else {
        body.to_string()
    };
    let mut final_value = if is_drive_root {
        format!("{}/", &fwd[..2])
    } else if !normalized.is_empty() {
        normalized
    } else {
        prefix.to_string()
    };

    if platform.as_str() == Some("darwin")
        && (final_value.starts_with("/private/var/") || final_value.starts_with("/private/tmp/"))
    {
        final_value = final_value["/private".len()..].to_string();
    }

    if is_likely_windows_root(trimmed, platform) {
        final_value.to_lowercase()
    } else {
        final_value
    }
}

/// `normalizeWorkspaceRootForComparison` (`threadWorkspace.ts:33`). NOTE: the
/// TS takes `(value: string, options?: { platform?: string })`; this signature
/// takes only `root`, so `options?.platform` is `undefined` — matching every
/// call that omits the option. `value.trim` throws on nullish/non-string input.
pub(crate) fn normalize_workspace_root_for_comparison(root: &Value) -> Result<Value, String> {
    match root {
        Value::String(s) => Ok(Value::String(normalize_root(s, &crate::js::undefined()))),
        Value::Null => Err("null is not an object (evaluating 'value.trim')".to_string()),
        v if crate::js::is_undefined(v) => {
            Err("undefined is not an object (evaluating 'value.trim')".to_string())
        }
        _ => Err(
            "value.trim is not a function. (In 'value.trim()', 'value.trim' is \
             undefined)"
                .to_string(),
        ),
    }
}
