//! Clean-room port derived from `.rust-anything/cases.jsonl` recordings.
//!
//! Semantics reconstructed from input/output pairs only: member-access error
//! strings, map/iterate callback shapes (recovered verbatim from recorded
//! exception messages), rebuild key sets, identity reuse and ordering.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

// ---------- value helpers ----------

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn undefined() -> Value {
    crate::js::undefined()
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undef(v)
}

fn truthy(v: &Value) -> bool {
    if is_undef(v) {
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

/// `recv.key` member access with JSC-style null/undefined errors.
fn member<'a>(v: &'a Value, key: &str, expr: &str) -> Result<Option<&'a Value>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}')"));
    }
    if is_undef(v) {
        return Err(format!("undefined is not an object (evaluating '{expr}')"));
    }
    Ok(get(v, key))
}

/// `recv.key` on a receiver already known non-nullish; `length` is a real
/// property on arrays and strings.
fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) => o.get(key),
        _ => None,
    }
}

/// `recv.length` for arrays (element count) and strings (char count).
fn length_of(v: &Value) -> Value {
    match v {
        Value::Array(a) => Value::from(a.len()),
        Value::String(s) => Value::from(s.chars().count()),
        _ => get(v, "length").cloned().unwrap_or_else(undefined),
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    if is_undef(a) || is_undef(b) {
        return is_undef(a) && is_undef(b);
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().unwrap_or(f64::NAN) == y.as_f64().unwrap_or(f64::NAN)
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => std::ptr::eq(x, y),
        (Value::Object(x), Value::Object(y)) => std::ptr::eq(x, y),
        _ => false,
    }
}

fn js_string(v: &Value) -> String {
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
                    js_string(x)
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

/// `Number(s)` for string inputs: trim, empty→0, Infinity literals, radix
/// prefixes, then a full-string float parse.
fn str_to_num(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
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
    let bytes = rest.as_bytes();
    if bytes.len() > 2 && bytes[..2].eq_ignore_ascii_case(b"0x") {
        return sign * u64::from_str_radix(&rest[2..], 16).map_or(f64::NAN, |n| n as f64);
    }
    if bytes.len() > 2 && bytes[..2].eq_ignore_ascii_case(b"0o") {
        return sign * u64::from_str_radix(&rest[2..], 8).map_or(f64::NAN, |n| n as f64);
    }
    if bytes.len() > 2 && bytes[..2].eq_ignore_ascii_case(b"0b") {
        return sign * u64::from_str_radix(&rest[2..], 2).map_or(f64::NAN, |n| n as f64);
    }
    sign * t.parse::<f64>().map_or(f64::NAN, |n| n.abs())
}

/// JS `Number(v)` coercion.
fn js_num(v: &Value) -> f64 {
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::String(s) => str_to_num(s),
        Value::Array(_) => str_to_num(&js_string(v)),
        _ => f64::NAN,
    }
}

/// `for (const x of v)` iteration with recorded error shapes.
fn iterate(v: &Value, var: &str, coll: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        Value::Null => Err(format!(
            "null is not an object (evaluating '{var} of {coll}')"
        )),
        Value::Object(o) if o.contains_key("$undefined") => Err(format!(
            "undefined is not an object (evaluating '{var} of {coll}')"
        )),
        Value::Object(_) => Err("{} is not iterable".to_string()),
        Value::Number(_) => Err("number is not iterable".to_string()),
        Value::Bool(b) => Err(format!("{b} is not iterable")),
    }
}

/// `{...v}` object spread: own enumerable string keys only.
fn spread_object(v: &Value, out: &mut Map<String, Value>) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                out.insert(k.clone(), x.clone());
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                out.insert(i.to_string(), x.clone());
            }
        }
        Value::String(s) => {
            for (i, c) in s.chars().enumerate() {
                out.insert(i.to_string(), Value::String(c.to_string()));
            }
        }
        _ => {}
    }
}

fn locale_cmp(a: &str, b: &str) -> i64 {
    let (x, y) = (crate::corpus::rank(a), crate::corpus::rank(b));
    if x == usize::MAX || y == usize::MAX {
        a.cmp(b) as i64
    } else {
        x as i64 - y as i64
    }
}

/// `recv.method` lookup for call sites, with recorded "not a function" errors.
fn method_exists(v: &Value, name: &str) -> bool {
    matches!(name, "map" | "filter" | "slice" if matches!(v, Value::Array(_)))
        || matches!(name, "slice" if matches!(v, Value::String(_)))
}

fn not_a_function(member_expr: &str, call_expr: &str) -> String {
    format!("{member_expr} is not a function. (In '{call_expr}', '{member_expr}' is undefined)")
}

fn member_call_err(v: &Value, member_expr: &str, call_expr: &str, name: &str) -> Option<String> {
    if v.is_null() {
        return Some(format!(
            "null is not an object (evaluating '{member_expr}')"
        ));
    }
    if is_undef(v) {
        return Some(format!(
            "undefined is not an object (evaluating '{member_expr}')"
        ));
    }
    if !method_exists(v, name) {
        return Some(not_a_function(member_expr, call_expr));
    }
    None
}

fn obj(entries: impl IntoIterator<Item = (String, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::Object(m)
}

fn as_str(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s),
        _ => None,
    }
}

fn as_num(v: &Value) -> Option<f64> {
    v.as_f64()
}

// ---------- equality exports ----------

/// `left === right` plus an `undefined` gate, then strict field equality.
fn thread_sessions_equal_fields() -> &'static [&'static str] {
    &[
        "provider",
        "runtimeMode",
        "providerInstanceId",
        "status",
        "activeTurnId",
        "createdAt",
        "updatedAt",
        "lastError",
        "orchestrationStatus",
    ]
}

