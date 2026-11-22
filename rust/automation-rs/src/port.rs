//! The Rust port of `apps/server/src/automation/`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

fn marker() -> Value {
    json!({"$undefined": true})
}

fn is_undef(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined").and_then(Value::as_bool) == Some(true))
}

fn prop<'a>(v: &'a Value, key: &str) -> &'a Value {
    match v {
        Value::Object(m) if !m.contains_key("$undefined") => {
            m.get(key).unwrap_or_else(|| marker_value())
        }
        _ => marker_value(),
    }
}

fn marker_value() -> &'static Value {
    static M: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    M.get_or_init(|| json!({"$undefined": true}))
}

/// JS `ToString` for template interpolation.
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() || is_undef(x) {
                    String::new()
                } else {
                    js_str(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(m) => {
            if m.contains_key("$undefined") {
                "undefined".into()
            } else if let Some(n) = m.get("$num") {
                js_str(&json!(
                    n.as_str()
                        .map_or(f64::NAN, |s| s.parse().unwrap_or(f64::NAN))
                ))
            } else {
                "[object Object]".into()
            }
        }
    }
}

fn js_num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Bool(b) => *b as i64 as f64,
        Value::String(s) => s.trim().parse().unwrap_or(f64::NAN),
        Value::Null => 0.0,
        Value::Object(m) if m.contains_key("$num") => m["$num"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

fn str_prop(v: &Value, key: &str) -> String {
    js_str(prop(v, key))
}

// ---------------------------------------------------------------------------
// proposalActivity.ts
// ---------------------------------------------------------------------------

/// Port of `automationProposalActivityId`.
pub fn automation_proposal_activity_id(
    automation_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!(format!(
        "automation-proposal:{}",
        js_str(&automation_id)
    )))
}

fn cadence_label(schedule: &Value) -> Result<String, String> {
    if schedule.is_null() {
        return Err("Cannot read properties of null (reading 'type')".into());
    }
    let ty = prop(schedule, "type");
    Ok(match ty.as_str().unwrap_or("") {
        "manual" => "Manual".into(),
        "project-event" => "On project events".into(),
        "once" => format!("Once {}", str_prop(schedule, "runAt")),
        "interval" => {
            let every = js_num(prop(schedule, "everySeconds"));
            if every % 60.0 == 0.0 {
                format!("Every {}m", crate::js::number_to_string(every / 60.0))
            } else {
                format!("Every {}s", crate::js::number_to_string(every))
            }
        }
        "daily" => format!("Daily at {}", str_prop(schedule, "timeOfDay")),
        "weekdays" => format!("Weekdays at {}", str_prop(schedule, "timeOfDay")),
        "weekly" => format!(
            "Weekly on day {} at {}",
            str_prop(schedule, "dayOfWeek"),
            str_prop(schedule, "timeOfDay")
        ),
        "cron" => format!("Cron {}", str_prop(schedule, "expression")),
        _ => return Ok(String::new()),
    })
}

/// Port of `automationCadenceLabel`. Garbage schedules yield `undefined`.
pub fn automation_cadence_label(schedule: serde_json::Value) -> Result<serde_json::Value, String> {
    if schedule.is_null() {
        return Err("Cannot read properties of null (reading 'type')".into());
    }
    let label = cadence_label(&schedule)?;
    if label.is_empty() {
        return Ok(marker());
    }
    Ok(json!(label))
}

/// Port of `buildAutomationProposalActivity`.
pub fn build_automation_proposal_activity(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot destructure property 'definition' of 'input' as it is null.".into());
    }
    let definition = prop(&input, "definition");
    if is_undef(definition) || definition.is_null() {
        let kind = if definition.is_null() {
            "null"
        } else {
            "undefined"
        };
        return Err(format!("Cannot read properties of {kind} (reading 'id')"));
    }
    let proposal_state = prop(&input, "proposalState");
    let state_label = match proposal_state.as_str() {
        Some("pending") => "Suggested",
        Some("accepted") => "Accepted",
        _ => "Dismissed",
    };
    Ok(json!({
        "id": format!("automation-proposal:{}", js_str(prop(definition, "id"))),
        "tone": "info",
        "kind": "automation.created",
        "summary": format!("{state_label} automation: {}", js_str(prop(definition, "name"))),
        "payload": {
            "source": "agent-gateway",
            "automationId": prop(definition, "id").clone(),
            "automationName": prop(definition, "name").clone(),
            "mode": prop(definition, "mode").clone(),
            "cadenceLabel": automation_cadence_label(prop(definition, "schedule").clone()).unwrap_or(Value::Null),
            "proposalState": proposal_state.clone(),
        },
        "turnId": null,
        "createdAt": prop(definition, "createdAt").clone(),
    }))
}

