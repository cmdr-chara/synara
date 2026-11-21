//! The Rust port of `apps/server/src/jwtClaims.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

/// base64url decode with atob semantics: whitespace stripped, `-`/`_` mapped to
/// `+`/`/`, any other invalid character or impossible length fails.
fn b64url_decode(input: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut nbits = 0u32;
    let mut out = Vec::with_capacity(input.len() * 3 / 4 + 3);
    let mut chars = input
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .peekable();
    while let Some(c) = chars.next() {
        let v = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '-' | '+' => 62,
            '_' | '/' => 63,
            '=' => {
                // Padding is only legal as the final one or two chars.
                if chars.clone().all(|r| r == '=') {
                    break;
                }
                return None;
            }
            _ => return None,
        };
        bits = (bits << 6) | v;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
        }
    }
    // A 1-symbol remainder (6 leftover bits) is not a valid base64 stream.
    if nbits == 6 {
        return None;
    }
    Some(out)
}

/// Port of `decodeJwtClaims` (`apps/server/src/jwtClaims.ts:9`).
pub fn decode_jwt_claims(jwt: serde_json::Value) -> Result<serde_json::Value, String> {
    let Value::String(token) = &jwt else {
        return Ok(js::undefined());
    };
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Ok(js::undefined());
    }
    let Some(payload) = b64url_decode(parts[1]) else {
        return Ok(js::undefined());
    };
    match serde_json::from_slice::<Value>(&payload) {
        Ok(v) if v.is_object() => Ok(v),
        _ => Ok(js::undefined()),
    }
}
