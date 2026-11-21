//! The Rust port of `packages/shared/src/mcpConnectors.ts`.

#![forbid(unsafe_code)]

mod url;

use crate::js;
use serde_json::{Map, Value, json};
use unicode_normalization::UnicodeNormalization;

const MAX_URL_LENGTH: usize = 2048;
const MAX_NAME_LENGTH: usize = 60;
const MAX_SLUG_LENGTH: usize = 24;
const SERVER_NAME_SUFFIX: &str = "_connector";
const MENTION_PREFIX: &str = "connector://";

fn presets() -> &'static [Value] {
    use std::sync::OnceLock;
    static PRESETS: OnceLock<Vec<Value>> = OnceLock::new();
    PRESETS.get_or_init(|| serde_json::from_str(include_str!("presets.json")).unwrap_or_default())
}

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\u{b}'
            | '\u{c}'
            | '\r'
            | '\u{a0}'
            | '\u{feff}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

fn js_trim(s: &str) -> String {
    s.trim_matches(is_js_space).to_string()
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

fn utf16_slice(s: &str, n: usize) -> String {
    let mut units = 0;
    let mut out = String::new();
    for c in s.chars() {
        if units + c.len_utf16() > n {
            break;
        }
        units += c.len_utf16();
        out.push(c);
    }
    out
}

fn fail(reason: &str, code: &str) -> Value {
    json!({ "ok": false, "reason": reason, "reasonCode": code })
}

/// Port of `findMcpConnectorPreset` (`packages/shared/src/mcpConnectors.ts:195`).
pub fn find_mcp_connector_preset(id: String) -> Result<serde_json::Value, String> {
    Ok(presets()
        .iter()
        .find(|p| p.get("id").and_then(Value::as_str) == Some(id.as_str()))
        .cloned()
        .unwrap_or_else(js::undefined))
}

/// Port of `isMcpLoopbackHostname` (`packages/shared/src/mcpConnectors.ts:227`).
pub fn is_mcp_loopback_hostname(hostname: String) -> Result<bool, String> {
    if hostname == "localhost" || hostname == "[::1]" {
        return Ok(true);
    }
    let parts: Vec<&str> = hostname.split('.').collect();
    Ok(parts.len() == 4
        && parts[0] == "127"
        && parts[1..].iter().all(|seg| {
            !seg.is_empty() && seg.len() <= 3 && seg.bytes().all(|b| b.is_ascii_digit())
        }))
}

/// Port of `normalizeMcpConnectorUrl` (`packages/shared/src/mcpConnectors.ts:234`).
pub fn normalize_mcp_connector_url(raw: String) -> Result<serde_json::Value, String> {
    let trimmed = js_trim(&raw);
    if trimmed.is_empty() {
        return Ok(fail("Enter the server URL.", "required"));
    }
    if utf16_len(&trimmed) > MAX_URL_LENGTH {
        return Ok(fail("The URL is too long.", "too-long"));
    }
    let Some(parsed) = url::parse_url(&trimmed) else {
        return Ok(fail(
            "Enter a full URL, like https://mcp.example.com/mcp.",
            "invalid",
        ));
    };
    let local = parsed.scheme == "http" && is_mcp_loopback_hostname(parsed.hostname().to_string())?;
    if parsed.scheme != "https" && !local {
        return Ok(fail(
            "Use https:// for public servers. HTTP is allowed only for loopback hosts (localhost, 127.0.0.0/8, or ::1).",
            "https-required",
        ));
    }
    if parsed.userinfo.as_deref().is_some_and(|u| !u.is_empty()) {
        return Ok(fail(
            "Remove the user name or password from the URL.",
            "credentials",
        ));
    }
    if !local && !parsed.hostname().contains('.') {
        return Ok(fail("Use the server's public address.", "public-host"));
    }
    let mut parsed = parsed;
    parsed.fragment = None;
    let mut out = Map::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("url".into(), Value::String(parsed.to_js_string()));
    Ok(Value::Object(out))
}

/// Port of `defaultMcpConnectorName` (`packages/shared/src/mcpConnectors.ts:274`).
pub fn default_mcp_connector_name(url: String) -> Result<String, String> {
    let parsed = url::parse_url(&url).ok_or_else(|| "Invalid URL".to_string())?;
    let host = parsed.hostname();
    let host = host
        .strip_prefix("mcp.")
        .or_else(|| host.strip_prefix("api."))
        .or_else(|| host.strip_prefix("www."))
        .unwrap_or(host);
    Ok(utf16_slice(host, MAX_NAME_LENGTH))
}

/// Port of `mcpConnectorSlug` (`packages/shared/src/mcpConnectors.ts:284`).
pub fn mcp_connector_slug(name: String) -> Result<String, String> {
    let normalized: String = name.nfkd().collect::<String>().to_lowercase();
    let mut slug = String::new();
    let mut sep = false;
    for c in normalized.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
            sep = false;
        } else if !slug.is_empty() && !sep {
            slug.push('_');
            sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if !slug.starts_with(|c: char| c.is_ascii_lowercase()) {
        slug = format!("mcp_{slug}");
        while slug.ends_with('_') {
            slug.pop();
        }
    }
    if slug.starts_with("synara") {
        slug = format!("custom_{slug}");
    }
    let slug = utf16_slice(&slug, MAX_SLUG_LENGTH);
    let mut slug = slug.as_str();
    while let Some(rest) = slug.strip_suffix('_') {
        slug = rest;
    }
    Ok(slug.to_string())
}

/// Port of `mcpConnectorServerName` (`packages/shared/src/mcpConnectors.ts:296`).
pub fn mcp_connector_server_name(slug: String) -> Result<String, String> {
    Ok(format!("{slug}{SERVER_NAME_SUFFIX}"))
}

/// Port of `mcpConnectorMentionPath` (`packages/shared/src/mcpConnectors.ts:303`).
pub fn mcp_connector_mention_path(server_name: String) -> Result<String, String> {
    Ok(format!("{MENTION_PREFIX}{server_name}"))
}

/// Port of `isMcpConnectorMentionPath` (`packages/shared/src/mcpConnectors.ts:307`).
pub fn is_mcp_connector_mention_path(path: String) -> Result<bool, String> {
    Ok(path.starts_with(MENTION_PREFIX))
}

/// Port of `serverNameFromMcpConnectorMentionPath` (`packages/shared/src/mcpConnectors.ts:312`).
pub fn server_name_from_mcp_connector_mention_path(
    path: String,
) -> Result<serde_json::Value, String> {
    let Some(name) = path.strip_prefix(MENTION_PREFIX) else {
        return Ok(Value::Null);
    };
    let valid = name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    Ok(if valid {
        Value::String(name.to_string())
    } else {
        Value::Null
    })
}

/// Port of `providerSupportsMcpConnectors` (`packages/shared/src/mcpConnectors.ts:335`).
pub fn provider_supports_mcp_connectors(provider: String) -> Result<bool, String> {
    Ok(matches!(
        provider.as_str(),
        "claudeAgent" | "codex" | "opencode" | "cursor" | "grok" | "droid" | "omp" | "antigravity"
    ))
}
