//! Provider data served from on-disk caches the upstream writes under
//! `userdata/`: `provider-status/*.json` snapshots (already wire-shaped
//! ServerProviderStatus objects) and `provider-models/catalogs.json`
//! (normalized model catalogs). Lets listModels and the statuses stream
//! answer without provider CLIs installed.

use rusqlite::params;
use serde_json::{Value, json};

use crate::orchestration_snapshot::open_state_copy;
use crate::todo::{WsErr, state_dir};

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("PROVIDER"))
}

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "provider.listModels"
            | "server.listExternalMcpIntegrations"
            | "server.refreshProviders"
            | "server.updateProvider"
    )
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "provider.listModels" => list_models(payload),
        "server.listExternalMcpIntegrations" => list_external_mcp_integrations(),
        "server.refreshProviders" => refresh_providers(),
        "server.updateProvider" => update_provider(payload),
        _ => Err(err("unhandled provider tag")),
    }
}

/// All provider-status snapshots on disk, wire order (sorted by file name
/// which is `<provider>.json` or `<instanceId>.json`).
pub(crate) fn provider_statuses() -> Vec<Value> {
    let dir = std::path::PathBuf::from(state_dir()).join("provider-status");
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return out;
    };
    let mut names: Vec<_> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    names.sort();
    for path in names {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                out.push(v);
            }
        }
    }
    out
}

/// `provider.listModels`: the upstream-normalized catalog cache keyed by a
/// JSON array whose first two elements are provider and instanceId.
fn list_models(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if provider.is_empty() {
        return Err(err("provider is required"));
    }
    let instance = payload
        .get("instanceId")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| provider.clone());
    let path = std::path::PathBuf::from(state_dir())
        .join("provider-models")
        .join("catalogs.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Err(err("no provider model catalog cache"));
    };
    let doc: Value = serde_json::from_str(&text).map_err(|e| err(e.to_string()))?;
    let entries = doc.get("entries").and_then(|e| e.as_array());
    let Some(entries) = entries else {
        return Err(err("malformed provider model catalog cache"));
    };
    for entry in entries {
        let key = entry.get("key").and_then(|k| k.as_str()).unwrap_or("");
        let Ok(parts) = serde_json::from_str::<Vec<Value>>(key) else {
            continue;
        };
        let k_provider = parts.first().and_then(|v| v.as_str()).unwrap_or("");
        let k_instance = parts.get(1).and_then(|v| v.as_str()).unwrap_or("");
        if k_provider != provider || k_instance != instance {
            continue;
        }
        let models = entry
            .get("result")
            .and_then(|r| r.get("models"))
            .cloned()
            .unwrap_or(json!([]));
        return Ok(json!({
            "models": models,
            "source": "cache",
            "cached": true,
        }));
    }
    Err(err(format!(
        "no cached model catalog for {provider}/{instance}"
    )))
}

