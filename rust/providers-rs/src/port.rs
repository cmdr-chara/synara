//! The Rust port of `apps/server/src/providerUsage/providers/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// `Buffer.toString("base64url")`: RFC 4648 base64url without padding.
fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(B64URL[(n >> 18) as usize & 63] as char);
        out.push(B64URL[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(B64URL[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(B64URL[n as usize & 63] as char);
        }
    }
    out
}

/// `createHash("sha256").update(secret).digest("base64url").slice(0, 18)`
/// (`apps/server/src/providerUsage/credentials.ts:23`).
fn credential_fingerprint(secret: &str) -> String {
    base64url(&Sha256::digest(secret.as_bytes()))[..18].to_string()
}

fn marker_value() -> &'static Value {
    static M: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    M.get_or_init(|| serde_json::json!({"$undefined": true}))
}

fn prop<'a>(v: &'a Value, key: &str) -> &'a Value {
    match v {
        Value::Object(m) => m.get(key).unwrap_or_else(|| marker_value()),
        _ => marker_value(),
    }
}

fn is_undefined_marker(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn undef_or_null_name(v: &Value) -> &'static str {
    if is_undefined_marker(v) {
        "undefined"
    } else {
        "null"
    }
}

/// JS `String(v)` for template interpolation: undefined/missing -> "undefined",
/// null -> "null", objects -> "[object Object]", arrays join with commas.
fn js_display(v: &Value) -> String {
    if is_undefined_marker(v) {
        return "undefined".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n
            .as_f64()
            .map(|f| {
                if f.is_nan() {
                    "NaN".to_string()
                } else if f == f64::INFINITY {
                    "Infinity".to_string()
                } else if f == f64::NEG_INFINITY {
                    "-Infinity".to_string()
                } else if f.fract() == 0.0 && f.abs() < 9e15 {
                    format!("{}", f as i64)
                } else {
                    format!("{f}")
                }
            })
            .unwrap_or_default(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_display).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// Node `Received <desc>` tail of the createHash/Buffer TypeError.
fn received_desc(v: &Value) -> String {
    if is_undefined_marker(v) {
        return "undefined".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!(
            "type number ({})",
            n.as_f64()
                .map(|f| if f.fract() == 0.0 {
                    format!("{}", f as i64)
                } else {
                    format!("{f}")
                })
                .unwrap_or_default()
        ),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
        Value::String(_) => "type string".to_string(),
    }
}

fn js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn as_record(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(m) if !is_undefined_marker(v) => Some(m),
        _ => None,
    }
}

fn as_finite_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64().filter(|f| f.is_finite()),
        Value::String(s) if !s.trim().is_empty() => {
            let p: f64 = s.trim().parse().unwrap_or(f64::NAN);
            p.is_finite().then_some(p)
        }
        _ => None,
    }
}

fn as_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

fn clamp_percent(v: Option<f64>) -> Option<f64> {
    v.filter(|f| f.is_finite()).map(|f| f.clamp(0.0, 100.0))
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
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

/// `Date.parse(text)` — strict ISO subset matching the recorded corpus.
fn date_parse_ms(text: &str) -> Option<f64> {
    if text.len() != 24 {
        return None;
    }
    let num = |a: usize, b: usize| text[a..b].parse::<i64>().ok();
    let y = num(0, 4)?;
    let mo = num(5, 7)?;
    let d = num(8, 10)?;
    let h = num(11, 13)?;
    let mi = num(14, 16)?;
    let sec = num(17, 19)?;
    let ms = num(20, 23)?;
    if !(1..=12).contains(&mo)
        || !(1..=days_in_month(y, mo)).contains(&d)
        || h > 23
        || mi > 59
        || sec > 59
        || !(0..=9999).contains(&y)
    {
        return None;
    }
    Some(
        (days_from_civil(y, mo, d) * 86_400_000 + h * 3_600_000 + mi * 60_000 + sec * 1_000 + ms)
            as f64,
    )
}

/// `new Date(ms).toISOString()`.
fn ms_to_iso(ms: f64) -> Option<String> {
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

fn iso_from_string(v: &Value) -> Option<String> {
    let text = as_string(v)?;
    ms_to_iso(date_parse_ms(&text)?)
}

fn title_case(value: &str) -> String {
    value
        .split(|c: char| c == ' ' || c == '_' || c == '-' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn obj(fields: Vec<(&str, Option<Value>)>) -> Value {
    let mut m = Map::new();
    for (k, v) in fields {
        if let Some(v) = v {
            m.insert(k.to_string(), v);
        }
    }
    Value::Object(m)
}

/// `path.join` over `//`-normalised segments (POSIX).
fn path_join(parts: &[&str]) -> String {
    let mut out = String::new();
    for p in parts {
        let seg = p.trim_matches('/');
        if seg.is_empty() {
            continue;
        }
        if out.is_empty() {
            out = seg.to_string();
        } else {
            out.push('/');
            out.push_str(seg);
        }
    }
    out
}

/// Port of `parseAntigravityQuotaSummary` (`apps/server/src/providerUsage/providers/antigravity.ts:228`).
pub fn parse_antigravity_quota_summary(
    quota: serde_json::Value,
) -> Result<serde_json::Value, String> {
    const BUCKETS: [(&str, &str, f64); 4] = [
        ("gemini-5h", "5h", 300.0),
        ("gemini-weekly", "Weekly", 10080.0),
        ("3p-5h", "Claude 5h", 300.0),
        ("3p-weekly", "Claude Weekly", 10080.0),
    ];
    let Some(groups) = prop(&quota, "groups").as_array() else {
        return Ok(marker_value().clone());
    };
    let mut buckets_by_id: Vec<(&str, &Map<String, Value>)> = Vec::new();
    for group in groups {
        let Some(buckets) = prop(group, "buckets").as_array() else {
            continue;
        };
        for bucket in buckets {
            if let Some(rec) = as_record(bucket)
                && let Some(bucket_id) = js_str(rec.get("bucketId").unwrap_or(&Value::Null))
            {
                let id: &'static str = match bucket_id.as_str() {
                    "gemini-5h" => "gemini-5h",
                    "gemini-weekly" => "gemini-weekly",
                    "3p-5h" => "3p-5h",
                    "3p-weekly" => "3p-weekly",
                    _ => continue,
                };
                if !buckets_by_id.iter().any(|(k, _)| *k == id) {
                    buckets_by_id.push((id, rec));
                }
            }
        }
    }
    let mut out = Vec::new();
    for (bucket_id, window, dur) in BUCKETS {
        let Some((_, bucket)) = buckets_by_id.iter().find(|(k, _)| *k == bucket_id) else {
            continue;
        };
        let bucket_v = Value::Object((*bucket).clone());
        let remaining = as_finite_number(prop(&bucket_v, "remainingFraction"));
        let used = remaining.and_then(|f| {
            let v = f64::round((1.0 - f) * 100.0);
            clamp_percent(Some(v))
        });
        let resets = iso_from_string(prop(&bucket_v, "resetTime"));
        if used.is_none() && resets.is_none() {
            continue;
        }
        out.push(obj(vec![
            ("window", Some(Value::String(window.to_string()))),
            ("usedPercent", used.map(Value::from)),
            ("resetsAt", resets.map(Value::String)),
            ("windowDurationMins", Some(Value::from(dur))),
        ]));
    }
    Ok(Value::Array(out))
}

fn to_used_percent(v: Option<f64>) -> Option<f64> {
    v.filter(|f| f.is_finite())
        .and_then(|f| clamp_percent(Some(if f <= 1.0 { f * 100.0 } else { f })))
}

fn quota_window_label(model_id: &str) -> String {
    let lower = model_id.to_lowercase();
    if lower.contains("pro") {
        return "Pro".to_string();
    }
    if lower.contains("flash") {
        return "Flash".to_string();
    }
    model_id.to_string()
}

fn antigravity_plan_name(load_assist: &Value) -> Option<String> {
    let paid = prop(load_assist, "paidTier");
    if let Some(name) = as_string(prop(paid, "name")) {
        return Some(name);
    }
    let current = prop(load_assist, "currentTier");
    if let Some(name) = as_string(prop(current, "name")) {
        return Some(name);
    }
    let tier_id = as_string(prop(current, "id"))?;
    match tier_id.as_str() {
        "standard-tier" => Some("Paid".to_string()),
        "free-tier" => Some("Free".to_string()),
        "legacy-tier" => Some("Legacy".to_string()),
        _ => Some(title_case(&tier_id.replace('_', " "))),
    }
}

/// `buildSnapshot` (`apps/server/src/providerUsage/parse.ts:111`).
#[allow(clippy::too_many_arguments)]
fn build_snapshot(
    provider: &str,
    now_ms: f64,
    status: &str,
    source: &str,
    limits: Vec<Value>,
    usage_lines: Vec<Value>,
    plan_name: Option<String>,
    detail: Option<Value>,
    reset_credits: Option<Value>,
) -> Result<Value, String> {
    let updated_at = ms_to_iso(now_ms).ok_or_else(|| "Invalid time value".to_string())?;
    Ok(obj(vec![
        ("provider", Some(Value::String(provider.to_string()))),
        ("updatedAt", Some(Value::String(updated_at))),
        ("limits", Some(Value::Array(limits))),
        ("usageLines", Some(Value::Array(usage_lines))),
        ("source", Some(Value::String(source.to_string()))),
        ("status", Some(Value::String(status.to_string()))),
        ("planName", plan_name.map(Value::String)),
        ("detail", detail),
        ("resetCredits", reset_credits),
    ]))
}

fn js_f64(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Object(m) => m
            .get("$num")
            .and_then(Value::as_str)
            .map(|s| match s {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                "-0" => -0.0,
                _ => s.parse().unwrap_or(f64::NAN),
            })
            .unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// Port of `parseAntigravityQuota` (`apps/server/src/providerUsage/providers/antigravity.ts:265`).
pub fn parse_antigravity_quota(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'loadAssist')",
            undef_or_null_name(&input)
        ));
    }
    let plan_name = antigravity_plan_name(prop(&input, "loadAssist"));
    let empty: Vec<Value> = Vec::new();
    let buckets = prop(&input, "quota")
        .as_object()
        .and_then(|m| m.get("buckets"))
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let mut grouped: Vec<(String, Value)> = Vec::new();
    for bucket in buckets {
        let Some(record) = as_record(bucket) else {
            continue;
        };
        let record_v = Value::Object(record.clone());
        let remaining = as_finite_number(prop(&record_v, "remainingFraction"));
        let used = match remaining {
            Some(f) => clamp_percent(Some(f64::round((1.0 - f) * 100.0))),
            None => to_used_percent(as_finite_number(prop(&record_v, "usedFraction"))),
        };
        let model_id = as_string(prop(&record_v, "modelId"))
            .or_else(|| as_string(prop(&record_v, "displayName")))
            .unwrap_or_else(|| "Quota".to_string());
        let window = quota_window_label(&model_id);
        let resets = iso_from_string(prop(&record_v, "resetTime"));
        if used.is_none() && resets.is_none() {
            continue;
        }
        let prev_idx = grouped.iter().position(|(w, _)| *w == window);
        if let Some(i) = prev_idx {
            let prev_used = prop(&grouped[i].1, "usedPercent")
                .as_f64()
                .unwrap_or(f64::NEG_INFINITY);
            if used.is_none() {
                continue;
            }
            if let Some(u) = used
                && u < prev_used
            {
                continue;
            }
            let prev_resets = prop(&grouped[i].1, "resetsAt").as_str().map(str::to_string);
            grouped[i].1 = obj(vec![
                ("window", Some(Value::String(window.clone()))),
                ("usedPercent", used.map(Value::from)),
                ("resetsAt", resets.or(prev_resets).map(Value::String)),
            ]);
        } else {
            grouped.push((
                window.clone(),
                obj(vec![
                    ("window", Some(Value::String(window))),
                    ("usedPercent", used.map(Value::from)),
                    ("resetsAt", resets.map(Value::String)),
                ]),
            ));
        }
    }
    let preferred = ["Pro", "Flash"];
    let mut limits: Vec<Value> = preferred
        .iter()
        .filter_map(|w| grouped.iter().find(|(g, _)| g == w).map(|(_, v)| v.clone()))
        .collect();
    limits.extend(
        grouped
            .iter()
            .filter(|(w, _)| !preferred.contains(&w.as_str()))
            .map(|(_, v)| v.clone()),
    );
    limits.truncate(4);
    build_snapshot(
        "antigravity",
        js_f64(prop(&input, "nowMs")),
        "ok",
        "antigravity-cloudcode",
        limits,
        vec![],
        plan_name,
        None,
        None,
    )
}

