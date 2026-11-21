//! The Rust port of `apps/web/src/pullRequestReference.ts`.

#![forbid(unsafe_code)]

use std::sync::OnceLock;

use regex::Regex;

fn url_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?i)^https://github\.com/[^/\s]+/[^/\s]+/pull/\d+(?:[/?#].*)?$").unwrap()
    })
}

fn number_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^#?(\d+)$").unwrap())
}

fn is_js_space(c: char) -> bool {
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

/// Port of `parsePullRequestReference` (`apps/web/src/pullRequestReference.ts:5`).
pub fn parse_pull_request_reference(input: String) -> Result<Option<String>, String> {
    let trimmed = input.trim_matches(is_js_space);
    if trimmed.is_empty() {
        return Ok(None);
    }
    if url_re().is_match(trimmed) {
        return Ok(Some(trimmed.to_string()));
    }
    if let Some(m) = number_re().captures(trimmed) {
        return Ok(Some(if trimmed.starts_with('#') {
            trimmed.to_string()
        } else {
            m[1].to_string()
        }));
    }
    Ok(None)
}