// ---------------------------------------------------------------------------
// runEnvelope.ts
// ---------------------------------------------------------------------------

const MEMORY_MAX_BYTES: usize = 8 * 1024;
const MEMORY_MARKER: &str = "[... older automation memory truncated ...]\n";

/// Port of `automationMemoryForEnvelope`.
pub fn automation_memory_for_envelope(content: String) -> Result<String, String> {
    let bytes = content.as_bytes();
    if bytes.len() <= MEMORY_MAX_BYTES {
        return Ok(if content.is_empty() {
            "(empty)".into()
        } else {
            content
        });
    }
    let suffix_budget = MEMORY_MAX_BYTES - MEMORY_MARKER.len();
    let mut start = bytes.len() - suffix_budget;
    while start < bytes.len() && (bytes[start] & 0xc0) == 0x80 {
        start += 1;
    }
    Ok(format!(
        "{MEMORY_MARKER}{}",
        String::from_utf8_lossy(&bytes[start..])
    ))
}

fn iteration_label(definition: &Value, run: &Value) -> String {
    let snapshot = prop(run, "permissionSnapshot");
    if is_undef(snapshot) {
        return format!(
            "undefined/{}",
            js_str(&if is_undef(prop(definition, "maxIterations")) {
                json!("∞")
            } else {
                prop(definition, "maxIterations").clone()
            })
        );
    }
    let iteration = prop(snapshot, "iterationNumber");
    let iter = if is_undef(iteration) || iteration.is_null() {
        js_num(prop(definition, "iterationCount")) + 1.0
    } else {
        js_num(iteration)
    };
    let max = prop(definition, "maxIterations");
    let max_s = if is_undef(max) || max.is_null() {
        "∞".to_string()
    } else {
        js_str(max)
    };
    format!("{}/{}", crate::js::number_to_string(iter), max_s)
}

const SELF_CANCEL_INSTRUCTION: &str =
    "You may call synara_cancel_automation on this automation when it is no longer needed.";

fn continues_thread(mode: &Value) -> bool {
    mode.as_str() == Some("heartbeat") || mode.as_str() == Some("dedicated")
}

fn owns_thread(mode: &Value) -> bool {
    mode.as_str() == Some("dedicated")
}

fn reporting_instructions(mode: &Value) -> String {
    if continues_thread(mode) {
        return [
            "Before finishing, call synara_report_automation_result.",
            "Use decision \"silent\" when nothing needs the user's attention; otherwise use \"notify\".",
            SELF_CANCEL_INSTRUCTION,
        ]
        .join(" ");
    }
    [
        "Before finishing, call synara_report_automation_result with a concise title and summary.",
        "Use decision \"notify\" unless the successful run genuinely requires no user attention.",
        SELF_CANCEL_INSTRUCTION,
    ]
    .join(" ")
}

fn thread_scope_line(definition: &Value) -> Option<String> {
    let mode = prop(definition, "mode");
    if is_undef(mode) {
        return None;
    }
    if owns_thread(mode) {
        return Some("Thread scope: this thread belongs to this automation and is reused by every run, so earlier runs are above and your work stays visible to the next one.".into());
    }
    if continues_thread(mode) {
        return Some("Thread scope: this is the thread the automation drives; it may also carry the user's own turns.".into());
    }
    None
}

