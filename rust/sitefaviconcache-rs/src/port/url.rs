//! Enough of WHATWG URL parsing to extract `.hostname` the way `new URL` does.
//! Adapted from the shared `port/url.rs` helper used by other ported crates;
//! path, query, fragment and serialization handling are dropped because this
//! port only reads the host.

use idna::domain_to_ascii as idna_to_ascii;

fn pct_encode(text: &str, keep: impl Fn(u8) -> bool) -> String {
    let mut out = String::with_capacity(text.len());
    for &b in text.as_bytes() {
        if keep(b) {
            out.push(b as char);
        } else {
            out += &format!("%{b:02X}");
        }
    }
    out
}
fn pct_decode(text: &str) -> Vec<u8> {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && b[i + 1].is_ascii_hexdigit()
            && b[i + 2].is_ascii_hexdigit()
        {
            out.push(u8::from_str_radix(&text[i + 1..i + 3], 16).unwrap_or(0));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}
fn forbidden_domain(b: u8) -> bool {
    b <= 0x20
        || matches!(
            b,
            b'"' | b'#'
                | b'%'
                | b'/'
                | b':'
                | b'<'
                | b'>'
                | b'?'
                | b'@'
                | b'['
                | b'\\'
                | b']'
                | b'^'
                | b'|'
        )
}

fn is_ipv4_number(p: &str) -> bool {
    if p.is_empty() {
        return false;
    }
    if let Some(h) = p.strip_prefix("0x").or_else(|| p.strip_prefix("0X")) {
        return !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit());
    }
    p.bytes().all(|b| b.is_ascii_digit())
}
fn ipv4_value(p: &str) -> Option<u64> {
    if let Some(h) = p.strip_prefix("0x").or_else(|| p.strip_prefix("0X")) {
        return u64::from_str_radix(h, 16).ok();
    }
    let (digits, radix) = if p.len() > 1 && p.starts_with('0') {
        (&p[1..], 8)
    } else {
        (p, 10)
    };
    u64::from_str_radix(if digits.is_empty() { "0" } else { digits }, radix).ok()
}
fn ipv4_normalize(host: &str) -> Option<Option<String>> {
    let mut parts: Vec<&str> = host.split('.').collect();
    if parts.len() > 1 && parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    let last = *parts.last()?;
    if !is_ipv4_number(last) {
        return None;
    }
    if parts.len() > 4 || parts.iter().any(|p| !is_ipv4_number(p)) {
        return Some(None);
    }
    let mut addr = 0u64;
    for (i, p) in parts[..parts.len() - 1].iter().enumerate() {
        let v = ipv4_value(p)?;
        if v > 255 {
            return Some(None);
        }
        addr += v << (24 - 8 * i as u64);
    }
    let last_v = ipv4_value(last)?;
    if last_v >= 1u64 << (8 * (5 - parts.len() as u32)) {
        return Some(None);
    }
    addr += last_v;
    Some(Some(format!(
        "{}.{}.{}.{}",
        addr >> 24,
        (addr >> 16) & 0xff,
        (addr >> 8) & 0xff,
        addr & 0xff
    )))
}
fn special_host(raw: &str, allow_empty: bool) -> Option<String> {
    let bytes = pct_decode(raw);
    let decoded = String::from_utf8_lossy(&bytes);
    if let Some(after_open) = decoded.strip_prefix('[') {
        // IPv6 literals parse to failure in Node without a full parser when they
        // don't close, lack a ':', or hold bytes outside hex/colon/dot/zone.
        let inner = after_open.strip_suffix(']').unwrap_or(after_open);
        if !decoded.ends_with(']')
            || !inner.contains(':')
            || inner
                .bytes()
                .any(|b| !(b.is_ascii_hexdigit() || matches!(b, b':' | b'.' | b'%')))
        {
            return None;
        }
        return Some(decoded.to_lowercase());
    }
    if bytes.iter().any(|&b| forbidden_domain(b)) {
        return None;
    }
    if decoded.is_empty() {
        return allow_empty.then(String::new);
    }
    // Domain to ASCII first, then the ends-in-a-number IPv4 check on its result:
    // IDNA can map non-ASCII onto digits (e.g. 𝟘𝟙 -> "01" -> "0.0.0.1").
    let ascii = idna_to_ascii(&decoded).ok()?;
    // The mapping can also produce code points the input check let through
    // (U+00A0 -> space), which Node's ToASCII rejects as disallowed.
    if ascii.bytes().any(forbidden_domain) {
        return None;
    }
    match ipv4_normalize(&ascii) {
        Some(ip) => ip,
        None => Some(ascii),
    }
}

/// `new URL(input).hostname`, or `None` when `new URL` would throw.
pub fn url_hostname(input: &str) -> Option<String> {
    let cleaned: String = input
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let s = cleaned.trim_matches(|c: char| (c as u32) <= 0x20);
    let b = s.as_bytes();
    if b.is_empty() || !b[0].is_ascii_alphabetic() {
        return None;
    }
    let mut i = 1;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'+' | b'-' | b'.')) {
        i += 1;
    }
    if i >= b.len() || b[i] != b':' {
        return None;
    }
    let scheme = s[..i].to_lowercase();
    let rest = &s[i + 1..];
    if !matches!(
        scheme.as_str(),
        "ftp" | "file" | "http" | "https" | "ws" | "wss"
    ) {
        let Some(r) = rest.strip_prefix("//") else {
            return Some(String::new());
        };
        let end = r.find(['/', '?', '#']).unwrap_or(r.len());
        let host = &r[..end];
        // Opaque host: the forbidden non-special host bytes fail the parse; the
        // rest is UTF-8 percent-encoded with the C0-control set.
        if host.bytes().any(|b| {
            b <= 0x20
                || matches!(
                    b,
                    b'"' | b'#'
                        | b'/'
                        | b':'
                        | b'<'
                        | b'>'
                        | b'?'
                        | b'@'
                        | b'['
                        | b'\\'
                        | b']'
                        | b'^'
                        | b'|'
                )
        }) {
            return None;
        }
        return Some(pct_encode(host, |b| (0x21..0x7f).contains(&b)));
    }
    let mut r = rest;
    let mut slashes = 0;
    while slashes < r.len() && matches!(r.as_bytes()[slashes], b'/' | b'\\') && slashes < 2 {
        slashes += 1;
    }
    r = &r[slashes..];
    let auth_end = r.find(['/', '?', '#', '\\']).unwrap_or(r.len());
    let authority = &r[..auth_end];
    let hostport = match authority.rfind('@') {
        Some(at) => &authority[at + 1..],
        None => authority,
    };
    let (host_raw, port_raw) = if hostport.ends_with(']') {
        (hostport, None)
    } else {
        match hostport.rfind(':') {
            Some(c) => (&hostport[..c], Some(&hostport[c + 1..])),
            None => (hostport, None),
        }
    };
    if let Some(p) = port_raw
        && !p.is_empty()
    {
        if !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let v: u64 = p.parse().ok()?;
        if v > 65535 {
            return None;
        }
    }
    let host = special_host(host_raw, scheme == "file")?;
    if scheme == "file" && host.eq_ignore_ascii_case("localhost") {
        Some(String::new())
    } else {
        Some(host)
    }
}
