//! The Rust port of `packages/shared/src/serverSettings.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

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

const DEFAULT_CODEX_ACCOUNT_ID: &str = "default";
const PROVIDER_INSTANCE_ID_MAX_CHARS: usize = 64;
const CODEX_ACCOUNT_INSTANCE_PREFIX: &str = "codex_";
const UNRESOLVED_AUTOMATION_INSTANCE_PREFIX: &str = "synara_unresolved_automation_";
const LEGACY_CODEX_ACCOUNT_IDENTITY_KEYS: [&str; 3] = ["accountId", "homePath", "shadowHomePath"];
const PROTOTYPE_MUTATION_KEYS: [&str; 3] = ["__proto__", "constructor", "prototype"];

fn default_model_for_provider(provider: &str) -> Option<&'static str> {
    if provider == "omp" {
        return None;
    }
    if provider == "pi" {
        return Some("openai/gpt-5.5");
    }
    Some(match provider {
        "codex" => "gpt-6-astra",
        "claudeAgent" => "claude-sonnet-5",
        "cursor" => "auto",
        "devin" => "adaptive",
        "antigravity" => "Gemini 3.5 Flash",
        "grok" => "grok-4.6",
        "droid" => "claude-opus-4-8",
        "opencode" => "openai/gpt-5",
        _ => return None,
    })
}

fn truthy(v: &Value) -> bool {
    if js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Object(o) if o.get("$num").is_some() => js::as_number(v).is_some_and(|x| x != 0.0),
        _ => true,
    }
}

fn defined(v: Option<&Value>) -> bool {
    !matches!(v, None | Some(Value::Null)) && !v.is_some_and(js::is_undefined)
}

fn is_not_undefined(v: &Value) -> bool {
    !js::is_undefined(v)
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => match (js::as_number(a), js::as_number(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
    }
}

fn trim_string(v: &Value) -> String {
    v.as_str().map(|s| s.trim().to_string()).unwrap_or_default()
}

/// `Object.entries` on a member: throws when the receiver is null/undefined,
/// yields nothing for other non-objects.
fn entries_of(v: &Value) -> Result<Vec<(String, Value)>, String> {
    if v.is_null() || js::is_undefined(v) {
        return Err("Cannot convert undefined or null to object".to_string());
    }
    match v {
        Value::Object(o) => Ok(o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        _ => Ok(Vec::new()),
    }
}

/// `{ ...v }` object spread: own enumerable entries, or indexed chars for strings.
fn spread(v: Option<&Value>) -> Map<String, Value> {
    match v {
        Some(o @ Value::Object(_)) if !js::is_undefined(o) => {
            o.as_object().cloned().unwrap_or_default()
        }
        Some(Value::String(s)) => s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect(),
        _ => Map::new(),
    }
}

fn get<'a>(v: &'a Value, key: &str) -> Result<&'a Value, String> {
    if js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => Ok(o.get(key).unwrap_or(&Value::Null)),
        _ => Ok(&Value::Null),
    }
}

fn prop<'a>(v: &'a Value, key: &str) -> &'a Value {
    static NULL: Value = Value::Null;
    v.as_object().and_then(|o| o.get(key)).unwrap_or(&NULL)
}

// ---- deepMerge (`./Struct`) ------------------------------------------------------------

fn deep_merge(base: &Value, patch: &Value) -> Value {
    if !base.is_object() || !patch.is_object() || js::is_undefined(base) || js::is_undefined(patch)
    {
        return patch.clone();
    }
    let mut next = base.as_object().cloned().unwrap_or_default();
    for (key, value) in patch.as_object().into_iter().flatten() {
        if js::is_undefined(value) || PROTOTYPE_MUTATION_KEYS.contains(&key.as_str()) {
            continue;
        }
        let current = next.get(key).cloned().unwrap_or(Value::Null);
        let merged = if current.is_object()
            && !js::is_undefined(&current)
            && value.is_object()
            && !js::is_undefined(value)
        {
            deep_merge(&current, value)
        } else {
            value.clone()
        };
        next.insert(key.clone(), merged);
    }
    Value::Object(next)
}

// ---- deriveProviderInstances (only instanceId + driver are observed) -------------------

fn is_provider_kind(v: &Value) -> bool {
    v.as_str()
        .is_some_and(|s| BUILT_IN_PROVIDER_KINDS.contains(&s))
}

fn stable_slug_hash(value: &str) -> String {
    let mut hash: u32 = 0x811c9dc5;
    for unit in value.encode_utf16() {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(0x01000193);
    }
    // (hash >>> 0).toString(36).padStart(7, "0")
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut n = hash;
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    let mut s: String = out.into_iter().rev().map(char::from).collect();
    while s.len() < 7 {
        s.insert(0, '0');
    }
    s
}

