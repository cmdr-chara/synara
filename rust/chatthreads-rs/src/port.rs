//! The Rust port of `packages/shared/src/chatThreads.ts`.
//!
//! JS `length`/`slice` count UTF-16 code units, so truncation goes through `utf16`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::{js, utf16};

const GENERIC_TITLE: &str = "New thread";
const MAX_TITLE_LEN: f64 = 60.0;
const CONTEXT_MAX_CHARS: f64 = 8_000.0;
const CONTEXT_MAX_MESSAGES: usize = 12;
const MESSAGE_MAX_CHARS: usize = 1_200;
const MAX_TITLE_WORDS: usize = 6;

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'
            | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// `value.replace(/\s+/g, " ").trim()`.
fn normalize_title_whitespace(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_ws = false;
    for c in value.chars() {
        if is_js_space(c) {
            in_ws = true;
        } else {
            if in_ws {
                out.push(' ');
            }
            out.push(c);
            in_ws = false;
        }
    }
    out.trim_matches(is_js_space).to_string()
}

/// `/^[\s"'`([{]+|[\s"'`)\]}:;,.!?]+$/g` trim.
fn trim_title_token(token: &str) -> &str {
    token
        .trim_start_matches(|c: char| {
            is_js_space(c) || matches!(c, '"' | '\'' | '`' | '(' | '[' | '{')
        })
        .trim_end_matches(|c: char| {
            is_js_space(c)
                || matches!(
                    c,
                    '"' | '\'' | '`' | ')' | ']' | '}' | ':' | ';' | ',' | '.' | '!' | '?'
                )
        })
}

fn title_words(value: &str) -> Vec<String> {
    normalize_title_whitespace(value)
        .split(' ')
        .map(trim_title_token)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// `removeReasoningWrappers`: drop `<analysis|reasoning|think>...</>` pairs
/// (case-insensitive, dotall), then an unclosed trailing wrapper.
/// ASCII-case-insensitive `starts_with` (the wrapper tags are ASCII-only, so no
/// Unicode case folding ever applies — `String::to_lowercase` would also change
/// byte lengths, e.g. `İ` → `i̇`, breaking the indices).
fn starts_with_tag(s: &str, tag: &str) -> bool {
    s.len() >= tag.len() && s.as_bytes()[..tag.len()].eq_ignore_ascii_case(tag.as_bytes())
}

/// Nearest closing `</analysis|reasoning|think>` — the JS regex's two
/// alternations are independent, so the closer need not match the opener.
fn find_close_tag(s: &str, closes: &[&str; 3]) -> Option<(usize, usize)> {
    s.match_indices('<').map(|(i, _)| i).find_map(|i| {
        closes
            .iter()
            .find(|tag| starts_with_tag(&s[i..], tag))
            .map(|tag| (i, tag.len()))
    })
}

fn remove_reasoning_wrappers(value: &str) -> String {
    let tags = ["<analysis>", "<reasoning>", "<think>"];
    let closes = ["</analysis>", "</reasoning>", "</think>"];
    let mut out = String::with_capacity(value.len());
    let mut i = 0usize;
    while i < value.len() {
        let open = tags
            .iter()
            .position(|tag| starts_with_tag(&value[i..], tag));
        match open {
            Some(t) => {
                let after = i + tags[t].len();
                if let Some((pos, len)) = find_close_tag(&value[after..], &closes) {
                    i = after + pos + len;
                } else {
                    // Unclosed wrapper: remove to end of string.
                    break;
                }
            }
            None => {
                let c = value[i..].chars().next();
                if let Some(c) = c {
                    out.push(c);
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
        }
    }
    out
}

fn first_generated_title_line(value: &str) -> String {
    let stripped = remove_reasoning_wrappers(value);
    for line in stripped.split('\n') {
        let line = line.trim_matches(is_js_space);
        if !line.is_empty() && !line.starts_with("```") {
            return line
                .trim_end_matches('\r')
                .trim_matches(is_js_space)
                .to_string();
        }
    }
    String::new()
}

/// `truncateConversationMessage`: head 60% of 1197 chars, tail 479 (UTF-16).
fn truncate_conversation_message(value: &str) -> String {
    let normalized = normalize_title_whitespace(value);
    let units = utf16::units(&normalized);
    if units.len() <= MESSAGE_MAX_CHARS {
        return normalized;
    }
    let head = ((MESSAGE_MAX_CHARS - 3) as f64 * 0.6).floor() as usize;
    let tail = MESSAGE_MAX_CHARS - head - 3;
    format!(
        "{}...{}",
        utf16::slice(&units, 0, head),
        utf16::slice(&units, units.len() - tail, units.len())
    )
}

fn member<'a>(v: &'a Value, k: &str) -> Result<&'a Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{k}')")),
        _ if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        )),
        Value::Object(o) => Ok(o.get(k).unwrap_or(&Value::Null)),
        _ => Ok(&Value::Null),
    }
}

