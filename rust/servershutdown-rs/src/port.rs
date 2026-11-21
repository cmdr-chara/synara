//! The Rust port of `apps/server/src/serverShutdown.ts`.
//!
//! Only the authorization helpers port: the route path constants and the Effect-based
//! `makeServerShutdownController` are not function declarations the bridge captures.
//! `isLoopbackHost` (`startupAccess.ts`) is inlined: `authorizeDesktopShutdown` calls
//! it on a raw JSON value, where a truthy non-string host raises a TypeError the
//! `Option<String>` signature of `startupaccess-rs` cannot express.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// JS whitespace — `\s` and `String.prototype.trim`'s set: ASCII whitespace, the
/// Unicode space separators, and U+FEFF. Rust's `char::is_whitespace` misses FEFF
/// and wrongly includes U+0085.
fn is_js_ws(u: u16) -> bool {
    matches!(
        u,
        0x09..=0x0d | 0x20 | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff
    )
}

/// `String.prototype.trim` on UTF-16 code units.
fn js_trim(units: &[u16]) -> &[u16] {
    let (mut start, mut end) = (0, units.len());
    while start < end && is_js_ws(units[start]) {
        start += 1;
    }
    while end > start && is_js_ws(units[end - 1]) {
        end -= 1;
    }
    &units[start..end]
}

/// The UTF-16 units of a transported string (`{"$utf16"}` markers included).
fn str_units(v: Option<&Value>) -> Option<Vec<u16>> {
    v.and_then(js::string_units)
}

/// `v === undefined`: a missing member read or the `{"$undefined"}` marker.
fn is_undef(v: Option<&Value>) -> bool {
    v.is_none_or(js::is_undefined)
}

/// `v == null`: `undefined` or `null`, what `?.` and `??` treat as absent.
fn nullish(v: Option<&Value>) -> bool {
    v.is_none_or(|v| v.is_null() || js::is_undefined(v))
}

/// `obj.key` on any JS value: a TypeError on a null/undefined receiver,
/// `undefined` (`None`) on primitives, arrays and missing keys.
fn get<'a>(v: Option<&'a Value>, key: &str) -> Result<Option<&'a Value>, String> {
    match v {
        Some(Value::Null) => Err(format!("Cannot read properties of null (reading '{key}')")),
        Some(v) if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        None => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Some(Value::Object(o)) => Ok(o.get(key)),
        Some(_) => Ok(None),
    }
}

/// `obj.key` where `obj` is already known non-nullish.
fn prop<'a>(v: Option<&'a Value>, key: &str) -> Option<&'a Value> {
    match v {
        Some(Value::Object(o)) => o.get(key),
        _ => None,
    }
}

/// JS falsy for transported values: `undefined`, `null`, `false`, `0`, `-0`, `NaN`,
/// `""` and `0n`, marker objects included.
fn js_falsy(v: Option<&Value>) -> bool {
    if is_undef(v) {
        return true;
    }
    match v {
        Some(Value::Null) | Some(Value::Bool(false)) => true,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty(),
        Some(Value::Object(o)) if o.len() == 1 => match o.iter().next() {
            Some((k, x)) => match k.as_str() {
                "$num" => matches!(x.as_str(), Some("NaN") | Some("-0")),
                "$bigint" => x.as_str() == Some("0"),
                "$utf16" => x.as_array().is_some_and(Vec::is_empty),
                _ => false,
            },
            None => false,
        },
        _ => false,
    }
}

