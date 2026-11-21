//! The Rust port of `packages/shared/src/hostUsage.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

mod tables;

use crate::js;

const MAX_COUNTER: f64 = 1_000_000_000_000.0;
const MAX_ROWS: usize = 100;
const MAX_TURNS: f64 = 5_000.0;
const RECEIPT_DAYS: f64 = 14.0;
const DAY_MS: f64 = 86_400_000.0;
const PROVIDERS: &[&str] = &[
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
const EFFORTS: &[&str] = &[
    "default",
    "none",
    "off",
    "minimal",
    "low",
    "medium",
    "high",
    "xhigh",
    "max",
    "ultrathink",
    "ultracode",
];
const INTERACTION_MODES: &[&str] = &["default", "plan", "debug"];
const RUNTIME_MODES: &[&str] = &["approval-required", "auto", "full-access"];
const ORIGINS: &[&str] = &["user", "automation", "agent"];
const COUNT_FIELDS: &[&str] = &[
    "turns",
    "durationMs",
    "durationSamples",
    "inputTokens",
    "inputSamples",
    "outputTokens",
    "outputSamples",
    "cachedInputTokens",
    "cachedInputSamples",
];
const QUALITY_FIELDS: &[&str] = &[
    "eligibleTurns",
    "includedTurns",
    "missingCompletionTimes",
    "tokenTruncatedTurns",
    "cappedMeasurements",
];

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'
            | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

fn is_record(v: &Value) -> bool {
    matches!(v, Value::Object(_))
}

fn to_num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Null => 0.0,
        Value::String(s) => s.trim_matches(is_js_space).parse().unwrap_or(f64::NAN),
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => to_num(&a[0]),
            _ => f64::NAN,
        },
        _ => f64::NAN,
    }
}

fn is_count(v: &Value) -> bool {
    match v {
        Value::Number(n) => n
            .as_f64()
            .is_some_and(|f| f.fract() == 0.0 && (0.0..=MAX_COUNTER).contains(&f)),
        _ => false,
    }
}

/// `/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/` + finite parse + ISO round-trip.
fn timestamp(v: &Value) -> bool {
    let Some(s) = v.as_str() else {
        return false;
    };
    if s.len() != 24
        || !s[..4].bytes().all(|b| b.is_ascii_digit())
        || s.as_bytes()[4] != b'-'
        || !s[5..7].bytes().all(|b| b.is_ascii_digit())
        || s.as_bytes()[7] != b'-'
        || !s[8..10].bytes().all(|b| b.is_ascii_digit())
        || s.as_bytes()[10] != b'T'
        || !s[11..13].bytes().all(|b| b.is_ascii_digit())
        || s.as_bytes()[13] != b':'
        || !s[14..16].bytes().all(|b| b.is_ascii_digit())
        || s.as_bytes()[16] != b':'
        || !s[17..19].bytes().all(|b| b.is_ascii_digit())
        || s.as_bytes()[19] != b'.'
        || !s[20..23].bytes().all(|b| b.is_ascii_digit())
        || s.as_bytes()[23] != b'Z'
    {
        return false;
    }
    let Some(ms) = parse_iso_ms(s) else {
        return false;
    };
    ms_to_iso(ms).as_deref() == Some(s)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Strict ISO `YYYY-MM-DDTHH:mm:ss.sssZ`: no day overflow (round-trip rejects it).
pub fn parse_iso_ms(s: &str) -> Option<f64> {
    if s.len() != 24 {
        return None;
    }
    let num = |a: usize, b: usize| s[a..b].parse::<i64>().ok();
    let y = num(0, 4)?;
    let mo = num(5, 7)?;
    let d = num(8, 10)?;
    let h = num(11, 13)?;
    let mi = num(14, 16)?;
    let sec = num(17, 19)?;
    let ms = num(20, 23)?;
    if !(1..=12).contains(&mo)
        || d < 1
        || d > days_in_month(y, mo)
        || h > 23
        || mi > 59
        || sec > 59
        || !(0..=9999).contains(&y)
    {
        return None;
    }
    Some(
        (days_from_civil(y, mo, d) * DAY_MS as i64 + h * 3_600_000 + mi * 60_000 + sec * 1_000 + ms)
            as f64,
    )
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn ms_to_iso(ms: f64) -> Option<String> {
    if !ms.is_finite() || ms.abs() > 8.64e15 {
        return None;
    }
    let ms_i = ms as i64;
    let days = ms_i.div_euclid(86_400_000);
    let rem = ms_i.rem_euclid(86_400_000);
    let (y, mo, d) = civil_from_days(days);
    if !(0..=9999).contains(&y) {
        return None;
    }
    let h = rem / 3_600_000;
    let mi = rem % 3_600_000 / 60_000;
    let sec = rem % 60_000 / 1_000;
    let msec = rem % 1_000;
    Some(format!(
        "{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{sec:02}.{msec:03}Z"
    ))
}

/// `/\[([^\]]+)\]$/` — trailing `[...]` group.
fn claude_suffix(s: &str) -> Option<String> {
    if !s.ends_with(']') {
        return None;
    }
    let start = s.rfind('[')?;
    let inner = &s[start + 1..s.len() - 1];
    if inner.contains(']') {
        return None;
    }
    Some(inner.to_lowercase())
}

fn strip_claude_suffix(s: &str) -> &str {
    match claude_suffix(s) {
        Some(_) => &s[..s.rfind('[').unwrap_or(s.len())],
        None => s,
    }
}

fn normalize_model_slug(model: &Value, provider: &str) -> Option<String> {
    let Value::String(model) = model else {
        return None;
    };
    let trimmed = model.trim_matches(is_js_space);
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    let scoped = match provider {
        "claudeAgent" => strip_claude_suffix(trimmed).to_string(),
        "devin" if trimmed == lower && trimmed.ends_with("-medium") => {
            trimmed[..trimmed.len() - "-medium".len()].to_string()
        }
        _ => trimmed.to_string(),
    };
    let alias_key = scoped.to_lowercase();
    let aliases = tables::MODEL_ALIASES
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, a)| *a)
        .unwrap_or(&[]);
    let aliased = aliases
        .iter()
        .find(|(k, _)| *k == alias_key)
        .map(|(_, v)| *v);
    let normalized = aliased.unwrap_or(&scoped).to_string();
    if provider == "claudeAgent" && claude_suffix(trimmed).as_deref() == Some("1m") {
        Some(format!("{normalized}[1m]"))
    } else {
        Some(normalized)
    }
}

