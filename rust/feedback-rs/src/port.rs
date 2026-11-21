//! The Rust port of `apps/web/src/feedback.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

use crate::js;

const APP_VERSION: &str = "0.0.0";
const UNCATEGORIZED_LEAD: &str = "I have some feedback";

const FEEDBACK_CATEGORIES: [(&str, &str, &str); 6] = [
    ("bug", "Bug", "I ran into a bug"),
    ("session", "Session", "I hit a session problem"),
    ("ui", "UI", "Something looked wrong"),
    ("performance", "Performance", "Synara felt slow"),
    ("idea", "Idea", "I have an idea"),
    ("other", "Other", "I have some feedback"),
];

fn is_nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    if is_nullish(v) {
        return false;
    }
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Object(o) => {
            if let Some(m) = o.get("$num") {
                return truthy(m);
            }
            true
        }
        _ => true,
    }
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) => o.get(key),
        _ => None,
    }
}

fn get(v: &Value, key: &str) -> Value {
    prop(v, key).cloned().unwrap_or_else(js::undefined)
}

/// JS `${value}` template interpolation.
fn js_str(v: &Value) -> String {
    if js::is_undefined(v) {
        return "undefined".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => js::number_to_string(js::as_number(v).unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_str).collect::<Vec<_>>().join(","),
        Value::Object(o) => {
            if o.contains_key("$num") {
                return js::number_to_string(js::as_number(v).unwrap_or(f64::NAN));
            }
            if o.contains_key("$undefined") {
                return "undefined".to_string();
            }
            "[object Object]".to_string()
        }
    }
}

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{09}'..='\u{0D}'
            | '\u{20}'
            | '\u{A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

fn js_trim(s: &str) -> String {
    s.trim_matches(is_js_ws).to_string()
}

fn category_of(input: &Value) -> Option<&'static (&'static str, &'static str, &'static str)> {
    FEEDBACK_CATEGORIES
        .iter()
        .find(|(value, _, _)| prop(input, "category").is_some_and(|c| c.as_str() == Some(*value)))
}

fn format_state_flags(diagnostics: &Value) -> String {
    let mut flags = Vec::new();
    if truthy(&get(diagnostics, "hasThreadError")) {
        flags.push("the thread was in an error state");
    }
    if truthy(&get(diagnostics, "hasPendingApproval")) {
        flags.push("an approval was pending");
    }
    if truthy(&get(diagnostics, "hasPendingUserInput")) {
        flags.push("the agent was waiting for input");
    }
    if flags.is_empty() {
        "nothing pending.".to_string()
    } else {
        format!("{}.", flags.join(", "))
    }
}

fn summary_with_diagnostics(category: Option<&(&str, &str, &str)>, diagnostics: &Value) -> String {
    let lead = category.map(|c| c.2).unwrap_or(UNCATEGORIZED_LEAD);
    let provider = get(diagnostics, "provider");
    let model = get(diagnostics, "model");
    let provider = &provider;
    let model = &model;
    let usage_context = if truthy(provider) {
        if truthy(model) {
            format!(", using {} with {}", js_str(provider), js_str(model))
        } else {
            format!(", using {}", js_str(provider))
        }
    } else {
        " outside an active chat".to_string()
    };
    let head = format!(
        "{} in Synara {}{}.",
        lead,
        js_str(&get(diagnostics, "appVersion")),
        usage_context
    );
    let label = category.map(|c| c.1).unwrap_or("Unspecified");
    let thread_size = format!(
        "{} messages, {} activities",
        js_str(&get(diagnostics, "messageCount")),
        js_str(&get(diagnostics, "activityCount"))
    );
    let platform_line = format!(
        "{}, viewport {}",
        js_str(&get(diagnostics, "platform")),
        js_str(&get(diagnostics, "viewport"))
    );
    let rows: [(&str, Value); 16] = [
        ("Report type", json!(label)),
        ("App version", get(diagnostics, "appVersion").clone()),
        ("Provider", provider.clone()),
        ("Model", model.clone()),
        ("Project kind", get(diagnostics, "projectKind").clone()),
        (
            "Environment mode",
            get(diagnostics, "environmentMode").clone(),
        ),
        ("Runtime mode", get(diagnostics, "runtimeMode").clone()),
        (
            "Interaction mode",
            get(diagnostics, "interactionMode").clone(),
        ),
        ("Session status", get(diagnostics, "sessionStatus").clone()),
        (
            "Latest turn state",
            get(diagnostics, "latestTurnState").clone(),
        ),
        ("Thread size", json!(thread_size)),
        ("At submission", json!(format_state_flags(diagnostics))),
        ("Platform", json!(platform_line)),
        ("Language", get(diagnostics, "language").clone()),
        ("User agent", get(diagnostics, "userAgent").clone()),
        ("Submitted at", get(diagnostics, "submittedAt").clone()),
    ];
    let mut lines = vec![head, String::new()];
    for (name, value) in rows {
        if value.is_null() || value.as_str() == Some("") {
            continue;
        }
        lines.push(format!("{}: {}", name, js_str(&value)));
    }
    lines.join("\n")
}

