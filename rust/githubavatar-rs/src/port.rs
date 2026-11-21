//! The Rust port of `packages/shared/src/githubAvatar.ts`.
//!
//! Behavior inferred clean-room from recorded cases: trim the login, return
//! `https://avatars.githubusercontent.com/<encodeURIComponent(login)>?size=64`,
//! `None` for missing/blank input.

#![forbid(unsafe_code)]

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
        )
}

fn encode_uri_component(s: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(s.len() * 3);
    for &b in s.as_bytes() {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0xF) as usize] as char);
        }
    }
    out
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}' || c == '\u{00A0}')
}

/// Port of `githubAvatarUrlForLogin` (`packages/shared/src/githubAvatar.ts:9`).
pub fn github_avatar_url_for_login(login: Option<String>) -> Result<Option<String>, String> {
    let Some(login) = login else { return Ok(None) };
    let trimmed = js_trim(&login);
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!(
        "https://avatars.githubusercontent.com/{}?size=64",
        encode_uri_component(trimmed)
    )))
}
