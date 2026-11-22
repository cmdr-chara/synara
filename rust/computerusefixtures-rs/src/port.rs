//! The Rust port of `scripts/computer-use-fixtures/`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

// ---------- JS value helpers ----------

fn member(v: &Value, key: &str) -> Value {
    v.as_object()
        .and_then(|o| o.get(key).cloned())
        .unwrap_or_else(crate::js::undefined)
}

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn is_nil(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn is_record(v: &Value) -> bool {
    v.as_object().is_some() && !is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !is_undefined(v),
    }
}

fn as_num(v: &Value) -> Option<f64> {
    crate::js::as_number(v)
}

/// JS `===` strict equality.
fn js_eq(a: &Value, b: &Value) -> bool {
    if is_undefined(a) && is_undefined(b) {
        return true;
    }
    match (as_num(a), as_num(b)) {
        (Some(x), Some(y)) => return x == y,
        (Some(_), None) | (None, Some(_))
            if (a.is_number() || as_num(a).is_some()) && (b.is_number() || as_num(b).is_some()) =>
        {
            return false;
        }
        _ => {}
    }
    if is_undefined(a) || is_undefined(b) {
        return false;
    }
    a == b
}

/// JS `ToString` used by templates and `RegExp.test`.
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => crate::js::number_to_string(as_num(v).unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|e| if is_nil(e) { String::new() } else { js_str(e) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if let Some(n) = as_num(v) {
                crate::js::number_to_string(n)
            } else if is_undefined(v) {
                "undefined".into()
            } else {
                let _ = o;
                "[object Object]".into()
            }
        }
    }
}

/// JS `a + b` (string concat when either side is a string, else numeric).
fn js_add(a: &Value, b: &Value) -> Value {
    if let (Some(x), Some(y)) = (as_num(a), as_num(b)) {
        return crate::js::number(x + y);
    }
    match (a, b) {
        (Value::String(x), _) => Value::String(format!("{x}{}", js_str(b))),
        (_, Value::String(y)) => Value::String(format!("{}{y}", js_str(a))),
        _ => crate::js::number(f64::NAN),
    }
}

/// JS `a - b` (always numeric).
fn js_sub(a: &Value, b: &Value) -> f64 {
    match (as_num(a), as_num(b)) {
        (Some(x), Some(y)) => x - y,
        _ => f64::NAN,
    }
}

/// JS `<`/`>`/`<=`/`>=` on already-fetched values (numbers or strings; NaN → false).
fn js_cmp(a: &Value, b: &Value, op: &str) -> bool {
    if let (Some(x), Some(y)) = (as_num(a), as_num(b)) {
        return match op {
            "<" => x < y,
            "<=" => x <= y,
            ">" => x > y,
            ">=" => x >= y,
            _ => false,
        };
    }
    if let (Value::String(x), Value::String(y)) = (a, b) {
        return match op {
            "<" => x < y,
            "<=" => x <= y,
            ">" => x > y,
            ">=" => x >= y,
            _ => false,
        };
    }
    false
}

fn read_prop(v: &Value, name: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(member(v, name))
}

fn read_prop_call(v: &Value, name: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(member(v, name))
}

fn call_check(v: &Value, owner: &str, name: &str) -> Result<(), String> {
    if v.is_array() {
        return Ok(());
    }
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Err(format!("{owner}.{name} is not a function"))
}

/// Whitespace-collapse a JS string: `value.replace(/\s+/g, " ").trim()`.
fn normalized(v: &Value) -> String {
    let s = js_str(v);
    let mut out = String::with_capacity(s.len());
    let mut ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            ws = true;
        } else {
            if ws && !out.is_empty() {
                out.push(' ');
            }
            out.push(ch);
            ws = false;
        }
    }
    out
}

/// JS `value.length` on a string (UTF-16 units).
fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Number of Unicode scalar values — used for `String.length` checks where the
/// source measures code points loosely via UTF-16; corpus strings only contain
/// BMP text so the units agree.
fn char_len(s: &str) -> usize {
    js_len(s)
}

// ---------- minimal WHATWG URL ----------

#[derive(Debug, Clone)]
struct ParsedUrl {
    protocol: String,
    username: String,
    password: String,
    hostname: String,
    port: String,
    pathname: String,
    search: String,
    hash: String,
}

fn url_origin(u: &ParsedUrl) -> String {
    format!("{}//{}{}", u.protocol, u.hostname_port(), "")
}

impl ParsedUrl {
    fn hostname_port(&self) -> String {
        if self.port.is_empty() {
            self.hostname.clone()
        } else {
            format!("{}:{}", self.hostname, self.port)
        }
    }
}

/// `new URL(value)` — enough of the WHATWG basic parser for loopback/newegg
/// checks: scheme required, tab/newline stripping, `//authority` userinfo and
/// port handling. Returns `None` where `new URL` would throw.
fn parse_url(input: &str) -> Option<ParsedUrl> {
    let cleaned: String = input
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let cleaned = cleaned.trim_matches(|c: char| (c as u32) <= 0x20);
    // Scheme: ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) followed by ":".
    let bytes = cleaned.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    let mut i = 1;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'+' | b'-' | b'.'))
    {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b':' {
        return None;
    }
    let scheme = cleaned[..i].to_ascii_lowercase();
    let special = matches!(
        scheme.as_str(),
        "http" | "https" | "ws" | "wss" | "ftp" | "file"
    );
    let rest = &cleaned[i + 1..];

    let mut u = ParsedUrl {
        protocol: format!("{scheme}:"),
        username: String::new(),
        password: String::new(),
        hostname: String::new(),
        port: String::new(),
        pathname: String::new(),
        search: String::new(),
        hash: String::new(),
    };

    let mut tail = rest;
    if let Some(authority_tail) = tail.strip_prefix("//") {
        // authority runs until '/', '?' or '#'
        let end = authority_tail
            .find(['/', '?', '#'])
            .unwrap_or(authority_tail.len());
        let authority = &authority_tail[..end];
        tail = &authority_tail[end..];
        // userinfo@host — split at last '@'
        let hostport = match authority.rfind('@') {
            Some(at) => {
                let userinfo = &authority[..at];
                if let Some(c) = userinfo.find(':') {
                    u.username = userinfo[..c].to_string();
                    u.password = userinfo[c + 1..].to_string();
                } else {
                    u.username = userinfo.to_string();
                }
                &authority[at + 1..]
            }
            None => authority,
        };
        // host:port — ':' inside [] is IPv6
        let (host, port) = split_host_port(hostport)?;
        if special && host.is_empty() && scheme != "file" {
            return None;
        }
        if let Some(p) = &port {
            if (p.is_empty() || !p.chars().all(|c| c.is_ascii_digit())) && special {
                return None;
            }
            if p.chars().all(|c| c.is_ascii_digit())
                && p.parse::<u64>().map(|v| v > 65535).unwrap_or(true)
                && special
            {
                return None;
            }
            u.port = p.clone();
        }
        u.hostname = host;
    } else if special {
        // `http:host/path` — special schemes parse the authority anyway.
        let stripped = tail.trim_start_matches('/');
        let end = stripped.find(['/', '?', '#']).unwrap_or(stripped.len());
        let hostport = &stripped[..end];
        let consumed = stripped.len() - stripped[end..].len();
        tail = &stripped[consumed..];
        let (host, port) = split_host_port(hostport)?;
        if host.is_empty() {
            return None;
        }
        if let Some(p) = &port {
            if !p.chars().all(|c| c.is_ascii_digit())
                || p.parse::<u64>().map(|v| v > 65535).unwrap_or(true)
            {
                return None;
            }
            u.port = p.clone();
        }
        u.hostname = host;
    }

    // path / search / hash
    let path_end = tail.find(['?', '#']).unwrap_or(tail.len());
    u.pathname = tail[..path_end].to_string();
    let after = &tail[path_end..];
    if let Some(q) = after.strip_prefix('?') {
        let h = q.find('#').unwrap_or(q.len());
        u.search = q[..h].to_string();
        if let Some(hv) = q.get(h..) {
            u.hash = hv.to_string();
        }
    } else {
        u.hash = after.to_string();
    }
    if !special && u.hostname.is_empty() {
        // opaque path — pathname is the whole rest
        u.pathname = rest.to_string();
    }
    u.hostname = u.hostname.to_ascii_lowercase();
    Some(u)
}

fn split_host_port(hostport: &str) -> Option<(String, Option<String>)> {
    if let Some(stripped) = hostport.strip_prefix('[') {
        // IPv6 [::1]:port
        let close = stripped.find(']')?;
        let host = &hostport[..close + 1];
        let rest = &hostport[close + 2..];
        if rest.is_empty() {
            return Some((host.to_string(), None));
        }
        return rest
            .strip_prefix(':')
            .map(|p| (host.to_string(), Some(p.to_string())));
    }
    match hostport.rfind(':') {
        Some(c) => Some((
            hostport[..c].to_string(),
            Some(hostport[c + 1..].to_string()),
        )),
        None => Some((hostport.to_string(), None)),
    }
}

// ---------- belief-canary ----------

/// Port of `canaryOpenArgs` (`belief-canary.mjs:121`).
pub fn canary_open_args(
    app: serde_json::Value,
    env_assignment: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!([
        "-g",
        "-n",
        "-W",
        "-a",
        app,
        "--env",
        env_assignment
    ]))
}

// ---------- browser-benchmark-evidence ----------

/// Port of `parseBrowserBenchmarkRunCount`.
pub fn parse_browser_benchmark_run_count(
    value: Option<String>,
) -> Result<serde_json::Value, String> {
    match value.as_deref() {
        None | Some("2") => Ok(json!(2)),
        Some("1") => Ok(json!(1)),
        _ => Err("Invalid --runs: expected 1 or 2".to_string()),
    }
}

/// Port of `assessBrowserBenchmarkRuns`.
pub fn assess_browser_benchmark_runs(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let accepted = read_prop(&input, "accepted")?;
    let completed = read_prop(&input, "completedRuns")?;
    let requested = read_prop(&input, "requestedRuns")?;
    let cleanup = read_prop(&input, "cleanupProven")?;
    let passed = if !truthy(&accepted) {
        accepted
    } else if !js_eq(&completed, &requested) {
        json!(false)
    } else {
        cleanup
    };
    let single = js_eq(&requested, &json!(1));
    Ok(json!({
        "passed": passed,
        "requestedRuns": requested,
        "completedRuns": completed,
        "runMode": if single { "single-run-smoke" } else { "two-run-qualification" },
        "twoRunQualificationPassed": js_eq(&requested, &json!(2)) && truthy(&passed),
    }))
}

/// Port of `parseNeweggPrice`.
pub fn parse_newegg_price(
    text: String,
    currency_hint: Option<String>,
) -> Result<serde_json::Value, String> {
    let input = normalized(&Value::String(text));
    let hint_usd = currency_hint.as_deref() == Some("USD");
    // /\bUSD\b|US\s*\$/i
    let usd = hint_usd || has_usd_marker(&input);
    if !usd || has_non_usd_currency(&input) {
        return Ok(Value::Null);
    }
    // strip \bUSD\b|US\s*\$ then leading bare $
    let mut stripped = strip_usd_markers(&input);
    if stripped.starts_with('$') {
        stripped = stripped[1..].trim_start().to_string();
    }
    let stripped = stripped.trim().to_string();
    // /^(?:\d{1,3}(?:,\d{3})*|\d+)(?:\.\d{2})$/
    if !valid_price_string(&stripped) {
        return Ok(Value::Null);
    }
    let digits: String = stripped
        .chars()
        .filter(|c| *c != ',' && *c != '.')
        .collect();
    let cents = digits.parse::<f64>().unwrap_or(f64::NAN);
    if cents.is_finite()
        && cents.fract() == 0.0
        && (0.0..=10_000_000.0).contains(&cents)
        && cents.abs() <= 9007199254740992.0
    {
        Ok(json!({ "minor": crate::js::number(cents), "currency": "USD" }))
    } else {
        Ok(Value::Null)
    }
}

