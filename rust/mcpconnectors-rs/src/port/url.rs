use idna::domain_to_ascii as idna_to_ascii;

pub struct ParsedUrl {
    pub scheme: String,
    pub opaque_host: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub userinfo: Option<String>,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
    pub opaque_path: bool,
}

impl ParsedUrl {
    pub fn to_js_string(&self) -> String {
        let mut out = format!("{}:", self.scheme);
        if let Some(h) = &self.opaque_host {
            out += "//";
            out += h;
        } else if !self.opaque_path {
            out += "//";
            if let Some(u) = &self.userinfo {
                out += u;
                out.push('@');
            }
            out += &self.host;
            if let Some(p) = self.port {
                out += &format!(":{p}");
            }
        }
        out += &self.path;
        if let Some(q) = &self.query {
            out += &format!("?{q}");
        }
        if let Some(f) = &self.fragment {
            out += &format!("#{f}");
        }
        out
    }
    pub fn hostname(&self) -> &str {
        self.opaque_host.as_deref().unwrap_or(&self.host)
    }
}

pub fn pct_encode(text: &str, keep: impl Fn(u8) -> bool) -> String {
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
pub fn pct_decode(text: &str) -> Vec<u8> {
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
fn pct_set(kind: u8, b: u8) -> bool {
    if !(0x21..0x7f).contains(&b) {
        return false;
    }
    match kind {
        0 => !matches!(b, b'"' | b'#' | b'<' | b'>' | b'?' | b'`' | b'{' | b'}'),
        1 => !matches!(b, b'"' | b'#' | b'<' | b'>' | b'\''),
        _ => !matches!(b, b'"' | b'<' | b'>' | b'`'),
    }
}
#[allow(dead_code)]
pub fn encode_uri_component(text: &str) -> String {
    pct_encode(text, |b| {
        b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
    })
}
pub fn forbidden_domain(b: u8) -> bool {
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

pub fn is_ipv4_number(p: &str) -> bool {
    if p.is_empty() {
        return false;
    }
    if let Some(h) = p.strip_prefix("0x").or_else(|| p.strip_prefix("0X")) {
        return !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit());
    }
    p.bytes().all(|b| b.is_ascii_digit())
}
pub fn ipv4_value(p: &str) -> Option<u64> {
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
pub fn ipv4_normalize(host: &str) -> Option<Option<String>> {
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
pub fn special_host(raw: &str, allow_empty: bool) -> Option<String> {
    let bytes = pct_decode(raw);
    let decoded = String::from_utf8_lossy(&bytes);
    if decoded.starts_with('[') {
        return Some(decoded.to_lowercase());
    }
    if bytes.iter().any(|&b| forbidden_domain(b)) {
        return None;
    }
    if decoded.is_empty() {
        return allow_empty.then(String::new);
    }
    match ipv4_normalize(&decoded) {
        Some(ip) => ip,
        None => idna_to_ascii(&decoded).ok(),
    }
}
pub fn normalize_path(path: &str) -> String {
    let kind = |s: &str| match s.to_ascii_lowercase().as_str() {
        "." | "%2e" => 1,
        ".." | "%2e." | ".%2e" | "%2e%2e" => 2,
        _ => 0,
    };
    let segs: Vec<&str> = path.split('/').collect();
    let mut out: Vec<&str> = Vec::new();
    for (i, seg) in segs.iter().enumerate() {
        match kind(seg) {
            2 => {
                out.pop();
                if i == segs.len() - 1 {
                    out.push("");
                }
            }
            1 if i == segs.len() - 1 => out.push(""),
            0 => out.push(seg),
            _ => {}
        }
    }
    out.join("/")
}
pub fn split_qf(s: &str) -> (&str, Option<&str>, Option<&str>) {
    let (head, fragment) = match s.find('#') {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    match head.find('?') {
        Some(i) => (&head[..i], Some(&head[i + 1..]), fragment),
        None => (head, None, fragment),
    }
}

pub fn parse_url(input: &str) -> Option<ParsedUrl> {
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
        let (opaque_host, after) = if let Some(r) = rest.strip_prefix("//") {
            let end = r.find(['/', '?', '#']).unwrap_or(r.len());
            let host = &r[..end];
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
            (Some(host.to_string()), &r[end..])
        } else {
            (None, rest)
        };
        let (path_part, query, fragment) = split_qf(after);
        return Some(ParsedUrl {
            scheme,
            opaque_host,
            host: String::new(),
            port: None,
            userinfo: None,
            path: pct_encode(path_part, |b| (0x20..0x7f).contains(&b)),
            query: query.map(|q| pct_encode(q, |b| (0x20..0x7f).contains(&b) && b != b'"')),
            fragment: fragment.map(|f| pct_encode(f, |b| pct_set(2, b))),
            opaque_path: true,
        });
    }
    let mut r = rest;
    let mut slashes = 0;
    while slashes < r.len() && matches!(r.as_bytes()[slashes], b'/' | b'\\') && slashes < 2 {
        slashes += 1;
    }
    r = &r[slashes..];
    let auth_end = r.find(['/', '?', '#', '\\']).unwrap_or(r.len());
    let authority = &r[..auth_end];
    let tail = &r[auth_end..];
    let (userinfo, hostport) = match authority.rfind('@') {
        Some(at) => (Some(authority[..at].to_string()), &authority[at + 1..]),
        None => (None, authority),
    };
    let (host_raw, port_raw) = if hostport.ends_with(']') {
        (hostport, None)
    } else {
        match hostport.rfind(':') {
            Some(c) => (&hostport[..c], Some(&hostport[c + 1..])),
            None => (hostport, None),
        }
    };
    let port = match port_raw {
        Some("") | None => None,
        Some(p) => {
            if !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let v: u64 = p.parse().ok()?;
            if v > 65535 {
                return None;
            }
            let v = v as u16;
            let default = match scheme.as_str() {
                "ftp" => 21,
                "http" | "ws" => 80,
                "https" | "wss" => 443,
                _ => 0,
            };
            (v != default).then_some(v)
        }
    };
    let host = special_host(host_raw, scheme == "file")?;
    let host = if scheme == "file" && host.eq_ignore_ascii_case("localhost") {
        String::new()
    } else {
        host
    };
    let (path_part, query, fragment) = split_qf(tail);
    let normalized = normalize_path(&path_part.replace('\\', "/"));
    let path = pct_encode(&normalized, |b| pct_set(0, b));
    let path = if !path.starts_with('/') && !host.is_empty() {
        format!("/{path}")
    } else {
        path
    };
    Some(ParsedUrl {
        scheme,
        opaque_host: None,
        host,
        port,
        userinfo,
        path: if path.is_empty() { "/".into() } else { path },
        query: query.map(|q| pct_encode(q, |b| pct_set(1, b))),
        fragment: fragment.map(|f| pct_encode(f, |b| pct_set(2, b))),
        opaque_path: false,
    })
}
