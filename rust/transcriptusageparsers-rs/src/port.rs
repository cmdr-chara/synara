//! The Rust port of `apps/server/src/transcriptUsageParsers.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

/// `record(value)` — objects only, arrays and null excluded.
fn record(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        _ if is_undefined(v) => None,
        Value::Object(m) => Some(m),
        _ => None,
    }
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        _ if is_undefined(v) => false,
        Value::Null | Value::Bool(false) => false,
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// `count(value)`: finite positive numbers truncated, everything else 0.
fn count(v: &Value) -> f64 {
    match crate::js::as_number(v) {
        Some(n) if n.is_finite() && n > 0.0 => n.trunc(),
        _ => 0.0,
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

/// `ToNumber` for the arithmetic the source does on record fields.
fn to_number(v: &Value) -> f64 {
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    match v {
        _ if is_undefined(v) => f64::NAN,
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::String(s) => {
            let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}');
            if t.is_empty() {
                0.0
            } else if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                u64::from_str_radix(hex, 16)
                    .map(|n| n as f64)
                    .unwrap_or(f64::NAN)
            } else {
                match t {
                    "Infinity" | "+Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    _ => t.parse::<f64>().unwrap_or(f64::NAN),
                }
            }
        }
        _ => f64::NAN,
    }
}

// --- Date.parse (ISO 8601 subset: the only form transcript timestamps use) ---

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9).rem_euclid(12);
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn parse_int(s: &[u8], from: usize, len: usize) -> Option<(i64, usize)> {
    let mut n: i64 = 0;
    let mut i = from;
    let end = (from + len).min(s.len());
    while i < end {
        if !s[i].is_ascii_digit() {
            break;
        }
        n = n * 10 + (s[i] - b'0') as i64;
        i += 1;
    }
    if i == from { None } else { Some((n, i)) }
}

/// `Date.parse` for ISO-style inputs: `YYYY[-MM[-DD]][T HH:mm[:ss[.sss]]][Z|±HH:mm]`.
/// Date-only forms are UTC (JS semantics). Anything else → `NaN` (`None`).
fn date_parse(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let (y, mut i) = parse_int(b, 0, usize::MAX)?;
    if y > 999_999 || i > 6 {
        return None;
    }
    let mut mo: i64 = 1;
    let mut d: i64 = 1;
    if i < b.len() && b[i] == b'-' {
        let (m, ni) = parse_int(b, i + 1, 2)?;
        if !(1..=12).contains(&m) || ni != i + 3 {
            return None;
        }
        mo = m;
        i = ni;
        if i < b.len() && b[i] == b'-' {
            let (dd, ni) = parse_int(b, i + 1, 2)?;
            if ni != i + 3 {
                return None;
            }
            d = dd;
            i = ni;
        }
    }
    if i < b.len() && b[i] == b'-' {
        return None;
    }
    let mut ms: f64 = 0.0;
    if i < b.len() && (b[i] == b'T' || b[i] == b' ' || b[i] == b't') {
        i += 1;
        let (hh, ni) = parse_int(b, i, 2)?;
        if ni != i + 2 || ni >= b.len() || b[ni] != b':' || hh > 24 {
            return None;
        }
        let (mm, ni) = parse_int(b, ni + 1, 2)?;
        if ni != i + 5 || mm > 59 {
            return None;
        }
        let mut ss = 0i64;
        i = ni;
        if i < b.len() && b[i] == b':' {
            let (s2, ni) = parse_int(b, i + 1, 2)?;
            if ni != i + 3 || s2 > 60 {
                return None;
            }
            ss = s2;
            i = ni;
            if i < b.len() && b[i] == b'.' {
                let mut frac = 0.0f64;
                let mut div = 1.0f64;
                let mut j = i + 1;
                let start = j;
                while j < b.len() && b[j].is_ascii_digit() {
                    frac = frac * 10.0 + (b[j] - b'0') as f64;
                    div *= 10.0;
                    j += 1;
                }
                if j == start {
                    return None;
                }
                // JS keeps at most millisecond precision.
                ms += (frac / div * 1000.0).floor();
                i = j;
            }
        }
        ms += ((hh * 60 + mm) * 60 + ss) as f64 * 1000.0;
    }
    if i >= b.len() {
        // Date-only or local-zone datetime; treated as UTC here (transcript
        // timestamps always carry `Z`).
        if d < 1 || d > days_in_month(y, mo) {
            return None;
        }
        return Some(days_from_civil(y, mo, d) as f64 * 86_400_000.0 + ms);
    }
    match b[i] {
        b'Z' | b'z' if i == b.len() - 1 => {}
        b'+' | b'-' => {
            let sign = if b[i] == b'-' { -1.0 } else { 1.0 };
            let (oh, ni) = parse_int(b, i + 1, 2)?;
            if ni != i + 3 || oh > 23 {
                return None;
            }
            let om = if ni < b.len() && b[ni] == b':' {
                let (m, nj) = parse_int(b, ni + 1, 2)?;
                if nj != ni + 3 {
                    return None;
                }
                if nj != b.len() {
                    return None;
                }
                m
            } else if ni < b.len() {
                let (m, nj) = parse_int(b, ni, 2)?;
                if nj != ni + 2 || nj != b.len() {
                    return None;
                }
                m
            } else {
                0
            };
            if om > 59 {
                return None;
            }
            ms -= sign * (oh * 60 + om) as f64 * 60_000.0;
        }
        _ => return None,
    }
    if d < 1 || d > days_in_month(y, mo) {
        return None;
    }
    Some(days_from_civil(y, mo, d) as f64 * 86_400_000.0 + ms)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
    }
}