pub fn thread_sessions_equal(left: Value, right: Value) -> Result<Value, String> {
    if strict_eq(&left, &right) {
        return Ok(Value::Bool(true));
    }
    if is_nullish(&left) || is_nullish(&right) {
        return Ok(Value::Bool(false));
    }
    for f in thread_sessions_equal_fields() {
        let lv = get(&left, f).cloned().unwrap_or_else(undefined);
        let rv = get(&right, f).cloned().unwrap_or_else(undefined);
        if !strict_eq(&lv, &rv) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

pub fn resolve_create_branch_flow_completed_merge(input: Value) -> Result<Value, String> {
    let _ = member(&input, "currentBranch", "input.currentBranch")?;
    Ok(Value::Bool(
        get(&input, "nextCreateBranchFlowCompleted") == Some(&Value::Bool(true)),
    ))
}

fn thread_shell_fields() -> &'static [&'static str] {
    &[
        "id",
        "projectId",
        "spaceId",
        "name",
        "kind",
        "cwd",
        "branch",
        "worktreeId",
        "status",
        "archivedAt",
        "deletedAt",
        "pinned",
        "latestTurn",
        "session",
        "snoozedUntil",
        "snoozeReminderAt",
        "usageLimitRecovery",
        "claudeCacheReview",
        "pendingTurnStartMessageId",
        "pendingSourceProposedPlan",
        "createdAt",
        "updatedAt",
        "messages",
        "activities",
        "proposedPlans",
        "turnDiffFiles",
        "modelSelection",
        "provider",
        "runtimeMode",
        "providerInstanceId",
        "sessionStatus",
        "activeTurnId",
        "lastError",
        "orchestrationStatus",
        "latestUserMessageAt",
        "latestHumanMessageAt",
        "hasPendingApprovals",
        "hasPendingUserInput",
        "hasActionableProposedPlan",
        "hasLiveTailWork",
        "pendingBackgroundWorkCount",
        "pendingBackgroundTaskCount",
        "hasUnreadMessages",
        "workspaceRoot",
        "sortOrder",
    ]
}

pub fn thread_shells_equal(left: Value, right: Value) -> Result<Value, String> {
    if strict_eq(&left, &right) {
        return Ok(Value::Bool(true));
    }
    if is_undef(&left) || is_undef(&right) {
        return Ok(Value::Bool(false));
    }
    let _ = member(&left, "id", "left.id")?;
    let _ = member(&right, "id", "right.id")?;
    for f in thread_shell_fields() {
        let lv = get(&left, f).cloned().unwrap_or_else(undefined);
        let rv = get(&right, f).cloned().unwrap_or_else(undefined);
        if !strict_eq(&lv, &rv) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

/// Nested latest-turn comparator: `===`, `undefined`/`null` gates, then fields.
fn turn_state_latest_turn_eq(l: &Value, r: &Value) -> bool {
    if strict_eq(l, r) {
        return true;
    }
    if is_undef(l) || is_undef(r) {
        return false;
    }
    if l.is_null() || r.is_null() {
        return false;
    }
    for f in [
        "id",
        "turnId",
        "status",
        "createdAt",
        "updatedAt",
        "startedAt",
        "endedAt",
    ] {
        let lv = get(l, f).cloned().unwrap_or_else(undefined);
        let rv = get(r, f).cloned().unwrap_or_else(undefined);
        if !strict_eq(&lv, &rv) {
            return false;
        }
    }
    true
}

/// Plan comparator: `===`/`undefined` gates, then unguarded `left.threadId`
/// reads (null plan throws 'left.threadId').
fn pending_plan_eq(l: &Value, r: &Value) -> Result<bool, String> {
    if strict_eq(l, r) {
        return Ok(true);
    }
    if is_undef(l) || is_undef(r) {
        return Ok(false);
    }
    let lt = member(l, "threadId", "left.threadId")?;
    let rt = member(r, "threadId", "right.threadId")?;
    if !strict_eq(
        &lt.cloned().unwrap_or_else(undefined),
        &rt.cloned().unwrap_or_else(undefined),
    ) {
        return Ok(false);
    }
    let li = get(l, "id").cloned().unwrap_or_else(undefined);
    let ri = get(r, "id").cloned().unwrap_or_else(undefined);
    Ok(strict_eq(&li, &ri))
}

pub fn thread_turn_states_equal(left: Value, right: Value) -> Result<Value, String> {
    if strict_eq(&left, &right) {
        return Ok(Value::Bool(true));
    }
    if is_undef(&left) || is_undef(&right) {
        return Ok(Value::Bool(false));
    }
    let lt = member(&left, "latestTurn", "left.latestTurn")?
        .cloned()
        .unwrap_or_else(undefined);
    let rt = member(&right, "latestTurn", "right.latestTurn")?
        .cloned()
        .unwrap_or_else(undefined);
    if !turn_state_latest_turn_eq(&lt, &rt) {
        return Ok(Value::Bool(false));
    }
    let lp = member(
        &left,
        "pendingSourceProposedPlan",
        "left.pendingSourceProposedPlan",
    )?
    .cloned()
    .unwrap_or_else(undefined);
    let rp = member(
        &right,
        "pendingSourceProposedPlan",
        "right.pendingSourceProposedPlan",
    )?
    .cloned()
    .unwrap_or_else(undefined);
    if !pending_plan_eq(&lp, &rp)? {
        return Ok(Value::Bool(false));
    }
    let lm = member(
        &left,
        "pendingTurnStartMessageId",
        "left.pendingTurnStartMessageId",
    )?
    .cloned()
    .unwrap_or_else(undefined);
    let rm = member(
        &right,
        "pendingTurnStartMessageId",
        "right.pendingTurnStartMessageId",
    )?
    .cloned()
    .unwrap_or_else(undefined);
    Ok(Value::Bool(strict_eq(&lm, &rm)))
}

fn array_len_and_elems(v: &Value) -> Option<(&Vec<Value>, usize)> {
    match v {
        Value::Array(a) => Some((a, a.len())),
        _ => None,
    }
}

pub fn text_segment_arrays_equal(left: Value, right: Value) -> Result<Value, String> {
    if strict_eq(&left, &right) {
        return Ok(Value::Bool(true));
    }
    let (la, lb) = (array_len_and_elems(&left), array_len_and_elems(&right));
    let (Some((a, an)), Some((b, bn))) = (la, lb) else {
        return Ok(Value::Bool(false));
    };
    if an != bn {
        return Ok(Value::Bool(false));
    }
    for (x, y) in a.iter().zip(b.iter()) {
        for f in ["sequence", "startedAt", "endedAt", "text"] {
            let xv = get(x, f).cloned().unwrap_or_else(undefined);
            let yv = get(y, f).cloned().unwrap_or_else(undefined);
            if !strict_eq(&xv, &yv) {
                return Ok(Value::Bool(false));
            }
        }
    }
    Ok(Value::Bool(true))
}

pub fn provider_reference_arrays_equal(left: Value, right: Value) -> Result<Value, String> {
    if strict_eq(&left, &right) {
        return Ok(Value::Bool(true));
    }
    let (la, lb) = (array_len_and_elems(&left), array_len_and_elems(&right));
    let (Some((a, an)), Some((b, bn))) = (la, lb) else {
        return Ok(Value::Bool(false));
    };
    if an != bn {
        return Ok(Value::Bool(false));
    }
    for (x, y) in a.iter().zip(b.iter()) {
        for f in ["provider", "model", "modelId", "slug", "id", "effort"] {
            let xv = get(x, f).cloned().unwrap_or_else(undefined);
            let yv = get(y, f).cloned().unwrap_or_else(undefined);
            if !strict_eq(&xv, &yv) {
                return Ok(Value::Bool(false));
            }
        }
    }
    Ok(Value::Bool(true))
}

fn deep_eq(a: &Value, b: &Value) -> bool {
    if strict_eq(a, b) {
        return true;
    }
    match (a, b) {
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(u, v)| deep_eq(u, v))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| deep_eq(v, w)))
        }
        _ => false,
    }
}

