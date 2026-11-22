//! `server.getSettings`, `server.updateSettings`, `server.subscribeSettings`
//! — the server-owned settings document persisted at
//! `<synara-home>/userdata/settings.json` as
//! `{ revision, migrationVersion, settings: {...} }`.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use tokio::sync::broadcast;

use crate::todo::WsErr;

const MIGRATION_VERSION: u64 = 3;
const MAX_SEND: usize = 16;

fn settings_path() -> PathBuf {
    PathBuf::from(crate::todo::state_dir()).join("settings.json")
}

fn secrets_dir() -> PathBuf {
    PathBuf::from(crate::todo::state_dir()).join("secrets")
}

fn default_settings() -> Value {
    json!({
        "autoResumeAfterUsageLimits": false,
        "usageLimitRecoveryEpoch": 0,
        "enableAssistantStreaming": true,
        "enableProviderUpdateChecks": true,
        "keepAwakeMode": "off",
        "lowerProviderProcessPriority": true,
        "defaultThreadEnvMode": "local",
        "addProjectBaseDirectory": "",
        "githubInboxIncludeUpstreams": false,
        "sidechatExpiry": "1h",
        "sourceControlWritingStyle": "repository",
        "sourceControlCustomInstructions": "",
        "gitAttributionEnabled": true,
        "textGenerationModelSelection": {"provider": "codex", "model": "gpt-6-luna"},
        "providers": {
            "codex": {"enabled": true, "binaryPath": "codex", "customModels": [], "homePath": "", "accounts": [], "selectedAccountId": "default"},
            "claudeAgent": {"enabled": true, "binaryPath": "claude", "customModels": [], "homePath": "", "launchArgs": "", "enableArtifacts": false, "confirmColdCacheResume": true},
            "cursor": {"enabled": true, "binaryPath": "cursor-agent", "customModels": [], "apiEndpoint": ""},
            "devin": {"enabled": true, "binaryPath": "devin", "customModels": []},
            "antigravity": {"enabled": true, "binaryPath": "agy", "customModels": []},
            "grok": {"enabled": true, "binaryPath": "grok", "customModels": []},
            "droid": {"enabled": true, "binaryPath": "droid", "customModels": []},
            "opencode": {"enabled": true, "binaryPath": "opencode", "customModels": [], "serverUrl": "", "serverPasswordConfigured": false, "experimentalWebSockets": false},
            "pi": {"enabled": true, "binaryPath": "pi", "customModels": [], "agentDir": ""},
            "omp": {"enabled": true, "binaryPath": "omp", "customModels": [], "agentDir": ""},
        },
        "providerInstances": {},
        "skills": {"disabled": []},
        "newThreadDefaults": {"modelSelectionByProvider": {}, "activeProvider": null, "runtimeMode": null},
    })
}

struct SettingsDoc {
    revision: u64,
    settings: Value,
}

fn load_doc() -> SettingsDoc {
    let path = settings_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return SettingsDoc {
            revision: 0,
            settings: default_settings(),
        };
    };
    let Ok(doc) = serde_json::from_str::<Value>(&text) else {
        return SettingsDoc {
            revision: 0,
            settings: default_settings(),
        };
    };
    let revision = doc.get("revision").and_then(|v| v.as_u64()).unwrap_or(0);
    let settings = doc
        .get("settings")
        .cloned()
        .unwrap_or_else(default_settings);
    SettingsDoc { revision, settings }
}

struct Store {
    doc: Mutex<SettingsDoc>,
    changes: broadcast::Sender<Value>,
}

static STORE: LazyLock<Store> = LazyLock::new(|| {
    let (changes, _) = broadcast::channel(MAX_SEND);
    Store {
        doc: Mutex::new(load_doc()),
        changes,
    }
});

fn persist(doc: &SettingsDoc) {
    let path = settings_path();
    let body = json!({
        "revision": doc.revision,
        "migrationVersion": MIGRATION_VERSION,
        "settings": doc.settings,
    });
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&body) {
        let _ = std::fs::write(&path, text);
    }
}

const PATCH_KEYS: &[&str] = &[
    "autoResumeAfterUsageLimits",
    "enableAssistantStreaming",
    "enableProviderUpdateChecks",
    "keepAwakeMode",
    "lowerProviderProcessPriority",
    "defaultThreadEnvMode",
    "addProjectBaseDirectory",
    "githubInboxIncludeUpstreams",
    "sidechatExpiry",
    "sourceControlWritingStyle",
    "sourceControlCustomInstructions",
    "gitAttributionEnabled",
    "textGenerationModelSelection",
    "providers",
    "providerInstances",
    "skills",
    "newThreadDefaults",
    "onboardingCompletedAt",
];

