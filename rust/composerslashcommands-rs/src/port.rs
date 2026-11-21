//! The Rust port of `packages/shared/src/composerSlashCommands.ts`.
//!
//! Behavior inferred clean-room from recorded cases and black-box probes:
//! normalize = trim, strip leading `/` run, JS-lowercase (final sigma rule).
//! Built-in check = normalize then membership in a fixed name set.

#![forbid(unsafe_code)]

use unicode_normalization::char::is_combining_mark;

const BUILT_INS: &[&str] = &[
    "automation",
    "clear",
    "compact",
    "debug",
    "export",
    "feedback",
    "fork",
    "model",
    "plan",
    "rename",
    "review",
    "status",
];

fn is_cased(c: char) -> bool {
    c.is_lowercase() || c.is_uppercase()
}

fn js_to_lowercase(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        if c == '\u{03A3}' {
            let before = chars[..i]
                .iter()
                .rev()
                .find(|&&p| !is_combining_mark(p))
                .is_some_and(|&p| is_cased(p));
            let after = chars[i + 1..]
                .iter()
                .find(|&&p| !is_combining_mark(p))
                .is_some_and(|&p| is_cased(p));
            if before && !after {
                out.push('\u{03C2}');
                continue;
            }
        }
        out.extend(c.to_lowercase());
    }
    out
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// Port of `normalizeComposerSlashCommandName` (`packages/shared/src/composerSlashCommands.ts:12`).
pub fn normalize_composer_slash_command_name(value: String) -> Result<String, String> {
    Ok(js_to_lowercase(js_trim(&value).trim_start_matches('/')))
}

/// Port of `isBuiltInComposerSlashCommandName` (`packages/shared/src/composerSlashCommands.ts:16`).
pub fn is_built_in_composer_slash_command_name(value: String) -> Result<bool, String> {
    Ok(BUILT_INS.contains(&normalize_composer_slash_command_name(value)?.as_str()))
}
