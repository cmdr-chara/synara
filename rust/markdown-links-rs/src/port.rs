//! Clean-room Rust implementation of the markdown-links unit, derived from the
//! recorded behavior in `cases.jsonl` (ADR-002).

#![forbid(unsafe_code)]

/// `encodeURIComponent` of one path segment: `A-Z a-z 0-9 - _ . ! ~ * ' ( )`
/// stay literal, every other UTF-8 byte becomes `%XX`.
fn escape_segment(text: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_alphanumeric()
            || matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
        {
            out.push(c);
            continue;
        }
        let mut buf = [0u8; 4];
        for &b in c.encode_utf8(&mut buf).as_bytes() {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 15) as usize] as char);
        }
    }
    out
}

/// Last path segment (`basename` for `/`-separated paths).
fn tail_segment(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// JS `\s` membership (includes NBSP, line/paragraph separators and the BOM).
fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// A `.` inside an ASCII last segment that is neither its first nor its last
/// character — `.name` and `name.` do not count as a file extension, `a.b` does.
/// Positions are UTF-16 code units, like `lastIndexOf`.
fn segment_has_extension(segment: &str) -> bool {
    if !segment.is_ascii() {
        return false;
    }
    let units: Vec<u16> = segment.encode_utf16().collect();
    let Some(dot) = units.iter().rposition(|&u| u == u16::from(b'.')) else {
        return false;
    };
    dot > 0 && dot + 1 < units.len()
}

/// Reference strings the resolvers reject outright: empty, scheme-like (`x:`),
/// or containing whitespace.
fn accepts_file_reference(reference: &str) -> bool {
    !reference.is_empty() && !reference.contains(':') && !reference.chars().any(is_js_whitespace)
}

/// `cwd + "/" + reference` when the reference resolves relative to `cwd`.
fn joined_target(reference: &str, cwd: &str) -> Option<String> {
    if cwd.is_empty() || !accepts_file_reference(reference) {
        return None;
    }
    let base = tail_segment(reference);
    if segment_has_extension(base) || reference == tail_segment(cwd) {
        Some(format!("{cwd}/{reference}"))
    } else {
        None
    }
}

/// `file:///`-style href for a filesystem path.
pub fn markdown_file_path_href(path: String) -> Result<String, String> {
    let encoded = path
        .split('/')
        .map(escape_segment)
        .collect::<Vec<_>>()
        .join("/");
    Ok(format!("file:///{encoded}"))
}

/// Rewrites `file:` URIs to the canonical `file:///path` form; anything else
/// has no markdown-file target.
pub fn rewrite_markdown_file_uri_href(href: Option<String>) -> Result<Option<String>, String> {
    let Some(href) = href else {
        return Ok(None);
    };
    let Some(rest) = href.strip_prefix("file://") else {
        return Ok(None);
    };
    Ok(Some(format!("file:///{}", rest.trim_start_matches('/'))))
}

/// Absolute filesystem paths embedded in `text`: runs of path characters
/// that begin at `/` or a `X:` drive prefix.
pub fn extract_absolute_filesystem_paths(text: String) -> Result<Vec<String>, String> {
    fn is_path_char(c: char) -> bool {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                '/' | '\\' | ':' | '.' | '-' | '_' | '~' | '+' | '=' | '@' | '%'
            )
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        let mut candidate = String::new();
        while i < chars.len() && is_path_char(chars[i]) {
            candidate.push(chars[i]);
            i += 1;
        }
        let bytes = candidate.as_bytes();
        let is_rooted = bytes.len() > 1 && bytes[0] == b'/';
        let is_drive = bytes.len() > 2
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\');
        if is_rooted || is_drive {
            out.push(candidate);
        }
        if i == start {
            i += 1;
        }
    }
    Ok(out)
}

/// The single `known_absolute_paths` entry that ends with `/<reference>`.
pub fn resolve_unique_absolute_suffix_target(
    reference: String,
    known_absolute_paths: Vec<String>,
) -> Result<Option<String>, String> {
    let needle = format!("/{reference}");
    let mut hits = known_absolute_paths
        .into_iter()
        .filter(|p| p.ends_with(&needle));
    match (hits.next(), hits.next()) {
        (Some(path), None) => Ok(Some(path)),
        _ => Ok(None),
    }
}

/// A chip reference resolves to the unique known path ending in it, else to
/// `<cwd>/<reference>` for file-like references.
pub fn resolve_chat_file_chip_target(
    reference: Option<String>,
    cwd: Option<String>,
    known_absolute_paths: Option<Vec<String>>,
) -> Result<Option<String>, String> {
    let Some(reference) = reference else {
        return Ok(None);
    };
    if let Some(paths) = known_absolute_paths
        && let Some(target) = resolve_unique_absolute_suffix_target(reference.clone(), paths)?
    {
        return Ok(Some(target));
    }
    Ok(match cwd {
        Some(cwd) => joined_target(&reference, &cwd),
        None => None,
    })
}

/// `<cwd>/<href>` for file-like hrefs resolved against a non-empty `cwd`.
pub fn resolve_markdown_file_link_target(
    href: Option<String>,
    cwd: Option<String>,
) -> Result<Option<String>, String> {
    let (Some(href), Some(cwd)) = (href, cwd) else {
        return Ok(None);
    };
    Ok(joined_target(&href, &cwd))
}
