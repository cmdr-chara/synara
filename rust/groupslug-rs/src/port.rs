//! The Rust port of `packages/shared/src/groupSlug.ts`.
//!
//! Behavior inferred clean-room from recorded cases: lowercase, NFKD-normalize,
//! drop combining marks, collapse runs of non `[a-z0-9]` into `-`, trim `-`,
//! fall back to `"group"` when nothing is left.

#![forbid(unsafe_code)]

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Port of `slugifyGroupTitle` (`packages/shared/src/groupSlug.ts:4`).
pub fn slugify_group_title(title: String) -> Result<String, String> {
    let mut out = String::with_capacity(title.len());
    let mut pending_dash = false;
    for c in title.nfkd().flat_map(char::to_lowercase) {
        if is_combining_mark(c) {
            continue;
        }
        if c.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            out.push(c);
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() {
        Ok("group".to_string())
    } else {
        Ok(out)
    }
}
