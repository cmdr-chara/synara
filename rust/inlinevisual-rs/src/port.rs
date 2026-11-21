//! The Rust port of `packages/shared/src/inlineVisual.ts`.

#![forbid(unsafe_code)]

mod data;

use crate::{js, utf16};
use data::{BASE_CSS, BOOTSTRAP_SCRIPT, CSP};
use regex::Regex;
use serde_json::{Value, json};

const SIZE_CHANGED_METHOD: &str = "ui/notifications/size-changed";
const OPEN_LINK_METHOD: &str = "ui/open-link";
const HOST_CONTEXT_CHANGED_METHOD: &str = "ui/notifications/host-context-changed";
const SCROLL_INTENT_METHOD: &str = "synara/notifications/scroll-intent";
const STOPPED_METHOD: &str = "synara/notifications/visual-stopped";
const MAX_LINK_LENGTH_UNITS: usize = 2048;
const THEME_STYLE_ID: &str = "synara-visual-theme";
const EMBED_FENCE: &str = "synara-visual";
const MAX_HEIGHT: f64 = 1600.0;
const MIN_HEIGHT: f64 = 80.0;

const JS_SPC: &str = "[\\t\\n\\x0b\\x0c\\r \\u{a0}\\u{1680}\\u{2000}-\\u{200a}\\u{2028}\\u{2029}\\u{202f}\\u{205f}\\u{3000}\\u{feff}]";

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
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

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(js::undefined)),
        _ if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        _ => Ok(js::undefined()),
    }
}

fn member_opt(v: &Value, key: &str) -> Value {
    match v {
        Value::Null | Value::Object(_) | Value::Array(_) => {
            v.get(key).cloned().unwrap_or_else(js::undefined)
        }
        _ => js::undefined(),
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if matches!(e, Value::Null) || js::is_undefined(e) {
                    String::new()
                } else {
                    js_to_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".into()
            } else if let Some(n) = js::as_number(v) {
                js::number_to_string(n)
            } else {
                "[object Object]".into()
            }
        }
    }
}

/// `Object.entries` key order: canonical array-index keys ascending, then the rest
/// in insertion order.
fn ordered_keys(map: &serde_json::Map<String, Value>) -> Vec<&String> {
    fn array_index(k: &str) -> Option<u32> {
        if k.is_empty() || (k.len() > 1 && k.starts_with('0')) {
            return None;
        }
        let n = k.parse::<u64>().ok()?;
        (n < 4_294_967_295 && n.to_string() == k).then_some(n as u32)
    }
    let mut indexed: Vec<(u32, &String)> = Vec::new();
    let mut rest: Vec<&String> = Vec::new();
    for k in map.keys() {
        match array_index(k) {
            Some(i) => indexed.push((i, k)),
            None => rest.push(k),
        }
    }
    indexed.sort_by_key(|(i, _)| *i);
    indexed.into_iter().map(|(_, k)| k).chain(rest).collect()
}

/// `Object.entries(v)`: objects and strings and arrays enumerate, `null`/`undefined` throw,
/// numbers and booleans are empty.
fn object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
    match v {
        Value::Null => Err("Cannot convert undefined or null to object".into()),
        Value::Object(o) => {
            if js::is_undefined(v) {
                return Err("Cannot convert undefined or null to object".into());
            }
            Ok(ordered_keys(o)
                .into_iter()
                .map(|k| (k.clone(), o[k].clone()))
                .collect())
        }
        Value::Array(a) => Ok(a
            .iter()
            .enumerate()
            .map(|(i, e)| (i.to_string(), e.clone()))
            .collect()),
        Value::String(s) => Ok(s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect()),
        _ => Ok(Vec::new()),
    }
}

/// Port of `clampInlineVisualHeight` (`packages/shared/src/inlineVisual.ts:65`).
pub fn clamp_inline_visual_height(height: f64) -> Result<f64, String> {
    let rounded = js::math_round(if height.is_finite() { height } else { 0.0 });
    Ok(MIN_HEIGHT.max(rounded).min(MAX_HEIGHT))
}

fn object_member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

