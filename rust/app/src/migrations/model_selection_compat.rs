//! Port of `persistence/modelSelectionCompatibility.ts` — the subset needed by
//! migration 035, with no settings access (migrations cannot read settings, so
//! `resolveProviderFromSettings` always yields undefined here).

use serde_json::{Map, Value};

const DROID_ONLY_MODEL_SLUGS: [&str; 18] = [
    "claude-haiku-4-5-20251001",
    "claude-opus-4-5-20251101",
    "claude-opus-4-7-fast",
    "claude-opus-4-8-fast",
    "claude-sonnet-4-5-20250929",
    "deepseek-v4-pro",
    "gemini-3-flash-preview",
    "gemini-3.1-pro-preview",
    "glm-5.1",
    "glm-5.2-fast",
    "gpt-5.3-codex-fast",
    "gpt-5.4-fast",
    "gpt-5.5-fast",
    "gpt-5.5-pro",
    "kimi-k2.6",
    "minimax-m2.7",
    "minimax-m3",
    "nemotron-3-ultra",
];

const KNOWN_PROVIDERS: [&str; 10] = [
    "codex",
    "claudeAgent",
    "cursor",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "devin",
    "omp",
];

fn legacy_gemini_label(model: &str) -> Option<&'static str> {
    match model {
        "gemini-3.1-pro-preview" => Some("Gemini 3.1 Pro"),
        "gemini-3-flash-preview" => Some("Gemini 3.5 Flash"),
        _ => None,
    }
}

/// ProviderSlug shape: trimmed non-empty, <= 64 chars, ^[A-Za-z][A-Za-z0-9_-]*$.
fn is_provider_instance_id(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 64
        && trimmed.len() == value.len()
        && {
            let mut chars = trimmed.chars();
            chars.next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        }
}

fn word_token(haystack: &str, token: &str) -> bool {
    // (^|[^a-z0-9])token([^a-z0-9]|$) on the lowercased label
    let bytes = haystack.as_bytes();
    let tlen = token.len();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(token) {
        let abs = start + pos;
        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !(b.is_ascii_lowercase() || b.is_ascii_digit())
        };
        let end = abs + tlen;
        let after_ok = end >= bytes.len() || {
            let b = bytes[end];
            !(b.is_ascii_lowercase() || b.is_ascii_digit())
        };
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
        if start >= haystack.len() {
            break;
        }
    }
    false
}

fn infer_provider_from_label(label: &str) -> Option<&'static str> {
    let lower = label.to_lowercase();
    if word_token(&lower, "omp") || lower.contains("oh-my-pi") || lower.contains("oh my pi") {
        return Some("omp");
    }
    if word_token(&lower, "pi") {
        return Some("pi");
    }
    if lower.contains("devin") {
        return Some("devin");
    }
    if lower.contains("opencode") {
        return Some("opencode");
    }
    if lower.contains("kilo") {
        return Some("opencode");
    }
    if lower.contains("cursor") {
        return Some("cursor");
    }
    if lower.contains("antigravity") {
        return Some("antigravity");
    }
    if lower.contains("claude") || lower.contains("anthropic") {
        return Some("claudeAgent");
    }
    if lower.contains("gemini") || lower.contains("google") {
        return Some("antigravity");
    }
    if lower.contains("grok") || lower.contains("xai") || lower.contains("x.ai") {
        return Some("grok");
    }
    if lower.contains("windsurf") {
        return Some("devin");
    }
    if lower.contains("droid") || lower.contains("factory") {
        return Some("droid");
    }
    if word_token(&lower, "cognition") {
        return Some("devin");
    }
    if lower.contains("codex") {
        return Some("codex");
    }
    None
}

fn infer_specific_model_provider(model: &str) -> Option<&'static str> {
    let lower = model.to_lowercase();
    if DROID_ONLY_MODEL_SLUGS.contains(&lower.as_str()) {
        return Some("droid");
    }
    if lower.contains("claude") {
        return Some("claudeAgent");
    }
    if lower.contains("gemini") {
        return Some("antigravity");
    }
    if lower.contains("grok") {
        return Some("grok");
    }
    if lower.contains("devin") {
        return Some("devin");
    }
    None
}

fn infer_legacy_model_provider(provider: &Value, model: &str) -> String {
    if let Some(p) = provider.as_str() {
        if KNOWN_PROVIDERS.contains(&p) {
            return p.to_string();
        }
        if p == "gemini" {
            return "antigravity".to_string();
        }
        if p == "kilo" {
            return "opencode".to_string();
        }
        if let Some(from_label) = infer_provider_from_label(p) {
            return from_label.to_string();
        }
    }
    infer_specific_model_provider(model)
        .unwrap_or("codex")
        .to_string()
}

fn read_legacy_provider_options<'a>(
    options: &'a Value,
    provider: &str,
    legacy_provider: Option<&str>,
) -> &'a Value {
    if !options.is_object() {
        return options;
    }
    let scoped = options
        .get(provider)
        .or_else(|| legacy_provider.and_then(|lp| options.get(lp)));
    match scoped {
        Some(v) => v,
        None => options,
    }
}

