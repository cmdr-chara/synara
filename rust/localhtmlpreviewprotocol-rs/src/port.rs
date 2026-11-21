//! The Rust port of `apps/desktop/src/localHtmlPreviewProtocol.ts`.

#![forbid(unsafe_code)]

const PREVIEW_SCHEME: &str = "synara-local-preview:";

fn scheme_of(value: &str) -> Option<&str> {
    let mut chars = value.char_indices();
    match chars.next() {
        Some((_, c)) if c.is_ascii_alphabetic() => {}
        _ => return None,
    }
    for (i, c) in chars {
        if c == ':' {
            return Some(&value[..i]);
        }
        if !(c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return None;
        }
    }
    None
}

/// WHATWG-style authority split for `scheme://authority/path` inputs; `None`
/// when the URL has no `//` authority block.
fn authority_of(rest: &str) -> Option<&str> {
    let rest = rest.strip_prefix("//")?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    Some(&rest[..end])
}

fn hostname_of(value: &str, scheme_len: usize) -> String {
    let rest = &value[scheme_len + 1..];
    match authority_of(rest) {
        Some(authority) => {
            let hostport = authority.rsplit('@').next().unwrap_or(authority);
            let end = hostport.find('[').map(|i| {
                hostport[i..]
                    .find(']')
                    .map(|j| i + j + 1)
                    .unwrap_or(hostport.len())
            });
            match end {
                Some(e) => hostport[..e].to_string(),
                None => match hostport.find(':') {
                    Some(i) => hostport[..i].to_string(),
                    None => hostport.to_string(),
                },
            }
        }
        None => String::new(),
    }
}

/// Port of `isLocalFileUrl` (`apps/desktop/src/localHtmlPreviewProtocol.ts:79`).
pub fn is_local_file_url(value: String) -> Result<bool, String> {
    Ok(matches!(scheme_of(&value), Some("file")))
}

/// Port of `isLocalHtmlPreviewUrl` (`apps/desktop/src/localHtmlPreviewProtocol.ts:83`).
pub fn is_local_html_preview_url(value: String) -> Result<bool, String> {
    let Some(scheme) = scheme_of(&value) else {
        return Ok(false);
    };
    if !scheme.eq_ignore_ascii_case("synara-local-preview") {
        return Ok(false);
    }
    let rest = &value[scheme.len() + 1..];
    if let Some(authority) = authority_of(rest) {
        let hostport = authority.rsplit('@').next().unwrap_or("");
        if authority.len() != hostport.len() {
            // userinfo present: username or password is non-empty
            let userinfo = &authority[..authority.len() - hostport.len() - 1];
            if !userinfo.is_empty() {
                return Ok(false);
            }
        }
        if !hostport.starts_with('[')
            && let Some(i) = hostport.rfind(':')
            && !hostport[i + 1..].is_empty()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Port of `isSameLocalHtmlPreviewGrant` (`apps/desktop/src/localHtmlPreviewProtocol.ts:88`).
pub fn is_same_local_html_preview_grant(
    source_url: String,
    target_url: String,
) -> Result<bool, String> {
    if !is_local_html_preview_url(source_url.clone())?
        || !is_local_html_preview_url(target_url.clone())?
    {
        return Ok(false);
    }
    let source_scheme = scheme_of(&source_url)
        .map(|s| s.len())
        .unwrap_or(PREVIEW_SCHEME.len() - 1);
    let target_scheme = scheme_of(&target_url)
        .map(|s| s.len())
        .unwrap_or(PREVIEW_SCHEME.len() - 1);
    Ok(hostname_of(&source_url, source_scheme) == hostname_of(&target_url, target_scheme))
}