/// A WHATWG-URL-subset parse for `new URL(url)`: recognizes `scheme:` + authority +
/// path + query + fragment, normalizes case, drops the default port and resolves
/// dot segments. Returns (protocol, href) on success.
fn parse_url(url: &str) -> Option<(String, String)> {
    let rest = url;
    // scheme: ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    let colon = rest.find(':')?;
    let scheme = &rest[..colon];
    if scheme.is_empty()
        || !scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return None;
    }
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        // A valid URL of another scheme: caller only accepts http/https anyway.
        return Some((scheme, String::new()));
    }
    // Special schemes (http/https): `new URL("http:x")` treats "x" as the host
    // with or without the "//" marker, so strip it when present.
    let after_scheme = &rest[colon + 1..];
    let rest = after_scheme.strip_prefix("//").unwrap_or(after_scheme);
    // Split authority / path / query / fragment.
    let (before_frag, fragment) = match rest.find('#') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };
    let (before_query, query) = match before_frag.find('?') {
        Some(i) => (&before_frag[..i], &before_frag[i + 1..]),
        None => (before_frag, ""),
    };
    let slash = before_query.find('/');
    let (authority, path) = match slash {
        Some(i) => (&before_query[..i], &before_query[i..]),
        None => (before_query, "/"),
    };
    // userinfo@host:port
    let hostport = authority.rsplit('@').next().unwrap_or("");
    let userinfo = if authority.len() > hostport.len() {
        Some(&authority[..authority.len() - hostport.len()])
    } else {
        None
    };
    let (host, port) = match hostport.rfind(':') {
        Some(i) if !hostport.ends_with(']') => (&hostport[..i], Some(&hostport[i + 1..])),
        _ => (hostport, None),
    };
    if host.is_empty() {
        return None;
    }
    let host = host.to_ascii_lowercase();
    let default_port = if scheme == "http" { "80" } else { "443" };
    let port = port.filter(|p| *p != default_port);
    // Resolve dot segments in the path.
    let mut segs: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "." => {}
            ".." => {
                segs.pop();
            }
            _ => segs.push(seg),
        }
    }
    let trailing = path.ends_with("/.") || path.ends_with("/..") || path.ends_with('/');
    let mut resolved = segs.join("/");
    if !resolved.starts_with('/') {
        resolved = format!("/{resolved}");
    }
    if trailing && !resolved.ends_with('/') {
        resolved.push('/');
    }
    if resolved.is_empty() {
        resolved = "/".into();
    }
    let mut href = format!("{scheme}://");
    if let Some(ui) = userinfo {
        href.push_str(ui);
    }
    href.push_str(&host);
    if let Some(p) = port {
        href.push(':');
        href.push_str(p);
    }
    href.push_str(&resolved);
    if !query.is_empty() || before_frag.contains('?') {
        href.push('?');
        href.push_str(query);
    }
    if !fragment.is_empty() || rest.contains('#') {
        href.push('#');
        href.push_str(fragment);
    }
    Some((scheme, href))
}

/// Port of `readInlineVisualFrameMessage` (`packages/shared/src/inlineVisual.ts:94`).
pub fn read_inline_visual_frame_message(
    data: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let is_object = matches!(data, Value::Object(_) | Value::Array(_));
    if !is_object {
        return Ok(Some(js::undefined()));
    }
    let jsonrpc = member_opt(&data, "jsonrpc");
    if !matches!(jsonrpc, Value::String(ref s) if s == "2.0") {
        return Ok(Some(js::undefined()));
    }
    let params = member_opt(&data, "params");
    let fields = if matches!(params, Value::Object(_) | Value::Array(_)) {
        params
    } else {
        Value::Object(serde_json::Map::new())
    };
    let method = member_opt(&data, "method");
    let method = match method {
        Value::String(s) => s,
        _ => return Ok(Some(js::undefined())),
    };
    match method.as_str() {
        m if m == SIZE_CHANGED_METHOD => {
            let height = object_member(&fields, "height");
            let h = js::as_number(&height);
            match h {
                Some(h) if h.is_finite() && h > 0.0 => {
                    Ok(Some(json!({"type": "size", "height": js::number(h)})))
                }
                _ => Ok(Some(js::undefined())),
            }
        }
        m if m == OPEN_LINK_METHOD => {
            let id = member_opt(&data, "id");
            let id_ok = matches!(id, Value::String(_)) || js::as_number(&id).is_some();
            let url = object_member(&fields, "url");
            let url = match url {
                Value::String(s) => s,
                _ => return Ok(Some(js::undefined())),
            };
            if !id_ok || utf16::units(&url).len() > MAX_LINK_LENGTH_UNITS {
                return Ok(Some(js::undefined()));
            }
            match parse_url(&url) {
                Some((scheme, href))
                    if (scheme == "http" || scheme == "https") && !href.is_empty() =>
                {
                    Ok(Some(json!({"type": "open-link", "id": id, "url": href})))
                }
                _ => Ok(Some(js::undefined())),
            }
        }
        m if m == STOPPED_METHOD => Ok(Some(json!({"type": "stopped"}))),
        m if m == SCROLL_INTENT_METHOD => {
            let upward = object_member(&fields, "upward");
            match upward {
                Value::Bool(b) => Ok(Some(json!({"type": "scroll-intent", "upward": b}))),
                _ => Ok(Some(js::undefined())),
            }
        }
        _ => Ok(Some(js::undefined())),
    }
}

/// Port of `inlineVisualLinkResult` (`packages/shared/src/inlineVisual.ts:134`).
pub fn inline_visual_link_result(id: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(json!({"jsonrpc": "2.0", "id": id, "result": {}}))
}