fn member_opt<'a>(v: &'a Value, k: &str) -> Result<Option<&'a Value>, String> {
    match member(v, k)? {
        Value::Null => Ok(None),
        v if js::is_undefined(v) => Ok(None),
        v => Ok(Some(v)),
    }
}

fn format_conversation_attachment(attachment: &Value) -> Result<String, String> {
    let ty = member(attachment, "type")?;
    if ty.as_str() == Some("assistant-selection") {
        let text = member(attachment, "text")?;
        let text = match text {
            Value::String(s) => s.clone(),
            Value::Null => {
                return Err("Cannot read properties of null".into());
            }
            _ => {
                return Err("value.replace is not a function".into());
            }
        };
        let sel = normalize_title_whitespace(&text);
        return Ok(if sel.is_empty() {
            "[assistant selection]".into()
        } else {
            format!("[assistant selection: {sel}]")
        });
    }
    let ty_name = ty.as_str().unwrap_or_default();
    let name = match member(attachment, "name")? {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => return Err("value.replace is not a function".into()),
    };
    let name = normalize_title_whitespace(&name);
    Ok(if name.is_empty() {
        format!("[{ty_name}]")
    } else {
        format!("[{ty_name}: {name}]")
    })
}

fn thread_title_message_text(message: &Value) -> Result<String, String> {
    let text = member(message, "text")?;
    let text = match text {
        Value::String(s) => s.clone(),
        _ => return Err("value.replace is not a function".into()),
    };
    let mut parts = vec![normalize_title_whitespace(&text)];
    if let Some(atts) = member_opt(message, "attachments")? {
        let Value::Array(list) = atts else {
            return Err("(message.attachments ?? []).map is not a function".into());
        };
        for a in list {
            parts.push(format_conversation_attachment(a)?);
        }
    }
    Ok(parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" "))
}

fn to_num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::String(s) => s.trim_matches(is_js_space).parse().unwrap_or(f64::NAN),
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => to_num(&a[0]),
            _ => f64::NAN,
        },
        _ => f64::NAN,
    }
}

fn utf16_len(s: &str) -> usize {
    utf16::units(s).len()
}

fn utf16_take(s: &str, n: usize) -> String {
    let u = utf16::units(s);
    utf16::slice(&u, 0, n.min(u.len()))
}

