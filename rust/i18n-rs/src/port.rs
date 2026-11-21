//! The Rust port of `apps/desktop/src/i18n.ts`.

#![forbid(unsafe_code)]

use std::sync::OnceLock;

use serde_json::Value;

use crate::js;

// `packages/shared/src/locales/en/desktop.json` — the English `desktop`
// namespace is the only catalog the interface reads.
const CATALOG_JSON: &str = include_str!("catalog.json");

fn catalog() -> &'static serde_json::Map<String, Value> {
    static CATALOG: OnceLock<serde_json::Map<String, Value>> = OnceLock::new();
    CATALOG.get_or_init(|| match serde_json::from_str::<Value>(CATALOG_JSON) {
        Ok(Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    })
}

// `isUiMessageKey`: `^([a-z][A-Za-z0-9_-]*):[A-Za-z0-9_][A-Za-z0-9_.-]*$` and the
// namespace must exist in the resources (only `desktop` does).
fn message_key(message: &str) -> Option<&str> {
    let (ns, key) = message.split_once(':')?;
    let mut ns_chars = ns.chars();
    if !ns_chars.next().is_some_and(|c| c.is_ascii_lowercase())
        || !ns_chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        return None;
    }
    let mut key_chars = key.chars();
    if !key_chars
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        || !key_chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return None;
    }
    if ns != "desktop" {
        return None;
    }
    Some(key)
}

// `{...values}` in the call site: only objects, arrays and strings spread into
// index-keyed props; everything else contributes nothing.
fn spread_get(values: &Value, key: &str) -> Option<Value> {
    match values {
        Value::Object(m) => m.get(key).cloned(),
        Value::Array(a) => key.parse::<usize>().ok().and_then(|i| a.get(i)).cloned(),
        Value::String(s) => key
            .parse::<usize>()
            .ok()
            .and_then(|i| s.chars().nth(i))
            .map(|c| Value::String(c.to_string())),
        _ => None,
    }
}

fn coerce(v: &Value) -> String {
    // `js::as_number` also unwraps `{"$num": ...}` markers (NaN/±Infinity/-0).
    if let Some(n) = js::as_number(v) {
        return js::number_to_string(n);
    }
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(coerce).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn interpolate(template: &str, values: Option<&Value>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(i) = rest.find("{{") {
        let Some(j) = rest[i + 2..].find("}}") else {
            break;
        };
        out.push_str(&rest[..i]);
        let inner = &rest[i + 2..i + 2 + j];
        match values.and_then(|v| spread_get(v, inner.trim())) {
            Some(v) if js::is_undefined(&v) || v.is_null() => {}
            Some(v) => out.push_str(&coerce(&v)),
            None => {
                out.push_str("{{");
                out.push_str(inner);
                out.push_str("}}");
            }
        }
        rest = &rest[i + 2 + j + 2..];
    }
    out.push_str(rest);
    out
}

/// Port of `t` (`apps/desktop/src/i18n.ts:20`).
pub fn t(message: String, values: Option<serde_json::Value>) -> Result<String, String> {
    let Some(key) = message_key(&message) else {
        return Ok(message);
    };
    let values = values.filter(|v| !js::is_undefined(v));
    match catalog().get(key) {
        Some(Value::String(template)) => Ok(interpolate(template, values.as_ref())),
        // i18next returns the key (without the namespace) when it misses.
        _ => Ok(key.to_string()),
    }
}
