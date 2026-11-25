//! Provider instance resolution, ported from upstream
//! `shared/providerInstances.ts`. Server settings carry a `providerInstances`
//! map; each entry is an account boundary with its own driver, config, and
//! environment. Instance environments never include redacted values, and a
//! non-default instance forces an isolated provider home so credentials and
//! CLI state never mix across accounts.

use serde_json::{json, Value};

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
    let mut out = Vec::new();
    let Some(map) = settings.get("providerInstances").and_then(Value::as_object) else {
        return out;
    };
    for (instance_id, raw) in map {
        if is_unresolved_automation_instance_id(instance_id) {
            continue;
        }
        let driver = raw.get("driver").and_then(Value::as_str).unwrap_or("");
        if !BUILT_IN_PROVIDER_KINDS.contains(&driver) {
            continue;
        }
        let config = raw.get("config").filter(|c| c.is_object()).cloned().unwrap_or(json!({}));
        let enabled = raw.get("enabled").and_then(Value::as_bool) != Some(false)
            && config.get("enabled").and_then(Value::as_bool) != Some(false);
        out.push(ResolvedInstance {
            instance_id: instance_id.clone(),
            driver: driver.to_string(),
            enabled,
            is_default: instance_id == driver,
            config,
            environment: materialize_environment(raw),
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
        .or_else(|| instances.iter().find(|i| i.driver == requested && i.is_default))
        .or_else(|| instances.iter().find(|i| i.instance_id == "codex"))
        .or_else(|| instances.first())
        .cloned()
}

/// Environment keys a profile directory implies per provider, upstream
/// `providerProfileDirectoryEnvironment`.
pub(crate) fn profile_directory_environment(driver: &str, profile_dir: &str) -> Vec<(String, String)> {
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
        env.extend(profile_directory_environment(&instance.driver, &profile_dir));
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
        "codex" => trim_config(&instance.config, "homePath").or_else(|| trim_config(&instance.config, "profileDir")),
        _ => trim_config(&instance.config, "homePath"),
    }
}

/// Resolve the default instance for a driver and apply its start
/// environment to a child command, after spawn sanitization. Shared by all
/// runtime spawners; driver-specific home isolation beyond environment
/// configuration (Codex overlays, Claude instance homes) stays in the
/// owning runner.
pub(crate) fn apply_instance_environment(command: &mut std::process::Command, driver: &str) {
    let settings = crate::server_settings::current();
    if let Some(instance) = resolve_instance(&settings, None, Some(driver)) {
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
        assert_eq!(instances.len(), 1);
        assert!(instances[0].is_default);
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
        assert_eq!(instances[0].environment, vec![("VISIBLE".to_string(), "yes".to_string())]);
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
        assert!(env.contains(&("CLAUDE_CONFIG_DIR".to_string(), "/tmp/work-claude".to_string())));
    }

    #[test]
    fn profile_dir_maps_per_driver() {
        assert_eq!(profile_directory_environment("cursor", "/p"), vec![("CURSOR_CONFIG_DIR".to_string(), "/p".to_string())]);
        assert_eq!(profile_directory_environment("grok", "/p"), vec![("GROK_HOME".to_string(), "/p".to_string())]);
        assert_eq!(profile_directory_environment("codex", "/p"), vec![("HOME".to_string(), "/p".to_string())]);
        assert!(profile_directory_environment("codex", "").is_empty());
    }
}
