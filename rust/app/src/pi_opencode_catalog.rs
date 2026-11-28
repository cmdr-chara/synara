//! Pi OpenCode model catalog, ported from upstream `piOpenCodeCatalog.ts`.
//!
//! Upstream registers the validated catalog into the in-process SDK runtime.
//! The Rust backend drives the official `pi` CLI as a subprocess instead, so
//! the same outcome is reached through the SDK's native composition point:
//! `<agentDir>/models.json`, which the official CLI merges over its built-in
//! provider base on every start. User-defined entries always win collisions,
//! matching upstream's "user registrations still win" ordering.
//!
//! Everything fetched is public metadata: no credentials are read, written,
//! or replayed — `auth.json` is only ever probed for the presence of the
//! `opencode` key, never its value.

use crate::acp_runner::err;
use crate::todo::WsErr;
use serde_json::{Value, json};
use std::sync::{LazyLock, Mutex};

const CATALOG_URL: &str = "https://pi.dev/api/models/providers/opencode";
const INVENTORY_URL: &str = "https://opencode.ai/inference/v1/models";
const CATALOG_TTL_MS: i64 = 60_000;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

fn base_urls(api: &str) -> Option<&'static [&'static str]> {
    match api {
        "anthropic-messages" => Some(&[
            "https://opencode.ai/inference/anthropic",
            "https://opencode.ai/inference",
            "https://opencode.ai/zen",
        ]),
        "google-generative-ai" => Some(&[
            "https://opencode.ai/inference/google/v1beta",
            "https://opencode.ai/inference/v1",
            "https://opencode.ai/zen/v1",
        ]),
        "openai-responses" => Some(&[
            "https://opencode.ai/inference/openai/v1",
            "https://opencode.ai/inference",
            "https://opencode.ai/zen/v1",
        ]),
        "openai-completions" => Some(&[
            "https://opencode.ai/inference/openai/v1",
            "https://opencode.ai/inference",
            "https://opencode.ai/zen/v1",
        ]),
        _ => None,
    }
}

/// Exact upstream `isCatalogModel` validation: only advertise models whose
/// complete Pi protocol metadata is known and whose baseUrl sits inside the
/// pinned allowlist for their API flavor.
fn is_catalog_model(value: &Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    let id_ok = obj
        .get("id")
        .and_then(Value::as_str)
        .map(|id| !id.is_empty() && id.trim() == id)
        .unwrap_or(false);
    let name_ok = obj
        .get("name")
        .and_then(Value::as_str)
        .map(|n| !n.trim().is_empty())
        .unwrap_or(false);
    if !id_ok || !name_ok || obj.get("provider").and_then(Value::as_str) != Some("opencode") {
        return false;
    }
    let api = obj.get("api").and_then(Value::as_str).unwrap_or("");
    let base_url = obj.get("baseUrl").and_then(Value::as_str).unwrap_or("");
    let Some(allowed) = base_urls(api) else {
        return false;
    };
    if !allowed.contains(&base_url) {
        return false;
    }
    if obj.get("reasoning").and_then(Value::as_bool).is_none() {
        return false;
    }
    let input_ok = obj
        .get("input")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter().any(|i| i.as_str() == Some("text"))
                && items
                    .iter()
                    .all(|i| matches!(i.as_str(), Some("text") | Some("image")))
        })
        .unwrap_or(false);
    if !input_ok {
        return false;
    }
    for key in ["contextWindow", "maxTokens"] {
        if obj
            .get(key)
            .and_then(Value::as_f64)
            .map(|n| n.is_finite() && n > 0.0)
            != Some(true)
        {
            return false;
        }
    }
    let Some(cost) = obj.get("cost").and_then(Value::as_object) else {
        return false;
    };
    ["input", "output", "cacheRead", "cacheWrite"]
        .iter()
        .all(|key| {
            cost.get(*key)
                .and_then(Value::as_f64)
                .map(|n| n.is_finite() && n >= 0.0)
                == Some(true)
        })
}