/// `server.listExternalMcpIntegrations`: rows from external_mcp_integrations
/// with their project grants.
fn list_external_mcp_integrations() -> Result<Value, WsErr> {
    let Some(copy) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &copy.conn;
    let mut stmt = conn
        .prepare(
            "SELECT integration_id, name, audience, client_kind, capabilities_json, \
                    project_scope, created_at, expires_at, last_used_at, paired_at, \
                    revoked_at, rate_limit_per_minute, concurrency_limit \
             FROM external_mcp_integrations ORDER BY created_at",
        )
        .map_err(|e| err(e.to_string()))?;
    let mut rows = stmt.query([]).map_err(|e| err(e.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
        let id: String = row.get(0).unwrap_or_default();
        let capabilities: Value = row
            .get::<_, String>(4)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(json!([]));
        let mut grants = Vec::new();
        if let Ok(mut ps) = conn.prepare(
            "SELECT p.project_id, COALESCE(pr.title, p.project_id) \
             FROM external_mcp_integration_projects p \
             LEFT JOIN projection_projects pr ON pr.project_id = p.project_id \
             WHERE p.integration_id = ?1 ORDER BY p.project_id",
        ) {
            if let Ok(mapped) = ps.query_map(params![id.clone()], |r| {
                Ok(json!({
                    "projectId": r.get::<_, String>(0)?,
                    "title": r.get::<_, String>(1)?,
                }))
            }) {
                grants = mapped.flatten().collect();
            }
        }
        out.push(json!({
            "integrationId": id,
            "name": row.get::<_, String>(1).unwrap_or_default(),
            "audience": row.get::<_, String>(2).unwrap_or_default(),
            "capabilities": capabilities,
            "projectScope": row.get::<_, String>(5).unwrap_or_else(|_| "selected".into()),
            "allowedProjects": grants,
            "createdAt": row.get::<_, String>(6).unwrap_or_default(),
            "expiresAt": row.get::<_, String>(7).unwrap_or_default(),
            "lastUsedAt": row.get::<_, Option<String>>(8).unwrap_or(None),
            "pairedAt": row.get::<_, Option<String>>(9).unwrap_or(None),
            "revokedAt": row.get::<_, Option<String>>(10).unwrap_or(None),
            "rateLimitPerMinute": row.get::<_, i64>(11).unwrap_or(0),
            "concurrencyLimit": row.get::<_, i64>(12).unwrap_or(0),
            "clientKind": row.get::<_, String>(3).unwrap_or_default(),
            "stdio": {"command": "", "args": []},
        }));
    }
    Ok(Value::Array(out))
}

/// `server.refreshProviders`: re-probe each provider's configured binary and
/// rewrite its provider-status snapshot in place, preserving fields only a
/// live probe can know (auth, version advisory).
fn refresh_providers() -> Result<Value, WsErr> {
    let dir = std::path::PathBuf::from(state_dir()).join("provider-status");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Ok(json!({"providers": []}));
    };
    let settings = std::fs::read_to_string(
        std::path::PathBuf::from(state_dir()).join("settings.json"),
    )
    .ok()
    .and_then(|t| serde_json::from_str::<Value>(&t).ok());
    let mut paths: Vec<_> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    paths.sort();
    for path in paths {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut snap) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let provider = snap
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        // Non-CLI providers (apiKey auth) are not probed for a binary.
        if snap.get("authType").and_then(|v| v.as_str()) == Some("apiKey") {
            continue;
        }
        let binary = settings
            .as_ref()
            .and_then(|v| v.pointer(&format!("/settings/providers/{provider}/binaryPath")))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                snap.get("autoRuntimeModeBinaryPath")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            });
        let Some(binary) = binary else {
            continue;
        };
        let enabled = settings
            .as_ref()
            .and_then(|v| v.pointer(&format!("/settings/providers/{provider}/enabled")))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let installed = binary_on_path(&binary);
        snap["installed"] = json!(installed);
        snap["enabled"] = json!(enabled);
        snap["checkedAt"] = json!(iso_now());
        if installed {
            snap["status"] = json!("ready");
            snap["available"] = json!(enabled);
            snap["message"] = Value::Null;
        } else {
            snap["status"] = json!("error");
            snap["available"] = json!(false);
            snap["message"] = json!(format!(
                "{} CLI (`{}`) is not installed or not on PATH.",
                snap.get("displayName").and_then(|v| v.as_str()).unwrap_or(&provider),
                binary
            ));
        }
        if let Ok(t) = serde_json::to_string_pretty(&snap) {
            let _ = std::fs::write(&path, t);
        }
    }
    Ok(json!({"providers": provider_statuses()}))
}

/// `server.updateProvider`: run the status snapshot's recorded update
/// command, then persist the outcome on `updateState`.
fn update_provider(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if provider.is_empty() {
        return Err(WsErr("provider is required".into(), Some("VALIDATION")));
    }
    let instance = payload
        .get("instanceId")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| provider.clone());
    let path = std::path::PathBuf::from(state_dir())
        .join("provider-status")
        .join(format!("{instance}.json"));
    let text = std::fs::read_to_string(&path)
        .map_err(|_| err(format!("no provider-status snapshot for {instance}")))?;
    let mut snap: Value = serde_json::from_str(&text).map_err(|e| err(e.to_string()))?;
    let cmd = snap
        .pointer("/versionAdvisory/updateCommand")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let Some(cmd) = cmd else {
        return Err(WsErr(
            format!("no update command available for {instance}"),
            Some("ServerProviderUpdateError"),
        ));
    };
    let started = iso_now();
    let out = std::process::Command::new("sh")
        .args(["-c", &cmd])
        .output();
    let (status, output) = match out {
        Ok(o) => (
            if o.status.success() { "succeeded" } else { "failed" },
            String::from_utf8_lossy(&o.stdout).to_string()
                + &String::from_utf8_lossy(&o.stderr),
        ),
        Err(e) => ("failed", e.to_string()),
    };
    snap["updateState"] = json!({
        "status": status,
        "startedAt": started,
        "finishedAt": iso_now(),
        "message": Value::Null,
        "output": output.chars().take(10_000).collect::<String>(),
    });
    if let Ok(t) = serde_json::to_string_pretty(&snap) {
        let _ = std::fs::write(&path, t);
    }
    Ok(json!({"providers": provider_statuses()}))
}

fn binary_on_path(binary: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {}", shell_escape(binary))])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn iso_now() -> String {
    // RFC3339 UTC seconds.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let t = secs % 86400;
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        t / 3600,
        (t % 3600) / 60,
        t % 60
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
