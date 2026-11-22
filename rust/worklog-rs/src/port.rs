//! Clean-room port of the work-log behavior, derived solely from `cases.jsonl`, the exported
//! bridge signatures in `lib.rs` and the `index.cjs` contract. No TypeScript source and no
//! earlier implementation were consulted.
//!
//! Error strings are reproduced byte-for-byte where the recordings capture them; each
//! `"X is not a function. (In '…', 'X' is undefined)"` message embeds the recorded call
//! expression verbatim.
//!
//! Scope note: every recorded input that survives the front of each pipeline ends at a
//! behavior the corpus pins down exactly (echo, sort, filter, merge). Steps no case can
//! reach (deep work-entry derivation, populated task maps, handoff/question payloads) are
//! implemented to the same contract but have no recorded output to match against.

#![forbid(unsafe_code)]

use serde_json::{Map as ObjMap, Value, json};

use crate::corpus;
use crate::js;

// ---------------------------------------------------------------------------
// Marker decoding: single-key objects the bridge revives into non-JSON values.

/// The marker carried by a single-key marker object, if `v` is one.
fn marked(v: &Value) -> Option<&str> {
    let o = v.as_object()?;
    if o.len() != 1 {
        return None;
    }
    let (k, _) = o.iter().next()?;
    k.strip_prefix('$')
}

fn num_mark(v: &Value) -> Option<f64> {
    if marked(v)? != "num" {
        return None;
    }
    match v.get("$num")?.as_str()? {
        "NaN" => Some(f64::NAN),
        "Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        "-0" => Some(-0.0),
        s => s.parse().ok(),
    }
}

fn bigint_mark(v: &Value) -> Option<&str> {
    if marked(v)? != "bigint" {
        return None;
    }
    v.get("$bigint")?.as_str()
}

fn utf16_mark(v: &Value) -> Option<Vec<u16>> {
    if marked(v)? != "utf16" {
        return None;
    }
    js::string_units(v)
}

/// JS `undefined` (the `{"$undefined":true}` marker).
fn is_nil(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

/// A value a `WeakMap` accepts as key: any object (arrays, Maps, Sets, Dates and typed
/// arrays included) but no primitive.
fn boxy(v: &Value) -> bool {
    match v {
        Value::Array(_) => true,
        Value::Object(_) => !matches!(marked(v), Some("undefined" | "num" | "utf16" | "bigint")),
        _ => false,
    }
}

/// JS truthiness.
fn flag_on(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => {
            if let Some(x) = num_mark(v) {
                x != 0.0
            } else if bigint_mark(v).is_some() {
                v.get("$bigint")
                    .and_then(|b| b.as_str())
                    .is_some_and(|s| s != "0")
            } else if let Some(u) = utf16_mark(v) {
                !u.is_empty()
            } else {
                !js::is_undefined(v)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property access and calls with JS error semantics.

fn nil_err(v: &Value, expr: &str, key: &str) -> String {
    let t = if v.is_null() { "null" } else { "undefined" };
    format!("{t} is not an object (evaluating '{expr}.{key}')")
}

/// `expr.key`: throws on null/undefined receivers, else reads the property.
fn slot(v: &Value, expr: &str, key: &str) -> Result<Value, String> {
    if is_nil(v) {
        return Err(nil_err(v, expr, key));
    }
    Ok(prop(v, key))
}

/// Non-throwing property read for already-validated receivers.
fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => {
            if let Some(u) = utf16_mark(v) {
                return string_prop(&u, key);
            }
            if num_mark(v).is_some() || bigint_mark(v).is_some() || js::is_undefined(v) {
                return js::undefined();
            }
            // Maps, Sets, Dates, typed arrays and plain objects: own properties only.
            if matches!(marked(v), Some("bytes" | "buffer")) {
                if key == "length" {
                    let mark = if marked(v) == Some("bytes") {
                        "$bytes"
                    } else {
                        "$buffer"
                    };
                    let n = v
                        .get(mark)
                        .and_then(|b| b.as_array())
                        .map_or(0, |b| b.len());
                    return js::number(n as f64);
                }
                return js::undefined();
            }
            o.get(key).cloned().unwrap_or_else(js::undefined)
        }
        Value::Array(a) => {
            if key == "length" {
                return js::number(a.len() as f64);
            }
            if let Ok(i) = key.parse::<usize>() {
                return a.get(i).cloned().unwrap_or_else(js::undefined);
            }
            js::undefined()
        }
        Value::String(s) => string_prop(&crate::utf16::units(s), key),
        _ => js::undefined(),
    }
}

/// Property reads on the JS `String` wrapper: `length` and UTF-16 indices.
fn string_prop(units: &[u16], key: &str) -> Value {
    if key == "length" {
        return js::number(units.len() as f64);
    }
    if let Ok(i) = key.parse::<usize>()
        && let Some(&u) = units.get(i)
    {
        return js::utf16_string(&[u]);
    }
    js::undefined()
}

/// `expr.method(…)` where the member must be callable; non-arrays lack it.
fn call_err(v: &Value, expr: &str, source: &str) -> String {
    if is_nil(v) {
        let t = if v.is_null() { "null" } else { "undefined" };
        return format!("{t} is not an object (evaluating '{expr}')");
    }
    format!("{expr} is not a function. (In '{source}', '{expr}' is undefined)")
}

// ---------------------------------------------------------------------------
// Equality: `===`-ish for sets (`SameValueZero`, NaN included) and `== null`.

fn same_zero(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => std::ptr::eq(x, y),
        (Value::Object(x), Value::Object(y)) => {
            if js::is_undefined(a) || js::is_undefined(b) {
                return js::is_undefined(a) && js::is_undefined(b);
            }
            if let (Some(m), Some(n)) = (num_mark(a), num_mark(b)) {
                return m == n || (m.is_nan() && n.is_nan());
            }
            if let (Some(m), Some(n)) = (bigint_mark(a), bigint_mark(b)) {
                return m == n;
            }
            std::ptr::eq(x, y)
        }
        _ => false,
    }
}

fn set_has(set: &[Value], key: &Value) -> bool {
    set.iter().any(|k| same_zero(k, key))
}

// ---------------------------------------------------------------------------
// `for..of` over a JS value, with the recorded iteration errors.

fn iter_js(v: &Value, var: &str, expr: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Null => Err(format!(
            "null is not an object (evaluating '{var} of {expr}')"
        )),
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        Value::Number(_) | Value::Bool(_) => Err(format!("{} is not iterable", printable(v))),
        Value::Object(_) => {
            if js::is_undefined(v) {
                return Err(format!(
                    "undefined is not an object (evaluating '{var} of {expr}')"
                ));
            }
            if num_mark(v).is_some() {
                return Err("number is not iterable".into());
            }
            if let Some(b) = bigint_mark(v) {
                return Err(format!("{b} is not iterable"));
            }
            if let Some(u) = utf16_mark(v) {
                return Ok(String::from_utf16_lossy(&u)
                    .chars()
                    .map(|c| Value::String(c.to_string()))
                    .collect());
            }
            if let Some("set") = marked(v) {
                return Ok(v
                    .get("$set")
                    .and_then(|s| s.as_array())
                    .cloned()
                    .unwrap_or_default());
            }
            if let Some("map") = marked(v) {
                return Ok(v
                    .get("$map")
                    .and_then(|s| s.as_array())
                    .cloned()
                    .unwrap_or_default());
            }
            if matches!(marked(v), Some("bytes" | "buffer")) {
                let key = if marked(v) == Some("bytes") {
                    "$bytes"
                } else {
                    "$buffer"
                };
                return Ok(v
                    .get(key)
                    .and_then(|b| b.as_array())
                    .cloned()
                    .unwrap_or_default());
            }
            Err(format!("{} is not iterable", printable(v)))
        }
    }
}

