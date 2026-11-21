//! The Rust port of `packages/shared/src/searchQuery.ts`.
//!
//! Behavior inferred clean-room from recorded cases: JS-lowercase, trim,
//! strip a leading run of `.`/`/` characters.

#![forbid(unsafe_code)]

use unicode_normalization::char::is_combining_mark;

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// Port of `normalizeWorkspaceEntrySearchQuery` (`packages/shared/src/searchQuery.ts:18`).
fn js_to_lowercase(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        if c == '\u{03A3}' {
            // Greek final-sigma rule: cased letter before, no cased letter after.
            let before = chars[..i]
                .iter()
                .rev()
                .find(|&&p| !is_combining_mark(p))
                .is_some_and(|&p| p.is_lowercase() || p.is_uppercase());
            let after = chars[i + 1..]
                .iter()
                .find(|&&p| !is_combining_mark(p))
                .is_some_and(|&p| p.is_lowercase() || p.is_uppercase());
            if before && !after {
                out.push('\u{03C2}');
                continue;
            }
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// Port of `normalizeWorkspaceEntrySearchQuery` (`packages/shared/src/searchQuery.ts:18`).
pub fn normalize_workspace_entry_search_query(input: String) -> Result<String, String> {
    let lowered = js_to_lowercase(&input);
    let trimmed = js_trim(&lowered);
    Ok(trimmed.trim_start_matches(['.', '/']).to_string())
}
