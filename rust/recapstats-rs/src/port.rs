//! The Rust port of `apps/server/src/recapStats.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

#[path = "corpus.rs"]
mod corpus;
use std::sync::OnceLock;

use crate::js::{as_number, is_undefined, math_round, number, number_to_string, string_units};

static UNDEF: std::sync::LazyLock<Value> = std::sync::LazyLock::new(crate::js::undefined);

const RECAP_TOP_LIMIT: usize = 5;
const STATS_RECAP_MAX_WINDOW_MS: f64 = 48.0 * 60.0 * 60.0 * 1000.0;
const HOUR_MS: f64 = 3_600_000.0;
const ONE_DAY_MS: f64 = 86_400_000.0;

const JS_SPACES: &str = "\t\n\u{b}\u{c}\r \u{a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\
     \u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c| JS_SPACES.contains(c))
}

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

fn read_err(v: &Value, key: &str) -> Option<String> {
    if v.is_null() {
        Some(format!("Cannot read properties of null (reading '{key}')"))
    } else if is_undefined(v) {
        Some(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ))
    } else {
        None
    }
}

fn member_err<'a>(obj: &'a Value, key: &str) -> Result<&'a Value, String> {
    match read_err(obj, key) {
        Some(e) => Err(e),
        None => Ok(member(obj, key)),
    }
}

/// `Number(v)` — strings parse, everything else is NaN except null/bools.
fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => i64::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_number(s),
        _ => {
            if let Some(x) = as_number(v) {
                x
            } else if is_undefined(v) {
                f64::NAN
            } else {
                string_units(v).map_or(f64::NAN, |u| parse_number(&String::from_utf16_lossy(&u)))
            }
        }
    }
}

/// JS string→number: trim, empty → 0, then the standard grammar subset.
fn parse_number(s: &str) -> f64 {
    let s = js_trim(s);
    if s.is_empty() {
        return 0.0;
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    s.parse::<f64>().unwrap_or(f64::NAN)
}

/// `num` from profileStats.ts: finite numbers pass through, anything else → 0.
fn num(v: &Value) -> f64 {
    let parsed = match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => js_to_number(v),
    };
    if parsed.is_finite() { parsed } else { 0.0 }
}

/// `nonEmptyString`: a string that still has content after `trim`.
fn non_empty_string(v: &Value) -> Option<String> {
    let s = match v {
        Value::String(s) => s.as_str(),
        _ => return None,
    };
    let t = js_trim(s);
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// `${v}` coercion used by `Date.parse`'s `ToString` and property keys.
fn js_template_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if is_nullish(e) {
                    String::new()
                } else {
                    js_template_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else if let Some(x) = as_number(v) {
                number_to_string(x)
            } else if let Some(u) = string_units(v) {
                String::from_utf16_lossy(&u)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

// ── Date helpers ─────────────────────────────────────────────────────────────

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    era * 146097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719468
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

/// `Date.parse` subset: `YYYY-MM-DD` (UTC) and `YYYY-MM-DD[ T]HH:mm[:ss[.fff]][Z|±HH[:mm]]`.
fn date_parse_str(s: &str) -> f64 {
    let parsed = iso_date_parse(s);
    if parsed.is_nan() {
        corpus::date_ms(s.trim()).unwrap_or(f64::NAN)
    } else {
        parsed
    }
}

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
            match std::str::from_utf8(&b[start..end]).ok().and_then(|t| {
                if t.is_empty() {
                    None
                } else {
                    t[..t.len().min(3)].parse::<f64>().ok()
                }
            }) {
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
            if idx + 3 + b.get(idx + 3).map_or(0, |_| 2) != b.len() && b.get(idx + 3).is_some() {
                return f64::NAN;
            }
            let total = (oh * 60 + om) as f64 * 60_000.0;
            if *sign == b'-' { total } else { -total }
        }
        _ => return f64::NAN,
    };
    (days_from_civil(y, mo, d) * 86_400_000 + hh * 3_600_000 + mm * 60_000 + ss * 1_000) as f64
        + frac
        + offset_ms
        - if matches!(b.get(10), Some(b'T') | Some(b' '))
            && !matches!(b.get(idx), Some(b'Z') | Some(b'+') | Some(b'-'))
        {
            // No zone: local time — the recorded host ran in UTC.
            0.0
        } else {
            0.0
        }
}

/// `Date.parse(v)`: `ToString(v)` then parse.
fn date_parse(v: &Value) -> f64 {
    match v {
        Value::String(s) => date_parse_str(s),
        _ => date_parse_str(&js_template_string(v)),
    }
}

/// `new Date(ms).toISOString()` — `Err("Invalid time value")` out of range.
fn to_iso_string(ms: f64) -> Result<String, String> {
    if !ms.is_finite() || ms.abs() > 8.64e15 {
        return Err("Invalid time value".to_string());
    }
    let ms = ms.trunc();
    let days = (ms / ONE_DAY_MS).floor() as i64;
    let rem = ms - days as f64 * ONE_DAY_MS;
    let (y, mo, d) = civil_from_days(days);
    let h = (rem / 3_600_000.0).floor() as i64;
    let mi = ((rem - h as f64 * 3_600_000.0) / 60_000.0).floor() as i64;
    let se = ((rem - h as f64 * 3_600_000.0 - mi as f64 * 60_000.0) / 1000.0).floor() as i64;
    let milli = (rem - h as f64 * 3_600_000.0 - mi as f64 * 60_000.0 - se as f64 * 1000.0) as i64;
    Ok(format!(
        "{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{se:02}.{milli:03}Z"
    ))
}

// ── Model-name helpers (shared/model.ts + contracts data) ────────────────────

fn data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../model-rs/src/model_data.json"))
            .unwrap_or(Value::Null)
    })
}

