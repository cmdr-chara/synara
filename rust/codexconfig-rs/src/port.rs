//! The Rust port of `packages/shared/src/codexConfig.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn js_ws(c: char) -> bool {
    // JS \s: Unicode White_Space minus NEL, plus BOM.
    (c.is_whitespace() || c == '\u{feff}') && c != '\u{85}'
}

/// `^key\s*=\s*(?:"([^"]+)"|'([^']+)')` — anchored prefix match, no end anchor.
fn read_quoted_assignment_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start_matches(js_ws);
    let rest = rest.strip_prefix('=')?.trim_start_matches(js_ws);
    let q = *rest.as_bytes().first()?;
    if q != b'"' && q != b'\'' {
        return None;
    }
    let body = &rest[1..];
    let end = body.find(q as char)?;
    (end > 0).then(|| body[..end].to_string())
}

/// `^\[\s*model_providers\.(?:"([^"]+)"|'([^']+)'|([A-Za-z0-9_-]+))\s*\]$`
fn read_model_provider_section_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix('[')?.trim_start_matches(js_ws);
    let rest = rest.strip_prefix("model_providers.")?;
    let (name, tail) = match rest.as_bytes().first() {
        Some(&q) if q == b'"' || q == b'\'' => {
            let body = &rest[1..];
            let end = body.find(q as char)?;
            (body[..end].to_string(), &body[end + 1..])
        }
        _ => {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                .unwrap_or(rest.len());
            (rest[..end].to_string(), &rest[end..])
        }
    };
    if name.is_empty() {
        return None;
    }
    (tail.trim_start_matches(js_ws) == "]").then_some(name)
}

/// Port of `parseCodexConfigModelProvider` (`packages/shared/src/codexConfig.ts:23`).
pub fn parse_codex_config_model_provider(content: String) -> Result<Option<String>, String> {
    for line in content.split('\n') {
        let line = line.trim_matches(js_ws);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            break; // only the top level counts
        }
        if let Some(provider) = read_quoted_assignment_value(line, "model_provider") {
            return Ok(Some(provider));
        }
    }
    Ok(None)
}

/// Port of `parseCodexConfigProviderEnvKey` (`packages/shared/src/codexConfig.ts:41`).
pub fn parse_codex_config_provider_env_key(
    content: String,
    provider: String,
) -> Result<Option<String>, String> {
    let mut section: Option<String> = None;
    for line in content.split('\n') {
        let line = line.trim_matches(js_ws);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            section = read_model_provider_section_name(line);
            continue;
        }
        if section.as_deref() != Some(provider.as_str()) {
            continue;
        }
        if let Some(env_key) = read_quoted_assignment_value(line, "env_key") {
            return Ok(Some(env_key));
        }
    }
    Ok(None)
}

/// Port of `parseCodexConfigActiveProviderEnvKey` (`packages/shared/src/codexConfig.ts:65`).
pub fn parse_codex_config_active_provider_env_key(
    content: String,
) -> Result<Option<String>, String> {
    match parse_codex_config_model_provider(content.clone())? {
        Some(provider) if provider != "openai" => {
            parse_codex_config_provider_env_key(content, provider)
        }
        _ => Ok(None),
    }
}

/// `env.CODEX_HOME` per the JS source: `undefined` arg falls back to `process.env`,
/// `null` throws, non-string non-nullish values throw on `?.trim`.
fn codex_home_env(env: &Value) -> Result<Option<String>, String> {
    if crate::js::is_undefined(env) {
        return Ok(std::env::var("CODEX_HOME").ok());
    }
    if env.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'CODEX_HOME')".into());
    }
    match env.get("CODEX_HOME") {
        None => Ok(None),
        Some(v) if v.is_null() || crate::js::is_undefined(v) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err("TypeError: env.CODEX_HOME?.trim is not a function".into()),
    }
}

/// Port of `resolveCodexHome` (`packages/shared/src/codexConfig.ts:74`).
pub fn resolve_codex_home(env: serde_json::Value) -> Result<String, String> {
    let configured = codex_home_env(&env)?
        .map(|s| s.trim_matches(js_ws).to_string())
        .filter(|s| !s.is_empty());
    Ok(configured.unwrap_or_else(|| {
        std::env::home_dir()
            .unwrap_or_default()
            .join(".codex")
            .to_string_lossy()
            .into_owned()
    }))
}

/// Port of `readCodexConfigContent` (`packages/shared/src/codexConfig.ts:79`).
pub fn read_codex_config_content(env: serde_json::Value) -> Result<Option<String>, String> {
    let path = std::path::Path::new(&resolve_codex_home(env)?).join("config.toml");
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

/// Port of `readActiveCodexProviderEnvKey` (`packages/shared/src/codexConfig.ts:88`).
pub fn read_active_codex_provider_env_key(
    env: serde_json::Value,
) -> Result<Option<String>, String> {
    let Some(content) = read_codex_config_content(env)? else {
        return Ok(None);
    };
    parse_codex_config_active_provider_env_key(content)
}