/// Port of `formatFeedbackSummary` (`apps/web/src/feedback.ts:73`).
pub fn format_feedback_summary(input: serde_json::Value) -> Result<String, String> {
    if is_nullish(&input) {
        return Err(
            "Cannot destructure property 'diagnostics' from null or undefined value".to_string(),
        );
    }
    let diagnostics = prop(&input, "diagnostics");
    match diagnostics {
        Some(d) if !is_nullish(d) => Ok(summary_with_diagnostics(category_of(&input), d)),
        _ => Err("undefined is not an object (evaluating 'diagnostics.provider')".to_string()),
    }
}

/// `Date.now()`. The corpus was recorded against a shim freezing it; the env
/// var replays that instant for `diff`/`gate`.
fn volatile_now_ms() -> f64 {
    std::env::var("SYNARA_FEEDBACK_NOW_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0)
        })
}

fn iso_from_ms(ms: f64) -> Option<String> {
    if !ms.is_finite() {
        return None;
    }
    let total_ms = ms.trunc() as i64;
    let mss = total_ms.rem_euclid(1000);
    let secs = total_ms.div_euclid(1000);
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let sec = tod % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    if !(0..=9999).contains(&year) {
        return None;
    }
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hour, min, sec, mss
    ))
}

fn submitted_at(now: &Value) -> Result<String, String> {
    if is_nullish(now) {
        return iso_from_ms(volatile_now_ms()).ok_or_else(|| "Invalid time value".to_string());
    }
    if let Some(d) = prop(now, "$date").and_then(|d| d.as_str()) {
        if d == "Invalid" {
            return Err("Invalid time value".to_string());
        }
        return Ok(d.to_string());
    }
    Err("(input.now ?? new Date).toISOString is not a function. (In '(input.now ?? new Date).toISOString()', '(input.now ?? new Date).toISOString' is undefined)".to_string())
}

fn spread_context(context: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    match context {
        Value::Object(o) => {
            for (k, v) in o {
                out.insert(k.clone(), v.clone());
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                out.insert(i.to_string(), v.clone());
            }
        }
        Value::String(s) => {
            for (i, c) in s.chars().enumerate() {
                out.insert(i.to_string(), json!(c.to_string()));
            }
        }
        _ => {}
    }
    out
}

/// Port of `buildFeedbackSubmission` (`apps/web/src/feedback.ts:117`).
pub fn build_feedback_submission(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_nullish(&input) {
        return Err("null is not an object (evaluating 'input.viewport')".to_string());
    }
    let viewport = prop(&input, "viewport").cloned().filter(|v| !is_nullish(v));
    let viewport = match viewport {
        Some(v) => v,
        None => return Err("window is not defined".to_string()),
    };
    let mut diagnostics = spread_context(prop(&input, "context").unwrap_or(&Value::Null));
    diagnostics.insert("appVersion".to_string(), json!(APP_VERSION));
    diagnostics.insert(
        "submittedAt".to_string(),
        json!(submitted_at(&get(&input, "now"))?),
    );
    diagnostics.insert(
        "userAgent".to_string(),
        prop(&input, "userAgent")
            .filter(|v| !is_nullish(v))
            .cloned()
            .unwrap_or_else(|| json!("Bun/1.4.5")),
    );
    diagnostics.insert(
        "platform".to_string(),
        prop(&input, "platform")
            .filter(|v| !is_nullish(v))
            .cloned()
            .unwrap_or_else(|| json!("MacIntel")),
    );
    diagnostics.insert(
        "language".to_string(),
        prop(&input, "language")
            .filter(|v| !is_nullish(v))
            .cloned()
            .unwrap_or_else(js::undefined),
    );
    diagnostics.insert(
        "viewport".to_string(),
        json!(format!(
            "{}x{}",
            js_str(&get(&viewport, "width")),
            js_str(&get(&viewport, "height"))
        )),
    );
    let diagnostics = Value::Object(diagnostics);
    let details = match prop(&input, "details") {
        Some(Value::String(s)) => js_trim(s),
        Some(v) if is_nullish(v) => {
            return Err("undefined is not an object (evaluating 'input.details.trim')".to_string());
        }
        Some(_) => {
            return Err("input.details.trim is not a function".to_string());
        }
        None => {
            return Err("undefined is not an object (evaluating 'input.details.trim')".to_string());
        }
    };
    let category = prop(&input, "category")
        .cloned()
        .unwrap_or_else(js::undefined);
    let summary = summary_with_diagnostics(category_of(&input), &diagnostics);
    Ok(json!({
        "category": category,
        "details": details,
        "summary": summary,
        "diagnostics": diagnostics,
    }))
}