/// Port of `buildAutomationRunEnvelope`.
pub fn build_automation_run_envelope(input: serde_json::Value) -> Result<String, String> {
    if input.is_null() {
        return Err("Cannot destructure property 'definition' of 'input' as it is null.".into());
    }
    let definition = prop(&input, "definition");
    let mode = prop(definition, "mode");
    if is_undef(mode) {
        // The original first dereferences `mode` inside `threadScopeLine` via the
        // `automationOwnsItsThread(mode)`/`automationContinuesThread(mode)` checks,
        // which never throw; the first observable throw is reading `mode` on a
        // missing definition.
        if is_undef(definition) {
            return Err("Cannot read properties of undefined (reading 'mode')".into());
        }
    }
    let run = prop(&input, "run");
    let trigger = prop(run, "trigger");
    if is_undef(trigger) && !run.is_object() {
        return Err("Cannot read properties of undefined (reading 'trigger')".into());
    }
    let trigger_type = prop(trigger, "type");
    let scheduled_for = prop(run, "scheduledFor");
    let last_run = prop(&input, "lastRunAt");
    let last_run_s = if is_undef(last_run) || last_run.is_null() {
        "never".to_string()
    } else {
        js_str(last_run)
    };
    let thread_scope = thread_scope_line(definition);
    let memory = prop(&input, "memoryContent");
    let memory_s = if let Value::String(s) = memory {
        automation_memory_for_envelope(s.clone())?
    } else {
        automation_memory_for_envelope(js_str(memory))?
    };
    let mut lines = vec![
        format!("Automation: {}", js_str(prop(definition, "name"))),
        format!("Automation ID: {}", js_str(prop(definition, "id"))),
        format!(
            "Run: {}, scheduled for {} (last run: {}, iteration {})",
            js_str(trigger_type),
            js_str(scheduled_for),
            last_run_s,
            iteration_label(definition, run)
        ),
        "Turn scope: this user message is the automation-dispatched turn. These automation-only completion duties do not carry into later manual follow-up turns.".to_string(),
    ];
    if let Some(scope) = thread_scope {
        lines.push(scope);
    }
    lines.extend([
        "Memory (persistent across runs — replace it via synara_update_automation_memory {\"memory\": \"...\"} before finishing):".to_string(),
        memory_s,
        String::new(),
        reporting_instructions(mode),
        String::new(),
        "---".to_string(),
        String::new(),
        js_str(prop(definition, "prompt")),
    ]);
    Ok(lines.join("\n"))
}

// ---------------------------------------------------------------------------
// runResult.ts
// ---------------------------------------------------------------------------

const SUMMARY_MAX_CHARS: usize = 2_000;
const REASON_MAX_CHARS: usize = 1_000;

fn utf16_take(s: &str, units: usize) -> String {
    let mut taken = Vec::new();
    for u in s.encode_utf16() {
        if taken.len() >= units {
            break;
        }
        taken.push(u);
    }
    String::from_utf16_lossy(&taken)
}

fn js_trim(s: &str) -> &str {
    s.trim().trim_matches('\u{feff}').trim()
}

fn summary_of(v: Option<&str>) -> Option<String> {
    let trimmed = js_trim(v?);
    if trimmed.is_empty() {
        None
    } else {
        Some(utf16_take(trimmed, SUMMARY_MAX_CHARS))
    }
}

/// Port of `automationRunResultSummary`.
pub fn automation_run_result_summary(
    value: Option<String>,
    fallback: Option<String>,
) -> Result<Option<String>, String> {
    let summary = value.or(fallback);
    Ok(summary_of(summary.as_deref()))
}

/// Port of `automationRunResultSummaryWithNotice`.
pub fn automation_run_result_summary_with_notice(
    base_summary: Option<String>,
    notice: String,
) -> Result<Option<String>, String> {
    let normalized_base = base_summary.as_deref().map(js_trim);
    if normalized_base.is_none_or(str::is_empty) {
        return automation_run_result_summary(Some(notice), None);
    }
    let separator = "\n\n";
    let available = SUMMARY_MAX_CHARS.saturating_sub(notice.len() + separator.len());
    let truncated = utf16_take(normalized_base.unwrap_or_default(), available);
    let truncated = truncated.trim_end();
    automation_run_result_summary(Some(format!("{truncated}{separator}{notice}")), None)
}

/// Port of `normalizeAutomationCompletionReason`.
pub fn normalize_automation_completion_reason(value: Option<String>) -> Result<String, String> {
    let trimmed = value.as_deref().map(js_trim).unwrap_or_default();
    if trimmed.is_empty() {
        return Ok("No reason given.".into());
    }
    Ok(utf16_take(trimmed, REASON_MAX_CHARS))
}

/// Port of `failedAutomationCompletionEvaluation`.
pub fn failed_automation_completion_evaluation(
    reason: String,
) -> Result<serde_json::Value, String> {
    Ok(json!({
        "stopMatched": false,
        "confidence": 0,
        "reason": normalize_automation_completion_reason(Some(reason))?,
    }))
}

