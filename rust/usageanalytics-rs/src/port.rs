use serde_json::{Value, json};

const PROVIDERS: &[&str] = &[
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

const MODELS: &[(&str, &[&str])] = &[
    (
        "codex",
        &[
            "gpt-6-astra",
            "gpt-6-sol",
            "gpt-6-luna",
            "gpt-5.5",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.3-codex",
            "gpt-5.3-codex-spark",
            "gpt-5.2-codex",
            "gpt-5.2",
        ],
    ),
    (
        "claudeAgent",
        &[
            "claude-fable-5-1",
            "claude-fable-5",
            "claude-opus-5-5",
            "claude-opus-5",
            "claude-opus-4-8",
            "claude-opus-4-7",
            "claude-opus-4-6",
            "claude-opus-4-5",
            "claude-sonnet-5",
            "claude-sonnet-4-6",
            "claude-haiku-4-5",
        ],
    ),
    ("antigravity", &[]),
    (
        "grok",
        &["grok-4.6", "grok-build-0.1", "grok-build", "grok-4.5"],
    ),
    (
        "droid",
        &[
            "auto",
            "claude-fable-5",
            "claude-opus-4-8",
            "claude-opus-4-8-fast",
            "claude-opus-4-7",
            "claude-opus-4-7-fast",
            "claude-opus-4-6",
            "claude-sonnet-5",
            "claude-sonnet-4-6",
            "claude-opus-4-5-20251101",
            "claude-sonnet-4-5-20250929",
            "claude-haiku-4-5-20251001",
            "gpt-5.6-sol",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            "gpt-5.5",
            "gpt-5.5-fast",
            "gpt-5.5-pro",
            "gpt-5.4",
            "gpt-5.4-fast",
            "gpt-5.4-mini",
            "gpt-5.3-codex",
            "gpt-5.3-codex-fast",
            "gpt-5.2",
            "gemini-3.1-pro-preview",
            "gemini-3.5-flash",
            "gemini-3-flash-preview",
            "glm-5.2",
            "glm-5.2-fast",
            "glm-5.1",
            "nemotron-3-ultra",
            "kimi-k2.7-code",
            "kimi-k2.6",
            "deepseek-v4-pro",
            "deepseek-v4-flash-0731",
            "minimax-m3",
            "minimax-m2.7",
        ],
    ),
    ("opencode", &["openai/gpt-5"]),
    ("pi", &[]),
    (
        "cursor",
        &[
            "auto",
            "composer-2.5",
            "claude-opus-5",
            "claude-opus-4-8",
            "claude-opus-4-7",
            "claude-opus-4-6",
            "claude-opus-4-5",
            "claude-fable-5",
            "claude-sonnet-5",
            "claude-sonnet-4-6",
            "claude-sonnet-4-5",
            "claude-sonnet-4",
            "claude-haiku-4-5",
            "gpt-5.6-sol",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            "gpt-5.5",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.4-nano",
            "gpt-5.3-codex",
            "gpt-5.2",
            "gpt-5.1",
            "gpt-5-mini",
            "grok-4.5",
            "grok-4.6",
            "gemini-3.1-pro",
            "gemini-3.6-flash",
            "gemini-3.5-flash",
            "gemini-3-flash",
            "gemini-2.5-flash",
            "kimi-k2.7-code",
            "glm-5.2",
        ],
    ),
    ("devin", &["adaptive", "swe-1-6", "swe-1-7"]),
    ("omp", &[]),
];

const ALIASES: &[(&str, &[(&str, &str)])] = &[
    (
        "codex",
        &[
            ("6", "gpt-6-astra"),
            ("sol", "gpt-6-sol"),
            ("luna", "gpt-6-luna"),
            ("astra", "gpt-6-astra"),
            ("gpt-6", "gpt-6-astra"),
            ("5.5", "gpt-5.5"),
            ("5.4", "gpt-5.4"),
            ("5.3", "gpt-5.3-codex"),
            ("gpt-5.3", "gpt-5.3-codex"),
            ("5.3-spark", "gpt-5.3-codex-spark"),
            ("gpt-5.3-spark", "gpt-5.3-codex-spark"),
        ],
    ),
    (
        "claudeAgent",
        &[
            ("fable", "claude-fable-5-1"),
            ("fable-5.1", "claude-fable-5-1"),
            ("claude-fable-5.1", "claude-fable-5-1"),
            ("claude-fable-5-1", "claude-fable-5-1"),
            ("fable-5", "claude-fable-5"),
            ("claude-fable-5", "claude-fable-5"),
            ("opus", "claude-opus-5-5"),
            ("opus-5.5", "claude-opus-5-5"),
            ("claude-opus-5.5", "claude-opus-5-5"),
            ("claude-opus-5-5", "claude-opus-5-5"),
            ("opus-5", "claude-opus-5"),
            ("claude-opus-5", "claude-opus-5"),
            ("opus-4.8", "claude-opus-4-8"),
            ("claude-opus-4.8", "claude-opus-4-8"),
            ("claude-opus-4-8-20260528", "claude-opus-4-8"),
            ("opus-4.7", "claude-opus-4-7"),
            ("claude-opus-4.7", "claude-opus-4-7"),
            ("claude-opus-4-7-20260416", "claude-opus-4-7"),
            ("opus-4.6", "claude-opus-4-6"),
            ("claude-opus-4.6", "claude-opus-4-6"),
            ("claude-opus-4-6-20251117", "claude-opus-4-6"),
            ("opus-4.5", "claude-opus-4-5"),
            ("claude-opus-4.5", "claude-opus-4-5"),
            ("claude-opus-4-5-20250120", "claude-opus-4-5"),
            ("sonnet", "claude-sonnet-5"),
            ("sonnet-5", "claude-sonnet-5"),
            ("claude-sonnet-5", "claude-sonnet-5"),
            ("sonnet-4.6", "claude-sonnet-4-6"),
            ("claude-sonnet-4.6", "claude-sonnet-4-6"),
            ("claude-sonnet-4-6-20251117", "claude-sonnet-4-6"),
            ("haiku", "claude-haiku-4-5"),
            ("haiku-4.5", "claude-haiku-4-5"),
            ("claude-haiku-4.5", "claude-haiku-4-5"),
            ("claude-haiku-4-5-20251001", "claude-haiku-4-5"),
        ],
    ),
    (
        "cursor",
        &[
            ("auto", "auto"),
            ("default", "auto"),
            ("composer", "composer-2.5"),
            ("composer-2.5", "composer-2.5"),
            ("composer-2", "composer-2.5"),
            ("opus", "claude-opus-5"),
            ("opus-5", "claude-opus-5"),
            ("opus-4.8", "claude-opus-4-8"),
            ("opus-4.7", "claude-opus-4-7"),
            ("opus-4.6", "claude-opus-4-6"),
            ("opus-4.6-thinking", "claude-opus-4-6"),
            ("sonnet", "claude-sonnet-5"),
            ("sonnet-5", "claude-sonnet-5"),
            ("sonnet-4.6", "claude-sonnet-4-6"),
            ("fable", "claude-fable-5"),
            ("fable-5", "claude-fable-5"),
            ("sol", "gpt-5.6-sol"),
            ("5.6", "gpt-5.6-sol"),
            ("gpt-5.3", "gpt-5.3-codex"),
            ("codex-5.3", "gpt-5.3-codex"),
            ("grok", "grok-4.6"),
            ("grok-4.5", "grok-4.5"),
            ("grok-4.6", "grok-4.6"),
            ("cursor-grok-4.5", "grok-4.5"),
            ("cursor-grok-4.6", "grok-4.6"),
            ("gemini", "gemini-3.1-pro"),
            ("gemini-3", "gemini-3.1-pro"),
            ("gemini-3-pro", "gemini-3.1-pro"),
            ("gemini-3.1-pro-preview", "gemini-3.1-pro"),
            ("glm", "glm-5.2"),
            ("kimi", "kimi-k2.7-code"),
        ],
    ),
    ("antigravity", &[]),
    (
        "droid",
        &[
            ("droid", "claude-opus-4-8"),
            ("factory", "claude-opus-4-8"),
            ("opus", "claude-opus-4-8"),
            ("opus-4.8", "claude-opus-4-8"),
            ("opus-fast", "claude-opus-4-8-fast"),
            ("opus-4.8-fast", "claude-opus-4-8-fast"),
            ("opus-4.7", "claude-opus-4-7"),
            ("opus-4.7-fast", "claude-opus-4-7-fast"),
            ("opus-4.6", "claude-opus-4-6"),
            ("sonnet", "claude-sonnet-5"),
            ("sonnet-5", "claude-sonnet-5"),
            ("sonnet-4.6", "claude-sonnet-4-6"),
            ("sonnet-4.5", "claude-sonnet-4-5-20250929"),
            ("fable", "claude-fable-5"),
            ("haiku", "claude-haiku-4-5-20251001"),
            ("5.5", "gpt-5.5"),
            ("5.5-fast", "gpt-5.5-fast"),
            ("5.5-pro", "gpt-5.5-pro"),
            ("5.4", "gpt-5.4"),
            ("5.4-fast", "gpt-5.4-fast"),
            ("5.4-mini", "gpt-5.4-mini"),
            ("5.3", "gpt-5.3-codex"),
            ("5.3-fast", "gpt-5.3-codex-fast"),
            ("gpt-5.3", "gpt-5.3-codex"),
            ("gemini-3-pro", "gemini-3.1-pro-preview"),
            ("gemini-3.1-pro", "gemini-3.1-pro-preview"),
            ("gemini-3.5-flash", "gemini-3.5-flash"),
            ("gemini-3-flash", "gemini-3-flash-preview"),
            ("glm", "glm-5.2"),
            ("glm-5.2", "glm-5.2"),
            ("glm-5.1", "glm-5.1"),
            ("nemotron", "nemotron-3-ultra"),
            ("kimi", "kimi-k2.7-code"),
            ("kimi-code", "kimi-k2.7-code"),
            ("deepseek", "deepseek-v4-pro"),
            ("minimax", "minimax-m3"),
        ],
    ),
    (
        "grok",
        &[
            ("grok", "grok-build-0.1"),
            ("build", "grok-build-0.1"),
            ("grok-build-0.1", "grok-build-0.1"),
            ("grok-build", "grok-build"),
            ("4.3", "grok-build"),
            ("grok-4", "grok-build"),
            ("grok-4.3", "grok-build"),
            ("grok-latest", "grok-build"),
            ("grok-code-fast", "grok-build-0.1"),
            ("grok-code-fast-1", "grok-build-0.1"),
            ("grok-code-fast-1-0825", "grok-build-0.1"),
            ("code-fast", "grok-build-0.1"),
            ("4.5", "grok-4.5"),
            ("grok-4.5", "grok-4.5"),
            ("4.6", "grok-4.6"),
            ("grok-4.6", "grok-4.6"),
        ],
    ),
    ("opencode", &[]),
    ("pi", &[]),
    (
        "devin",
        &[
            ("adaptive", "adaptive"),
            ("auto", "adaptive"),
            ("fast", "swe-1-6"),
            ("swe-1.6-fast", "swe-1-6"),
            ("swe-1.6", "swe-1-6"),
            ("swe-1.7", "swe-1-7"),
            ("swe", "swe-1-6"),
            ("swe-1-6", "swe-1-6"),
            ("swe-1-7", "swe-1-7"),
            ("opus", "claude-opus-4-8"),
            ("sonnet", "claude-sonnet-5"),
            ("fable", "claude-fable-5"),
        ],
    ),
    ("omp", &[]),
];

const EFFORTS: &[&str] = &[
    "default",
    "none",
    "off",
    "minimal",
    "low",
    "medium",
    "high",
    "xhigh",
    "max",
    "ultrathink",
    "ultracode",
];
const INTERACTION_MODES: &[&str] = &["default", "plan", "debug"];
const RUNTIME_MODES: &[&str] = &["approval-required", "auto", "full-access"];
const ORIGINS: &[&str] = &["user", "automation", "agent"];

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

fn strip_ctx_suffix(model: &str) -> &str {
    // /\[([^\]]+)\]$/u
    if model.ends_with(']')
        && let Some(open) = model.rfind('[')
        && open + 1 < model.len() - 1
        && !model[open + 1..model.len() - 1].contains(']')
    {
        return &model[..open];
    }
    model
}

fn ctx_suffix(model: &str) -> Option<String> {
    let stripped = strip_ctx_suffix(model);
    if stripped.len() == model.len() {
        None
    } else {
        Some(model[stripped.len() + 1..model.len() - 1].to_lowercase())
    }
}

fn lookup<'a>(table: &[(&str, &'a str)], key: &str) -> Option<&'a str> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

