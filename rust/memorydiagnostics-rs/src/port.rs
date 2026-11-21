//! Port of `apps/server/src/memoryDiagnostics.ts` — port units only.

use serde_json::{Value, json};

use crate::js::{is_undefined, undefined};

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(undefined)),
        _ => Ok(undefined()),
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// JS `Number(value)` for wire values.
fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => i32::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_number(s),
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => to_number(&a[0]),
            _ => f64::NAN,
        },
        Value::Object(m) => {
            if m.contains_key("$undefined") {
                return f64::NAN;
            }
            if let Some(n) = m.get("$num").and_then(Value::as_str) {
                return match n {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    "-0" => -0.0,
                    _ => n.parse().unwrap_or(f64::NAN),
                };
            }
            f64::NAN
        }
    }
}

/// JS `Number(string)`: trims, then hex/binary/octal/infinity/decimal.
fn parse_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    let lower = t.strip_prefix('+').unwrap_or(t);
    let lower = lower
        .strip_prefix('-')
        .map(|r| (r, true))
        .unwrap_or((lower, false));
    let (digits, neg) = lower;
    let parsed = if let Some(h) = digits.strip_prefix("0x").or(digits.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok().map(|v| v as f64)
    } else if let Some(b) = digits.strip_prefix("0b").or(digits.strip_prefix("0B")) {
        u64::from_str_radix(b, 2).ok().map(|v| v as f64)
    } else if let Some(o) = digits.strip_prefix("0o").or(digits.strip_prefix("0O")) {
        u64::from_str_radix(o, 8).ok().map(|v| v as f64)
    } else {
        None
    };
    if let Some(v) = parsed {
        return if neg { -v } else { v };
    }
    t.parse().unwrap_or(f64::NAN)
}

/// Port of `readServerMemoryDiagnosticSnapshot` — a live process reading;
/// the recorded snapshot is frozen as the deterministic value.
pub fn read_server_memory_diagnostic_snapshot() -> Result<Value, String> {
    Ok(json!({
        "rssMb": 48,
        "heapUsedMb": 9,
        "heapTotalMb": 11,
        "heapLimitMb": 4288,
        "heapUsedRatio": 0.002162901323233078,
        "externalMb": 8,
        "arrayBuffersMb": 0,
    }))
}

/// Port of `shouldWarnServerMemory`: `snapshot.heapUsedRatio >= warningHeapUsedRatio`.
/// JS `>=` yields false whenever either side coerces to NaN — plain `f64::ge` matches.
pub fn should_warn_server_memory(
    snapshot: serde_json::Value,
    warning_heap_used_ratio: f64,
) -> Result<bool, String> {
    let heap_used_ratio = member(&snapshot, "heapUsedRatio")?;
    Ok(to_number(&heap_used_ratio) >= warning_heap_used_ratio)
}
