//! The Rust port of `apps/desktop/src/browserSessionPolicy.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_nil(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn is_ws(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(o) => {
            if crate::js::is_undefined(v) {
                return false;
            }
            if let Some(x) = crate::js::as_number(v) {
                return x != 0.0 && !x.is_nan();
            }
            if let Some(units) = crate::js::string_units(v) {
                return !units.is_empty();
            }
            if o.len() == 1
                && let Some(d) = o.get("$bigint").and_then(Value::as_str)
            {
                return d != "0";
            }
            true
        }
    }
}

/// `String(value)` for `values.join(",")` elements: null/undefined render as "".
fn join_elem(v: &Value) -> String {
    if is_nil(v) {
        return String::new();
    }
    js_to_string(v)
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(join_elem).collect::<Vec<_>>().join(","),
        Value::Object(o) => {
            if let Some(units) = crate::js::string_units(v) {
                return String::from_utf16_lossy(&units);
            }
            if let Some(n) = crate::js::as_number(v) {
                return crate::js::number_to_string(n);
            }
            if o.len() == 1 {
                if let Some(d) = o.get("$bigint").and_then(Value::as_str) {
                    return d.to_string();
                }
                if o.contains_key("$undefined") {
                    return "undefined".into();
                }
            }
            "[object Object]".into()
        }
    }
}

/// `Object.entries(headers)`: own enumerable string keys; primitives have none.
fn entries_of(headers: &Value) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    match headers {
        Value::Object(o) => {
            if let Some(units) = crate::js::string_units(headers) {
                for (i, u) in units.iter().enumerate() {
                    out.push((i.to_string(), crate::js::utf16_string(&[*u])));
                }
            } else if o.len() == 1
                && let Some(bytes) = o
                    .get("$bytes")
                    .or_else(|| o.get("$buffer"))
                    .and_then(Value::as_array)
            {
                for (i, b) in bytes.iter().enumerate() {
                    out.push((i.to_string(), b.clone()));
                }
            } else if !(o.len() == 1
                && (o.contains_key("$undefined")
                    || o.contains_key("$num")
                    || o.contains_key("$bigint")
                    || o.contains_key("$date")
                    || o.contains_key("$map")
                    || o.contains_key("$set")))
            {
                for (k, v) in o {
                    out.push((k.clone(), v.clone()));
                }
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                out.push((i.to_string(), v.clone()));
            }
        }
        Value::String(s) => {
            for (i, u) in s.encode_utf16().enumerate() {
                out.push((i.to_string(), crate::js::utf16_string(&[u])));
            }
        }
        _ => {}
    }
    out
}