/// Port of `automationCompletionRunResult`.
pub fn automation_completion_run_result(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'baseResult')".into());
    }
    if !input.is_object() {
        return Err("Cannot read properties of undefined (reading 'baseResult')".into());
    }
    let base = prop(&input, "baseResult");
    let base_obj: Map<String, Value> = match base {
        Value::Object(m) if !is_undef(base) => m.clone(),
        _ => json!({"outcome": "unknown", "summary": null, "unread": true, "archivedAt": null})
            .as_object()
            .cloned()
            .unwrap_or_default(),
    };
    let evaluation = prop(&input, "evaluation");
    if is_undef(evaluation) {
        return Err("Cannot read properties of undefined (reading 'reason')".into());
    }
    let matched = prop(&input, "matched");
    let matched = matches!(matched, Value::Bool(true));
    let reason = prop(evaluation, "reason");
    let mut out = base_obj.clone();
    if matched {
        out.insert("outcome".into(), json!("no-findings"));
    }
    let summary = if matched {
        summary_of(Some(&format!("Stopped: {}", js_str(reason))))
    } else {
        let explicit = prop(&input, "summary");
        if is_undef(explicit) {
            summary_of(Some(&js_str(reason)))
        } else {
            summary_of(Some(&js_str(explicit)))
        }
    };
    out.insert("summary".into(), summary.map_or(Value::Null, Value::String));
    if matched {
        out.insert("severity".into(), json!("info"));
    } else {
        let sev = prop(&input, "severity");
        if !is_undef(sev) {
            out.insert("severity".into(), sev.clone());
        }
    }
    let unread = prop(&input, "unread");
    if !is_undef(unread) {
        out.insert("unread".into(), unread.clone());
    }
    out.insert("completionEvaluation".into(), evaluation.clone());
    Ok(Value::Object(out))
}

// ---------------------------------------------------------------------------
// schedule.ts
// ---------------------------------------------------------------------------

const MINUTE_MS: f64 = 60.0 * 1000.0;
const DAY_MS: f64 = 24.0 * 60.0 * MINUTE_MS;
const MAX_CRON_SEARCH_DAYS: i64 = 366;

/// Port of `deterministicAutomationJitterSeconds`.
pub fn deterministic_automation_jitter_seconds(input: serde_json::Value) -> Result<f64, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'installSalt')".into());
    }
    if is_undef(&input) {
        return Err("Cannot read properties of undefined (reading 'installSalt')".into());
    }
    let salt = str_prop(&input, "installSalt");
    let id = str_prop(&input, "automationId");
    let digest = Sha256::digest(format!("{salt}:{id}").as_bytes());
    let mut remainder = 0i64;
    for byte in digest {
        remainder = (remainder * 256 + i64::from(byte)) % 120;
    }
    Ok(remainder as f64)
}

fn schedule_uses_jitter(ty: &str) -> bool {
    matches!(ty, "daily" | "weekdays" | "weekly" | "cron")
}

fn schedule_jitter_ms(ty: &str, context: Option<&Value>) -> Result<f64, String> {
    let Some(ctx) = context else {
        return Ok(0.0);
    };
    if ctx.is_null() || !schedule_uses_jitter(ty) {
        return Ok(0.0);
    }
    Ok(deterministic_automation_jitter_seconds(ctx.clone())? * 1_000.0)
}

fn parse_time_of_day(value: &Value) -> (i64, i64) {
    let s = match value {
        Value::String(s) => s.clone(),
        _ => return (0, 0),
    };
    let mut it = s.split(':');
    let hours = it.next().unwrap_or("0").parse::<i64>().unwrap_or(0);
    let minutes = it.next().unwrap_or("0").parse::<i64>().unwrap_or(0);
    (hours, minutes)
}

// --- civil date arithmetic (days-from-civil, Howard Hinnant) -----------------

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn utc_ms(y: i64, m: i64, d: i64, h: i64, mi: i64, s: i64) -> f64 {
    days_from_civil(y, m, d) as f64 * DAY_MS
        + h as f64 * 3_600_000.0
        + mi as f64 * 60_000.0
        + s as f64 * 1_000.0
}

fn dow_of(y: i64, m: i64, d: i64) -> i64 {
    (days_from_civil(y, m, d) + 4).rem_euclid(7)
}

fn fmt_iso(ms: f64) -> String {
    let days = (ms / DAY_MS).floor() as i64;
    let (y, m, d) = civil_from_days(days);
    let rem = (ms - days as f64 * DAY_MS) as i64;
    let h = rem / 3_600_000;
    let mi = rem % 3_600_000 / 60_000;
    let s = rem % 60_000 / 1_000;
    let msec = rem % 1_000;
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{msec:03}Z")
}

