//! The Rust port of `apps/web/src/pendingInteractionDerivation.ts`.
//!
//! `orderedActivities` and `parseUserInputQuestions` come from
//! `apps/web/src/workLog.ts`; `pendingRequestInstanceKey`,
//! `approvalRequestKindFromRequestType` and `isStalePendingRequestFailureDetail`
//! from `packages/shared/src/threadSummary.ts`; the stale-matcher and reclaim
//! policy from `packages/shared/src/pendingInteractions.ts`. All are inlined so
//! the crate is self-contained. `localeCompare` uses `corpus.rs` ranks.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

// ---------- JS value helpers ----------

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

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) => o.get(key),
        _ => None,
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
                if x.is_null() || crate::js::is_undefined(x) {
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

fn locale_compare(a: &str, b: &str) -> i64 {
    match (crate::corpus::rank(a), crate::corpus::rank(b)) {
        (Some(x), Some(y)) => x - y,
        _ => a.cmp(b) as i64,
    }
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

fn locale_compare_field(
    left: &Value,
    right: &Value,
    field: &str,
    lname: &str,
    rname: &str,
) -> Result<i64, String> {
    let member_expr = format!("{lname}.{field}");
    let lv = member(left, field, &member_expr)?;
    if lv.is_none() || lv.is_some_and(crate::js::is_undefined) {
        return Err(format!(
            "undefined is not an object (evaluating '{member_expr}.localeCompare')"
        ));
    }
    let lv = lv.unwrap_or(&Value::Null);
    if lv.is_null() {
        return Err(format!(
            "null is not an object (evaluating '{member_expr}.localeCompare')"
        ));
    }
    let call_expr = format!("{lname}.{field}.localeCompare({rname}.{field})");
    let Some(ls) = lv.as_str() else {
        return Err(format!(
            "{member_expr}.localeCompare is not a function. (In '{call_expr}', \
             '{member_expr}.localeCompare' is undefined)"
        ));
    };
    let rs = get(right, field)
        .map(js_to_string)
        .unwrap_or_else(|| "undefined".to_string());
    Ok(locale_compare(ls, &rs))
}

fn compare_activities(a: &Value, b: &Value) -> Result<f64, String> {
    let ls = member(a, "sequence", "left.sequence")?;
    let rs = member(b, "sequence", "right.sequence")?;
    let l_def = ls.is_some_and(|v| !crate::js::is_undefined(v));
    let r_def = rs.is_some_and(|v| !crate::js::is_undefined(v));
    if l_def && r_def {
        let (x, y) = (ls.unwrap_or(&Value::Null), rs.unwrap_or(&Value::Null));
        if !(x == y) {
            return Ok(x.as_f64().unwrap_or(f64::NAN) - y.as_f64().unwrap_or(f64::NAN));
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

fn ordered_activities(activities: &Value) -> Result<Value, String> {
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
        return Ok(ordered);
    }
    Err("WeakMap keys must be objects or non-registered symbols".to_string())
}

// ---------- shared helpers ----------

fn pending_request_instance_key(request_id: &str, lifecycle_generation: Option<&str>) -> String {
    format!(
        "{request_id}\u{0}{}",
        lifecycle_generation.unwrap_or("legacy")
    )
}

fn approval_request_kind_from_request_type(request_type: Option<&Value>) -> Option<&'static str> {
    match request_type.and_then(Value::as_str) {
        Some("command_execution_approval") | Some("exec_command_approval") => Some("command"),
        Some("file_read_approval") => Some("file-read"),
        Some("file_change_approval") | Some("apply_patch_approval") => Some("file-change"),
        Some("permissions_approval") => Some("permissions"),
        Some("tool_approval") | Some("dynamic_tool_call") => Some("tool"),
        _ => None,
    }
}

fn is_stale_pending_request_failure_detail(detail: Option<&str>) -> bool {
    let Some(detail) = detail else {
        return false;
    };
    let n = detail.to_lowercase();
    n.contains("stale pending approval request")
        || n.contains("stale pending user-input request")
        || n.contains("unknown pending approval request")
        || n.contains("unknown pending permission request")
        || n.contains("unknown pending user-input request")
        || n.contains("stale pending user input request")
        || n.contains("unknown pending user input request")
}

// ---------- stale matcher (pendingInteractions.ts) ----------

fn stale_key_of(kind: &str, request_id: &str, generation: Option<&str>) -> String {
    format!(
        "{kind}\u{0}{}",
        pending_request_instance_key(request_id, generation)
    )
}

/// `createStalePendingInteractionMatcher` — returns the staleAt table plus a
/// `check(interaction_kind, request_id, lifecycle_generation, created_at)` impl.
struct StaleMatcher {
    stale_at: Vec<(String, String)>,
}

impl StaleMatcher {
    fn build(activities: &[Value]) -> Self {
        let mut stale_at: Vec<(String, String)> = Vec::new();
        for activity in activities {
            let kind = match get(activity, "kind").and_then(Value::as_str) {
                Some("provider.approval.respond.failed") => "approval",
                Some("provider.user-input.respond.failed") => "userInput",
                _ => continue,
            };
            let Some(payload) = get(activity, "payload").and_then(|p| match p {
                Value::Object(o) => Some(o),
                _ => None,
            }) else {
                continue;
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
            let key = stale_key_of(kind, request_id, generation);
            let created = get(activity, "createdAt")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            match stale_at.iter_mut().find(|(k, _)| k == &key) {
                Some((_, at)) => {
                    if created > *at {
                        *at = created;
                    }
                }
                None => stale_at.push((key, created)),
            }
        }
        Self { stale_at }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.stale_at
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn is_stale(
        &self,
        interaction_kind: &str,
        request_id: &str,
        lifecycle_generation: Option<&str>,
        created_at: &str,
    ) -> bool {
        if let Some(g) = lifecycle_generation
            && self
                .get(&stale_key_of(interaction_kind, request_id, Some(g)))
                .is_some()
        {
            return true;
        }
        match self.get(&stale_key_of(interaction_kind, request_id, None)) {
            Some(legacy) => legacy >= created_at,
            None => false,
        }
    }
}

// ---------- Date.parse + reclaim (pendingInteractions.ts) ----------

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
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

fn date_parse(t: &str) -> f64 {
    let t = t.trim();
    let b = t.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return f64::NAN;
    }
    let (Some(y), Some(m), Some(d)) = (t.get(0..4), t.get(5..7), t.get(8..10)) else {
        return f64::NAN;
    };
    let (Ok(y), Ok(m), Ok(d)) = (y.parse::<i64>(), m.parse::<i64>(), d.parse::<i64>()) else {
        return f64::NAN;
    };
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return f64::NAN;
    }
    if b.len() == 10 {
        return epoch_ms(y, m, d, 0, 0, 0, 0);
    }
    if b[10] != b'T' && b[10] != b' ' {
        return f64::NAN;
    }
    let Some(rest) = t.get(11..) else {
        return f64::NAN;
    };
    let (time_part, tz) = match rest.find(['Z', '+', '-']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let segs: Vec<&str> = time_part.split(':').collect();
    if segs.len() < 2 || segs.len() > 3 {
        return f64::NAN;
    }
    let (Ok(h), Ok(min)) = (segs[0].parse::<i64>(), segs[1].parse::<i64>()) else {
        return f64::NAN;
    };
    let (s, ms) = match segs.get(2) {
        None => (0, 0),
        Some(x) => {
            let (ss, f) = x.split_once('.').unwrap_or((*x, "0"));
            let mut f3 = f.chars().take(3).collect::<String>();
            while f3.len() < 3 {
                f3.push('0');
            }
            match (ss.parse::<i64>(), f3.parse::<i64>()) {
                (Ok(a), Ok(b)) => (a, b),
                _ => return f64::NAN,
            }
        }
    };
    let mut offset: i64 = 0;
    if !tz.is_empty() && tz != "Z" {
        let sign: i64 = if tz.starts_with('-') { -1 } else { 1 };
        let body = &tz[1..];
        let (hh, mm) = body.split_once(':').unwrap_or((body, "0"));
        match (hh.parse::<i64>(), mm.parse::<i64>()) {
            (Ok(a), Ok(b)) => offset = sign * (a * 3_600_000 + b * 60_000),
            _ => return f64::NAN,
        }
    }
    epoch_ms(y, m, d, h, min, s, ms) - offset as f64
}

fn to_iso_string(ms: f64) -> String {
    let days = (ms / 86_400_000.0).floor() as i64;
    let mut rem = (ms - days as f64 * 86_400_000.0) as i64;
    // civil_from_days (Hinnant)
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let h = rem / 3_600_000;
    rem %= 3_600_000;
    let min = rem / 60_000;
    rem %= 60_000;
    let s = rem / 1_000;
    let mss = rem % 1_000;
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}.{mss:03}Z")
}

fn responding_interaction_reclaim_cutoff(requested_at: &str) -> String {
    let ms = date_parse(requested_at);
    if ms.is_nan() {
        requested_at.to_string()
    } else {
        to_iso_string(ms - 30_000.0)
    }
}

/// `isPendingInteractionResponseClaimable`.
fn is_response_claimable(
    status: &str,
    response_requested_at: Option<&str>,
    requested_at: &str,
) -> bool {
    match status {
        "pending" | "retryable" | "uncertain" => true,
        "responding" => match response_requested_at {
            None => true,
            Some(rr) => rr <= responding_interaction_reclaim_cutoff(requested_at).as_str(),
        },
        _ => false,
    }
}

// ---------- parseUserInputQuestions (workLog.ts) ----------

fn parse_user_input_questions(payload: Option<&Map<String, Value>>) -> Option<Vec<Value>> {
    let questions = payload?.get("questions")?.as_array()?;
    let mut parsed = Vec::new();
    for entry in questions {
        let Some(q) = entry.as_object() else {
            continue;
        };
        let (Some(id), Some(header), Some(question), Some(options)) = (
            q.get("id").and_then(Value::as_str),
            q.get("header").and_then(Value::as_str),
            q.get("question").and_then(Value::as_str),
            q.get("options").and_then(Value::as_array),
        ) else {
            continue;
        };
        let mut opts = Vec::new();
        for option in options {
            let Some(o) = option.as_object() else {
                continue;
            };
            let (Some(label), Some(desc)) = (
                o.get("label").and_then(Value::as_str),
                o.get("description").and_then(Value::as_str),
            ) else {
                continue;
            };
            opts.push(Value::Object(Map::from_iter([
                ("label".to_string(), Value::String(label.to_string())),
                ("description".to_string(), Value::String(desc.to_string())),
            ])));
        }
        let mut qv = Map::from_iter([
            ("id".to_string(), Value::String(id.to_string())),
            ("header".to_string(), Value::String(header.to_string())),
            ("question".to_string(), Value::String(question.to_string())),
            ("options".to_string(), Value::Array(opts)),
        ]);
        if q.get("multiSelect") == Some(&Value::Bool(true)) {
            qv.insert("multiSelect".to_string(), Value::Bool(true));
        }
        parsed.push(Value::Object(qv));
    }
    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

fn parse_tool_params_display(value: Option<&Value>) -> Option<Vec<Value>> {
    let arr = value?.as_array()?;
    let mut entries = Vec::new();
    for entry in arr {
        let Some(record) = entry.as_object() else {
            continue;
        };
        let Some(name) = record.get("name").and_then(Value::as_str) else {
            continue;
        };
        if !record.contains_key("value") {
            continue;
        }
        let display_name = record
            .get("displayName")
            .and_then(Value::as_str)
            .or_else(|| record.get("display_name").and_then(Value::as_str));
        let mut e = Map::from_iter([
            ("name".to_string(), Value::String(name.to_string())),
            (
                "value".to_string(),
                record.get("value").cloned().unwrap_or(Value::Null),
            ),
        ]);
        if let Some(dn) = display_name.filter(|d| !d.is_empty()) {
            e.insert("displayName".to_string(), Value::String(dn.to_string()));
        }
        entries.push(Value::Object(e));
    }
    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

// ---------- replay engine ----------

fn lifecycle_generation_of(payload: Option<&Map<String, Value>>) -> Option<String> {
    payload
        .and_then(|p| p.get("lifecycleGeneration"))
        .and_then(Value::as_str)
        .filter(|g| !g.is_empty())
        .map(str::to_string)
}

fn delete_pending(
    open: &mut Vec<(String, Value)>,
    request_id: &str,
    lifecycle_generation: Option<&str>,
) {
    if let Some(g) = lifecycle_generation {
        let key = pending_request_instance_key(request_id, Some(g));
        open.retain(|(k, _)| k != &key);
        return;
    }
    open.retain(|(_, p)| get(p, "requestId").and_then(Value::as_str) != Some(request_id));
}

fn replace_pending(
    open: &mut Vec<(String, Value)>,
    pending: Value,
    lifecycle_generation: Option<&str>,
) {
    let request_id = get(&pending, "requestId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    delete_pending(open, &request_id, None);
    open.push((
        pending_request_instance_key(&request_id, lifecycle_generation),
        pending,
    ));
}

type ParseRequested =
    dyn Fn(&Value, Option<&Map<String, Value>>, &str, Option<&str>) -> Option<Value>;

#[allow(clippy::too_many_arguments)]
fn replay_pending_interactions(
    activities: &Value,
    settlements: Option<&Value>,
    interaction_kind: &str,
    requested_kind: &str,
    resolved_kind: &str,
    parse_requested: &ParseRequested,
    options: Option<&Value>,
) -> Result<Vec<Value>, String> {
    let mut open: Vec<(String, Value)> = Vec::new();
    // JS: `settlements === undefined && options !== undefined` — null passes `!== undefined`.
    let is_aggregate_fallback =
        settlements.is_none_or(crate::js::is_undefined) && options.is_some();
    let fallback_latest_turn_id = if is_aggregate_fallback {
        options.and_then(|o| match o {
            Value::Object(ob) => ob.get("latestTurnId"),
            Value::Null => None,
            _ => None,
        })
    } else {
        None
    };
    let mut latest_turn_requested_keys: Vec<String> = Vec::new();
    let authoritative = match options {
        Some(Value::Null) => {
            return Err(
                "null is not an object (evaluating 'options.authoritativeHasPending')".to_string(),
            );
        }
        Some(Value::Object(o)) => o.get("authoritativeHasPending"),
        _ => None,
    };
    let replay_activities = if !is_aggregate_fallback || authoritative != Some(&Value::Bool(false))
    {
        activities
    } else {
        &Value::Array(Vec::new())
    };

    let ordered = ordered_activities(replay_activities)?;
    let items: Vec<Value> = match &ordered {
        Value::Array(a) => a.clone(),
        other => return Err(format!("{} is not iterable", js_to_string(other))),
    };

    for activity in &items {
        let payload = get(activity, "payload").and_then(|p| match p {
            Value::Object(o) => Some(o),
            _ => None,
        });
        let Some(request_id) = payload
            .and_then(|p| p.get("requestId"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let generation = lifecycle_generation_of(payload);
        let kind = get(activity, "kind").and_then(Value::as_str);
        if kind == Some(requested_kind) {
            let is_latest_turn = fallback_latest_turn_id.is_some()
                && get(activity, "turnId") == fallback_latest_turn_id;
            if is_aggregate_fallback && authoritative != Some(&Value::Bool(true)) && !is_latest_turn
            {
                continue;
            }
            if let Some(pending) =
                parse_requested(activity, payload, request_id, generation.as_deref())
            {
                replace_pending(&mut open, pending, generation.as_deref());
                if is_latest_turn {
                    latest_turn_requested_keys.push(pending_request_instance_key(
                        request_id,
                        generation.as_deref(),
                    ));
                }
            }
            continue;
        }
        if kind == Some(resolved_kind) {
            delete_pending(&mut open, request_id, generation.as_deref());
        }
    }

    if !open.is_empty() {
        let matcher = StaleMatcher::build(&items);
        open.retain(|(_, pending)| {
            !matcher.is_stale(
                interaction_kind,
                get(pending, "requestId")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                get(pending, "lifecycleGeneration").and_then(Value::as_str),
                get(pending, "createdAt")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
        });
    }

    // retainActionableSettlements — only runs when `settlements !== undefined`.
    if let Some(settlements) = settlements.filter(|v| !crate::js::is_undefined(v)) {
        let list: &[Value] = match settlements {
            Value::Array(a) => a,
            Value::Null => {
                return Err("null is not an object (evaluating 'settlements.filter')".to_string());
            }
            _ => {
                return Err(
                    "settlements.filter is not a function. (In 'settlements.filter((settlement) => \
                     settlement.interactionKind === interactionKind && ...', \
                     'settlements.filter' is undefined)"
                        .to_string(),
                );
            }
        };
        let claim_ref = options
            .and_then(|o| match o {
                Value::Object(ob) => ob.get("responseClaimReferenceAt"),
                _ => None,
            })
            .and_then(Value::as_str);
        let mut actionable: Vec<String> = Vec::new();
        for settlement in list {
            if get(settlement, "interactionKind").and_then(Value::as_str) != Some(interaction_kind)
            {
                continue;
            }
            let status = get(settlement, "status").and_then(Value::as_str);
            let claimable = matches!(status, Some("pending") | Some("retryable"))
                || (claim_ref.is_some()
                    && status.is_some_and(|s| {
                        is_response_claimable(
                            s,
                            get(settlement, "responseRequestedAt").and_then(Value::as_str),
                            claim_ref.unwrap_or(""),
                        )
                    }));
            if claimable {
                let key = pending_request_instance_key(
                    get(settlement, "requestId")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    get(settlement, "lifecycleGeneration").and_then(Value::as_str),
                );
                if !actionable.contains(&key) {
                    actionable.push(key);
                }
            }
        }
        open.retain(|(k, _)| actionable.contains(k));
    }

    if is_aggregate_fallback && authoritative == Some(&Value::Bool(true)) {
        let actionable_latest: Vec<String> = open
            .iter()
            .map(|(k, _)| k.clone())
            .filter(|k| latest_turn_requested_keys.contains(k))
            .collect();
        if !actionable_latest.is_empty() {
            open.retain(|(k, _)| actionable_latest.contains(k));
        } else if open.len() > 1 {
            let mut entries = open.clone();
            entries.sort_by(|(_, l), (_, r)| {
                let lc = get(l, "createdAt").and_then(Value::as_str).unwrap_or("");
                let rc = get(r, "createdAt").and_then(Value::as_str).unwrap_or("");
                if lc == rc {
                    let li = get(l, "requestId").and_then(Value::as_str).unwrap_or("");
                    let ri = get(r, "requestId").and_then(Value::as_str).unwrap_or("");
                    locale_compare(li, ri).cmp(&0)
                } else {
                    locale_compare(lc, rc).cmp(&0)
                }
            });
            open.clear();
            if let Some(last) = entries.pop() {
                open.push(last);
            }
        }
    }

    open.sort_by(|(_, l), (_, r)| {
        let lc = get(l, "createdAt").and_then(Value::as_str).unwrap_or("");
        let rc = get(r, "createdAt").and_then(Value::as_str).unwrap_or("");
        locale_compare(lc, rc).cmp(&0)
    });
    Ok(open.into_iter().map(|(_, v)| v).collect())
}

// ---------- exported ports ----------

/// Port of `derivePendingApprovals` (`apps/web/src/pendingInteractionDerivation.ts:254`).
pub fn derive_pending_approvals(
    activities: serde_json::Value,
    settlements: serde_json::Value,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // JS `!== undefined` keeps null meaningful for both params.
    let opt = (!crate::js::is_undefined(&options)).then_some(&options);
    let parse_requested = |activity: &Value,
                           payload: Option<&Map<String, Value>>,
                           request_id: &str,
                           lifecycle_generation: Option<&str>|
     -> Option<Value> {
        let request_kind = match payload
            .and_then(|p| p.get("requestKind"))
            .and_then(Value::as_str)
        {
            Some("command") => Some("command"),
            Some("file-read") => Some("file-read"),
            Some("file-change") => Some("file-change"),
            Some("permissions") => Some("permissions"),
            Some("tool") => Some("tool"),
            _ => {
                approval_request_kind_from_request_type(payload.and_then(|p| p.get("requestType")))
            }
        }?;
        let mut out = Map::new();
        out.insert(
            "requestId".to_string(),
            Value::String(request_id.to_string()),
        );
        if let Some(g) = lifecycle_generation {
            out.insert(
                "lifecycleGeneration".to_string(),
                Value::String(g.to_string()),
            );
        }
        out.insert(
            "requestKind".to_string(),
            Value::String(request_kind.to_string()),
        );
        out.insert(
            "createdAt".to_string(),
            get(activity, "createdAt")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        );
        if let Some(detail) = payload
            .and_then(|p| p.get("detail"))
            .and_then(Value::as_str)
            .filter(|d| !d.is_empty())
        {
            out.insert("detail".to_string(), Value::String(detail.to_string()));
        }
        if let Some(profile) = payload
            .and_then(|p| p.get("permissionProfile"))
            .filter(|v| !v.is_null())
            .and_then(|v| match v {
                Value::Object(o) => Some(o.clone()),
                _ => None,
            })
        {
            out.insert("permissionProfile".to_string(), Value::Object(profile));
        }
        if let Some(avail) = payload
            .and_then(|p| p.get("sessionApprovalAvailable"))
            .and_then(Value::as_bool)
        {
            out.insert("sessionApprovalAvailable".to_string(), Value::Bool(avail));
        }
        match payload
            .and_then(|p| p.get("approvalScope"))
            .and_then(Value::as_str)
        {
            Some("computer-task") => {
                out.insert(
                    "approvalScope".to_string(),
                    Value::String("computer-task".to_string()),
                );
            }
            Some("device-task") => {
                out.insert(
                    "approvalScope".to_string(),
                    Value::String("device-task".to_string()),
                );
            }
            Some("computer-foreground") => {
                out.insert(
                    "approvalScope".to_string(),
                    Value::String("computer-foreground".to_string()),
                );
            }
            _ => {}
        }
        if let Some(tool) = payload
            .and_then(|p| p.get("toolName"))
            .and_then(Value::as_str)
            .filter(|t| !t.is_empty())
        {
            out.insert("toolName".to_string(), Value::String(tool.to_string()));
        }
        if let Some(display) =
            parse_tool_params_display(payload.and_then(|p| p.get("toolParamsDisplay")))
        {
            out.insert("toolParamsDisplay".to_string(), Value::Array(display));
        }
        Some(Value::Object(out))
    };
    let mut approvals = replay_pending_interactions(
        &activities,
        Some(&settlements),
        "approval",
        "approval.requested",
        "approval.resolved",
        &parse_requested,
        opt,
    )?;
    if crate::js::is_undefined(&settlements) {
        return Ok(Value::Array(approvals));
    }
    // retryable attempt keys
    let mut attempt_keys: Vec<(String, String)> = Vec::new();
    if let Value::Array(list) = &settlements {
        for settlement in list {
            if get(settlement, "interactionKind").and_then(Value::as_str) != Some("approval")
                || get(settlement, "status").and_then(Value::as_str) != Some("retryable")
            {
                continue;
            }
            let key = pending_request_instance_key(
                get(settlement, "requestId")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                get(settlement, "lifecycleGeneration").and_then(Value::as_str),
            );
            let pair = Value::Array(vec![
                get(settlement, "responseCommandId")
                    .cloned()
                    .unwrap_or(Value::Null),
                get(settlement, "responseRequestedAt")
                    .cloned()
                    .unwrap_or(Value::Null),
            ]);
            attempt_keys.push((key, serde_json::to_string(&pair).unwrap_or_default()));
        }
    }
    for approval in &mut approvals {
        let Some(obj) = approval.as_object_mut() else {
            continue;
        };
        let key = pending_request_instance_key(
            obj.get("requestId").and_then(Value::as_str).unwrap_or(""),
            obj.get("lifecycleGeneration").and_then(Value::as_str),
        );
        if let Some((_, attempt)) = attempt_keys.iter().find(|(k, _)| k == &key) {
            obj.insert(
                "responseAttemptKey".to_string(),
                Value::String(attempt.clone()),
            );
        }
    }
    Ok(Value::Array(approvals))
}

/// Port of `derivePendingUserInputs` (`apps/web/src/pendingInteractionDerivation.ts:365`).
pub fn derive_pending_user_inputs(
    activities: serde_json::Value,
    settlements: serde_json::Value,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let opt = (!crate::js::is_undefined(&options)).then_some(&options);
    let parse_requested = |activity: &Value,
                           payload: Option<&Map<String, Value>>,
                           request_id: &str,
                           lifecycle_generation: Option<&str>|
     -> Option<Value> {
        let questions = parse_user_input_questions(payload)?;
        let mut out = Map::new();
        out.insert(
            "requestId".to_string(),
            Value::String(request_id.to_string()),
        );
        if let Some(g) = lifecycle_generation {
            out.insert(
                "lifecycleGeneration".to_string(),
                Value::String(g.to_string()),
            );
        }
        out.insert(
            "createdAt".to_string(),
            get(activity, "createdAt")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        );
        out.insert("questions".to_string(), Value::Array(questions));
        Some(Value::Object(out))
    };
    let inputs = replay_pending_interactions(
        &activities,
        Some(&settlements),
        "userInput",
        "user-input.requested",
        "user-input.resolved",
        &parse_requested,
        opt,
    )?;
    Ok(Value::Array(inputs))
}
