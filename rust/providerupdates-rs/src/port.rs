//! The Rust port of `apps/web/src/providerUpdates.ts` — the provider-update
//! filtering helpers (`isProviderUpdateActive`, `isProviderLatestVersionKnowable`,
//! `shouldOfferProviderUpdateAction`, `shouldPromptProviderUpdate`,
//! `shouldShowProviderUpdateStatus`, `getVisibleProviderUpdateStatuses`,
//! `getNotifiableProviderUpdateStatuses`, `providerUpdateNotificationKey`).

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const PROVIDER_KINDS: [&str; 10] = [
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

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn is_falsy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => s.is_empty(),
        Value::Number(n) => n.as_f64().is_some_and(|x| x == 0.0 || x.is_nan()),
        Value::Object(o) => {
            o.contains_key("$undefined")
                || (o.contains_key("$num") || o.contains_key("$bigint"))
                    && crate::js::as_number(v).is_some_and(|x| x == 0.0 || x.is_nan())
                || o.contains_key("$utf16")
                    && crate::js::string_units(v).is_some_and(|u| u.is_empty())
        }
        _ => false,
    }
}

/// `String(v)` for the pieces `Array.join` sees.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() || is_undefined(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else if let Some(units) = crate::js::string_units(v) {
                String::from_utf16_lossy(&units)
            } else if let Some(n) = crate::js::as_number(v) {
                crate::js::number_to_string(n)
            } else if o.contains_key("$bytes")
                || o.contains_key("$buffer")
                || o.contains_key("$typed")
            {
                "[object Uint8Array]".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// `value[key]` with JS semantics: null/undefined receivers throw TypeError,
/// primitives and marker-objects yield `undefined` (`None`).
fn prop<'v>(v: &'v Value, k: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() || is_undefined(v) {
        return Err(format!(
            "Cannot read properties of {} (reading '{k}')",
            if v.is_null() { "null" } else { "undefined" }
        ));
    }
    match v {
        Value::Object(o) => Ok(o.get(k).filter(|x| !is_undefined(x))),
        _ => Ok(None),
    }
}

/// `value === null || value === undefined` under the wire codec.
fn is_nullish(v: Option<&Value>) -> bool {
    v.is_none_or(|x| x.is_null())
}

/// `v?.k` — short-circuits on null/undefined instead of throwing.
fn opt_prop<'v>(v: Option<&'v Value>, k: &str) -> Result<Option<&'v Value>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(x) => prop(x, k),
    }
}

/// `v === false` on any wire value.
fn is_false(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Bool(false)))
}

fn str_value(v: &Value) -> Option<String> {
    crate::js::string_units(v).and_then(|u| String::from_utf16(&u).ok())
}

fn is_provider_kind(v: Option<&Value>) -> bool {
    v.and_then(str_value)
        .is_some_and(|s| PROVIDER_KINDS.contains(&s.as_str()))
}

/// `provider.updateState?.status === "queued" || === "running"`.
pub fn is_provider_update_active(provider: serde_json::Value) -> Result<bool, String> {
    let status = opt_prop(prop(&provider, "updateState")?, "status")?;
    Ok(status
        .and_then(str_value)
        .is_some_and(|s| s == "queued" || s == "running"))
}

/// `provider.versionAdvisory?.latestVersionKnowable !== false`.
pub fn is_provider_latest_version_knowable(provider: serde_json::Value) -> Result<bool, String> {
    let advisory = prop(&provider, "versionAdvisory")?;
    Ok(!is_false(opt_prop(advisory, "latestVersionKnowable")?))
}

/// `advisory?.canUpdate === true && currentVersion !== null &&
///  updateCommand !== null && status ∈ {"behind_latest","unknown"}`.
pub fn should_offer_provider_update_action(provider: serde_json::Value) -> Result<bool, String> {
    let advisory = prop(&provider, "versionAdvisory")?;
    if is_nullish(advisory) {
        return Ok(false);
    }
    // INFALLIBLE: the nullish check above guarantees `advisory` is present and non-null.
    let advisory = advisory.unwrap();
    if prop(advisory, "canUpdate")? != Some(&Value::Bool(true))
        || is_nullish(prop(advisory, "currentVersion")?)
        || is_nullish(prop(advisory, "updateCommand")?)
    {
        return Ok(false);
    }
    Ok(prop(advisory, "status")?
        .and_then(str_value)
        .is_some_and(|s| s == "behind_latest" || s == "unknown"))
}

/// `shouldOfferProviderUpdateAction(provider) && isProviderLatestVersionKnowable(provider)`.
pub fn should_prompt_provider_update(provider: serde_json::Value) -> Result<bool, String> {
    if !should_offer_provider_update_action(provider.clone())? {
        return Ok(false);
    }
    is_provider_latest_version_knowable(provider)
}

