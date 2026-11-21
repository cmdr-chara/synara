//! The Rust port of `packages/shared/src/errorMessages.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c.is_whitespace() || c == '\u{feff}') && c != '\u{85}')
}

fn loc_is_path(loc: &str) -> bool {
    let loc = loc.strip_prefix("file://").unwrap_or(loc);
    loc.starts_with('/')
        || (loc.len() >= 3 && loc.as_bytes()[1] == b':' && loc.as_bytes()[2] == b'\\')
}

/// `… at <tok> (<loc>)\s*$` where loc starts with `/` or `<drive>:\` and ends `:\d+`.
fn strip_paren_frame(s: &str) -> &str {
    let t = s.trim_end_matches(|c: char| c.is_whitespace() && c != '\u{85}');
    let Some(t) = t.strip_suffix(')') else {
        return s;
    };
    let Some(open) = t.rfind('(') else { return s };
    let loc = &t[open + 1..];
    if !loc_is_path(loc) || !loc.ends_with(|c: char| c.is_ascii_digit()) {
        return s;
    }
    // location must end with `:<digits>` (optionally `:<digits>:<digits>`).
    let after_colon = loc.rsplit(':').next().unwrap_or("");
    if after_colon.is_empty() || !after_colon.chars().all(|c| c.is_ascii_digit()) {
        return s;
    }
    let before = &t[..open]; // "<pre> at <tok> " — one space before '(' required.
    let Some(before) = before.strip_suffix(' ') else {
        return s;
    };
    let Some(at) = before.rfind("at ") else {
        return s;
    };
    if at == 0 {
        return s;
    }
    let fname = before[at + 3..]
        .strip_prefix("async ")
        .unwrap_or(&before[at + 3..]);
    if fname.is_empty() || fname.chars().any(|c| c.is_whitespace()) {
        return s; // single-token function name (optional `async ` prefix) only
    }
    // `at` needs whitespace before it.
    let head = &before[..at];
    head.strip_suffix(|c: char| c.is_whitespace())
        .unwrap_or(head)
}

/// `\s+at file://…$` — strips a bare `file://` frame tail.
fn strip_file_frame(s: &str) -> &str {
    let Some(at) = s.rfind("at file://") else {
        return s;
    };
    if at == 0 {
        return s;
    }
    if !s[..at].ends_with(|c: char| c.is_whitespace()) {
        return s;
    }
    s[..at].trim_end_matches(|c: char| c.is_whitespace() && c != '\u{85}')
}

/// Port of `sanitizeUserFacingErrorMessage` (`packages/shared/src/errorMessages.ts:18`).
pub fn sanitize_user_facing_error_message(
    message: String,
    fallback: String,
) -> Result<String, String> {
    let first = message.split('\n').next().unwrap_or("");
    let mut cur = js_trim(first).to_string();
    loop {
        let next = strip_file_frame(strip_paren_frame(&cur)).to_string();
        if next == cur {
            break;
        }
        cur = next;
    }
    Ok(if cur.is_empty() { fallback } else { cur })
}

fn collect(error: &Value, messages: &mut Vec<String>, depth: usize) {
    if depth > 100 {
        return;
    }
    if let Value::String(s) = error {
        let m = js_trim(s);
        if !m.is_empty() && !messages.iter().any(|x| x == m) {
            messages.push(m.to_string());
        }
        return;
    }
    let Value::Object(_) = error else {
        return;
    };
    if let Some(Value::String(m)) = error.get("message") {
        let m = js_trim(m);
        if !m.is_empty() && !messages.iter().any(|x| x == m) {
            messages.push(m.to_string());
        }
    }
    if let Some(cause) = error.get("cause") {
        collect(cause, messages, depth + 1);
    }
}

/// Port of `describeErrorMessage` (`packages/shared/src/errorMessages.ts:60`).
pub fn describe_error_message(
    error: serde_json::Value,
    fallback_message: String,
) -> Result<String, String> {
    let mut messages = Vec::new();
    collect(&error, &mut messages, 0);
    Ok(if messages.is_empty() {
        fallback_message
    } else {
        messages.join(": ")
    })
}