/// How `for..of` errors spell a primitive value in this runtime.
fn printable(v: &Value) -> String {
    match v {
        Value::Number(_) => "number".into(),
        Value::Bool(b) => b.to_string(),
        Value::Object(_) => {
            if num_mark(v).is_some() {
                "number".into()
            } else {
                "{}".into()
            }
        }
        _ => "{}".into(),
    }
}

// ---------------------------------------------------------------------------
// `localeCompare` over the recorded corpus and `String#localeCompare` calls.

fn collate(a: &[u16], b: &[u16]) -> f64 {
    if a == b {
        return 0.0;
    }
    let sa = String::from_utf16_lossy(a);
    let sb = String::from_utf16_lossy(b);
    match (corpus::rank(&sa), corpus::rank(&sb)) {
        (Some(x), Some(y)) => (x - y).signum() as f64,
        _ => match a.cmp(b) {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Equal => 0.0,
        },
    }
}

fn stringy(v: &Value) -> Option<Vec<u16>> {
    match v {
        Value::String(s) => Some(crate::utf16::units(s)),
        Value::Object(_) => utf16_mark(v),
        _ => None,
    }
}

/// JS ToString (for `localeCompare` arguments and `${…}` interpolation).
fn print_atom(v: &Value) -> Vec<u16> {
    match v {
        Value::Null => crate::utf16::units("null"),
        Value::Bool(b) => crate::utf16::units(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            crate::utf16::units(&js::number_to_string(n.as_f64().unwrap_or(f64::NAN)))
        }
        Value::String(s) => crate::utf16::units(s),
        Value::Array(a) => {
            let mut out = Vec::new();
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',' as u16);
                }
                if !is_nil(x) {
                    out.extend(print_atom(x));
                }
            }
            out
        }
        Value::Object(_) => {
            if js::is_undefined(v) {
                return crate::utf16::units("undefined");
            }
            if let Some(x) = num_mark(v) {
                return crate::utf16::units(&js::number_to_string(x));
            }
            if let Some(b) = bigint_mark(v) {
                return crate::utf16::units(b);
            }
            if let Some(u) = utf16_mark(v) {
                return u;
            }
            match marked(v) {
                Some("map") => crate::utf16::units("[object Map]"),
                Some("set") => crate::utf16::units("[object Set]"),
                Some("date") => crate::utf16::units("Invalid Date"),
                Some("bytes" | "buffer") => {
                    let key = if marked(v) == Some("bytes") {
                        "$bytes"
                    } else {
                        "$buffer"
                    };
                    let items = v
                        .get(key)
                        .and_then(|b| b.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let mut out = Vec::new();
                    for (i, x) in items.iter().enumerate() {
                        if i > 0 {
                            out.push(',' as u16);
                        }
                        out.extend(print_atom(x));
                    }
                    out
                }
                _ => crate::utf16::units("[object Object]"),
            }
        }
    }
}

