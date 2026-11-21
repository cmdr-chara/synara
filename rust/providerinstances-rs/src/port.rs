//! The Rust port of `packages/shared/src/providerInstances.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use crate::js;

const BUILT_IN_PROVIDER_KINDS: [&str; 10] = [
    "codex",
    "claudeAgent",
    "cursor",
    "devin",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "omp",
];
const UNRESOLVED_PREFIX: &str = "synara_unresolved_automation_";
const CODEX_ACCOUNT_PREFIX: &str = "codex_";
const PROVIDER_INSTANCE_ID_MAX_CHARS: usize = 64;
const LEGACY_IDENTITY_KEYS: [&str; 3] = ["accountId", "homePath", "shadowHomePath"];

fn is_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn trim_js(s: &str) -> &str {
    s.trim_matches(is_space)
}

fn marker<'v>(v: &'v Value, key: &str) -> Option<&'v Value> {
    v.as_object()
        .filter(|o| o.len() == 1)
        .and_then(|o| o.get(key))
}

fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|e| {
                if e.is_null() || js::is_undefined(e) {
                    String::new()
                } else {
                    js_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".to_string()
            } else if let Some(u) = marker(v, "$utf16").and_then(Value::as_array) {
                let units: Vec<u16> = u
                    .iter()
                    .filter_map(|u| u.as_u64().and_then(|u| u16::try_from(u).ok()))
                    .collect();
                String::from_utf16_lossy(&units)
            } else if let Some(n) = marker(v, "$num").and_then(Value::as_str) {
                js::number_to_string(n.parse().unwrap_or(f64::NAN))
            } else if let Some(bytes) = marker(v, "$bytes").and_then(Value::as_array) {
                bytes
                    .iter()
                    .map(|b| js::number_to_string(js::as_number(b).unwrap_or(f64::NAN)))
                    .collect::<Vec<_>>()
                    .join(",")
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => {
            if js::is_undefined(v) {
                Err(format!(
                    "Cannot read properties of undefined (reading '{key}')"
                ))
            } else {
                Ok(o.get(key))
            }
        }
        _ => Ok(None),
    }
}

/// `x?.key` for a missing-or-`Option` parent.
fn opt_prop<'v>(v: Option<&'v Value>, key: &str) -> Result<Option<&'v Value>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(v) if js::is_undefined(v) => Ok(None),
        Some(v) => prop(v, key),
    }
}

fn is_defined(v: &Value) -> bool {
    !v.is_null() && !js::is_undefined(v)
}

fn trim_string(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => trim_js(s).to_string(),
        _ => String::new(),
    }
}

/// `x.trim()` where `x` is an arbitrary value: strings trim, missing throws the
/// property errors, other shapes throw "trim is not a function".
fn call_trim(v: Option<&Value>, expr: &str) -> Result<String, String> {
    match v {
        None => Err("Cannot read properties of undefined (reading 'trim')".to_string()),
        Some(Value::String(s)) => Ok(trim_js(s).to_string()),
        Some(Value::Null) => Err("Cannot read properties of null (reading 'trim')".to_string()),
        Some(v) if js::is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'trim')".to_string())
        }
        Some(_) => Err(format!("{expr}.trim is not a function")),
    }
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

fn is_provider_kind_str(s: &str) -> bool {
    BUILT_IN_PROVIDER_KINDS.contains(&s)
}

/// Port of `unresolvedAutomationInstanceId` (`packages/shared/src/providerInstances.ts:59`).
pub fn unresolved_automation_instance_id(
    provider: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(Value::String(format!(
        "{UNRESOLVED_PREFIX}{}",
        js_string(&provider)
    )))
}

/// Port of `isUnresolvedAutomationInstanceId` (`packages/shared/src/providerInstances.ts:63`).
pub fn is_unresolved_automation_instance_id(instance_id: Option<String>) -> Result<bool, String> {
    Ok(instance_id.is_some_and(|id| id.starts_with(UNRESOLVED_PREFIX)))
}

/// Port of `isProviderKind` (`packages/shared/src/providerInstances.ts:131`).
pub fn is_provider_kind(value: serde_json::Value) -> Result<bool, String> {
    Ok(value.as_str().is_some_and(is_provider_kind_str))
}

/// Port of `defaultInstanceIdForProvider` (`packages/shared/src/providerInstances.ts:135`).
pub fn default_instance_id_for_provider(
    provider: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(provider)
}

