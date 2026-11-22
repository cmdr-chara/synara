//! The subset of the WHATWG `URL` `wsTransport.ts` exercises: scheme-prefixed
//! absolute URLs with opaque or hierarchical paths, `?`/`#` splits, the
//! `protocol`/`pathname`/`hostname` getters and setters it touches, and
//! `searchParams` serialization.

#![forbid(unsafe_code)]

/// A parsed absolute URL. `kind` is `Opaque` for `scheme:rest` URLs (the whole
/// recorded corpus) and `Path` for `scheme:/...` and `scheme://host/...`.
#[derive(Debug)]
pub struct Url {
    scheme: String,
    kind: Kind,
    query: Option<String>,
    fragment: Option<String>,
}

#[derive(Debug)]
enum Kind {
    /// Cannot-be-a-base-URL: `pathname` setter is a no-op, no host.
    Opaque { path: String },
    /// `authority` records whether the source used `//` (`a:/x` has an empty
    /// host but serializes without a marker).
    Path {
        authority: bool,
        host: String,
        port: Option<String>,
        path: String,
    },
}

fn is_special(scheme: &str) -> bool {
    matches!(scheme, "http" | "https" | "ws" | "wss" | "ftp" | "file")
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}

fn pct(byte: u8, out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push('%');
    out.push(HEX[(byte >> 4) as usize] as char);
    out.push(HEX[(byte & 15) as usize] as char);
}

/// C0 control percent-encode set: < 0x20 and > 0x7E (opaque paths). Space stays
/// literal — Bun serializes `a:x y` as `a:x y`, not `a:x%20y`.
fn encode_opaque_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let mut buf = [0u8; 4];
        for &b in c.encode_utf8(&mut buf).as_bytes() {
            if !(0x20..=0x7e).contains(&b) {
                pct(b, &mut out);
            } else {
                out.push(b as char);
            }
        }
    }
    out
}

/// Path percent-encode set: C0 plus space `"` `#` `<` `>` `?` `` ` `` `{` `}`.
fn encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let mut buf = [0u8; 4];
        for &b in c.encode_utf8(&mut buf).as_bytes() {
            if !(0x21..=0x7e).contains(&b)
                || matches!(b, b'"' | b'#' | b'<' | b'>' | b'?' | b'`' | b'{' | b'}')
            {
                pct(b, &mut out);
            } else {
                out.push(b as char);
            }
        }
    }
    out
}

/// Opaque-host encode set: C0 + space `"` `#` `/` `:` `<` `>` `?` `@` `[` `\`
/// `]` `^` `|`.
fn encode_opaque_host(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let mut buf = [0u8; 4];
        for &b in c.encode_utf8(&mut buf).as_bytes() {
            if !(0x21..=0x7e).contains(&b)
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
            {
                pct(b, &mut out);
            } else {
                out.push(b as char);
            }
        }
    }
    out
}

/// Query encode set: C0 + space `"` `#` `<` `>` (plus `'` for special schemes).
fn encode_query(s: &str, special: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let mut buf = [0u8; 4];
        for &b in c.encode_utf8(&mut buf).as_bytes() {
            if !(0x21..=0x7e).contains(&b)
                || matches!(b, b'"' | b'#' | b'<' | b'>')
                || (special && b == b'\'')
            {
                pct(b, &mut out);
            } else {
                out.push(b as char);
            }
        }
    }
    out
}

/// application/x-www-form-urlencoded: alnum `*` `-` `.` `_` literal, space `+`.
fn encode_urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == ' ' {
            out.push('+');
            continue;
        }
        let mut buf = [0u8; 4];
        for &b in c.encode_utf8(&mut buf).as_bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'*' | b'-' | b'.' | b'_') {
                out.push(b as char);
            } else {
                pct(b, &mut out);
            }
        }
    }
    out
}

