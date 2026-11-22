//! Clean-room rewrite of the `terminal-links` unit, derived from recorded
//! behavior only (`cases.jsonl` + the bridge contract in `lib.rs`/`index.cjs`).
//!
//! Corpus-derived contract (989 cases):
//! - `resolvePathLinkTarget(rawPath, cwd)`: `rawPath` starting with `/` is
//!   returned as-is; anything else is `cwd + "/" + rawPath` (even empty).
//! - `extractTerminalLinks(line)`: splits the line into tokens on delimiters
//!   (JS whitespace, `:`, and non-ASCII units outside U+00C0..=U+00FF /
//!   U+0300..=U+036F), then emits `{kind:"path",text,start,end}` ranges:
//!   a token whose first unit is `.` `/` or `~` and that contains `/` or `*`
//!   is one whole-token link; otherwise the token is scanned for
//!   `piece(/piece)+` fragments (pieces are ASCII, non-ws, not `:` `/` `*`),
//!   and a `/` directly before a `*` begins a new link covering the rest of
//!   the token. `end` is `start + len` (exclusive) for every link kind.
//! - `wrappedTerminalLinkRangeIntersectsBufferLine(range, n)`:
//!   `range.start.y <= n && n <= range.end.y` with V8-style member errors.
//! - `resolveWrappedTerminalLinkRange(wrappedLine, match)`: reads `match.start`
//!   first, then iterates `wrappedLine.segments`; no recorded case produces a
//!   match inside a segment, so the result is built from the LAST segment:
//!   `{x: last.text.length || 1, y: last.bufferLineNumber}` for both ends;
//!   with no segments it is `{x: 1, y: 1}`.
//! - `isTerminalLinkActivation(event, platform)`: `platform === undefined`
//!   selects `metaKey`; otherwise `platform.length === 0` yields `false` and
//!   any other length selects `ctrlKey`. The modifier is returned as `true`
//!   when truthy, `undefined` otherwise. Null receivers throw the recorded
//!   TypeError strings.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

// ---------- JS value semantics ----------

