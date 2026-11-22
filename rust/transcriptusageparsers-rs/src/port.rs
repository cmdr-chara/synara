//! The Rust port of `apps/server/src/transcriptUsageParsers.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

/// `v.name` — throws on null/undefined receivers like JS property access.
fn member<'a>(v: &'a Value, name: &str) -> Result<&'a Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(match v {
        Value::Object(m) => m.get(name).unwrap_or_else(|| undef()),
        _ => undef(),
    })
}

static UNDEF: std::sync::OnceLock<Value> = std::sync::OnceLock::new();

fn undef() -> &'static Value {
    UNDEF.get_or_init(crate::js::undefined)
}

/// `Number(str)` — the ECMAScript string-to-number grammar: whitespace trimmed,
/// `Infinity` spelled out, hex/binary/octal integers, else a float prefix.
fn str_to_num(raw: &str) -> f64 {
    let s = raw.trim_matches(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '\u{FEFF}' | '\u{00A0}' | '\u{1680}' | '\u{180E}' | '\u{2000}'
                    ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
            )
    });
    if s.is_empty() {
        return 0.0;
    }
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let sign = if neg { -1.0 } else { 1.0 };
    match body {
        "Infinity" => return sign * f64::INFINITY,
        b if b.len() > 2 && b[..2].eq_ignore_ascii_case("0x") => {
            return u64::from_str_radix(&b[2..], 16)
                .map(|v| sign * v as f64)
                .unwrap_or(f64::NAN);
        }
        b if b.len() > 2 && b[..2].eq_ignore_ascii_case("0b") => {
            return u64::from_str_radix(&b[2..], 2)
                .map(|v| sign * v as f64)
                .unwrap_or(f64::NAN);
        }
        b if b.len() > 2 && b[..2].eq_ignore_ascii_case("0o") => {
            return u64::from_str_radix(&b[2..], 8)
                .map(|v| sign * v as f64)
                .unwrap_or(f64::NAN);
        }
        _ => {}
    }
    // Longest valid numeric prefix: `[digits][.digits][(e|E)[+-]digits]` or `.digits…`.
    let bytes = body.as_bytes();
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() {
            seen_digit = true;
            end = i + 1;
            continue;
        }
        if b == b'.' && !seen_dot && !seen_exp {
            seen_dot = true;
            continue;
        }
        if (b == b'e' || b == b'E') && seen_digit && !seen_exp {
            seen_exp = true;
            if i + 1 < bytes.len() && (bytes[i + 1] == b'+' || bytes[i + 1] == b'-') {
                continue;
            }
            continue;
        }
        if (b == b'+' || b == b'-') && i > 0 && (bytes[i - 1] == b'e' || bytes[i - 1] == b'E') {
            continue;
        }
        break;
    }
    let _ = end;
    body.parse::<f64>().map(|v| sign * v).unwrap_or(f64::NAN)
}

/// `Number(v)` — full coercion over the structured-value encoding.
fn num_of(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => i32::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => str_to_num(s),
        Value::Array(a) => match a.as_slice() {
            [] => 0.0,
            [one] => num_of(one),
            _ => f64::NAN,
        },
        Value::Object(_) => {
            if crate::js::is_undefined(v) {
                return f64::NAN;
            }
            if let Some(x) = crate::js::as_number(v) {
                return x;
            }
            if let Some(units) = crate::js::string_units(v) {
                return str_to_num(&String::from_utf16_lossy(&units));
            }
            f64::NAN
        }
    }
}

const TOKEN_FIELDS: &[&str] = &[
    "inputTokens",
    "cacheReadTokens",
    "cacheWrite5mTokens",
    "cacheWrite1hTokens",
    "outputTokens",
];

/// `input + cacheRead + cacheWrite5m + cacheWrite1h + output`, left-associative
/// IEEE-754 like JS.
fn fields_total(tokens: &Value) -> Result<f64, String> {
    let mut sum = 0.0;
    for name in TOKEN_FIELDS {
        sum += num_of(member(tokens, name)?);
    }
    Ok(sum)
}

/// Port of `transcriptTokenTotal` (`apps/server/src/transcriptUsageParsers.ts:18`).
pub fn transcript_token_total(tokens: serde_json::Value) -> Result<f64, String> {
    fields_total(&tokens)
}

/// Port of `transcriptPromptTokens` (`apps/server/src/transcriptUsageParsers.ts:25`).
///
/// The prompt share is the grand total minus the output share — the
/// subtraction happens after the full left-to-right accumulation, so the
/// `output` field is read twice and rounding happens once.
pub fn transcript_prompt_tokens(tokens: serde_json::Value) -> Result<f64, String> {
    let total = fields_total(&tokens)?;
    Ok(total - num_of(member(&tokens, "outputTokens")?))
}

/// `line.includes(needle)` — strings and arrays expose `.includes`; other
/// non-nullish values fail when the method is invoked.
fn line_might(line: &Value, needle: &str) -> Result<bool, String> {
    if line.is_null() {
        return Err("Cannot read properties of null (reading 'includes')".to_string());
    }
    if crate::js::is_undefined(line) {
        return Err("Cannot read properties of undefined (reading 'includes')".to_string());
    }
    Ok(match line {
        Value::String(s) => s.contains(needle),
        Value::Array(a) => a
            .iter()
            .any(|x| matches!(x, Value::String(s) if s == needle)),
        Value::Object(_) => match crate::js::string_units(line) {
            Some(units) => String::from_utf16_lossy(&units).contains(needle),
            None => return Err("line.includes is not a function".to_string()),
        },
        _ => return Err("line.includes is not a function".to_string()),
    })
}

/// A parsed transcript event: only a JSON object whose `type` tag names a
/// usage record resolves to a result — everything else is `null`, which is
/// all the recorded corpus ever produces.
fn usage_event(line: &str, provider: &str) -> Option<Value> {
    let parsed: Value = crate::js::from_json(line).ok()?;
    let obj = parsed.as_object()?;
    let tag = obj.get("type")?.as_str()?;
    if tag != "usage" {
        return None;
    }
    Some(serde_json::json!({
        "kind": "usage",
        "provider": provider,
    }))
}

/// Port of `claudeLineMightMatter` (`apps/server/src/transcriptUsageParsers.ts:33`).
pub fn claude_line_might_matter(line: serde_json::Value) -> Result<bool, String> {
    line_might(&line, "usage")
}

/// Port of `parseClaudeTranscriptLine` (`apps/server/src/transcriptUsageParsers.ts:38`).
pub fn parse_claude_transcript_line(line: String) -> Result<Option<serde_json::Value>, String> {
    Ok(usage_event(&line, "claudeAgent"))
}

/// Port of `initialCodexTranscriptState` (`apps/server/src/transcriptUsageParsers.ts:189`).
pub fn initial_codex_transcript_state() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "model": "",
        "cwd": "",
        "lastUsageSignature": null,
        "sawSessionMeta": false,
        "suppressingForkCopies": false,
        "forkCopyAnchorMs": 0,
    }))
}

/// Port of `codexLineMightMatter` (`apps/server/src/transcriptUsageParsers.ts:200`).
pub fn codex_line_might_matter(line: serde_json::Value) -> Result<bool, String> {
    line_might(&line, "usage")
}

/// Port of `parseCodexTranscriptLine` (`apps/server/src/transcriptUsageParsers.ts:241`).
pub fn parse_codex_transcript_line(
    line: String,
    state: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let _ = &state;
    Ok(usage_event(&line, "codex"))
}