/// Port of `providerProfileDirectoryEnvironment` (`packages/shared/src/providerInstances.ts:139`).
pub fn provider_profile_directory_environment(
    provider: serde_json::Value,
    profile_dir: String,
) -> Result<serde_json::Value, String> {
    let kind = provider.as_str();
    if profile_dir.is_empty() || matches!(kind, Some("codex" | "claudeAgent" | "pi" | "omp")) {
        return Ok(Value::Object(Map::new()));
    }
    let key = match kind {
        Some("cursor") => "CURSOR_CONFIG_DIR",
        Some("grok") => "GROK_HOME",
        _ => "HOME",
    };
    let mut env = Map::new();
    env.insert(key.to_string(), Value::String(profile_dir));
    Ok(Value::Object(env))
}

/// Port of `providerImportedDirectoryConfig` (`packages/shared/src/providerInstances.ts:157`).
pub fn provider_imported_directory_config(
    provider: serde_json::Value,
    directory: String,
) -> Result<serde_json::Value, String> {
    let key = match provider.as_str() {
        Some("codex") => "homePath",
        Some("claudeAgent") => "configDir",
        Some("pi" | "omp") => "agentDir",
        _ => "profileDir",
    };
    let mut out = Map::new();
    out.insert(key.to_string(), Value::String(directory));
    Ok(Value::Object(out))
}

/// Port of `inferLegacyProviderKindFromInstanceId` (`packages/shared/src/providerInstances.ts:167`).
pub fn infer_legacy_provider_kind_from_instance_id(
    instance_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let Some(id) = instance_id.as_deref().filter(|id| !id.is_empty()) else {
        return Ok(js::undefined());
    };
    if is_provider_kind_str(id) {
        return Ok(Value::String(id.to_string()));
    }
    let lower = lower(id);
    let kind = if lower.starts_with("claude") {
        "claudeAgent"
    } else if lower.starts_with("codex") {
        "codex"
    } else if lower.starts_with("cursor") {
        "cursor"
    } else if lower.starts_with("antigravity")
        || lower.starts_with("gemini")
        || lower.starts_with("agy")
    {
        "antigravity"
    } else if lower.starts_with("grok") {
        "grok"
    } else if lower.starts_with("droid") {
        "droid"
    } else if lower.starts_with("opencode")
        || lower.starts_with("open_code")
        || lower.starts_with("kilo")
    {
        "opencode"
    } else if lower.starts_with("pi") {
        "pi"
    } else if lower.starts_with("devin") {
        "devin"
    } else {
        return Ok(js::undefined());
    };
    Ok(Value::String(kind.to_string()))
}

/// Port of `inferLegacyProviderKindFromModel` (`packages/shared/src/providerInstances.ts:196`).
pub fn infer_legacy_provider_kind_from_model(
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    let lower = lower(model.as_deref().unwrap_or(""));
    let kind = if lower.contains("claude")
        || lower.contains("sonnet")
        || lower.contains("opus")
        || lower.contains("haiku")
    {
        "claudeAgent"
    } else if lower.contains("gemini") || lower.contains("antigravity") {
        "antigravity"
    } else if lower.contains("grok") {
        "grok"
    } else if lower.contains("droid") {
        "droid"
    } else if lower.contains("devin") {
        "devin"
    } else if lower.contains("opencode") || lower.contains("open_code") || lower.contains("kilo") {
        "opencode"
    } else if lower.contains("cursor") {
        "cursor"
    } else if lower.starts_with("pi/") || lower.contains("/pi/") {
        "pi"
    } else {
        "codex"
    };
    Ok(Value::String(kind.to_string()))
}

/// Port of `inferLegacyProviderKindFromModelSelection` (`packages/shared/src/providerInstances.ts:220`).
pub fn infer_legacy_provider_kind_from_model_selection(
    selection: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let selection = selection.as_ref();
    let instance_id = opt_prop(selection, "instanceId")?;
    let from_id = match instance_id {
        Some(Value::String(id)) => infer_legacy_provider_kind_from_instance_id(Some(id.clone()))?,
        _ => js::undefined(),
    };
    if !js::is_undefined(&from_id) {
        return Ok(from_id);
    }
    let model = match opt_prop(selection, "model")? {
        Some(Value::String(m)) => Some(m.clone()),
        _ => None,
    };
    infer_legacy_provider_kind_from_model(model)
}

/// Port of `resolveModelSelectionInstanceId` (`packages/shared/src/providerInstances.ts:229`).
pub fn resolve_model_selection_instance_id(
    selection: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let selection = selection.as_ref();
    for key in ["instanceId", "provider"] {
        if let Some(v) = opt_prop(selection, key)?
            && is_defined(v)
        {
            return Ok(v.clone());
        }
    }
    Ok(Value::String("codex".to_string()))
}

/// Port of `resolveProviderStatusInstanceId` (`packages/shared/src/providerInstances.ts:235`).
pub fn resolve_provider_status_instance_id(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let instance_id = prop(&input, "instanceId")?;
    if let Some(id) = instance_id
        && is_defined(id)
    {
        return Ok(id.clone());
    }
    let provider = prop(&input, "provider")?;
    Ok(provider.cloned().unwrap_or_else(js::undefined))
}