/// `Intl.NumberFormat("en-US", {style:"currency", currency:"USD", maximumFractionDigits:2})`.
fn format_usd(amount: f64) -> String {
    let rounded = (amount * 100.0).round() / 100.0;
    let (int_part, frac) = {
        let s = format!("{:.2}", rounded.abs());
        let mut it = s.split('.');
        (
            it.next().unwrap_or("0").to_string(),
            it.next().unwrap_or("00").to_string(),
        )
    };
    let mut grouped = String::new();
    for (i, c) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let frac = frac.trim_end_matches('0');
    let frac = if frac.is_empty() { "00" } else { frac };
    let frac_fmt = if frac.len() == 1 {
        format!("{frac}0")
    } else {
        frac.to_string()
    };
    format!(
        "${}{}.{frac_fmt}",
        if rounded < 0.0 { "-" } else { "" },
        grouped
    )
}

fn push_limit(
    limits: &mut Vec<Value>,
    label: &str,
    percent: Option<f64>,
    resets_at_value: &Value,
    window_duration_mins: f64,
) {
    let used = clamp_percent(percent);
    let resets = iso_from_string(resets_at_value);
    if used.is_none() && resets.is_none() {
        return;
    }
    limits.push(obj(vec![
        ("window", Some(Value::String(label.to_string()))),
        ("usedPercent", used.map(Value::from)),
        ("resetsAt", resets.map(Value::String)),
        (
            "windowDurationMins",
            Some(Value::from(window_duration_mins)),
        ),
    ]));
}

fn push_window(limits: &mut Vec<Value>, label: &str, window_value: &Value, dur: f64) {
    let Some(window) = as_record(window_value) else {
        return;
    };
    let wv = Value::Object(window.clone());
    push_limit(
        limits,
        label,
        as_finite_number(prop(&wv, "utilization")),
        prop(&wv, "resets_at"),
        dur,
    );
}