/// Patch merge where every top-level key is a field patch; `providers` and
/// `providerInstances` merge per provider/field, `textGenerationModelSelection`
/// and `newThreadDefaults` merge one level deeper — and
/// `newThreadDefaults.modelSelectionByProvider` merges per provider key.
fn apply_patch(settings: &mut Value, patch: &Value) {
    let Some(obj) = patch.as_object() else {
        return;
    };
    for (key, value) in obj {
        if !PATCH_KEYS.contains(&key.as_str()) {
            continue;
        }
        match key.as_str() {
            "providers" | "providerInstances" => {
                let Some(providers_patch) = value.as_object() else {
                    continue;
                };
                let Some(dst) = settings
                    .as_object_mut()
                    .and_then(|o| o.get_mut(key))
                    .and_then(|v| v.as_object_mut())
                else {
                    settings
                        .as_object_mut()
                        .unwrap()
                        .insert(key.clone(), value.clone());
                    continue;
                };
                for (name, provider_patch) in providers_patch {
                    let entry = dst.entry(name.clone()).or_insert_with(|| json!({}));
                    if let (Some(e_obj), Some(p_obj)) =
                        (entry.as_object_mut(), provider_patch.as_object())
                    {
                        for (k, v) in p_obj {
                            if k == "serverPassword" {
                                continue;
                            }
                            e_obj.insert(k.clone(), v.clone());
                        }
                    } else {
                        dst.insert(name.clone(), provider_patch.clone());
                    }
                }
            }
            "textGenerationModelSelection" => {
                let entry = settings
                    .as_object_mut()
                    .unwrap()
                    .entry(key.clone())
                    .or_insert_with(|| json!({}));
                if let (Some(e_obj), Some(p_obj)) = (entry.as_object_mut(), value.as_object()) {
                    for (k, v) in p_obj {
                        e_obj.insert(k.clone(), v.clone());
                    }
                } else {
                    settings
                        .as_object_mut()
                        .unwrap()
                        .insert(key.clone(), value.clone());
                }
            }
            "newThreadDefaults" => {
                let entry = settings
                    .as_object_mut()
                    .unwrap()
                    .entry(key.clone())
                    .or_insert_with(|| json!({}));
                let Some(p_obj) = value.as_object() else {
                    continue;
                };
                let e_obj = entry.as_object_mut().unwrap();
                for (k, v) in p_obj {
                    if k == "modelSelectionByProvider" {
                        if let (Some(dst_map), Some(src_map)) = (
                            e_obj.get_mut(k).and_then(|m| m.as_object_mut()),
                            v.as_object(),
                        ) {
                            for (provider, selection) in src_map {
                                dst_map.insert(provider.clone(), selection.clone());
                            }
                            continue;
                        }
                    }
                    e_obj.insert(k.clone(), v.clone());
                }
            }
            _ => {
                settings
                    .as_object_mut()
                    .unwrap()
                    .insert(key.clone(), value.clone());
            }
        }
    }
}

pub fn get_settings(_payload: &Value) -> Result<Value, WsErr> {
    let doc = STORE.doc.lock().unwrap();
    Ok(doc.settings.clone())
}

pub fn subscribe() -> (Value, broadcast::Receiver<Value>) {
    let snapshot = {
        let doc = STORE.doc.lock().unwrap();
        doc.settings.clone()
    };
    (snapshot, STORE.changes.subscribe())
}

pub fn current() -> Value {
    STORE.doc.lock().unwrap().settings.clone()
}

pub fn changes() -> broadcast::Receiver<Value> {
    STORE.changes.subscribe()
}

pub fn update_settings(payload: &Value) -> Result<Value, WsErr> {
    let legacy_ack = payload.get("legacyProviders").is_some()
        || payload.get("legacyProviderInstances").is_some();
    let mut patch = payload.clone();
    if let Some(obj) = patch.as_object_mut() {
        obj.remove("legacyProviders");
        obj.remove("legacyProviderInstances");
    }
    // A provided opencode password moves into the secrets store; the settings
    // document only records that one is configured.
    if let Some(password) = payload
        .pointer("/providers/opencode/serverPassword")
        .and_then(|v| v.as_str())
    {
        if !password.is_empty() {
            let dir = secrets_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("opencode-server-password"), password);
            if let Some(obj) = patch
                .pointer_mut("/providers/opencode")
                .and_then(|v| v.as_object_mut())
            {
                obj.remove("serverPassword");
                obj.insert("serverPasswordConfigured".to_string(), json!(true));
            }
        }
    }
    let settings = {
        let mut doc = STORE.doc.lock().unwrap();
        apply_patch(&mut doc.settings, &patch);
        // Legacy provider imports land as the authoritative provider config.
        if let Some(providers) = payload.get("legacyProviders") {
            apply_patch(&mut doc.settings, &json!({ "providers": providers }));
        }
        if let Some(instances) = payload.get("legacyProviderInstances") {
            apply_patch(
                &mut doc.settings,
                &json!({ "providerInstances": instances }),
            );
        }
        doc.revision += 1;
        persist(&doc);
        doc.settings.clone()
    };
    let _ = STORE.changes.send(settings.clone());
    let mut result = settings;
    if legacy_ack {
        result["legacyProviderImportAcknowledged"] = json!(true);
    }
    Ok(result)
}
