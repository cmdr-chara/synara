//! The Rust port of `apps/server/src/codexErrorClassification.ts`.

#![forbid(unsafe_code)]

const NON_FATAL_CODEX_ERROR_SNIPPETS: &[&str] =
    &["write_stdin failed: stdin is closed for this session"];

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

/// Port of `isNonFatalCodexErrorMessage` (`apps/server/src/codexErrorClassification.ts:14`).
pub fn is_non_fatal_codex_error_message(message: String) -> Result<bool, String> {
    let normalized = message.trim_matches(is_js_space).to_lowercase();
    Ok(NON_FATAL_CODEX_ERROR_SNIPPETS
        .iter()
        .any(|snippet| normalized.contains(snippet)))
}