fn slug_codex_account_id(account_id: &str) -> String {
    let mut slug = String::new();
    let mut last_underscore = true; // leading `_+` trimmed
    for c in account_id.trim().chars() {
        if c.is_ascii_alphanumeric() || c == '-' {
            slug.push(c);
            last_underscore = false;
        } else if !last_underscore {
            slug.push('_');
            last_underscore = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        "account".to_string()
    } else {
        slug
    }
}

fn codex_account_instance_id(account_id: &str) -> String {
    let normalized = account_id.trim();
    let slug = slug_codex_account_id(normalized);
    let raw = format!("{CODEX_ACCOUNT_INSTANCE_PREFIX}{slug}");
    if raw.len() <= PROVIDER_INSTANCE_ID_MAX_CHARS && slug == normalized {
        return raw;
    }
    let hash = stable_slug_hash(normalized);
    let available =
        PROVIDER_INSTANCE_ID_MAX_CHARS - CODEX_ACCOUNT_INSTANCE_PREFIX.len() - 1 - hash.len();
    let head: String = slug.chars().take(available).collect();
    format!("{CODEX_ACCOUNT_INSTANCE_PREFIX}{head}_{hash}")
}

fn instance_config_record(config: &Value) -> Map<String, Value> {
    match config {
        Value::Object(o) if !js::is_undefined(config) => o.clone(),
        _ => Map::new(),
    }
}

fn conflicts_with_derived_codex_account_identity(derived: &Value, explicit: &Value) -> bool {
    let derived_config = instance_config_record(prop(derived, "config"));
    let derived_account_id = trim_string(derived_config.get("accountId").unwrap_or(&Value::Null));
    if prop(derived, "driver").as_str() != Some("codex") || derived_account_id.is_empty() {
        return false;
    }
    if prop(explicit, "driver").as_str() != Some("codex") {
        return true;
    }
    let explicit_config = instance_config_record(prop(explicit, "config"));
    for key in LEGACY_CODEX_ACCOUNT_IDENTITY_KEYS {
        if !explicit_config.contains_key(key) {
            continue;
        }
        let explicit_value = &explicit_config[key];
        if explicit_value.as_str().is_none_or(|s| {
            s.trim() != trim_string(derived_config.get(key).unwrap_or(&Value::Null))
        }) {
            return true;
        }
    }
    prop(explicit, "environment")
        .as_array()
        .is_some_and(|a| !a.is_empty())
}

fn merge_derived_provider_instance_config(derived: &Value, explicit: &Value) -> Value {
    let mut merged = instance_config_record(derived);
    merged.extend(instance_config_record(explicit));
    let mut config = instance_config_record(prop(derived, "config"));
    config.extend(instance_config_record(prop(explicit, "config")));
    merged.insert("config".to_string(), Value::Object(config));
    Value::Object(merged)
}

fn derive_provider_instance_config_map(settings: &Value) -> Result<Map<String, Value>, String> {
    let mut merged = Map::new();
    let providers = get(settings, "providers")?.clone();
    for provider in BUILT_IN_PROVIDER_KINDS {
        let legacy = get(&providers, provider)?.clone();
        if legacy.is_null() || js::is_undefined(&legacy) {
            return Err("Cannot read properties of undefined (reading 'enabled')".to_string());
        }
        let enabled = !matches!(prop(&legacy, "enabled"), Value::Bool(false));
        let mut entry = Map::new();
        entry.insert("driver".to_string(), json!(provider));
        entry.insert("enabled".to_string(), Value::Bool(enabled));
        entry.insert("config".to_string(), Value::Object(spread(Some(&legacy))));
        merged.insert(provider.to_string(), Value::Object(entry));
    }

    // deriveLegacyCodexAccountInstances
    let codex = get(&providers, "codex")?.clone();
    let accounts = prop(&codex, "accounts");
    {
        let list = accounts
            .as_array()
            .ok_or_else(|| "codex.accounts is not iterable".to_string())?;
        let codex_enabled = prop(&codex, "enabled");
        let codex_binary_path = prop(&codex, "binaryPath").clone();
        let codex_custom_models = prop(&codex, "customModels").clone();
        let trimmed_field = |v: &Value, key: &str| -> Result<String, String> {
            prop(v, key)
                .as_str()
                .map(|s| s.trim().to_string())
                .ok_or_else(|| format!("{key}.trim is not a function"))
        };
        for account in list {
            let account_id = trimmed_field(account, "id")?;
            if account_id.is_empty() || account_id == DEFAULT_CODEX_ACCOUNT_ID {
                continue;
            }
            let instance_id = codex_account_instance_id(&account_id);
            let label = trimmed_field(account, "label")?;
            let display = if label.is_empty() {
                account_id.clone()
            } else {
                label
            };
            let mut config = Map::new();
            config.insert("binaryPath".to_string(), codex_binary_path.clone());
            config.insert(
                "homePath".to_string(),
                json!(trimmed_field(account, "homePath")?),
            );
            config.insert(
                "shadowHomePath".to_string(),
                json!(trimmed_field(account, "shadowHomePath")?),
            );
            config.insert("accountId".to_string(), json!(account_id));
            config.insert("customModels".to_string(), codex_custom_models.clone());
            let mut entry = Map::new();
            entry.insert("driver".to_string(), json!("codex"));
            entry.insert("displayName".to_string(), json!(display));
            entry.insert("enabled".to_string(), codex_enabled.clone());
            entry.insert("config".to_string(), Value::Object(config));
            merged.insert(instance_id, Value::Object(entry));
        }
    }

    for (instance_id, explicit) in entries_of(get(settings, "providerInstances")?)? {
        let derived = merged.get(&instance_id).cloned();
        let built_in_driver = BUILT_IN_PROVIDER_KINDS.iter().find(|p| **p == instance_id);
        if built_in_driver.is_some()
            && prop(&explicit, "driver").as_str() != built_in_driver.copied()
        {
            continue;
        }
        if let Some(d) = &derived
            && conflicts_with_derived_codex_account_identity(d, &explicit)
        {
            merged.remove(&instance_id);
            continue;
        }
        let entry = match &derived {
            Some(d) if prop(d, "driver") == prop(&explicit, "driver") => {
                merge_derived_provider_instance_config(d, &explicit)
            }
            _ => explicit,
        };
        merged.insert(instance_id, entry);
    }
    Ok(merged)
}

/// `(instanceId, driver)` pairs of `deriveProviderInstances`, in map order.
fn derive_provider_instances(settings: &Value) -> Result<Vec<(String, String)>, String> {
    let map = derive_provider_instance_config_map(settings)?;
    let mut resolved = Vec::new();
    for (instance_id, raw) in &map {
        if instance_id.starts_with(UNRESOLVED_AUTOMATION_INSTANCE_PREFIX) {
            continue;
        }
        let driver = prop(raw, "driver");
        if !is_provider_kind(driver) {
            continue;
        }
        resolved.push((
            instance_id.clone(),
            driver.as_str().unwrap_or_default().to_string(),
        ));
    }
    Ok(resolved)
}

// ---- applyServerSettingsPatch -----------------------------------------------------------

fn text_gen_model_selection_needs_replace(patch: &Value) -> bool {
    truthy(patch)
        && patch.as_object().is_some_and(|o| {
            o.get("provider").is_some_and(is_not_undefined)
                || o.get("instanceId").is_some_and(is_not_undefined)
                || o.get("model").is_some_and(is_not_undefined)
        })
}

/// Port of `applyServerSettingsPatch` (`packages/shared/src/serverSettings.ts:30`).
pub fn apply_server_settings_patch(current: Value, patch: Value) -> Result<Value, String> {
    let selection_patch = get(&patch, "textGenerationModelSelection")?.clone();
    let merged = deep_merge(&current, &patch);
    let with_instances = if patch
        .as_object()
        .and_then(|o| o.get("providerInstances"))
        .is_some_and(is_not_undefined)
    {
        let mut m = spread(Some(&merged));
        m.insert(
            "providerInstances".to_string(),
            patch["providerInstances"].clone(),
        );
        Value::Object(m)
    } else {
        merged.clone()
    };

    let new_thread_selections = patch
        .as_object()
        .and_then(|o| o.get("newThreadDefaults"))
        .and_then(|ntd| ntd.as_object())
        .and_then(|o| o.get("modelSelectionByProvider"))
        .cloned()
        .filter(is_not_undefined);

    let next = match new_thread_selections {
        Some(selections) => {
            // `current.newThreadDefaults.modelSelectionByProvider` is read directly:
            // a missing newThreadDefaults throws in the source.
            let current_ntd = get(&current, "newThreadDefaults")?;
            let mut base = spread(
                current_ntd
                    .as_object()
                    .and_then(|o| o.get("modelSelectionByProvider")),
            );
            base.extend(spread(Some(&selections)));
            let mut ntd = spread(
                with_instances
                    .as_object()
                    .and_then(|o| o.get("newThreadDefaults")),
            );
            ntd.insert("modelSelectionByProvider".to_string(), Value::Object(base));
            let mut m = spread(Some(&with_instances));
            m.insert("newThreadDefaults".to_string(), Value::Object(ntd));
            Value::Object(m)
        }
        None => with_instances,
    };

    if !truthy(&selection_patch) {
        return Ok(next);
    }

    let sp = &selection_patch;
    let sp_instance_id = sp.as_object().and_then(|o| o.get("instanceId"));
    let sp_provider = sp.as_object().and_then(|o| o.get("provider"));
    let sp_model = sp.as_object().and_then(|o| o.get("model"));
    let sp_options = sp.as_object().and_then(|o| o.get("options"));
    let current_selection = get(&current, "textGenerationModelSelection")?.clone();

    let patched_instance_id: Option<Value> = if defined(sp_instance_id) {
        Some(sp_instance_id.unwrap_or(&Value::Null).clone())
    } else if truthy(sp_provider.unwrap_or(&Value::Null)) {
        Some(sp_provider.unwrap_or(&Value::Null).clone())
    } else {
        let v = get(&current_selection, "instanceId")?.clone();
        if js::is_undefined(&v) { None } else { Some(v) }
    };

    let patched_instance: Option<(String, String)> = match &patched_instance_id {
        Some(id) if !js::is_undefined(id) => {
            let instances = derive_provider_instances(&next)?;
            instances
                .into_iter()
                .find(|(iid, _)| &Value::String(iid.clone()) == id)
        }
        _ => None,
    };

    let provider = match &patched_instance {
        Some((_, driver)) => Value::String(driver.clone()),
        None => match sp_provider {
            Some(p) if defined(sp_provider) => p.clone(),
            _ => get(&current_selection, "provider")?.clone(),
        },
    };

    let instance_id: Option<Value> = match &patched_instance {
        Some((iid, _)) => Some(Value::String(iid.clone())),
        None => patched_instance_id,
    };

    let provider_changed = !strict_eq(&provider, get(&current_selection, "provider")?);

    let model = if defined(sp_model) {
        sp_model.unwrap_or(&Value::Null).clone()
    } else if provider_changed
        && let Some(m) = provider.as_str().and_then(default_model_for_provider)
    {
        json!(m)
    } else {
        get(&current_selection, "model")?.clone()
    };

    let options = if text_gen_model_selection_needs_replace(sp) {
        sp_options.cloned().unwrap_or_else(js::undefined)
    } else {
        match sp_options {
            Some(o) if is_not_undefined(o) && !o.is_null() => o.clone(),
            _ => get(&current_selection, "options")?.clone(),
        }
    };

    let mut sel = Map::new();
    sel.insert("provider".to_string(), provider);
    if let Some(iid) = instance_id.filter(is_not_undefined) {
        sel.insert("instanceId".to_string(), iid);
    }
    sel.insert("model".to_string(), model);
    if is_not_undefined(&options) {
        sel.insert("options".to_string(), options);
    }

    let mut out = spread(Some(&next));
    out.insert(
        "textGenerationModelSelection".to_string(),
        Value::Object(sel),
    );
    Ok(Value::Object(out))
}

// ---- providerStartOptionsFromServerSettings ---------------------------------------------

/// `.trim()` on a member that must be a string (TypeError otherwise).
fn required_trimmed(obj: &Value, key: &str) -> Result<String, String> {
    match prop(obj, key).as_str() {
        Some(s) => Ok(s.trim().to_string()),
        None => Err(format!("{}.trim is not a function", key)),
    }
}

/// truthy non-trimmed string pass-through (`binaryPath ? { binaryPath } : {}`).
fn optional_raw(obj: &Value, key: &str) -> Option<Value> {
    let v = prop(obj, key);
    if truthy(v) { Some(v.clone()) } else { None }
}

/// Port of `providerStartOptionsFromServerSettings` (`packages/shared/src/serverSettings.ts:108`).
pub fn provider_start_options_from_server_settings(settings: Value) -> Result<Value, String> {
    let providers = get(&settings, "providers")?.clone();
    let codex = get(&providers, "codex")?.clone();
    let claude = get(&providers, "claudeAgent")?.clone();
    let cursor = get(&providers, "cursor")?.clone();
    let antigravity = get(&providers, "antigravity")?.clone();
    let grok = get(&providers, "grok")?.clone();
    let droid = get(&providers, "droid")?.clone();
    let opencode = get(&providers, "opencode")?.clone();
    let pi = get(&providers, "pi")?.clone();
    let devin = get(&providers, "devin")?.clone();
    let omp = get(&providers, "omp")?.clone();

    let selected_codex_account: Option<Value> = {
        let selected = prop(&codex, "selectedAccountId");
        if strict_eq(selected, &json!(DEFAULT_CODEX_ACCOUNT_ID)) {
            None
        } else {
            let accounts = prop(&codex, "accounts");
            let list = accounts
                .as_array()
                .ok_or_else(|| "providers.codex.accounts.find is not a function".to_string())?;
            list.iter()
                .find(|a| strict_eq(prop(a, "id"), selected))
                .cloned()
        }
    };

    let codex_binary_path = required_trimmed(&codex, "binaryPath")?;
    let selected_home = selected_codex_account
        .as_ref()
        .map(|a| prop(a, "homePath"))
        .cloned()
        .unwrap_or(Value::Null);
    let codex_home_path = if truthy(&selected_home) {
        selected_home
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| "homePath.trim is not a function".to_string())?
    } else {
        required_trimmed(&codex, "homePath")?
    };
    let claude_binary_path = required_trimmed(&claude, "binaryPath")?;

    let mut codex_out = Map::new();
    if !codex_binary_path.is_empty() {
        codex_out.insert("binaryPath".to_string(), json!(codex_binary_path));
    }
    if !codex_home_path.is_empty() {
        codex_out.insert("homePath".to_string(), json!(codex_home_path));
    }
    if let Some(account) = &selected_codex_account
        && truthy(prop(account, "shadowHomePath"))
    {
        codex_out.insert(
            "shadowHomePath".to_string(),
            prop(account, "shadowHomePath").clone(),
        );
    }
    if let Some(account) = &selected_codex_account {
        codex_out.insert("accountId".to_string(), prop(account, "id").clone());
    }

    let mut claude_out = Map::new();
    if !claude_binary_path.is_empty() {
        claude_out.insert("binaryPath".to_string(), json!(claude_binary_path));
    }
    let claude_home = required_trimmed(&claude, "homePath")?;
    if !claude_home.is_empty() {
        claude_out.insert("homePath".to_string(), json!(claude_home));
    }
    claude_out.insert(
        "enableArtifacts".to_string(),
        prop(&claude, "enableArtifacts").clone(),
    );

    let mut cursor_out = Map::new();
    if let Some(v) = optional_raw(&cursor, "binaryPath") {
        cursor_out.insert("binaryPath".to_string(), v);
    }
    if let Some(v) = optional_raw(&cursor, "apiEndpoint") {
        cursor_out.insert("apiEndpoint".to_string(), v);
    }

    let mut antigravity_out = Map::new();
    if let Some(v) = optional_raw(&antigravity, "binaryPath") {
        antigravity_out.insert("binaryPath".to_string(), v);
    }

    let mut grok_out = Map::new();
    if let Some(v) = optional_raw(&grok, "binaryPath") {
        grok_out.insert("binaryPath".to_string(), v);
    }

    let mut droid_out = Map::new();
    if let Some(v) = optional_raw(&droid, "binaryPath") {
        droid_out.insert("binaryPath".to_string(), v);
    }

    let mut opencode_out = Map::new();
    if let Some(v) = optional_raw(&opencode, "binaryPath") {
        opencode_out.insert("binaryPath".to_string(), v);
    }
    if let Some(v) = optional_raw(&opencode, "serverUrl") {
        opencode_out.insert("serverUrl".to_string(), v);
    }
    opencode_out.insert(
        "experimentalWebSockets".to_string(),
        prop(&opencode, "experimentalWebSockets").clone(),
    );

    let mut pi_out = Map::new();
    if let Some(v) = optional_raw(&pi, "binaryPath") {
        pi_out.insert("binaryPath".to_string(), v);
    }
    if let Some(v) = optional_raw(&pi, "agentDir") {
        pi_out.insert("agentDir".to_string(), v);
    }

    let mut devin_out = Map::new();
    if let Some(v) = optional_raw(&devin, "binaryPath") {
        devin_out.insert("binaryPath".to_string(), v);
    }

    let mut omp_out = Map::new();
    if let Some(v) = optional_raw(&omp, "binaryPath") {
        omp_out.insert("binaryPath".to_string(), v);
    }
    if let Some(v) = optional_raw(&omp, "agentDir") {
        omp_out.insert("agentDir".to_string(), v);
    }

    Ok(json!({
        "codex": codex_out,
        "claudeAgent": claude_out,
        "cursor": cursor_out,
        "antigravity": antigravity_out,
        "grok": grok_out,
        "droid": droid_out,
        "opencode": opencode_out,
        "pi": pi_out,
        "devin": devin_out,
        "omp": omp_out,
    }))
}