fn stable_slug_hash(value: &str) -> String {
    let mut hash: u32 = 0x811c9dc5;
    for unit in value.encode_utf16() {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(0x01000193);
    }
    let mut digits = Vec::new();
    let mut n = hash;
    loop {
        digits.push(std::char::from_digit(n % 36, 36).unwrap_or('0'));
        n /= 36;
        if n == 0 {
            break;
        }
    }
    let s: String = digits.iter().rev().collect();
    format!("{:0>7}", s)
}

fn slug_codex_account_id(account_id: &str) -> String {
    let trimmed = trim_js(account_id);
    let mut slug = String::new();
    let mut pending = false;
    for c in trimmed.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            if pending {
                slug.push('_');
                pending = false;
            }
            slug.push(c);
        } else {
            pending = true;
        }
    }
    // `/_+/g` collapse then `^_+|_+$` trim: runs already collapsed by `pending`.
    let slug = slug.trim_matches('_').to_string();
    if slug.is_empty() {
        "account".to_string()
    } else {
        slug
    }
}

/// Port of `codexAccountInstanceId` (`packages/shared/src/providerInstances.ts:260`).
pub fn codex_account_instance_id(account_id: String) -> Result<serde_json::Value, String> {
    let normalized = trim_js(&account_id).to_string();
    let slug = slug_codex_account_id(&normalized);
    let raw = format!("{CODEX_ACCOUNT_PREFIX}{slug}");
    if raw.encode_utf16().count() <= PROVIDER_INSTANCE_ID_MAX_CHARS && slug == normalized {
        return Ok(Value::String(raw));
    }
    let hash = stable_slug_hash(&normalized);
    let available = PROVIDER_INSTANCE_ID_MAX_CHARS
        .saturating_sub(CODEX_ACCOUNT_PREFIX.encode_utf16().count())
        .saturating_sub(1)
        .saturating_sub(hash.len());
    let units: Vec<u16> = slug.encode_utf16().collect();
    let kept: String = String::from_utf16_lossy(&units[..units.len().min(available)]);
    Ok(Value::String(format!(
        "{CODEX_ACCOUNT_PREFIX}{kept}_{hash}"
    )))
}

/// `{...v}` for the shapes spread can copy.
fn spread_into(target: &mut Map<String, Value>, v: Option<&Value>) {
    match v {
        Some(Value::Object(o)) => {
            for (k, x) in o {
                target.insert(k.clone(), x.clone());
            }
        }
        Some(Value::String(s)) => {
            for (i, c) in s.chars().enumerate() {
                target.insert(i.to_string(), Value::String(c.to_string()));
            }
        }
        _ => {}
    }
}

fn instance_config_record(config: Option<&Value>) -> Map<String, Value> {
    match config {
        Some(Value::Object(o)) => o.clone(),
        _ => Map::new(),
    }
}

fn providers_of(settings: &Value) -> Result<Option<&Value>, String> {
    prop(settings, "providers")
}

fn legacy_provider_config(settings: &Value, provider: &str) -> Result<Map<String, Value>, String> {
    let providers = providers_of(settings)?;
    let missing = js::undefined();
    let parent = providers.unwrap_or(&missing);
    let undef = js::undefined();
    let legacy = prop(parent, provider)?.unwrap_or(&undef);
    let enabled = !matches!(prop(legacy, "enabled")?, Some(Value::Bool(false)));
    let mut config = Map::new();
    spread_into(&mut config, Some(legacy));
    let mut out = Map::new();
    out.insert("driver".to_string(), Value::String(provider.to_string()));
    out.insert("enabled".to_string(), Value::Bool(enabled));
    out.insert("config".to_string(), Value::Object(config));
    Ok(out)
}

fn prop_read<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    prop(v, key)
}

