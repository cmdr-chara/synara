//! The Rust port of `packages/shared/src/browserAnnotations.ts`.

#![forbid(unsafe_code)]

mod url;

use std::sync::OnceLock;

use regex::Regex;

const REDACTED_VALUE: &str = "REDACTED";

const SAFE_QUERY_KEYS: &[&str] = &[
    "lang", "language", "locale", "mode", "order", "page", "section", "sort", "tab", "view",
];

const SAFE_STATIC_PATH_SEGMENTS: &[&str] = &[
    "about",
    "activity",
    "app",
    "articles",
    "blog",
    "board",
    "boards",
    "calendar",
    "code",
    "dashboard",
    "details",
    "docs",
    "documentation",
    "download",
    "downloads",
    "editor",
    "features",
    "files",
    "general",
    "help",
    "history",
    "home",
    "index",
    "install",
    "installation",
    "issues",
    "landing",
    "next",
    "overview",
    "page",
    "pages",
    "pricing",
    "preview",
    "product",
    "products",
    "project",
    "projects",
    "repos",
    "repositories",
    "search",
    "settings",
    "support",
    "task",
    "tasks",
    "team",
    "teams",
    "thread",
    "threads",
    "tickets",
    "timeline",
    "workspace",
    "workspaces",
];

const SENSITIVE_PATH_KEYS: &[&str] = &[
    "account",
    "accounts",
    "auth",
    "authorize",
    "callback",
    "client",
    "clients",
    "contact",
    "contacts",
    "credential",
    "credentials",
    "customer",
    "customers",
    "employee",
    "employees",
    "invite",
    "invites",
    "join",
    "magic",
    "member",
    "members",
    "oauth",
    "patient",
    "patients",
    "people",
    "person",
    "profile",
    "profiles",
    "record",
    "records",
    "reset",
    "s",
    "session",
    "sessions",
    "share",
    "shared",
    "shares",
    "staff",
    "token",
    "tokens",
    "user",
    "users",
    "verification",
    "verify",
];

fn safe_static_query_values(key: &str) -> Option<&'static [&'static str]> {
    match key {
        "mode" => Some(&[
            "auto", "compact", "dark", "default", "edit", "expanded", "light", "preview", "view",
        ]),
        "order" => Some(&["asc", "desc"]),
        "section" => Some(&[
            "activity",
            "code",
            "comments",
            "details",
            "features",
            "files",
            "general",
            "history",
            "install",
            "installation",
            "overview",
            "preview",
            "settings",
        ]),
        "sort" => Some(&[
            "created", "date", "name", "position", "priority", "status", "title", "updated",
        ]),
        "tab" => Some(&[
            "activity", "code", "comments", "details", "files", "general", "history", "overview",
            "preview", "settings",
        ]),
        "view" => Some(&[
            "board", "calendar", "compact", "details", "expanded", "grid", "kanban", "list",
            "preview", "table", "timeline",
        ]),
        _ => None,
    }
}

fn email_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?-u:\b)[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}(?-u:\b)").unwrap()
    })
}

fn exposed_credential_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r"(?i)(?-u:\b)(?:authorization|password|passwd|secret|api[-_ ]?key|access[-_ ]?token|refresh[-_ ]?token|session[-_ ]?token|credential)s?\s*(?::|=)\s*\S+",
        )
        .unwrap()
    })
}

fn bearer_credential_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)(?-u:\b)bearer\s+\S+").unwrap())
}

fn url_credential_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)[a-z][a-z0-9+.-]*://[^/\s:@]+:[^/\s@]+@").unwrap())
}

fn jwt_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?-u:\b)eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+)?(?-u:\b)")
            .unwrap()
    })
}

fn uuid_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r"(?i)(?-u:\b)[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}(?-u:\b)",
        )
        .unwrap()
    })
}

fn long_hex_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)(?-u:\b)[0-9a-f]{24,}(?-u:\b)").unwrap())
}