/// JS `Date.parse` for the ISO-ish forms the server produces.
fn parse_date_ms(raw: &str) -> Option<f64> {
    let s = raw.trim();
    // YYYY-MM-DD[T| ]HH:MM[:SS[.sss]][Z|±HH:MM]
    let bytes = s.as_bytes();
    if bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let rest = &s[10..];
    if rest.is_empty() {
        return Some(utc_ms(year, month, day, 0, 0, 0));
    }
    let rest = rest.strip_prefix(['T', ' '])?;
    let (time_part, offset_ms) = if let Some(t) = rest.strip_suffix('Z') {
        (t, 0.0)
    } else if let Some(i) = rest.rfind(['+', '-']) {
        let zone = &rest[i..];
        let sign = if zone.starts_with('-') { -1.0 } else { 1.0 };
        let mut zi = zone[1..].split(':');
        let zh: f64 = zi.next().unwrap_or("0").parse().ok()?;
        let zm: f64 = zi.next().unwrap_or("0").parse().ok()?;
        (&rest[..i], sign * (zh * 3_600_000.0 + zm * 60_000.0))
    } else {
        (rest, 0.0)
    };
    let mut tp = time_part.split(':');
    let h: i64 = tp.next()?.parse().ok()?;
    let mi: i64 = tp.next().unwrap_or("0").parse().ok()?;
    let sec_part = tp.next().unwrap_or("0");
    let (sec, frac) = match sec_part.split_once('.') {
        Some((a, b)) => (a.parse::<i64>().ok()?, {
            let mut f = b.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
            f.truncate(3);
            while f.len() < 3 {
                f.push('0');
            }
            f.parse::<f64>().unwrap_or(0.0)
        }),
        None => (sec_part.parse::<i64>().ok()?, 0.0),
    };
    if !(0..=12).contains(&month) || !(1..=31).contains(&day) || h > 24 || mi > 59 || sec > 60 {
        return None;
    }
    Some(utc_ms(year, month, day, h, mi, sec) + frac - offset_ms)
}

fn require_valid_date(value: &str, label: &str) -> Result<f64, String> {
    parse_date_ms(value).ok_or_else(|| format!("Invalid automation schedule {label}: {value}"))
}

// --- timezone via jiff (IANA) ------------------------------------------------

#[derive(Clone, Copy, Default)]
struct LocalParts {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
}

fn local_parts_for(ms: f64, timezone: &str) -> Result<LocalParts, String> {
    let ts = jiff::Timestamp::from_millisecond(ms as i64)
        .map_err(|_| format!("Invalid automation schedule timezone: {timezone}"))?;
    let z = ts
        .in_tz(timezone)
        .map_err(|_| format!("Invalid automation schedule timezone: {timezone}"))?;
    let dt = z.datetime();
    Ok(LocalParts {
        year: i64::from(dt.year()),
        month: i64::from(dt.month()),
        day: i64::from(dt.day()),
        hour: i64::from(dt.hour()),
        minute: i64::from(dt.minute()),
        second: i64::from(dt.second()),
    })
}

fn assert_valid_timezone(timezone: &str) -> Result<(), String> {
    jiff::Timestamp::UNIX_EPOCH
        .in_tz(timezone)
        .map(|_| ())
        .map_err(|_| format!("Invalid automation schedule timezone: {timezone}"))
}

fn timezone_offset_ms(ms: f64, timezone: &str) -> Result<f64, String> {
    let parts = local_parts_for(ms, timezone)?;
    Ok(utc_ms(
        parts.year,
        parts.month,
        parts.day,
        parts.hour,
        parts.minute,
        parts.second,
    ) - ms)
}

fn zoned_wall_clock_to_utc(
    timezone: &str,
    year: i64,
    month: i64,
    day: i64,
    time_of_day: &Value,
) -> Result<Option<f64>, String> {
    let (hours, minutes) = parse_time_of_day(time_of_day);
    let naive = utc_ms(year, month, day, hours, minutes, 0);
    let first = naive - timezone_offset_ms(naive, timezone)?;
    let second = naive - timezone_offset_ms(first, timezone)?;
    let rt = local_parts_for(second, timezone)?;
    Ok(
        if rt.year == year
            && rt.month == month
            && rt.day == day
            && rt.hour == hours
            && rt.minute == minutes
        {
            Some(second)
        } else {
            None
        },
    )
}