/// Port of `parsePiOpenCodeCatalog`: intersect the catalog with the live
/// inventory and strip anything that could supply authentication or
/// arbitrary request headers.
pub fn parse_pi_opencode_catalog(catalog: &Value, inventory: &Value) -> Result<Vec<Value>, WsErr> {
    let data = inventory.get("data").and_then(Value::as_array);
    let valid_inventory = data
        .map(|items| {
            !items.is_empty()
                && items.iter().all(|item| {
                    item.get("id")
                        .and_then(Value::as_str)
                        .map(|id| !id.trim().is_empty())
                        == Some(true)
                })
        })
        .unwrap_or(false);
    if !catalog.is_object() || !valid_inventory {
        return Err(err("Invalid OpenCode model inventory"));
    }
    let active: std::collections::HashSet<&str> = data
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    let mut models: Vec<Value> = catalog
        .as_object()
        .map(|obj| {
            obj.values()
                .filter(|v| is_catalog_model(v))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    models.retain(|m| {
        m.get("id")
            .and_then(Value::as_str)
            .map(|id| active.contains(id))
            == Some(true)
    });
    if models.is_empty() {
        return Err(err("No supported OpenCode models in catalog"));
    }
    // Public metadata must not supply authentication or arbitrary headers.
    for model in &mut models {
        if let Some(obj) = model.as_object_mut() {
            obj.remove("headers");
        }
    }
    Ok(models)
}

struct Cache {
    models: Vec<Value>,
    fetched_at_ms: i64,
}
static PUBLIC_CATALOG: LazyLock<Mutex<Option<Cache>>> = LazyLock::new(|| Mutex::new(None));

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Blocking HTTP inside an isolated thread: reqwest's blocking client panics
/// when dropped in async context. Same hardening as upstream outboundHttp:
/// 5s timeout, response byte cap, no redirects.
fn fetch_json(url: &str) -> Result<Value, WsErr> {
    let url_owned = url.to_string();
    std::thread::scope(|scope| {
        scope
            .spawn(move || -> Result<Value, String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .map_err(|e| e.to_string())?;
                let response = client.get(&url_owned).send().map_err(|e| e.to_string())?;
                if response.status().as_u16() != 200 {
                    return Err("OpenCode catalog unavailable".to_string());
                }
                let bytes = response.bytes().map_err(|e| e.to_string())?;
                if bytes.len() > MAX_RESPONSE_BYTES {
                    return Err("OpenCode catalog response too large".to_string());
                }
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())
            })
            .join()
            .map_err(|_| err("catalog fetch thread panicked"))?
            .map_err(|e| err(e))
    })
}

fn opencode_auth_present(agent_dir: &std::path::Path) -> bool {
    // Presence of the key only — the value is a credential and never read.
    std::fs::read_to_string(agent_dir.join("auth.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| v.get("opencode").map(|_| true))
        .unwrap_or(false)
}

fn write_atomic(path: &std::path::Path, content: &str) -> Result<(), WsErr> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, content).map_err(|e| err(format!("models.json write: {e}")))?;
    std::fs::rename(&tmp, path).map_err(|e| err(format!("models.json rename: {e}")))?;
    Ok(())
}

