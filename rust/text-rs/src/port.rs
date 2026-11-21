//! The Rust port of `packages/shared/src/text.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `normalizeLineEndings`: `\r\n` and lone `\r` become `\n`.
pub fn normalize_line_endings(contents: String) -> Result<String, String> {
    let mut out = String::with_capacity(contents.len());
    let mut chars = contents.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

/// Port of `splitsSurrogatePair`: `offset` is an integer index landing
/// between a high and a low surrogate of the text.
pub fn splits_surrogate_pair(text: Value, offset_chars: f64) -> Result<bool, String> {
    if offset_chars.fract() != 0.0 || offset_chars < 1.0 {
        return Ok(false);
    }
    let u = crate::js::string_units(&text).unwrap_or_default();
    let o = offset_chars as usize;
    Ok(o < u.len() && (0xD800..=0xDBFF).contains(&u[o - 1]) && (0xDC00..=0xDFFF).contains(&u[o]))
}

/// Port of `unicodeSafeEndOffset`.
pub fn unicode_safe_end_offset(text: Value, offset: f64) -> Result<f64, String> {
    Ok(if splits_surrogate_pair(text, offset)? {
        offset - 1.0
    } else {
        offset
    })
}

/// What JavaScript's `String.prototype.trim` removes.
fn js_ws(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{b}'
            | '\x0c'
            | '\r'
            | ' '
            | '\u{a0}'
            | '\u{feff}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

/// Port of `nonEmptyTrimmed`: JS-trim a string, `undefined` when empty or
/// not a string.
pub fn non_empty_trimmed(value: Value) -> Result<Option<String>, String> {
    Ok(match &value {
        Value::String(s) => {
            let t = s.trim_matches(js_ws);
            (!t.is_empty()).then(|| t.to_string())
        }
        _ => None,
    })
}

/// Port of `stripTerminalControlSequences`: strips CSI (`ESC [`, C1 `9B`)
/// and OSC (`ESC ]`, C1 `9D`, ended by BEL, `ESC \` or `9C`) sequences.
pub fn strip_terminal_control_sequences(value: String) -> Result<String, String> {
    let cs: Vec<char> = value.chars().collect();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < cs.len() {
        let (c, n1) = (cs[i], cs.get(i + 1).copied());
        if c == '\u{9b}' || (c == '\u{1b}' && n1 == Some('[')) {
            let mut j = i + (c == '\u{1b}') as usize + 1;
            while j < cs.len() && matches!(cs[j], '0'..='?') {
                j += 1;
            }
            while j < cs.len() && matches!(cs[j], ' '..='/') {
                j += 1;
            }
            if j < cs.len() && matches!(cs[j], '@'..='~') {
                i = j + 1;
                continue;
            }
        } else if c == '\u{9d}' || (c == '\u{1b}' && n1 == Some(']')) {
            let mut j = i + (c == '\u{1b}') as usize + 1;
            let mut end = None;
            while j < cs.len() {
                if matches!(cs[j], '\u{7}' | '\u{9c}') {
                    end = Some(j + 1);
                    break;
                }
                if cs[j] == '\u{1b}' && cs.get(j + 1) == Some(&'\\') {
                    end = Some(j + 2);
                    break;
                }
                j += 1;
            }
            if let Some(e) = end {
                i = e;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    Ok(out)
}

/// JS `String(v)` for the values that can reach `+ "s"`.
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".into(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or_default()),
        Value::Object(_) if crate::js::as_number(v).is_some() => {
            crate::js::number_to_string(crate::js::as_number(v).unwrap_or_default())
        }
        Value::Object(_) if crate::js::is_undefined(v) => "undefined".into(),
        Value::Object(_) => "[object Object]".into(),
        Value::Array(a) => a
            .iter()
            .map(|i| {
                if i.is_null() || crate::js::is_undefined(i) {
                    String::new()
                } else {
                    js_str(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
    }
}

/// Port of `pluralize`: `count === 1` returns the singular verbatim;
/// `plural === undefined` appends `s`; anything else returns the plural.
pub fn pluralize(count: Value, singular: Value, plural: Value) -> Result<Value, String> {
    if matches!(crate::js::as_number(&count), Some(n) if n == 1.0) {
        return Ok(singular);
    }
    Ok(if crate::js::is_undefined(&plural) {
        Value::String(format!("{}s", js_str(&singular)))
    } else {
        plural
    })
}
