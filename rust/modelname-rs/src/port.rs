//! The Rust port of `packages/shared/src/modelName.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const DISPLAY_NAMES: &[(&str, &str)] = &[
    ("deepseek", "DeepSeek"),
    ("glm", "GLM"),
    ("gpt", "GPT"),
    ("minimax", "MiniMax"),
    ("openai", "OpenAI"),
    ("opencode", "OpenCode"),
    ("swe", "SWE"),
    ("xai", "xAI"),
    ("xhigh", "XHigh"),
];

const FAMILY_TOKENS: &[&str] = &[
    "deepseek", "glm", "gpt", "minimax", "openai", "opencode", "swe", "xai", "xhigh", "adaptive",
    "auto", "claude", "codex", "composer", "cursor", "devin", "gemini", "grok", "inkling", "kimi",
    "nemotron",
];

fn is_js_space(c: char) -> bool {
    matches!(c, '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
        | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn utf16_decode(v: &Value) -> String {
    let units: Vec<u16> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_u64().map(|u| u as u16))
                .collect()
        })
        .unwrap_or_default();
    String::from_utf16_lossy(&units)
}

fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(is_js_space)
}

/// `s.split(/run of sep chars/g)` keeping the empty pieces JS keeps.
fn split_runs(s: &str, sep: impl Fn(char) -> bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_sep = false;
    for c in s.chars() {
        if sep(c) {
            if !in_sep {
                out.push(std::mem::take(&mut cur));
                in_sep = true;
            }
        } else {
            cur.push(c);
            in_sep = false;
        }
    }
    out.push(cur);
    out
}

/// `Object.hasOwn(MODEL_TOKEN_DISPLAY_NAMES, key)` — plain-object keys only.
fn display_name(token: &str) -> Option<&'static str> {
    let key = token.to_lowercase();
    DISPLAY_NAMES
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
}

/// Rejoin pure-digit fragments onto a token already ending in a digit;
/// `0`-prefixed tokens and 8-digit date/build stamps stay separate.
fn join_version_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut merged: Vec<String> = Vec::with_capacity(tokens.len());
    for token in tokens {
        let prev = merged.last();
        let merge = token.chars().all(|c| c.is_ascii_digit())
            && !token.is_empty()
            && (token == "0" || !token.starts_with('0'))
            && token.len() != 8
            && prev.is_some_and(|p| p.ends_with(|c: char| c.is_ascii_digit()));
        if merge {
            if let Some(p) = merged.last_mut() {
                p.push('.');
                p.push_str(&token);
            }
        } else {
            merged.push(token);
        }
    }
    merged
}

/// `/\bGPT (\d)/gu` → `GPT-$1`.
fn restore_separators(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut i = 0;
    while i < name.len() {
        if name[i..].starts_with("GPT ")
            && name
                .as_bytes()
                .get(i + 4)
                .is_some_and(|b| b.is_ascii_digit())
            && (i == 0
                || !name.as_bytes()[i - 1].is_ascii_alphanumeric()
                    && name.as_bytes()[i - 1] != b'_')
        {
            out.push_str("GPT-");
            i += 4;
        } else if let Some(c) = name[i..].chars().next() {
            out.push(c);
            i += c.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// `token.charAt(0).toUpperCase() + token.slice(1)` for non-brand tokens.
fn humanize_token(token: &str) -> String {
    if let Some(name) = display_name(token) {
        return name.to_string();
    }
    let mut chars = token.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Port of `humanizeModelSlug` (`packages/shared/src/modelName.ts:82`).
pub fn humanize_model_slug(slug: serde_json::Value) -> Result<String, String> {
    let slug: String = match &slug {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'includes')".to_string());
        }
        Value::String(s) => s.clone(),
        Value::Object(o) if o.get("$undefined").is_some() => {
            return Err("Cannot read properties of undefined (reading 'includes')".to_string());
        }
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => utf16_decode(&o["$utf16"]),
        _ => return Err("slug.includes is not a function".to_string()),
    };
    if slug.contains('/') {
        return Ok(slug);
    }
    let tokens = join_version_tokens(split_runs(&slug, |c| c == '-' || c == '_'));
    let mapped: Vec<String> = tokens.iter().map(|t| humanize_token(t)).collect();
    Ok(restore_separators(&mapped.join(" ")))
}

/// Port of `normalizeModelDisplayName` (`packages/shared/src/modelName.ts:98`).
pub fn normalize_model_display_name(name: serde_json::Value) -> Result<String, String> {
    let name: String = match &name {
        Value::Null => return Err("Cannot read properties of null (reading 'trim')".to_string()),
        Value::String(s) => s.clone(),
        Value::Object(o) if o.get("$undefined").is_some() => {
            return Err("Cannot read properties of undefined (reading 'trim')".to_string());
        }
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => utf16_decode(&o["$utf16"]),
        _ => return Err("name.trim is not a function".to_string()),
    };
    let trimmed = js_trim(&name);
    let (head, tail) = match trimmed.find('(') {
        Some(i) => (js_trim_end(&trimmed[..i]), &trimmed[i..]),
        None => (trimmed, ""),
    };
    let tokens: Vec<String> = split_runs(head, |c| c == '-' || c == '_' || is_js_space(c))
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();
    match tokens.first() {
        Some(first) if FAMILY_TOKENS.contains(&first.to_lowercase().as_str()) => {}
        _ => return Ok(trimmed.to_string()),
    }
    let normalized = join_version_tokens(tokens)
        .iter()
        .map(|t| {
            display_name(t)
                .map(str::to_string)
                .unwrap_or_else(|| t.clone())
        })
        .collect::<Vec<_>>()
        .join(" ");
    Ok(format!(
        "{}{}",
        restore_separators(&normalized),
        if tail.is_empty() {
            String::new()
        } else {
            format!(" {tail}")
        }
    ))
}