/// Refresh the OpenCode model catalog into `<agentDir>/models.json`.
/// Failures are deliberately silent like upstream: offline, malformed, or
/// unavailable public catalogs retain the SDK baseline.
pub fn refresh_pi_opencode_catalog(agent_dir: &std::path::Path) {
    if std::env::var_os("PI_OFFLINE").is_some() {
        return;
    }
    if !opencode_auth_present(agent_dir) {
        return;
    }
    let models_path = agent_dir.join("models.json");
    // If the user's models.json already carries opencode entries whose
    // baseUrl leaves the pinned allowlist, their customization wins outright.
    let existing: Value = std::fs::read_to_string(&models_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(json!({}));
    if let Some(models) = existing
        .pointer("/providers/opencode/models")
        .and_then(Value::as_array)
    {
        let foreign = models.iter().any(|m| {
            let api = m.get("api").and_then(Value::as_str).unwrap_or("");
            let base = m.get("baseUrl").and_then(Value::as_str).unwrap_or("");
            base_urls(api)
                .map(|allowed| !allowed.contains(&base))
                .unwrap_or(false)
        });
        if foreign {
            return;
        }
    }

    let cached = PUBLIC_CATALOG.lock().ok().and_then(|c| {
        c.as_ref()
            .map(|c| (c.models.clone(), now_ms() - c.fetched_at_ms))
    });
    let models = match cached {
        Some((models, age)) if age < CATALOG_TTL_MS => models,
        _ => {
            let (Ok(catalog), Ok(inventory)) = (fetch_json(CATALOG_URL), fetch_json(INVENTORY_URL))
            else {
                return;
            };
            let Ok(models) = parse_pi_opencode_catalog(&catalog, &inventory) else {
                return;
            };
            if let Ok(mut c) = PUBLIC_CATALOG.lock() {
                *c = Some(Cache {
                    models: models.clone(),
                    fetched_at_ms: now_ms(),
                });
            }
            models
        }
    };

    // Merge: user-defined per-id entries win; catalog fills the rest.
    let mut root = if existing.is_object() {
        existing.clone()
    } else {
        json!({})
    };
    let providers = root
        .as_object_mut()
        .unwrap()
        .entry("providers".to_string())
        .or_insert_with(|| json!({}));
    let opencode = providers
        .as_object_mut()
        .unwrap()
        .entry("opencode".to_string())
        .or_insert_with(|| json!({}));
    let oc_obj = opencode.as_object_mut().unwrap();
    let user_ids: std::collections::HashSet<String> = oc_obj
        .get("models")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut merged: Vec<Value> = oc_obj
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for model in models {
        let dominated = model
            .get("id")
            .and_then(Value::as_str)
            .map(|id| user_ids.contains(id))
            .unwrap_or(false);
        if !dominated {
            merged.push(model);
        }
    }
    oc_obj.insert("models".to_string(), Value::Array(merged));
    if let Some(parent) = models_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let serialized = serde_json::to_string_pretty(&root).unwrap_or_default();
    let _ = write_atomic(&models_path, &serialized);
}

/// Resolve the pi agent dir for the home a child process will inherit:
/// `PI_AGENT_DIR` wins, else `<home>/.pi/agent` — the SDK default.
pub fn agent_dir_for_home(home: &str) -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("PI_AGENT_DIR") {
        return std::path::PathBuf::from(dir);
    }
    std::path::PathBuf::from(home).join(".pi").join("agent")
}