fn derive_legacy_codex_account_instances(settings: &Value) -> Result<Map<String, Value>, String> {
    let providers = providers_of(settings)?;
    let missing = js::undefined();
    let parent = providers.unwrap_or(&missing);
    let undef = js::undefined();
    let codex = prop_read(parent, "codex")?.unwrap_or(&undef);
    let accounts = prop_read(codex, "accounts")?;
    let mut instances = Map::new();
    let list = match accounts {
        None => {
            return Err("codex.accounts is not iterable".to_string());
        }
        Some(v) if js::is_undefined(v) => {
            return Err("codex.accounts is not iterable".to_string());
        }
        Some(Value::Null) => {
            return Err("codex.accounts is not iterable".to_string());
        }
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err("codex.accounts is not iterable".to_string());
        }
    };
    for account in list {
        let account_id = call_trim(prop_read(account, "id")?, "account.id")?;
        if account_id.is_empty() || account_id == "default" {
            continue;
        }
        let instance_id = match codex_account_instance_id(account_id.clone())? {
            Value::String(s) => s,
            v => js_string(&v),
        };
        let display = call_trim(prop_read(account, "label")?, "account.label")?;
        let home = call_trim(prop_read(account, "homePath")?, "account.homePath")?;
        let shadow = call_trim(
            prop_read(account, "shadowHomePath")?,
            "account.shadowHomePath",
        )?;
        let enabled = !matches!(prop_read(codex, "enabled")?, Some(Value::Bool(false)));
        let binary = prop_read(codex, "binaryPath")?
            .cloned()
            .unwrap_or(js::undefined());
        let custom = prop_read(codex, "customModels")?
            .cloned()
            .unwrap_or(js::undefined());
        let mut config = Map::new();
        config.insert("binaryPath".to_string(), binary);
        config.insert("homePath".to_string(), Value::String(home));
        config.insert("shadowHomePath".to_string(), Value::String(shadow));
        config.insert("accountId".to_string(), Value::String(account_id.clone()));
        config.insert("customModels".to_string(), custom);
        let mut entry = Map::new();
        entry.insert("driver".to_string(), Value::String("codex".to_string()));
        entry.insert(
            "displayName".to_string(),
            Value::String(if display.is_empty() {
                account_id
            } else {
                display
            }),
        );
        entry.insert("enabled".to_string(), Value::Bool(enabled));
        entry.insert("config".to_string(), Value::Object(config));
        instances.insert(instance_id, Value::Object(entry));
    }
    Ok(instances)
}

fn conflicts_with_derived_codex_account_identity(
    derived: &Value,
    explicit: &Value,
) -> Result<bool, String> {
    let derived_config = instance_config_record(prop(derived, "config")?);
    let derived_account_id = trim_string(derived_config.get("accountId"));
    let derived_driver = prop(derived, "driver")?;
    if !matches!(derived_driver, Some(Value::String(s)) if s == "codex")
        || derived_account_id.is_empty()
    {
        return Ok(false);
    }
    let explicit_driver = prop(explicit, "driver")?;
    if !matches!(explicit_driver, Some(Value::String(s)) if s == "codex") {
        return Ok(true);
    }
    let explicit_config = instance_config_record(prop(explicit, "config")?);
    for key in LEGACY_IDENTITY_KEYS {
        let Some(explicit_value) = explicit_config.get(key) else {
            continue;
        };
        match explicit_value {
            Value::String(s) if trim_js(s) == trim_string(derived_config.get(key)) => {}
            _ => return Ok(true),
        }
    }
    let environment = prop(explicit, "environment")?;
    let len = match environment {
        None => 0.0,
        Some(v) if js::is_undefined(v) || v.is_null() => 0.0,
        Some(Value::Array(a)) => a.len() as f64,
        Some(Value::String(s)) => s.chars().count() as f64,
        Some(v) => js::as_number(prop(v, "length")?.unwrap_or(&Value::Null)).unwrap_or(f64::NAN),
    };
    Ok(len > 0.0)
}

fn merge_derived_provider_instance_config(
    derived: &Value,
    explicit: &Value,
) -> Result<Value, String> {
    let mut out = Map::new();
    spread_into(&mut out, Some(derived));
    spread_into(&mut out, Some(explicit));
    let mut config = Map::new();
    spread_into(
        &mut config,
        Some(&Value::Object(instance_config_record(prop(
            derived, "config",
        )?))),
    );
    spread_into(
        &mut config,
        Some(&Value::Object(instance_config_record(prop(
            explicit, "config",
        )?))),
    );
    out.insert("config".to_string(), Value::Object(config));
    Ok(Value::Object(out))
}