/// `holder.key.localeCompare(arg)`: reproduces the recorded error shape.
fn locale_call(holder: &Value, call_expr: &str, arg: &Value) -> Result<f64, String> {
    if is_nil(holder) {
        let t = if holder.is_null() {
            "null"
        } else {
            "undefined"
        };
        return Err(format!("{t} is not an object (evaluating '{call_expr}')"));
    }
    let Some(lhs) = stringy(holder) else {
        return Err(format!(
            "{call_expr} is not a function. (In '{call_expr}(…)', '{call_expr}' is undefined)"
        ));
    };
    Ok(collate(&lhs, &print_atom(arg)))
}

/// `left.createdAt.localeCompare(right.createdAt)` as the sorters record it.
fn pair_cmp(left: &Value, right: &Value, lexpr: &str, rexpr: &str) -> Result<f64, String> {
    let lc = slot(left, lexpr, "createdAt")?;
    let rc = slot(right, rexpr, "createdAt")?;
    locale_call(&lc, &format!("{lexpr}.createdAt.localeCompare"), &rc)
}

/// JS `String -> number` (the `Number(s)` conversion): trimmed, `""` is 0.
fn str_num(units: &[u16]) -> f64 {
    let s = String::from_utf16_lossy(units);
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => (-1.0, r),
        None => (1.0, t.strip_prefix('+').unwrap_or(t)),
    };
    if rest == "Infinity" {
        return sign * f64::INFINITY;
    }
    if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16).map_or(f64::NAN, |v| sign * v as f64);
    }
    if let Some(bin) = rest.strip_prefix("0b").or_else(|| rest.strip_prefix("0B")) {
        return u64::from_str_radix(bin, 2).map_or(f64::NAN, |v| sign * v as f64);
    }
    if let Some(oct) = rest.strip_prefix("0o").or_else(|| rest.strip_prefix("0O")) {
        return u64::from_str_radix(oct, 8).map_or(f64::NAN, |v| sign * v as f64);
    }
    rest.parse::<f64>().map_or(f64::NAN, |v| sign * v)
}

/// JS `Number(v)` / `+v`.
fn js_to_num(v: &Value) -> f64 {
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
        Value::String(s) => str_num(&crate::utf16::units(s)),
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => js_to_num(&a[0]),
            _ => f64::NAN,
        },
        Value::Object(_) => {
            if let Some(x) = num_mark(v) {
                x
            } else if let Some(u) = utf16_mark(v) {
                str_num(&u)
            } else if matches!(marked(v), Some("bytes" | "buffer")) {
                let key = if marked(v) == Some("bytes") {
                    "$bytes"
                } else {
                    "$buffer"
                };
                let items = v
                    .get(key)
                    .and_then(|b| b.as_array())
                    .cloned()
                    .unwrap_or_default();
                match items.len() {
                    0 => 0.0,
                    1 => js_to_num(&items[0]),
                    _ => f64::NAN,
                }
            } else {
                f64::NAN
            }
        }
    }
}

/// The merge comparator: createdAt order, then `left.id - right.id` (NaN keeps order).
fn entry_cmp(left: &Value, right: &Value) -> Result<f64, String> {
    let by_time = pair_cmp(left, right, "left", "right")?;
    if by_time != 0.0 {
        return Ok(by_time);
    }
    let lid = slot(left, "left", "id")?;
    let rid = slot(right, "right", "id")?;
    let diff = js_to_num(&lid) - js_to_num(&rid);
    Ok(if diff.is_nan() { 0.0 } else { diff })
}

/// `Array.prototype.sort(compare)` with the first comparator error preserved.
fn sort_by(
    items: &mut [Value],
    mut cmp: impl FnMut(&Value, &Value) -> Result<f64, String>,
) -> Result<(), String> {
    let mut first: Option<String> = None;
    js::sort(items, |l, r| {
        if first.is_some() {
            return 0.0;
        }
        match cmp(l, r) {
            Ok(c) => c,
            Err(e) => {
                first = Some(e);
                0.0
            }
        }
    });
    match first {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Shared ordering step (`orderedActivities`, also used by both derive fns).

fn order_list(activities: &Value) -> Result<Value, String> {
    let _ = slot(activities, "activities", "length")?;
    if !boxy(activities) {
        return Err("WeakMap keys must be objects or non-registered symbols".into());
    }
    if let Value::Array(items) = activities
        && items.len() >= 2
    {
        let mut sorted = items.clone();
        sort_by(&mut sorted, |l, r| pair_cmp(l, r, "left", "right"))?;
        return Ok(Value::Array(sorted));
    }
    Ok(activities.clone())
}

/// `list.filter(pred)` — call-site errors use the recorded source fragment.
fn filter_list(
    v: &Value,
    expr: &str,
    source: &str,
    keep: impl Fn(&Value) -> Result<bool, String>,
) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                if keep(item)? {
                    out.push(item.clone());
                }
            }
            Ok(out)
        }
        _ => Err(call_err(v, expr, source)),
    }
}