/// `crypto.timingSafeEqual` over the two sha256 digests: fixed 32-byte buffers, so
/// the length check never throws and the equality can stay constant-time.
fn digests_equal(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b.iter()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// `digestToken`: `createHash("sha256").update(token, "utf8")` — a lone surrogate
/// encodes as U+FFFD.
fn digest_token(units: &[u16]) -> [u8; 32] {
    Sha256::digest(String::from_utf16_lossy(units).as_bytes()).into()
}

/// `normalized === "127.0.0.1" || normalized === "::1" || normalized === "::ffff:127.0.0.1"`,
/// `normalized` being the trimmed, lowercased address.
fn loopback_peer(units: &[u16]) -> bool {
    let normalized = String::from_utf16_lossy(js_trim(units)).to_lowercase();
    matches!(
        normalized.as_str(),
        "127.0.0.1" | "::1" | "::ffff:127.0.0.1"
    )
}

/// `isDesktopShutdownLoopbackPeer` on a transported value: `?.` short-circuits on
/// null/undefined; a non-string peer raises the `trim` TypeError.
fn loopback_peer_value(remote: Option<&Value>) -> Result<bool, String> {
    match str_units(remote) {
        Some(units) => Ok(loopback_peer(&units)),
        None if nullish(remote) => Ok(false),
        None => Err("remoteAddress?.trim is not a function".to_string()),
    }
}

/// `isLoopbackHost` (`startupAccess.ts`): a falsy host means loopback; a truthy
/// non-string raises the `startsWith` TypeError.
fn loopback_host(host: Option<&Value>) -> Result<bool, String> {
    if js_falsy(host) {
        return Ok(true);
    }
    let Some(units) = str_units(host) else {
        return Err("host.startsWith is not a function".to_string());
    };
    // `host.startsWith("[") && host.endsWith("]") ? host.slice(1, -1) : host`
    let bracketed = matches!(
        (units.first(), units.last()),
        (Some(f), Some(l)) if *f == u16::from(b'[') && *l == u16::from(b']')
    );
    let sliced = if bracketed {
        &units[1..units.len() - 1]
    } else {
        &units
    };
    let normalized = String::from_utf16_lossy(sliced);
    Ok(matches!(
        normalized.as_str(),
        "localhost" | "127.0.0.1" | "::1"
    ))
}

/// `ToString(v)` for a transported value, as `RegExp.exec` coerces a non-string
/// argument. `null`/`undefined` array elements join as the empty string; marker
/// objects keep their meaning (`$utf16` strings, `$num`/`$bigint` numbers,
/// `{"$buffer"}` Buffers decode utf8, `{"$bytes"}` joins like arrays).
fn js_to_units(v: &Value, out: &mut Vec<u16>) {
    if let Some(units) = js::string_units(v) {
        out.extend(units);
        return;
    }
    if let Some(n) = js::as_number(v) {
        out.extend(js::number_to_string(n).encode_utf16());
        return;
    }
    match v {
        Value::Bool(b) => out.extend(if *b { "true" } else { "false" }.encode_utf16()),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(u16::from(b','));
                }
                if !nullish(Some(item)) {
                    js_to_units(item, out);
                }
            }
        }
        Value::Object(o) => {
            let marker = (o.len() == 1).then(|| o.iter().next()).flatten();
            match marker.map(|(k, x)| (k.as_str(), x)) {
                Some(("$bigint", x)) => out.extend(x.as_str().unwrap_or("0").encode_utf16()),
                Some(("$buffer", x)) => {
                    let bytes: Vec<u8> = x
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|u| u8::try_from(u.as_u64().unwrap_or(u64::MAX)).ok())
                        .collect();
                    out.extend(String::from_utf8_lossy(&bytes).encode_utf16());
                }
                Some(("$bytes", x)) => js_to_units(x, out),
                Some(("$map", _)) => out.extend("[object Map]".encode_utf16()),
                Some(("$set", _)) => out.extend("[object Set]".encode_utf16()),
                _ => out.extend("[object Object]".encode_utf16()),
            }
        }
        _ => {}
    }
}

/// `readBearerToken`: `/^Bearer ([^\s]+)$/iu.exec(authorization ?? "")`.
fn bearer_token(authorization: Option<&Value>) -> Option<Vec<u16>> {
    if nullish(authorization) {
        return None; // `?? ""` — the empty string never matches
    }
    let units = match str_units(authorization) {
        Some(u) => u,
        None => {
            let mut out = Vec::new();
            if let Some(v) = authorization {
                js_to_units(v, &mut out);
            }
            out
        }
    };
    // `^Bearer ` is one literal space, matched ASCII case-insensitively; the token
    // is one or more non-`\s` code units to the end of the string.
    let token = units.get(7..).filter(|t| !t.is_empty())?;
    if !units[..7]
        .iter()
        .zip(b"Bearer ".iter())
        .all(|(u, b)| u8::try_from(*u).is_ok_and(|c| c.eq_ignore_ascii_case(b)))
        || token.iter().any(|u| is_js_ws(*u))
    {
        return None;
    }
    Some(token.to_vec())
}

/// Port of `matchesDesktopShutdownToken` (`apps/server/src/serverShutdown.ts:50`).
pub fn matches_desktop_shutdown_token(expected: String, presented: String) -> Result<bool, String> {
    let expected: Vec<u16> = expected.encode_utf16().collect();
    let presented: Vec<u16> = presented.encode_utf16().collect();
    Ok(digests_equal(
        &digest_token(&expected),
        &digest_token(&presented),
    ))
}

