//! Port of `migrations/automationProviderIdentity.ts`: resolves the account
//! identity carried by pre-instance automation snapshots. Migrations cannot
//! read settings.json, so arbitrary home/env/server overrides are never
//! guessed; ambiguous rows fail closed into a redacted tombstone.

use serde_json::{Map, Value};

pub const PROVIDERS: [&str; 11] = [
    "codex",
    "claudeAgent",
    "cursor",
    "antigravity",
    "gemini",
    "grok",
    "droid",
    "kilo",
    "opencode",
    "pi",
    "devin",
];

const UNRESOLVED_MODEL: &str = "legacy-automation-unresolved";
const UNRESOLVED_PREFIX: &str = "synara_unresolved_automation_";

fn identity_keys(provider: &str) -> &'static [&'static str] {
    match provider {
        "codex" => &["homePath", "shadowHomePath", "environment"],
        "claudeAgent" => &["homePath", "environment"],
        "cursor" => &["apiEndpoint", "environment"],
        "kilo" | "opencode" => &["serverUrl", "serverPassword", "environment"],
        "pi" => &["agentDir", "environment"],
        _ => &["environment"],
    }
}

fn is_provider(value: &str) -> bool {
    PROVIDERS.contains(&value)
}

fn non_empty_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

enum FieldState {
    Absent,
    Present,
    Invalid,
}

fn identity_field_state(options: &Map<String, Value>, key: &str) -> FieldState {
    if !options.contains_key(key) {
        return FieldState::Absent;
    }
    let value = &options[key];
    if key == "environment" {
        return if value.is_object() {
            FieldState::Present
        } else {
            FieldState::Invalid
        };
    }
    if non_empty_string(value).is_none() {
        FieldState::Invalid
    } else {
        FieldState::Present
    }
}

/// Strip the legacy `provider` key and pin the resolved instance id.
fn canonical_selection(selection: &Map<String, Value>, instance_id: &str) -> Map<String, Value> {
    let mut canonical = selection.clone();
    canonical.remove("provider");
    canonical.insert("instanceId".to_string(), Value::String(instance_id.to_string()));
    canonical
}

pub enum IdentityResolution {
    Safe(Map<String, Value>),
    Unsafe,
}

pub fn resolve_automation_provider_identity(
    model_selection: &Value,
    provider_options: Option<&Value>,
    provider_hint: Option<&Value>,
) -> IdentityResolution {
    let selection = match model_selection.as_object() {
        Some(s) if non_empty_string(model_selection.get("model").unwrap_or(&Value::Null)).is_some() => s,
        _ => return IdentityResolution::Unsafe,
    };

    let raw_instance = selection.get("instanceId");
    let instance_id = raw_instance.and_then(non_empty_string);
    if let Some(raw) = raw_instance {
        if !raw.is_null() && raw.as_str() != Some("") && instance_id.is_none() {
            return IdentityResolution::Unsafe;
        }
    }

    if let Some(id) = &instance_id {
        if !is_provider(id) {
            return IdentityResolution::Safe(canonical_selection(selection, id));
        }
    }

    let provider: String = if let Some(id) = &instance_id {
        if is_provider(id) {
            id.clone()
        } else {
            unreachable!()
        }
    } else if let Some(p) = selection.get("provider").and_then(|v| v.as_str()) {
        if is_provider(p) {
            p.to_string()
        } else {
            return IdentityResolution::Unsafe;
        }
    } else {
        return IdentityResolution::Unsafe;
    };

    if let Some(sel_provider) = selection.get("provider") {
        let valid = sel_provider.as_str().map(is_provider).unwrap_or(false)
            && sel_provider.as_str() == Some(provider.as_str());
        if !valid {
            return IdentityResolution::Unsafe;
        }
    }
    if let Some(hint) = provider_hint {
        if !hint.is_null() {
            let valid = hint.as_str().map(is_provider).unwrap_or(false)
                && hint.as_str() == Some(provider.as_str());
            if !valid {
                return IdentityResolution::Unsafe;
            }
        }
    }

    let options_value = match provider_options {
        None => return IdentityResolution::Safe(canonical_selection(selection, &provider)),
        Some(v) if v.is_null() => {
            return IdentityResolution::Safe(canonical_selection(selection, &provider))
        }
        Some(v) => v,
    };
    let options = match options_value.as_object() {
        Some(o) => o,
        None => return IdentityResolution::Unsafe,
    };

    let selected = match options.get(&provider) {
        None => return IdentityResolution::Safe(canonical_selection(selection, &provider)),
        Some(v) if v.is_null() => {
            return IdentityResolution::Safe(canonical_selection(selection, &provider))
        }
        Some(v) => v,
    };
    let selected_options = match selected.as_object() {
        Some(o) => o,
        None => return IdentityResolution::Unsafe,
    };

    if provider == "codex" && selected_options.contains_key("accountId") {
        // Even a well-formed legacy account id cannot prove ownership after
        // settings may have reused its derived id: fail closed either way.
        return IdentityResolution::Unsafe;
    }

    let carries_ambiguous = identity_keys(&provider).iter().any(|key| {
        matches!(
            identity_field_state(selected_options, key),
            FieldState::Present | FieldState::Invalid
        )
    });
    if carries_ambiguous {
        IdentityResolution::Unsafe
    } else {
        IdentityResolution::Safe(canonical_selection(selection, &provider))
    }
}