fn timestamp(v: &Value) -> Option<f64> {
    match v {
        Value::String(s) => date_parse(s),
        _ => None,
    }
}

fn parse_json(line: &str) -> Option<Value> {
    serde_json::from_str(line).ok()
}

// --- JSON.stringify for the duplicate-detection signature -------------------

fn js_escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn js_json_stringify_into(v: &Value, out: &mut String) {
    match v {
        _ if is_undefined(v) => out.push_str("null"),
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&crate::js::number_to_string(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => js_escape(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, e) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                js_json_stringify_into(e, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            for (i, (k, e)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                js_escape(k, out);
                out.push(':');
                js_json_stringify_into(e, out);
            }
            out.push('}');
        }
    }
}

fn js_json_stringify(v: &Value) -> String {
    let mut out = String::new();
    js_json_stringify_into(v, &mut out);
    out
}

// --- shared `includes` gate for the Buffer line -----------------------------

/// `line.includes(needle)` where the bridge passes the wire value verbatim:
/// strings scan text, arrays strict-compare elements, everything else throws
/// (or the property-access error for null/undefined).
fn buffer_includes(line: &Value, needle: &str) -> Result<bool, String> {
    match line {
        Value::Null => Err("Cannot read properties of null (reading 'includes')".to_string()),
        _ if is_undefined(line) => {
            Err("Cannot read properties of undefined (reading 'includes')".to_string())
        }
        Value::String(s) => Ok(s.contains(needle)),
        Value::Array(a) => Ok(a
            .iter()
            .any(|e| matches!(e, Value::String(s) if s == needle))),
        _ => Err("line.includes is not a function".to_string()),
    }
}

// ── ports ─────────────────────────────────────────────────────────────────

/// Port of `transcriptTokenTotal` (`apps/server/src/transcriptUsageParsers.ts:57`).
pub fn transcript_token_total(tokens: serde_json::Value) -> Result<f64, String> {
    Ok(to_number(&member(&tokens, "inputTokens")?)
        + to_number(&member(&tokens, "cacheReadTokens")?)
        + to_number(&member(&tokens, "cacheWrite5mTokens")?)
        + to_number(&member(&tokens, "cacheWrite1hTokens")?)
        + to_number(&member(&tokens, "outputTokens")?))
}

/// Port of `transcriptPromptTokens` (`apps/server/src/transcriptUsageParsers.ts:68`).
pub fn transcript_prompt_tokens(tokens: serde_json::Value) -> Result<f64, String> {
    Ok(transcript_token_total(tokens.clone())? - to_number(&member(&tokens, "outputTokens")?))
}

/// Port of `claudeLineMightMatter` (`apps/server/src/transcriptUsageParsers.ts:107`).
pub fn claude_line_might_matter(line: serde_json::Value) -> Result<bool, String> {
    if buffer_includes(&line, "\"usage\"")? && buffer_includes(&line, "\"assistant\"")? {
        return Ok(true);
    }
    Ok(buffer_includes(&line, "\"type\":\"user\"")? && !buffer_includes(&line, "\"tool_use_id\"")?)
}

/// `isClaudePrompt` — text content, not a tool result, meta line or subagent.
fn is_claude_prompt(parsed: &Map<String, Value>) -> Result<bool, String> {
    for flag in ["isMeta", "isSidechain", "isCompactSummary"] {
        if parsed.get(flag) == Some(&Value::Bool(true)) {
            return Ok(false);
        }
    }
    let message = member(&Value::Object(parsed.clone()), "message")?;
    let content = match record(&message) {
        Some(m) => m
            .get("content")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        None => crate::js::undefined(),
    };
    match &content {
        Value::String(s) => Ok(!s
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
            .is_empty()),
        Value::Array(parts) => {
            let has_text = parts.iter().any(|p| {
                record(p).is_some_and(|r| r.get("type") == Some(&Value::String("text".into())))
            });
            let has_tool = parts.iter().any(|p| {
                record(p)
                    .is_some_and(|r| r.get("type") == Some(&Value::String("tool_result".into())))
            });
            Ok(has_text && !has_tool)
        }
        _ => Ok(false),
    }
}

/// Port of `parseClaudeTranscriptLine` (`apps/server/src/transcriptUsageParsers.ts:127`).
pub fn parse_claude_transcript_line(line: String) -> Result<Option<serde_json::Value>, String> {
    let parsed_value = parse_json(&line).unwrap_or_else(crate::js::undefined);
    let Some(parsed) = record(&parsed_value) else {
        return Ok(None);
    };
    let timestamp_ms = match timestamp(&member(&parsed_value, "timestamp")?) {
        Some(t) => t,
        None => return Ok(None),
    };

    let type_value = parsed
        .get("type")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    if type_value == Value::String("user".to_string()) {
        let uuid = text(&member(&parsed_value, "uuid")?);
        if uuid.is_empty() || !is_claude_prompt(parsed)? {
            return Ok(None);
        }
        return Ok(Some(json!({
            "kind": "turn",
            "provider": "claudeAgent",
            "timestampMs": crate::js::number(timestamp_ms),
            "model": "",
            "cwd": text(&member(&parsed_value, "cwd")?),
            "dedupeKey": format!("prompt:{uuid}"),
        })));
    }
    if type_value != Value::String("assistant".to_string()) {
        return Ok(None);
    }

    let message_value = member(&parsed_value, "message")?;
    let Some(message) = record(&message_value) else {
        return Ok(None);
    };
    let usage_value = message
        .get("usage")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let Some(usage) = record(&usage_value) else {
        return Ok(None);
    };
    let model = text(
        &message
            .get("model")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
    );
    let failed = parsed.get("isApiErrorMessage") == Some(&Value::Bool(true));
    if model.is_empty() && !failed {
        return Ok(None);
    }

    let message_id = text(
        &message
            .get("id")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
    );
    let request_id = text(&member(&parsed_value, "requestId")?);
    let uuid = text(&member(&parsed_value, "uuid")?);
    let dedupe_key: Value = if !message_id.is_empty() || !request_id.is_empty() {
        Value::String(format!("{message_id}:{request_id}"))
    } else if failed && !uuid.is_empty() {
        Value::String(format!("error:{uuid}"))
    } else {
        Value::Null
    };

    let usage_member =
        |key: &str| -> Value { usage.get(key).cloned().unwrap_or_else(crate::js::undefined) };
    let cache_write = count(&usage_member("cache_creation_input_tokens"));
    let cache_write_1h = {
        let inner = usage
            .get("cache_creation")
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        let part = record(&inner)
            .and_then(|r| r.get("ephemeral_1h_input_tokens"))
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        count(&part).min(cache_write)
    };
    let cost = member(&parsed_value, "costUSD")?;
    let reported_cost = match crate::js::as_number(&cost) {
        Some(n) if n.is_finite() && n >= 0.0 => crate::js::number(n),
        _ => Value::Null,
    };
    Ok(Some(json!({
        "kind": "usage",
        "provider": "claudeAgent",
        "timestampMs": crate::js::number(timestamp_ms),
        "model": model,
        "cwd": text(&member(&parsed_value, "cwd")?),
        "tokens": {
            "inputTokens": crate::js::number(count(&usage_member("input_tokens"))),
            "cacheReadTokens": crate::js::number(count(&usage_member("cache_read_input_tokens"))),
            "cacheWrite5mTokens": crate::js::number(cache_write - cache_write_1h),
            "cacheWrite1hTokens": crate::js::number(cache_write_1h),
            "outputTokens": crate::js::number(count(&usage_member("output_tokens"))),
        },
        "reportedCostUsd": reported_cost,
        "fast": usage_member("speed") == Value::String("fast".to_string()),
        "failed": failed,
        "dedupeKey": dedupe_key,
    })))
}

/// Port of `initialCodexTranscriptState` (`apps/server/src/transcriptUsageParsers.ts:203`).
pub fn initial_codex_transcript_state() -> Result<serde_json::Value, String> {
    Ok(json!({
        "model": "",
        "cwd": "",
        "lastUsageSignature": null,
        "sawSessionMeta": false,
        "suppressingForkCopies": false,
        "forkCopyAnchorMs": 0,
    }))
}

/// Port of `codexLineMightMatter` (`apps/server/src/transcriptUsageParsers.ts:221`).
pub fn codex_line_might_matter(line: serde_json::Value) -> Result<bool, String> {
    Ok(buffer_includes(&line, "\"token_count\"")?
        || buffer_includes(&line, "\"turn_context\"")?
        || buffer_includes(&line, "\"session_meta\"")?)
}

fn state_member(state: &Value, key: &str) -> Value {
    member(state, key).unwrap_or_else(|_| crate::js::undefined())
}

fn is_forked_session_meta(payload: &Map<String, Value>) -> bool {
    if matches!(payload.get("forked_from_id"), Some(Value::String(_)))
        || matches!(payload.get("parent_thread_id"), Some(Value::String(_)))
    {
        return true;
    }
    let source = payload.get("source").and_then(record);
    let subagent = source.and_then(|r| r.get("subagent")).and_then(record);
    let spawn = subagent
        .and_then(|r| r.get("thread_spawn"))
        .and_then(record);
    matches!(
        spawn.and_then(|r| r.get("parent_thread_id")),
        Some(Value::String(_))
    )
}

const FORK_COPY_MAX_GAP_MS: f64 = 1000.0;

/// Port of `parseCodexTranscriptLine` (`apps/server/src/transcriptUsageParsers.ts:241`).
pub fn parse_codex_transcript_line(
    line: String,
    state: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let parsed_value = parse_json(&line).unwrap_or_else(crate::js::undefined);
    let Some(parsed) = record(&parsed_value) else {
        return Ok(None);
    };
    let payload_value = parsed
        .get("payload")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let Some(payload) = record(&payload_value) else {
        return Ok(None);
    };
    let timestamp_ms = timestamp(&member(&parsed_value, "timestamp")?);

    let mut state = state;
    let set_state = |key: &str, v: Value, state: &mut Value| {
        if let Value::Object(m) = state {
            m.insert(key.to_string(), v);
        }
    };

    let type_value = parsed
        .get("type")
        .cloned()
        .unwrap_or_else(crate::js::undefined);

    if type_value == Value::String("session_meta".to_string()) {
        if truthy(&state_member(&state, "sawSessionMeta")) {
            return Ok(None);
        }
        set_state("sawSessionMeta", Value::Bool(true), &mut state);
        if !truthy(&state_member(&state, "cwd")) {
            let cwd = payload
                .get("cwd")
                .cloned()
                .unwrap_or_else(crate::js::undefined);
            set_state("cwd", Value::String(text(&cwd)), &mut state);
        }
        if timestamp_ms.is_some() && is_forked_session_meta(payload) {
            set_state("suppressingForkCopies", Value::Bool(true), &mut state);
            set_state(
                "forkCopyAnchorMs",
                crate::js::number(timestamp_ms.unwrap_or(0.0)),
                &mut state,
            );
        }
        return Ok(None);
    }

    if type_value == Value::String("turn_context".to_string()) {
        if let Some(Value::String(m)) = payload.get("model") {
            set_state("model", Value::String(m.clone()), &mut state);
        }
        if let Some(Value::String(c)) = payload.get("cwd")
            && !c.is_empty()
        {
            set_state("cwd", Value::String(c.clone()), &mut state);
        }
        let turn_id = match payload.get("turn_id") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        let Some(ts) = timestamp_ms else {
            return Ok(None);
        };
        if turn_id.is_empty() {
            return Ok(None);
        }
        return Ok(Some(json!({
            "kind": "turn",
            "provider": "codex",
            "timestampMs": crate::js::number(ts),
            "model": state_member(&state, "model"),
            "cwd": state_member(&state, "cwd"),
            "dedupeKey": format!("turn:{turn_id}"),
        })));
    }

    if payload.get("type") != Some(&Value::String("token_count".to_string())) {
        return Ok(None);
    }
    let info = payload.get("info").and_then(record);
    let last_value = info
        .and_then(|r| r.get("last_token_usage"))
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let Some(last) = record(&last_value) else {
        return Ok(None);
    };
    let Some(ts) = timestamp_ms else {
        return Ok(None);
    };
    if !truthy(&state_member(&state, "model")) {
        return Ok(None);
    }

    let signature = js_json_stringify(&last_value);
    if matches!(&state_member(&state, "lastUsageSignature"), Value::String(s) if s == &signature) {
        return Ok(None);
    }
    set_state("lastUsageSignature", Value::String(signature), &mut state);

    if truthy(&state_member(&state, "suppressingForkCopies")) {
        let anchor = to_number(&state_member(&state, "forkCopyAnchorMs"));
        if ts - anchor < FORK_COPY_MAX_GAP_MS {
            set_state("forkCopyAnchorMs", crate::js::number(ts), &mut state);
            return Ok(None);
        }
        set_state("suppressingForkCopies", Value::Bool(false), &mut state);
    }

    let last_member =
        |key: &str| -> Value { last.get(key).cloned().unwrap_or_else(crate::js::undefined) };
    let input = count(&last_member("input_tokens"));
    let cache_read = count(&last_member("cached_input_tokens"));
    let cache_write = count(&last_member("cache_write_input_tokens"));
    let tokens = [
        ("inputTokens", (input - cache_read - cache_write).max(0.0)),
        ("cacheReadTokens", cache_read),
        ("cacheWrite5mTokens", cache_write),
        ("cacheWrite1hTokens", 0.0),
        ("outputTokens", count(&last_member("output_tokens"))),
    ];
    if tokens.iter().map(|(_, n)| *n).sum::<f64>() == 0.0 {
        return Ok(None);
    }
    let mut token_map = Map::new();
    for (k, n) in tokens {
        token_map.insert(k.to_string(), crate::js::number(n));
    }
    Ok(Some(json!({
        "kind": "usage",
        "provider": "codex",
        "timestampMs": crate::js::number(ts),
        "model": state_member(&state, "model"),
        "cwd": state_member(&state, "cwd"),
        "tokens": Value::Object(token_map),
        "reportedCostUsd": null,
        "fast": false,
        "failed": false,
        "dedupeKey": null,
    })))
}