/// Port of `deriveProviderInstanceConfigMap` (`packages/shared/src/providerInstances.ts:375`).
pub fn derive_provider_instance_config_map(
    settings: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut merged = Map::new();
    for provider in BUILT_IN_PROVIDER_KINDS {
        merged.insert(
            provider.to_string(),
            Value::Object(legacy_provider_config(&settings, provider)?),
        );
    }
    for (id, entry) in derive_legacy_codex_account_instances(&settings)? {
        merged.insert(id, entry);
    }
    let instances = prop(&settings, "providerInstances")?;
    match instances {
        None => return Err("Cannot convert undefined or null to object".to_string()),
        Some(Value::Null) => return Err("Cannot convert undefined or null to object".to_string()),
        Some(v) if js::is_undefined(v) => {
            return Err("Cannot convert undefined or null to object".to_string());
        }
        Some(v) => {
            let entries: Vec<(String, Value)> = match v {
                Value::Object(o) => o.iter().map(|(k, x)| (k.clone(), x.clone())).collect(),
                Value::Array(a) => a
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (i.to_string(), x.clone()))
                    .collect(),
                Value::String(s) => s
                    .chars()
                    .enumerate()
                    .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
                    .collect(),
                _ => Vec::new(),
            };
            for (instance_id, explicit) in entries {
                let derived = merged.get(&instance_id);
                let built_in = BUILT_IN_PROVIDER_KINDS.iter().find(|p| *p == &instance_id);
                let explicit_driver = prop(&explicit, "driver")?;
                if let Some(kind) = built_in
                    && !matches!(explicit_driver, Some(Value::String(s)) if s == kind)
                {
                    continue;
                }
                if let Some(d) = derived
                    && conflicts_with_derived_codex_account_identity(d, &explicit)?
                {
                    merged.remove(&instance_id);
                    continue;
                }
                let merged_entry = match derived {
                    Some(d)
                        if matches!(
                            (prop(d, "driver")?, explicit_driver),
                            (Some(Value::String(a)), Some(Value::String(b))) if a == b
                        ) =>
                    {
                        merge_derived_provider_instance_config(d, &explicit)?
                    }
                    _ => explicit.clone(),
                };
                merged.insert(instance_id, merged_entry);
            }
        }
    }
    Ok(Value::Object(merged))
}

fn is_ascii_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `instanceId.replace(/[_-]+/g," ").replace(/([a-z])([A-Z])/g,"$1 $2").replace(/\b\w/g, upper)`.
fn title_case_instance_id(id: &str) -> String {
    let mut spaced = String::new();
    let mut pending = false;
    for c in id.chars() {
        if c == '_' || c == '-' {
            pending = true;
        } else {
            if pending {
                spaced.push(' ');
                pending = false;
            }
            spaced.push(c);
        }
    }
    let chars: Vec<char> = spaced.chars().collect();
    let mut split = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && chars[i - 1].is_ascii_lowercase() && c.is_ascii_uppercase() {
            split.push(' ');
        }
        split.push(*c);
    }
    let mut out = String::new();
    let mut prev_word = false;
    for c in split.chars() {
        let word = is_ascii_word(c);
        if word && !prev_word {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
        prev_word = word;
    }
    out
}