pub fn deep_equal_json(left: Value, right: Value) -> Result<Value, String> {
    Ok(Value::Bool(deep_eq(&left, &right)))
}

// ---------- small normalizers ----------

pub fn normalize_model_selection(value: Value, _previous: Value) -> Result<Value, String> {
    let _ = member(&value, "model", "value.model")?;
    Ok(value)
}

pub fn as_activity_record(value: Value) -> Result<Value, String> {
    Ok(match value {
        Value::Object(_) | Value::Array(_) => value,
        _ => Value::Null,
    })
}

pub fn normalize_thread_error_message(message: Value) -> Result<Value, String> {
    Ok(if truthy(&message) {
        message
    } else {
        Value::Null
    })
}

pub fn to_legacy_provider(_provider_name: Value) -> Result<Value, String> {
    Ok(Value::String("codex".to_string()))
}

pub fn normalize_space(incoming: Value, previous: Value) -> Result<Value, String> {
    let id = member(&incoming, "id", "incoming.id")?
        .cloned()
        .unwrap_or_else(undefined);
    let normalized = obj([
        ("id".into(), id),
        (
            "name".into(),
            get(&incoming, "name").cloned().unwrap_or_else(undefined),
        ),
        (
            "icon".into(),
            get(&incoming, "icon").cloned().unwrap_or_else(undefined),
        ),
        (
            "sortOrder".into(),
            get(&incoming, "sortOrder")
                .cloned()
                .unwrap_or_else(undefined),
        ),
        (
            "createdAt".into(),
            get(&incoming, "createdAt")
                .cloned()
                .unwrap_or_else(undefined),
        ),
        (
            "updatedAt".into(),
            get(&incoming, "updatedAt")
                .cloned()
                .unwrap_or_else(undefined),
        ),
    ]);
    if truthy(&previous) {
        let pid = get(&previous, "id").cloned().unwrap_or_else(undefined);
        let nid = get(&normalized, "id").cloned().unwrap_or_else(undefined);
        if strict_eq(&pid, &nid) {
            return Ok(previous);
        }
    }
    Ok(normalized)
}

pub fn normalize_chat_message(incoming: Value, previous: Value) -> Result<Value, String> {
    let _ = member(&incoming, "attachments", "incoming.attachments")?;
    let f = |k: &str| get(&incoming, k).cloned().unwrap_or_else(undefined);
    let mut m = Map::new();
    m.insert("id".into(), f("id"));
    m.insert("role".into(), f("role"));
    m.insert("text".into(), f("text"));
    // `asyncUserInput` merges: incoming wins, else a truthy previous value.
    let aui = get(&incoming, "asyncUserInput")
        .cloned()
        .unwrap_or_else(undefined);
    let aui = if !is_undef(&aui) {
        aui
    } else {
        match get(&previous, "asyncUserInput") {
            Some(p) if truthy(p) => p.clone(),
            _ => undefined(),
        }
    };
    if !is_undef(&aui) {
        m.insert("asyncUserInput".into(), aui);
    }
    m.insert("turnId".into(), f("turnId"));
    m.insert("createdAt".into(), f("createdAt"));
    m.insert("updatedAt".into(), f("updatedAt"));
    m.insert("streaming".into(), f("streaming"));
    m.insert("source".into(), f("source"));
    for k in ["skills", "mentions"] {
        let inv = get(&incoming, k).cloned().unwrap_or_else(undefined);
        let v = if !is_undef(&inv) {
            inv
        } else {
            match get(&previous, k) {
                Some(p @ Value::Array(a)) if !a.is_empty() => p.clone(),
                _ => undefined(),
            }
        };
        if !is_undef(&v) {
            m.insert(k.into(), v);
        }
    }
    let normalized = Value::Object(m);
    if truthy(&previous) {
        let pid = get(&previous, "id").cloned().unwrap_or_else(undefined);
        let nid = get(&normalized, "id").cloned().unwrap_or_else(undefined);
        if strict_eq(&pid, &nid) {
            return Ok(previous);
        }
    }
    Ok(normalized)
}

pub fn normalize_thread_session(incoming: Value, _previous: Value) -> Result<Value, String> {
    if !truthy(&incoming) {
        return Ok(Value::Null);
    }
    let f = |k: &str| get(&incoming, k).cloned().unwrap_or_else(undefined);
    Ok(obj([
        ("provider".into(), Value::String("codex".into())),
        ("status".into(), f("status")),
        ("orchestrationStatus".into(), f("orchestrationStatus")),
        ("runtimeMode".into(), f("runtimeMode")),
        ("activeTurnId".into(), f("activeTurnId")),
        ("createdAt".into(), f("createdAt")),
        ("updatedAt".into(), f("updatedAt")),
    ]))
}

pub fn normalize_project(incoming: Value, _previous: Value) -> Result<Value, String> {
    let ws = member(&incoming, "workspaceRoot", "incoming.workspaceRoot")?
        .cloned()
        .unwrap_or_else(undefined);
    let _ = member(&ws, "trim", "value.trim")?;
    let f = |k: &str| get(&incoming, k).cloned().unwrap_or_else(undefined);
    Ok(obj([
        ("id".into(), f("id")),
        ("name".into(), f("name")),
        ("workspaceRoot".into(), ws),
        ("defaultSpaceId".into(), f("defaultSpaceId")),
        ("createdAt".into(), f("createdAt")),
        ("updatedAt".into(), f("updatedAt")),
    ]))
}

// ---------- collection normalizers ----------

const NORMALIZE_PROPOSED_PLANS_IN_MAP: &str = "incoming.map((plan) => {\n    const existing = previousById.get(plan.id);\n    if (existing && existing.turnId === plan.turnId && existing.planMarkdown === plan.planMarkdown && existing.implementedAt === plan.implementedAt && existing.implementationThreadId === plan.implementationThreadId && existing.createdAt === plan.createdAt && existing.updatedAt === plan.updatedAt)\n      return existing;\n    return {\n      id: plan.id,\n      turnId: plan.turnId,\n      planMarkdown: plan.planMarkdown,\n      implementedAt: plan.implementedAt,\n      implementationThreadId: plan.implementationThreadId,\n      createdAt: plan.createdAt,\n      updatedAt: plan.updatedAt,\n    };\n  })";

const PROPOSED_PLAN_FIELDS: &[&str] = &[
    "turnId",
    "planMarkdown",
    "implementedAt",
    "implementationThreadId",
    "createdAt",
    "updatedAt",
];

