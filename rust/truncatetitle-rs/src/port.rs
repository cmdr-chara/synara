//! The Rust port of `apps/web/src/truncateTitle.ts`.

#![forbid(unsafe_code)]

use crate::utf16;

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

/// Port of `truncateTitle` (`apps/web/src/truncateTitle.ts:1`).
pub fn truncate_title(text: String, max_length: Option<f64>) -> Result<String, String> {
    let max_length = max_length.unwrap_or(50.0);
    let trimmed = text.trim_matches(is_js_space);
    let units = utf16::units(trimmed);
    let len = units.len();
    if (len as f64) <= max_length {
        return Ok(trimmed.to_string());
    }
    // JS slice(0, maxLength): end is ToIntegerOrInfinity(maxLength) clamped to [0, len],
    // negatives count back from the end. NaN and -0 become 0.
    let end = if max_length.is_nan() || max_length == f64::NEG_INFINITY {
        0
    } else if max_length == f64::INFINITY {
        len
    } else {
        let e = max_length.trunc() as i64;
        if e < 0 {
            (len as i64 + e).max(0) as usize
        } else {
            (e as usize).min(len)
        }
    };
    Ok(format!("{}...", utf16::slice(&units, 0, end)))
}