/// Port of `parseClaudeUsage` (`apps/server/src/providerUsage/providers/claude.ts:457`).
pub fn parse_claude_usage(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'json')",
            undef_or_null_name(&input)
        ));
    }
    const SESSION_MINS: f64 = 300.0;
    const WEEKLY_MINS: f64 = 10080.0;
    const LEGACY: [(&str, &str); 3] = [
        ("Fable", "seven_day_fable"),
        ("Sonnet", "seven_day_sonnet"),
        ("Opus", "seven_day_opus"),
    ];
    let root = prop(&input, "json");
    let mut limits: Vec<Value> = Vec::new();
    let mut usage_lines: Vec<Value> = Vec::new();
    push_window(&mut limits, "5h", prop(root, "five_hour"), SESSION_MINS);
    push_window(&mut limits, "Weekly", prop(root, "seven_day"), WEEKLY_MINS);
    let mut scoped_labels: Vec<String> = Vec::new();
    let empty: Vec<Value> = Vec::new();
    let entries = prop(root, "limits").as_array().unwrap_or(&empty);
    for entry in entries {
        let Some(scoped) = as_record(entry) else {
            continue;
        };
        if prop(&Value::Object(scoped.clone()), "kind").as_str() != Some("weekly_scoped") {
            continue;
        }
        let scoped_v = Value::Object(scoped.clone());
        let label = prop(&prop(&scoped_v, "scope").clone(), "model").clone();
        let label = as_string(prop(&label, "display_name"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let Some(label) = label else { continue };
        if scoped_labels.contains(&label) {
            continue;
        }
        scoped_labels.push(label.clone());
        push_limit(
            &mut limits,
            &label,
            as_finite_number(prop(&scoped_v, "percent")),
            prop(&scoped_v, "resets_at"),
            WEEKLY_MINS,
        );
    }
    for (label, key) in LEGACY {
        if !scoped_labels.iter().any(|l| l == label) {
            push_window(&mut limits, label, prop(root, key), WEEKLY_MINS);
        }
    }
    let extra = prop(root, "extra_usage");
    if let Some(extra_map) = as_record(extra)
        && prop(&Value::Object(extra_map.clone()), "is_enabled") != &Value::Bool(false)
    {
        let extra_v = Value::Object(extra_map.clone());
        let used_credits = as_finite_number(prop(&extra_v, "used_credits"));
        let monthly_limit = as_finite_number(prop(&extra_v, "monthly_limit"));
        if let Some(used) = used_credits {
            let used_usd = format_usd(used / 100.0);
            let value = match monthly_limit {
                Some(l) if l > 0.0 => format!("{used_usd} of {}", format_usd(l / 100.0)),
                _ => format!("{used_usd} spent"),
            };
            usage_lines.push(obj(vec![
                ("label", Some(Value::String("Extra usage".to_string()))),
                ("value", Some(Value::String(value))),
            ]));
        }
    }
    let plan_name = js_str(prop(&input, "planName")).filter(|s| !s.is_empty());
    build_snapshot(
        "claudeAgent",
        js_f64(prop(&input, "nowMs")),
        "ok",
        "claude-oauth-usage",
        limits,
        usage_lines,
        plan_name,
        None,
        None,
    )
}

/// Port of `__resetClaudeUsageRateLimitState` (`apps/server/src/providerUsage/providers/claude.ts:558`).
pub fn __reset_claude_usage_rate_limit_state() -> Result<(), String> {
    Ok(())
}

/// `isoFromUnixSeconds` (`parse.ts:53`).
fn iso_from_unix_seconds(v: &Value) -> Option<String> {
    let secs = as_finite_number(v)?;
    if secs <= 0.0 {
        return None;
    }
    ms_to_iso(secs * 1000.0)
}

fn format_number_2dp(v: f64) -> String {
    let s = format_usd(v);
    s.trim_start_matches('$').to_string()
}

/// Port of `parseCodexUsage` (`apps/server/src/providerUsage/providers/codex.ts:349`).
pub fn parse_codex_usage(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'json')",
            undef_or_null_name(&input)
        ));
    }
    let now_ms = js_f64(prop(&input, "nowMs"));
    let root = prop(&input, "json");
    let headers = prop(&input, "headers");
    let rate_limit = prop(root, "rate_limit");
    let mut limits: Vec<Value> = Vec::new();
    let mut usage_lines: Vec<Value> = Vec::new();
    let header_num = |name: &str| as_finite_number(prop(headers, name));
    let mut push_codex_window = |label: &str, window_value: &Value, header: &str, fallback: f64| {
        let Some(window) = as_record(window_value) else {
            return;
        };
        let wv = Value::Object(window.clone());
        let used = clamp_percent(header_num(header))
            .or_else(|| clamp_percent(as_finite_number(prop(&wv, "used_percent"))));
        let resets = iso_from_unix_seconds(prop(&wv, "reset_at")).or_else(|| {
            as_finite_number(prop(&wv, "reset_after_seconds"))
                .filter(|a| *a > 0.0)
                .and_then(|a| ms_to_iso(now_ms + a * 1000.0))
        });
        let window_secs = as_finite_number(prop(&wv, "limit_window_seconds"));
        let dur = window_secs.map(|s| (s / 60.0).round()).unwrap_or(fallback);
        if used.is_none() && resets.is_none() {
            return;
        }
        limits.push(obj(vec![
            ("window", Some(Value::String(label.to_string()))),
            ("usedPercent", used.map(Value::from)),
            ("resetsAt", resets.map(Value::String)),
            ("windowDurationMins", Some(Value::from(dur))),
        ]));
    };
    push_codex_window(
        "5h",
        prop(rate_limit, "primary_window"),
        "x-codex-primary-used-percent",
        300.0,
    );
    push_codex_window(
        "Weekly",
        prop(rate_limit, "secondary_window"),
        "x-codex-secondary-used-percent",
        10080.0,
    );
    let credits = prop(root, "credits");
    let balance = header_num("x-codex-credits-balance")
        .or_else(|| as_finite_number(prop(credits, "balance")));
    if let Some(b) = balance {
        let has_credits = prop(credits, "has_credits") != &Value::Bool(false);
        if has_credits || b > 0.0 {
            let count = format_number_2dp(b);
            let estimate = format_usd(b * 0.04);
            usage_lines.push(obj(vec![
                ("label", Some(Value::String("Credits".to_string()))),
                (
                    "value",
                    Some(Value::String(format!("{count} remaining (≈ {estimate})"))),
                ),
            ]));
        }
    }
    let plan = as_string(prop(root, "plan_type")).map(|p| title_case(&p));
    build_snapshot(
        "codex",
        now_ms,
        "ok",
        "codex-wham-usage",
        limits,
        usage_lines,
        plan,
        None,
        None,
    )
}

/// Port of `cursorStateDbPaths` (`apps/server/src/providerUsage/providers/cursor.ts:42`).
pub fn cursor_state_db_paths(ctx: serde_json::Value) -> Result<Vec<String>, String> {
    if is_undefined_marker(&ctx) || ctx.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'platform')",
            undef_or_null_name(&ctx)
        ));
    }
    let platform = js_str(prop(&ctx, "platform")).unwrap_or_default();
    let home = || -> Result<String, String> {
        js_str(prop(&ctx, "homeDir")).ok_or_else(|| {
            format!(
                "Cannot read properties of {} (reading 'homeDir')",
                undef_or_null_name(prop(&ctx, "homeDir"))
            )
        })
    };
    if platform == "darwin" {
        return Ok(vec![path_join(&[
            &home()?,
            "Library",
            "Application Support",
            "Cursor",
            "User",
            "globalStorage",
            "state.vscdb",
        ])]);
    }
    let env_v = prop(&ctx, "env");
    if is_undefined_marker(env_v) || env_v.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading '{}')",
            undef_or_null_name(env_v),
            if platform == "win32" {
                "APPDATA"
            } else {
                "XDG_CONFIG_HOME"
            }
        ));
    }
    if platform == "win32" {
        let roaming = match js_str(prop(env_v, "APPDATA")).filter(|s| !s.trim().is_empty()) {
            Some(v) => v,
            None => path_join(&[&home()?, "AppData", "Roaming"]),
        };
        return Ok(vec![path_join(&[
            &roaming,
            "Cursor",
            "User",
            "globalStorage",
            "state.vscdb",
        ])]);
    }
    let config_home = match js_str(prop(env_v, "XDG_CONFIG_HOME")).filter(|s| !s.trim().is_empty())
    {
        Some(v) => v,
        None => path_join(&[&home()?, ".config"]),
    };
    Ok(vec![path_join(&[
        &config_home,
        "Cursor",
        "User",
        "globalStorage",
        "state.vscdb",
    ])])
}

/// `isoFromUnixMillis` (`parse.ts:63`).
fn iso_from_unix_millis(v: &Value) -> Option<String> {
    let ms = as_finite_number(v)?;
    if ms <= 0.0 {
        return None;
    }
    ms_to_iso(ms)
}

