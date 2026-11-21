//! The Rust port of `packages/shared/src/windowsCertificate.ts`.
//!
//! Distinguished-name parsing and matching for Windows client certificates.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn trim_err(v: &Value) -> Result<String, String> {
    match v.as_str() {
        Some(s) => Ok(s.trim().to_string()),
        None if v.is_null() => Err("Cannot read properties of null (reading 'trim')".to_string()),
        None if crate::js::is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'trim')".to_string())
        }
        None => Err("dn.trim is not a function".to_string()),
    }
}

fn unescape(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let c = chars[i + 1];
            if c.is_ascii_hexdigit()
                && i + 2 < chars.len()
                && chars[i + 2].is_ascii_hexdigit()
                && let Ok(b) = u8::from_str_radix(&format!("{}{}", c, chars[i + 2]), 16)
            {
                out.push(b as char);
                i += 3;
                continue;
            }
            out.push(c);
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn split_parts(dn: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = dn.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            '\\' => {
                cur.push('\\');
                if let Some(&n) = chars.peek() {
                    cur.push(n);
                    chars.next();
                }
            }
            ',' if !quoted => {
                parts.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    parts
}

fn parse_map(dn: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for part in split_parts(dn) {
        let p = part.trim();
        let Some(eq) = p.find('=') else { continue };
        let key = p[..eq].trim();
        let mut val = p[eq + 1..].trim().to_string();
        if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
            val = val[1..val.len() - 1].to_string();
        }
        let val = unescape(&val);
        if key.is_empty() {
            continue;
        }
        if let Some(e) = out.iter_mut().find(|(k, _)| *k == key) {
            e.1 = val;
        } else {
            out.push((key.to_string(), val));
        }
    }
    out
}

/// Port of `parseDistinguishedName` (`packages/shared/src/windowsCertificate.ts:5`).
pub fn parse_distinguished_name(sequence: Value) -> Result<Value, String> {
    let dn = trim_err(&sequence)?;
    let map: Vec<Value> = parse_map(&dn)
        .into_iter()
        .map(|(k, v)| json!([k, v]))
        .collect();
    Ok(json!({ "$map": map }))
}

/// Port of `matchesDistinguishedName` (`packages/shared/src/windowsCertificate.ts:84`).
pub fn matches_distinguished_name(expected: Value, actual: Value) -> Result<bool, String> {
    let dn = trim_err(&expected)?;
    let spec = trim_err(&actual)?;
    let dn_map = parse_map(&dn);
    let spec_map = parse_map(&spec);
    if dn_map.len() < 2 {
        return Ok(false);
    }
    Ok(dn_map
        .iter()
        .all(|(k, v)| spec_map.iter().any(|(k2, v2)| k2 == k && v2 == v)))
}