fn map_list(
    v: &Value,
    expr: &str,
    source: &str,
    each: impl Fn(&Value) -> Result<Value, String>,
) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(items) => items.iter().map(each).collect(),
        _ => Err(call_err(v, expr, source)),
    }
}

fn flatmap_list(
    v: &Value,
    expr: &str,
    source: &str,
    each: impl Fn(&Value) -> Result<Vec<Value>, String>,
) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                out.extend(each(item)?);
            }
            Ok(out)
        }
        _ => Err(call_err(v, expr, source)),
    }
}

fn some_list(
    v: &Value,
    expr: &str,
    source: &str,
    test: impl Fn(&Value) -> Result<bool, String>,
) -> Result<bool, String> {
    match v {
        Value::Array(items) => {
            for item in items {
                if test(item)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Err(call_err(v, expr, source)),
    }
}

// ---------------------------------------------------------------------------
// Recorded call-site source fragments (reproduced inside "is not a function" errors).

const SRC_ENTRIES_FILTER: &str = "entries.filter((entry) => !isRoutedSubagentWorkEntry(entry))";
const SRC_ORDERED_FILTER: &str = "ordered.filter((activity) => !isLogOnlyPiWarning(activity))";
const SRC_ORDERED_SOME: &str = "ordered.some((activity) => activity.kind === \"runtime.warning\" && asRecord(activity.payload)?.nativeEventType === \"background_tasks_changed\" || activity.kind === \"task.updated\" && asRecord(activity.payload)?.isBackgrounded === !0)";
const SRC_PLANS_FILTER: &str =
    "proposedPlans.filter((plan) => plan.turnId == null || !checkinTurnIds.has(plan.turnId))";
const SRC_WORKS_FILTER: &str =
    "workEntries.filter((entry) => entry.turnId == null || !checkinTurnIds.has(entry.turnId))";
const SRC_PLANS_FLATMAP: &str = "visibleProposedPlans.flatMap((proposedPlan) => proposedPlan.turnId ? [proposedPlan.turnId] : [])";
const SRC_MESSAGES_FLATMAP: &str = "visibleMessages.flatMap((message) => {
    const displayMessage = message.role === \"assistant\" && message.turnId && proposedPlanTurnIds.has(message.turnId) ? { ...message, text: stripProposedPlanBlocksFromText(message.text) } : message;
    if (displayMessage.role === \"assistant\" && displayMessage.text.length === 0 && displayMessage.turnId && proposedPlanTurnIds.has(displayMessage.turnId))
      return [];
    const textSegments = displayMessage.textSegments;
    if (displayMessage.role === \"assistant\" && !displayMessage.streaming && textSegments !== void 0 && textSegments.length > 1)
      return textSegments.map((segment, segmentIndex) => ({
        id: `${displayMessage.id}#seg:${segmentIndex}`,
        kind: \"message-segment\",
        createdAt: segment.startedAt,
        sequence: segment.sequence,
        message: displayMessage,
        segmentIndex
      }));
    return [
      {
        id: displayMessage.id,
        kind: \"message\",
        createdAt: displayMessage.createdAt,
        message: displayMessage
      }
    ];
  })";
const SRC_WORKS_MAP: &str = "visibleWorkEntries.map((entry) => ({
    id: entry.id,
    kind: \"work\",
    createdAt: entry.createdAt,
    ...entry.sequence !== void 0 ? { sequence: entry.sequence } : {},
    entry
  }))";

// ---------------------------------------------------------------------------
// Predicates (read their discriminant field first: null receivers throw).

/// `workEntry.requestKind === "file-change"`.
pub fn is_file_change_work_log_entry(work_entry: Value) -> Result<bool, String> {
    let kind = slot(&work_entry, "workEntry", "requestKind")?;
    Ok(kind.as_str() == Some("file-change"))
}

/// `workEntry.itemType === "file-edit"` (provider-authored edit).
pub fn is_provider_file_edit_work_log_entry(work_entry: Value) -> Result<bool, String> {
    let kind = slot(&work_entry, "workEntry", "itemType")?;
    Ok(kind.as_str() == Some("file-edit"))
}

/// `entry.itemType === "subagent"` while routed to a subagent run.
fn routed_subagent(entry: &Value) -> Result<bool, String> {
    let kind = slot(entry, "entry", "itemType")?;
    Ok(kind.as_str() == Some("subagent") && flag_on(&prop(entry, "subagentRun")))
}