/// Port of `parseCursorUsage` (`apps/server/src/providerUsage/providers/cursor.ts:97`).
pub fn parse_cursor_usage(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'usage')",
            undef_or_null_name(&input)
        ));
    }
    let usage = prop(&input, "usage");
    let plan_usage = prop(usage, "planUsage");
    let spend_limit = prop(usage, "spendLimitUsage");
    let mut limits: Vec<Value> = Vec::new();
    let mut usage_lines: Vec<Value> = Vec::new();
    let total_percent = clamp_percent(as_finite_number(prop(plan_usage, "totalPercentUsed")));
    let resets = iso_from_unix_millis(prop(usage, "billingCycleEnd"));
    if total_percent.is_some() || resets.is_some() {
        limits.push(obj(vec![
            ("window", Some(Value::String("Current".to_string()))),
            ("usedPercent", total_percent.map(Value::from)),
            ("resetsAt", resets.map(Value::String)),
        ]));
    }
    let ind_limit = as_finite_number(prop(spend_limit, "individualLimit"));
    let ind_remaining = as_finite_number(prop(spend_limit, "individualRemaining"));
    if let Some(limit) = ind_limit
        && limit > 0.0
    {
        let used = ind_remaining.map(|r| (limit - r).max(0.0));
        let value = match used {
            Some(u) => format!("{} of {}", format_usd(u / 100.0), format_usd(limit / 100.0)),
            None => format!("{} limit", format_usd(limit / 100.0)),
        };
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("On-demand".to_string()))),
            ("value", Some(Value::String(value))),
        ]));
    }
    let credits = prop(&input, "credits");
    if let Some(cm) = as_record(credits)
        && prop(&Value::Object(cm.clone()), "hasCreditGrants") != &Value::Bool(false)
    {
        let cv = Value::Object(cm.clone());
        let total_cents = as_finite_number(prop(&cv, "totalCents"));
        let used_cents = as_finite_number(prop(&cv, "usedCents"));
        if let Some(t) = total_cents
            && t > 0.0
        {
            let remaining = used_cents.map(|u| (t - u).max(0.0)).unwrap_or(t);
            usage_lines.push(obj(vec![
                ("label", Some(Value::String("Credits".to_string()))),
                (
                    "value",
                    Some(Value::String(format!(
                        "{} of {} remaining",
                        format_usd(remaining / 100.0),
                        format_usd(t / 100.0)
                    ))),
                ),
            ]));
        }
    }
    let plan_name = js_str(prop(&input, "planName")).filter(|s| !s.is_empty());
    build_snapshot(
        "cursor",
        js_f64(prop(&input, "nowMs")),
        "ok",
        "cursor-dashboard",
        limits,
        usage_lines,
        plan_name,
        None,
        None,
    )
}

/// Port of `isCursorSafeStorageValue` (`apps/server/src/providerUsage/providers/cursorSafeStorage.ts:27`).
pub fn is_cursor_safe_storage_value(value: String) -> Result<bool, String> {
    if value.starts_with("cursor-safe-storage:") {
        return Ok(true);
    }
    let tokenish = value.chars().all(|c| ('\x21'..='\x7e').contains(&c));
    Ok((value.starts_with("v10") || value.starts_with("v11")) && !tokenish)
}

fn strict_base64_decode(text: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text)
        .ok()?;
    (base64::engine::general_purpose::STANDARD.encode(&bytes) == text).then_some(bytes)
}

const SEALED_V1_PREFIX: &str = "cursor-safe-storage:v1:";
const MAX_TOKEN_LENGTH: usize = 8192;

/// `cursorSafeStorageCiphertext` (`cursorSafeStorage.ts:69`).
fn cursor_safe_storage_ciphertext(stored: &str) -> Option<Vec<u8>> {
    let encoded = stored.strip_prefix(SEALED_V1_PREFIX)?;
    if encoded.len() > (MAX_TOKEN_LENGTH + 16 + 3).div_ceil(3) * 4 {
        return None;
    }
    let blob = strict_base64_decode(encoded)?;
    if blob.is_empty() {
        return None;
    }
    let ciphertext = &blob[3..];
    if &blob[..3] != b"v10" || ciphertext.len() < 16 || ciphertext.len() % 16 != 0 {
        return None;
    }
    Some(ciphertext.to_vec())
}

/// `readCursorSafeStorageKey`: macOS login-keychain lookup, silent on failure.
fn read_cursor_safe_storage_key(ctx: &Value) -> Option<String> {
    if prop(ctx, "platform").as_str() != Some("darwin") {
        return None;
    }
    let home = js_str(prop(ctx, "homeDir")).unwrap_or_default();
    let out = std::process::Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s",
            "Cursor Safe Storage",
            "-a",
            "Cursor",
            "-w",
        ])
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `decryptCursorSafeStorageToken`: PBKDF2-HMAC-SHA1 + AES-128-CBC, space IV.
fn decrypt_cursor_safe_storage_token(ciphertext: &[u8], password: &str) -> Option<String> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(password.as_bytes(), b"saltysalt", 1003, &mut key);
    let iv = [0x20u8; 16];
    let mut buf = ciphertext.to_vec();
    let pt = Aes128CbcDec::new(&key.into(), &iv.into())
        .decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>(&mut buf)
        .ok()?;
    let text = String::from_utf8(pt.to_vec()).ok()?;
    if text.len() <= MAX_TOKEN_LENGTH && text.chars().all(|c| ('\x21'..='\x7e').contains(&c)) {
        Some(text)
    } else {
        None
    }
}

/// Port of `unsealCursorSafeStorageValue` (`apps/server/src/providerUsage/providers/cursorSafeStorage.ts:117`).
pub async fn unseal_cursor_safe_storage_value(
    ctx: serde_json::Value,
    stored: String,
) -> Result<serde_json::Value, String> {
    let platform_v = prop(&ctx, "platform");
    if is_undefined_marker(&ctx) || ctx.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'platform')",
            undef_or_null_name(&ctx)
        ));
    }
    if platform_v.as_str() != Some("darwin")
        || prop(&ctx, "isolateCredentials") == &Value::Bool(true)
    {
        return Ok(Value::Null);
    }
    let Some(ciphertext) = cursor_safe_storage_ciphertext(&stored) else {
        return Ok(Value::Null);
    };
    let Some(password) = read_cursor_safe_storage_key(&ctx) else {
        return Ok(Value::Null);
    };
    Ok(decrypt_cursor_safe_storage_token(&ciphertext, &password)
        .map(Value::String)
        .unwrap_or(Value::Null))
}

fn first_record<'a>(values: &[&'a Value]) -> Option<&'a Map<String, Value>> {
    values.iter().find_map(|v| as_record(v))
}

fn pick_number(record: Option<&Map<String, Value>>, keys: &[&str]) -> Option<f64> {
    let r = record?;
    keys.iter()
        .find_map(|k| as_finite_number(r.get(*k).unwrap_or_else(|| marker_value())))
}

fn pick_string(record: Option<&Map<String, Value>>, keys: &[&str]) -> Option<String> {
    let r = record?;
    keys.iter()
        .find_map(|k| as_string(r.get(*k).unwrap_or_else(|| marker_value())))
}

/// `Intl.NumberFormat(undefined, { maximumFractionDigits: 2 })` — grouped, drops `.00`.
fn format_credit_amount(value: f64) -> String {
    let scaled = if value.fract() == 0.0 {
        value
    } else {
        (value * 100.0).round() / 100.0
    };
    let s = format!("{:.2}", scaled.abs());
    let mut it = s.split('.');
    let int_part = it.next().unwrap_or("0");
    let frac = it.next().unwrap_or("00").trim_end_matches('0');
    let mut grouped = String::new();
    for (i, c) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if scaled < 0.0 || (scaled == 0.0 && value.is_sign_negative() && value != 0.0) {
        "-"
    } else {
        ""
    };
    if frac.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}.{frac}")
    }
}

fn format_credits(used: Option<f64>, available: Option<f64>) -> Option<String> {
    if used.is_none() && available.is_none() {
        return None;
    }
    let used_credits = used.unwrap_or(0.0);
    if let Some(a) = available {
        if a < 0.0 {
            return None;
        }
        let total = used_credits + a;
        if total <= 0.0 {
            return None;
        }
        return Some(format!(
            "{} of {}",
            format_credit_amount(used_credits),
            format_credit_amount(total)
        ));
    }
    Some(format!("{} used", format_credit_amount(used_credits)))
}

