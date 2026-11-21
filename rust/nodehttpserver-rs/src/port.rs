//! Port of `apps/server/src/nodeHttpServer.ts` — port units only.

use serde_json::{Value, json};
use std::sync::LazyLock;

/// `WS_FEATURE_PATH` — the one upgrade route allowed to negotiate compression.
const COMPRESSED_UPGRADE_PATH: &str = "/ws";

/// `WS_FEATURE_UNCOMPRESSED_QUERY` — client-declared compression opt-out.
const UNCOMPRESSED_QUERY_NAME: &str = "x-synara-uncompressed";
const UNCOMPRESSED_QUERY_VALUE: &str = "1";

static UPGRADE_BASE: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://upgrade.invalid").expect("valid base URL"));

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// JS `decodeURIComponent`: `%XX` decodes to UTF-8 bytes; malformed escapes or
/// invalid UTF-8 throw — `None` here.
fn decode_uri_component(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if i + 2 >= b.len() {
                return None;
            }
            let hi = hex_val(b[i + 1])?;
            let lo = hex_val(b[i + 2])?;
            out.push(hi << 4 | lo);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Byte offset just past `scheme://` when `target` is absolute-form.
fn absolute_form_end(target: &str) -> Option<usize> {
    let b = target.as_bytes();
    if b.is_empty() || !b[0].is_ascii_alphabetic() {
        return None;
    }
    let mut i = 1;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'+' | b'-' | b'.')) {
        i += 1;
    }
    if b[i..].starts_with(b"://") {
        Some(i + 3)
    } else {
        None
    }
}

/// `normalizeUpgradePath`: strips authority, query/fragment, `;params`,
/// percent-decoding, duplicate slashes, case and trailing slashes.
fn normalize_upgrade_path(request_url: Option<&str>) -> String {
    let target = request_url.unwrap_or("");
    let after_authority = match absolute_form_end(target) {
        Some(end) => match target[end..].find('/') {
            Some(i) => &target[end + i..],
            None => "/",
        },
        None => target,
    };
    let path_only = after_authority
        .split('#')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");
    let decoded = decode_uri_component(path_only).unwrap_or_else(|| path_only.to_string());
    let without_params = decoded
        .split('/')
        .map(|segment| segment.split(';').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("/");
    let mut merged = String::with_capacity(without_params.len());
    let mut in_slash_run = false;
    for c in without_params.chars() {
        if c == '/' {
            if in_slash_run {
                continue;
            }
            in_slash_run = true;
        } else {
            in_slash_run = false;
        }
        merged.push(c);
    }
    let collapsed = merged.to_lowercase();
    // `.length > 1` counts UTF-16 units.
    if collapsed.encode_utf16().count() > 1 {
        collapsed.trim_end_matches('/').to_string()
    } else {
        collapsed
    }
}

/// Port of `upgradePathAllowsCompression` — fail closed: only the exact
/// feature route negotiates compression.
pub fn upgrade_path_allows_compression(request_url: Option<String>) -> Result<bool, String> {
    Ok(normalize_upgrade_path(request_url.as_deref()) == COMPRESSED_UPGRADE_PATH)
}

/// Port of `upgradeOptsOutOfCompression`: the `x-synara-uncompressed=1` query
/// flag on a WHATWG-parsed upgrade target. Malformed targets keep compression.
pub fn upgrade_opts_out_of_compression(request_url: Option<String>) -> Result<bool, String> {
    let Some(u) = request_url.filter(|u| !u.is_empty()) else {
        return Ok(false);
    };
    let Ok(parsed) = UPGRADE_BASE.join(&u) else {
        return Ok(false);
    };
    Ok(parsed
        .query_pairs()
        .find(|(k, _)| k == UNCOMPRESSED_QUERY_NAME)
        .is_some_and(|(_, v)| v == UNCOMPRESSED_QUERY_VALUE))
}

/// Port of `upgradeNativeWebSocket` — returns an unexecuted `Effect`
/// (acquireRelease marker); the socket work only runs when the Effect does.
pub fn upgrade_native_web_socket(_source: Value) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