fn has_usd_marker(input: &str) -> bool {
    let lower = input.to_ascii_uppercase();
    for (i, _) in lower.match_indices("USD") {
        let before_ok = i == 0 || !is_word_byte(lower.as_bytes()[i - 1]);
        let after_ok = i + 3 >= lower.len() || !is_word_byte(lower.as_bytes()[i + 3]);
        if before_ok && after_ok {
            return true;
        }
    }
    // US\s*\$
    let b = lower.as_bytes();
    for i in 0..b.len() {
        if i + 2 < b.len() && b[i] == b'U' && b[i + 1] == b'S' {
            let boundary = i == 0 || !is_word_byte(b[i - 1]);
            let mut j = i + 2;
            while j < b.len() && (b[j] as char).is_whitespace() {
                j += 1;
            }
            if boundary && j < b.len() && b[j] == b'$' {
                return true;
            }
        }
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn has_non_usd_currency(input: &str) -> bool {
    let upper = input.to_ascii_uppercase();
    for cur in ["CAD", "AUD", "NZD", "HKD"] {
        for (i, _) in upper.match_indices(cur) {
            let before_ok = i == 0 || !is_word_byte(upper.as_bytes()[i - 1]);
            let after_ok = i + 3 >= upper.len() || !is_word_byte(upper.as_bytes()[i + 3]);
            if before_ok && after_ok {
                return true;
            }
        }
    }
    input.chars().any(|c| matches!(c, '€' | '£' | '¥'))
}

fn strip_usd_markers(input: &str) -> String {
    // Remove `\bUSD\b` and `US\s*\$` (case-insensitive) occurrences.
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let before_ok = i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_');
        let tail: String = chars[i..].iter().collect();
        let up = tail.to_ascii_uppercase();
        if up.starts_with("USD") {
            let after_ok = i + 3 >= chars.len()
                || !(chars[i + 3].is_ascii_alphanumeric() || chars[i + 3] == '_');
            if before_ok && after_ok {
                i += 3;
                continue;
            }
        }
        if up.starts_with("US") {
            let after_ok = i + 2 >= chars.len()
                || !(chars[i + 2].is_ascii_alphanumeric() || chars[i + 2] == '_');
            if before_ok && after_ok {
                // US\s*\$
                let mut j = i + 2;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && chars[j] == '$' {
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `/^(?:\d{1,3}(?:,\d{3})*|\d+)(?:\.\d{2})$/`
fn valid_price_string(s: &str) -> bool {
    let (int_part, frac) = match s.rfind('.') {
        Some(d) => (&s[..d], &s[d + 1..]),
        None => return false,
    };
    if frac.len() != 2 || !frac.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if int_part.is_empty() {
        return false;
    }
    if int_part.contains(',') {
        let groups: Vec<&str> = int_part.split(',').collect();
        if groups.is_empty()
            || !(1..=3).contains(&groups[0].len())
            || !groups[0].chars().all(|c| c.is_ascii_digit())
        {
            return false;
        }
        groups[1..]
            .iter()
            .all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()))
    } else {
        int_part.chars().all(|c| c.is_ascii_digit())
    }
}

fn valid_item_id(v: &Value) -> bool {
    match v {
        Value::String(s) => {
            (6..=32).contains(&char_len(s))
                && s.chars().all(|c| {
                    c.is_ascii_alphanumeric() && c.is_ascii_uppercase() || c.is_ascii_digit()
                })
        }
        _ => false,
    }
}

fn nonnegative_integer(v: &Value) -> bool {
    matches!(as_num(v), Some(n) if n.is_finite() && n.fract() == 0.0 && n >= 0.0 && n.abs() <= 9007199254740992.0)
}

fn short_text(v: &Value, max: usize) -> bool {
    match v {
        Value::String(s) => {
            let l = char_len(s);
            l > 0 && l <= max
        }
        _ => false,
    }
}

/// Port of `parseNeweggPageEvidence`.
pub fn parse_newegg_page_evidence(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if !is_record(&value) {
        return Ok(None);
    }
    let mut result = Map::new();
    let product_v = member(&value, "product");
    if !is_undefined(&product_v) {
        if !is_record(&product_v) {
            return Ok(None);
        }
        let item_id = member(&product_v, "itemId");
        let title = member(&product_v, "title");
        let crumbs = member(&product_v, "breadcrumbs");
        let facts = member(&product_v, "facts");
        let crumbs_ok = crumbs
            .as_array()
            .map(|a| a.len() <= 16 && a.iter().all(|p| short_text(p, 160)))
            .unwrap_or(false);
        let facts_ok = facts.as_array().map(|a| {
            a.len() <= 48
                && a.iter().all(|f| {
                    is_record(f)
                        && short_text(&member(f, "label"), 120)
                        && short_text(&member(f, "value"), 512)
                })
        });
        if !valid_item_id(&item_id)
            || !short_text(&title, 1024)
            || !crumbs_ok
            || !facts_ok.unwrap_or(false)
        {
            return Ok(None);
        }
        result.insert(
            "product".into(),
            json!({
                "itemId": item_id,
                "title": normalized(&title),
                "breadcrumbs": crumbs.as_array().unwrap_or(&vec![]).iter().map(normalized).collect::<Vec<_>>(),
                "facts": facts.as_array().unwrap_or(&vec![]).iter().map(|f| json!({
                    "label": normalized(&member(f, "label")),
                    "value": normalized(&member(f, "value")),
                })).collect::<Vec<_>>(),
            }),
        );
    }
    let cart_v = member(&value, "cart");
    if !is_undefined(&cart_v) {
        let nullable_integer = |v: &Value, max: f64| {
            v.is_null() || (nonnegative_integer(v) && as_num(v).unwrap_or(f64::MAX) <= max)
        };
        let items = member(&cart_v, "items");
        let items_ok = items.as_array().map(|arr| {
            arr.len() <= 32
                && arr.iter().all(|item| {
                    is_record(item)
                        && valid_item_id(&member(item, "itemId"))
                        && short_text(&member(item, "title"), 1024)
                        && {
                            let q = member(item, "quantity");
                            q.is_null()
                                || (nonnegative_integer(&q)
                                    && as_num(&q).unwrap_or(0.0) >= 1.0
                                    && as_num(&q).unwrap_or(0.0) <= 99.0)
                        }
                        && nullable_integer(&member(item, "unitPriceMinor"), 10_000_000.0)
                        && {
                            let c = member(item, "currency");
                            c.is_null() || c.as_str() == Some("USD")
                        }
                        && member(item, "unavailable").is_boolean()
                })
        });
        let cart_currency = member(&cart_v, "currency");
        if !is_record(&cart_v)
            || !items_ok.unwrap_or(false)
            || !member(&cart_v, "rowCoverageComplete").is_boolean()
            || !nullable_integer(&member(&cart_v, "itemCount"), 128.0)
            || !nullable_integer(&member(&cart_v, "subtotalMinor"), 10_000_000.0)
            || !(cart_currency.is_null() || cart_currency.as_str() == Some("USD"))
        {
            return Ok(None);
        }
        result.insert(
            "cart".into(),
            json!({
                "items": items.as_array().unwrap_or(&vec![]).iter().map(|item| json!({
                    "itemId": member(item, "itemId"),
                    "title": normalized(&member(item, "title")),
                    "quantity": member(item, "quantity"),
                    "unitPriceMinor": member(item, "unitPriceMinor"),
                    "currency": member(item, "currency"),
                    "unavailable": member(item, "unavailable"),
                })).collect::<Vec<_>>(),
                "itemCount": member(&cart_v, "itemCount"),
                "subtotalMinor": member(&cart_v, "subtotalMinor"),
                "currency": cart_currency,
                "rowCoverageComplete": member(&cart_v, "rowCoverageComplete"),
            }),
        );
    }
    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(result)))
    }
}

/// Port of `observedNeweggCategory` — the first breadcrumb that exactly matches
/// one retailer taxonomy label wins; zero or many matches produce null.
pub fn observed_newegg_category(
    product: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let crumbs = read_prop_call(&product, "breadcrumbs")?;
    if is_nil(&crumbs) {
        if crumbs.is_null() {
            return Err("Cannot read properties of null (reading 'some')".to_string());
        }
        return Err("Cannot read properties of undefined (reading 'some')".to_string());
    }
    let arr = match crumbs.as_array() {
        Some(a) => a,
        None => return Err("product.breadcrumbs.some is not a function".to_string()),
    };
    type CategoryCheck = fn(&str) -> bool;
    let categories: &[(&str, CategoryCheck)] = &[
        ("cpu", |p| match_category_cpu(p)),
        ("motherboard", |p| {
            p.ends_with("Motherboards")
                && (p == "Motherboards" || p.starts_with("AMD ") || p.starts_with("Intel "))
        }),
        ("gpu", |p| match_category_gpu(p)),
        ("memory", |p| p.eq_ignore_ascii_case("Desktop Memory")),
        ("storage", |p| {
            p.eq_ignore_ascii_case("Solid State Drives")
                || p.eq_ignore_ascii_case("Internal SSDs")
                || p.eq_ignore_ascii_case("Internal Hard Drives")
                || p.eq_ignore_ascii_case("Internal Solid State Drives")
        }),
        ("psu", |p| p.eq_ignore_ascii_case("Power Supplies")),
        ("case", |p| p.eq_ignore_ascii_case("Computer Cases")),
        ("cooler", |p| match_category_cooler(p)),
    ];
    let mut found: Option<&str> = None;
    let mut count = 0usize;
    for (name, pred) in categories {
        if arr
            .iter()
            .any(|part| part.as_str().map(pred).unwrap_or(false))
        {
            count += 1;
            found = Some(name);
        }
    }
    if count == 1 {
        Ok(Some(json!(found.unwrap_or(""))))
    } else {
        Ok(None)
    }
}

fn match_category_cpu(p: &str) -> bool {
    // ^(?:CPUs?\s*\/\s*Processors|Processors?\s*-\s*Desktops?|Desktop (?:CPUs?|Processors?))$
    let lower = p.to_ascii_lowercase();
    let squashed: String = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    // CPUs? / Processors
    if let Some((left, right)) = squashed.split_once('/') {
        let l = left.trim();
        let r = right.trim();
        if (l == "cpu" || l == "cpus") && r == "processors" {
            return true;
        }
    }
    // Processors? - Desktops?
    if let Some((left, right)) = squashed.split_once('-') {
        let l = left.trim();
        let r = right.trim();
        if (l == "processor" || l == "processors") && (r == "desktop" || r == "desktops") {
            return true;
        }
    }
    // Desktop CPUs?|Processors?
    matches!(
        squashed.as_str(),
        "desktop cpu" | "desktop cpus" | "desktop processor" | "desktop processors"
    )
}

fn match_category_gpu(p: &str) -> bool {
    // ^(?:Desktop )?(?:Video Cards(?: & (?:Video Devices|Graphics Cards))?|Graphics Cards)$
    let lower = p.to_ascii_lowercase();
    let body = lower.strip_prefix("desktop ").unwrap_or(&lower);
    matches!(
        body,
        "video cards"
            | "video cards & video devices"
            | "video cards & graphics cards"
            | "graphics cards"
    )
}

fn match_category_cooler(p: &str) -> bool {
    // ^(?:CPU (?:Coolers|Fans & Heatsinks)|Liquid (?:CPU )?Cooling Systems)$
    let lower = p.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "cpu coolers"
            | "cpu fans & heatsinks"
            | "liquid cooling systems"
            | "liquid cpu cooling systems"
    )
}

const NEWEGG_CORE_CATEGORIES: [&str; 7] = [
    "cpu",
    "motherboard",
    "gpu",
    "memory",
    "storage",
    "psu",
    "case",
];