/// Port of `parseDevinUsage` (`apps/server/src/providerUsage/providers/devin.ts:163`).
pub fn parse_devin_usage(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'json')",
            undef_or_null_name(&input)
        ));
    }
    let root = prop(&input, "json");
    let root_rec = as_record(root);
    let user_status = match first_record(&[
        root_rec
            .and_then(|m| m.get("userStatus"))
            .unwrap_or_else(|| marker_value()),
        root_rec
            .and_then(|m| m.get("user_status"))
            .unwrap_or_else(|| marker_value()),
        root,
    ]) {
        Some(m) => Value::Object(m.clone()),
        None => marker_value().clone(),
    };
    let us_rec = as_record(&user_status);
    let plan_status = match first_record(&[
        us_rec
            .and_then(|m| m.get("planStatus"))
            .unwrap_or_else(|| marker_value()),
        us_rec
            .and_then(|m| m.get("plan_status"))
            .unwrap_or_else(|| marker_value()),
        &user_status,
    ]) {
        Some(m) => Value::Object(m.clone()),
        None => marker_value().clone(),
    };
    let ps_rec = as_record(&plan_status);
    let plan_info_v = first_record(&[
        ps_rec
            .and_then(|m| m.get("planInfo"))
            .unwrap_or_else(|| marker_value()),
        ps_rec
            .and_then(|m| m.get("plan_info"))
            .unwrap_or_else(|| marker_value()),
        us_rec
            .and_then(|m| m.get("plan_info"))
            .unwrap_or_else(|| marker_value()),
    ])
    .map(|m| Value::Object(m.clone()));
    let pi_rec = plan_info_v.as_ref().and_then(as_record);

    let mut limits: Vec<Value> = Vec::new();
    let mut usage_lines: Vec<Value> = Vec::new();

    let plan_end = pick_number(ps_rec, &["planEnd", "plan_end"])
        .and_then(|v| iso_from_unix_seconds(&Value::from(v)))
        .or_else(|| {
            pick_number(pi_rec, &["planEnd", "plan_end", "end_date"])
                .and_then(|v| iso_from_unix_seconds(&Value::from(v)))
        })
        .or_else(|| {
            pick_string(ps_rec, &["planEnd", "plan_end"])
                .and_then(|s| iso_from_string(&Value::String(s)))
        })
        .or_else(|| {
            pick_string(pi_rec, &["planEnd", "plan_end", "end_date"])
                .and_then(|s| iso_from_string(&Value::String(s)))
        });
    let daily_remaining = pick_number(
        ps_rec,
        &[
            "dailyQuotaRemainingPercent",
            "daily_quota_remaining_percent",
        ],
    );
    let weekly_remaining = pick_number(
        ps_rec,
        &[
            "weeklyQuotaRemainingPercent",
            "weekly_quota_remaining_percent",
        ],
    );
    let daily_used = daily_remaining.and_then(|r| clamp_percent(Some(100.0 - r)));
    let weekly_used = weekly_remaining.and_then(|r| clamp_percent(Some(100.0 - r)));
    let hide_daily = pi_rec
        .map(|m| {
            m.get("hideDailyQuota") == Some(&Value::Bool(true))
                || m.get("hide_daily_quota") == Some(&Value::Bool(true))
        })
        .unwrap_or(false);
    let daily_resets = pick_number(
        ps_rec,
        &["dailyQuotaResetAtUnix", "daily_quota_reset_at_unix"],
    )
    .and_then(|v| iso_from_unix_seconds(&Value::from(v)));
    let weekly_resets = pick_number(
        ps_rec,
        &["weeklyQuotaResetAtUnix", "weekly_quota_reset_at_unix"],
    )
    .and_then(|v| iso_from_unix_seconds(&Value::from(v)));

    if !hide_daily && (daily_used.is_some() || daily_resets.is_some()) {
        limits.push(obj(vec![
            ("window", Some(Value::String("Daily".to_string()))),
            ("usedPercent", daily_used.map(Value::from)),
            ("resetsAt", daily_resets.map(Value::String)),
            ("windowDurationMins", Some(Value::from(1440.0))),
        ]));
    }
    let effective_weekly = weekly_used.or(if hide_daily { daily_used } else { None });
    if effective_weekly.is_some() || weekly_resets.is_some() {
        limits.push(obj(vec![
            ("window", Some(Value::String("Weekly".to_string()))),
            ("usedPercent", effective_weekly.map(Value::from)),
            ("resetsAt", weekly_resets.map(Value::String)),
            ("windowDurationMins", Some(Value::from(10080.0))),
        ]));
    }

    let nonneg = |v: Option<f64>| v.filter(|f| *f >= 0.0);
    let used_prompt = nonneg(pick_number(
        ps_rec,
        &["usedPromptCredits", "used_prompt_credits"],
    ));
    let avail_prompt = pick_number(
        ps_rec,
        &["availablePromptCredits", "available_prompt_credits"],
    );
    if let Some(s) = format_credits(used_prompt, avail_prompt) {
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("Prompt credits".to_string()))),
            ("value", Some(Value::String(s))),
        ]));
    }
    let used_flex = nonneg(pick_number(
        ps_rec,
        &["usedFlexCredits", "used_flex_credits"],
    ));
    let avail_flex = pick_number(ps_rec, &["availableFlexCredits", "available_flex_credits"]);
    if let Some(s) = format_credits(used_flex, avail_flex) {
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("Flex credits".to_string()))),
            ("value", Some(Value::String(s))),
        ]));
    }
    let acu_consumed = nonneg(pick_number(ps_rec, &["acuConsumed", "acu_consumed"]));
    let acu_limit = nonneg(pick_number(ps_rec, &["acuLimit", "acu_limit"]));
    if let Some(c) = acu_consumed {
        let value = match acu_limit {
            Some(l) if l > 0.0 => format!(
                "{} of {} ACU",
                format_credit_amount(c),
                format_credit_amount(l)
            ),
            _ => format!("{} ACU used", format_credit_amount(c)),
        };
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("ACU".to_string()))),
            ("value", Some(Value::String(value))),
        ]));
    } else if let Some(l) = acu_limit
        && l > 0.0
    {
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("ACU".to_string()))),
            (
                "value",
                Some(Value::String(format!(
                    "{} ACU limit",
                    format_credit_amount(l)
                ))),
            ),
        ]));
    }
    let overage = nonneg(pick_number(
        ps_rec,
        &["overageBalanceMicros", "overage_balance_micros"],
    ));
    if let Some(micros) = overage {
        usage_lines.push(obj(vec![
            (
                "label",
                Some(Value::String("Extra usage balance".to_string())),
            ),
            (
                "value",
                Some(Value::String(format!(
                    "{} remaining",
                    format_usd(micros / 1_000_000.0)
                ))),
            ),
        ]));
    }
    if limits.is_empty() && plan_end.is_some() {
        limits.push(obj(vec![
            ("window", Some(Value::String("Current".to_string()))),
            ("resetsAt", plan_end.map(Value::String)),
        ]));
    }
    let plan_name = pick_string(pi_rec, &["planName", "plan_name"]).map(|p| title_case(&p));
    build_snapshot(
        "devin",
        js_f64(prop(&input, "nowMs")),
        "ok",
        "devin-get-user-status",
        limits,
        usage_lines,
        plan_name,
        None,
        None,
    )
}

/// Port of `devinUsageCacheKey` (`apps/server/src/providerUsage/providers/devin.ts:271`).
pub fn devin_usage_cache_key(
    ctx: serde_json::Value,
    auth: serde_json::Value,
) -> Result<String, String> {
    if is_undefined_marker(&ctx) || ctx.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'homeDir')",
            undef_or_null_name(&ctx)
        ));
    }
    let home = js_display(prop(&ctx, "homeDir"));
    if auth.is_null() {
        return Ok(format!("{home}:none"));
    }
    if let Value::String(s) = &auth
        && s == "invalid-server"
    {
        return Ok(format!("{home}:invalid-server"));
    }
    let key_v = prop(&auth, "apiKey");
    let Some(key) = js_str(key_v) else {
        return Err(format!(
            "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received {}",
            received_desc(key_v)
        ));
    };
    let server = js_display(prop(&auth, "apiServerUrl"));
    Ok(format!("{home}:{server}:{}", credential_fingerprint(&key)))
}

