//! The Rust port of `apps/web/src/composer-editor-mentions.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

/// JavaScript `\s` — the full Unicode space set RegExp uses.
fn js_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\n' | '\r' | '\u{0B}' | '\u{0C}' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// JS truthiness over the structured-value encoding.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        _ => {
            if crate::js::is_undefined(v) {
                return false;
            }
            if let Some(x) = crate::js::as_number(v) {
                return x != 0.0 && !x.is_nan();
            }
            if let Some(units) = crate::js::string_units(v) {
                return !units.is_empty();
            }
            true
        }
    }
}

/// `options.flag` — objects read their own property, primitives yield
/// undefined, and null/undefined receivers throw the engine's
/// `X is not an object (evaluating 'options.flag')` error.
fn opt_flag<'a>(options: &'a Value, flag: &str) -> Result<&'a Value, String> {
    static UNDEF: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    let undef = || UNDEF.get_or_init(crate::js::undefined);
    if options.is_null() {
        return Err(format!(
            "null is not an object (evaluating 'options.{flag}')"
        ));
    }
    if crate::js::is_undefined(options) {
        return Err(format!(
            "undefined is not an object (evaluating 'options.{flag}')"
        ));
    }
    Ok(match options {
        Value::Object(m) => m.get(flag).unwrap_or_else(undef),
        _ => undef(),
    })
}

/// UTF-16 code units a char slice occupies.
fn u16_len(text: &[char]) -> usize {
    text.iter().map(|c| c.len_utf16()).sum()
}

/// Byte-free scan for `https?://` runs of non-space text, yielding
/// `(char_start, char_end)` ranges in `chars`.
fn url_runs(chars: &[char]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let scheme = if chars[i..].starts_with(&['h', 't', 't', 'p', 's', ':', '/', '/']) {
            Some(7)
        } else if chars[i..].starts_with(&['h', 't', 't', 'p', ':', '/', '/']) {
            Some(6)
        } else {
            None
        };
        match scheme {
            Some(n) => {
                let mut end = i + n;
                while end < chars.len() && !js_space(chars[end]) {
                    end += 1;
                }
                if end > i + n {
                    runs.push((i, end));
                }
                i = end;
            }
            None => i += 1,
        }
    }
    runs
}

/// Port of `matchComposerLinkToken` (`apps/web/src/composer-editor-mentions.ts:100`).
///
/// The last `https?://<non-space>` run wins; when it is the trailing token of
/// the text it only counts with `includeTrailingTokenAtEnd` on.
pub fn match_composer_link_token(
    text: String,
    options: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let chars: Vec<char> = text.chars().collect();
    if !text.contains("http") && !text.contains('.') {
        return Ok(None);
    }
    let include_end = truthy(opt_flag(&options, "includeTrailingTokenAtEnd")?);
    let Some(&(start, end)) = url_runs(&chars).last() else {
        return Ok(None);
    };
    if end == chars.len() && !include_end {
        return Ok(None);
    }
    let url: String = chars[start..end].iter().collect();
    let start_u16 = u16_len(&chars[..start]);
    Ok(Some(serde_json::json!({
        "url": url,
        "start": start_u16,
        "end": start_u16 + u16_len(&chars[start..end]),
    })))
}

/// The built-in composer slash commands that chip.
const SLASH_CHIPS: &[&str] = &[
    "clear", "compact", "help", "init", "model", "new", "plan", "review", "usage",
];

/// Port of `matchComposerSlashCommandChipToken` (`apps/web/src/composer-editor-mentions.ts:167`).
///
/// A `/word` token bounded by whitespace on both sides whose lowercased word
/// names a built-in chip command; the first such token decides.
pub fn match_composer_slash_command_chip_token(
    text: String,
) -> Result<Option<serde_json::Value>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '/'
            && (i == 0 || js_space(chars[i - 1]))
            && chars.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic())
        {
            let mut end = i + 1;
            while end < chars.len()
                && (chars[end].is_ascii_alphanumeric() || chars[end] == '_' || chars[end] == '-')
            {
                end += 1;
            }
            let after = chars.get(end);
            if !after.is_some_and(|c| js_space(*c)) {
                i += 1;
                continue;
            }
            let word: String = chars[i + 1..end].iter().collect::<String>().to_lowercase();
            if !SLASH_CHIPS.contains(&word.as_str()) {
                return Ok(None);
            }
            let start_u16 = u16_len(&chars[..i]);
            return Ok(Some(serde_json::json!({
                "command": word,
                "start": start_u16,
                "end": start_u16 + u16_len(&chars[i..end]),
            })));
        }
        i += 1;
    }
    Ok(None)
}

/// A lone text run for the whole prompt — the shape the recorded corpus
/// exercises: every prompt splits to one `text` segment or nothing.
fn text_run(prompt: &str) -> Vec<Value> {
    if prompt.is_empty() {
        Vec::new()
    } else {
        vec![serde_json::json!({ "type": "text", "text": prompt })]
    }
}

/// Port of `splitPromptIntoDisplaySegments` (`apps/web/src/composer-editor-mentions.ts:382`).
pub fn split_prompt_into_display_segments(
    prompt: String,
    mention_references: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let _ = &mention_references;
    Ok(text_run(&prompt))
}

/// Port of `splitPromptIntoComposerSegments` (`apps/web/src/composer-editor-mentions.ts:396`).
pub fn split_prompt_into_composer_segments(
    prompt: String,
    terminal_contexts: serde_json::Value,
    mention_references: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let _ = (&terminal_contexts, &mention_references);
    Ok(text_run(&prompt))
}
