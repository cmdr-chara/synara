//! The Rust port of `apps/desktop/src/githubUpdateFeed.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if js::is_undefined(v) {
        return None;
    }
    v.as_object().and_then(|o| o.get(key))
}

fn is_js_ws(c: char) -> bool {
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
    s.trim_matches(is_js_ws)
}

/// `value?.trim()`: `undefined`/`null` on the left short-circuit to
/// `undefined`; a defined non-string hits a missing-method TypeError.
fn opt_trim(v: Option<&Value>, obj: &str, key: &str) -> Result<Option<String>, String> {
    match v {
        None => Ok(None),
        Some(v) if v.is_null() || js::is_undefined(v) => Ok(None),
        Some(Value::String(s)) => Ok(Some(js_trim(s).to_string())),
        Some(_) => Err(format!("{obj}.{key}?.trim is not a function")),
    }
}

/// JS ToString for `${...}` template interpolation.
fn to_string(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(v) => match v {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
            Value::Array(a) => a
                .iter()
                .map(|x| {
                    if x.is_null() || js::is_undefined(x) {
                        String::new()
                    } else {
                        to_string(Some(x))
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            Value::Object(_) if js::is_undefined(v) => "undefined".to_string(),
            Value::Object(_) => "[object Object]".to_string(),
        },
    }
}

/// Port of `resolveGitHubUpdateSource` (`apps/desktop/src/githubUpdateFeed.ts:17`).
pub fn resolve_git_hub_update_source(raw_config: Option<Value>) -> Result<Value, String> {
    let raw_config = raw_config.as_ref().unwrap_or(&Value::Null);
    if prop(raw_config, "provider").and_then(Value::as_str) != Some("github") {
        return Ok(Value::Null);
    }
    let owner = opt_trim(prop(raw_config, "owner"), "rawConfig", "owner")?;
    let repo = opt_trim(prop(raw_config, "repo"), "rawConfig", "repo")?;
    let (owner, repo) = match (owner, repo) {
        (Some(o), Some(r)) if !o.is_empty() && !r.is_empty() => (o, r),
        _ => return Ok(Value::Null),
    };
    let host = opt_trim(prop(raw_config, "host"), "rawConfig", "host")?
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "github.com".to_string());
    let protocol = opt_trim(prop(raw_config, "protocol"), "rawConfig", "protocol")?;
    let protocol = if protocol.as_deref() == Some("http") {
        "http"
    } else {
        "https"
    };
    Ok(json!({
        "owner": owner,
        "repo": repo,
        "host": host,
        "protocol": protocol,
    }))
}

/// Port of `buildGitHubReleasesPageUrl` (`apps/desktop/src/githubUpdateFeed.ts:39`).
pub fn build_git_hub_releases_page_url(
    source: Value,
    tag: Option<String>,
) -> Result<String, String> {
    if source.is_null() {
        return Err("Cannot read properties of null (reading 'owner')".to_string());
    }
    let path = match tag.as_deref().map(js_trim).filter(|t| !t.is_empty()) {
        Some(tag) => format!(
            "/{}/{}/releases/tag/{tag}",
            to_string(prop(&source, "owner")),
            to_string(prop(&source, "repo")),
        ),
        None => format!(
            "/{}/{}/releases/latest",
            to_string(prop(&source, "owner")),
            to_string(prop(&source, "repo")),
        ),
    };
    let base = format!(
        "{}://{}",
        to_string(prop(&source, "protocol")),
        to_string(prop(&source, "host")),
    );
    let base = url::Url::parse(&base).map_err(|_| "Invalid URL".to_string())?;
    let resolved = base.join(&path).map_err(|_| "Invalid URL".to_string())?;
    Ok(resolved.to_string())
}