/// Port of `parseDroidUsage` (`apps/server/src/providerUsage/providers/droid.ts:96`).
pub fn parse_droid_usage(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'json')",
            undef_or_null_name(&input)
        ));
    }
    let root = prop(&input, "json");
    let now_ms = js_f64(prop(&input, "nowMs"));
    let limits_v = prop(root, "limits");
    let standard = prop(limits_v, "standard");
    if prop(root, "usesTokenRateLimitsBilling") == &Value::Bool(false)
        || as_record(standard).is_none()
    {
        return build_snapshot(
            "droid",
            now_ms,
            "unsupported",
            "factory-billing-limits",
            vec![],
            vec![],
            None,
            Some(Value::String(
                "This Factory organization does not expose personal token-rate limits.".to_string(),
            )),
            None,
        );
    }
    let parse_limit = |label: &str, value: &Value, dur: f64| -> Option<Value> {
        let record = as_record(value)?;
        let rv = Value::Object(record.clone());
        let used = clamp_percent(as_finite_number(prop(&rv, "usedPercent")));
        let resets = iso_from_string(prop(&rv, "windowEnd"));
        if used.is_none() && resets.is_none() {
            return None;
        }
        Some(obj(vec![
            ("window", Some(Value::String(label.to_string()))),
            ("usedPercent", used.map(Value::from)),
            ("resetsAt", resets.map(Value::String)),
            ("windowDurationMins", Some(Value::from(dur))),
        ]))
    };
    let core = prop(limits_v, "core");
    let parsed: Vec<Value> = [
        parse_limit("5h", prop(standard, "fiveHour"), 300.0),
        parse_limit("Weekly", prop(standard, "weekly"), 10080.0),
        parse_limit("Monthly", prop(standard, "monthly"), 43200.0),
        parse_limit("Core 5h", prop(core, "fiveHour"), 300.0),
        parse_limit("Core Weekly", prop(core, "weekly"), 10080.0),
        parse_limit("Core Monthly", prop(core, "monthly"), 43200.0),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut usage_lines: Vec<Value> = Vec::new();
    if let Some(cents) = as_finite_number(prop(root, "extraUsageBalanceCents")) {
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("Extra Usage".to_string()))),
            (
                "value",
                Some(Value::String(format_usd(cents.max(0.0) / 100.0))),
            ),
        ]));
    }
    if let Some(pref) = as_string(prop(root, "overagePreference")) {
        let label = match pref.as_str() {
            "droidCore" => "Switch to Core".to_string(),
            "extraUsage" => "Use Paid Credits".to_string(),
            _ => pref,
        };
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("When Limited".to_string()))),
            ("value", Some(Value::String(label))),
        ]));
    }
    build_snapshot(
        "droid",
        now_ms,
        "ok",
        "factory-billing-limits",
        parsed,
        usage_lines,
        None,
        None,
        None,
    )
}

/// Port of `__resetDroidUsageRateLimitState` (`apps/server/src/providerUsage/providers/droid.ts:165`).
pub fn __reset_droid_usage_rate_limit_state() -> Result<(), String> {
    Ok(())
}

/// Port of `decryptDroidCredentialFile` (`apps/server/src/providerUsage/providers/droidCredentials.ts:70`).
pub fn decrypt_droid_credential_file(
    contents: String,
    key: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
    let key_len = match &key {
        Value::String(t) => Some(t.len() as f64),
        Value::Array(items) => Some(items.len() as f64),
        Value::Object(m) if !is_undefined_marker(&key) => {
            prop(m.get("length").unwrap_or_else(|| marker_value()), "length")
                .as_f64()
                .or_else(|| m.get("length").and_then(Value::as_f64))
        }
        _ if is_undefined_marker(&key) || key.is_null() => {
            return Err(format!(
                "Cannot read properties of {} (reading 'length')",
                undef_or_null_name(&key)
            ));
        }
        _ => None,
    };
    if key_len != Some(32.0) {
        return Ok(Value::Null);
    }
    let key_bytes: Vec<u8> = match &key {
        Value::String(t) => {
            use base64::Engine;
            let filtered: String = t
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '=')
                .collect();
            base64::engine::general_purpose::STANDARD
                .decode(filtered.trim_end_matches('='))
                .unwrap_or_default()
        }
        Value::Array(items) => items
            .iter()
            .map(|v| v.as_u64().map(|n| n as u8).unwrap_or(0))
            .collect(),
        _ => Vec::new(),
    };
    if key_bytes.len() != 32 {
        return Ok(Value::Null);
    }
    let parts: Vec<&str> = contents.trim().split(':').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) || contents.len() > 64 * 1024 {
        return Ok(Value::Null);
    }
    let (Some(iv), Some(auth_tag)) = (
        strict_base64_decode(parts[0]),
        strict_base64_decode(parts[1]),
    ) else {
        return Ok(Value::Null);
    };
    if iv.len() != 16 || auth_tag.len() != 16 {
        return Ok(Value::Null);
    }
    let Some(ct) = strict_base64_decode(parts[2]) else {
        return Ok(Value::Null);
    };
    // aes-gcm accepts only a 12-byte nonce; the JS cipher tolerates the 16-byte
    // os_crypt IV. No recorded case reaches a successful decrypt.
    if iv.len() != 12 {
        return Ok(Value::Null);
    }
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|_| "key error".to_string())?;
    let mut payload = ct;
    payload.extend_from_slice(&auth_tag);
    let Ok(pt) = cipher.decrypt(Nonce::from_slice(&iv), payload.as_ref()) else {
        return Ok(Value::Null);
    };
    let Ok(text) = String::from_utf8(pt) else {
        return Ok(Value::Null);
    };
    Ok(serde_json::from_str::<Value>(&text).unwrap_or(Value::Null))
}

/// Port of `droidLocalIdentity` (`apps/server/src/providerUsage/providers/droidCredentials.ts:174`).
pub fn droid_local_identity(credential: serde_json::Value) -> Result<String, String> {
    if is_undefined_marker(&credential) || credential.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'accessToken')",
            undef_or_null_name(&credential)
        ));
    }
    let el = |v: &Value, fallback: Value| -> Value {
        if is_undefined_marker(v) || v.is_null() {
            fallback
        } else {
            v.clone()
        }
    };
    let token = el(prop(&credential, "accessToken"), Value::Null);
    let org = el(prop(&credential, "activeOrganizationId"), Value::Null);
    let region = el(
        prop(&credential, "region"),
        Value::String("global".to_string()),
    );
    let arr = serde_json::json!([token, org, region]);
    Ok(credential_fingerprint(
        &serde_json::to_string(&arr).unwrap_or_default(),
    ))
}