/// Port of `inlineVisualThemeMessage` (`packages/shared/src/inlineVisual.ts:139`).
pub fn inline_visual_theme_message(theme: serde_json::Value) -> Result<serde_json::Value, String> {
    let appearance = member(&theme, "appearance")?;
    let variables = member(&theme, "variables")?;
    Ok(json!({
        "jsonrpc": "2.0",
        "method": HOST_CONTEXT_CHANGED_METHOD,
        "params": {"theme": appearance, "styles": {"variables": variables}},
    }))
}

fn valid_variable_name(name: &str) -> bool {
    name.len() > 2
        && name.starts_with("--")
        && name[2..]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn theme_css(theme: &Value) -> Result<String, String> {
    let appearance = member(theme, "appearance")?;
    let scheme = if matches!(&appearance, Value::String(s) if s == "light") {
        "light"
    } else {
        "dark"
    };
    let variables = member(theme, "variables")?;
    let mut declarations = format!("color-scheme:{scheme};");
    for (name, value) in object_entries(&variables)? {
        if !valid_variable_name(&name) {
            continue;
        }
        let v: String = js_to_string(&value)
            .chars()
            .filter(|c| !matches!(c, ';' | '{' | '}' | '<' | '>'))
            .collect();
        declarations.push_str(&format!("{name}:{v};"));
    }
    Ok(format!(":root{{{declarations}}}{BASE_CSS}"))
}

/// Port of `buildInlineVisualDocument` (`packages/shared/src/inlineVisual.ts:219`).
pub fn build_inline_visual_document(
    html: String,
    theme: serde_json::Value,
) -> Result<String, String> {
    let style = theme_css(&theme)?;
    // /^﻿?\s*<!doctype[^>]*>/i
    let doctype = Regex::new(&format!(
        "^\\u{{feff}}?{JS_SPC}*{}{}[^>]*>",
        ai("<!"),
        ai("doctype")
    ))
    .map_err(|e| e.to_string())?;
    let body = doctype.replace(&html, "");
    Ok([
        "<!doctype html>".to_string(),
        format!("<meta http-equiv=\"Content-Security-Policy\" content=\"{CSP}\">"),
        "<meta http-equiv=\"x-dns-prefetch-control\" content=\"off\">".to_string(),
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">".to_string(),
        format!("<style id=\"{THEME_STYLE_ID}\">{style}</style>"),
        format!("<script>{BOOTSTRAP_SCRIPT}</script>"),
        body.into_owned(),
    ]
    .join(""))
}

fn ai(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                format!("[{}{}]", c.to_ascii_lowercase(), c.to_ascii_uppercase())
            } else {
                regex::escape(&c.to_string())
            }
        })
        .collect()
}

/// Port of `readInlineVisualEmbedId` (`packages/shared/src/inlineVisual.ts:232`).
pub fn read_inline_visual_embed_id(body: String) -> Result<Option<String>, String> {
    let id = js_trim(&body);
    let ok = id.len() == 36
        && id.starts_with("vis_")
        && id[4..]
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'));
    Ok(ok.then(|| id.to_string()))
}

/// Port of `readInlineVisualEmbedIds` (`packages/shared/src/inlineVisual.ts:245`).
pub fn read_inline_visual_embed_ids(text: String) -> Result<Vec<String>, String> {
    if !text.contains(EMBED_FENCE) {
        return Ok(Vec::new());
    }
    // JS multiline `^`/`$` anchor around \n, \r, U+2028 and U+2029: consume the
    // terminator where Rust's `(?m)` cannot, and fold the close-fence `^` into
    // group 2 (it must end on a terminator, or be empty right after the open).
    let pattern = format!(
        "(?:\\A|\\r\\n|[\\n\\r\\u{{2028}}\\u{{2029}}]) {{0,3}}(`{{3,}}|~{{3,}})[ \\t]*{fence}[ \\t]*\\r?\\n([\\s\\S]*?[\\n\\r\\u{{2028}}\\u{{2029}}]|) {{0,3}}(`{{3,}}|~{{3,}})[ \\t]*(?:\\r\\n|[\\n\\r\\u{{2028}}\\u{{2029}}]|\\z)",
        fence = EMBED_FENCE
    );
    let re = Regex::new(&pattern).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for caps in re.captures_iter(&text) {
        let (Some(open), Some(body), Some(close)) = (caps.get(1), caps.get(2), caps.get(3)) else {
            continue;
        };
        if !close.as_str().starts_with(open.as_str()) {
            continue;
        }
        let id = js_trim(body.as_str());
        if id.len() == 36
            && id.starts_with("vis_")
            && id[4..]
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
        {
            out.push(id.to_string());
        }
    }
    Ok(out)
}