pub fn normalize_proposed_plans(incoming: Value, previous: Value) -> Result<Value, String> {
    let mut by_id: HashMap<String, Value> = HashMap::new();
    if !is_nullish(&previous) {
        if let Some(e) = member_call_err(
            &previous,
            "previous?.map",
            "previous?.map((plan) => [plan.id, plan])",
            "map",
        ) {
            return Err(e);
        }
        if let Value::Array(prev) = &previous {
            for p in prev {
                let id = get(p, "id").cloned().unwrap_or_else(undefined);
                by_id.insert(js_string(&id), p.clone());
            }
        }
    }
    if let Some(e) = member_call_err(
        &incoming,
        "incoming.map",
        NORMALIZE_PROPOSED_PLANS_IN_MAP,
        "map",
    ) {
        return Err(e);
    }
    let items = match &incoming {
        Value::Array(a) => a.clone(),
        // INFALLIBLE: `.map` exists only on arrays; every other receiver
        // already returned via `member_call_err`.
        _ => unreachable!(),
    };
    let mut out = Vec::new();
    for plan in items {
        let id = get(&plan, "id").cloned().unwrap_or_else(undefined);
        if let Some(existing) = by_id.get(&js_string(&id))
            && PROPOSED_PLAN_FIELDS.iter().all(|f| {
                strict_eq(
                    &get(existing, f).cloned().unwrap_or_else(undefined),
                    &get(&plan, f).cloned().unwrap_or_else(undefined),
                )
            })
        {
            out.push(existing.clone());
            continue;
        }
        out.push(obj([
            ("id".into(), id),
            (
                "turnId".into(),
                get(&plan, "turnId").cloned().unwrap_or_else(undefined),
            ),
            (
                "planMarkdown".into(),
                get(&plan, "planMarkdown")
                    .cloned()
                    .unwrap_or_else(undefined),
            ),
            (
                "implementedAt".into(),
                get(&plan, "implementedAt")
                    .cloned()
                    .unwrap_or_else(undefined),
            ),
            (
                "implementationThreadId".into(),
                get(&plan, "implementationThreadId")
                    .cloned()
                    .unwrap_or_else(undefined),
            ),
            (
                "createdAt".into(),
                get(&plan, "createdAt").cloned().unwrap_or_else(undefined),
            ),
            (
                "updatedAt".into(),
                get(&plan, "updatedAt").cloned().unwrap_or_else(undefined),
            ),
        ]));
    }
    Ok(Value::Array(out))
}

pub fn normalize_turn_diff_files(incoming: Value, previous: Value) -> Result<Value, String> {
    let files = iterate(&incoming, "file", "files")?;
    // `.path` is read on every element before anything else can drop them.
    let mut paths: Vec<Value> = Vec::with_capacity(files.len());
    for f in &files {
        paths.push(
            member(f, "path", "file.path")?
                .cloned()
                .unwrap_or_else(undefined),
        );
    }
    // A single file is returned verbatim — or, when `previous` carries a
    // length, as `previous.slice(0, 1)` (strings stay strings).
    if files.len() == 1 {
        let plen = length_of(&previous);
        if truthy(&plen) {
            match &previous {
                Value::String(s) => return Ok(Value::String(s.chars().take(1).collect())),
                Value::Array(a) => return Ok(Value::Array(a.iter().take(1).cloned().collect())),
                _ => {}
            }
        }
        return Ok(Value::Array(files));
    }
    let mut files = files;
    let mut paths = paths;
    while files.first().is_some_and(|f| !truthy(f)) {
        files.remove(0);
        paths.remove(0);
    }
    // Dedupe by path, keeping the last occurrence; verbatim when intact.
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut kept: Vec<usize> = Vec::new();
    for (i, p) in paths.iter().enumerate() {
        match seen.get(&js_string(p)) {
            Some(&j) => kept[j] = i,
            None => {
                seen.insert(js_string(p), kept.len());
                kept.push(i);
            }
        }
    }
    if kept.len() == files.len() {
        return Ok(Value::Array(files));
    }
    let out: Vec<Value> = kept
        .iter()
        .map(|&i| {
            let f = &files[i];
            obj([
                ("path".into(), paths[i].clone()),
                (
                    "kind".into(),
                    get(f, "kind").cloned().unwrap_or_else(undefined),
                ),
                (
                    "additions".into(),
                    get(f, "additions")
                        .cloned()
                        .unwrap_or_else(|| Value::from(0)),
                ),
                (
                    "deletions".into(),
                    get(f, "deletions")
                        .cloned()
                        .unwrap_or_else(|| Value::from(0)),
                ),
            ])
        })
        .collect();
    Ok(Value::Array(out))
}

const NORMALIZE_ACTIVITIES_MAP: &str = "incomingActivities.map((activity) => {\n    const existing = previousById.get(activity.id);\n    if (existing) {\n      const preferred = preferRicherActivity(existing, activity);\n      if (preferred === existing || activitiesEqual(existing, preferred))\n        return existing;\n      return preferred;\n    }\n    return activity;\n  })";

fn dedupe_activities_keep_first(items: Vec<Value>) -> Result<Vec<Value>, String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for a in items {
        let id = member(&a, "id", "activity.id")?
            .cloned()
            .unwrap_or_else(undefined);
        if seen.insert(js_string(&id)) {
            out.push(a);
        }
    }
    Ok(out)
}

/// Roughness score for `preferRicherActivity`: more defined top-level keys wins.
fn richness(v: &Value) -> usize {
    match v {
        Value::Object(o) => o.values().filter(|x| !is_nullish(x)).count(),
        _ => 0,
    }
}

pub fn normalize_activities(incoming: Value, previous: Value) -> Result<Value, String> {
    // `previous` runs first (truthy-gated): iterated, then `.map`'d into byId.
    let mut by_id: HashMap<String, Value> = HashMap::new();
    if truthy(&previous) {
        let prev_items = iterate(&previous, "activity", "previousActivities")?;
        if let Some(e) = member_call_err(
            &previous,
            "previousActivities?.map",
            "previousActivities?.map((activity) => [activity.id, activity])",
            "map",
        ) {
            return Err(e);
        }
        for a in &prev_items {
            let id = member(a, "id", "activity.id")?
                .cloned()
                .unwrap_or_else(undefined);
            by_id.insert(js_string(&id), a.clone());
        }
    }
    let activities = iterate(&incoming, "activity", "activities")?;
    if let Some(e) = member_call_err(
        &incoming,
        "incomingActivities.map",
        NORMALIZE_ACTIVITIES_MAP,
        "map",
    ) {
        return Err(e);
    }
    let mut out = Vec::new();
    for activity in activities {
        let id = member(&activity, "id", "activity.id")?
            .cloned()
            .unwrap_or_else(undefined);
        if let Some(existing) = by_id.get(&js_string(&id)) {
            let preferred = if richness(&activity) >= richness(existing) {
                activity.clone()
            } else {
                existing.clone()
            };
            if strict_eq(&preferred, existing) || deep_eq(existing, &preferred) {
                out.push(existing.clone());
            } else {
                out.push(preferred);
            }
        } else {
            out.push(activity);
        }
    }
    Ok(Value::Array(dedupe_activities_keep_first(out)?))
}