fn provider_for_unresolved(
    model_selection: &Value,
    provider_options: Option<&Value>,
    provider_hint: Option<&Value>,
) -> String {
    if let Some(hint) = provider_hint.and_then(|v| v.as_str()) {
        if is_provider(hint) {
            return hint.to_string();
        }
    }
    if let Some(obj) = model_selection.as_object() {
        for key in ["instanceId", "provider"] {
            if let Some(p) = obj.get(key).and_then(|v| v.as_str()) {
                if is_provider(p) {
                    return p.to_string();
                }
            }
        }
    }
    if let Some(opts) = provider_options.and_then(|v| v.as_object()) {
        let configured: Vec<&&str> = PROVIDERS
            .iter()
            .filter(|p| opts.contains_key(**p))
            .collect();
        if configured.len() == 1 {
            return configured[0].to_string();
        }
    }
    "codex".to_string()
}

pub fn make_unresolved_model_selection(
    model_selection: &Value,
    provider_options: Option<&Value>,
    provider_hint: Option<&Value>,
) -> (String, Map<String, Value>) {
    let provider = provider_for_unresolved(model_selection, provider_options, provider_hint);
    let model = model_selection
        .as_object()
        .and_then(|o| o.get("model"))
        .and_then(non_empty_string)
        .unwrap_or_else(|| UNRESOLVED_MODEL.to_string());
    let mut selection = Map::new();
    selection.insert(
        "instanceId".to_string(),
        Value::String(format!("{UNRESOLVED_PREFIX}{provider}")),
    );
    selection.insert("model".to_string(), Value::String(model));
    (provider, selection)
}

fn iso_valid(value: &str) -> bool {
    // ^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{3})?Z$
    let b = value.as_bytes();
    let digit = |i: usize| b.get(i).map(u8::is_ascii_digit).unwrap_or(false);
    let len = value.len();
    if len != 20 && len != 24 {
        return false;
    }
    if !digit(0) || !digit(1) || !digit(2) || !digit(3) || b[4] != b'-' || !digit(5) || !digit(6)
        || b[7] != b'-' || !digit(8) || !digit(9) || b[10] != b'T' || !digit(11) || !digit(12)
        || b[13] != b':' || !digit(14) || !digit(15) || b[16] != b':' || !digit(17) || !digit(18)
    {
        return false;
    }
    if len == 20 {
        b[19] == b'Z'
    } else {
        b[19] == b'.' && digit(20) && digit(21) && digit(22) && b[23] == b'Z'
    }
}

pub fn make_unresolved_permission_snapshot(
    snapshot_value: &Value,
    created_at_fallback: &str,
) -> Map<String, Value> {
    let empty = Map::new();
    let snapshot = snapshot_value.as_object().unwrap_or(&empty);
    let null = Value::Null;
    let (provider, model_selection) = make_unresolved_model_selection(
        snapshot.get("modelSelection").unwrap_or(&null),
        snapshot.get("providerOptions"),
        snapshot.get("provider"),
    );
    let created_at = snapshot
        .get("createdAt")
        .and_then(|v| v.as_str())
        .filter(|s| iso_valid(s))
        .map(str::to_string)
        .or_else(|| iso_valid(created_at_fallback).then(|| created_at_fallback.to_string()))
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string());
    let allowed: Vec<Value> = snapshot
        .get("allowedCapabilities")
        .and_then(|v| v.as_array())
        .map(|arr| {
            let mut seen = Vec::new();
            for cap in arr {
                if let Some(s) = cap.as_str() {
                    if matches!(s, "send-turn" | "create-worktree" | "full-access")
                        && !seen.iter().any(|v: &Value| v.as_str() == Some(s))
                    {
                        seen.push(cap.clone());
                    }
                }
            }
            seen
        })
        .unwrap_or_default();

    let mut out = Map::new();
    out.insert("provider".to_string(), Value::String(provider));
    out.insert("modelSelection".to_string(), Value::Object(model_selection));
    if let Some(v) = snapshot.get("completionPolicyVersion") {
        if v.as_i64().map(|n| n >= 0).unwrap_or(false) && v.is_number() {
            out.insert("completionPolicyVersion".to_string(), v.clone());
        }
    }
    out.insert(
        "runtimeMode".to_string(),
        Value::String(
            if snapshot.get("runtimeMode").and_then(|v| v.as_str()) == Some("full-access") {
                "full-access"
            } else {
                "approval-required"
            }
            .to_string(),
        ),
    );
    out.insert(
        "interactionMode".to_string(),
        Value::String(
            if snapshot.get("interactionMode").and_then(|v| v.as_str()) == Some("plan") {
                "plan"
            } else {
                "default"
            }
            .to_string(),
        ),
    );
    out.insert(
        "worktreeMode".to_string(),
        Value::String(
            match snapshot.get("worktreeMode").and_then(|v| v.as_str()) {
                Some("local") => "local",
                Some("worktree") => "worktree",
                _ => "auto",
            }
            .to_string(),
        ),
    );
    out.insert("allowedCapabilities".to_string(), Value::Array(allowed));
    out.insert("createdAt".to_string(), Value::String(created_at));
    out
}

/// Structural validation approximating `Schema.is(AutomationPermissionSnapshot)`:
/// the fields the schema requires, with the right primitive types.
pub fn is_valid_permission_snapshot(value: &Map<String, Value>) -> bool {
    let str_field = |key: &str| value.get(key).and_then(|v| v.as_str()).is_some();
    str_field("provider")
        && value.get("modelSelection").map(|v| v.is_object()).unwrap_or(false)
        && str_field("runtimeMode")
        && str_field("interactionMode")
        && str_field("worktreeMode")
        && str_field("createdAt")
        && value.get("allowedCapabilities").map(|v| v.is_array()).unwrap_or(false)
}