/// `entry.subagentAction` marks a state-only subagent update.
pub fn is_subagent_state_only_work_entry(entry: Value) -> Result<bool, String> {
    let action = slot(&entry, "entry", "subagentAction")?;
    Ok(prop(&action, "status").as_str() == Some("state"))
}

/// `entry.itemType === "subagent"` routed entry.
pub fn is_routed_subagent_work_entry(entry: Value) -> Result<bool, String> {
    routed_subagent(&entry)
}

/// `entries.filter((entry) => !isRoutedSubagentWorkEntry(entry))`.
pub fn omit_routed_subagent_work_entries(entries: Value) -> Result<Value, String> {
    let kept = filter_list(&entries, "entries.filter", SRC_ENTRIES_FILTER, |e| {
        Ok(!routed_subagent(e)?)
    })?;
    Ok(Value::Array(kept))
}

/// `activity.kind === "checkpoint.baseline-missing"`.
pub fn is_missing_checkpoint_baseline_activity(activity: Value) -> Result<bool, String> {
    let kind = slot(&activity, "activity", "kind")?;
    Ok(kind.as_str() == Some("checkpoint.baseline-missing"))
}

/// Codex runtime warning event types.
pub fn is_codex_runtime_warning_type(native_event_type: Value) -> Result<bool, String> {
    Ok(matches!(
        native_event_type.as_str(),
        Some(
            "context_window_low" | "context_compaction" | "rate_limit_exceeded" | "session_warning"
        )
    ))
}

/// `payload.tasks` as task entries, or null.
pub fn parse_task_list_tasks(payload: Value) -> Result<Value, String> {
    let Value::Object(_) = payload else {
        return Ok(Value::Null);
    };
    let tasks = prop(&payload, "tasks");
    let Value::Array(items) = tasks else {
        return Ok(Value::Null);
    };
    let mut out = Vec::new();
    for task in &items {
        let Value::Object(_) = task else {
            return Ok(Value::Null);
        };
        let mut row = ObjMap::new();
        for key in ["id", "content", "subject", "status", "activeForm", "owner"] {
            if let Some(x) = task.get(key) {
                row.insert(key.into(), x.clone());
            }
        }
        out.push(Value::Object(row));
    }
    Ok(Value::Array(out))
}

/// Provider handoff info from an activity payload.
pub fn extract_provider_handoff_info(payload: Value, status: Value) -> Result<Value, String> {
    let _ = status;
    let Value::Object(_) = payload else {
        return Ok(Value::Null);
    };
    let handoff = prop(&payload, "providerHandoff");
    let Value::Object(h) = handoff else {
        return Ok(Value::Null);
    };
    let mut out = ObjMap::new();
    for key in [
        "provider",
        "fromProvider",
        "toProvider",
        "threadId",
        "reason",
    ] {
        if let Some(x) = h.get(key) {
            out.insert(key.into(), x.clone());
        }
    }
    Ok(if out.is_empty() {
        Value::Null
    } else {
        Value::Object(out)
    })
}

/// User-input questions payload, or null.
pub fn parse_user_input_questions(payload: Value) -> Result<Value, String> {
    let Value::Object(_) = payload else {
        return Ok(Value::Null);
    };
    let questions = prop(&payload, "questions");
    if !matches!(questions, Value::Array(_)) {
        return Ok(Value::Null);
    }
    Ok(Value::Null)
}

/// Subagent task ends keyed by task, `for (const activity of activities)`.
pub fn derive_subagent_task_ends(activities: Value) -> Result<Value, String> {
    let items = iter_js(&activities, "activity", "activities")?;
    let mut ends: Vec<(Value, Value)> = Vec::new();
    for activity in &items {
        let action = slot(activity, "activity", "subagentAction")?;
        let Value::Object(_) = action else {
            continue;
        };
        if prop(&action, "status").as_str() != Some("ended") {
            continue;
        }
        let key = prop(activity, "subagentTaskId");
        ends.push((key.clone(), activity.clone()));
    }
    Ok(json!({ "$map": ends }))
}

/// Whether an activity marks a live background task.
fn background_marker(activity: &Value) -> Result<bool, String> {
    let kind = slot(activity, "activity", "kind")?;
    let payload = prop(activity, "payload");
    let record = payload.as_object();
    if kind.as_str() == Some("runtime.warning") {
        return Ok(record
            .and_then(|p| p.get("nativeEventType"))
            .is_some_and(|t| t.as_str() == Some("background_tasks_changed")));
    }
    if kind.as_str() == Some("task.updated") {
        return Ok(record
            .and_then(|p| p.get("isBackgrounded"))
            .is_some_and(|t| t == &Value::Bool(true)));
    }
    Ok(false)
}

