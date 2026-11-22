//! The Rust port of `apps/api/src/routes/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `isLoopbackRedirectUri` (`apps/api/src/routes/v1.ts:358`).
pub fn is_loopback_redirect_uri(redirect_uri: String) -> Result<bool, String> {
    let Some(rest) = redirect_uri
        .get(..7)
        .filter(|s| s.eq_ignore_ascii_case("http://"))
        .map(|_| &redirect_uri[7..])
    else {
        return Ok(false);
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority.rsplit('@').next().unwrap_or("");
    let hostname = if let Some(br) = hostport.strip_prefix('[') {
        br.split(']').next().unwrap_or("").to_string()
    } else {
        hostport
            .split(':')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    Ok(matches!(
        hostname.as_str(),
        "127.0.0.1" | "localhost" | "::1"
    ))
}
