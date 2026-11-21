#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value};
mod url;
use url::{encode_uri_component, parse_url};

const BLANK: &str = "about:blank";
const SEARCH: &str = "https://www.google.com/search?q=";
const OAUTH_HOSTS: [&str; 7] = [
    "accounts.google.com",
    "accounts.youtube.com",
    "appleid.apple.com",
    "login.microsoftonline.com",
    "login.live.com",
    "auth0.com",
    "okta.com",
];

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
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => !js::is_undefined(v),
    }
}
fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}
fn read_member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(member(v, key))
}
fn opt_trimmed(v: &Value, key: &str, expr: &str) -> Result<String, String> {
    match member(v, key) {
        Value::String(s) => Ok(js_trim(&s).to_string()),
        Value::Null => Ok(String::new()),
        m if js::is_undefined(&m) => Ok(String::new()),
        _ => Err(format!("{expr}.trim is not a function")),
    }
}
fn trimmed(v: &Value, key: &str, expr: &str) -> Result<String, String> {
    match read_member(v, key)? {
        Value::String(s) => Ok(js_trim(&s).to_string()),
        Value::Null => Err("Cannot read properties of null (reading 'trim')".into()),
        m if js::is_undefined(&m) => {
            Err("Cannot read properties of undefined (reading 'trim')".into())
        }
        _ => Err(format!("{expr}.trim is not a function")),
    }
}

/// Port of `resolveFloatingBrowserGuestLayout` (`packages/shared/src/browserSession.ts:31`).
pub fn resolve_floating_browser_guest_layout(
    slot: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let w = match js::as_number(&read_member(&slot, "width")?) {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => 1.0,
    };
    let h = match js::as_number(&read_member(&slot, "height")?) {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => 1.0,
    };
    let scale = (w / 1280.0).min(h / 800.0).min(1.0);
    let x = js::math_round((w - 1280.0 * scale) / 2.0).max(0.0);
    let y = js::math_round((h - 800.0 * scale) / 2.0).max(0.0);
    let mut out = Map::new();
    out.insert("width".into(), js::number(1280.0));
    out.insert("height".into(), js::number(800.0));
    out.insert("scale".into(), js::number(scale));
    out.insert("x".into(), js::number(x));
    out.insert("y".into(), js::number(y));
    Ok(Value::Object(out))
}

/// Port of `normalizeBrowserPageZoomFactor` (`packages/shared/src/browserSession.ts:49`).
pub fn normalize_browser_page_zoom_factor(value: serde_json::Value) -> Result<f64, String> {
    Ok(match js::as_number(&value) {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => 1.0,
    })
}

/// Port of `normalizeBrowserUrlInput` (`packages/shared/src/browserSession.ts:84`).
pub fn normalize_browser_url_input(input: Option<String>) -> Result<String, String> {
    let trimmed = js_trim(input.as_deref().unwrap_or(""));
    if trimmed.is_empty() {
        return Ok(BLANK.into());
    }
    if let Some(u) = parse_url(trimmed)
        && matches!(u.scheme.as_str(), "http" | "https" | "about" | "file")
    {
        return Ok(u.to_js_string());
    }
    if trimmed.contains(' ') {
        return Ok(format!("{SEARCH}{}", encode_uri_component(trimmed)));
    }
    let local = trimmed.starts_with("localhost")
        || trimmed.starts_with("127.0.0.1")
        || trimmed.starts_with("0.0.0.0")
        || trimmed.starts_with("[::1]");
    if local || trimmed.contains('.') {
        let scheme = if local { "http" } else { "https" };
        if let Some(u) = parse_url(&format!("{scheme}://{trimmed}")) {
            return Ok(u.to_js_string());
        }
    }
    Ok(format!("{SEARCH}{}", encode_uri_component(trimmed)))
}

