//! Provider instance resolution, ported from upstream
//! `shared/providerInstances.ts`. Server settings carry a `providerInstances`
//! map; each entry is an account boundary with its own driver, config, and
//! environment. Instance environments never include redacted values, and a
//! non-default instance forces an isolated provider home so credentials and
//! CLI state never mix across accounts.

use serde_json::{Value, json};

pub const BUILT_IN_PROVIDER_KINDS: &[&str] = &[
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

const UNRESOLVED_AUTOMATION_INSTANCE_PREFIX: &str = "synara_unresolved_automation_";

pub(crate) fn is_unresolved_automation_instance_id(id: &str) -> bool {
    id.starts_with(UNRESOLVED_AUTOMATION_INSTANCE_PREFIX)
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedInstance {
    pub instance_id: String,
    pub driver: String,
    pub enabled: bool,
    pub is_default: bool,
    pub config: Value,
    /// Materialized environment: redacted variables are never present.
    pub environment: Vec<(String, String)>,
}

fn materialize_environment(raw: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(vars) = raw.get("environment").and_then(Value::as_array) {
        for var in vars {
            if var.get("valueRedacted").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let name = var.get("name").and_then(Value::as_str).unwrap_or("").trim();
            if name.is_empty() {
                continue;
            }
            if let Some(value) = var.get("value").and_then(Value::as_str) {
                out.push((name.to_string(), value.to_string()));
            }
        }
    }
    out
}

/// Derive instances from the settings document, upstream
/// `deriveProviderInstances`: unresolved automation entries and unknown
/// drivers are skipped; `isDefault` holds when the instance id is the driver
/// name itself.
pub(crate) fn derive_instances(settings: &Value) -> Vec<ResolvedInstance> {
    // Upstream deriveProviderInstanceConfigMap always seeds every built-in
    // driver from the legacy `providers` section before overlaying explicit
    // providerInstances. Without those implicit defaults, a thread selecting a
    // provider with no explicit rows silently falls back to codex.
    let mut merged: std::collections::BTreeMap<String, (String, bool, Value, Value)> =
        std::collections::BTreeMap::new();
    for provider in BUILT_IN_PROVIDER_KINDS {
        let config = settings
            .get("providers")
            .and_then(|providers| providers.get(*provider))
            .filter(|config| config.is_object())
            .cloned()
            .unwrap_or(json!({}));
        let enabled = config.get("enabled").and_then(Value::as_bool) != Some(false);
        merged.insert(
            provider.to_string(),
            (
                provider.to_string(),
                enabled,
                config.clone(),
                json!({
                    "driver": provider,
                    "enabled": enabled,
                    "config": config,
                }),
            ),
        );
    }
    if let Some(map) = settings.get("providerInstances").and_then(Value::as_object) {
        for (instance_id, raw) in map {
            if is_unresolved_automation_instance_id(instance_id) {
                merged.remove(instance_id);
                continue;
            }
            let driver = raw.get("driver").and_then(Value::as_str).unwrap_or("");
            if !BUILT_IN_PROVIDER_KINDS.contains(&driver) {
                merged.remove(instance_id);
                continue;
            }
            if BUILT_IN_PROVIDER_KINDS.contains(&instance_id.as_str()) && instance_id != driver {
                // Canonical built-in ids are routing authorities and cannot be
                // retargeted to another provider's runtime.
                continue;
            }
            let explicit_config = raw
                .get("config")
                .filter(|config| config.is_object())
                .cloned()
                .unwrap_or(json!({}));
            let (config, _derived_enabled, merged_raw) = match merged.get(instance_id) {
                Some((derived_driver, derived_enabled, derived_config, derived_raw))
                    if derived_driver == driver =>
                {
                    // Explicit rows merge key-by-key over derived built-in config,
                    // so sparse rows (for example customModels only) keep live
                    // launch settings such as homes and binary paths.
                    let mut config = derived_config.clone();
                    if let (Some(target), Some(overlay)) =
                        (config.as_object_mut(), explicit_config.as_object())
                    {
                        for (key, value) in overlay {
                            target.insert(key.clone(), value.clone());
                        }
                    }
                    let mut merged_raw = derived_raw.clone();
                    if let (Some(target), Some(overlay)) =
                        (merged_raw.as_object_mut(), raw.as_object())
                    {
                        for (key, value) in overlay {
                            target.insert(key.clone(), value.clone());
                        }
                        target.insert("config".to_string(), config.clone());
                    }
                    (config, *derived_enabled, merged_raw)
                }
                _ => (explicit_config, true, raw.clone()),
            };
            let enabled = merged_raw.get("enabled").and_then(Value::as_bool) != Some(false)
                && config.get("enabled").and_then(Value::as_bool) != Some(false);
            merged.insert(
                instance_id.clone(),
                (driver.to_string(), enabled, config, merged_raw),
            );
        }
    }
    let mut out = Vec::new();
    for (instance_id, (driver, enabled, config, raw)) in merged {
        out.push(ResolvedInstance {
            is_default: instance_id == driver,
            environment: materialize_environment(&raw),
            instance_id,
            driver,
            enabled,
            config,
        });
    }
    out
}

/// Resolve one instance, upstream `resolveProviderInstance`: explicit id wins
/// (optionally constrained by driver), otherwise the provider default, then
/// any default-flagged instance of the driver, then codex, then the first.
pub(crate) fn resolve_instance(
    settings: &Value,
    instance_id: Option<&str>,
    provider: Option<&str>,
) -> Option<ResolvedInstance> {
    if instance_id.map(is_unresolved_automation_instance_id) == Some(true) {
        return None;
    }
    let instances = derive_instances(settings);
    if let Some(id) = instance_id {
        return instances
            .into_iter()
            .find(|i| i.instance_id == id && provider.map(|p| p == i.driver).unwrap_or(true));
    }
    let requested = provider.unwrap_or("codex");
    instances
        .iter()
        .find(|i| i.instance_id == requested)
        .or_else(|| {
            instances
                .iter()
                .find(|i| i.driver == requested && i.is_default)
        })
        .or_else(|| instances.iter().find(|i| i.instance_id == "codex"))
        .or_else(|| instances.first())
        .cloned()
}

/// Environment keys a profile directory implies per provider, upstream
/// `providerProfileDirectoryEnvironment`.
pub(crate) fn profile_directory_environment(
    driver: &str,
    profile_dir: &str,
) -> Vec<(String, String)> {
    if profile_dir.trim().is_empty() {
        return Vec::new();
    }
    let key = match driver {
        "cursor" => "CURSOR_CONFIG_DIR",
        "grok" => "GROK_HOME",
        _ => "HOME",
    };
    vec![(key.to_string(), profile_dir.to_string())]
}

fn trim_config(config: &Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Start environment for one instance, the environment portion of upstream
/// `providerStartOptionsFromInstance`: instance variables first, then the
/// profile-directory mapping, then driver-specific keys (Claude config dir
/// and secure storage). Callers apply these after spawn sanitization so a
/// deliberate instance configuration always wins over ambient filtering.
pub(crate) fn instance_start_environment(instance: &ResolvedInstance) -> Vec<(String, String)> {
    let mut env = instance.environment.clone();
    if let Some(profile_dir) = trim_config(&instance.config, "profileDir") {
        env.extend(profile_directory_environment(
            &instance.driver,
            &profile_dir,
        ));
    }
    if instance.driver == "claudeAgent" {
        let config_dir = trim_config(&instance.config, "configDir")
            .or_else(|| trim_config(&instance.config, "profileDir"));
        if let Some(dir) = config_dir {
            env.push(("CLAUDE_CONFIG_DIR".to_string(), dir));
        }
        if let Some(dir) = trim_config(&instance.config, "secureStorageDir") {
            env.push(("CLAUDE_SECURESTORAGE_CONFIG_DIR".to_string(), dir));
        }
    }
    env
}

/// Home path override for drivers that carry one (codex `homePath` falling
/// back to `profileDir`, claude `homePath`). When a non-default instance has
/// no explicit home, the caller must force the isolated instance home so
/// accounts never share CLI state.
pub(crate) fn instance_home_path(instance: &ResolvedInstance) -> Option<String> {
    match instance.driver.as_str() {
        "codex" => trim_config(&instance.config, "homePath")
            .or_else(|| trim_config(&instance.config, "profileDir")),
        _ => trim_config(&instance.config, "homePath"),
    }
}

/// Resolve the default instance for a driver and apply its start
/// environment to a child command, after spawn sanitization. Shared by all
/// runtime spawners; driver-specific home isolation beyond environment
/// configuration (Codex overlays, Claude instance homes) stays in the
/// owning runner.
pub(crate) fn apply_instance_environment(command: &mut std::process::Command, driver: &str) {
    apply_instance_environment_for(command, driver, None);
}

/// Instance-id-aware variant: the thread's model selection carries the
/// account when one was chosen, upstream modelSelection.instanceId.
pub(crate) fn apply_instance_environment_for(
    command: &mut std::process::Command,
    driver: &str,
    instance_id: Option<&str>,
) {
    let settings = crate::server_settings::current();
    if let Some(instance) = resolve_instance(&settings, instance_id, Some(driver)) {
        for (key, value) in instance_start_environment(&instance) {
            command.env(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(map: Value) -> Value {
        json!({ "providerInstances": map })
    }

    #[test]
    fn derive_skips_unknown_drivers_and_automation_entries() {
        let settings = settings_with(json!({
            "codex": { "driver": "codex" },
            "synara_unresolved_automation_codex": { "driver": "codex" },
            "mystery": { "driver": "notAProvider" },
        }));
        let instances = derive_instances(&settings);
        assert_eq!(instances.len(), BUILT_IN_PROVIDER_KINDS.len());
        let codex = instances
            .iter()
            .find(|instance| instance.instance_id == "codex")
            .unwrap();
        assert!(codex.is_default);
        assert!(
            !instances
                .iter()
                .any(|instance| instance.instance_id == "mystery")
        );
        assert!(
            !instances
                .iter()
                .any(|instance| instance.instance_id == "synara_unresolved_automation_codex")
        );
    }

    #[test]
    fn redacted_environment_values_never_materialize() {
        let settings = settings_with(json!({
            "codex": { "driver": "codex", "environment": [
                { "name": "TOKEN", "value": "secret", "valueRedacted": true },
                { "name": "VISIBLE", "value": "yes" },
                { "name": "  ", "value": "skipped" },
            ]},
        }));
        let instances = derive_instances(&settings);
        let codex = instances
            .iter()
            .find(|instance| instance.instance_id == "codex")
            .unwrap();
        assert_eq!(
            codex.environment,
            vec![("VISIBLE".to_string(), "yes".to_string())]
        );
    }

    #[test]
    fn resolve_prefers_explicit_id_then_default() {
        let settings = settings_with(json!({
            "codex": { "driver": "codex" },
            "codex_work": { "driver": "codex" },
        }));
        let resolved = resolve_instance(&settings, Some("codex_work"), None).unwrap();
        assert_eq!(resolved.instance_id, "codex_work");
        assert!(!resolved.is_default);
        let fallback = resolve_instance(&settings, None, Some("codex")).unwrap();
        assert_eq!(fallback.instance_id, "codex");
    }

    #[test]
    fn claude_config_dir_env_from_config() {
        let settings = settings_with(json!({
            "claude_work": { "driver": "claudeAgent", "config": { "configDir": "/tmp/work-claude" } },
        }));
        let instance = resolve_instance(&settings, Some("claude_work"), None).unwrap();
        let env = instance_start_environment(&instance);
        assert!(env.contains(&(
            "CLAUDE_CONFIG_DIR".to_string(),
            "/tmp/work-claude".to_string()
        )));
    }

    #[test]
    fn profile_dir_maps_per_driver() {
        assert_eq!(
            profile_directory_environment("cursor", "/p"),
            vec![("CURSOR_CONFIG_DIR".to_string(), "/p".to_string())]
        );
        assert_eq!(
            profile_directory_environment("grok", "/p"),
            vec![("GROK_HOME".to_string(), "/p".to_string())]
        );
        assert_eq!(
            profile_directory_environment("codex", "/p"),
            vec![("HOME".to_string(), "/p".to_string())]
        );
        assert!(profile_directory_environment("codex", "").is_empty());
    }
}