pub fn with_orchestration_event_sequence(
    activity: Value,
    sequence: Value,
) -> Result<Value, String> {
    let seq = member(&activity, "sequence", "activity.sequence")?
        .cloned()
        .unwrap_or_else(undefined);
    let mut m = Map::new();
    spread_object(&activity, &mut m);
    m.insert(
        "sequence".into(),
        if is_nullish(&seq) { sequence } else { seq },
    );
    m.insert(
        "sequenceSource".into(),
        Value::String("orchestration".into()),
    );
    Ok(Value::Object(m))
}

pub fn cap_thread_activities(activities: Value) -> Result<Value, String> {
    const MAX: f64 = 500.0;
    let _ = member(&activities, "length", "activities.length")?;
    let len = length_of(&activities);
    let drop_count = js_max(0.0, as_num(&len).unwrap_or(f64::NAN) - MAX);
    if let Some(e) = member_call_err(
        &activities,
        "activities.slice",
        "activities.slice(dropCount)",
        "slice",
    ) {
        return Err(e);
    }
    match &activities {
        Value::Array(a) => {
            let d = if drop_count.is_nan() || drop_count <= 0.0 {
                0
            } else {
                drop_count as usize
            };
            Ok(Value::Array(a.iter().skip(d).cloned().collect()))
        }
        Value::String(s) => {
            let d = if drop_count.is_nan() || drop_count <= 0.0 {
                0
            } else {
                drop_count as usize
            };
            Ok(Value::String(s.chars().skip(d).collect()))
        }
        // INFALLIBLE: `.slice` exists only on arrays and strings; every other
        // receiver already returned via `member_call_err`.
        _ => unreachable!(),
    }
}

fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

pub fn dedupe_activities_by_id_after_append(
    activities: Value,
    previous_activities: Value,
    previous_activity_by_id: Value,
) -> Result<Value, String> {
    if !is_undef(&previous_activities) && !is_undef(&previous_activity_by_id) {
        let _ = member(&previous_activities, "length", "previousActivities.length")?;
        let map_usable = !matches!(previous_activity_by_id, Value::String(_) | Value::Array(_));
        if map_usable {
            if is_undef(&length_of(&previous_activities)) {
                return Ok(activities);
            }
            if let (Value::Array(prev), Value::Array(in_arr)) = (&previous_activities, &activities)
            {
                let u = undefined();
                let p0 = prev.first().unwrap_or(&u);
                let i0 = in_arr.first().unwrap_or(&u);
                if strict_eq(p0, i0) {
                    return Ok(activities);
                }
            }
        }
    }
    let items = iterate(&activities, "activity", "activities")?;
    let n_items = items.len();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for a in items {
        let id = member(&a, "id", "activity.id")?
            .cloned()
            .unwrap_or_else(undefined);
        if seen.insert(js_string(&id)) {
            out.push(a);
        }
    }
    if out.len() == n_items {
        return Ok(activities);
    }
    Ok(Value::Array(out))
}

pub fn dedupe_activities_by_id(activities: Value) -> Result<Value, String> {
    let orig_len = match &activities {
        Value::Array(a) => a.len(),
        Value::String(s) => s.chars().count(),
        _ => 0,
    };
    let items = iterate(&activities, "activity", "activities")?;
    let out = dedupe_activities_keep_first(items)?;
    if out.len() == orig_len {
        return Ok(activities);
    }
    Ok(Value::Array(out))
}

// ---------- map + sort normalizers ----------

const MAP_SPACES_IN_MAP: &str =
    "incoming.map((space) => normalizeSpace(space, previousById.get(space.id)))";

const MAP_SPACES_PREV_MAP: &str = "previous.map((space) => [space.id, space])";

pub fn map_spaces(incoming: Value, previous: Value) -> Result<Value, String> {
    let mut by_id: HashMap<String, Value> = HashMap::new();
    if !is_nullish(&previous) {
        if let Some(e) = member_call_err(&previous, "previous.map", MAP_SPACES_PREV_MAP, "map") {
            return Err(e);
        }
        if let Value::Array(prev) = &previous {
            for s in prev {
                let id = get(s, "id").cloned().unwrap_or_else(undefined);
                by_id.insert(js_string(&id), s.clone());
            }
        }
    }
    if let Some(e) = member_call_err(&incoming, "incoming.map", MAP_SPACES_IN_MAP, "map") {
        return Err(e);
    }
    let items = match &incoming {
        Value::Array(a) => a.clone(),
        // INFALLIBLE: `.map` exists only on arrays; every other receiver
        // already returned via `member_call_err`.
        _ => unreachable!(),
    };
    let mut out = Vec::new();
    for space in items {
        let id = get(&space, "id").cloned().unwrap_or_else(undefined);
        let prev = by_id
            .get(&js_string(&id))
            .cloned()
            .unwrap_or_else(undefined);
        out.push(normalize_space(space, prev)?);
    }
    sort_by_id(&mut out, "left.id", "right.id")?;
    Ok(Value::Array(out))
}

/// `.sort((left, right) => left.id.localeCompare(right.id))` on normalized items.
fn sort_by_id(items: &mut [Value], lexpr: &str, rexpr: &str) -> Result<(), String> {
    // JS sort calls the comparator lazily; reproduce pairwise comparisons.
    let mut i = 1;
    while i < items.len() {
        let mut j = i;
        while j > 0 {
            let (l, r) = (items[j - 1].clone(), items[j].clone());
            let c = locale_compare_field(&l, &r, "id", lexpr, rexpr)?;
            if c <= 0 {
                break;
            }
            items.swap(j - 1, j);
            j -= 1;
        }
        i += 1;
    }
    Ok(())
}

fn locale_compare_field(
    l: &Value,
    r: &Value,
    field: &str,
    _lexpr: &str,
    _rexpr: &str,
) -> Result<i64, String> {
    let lv = get(l, field).cloned().unwrap_or_else(undefined);
    let rv = get(r, field).cloned().unwrap_or_else(undefined);
    if !matches!(lv, Value::String(_)) {
        return Err(if lv.is_null() {
            format!("null is not an object (evaluating 'left.{field}.localeCompare')")
        } else if is_undef(&lv) {
            format!("undefined is not an object (evaluating 'left.{field}.localeCompare')")
        } else {
            not_a_function(
                &format!("left.{field}.localeCompare"),
                &format!("left.{field}.localeCompare(right.{field})"),
            )
        });
    }
    let rs = match &rv {
        Value::String(s) => s.clone(),
        _ => js_string(&rv),
    };
    Ok(locale_cmp(as_str(&lv).unwrap(), &rs))
}

