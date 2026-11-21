//! The Rust port of `apps/web/src/composerTriggerInsertion.ts`.
//!
//! Character indexing is in UTF-16 code units, matching JS `text[i]`.

#![forbid(unsafe_code)]

/// The `String.prototype.trim` / `\s` whitespace set.
fn js_ws(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}'
}

/// `text[i]` in JS: the character at UTF-16 index `i`, or undefined
/// for non-integer, negative or out-of-range indices.
fn char_at(text: &str, i: f64) -> Option<char> {
    if !i.is_finite() || i < 0.0 || i.fract() != 0.0 {
        return None;
    }
    let i = i as usize;
    let mut unit = 0usize;
    for ch in text.chars() {
        let width = ch.len_utf16();
        if i == unit {
            return Some(ch);
        }
        if i < unit + width {
            // Index inside a surrogate pair: JS sees a lone surrogate
            // code unit — truthy and not whitespace.
            return Some('\u{0080}');
        }
        unit += width;
    }
    None
}

/// Port of `extendReplacementRangeForTrailingSpace`
/// (`apps/web/src/composerTriggerInsertion.ts:10`).
///
/// When the replacement ends in a space and the character at
/// `rangeEnd` is already a space, the range grows by one to swallow it.
pub fn extend_replacement_range_for_trailing_space(
    text: String,
    range_end: f64,
    replacement: String,
) -> Result<f64, String> {
    if !replacement.ends_with(' ') {
        return Ok(range_end);
    }
    Ok(if char_at(&text, range_end) == Some(' ') {
        range_end + 1.0
    } else {
        range_end
    })
}

/// Port of `ensureLeadingSpaceForReplacement`
/// (`apps/web/src/composerTriggerInsertion.ts:26`).
///
/// Prepends a space unless the replacement is empty, `rangeStart` is
/// 0, or the preceding character is whitespace/missing.
pub fn ensure_leading_space_for_replacement(
    text: String,
    range_start: f64,
    replacement: String,
) -> Result<String, String> {
    if replacement.is_empty() || range_start == 0.0 {
        return Ok(replacement);
    }
    let preceding = char_at(&text, range_start - 1.0);
    match preceding {
        None => Ok(replacement),
        Some(c) if js_ws(c) => Ok(replacement),
        Some(_) => Ok(format!(" {replacement}")),
    }
}