/// `/(?:^|,)\s*tools\s*=\s*(\*|\([^)]*\))/giu` — returns the captured allow-lists.
fn tools_directives(policy: &str) -> Vec<String> {
    let chars: Vec<char> = policy.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i <= chars.len() {
        let anchored = i == 0 || chars[i - 1] == ',';
        if !anchored {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < chars.len() && is_ws(chars[j]) {
            j += 1;
        }
        if !policy_tools_at(&chars, j) {
            i += 1;
            continue;
        }
        j += 5;
        while j < chars.len() && is_ws(chars[j]) {
            j += 1;
        }
        if j >= chars.len() || chars[j] != '=' {
            i += 1;
            continue;
        }
        j += 1;
        while j < chars.len() && is_ws(chars[j]) {
            j += 1;
        }
        if j < chars.len() && chars[j] == '*' {
            out.push("*".to_string());
            i = j + 1;
            continue;
        }
        if j < chars.len() && chars[j] == '(' {
            let mut k = j + 1;
            while k < chars.len() && chars[k] != ')' {
                k += 1;
            }
            if k < chars.len() {
                out.push(chars[j..=k].iter().collect());
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn policy_tools_at(chars: &[char], j: usize) -> bool {
    let word = ['T', 'O', 'O', 'L', 'S'];
    j + 5 <= chars.len()
        && chars[j..j + 5]
            .iter()
            .zip(word.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// `/(?:^|,)\s*tools\s*=/iu.test(policy)`
fn has_tools_directive(policy: &str) -> bool {
    let chars: Vec<char> = policy.chars().collect();
    for i in 0..=chars.len() {
        if i != 0 && chars[i - 1] != ',' {
            continue;
        }
        let mut j = i;
        while j < chars.len() && is_ws(chars[j]) {
            j += 1;
        }
        if !policy_tools_at(&chars, j) {
            continue;
        }
        j += 5;
        while j < chars.len() && is_ws(chars[j]) {
            j += 1;
        }
        if j < chars.len() && chars[j] == '=' {
            return true;
        }
    }
    false
}

/// `allowList.slice(1, -1).match(/(?:^|\s)(self|'self'|"[^"]+")(?=\s|$)/giu)`
/// returns the trimmed entry strings.
fn allow_list_entries(inner: &str) -> Vec<String> {
    let chars: Vec<char> = inner.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i <= chars.len() {
        let anchored = i == 0 || is_ws(chars[i - 1]);
        if !anchored {
            i += 1;
            continue;
        }
        let ci = |from: usize, word: &str| -> bool {
            from + word.len() <= chars.len()
                && chars[from..from + word.len()]
                    .iter()
                    .zip(word.chars())
                    .all(|(a, b)| a.eq_ignore_ascii_case(&b))
        };
        let end = if ci(i, "self") {
            Some(i + 4)
        } else if ci(i, "'self'") {
            Some(i + 6)
        } else if i < chars.len() && chars[i] == '"' {
            let mut k = i + 1;
            while k < chars.len() && chars[k] != '"' {
                k += 1;
            }
            if k < chars.len() && k > i + 1 {
                Some(k + 1)
            } else {
                None
            }
        } else {
            None
        };
        match end {
            Some(e) if e == chars.len() || is_ws(chars[e]) => {
                out.push(chars[i..e].iter().collect());
                i = e;
            }
            _ => i += 1,
        }
    }
    out
}

fn is_self_entry(entry: &str) -> bool {
    entry.eq_ignore_ascii_case("self") || entry.eq_ignore_ascii_case("'self'")
}

/// `new URL(value).origin`, enough of WHATWG for scheme://authority URLs.
fn url_origin(value: &str) -> Result<String, String> {
    let s =
        value.trim_matches(|c: char| matches!(c, '\u{9}' | '\u{a}' | '\u{c}' | '\u{d}' | '\u{20}'));
    let lower = s.to_lowercase();
    for (scheme, default_port) in [
        ("https://", 443),
        ("http://", 80),
        ("wss://", 443),
        ("ws://", 80),
        ("ftp://", 21),
    ] {
        if let Some(rest) = lower.strip_prefix(scheme) {
            let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            let authority = &rest[..authority_end];
            let host_port = authority.rsplit('@').next().unwrap_or(authority);
            let (host, port) = match host_port.rfind(':') {
                Some(colon) if host_port[colon + 1..].chars().all(|c| c.is_ascii_digit()) => {
                    (&host_port[..colon], Some(&host_port[colon + 1..]))
                }
                _ => (host_port, None),
            };
            if host.is_empty() {
                return Err("invalid URL".into());
            }
            let port_suffix = match port.and_then(|p| p.parse::<u32>().ok()) {
                Some(p) if p != default_port => format!(":{p}"),
                _ => String::new(),
            };
            return Ok(format!("{scheme}{host}{port_suffix}"));
        }
    }
    Err("invalid URL".into())
}

/// Port of `isWebMcpCompatibilityAllowedByHeaders` (`apps/desktop/src/browserSessionPolicy.ts:35`).
pub fn is_web_mcp_compatibility_allowed_by_headers(
    url: String,
    headers: Option<serde_json::Value>,
) -> Result<bool, String> {
    let mut values: Vec<String> = Vec::new();
    for (name, value) in entries_of(&headers.unwrap_or_else(|| Value::Object(Default::default()))) {
        if name.to_lowercase() != "permissions-policy" {
            continue;
        }
        match &value {
            Value::Array(items) => values.extend(items.iter().map(join_elem)),
            v if js_truthy(v) => values.push(js_to_string(v)),
            _ => {}
        }
    }
    if values.is_empty() {
        return Ok(true);
    }
    let policy = values.join(",");
    let directives = tools_directives(&policy);
    if !has_tools_directive(&policy) {
        return Ok(true);
    }
    if directives.len() != 1 {
        return Ok(false);
    }
    let allow_list = &directives[0];
    if allow_list == "*" {
        return Ok(true);
    }
    if !(allow_list.starts_with('(') && allow_list.ends_with(')')) {
        return Ok(false);
    }
    let entries = allow_list_entries(&allow_list[1..allow_list.len() - 1]);
    if entries.iter().any(|e| is_self_entry(e)) {
        return Ok(true);
    }
    let page_origin = match url_origin(&url) {
        Ok(o) => o,
        Err(_) => return Ok(false),
    };
    Ok(entries.iter().any(|entry| {
        entry
            .strip_prefix('"')
            .and_then(|e| e.strip_suffix('"'))
            .and_then(|inner| url_origin(inner).ok())
            .is_some_and(|origin| origin == page_origin)
    }))
}
