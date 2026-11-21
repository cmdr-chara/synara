//! The Rust port of `packages/shared/src/providerUsage.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

/// (kind, displayName, signInCommand, learnMoreHref) in display order; `omp` is absent
/// because it has no usage source.
const USAGE_PROVIDERS: &[(&str, &str, &str, &str)] = &[
    (
        "codex",
        "Codex",
        "codex login",
        "https://platform.openai.com/usage",
    ),
    (
        "claudeAgent",
        "Claude",
        "claude",
        "https://docs.anthropic.com/en/docs/about-claude/models#rate-limits",
    ),
    (
        "cursor",
        "Cursor",
        "cursor-agent login",
        "https://cursor.com/dashboard",
    ),
    (
        "antigravity",
        "Antigravity",
        "agy",
        "https://antigravity.google",
    ),
    ("grok", "Grok", "grok login", "https://console.x.ai"),
    ("droid", "Droid", "droid", "https://docs.factory.ai/pricing"),
    (
        "opencode",
        "OpenCode",
        "opencode auth login",
        "https://opencode.ai",
    ),
    ("pi", "Pi", "pi", "https://pi.dev"),
    (
        "devin",
        "Devin",
        "devin auth login",
        "https://app.devin.ai/usage",
    ),
];

/// JS `ToString` for the property-key lookup (`PROVIDER_DESCRIPTOR_BY_KIND[provider]`).
fn to_key(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Array(a) => a
            .iter()
            .map(|x| match x {
                Value::Null => String::new(),
                _ if crate::js::is_undefined(x) => String::new(),
                _ => to_key(x),
            })
            .collect::<Vec<_>>()
            .join(","),
        _ if crate::js::as_number(v).is_some() => {
            crate::js::number_to_string(crate::js::as_number(v).unwrap_or(f64::NAN))
        }
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => crate::js::string_units(v)
            .and_then(|u| String::from_utf16(&u).ok())
            .unwrap_or_default(),
        _ => "[object Object]".to_string(),
    }
}

fn js_falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::String(s) => s.is_empty(),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN) == 0.0,
        _ if crate::js::is_undefined(v) => true,
        _ if crate::js::as_number(v).is_some() => {
            crate::js::as_number(v).unwrap_or(f64::NAN) == 0.0
        }
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            crate::js::string_units(v).is_none_or(|u| u.is_empty())
        }
        _ => false,
    }
}

/// `lookupMeta`: falsy provider or a kind without a usage source resolves to no descriptor.
fn lookup_meta(
    provider: &Value,
) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    if js_falsy(provider) {
        return None;
    }
    let key = to_key(provider);
    USAGE_PROVIDERS.iter().find(|d| d.0 == key)
}

/// Port of `providerUsageLabel` (`packages/shared/src/providerUsage.ts:27`).
pub fn provider_usage_label(provider: serde_json::Value) -> Result<String, String> {
    Ok(match lookup_meta(&provider) {
        Some(meta) => format!("{} usage", meta.1),
        None => "Usage".to_string(),
    })
}

/// Port of `providerUsageDisplayName` (`packages/shared/src/providerUsage.ts:32`).
pub fn provider_usage_display_name(provider: serde_json::Value) -> Result<String, String> {
    Ok(lookup_meta(&provider)
        .map(|m| m.1)
        .unwrap_or("Provider")
        .to_string())
}

/// Port of `providerUsageLearnMoreHref` (`packages/shared/src/providerUsage.ts:36`).
pub fn provider_usage_learn_more_href(
    provider: serde_json::Value,
) -> Result<Option<String>, String> {
    Ok(lookup_meta(&provider).map(|m| m.3.to_string()))
}

/// Port of `providerUsageNeedsAuthDetail` (`packages/shared/src/providerUsage.ts:41`).
pub fn provider_usage_needs_auth_detail(provider: serde_json::Value) -> Result<String, String> {
    Ok(match lookup_meta(&provider) {
        Some(meta) => format!("Sign in with `{}` to see usage.", meta.2),
        None => "Sign in with the provider CLI to see usage.".to_string(),
    })
}

fn provider_of(snapshot: &Value) -> Result<Option<&str>, String> {
    match snapshot {
        Value::Null => Err("Cannot read properties of null (reading 'provider')".to_string()),
        _ if crate::js::is_undefined(snapshot) => {
            Err("Cannot read properties of undefined (reading 'provider')".to_string())
        }
        Value::Object(o) => match o.get("provider") {
            Some(Value::String(s)) => Ok(Some(s.as_str())),
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

/// Port of `selectVisibleProviderUsageSnapshots` (`packages/shared/src/providerUsage.ts:54`).
pub fn select_visible_provider_usage_snapshots(
    snapshots: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let list = match &snapshots {
        Value::Array(a) => a,
        Value::Null => {
            return Err("Cannot read properties of null (reading 'filter')".to_string());
        }
        _ if crate::js::is_undefined(&snapshots) => {
            return Err("Cannot read properties of undefined (reading 'filter')".to_string());
        }
        _ => return Err("snapshots.filter is not a function".to_string()),
    };
    let mut ordered: Vec<Value> = Vec::new();
    for (kind, _, _, _) in USAGE_PROVIDERS {
        for snap in list {
            if provider_of(snap)? == Some(*kind) {
                ordered.push(snap.clone());
            }
        }
    }
    let connected: Vec<Value> = ordered
        .iter()
        .filter(|s| {
            !matches!(
                s.get("status"),
                Some(Value::String(st)) if st == "needs-auth"
            )
        })
        .cloned()
        .collect();
    Ok(if connected.is_empty() {
        ordered
    } else {
        connected
    })
}