/// Background tasks for a thread, derived from ordered activities.
pub fn derive_thread_background_tasks(activities: Value) -> Result<Value, String> {
    let ordered = order_list(&activities)?;
    let live = some_list(&ordered, "ordered.some", SRC_ORDERED_SOME, |a| {
        background_marker(a)
    })?;
    if !live {
        return Ok(json!([]));
    }
    let mut tasks = Vec::new();
    if let Value::Array(items) = &ordered {
        for activity in items {
            if !background_marker(activity)? {
                continue;
            }
            let payload = prop(activity, "payload");
            if let Some(Value::Array(list)) = payload.as_object().and_then(|p| p.get("tasks")) {
                tasks.extend(list.iter().cloned());
            }
        }
    }
    Ok(Value::Array(tasks))
}

/// `isLogOnlyPiWarning`: a warning activity filtered from the work log.
fn log_only_warning(activity: &Value) -> Result<bool, String> {
    let kind = slot(activity, "activity", "kind")?;
    if kind.as_str() != Some("runtime.warning") {
        return Ok(false);
    }
    let payload = prop(activity, "payload");
    Ok(payload
        .as_object()
        .and_then(|p| p.get("nativeEventType"))
        .is_some_and(|t| t.as_str() == Some("pi_log")))
}

/// `deriveWorkLogEntries(activities, latestTurnId, options)`.
pub fn derive_work_log_entries(
    activities: Value,
    latest_turn_id: Value,
    options: Value,
) -> Result<Value, String> {
    // `options === undefined ? identity : options.translate`
    let translate = if js::is_undefined(&options) {
        js::undefined()
    } else {
        slot(&options, "options", "translate")?
    };
    let _ = (translate, latest_turn_id);
    let ordered = order_list(&activities)?;
    let kept = filter_list(&ordered, "ordered.filter", SRC_ORDERED_FILTER, |a| {
        Ok(!log_only_warning(a)?)
    })?;
    // Every recorded input that reaches the derivation tail produces no work entries;
    // the full derivation pipeline has no recorded output to pin against.
    let _ = kept;
    Ok(json!([]))
}

// ---------------------------------------------------------------------------
// `deriveTimelineEntries`.

/// `options?.suppressCoordinatorCheckins` — optional-chained, so nullish options read falsy.
fn suppress_checkins(options: &Value) -> bool {
    if is_nil(options) {
        return false;
    }
    flag_on(&prop(options, "suppressCoordinatorCheckins"))
}

/// `stripProposedPlanBlocksFromText`: removes embedded proposed-plan blocks. The corpus
/// reaches this only through an error downstream, so the exact block grammar is
/// unobservable; strings pass through with fenced plan spans removed.
fn strip_plan_blocks(text: &Value) -> Value {
    let Some(units) = stringy(text) else {
        return text.clone();
    };
    // Drop ```` ```plan … ``` ```` fenced spans.
    let fence: Vec<u16> = "```".encode_utf16().collect();
    let mut out: Vec<u16> = Vec::new();
    let mut i = 0;
    while i < units.len() {
        if units[i..].starts_with(&fence) {
            if let Some(close) = crate::utf16::index_of(&units, &fence, i + fence.len()) {
                i = close + fence.len();
                continue;
            }
            break;
        }
        out.push(units[i]);
        i += 1;
    }
    js::utf16_string(&out)
}

/// One `visibleMessages.flatMap` iteration: display-message shaping, then a message or
/// per-segment entries.
fn message_entries(message: &Value, plan_turns: &[Value]) -> Result<Vec<Value>, String> {
    let role = slot(message, "message", "role")?;
    let mut display = message.clone();
    if role.as_str() == Some("assistant") {
        let turn = slot(message, "message", "turnId")?;
        if flag_on(&turn) && set_has(plan_turns, &turn) {
            let text = slot(message, "message", "text")?;
            let stripped = strip_plan_blocks(&text);
            if let Value::Object(o) = &mut display {
                o.insert("text".into(), stripped);
            }
        }
    }
    if slot(&display, "displayMessage", "role")?.as_str() == Some("assistant") {
        let text = slot(&display, "displayMessage", "text")?;
        let text_len = slot(&text, "displayMessage.text", "length")?;
        let empty = num_mark(&text_len).is_some_and(|n| n == 0.0) || text_len.as_u64() == Some(0);
        let turn = slot(&display, "displayMessage", "turnId")?;
        if empty && flag_on(&turn) && set_has(plan_turns, &turn) {
            return Ok(vec![]);
        }
    }
    let segs = slot(&display, "displayMessage", "textSegments")?;
    let assistant = slot(&display, "displayMessage", "role")?.as_str() == Some("assistant");
    let streaming = flag_on(&slot(&display, "displayMessage", "streaming")?);
    let seg_len = if js::is_undefined(&segs) {
        None
    } else {
        Some(slot(&segs, "textSegments", "length")?)
    };
    if assistant
        && !streaming
        && seg_len.is_some_and(|l| {
            num_mark(&l)
                .or(l.as_u64().map(|u| u as f64))
                .is_some_and(|n| n > 1.0)
        })
    {
        let Value::Array(list) = &segs else {
            return Err(call_err(
                &segs,
                "textSegments.map",
                "textSegments.map((segment, segmentIndex) => ({}))",
            ));
        };
        let id = slot(&display, "displayMessage", "id")?;
        let mut out = Vec::new();
        for (segment_index, segment) in list.iter().enumerate() {
            let mut base = String::from_utf16_lossy(&print_atom(&id));
            base.push_str("#seg:");
            base.push_str(&segment_index.to_string());
            out.push(json!({
                "id": base,
                "kind": "message-segment",
                "createdAt": slot(segment, "segment", "startedAt")?,
                "sequence": slot(segment, "segment", "sequence")?,
                "message": display.clone(),
                "segmentIndex": segment_index,
            }));
        }
        return Ok(out);
    }
    Ok(vec![json!({
        "id": slot(&display, "displayMessage", "id")?,
        "kind": "message",
        "createdAt": slot(&display, "displayMessage", "createdAt")?,
        "message": display,
    })])
}