/// Port of `resolveCopyableBrowserTabUrl` (`packages/shared/src/browserSession.ts:132`).
pub fn resolve_copyable_browser_tab_url(
    tab: Option<serde_json::Value>,
    live_url: Option<String>,
) -> Result<Option<String>, String> {
    let live = js_trim(live_url.as_deref().unwrap_or(""));
    if !live.is_empty() && live != BLANK {
        return Ok(Some(live.into()));
    }
    let read = |key: &str, expr: &str| match &tab {
        Some(t) => opt_trimmed(t, key, expr),
        None => Ok(String::new()),
    };
    let committed = read("lastCommittedUrl", "tab.lastCommittedUrl")?;
    if !committed.is_empty() && committed != BLANK {
        return Ok(Some(committed));
    }
    let current = read("url", "tab.url")?;
    Ok((!current.is_empty() && current != BLANK).then_some(current))
}

/// Port of `isBlankBrowserTabUrl` (`packages/shared/src/browserSession.ts:150`).
pub fn is_blank_browser_tab_url(tab: Option<serde_json::Value>) -> Result<bool, String> {
    let tab = match tab {
        Some(t) if truthy(&t) => t,
        _ => return Ok(true),
    };
    let current = opt_trimmed(&tab, "url", "tab.url")?;
    let committed = opt_trimmed(&tab, "lastCommittedUrl", "tab.lastCommittedUrl")?;
    Ok((current.is_empty() || current == BLANK) && (committed.is_empty() || committed == BLANK))
}

fn is_likely_oauth_url(url: &str) -> bool {
    let parsed = match parse_url(url) {
        Some(p) => p,
        None => return false,
    };
    let host = parsed.hostname().to_lowercase();
    let path = parsed.path.to_lowercase();
    let normalized = js_trim(&host);
    if !normalized.is_empty()
        && OAUTH_HOSTS
            .iter()
            .any(|h| normalized == *h || normalized.ends_with(&format!(".{h}")))
    {
        return true;
    }
    let under = |suffix: &str| host == suffix[1..] || host.ends_with(suffix);
    if host == "github.com" {
        return path == "/login/oauth/authorize";
    }
    if under(".gitlab.com") {
        return path == "/oauth/authorize";
    }
    if under(".facebook.com") {
        if path == "/dialog/oauth" {
            return true;
        }
        if let Some(rest) = path.strip_prefix("/v")
            && let Some((major, tail)) = rest.split_once('.')
            && let Some(minor) = tail.strip_suffix("/dialog/oauth")
        {
            return !major.is_empty()
                && !minor.is_empty()
                && major.bytes().all(|b| b.is_ascii_digit())
                && minor.bytes().all(|b| b.is_ascii_digit());
        }
        return false;
    }
    if under(".slack.com") {
        return path == "/oauth/v2/authorize" || path == "/openid/connect/authorize";
    }
    if under(".discord.com") {
        return path == "/oauth2/authorize" || path == "/api/oauth2/authorize";
    }
    if under(".linkedin.com") {
        return path == "/oauth/v2/authorization";
    }
    false
}

/// Port of `classifyBrowserWindowOpen` (`packages/shared/src/browserSession.ts:208`).
pub fn classify_browser_window_open(intent: serde_json::Value) -> Result<String, String> {
    let url = trimmed(&intent, "url", "intent.url")?;
    if url.to_lowercase() == BLANK {
        return Ok("popup".into());
    }
    if !trimmed(&intent, "features", "intent.features")?.is_empty() {
        return Ok("popup".into());
    }
    let frame = trimmed(&intent, "frameName", "intent.frameName")?;
    if !matches!(
        frame.to_lowercase().as_str(),
        "" | "_blank" | "_self" | "_parent" | "_top"
    ) {
        return Ok("popup".into());
    }
    if let Value::String(raw) = member(&intent, "url")
        && is_likely_oauth_url(&raw)
    {
        return Ok("popup".into());
    }
    Ok("tab".into())
}