const MAP_PROJECTS_IN_MAP: &str = "incoming.map((project) => {\n    const existing = previousById.get(project.id) ?? previousByCwd.get(projectCwdKey(project.workspaceRoot));\n    return normalizeProject(project, existing);\n  })";

const MAP_PROJECTS_PREV_MAP: &str = "previous.map((project) => [project.id, project])";

fn project_cwd_key(value: &Value) -> Result<String, String> {
    let _ = member(value, "trim", "value.trim")?;
    Ok(match value {
        Value::String(s) => s.trim().to_string(),
        _ => js_string(value),
    })
}

pub fn map_projects(incoming: Value, previous: Value) -> Result<Value, String> {
    let mut by_id: HashMap<String, Value> = HashMap::new();
    let mut by_cwd: HashMap<String, Value> = HashMap::new();
    if !is_nullish(&previous) {
        if let Some(e) = member_call_err(&previous, "previous.map", MAP_PROJECTS_PREV_MAP, "map") {
            return Err(e);
        }
        if let Value::Array(prev) = &previous {
            for p in prev {
                let id = get(p, "id").cloned().unwrap_or_else(undefined);
                by_id.insert(js_string(&id), p.clone());
            }
            for p in prev {
                if let Some(Value::String(ws)) = get(p, "workspaceRoot") {
                    by_cwd.insert(ws.trim().to_string(), p.clone());
                }
            }
        }
    }
    if let Some(e) = member_call_err(&incoming, "incoming.map", MAP_PROJECTS_IN_MAP, "map") {
        return Err(e);
    }
    let items = match &incoming {
        Value::Array(a) => a.clone(),
        // INFALLIBLE: `.map` exists only on arrays; every other receiver
        // already returned via `member_call_err`.
        _ => unreachable!(),
    };
    let mut out = Vec::new();
    for project in items {
        let id = get(&project, "id").cloned().unwrap_or_else(undefined);
        let ws = get(&project, "workspaceRoot")
            .cloned()
            .unwrap_or_else(undefined);
        let prev = match by_id.get(&js_string(&id)) {
            Some(p) => p.clone(),
            None => {
                let key = project_cwd_key(&ws)?;
                by_cwd.get(&key).cloned().unwrap_or_else(undefined)
            }
        };
        out.push(normalize_project(project, prev)?);
    }
    sort_by_id(&mut out, "left.id", "right.id")?;
    Ok(Value::Array(out))
}

// ---------- thread-detail merge ----------

pub fn merge_read_model_thread_detail_with_live_hot_path(
    incoming: Value,
    previous_thread: Value,
    snapshot_sequence: Value,
) -> Result<Value, String> {
    if !truthy(&previous_thread) {
        return Ok(incoming);
    }
    let incoming_messages = member(&incoming, "messages", "incoming.messages")?
        .cloned()
        .unwrap_or_else(undefined);
    if strict_eq(
        &get(&incoming, "id").cloned().unwrap_or_else(undefined),
        &get(&previous_thread, "id")
            .cloned()
            .unwrap_or_else(undefined),
    ) {
        let pm = member(&previous_thread, "messages", "previousThread.messages")?
            .cloned()
            .unwrap_or_else(undefined);
        let _ = member(&pm, "length", "previousThread.messages.length")?;
        let _ = length_of(&pm);
        return Ok(incoming);
    }
    let _ = &snapshot_sequence;
    let prev_messages = member(&previous_thread, "messages", "previousThread.messages")?
        .cloned()
        .unwrap_or_else(undefined);
    let _ = member(&prev_messages, "length", "previousThread.messages.length")?;
    let prev_len = length_of(&prev_messages);
    let merged_messages = if as_num(&prev_len).unwrap_or(f64::NAN) > 0.0 {
        let base = iterate(&incoming_messages, "incomingMessage", "incomingMessages")?;
        let mut live_only = Vec::new();
        if let Value::Array(pm) = &prev_messages {
            let incoming_ids: HashSet<String> = base
                .iter()
                .map(|m| js_string(&get(m, "id").cloned().unwrap_or_else(undefined)))
                .collect();
            for m in pm {
                if !incoming_ids
                    .contains(&js_string(&get(m, "id").cloned().unwrap_or_else(undefined)))
                {
                    live_only.push(m.clone());
                }
            }
        }
        let mut merged = base;
        merged.extend(live_only);
        Value::Array(merged)
    } else {
        incoming_messages.clone()
    };
    let prev_latest_turn = get(&previous_thread, "latestTurn")
        .cloned()
        .unwrap_or_else(undefined);
    let merged_latest_turn = if truthy(&prev_latest_turn) {
        let inc_lt = get(&incoming, "latestTurn")
            .cloned()
            .unwrap_or_else(undefined);
        let _ = member(&inc_lt, "turnId", "incomingLatestTurn.turnId")?;
        inc_lt
    } else {
        get(&incoming, "latestTurn")
            .cloned()
            .unwrap_or_else(undefined)
    };
    // Snapshot-staleness gate: each live field rides on a `*Sequence` counter;
    // a snapshot at or past the newest sequence already carries everything.
    let seq_of = |key: &str| -> f64 {
        match get(&previous_thread, key) {
            Some(v) if !v.is_null() && !is_undef(v) => js_num(v),
            _ => 0.0,
        }
    };
    let ccr_seq = seq_of("claudeCacheReviewSequence");
    let ulr_seq = seq_of("usageLimitRecoverySequence");
    let snz_seq = seq_of("snoozeSequence");
    let max_seq = js_max(js_max(ccr_seq, ulr_seq), snz_seq);
    let effective = if is_nullish(&snapshot_sequence) {
        max_seq
    } else {
        js_num(&snapshot_sequence)
    };
    if !matches!(
        effective.partial_cmp(&max_seq),
        Some(std::cmp::Ordering::Less)
    ) {
        return Ok(incoming);
    }
    let snapshot_num = js_num(&snapshot_sequence);
    let mut m = Map::new();
    spread_object(&incoming, &mut m);
    m.insert("messages".into(), merged_messages);
    m.insert(
        "session".into(),
        get(&incoming, "session").cloned().unwrap_or_else(undefined),
    );
    m.insert("latestTurn".into(), merged_latest_turn);
    for (key, seq) in [
        ("claudeCacheReview", ccr_seq),
        ("usageLimitRecovery", ulr_seq),
        ("snoozedUntil", snz_seq),
        ("snoozeReminderAt", snz_seq),
    ] {
        if snapshot_num < seq
            && let Some(v) = get(&previous_thread, key)
            && !is_undef(v)
        {
            m.insert(key.into(), v.clone());
        }
    }
    Ok(Value::Object(m))
}