fn normalize_model_options(input: &Value) -> Value {
    let arr = match input.as_array() {
        Some(a) => a,
        None => return input.clone(),
    };
    let mut entries = Map::new();
    for option in arr {
        let obj = match option.as_object() {
            Some(o) => o,
            None => return input.clone(),
        };
        let id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let id = match id {
            Some(i) => i,
            None => return input.clone(),
        };
        entries.insert(
            id.to_string(),
            obj.get("value").cloned().unwrap_or(Value::Null),
        );
    }
    Value::Object(entries)
}

/// Split "Model Name (effort)" labels; effort must be low/medium/high/thinking.
fn split_legacy_antigravity_model_label(model: &str) -> (String, Option<String>) {
    let trimmed = model.trim();
    if let Some(open) = trimmed.rfind('(') {
        if trimmed.ends_with(')') && open > 0 {
            let inner = trimmed[open + 1..trimmed.len() - 1].trim().to_lowercase();
            let head = trimmed[..open].trim();
            if !head.is_empty()
                && matches!(inner.as_str(), "low" | "medium" | "high" | "thinking")
            {
                return (head.to_string(), Some(inner));
            }
        }
    }
    (trimmed.to_string(), None)
}

fn migrate_legacy_gemini_model(model: &str) -> String {
    let trimmed = model.trim();
    legacy_gemini_label(&trimmed.to_lowercase())
        .map(str::to_string)
        .unwrap_or_else(|| trimmed.to_string())
}

pub fn normalize_legacy_model_selection(
    provider_in: &Value,
    instance_id_in: Option<&Value>,
    model: &str,
    options_in: &Value,
) -> Map<String, Value> {
    let provider = infer_legacy_model_provider(provider_in, model);
    let migrated_gemini = provider_in.as_str() == Some("gemini");
    let normalized_options = if migrated_gemini {
        None
    } else {
        Some(normalize_model_options(read_legacy_provider_options(
            options_in,
            &provider,
            provider_in.as_str(),
        )))
    };
    let antigravity_model = if provider == "antigravity" {
        let input_model = if migrated_gemini {
            migrate_legacy_gemini_model(model)
        } else {
            model.to_string()
        };
        Some(split_legacy_antigravity_model_label(&input_model))
    } else {
        None
    };
    let options = match (&antigravity_model, &normalized_options) {
        (Some((_, Some(effort))), norm @ (None | Some(Value::Object(_)))) => {
            let mut obj = match norm {
                Some(Value::Object(o)) => o.clone(),
                _ => Map::new(),
            };
            obj.insert("reasoningEffort".to_string(), Value::String(effort.clone()));
            Some(Value::Object(obj))
        }
        _ => normalized_options.clone(),
    };

    let instance_id = instance_id_in
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| is_provider_instance_id(s))
        .map(str::to_string);

    let mut out = Map::new();
    out.insert("provider".to_string(), Value::String(provider));
    if let Some(id) = instance_id {
        out.insert("instanceId".to_string(), Value::String(id));
    }
    let final_model = match &antigravity_model {
        Some((m, _)) => m.clone(),
        None => model.to_string(),
    };
    out.insert("model".to_string(), Value::String(final_model));
    if let Some(opts) = options {
        out.insert("options".to_string(), opts);
    }
    out
}

pub fn normalize_persisted_model_selection(input: &Value) -> Value {
    let obj = match input.as_object() {
        Some(o) => o,
        None => return input.clone(),
    };
    let model = obj
        .get("model")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let model = match model {
        Some(m) => m,
        None => return input.clone(),
    };
    let instance_id = obj
        .get("instanceId")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    // No settings inside a migration: providerFromSettings is always absent.
    if obj.get("provider").is_none()
        && instance_id.is_some()
        && instance_id.and_then(infer_provider_from_label).is_none()
        && infer_specific_model_provider(model).is_none()
    {
        return input.clone();
    }
    let provider_value = match obj.get("provider") {
        Some(p) => p.clone(),
        None => instance_id
            .map(|id| Value::String(id.to_string()))
            .unwrap_or(Value::Null),
    };
    Value::Object(normalize_legacy_model_selection(
        &provider_value,
        obj.get("instanceId"),
        model,
        obj.get("options").unwrap_or(&Value::Null),
    ))
}

/// Normalize when the serialized form actually changes; mirror the upstream
/// `JSON.stringify` comparison as closely as serde ordering allows.
pub fn normalize_model_selection_json(json: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(json).ok()?;
    let normalized = normalize_persisted_model_selection(&parsed);
    let normalized_json = serde_json::to_string(&normalized).ok()?;
    let original_reserialized = serde_json::to_string(&parsed).ok()?;
    if normalized_json != original_reserialized {
        Some(normalized_json)
    } else {
        None
    }
}