fn add_local_days(parts: (i64, i64, i64), days: i64) -> (i64, i64, i64) {
    civil_from_days(days_from_civil(parts.0, parts.1, parts.2) + days)
}

fn compute_next_zoned_wall_clock(
    schedule: &Value,
    ty: &str,
    from_ms: f64,
) -> Result<Option<f64>, String> {
    let timezone = prop(schedule, "timezone");
    let tod = prop(schedule, "timeOfDay");
    let (hours, minutes) = if tod.is_string() {
        parse_time_of_day(tod)
    } else {
        return Err("Cannot read properties of undefined (reading 'split')".into());
    };

    if !timezone.is_string() {
        // UTC path.
        let candidate_days = (from_ms / DAY_MS).floor() as i64;
        let mut candidate = utc_ms_from_parts_of(civil_from_days(candidate_days), hours, minutes);
        match ty {
            "daily" => {
                if candidate <= from_ms {
                    candidate += DAY_MS;
                }
                return Ok(Some(candidate));
            }
            "weekdays" => loop {
                let dow = dow_of_parts(candidate);
                if candidate > from_ms && dow != 0 && dow != 6 {
                    return Ok(Some(candidate));
                }
                candidate += DAY_MS;
            },
            "weekly" => {
                let target = js_num(prop(schedule, "dayOfWeek")) as i64;
                let mut days = (target - dow_of_parts(candidate) + 7) % 7;
                if days == 0 && candidate <= from_ms {
                    days = 7;
                }
                candidate += days as f64 * DAY_MS;
                if candidate <= from_ms {
                    candidate += 7.0 * DAY_MS;
                }
                return Ok(Some(candidate));
            }
            _ => return Ok(None),
        }
    }

    let tz = timezone.as_str().unwrap_or_default();
    let start = local_parts_for(from_ms, tz)?;
    for offset in 0..=370i64 {
        let ld = add_local_days((start.year, start.month, start.day), offset);
        let dow = dow_of(ld.0, ld.1, ld.2);
        if ty == "weekdays" && (dow == 0 || dow == 6) {
            continue;
        }
        if ty == "weekly" && dow != js_num(prop(schedule, "dayOfWeek")) as i64 {
            continue;
        }
        if let Some(candidate) = zoned_wall_clock_to_utc(tz, ld.0, ld.1, ld.2, tod)?
            && candidate > from_ms
        {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn utc_ms_from_parts_of(parts: (i64, i64, i64), h: i64, mi: i64) -> f64 {
    utc_ms(parts.0, parts.1, parts.2, h, mi, 0)
}

fn dow_of_parts(ms: f64) -> i64 {
    ((ms / DAY_MS).floor() as i64 + 4).rem_euclid(7)
}

// --- cron --------------------------------------------------------------------

struct CronField {
    values: std::collections::BTreeSet<i64>,
    is_wildcard: bool,
}

fn parse_cron_integer(raw: Option<&str>, name: &str, reason: &str) -> Result<i64, String> {
    match raw {
        Some(s) if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) => {
            Ok(s.parse::<i64>().unwrap_or(i64::MAX))
        }
        _ => Err(format!("Invalid cron {name}: {reason}")),
    }
}

fn parse_cron_dow_value(raw: Option<&str>, name: &str, reason: &str) -> Result<i64, String> {
    if let Some(s) = raw {
        let named = match s.to_ascii_lowercase().as_str() {
            "sun" => Some(0),
            "mon" => Some(1),
            "tue" => Some(2),
            "wed" => Some(3),
            "thu" => Some(4),
            "fri" => Some(5),
            "sat" => Some(6),
            _ => None,
        };
        if let Some(n) = named {
            return Ok(n);
        }
    }
    parse_cron_integer(raw, name, reason)
}

fn parse_cron_field(
    raw: &str,
    min: i64,
    max: i64,
    name: &str,
    dow_names: bool,
    normalize_dow: bool,
) -> Result<CronField, String> {
    let mut values = std::collections::BTreeSet::new();
    let mut is_wildcard = false;
    for token in raw.split(',') {
        let part = token.trim();
        if part.is_empty() {
            return Err(format!("Invalid cron {name}: empty token"));
        }
        let step_parts: Vec<&str> = part.split('/').collect();
        if step_parts.len() > 2 {
            return Err(format!("Invalid cron {name}: bad step"));
        }
        let range_part = step_parts[0];
        let step = if step_parts.len() == 2 {
            let s = parse_cron_integer(Some(step_parts[1]), name, "bad step")?;
            if s <= 0 {
                return Err(format!("Invalid cron {name}: bad step"));
            }
            s
        } else {
            1
        };
        if range_part == "*" && step_parts.len() == 1 {
            is_wildcard = true;
        }
        let range_parts: Vec<String> = if range_part == "*" {
            vec![min.to_string(), max.to_string()]
        } else {
            range_part.split('-').map(str::to_string).collect()
        };
        if range_parts.len() > 2 {
            return Err(format!("Invalid cron {name}: out of range"));
        }
        let parse_value = |r: &str| -> Result<i64, String> {
            if dow_names {
                parse_cron_dow_value(Some(r), name, "out of range")
            } else {
                parse_cron_integer(Some(r), name, "out of range")
            }
        };
        let start = parse_value(&range_parts[0])?;
        let end = if range_parts.len() == 2 {
            parse_value(&range_parts[1])?
        } else {
            start
        };
        if start < min || end > max || start > end {
            return Err(format!("Invalid cron {name}: out of range"));
        }
        let mut v = start;
        while v <= end {
            values.insert(if normalize_dow && v == 7 { 0 } else { v });
            v += step;
        }
    }
    Ok(CronField {
        values,
        is_wildcard,
    })
}

struct CronExpr {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
}

fn parse_cron_expression(expression: &str) -> Result<CronExpr, String> {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    if fields.len() != 5 {
        return Err("Cron schedules must use a constrained 5-field expression.".into());
    }
    Ok(CronExpr {
        minute: parse_cron_field(fields[0], 0, 59, "minute", false, false)?,
        hour: parse_cron_field(fields[1], 0, 23, "hour", false, false)?,
        day_of_month: parse_cron_field(fields[2], 1, 31, "day-of-month", false, false)?,
        month: parse_cron_field(fields[3], 1, 12, "month", false, false)?,
        day_of_week: parse_cron_field(fields[4], 0, 7, "day-of-week", true, true)?,
    })
}

fn cron_day_matches(cron: &CronExpr, y: i64, m: i64, d: i64) -> bool {
    let dom = cron.day_of_month.values.contains(&d);
    let dow = cron.day_of_week.values.contains(&dow_of(y, m, d));
    if cron.day_of_month.is_wildcard || cron.day_of_week.is_wildcard {
        dom && dow
    } else {
        dom || dow
    }
}

fn compute_next_cron_run_at(schedule: &Value, from_ms: f64) -> Result<Option<f64>, String> {
    let timezone = prop(schedule, "timezone");
    let tz = timezone.as_str().unwrap_or_default();
    assert_valid_timezone(tz)?;
    let expression = prop(schedule, "expression");
    let cron = parse_cron_expression(expression.as_str().unwrap_or_default())?;
    let start = local_parts_for(from_ms, tz)?;
    let tod = prop(schedule, "timeOfDay");

    for day_offset in 0..=MAX_CRON_SEARCH_DAYS {
        let ld = add_local_days((start.year, start.month, start.day), day_offset);
        if !cron.month.values.contains(&ld.1) || !cron_day_matches(&cron, ld.0, ld.1, ld.2) {
            continue;
        }
        for &hour in &cron.hour.values {
            if day_offset == 0 && hour < start.hour {
                continue;
            }
            for &minute in &cron.minute.values {
                if day_offset == 0 && hour == start.hour && minute <= start.minute {
                    continue;
                }
                let tod_val = json!(format!("{hour:02}:{minute:02}"));
                let tod_ref = if tod.is_string() { tod } else { &tod_val };
                if let Some(candidate) = zoned_wall_clock_to_utc(tz, ld.0, ld.1, ld.2, tod_ref)?
                    && candidate > from_ms
                {
                    return Ok(Some(candidate));
                }
            }
        }
    }
    Ok(None)
}

/// Port of `computeNextAutomationRunAt`.
pub fn compute_next_automation_run_at(
    schedule: serde_json::Value,
    from_iso: String,
    jitter_context: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    if schedule.is_null() {
        return Err("Cannot read properties of null (reading 'type')".into());
    }
    let ty = prop(&schedule, "type").as_str().unwrap_or("").to_string();
    if ty == "manual" || ty == "project-event" {
        return Ok(None);
    }
    let from = require_valid_date(&from_iso, "timestamp")?;

    if ty == "once" {
        let run_at = require_valid_date(&str_prop(&schedule, "runAt"), "runAt")?;
        return Ok(if run_at > from {
            Some(fmt_iso(run_at))
        } else {
            None
        });
    }
    if ty == "interval" {
        let every = js_num(prop(&schedule, "everySeconds"));
        return Ok(Some(fmt_iso(from + every * 1000.0)));
    }

    let jitter_ms = schedule_jitter_ms(&ty, jitter_context.as_ref())?;
    let base_from = from - jitter_ms;
    let next = if ty == "cron" {
        compute_next_cron_run_at(&schedule, base_from)?
    } else {
        compute_next_zoned_wall_clock(&schedule, &ty, base_from)?
    };
    Ok(next.map(|ms| fmt_iso(ms + jitter_ms)))
}

/// Port of `computeNextAutomationRunAtAfter`.
pub fn compute_next_automation_run_at_after(
    schedule: serde_json::Value,
    from_iso: String,
    not_before_iso: String,
    jitter_context: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    if schedule.is_null() {
        return Err("Cannot read properties of null (reading 'type')".into());
    }
    let ty = prop(&schedule, "type").as_str().unwrap_or("").to_string();
    if ty == "manual" || ty == "project-event" || ty == "once" {
        return Ok(None);
    }
    if ty == "interval" {
        let from = require_valid_date(&from_iso, "timestamp")?;
        let not_before = parse_date_ms(&not_before_iso).unwrap_or(from);
        let step_ms = js_num(prop(&schedule, "everySeconds")) * 1000.0;
        let mut next = from + step_ms;
        if next <= not_before {
            let missed = ((not_before - next + 1.0) / step_ms).ceil();
            next += missed * step_ms;
        }
        return Ok(Some(fmt_iso(next)));
    }
    compute_next_automation_run_at(schedule, not_before_iso, jitter_context)
}

/// Port of `computeAutomationScheduleSpacingSeconds`.
pub fn compute_automation_schedule_spacing_seconds(
    schedule: serde_json::Value,
    from_iso: String,
) -> Result<Option<f64>, String> {
    if schedule.is_null() {
        return Err("Cannot read properties of null (reading 'type')".into());
    }
    let ty = prop(&schedule, "type").as_str().unwrap_or("").to_string();
    if ty == "manual" || ty == "once" || ty == "project-event" {
        return Ok(None);
    }
    let Some(first) = compute_next_automation_run_at(schedule.clone(), from_iso, None)? else {
        return Ok(None);
    };
    let Some(second) =
        compute_next_automation_run_at_after(schedule, first.clone(), first.clone(), None)?
    else {
        return Ok(None);
    };
    let spacing =
        parse_date_ms(&second).unwrap_or(f64::NAN) - parse_date_ms(&first).unwrap_or(f64::NAN);
    if spacing.is_finite() && spacing > 0.0 {
        Ok(Some(spacing / 1000.0))
    } else {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// stopPolicy.ts
// ---------------------------------------------------------------------------

const DEFAULT_STOP_AFTER_CONSECUTIVE_FAILURES: f64 = 3.0;

/// Port of `resolveAutomationStopPolicy`. Returns the raw value so that
/// `undefined` (marker), `null` and numbers stay distinct on the wire.
pub fn resolve_automation_stop_policy(
    input: serde_json::Value,
    fallback: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err(
            "Cannot read properties of null (reading 'stopAfterConsecutiveFailures')".into(),
        );
    }
    if is_undef(&input) {
        return Err(
            "Cannot read properties of undefined (reading 'stopAfterConsecutiveFailures')".into(),
        );
    }
    let stop_after = prop(&input, "stopAfterConsecutiveFailures");
    if !is_undef(stop_after) {
        return Ok(stop_after.clone());
    }
    let stop_on_error = prop(&input, "stopOnError");
    if !is_undef(stop_on_error) {
        let truthy = match stop_on_error {
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64().is_none_or(|x| x != 0.0 && !x.is_nan()),
            Value::String(s) => !s.is_empty(),
            Value::Null => false,
            Value::Array(_) => true,
            Value::Object(m) => match m.get("$num").and_then(Value::as_str) {
                Some(s) => s.parse::<f64>().map_or(true, |x| x != 0.0 && !x.is_nan()),
                None => true,
            },
        };
        return Ok(if truthy {
            json!(DEFAULT_STOP_AFTER_CONSECUTIVE_FAILURES)
        } else {
            Value::Null
        });
    }
    Ok(fallback)
}