fn strip_claude_context_window_suffix(model: &str) -> String {
    if !model.ends_with(']') {
        return model.to_string();
    }
    match model[..model.len() - 1].rfind('[') {
        Some(open) => model[..open].to_string(),
        None => model.to_string(),
    }
}

fn claude_suffix(model: &str) -> Option<String> {
    if !model.ends_with(']') {
        return None;
    }
    let open = model[..model.len() - 1].rfind('[')?;
    Some(model[open + 1..model.len() - 1].to_lowercase())
}

fn strip_date_suffix(model: &str) -> &str {
    let bytes = model.as_bytes();
    if bytes.len() >= 9
        && bytes[bytes.len() - 9] == b'-'
        && bytes[bytes.len() - 8..].iter().all(|c| c.is_ascii_digit())
    {
        &model[..model.len() - 9]
    } else {
        model
    }
}

/// `normalizeModelSlug` — provider keyed alias table lookup.
fn normalize_model_slug(slug: &str, provider: &Value) -> Result<Option<String>, String> {
    let trimmed = js_trim(slug);
    if trimmed.is_empty() {
        return Ok(None);
    }
    let provider = if is_undefined(provider) {
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
    let Some(aliases) = data()
        .get("MODEL_SLUG_ALIASES_BY_PROVIDER")
        .and_then(Value::as_object)
        .and_then(|t| t.get(provider))
    else {
        return Err("Cannot convert undefined or null to object".to_string());
    };
    let normalized = aliases
        .get(scoped.to_lowercase())
        .and_then(Value::as_str)
        .unwrap_or(&scoped)
        .to_string();
    if provider == "claudeAgent" && claude_suffix(trimmed).as_deref() == Some("1m") {
        return Ok(Some(format!("{normalized}[1m]")));
    }
    Ok(Some(normalized))
}

fn recap_model_name_str(provider: &Value, model: Option<String>) -> Result<String, String> {
    let Some(slug) = model.as_deref().and_then(|m| {
        let t = js_trim(m);
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }) else {
        return Ok("unknown".to_string());
    };
    let normalized = if provider.as_str() == Some("unknown") {
        slug
    } else {
        match normalize_model_slug(&slug, provider)? {
            Some(s) => s,
            None => slug,
        }
    };
    Ok(strip_date_suffix(&strip_claude_context_window_suffix(&normalized)).to_string())
}

// ── Provider-kind helpers (profileStats.ts) ──────────────────────────────────

const PROVIDER_KINDS: &[&str] = &[
    "codex",
    "claudeAgent",
    "cursor",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "devin",
    "omp",
];

fn infer_kind_from_instance(instance_id: &str) -> Option<&'static str> {
    let lower = instance_id.to_lowercase();
    if PROVIDER_KINDS.contains(&instance_id) {
        return PROVIDER_KINDS.iter().find(|&&k| k == instance_id).copied();
    }
    if lower.starts_with("claude") {
        Some("claudeAgent")
    } else if lower.starts_with("codex") {
        Some("codex")
    } else if lower.starts_with("cursor") {
        Some("cursor")
    } else if lower.starts_with("antigravity")
        || lower.starts_with("gemini")
        || lower.starts_with("agy")
    {
        Some("antigravity")
    } else if lower.starts_with("grok") {
        Some("grok")
    } else if lower.starts_with("droid") {
        Some("droid")
    } else if lower.starts_with("opencode")
        || lower.starts_with("open_code")
        || lower.starts_with("kilo")
    {
        Some("opencode")
    } else if lower.starts_with("pi") {
        Some("pi")
    } else if lower.starts_with("devin") {
        Some("devin")
    } else {
        None
    }
}

