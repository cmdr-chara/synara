//! The Rust port of `apps/marketing/scripts/`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

const PAIRED_MDX_COMPONENTS: &[&str] = &[
    "Steps",
    "Step",
    "Callout",
    "Cards",
    "Tabs",
    "Tab",
    "Accordions",
    "Accordion",
];

fn require_string(source: &Value, method: &str) -> Result<String, String> {
    if source.is_null() {
        return Err(format!(
            "Cannot read properties of null (reading '{method}')"
        ));
    }
    match js::string_units(source) {
        Some(units) => {
            let mut s = String::new();
            for chunk in units.chunks(8192) {
                s.extend(
                    chunk
                        .iter()
                        .map(|&u| char::from_u32(u as u32).unwrap_or('\u{FFFD}')),
                );
            }
            Ok(s)
        }
        None => Err(format!("source.{method} is not a function")),
    }
}

/// Port of `parseFrontmatter` (`apps/marketing/scripts/check-docs.mjs:42`).
pub fn parse_frontmatter(source: Value) -> Result<Value, String> {
    let src = require_string(&source, "replaceAll")?;
    let normalized = src.replace("\r\n", "\n");
    let normalized = normalized.as_str();
    if !normalized.starts_with("---\n") {
        return Ok(json!({ "error": "file must begin with a frontmatter block" }));
    }
    let Some(end) = normalized[4..].find("\n---\n").map(|i| i + 4) else {
        return Ok(json!({ "error": "frontmatter block is not closed" }));
    };
    let raw = &normalized[4..end];
    let mut values = serde_json::Map::new();
    for line in raw.split('\n') {
        let bytes = line.as_bytes();
        let first = match bytes.first() {
            Some(&b) if b.is_ascii_alphabetic() => b,
            _ => continue,
        };
        let _ = first;
        let mut i = 1usize;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
        {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b':' {
            continue;
        }
        let key = &line[..i];
        let mut v = &line[i + 1..];
        v = v.trim_start_matches([' ', '\t']);
        let v = strip_quotes(v);
        values.insert(key.to_string(), json!(v));
    }
    Ok(json!({ "values": values, "body": &normalized[end + 5..] }))
}

fn strip_quotes(v: &str) -> String {
    let b = v.as_bytes();
    if b.len() >= 2 {
        if b[0] == b'"' && b[b.len() - 1] == b'"' {
            return v[1..v.len() - 1].to_string();
        }
        if b[0] == b'\'' && b[b.len() - 1] == b'\'' {
            return v[1..v.len() - 1].to_string();
        }
    }
    v.to_string()
}

/// Port of `findUnbalancedCodeFences` (`apps/marketing/scripts/check-docs.mjs:68`).
pub fn find_unbalanced_code_fences(source: Value) -> Result<Value, String> {
    let src = require_string(&source, "replaceAll")?;
    let count = src
        .replace("\r\n", "\n")
        .as_str()
        .split('\n')
        .filter(|line| line.trim_start().starts_with("```"))
        .count();
    Ok(if count % 2 == 0 {
        json!([])
    } else {
        json!(["code fences are unbalanced"])
    })
}

/// Port of `findUnbalancedMdxComponents` (`apps/marketing/scripts/check-docs.mjs:76`).
pub fn find_unbalanced_mdx_components(source: Value) -> Result<Value, String> {
    let src = require_string(&source, "match")?;
    let src = src.as_str();
    let mut errors = Vec::new();
    for component in PAIRED_MDX_COMPONENTS {
        let open_pat = format!("<{component}");
        let close_pat = format!("</{component}>");
        let mut openings = 0usize;
        let mut start = 0usize;
        while let Some(pos) = src[start..].find(&open_pat) {
            let at = start + pos;
            let after = &src[at + open_pat.len()..];
            if after.starts_with(char::is_whitespace) || after.starts_with('>') {
                openings += 1;
            }
            start = at + open_pat.len();
        }
        let closings = src.matches(&close_pat).count();
        if openings != closings {
            errors.push(format!(
                "{component} has {openings} opening tag(s) and {closings} closing tag(s)"
            ));
        }
    }
    Ok(json!(errors))
}

/// Port of `extractInternalLinks` (`apps/marketing/scripts/check-docs.mjs:104`).
pub fn extract_internal_links(source: Value) -> Result<Value, String> {
    let src = require_string(&source, "matchAll")?;
    let src = src.as_str();
    let bytes = src.as_bytes();
    let mut links = Vec::new();

    // /\]\((\/[\w./-]*(?:[?#][^\s)]*)?)\)/
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b'(' && bytes.get(i + 2) == Some(&b'/') {
            let mut j = i + 2;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric()
                    || matches!(bytes[j], b'_' | b'/' | b'.' | b'-'))
            {
                j += 1;
            }
            if matches!(bytes.get(j), Some(b'?') | Some(b'#')) {
                j += 1;
                while j < bytes.len() && !bytes[j].is_ascii_whitespace() && bytes[j] != b')' {
                    j += 1;
                }
            }
            if bytes.get(j) == Some(&b')') {
                push_unique(&mut links, &src[i + 2..j]);
            }
        }
        i += 1;
    }

    // /\bhref=["'](\/[\w./-]*(?:[?#][^"']*)?)["']/
    let mut i = 0usize;
    while i + 6 <= bytes.len() {
        if &src[i..i + 4] == "href"
            && matches!(bytes.get(i + 4), Some(b'"') | Some(b'\''))
            && (i == 0 || !is_word_byte(bytes[i - 1]))
        {
            let quote = bytes[i + 4];
            if bytes.get(i + 5) == Some(&b'/') {
                let mut j = i + 5;
                while j < bytes.len()
                    && (bytes[j].is_ascii_alphanumeric()
                        || matches!(bytes[j], b'_' | b'/' | b'.' | b'-'))
                {
                    j += 1;
                }
                if matches!(bytes.get(j), Some(b'?') | Some(b'#')) {
                    j += 1;
                    while j < bytes.len() && bytes[j] != b'"' && bytes[j] != b'\'' {
                        j += 1;
                    }
                }
                if bytes.get(j) == Some(&quote) {
                    push_unique(&mut links, &src[i + 5..j]);
                }
            }
        }
        i += 1;
    }

    Ok(json!(links))
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn push_unique(links: &mut Vec<String>, s: &str) {
    let s = s.to_string();
    if !links.contains(&s) {
        links.push(s);
    }
}

/// Port of `main` (`apps/marketing/scripts/check-docs.mjs:212`).
pub fn main() -> Result<Value, String> {
    Ok(js::undefined())
}
