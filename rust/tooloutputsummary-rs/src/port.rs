//! The Rust port of `packages/shared/src/toolOutputSummary.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

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

fn first_line(text: &str) -> &str {
    text.split('\n').next().unwrap_or(text)
}

/// Port of `stripTrailingToolExitCode` (`packages/shared/src/toolOutputSummary.ts:18`).
pub fn strip_trailing_tool_exit_code(value: String) -> Result<serde_json::Value, String> {
    let output = if value.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(value)
    };
    Ok(serde_json::json!({ "output": output }))
}

/// Port of `countTextLines` (`packages/shared/src/toolOutputSummary.ts:38`).
pub fn count_text_lines(content: String) -> Result<f64, String> {
    Ok(line_count(&content))
}

/// Port of `summarizeToolRawOutput` (`packages/shared/src/toolOutputSummary.ts:49`).
pub fn summarize_tool_raw_output(raw_output: serde_json::Value) -> Result<Option<String>, String> {
    let obj = match raw_output.as_object() {
        Some(obj) => obj,
        None => return Ok(None),
    };

    if let Some(total) = obj.get("totalFiles").and_then(|v| v.as_f64()) {
        let truncated = obj
            .get("truncated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let unit = if total == 1.0 { "file" } else { "files" };
        let mut text = format!("{} {} found", total as u64, unit);
        if truncated {
            text.push_str(" (truncated)");
        }
        return Ok(Some(text));
    }

    if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
        let lines = line_count(content) as u64;
        let unit = if lines == 1 { "line" } else { "lines" };
        return Ok(Some(format!("Read {} {}", lines, unit)));
    }

    if let Some(stdout) = obj.get("stdout").and_then(|v| v.as_str()) {
        return Ok(Some(first_line(stdout).to_string()));
    }

    if obj.get("is_error").and_then(|v| v.as_bool()) == Some(true)
        && let Some(err) = obj
            .get("output")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("Error"))
            .and_then(|v| v.as_str())
    {
        return Ok(Some(first_line(err).to_string()));
    }

    Ok(None)
}
