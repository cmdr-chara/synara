//! The Rust port of `packages/shared/src/toolOutputSummary.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::sync::LazyLock;

macro_rules! regex {
    ($e:expr) => {{
        static R: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new($e).expect("regex compiles"));
        &*R
    }};
}

/// The JavaScript WhiteSpace + LineTerminator set (`\s`, `String#trim`).
fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{9}'..='\u{d}'
            | '\u{20}'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_ws)
}

fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(is_js_ws)
}

fn line_count(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let newlines = text.bytes().filter(|b| *b == b'\n').count() as f64;
    if text.ends_with('\n') {
        newlines
    } else {
        newlines + 1.0
    }
}

/// `firstTextLine`: non-strings are undefined; otherwise the first `\r?\n`
/// segment, JS-trimmed, or undefined when empty.
fn first_text_line(value: Option<&Value>) -> Option<String> {
    let s = value?.as_str()?;
    let trimmed = js_trim(s);
    let end = match trimmed.find('\n') {
        Some(p) if p > 0 && trimmed.as_bytes()[p - 1] == b'\r' => p - 1,
        Some(p) => p,
        None => trimmed.len(),
    };
    let line = js_trim(&trimmed[..end]);
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

/// Port of `stripTrailingToolExitCode` (`packages/shared/src/toolOutputSummary.ts:18`).
pub fn strip_trailing_tool_exit_code(value: String) -> Result<Value, String> {
    let re = regex!(
        "(?i)^<exited with exit code ([0-9]+)[\\t-\\r \\u{a0}\\u{1680}\\u{2000}-\\u{200a}\\u{2028}\\u{2029}\\u{202f}\\u{205f}\\u{3000}\\u{feff}]*$"
    );

    if let Some(idx) = value.rfind('<')
        && let Some(cap) = re.captures(&value[idx..])
    {
        let output = js_trim_end(&value[..idx]);
        let mut result = serde_json::Map::new();
        result.insert(
            "output".to_string(),
            if js_trim(output).is_empty() {
                Value::Null
            } else {
                Value::String(output.to_string())
            },
        );
        // Number.parseInt(digits, 10): f64 gives the same rounding; a value past
        // f64 range is Infinity, which Number.isInteger rejects.
        let digits = &cap[1];
        let exit_code = digits.parse::<f64>().unwrap_or(f64::INFINITY);
        if exit_code.is_finite() {
            result.insert("exitCode".to_string(), json!(exit_code));
        }
        return Ok(Value::Object(result));
    }
    Ok(json!({
        "output": if js_trim(&value).is_empty() { Value::Null } else { Value::String(value) },
    }))
}

/// Port of `countTextLines` (`packages/shared/src/toolOutputSummary.ts:38`).
pub fn count_text_lines(content: String) -> Result<f64, String> {
    Ok(line_count(&content))
}

/// `pluralize(count, "file")` / `pluralize(count, "line")` from `text.ts`.
fn pluralize(count: f64, singular: &str) -> String {
    if count == 1.0 {
        singular.to_string()
    } else {
        format!("{singular}s")
    }
}

/// Port of `summarizeToolRawOutput` (`packages/shared/src/toolOutputSummary.ts:49`).
pub fn summarize_tool_raw_output(raw_output: Value) -> Result<Option<String>, String> {
    let obj = match raw_output.as_object() {
        Some(obj) => obj,
        None => return Ok(None),
    };

    let is_error = obj.get("isError") == Some(&Value::Bool(true))
        || obj.get("is_error") == Some(&Value::Bool(true))
        || obj.get("is_error").is_some_and(|v| v.as_f64() == Some(1.0))
        || obj.get("is_error").and_then(Value::as_str) == Some("true");
    if is_error {
        let output = obj.get("output").filter(|v| v.is_object());
        let candidate = output
            .and_then(|o| o.get("Error").or_else(|| o.get("error")))
            .or_else(|| obj.get("error"))
            .or_else(|| obj.get("message"));
        if let Some(summary) = first_text_line(candidate) {
            return Ok(Some(summary));
        }
    }

    if let Some(total) = obj.get("totalFiles").and_then(Value::as_f64)
        && total.is_finite()
        && total.fract() == 0.0
        && total >= 0.0
    {
        let suffix = if obj.get("truncated") == Some(&Value::Bool(true)) {
            " (truncated)"
        } else {
            ""
        };
        return Ok(Some(format!(
            "{} {} found{suffix}",
            crate::js::number_to_string(total),
            pluralize(total, "file"),
        )));
    }

    if let Some(content) = obj.get("content").and_then(Value::as_str) {
        let lines = line_count(content);
        return Ok(Some(format!(
            "Read {} {}",
            crate::js::number_to_string(lines),
            pluralize(lines, "line"),
        )));
    }

    Ok(first_text_line(obj.get("stdout")))
}
