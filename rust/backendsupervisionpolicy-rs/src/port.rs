//! The Rust port of `apps/desktop/src/backendSupervisionPolicy.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// JS `String.prototype.trimEnd` (Unicode whitespace + `FEFF`).
fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// The `Number(x)` coercion used for the optional `maxLines` argument.
fn as_count(v: &Value) -> f64 {
    if v.is_null() || is_undef(v) {
        return f64::NAN;
    }
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    match v {
        Value::Bool(b) => usize::from(*b) as f64,
        Value::String(s) => crate::js::string_units(v)
            .map(|u| {
                String::from_utf16_lossy(&u)
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(f64::NAN)
            })
            .unwrap_or_else(|| s.trim().parse::<f64>().unwrap_or(f64::NAN)),
        Value::Array(_) | Value::Object(_) => f64::NAN,
        _ => f64::NAN,
    }
}

/// JS `String(value)` for the chunks pushed into the tail detector.
fn chunk_text(v: &Value) -> String {
    if is_undef(v) {
        return "undefined".to_string();
    }
    if let Some(n) = crate::js::as_number(v) {
        return crate::js::number_to_string(n);
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(_) => js_text(v),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || is_undef(i) {
                    String::new()
                } else {
                    chunk_text(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn js_text(v: &Value) -> String {
    if let Value::String(s) = v {
        return s.clone();
    }
    crate::js::string_units(v)
        .map(|u| String::from_utf16_lossy(&u))
        .unwrap_or_default()
}

/// Port of `backendRestartDelayMs`
/// (`apps/desktop/src/backendSupervisionPolicy.ts:28`).
pub fn backend_restart_delay_ms(attempt: f64) -> Result<f64, String> {
    let delay = 500.0 * (attempt - 1.0).exp2();
    Ok(if delay.is_nan() {
        f64::NAN
    } else {
        delay.clamp(500.0, 10_000.0)
    })
}

/// Port of `summarizeBackendFailureOutput`
/// (`apps/desktop/src/backendSupervisionPolicy.ts:168`).
pub fn summarize_backend_failure_output(
    output: String,
    max_lines: Value,
) -> Result<String, String> {
    let limit = if max_lines.is_null() || is_undef(&max_lines) {
        8.0
    } else {
        as_count(&max_lines)
    };
    if limit.is_nan() || limit <= 0.0 {
        return Ok(String::new());
    }
    let lines: Vec<&str> = js_trim_end(&output).split('\n').collect();
    let len = lines.len() as f64;
    let start = len - limit;
    let head = if start < 0.0 {
        (len + start).max(0.0) as usize
    } else {
        (start as usize).min(lines.len())
    };
    Ok(lines[head..].join("\n"))
}

/// Port of the `BackendOutputTailDetector` class
/// (`apps/desktop/src/backendSupervisionPolicy.ts:121`): `push` appends the
/// `String(chunk)` text and returns nothing; `read` renders everything pushed
/// so far and never resets.
pub struct BackendOutputTailDetector {
    text: String,
}

impl BackendOutputTailDetector {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            text: String::new(),
        })
    }

    pub fn push(&mut self, chunk: Value) -> Result<(), String> {
        self.text.push_str(&chunk_text(&chunk));
        Ok(())
    }

    pub fn rendered(&self) -> Result<Value, String> {
        Ok(Value::String(self.text.clone()))
    }
}