/// Port of `buildThreadTitleConversationContext` (`packages/shared/src/chatThreads.ts:98`).
pub fn build_thread_title_conversation_context(
    messages: serde_json::Value,
) -> Result<Option<String>, String> {
    if messages.is_null() {
        return Err("Cannot read properties of null (reading 'filter')".into());
    }
    let Value::Array(messages) = messages else {
        return Err("messages.filter is not a function".into());
    };
    let mut eligible: Vec<(bool, String)> = Vec::new(); // (is_user, text)
    for m in &messages {
        let is_user = member(m, "role")?.as_str() == Some("user")
            || member(m, "role")?.as_str() == Some("assistant");
        let streaming_true = member(m, "streaming")? == &Value::Bool(true);
        if !is_user || streaming_true {
            continue;
        }
        let text = thread_title_message_text(m)?;
        if !text.is_empty() {
            eligible.push((member(m, "role")?.as_str() == Some("user"), text));
        }
    }
    let Some(latest_user) = eligible.iter().rposition(|(u, _)| *u) else {
        return Ok(None);
    };
    let n = eligible.len();
    let tail_start = n.saturating_sub(CONTEXT_MAX_MESSAGES - 1);
    let tail_has_user = eligible[tail_start..].iter().any(|(u, _)| *u);
    let candidates: Vec<&(bool, String)> = if tail_has_user {
        eligible[n.saturating_sub(CONTEXT_MAX_MESSAGES)..]
            .iter()
            .collect()
    } else {
        let mut c = vec![&eligible[latest_user]];
        c.extend(eligible[tail_start..].iter());
        c
    };
    let lines: Vec<String> = candidates
        .iter()
        .map(|(u, t)| {
            format!(
                "{}: {}",
                if *u { "User" } else { "Assistant" },
                truncate_conversation_message(t)
            )
        })
        .collect();
    let required = lines
        .iter()
        .enumerate()
        .rev()
        .find(|(i, _)| candidates[*i].0)
        .map(|(i, _)| i);
    let Some(required) = required else {
        return Ok(None);
    };
    let mut selected = vec![required];
    let mut context_len = utf16_len(&lines[required]) as f64;
    for index in (0..lines.len()).rev() {
        if index == required {
            continue;
        }
        let next_len = context_len + utf16_len(&lines[index]) as f64 + 1.0;
        if next_len > CONTEXT_MAX_CHARS {
            continue;
        }
        selected.push(index);
        context_len = next_len;
    }
    selected.sort_unstable();
    Ok(Some(
        selected
            .into_iter()
            .map(|i| lines[i].clone())
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

/// Port of `truncateChatThreadTitle` (`packages/shared/src/chatThreads.ts:141`).
pub fn truncate_chat_thread_title(
    text: String,
    max_length: serde_json::Value,
) -> Result<String, String> {
    let max = if js::is_undefined(&max_length) {
        MAX_TITLE_LEN
    } else {
        to_num(&max_length)
    };
    let trimmed = normalize_title_whitespace(&text);
    if (utf16_len(&trimmed) as f64) <= max {
        return Ok(trimmed);
    }
    let n = if max.is_nan() || max < 0.0 {
        0
    } else {
        max.trunc() as usize
    };
    Ok(format!("{}...", utf16_take(&trimmed, n)))
}

/// Port of `buildPromptThreadTitleFallback` (`packages/shared/src/chatThreads.ts:153`).
pub fn build_prompt_thread_title_fallback(message: String) -> Result<String, String> {
    let words = title_words(&message);
    let words: Vec<String> = words.into_iter().take(MAX_TITLE_WORDS).collect();
    if words.is_empty() {
        return Ok(GENERIC_TITLE.into());
    }
    truncate_chat_thread_title(words.join(" "), js::undefined())
}

/// Port of `sanitizeGeneratedThreadTitle` (`packages/shared/src/chatThreads.ts:162`).
pub fn sanitize_generated_thread_title(raw: String) -> Result<String, String> {
    let unquoted = first_generated_title_line(&raw)
        .trim_start_matches(['\'', '"', '`'])
        .trim_end_matches(['\'', '"', '`'])
        .to_string();
    let words: Vec<String> = title_words(&unquoted)
        .into_iter()
        .take(MAX_TITLE_WORDS)
        .collect();
    if words.is_empty() {
        return Ok(GENERIC_TITLE.into());
    }
    truncate_chat_thread_title(words.join(" "), js::undefined())
}

/// Port of `isUsableGeneratedThreadTitle` (`packages/shared/src/chatThreads.ts:171`).
pub fn is_usable_generated_thread_title(title: Option<String>) -> Result<bool, String> {
    let normalized = normalize_title_whitespace(title.as_deref().unwrap_or("")).to_lowercase();
    Ok(!normalized.is_empty()
        && ![
            "chat",
            "conversation",
            "new chat",
            "new conversation",
            "new session",
            "new thread",
            "session",
            "thread",
            "untitled",
        ]
        .contains(&normalized.as_str()))
}

/// Port of `isGenericChatThreadTitle` (`packages/shared/src/chatThreads.ts:176`).
pub fn is_generic_chat_thread_title(title: Option<String>) -> Result<bool, String> {
    Ok(normalize_title_whitespace(title.as_deref().unwrap_or("")) == GENERIC_TITLE)
}