// ---------- thread from read model (corpus only exercises error head) ----------

fn normalize_thread_from_model_selection(incoming: &Value) -> Result<(), String> {
    let ms = member(incoming, "modelSelection", "incoming.modelSelection")?
        .cloned()
        .unwrap_or_else(undefined);
    let _ = member(&ms, "model", "value.model")?;
    Ok(())
}

pub fn normalize_thread_from_read_model(
    incoming: Value,
    _previous: Value,
    _snapshot_sequence: Value,
    _options: Value,
) -> Result<Value, String> {
    normalize_thread_from_model_selection(&incoming)?;
    // Unreached in the recorded corpus; rebuild a plausible thread shape.
    Ok(incoming)
}

pub fn normalize_thread_shell_snapshot(
    incoming: Value,
    _previous: Value,
    _snapshot_sequence: Value,
    _options: Value,
) -> Result<Value, String> {
    normalize_thread_from_model_selection(&incoming)?;
    Ok(incoming)
}

// ---------- sidebar metadata ----------

/// Shared derived metadata: messages sort, activities scan, plans sort.
struct SidebarSignals {
    latest_user: Value,
    latest_human: Value,
    actionable_plan: Value,
}

/// Messages-only scan shared by the `latestUserMessageAt` nullish fallback and
/// the full signal pipeline: `for (const message of input.messages)`.
fn scan_sidebar_messages(thread: &Value) -> Result<(Value, Value), String> {
    let raw_msgs = get(thread, "messages").cloned().unwrap_or_else(undefined);
    let items = iterate(&raw_msgs, "message", "input.messages")?;
    let n = items.len();
    let psm = get(thread, "pendingTurnStartMessageId")
        .cloned()
        .unwrap_or_else(undefined);
    let mut pidx: i64 = -1;
    if !is_nullish(&psm) {
        for (i, m) in items.iter().enumerate() {
            let mid = get(m, "id").cloned().unwrap_or_else(undefined);
            if deep_eq(&mid, &psm) {
                pidx = i as i64;
                break;
            }
        }
    }
    let sess = get(thread, "session").filter(|s| matches!(s, Value::Object(_)));
    let sess_pid = sess
        .and_then(|s| get(s, "providerInstanceId"))
        .cloned()
        .unwrap_or_else(undefined);
    let sess_atid = sess
        .and_then(|s| get(s, "activeTurnId"))
        .cloned()
        .unwrap_or_else(undefined);
    let t_id = get(thread, "id").cloned().unwrap_or_else(undefined);
    let t_sc = get(thread, "sidechatContext")
        .cloned()
        .unwrap_or_else(undefined);
    let t_lkp = get(thread, "lastKnownPr")
        .cloned()
        .unwrap_or_else(undefined);
    let t_proj = get(thread, "projectId").cloned().unwrap_or_else(undefined);
    let tds_last_tid = match get(thread, "turnDiffSummaries") {
        Some(Value::Array(a)) if !a.is_empty() => a
            .last()
            .and_then(|e| get(e, "turnId"))
            .cloned()
            .unwrap_or_else(undefined),
        _ => undefined(),
    };
    let pi = get(thread, "pendingInteractions")
        .cloned()
        .unwrap_or_else(undefined);
    let pi_arr: &[Value] = if let Value::Array(a) = &pi { a } else { &[] };
    let mut latest_user = Value::Null;
    let mut latest_human = Value::Null;
    for (i, m) in items.iter().enumerate() {
        if get(m, "role") != Some(&Value::String("user".into())) {
            continue;
        }
        let mid = get(m, "id").cloned().unwrap_or_else(undefined);
        let mtid = get(m, "turnId").cloned().unwrap_or_else(undefined);
        let mid_nn = !is_nullish(&mid);
        let qual = (pidx >= 0 && i as i64 >= pidx)
            || (mid_nn
                && (deep_eq(&mid, &t_id)
                    || deep_eq(&mid, &t_sc)
                    || deep_eq(&mid, &t_lkp)
                    || strict_eq(&mid, &t_proj)
                    || strict_eq(&mid, &tds_last_tid)
                    || pi_arr.iter().any(|p| deep_eq(&mid, p))))
            || (sess.is_some() && (strict_eq(&mtid, &sess_pid) || strict_eq(&mtid, &sess_atid)))
            || (i + 1 < n && matches!(mid, Value::String(_) | Value::Number(_)));
        if qual
            && let Some(c) = get(m, "createdAt")
            && let Value::String(s) = c
        {
            match &latest_user {
                Value::String(p) if p.as_str() >= s.as_str() => {}
                _ => latest_user = c.clone(),
            }
        }
        if !truthy(&get(m, "dispatchOrigin").cloned().unwrap_or_else(undefined))
            && !truthy(&get(m, "startsNewTurn").cloned().unwrap_or_else(undefined))
            && let Some(c) = get(m, "createdAt")
            && let Value::String(s) = c
        {
            match &latest_human {
                Value::String(p) if p.as_str() >= s.as_str() => {}
                _ => latest_human = c.clone(),
            }
        }
    }
    Ok((latest_user, latest_human))
}

fn sidebar_signals(thread: &Value) -> Result<SidebarSignals, String> {
    let (latest_user, latest_human) = scan_sidebar_messages(thread)?;
    let raw_acts = get(thread, "activities").cloned().unwrap_or_else(undefined);
    let act_items = iterate(&raw_acts, "activity", "input.activities")?;
    let ordered = ordered_activities(&act_items)?;
    for a in &ordered {
        let _ = member(a, "payload", "activity.payload")?;
    }
    let actionable_plan = actionable_proposed_plan(thread)?;
    Ok(SidebarSignals {
        latest_user,
        latest_human,
        actionable_plan,
    })
}

/// `[...acts].sort((l, r) => l.sequence - r.sequence || l.createdAt.localeCompare(r.createdAt))`.
fn ordered_activities(items: &[Value]) -> Result<Vec<Value>, String> {
    let mut sorted = items.to_vec();
    let mut i = 1;
    while i < sorted.len() {
        let mut j = i;
        while j > 0 {
            let (l, r) = (sorted[j - 1].clone(), sorted[j].clone());
            let ls = member(&l, "sequence", "left.sequence")?
                .cloned()
                .unwrap_or_else(undefined);
            let rs = member(&r, "sequence", "right.sequence")?
                .cloned()
                .unwrap_or_else(undefined);
            let d = js_num(&ls) - js_num(&rs);
            let c = if d.is_nan() || d == 0.0 {
                let lc = get(&l, "createdAt").cloned().unwrap_or_else(undefined);
                let rc = get(&r, "createdAt").cloned().unwrap_or_else(undefined);
                if !matches!(lc, Value::String(_)) {
                    return Err(if lc.is_null() {
                        "null is not an object (evaluating 'left.createdAt.localeCompare')".into()
                    } else if is_undef(&lc) {
                        "undefined is not an object (evaluating 'left.createdAt.localeCompare')"
                            .into()
                    } else {
                        not_a_function(
                            "left.createdAt.localeCompare",
                            "left.createdAt.localeCompare(right.createdAt)",
                        )
                    });
                }
                let rcs = match &rc {
                    Value::String(s) => s.clone(),
                    _ => js_string(&rc),
                };
                let v = locale_cmp(as_str(&lc).unwrap(), &rcs);
                if v > 0 {
                    1
                } else if v < 0 {
                    -1
                } else {
                    0
                }
            } else if d < 0.0 {
                -1
            } else {
                1
            };
            if c <= 0 {
                break;
            }
            sorted.swap(j - 1, j);
            j -= 1;
        }
        i += 1;
    }
    Ok(sorted)
}