fn category<'a>(v: &'a Value, allowed: &[&str]) -> &'a str {
    match v {
        Value::String(s) if allowed.contains(&s.as_str()) => s.as_str(),
        _ => "other",
    }
}

fn usage_model_category(provider: &str, model: &Value) -> String {
    if !model.is_string() || !PROVIDERS.contains(&provider) {
        return "other".into();
    }
    let Some(normalized) = normalize_model_slug(model, provider) else {
        return "other".into();
    };
    let slug = strip_claude_suffix(&normalized).to_lowercase();
    let models = tables::MODEL_SLUGS
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, m)| *m)
        .unwrap_or(&[]);
    if models.contains(&slug.as_str()) {
        slug
    } else {
        "other".into()
    }
}

fn num_field(m: &serde_json::Map<String, Value>, k: &str) -> f64 {
    m.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

fn now_ms(now: &Value) -> f64 {
    if js::is_undefined(now) {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(f64::NAN)
    } else {
        to_num(now)
    }
}

/// Port of `normalizeHostUsageWindow` (`packages/shared/src/hostUsage.ts:50`).
pub fn normalize_host_usage_window(
    raw: serde_json::Value,
    now: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let now = match now.as_ref() {
        None => now_ms(&js::undefined()),
        Some(v) => now_ms(v),
    };
    let Value::Object(raw) = raw else {
        return Ok(None);
    };
    // `raw.v !== 1` is strict: the number 1, no other type.
    if raw.get("v").and_then(Value::as_f64) != Some(1.0) {
        return Ok(None);
    }
    let Some(window_id) = raw
        .get("windowId")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return Ok(None);
    };
    if window_id.len() != 64
        || !window_id
            .chars()
            .all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    {
        return Ok(None);
    }
    let channel = raw.get("channel").and_then(Value::as_str).unwrap_or("");
    if !matches!(channel, "stable" | "beta" | "remote") {
        return Ok(None);
    }
    for key in ["windowStart", "windowEnd", "coveredFrom", "generatedAt"] {
        if !raw.get(key).is_some_and(timestamp) {
            return Ok(None);
        }
    }
    let str_at = |k: &str| raw.get(k).and_then(Value::as_str).unwrap_or("");
    let start = parse_iso_ms(str_at("windowStart")).unwrap_or(f64::NAN);
    let end = parse_iso_ms(str_at("windowEnd")).unwrap_or(f64::NAN);
    let covered = parse_iso_ms(str_at("coveredFrom")).unwrap_or(f64::NAN);
    let generated = parse_iso_ms(str_at("generatedAt")).unwrap_or(f64::NAN);
    if start % DAY_MS != 0.0
        || end != start + DAY_MS
        || end > now
        || end < now - RECEIPT_DAYS * DAY_MS
        || covered < start
        || covered >= end
        || generated < end
        || generated > now + 300_000.0
    {
        return Ok(None);
    }
    if !QUALITY_FIELDS
        .iter()
        .all(|k| raw.get(*k).is_some_and(is_count))
        || !raw.get("rows").is_some_and(Value::is_array)
        || raw
            .get("rows")
            .and_then(Value::as_array)
            .map_or(0, |r| r.len())
            > MAX_ROWS
    {
        return Ok(None);
    }
    let rows_in = raw
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut capped = raw
        .get("cappedMeasurements")
        .and_then(Value::as_f64)
        .unwrap_or(f64::NAN);

    struct Group {
        row: serde_json::Map<String, Value>,
        limited: Vec<&'static str>,
    }
    let mut groups: Vec<(String, Group)> = Vec::new();

    let sum_pairs: [(&str, &str); 4] = [
        ("durationMs", "durationSamples"),
        ("inputTokens", "inputSamples"),
        ("outputTokens", "outputSamples"),
        ("cachedInputTokens", "cachedInputSamples"),
    ];

    for entry in &rows_in {
        if !is_record(entry)
            || !COUNT_FIELDS
                .iter()
                .all(|k| entry.get(*k).is_some_and(is_count))
        {
            return Ok(None);
        }
        let e = entry.as_object();
        let Some(e) = e else { return Ok(None) };
        let outcome = e.get("outcome").and_then(Value::as_str).unwrap_or("");
        if !matches!(outcome, "succeeded" | "failed" | "cancelled") {
            return Ok(None);
        }
        let provider = category(e.get("provider").unwrap_or(&Value::Null), PROVIDERS);
        let model = usage_model_category(provider, e.get("model").unwrap_or(&Value::Null));
        let mut row = serde_json::Map::new();
        row.insert("provider".into(), provider.into());
        row.insert("model".into(), model.clone().into());
        row.insert(
            "effort".into(),
            category(e.get("effort").unwrap_or(&Value::Null), EFFORTS).into(),
        );
        row.insert(
            "interactionMode".into(),
            category(
                e.get("interactionMode").unwrap_or(&Value::Null),
                INTERACTION_MODES,
            )
            .into(),
        );
        row.insert(
            "runtimeMode".into(),
            category(e.get("runtimeMode").unwrap_or(&Value::Null), RUNTIME_MODES).into(),
        );
        row.insert(
            "origin".into(),
            category(e.get("origin").unwrap_or(&Value::Null), ORIGINS).into(),
        );
        row.insert("outcome".into(), outcome.into());
        for k in COUNT_FIELDS {
            row.insert((*k).into(), e.get(*k).cloned().unwrap_or(Value::Null));
        }
        let turns = num_field(&row, "turns");
        if turns == 0.0 || turns > MAX_TURNS {
            return Ok(None);
        }
        for (sum_f, samples_f) in &sum_pairs {
            let samples = num_field(&row, samples_f);
            let sum = num_field(&row, sum_f);
            if samples > turns || (samples == 0.0 && sum != 0.0) {
                return Ok(None);
            }
        }
        let key = [
            provider,
            &model,
            row.get("effort").and_then(Value::as_str).unwrap_or(""),
            row.get("interactionMode")
                .and_then(Value::as_str)
                .unwrap_or(""),
            row.get("runtimeMode").and_then(Value::as_str).unwrap_or(""),
            row.get("origin").and_then(Value::as_str).unwrap_or(""),
            outcome,
        ]
        .join("|");
        match groups.iter_mut().find(|(k, _)| *k == key) {
            None => groups.push((
                key,
                Group {
                    row,
                    limited: vec![],
                },
            )),
            Some((_, g)) => {
                let new_turns = num_field(&g.row, "turns") + turns;
                g.row.insert("turns".into(), Value::from(new_turns));
                for (field, samples) in &sum_pairs {
                    if g.limited.contains(field) {
                        continue;
                    }
                    let total = num_field(&g.row, field) + num_field(&row, field);
                    if total > MAX_COUNTER {
                        g.row.insert((*field).into(), Value::from(0));
                        g.row.insert((*samples).into(), Value::from(0));
                        g.limited.push(field);
                        capped += 1.0;
                    } else {
                        g.row.insert((*field).into(), Value::from(total));
                        let s = num_field(&g.row, samples) + num_field(&row, samples);
                        g.row.insert((*samples).into(), Value::from(s));
                    }
                }
            }
        }
    }

    if !is_count(&Value::from(capped)) {
        return Ok(None);
    }
    let included: f64 = groups.iter().map(|(_, g)| num_field(&g.row, "turns")).sum();
    if included
        != raw
            .get("includedTurns")
            .and_then(Value::as_f64)
            .unwrap_or(f64::NAN)
        || included > MAX_TURNS
        || raw
            .get("eligibleTurns")
            .and_then(Value::as_f64)
            .unwrap_or(f64::NAN)
            < included
        || raw
            .get("tokenTruncatedTurns")
            .and_then(Value::as_f64)
            .unwrap_or(f64::NAN)
            > included
    {
        return Ok(None);
    }
    Ok(Some(serde_json::json!({
        "v": 1,
        "windowId": window_id,
        "channel": channel,
        "windowStart": str_at("windowStart"),
        "windowEnd": str_at("windowEnd"),
        "coveredFrom": str_at("coveredFrom"),
        "generatedAt": str_at("generatedAt"),
        "eligibleTurns": raw.get("eligibleTurns"),
        "includedTurns": included,
        "missingCompletionTimes": raw.get("missingCompletionTimes"),
        "tokenTruncatedTurns": raw.get("tokenTruncatedTurns"),
        "cappedMeasurements": capped,
        "rows": groups.into_iter().map(|(_, g)| Value::Object(g.row)).collect::<Vec<_>>(),
    })))
}