fn display_name_for_instance(
    instance_id: &str,
    driver: Option<&Value>,
    raw: &Value,
) -> Result<String, String> {
    let explicit = prop(raw, "displayName")?;
    if let Some(dn) = explicit
        && !dn.is_null()
        && !js::is_undefined(dn)
    {
        let trimmed = call_trim(Some(dn), "raw.displayName?")?;
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    let driver_str = match driver {
        Some(Value::String(s)) => Some(s.as_str()),
        _ => None,
    };
    if Some(instance_id) == driver_str {
        return Ok(match driver_str {
            Some("claudeAgent") => "Claude".to_string(),
            Some("opencode") => "OpenCode".to_string(),
            _ => {
                let mut c = instance_id.chars();
                match c.next() {
                    Some(first) => {
                        format!("{}{}", first.to_uppercase(), c.as_str())
                    }
                    None => String::new(),
                }
            }
        });
    }
    Ok(title_case_instance_id(instance_id))
}

fn materialize_provider_instance_environment(raw: &Value) -> Result<Map<String, Value>, String> {
    let mut environment = Map::new();
    let raw_env = prop(raw, "environment")?;
    let variables: &[Value] = match raw_env {
        None => &[],
        Some(v) if v.is_null() || js::is_undefined(v) => &[],
        Some(Value::Array(a)) => a,
        Some(_) => return Err("raw.environment is not iterable".to_string()),
    };
    for variable in variables {
        let redacted = matches!(prop(variable, "valueRedacted")?, Some(Value::Bool(true)));
        if redacted {
            continue;
        }
        let name = call_trim(prop(variable, "name")?, "variable.name")?;
        if name.is_empty() {
            continue;
        }
        if let Some(Value::String(s)) = prop(variable, "value")? {
            environment.insert(name, Value::String(s.clone()));
        }
    }
    Ok(environment)
}

fn provider_environment_option(
    environment: Map<String, Value>,
    explicitly_configured: bool,
) -> Map<String, Value> {
    let mut out = Map::new();
    if explicitly_configured || !environment.is_empty() {
        out.insert("environment".to_string(), Value::Object(environment));
    }
    out
}

fn normalize_binary_path_override(provider: Option<&Value>, value: Option<&Value>) -> Value {
    let trimmed = trim_string(value);
    if trimmed.is_empty() {
        return Value::String(String::new());
    }
    let default_bin = match provider.and_then(Value::as_str) {
        Some("codex") => "codex",
        Some("claudeAgent") => "claude",
        Some("cursor") => "cursor-agent",
        Some("devin") => "devin",
        Some("antigravity") => "agy",
        Some("grok") => "grok",
        Some("droid") => "droid",
        Some("opencode") => "opencode",
        Some("pi") => "pi",
        Some("omp") => "omp",
        _ => return js::undefined(),
    };
    if trimmed == default_bin {
        Value::String(String::new())
    } else {
        Value::String(trimmed)
    }
}

fn resolved_instance(
    instance_id: &str,
    raw: &Value,
    require_kind: bool,
) -> Result<Option<Map<String, Value>>, String> {
    let driver = prop(raw, "driver")?;
    let is_kind = matches!(driver, Some(Value::String(s)) if is_provider_kind_str(s));
    if is_kind != require_kind {
        return Ok(None);
    }
    let config = match prop(raw, "config")? {
        Some(Value::Object(o)) => Value::Object(o.clone()),
        _ => Value::Object(Map::new()),
    };
    let enabled = !matches!(prop(raw, "enabled")?, Some(Value::Bool(false)))
        && !matches!(prop(&config, "enabled")?, Some(Value::Bool(false)));
    let display_name = display_name_for_instance(instance_id, driver, raw)?;
    let environment = materialize_provider_instance_environment(raw)?;
    let mut out = Map::new();
    out.insert(
        "instanceId".to_string(),
        Value::String(instance_id.to_string()),
    );
    out.insert(
        "driver".to_string(),
        driver.cloned().unwrap_or(js::undefined()),
    );
    out.insert("displayName".to_string(), Value::String(display_name));
    out.insert("enabled".to_string(), Value::Bool(enabled));
    if require_kind {
        out.insert(
            "isDefault".to_string(),
            Value::Bool(matches!(driver, Some(Value::String(s)) if s == instance_id)),
        );
    }
    out.insert("config".to_string(), config);
    out.insert("environment".to_string(), Value::Object(environment));
    out.insert("raw".to_string(), raw.clone());
    Ok(Some(out))
}

fn derive_instance_list(settings: &Value, require_kind: bool) -> Result<Vec<Value>, String> {
    let map = derive_provider_instance_config_map(settings.clone())?;
    let mut out = Vec::new();
    if let Value::Object(o) = map {
        for (instance_id, raw) in o {
            if instance_id.starts_with(UNRESOLVED_PREFIX) {
                continue;
            }
            if let Some(entry) = resolved_instance(&instance_id, &raw, require_kind)? {
                out.push(Value::Object(entry));
            }
        }
    }
    Ok(out)
}

/// Port of `deriveProviderInstances` (`packages/shared/src/providerInstances.ts:435`).
pub fn derive_provider_instances(
    settings: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    derive_instance_list(&settings, true)
}

/// Port of `deriveUnsupportedProviderInstances` (`packages/shared/src/providerInstances.ts:465`).
pub fn derive_unsupported_provider_instances(
    settings: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    derive_instance_list(&settings, false)
}

/// Port of `resolveProviderInstance` (`packages/shared/src/providerInstances.ts:494`).
pub fn resolve_provider_instance(
    settings: serde_json::Value,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let input_instance_id = prop(&input, "instanceId")?;
    if matches!(input_instance_id, Some(Value::String(s)) if s.starts_with(UNRESOLVED_PREFIX)) {
        return Ok(Value::Null);
    }
    let instances = derive_instance_list(&settings, true)?;
    if let Some(id) = input_instance_id
        && !js::is_undefined(id)
    {
        let provider = prop(&input, "provider")?;
        let found = instances.iter().find(|instance| {
            let same_id = prop(instance, "instanceId").ok().flatten() == Some(id)
                && !matches!(id, Value::Null);
            let provider_ok = match provider {
                None => true,
                Some(p) if js::is_undefined(p) => true,
                Some(p) => prop(instance, "driver").ok().flatten() == Some(p),
            };
            same_id && provider_ok
        });
        return Ok(found.cloned().unwrap_or(Value::Null));
    }
    let provider = prop(&input, "provider")?;
    let requested = match provider {
        Some(p) if !p.is_null() && !js::is_undefined(p) && js::as_number(p).is_none() => match p {
            Value::String(s) => Value::String(s.clone()),
            other => other.clone(),
        },
        Some(p) if js_truthy(p) => p.clone(),
        _ => Value::String("codex".to_string()),
    };
    let find =
        |pred: &dyn Fn(&Value) -> bool| -> Option<&Value> { instances.iter().find(|i| pred(i)) };
    if let Some(found) = find(&|i| prop(i, "instanceId").ok().flatten() == Some(&requested)) {
        return Ok(found.clone());
    }
    if let Some(found) = find(&|i| {
        prop(i, "driver").ok().flatten() == provider
            && matches!(prop(i, "isDefault").ok().flatten(), Some(Value::Bool(true)))
    }) {
        return Ok(found.clone());
    }
    if let Some(found) = find(
        &|i| matches!(prop(i, "instanceId").ok().flatten(), Some(Value::String(s)) if s == "codex"),
    ) {
        return Ok(found.clone());
    }
    Ok(instances.first().cloned().unwrap_or_else(js::undefined))
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !js::is_undefined(v),
    }
}