/// Port of `isDesktopShutdownLoopbackPeer` (`apps/server/src/serverShutdown.ts:54`).
pub fn is_desktop_shutdown_loopback_peer(remote_address: Option<String>) -> Result<bool, String> {
    // `remoteAddress?.trim().toLowerCase()`: null/undefined never match.
    Ok(remote_address.is_some_and(|s| loopback_peer(&s.encode_utf16().collect::<Vec<_>>())))
}

/// Port of `authorizeDesktopShutdown` (`apps/server/src/serverShutdown.ts:70`).
pub fn authorize_desktop_shutdown(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let denied = |reason: &str, status: u16| json!({ "authorized": false, "reason": reason, "status": status });

    // `const expectedToken = input.config.desktopShutdownToken` — two member reads,
    // each a TypeError on a null/undefined receiver.
    let config = get(Some(&input), "config")?;
    let expected_token = get(config, "desktopShutdownToken")?;

    // `mode !== "desktop" || !isLoopbackHost(host) || publicUrl !== undefined ||
    //  !isDesktopShutdownLoopbackPeer(remoteAddress) || !expectedToken?.trim()` —
    // a `||` chain evaluated in that order.
    if str_units(prop(config, "mode"))
        .map(|u| String::from_utf16_lossy(&u))
        .as_deref()
        != Some("desktop")
        || !loopback_host(prop(config, "host"))?
        || !is_undef(prop(config, "publicUrl"))
        || !loopback_peer_value(prop(Some(&input), "remoteAddress"))?
    {
        return Ok(denied("unavailable", 404));
    }
    let expected = match str_units(expected_token) {
        Some(u) if js_trim(&u).is_empty() => return Ok(denied("unavailable", 404)),
        Some(u) => u,
        None if nullish(expected_token) => return Ok(denied("unavailable", 404)),
        None => return Err("expectedToken?.trim is not a function".to_string()),
    };

    // `const presentedToken = readBearerToken(input.authorization)`
    let Some(presented) = bearer_token(prop(Some(&input), "authorization")) else {
        return Ok(denied("unauthorized", 401));
    };
    // `matchesDesktopShutdownToken(expectedToken, presentedToken)`
    if !digests_equal(&digest_token(&expected), &digest_token(&presented)) {
        return Ok(denied("unauthorized", 401));
    }
    Ok(json!({ "authorized": true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desktop(remote: Value, authorization: Value) -> Value {
        json!({
            "config": {"mode": "desktop", "host": "127.0.0.1", "desktopShutdownToken": "tok"},
            "remoteAddress": remote,
            "authorization": authorization,
        })
    }

    #[test]
    fn peers_and_denials() {
        let p = |s: &str| is_desktop_shutdown_loopback_peer(Some(s.into()));
        assert_eq!(p("  ::FFFF:127.0.0.1\u{feff}"), Ok(true));
        assert_eq!(p("\u{85}127.0.0.1"), Ok(false)); // NEL is not JS whitespace
        assert_eq!(is_desktop_shutdown_loopback_peer(None), Ok(false));

        let unavail = json!({ "authorized": false, "reason": "unavailable", "status": 404 });
        let unauth = json!({ "authorized": false, "reason": "unauthorized", "status": 401 });
        let a = |r: Value, z: Value| authorize_desktop_shutdown(desktop(r, z));
        assert_eq!(
            a(json!("::1"), json!("Bearer tok")),
            Ok(json!({"authorized": true}))
        );
        assert_eq!(
            a(json!("::1"), json!("bearer tok")),
            Ok(json!({"authorized": true}))
        ); // i flag
        assert_eq!(
            a(json!("::1"), json!(["Bearer", "tok"])),
            Ok(unauth.clone())
        ); // "Bearer,tok"
        assert_eq!(a(json!("::1"), json!("Bearer tok ")), Ok(unauth));
        assert_eq!(
            a(json!("1.2.3.4"), json!("Bearer tok")),
            Ok(unavail.clone())
        );
        assert_eq!(a(Value::Null, json!("Bearer tok")), Ok(unavail.clone()));
        let mut c = desktop(json!("127.0.0.1"), json!("Bearer tok"));
        c["config"]["publicUrl"] = Value::Null;
        assert_eq!(authorize_desktop_shutdown(c), Ok(unavail.clone()));
        let mut c = desktop(json!("127.0.0.1"), json!("Bearer tok"));
        c["config"]["desktopShutdownToken"] = json!(5);
        assert_eq!(
            authorize_desktop_shutdown(c),
            Err("expectedToken?.trim is not a function".into())
        );
        assert_eq!(
            authorize_desktop_shutdown(Value::Null),
            Err("Cannot read properties of null (reading 'config')".into())
        );
    }
}