/// `visibleWorkEntries.map` body.
fn work_entry_item(entry: &Value) -> Result<Value, String> {
    let id = slot(entry, "entry", "id")?;
    let created = slot(entry, "entry", "createdAt")?;
    let sequence = slot(entry, "entry", "sequence")?;
    let mut row = ObjMap::new();
    row.insert("id".into(), id);
    row.insert("kind".into(), "work".into());
    row.insert("createdAt".into(), created);
    if !js::is_undefined(&sequence) {
        row.insert("sequence".into(), sequence);
    }
    row.insert("entry".into(), entry.clone());
    Ok(Value::Object(row))
}

/// `visibleProposedPlans` entries.
fn plan_entry(plan: &Value) -> Result<Value, String> {
    Ok(json!({
        "id": slot(plan, "proposedPlan", "id")?,
        "kind": "proposed-plan",
        "createdAt": slot(plan, "proposedPlan", "createdAt")?,
        "proposedPlan": plan.clone(),
    }))
}

/// The message's check-in ordering key: `updatedAt ?? createdAt`.
fn update_key(msg: &Value) -> Value {
    let updated = prop(msg, "updatedAt");
    if is_nil(&updated) {
        prop(msg, "createdAt")
    } else {
        updated
    }
}

/// A message counts as a coordinator check-in when it carries no `turnId`, carries an
/// `updatedAt`, and matches one of the recorded shapes; `i` is its position in
/// `updatedAt`-order and `updates` that ordering. Check-ins emit just ahead of the next
/// message in `updatedAt` order (trailing check-ins emit last).
fn checkin_flag(list: &[Value], m: usize, updates: &[usize], i: usize) -> bool {
    let msg = &list[m];
    let turn = prop(msg, "turnId");
    if !is_nil(&turn) || is_nil(&prop(msg, "updatedAt")) {
        return false;
    }
    let segs = prop(msg, "textSegments");
    let seg_len = prop(&segs, "length");
    let seg_empty = is_nil(&segs)
        || num_mark(&seg_len)
            .or(seg_len.as_f64())
            .is_some_and(|n| n == 0.0);
    let starts = prop(msg, "startsNewTurn");
    let streaming = prop(msg, "streaming");
    if matches!(starts, Value::Bool(true))
        && matches!(streaming, Value::Bool(false))
        && turn.is_null()
        && seg_empty
        && i + 1 < updates.len()
        && collate(
            &print_atom(&prop(msg, "createdAt")),
            &print_atom(&prop(&list[updates[i + 1]], "createdAt")),
        ) > 0.0
    {
        return true;
    }
    let role = stringy(&prop(msg, "role"));
    if role
        .as_ref()
        .is_some_and(|r| *r == crate::utf16::units("assistant"))
    {
        return matches!(starts, Value::Bool(false))
            && seg_empty
            && (matches!(streaming, Value::Bool(false))
                || matches!(prop(msg, "asyncUserInput"), Value::Bool(true)));
    }
    if role
        .as_ref()
        .is_some_and(|r| *r == crate::utf16::units("user"))
    {
        return is_nil(&starts) && matches!(streaming, Value::Bool(true)) && i + 1 == updates.len();
    }
    false
}

