//! The Rust port of `apps/server/src/shell-command-detection.ts`.

#![forbid(unsafe_code)]

use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

use crate::js;

fn not_recognized_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?i)is not recognized as an internal or external command").unwrap()
    })
}

/// `String(v)` as a JS regex test coerces it.
fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| match i {
                Value::Null => String::new(),
                other => js_string(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// Port of `isWindowsShellCommandMissingResult` (`apps/server/src/shell-command-detection.ts:1`).
pub fn is_windows_shell_command_missing_result(input: serde_json::Value) -> Result<bool, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'platform')".to_string());
    }
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'platform')".to_string());
    }
    // `input.platform ?? process.platform`: null/undefined/absent fall back to the
    // host platform; a non-string value can never equal "win32" under ===.
    let is_win32 = matches!(input.get("platform"), Some(Value::String(s)) if s == "win32");
    if !is_win32 {
        return Ok(false);
    }
    if input.get("code").and_then(|c| c.as_f64()) == Some(9009.0) {
        return Ok(true);
    }
    let stderr = input.get("stderr").map_or_else(
        || "undefined".to_string(),
        |v| {
            if js::is_undefined(v) {
                "undefined".to_string()
            } else {
                js_string(v)
            }
        },
    );
    Ok(not_recognized_re().is_match(&stderr))
}