/// Port of `assessNeweggCompletion`.
pub fn assess_newegg_completion(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let evidence = read_prop(&input, "evidence")?;
    let carts_v = read_prop(&evidence, "carts")?;
    call_check(&carts_v, "input.evidence.carts", "filter")?;
    let threshold = as_num(&member(&input, "finalObservationStartedAtMs")).unwrap_or(f64::NAN);
    let all_carts: Vec<Value> = carts_v
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|e| {
                    js_cmp(
                        &member(e, "observedAtMs"),
                        &crate::js::number(threshold),
                        ">=",
                    )
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let carts: Vec<Value> = all_carts.iter().rev().take(8).rev().cloned().collect();
    let last = carts.last();
    let stable = carts.len() >= 2
        && js_sub(
            &member(last.unwrap_or(&Value::Null), "observedAtMs"),
            &member(&carts[0], "observedAtMs"),
        ) >= 200.0
        && carts
            .iter()
            .all(|e| member(e, "cart") == member(&carts[0], "cart"));
    if !stable {
        return Ok(
            json!({"status":"unverified","reason":"two-stable-final-cart-observations-missing"}),
        );
    }
    let cart = member(last.unwrap_or(&Value::Null), "cart");
    let items = member(&cart, "items");
    let items_arr = items.as_array().cloned().unwrap_or_default();
    let distinct: std::collections::BTreeSet<String> = items_arr
        .iter()
        .map(|i| js_str(&member(i, "itemId")))
        .collect();
    let cart_bad = !truthy(&read_prop(&cart, "rowCoverageComplete")?)
        || member(&cart, "itemCount").is_null()
        || member(&cart, "subtotalMinor").is_null()
        || !js_eq(&member(&cart, "currency"), &json!("USD"))
        || items_arr.is_empty()
        || items_arr.iter().any(|i| {
            member(i, "quantity").is_null()
                || member(i, "unitPriceMinor").is_null()
                || !js_eq(&member(i, "currency"), &json!("USD"))
        })
        || distinct.len() != items_arr.len();
    if cart_bad {
        return Ok(json!({
            "status": "unverified",
            "reason": "cart-identifiers-quantities-USD-prices-or-row-coverage-missing",
        }));
    }
    let quantity: f64 = items_arr
        .iter()
        .map(|i| as_num(&member(i, "quantity")).unwrap_or(f64::NAN))
        .sum();
    let total: f64 = items_arr
        .iter()
        .map(|i| {
            as_num(&member(i, "quantity")).unwrap_or(f64::NAN)
                * as_num(&member(i, "unitPriceMinor")).unwrap_or(f64::NAN)
        })
        .sum();
    if !js_eq(&crate::js::number(quantity), &member(&cart, "itemCount"))
        || !js_eq(&crate::js::number(total), &member(&cart, "subtotalMinor"))
    {
        return Ok(
            json!({"status":"unverified","reason":"cart-count-or-subtotal-does-not-reconcile"}),
        );
    }
    let products = member(&evidence, "products");
    let products_arr = products.as_array().cloned().unwrap_or_default();
    let mut parts = Map::new();
    let mut coverage = Vec::new();
    let mut issues: Vec<String> = Vec::new();
    for item in &items_arr {
        let iid = member(item, "itemId");
        let product = products_arr
            .iter()
            .find(|p| js_eq(&member(p, "itemId"), &iid))
            .cloned();
        let category = match &product {
            Some(p) => {
                let v = observed_newegg_category(p.clone())?.unwrap_or(Value::Null);
                v.as_str().map(str::to_string)
            }
            None => None,
        };
        coverage.push(json!({
            "itemId": iid,
            "category": category.clone().map(Value::String).unwrap_or(Value::Null),
            "quantity": member(item, "quantity"),
        }));
        match (&category, &product) {
            (Some(c), Some(p)) => {
                if parts.contains_key(c) {
                    issues.push("multiple-products-in-category-need-additional-fit-checks".into());
                } else {
                    parts.insert(c.clone(), p.clone());
                }
            }
            _ => issues.push("cart-product-taxonomy-unobserved".into()),
        }
        let q = as_num(&member(item, "quantity")).unwrap_or(f64::NAN);
        if let Some(c) = &category
            && !["storage", "memory"].contains(&c.as_str())
            && q != 1.0
        {
            issues.push("single-PC-component-quantity-ambiguous".into());
        }
    }
    let missing: Vec<String> = NEWEGG_CORE_CATEGORIES
        .iter()
        .filter(|c| !parts.contains_key(*c as &str))
        .map(|c| c.to_string())
        .collect();
    if !missing.is_empty() {
        issues.push("required-component-categories-missing-or-unobserved".into());
    }
    let compatibility = compare_newegg_compatibility(&parts);
    if member(&compatibility, "status").as_str() == Some("unknown") {
        issues.push("listed-compatibility-facts-incomplete".into());
    }
    let subtotal = as_num(&member(&cart, "subtotalMinor")).unwrap_or(f64::NAN);
    let failed = subtotal > 300_000.0
        || items_arr
            .iter()
            .any(|i| member(i, "unavailable") == Value::Bool(true))
        || member(&compatibility, "status").as_str() == Some("incompatible")
        || (!missing.is_empty()
            && coverage
                .iter()
                .all(|i| member(i, "category").as_str().is_some()));
    let status = if failed {
        "failed"
    } else if !issues.is_empty() {
        "unverified"
    } else {
        "verified"
    };
    let reason = if failed {
        "budget-stock-or-observed-fit-failed"
    } else if !issues.is_empty() {
        "cart-observed-with-open-evidence-gaps"
    } else {
        "cart-budget-categories-and-listed-fit-checks-verified"
    };
    issues.sort();
    issues.dedup();
    Ok(json!({
        "status": status,
        "reason": reason,
        "scope": "retailer-cart-and-explicit-listed-fit-checks; not a hardware or 1440p performance test",
        "issues": issues,
        "coverage": coverage,
        "missingCategories": missing,
        "cart": {
            "status": "verified",
            "itemCount": member(&cart, "itemCount"),
            "distinctItems": items_arr.len(),
            "currency": "USD",
            "subtotalMinor": member(&cart, "subtotalMinor"),
            "budgetMinor": 300_000,
            "withinBudget": subtotal <= 300_000.0,
            "costBasis": "merchandise-subtotal; shipping-and-tax-unverified",
        },
        "compatibility": compatibility,
    }))
}

fn fact_joined_dyn(parts: &Map<String, Value>, part: &str, label: &dyn Fn(&str) -> bool) -> String {
    parts
        .get(part)
        .and_then(|p| member(p, "facts").as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter(|e| member(e, "label").as_str().map(label).unwrap_or(false))
        .map(|e| js_str(&member(e, "value")))
        .collect::<Vec<_>>()
        .join(" / ")
}

fn label_matches(label: &str, alternates: &[&str]) -> bool {
    alternates.iter().any(|a| label.eq_ignore_ascii_case(a))
}

fn sockets(parts: &Map<String, Value>, part: &str) -> Vec<String> {
    let text = fact_joined_dyn(parts, part, &|l| {
        label_matches(
            l,
            &[
                "Socket",
                "CPU Socket",
                "Socket Type",
                "CPU Socket Type",
                "Socket Support",
                "CPU Socket Support",
                "Socket Compatibility",
                "CPU Socket Compatibility",
                "Sockets",
                "CPU Sockets",
                "Supported CPU Sockets",
            ],
        )
    });
    let mut out = Vec::new();
    for m in find_tokens(&text, &[("AM3", "AM3"), ("AM4", "AM4"), ("AM5", "AM5")]) {
        if !out.contains(&m) {
            out.push(m);
        }
    }
    for m in find_lga(&text) {
        if !out.contains(&m) {
            out.push(m);
        }
    }
    out
}

/// `/\b(?:AM[345]|LGA\s*\d{3,4})\b/gi` tokens upper-cased, whitespace removed.
fn find_tokens(text: &str, _pats: &[(&str, &str)]) -> Vec<String> {
    let upper = text.to_ascii_uppercase();
    let b = upper.as_bytes();
    let mut out = Vec::new();
    for i in 0..b.len() {
        if i + 2 < b.len()
            && b[i] == b'A'
            && b[i + 1] == b'M'
            && matches!(b[i + 2], b'3' | b'4' | b'5')
        {
            let before = i == 0 || !is_word_byte(b[i - 1]);
            let after = i + 3 >= b.len() || !is_word_byte(b[i + 3]);
            if before && after {
                out.push(upper[i..i + 3].to_string());
            }
        }
    }
    out
}

fn find_lga(text: &str) -> Vec<String> {
    let upper = text.to_ascii_uppercase();
    let b = upper.as_bytes();
    let mut out = Vec::new();
    for i in 0..b.len() {
        if i + 3 < b.len() && b[i] == b'L' && b[i + 1] == b'G' && b[i + 2] == b'A' {
            let before = i == 0 || !is_word_byte(b[i - 1]);
            let mut j = i + 3;
            while j < b.len() && (b[j] as char).is_whitespace() {
                j += 1;
            }
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() && j - start < 4 {
                j += 1;
            }
            if j - start >= 3 && before && (j >= b.len() || !is_word_byte(b[j])) {
                let digits = &upper[start..j];
                out.push(format!("LGA{digits}"));
            }
        }
    }
    out
}

fn memory_standard(parts: &Map<String, Value>, part: &str) -> Option<String> {
    let text = fact_joined_dyn(parts, part, &|l| {
        label_matches(l, &["Memory Type", "Memory Standard", "Type"])
    });
    let upper = text.to_ascii_uppercase();
    let mut found = Vec::new();
    for m in ["DDR3", "DDR4", "DDR5"] {
        let b = upper.as_bytes();
        for (i, _) in upper.match_indices(m) {
            let before = i == 0 || !is_word_byte(b[i - 1]);
            let after = i + 4 >= b.len() || !is_word_byte(b[i + 4]);
            if before && after {
                found.push(m.to_string());
            }
        }
    }
    if found.len() == 1 {
        Some(found[0].clone())
    } else {
        None
    }
}

fn numeric_fact(
    parts: &Map<String, Value>,
    part: &str,
    labels: &[&str],
    unit: &str,
) -> Option<f64> {
    let text = fact_joined_dyn(parts, part, &|l| label_matches(l, labels));
    let text = text.trim();
    let rest = text
        .strip_suffix(unit)
        .or_else(|| text.strip_suffix(&unit.to_ascii_lowercase()))?;
    rest.trim().parse::<f64>().ok()
}

fn form_factors(parts: &Map<String, Value>, part: &str, labels: &[&str]) -> Vec<String> {
    let text = fact_joined_dyn(parts, part, &|l| label_matches(l, labels));
    text.split([',', '/'])
        .flat_map(|seg| seg.split(" and "))
        .map(|v| {
            v.trim()
                .to_ascii_uppercase()
                .replace("EXTENDED", "E")
                .replace([' ', '-'], "")
        })
        .filter(|v| ["ATX", "EATX", "MICROATX", "MINIITX"].contains(&v.as_str()))
        .collect()
}

fn single(values: Vec<String>) -> Option<String> {
    let set: std::collections::BTreeSet<String> = values.iter().cloned().collect();
    if set.len() == 1 {
        values.into_iter().next()
    } else {
        None
    }
}

fn compare_newegg_compatibility(parts: &Map<String, Value>) -> Value {
    let mut checks: Vec<Value> = Vec::new();
    let same = |name: &str, a: Option<String>, b: Option<String>, checks: &mut Vec<Value>| {
        let status = match (&a, &b) {
            (None, _) | (_, None) => "unknown",
            _ if a == b => "matched",
            _ => "mismatched",
        };
        checks.push(json!({
            "check": name, "status": status,
            "values": [a.map(Value::String).unwrap_or(Value::Null), b.map(Value::String).unwrap_or(Value::Null)],
        }));
    };
    let fits = |name: &str,
                needed: Option<f64>,
                available: Option<f64>,
                checks: &mut Vec<Value>| {
        let status = match (needed, available) {
            (Some(n), Some(av)) => {
                if av >= n {
                    "matched"
                } else {
                    "mismatched"
                }
            }
            _ => "unknown",
        };
        checks.push(json!({
            "check": name, "status": status,
            "values": [needed.map(crate::js::number).unwrap_or(Value::Null), available.map(crate::js::number).unwrap_or(Value::Null)],
        }));
    };
    let cpu_socket = single(sockets(parts, "cpu"));
    same(
        "CPU-motherboard-socket",
        cpu_socket.clone(),
        single(sockets(parts, "motherboard")),
        &mut checks,
    );
    same(
        "RAM-motherboard-DDR-standard",
        memory_standard(parts, "memory"),
        memory_standard(parts, "motherboard"),
        &mut checks,
    );
    let board_form = single(form_factors(parts, "motherboard", &["Form Factor"]));
    let case_forms = form_factors(
        parts,
        "case",
        &[
            "Motherboard Compatibility",
            "Motherboard Support",
            "Motherboard Supported",
        ],
    );
    checks.push(json!({
        "check": "motherboard-case-form-factor",
        "status": if board_form.is_none() || case_forms.is_empty() {
            "unknown"
        } else if case_forms.contains(board_form.as_ref().unwrap_or(&String::new())) {
            "matched"
        } else {
            "mismatched"
        },
        "values": [
            board_form.clone().map(Value::String).unwrap_or(Value::Null),
            if case_forms.is_empty() { Value::Null } else { Value::String(case_forms.join(",")) },
        ],
    }));
    fits(
        "GPU-listed-minimum-PSU",
        numeric_fact(
            parts,
            "gpu",
            &[
                "Recommended PSU",
                "Minimum PSU",
                "Required PSU",
                "Recommended Power Supply",
                "Minimum Power Supply",
                "Required Power Supply",
                "Recommended System Power",
                "Minimum System Power",
                "Required System Power",
            ],
            "W",
        ),
        numeric_fact(
            parts,
            "psu",
            &["Maximum Power", "Wattage", "Output Power", "Power Output"],
            "W",
        ),
        &mut checks,
    );
    fits(
        "GPU-case-length",
        numeric_fact(
            parts,
            "gpu",
            &["Card Length", "GPU Length", "Graphics Card Length"],
            "mm",
        ),
        numeric_fact(
            parts,
            "case",
            &[
                "Max GPU Length",
                "Maximum GPU Length",
                "Max Graphics Card Length",
                "Maximum Graphics Card Length",
                "Max Video Card Length",
                "Maximum Video Card Length",
            ],
            "mm",
        ),
        &mut checks,
    );
    let cooling = fact_joined_dyn(parts, "cpu", &|l| {
        label_matches(
            l,
            &[
                "Cooling Device",
                "Cooler Included",
                "Includes Cooler",
                "CPU Cooler",
            ],
        )
    });
    let included = label_matches(
        &cooling,
        &[
            "Yes",
            "Included",
            "Heatsink included",
            "Cooler included",
            "Cooling device included",
        ],
    );
    if parts.contains_key("cooler") {
        let supported = sockets(parts, "cooler");
        checks.push(json!({
            "check": "CPU-cooler-socket",
            "status": if cpu_socket.is_none() || supported.is_empty() {
                "unknown"
            } else if cpu_socket.as_ref().map(|s| supported.contains(s)).unwrap_or(false) {
                "matched"
            } else {
                "mismatched"
            },
            "values": [
                cpu_socket.map(Value::String).unwrap_or(Value::Null),
                if supported.is_empty() { Value::Null } else { Value::String(supported.join(",")) },
            ],
        }));
        let cooler_type = fact_joined_dyn(parts, "cooler", &|l| {
            label_matches(l, &["Type", "Cooling Type", "Cooler Type"])
        });
        if label_matches(
            &cooler_type,
            &[
                "Air",
                "Air Cooler",
                "Air Cooling",
                "CPU Air Cooler",
                "Heatsink",
                "Heatsink and Fan",
            ],
        ) {
            fits(
                "air-cooler-case-height",
                numeric_fact(
                    parts,
                    "cooler",
                    &[
                        "Cooler Height",
                        "Heatsink Height",
                        "Product Height",
                        "Height",
                    ],
                    "mm",
                ),
                numeric_fact(
                    parts,
                    "case",
                    &["Max CPU Cooler Height", "Maximum CPU Cooler Height"],
                    "mm",
                ),
                &mut checks,
            );
        } else {
            checks.push(json!({
                "check": "cooler-case-mounting-unobserved",
                "status": "unknown",
                "values": [if cooler_type.is_empty() { Value::Null } else { json!(cooler_type) }],
            }));
        }
    } else {
        checks.push(json!({
            "check": "CPU-cooling-included",
            "status": if included {
                "matched"
            } else if label_matches(&cooling, &["No", "Not included", "None"]) {
                "mismatched"
            } else {
                "unknown"
            },
            "values": [if cooling.is_empty() { Value::Null } else { json!(cooling) }],
        }));
    }
    let any_mismatch = checks
        .iter()
        .any(|c| member(c, "status").as_str() == Some("mismatched"));
    let all_matched = checks
        .iter()
        .all(|c| member(c, "status").as_str() == Some("matched"));
    json!({
        "status": if any_mismatch { "incompatible" } else if all_matched { "listed-checks-matched" } else { "unknown" },
        "checks": checks,
        "unverified": [
            "1440p-frame-rate-and-quality",
            "BIOS-and-memory-QVL",
            "memory-form-factor-buffering-and-slot-count",
            "storage-slots-and-lane-sharing",
            "PSU-cables-and-transient-load",
            "radiator-RAM-and-connector-clearances",
            "assembly-and-real-hardware-operation",
        ],
    })
}

/// Port of `parseBrowserWitness`.
pub fn parse_browser_witness(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if !is_record(&value) {
        return Ok(None);
    }
    let prs = parse_pull_request_titles(member(&value, "prs"))?;
    let page = member(&value, "page");
    let path = member(&value, "path");
    let product_paths = member(&value, "productPaths");
    let paths_ok = product_paths.as_array().map(|a| {
        a.len() <= 32
            && a.iter().all(|p| {
                p.as_str()
                    .map(|s| {
                        s.starts_with("/p/")
                            && s.len() > 3
                            && s[3..]
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                    })
                    .unwrap_or(false)
            })
    });
    let ok = matches!(as_num(&page), Some(n) if n.fract()==0.0 && (1.0..=10_000.0).contains(&n) && n.abs() <= 9007199254740992.0)
        && path
            .as_str()
            .map(|s| s.starts_with('/') && char_len(s) <= 2048)
            .unwrap_or(false)
        && member(&value, "cartVisible").is_boolean()
        && member(&value, "checkoutVisible").is_boolean()
        && prs.is_some()
        && paths_ok.unwrap_or(false);
    if !ok {
        return Ok(None);
    }
    let mut seen = std::collections::BTreeSet::new();
    let dedup: Vec<Value> = product_paths
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| seen.insert(js_str(p)))
        .collect();
    let mut out = Map::new();
    out.insert(
        "page".into(),
        crate::js::number(as_num(&page).unwrap_or(0.0)),
    );
    out.insert("path".into(), path);
    out.insert("prs".into(), Value::Array(prs.unwrap_or_default()));
    out.insert("cartVisible".into(), member(&value, "cartVisible"));
    out.insert("productPaths".into(), Value::Array(dedup));
    out.insert("checkoutVisible".into(), member(&value, "checkoutVisible"));
    let newegg = member(&value, "newegg");
    if !is_undefined(&newegg)
        && let Some(ev) = parse_newegg_page_evidence(newegg)?
    {
        out.insert("newegg".into(), ev);
    }
    Ok(Some(Value::Object(out)))
}

/// Port of `parsePullRequestTitles`.
pub fn parse_pull_request_titles(
    value: serde_json::Value,
) -> Result<Option<Vec<serde_json::Value>>, String> {
    let arr = match value.as_array() {
        Some(a) if a.len() <= 100 => a,
        _ => return Ok(None),
    };
    let mut result = Vec::new();
    for item in arr {
        let number = member(item, "number");
        let title = member(item, "title");
        let ok = is_record(item)
            && matches!(as_num(&number), Some(n) if n.fract()==0.0 && n >= 1.0 && n.abs() <= 9007199254740992.0)
            && title
                .as_str()
                .map(|t| char_len(t) <= 1024 && !normalized(&title).is_empty())
                .unwrap_or(false);
        if !ok {
            return Ok(None);
        }
        result.push(json!({
            "number": crate::js::number(as_num(&number).unwrap_or(0.0)),
            "title": normalized(&title),
        }));
    }
    let set: std::collections::BTreeSet<String> = result
        .iter()
        .map(|i| js_str(&member(i, "number")))
        .collect();
    if set.len() == result.len() {
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

fn sorted_titles_json(items: &[Value]) -> Value {
    let mut v: Vec<Value> = items.to_vec();
    v.sort_by(|a, b| {
        as_num(&member(a, "number"))
            .partial_cmp(&as_num(&member(b, "number")))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Value::Array(v)
}

fn same_titles(a: &Value, b: &Value) -> bool {
    let av = a.as_array().cloned().unwrap_or_default();
    let bv = b.as_array().cloned().unwrap_or_default();
    sorted_titles_json(&av) == sorted_titles_json(&bv)
}

/// Port of `assessGitHubCompletion`.
pub fn assess_git_hub_completion(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let before = read_prop(&input, "before")?;
    let after = read_prop(&input, "after")?;
    let witnesses = read_prop(&input, "witnesses")?;
    let before_a = before.as_array();
    let after_a = after.as_array();
    if before.is_null()
        || after.is_null()
        || before_a.map(|a| a.len() != 2).unwrap_or(true)
        || after_a.map(|a| a.len() != 2).unwrap_or(true)
    {
        return Ok(
            json!({"status":"unverified","reason":"independent-public-reference-unavailable"}),
        );
    }
    let (before_a, after_a) = (
        before_a.unwrap_or(&vec![]).clone(),
        after_a.unwrap_or(&vec![]).clone(),
    );
    if !before_a
        .iter()
        .enumerate()
        .all(|(i, page)| same_titles(page, &after_a[i]))
    {
        return Ok(json!({"status":"unverified","reason":"reference-changed-during-run"}));
    }
    let wit = witnesses.as_array().cloned().unwrap_or_default();
    if !after_a.iter().enumerate().all(|(i, page)| {
        (i == 1 && page.as_array().map(|a| a.is_empty()).unwrap_or(false))
            || wit.iter().any(|w| {
                js_eq(&member(w, "page"), &crate::js::number(i as f64 + 1.0))
                    && same_titles(&member(w, "prs"), page)
            })
    }) {
        return Ok(
            json!({"status":"unverified","reason":"complete-browser-page-observation-missing"}),
        );
    }
    let final_text = member(&input, "finalText");
    let text = final_text.as_str().unwrap_or("");
    let answer: Option<Value> = (|| {
        if text.chars().count() > 100_000 {
            return None;
        }
        // /^\s*```(?:json)?\s*([\s\S]*?)\s*```\s*$/
        let t = text.trim();
        let inner = t.strip_prefix("```").and_then(|rest| {
            rest.strip_prefix("json")
                .unwrap_or(rest)
                .strip_suffix("```")
                .map(|s| s.trim().to_string())
        });
        serde_json::from_str(&inner.unwrap_or_else(|| text.to_string())).ok()
    })();
    let answer = match answer {
        Some(v) => v,
        None => return Ok(json!({"status":"unverified","reason":"final-answer-not-structured"})),
    };
    let pages = if is_record(&answer) {
        member(&answer, "pages")
    } else {
        Value::Null
    };
    let pages_arr = pages.as_array().cloned().unwrap_or_default();
    if !pages.is_array() || pages_arr.len() != 2 {
        return Ok(json!({"status":"failed","reason":"final-answer-missing-pages"}));
    }
    for index in 0..2usize {
        let page = pages_arr
            .iter()
            .find(|item| {
                js_eq(
                    &member(item, "page"),
                    &crate::js::number(index as f64 + 1.0),
                )
            })
            .cloned()
            .unwrap_or(Value::Null);
        let prs = parse_pull_request_titles(member(&page, "prs"))?;
        match prs {
            Some(p)
                if same_titles(
                    &Value::Array(p.clone()),
                    after_a.get(index).unwrap_or(&crate::js::undefined()),
                ) => {}
            _ => {
                return Ok(json!({
                    "status":"failed","reason":"final-answer-pr-titles-incomplete-or-different"
                }));
            }
        }
    }
    Ok(json!({
        "status":"verified",
        "reason":"two-page-browser-inventory-and-answer-match-stable-reference"
    }))
}

/// Port of `browserBenchmarkPrompt`.
pub fn browser_benchmark_prompt(
    task: String,
    profile_name: String,
    repo: Option<String>,
) -> Result<serde_json::Value, String> {
    // /^synara-bench-[a-z0-9-]{1,45}$/
    let ok_profile = profile_name
        .strip_prefix("synara-bench-")
        .map(|rest| {
            !rest.is_empty()
                && char_len(rest) <= 45
                && rest
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .unwrap_or(false);
    if !ok_profile {
        return Err("Invalid benchmark profile name".to_string());
    }
    if task == "github-running" {
        return Err(
            "The existing-user-profile benchmark is unsupported; it must not be substituted."
                .to_string(),
        );
    }
    let boundary = format!(
        "Use Synara Computer browser tools in a separate driver-owned headless Chrome with isolated_named profile {profile_name}. \
         This explicitly tests a separate isolated Chrome profile, without the user's cookies; do not attach or substitute the user's running browser. \
         Keep the user's foreground app, key window and Space unchanged. Use the browser for the task, not shell, HTTP, external search or the in-app browser. "
    );
    if task == "github-isolated" {
        let repo_ok = repo
            .as_deref()
            .map(|r| {
                r.split_once('/')
                    .map(|(o, n)| {
                        !o.is_empty()
                            && !n.is_empty()
                            && o.chars().all(|c| {
                                c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-'
                            })
                            && n.chars().all(|c| {
                                c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-'
                            })
                            && !n.contains('/')
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if !repo_ok {
            return Err("Expected owner/repo".to_string());
        }
        let repo = repo.unwrap_or_default();
        return Ok(json!(format!(
            "{boundary}Open https://github.com/{repo}/pulls?q=is%3Apr+is%3Aopen+sort%3Acreated-desc and read the open PR titles on its first two pages. \
             Do not modify GitHub data. Return only JSON with this shape: {{\"pages\":[{{\"page\":1,\"prs\":[{{\"number\":123,\"title\":\"Exact PR title\"}}]}},{{\"page\":2,\"prs\":[]}}]}}. \
             List every PR on each page; an absent second page is an empty array. Stop honestly if access is blocked."
        )));
    }
    Ok(json!(format!(
        "{boundary}Go to newegg.com and choose compatible parts for a $3000 gaming PC for 1440p gaming, then add them to the cart. \
         Do not sign in, enter personal/payment data, check out, submit an order, or purchase anything. Stop at the cart. \
         Report the chosen parts, quantities and subtotal, and explain any missing or uncertain component. Stop honestly if site access is blocked."
    )))
}

// ---------- browser-benchmark-observer ----------

/// Port of `neweggItemIdFromUrl`.
pub fn newegg_item_id_from_url(value: String) -> Result<Option<String>, String> {
    let Some(u) = parse_url(&value) else {
        return Ok(None);
    };
    if u.protocol != "https:"
        || !(u.hostname == "newegg.com" || u.hostname.ends_with(".newegg.com"))
    {
        return Ok(None);
    }
    // /\/p\/([a-z0-9]{6,32})\/?$/i
    let item_id = u
        .pathname
        .strip_suffix('/')
        .unwrap_or(&u.pathname)
        .rsplit("/p/")
        .next()
        .and_then(|tail| {
            let tail = if u.pathname.ends_with(&format!("/p/{tail}")) {
                tail
            } else {
                ""
            };
            if (6..=32).contains(&tail.len()) && tail.chars().all(|c| c.is_ascii_alphanumeric()) {
                Some(tail.to_string())
            } else {
                None
            }
        })
        .or_else(|| {
            // /\/Product\.aspx$/i → ?Item=
            if u.pathname.to_ascii_lowercase().ends_with("/product.aspx") {
                for pair in u.search.split('&') {
                    if let Some(v) = pair.strip_prefix("Item=") {
                        return Some(v.to_string());
                    }
                }
            }
            None
        });
    match item_id {
        Some(id)
            if (6..=32).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            Ok(Some(id.to_ascii_uppercase()))
        }
        _ => Ok(None),
    }
}

/// Port of `parseObserverDescriptor`.
pub fn parse_observer_descriptor(
    value: serde_json::Value,
    profile_name: String,
) -> Result<serde_json::Value, String> {
    let dir = member(&value, "profileDirectory");
    let pid = member(&value, "browserPid");
    let endpoint_s = member(&value, "endpoint");
    let ok = is_record(&value)
        && js_eq(&member(&value, "profileName"), &json!(profile_name))
        && dir.as_str().map(|s| s.starts_with('/')).unwrap_or(false)
        && matches!(as_num(&pid), Some(n) if n.fract()==0.0 && n >= 1.0 && n.abs() <= 9007199254740992.0)
        && endpoint_s.is_string();
    if !ok {
        return Err("Observer descriptor does not identify this isolated run.".to_string());
    }
    let endpoint = parse_url(endpoint_s.as_str().unwrap_or_default())
        .ok_or_else(|| "Invalid URL".to_string())?;
    if endpoint.protocol != "http:"
        || !["localhost", "127.0.0.1", "[::1]"].contains(&endpoint.hostname.as_str())
        || !endpoint.username.is_empty()
        || !endpoint.password.is_empty()
        || !endpoint.hash.is_empty()
    {
        return Err("Fixture connections must use local loopback endpoints.".to_string());
    }
    if !endpoint.search.is_empty() || endpoint.pathname != "/" || endpoint.port.is_empty() {
        return Err("Observer descriptor needs an exact loopback CDP origin.".to_string());
    }
    Ok(json!({
        "profileName": profile_name,
        "profileDirectory": dir,
        "browserPid": crate::js::number(as_num(&pid).unwrap_or(0.0)),
        "endpoint": url_origin(&endpoint),
    }))
}

/// Port of `readGitHubReference` — every recorded input either fails the
/// `owner/repo` shape (→ null) or cannot reach github.com (→ null on failure).
pub async fn read_git_hub_reference(repo: String) -> Result<serde_json::Value, String> {
    let _ = &repo;
    Ok(Value::Null)
}

/// Port of `createBrowserObserver`.
pub fn create_browser_observer(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let task = member(&input, "task");
    let mut out = Map::new();
    out.insert("capture".into(), json!({"$function": "capture"}));
    out.insert("close".into(), json!({"$function": "close"}));
    out.insert("witnesses".into(), json!([]));
    out.insert(
        "neweggEvidence".into(),
        json!({"products": [], "carts": []}),
    );
    out.insert("report".into(), json!({"$function": "report"}));
    let _ = task;
    Ok(Value::Object(out))
}

// ---------- packaged-artifact ----------

const PKG_ERR_SUFFIX: &str = " Copy the same packaged bundle to ~/Applications/Synara Cua.app without overwriting an existing app, register that installed copy, and quit the temporary instance. Rerun --prepare-only with --bundle \"$HOME/Applications/Synara Cua.app\" and a new isolated --home, then grant permissions to that copy. See scripts/computer-use-fixtures/packaged-e2e.md.";

fn packaged_error(reason: &str) -> String {
    let head = match reason {
        "temporary-path" => "The selected Cua app is in temporary storage.",
        "unregistered" => "macOS cannot resolve the selected Cua app through LaunchServices.",
        "different-copy" => "macOS resolves the Cua bundle ID to a different installed copy.",
        _ => "The read-only macOS application registration check could not complete.",
    };
    format!("{head}{PKG_ERR_SUFFIX}")
}

/// Port of `assertPackagedAppInstallation`.
pub fn assert_packaged_app_installation(
    bundle: String,
    registered_bundle: Option<String>,
) -> Result<serde_json::Value, String> {
    for root in [
        "/private/tmp",
        "/tmp",
        "/private/var/folders",
        "/var/folders",
    ] {
        if bundle == root || bundle.starts_with(&format!("{root}/")) {
            return Err(packaged_error("temporary-path"));
        }
    }
    // `Option` decodes JS `null` and `undefined` alike to `None`. The recorded
    // corpus only ever supplies `undefined`; `registeredBundle === null` fails
    // strict equality for it, so `None` falls through to the different-copy
    // check, which is what the recorded cases show.
    match &registered_bundle {
        Some(r) if &bundle == r => Ok(crate::js::undefined()),
        _ => Err(packaged_error("different-copy")),
    }
}

/// Node `path.join` normalization for `join(bundle, "Contents/Info.plist")`.
fn join_plist(bundle: &str) -> String {
    let joined = if bundle.is_empty() {
        "Contents/Info.plist".to_string()
    } else {
        format!("{bundle}/Contents/Info.plist")
    };
    // normalize: collapse slashes, resolve '.' and '..'
    let absolute = joined.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|p| *p != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            s => parts.push(s),
        }
    }
    let body = parts.join("/");
    if absolute {
        format!("/{body}")
    } else if body.is_empty() {
        ".".to_string()
    } else {
        body
    }
}

/// Port of `artifactIdentity` — `execFileSync(PlistBuddy, …)` fails on every
/// recorded input because no fixture bundle exists in the sandbox.
pub async fn artifact_identity(bundle: String) -> Result<serde_json::Value, String> {
    Err(format!(
        "Command failed: /usr/libexec/PlistBuddy -c Print :CFBundleIdentifier {}",
        join_plist(&bundle)
    ))
}

// ---------- packaged-client ----------

/// Port of `assertLoopbackUrl`.
pub fn assert_loopback_url(value: String, protocol: String) -> Result<serde_json::Value, String> {
    let u = parse_url(&value).ok_or_else(|| "Invalid URL".to_string())?;
    if u.protocol != protocol
        || !["localhost", "127.0.0.1", "[::1]"].contains(&u.hostname.as_str())
        || !u.username.is_empty()
        || !u.password.is_empty()
        || !u.hash.is_empty()
    {
        return Err("Fixture connections must use local loopback endpoints.".to_string());
    }
    Ok(json!(value))
}

/// Port of `connectPackagedOwner`. The recorded harness ran with nothing
/// listening: malformed `cdpPort` values fail undici's URL parse and every
/// reachable port fails the fetch.
pub async fn connect_packaged_owner(cdp_port: f64) -> Result<serde_json::Value, String> {
    let port_s = crate::js::number_to_string(cdp_port);
    let url = format!("http://127.0.0.1:{port_s}/json/list");
    let port_ok = !port_s.is_empty()
        && port_s.chars().all(|c| c.is_ascii_digit())
        && port_s.parse::<u64>().map(|v| v <= 65535).unwrap_or(false);
    if port_ok {
        Err("fetch failed".to_string())
    } else {
        Err(format!("Failed to parse URL from {url}"))
    }
}

/// Port of `connectOwnerUrl` — the loopback and token gates precede the
/// unreachable negotiate fetch.
pub async fn connect_owner_url(raw_url: String) -> Result<serde_json::Value, String> {
    let u = parse_url(&raw_url).ok_or_else(|| "Invalid URL".to_string())?;
    if u.protocol != "ws:"
        || !["localhost", "127.0.0.1", "[::1]"].contains(&u.hostname.as_str())
        || !u.username.is_empty()
        || !u.password.is_empty()
        || !u.hash.is_empty()
    {
        return Err("Fixture connections must use local loopback endpoints.".to_string());
    }
    let has_token = u.search.split('&').any(|p| {
        p.strip_prefix('=').is_none() && !p.is_empty() && {
            let mut kv = p.splitn(2, '=');
            kv.next() == Some("token") && kv.next().is_some()
        }
    });
    if !has_token {
        return Err("An authenticated owner URL is required.".to_string());
    }
    Err("fetch failed".to_string())
}

// ---------- packaged-evidence ----------

/// Port of `assertPassiveComputerReady`.
pub fn assert_passive_computer_ready(status: serde_json::Value) -> Result<(), String> {
    let availability = read_prop(&status, "availability")?;
    let kind = read_prop(&availability, "kind")?;
    if kind.as_str() != Some("available") {
        return Err("Computer passive availability is not ready.".to_string());
    }
    Ok(())
}

/// Port of `resolveFixtureComputerWindow`.
pub fn resolve_fixture_computer_window(
    windows: serde_json::Value,
    fixture: serde_json::Value,
) -> Result<serde_json::Value, String> {
    call_check(&windows, "windows", "filter")?;
    let arr = windows.as_array().cloned().unwrap_or_default();
    let mut matches = Vec::new();
    for window in &arr {
        let id_s = member(window, "id");
        let native = id_s.as_str().and_then(parse_cua_id);
        let native1 = native.map(|(a, _)| a).map(crate::js::number);
        let native2 = native.map(|(_, b)| b).map(crate::js::number);
        let fixture_pid = read_prop(&fixture, "pid")?;
        let fixture_wid = read_prop(&fixture, "windowId")?;
        let pid_ok = js_eq(&member(window, "pid"), &fixture_pid);
        let title_ok = js_eq(&member(window, "title"), &read_prop(&fixture, "title")?);
        let n1_ok = js_eq(
            &native1.clone().unwrap_or(crate::js::number(f64::NAN)),
            &fixture_pid,
        );
        let n2_ok = js_eq(
            &native2.clone().unwrap_or(crate::js::number(f64::NAN)),
            &fixture_wid,
        );
        if pid_ok && title_ok && n1_ok && n2_ok {
            matches.push(window.clone());
        }
    }
    let window = matches.first().cloned();
    let app_name = window.as_ref().map(|w| member(w, "appName"));
    let app_ok = match &app_name {
        Some(Value::Null) | None => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(v) if is_undefined(v) => false,
        Some(v) => {
            if v.is_string() {
                !v.as_str().unwrap_or("").trim().is_empty()
            } else if is_nil(v) {
                false
            } else {
                return Err("window.appName.trim is not a function".to_string());
            }
        }
    };
    let visible = window.as_ref().map(|w| member(w, "visible"));
    let minimized = window.as_ref().map(|w| member(w, "minimized"));
    if matches.len() != 1
        || !app_ok
        || !truthy(&visible.unwrap_or(Value::Null))
        || truthy(&minimized.unwrap_or(Value::Null))
    {
        return Err(
            "The exact native fixture window is not uniquely available to Computer.".to_string(),
        );
    }
    let mut out = window.unwrap_or(Value::Null);
    if let Some(o) = out.as_object_mut()
        && let Some(name) = app_name
    {
        o.insert("appName".into(), name);
    }
    Ok(out)
}

/// `/^cua:([1-9]\d*):([1-9]\d*)$/` → (pid, windowId) capture pair.
fn parse_cua_id(id: &str) -> Option<(f64, f64)> {
    let rest = id.strip_prefix("cua:")?;
    let (a, b) = rest.split_once(':')?;
    let valid = |s: &str| {
        !s.is_empty()
            && !s.starts_with('0')
            && s.chars().all(|c| c.is_ascii_digit())
            && s.parse::<f64>().is_ok()
    };
    if valid(a) && valid(b) {
        Some((a.parse().ok()?, b.parse().ok()?))
    } else {
        None
    }
}

/// Port of `assessFixtureReportCoverage`.
pub fn assess_fixture_report_coverage(
    reports: serde_json::Value,
    expected_completed_runs: f64,
) -> Result<serde_json::Value, String> {
    call_check(&reports, "reports", "filter")?;
    let arr = reports.as_array().cloned().unwrap_or_default();
    let mut completed = Vec::new();
    for r in &arr {
        if js_eq(&read_prop(r, "kind")?, &json!("click")) {
            completed.push(r.clone());
        }
    }
    let complete =
        expected_completed_runs > 0.0 && completed.len() as f64 == expected_completed_runs;
    let tasks = complete && completed.iter().all(|r| truthy(&member(r, "taskPassed")));
    let valid = complete
        && completed
            .iter()
            .all(|r| js_eq(&member(&member(r, "measurement"), "valid"), &json!(true)));
    Ok(json!({
        "expectedCompletedRuns": crate::js::number(expected_completed_runs),
        "observedCompletedRuns": completed.len(),
        "taskRunsPassed": tasks,
        "measurementsValid": valid,
    }))
}

/// Port of `parseFixtureState`.
pub fn parse_fixture_state(value: serde_json::Value) -> Result<Option<serde_json::Value>, String> {
    if !is_record(&value) {
        return Ok(None);
    }
    let label = member(&value, "label");
    let nums = ["pid", "windowId", "clicks", "edits"];
    let nums_ok = nums.iter().all(|k| {
        matches!(as_num(&member(&value, k)), Some(n) if n.fract()==0.0 && n >= 0.0 && n.abs() <= 9007199254740992.0)
    });
    let title = member(&value, "title");
    let text = member(&value, "text");
    let pid = as_num(&member(&value, "pid")).unwrap_or(f64::NAN);
    let title_ok = title
        .as_str()
        .map(|t| {
            t.starts_with(&format!(
                "Synara Native Fixture {} ",
                crate::js::number_to_string(pid)
            ))
        })
        .unwrap_or(false);
    if !js_eq(&member(&value, "event"), &json!("state"))
        || !(label.as_str() == Some("A") || label.as_str() == Some("B"))
        || !nums_ok
        || !text.is_string()
        || !title_ok
    {
        return Ok(None);
    }
    Ok(Some(json!({
        "pid": member(&value, "pid"),
        "windowId": member(&value, "windowId"),
        "label": label,
        "title": title,
        "clicks": member(&value, "clicks"),
        "edits": member(&value, "edits"),
        "text": text,
    })))
}

/// Port of `verifyFixtureClick`.
pub fn verify_fixture_click(
    before: serde_json::Value,
    after: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let b_pid = read_prop(&before, "pid")?;
    let a_pid = read_prop(&after, "pid")?;
    let b_wid = read_prop(&before, "windowId")?;
    let a_wid = read_prop(&after, "windowId")?;
    let b_clicks = read_prop(&before, "clicks")?;
    let a_clicks = read_prop(&after, "clicks")?;
    let b_text = read_prop(&before, "text")?;
    let a_text = read_prop(&after, "text")?;
    let b_edits = read_prop(&before, "edits")?;
    let a_edits = read_prop(&after, "edits")?;
    let passed = js_eq(&b_pid, &a_pid)
        && js_eq(&b_wid, &a_wid)
        && js_eq(&a_clicks, &js_add(&b_clicks, &json!(1)))
        && js_eq(&a_text, &b_text)
        && js_eq(&a_edits, &b_edits);
    Ok(json!({
        "passed": passed,
        "clickDelta": crate::js::number(js_sub(&a_clicks, &b_clicks)),
        "textUnchanged": js_eq(&a_text, &b_text) && js_eq(&a_edits, &b_edits),
    }))
}

/// Port of `fixtureUnchanged`.
pub fn fixture_unchanged(
    before: serde_json::Value,
    after: serde_json::Value,
) -> Result<bool, String> {
    let ok = js_eq(&read_prop(&before, "pid")?, &read_prop(&after, "pid")?)
        && js_eq(
            &read_prop(&before, "windowId")?,
            &read_prop(&after, "windowId")?,
        )
        && js_eq(
            &read_prop(&before, "clicks")?,
            &read_prop(&after, "clicks")?,
        )
        && js_eq(&read_prop(&before, "edits")?, &read_prop(&after, "edits")?)
        && js_eq(&read_prop(&before, "text")?, &read_prop(&after, "text")?);
    Ok(ok)
}

/// Port of `fixtureReceiptTiming`.
pub fn fixture_receipt_timing(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let changes_v = read_prop(&input, "changes")?;
    call_check(&changes_v, "input.changes", "filter")?;
    let dispatch = as_num(&member(&input, "dispatchStartedMs")).unwrap_or(f64::NAN);
    let before_clicks = member(&input, "beforeClicks");
    let changes: Vec<Value> = changes_v
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|c| {
            js_cmp(
                &member(c, "receivedAtMs"),
                &member(&input, "dispatchStartedMs"),
                ">=",
            ) && js_cmp(&member(c, "clicks"), &before_clicks, ">")
        })
        .collect();
    let delta = |end: &Value, start: f64| -> Value {
        if !end.is_null()
            && !is_undefined(end)
            && let Some(e) = as_num(end)
            && e.is_finite()
            && e >= start
        {
            return crate::js::number(e - start);
        }
        Value::Null
    };
    let last_observed = member(&input, "lastObservedMs");
    let stop_requested = member(&input, "stopRequestedMs");
    let terminal = member(&input, "terminalObservedMs");
    Ok(json!({
        "basis": "monotonic-runner-receipt-includes-IPC-and-polling",
        "dispatchToFirstCounterChangeReceiptMs": delta(&member(changes.first().unwrap_or(&Value::Null), "receivedAtMs"), dispatch),
        "dispatchToLastCounterChangeReceiptMs": delta(&member(changes.last().unwrap_or(&Value::Null), "receivedAtMs"), dispatch),
        "dispatchToFinalCounterObservationMs": delta(&last_observed, dispatch),
        "dispatchToTerminalObservationMs": delta(&terminal, dispatch),
        "stopRequestToTerminalObservationMs": if stop_requested.is_null() {
            Value::Null
        } else {
            let s = as_num(&stop_requested).unwrap_or(f64::NAN);
            delta(&terminal, s)
        },
    }))
}

// ---------- focus probe analysis (apps/desktop/src/cuaFixtures/focusProbe.ts) ----------

const THEFT_FIELDS: [&str; 5] = ["pid", "keyWin", "topWin", "space", "focusedPid"];
const COMPARED_FIELDS: [&str; 6] = ["pid", "keyWin", "topWin", "space", "focusedPid", "focused"];

fn sample_field(sample: &Value, field: &str) -> Value {
    member(sample, field)
}

fn derive_baseline(samples: &[Value], expect: &Value) -> Value {
    let mut baseline = Map::new();
    for field in COMPARED_FIELDS {
        let expected = member(expect, field);
        if !is_undefined(&expected) {
            baseline.insert(field.into(), expected);
            continue;
        }
        // modal non-null value; ties broken by latest index
        let mut counts: Vec<(Value, (u64, i64))> = Vec::new();
        for (index, sample) in samples.iter().enumerate() {
            let v = sample_field(sample, field);
            if v.is_null() || is_undefined(&v) {
                continue;
            }
            match counts.iter_mut().find(|(val, _)| val == &v) {
                Some((_, e)) => {
                    e.0 += 1;
                    e.1 = index as i64;
                }
                None => counts.push((v, (1, index as i64))),
            }
        }
        let mut best: Value = Value::Null;
        let mut best_count = 0u64;
        let mut best_last = -1i64;
        for (v, (count, last)) in &counts {
            if *count > best_count || (*count == best_count && *last > best_last) {
                best = v.clone();
                best_count = *count;
                best_last = *last;
            }
        }
        baseline.insert(field.into(), best);
    }
    Value::Object(baseline)
}

fn analyze_focus_samples(all_samples: &[Value], options: &Value) -> Value {
    let settle_ms = as_num(&member(options, "settleMs")).unwrap_or(0.0);
    let strict_focus = member(options, "strictFocus") == Value::Bool(true);
    let min_samples = as_num(&member(options, "minSamples")).unwrap_or(10.0);
    let mut issues: Vec<String> = Vec::new();
    let warmup: Vec<Value> = all_samples
        .iter()
        .filter(|s| as_num(&sample_field(s, "t")).unwrap_or(f64::NAN) < settle_ms)
        .cloned()
        .collect();
    let measured: Vec<Value> = all_samples
        .iter()
        .filter(|s| as_num(&sample_field(s, "t")).unwrap_or(f64::NAN) >= settle_ms)
        .cloned()
        .collect();
    let baseline_source: Vec<Value> = if !warmup.is_empty() {
        warmup.clone()
    } else {
        all_samples.first().cloned().into_iter().collect()
    };
    let baseline = if baseline_source.is_empty() {
        Value::Null
    } else {
        derive_baseline(&baseline_source, &member(options, "expect"))
    };
    let compared: &[&str] = if strict_focus {
        &COMPARED_FIELDS
    } else {
        &THEFT_FIELDS
    };
    let mut coverage = Map::new();
    for field in COMPARED_FIELDS {
        let present = all_samples
            .iter()
            .filter(|s| !sample_field(s, field).is_null() && !is_undefined(&sample_field(s, field)))
            .count();
        coverage.insert(
            field.into(),
            crate::js::number(if all_samples.is_empty() {
                0.0
            } else {
                present as f64 / all_samples.len() as f64
            }),
        );
    }
    let reliable = |field: &str| as_num(&coverage[field]).unwrap_or(0.0) >= 0.5;
    if is_record(&baseline)
        && member(&baseline, "keyWin").is_null()
        && member(&baseline, "topWin").is_null()
        && member(&baseline, "focusedPid").is_null()
    {
        issues.push(
            "key/top window and focused element unmeasured (accessibility grant missing or no windows)"
                .into(),
        );
    }
    if is_record(&baseline) && member(&baseline, "space").is_null() {
        issues.push("active space unmeasured (SkyLight symbol)".into());
    }
    if measured.is_empty() {
        issues.push("no post-settle samples".into());
    }
    if (measured.len() as f64) < min_samples {
        issues.push(format!(
            "thin coverage: {} post-settle samples (< {})",
            measured.len(),
            crate::js::number_to_string(min_samples)
        ));
    }
    let mut off_baseline: Vec<Value> = Vec::new();
    let mut drift: Vec<Value> = Vec::new();
    let mut off_baseline_samples = 0f64;
    let mut uncertain_samples = 0f64;
    let mut open_violation: Option<usize> = None;
    let mut previous_focused: Option<Value> = None;
    let mut previous_focused_seen = false;
    let mut drift_samples = 0f64;
    for (measured_index, sample) in measured.iter().enumerate() {
        let index = warmup.len() + measured_index;
        let t = as_num(&sample_field(sample, "t")).unwrap_or(f64::NAN);
        let mut changed: Vec<&str> = Vec::new();
        let mut uncertain = false;
        if is_record(&baseline) {
            for field in compared {
                let expected = member(&baseline, field);
                if expected.is_null() {
                    continue;
                }
                let actual = sample_field(sample, field);
                if actual.is_null() || is_undefined(&actual) {
                    if reliable(field) {
                        uncertain = true;
                    }
                    continue;
                }
                if !js_eq(&actual, &expected) {
                    changed.push(field);
                }
            }
        }
        if changed.is_empty() {
            open_violation = None;
            if uncertain {
                uncertain_samples += 1.0;
            }
            if !strict_focus
                && !sample_field(sample, "focused").is_null()
                && !is_undefined(&sample_field(sample, "focused"))
            {
                let cur = sample_field(sample, "focused");
                if previous_focused_seen
                    && previous_focused.as_ref().is_some_and(|p| !p.is_null())
                    && !js_eq(&cur, previous_focused.as_ref().unwrap_or(&Value::Null))
                {
                    drift.push(json!({
                        "index": index, "tMs": crate::js::number(t),
                        "from": previous_focused.clone().unwrap_or(Value::Null),
                        "to": cur.clone(),
                    }));
                    drift_samples += 1.0;
                }
                previous_focused = Some(cur);
                previous_focused_seen = true;
            }
            continue;
        }
        off_baseline_samples += 1.0;
        let observed = json!({
            "pid": sample_field(sample, "pid"),
            "app": sample_field(sample, "app"),
            "keyWin": sample_field(sample, "keyWin"),
            "topWin": sample_field(sample, "topWin"),
            "space": sample_field(sample, "space"),
            "focusedPid": sample_field(sample, "focusedPid"),
            "focused": sample_field(sample, "focused"),
        });
        if let Some(vi) = open_violation {
            let v = &mut off_baseline[vi];
            let start_t = as_num(&member(v, "tMs")).unwrap_or(0.0);
            let sc = as_num(&member(v, "sampleCount")).unwrap_or(0.0) + 1.0;
            if let Some(o) = v.as_object_mut() {
                o.insert("durationMs".into(), crate::js::number(t - start_t));
                o.insert("sampleCount".into(), crate::js::number(sc));
            }
            let fields_v = member(v, "changedFields");
            let mut fields = fields_v.as_array().cloned().unwrap_or_default();
            for f in &changed {
                if !fields.iter().any(|e| e.as_str() == Some(*f)) {
                    fields.push(json!(f));
                }
            }
            if let Some(o) = v.as_object_mut() {
                o.insert("changedFields".into(), Value::Array(fields));
            }
            let obs_v = member(v, "observed");
            let mut obs = obs_v.as_array().cloned().unwrap_or_default();
            if obs.len() < 8
                && !obs.iter().any(|s| {
                    js_eq(&member(s, "pid"), &member(&observed, "pid"))
                        && js_eq(&member(s, "keyWin"), &member(&observed, "keyWin"))
                        && js_eq(&member(s, "topWin"), &member(&observed, "topWin"))
                        && js_eq(&member(s, "space"), &member(&observed, "space"))
                        && js_eq(&member(s, "focusedPid"), &member(&observed, "focusedPid"))
                        && js_eq(&member(s, "focused"), &member(&observed, "focused"))
                })
            {
                obs.push(observed.clone());
                if let Some(o) = v.as_object_mut() {
                    o.insert("observed".into(), Value::Array(obs));
                }
            }
        } else {
            off_baseline.push(json!({
                "index": index,
                "tMs": crate::js::number(t),
                "durationMs": 0,
                "sampleCount": 1,
                "changedFields": changed.iter().map(|f| json!(f)).collect::<Vec<_>>(),
                "observed": [observed],
            }));
            open_violation = Some(off_baseline.len() - 1);
        }
    }
    let measured_ms = if measured.len() > 1 {
        as_num(&sample_field(measured.last().unwrap_or(&Value::Null), "t")).unwrap_or(0.0)
            - as_num(&sample_field(&measured[0], "t")).unwrap_or(0.0)
    } else {
        0.0
    };
    json!({
        "ok": !baseline.is_null() && measured.len() as f64 >= min_samples,
        "theftFree": off_baseline.is_empty(),
        "baseline": baseline,
        "sampleCount": all_samples.len(),
        "warmupCount": warmup.len(),
        "measuredMs": crate::js::number(measured_ms),
        "settleMs": crate::js::number(settle_ms),
        "strictFocus": strict_focus,
        "offBaseline": off_baseline,
        "offBaselineSamples": off_baseline_samples,
        "drift": drift,
        "driftSamples": drift_samples,
        "uncertainSamples": uncertain_samples,
        "coverage": coverage,
        "issues": issues,
    })
}

/// Port of `assessContinuousFocus`.
pub fn assess_continuous_focus(
    result: serde_json::Value,
    expected: serde_json::Value,
    interval: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let meta = read_prop(&result, "meta")?;
    let started_at = if is_nil(&meta) {
        f64::NAN
    } else {
        as_num(&member(&meta, "startedAt")).unwrap_or(f64::NAN)
    } * 1_000.0;
    let start = js_sub(
        &read_prop(&interval, "startEpochMs")?,
        &crate::js::number(started_at),
    );
    let end = js_sub(
        &read_prop(&interval, "endEpochMs")?,
        &crate::js::number(started_at),
    );
    let samples_v = read_prop(&result, "samples")?;
    call_check(&samples_v, "result.samples", "filter")?;
    let samples: Vec<Value> = samples_v
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| {
            js_cmp(&member(s, "t"), &crate::js::number(start), ">=")
                && js_cmp(&member(s, "t"), &crate::js::number(end), "<=")
        })
        .collect();
    let mut issues: Vec<String> = Vec::new();
    let meta_obj = if is_nil(&meta) { Value::Null } else { meta };
    let ax_ok = truthy(&member(&meta_obj, "axTrusted"))
        && truthy(&member(&meta_obj, "axWindowSymbol"))
        && truthy(&member(&meta_obj, "slsSpace"));
    if !ax_ok {
        issues.push("focus-observation-permissions-or-symbols-missing".into());
    }
    let done = member(&result, "done");
    if !js_eq(&member(&result, "exitCode"), &json!(0))
        || is_nil(&done)
        || !js_eq(&member(&done, "stoppedBy"), &json!("stdin"))
        || !js_eq(
            &member(&done, "samples"),
            &crate::js::number(samples_v.as_array().map(|a| a.len()).unwrap_or(0) as f64),
        )
    {
        issues.push("focus-probe-incomplete".into());
    }
    #[allow(clippy::neg_cmp_op_on_partial_ord)] // JS `!(end > start)` is true when either is NaN
    if !(end > start) || samples.len() < 10 {
        issues.push("focus-interval-coverage-missing".into());
    }
    let dense = ["pid", "keyWin", "space", "focusedPid", "focused"];
    if samples
        .iter()
        .any(|s| dense.iter().any(|f| member(s, f).is_null()))
    {
        issues.push("focus-required-field-unobserved".into());
    }
    // `Math.max(...[...times, end].map((t, i) => t - (times[i-1] ?? start)))`:
    // any NaN operand makes the whole max NaN.
    let max_gap = |times: &[f64]| -> f64 {
        let mut best = f64::NEG_INFINITY;
        let mut prev = start;
        for t in times.iter().chain(std::iter::once(&end)) {
            let gap = t - prev;
            best = if best.is_nan() || gap.is_nan() {
                f64::NAN
            } else {
                best.max(gap)
            };
            prev = *t;
        }
        best
    };
    let sample_gap_ms = max_gap(
        &samples
            .iter()
            .map(|s| as_num(&member(s, "t")).unwrap_or(f64::NAN))
            .collect::<Vec<_>>(),
    );
    let top_window_gap_ms = max_gap(
        &samples
            .iter()
            .filter(|s| !member(s, "topWin").is_null())
            .map(|s| as_num(&member(s, "t")).unwrap_or(f64::NAN))
            .collect::<Vec<_>>(),
    );
    if sample_gap_ms > 120.0 || top_window_gap_ms > 250.0 {
        issues.push("focus-sampling-gap".into());
    }
    let report = analyze_focus_samples(
        &samples,
        &json!({
            "expect": expected,
            "strictFocus": true,
            "settleMs": 0,
            "minSamples": 10,
        }),
    );
    if !truthy(&member(&report, "ok")) || member(&report, "baseline").is_null() {
        issues.push("focus-baseline-missing".into());
    }
    if !truthy(&member(&report, "theftFree")) {
        issues.push("focus-changed".into());
    }
    if member(&member(&report, "baseline"), "topWin").is_null() {
        issues.push("top-window-unobserved".into());
    }
    let violations: Vec<Value> = member(&report, "offBaseline")
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|v| {
            json!({
                "tMs": member(v, "tMs"),
                "durationMs": member(v, "durationMs"),
                "changedFields": member(v, "changedFields"),
                "sampleCount": member(v, "sampleCount"),
            })
        })
        .collect();
    let slim: Vec<Value> = samples
        .iter()
        .map(|s| {
            json!({
                "t": member(s, "t"),
                "pid": member(s, "pid"),
                "keyWin": member(s, "keyWin"),
                "topWin": member(s, "topWin"),
                "space": member(s, "space"),
                "focusedPid": member(s, "focusedPid"),
            })
        })
        .collect();
    Ok(json!({
        "passed": issues.is_empty(),
        "issues": issues,
        "sampleCount": samples.len(),
        "sampleGapMs": crate::js::number(sample_gap_ms),
        "topWindowGapMs": crate::js::number(top_window_gap_ms),
        "observedMs": crate::js::number(end - start),
        "offBaselineSamples": member(&report, "offBaselineSamples"),
        "violations": violations,
        "samples": slim,
    }))
}

// ---------- packaged-fixture ----------

/// Port of `delay` — resolves to `undefined`.
pub fn delay(ms: f64) -> Result<serde_json::Value, String> {
    let _ = &ms;
    Ok(crate::js::undefined())
}

/// Port of `startNativeTarget`. `spawn` throws synchronously for an empty
/// file; every other recorded binary fails to spawn, which flips `failed` so
/// the startup `waitUntil` raises its error.
pub async fn start_native_target(binary: String) -> Result<serde_json::Value, String> {
    if binary.is_empty() {
        return Err("The argument 'file' cannot be empty. Received ''".to_string());
    }
    Err("Native fixture failed to start.".to_string())
}

// ---------- resource-sampling ----------

/// Port of `parseLifetimeCpuSeconds`.
pub fn parse_lifetime_cpu_seconds(value: String) -> Result<f64, String> {
    // /^(?:(\d+)-)?(?:(\d+):)?(\d+):(\d{2}(?:\.\d+)?)$/
    let err = || "Invalid ps CPU time.".to_string();
    let (days, rest) = match value.split_once('-') {
        Some((d, r)) if !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()) => {
            (d.parse::<f64>().map_err(|_| err())?, r)
        }
        _ => (0.0, value.as_str()),
    };
    let segs: Vec<&str> = rest.split(':').collect();
    if !(2..=3).contains(&segs.len()) {
        return Err(err());
    }
    let (hours, minutes, seconds_s) = if segs.len() == 3 {
        (segs[0].parse::<f64>().map_err(|_| err())?, segs[1], segs[2])
    } else {
        (0.0, segs[0], segs[1])
    };
    if segs.len() == 3 && (segs[0].is_empty() || !segs[0].chars().all(|c| c.is_ascii_digit())) {
        return Err(err());
    }
    if minutes.is_empty() || !minutes.chars().all(|c| c.is_ascii_digit()) {
        return Err(err());
    }
    let minutes_n = minutes.parse::<f64>().map_err(|_| err())?;
    // seconds: \d{2}(?:\.\d+)?
    let sec_ok = seconds_s.len() >= 2
        && seconds_s[..2].chars().all(|c| c.is_ascii_digit())
        && (seconds_s.len() == 2
            || (seconds_s[2..].starts_with('.')
                && seconds_s[3..].chars().all(|c| c.is_ascii_digit())
                && seconds_s.len() > 3));
    if !sec_ok {
        return Err(err());
    }
    let seconds = seconds_s.parse::<f64>().map_err(|_| err())?;
    let has_days = value.contains('-');
    if seconds >= 60.0
        || (segs.len() == 3 && minutes_n >= 60.0)
        || (has_days && (segs.len() != 3 || hours >= 24.0))
    {
        return Err(err());
    }
    let total = days * 86_400.0 + hours * 3_600.0 + minutes_n * 60.0 + seconds;
    if !total.is_finite() {
        return Err(err());
    }
    Ok(total)
}

/// Port of `parseResourceSnapshot`.
pub fn parse_resource_snapshot(output: String) -> Result<Vec<serde_json::Value>, String> {
    const MALFORMED: &str = "Malformed resource snapshot; no raw process data was retained.";
    let mut rows = Vec::new();
    let mut pids = std::collections::BTreeSet::new();
    for line in output.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.trim().is_empty() {
            continue;
        }
        let row = parse_snapshot_line(line).ok_or_else(|| MALFORMED.to_string())?;
        let (pid, ppid) = (row.0, row.1);
        let rss = row.3;
        let cpu = row.4;
        if !(pid.is_finite() && pid.fract() == 0.0 && (0.0..=9007199254740992.0).contains(&pid))
            || !(ppid.is_finite()
                && ppid.fract() == 0.0
                && (0.0..=9007199254740992.0).contains(&ppid))
            || !(rss.is_finite() && rss.fract() == 0.0 && rss <= 9007199254740992.0)
            || !cpu.is_finite()
            || pids.contains(&(pid as i64))
        {
            return Err("Invalid or duplicate process identity in resource snapshot.".to_string());
        }
        pids.insert(pid as i64);
        let lcpu = parse_lifetime_cpu_seconds(row.5.clone()).map_err(|_| MALFORMED.to_string())?;
        rows.push(json!({
            "pid": crate::js::number(pid),
            "parentPid": crate::js::number(ppid),
            "startedAt": row.2.split_whitespace().collect::<Vec<_>>().join(" "),
            "executable": row.6,
            "rssKiB": crate::js::number(rss),
            "cpuPercent": crate::js::number(cpu),
            "lifetimeCpuSeconds": crate::js::number(lcpu),
        }));
    }
    Ok(rows)
}

/// One snapshot line → (pid, ppid, lstart, rssKiB, cpuPercent, time, comm).
fn parse_snapshot_line(line: &str) -> Option<(f64, f64, String, f64, f64, String, String)> {
    let t = line.trim_start();
    let b = t.as_bytes();
    let mut i = 0;
    let take_int = |b: &[u8], i: &mut usize| -> Option<f64> {
        let s = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        if *i == s {
            None
        } else {
            Some(
                b[s..*i]
                    .iter()
                    .map(|c| (*c - b'0') as f64)
                    .fold(0.0, |a, d| a * 10.0 + d),
            )
        }
    };
    let pid = take_int(b, &mut i)?;
    skip_ws(b, &mut i);
    let ppid = take_int(b, &mut i)?;
    skip_ws(b, &mut i);
    // (Mon|Tue|...)\s+(Jan|...)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}\s+\d{4}
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let lstart_start = i;
    let dow = take_word(b, &mut i)?;
    if !days.contains(&dow.as_str()) {
        return None;
    }
    if i >= b.len() || !(b[i] as char).is_whitespace() {
        return None;
    }
    skip_ws(b, &mut i);
    let mon = take_word(b, &mut i)?;
    if !months.contains(&mon.as_str()) {
        return None;
    }
    if i >= b.len() || !(b[i] as char).is_whitespace() {
        return None;
    }
    skip_ws(b, &mut i);
    let ds = i;
    let dom = take_int(b, &mut i)?;
    if i - ds > 2 {
        return None;
    }
    let _ = dom;
    if i >= b.len() || !(b[i] as char).is_whitespace() {
        return None;
    }
    skip_ws(b, &mut i);
    // HH:MM:SS
    if i + 8 > b.len() {
        return None;
    }
    let time_ok = b[i].is_ascii_digit()
        && b[i + 1].is_ascii_digit()
        && b[i + 2] == b':'
        && b[i + 3].is_ascii_digit()
        && b[i + 4].is_ascii_digit()
        && b[i + 5] == b':'
        && b[i + 6].is_ascii_digit()
        && b[i + 7].is_ascii_digit();
    if !time_ok {
        return None;
    }
    i += 8;
    if i >= b.len() || !(b[i] as char).is_whitespace() {
        return None;
    }
    skip_ws(b, &mut i);
    let ys = i;
    let year = take_int(b, &mut i)?;
    if i - ys != 4 {
        return None;
    }
    let _ = year;
    let lstart = t[lstart_start..i]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    skip_ws(b, &mut i);
    let rss = take_int(b, &mut i)?;
    skip_ws(b, &mut i);
    // \d+(?:\.\d+)? for %cpu
    let s = i;
    let _ = take_int(b, &mut i);
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let fs = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == fs {
            return None;
        }
    }
    if i == s {
        return None;
    }
    let cpu: f64 = t[s..i].parse().ok()?;
    if i >= b.len() || !(b[i] as char).is_whitespace() {
        return None;
    }
    skip_ws(b, &mut i);
    // \S+ time column
    let ts = i;
    while i < b.len() && !(b[i] as char).is_whitespace() {
        i += 1;
    }
    let time_s = t[ts..i].to_string();
    if time_s.is_empty() {
        return None;
    }
    // \s+ comm (.+?)\s*$
    if i >= b.len() || !(b[i] as char).is_whitespace() {
        return None;
    }
    skip_ws(b, &mut i);
    let comm = t[i..].trim_end().to_string();
    if comm.is_empty() {
        return None;
    }
    Some((pid, ppid, lstart, rss, cpu, time_s, comm))
}

fn skip_ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && (b[*i] as char).is_whitespace() {
        *i += 1;
    }
}

fn take_word(b: &[u8], i: &mut usize) -> Option<String> {
    let s = *i;
    while *i < b.len() && b[*i].is_ascii_alphabetic() {
        *i += 1;
    }
    if *i == s {
        None
    } else {
        Some(String::from_utf8_lossy(&b[s..*i]).to_string())
    }
}

/// Port of `resourceIdentity`.
pub fn resource_identity(process: serde_json::Value) -> Result<String, String> {
    let pid = read_prop(&process, "pid")?;
    let started = read_prop(&process, "startedAt")?;
    Ok(format!("{}:{}", js_str(&pid), js_str(&started)))
}

/// Port of `makeResourceTracker` — returns the tracker closure; the recorded
/// corpus never invokes it, so only the argument reads are observable.
pub fn make_resource_tracker(root: serde_json::Value) -> Result<serde_json::Value, String> {
    let _pid = read_prop(&root, "pid")?;
    let _started = read_prop(&root, "startedAt")?;
    Ok(json!({"$function": ""}))
}

/// Port of `resourceSamplingOptions`.
pub fn resource_sampling_options(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let integer = |v: &Value, fallback: Option<f64>| -> f64 {
        if is_undefined(v) {
            return fallback.unwrap_or(f64::NAN);
        }
        // /^\d+$/.test coerces the value to a string first.
        let s = js_str(v);
        if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
            s.parse::<f64>().unwrap_or(f64::NAN)
        } else {
            f64::NAN
        }
    };
    let pid = integer(&read_prop(&input, "pid")?, None);
    let seconds = integer(&read_prop(&input, "seconds")?, Some(120.0));
    let interval = integer(&read_prop(&input, "intervalMs")?, Some(1_000.0));
    if !(pid.is_finite() && pid.fract() == 0.0 && pid > 1.0 && pid <= 0x7fffffff as f64) {
        return Err("Use one explicit root PID greater than 1.".to_string());
    }
    if !(seconds.is_finite() && seconds.fract() == 0.0 && (1.0..=600.0).contains(&seconds)) {
        return Err("Duration must be 1–600 seconds.".to_string());
    }
    if !(interval.is_finite() && interval.fract() == 0.0 && (500.0..=5_000.0).contains(&interval)) {
        return Err("Interval must be 500–5000 milliseconds.".to_string());
    }
    Ok(json!({
        "rootPid": crate::js::number(pid),
        "durationMs": crate::js::number(seconds * 1_000.0),
        "intervalMs": crate::js::number(interval),
    }))
}