fn is_provider_enabled(provider: &Value, server_settings: Option<&Value>) -> Result<bool, String> {
    let Some(settings) = server_settings else {
        return Ok(false);
    };
    if is_falsy(settings) {
        return Ok(false);
    }
    let driver = match prop(provider, "driver")? {
        Some(d) => Some(d),
        None => prop(provider, "provider")?,
    };
    if !is_provider_kind(driver) {
        return Ok(false);
    }
    let driver_key = str_value(driver.unwrap()).unwrap_or_default();
    // `serverSettings.providers[driver]` — nullish `providers` throws in JS.
    let providers = prop(settings, "providers")?
        .ok_or_else(|| "Cannot read properties of undefined".to_string())?;
    let providers_entry = prop(providers, &driver_key)?;
    if is_false(opt_prop(providers_entry, "enabled")?) {
        return Ok(false);
    }
    let instance_id = match prop(provider, "instanceId")? {
        Some(x) => x,
        None => match prop(provider, "provider")? {
            Some(x) => x,
            None => &Value::Null,
        },
    };
    let key = match str_value(instance_id) {
        Some(s) => s,
        None if instance_id.is_null() => "undefined".to_string(),
        None => js_to_string(instance_id),
    };
    let instances = prop(settings, "providerInstances")?
        .ok_or_else(|| "Cannot read properties of undefined".to_string())?;
    let instance = prop(instances, &key)?;
    match instance {
        Some(inst) if !is_falsy(inst) => {
            let config_enabled = match prop(inst, "config")? {
                Some(c @ Value::Object(_)) => prop(c, "enabled")?,
                _ => None,
            };
            Ok(!is_false(prop(inst, "enabled")?) && !is_false(config_enabled))
        }
        _ => Ok(true),
    }
}

fn hidden_set(input: &Value) -> Result<Vec<Value>, String> {
    if let Some(set) = prop(input, "hiddenProviderSet")?
        && let Some(Value::Array(items)) = set.get("$set")
    {
        return Ok(items.clone());
    }
    Ok(match prop(input, "hiddenProviders")? {
        Some(Value::Array(a)) => a.clone(),
        _ => Vec::new(),
    })
}

/// `shouldShowProviderUpdateStatus`.
pub fn should_show_provider_update_status(input: serde_json::Value) -> Result<bool, String> {
    let provider = prop(&input, "provider")?
        .ok_or_else(|| "Cannot read properties of undefined".to_string())?;
    let advisory = prop(provider, "versionAdvisory")?;
    if advisory.is_none_or(is_falsy) {
        return Ok(false);
    }
    // INFALLIBLE: the falsy check above guarantees `advisory` is present and non-null.
    let advisory = advisory.unwrap();
    let driver = match prop(provider, "driver")? {
        Some(d) => Some(d),
        None => prop(provider, "provider")?,
    };
    let server_settings = prop(&input, "serverSettings")?;
    if is_false(opt_prop(server_settings, "enableProviderUpdateChecks")?)
        || prop(advisory, "status")?.and_then(str_value).as_deref() != Some("behind_latest")
        || is_nullish(prop(advisory, "latestVersion")?)
        || !is_provider_kind(driver)
        || hidden_set(&input)?.iter().any(|h| Some(h) == driver)
        || !is_provider_enabled(provider, server_settings)?
    {
        return Ok(false);
    }
    if prop(&input, "oneClickOnly")? == Some(&Value::Bool(true)) {
        return Ok(prop(advisory, "canUpdate")? == Some(&Value::Bool(true))
            && !is_nullish(prop(advisory, "updateCommand")?));
    }
    Ok(true)
}

/// `providers.filter(shouldShowProviderUpdateStatus)`.
pub fn get_visible_provider_update_statuses(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let providers = prop(&input, "providers")?;
    let Value::Array(items) = providers.unwrap_or(&Value::Null) else {
        return Err("input.providers.filter is not a function".to_string());
    };
    let mut vis_input = serde_json::Map::new();
    for key in [
        "serverSettings",
        "hiddenProviders",
        "hiddenProviderSet",
        "oneClickOnly",
    ] {
        if let Some(v) = prop(&input, key)? {
            vis_input.insert(key.to_string(), v.clone());
        }
    }
    let mut out = Vec::new();
    for provider in items {
        let mut vis = vis_input.clone();
        vis.insert("provider".into(), provider.clone());
        if should_show_provider_update_status(Value::Object(vis))? {
            out.push(provider.clone());
        }
    }
    Ok(Value::Array(out))
}

/// `liveVersionCheckCompleted ? visible(oneClickOnly: true) : []`.
pub fn get_notifiable_provider_update_statuses(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !prop(&input, "liveVersionCheckCompleted")?.is_some_and(|v| !is_falsy(v)) {
        return Ok(Value::Array(Vec::new()));
    }
    let Value::Object(mut obj) = input else {
        return Ok(Value::Array(Vec::new()));
    };
    obj.insert("oneClickOnly".into(), Value::Bool(true));
    get_visible_provider_update_statuses(Value::Object(obj))
}

/// `Array.join` renders null/undefined elements as "".
fn join_element(v: Option<&Value>) -> String {
    match v {
        None => String::new(),
        Some(x) if x.is_null() || is_undefined(x) => String::new(),
        Some(x) => js_to_string(x),
    }
}

/// `providers.map(p => [instanceId ?? provider, latestVersion ?? "unknown"].join(":")).toSorted().join("|")`.
pub fn provider_update_notification_key(
    providers: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Value::Array(items) = &providers else {
        return Err("providers.map is not a function".to_string());
    };
    let mut parts: Vec<String> = Vec::new();
    for provider in items {
        let key = match prop(provider, "instanceId")? {
            Some(x) => Some(x),
            None => prop(provider, "provider")?,
        };
        let latest = opt_prop(prop(provider, "versionAdvisory")?, "latestVersion")?;
        let latest = match latest {
            Some(Value::Null) | None => json!("unknown"),
            Some(x) => x.clone(),
        };
        parts.push(format!("{}:{}", join_element(key), js_to_string(&latest)));
    }
    parts.sort();
    if parts.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(json!(parts.join("|")))
    }
}
