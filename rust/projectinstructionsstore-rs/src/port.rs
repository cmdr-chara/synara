//! The Rust port of `apps/web/src/projectInstructionsStore.ts`
//! (`mergeProjectInstructionsIntoThreadNotes` only).

#![forbid(unsafe_code)]

use serde_json::Value;

const THREAD_NOTES_MAX_UNITS: usize = 16_384;

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

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn trim_end(s: &str) -> &str {
    s.trim_end_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn utf16_slice(s: &str, max_units: usize) -> String {
    if s.chars().map(|c| c.len_utf16()).sum::<usize>() <= max_units {
        return s.to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for c in s.chars() {
        let u = c.len_utf16();
        if used + u > max_units {
            break;
        }
        out.push(c);
        used += u;
    }
    out
}

fn escape_reg_exp(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if ".*+?^${}()|[]\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn contains_block(current: &str, instructions: &str) -> bool {
    // Equivalent of `(?:^|\n\n)ESCAPED(?:\n\n|$)` on `current`.
    let needle = escape_reg_exp(instructions);
    let pat = format!("(?:^|\\n\\n){needle}(?:\\n\\n|$)");
    regex_lite(&pat, current)
}

fn regex_lite(pattern: &str, text: &str) -> bool {
    // The escaped needle contains no regex metacharacters, so the pattern is a
    // literal string flanked by (?:^|\n\n) ... (?:\n\n|$). Test by scanning for
    // the literal with the boundary constraints.
    let lit = pattern
        .strip_prefix("(?:^|\\n\\n)")
        .and_then(|p| p.strip_suffix("(?:\\n\\n|$)"))
        .unwrap_or(pattern);
    let lit = unescape(lit);
    for (i, _) in text.match_indices(&lit) {
        let before_ok = i == 0 || text[..i].ends_with("\n\n");
        let after = &text[i + lit.len()..];
        let after_ok = after.is_empty() || after.starts_with("\n\n");
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&n) = chars.peek()
            && ".*+?^${}()|[]\\".contains(n)
        {
            out.push(n);
            chars.next();
            continue;
        }
        out.push(c);
    }
    out
}

fn clamp(s: &str) -> String {
    utf16_slice(s, THREAD_NOTES_MAX_UNITS)
}

/// Port of `mergeProjectInstructionsIntoThreadNotes`
/// (`apps/web/src/projectInstructionsStore.ts:69`).
pub fn merge_project_instructions_into_thread_notes(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let raw_instructions = member(&input, "projectInstructions")?;
    let Value::String(raw_instructions) = &raw_instructions else {
        if raw_instructions.is_null() {
            return Err("Cannot read properties of null (reading 'trim')".to_string());
        }
        if is_undefined(&raw_instructions) {
            return Err("Cannot read properties of undefined (reading 'trim')".to_string());
        }
        return Err("input.projectInstructions.trim is not a function".to_string());
    };
    let instructions = js_trim(raw_instructions);
    let thread_notes = member(&input, "threadNotes")?;
    if instructions.is_empty() {
        return Ok(thread_notes);
    }
    let Value::String(raw_notes) = &thread_notes else {
        if thread_notes.is_null() {
            return Err("Cannot read properties of null (reading 'trim')".to_string());
        }
        if is_undefined(&thread_notes) {
            return Err("Cannot read properties of undefined (reading 'trim')".to_string());
        }
        return Err("input.threadNotes.trim is not a function".to_string());
    };
    let current = js_trim(&raw_notes.replace("\r\n", "\n")).to_string();
    if current.is_empty() {
        return Ok(Value::String(clamp(instructions)));
    }
    let instructions_lf = instructions.replace("\r\n", "\n");
    let normalized_instructions = js_trim(&instructions_lf);
    if contains_block(&current, normalized_instructions) {
        return Ok(Value::String(raw_notes.clone()));
    }
    Ok(Value::String(clamp(&format!(
        "{}\n\n{instructions}",
        trim_end(raw_notes)
    ))))
}
