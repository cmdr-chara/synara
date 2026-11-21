//! The Rust port of `apps/server/src/staticAssets.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const STATIC_IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
const STATIC_REVALIDATE_CACHE_CONTROL: &str = "no-cache";
const STATIC_ICON_CACHE_CONTROL: &str = "public, max-age=86400";

const ICON_DIRECTORY_PREFIXES: [&str; 2] = ["central-icons-reversed/", "central-icons-fill/"];

/// `(encoding, sidecarExtension)` in server preference order: brotli beats gzip.
const STATIC_ENCODING_CANDIDATES: [(&str, &str); 2] = [("br", ".br"), ("gzip", ".gz")];

const HEX_DIGITS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

/// JS `trim`/`trimEnd`/`trimStart` whitespace: Unicode `White_Space` minus NEL (U+0085 is
/// `White_Space` for Rust but not JS whitespace), plus the BOM U+FEFF JS strips.
fn is_js_space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{0085}') || c == '\u{FEFF}'
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(is_js_space)
}

/// Port of `staticCacheControl` (`apps/server/src/staticAssets.ts:19`).
pub fn static_cache_control(relative_path: String) -> Result<String, String> {
    let normalized = relative_path.replace('\\', "/");
    if normalized.starts_with("assets/") {
        return Ok(STATIC_IMMUTABLE_CACHE_CONTROL.into());
    }
    if ICON_DIRECTORY_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return Ok(STATIC_ICON_CACHE_CONTROL.into());
    }
    Ok(STATIC_REVALIDATE_CACHE_CONTROL.into())
}

/// `QVALUE_PATTERN`: `^(?:0(?:\.\d{0,3})?|1(?:\.0{0,3})?)$` — `\d` is ASCII-only in JS.
fn is_qvalue(value: &str) -> bool {
    fn frac(rest: &str, digit: impl Fn(u8) -> bool) -> bool {
        match rest.strip_prefix('.') {
            None => rest.is_empty(),
            Some(d) => d.len() <= 3 && d.bytes().all(digit),
        }
    }
    if let Some(rest) = value.strip_prefix('0') {
        frac(rest, |c| c.is_ascii_digit())
    } else if let Some(rest) = value.strip_prefix('1') {
        frac(rest, |c| c == b'0')
    } else {
        false
    }
}

/// Port of `parseQValue` (`apps/server/src/staticAssets.ts:48`): `None` is the JS `null`.
fn parse_q_value(raw_params: &[&str]) -> Option<f64> {
    for param in raw_params {
        let normalized = js_trim(param).to_lowercase();
        let Some(separator) = normalized.find('=') else {
            continue;
        };
        let name = &normalized[..separator];
        if js_trim_end(name) != "q" {
            continue;
        }
        let value = &normalized[separator + 1..];
        if name != "q" || value != js_trim(value) {
            return None;
        }
        return if is_qvalue(value) {
            value.parse::<f64>().ok()
        } else {
            None
        };
    }
    Some(1.0)
}

/// Port of `negotiateStaticEncodingPreference` (`apps/server/src/staticAssets.ts:84`).
pub fn negotiate_static_encoding_preference(
    accept_encoding: Option<String>,
) -> Result<Value, String> {
    let Some(header) = accept_encoding.filter(|h| !h.is_empty()) else {
        return Ok(json!({"candidates": [Value::Null], "identityAcceptable": true}));
    };
    // First occurrence of a name wins; `explicit` never overwrites.
    let mut explicit: Vec<(String, f64)> = Vec::new();
    for raw_entry in header.split(',') {
        let mut parts = js_trim(raw_entry).split(';');
        let name = js_trim(parts.next().unwrap_or("")).to_lowercase();
        if name.is_empty() {
            continue;
        }
        let raw_params: Vec<&str> = parts.collect();
        let Some(weight) = parse_q_value(&raw_params) else {
            continue;
        };
        if !explicit.iter().any(|(n, _)| n == &name) {
            explicit.push((name, weight));
        }
    }
    let get = |key: &str| {
        explicit
            .iter()
            .find(|(n, _)| n.as_str() == key)
            .map(|(_, w)| *w)
    };
    let wildcard = get("*");
    let identity_weight = get("identity").or(wildcard).unwrap_or(1.0);
    // (candidate index — `None` is identity —, weight, server preference)
    let mut ranked: Vec<(Option<usize>, f64, usize)> = STATIC_ENCODING_CANDIDATES
        .iter()
        .enumerate()
        .map(|(index, (encoding, _))| {
            (
                Some(index),
                get(encoding).or(wildcard).unwrap_or(0.0),
                index,
            )
        })
        .collect();
    ranked.push((None, identity_weight, STATIC_ENCODING_CANDIDATES.len()));
    ranked.retain(|(_, weight, _)| *weight > 0.0);
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.2.cmp(&b.2)));
    let candidates: Vec<Value> = ranked
        .iter()
        .map(|(candidate, _, _)| match candidate {
            Some(index) => {
                let (encoding, sidecar_extension) = STATIC_ENCODING_CANDIDATES[*index];
                json!({"encoding": encoding, "sidecarExtension": sidecar_extension})
            }
            None => Value::Null,
        })
        .collect();
    Ok(json!({
        "candidates": candidates,
        "identityAcceptable": identity_weight > 0.0,
    }))
}