fn remove_ua_token(ua: &str, literal: &str) -> String {
    let lower: Vec<char> = literal.chars().flat_map(|c| c.to_lowercase()).collect();
    let chars: Vec<char> = ua.chars().collect();
    let mut out = String::with_capacity(ua.len());
    let mut i = 0;
    while i < chars.len() {
        if is_js_space(chars[i]) {
            let mut k = i + 1;
            let mut ok = true;
            for want in &lower {
                if k >= chars.len() || chars[k].to_lowercase().next() != Some(*want) {
                    ok = false;
                    break;
                }
                k += 1;
            }
            if ok && k < chars.len() && chars[k] == '/' {
                let mut end = k + 1;
                while end < chars.len() && !is_js_space(chars[end]) {
                    end += 1;
                }
                if end > k + 1 {
                    i = end;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Port of `deriveChromeUserAgent` (`packages/shared/src/browserSession.ts:237`).
pub fn derive_chrome_user_agent(
    base_user_agent: String,
    app_product_tokens: Option<Vec<String>>,
) -> Result<String, String> {
    let mut ua = remove_ua_token(&base_user_agent, "Electron");
    for token in app_product_tokens.unwrap_or_default() {
        let t = js_trim(&token);
        if !t.is_empty() {
            ua = remove_ua_token(&ua, t);
        }
    }
    let mut collapsed = String::with_capacity(ua.len());
    let mut run = 0usize;
    let mut last_space = ' ';
    for c in ua.chars() {
        if is_js_space(c) {
            run += 1;
            last_space = c;
        } else {
            if run > 0 {
                collapsed.push(if run > 1 { ' ' } else { last_space });
            }
            run = 0;
            collapsed.push(c);
        }
    }
    Ok(js_trim(&collapsed).to_string())
}

fn chrome_major_version(user_agent: &str) -> Option<String> {
    let chars: Vec<char> = user_agent.chars().collect();
    let needle = ['c', 'h', 'r', 'o', 'm', 'e', '/'];
    if chars.len() < needle.len() {
        return None;
    }
    for i in 0..=chars.len() - needle.len() {
        if needle
            .iter()
            .enumerate()
            .all(|(j, n)| chars[i + j].to_lowercase().next() == Some(*n))
        {
            let start = i + needle.len();
            let mut end = start;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            return (end > start).then(|| chars[start..end].iter().collect());
        }
    }
    None
}

/// Port of `chromeClientHintPlatform` (`packages/shared/src/browserSession.ts:258`).
pub fn chrome_client_hint_platform(platform: String) -> Result<String, String> {
    Ok(match platform.as_str() {
        "darwin" => "macOS",
        "win32" => "Windows",
        _ => "Linux",
    }
    .into())
}

/// Port of `buildChromeClientHints` (`packages/shared/src/browserSession.ts:279`).
pub fn build_chrome_client_hints(
    user_agent: String,
    platform: String,
) -> Result<Option<serde_json::Value>, String> {
    let major = match chrome_major_version(&user_agent) {
        Some(m) => m,
        None => return Ok(None),
    };
    let mut out = Map::new();
    out.insert(
        "sec-ch-ua".into(),
        Value::String(format!(
            "\"Chromium\";v=\"{major}\", \"Google Chrome\";v=\"{major}\", \"Not=A?Brand\";v=\"24\""
        )),
    );
    out.insert("sec-ch-ua-mobile".into(), Value::String("?0".into()));
    out.insert(
        "sec-ch-ua-platform".into(),
        Value::String(format!("\"{}\"", chrome_client_hint_platform(platform)?)),
    );
    Ok(Some(Value::Object(out)))
}

/// Port of `buildAcceptLanguageHeader` (`packages/shared/src/browserSession.ts:296`).
pub fn build_accept_language_header(languages: Vec<String>) -> Result<Option<String>, String> {
    let normalized: Vec<String> = languages
        .iter()
        .map(|l| js_trim(l).to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if normalized.is_empty() {
        return Ok(None);
    }
    let parts: Vec<String> = normalized
        .iter()
        .enumerate()
        .map(|(i, language)| {
            if i == 0 {
                language.clone()
            } else {
                format!("{language};q={:.1}", (1.0 - i as f64 * 0.1).max(0.1))
            }
        })
        .collect();
    Ok(Some(parts.join(",")))
}