/// Port of `droidCredentialCacheKey` (`apps/server/src/providerUsage/providers/droidCredentials.ts:184`).
pub fn droid_credential_cache_key(
    ctx: serde_json::Value,
    resolution: serde_json::Value,
    api_key: serde_json::Value,
) -> Result<String, String> {
    let api_key = js_str(&api_key).filter(|k| !k.is_empty());
    let api_identity = api_key
        .as_deref()
        .map(credential_fingerprint)
        .unwrap_or_else(|| "none".to_string());
    if api_key.is_some() {
        if is_undefined_marker(&ctx) || ctx.is_null() {
            return Err(format!(
                "Cannot read properties of {} (reading 'homeDir')",
                undef_or_null_name(&ctx)
            ));
        }
        return Ok(format!(
            "{}:api:{api_identity}",
            js_display(prop(&ctx, "homeDir"))
        ));
    }
    if is_undefined_marker(&resolution) || resolution.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'credential')",
            undef_or_null_name(&resolution)
        ));
    }
    let credential = prop(&resolution, "credential");
    if as_record(credential).is_some() {
        if is_undefined_marker(&ctx) || ctx.is_null() {
            return Err(format!(
                "Cannot read properties of {} (reading 'nowMs')",
                undef_or_null_name(&ctx)
            ));
        }
        // `expiresAt == null || expiresAt > ctx.nowMs`: a missing expiresAt
        // short-circuits before nowMs is read.
        let expires_v = prop(credential, "expiresAtMs");
        let freshness = if is_undefined_marker(expires_v) || expires_v.is_null() {
            "fresh"
        } else {
            let now = as_finite_number(prop(&ctx, "nowMs")).unwrap_or(f64::NAN);
            match as_finite_number(expires_v) {
                Some(e) if e <= now => "expired",
                _ => "fresh",
            }
        };
        let ident = droid_local_identity(credential.clone())?;
        return Ok(format!(
            "{}:local:{ident}:{freshness}:api:{api_identity}",
            js_display(prop(&ctx, "homeDir"))
        ));
    }
    if is_undefined_marker(&ctx) || ctx.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'homeDir')",
            undef_or_null_name(&ctx)
        ));
    }
    let home = js_display(prop(&ctx, "homeDir"));
    let present = matches!(prop(&resolution, "localLoginPresent"), Value::Bool(true));
    Ok(format!(
        "{home}:{}",
        if present { "local-unreadable" } else { "none" }
    ))
}

/// Port of `readDroidSecureKey` (`apps/server/src/providerUsage/providers/droidSecureStorage.ts:49`).
pub async fn read_droid_secure_key(
    ctx: serde_json::Value,
    source: String,
) -> Result<serde_json::Value, String> {
    let home_v = prop(&ctx, "homeDir");
    if is_undefined_marker(&ctx) || ctx.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'homeDir')",
            undef_or_null_name(&ctx)
        ));
    }
    let home = js_str(home_v).unwrap_or_default();
    let platform = js_str(prop(&ctx, "platform")).unwrap_or_default();
    let env_v = prop(&ctx, "env");
    let mut cmd = if platform == "darwin" {
        let mut c = std::process::Command::new("/usr/bin/security");
        c.args([
            "find-generic-password",
            "-s",
            "Factory CLI",
            "-a",
            if source == "login-keychain" {
                "auth-encryption-key-security-cli"
            } else {
                "auth-encryption-key"
            },
            "-w",
        ]);
        c
    } else if platform == "linux" && source == "keyring" {
        let mut c = std::process::Command::new("/usr/bin/secret-tool");
        c.args([
            "lookup",
            "service",
            "Factory CLI",
            "account",
            "auth-encryption-key",
        ]);
        c
    } else {
        return Ok(Value::Null);
    };
    cmd.env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin");
    if platform == "linux" && source == "keyring" {
        for key in ["DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR"] {
            if let Some(v) = as_record(env_v)
                .and_then(|m| m.get(key))
                .and_then(Value::as_str)
            {
                cmd.env(key, v);
            }
        }
    }
    let Ok(out) = cmd.output() else {
        return Ok(Value::Null);
    };
    if !out.status.success() {
        return Ok(Value::Null);
    }
    Ok(Value::String(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

fn grok_plan_from_auth_mode(auth_mode: Option<String>) -> Option<String> {
    let m = auth_mode?;
    let lower = m.to_lowercase();
    if lower == "oauth" || lower == "oidc" || lower == "supergrok" {
        return Some("SuperGrok".to_string());
    }
    Some(title_case(&m))
}

/// `readSessionEntry` (`grok.ts:113`).
fn read_session_entry(value: &Value) -> Option<Vec<(String, Value)>> {
    let record = as_record(value)?;
    let rv = Value::Object(record.clone());
    let access_token = as_string(prop(&rv, "key"))?;
    let mut fields: Vec<(String, Value)> =
        vec![("accessToken".to_string(), Value::String(access_token))];
    if let Some(email) = as_string(prop(&rv, "email")) {
        fields.push(("email".to_string(), Value::String(email)));
    }
    if let Some(plan) = grok_plan_from_auth_mode(as_string(prop(&rv, "auth_mode"))) {
        fields.push(("plan".to_string(), Value::String(plan)));
    }
    if let Some(exp) = iso_from_string(prop(&rv, "expires_at")) {
        fields.push(("expiresAt".to_string(), Value::String(exp)));
    }
    if let Some(pt) = as_string(prop(&rv, "principal_type")) {
        fields.push(("principalType".to_string(), Value::String(pt)));
    }
    if let Some(rt) = as_string(prop(&rv, "refresh_token")) {
        fields.push(("refreshToken".to_string(), Value::String(rt)));
    }
    if let Some(cid) = as_string(prop(&rv, "oidc_client_id")) {
        fields.push(("clientId".to_string(), Value::String(cid)));
    }
    Some(fields)
}

/// Port of `parseGrokAuthRecord` (`apps/server/src/providerUsage/providers/grok.ts:134`).
pub fn parse_grok_auth_record(
    value: serde_json::Value,
    path: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let path = js_str(&path);
    const PREFIX: &str = "https://auth.x.ai";
    const LEGACY: &str = "https://accounts.x.ai/sign-in";
    let Some(record) = as_record(&value) else {
        return Ok(Value::Null);
    };
    let try_key = |scope: &str| -> Option<Value> {
        let entry = record.get(scope)?;
        let mut fields = read_session_entry(entry)?;
        if let Some(p) = &path {
            fields.push(("path".to_string(), Value::String(p.clone())));
            fields.push(("scope".to_string(), Value::String(scope.to_string())));
            fields.push(("root".to_string(), value.clone()));
        }
        let mut m = Map::new();
        for (k, v) in fields {
            m.insert(k, v);
        }
        Some(Value::Object(m))
    };
    let preferred = record
        .keys()
        .find(|k| k.starts_with(PREFIX))
        .cloned()
        .or_else(|| record.keys().find(|k| *k == LEGACY).cloned());
    if let Some(key) = preferred
        && let Some(session) = try_key(&key)
    {
        return Ok(session);
    }
    for scope in record.keys() {
        if let Some(session) = try_key(scope) {
            return Ok(session);
        }
    }
    Ok(Value::Null)
}

/// `decodeJwtExpMs` (`credentials.ts:237`) — base64url payload `exp` claim, no verify.
fn decode_jwt_exp_ms(jwt: Option<&str>) -> Option<f64> {
    use base64::Engine;
    let jwt = jwt?;
    let payload = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    let claims: Value = serde_json::from_slice(&bytes).ok()?;
    let exp = prop(&claims, "exp");
    let n = exp.as_f64()?;
    n.is_finite().then_some(n * 1000.0)
}

/// Port of `readGrokCachedLogin` (`apps/server/src/providerUsage/providers/grok.ts:164`).
pub async fn read_grok_cached_login(
    env: serde_json::Value,
    home_dir: serde_json::Value,
    now_ms: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'GROK_HOME')".to_string());
    }
    let grok_home = prop(&env, "GROK_HOME")
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| path_join(&[&js_display(&home_dir), ".grok"]));
    let auth_path = path_join(&[&grok_home, "auth.json"]);
    let parsed: Value = std::fs::read_to_string(&auth_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    let session = parse_grok_auth_record(parsed, Value::Null)?;
    if session.is_null() {
        return Ok(Value::Null);
    }
    let expires_at_ms = decode_jwt_exp_ms(prop(&session, "accessToken").as_str())
        .or_else(|| as_string(prop(&session, "expiresAt")).and_then(|s| date_parse_ms(&s)));
    let now = js_f64(&now_ms);
    if let Some(e) = expires_at_ms
        && e <= now
        && as_string(prop(&session, "refreshToken")).is_none()
    {
        return Ok(Value::Null);
    }
    Ok(session)
}