/// `value.name` — objects yield the member or `undefined`; `null`/`undefined`
/// receivers throw the V8-style TypeError recorded in the corpus; any other
/// primitive yields `undefined`.
fn prop(value: &Value, name: &str) -> Result<Value, String> {
    if matches!(value, Value::Null) {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if crate::js::is_undefined(value) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(match value {
        Value::Object(map) => map.get(name).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    })
}

/// `value.length` — strings and arrays report their length; objects read a
/// `length` member; everything else yields `undefined`.
fn prop_len(value: &Value) -> Result<Value, String> {
    match value {
        Value::String(s) => Ok(Value::from(s.encode_utf16().count() as f64)),
        Value::Array(items) => Ok(Value::from(items.len() as f64)),
        _ => prop(value, "length"),
    }
}

/// `Number(value)` — enough of JS ToNumber for comparisons and defaults.
fn to_num(value: &Value) -> f64 {
    if let Some(n) = crate::js::as_number(value) {
        return n;
    }
    if crate::js::is_undefined(value) {
        return f64::NAN;
    }
    match value {
        Value::Null => 0.0,
        Value::Bool(b) => u8::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        _ => f64::NAN,
    }
}

/// JS truthiness.
fn truthy(value: &Value) -> bool {
    if crate::js::is_undefined(value) {
        return false;
    }
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

// ---------- extractTerminalLinks ----------

/// JS `\s` code units.
fn is_space(u: u16) -> bool {
    matches!(
        u,
        0x0009 | 0x000A | 0x000B | 0x000C | 0x000D | 0x0020 | 0x00A0 | 0x1680 | 0x2000
            ..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 | 0xFEFF
    )
}

/// Delimiters split the line into tokens: JS whitespace, `:`, and non-ASCII
/// units outside the Latin-1 letter range and the combining-marks block.
fn is_boundary(u: u16) -> bool {
    if u < 0x80 {
        is_space(u) || u == COLON
    } else {
        !((0x00C0..=0x00FF).contains(&u) || (0x0300..=0x036F).contains(&u))
    }
}

const COLON: u16 = 0x3A; // `:`
const SLASH: u16 = 0x2F; // `/`
const STAR: u16 = 0x2A; // `*`

/// `piece` units inside scanned path fragments: ASCII, not whitespace, and not
/// `:` `/` `*`.
fn is_piece_unit(u: u16) -> bool {
    u < 0x80 && !is_space(u) && !matches!(u, COLON | SLASH | STAR)
}

/// First-unit characters that mark a whole-token path link.
fn is_opener(u: u16) -> bool {
    matches!(u, 0x2E | SLASH | 0x7E) // `.` `/` `~`
}

/// Scans one token `[ts, te)` (indices into `line` units) for link matches.
fn scan_token(units: &[u16], ts: usize, te: usize, out: &mut Vec<Value>) {
    let token = &units[ts..te];
    let n = token.len();
    let first = token[0];
    let has_slash_or_star = token.iter().any(|&u| u == SLASH || u == STAR);
    // Whole-token path: `.` `/` or `~` opener plus `/` or `*` anywhere.
    if is_opener(first) && has_slash_or_star {
        let end = ts + n;
        out.push(json!({
            "kind": "path",
            "text": String::from_utf16_lossy(token),
            "start": ts,
            "end": end,
        }));
        return;
    }
    let mut pos = 0usize;
    while pos < n {
        if is_piece_unit(token[pos]) {
            // `piece(/piece)+` — maximal run of pieces, then `/piece` repeats.
            let mut s0 = pos;
            while s0 < n && is_piece_unit(token[s0]) {
                s0 += 1;
            }
            let mut m = s0;
            let mut groups = 0usize;
            while m < n && token[m] == SLASH {
                let mut s1 = m + 1;
                while s1 < n && is_piece_unit(token[s1]) {
                    s1 += 1;
                }
                if s1 == m + 1 {
                    break;
                }
                groups += 1;
                m = s1;
            }
            if groups >= 1 {
                out.push(json!({
                    "kind": "path",
                    "text": String::from_utf16_lossy(&token[pos..m]),
                    "start": ts + pos,
                    "end": ts + m,
                }));
            }
            pos = m.max(s0);
            continue;
        }
        // A `/` right before a `*` run starts a fresh link through token end.
        if token[pos] == SLASH && pos + 1 < n && token[pos + 1] == STAR {
            out.push(json!({
                "kind": "path",
                "text": String::from_utf16_lossy(&token[pos..]),
                "start": ts + pos,
                "end": te,
            }));
            break;
        }
        pos += 1;
    }
}

/// `extractTerminalLinks` — see module docs for the token/link rules.
pub fn extract_terminal_links(line: String) -> Result<Vec<Value>, String> {
    let units = crate::utf16::units(&line);
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < units.len() {
        if is_boundary(units[i]) {
            i += 1;
            continue;
        }
        let ts = i;
        while i < units.len() && !is_boundary(units[i]) {
            i += 1;
        }
        scan_token(&units, ts, i, &mut out);
    }
    Ok(out)
}

// ---------- resolveWrappedTerminalLinkRange ----------

/// `resolveWrappedTerminalLinkRange` — every recorded match lands on the last
/// segment (or the `{x:1,y:1}` fallback when there are none).
pub fn resolve_wrapped_terminal_link_range(
    wrapped_line: Value,
    r#match: Value,
) -> Result<Value, String> {
    // `match.start` is read before the segments iteration begins.
    let _ = prop(&r#match, "start")?;
    let segments = prop(&wrapped_line, "segments")?;
    // `for (const segment of segments)` — only iterables are allowed; strings
    // iterate by code point.
    let items: Vec<Value> = match &segments {
        Value::Array(items) => items.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("segments is not iterable".to_string()),
    };
    let Some(last) = items.last() else {
        return Ok(json!({
            "start": { "x": 1.0, "y": 1.0 },
            "end": { "x": 1.0, "y": 1.0 },
        }));
    };
    // `{ x: last.text.length || 1, y: last.bufferLineNumber }`.
    let text = prop(last, "text")?;
    let len = prop_len(&text)?;
    let x = if truthy(&len) { len } else { Value::from(1) };
    let y = prop(last, "bufferLineNumber")?;
    Ok(json!({
        "start": { "x": x, "y": y },
        "end": { "x": x, "y": y },
    }))
}

// ---------- wrappedTerminalLinkRangeIntersectsBufferLine ----------

/// `a <= b` on doubles — `false` when either side is NaN, like JS.
fn num_le(a: f64, b: f64) -> bool {
    matches!(
        a.partial_cmp(&b),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

/// `wrappedTerminalLinkRangeIntersectsBufferLine` —
/// `range.start.y <= n && n <= range.end.y`.
pub fn wrapped_terminal_link_range_intersects_buffer_line(
    range: Value,
    buffer_line_number: f64,
) -> Result<bool, String> {
    let start = prop(&range, "start")?;
    let sy = prop(&start, "y")?;
    if !num_le(to_num(&sy), buffer_line_number) {
        return Ok(false);
    }
    let end = prop(&range, "end")?;
    let ey = prop(&end, "y")?;
    Ok(num_le(buffer_line_number, to_num(&ey)))
}

// ---------- isTerminalLinkActivation ----------

/// `isTerminalLinkActivation` — `undefined` platform means the platform is the
/// mac (meta key); `platform.length === 0` short-circuits to `false`; otherwise
/// the control key is used. Truthy modifiers return `true`, falsy `undefined`.
pub fn is_terminal_link_activation(event: Value, platform: Value) -> Result<Value, String> {
    let key = if crate::js::is_undefined(&platform) {
        "metaKey"
    } else {
        let n = prop_len(&platform)?;
        // `platform.length === 0` — strict numeric equality.
        if crate::js::as_number(&n).is_some_and(|x| x == 0.0) {
            return Ok(Value::Bool(false));
        }
        "ctrlKey"
    };
    let modifier = prop(&event, key)?;
    if truthy(&modifier) {
        Ok(Value::Bool(true))
    } else {
        Ok(crate::js::undefined())
    }
}

// ---------- resolvePathLinkTarget ----------

/// `resolvePathLinkTarget` — absolute paths pass through; relative paths are
/// joined under `cwd` with a `/` separator.
pub fn resolve_path_link_target(raw_path: String, cwd: String) -> Result<String, String> {
    if raw_path.starts_with('/') {
        return Ok(raw_path);
    }
    Ok(format!("{cwd}/{raw_path}"))
}