/// Refresh for the pi child about to spawn with this instance: resolve the
/// effective HOME exactly the way the spawn environment does.
pub fn refresh_for_instance(instance_id: Option<&str>) {
    let settings = crate::server_settings::current();
    let home = crate::provider_instances::resolve_instance(&settings, instance_id, Some("pi"))
        .and_then(|instance| {
            crate::provider_instances::instance_start_environment(&instance)
                .into_iter()
                .find(|(k, _)| k == "HOME")
                .map(|(_, v)| v)
        })
        .or_else(|| std::env::var("HOME").ok());
    if let Some(home) = home {
        refresh_pi_opencode_catalog(&agent_dir_for_home(&home));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_model(id: &str) -> Value {
        json!({
            "id": id,
            "name": "Test Model",
            "provider": "opencode",
            "api": "openai-responses",
            "baseUrl": "https://opencode.ai/zen/v1",
            "reasoning": false,
            "input": ["text"],
            "contextWindow": 128000,
            "maxTokens": 8192,
            "cost": {"input": 1.0, "output": 2.0, "cacheRead": 0.5, "cacheWrite": 1.5},
        })
    }

    #[test]
    fn catalog_parsing_matches_upstream_validation() {
        let catalog = json!({
            "a": valid_model("m-alpha"),
            "b": valid_model("m-beta"),
            "bad": {"id": "evil", "provider": "opencode"},
        });
        let inventory = json!({"data": [{"id": "m-alpha"}, {"id": "m-beta"}, {"id": "inactive"}]});
        let models = parse_pi_opencode_catalog(&catalog, &inventory)
            .ok()
            .unwrap();
        assert_eq!(models.len(), 2);
        // Inventory filter: drop models the inventory does not list.
        let inventory2 = json!({"data": [{"id": "m-alpha"}]});
        let models2 = parse_pi_opencode_catalog(&catalog, &inventory2)
            .ok()
            .unwrap();
        assert_eq!(models2.len(), 1);
        assert_eq!(models2[0]["id"], "m-alpha");
    }

    #[test]
    fn headers_are_stripped_and_foreign_baseurls_rejected() {
        let mut with_headers = valid_model("m-x");
        with_headers["headers"] = json!({"Authorization": "Bearer evil"});
        let catalog = json!({"x": with_headers});
        let inventory = json!({"data": [{"id": "m-x"}]});
        let models = parse_pi_opencode_catalog(&catalog, &inventory)
            .ok()
            .unwrap();
        assert!(models[0].get("headers").is_none());

        let mut foreign = valid_model("m-y");
        foreign["baseUrl"] = json!("https://evil.example.com/v1");
        let catalog2 = json!({"y": foreign});
        let inventory2 = json!({"data": [{"id": "m-y"}]});
        assert!(parse_pi_opencode_catalog(&catalog2, &inventory2).is_err());
    }

    #[test]
    fn invalid_inventory_fails_closed() {
        let catalog = json!({"a": valid_model("m-alpha")});
        assert!(parse_pi_opencode_catalog(&catalog, &json!({"data": []})).is_err());
        assert!(parse_pi_opencode_catalog(&catalog, &json!({})).is_err());
    }

    #[test]
    fn user_entries_win_merge() {
        let dir = std::env::temp_dir().join(format!("pi-catalog-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("auth.json"), r#"{"opencode": "redacted"}"#).unwrap();
        std::fs::write(
            dir.join("models.json"),
            r#"{"providers": {"opencode": {"models": [{"id": "user-model", "api": "openai-responses", "baseUrl": "https://opencode.ai/zen/v1"}]}, "other": {"models": []}}}"#,
        )
        .unwrap();
        // Seed the cache so no network is needed.
        if let Ok(mut c) = PUBLIC_CATALOG.lock() {
            *c = Some(Cache {
                models: vec![valid_model("user-model"), valid_model("catalog-model")],
                fetched_at_ms: now_ms(),
            });
        }
        refresh_pi_opencode_catalog(&dir);
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("models.json")).unwrap())
                .unwrap();
        let ids: Vec<&str> = written
            .pointer("/providers/opencode/models")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(|m| m.get("id").and_then(Value::as_str))
            .collect();
        // User entry kept exactly once, catalog model appended, other provider intact.
        assert_eq!(ids.iter().filter(|id| **id == "user-model").count(), 1);
        assert!(ids.contains(&"catalog-model"));
        assert!(written.pointer("/providers/other/models").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn foreign_user_baseurl_skips_refresh() {
        let dir =
            std::env::temp_dir().join(format!("pi-catalog-test-foreign-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("auth.json"), r#"{"opencode": "redacted"}"#).unwrap();
        let original = r#"{"providers": {"opencode": {"models": [{"id": "custom", "api": "openai-responses", "baseUrl": "https://my-proxy.example.com/v1"}]}}}"#;
        std::fs::write(dir.join("models.json"), original).unwrap();
        if let Ok(mut c) = PUBLIC_CATALOG.lock() {
            *c = Some(Cache {
                models: vec![valid_model("catalog-model")],
                fetched_at_ms: now_ms(),
            });
        }
        refresh_pi_opencode_catalog(&dir);
        let after = std::fs::read_to_string(dir.join("models.json")).unwrap();
        assert_eq!(after, original);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