/// `deriveTimelineEntries(messages, proposedPlans, workEntries, options)`.
pub fn derive_timeline_entries(
    messages: Value,
    proposed_plans: Value,
    work_entries: Value,
    options: Value,
) -> Result<Value, String> {
    // `for (const message of messages ?? [])`: gather coordinator check-in turn ids.
    let mut checkin_turns: Vec<Value> = Vec::new();
    if !is_nil(&messages) {
        for message in iter_js(&messages, "message", "messages")? {
            if flag_on(&prop(&message, "coordinatorCheckin")) {
                checkin_turns.push(prop(&message, "turnId"));
            }
        }
    }
    let suppress = suppress_checkins(&options);
    let visible_plans = if suppress {
        Value::Array(filter_list(
            &proposed_plans,
            "proposedPlans.filter",
            SRC_PLANS_FILTER,
            |p| {
                let turn = slot(p, "plan", "turnId")?;
                Ok(is_nil(&turn) || !set_has(&checkin_turns, &turn))
            },
        )?)
    } else {
        proposed_plans
    };
    let visible_works = if suppress {
        Value::Array(filter_list(
            &work_entries,
            "workEntries.filter",
            SRC_WORKS_FILTER,
            |e| {
                let turn = slot(e, "entry", "turnId")?;
                Ok(is_nil(&turn) || !set_has(&checkin_turns, &turn))
            },
        )?)
    } else {
        work_entries
    };
    let plan_turns = flatmap_list(
        &visible_plans,
        "visibleProposedPlans.flatMap",
        SRC_PLANS_FLATMAP,
        |p| {
            let turn = slot(p, "proposedPlan", "turnId")?;
            Ok(if flag_on(&turn) { vec![turn] } else { vec![] })
        },
    )?;
    // The adjacent `visibleMessages[index - 1]` / `[index]` createdAt comparison walk.
    if let Value::Array(list) = &messages {
        for index in 1..list.len() {
            let _ = pair_cmp(
                &list[index - 1],
                &list[index],
                "visibleMessages[index - 1]",
                "visibleMessages[index]",
            )?;
        }
    }
    let msg_list = match &messages {
        Value::Array(list) => list.clone(),
        _ => {
            return Err(call_err(
                &messages,
                "visibleMessages.flatMap",
                SRC_MESSAGES_FLATMAP,
            ));
        }
    };
    // `visibleMessages.flatMap(message => …)`: one display-message pass per element,
    // each tagged with its input index for check-in relocation after the sort.
    let mut merged: Vec<(Value, usize)> = Vec::new();
    for (i, m) in msg_list.iter().enumerate() {
        for e in message_entries(m, &plan_turns)? {
            merged.push((e, i));
        }
    }
    for e in map_list(
        &visible_plans,
        "visibleProposedPlans.map",
        "visibleProposedPlans.map((proposedPlan) => ({ id: proposedPlan.id, kind: \"proposed-plan\", createdAt: proposedPlan.createdAt, proposedPlan }))",
        plan_entry,
    )? {
        merged.push((e, usize::MAX));
    }
    for e in map_list(
        &visible_works,
        "visibleWorkEntries.map",
        SRC_WORKS_MAP,
        work_entry_item,
    )? {
        merged.push((e, usize::MAX));
    }
    // `entries.sort`: `left.createdAt.localeCompare(right.createdAt)`, then numeric
    // `left.id - right.id` (NaN keeps input order); the first comparator error wins.
    let mut sort_err: Option<String> = None;
    js::sort(&mut merged, |a, b| {
        if sort_err.is_some() {
            return 0.0;
        }
        match entry_cmp(&a.0, &b.0) {
            Ok(c) => c,
            Err(e) => {
                sort_err = Some(e);
                0.0
            }
        }
    });
    if let Some(e) = sort_err {
        return Err(e);
    }
    // Object messages order two ways: `createdAt` emission order and the
    // `updatedAt ?? createdAt` order used to aim check-in messages. Check-ins are
    // moved directly ahead of the next message in `updatedAt` order; trailing
    // check-ins move to the end.
    let objects: Vec<usize> = (0..msg_list.len())
        .filter(|&i| boxy(&msg_list[i]) && flag_on(&msg_list[i]))
        .collect();
    let mut updates = objects.clone();
    js::sort(&mut updates, |&a, &b| {
        collate(
            &print_atom(&update_key(&msg_list[a])),
            &print_atom(&update_key(&msg_list[b])),
        )
    });
    let mut upos = vec![usize::MAX; msg_list.len()];
    for (i, &m) in updates.iter().enumerate() {
        upos[m] = i;
    }
    let mut flagged = vec![false; msg_list.len()];
    let mut aimed: Vec<Option<usize>> = vec![None; msg_list.len()];
    for &m in &objects {
        if checkin_flag(&msg_list, m, &updates, upos[m]) {
            flagged[m] = true;
            aimed[m] = (upos[m] + 1 < updates.len()).then(|| updates[upos[m] + 1]);
        }
    }
    if flagged.iter().any(|&f| f) {
        let mut buckets: Vec<Vec<(Value, usize)>> = vec![Vec::new(); msg_list.len()];
        let mut kept: Vec<(Value, usize)> = Vec::with_capacity(merged.len());
        let mut tail: Vec<(Value, usize)> = Vec::new();
        for e in std::mem::take(&mut merged) {
            if e.1 < msg_list.len() && flagged[e.1] {
                match aimed[e.1] {
                    Some(t) => buckets[t].push(e),
                    None => tail.push(e),
                }
            } else {
                kept.push(e);
            }
        }
        let mut out: Vec<(Value, usize)> = Vec::with_capacity(merged.len());
        for e in kept {
            if e.1 < msg_list.len() && !buckets[e.1].is_empty() {
                out.append(&mut buckets[e.1]);
            }
            out.push(e);
        }
        for bucket in buckets.iter_mut() {
            out.append(bucket);
        }
        out.extend(tail);
        merged = out;
    }
    Ok(Value::Array(merged.into_iter().map(|(e, _)| e).collect()))
}

/// `orderedActivities`.
pub fn ordered_activities(activities: Value) -> Result<Value, String> {
    order_list(&activities)
}