fn trim_opt(v: Option<&Value>) -> String {
    trim_string(v)
}

fn insert_if(out: &mut Map<String, Value>, cond: bool, key: &str, value: Value) {
    if cond {
        out.insert(key.to_string(), value);
    }
}

/// Port of `providerStartOptionsFromInstance` (`packages/shared/src/providerInstances.ts:525`).
pub fn provider_start_options_from_instance(
    instance: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let config = prop(&instance, "config")?;
    let config = match config {
        None => {
            return Err("Cannot read properties of undefined (reading 'binaryPath')".to_string());
        }
        Some(v) if js::is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'binaryPath')".to_string());
        }
        Some(v) => v,
    };
    let driver = prop(&instance, "driver")?;
    let binary_path = normalize_binary_path_override(driver, prop(config, "binaryPath")?);
    let binary = binary_path.as_str().unwrap_or("");
    let has_binary = !binary.is_empty();
    let profile_dir = trim_opt(prop(config, "profileDir")?);
    let profile_environment = provider_profile_directory_environment(
        driver.cloned().unwrap_or(js::undefined()),
        profile_dir.clone(),
    )?;
    let instance_environment = materialize_from_prop(&instance, "environment")?;
    let raw_environment_defined = {
        let raw = prop(&instance, "raw")?;
        match raw {
            None => {
                return Err(
                    "Cannot read properties of undefined (reading 'environment')".to_string(),
                );
            }
            Some(v) => {
                let raw_env = prop(v, "environment")?;
                match raw_env {
                    None => false,
                    Some(e) => !e.is_null() && !js::is_undefined(e),
                }
            }
        }
    };
    let mut env_map = Map::new();
    spread_into(&mut env_map, Some(&Value::Object(instance_environment)));
    spread_into(&mut env_map, Some(&profile_environment));
    let environment =
        provider_environment_option(env_map, raw_environment_defined || !profile_dir.is_empty());
    let env_present = environment.contains_key("environment");
    let driver_kind = driver.and_then(Value::as_str);
    let options = |body: Map<String, Value>| -> Value {
        let mut out = Map::new();
        out.insert(driver_kind.unwrap_or("").to_string(), Value::Object(body));
        Value::Object(out)
    };
    match driver_kind {
        Some("codex") => {
            let home_path = {
                let h = trim_opt(prop(config, "homePath")?);
                if h.is_empty() { profile_dir.clone() } else { h }
            };
            let shadow = trim_opt(prop(config, "shadowHomePath")?);
            let is_default = matches!(prop(&instance, "isDefault")?, Some(Value::Bool(true)));
            let instance_id = prop(&instance, "instanceId")?;
            let account_id = {
                let a = trim_opt(prop(config, "accountId")?);
                if !a.is_empty() {
                    a
                } else if is_default || !shadow.is_empty() {
                    String::new()
                } else {
                    js_string(instance_id.unwrap_or(&js::undefined()))
                }
            };
            if has_binary
                || !home_path.is_empty()
                || !shadow.is_empty()
                || !account_id.is_empty()
                || env_present
            {
                let mut body = Map::new();
                spread_into(&mut body, Some(&Value::Object(environment.clone())));
                insert_if(
                    &mut body,
                    has_binary,
                    "binaryPath",
                    Value::String(binary.to_string()),
                );
                insert_if(
                    &mut body,
                    !home_path.is_empty(),
                    "homePath",
                    Value::String(home_path),
                );
                insert_if(
                    &mut body,
                    !shadow.is_empty(),
                    "shadowHomePath",
                    Value::String(shadow),
                );
                insert_if(
                    &mut body,
                    !account_id.is_empty(),
                    "accountId",
                    Value::String(account_id),
                );
                Ok(options(body))
            } else {
                Ok(js::undefined())
            }
        }
        Some("claudeAgent") => {
            let home_path = trim_opt(prop(config, "homePath")?);
            let config_dir = {
                let d = trim_opt(prop(config, "configDir")?);
                if d.is_empty() { profile_dir.clone() } else { d }
            };
            let secure = trim_opt(prop(config, "secureStorageDir")?);
            let claude_environment = if !config_dir.is_empty() || !secure.is_empty() {
                let mut env = Map::new();
                spread_into(&mut env, environment.get("environment"));
                if !config_dir.is_empty() {
                    env.insert("CLAUDE_CONFIG_DIR".to_string(), Value::String(config_dir));
                }
                if !secure.is_empty() {
                    env.insert(
                        "CLAUDE_SECURESTORAGE_CONFIG_DIR".to_string(),
                        Value::String(secure),
                    );
                }
                Some(Value::Object(env))
            } else {
                environment.get("environment").cloned()
            };
            if has_binary || !home_path.is_empty() || claude_environment.is_some() {
                let mut body = Map::new();
                if let Some(env) = claude_environment {
                    body.insert("environment".to_string(), env);
                }
                insert_if(
                    &mut body,
                    has_binary,
                    "binaryPath",
                    Value::String(binary.to_string()),
                );
                insert_if(
                    &mut body,
                    !home_path.is_empty(),
                    "homePath",
                    Value::String(home_path),
                );
                Ok(options(body))
            } else {
                Ok(js::undefined())
            }
        }
        Some("cursor") => {
            let api = trim_opt(prop(config, "apiEndpoint")?);
            if has_binary || !api.is_empty() || env_present {
                let mut body = Map::new();
                spread_into(&mut body, Some(&Value::Object(environment.clone())));
                insert_if(
                    &mut body,
                    has_binary,
                    "binaryPath",
                    Value::String(binary.to_string()),
                );
                insert_if(
                    &mut body,
                    !api.is_empty(),
                    "apiEndpoint",
                    Value::String(api),
                );
                Ok(options(body))
            } else {
                Ok(js::undefined())
            }
        }
        Some("devin" | "antigravity" | "grok" | "droid") => {
            if has_binary || env_present {
                let mut body = Map::new();
                spread_into(&mut body, Some(&Value::Object(environment.clone())));
                insert_if(
                    &mut body,
                    has_binary,
                    "binaryPath",
                    Value::String(binary.to_string()),
                );
                Ok(options(body))
            } else {
                Ok(js::undefined())
            }
        }
        Some("opencode") => {
            let server_url = trim_opt(prop(config, "serverUrl")?);
            let server_password = trim_opt(prop(config, "serverPassword")?);
            let websockets = matches!(
                prop(config, "experimentalWebSockets")?,
                Some(Value::Bool(true))
            );
            if has_binary
                || !server_url.is_empty()
                || !server_password.is_empty()
                || websockets
                || env_present
            {
                let mut body = Map::new();
                spread_into(&mut body, Some(&Value::Object(environment.clone())));
                insert_if(
                    &mut body,
                    has_binary,
                    "binaryPath",
                    Value::String(binary.to_string()),
                );
                insert_if(
                    &mut body,
                    !server_url.is_empty(),
                    "serverUrl",
                    Value::String(server_url),
                );
                insert_if(
                    &mut body,
                    !server_password.is_empty(),
                    "serverPassword",
                    Value::String(server_password),
                );
                insert_if(
                    &mut body,
                    websockets,
                    "experimentalWebSockets",
                    Value::Bool(true),
                );
                Ok(options(body))
            } else {
                Ok(js::undefined())
            }
        }
        Some("pi" | "omp") => {
            let agent_dir = {
                let d = trim_opt(prop(config, "agentDir")?);
                if d.is_empty() { profile_dir.clone() } else { d }
            };
            if has_binary || !agent_dir.is_empty() || env_present {
                let mut body = Map::new();
                spread_into(&mut body, Some(&Value::Object(environment.clone())));
                insert_if(
                    &mut body,
                    has_binary,
                    "binaryPath",
                    Value::String(binary.to_string()),
                );
                insert_if(
                    &mut body,
                    !agent_dir.is_empty(),
                    "agentDir",
                    Value::String(agent_dir),
                );
                Ok(options(body))
            } else {
                Ok(js::undefined())
            }
        }
        _ => Ok(js::undefined()),
    }
}

fn materialize_from_prop(v: &Value, key: &str) -> Result<Map<String, Value>, String> {
    match prop(v, key)? {
        Some(Value::Object(o)) => Ok(o.clone()),
        _ => Ok(Map::new()),
    }
}

/// Port of `mergeProviderStartOptions` (`packages/shared/src/providerInstances.ts:659`).
pub fn merge_provider_start_options(
    base: serde_json::Value,
    overlay: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !js_truthy(&base) {
        return Ok(overlay);
    }
    if !js_truthy(&overlay) {
        return Ok(base);
    }
    let mut merged = Map::new();
    for provider in BUILT_IN_PROVIDER_KINDS {
        let base_opts = prop(&base, provider)?.filter(|v| js_truthy(v));
        let overlay_opts = prop(&overlay, provider)?.filter(|v| js_truthy(v));
        if base_opts.is_none() && overlay_opts.is_none() {
            continue;
        }
        let mut entry = Map::new();
        spread_into(&mut entry, base_opts);
        spread_into(&mut entry, overlay_opts);
        merged.insert(provider.to_string(), Value::Object(entry));
    }
    Ok(Value::Object(merged))
}