fn models_for(provider: &str) -> &'static [&'static str] {
    MODELS
        .iter()
        .find(|(k, _)| *k == provider)
        .map(|(_, v)| *v)
        .unwrap_or(&[])
}

fn aliases_for(provider: &str) -> &'static [(&'static str, &'static str)] {
    ALIASES
        .iter()
        .find(|(k, _)| *k == provider)
        .map(|(_, v)| *v)
        .unwrap_or(&[])
}

/// `normalizeModelSlug(model, provider)` — provider is already validated to be a literal.
fn normalize_model_slug(model: &Value, provider: &str) -> Option<String> {
    let Value::String(raw) = model else {
        return None;
    };
    let trimmed = raw.trim_matches(is_js_space);
    if trimmed.is_empty() {
        return None;
    }
    let scoped = if provider == "claudeAgent" {
        strip_ctx_suffix(trimmed).to_string()
    } else if provider == "devin"
        && trimmed == trimmed.to_lowercase()
        && trimmed.ends_with("-medium")
    {
        trimmed[..trimmed.len() - "-medium".len()].to_string()
    } else {
        trimmed.to_string()
    };
    let alias_key = scoped.to_lowercase();
    let normalized = lookup(aliases_for(provider), &alias_key)
        .map(str::to_string)
        .unwrap_or(scoped);
    Some(
        if provider == "claudeAgent" && ctx_suffix(trimmed).as_deref() == Some("1m") {
            format!("{normalized}[1m]")
        } else {
            normalized
        },
    )
}