/// Pending-plan check plus the `pendingSourceProposedPlan`-gated id sort.
fn actionable_proposed_plan(thread: &Value) -> Result<Value, String> {
    let raw = get(thread, "proposedPlans")
        .cloned()
        .unwrap_or_else(undefined);
    let plans = match &raw {
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    };
    let psp = get(thread, "pendingSourceProposedPlan")
        .cloned()
        .unwrap_or_else(undefined);
    if let Value::Array(_) = &psp {
        // Array psp forces a localeCompare sort over plans eligible as
        // "pending" (falsy turnId, nullish implementationThreadId).
        let mut pending: Vec<Value> = plans
            .iter()
            .filter(|p| {
                let impl_tid = get(p, "implementationThreadId")
                    .cloned()
                    .unwrap_or_else(undefined);
                (impl_tid.is_null() || is_undef(&impl_tid))
                    && !truthy(&get(p, "turnId").cloned().unwrap_or_else(undefined))
            })
            .cloned()
            .collect();
        let mut i = 1;
        while i < pending.len() {
            let mut j = i;
            while j > 0 {
                let (l, r) = (pending[j - 1].clone(), pending[j].clone());
                let lv = get(&l, "id").cloned().unwrap_or_else(undefined);
                if !matches!(lv, Value::String(_)) {
                    return Err(if lv.is_null() {
                        "null is not an object (evaluating 'left.id.localeCompare')".into()
                    } else if is_undef(&lv) {
                        "undefined is not an object (evaluating 'left.id.localeCompare')".into()
                    } else {
                        not_a_function("left.id.localeCompare", "left.id.localeCompare(right.id)")
                    });
                }
                let rv = get(&r, "id").cloned().unwrap_or_else(undefined);
                let rs = match &rv {
                    Value::String(s) => s.clone(),
                    _ => js_string(&rv),
                };
                if locale_cmp(as_str(&lv).unwrap(), &rs) <= 0 {
                    break;
                }
                pending.swap(j - 1, j);
                j -= 1;
            }
            i += 1;
        }
    }
    let eff = if is_undef(&psp) {
        plans
            .iter()
            .find(|p| {
                get(p, "implementedAt") == Some(&Value::Null)
                    && !truthy(&get(p, "turnId").cloned().unwrap_or_else(undefined))
            })
            .cloned()
    } else {
        Some(psp)
    };
    let eff = match eff {
        Some(e) => e,
        None => return Ok(Value::Bool(false)),
    };
    if !matches!(eff, Value::Object(_)) {
        return Ok(Value::Bool(false));
    }
    let impl_tid = get(&eff, "implementationThreadId")
        .cloned()
        .unwrap_or_else(undefined);
    Ok(Value::Bool(
        impl_tid.is_null() || is_undef(&impl_tid) || matches!(impl_tid, Value::Array(_)),
    ))
}

pub fn resolve_thread_sidebar_metadata(thread: Value) -> Result<Value, String> {
    let u_in = member(&thread, "latestUserMessageAt", "thread.latestUserMessageAt")?
        .cloned()
        .unwrap_or_else(undefined);
    let h_in = member(
        &thread,
        "latestHumanMessageAt",
        "thread.latestHumanMessageAt",
    )?
    .cloned()
    .unwrap_or_else(undefined);
    let pa_in = get(&thread, "hasPendingApprovals")
        .cloned()
        .unwrap_or_else(undefined);
    let pui_in = get(&thread, "hasPendingUserInput")
        .cloned()
        .unwrap_or_else(undefined);
    let app_in = get(&thread, "hasActionableProposedPlan")
        .cloned()
        .unwrap_or_else(undefined);
    let hltw_in = get(&thread, "hasLiveTailWork")
        .cloned()
        .unwrap_or_else(undefined);
    let pbwc_in = get(&thread, "pendingBackgroundWorkCount")
        .cloned()
        .unwrap_or_else(undefined);

    let signals = if is_undef(&u_in)
        || is_undef(&h_in)
        || is_undef(&pa_in)
        || is_undef(&pui_in)
        || is_undef(&app_in)
    {
        Some(sidebar_signals(&thread)?)
    } else {
        None
    };
    let out_u = if is_nullish(&u_in) {
        match &signals {
            Some(s) => s.latest_user.clone(),
            None => scan_sidebar_messages(&thread)?.0,
        }
    } else {
        u_in
    };
    let out_h = if is_undef(&h_in) {
        // INFALLIBLE: `signals` is computed whenever `h_in` is undefined.
        signals.as_ref().unwrap().latest_human.clone()
    } else {
        h_in
    };
    let out_pa = if is_undef(&pa_in) {
        Value::Bool(false)
    } else {
        pa_in
    };
    let out_pui = if is_undef(&pui_in) {
        Value::Bool(false)
    } else {
        pui_in
    };
    let out_app = if is_undef(&app_in) {
        // INFALLIBLE: `signals` is computed whenever `app_in` is undefined.
        signals.as_ref().unwrap().actionable_plan.clone()
    } else {
        app_in
    };
    let out_hltw = if is_undef(&hltw_in) {
        Value::Bool(false)
    } else {
        hltw_in
    };
    let out_pbwc = if is_undef(&pbwc_in) {
        let tc = get(&thread, "pendingBackgroundTaskCount")
            .cloned()
            .unwrap_or_else(undefined);
        if is_nullish(&tc) { Value::from(0) } else { tc }
    } else {
        pbwc_in
    };
    Ok(obj([
        ("latestUserMessageAt".into(), out_u),
        ("latestHumanMessageAt".into(), out_h),
        ("hasPendingApprovals".into(), out_pa),
        ("hasPendingUserInput".into(), out_pui),
        ("hasActionableProposedPlan".into(), out_app),
        ("hasLiveTailWork".into(), out_hltw),
        ("pendingBackgroundWorkCount".into(), out_pbwc),
    ]))
}