fn opaque_candidate_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[A-Za-z0-9._~+/-]{12,}").unwrap())
}

fn card_candidate_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?:\d[ -]?){13,19}").unwrap())
}

fn private_word_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r"(?i)(?:^|[^A-Za-z0-9])(?:bearer|password|passwd|secret|token|credential)(?:$|[^A-Za-z0-9])",
        )
        .unwrap()
    })
}

fn jwt_full_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"^eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+)?$").unwrap()
    })
}

fn uuid_full_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap()
    })
}

fn digits4_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^\d{4,}$").unwrap())
}

fn opaque_full_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[A-Za-z0-9._~-]+$").unwrap())
}

fn lang_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[A-Za-z]{2,3}(?:-[A-Za-z]{2,4})?$").unwrap())
}

fn page_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^\d{1,6}$").unwrap())
}

fn is_js_space(c: char) -> bool {
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

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

fn decode_uri_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            let b = u8::from_str_radix(hex, 16).ok()?;
            out.push(b);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn decode_for_inspection(value: &str) -> String {
    decode_uri_component(value).unwrap_or_else(|| value.to_string())
}

/// Lossy query-component decoding: `+` becomes space, percent bytes decode as UTF-8
/// with invalid sequences replaced, like URLSearchParams.
fn decode_query_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
                    && let Ok(b) = u8::from_str_radix(hex, 16)
                {
                    out.push(b);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// application/x-www-form-urlencoded percent encoding used by URLSearchParams.toString.
fn encode_query_component(value: &str) -> String {
    let mut out = String::new();
    for &b in value.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

fn is_payment_card_number(value: &str) -> bool {
    let digits: String = value.chars().filter(|c| *c != ' ' && *c != '-').collect();
    let dlen = digits.len();
    if !(13..=19).contains(&dlen)
        || !digits.bytes().all(|b| b.is_ascii_digit())
        || digits.bytes().all(|b| b == digits.as_bytes()[0])
    {
        return false;
    }
    let parity = dlen % 2;
    let mut checksum = 0u32;
    for (index, b) in digits.bytes().enumerate() {
        let mut digit = u32::from(b - b'0');
        if index % 2 == parity {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }
        checksum += digit;
    }
    checksum.is_multiple_of(10)
}

fn contains_payment_card_number(value: &str) -> bool {
    card_candidate_re()
        .find_iter(value)
        .any(|m| is_payment_card_number(m.as_str()))
}

fn is_opaque_identifier(value: &str, full_title: &str) -> bool {
    let has_letter = value.bytes().any(|b| b.is_ascii_alphabetic());
    let has_digit = value.bytes().any(|b| b.is_ascii_digit());
    if !has_letter || !has_digit {
        return false;
    }
    if value.len() >= 24 {
        return true;
    }
    if value != full_title || value.len() < 12 {
        return false;
    }
    value.bytes().any(|b| b.is_ascii_uppercase()) && value.bytes().any(|b| b.is_ascii_lowercase())
}

/// Port of `sanitizeBrowserAnnotationPageTitle` (`packages/shared/src/browserAnnotations.ts:237`).
pub fn sanitize_browser_annotation_page_title(value: String) -> Result<String, String> {
    let title = js_trim(&value);
    if email_re().is_match(title)
        || exposed_credential_re().is_match(title)
        || bearer_credential_re().is_match(title)
        || url_credential_re().is_match(title)
        || jwt_re().is_match(title)
        || uuid_re().is_match(title)
        || long_hex_re().is_match(title)
        || contains_payment_card_number(title)
    {
        return Ok(String::new());
    }
    let redact = opaque_candidate_re()
        .find_iter(title)
        .any(|m| is_opaque_identifier(m.as_str(), title));
    Ok(if redact { String::new() } else { value })
}

fn looks_private(value: &str) -> bool {
    let decoded = decode_for_inspection(value);
    utf16_len(&decoded) > 96
        || private_word_re().is_match(&decoded)
        || email_re().is_match(&decoded)
        || jwt_full_re().is_match(&decoded)
        || uuid_full_re().is_match(&decoded)
        || digits4_re().is_match(&decoded)
        || (utf16_len(&decoded) >= 6
            && opaque_full_re().is_match(&decoded)
            && decoded.bytes().any(|b| b.is_ascii_alphabetic())
            && decoded.bytes().any(|b| b.is_ascii_digit()))
}

fn is_sensitive_path_key(value: &str) -> bool {
    value
        .to_lowercase()
        .split(['-', '_'])
        .any(|part| SENSITIVE_PATH_KEYS.contains(&part))
}

fn is_explicitly_static_path_segment(value: &str) -> bool {
    if value == REDACTED_VALUE {
        return true;
    }
    let normalized = value.to_lowercase();
    SAFE_STATIC_PATH_SEGMENTS.contains(&normalized.as_str()) || is_sensitive_path_key(&normalized)
}

fn sanitize_pathname(pathname: &str) -> String {
    let mut redact_next = false;
    pathname
        .split('/')
        .map(|segment| {
            if segment.is_empty() {
                return segment.to_string();
            }
            let decoded = decode_for_inspection(segment);
            let should_redact = redact_next
                || looks_private(&decoded)
                || !is_explicitly_static_path_segment(&decoded);
            redact_next = is_sensitive_path_key(&decoded);
            if should_redact {
                REDACTED_VALUE.to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn is_safe_query_value(key: &str, value: &str) -> bool {
    if value == REDACTED_VALUE {
        return true;
    }
    if looks_private(value) {
        return false;
    }
    if key == "lang" || key == "language" || key == "locale" {
        return lang_re().is_match(value);
    }
    if key == "page" {
        return page_re().is_match(value);
    }
    match safe_static_query_values(key) {
        Some(values) => values.contains(&value.to_lowercase().as_str()),
        None => false,
    }
}

fn sanitize_query(query: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    if !query.is_empty() {
        for pair in query.split('&') {
            let (raw_key, raw_value) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };
            let key = decode_query_component(raw_key);
            let value = decode_query_component(raw_value);
            let normalized_key = js_trim(&key).to_lowercase();
            if !SAFE_QUERY_KEYS.contains(&normalized_key.as_str()) {
                continue;
            }
            let kept = if is_safe_query_value(&normalized_key, &value) {
                value
            } else {
                REDACTED_VALUE.to_string()
            };
            out.push(format!(
                "{}={}",
                encode_query_component(&normalized_key),
                encode_query_component(&kept)
            ));
        }
    }
    out.join("&")
}

/// Port of `sanitizeBrowserAnnotationUrl` (`packages/shared/src/browserAnnotations.ts:333`).
pub fn sanitize_browser_annotation_url(value: String) -> Result<String, String> {
    let Some(mut parsed) = url::parse_url(&value) else {
        return Ok(String::new());
    };
    if parsed.scheme != "http" && parsed.scheme != "https" {
        return Ok(String::new());
    }
    let canonical_input = parsed.to_js_string();
    parsed.userinfo = None;
    parsed.path = sanitize_pathname(&parsed.path);
    let sanitized_query = sanitize_query(parsed.query.as_deref().unwrap_or(""));
    parsed.query = if sanitized_query.is_empty() {
        None
    } else {
        Some(sanitized_query)
    };
    parsed.fragment = None;
    let out = parsed.to_js_string();
    Ok(if out == canonical_input { value } else { out })
}

/// Port of `browserAnnotationDocumentIdentityUrl` (`packages/shared/src/browserAnnotations.ts:357`).
pub fn browser_annotation_document_identity_url(value: String) -> Result<String, String> {
    let Some(mut parsed) = url::parse_url(&value) else {
        return Ok(String::new());
    };
    if parsed.scheme != "http" && parsed.scheme != "https" {
        return Ok(String::new());
    }
    parsed.fragment = None;
    Ok(parsed.to_js_string())
}
