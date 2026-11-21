//! The Rust port of `apps/server/src/threadSearch.ts`.

#![forbid(unsafe_code)]

const MAX_EXCERPT_LEN: usize = 320;
const EXCERPT_CONTEXT_BEFORE: isize = 80;
const ELLIPSIS: &str = "...";
const MAX_BODY: usize = MAX_EXCERPT_LEN - ELLIPSIS.len() * 2; // 314

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn to_string(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

/// The exact ECMAScript `\s` / trim set (excludes U+0085, which Rust's
/// `char::is_whitespace` wrongly counts as a space).
fn is_js_ws(u: u16) -> bool {
    matches!(
        u,
        0x0009..=0x000D | 0x0020 | 0x00A0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F
            | 0x205F | 0x3000 | 0xFEFF
    )
}

/// `value.trim().replaceAll(/\s+/g, " ")` on UTF-16 code units.
fn normalize_ws(u: &[u16]) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::with_capacity(u.len());
    let mut pending_space = false;
    let mut started = false;
    for &c in u {
        if is_js_ws(c) {
            if started {
                pending_space = true;
            }
            continue;
        }
        started = true;
        if pending_space {
            out.push(0x20);
            pending_space = false;
        }
        out.push(c);
    }
    out
}

fn lower(u: &[u16]) -> Vec<u16> {
    to_string(u).to_lowercase().encode_utf16().collect()
}

/// `String.prototype.indexOf` on code units (empty needle → 0).
fn index_of(hay: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > hay.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Port of `tokenizeThreadSearchQuery` (`apps/server/src/threadSearch.ts:35`).
pub fn tokenize_thread_search_query(query: String) -> Result<Vec<String>, String> {
    let normalized = lower(&normalize_ws(&units(&query)));
    let mut seen: Vec<Vec<u16>> = Vec::new();
    for token in normalized.split(|&u| u == 0x20) {
        if token.is_empty() {
            continue;
        }
        let token = token.to_vec();
        if !seen.iter().any(|s| s == &token) {
            seen.push(token);
        }
    }
    Ok(seen.iter().map(|t| to_string(t)).collect())
}

/// `str.trim()` on UTF-16 code units.
fn trim_u16(u: &[u16]) -> &[u16] {
    let start = u.iter().position(|&c| !is_js_ws(c)).unwrap_or(u.len());
    let end = u
        .iter()
        .rposition(|&c| !is_js_ws(c))
        .map(|i| i + 1)
        .unwrap_or(start);
    &u[start..end.max(start)]
}

/// Port of `buildThreadSearchExcerpt` (`apps/server/src/threadSearch.ts:44`).
pub fn build_thread_search_excerpt(
    text: String,
    query: String,
    tokens: Vec<String>,
) -> Result<String, String> {
    let display = normalize_ws(&units(&text));
    if display.len() <= MAX_EXCERPT_LEN {
        return Ok(to_string(&display));
    }
    let lower_display = lower(&display);
    let query_units = lower(&normalize_ws(&units(&query)));
    let mut hit_index = index_of(&lower_display, &query_units)
        .map(|i| i as isize)
        .unwrap_or(-1);
    if hit_index < 0 {
        let token_indexes: Vec<isize> = tokens
            .iter()
            .filter_map(|t| index_of(&lower_display, &units(t)).map(|i| i as isize))
            .collect();
        hit_index = token_indexes.iter().min().copied().unwrap_or(0);
    }
    let len = display.len() as isize;
    let start = (hit_index - EXCERPT_CONTEXT_BEFORE)
        .max(0)
        .min((len - MAX_BODY as isize).max(0)) as usize;
    let end = (display.len()).min(start + MAX_BODY);
    let mut out = String::new();
    if start > 0 {
        out.push_str(ELLIPSIS);
    }
    out.push_str(&to_string(trim_u16(&display[start..end])));
    if end < display.len() {
        out.push_str(ELLIPSIS);
    }
    Ok(out)
}