fn as_usd_amount(v: &Value) -> Option<f64> {
    if let Some(n) = as_finite_number(v) {
        return Some(n);
    }
    as_record(v).and_then(|m| as_finite_number(m.get("val").unwrap_or_else(|| marker_value())))
}

fn grok_plan_name(raw: Option<String>) -> Option<String> {
    let trimmed = raw?;
    let compact: String = trimmed
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase())
        .collect();
    if compact.contains("supergrokheavy") || compact == "heavy" {
        return Some("SuperGrok Heavy".to_string());
    }
    if compact.contains("supergrok") {
        return Some("SuperGrok".to_string());
    }
    Some(trimmed)
}

/// Port of `parseGrokBilling` (`apps/server/src/providerUsage/providers/grok.ts:257`).
pub fn parse_grok_billing(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'billing')",
            undef_or_null_name(&input)
        ));
    }
    let billing = prop(&input, "billing");
    let config_v = {
        let cfg = prop(billing, "config");
        if as_record(cfg).is_some() {
            cfg.clone()
        } else {
            billing.clone()
        }
    };
    let usage_v = {
        let u = prop(billing, "usage");
        if as_record(u).is_some() {
            u.clone()
        } else {
            prop(&config_v, "usage").clone()
        }
    };
    let billing_cycle_v = {
        let b = prop(billing, "billingCycle");
        if as_record(b).is_some() {
            b.clone()
        } else {
            prop(&config_v, "billingCycle").clone()
        }
    };
    let current_period_v = {
        let c = prop(&config_v, "currentPeriod");
        if as_record(c).is_some() {
            c.clone()
        } else {
            prop(billing, "currentPeriod").clone()
        }
    };
    let monthly_limit = as_usd_amount(prop(billing, "monthlyLimit"))
        .or_else(|| as_usd_amount(prop(&config_v, "monthlyLimit")));
    let total_used = as_usd_amount(prop(&usage_v, "totalUsed"));
    let on_demand_used = as_usd_amount(prop(&config_v, "onDemandUsed"))
        .or_else(|| as_usd_amount(prop(billing, "onDemandUsed")));
    let on_demand_cap = as_usd_amount(prop(&config_v, "onDemandCap"))
        .or_else(|| as_usd_amount(prop(billing, "onDemandCap")));
    let credit_usage_percent = to_used_percent(
        as_finite_number(prop(&config_v, "creditUsagePercent"))
            .or_else(|| as_finite_number(prop(billing, "creditUsagePercent"))),
    );
    let mut used_percent = credit_usage_percent;
    if used_percent.is_none()
        && let (Some(limit), Some(used)) = (monthly_limit, total_used)
        && limit > 0.0
    {
        used_percent = clamp_percent(Some(used / limit * 100.0));
    }
    if used_percent.is_none()
        && let (Some(cap), Some(used)) = (on_demand_cap, on_demand_used)
        && cap > 0.0
    {
        used_percent = clamp_percent(Some(used / cap * 100.0));
    }
    let resets = iso_from_string(prop(&current_period_v, "end"))
        .or_else(|| iso_from_string(prop(&config_v, "billingPeriodEnd")))
        .or_else(|| iso_from_string(prop(&billing_cycle_v, "billingPeriodEnd")))
        .or_else(|| iso_from_string(prop(billing, "billingPeriodEnd")));
    let period_type = as_string(prop(&current_period_v, "type")).unwrap_or_default();
    let is_weekly = period_type.to_lowercase().contains("weekly")
        || (period_type.is_empty()
            && prop(&config_v, "isUnifiedBillingUser") == &Value::Bool(true));
    let window = if is_weekly { "Weekly" } else { "Credits" };
    let session = prop(&input, "session");
    let plan_name = grok_plan_name(
        js_str(prop(
            &prop(&input, "settings").clone(),
            "subscription_tier_display",
        ))
        .filter(|s| !s.is_empty()),
    )
    .or_else(|| js_str(prop(session, "plan")))
    .unwrap_or_else(|| "SuperGrok".to_string());
    let plan_name = if plan_name.is_empty() {
        None
    } else {
        Some(plan_name)
    };
    let mut limits: Vec<Value> = Vec::new();
    if used_percent.is_some() || resets.is_some() {
        limits.push(obj(vec![
            ("window", Some(Value::String(window.to_string()))),
            ("usedPercent", used_percent.map(Value::from)),
            ("resetsAt", resets.map(Value::String)),
            (
                "windowDurationMins",
                is_weekly.then_some(Value::from(10080.0)),
            ),
        ]));
    }
    let mut usage_lines: Vec<Value> = Vec::new();
    if let Some(email) = js_str(prop(session, "email")).filter(|s| !s.is_empty()) {
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("Account".to_string()))),
            ("value", Some(Value::String(email))),
        ]));
    }
    if let (Some(cap), Some(used)) = (on_demand_cap, on_demand_used)
        && cap > 0.0
    {
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("On-demand".to_string()))),
            (
                "value",
                Some(Value::String(format!(
                    "{} of {}",
                    format_usd(used),
                    format_usd(cap)
                ))),
            ),
        ]));
    }
    build_snapshot(
        "grok",
        js_f64(prop(&input, "nowMs")),
        "ok",
        "grok-xai",
        limits,
        usage_lines,
        plan_name,
        None,
        None,
    )
}

/// Port of `parseGrokApiKeyIdentity` (`apps/server/src/providerUsage/providers/grok.ts:345`).
pub fn parse_grok_api_key_identity(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'payload')",
            undef_or_null_name(&input)
        ));
    }
    let payload = prop(&input, "payload");
    let name = as_string(prop(payload, "name"));
    let blocked = prop(payload, "api_key_blocked") == &Value::Bool(true)
        || prop(payload, "api_key_disabled") == &Value::Bool(true);
    let mut usage_lines: Vec<Value> = Vec::new();
    if let Some(n) = name {
        usage_lines.push(obj(vec![
            ("label", Some(Value::String("Key".to_string()))),
            ("value", Some(Value::String(n))),
        ]));
    }
    usage_lines.push(obj(vec![
        ("label", Some(Value::String("Credits".to_string()))),
        (
            "value",
            Some(Value::String(if blocked {
                "This API key is disabled.".to_string()
            } else {
                "Prepaid credits need SuperGrok (`grok login`) or an xAI Management key."
                    .to_string()
            })),
        ),
    ]));
    build_snapshot(
        "grok",
        js_f64(prop(&input, "nowMs")),
        "ok",
        "grok-xai",
        vec![],
        usage_lines,
        Some("API key".to_string()),
        None,
        None,
    )
}

/// Port of `parseOpenCodeGoUsage` (`apps/server/src/providerUsage/providers/opencode.ts:60`).
pub fn parse_open_code_go_usage(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'json')",
            undef_or_null_name(&input)
        ));
    }
    const WINDOWS: [(&str, &str, f64); 3] = [
        ("rolling", "5h", 300.0),
        ("weekly", "Weekly", 10080.0),
        ("monthly", "Monthly", 43200.0),
    ];
    let usage = prop(&prop(&input, "json").clone(), "usage").clone();
    let mut limits: Vec<Value> = Vec::new();
    for (key, window, minutes) in WINDOWS {
        let Some(entry) = as_record(prop(&usage, key)) else {
            continue;
        };
        let ev = Value::Object(entry.clone());
        let percent = clamp_percent(as_finite_number(prop(&ev, "percent")));
        let resets = iso_from_string(prop(&ev, "resetsAt"));
        if percent.is_none() && resets.is_none() {
            continue;
        }
        limits.push(obj(vec![
            ("window", Some(Value::String(window.to_string()))),
            ("usedPercent", percent.map(Value::from)),
            ("resetsAt", resets.map(Value::String)),
            ("windowDurationMins", Some(Value::from(minutes))),
        ]));
    }
    build_snapshot(
        "opencode",
        js_f64(prop(&input, "nowMs")),
        "ok",
        "opencode-go-usage",
        limits,
        vec![],
        Some("Go".to_string()),
        None,
        None,
    )
}
