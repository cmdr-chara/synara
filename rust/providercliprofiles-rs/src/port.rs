//! The Rust port of `packages/shared/src/providerCliProfiles.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const PROVIDER_CLI_BINARIES: &[(&str, &str)] = &[
    ("claudeAgent", "claude"),
    ("antigravity", "agy"),
    ("codex", "codex"),
    ("opencode", "opencode"),
    ("droid", "droid"),
    ("devin", "devin"),
    ("grok", "grok"),
    ("pi", "pi"),
    ("cursor", "cursor-agent"),
];

const CANONICAL_ALIASES: &[&str] = &[
    "claude",
    "codex",
    "opencode",
    "droid",
    "devin",
    "pi",
    "grok",
    "agy",
    "cursor-agent",
];

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn is_valid_alias(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn normalize_alias(alias: &Value) -> Option<String> {
    let s = alias.as_str()?;
    let t = js_trim(s);
    if t.is_empty() || !is_valid_alias(t) || CANONICAL_ALIASES.contains(&t) {
        None
    } else {
        Some(t.to_string())
    }
}

/// Port of `normalizeProviderCliAlias` (`packages/shared/src/providerCliProfiles.ts:23`).
pub fn normalize_provider_cli_alias(value: serde_json::Value) -> Result<Option<String>, String> {
    Ok(normalize_alias(&value))
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::String(x), Value::String(y)) => x == y,
        _ => crate::js::is_undefined(a) && crate::js::is_undefined(b),
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || crate::js::is_undefined(i) {
                    String::new()
                } else {
                    js_to_string(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ if crate::js::is_undefined(v) => "undefined".to_string(),
        _ => "[object Object]".to_string(),
    }
}

fn normalize_suffix(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !(c.is_ascii_alphanumeric()))
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Port of `providerCliCommandName` (`packages/shared/src/providerCliProfiles.ts:51`).
pub fn provider_cli_command_name(input: serde_json::Value) -> Result<String, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'config')".to_string());
    }
    if crate::js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'config')".to_string());
    }
    let config = input.get("config");
    if let Some(alias) = config
        .and_then(|c| c.get("cliAlias"))
        .and_then(normalize_alias)
    {
        return Ok(alias);
    }
    let missing = crate::js::undefined();
    let provider = input.get("provider").unwrap_or(&missing);
    let instance_id = input.get("instanceId").unwrap_or(&missing);
    let binary = provider
        .as_str()
        .and_then(|p| PROVIDER_CLI_BINARIES.iter().find(|(k, _)| *k == p))
        .map(|(_, b)| *b);
    if js_strict_eq(instance_id, provider) {
        return Ok(format!("{}-default", binary.unwrap_or("undefined")));
    }
    match provider {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'toLowerCase')".to_string());
        }
        Value::String(_) => {}
        _ if crate::js::is_undefined(provider) => {
            return Err("Cannot read properties of undefined (reading 'toLowerCase')".to_string());
        }
        _ => return Err("provider.toLowerCase is not a function".to_string()),
    }
    let binary = match binary {
        Some(b) => b,
        None => {
            return Err("Cannot read properties of undefined (reading 'toLowerCase')".to_string());
        }
    };
    let provider_lower = provider.as_str().unwrap_or_default().to_lowercase();
    let s = js_to_string(instance_id);
    let lower = s.to_lowercase();
    let prefixes = [
        format!("{provider_lower}_"),
        format!("{provider_lower}-"),
        format!("{binary}_"),
        format!("{binary}-"),
    ];
    let matched = prefixes
        .iter()
        .filter(|c| lower.starts_with(c.as_str()))
        .max_by_key(|c| c.len());
    let suffix = match matched {
        Some(cand) => {
            let n = normalize_suffix(&s[cand.len()..]);
            if n.is_empty() { s.clone() } else { n }
        }
        None => {
            let n = normalize_suffix(&s);
            if n.is_empty() { s.clone() } else { n }
        }
    };
    Ok(format!("{binary}-{suffix}"))
}