fn category<'a>(value: Option<&Value>, allowed: &[&'a str]) -> &'a str {
    match value {
        Some(Value::String(s)) => allowed.iter().copied().find(|x| *x == s).unwrap_or("other"),
        _ => "other",
    }
}

fn field<'a>(row: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    match row {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => Ok(m.get(key)),
        _ => Ok(None),
    }
}

/// Port of `usageProviderCategory` (`packages/shared/src/usageAnalytics.ts:47`).
pub fn usage_provider_category(value: serde_json::Value) -> Result<String, String> {
    Ok(match &value {
        Value::String(s) if PROVIDERS.contains(&s.as_str()) => s.clone(),
        _ => "other".to_string(),
    })
}

/// Port of `usageTurnCategories` (`packages/shared/src/usageAnalytics.ts:59`).
pub fn usage_turn_categories(row: serde_json::Value) -> Result<serde_json::Value, String> {
    let provider =
        usage_provider_category(field(&row, "provider")?.cloned().unwrap_or(Value::Null))?;
    let model_v = field(&row, "model")?.cloned().unwrap_or(Value::Null);
    let model = if matches!(model_v, Value::String(_)) && PROVIDERS.contains(&provider.as_str()) {
        match normalize_model_slug(&model_v, &provider) {
            Some(normalized) => {
                let slug = strip_ctx_suffix(&normalized).to_lowercase();
                if models_for(&provider).contains(&slug.as_str()) {
                    slug
                } else {
                    "other".to_string()
                }
            }
            None => "other".to_string(),
        }
    } else {
        "other".to_string()
    };
    Ok(json!({
        "provider": provider,
        "model": model,
        "effort": category(field(&row, "effort")?, EFFORTS),
        "interactionMode": category(field(&row, "interactionMode")?, INTERACTION_MODES),
        "runtimeMode": category(field(&row, "runtimeMode")?, RUNTIME_MODES),
        "origin": category(field(&row, "origin")?, ORIGINS),
    }))
}