/// Port of `isSidecarRequestPath` (`apps/server/src/staticAssets.ts:129`).
pub fn is_sidecar_request_path(relative_path: String) -> Result<bool, String> {
    let lowered = relative_path.to_lowercase();
    Ok(lowered.ends_with(".br") || lowered.ends_with(".gz"))
}

/// Port of `opaqueTag` (`apps/server/src/staticAssets.ts:138`): trim, then drop `W/`.
fn opaque_tag(value: &str) -> &str {
    let trimmed = js_trim(value);
    trimmed.strip_prefix("W/").unwrap_or(trimmed)
}

/// Port of `ifNoneMatchSatisfies` (`apps/server/src/staticAssets.ts:143`).
pub fn if_none_match_satisfies(header_value: Option<String>, etag: String) -> Result<bool, String> {
    let Some(header) = header_value.filter(|h| !h.is_empty()) else {
        return Ok(false);
    };
    let trimmed = js_trim(&header);
    if trimmed == "*" {
        return Ok(true);
    }
    let target = opaque_tag(&etag);
    Ok(trimmed.split(',').any(|c| opaque_tag(c) == target))
}

/// `Number.prototype.toString(16)` as V8 prints it: `NaN`, `Infinity`, `0` for `-0`, and the
/// exact base-16 expansion of the double — every finite double terminates in base 16, so the
/// integer part is up to 256 digits and the fraction up to 269 (`5e-324`).
fn js_f64_to_hex16(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value == 0.0 {
        return "0".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let bits = value.abs().to_bits();
    let exp_field = ((bits >> 52) & 0x7ff) as i64;
    // The exact value is m * 2^e; bit `p` of the value (0 is the units bit, negative
    // positions are the fraction) is bit `p - e` of `m`.
    let (m, e) = if exp_field == 0 {
        (bits & ((1 << 52) - 1), -1074)
    } else {
        ((bits & ((1 << 52) - 1)) | (1 << 52), exp_field - 1075)
    };
    let bit = |p: i64| -> u8 {
        let i = p - e;
        if (0..64).contains(&i) {
            ((m >> i) & 1) as u8
        } else {
            0
        }
    };
    // Hex digit covering value bits `base + 3 ..= base`, `base` a multiple of 4.
    let digit = |base: i64| -> char {
        HEX_DIGITS
            [(bit(base + 3) << 3 | bit(base + 2) << 2 | bit(base + 1) << 1 | bit(base)) as usize]
    };
    let hi = 63 - m.leading_zeros() as i64 + e;
    let lo = m.trailing_zeros() as i64 + e;
    let mut out = String::new();
    if value.is_sign_negative() {
        out.push('-');
    }
    if hi >= 0 {
        for k in (0..=hi / 4).rev() {
            out.push(digit(4 * k));
        }
    } else {
        out.push('0');
    }
    if lo < 0 {
        out.push('.');
        for k in 1..=(-lo + 3) / 4 {
            out.push(digit(-4 * k));
        }
    }
    out
}

/// Port of `staticEtag` (`apps/server/src/staticAssets.ts:154`).
pub fn static_etag(size: f64, mtime_ms: f64, encoding: Option<String>) -> Result<String, String> {
    let mut out = format!(
        "W/\"{}-{}",
        js_f64_to_hex16(size),
        js_f64_to_hex16(mtime_ms.trunc())
    );
    // `encoding ? ... : ""` — the empty string is falsy in JS, same as absent.
    if let Some(enc) = encoding.filter(|e| !e.is_empty()) {
        out.push('-');
        out.push_str(&enc);
    }
    out.push('"');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_hex_as_js_does() {
        for (x, js) in [
            (0.0, "0"),
            (-0.0, "0"),
            (2.0, "2"),
            (-2.5, "-2.8"),
            (0.5, "0.8"),
            (7.5, "7.8"),
            (1.5, "1.8"),
            (0.1, "0.1999999999999a"),
            (0.30000000000000004, "0.4cccccccccccd"),
            (1e-7, "0.000001ad7f29abcaf48"),
            (1e21, "3635c9adc5dea00000"),
            (9007199254740992.0, "20000000000000"),
            (2147483648.0, "80000000"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
        ] {
            assert_eq!(js_f64_to_hex16(x), js, "{x:?}");
        }
        assert_eq!(js_f64_to_hex16(5e-324).len(), "0.".len() + 269);
        assert!(js_f64_to_hex16(5e-324).ends_with('4'));
        assert_eq!(js_f64_to_hex16(f64::MAX).len(), 256);
    }

    #[test]
    fn negotiates_as_js_does() {
        // Identity defaults to weight 1, tying gzip and outranking `br;q=0.5`.
        let pref = negotiate_static_encoding_preference(Some("gzip, br;q=0.5".into())).unwrap();
        assert_eq!(
            pref,
            json!({"candidates": [{"encoding": "gzip", "sidecarExtension": ".gz"},
                                 null,
                                 {"encoding": "br", "sidecarExtension": ".br"}],
                   "identityAcceptable": true})
        );
        let pref = negotiate_static_encoding_preference(Some("br;q =0".into())).unwrap();
        // `q =0` is malformed: br falls back to weight 1, not refused.
        assert_eq!(pref["identityAcceptable"], json!(true));
    }
}