fn decode_urlencoded(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len()
                && bytes[i + 1].is_ascii_hexdigit()
                && bytes[i + 2].is_ascii_hexdigit() =>
            {
                out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap_or(b'%'));
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn split_port(authority: &str, special: bool, scheme: &str) -> (String, Option<String>) {
    let (host, port) = if authority.starts_with('[') {
        match authority.find(']') {
            Some(e) => {
                let host = &authority[..=e];
                let rest = &authority[e + 1..];
                (host, rest.strip_prefix(':'))
            }
            None => (authority, None),
        }
    } else {
        match authority.rfind(':') {
            Some(c)
                if !authority[c + 1..].is_empty()
                    && authority[c + 1..].chars().all(|c| c.is_ascii_digit()) =>
            {
                (&authority[..c], Some(&authority[c + 1..]))
            }
            _ => (authority, None),
        }
    };
    let host = if special {
        host.to_lowercase()
    } else {
        encode_opaque_host(host)
    };
    let port = port.and_then(|p| {
        if p.parse::<u16>().ok() == default_port(scheme) {
            None
        } else {
            Some(p.to_string())
        }
    });
    (host, port)
}

/// `new URL(input)` — absolute parse only; anything without a `scheme:` prefix
/// fails like the constructor does.
pub fn parse(input: &str) -> Result<Url, String> {
    let cleaned: String = input
        .chars()
        .filter(|c| *c != '\t' && *c != '\n' && *c != '\r')
        .collect();
    let t = cleaned.trim_matches(|c: char| (c as u32) <= 0x20);
    let colon = match t.find(':') {
        Some(c) => c,
        None => return Err("Invalid URL".to_string()),
    };
    let scheme_raw = &t[..colon];
    let mut chars = scheme_raw.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !valid {
        return Err("Invalid URL".to_string());
    }
    let scheme = scheme_raw.to_ascii_lowercase();
    let special = is_special(&scheme);
    let rest = &t[colon + 1..];

    let (rest, fragment) = match rest.find('#') {
        Some(p) => (&rest[..p], Some(rest[p + 1..].to_string())),
        None => (rest, None),
    };
    let (main, query) = match rest.find('?') {
        Some(p) => (&rest[..p], Some(rest[p + 1..].to_string())),
        None => (rest, None),
    };

    let kind = if let Some(auth) = main.strip_prefix("//") {
        let end = auth.find(['/', '?', '#']).unwrap_or(auth.len());
        let (host, port) = split_port(&auth[..end], special, &scheme);
        Kind::Path {
            authority: true,
            host,
            port,
            path: encode_path(&auth[end..]),
        }
    } else if main.starts_with('/') {
        Kind::Path {
            authority: false,
            host: String::new(),
            port: None,
            path: encode_path(main),
        }
    } else if special && !main.is_empty() {
        // `ws:x` parses like `ws://x/`.
        let end = main.find('/').unwrap_or(main.len());
        let (host, port) = split_port(&main[..end].to_lowercase(), true, &scheme);
        let path = if end < main.len() {
            encode_path(&main[end..])
        } else {
            String::new()
        };
        Kind::Path {
            authority: true,
            host,
            port,
            path,
        }
    } else if special {
        Kind::Path {
            authority: true,
            host: String::new(),
            port: None,
            path: String::new(),
        }
    } else {
        Kind::Opaque {
            path: encode_opaque_path(main),
        }
    };

    Ok(Url {
        scheme,
        kind,
        query: query.map(|q| encode_query(&q, special)),
        fragment: fragment.map(|f| {
            let mut out = String::new();
            for c in f.chars() {
                let mut buf = [0u8; 4];
                for &b in c.encode_utf8(&mut buf).as_bytes() {
                    if !(0x21..=0x7e).contains(&b) || matches!(b, b'"' | b'<' | b'>' | b'`') {
                        pct(b, &mut out);
                    } else {
                        out.push(b as char);
                    }
                }
            }
            out
        }),
    })
}

impl Url {
    /// `url.protocol` — `"scheme:"`.
    pub fn protocol(&self) -> String {
        format!("{}:", self.scheme)
    }

    /// `url.protocol = "x:"` — refused when it would switch the special set.
    pub fn set_protocol(&mut self, new_scheme: &str) {
        let new_scheme = new_scheme.trim_end_matches(':').to_ascii_lowercase();
        if is_special(&self.scheme) != is_special(&new_scheme) {
            return;
        }
        self.scheme = new_scheme;
    }

    /// `url.hostname` — `""` for opaque-path URLs.
    pub fn hostname(&self) -> &str {
        match &self.kind {
            Kind::Opaque { .. } => "",
            Kind::Path { host, .. } => host,
        }
    }

    /// `url.pathname` — the opaque path, or the `/`-rooted path ("/" when unset).
    pub fn pathname(&self) -> String {
        match &self.kind {
            Kind::Opaque { path } => path.clone(),
            Kind::Path { path, .. } => {
                if path.is_empty() {
                    "/".to_string()
                } else {
                    path.clone()
                }
            }
        }
    }

    /// `url.pathname = ...` — a no-op for cannot-be-a-base-URL paths. The path
    /// parser roots the value at `/` when it is missing one.
    pub fn set_pathname(&mut self, new_path: &str) {
        if let Kind::Path { path, .. } = &mut self.kind {
            if new_path.starts_with('/') {
                *path = encode_path(new_path);
            } else {
                *path = encode_path(&format!("/{new_path}"));
            }
        }
    }

    fn params(&self) -> Vec<(String, String)> {
        let Some(q) = &self.query else {
            return Vec::new();
        };
        q.split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                (decode_urlencoded(k), decode_urlencoded(v))
            })
            .collect()
    }

    fn write_params(&mut self, params: &[(String, String)]) {
        if params.is_empty() {
            self.query = None;
            return;
        }
        self.query = Some(
            params
                .iter()
                .map(|(k, v)| format!("{}={}", encode_urlencoded(k), encode_urlencoded(v)))
                .collect::<Vec<_>>()
                .join("&"),
        );
    }

    /// `url.searchParams.set(name, value)`.
    pub fn set_param(&mut self, name: &str, value: &str) {
        let mut params = self.params();
        if let Some(first) = params.iter_mut().find(|(k, _)| k == name) {
            first.1 = value.to_string();
            params.retain(|(k, _)| k == name && false || k != name);
        } else {
            params.push((name.to_string(), value.to_string()));
        }
        self.write_params(&params);
    }

    /// `url.searchParams.append(name, value)`.
    pub fn append_param(&mut self, name: &str, value: &str) {
        let mut params = self.params();
        params.push((name.to_string(), value.to_string()));
        self.write_params(&params);
    }

    /// `url.toString()`.
    pub fn serialize(&self) -> String {
        let mut out = format!("{}:", self.scheme);
        match &self.kind {
            Kind::Opaque { path } => out.push_str(path),
            Kind::Path {
                authority,
                host,
                port,
                path,
            } => {
                if *authority {
                    out.push_str("//");
                    out.push_str(host);
                    if let Some(p) = port {
                        out.push(':');
                        out.push_str(p);
                    }
                    if path.is_empty() {
                        out.push('/');
                    } else {
                        out.push_str(path);
                    }
                } else {
                    out.push_str(path);
                }
            }
        }
        if let Some(q) = &self.query {
            out.push('?');
            out.push_str(q);
        }
        if let Some(f) = &self.fragment {
            out.push('#');
            out.push_str(f);
        }
        out
    }
}
