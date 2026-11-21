//! The Rust port of `apps/desktop/src/backendSupervisionPolicy.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

const TAIL_CHARS: usize = 8_192;
const SUMMARY_MAX_LINES: f64 = 8.0;

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn trim_end_js(s: &str) -> &str {
    s.trim_end_matches(is_js_ws)
}

fn trim_js(s: &str) -> &str {
    s.trim_matches(is_js_ws)
}

fn fold_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{ff01}'..='\u{ff5e}').contains(&c) {
                char::from_u32(c as u32 - 0xfee0).unwrap_or(c)
            } else {
                c
            }
        })
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::String(s) => js_parse_number(s),
        Value::Array(items) => {
            let joined = items
                .iter()
                .map(|i| {
                    if i.is_null() || js::is_undefined(i) {
                        String::new()
                    } else {
                        js_repr_simple(i)
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            js_parse_number(&joined)
        }
        Value::Number(_) | Value::Object(_) => js::as_number(v).unwrap_or(f64::NAN),
    }
}

fn js_repr_simple(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || js::is_undefined(i) {
                    String::new()
                } else {
                    js_repr_simple(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".to_string()
            } else if let Some(n) = js::as_number(v) {
                js::number_to_string(n)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn js_parse_number(s: &str) -> f64 {
    let t = s.trim_matches(is_js_ws);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let (neg, hex) = if let Some(rest) = t.strip_prefix("-0x").or_else(|| t.strip_prefix("-0X")) {
        (true, rest)
    } else if let Some(rest) = t.strip_prefix("+0x").or_else(|| t.strip_prefix("+0X")) {
        (false, rest)
    } else {
        (
            false,
            t.strip_prefix("0x")
                .or_else(|| t.strip_prefix("0X"))
                .unwrap_or(""),
        )
    };
    if !hex.is_empty() {
        return u64::from_str_radix(hex, 16)
            .map(|n| if neg { -(n as f64) } else { n as f64 })
            .unwrap_or(f64::NAN);
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// ToIntegerOrInfinity as used by Array.prototype.slice index clamping.
fn slice_index(x: f64, len: usize) -> usize {
    if x.is_nan() {
        return 0;
    }
    let i = x.trunc();
    if i <= f64::NEG_INFINITY {
        return 0;
    }
    if i >= f64::INFINITY {
        return len;
    }
    if i < 0.0 {
        (len as f64 + i).max(0.0) as usize
    } else {
        (i as usize).min(len)
    }
}

/// Port of `backendRestartDelayMs` (`apps/desktop/src/backendSupervisionPolicy.ts:28`).
pub fn backend_restart_delay_ms(attempt: f64) -> Result<f64, String> {
    let floored = attempt.floor();
    let step = if floored.is_nan() || floored > 1.0 {
        floored
    } else {
        1.0
    } - 1.0;
    let delay = 500.0 * 2.0_f64.powf(step);
    Ok(if delay.is_nan() {
        delay
    } else {
        delay.min(10_000.0)
    })
}

/// Port of `summarizeBackendFailureOutput` (`apps/desktop/src/backendSupervisionPolicy.ts:168`).
pub fn summarize_backend_failure_output(
    output: String,
    max_lines: serde_json::Value,
) -> Result<String, String> {
    let max_lines = if js::is_undefined(&max_lines) {
        SUMMARY_MAX_LINES
    } else {
        to_number(&max_lines)
    };
    let normalized = output.replace('\r', "");
    let lines: Vec<&str> = normalized
        .split('\n')
        .map(trim_end_js)
        .filter(|line| !trim_js(line).is_empty())
        .collect();
    if lines.is_empty() {
        return Ok(String::new());
    }

    let mut error_index: Option<usize> = None;
    for (index, line) in lines.iter().enumerate().rev() {
        if fold_case(line).contains("error") {
            error_index = Some(index);
            break;
        }
    }

    let len = lines.len() as f64;
    let start_raw = match error_index {
        Some(i) => i as f64,
        None => (len - max_lines).max(0.0),
    };
    let start = slice_index(start_raw, lines.len());
    let end = slice_index(start_raw + max_lines, lines.len());
    Ok(if end > start {
        lines[start..end].join("\n")
    } else {
        String::new()
    })
}

fn string_coerce(v: &Value) -> Vec<u16> {
    match v {
        Value::String(s) => s.encode_utf16().collect(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
            .encode_utf16()
            .collect(),
        Value::Bool(b) => (if *b { "true" } else { "false" }).encode_utf16().collect(),
        Value::Null => "null".encode_utf16().collect(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || js::is_undefined(i) {
                    String::new()
                } else {
                    String::from_utf16_lossy(&string_coerce(i))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
            .encode_utf16()
            .collect(),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".encode_utf16().collect()
            } else {
                "[object Object]".encode_utf16().collect()
            }
        }
    }
}

/// Port of class `BackendOutputTailDetector` (`apps/desktop/src/backendSupervisionPolicy.ts:121`): holds the instance state.
pub struct BackendOutputTailDetector {
    chunks: Vec<Vec<u16>>,
    head: usize,
    length: usize,
}

impl BackendOutputTailDetector {
    /// Port of `BackendOutputTailDetector` constructor (`apps/desktop/src/backendSupervisionPolicy.ts:121`).
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            chunks: Vec::new(),
            head: 0,
            length: 0,
        })
    }

    /// Port of `BackendOutputTailDetector.push` (`apps/desktop/src/backendSupervisionPolicy.ts:128`).
    pub fn push(&mut self, chunk: serde_json::Value) -> Result<(), String> {
        let raw = string_coerce(&chunk);
        let text: Vec<u16> = if raw.contains(&13) {
            raw.iter().copied().filter(|&c| c != 13).collect()
        } else {
            raw
        };
        if text.is_empty() {
            return Ok(());
        }
        if text.len() >= TAIL_CHARS {
            self.chunks = vec![text[text.len() - TAIL_CHARS..].to_vec()];
            self.head = 0;
            self.length = TAIL_CHARS;
            return Ok(());
        }
        self.chunks.push(text);
        self.length += self.chunks.last().map_or(0, Vec::len);
        while self.length - self.chunks.get(self.head).map_or(0, Vec::len) >= TAIL_CHARS {
            self.length -= self.chunks.get(self.head).map_or(0, Vec::len);
            if let Some(slot) = self.chunks.get_mut(self.head) {
                *slot = Vec::new();
            }
            self.head += 1;
        }
        if self.head > 64 && self.head * 2 > self.chunks.len() {
            self.chunks.drain(..self.head);
            self.head = 0;
        }
        Ok(())
    }

    /// Port of `BackendOutputTailDetector.read` (`apps/desktop/src/backendSupervisionPolicy.ts:156`).
    pub fn read_policy_field(&mut self) -> Result<serde_json::Value, String> {
        let tail: Vec<u16> = self.chunks[self.head..]
            .iter()
            .flat_map(|c| c.iter().copied())
            .collect();
        Ok(if tail.len() > TAIL_CHARS {
            js::utf16_string(&tail[tail.len() - TAIL_CHARS..])
        } else {
            js::utf16_string(&tail)
        })
    }
}