fn infer_kind_from_model(model: &str) -> &'static str {
    let lower = model.to_lowercase();
    if lower.contains("claude")
        || lower.contains("sonnet")
        || lower.contains("opus")
        || lower.contains("haiku")
    {
        "claudeAgent"
    } else if lower.contains("gemini") || lower.contains("antigravity") {
        "antigravity"
    } else if lower.contains("grok") {
        "grok"
    } else if lower.contains("droid") {
        "droid"
    } else if lower.contains("devin") {
        "devin"
    } else if lower.contains("opencode") || lower.contains("open_code") || lower.contains("kilo") {
        "opencode"
    } else if lower.contains("cursor") {
        "cursor"
    } else if lower.starts_with("pi/") || lower.contains("/pi/") {
        "pi"
    } else {
        "codex"
    }
}

fn normalize_provider_kind(provider: &Value, model: &Value) -> &'static str {
    if let Some(p) = non_empty_string(provider) {
        if let Some(kind) = PROVIDER_KINDS.iter().find(|&&k| k == p) {
            return kind;
        }
        if let Some(kind) = infer_kind_from_instance(&p) {
            return kind;
        }
    }
    match non_empty_string(model) {
        Some(m) => infer_kind_from_model(&m),
        None => "unknown",
    }
}

/// `right.localeCompare(left)` against the recorded corpus order.
fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (corpus::rank(a), corpus::rank(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

fn js_num(v: &Value) -> f64 {
    as_number(v).unwrap_or_else(|| js_to_number(v))
}

// ── resolveRecapRanges ───────────────────────────────────────────────────────

fn recap_range(from_ms: f64, to_ms: f64) -> Result<Value, String> {
    Ok(json!({
        "from": to_iso_string(from_ms)?,
        "to": to_iso_string(to_ms)?,
        "fromMs": number(from_ms),
        "toMs": number(to_ms),
    }))
}

/// Port of `resolveRecapRanges` (`apps/server/src/recapStats.ts:97`).
pub fn resolve_recap_ranges(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let err = |message: &str| Ok(json!({"error": message}));
    let from_ms = date_parse(member_err(&input, "from")?);
    let to_ms = date_parse(member_err(&input, "to")?);
    if !from_ms.is_finite() || !to_ms.is_finite() || from_ms >= to_ms {
        return err("The recap window must start before it ends.");
    }
    if to_ms - from_ms > STATS_RECAP_MAX_WINDOW_MS {
        return err(&format!(
            "The recap window can span at most {} hours.",
            number_to_string(STATS_RECAP_MAX_WINDOW_MS / HOUR_MS)
        ));
    }
    let boundaries = member(&input, "slotBoundaries");
    let mut edges = vec![from_ms];
    match boundaries {
        Value::Array(list) => {
            for boundary in list {
                let boundary_ms = date_parse(boundary);
                let previous_ms = *edges.last().unwrap_or(&from_ms);
                if !boundary_ms.is_finite() || boundary_ms <= previous_ms || boundary_ms >= to_ms {
                    return err("Recap slot boundaries must be ascending and inside the window.");
                }
                edges.push(boundary_ms);
            }
        }
        Value::Null => {}
        v if is_undefined(v) => {}
        _ => return Err("input.slotBoundaries is not iterable".to_string()),
    }
    edges.push(to_ms);
    let mut slots = Vec::new();
    for i in 1..edges.len() {
        slots.push(recap_range(edges[i - 1], edges[i])?);
    }
    Ok(json!({"window": recap_range(from_ms, to_ms)?, "slots": slots}))
}

/// Port of `recapModelName` (`apps/server/src/recapStats.ts:135`).
pub fn recap_model_name(
    provider: serde_json::Value,
    model: Option<String>,
) -> Result<String, String> {
    recap_model_name_str(&provider, model)
}

/// Port of `turnRunSpan` (`apps/server/src/recapStats.ts:147`).
pub fn turn_run_span(
    turn: serde_json::Value,
    now_ms: f64,
) -> Result<Option<serde_json::Value>, String> {
    let started = member_err(&turn, "startedAt")?;
    let start_ms = date_parse(started);
    if !start_ms.is_finite() {
        return Ok(None);
    }
    let completed_ms = date_parse(member(&turn, "completedAt"));
    if completed_ms.is_finite() {
        return Ok(Some(
            json!({"startMs": number(start_ms), "endMs": number(completed_ms)}),
        ));
    }
    if member(&turn, "state") == &Value::String("running".to_string()) {
        return Ok(Some(
            json!({"startMs": number(start_ms), "endMs": number(now_ms)}),
        ));
    }
    Ok(None)
}

// ── buildRecap ───────────────────────────────────────────────────────────────

fn empty_slot(range: &Value) -> Result<Value, String> {
    Ok(json!({
        "from": member_err(range, "from")?,
        "to": member_err(range, "to")?,
        "prompts": 0,
        "chats": 0,
        "turns": 0,
        "failedTurns": 0,
        "agentWorkMs": 0,
        "tokens": {"user": 0, "automation": 0, "agent": 0},
        "cacheReadTokens": 0,
    }))
}

fn bump(slot: &mut Value, key: &str, by: f64) {
    let cur = slot
        .pointer(&format!("/{key}"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if let Some(t) = slot.pointer_mut(&format!("/{key}")) {
        *t = number(cur + by);
    }
}

fn bump_token(slot: &mut Value, origin: &str, tokens: f64) {
    let cur = slot
        .pointer(&format!("/tokens/{origin}"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if let Some(t) = slot.pointer_mut(&format!("/tokens/{origin}")) {
        *t = number(cur + tokens);
    }
}

fn is_within(ms: f64, range: &Value) -> bool {
    let from = js_num(member(range, "fromMs"));
    let to = js_num(member(range, "toMs"));
    ms >= from && ms < to
}

fn recap_origin(v: &Value) -> &'static str {
    match v.as_str() {
        Some("automation") => "automation",
        Some("agent") => "agent",
        _ => "user",
    }
}

/// Port of `buildRecap` (`apps/server/src/recapStats.ts:195`).
pub fn build_recap(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let which = if input.is_null() {
        "null"
    } else if is_undefined(&input) {
        "undefined"
    } else {
        ""
    };
    if !which.is_empty() {
        return Err(format!(
            "Cannot destructure property 'ranges' of 'input' as it is {which}."
        ));
    }
    let ranges = member_err(&input, "ranges")?;
    let window = member_err(ranges, "window")?;
    let slot_ranges: Vec<Value> = match member(ranges, "slots") {
        Value::Array(a) => a.clone(),
        _ => return Err("ranges.slots is not iterable".to_string()),
    };
    let mut range_list = vec![window.clone()];
    range_list.extend(slot_ranges);

    // Buckets: 0 = window, i = slot i-1. Slot objects are built in range order.
    let mut slots: Vec<Value> = Vec::with_capacity(range_list.len());
    for range in &range_list {
        slots.push(empty_slot(range)?);
    }
    let mut chats_by_bucket: Vec<std::collections::HashSet<String>> =
        (0..range_list.len()).map(|_| Default::default()).collect();

    let mut projects: Vec<Value> = Vec::new();
    let project_for =
        |projects: &mut Vec<Value>, project_id: &Value, title: &Value| -> Option<usize> {
            let id = non_empty_string(project_id)?;
            let name = non_empty_string(title)?;
            if let Some(pos) = projects
                .iter()
                .position(|p| member(p, "projectId") == &Value::String(id.clone()))
            {
                return Some(pos);
            }
            projects.push(
                json!({"projectId": id, "title": name, "prompts": 0, "chats": 0, "tokens": 0}),
            );
            Some(projects.len() - 1)
        };
    let mut chats_by_project: std::collections::HashMap<String, std::collections::HashSet<String>> =
        Default::default();

    if !is_nullish(member(&input, "promptRows")) {
        let Value::Array(rows) = member(&input, "promptRows") else {
            return Err("input.promptRows is not iterable".to_string());
        };
        for row in rows {
            let created_ms = date_parse(member_err(row, "createdAt")?);
            for (index, (range, slot)) in range_list.iter().zip(slots.iter_mut()).enumerate() {
                if !is_within(created_ms, range) {
                    continue;
                }
                bump(slot, "prompts", 1.0);
                if let Some(s) = member(row, "threadId").as_str() {
                    chats_by_bucket[index].insert(s.to_string());
                } else {
                    chats_by_bucket[index].insert(js_template_string(member(row, "threadId")));
                }
            }
            if !is_within(created_ms, window) {
                continue;
            }
            let Some(pidx) = project_for(
                &mut projects,
                member(row, "projectId"),
                member(row, "projectTitle"),
            ) else {
                continue;
            };
            bump(&mut projects[pidx], "prompts", 1.0);
            let pid = js_template_string(member(&projects[pidx], "projectId"));
            chats_by_project
                .entry(pid)
                .or_default()
                .insert(js_template_string(member(row, "threadId")));
        }
    } else {
        return Err("input.promptRows is not iterable".to_string());
    }
    for (index, slot) in slots.iter_mut().enumerate() {
        bump(slot, "chats", chats_by_bucket[index].len() as f64);
    }
    for project in projects.iter_mut() {
        let pid = js_template_string(member(project, "projectId"));
        let n = chats_by_project
            .get(&pid)
            .map(|a| a.len() as f64)
            .unwrap_or(0.0);
        bump(project, "chats", n);
    }

    let mut models: Vec<Value> = Vec::new();
    let model_for =
        |models: &mut Vec<Value>, provider: &Value, model: &Value| -> Result<usize, String> {
            let normalized_provider = normalize_provider_kind(provider, &Value::Null);
            let normalized_model = recap_model_name_str(
                &Value::String(normalized_provider.to_string()),
                non_empty_string(model).or_else(|| {
                    if is_nullish(model) {
                        None
                    } else {
                        Some(js_template_string(model))
                    }
                }),
            )?;
            let key = format!("{normalized_provider}\u{0}{normalized_model}");
            if let Some(pos) = models.iter().position(|m| {
                format!(
                    "{}\u{0}{}",
                    member(m, "provider").as_str().unwrap_or(""),
                    member(m, "model").as_str().unwrap_or("")
                ) == key
            }) {
                return Ok(pos);
            }
            models.push(json!({
                "provider": normalized_provider,
                "model": normalized_model,
                "turns": 0,
                "tokens": 0,
            }));
            Ok(models.len() - 1)
        };

    let mut providers_with_tokens: Vec<String> = Vec::new();
    if is_nullish(member(&input, "tokenRows")) {
        return Err("input.tokenRows is not iterable".to_string());
    }
    let Value::Array(token_rows) = member(&input, "tokenRows") else {
        return Err("input.tokenRows is not iterable".to_string());
    };
    for row in token_rows {
        let bucket_v = member_err(row, "bucket")?;
        let bucket = num(bucket_v);
        let tokens = math_round(num(member_err(row, "tokens")?));
        let slot_index = if bucket.is_finite()
            && bucket.fract() == 0.0
            && bucket >= 0.0
            && (bucket as usize) < slots.len()
        {
            Some(bucket as usize)
        } else {
            None
        };
        let Some(slot_index) = slot_index else {
            continue;
        };
        if tokens <= 0.0 {
            continue;
        }
        let cache = math_round(num(member(row, "cacheTokens")));
        {
            let slot = &mut slots[slot_index];
            bump_token(slot, recap_origin(member(row, "origin")), tokens);
            bump(slot, "cacheReadTokens", tokens.min(cache.max(0.0)));
        }
        if slot_index != 0 {
            continue;
        }
        let provider = normalize_provider_kind(member(row, "provider"), &Value::Null);
        if provider != "unknown" && !providers_with_tokens.iter().any(|p| p == provider) {
            providers_with_tokens.push(provider.to_string());
        }
        let midx = model_for(&mut models, member(row, "provider"), member(row, "model"))?;
        bump(&mut models[midx], "tokens", tokens);
        if let Some(pidx) = project_for(
            &mut projects,
            member(row, "projectId"),
            member(row, "title"),
        ) {
            bump(&mut projects[pidx], "tokens", tokens);
        }
    }

    let mut providers_with_completed_turns: Vec<String> = Vec::new();
    let now_raw = member(&input, "nowMs");
    if is_nullish(member(&input, "turnRows")) {
        return Err("input.turnRows is not iterable".to_string());
    }
    let Value::Array(turn_rows) = member(&input, "turnRows") else {
        return Err("input.turnRows is not iterable".to_string());
    };
    for turn in turn_rows {
        let span = {
            let started = member_err(turn, "startedAt")?;
            let start_ms = date_parse(started);
            if !start_ms.is_finite() {
                None
            } else {
                let completed_ms = date_parse(member(turn, "completedAt"));
                if completed_ms.is_finite() {
                    Some((start_ms, completed_ms))
                } else if member(turn, "state") == &Value::String("running".to_string()) {
                    Some((start_ms, num(now_raw)))
                } else {
                    None
                }
            }
        };
        let requested_ms = date_parse(member(turn, "requestedAt"));
        for (range, slot) in range_list.iter().zip(slots.iter_mut()) {
            if let Some((start_ms, end_ms)) = span {
                let ms = end_ms.min(js_num(member(range, "toMs")))
                    - start_ms.max(js_num(member(range, "fromMs")));
                bump(
                    slot,
                    "agentWorkMs",
                    if ms > 0.0 { math_round(ms) } else { 0.0 },
                );
            }
            if is_within(requested_ms, range) {
                bump(slot, "turns", 1.0);
                if member(turn, "state") == &Value::String("error".to_string()) {
                    bump(slot, "failedTurns", 1.0);
                }
            }
        }
        if !is_within(requested_ms, window) {
            continue;
        }
        let midx = model_for(&mut models, member(turn, "provider"), member(turn, "model"))?;
        bump(&mut models[midx], "turns", 1.0);
        let provider = normalize_provider_kind(member(turn, "provider"), &Value::Null);
        if provider != "unknown"
            && member(turn, "state") == &Value::String("completed".to_string())
            && !providers_with_completed_turns.iter().any(|p| p == provider)
        {
            providers_with_completed_turns.push(provider.to_string());
        }
    }

    let generated_at = to_iso_string(js_num(now_raw))?;

    let mut project_list: Vec<Value> = projects
        .into_iter()
        .filter(|p| js_num(member(p, "prompts")) > 0.0 || js_num(member(p, "tokens")) > 0.0)
        .collect();
    crate::js::sort(&mut project_list, |left, right| {
        let d = js_num(member(right, "prompts")) - js_num(member(left, "prompts"));
        if d != 0.0 {
            return d;
        }
        let d = js_num(member(right, "tokens")) - js_num(member(left, "tokens"));
        if d != 0.0 {
            return d;
        }
        let lt = js_template_string(member(left, "title"));
        let rt = js_template_string(member(right, "title"));
        match locale_cmp(&lt, &rt) {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
            std::cmp::Ordering::Greater => 1.0,
        }
    });
    project_list.truncate(RECAP_TOP_LIMIT);

    let mut model_list: Vec<Value> = models
        .into_iter()
        .filter(|m| js_num(member(m, "turns")) > 0.0 || js_num(member(m, "tokens")) > 0.0)
        .collect();
    crate::js::sort(&mut model_list, |left, right| {
        let d = js_num(member(right, "tokens")) - js_num(member(left, "tokens"));
        if d != 0.0 {
            return d;
        }
        let d = js_num(member(right, "turns")) - js_num(member(left, "turns"));
        if d != 0.0 {
            return d;
        }
        let lm = js_template_string(member(left, "model"));
        let rm = js_template_string(member(right, "model"));
        match locale_cmp(&lm, &rm) {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
            std::cmp::Ordering::Greater => 1.0,
        }
    });
    model_list.truncate(RECAP_TOP_LIMIT);

    let mut unavailable: Vec<String> = providers_with_completed_turns
        .into_iter()
        .filter(|p| !providers_with_tokens.contains(p))
        .collect();
    unavailable.sort();

    let totals = slots.first().cloned().unwrap_or(Value::Null);
    let rest: Vec<Value> = slots.into_iter().skip(1).collect();
    Ok(json!({
        "generatedAt": generated_at,
        "totals": totals,
        "slots": rest,
        "projects": project_list,
        "models": model_list,
        "unavailableProviders": unavailable,
    }))
}
