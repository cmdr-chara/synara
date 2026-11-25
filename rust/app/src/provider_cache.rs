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
            | "provider.compactThread"
            | "provider.getComposerCapabilities"
            | "provider.listCommands"
            | "provider.prewarmThread"
            | "server.listExternalMcpIntegrations"
            | "server.refreshProviders"
            | "server.updateProvider"
    ) || (std::env::var("SYNARA_NO_UPSTREAM").is_ok()
        && matches!(
            tag,
            "server.createExternalMcpIntegration"
                | "server.revokeExternalMcpIntegration"
                | "server.refreshExternalMcpPairing"
        ))
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "provider.listModels" => list_models(payload),
        "provider.compactThread" => compact_thread(payload),
        "provider.getComposerCapabilities" => composer_capabilities(payload),
        "provider.listCommands" => list_commands(payload),
        "provider.prewarmThread" => prewarm_thread(payload),
        "server.listExternalMcpIntegrations" => list_external_mcp_integrations(),
        "server.createExternalMcpIntegration" => create_external_mcp_integration(payload),
        "server.revokeExternalMcpIntegration" => revoke_external_mcp_integration(payload),
        "server.refreshExternalMcpPairing" => refresh_external_mcp_pairing(payload),
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


fn opencode_binary() -> String {
    for key in ["SYNARA_OPENCODE_CLI", "OPENCODE_BINARY_PATH"] {
        if let Ok(path) = std::env::var(key) {
            if !path.trim().is_empty() {
                return path;
            }
        }
    }
    "opencode".to_string()
}

fn fallback_model_name(slug: &str) -> String {
    slug.rsplit('/').next().unwrap_or(slug).to_string()
}

fn parse_opencode_models_output(output: &str) -> Vec<Value> {
    let lines: Vec<&str> = output.lines().collect();
    let mut models = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let candidate = lines[index].trim();
        index += 1;
        let Some((provider_id, model_id)) = candidate.split_once('/') else {
            continue;
        };
        if provider_id.is_empty() || model_id.is_empty() || candidate.contains(char::is_whitespace) {
            continue;
        }
        let mut object = Value::Null;
        if index < lines.len() && lines[index].trim_start().starts_with('{') {
            let mut json_text = String::new();
            let mut depth = 0i32;
            let mut started = false;
            while index < lines.len() {
                let line = lines[index];
                index += 1;
                json_text.push_str(line);
                json_text.push('\n');
                for ch in line.chars() {
                    match ch {
                        '{' => { depth += 1; started = true; }
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                if started && depth <= 0 {
                    break;
                }
            }
            object = serde_json::from_str(&json_text).unwrap_or(Value::Null);
        }
        let variants = object
            .get("variants")
            .and_then(Value::as_object)
            .map(|variants| variants.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let efforts = object
            .get("reasoning_options")
            .or_else(|| object.get("reasoningOptions"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut model = json!({
            "slug": candidate,
            "providerID": object.get("providerID").and_then(Value::as_str).unwrap_or(provider_id),
            "modelID": object.get("id").and_then(Value::as_str).unwrap_or(model_id),
            "name": object.get("name").and_then(Value::as_str).unwrap_or_else(|| fallback_model_name(candidate).leak()),
            "variants": variants,
            "supportedReasoningEfforts": efforts,
        });
        if let Some(default_effort) = object
            .get("defaultReasoningEffort")
            .or_else(|| object.get("default_reasoning_effort"))
            .cloned()
        {
            model["defaultReasoningEffort"] = default_effort;
        }
        models.push(model);
    }
    models.sort_by(|a, b| {
        a.get("name").and_then(Value::as_str).unwrap_or("")
            .cmp(b.get("name").and_then(Value::as_str).unwrap_or(""))
            .then(a.get("slug").and_then(Value::as_str).unwrap_or("")
                .cmp(b.get("slug").and_then(Value::as_str).unwrap_or("")))
    });
    models.dedup_by(|a, b| a.get("slug") == b.get("slug"));
    models
}

fn discover_opencode_models() -> Result<Vec<Value>, WsErr> {
    let binary = opencode_binary();
    let mut output = std::process::Command::new(&binary)
        .args(["models", "--verbose"])
        .output()
        .map_err(|e| err(format!("failed to execute opencode models: {e}")))?;
    if !output.status.success() {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_lowercase();
        if combined.contains("unknown") && combined.contains("verbose") {
            output = std::process::Command::new(&binary)
                .args(["models"])
                .output()
                .map_err(|e| err(format!("failed to execute opencode models: {e}")))?;
        }
    }
    if !output.status.success() {
        return Err(err(format!(
            "opencode models exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let models = parse_opencode_models_output(&stdout);
    if models.is_empty() {
        Err(err("opencode models returned no models"))
    } else {
        Ok(models)
    }
}

fn pi_discovery_result(payload: &Value) -> Result<Value, WsErr> {
    let cwd = payload.get("cwd").and_then(Value::as_str).unwrap_or("");
    let models = crate::pi_runner::discover_models(cwd)?;
    Ok(json!({
        "models": models,
        "source": "cli-rpc",
        "cached": false,
    }))
}

fn opencode_discovery_result() -> Result<Value, WsErr> {
    let models = discover_opencode_models()?;
    Ok(json!({
        "models": models,
        "source": "cli",
        "cached": false,
    }))
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
        if provider == "opencode" {
            return opencode_discovery_result();
        }
        if provider == "pi" {
            return pi_discovery_result(payload);
        }
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
    if provider == "opencode" {
        return opencode_discovery_result();
    }
    if provider == "pi" {
        return pi_discovery_result(payload);
    }
    Err(err(format!(
        "no cached model catalog for {provider}/{instance}"
    )))
}



fn composer_capabilities(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(Value::as_str)
        .ok_or_else(|| err("provider is required"))?;
    // Mirrors the static capability declarations in the upstream provider
    // adapters. Optional compaction/import fields are omitted by some older
    // adapters upstream; exposing false here is equivalent for clients.
    let caps = match provider {
        "antigravity" => json!({"provider":provider,"supportsSkillMentions":true,"supportsSkillDiscovery":true,"supportsNativeSlashCommandDiscovery":false,"supportsPluginMentions":false,"supportsPluginDiscovery":false,"supportsRuntimeModelList":true,"supportsThreadCompaction":false,"supportsThreadImport":false}),
        "claudeAgent" => json!({"provider":provider,"supportsSkillMentions":false,"supportsSkillDiscovery":false,"supportsNativeSlashCommandDiscovery":true,"supportsPluginMentions":false,"supportsPluginDiscovery":false,"supportsRuntimeModelList":true,"supportsThreadCompaction":false,"supportsThreadImport":true}),
        "codex" => json!({"provider":provider,"supportsSkillMentions":true,"supportsSkillDiscovery":true,"supportsNativeSlashCommandDiscovery":false,"supportsPluginMentions":true,"supportsPluginDiscovery":true,"supportsRuntimeModelList":true,"supportsThreadCompaction":false,"supportsThreadImport":false}),
        "cursorAgent" => json!({"provider":provider,"supportsSkillMentions":true,"supportsSkillDiscovery":true,"supportsNativeSlashCommandDiscovery":false,"supportsPluginMentions":false,"supportsPluginDiscovery":false,"supportsRuntimeModelList":true,"supportsThreadCompaction":false,"supportsThreadImport":true}),
        "devin" => json!({"provider":provider,"supportsSkillMentions":false,"supportsSkillDiscovery":false,"supportsNativeSlashCommandDiscovery":true,"supportsPluginMentions":false,"supportsPluginDiscovery":false,"supportsRuntimeModelList":true,"supportsThreadCompaction":true,"supportsThreadImport":false}),
        "droid" => json!({"provider":provider,"supportsSkillMentions":false,"supportsSkillDiscovery":false,"supportsNativeSlashCommandDiscovery":true,"supportsPluginMentions":true,"supportsPluginDiscovery":true,"supportsRuntimeModelList":true,"supportsThreadCompaction":false,"supportsThreadImport":true}),
        "grok" => json!({"provider":provider,"supportsSkillMentions":false,"supportsSkillDiscovery":false,"supportsNativeSlashCommandDiscovery":false,"supportsPluginMentions":false,"supportsPluginDiscovery":false,"supportsRuntimeModelList":true,"supportsThreadCompaction":true,"supportsThreadImport":false}),
        "omp" => json!({"provider":provider,"supportsSkillMentions":false,"supportsSkillDiscovery":false,"supportsNativeSlashCommandDiscovery":true,"supportsPluginMentions":false,"supportsPluginDiscovery":false,"supportsRuntimeModelList":true,"supportsThreadCompaction":false,"supportsThreadImport":true}),
        "opencode" => json!({"provider":provider,"supportsSkillMentions":false,"supportsSkillDiscovery":false,"supportsNativeSlashCommandDiscovery":true,"supportsPluginMentions":false,"supportsPluginDiscovery":false,"supportsRuntimeModelList":true,"supportsThreadCompaction":true,"supportsThreadImport":true}),
        "pi" => json!({"provider":provider,"supportsSkillMentions":true,"supportsSkillDiscovery":true,"supportsNativeSlashCommandDiscovery":true,"supportsPluginMentions":false,"supportsPluginDiscovery":false,"supportsRuntimeModelList":true,"supportsThreadCompaction":true,"supportsThreadImport":false}),
        _ => return Err(err(format!("unsupported provider '{provider}'"))),
    };
    Ok(caps)
}


fn list_commands(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(Value::as_str)
        .ok_or_else(|| err("provider is required"))?;
    let cwd = payload.get("cwd").and_then(Value::as_str).unwrap_or("");
    if provider == "pi" {
        return Ok(json!({
            "commands": crate::pi_runner::discover_commands(cwd)?,
            "source": "cli-rpc",
            "cached": false,
        }));
    }
    Ok(json!({"commands": [], "source": "unsupported", "cached": false}))
}


fn prewarm_thread(payload: &Value) -> Result<Value, WsErr> {
    let thread_id = payload
        .get("threadId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("threadId is required"))?;
    let provider = {
        let conn = crate::orchestration_engine::open_state_rw()?;
        conn.query_row(
            "SELECT COALESCE(                (SELECT provider_name FROM projection_thread_sessions WHERE thread_id=?1),                json_extract((SELECT model_selection_json FROM projection_threads WHERE thread_id=?1), '$.provider'),                ''             )",
            params![thread_id],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default()
    };
    let started = match provider.as_str() {
        "pi" => crate::pi_runner::prewarm_thread(thread_id)?,
        _ => false,
    };
    Ok(json!({"started": started}))
}

fn compact_thread(payload: &Value) -> Result<Value, WsErr> {
    let thread_id = payload
        .get("threadId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("threadId is required"))?;
    let provider = {
        let conn = crate::orchestration_engine::open_state_rw()?;
        conn.query_row(
            "SELECT provider_name FROM projection_thread_sessions WHERE thread_id=?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or_default()
    };
    let compacted = match provider.as_str() {
        "opencode" => crate::opencode_runner::compact_thread(thread_id)?,
        "pi" => crate::pi_runner::compact_thread(thread_id)?,
        _ => false,
    };
    Ok(json!({"compacted": compacted}))
}

/// External MCP helpers mirroring `externalMcp/Layers/ExternalMcpService.ts`
/// and `launcher.ts`: hashed opaque secrets, the `mcp serve`/`mcp pair`
/// stdio configs, and the record → `ExternalMcpIntegration` view.
const EXTERNAL_MCP_AUDIENCE: &str = "synara.external-mcp";
const PAIRING_PREFIX: &str = "syn_pair_v1_";
const PAIRING_TTL_MS: i64 = 10 * 60 * 1_000;
const DEFAULT_EXPIRY_DAYS: i64 = 30;
const DEFAULT_RATE_LIMIT_PER_MINUTE: i64 = 60;
const DEFAULT_ACTIVE_TASK_LIMIT: i64 = 2;

fn mcp_home_dir() -> String {
    // `state_dir()` is `<home>/userdata`; the launcher's `--home-dir` is the
    // parent, matching ServerConfig.baseDir.
    state_dir().trim_end_matches("/userdata").to_string()
}

fn hash_external_mcp_secret(secret: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(EXTERNAL_MCP_AUDIENCE.as_bytes());
    h.update(b"\0");
    h.update(secret.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn base64url(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(T[(b0 >> 2) as usize] as char);
        out.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(T[(b2 & 63) as usize] as char);
        }
    }
    out
}

fn random_opaque(prefix: &str) -> String {
    let mut bytes = [0u8; 32];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut bytes);
    }
    format!("{prefix}{}", base64url(&bytes))
}

/// `externalMcpLauncher`: the native binary stands in for
/// `process.execPath` + the server entry script.
fn mcp_stdio(args: &[&str]) -> Value {
    let command = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "synara".to_string());
    let args: Vec<Value> = args.iter().map(|a| json!(a)).collect();
    json!({"command": command, "args": args})
}

fn mcp_serve_stdio(integration_id: &str) -> Value {
    mcp_stdio(&[
        "mcp",
        "serve",
        "--integration",
        integration_id,
        "--home-dir",
        &mcp_home_dir(),
    ])
}

fn mcp_pair_stdio(pairing_code: &str) -> Value {
    mcp_stdio(&[
        "mcp",
        "pair",
        "--code",
        pairing_code,
        "--home-dir",
        &mcp_home_dir(),
    ])
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn mcp_shell_command(config: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(c) = config.get("command").and_then(Value::as_str) {
        parts.push(shell_quote(c));
    }
    if let Some(args) = config.get("args").and_then(Value::as_array) {
        for a in args {
            if let Some(a) = a.as_str() {
                parts.push(shell_quote(a));
            }
        }
    }
    parts.join(" ")
}

fn load_active_projects(conn: &rusqlite::Connection) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT project_id, title FROM projection_projects \
         WHERE deleted_at IS NULL ORDER BY created_at ASC, project_id ASC",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) {
            out = rows.flatten().collect();
        }
    }
    out
}

struct McpIntegrationRow {
    integration_id: String,
    name: String,
    audience: String,
    client_kind: String,
    capabilities: Value,
    project_scope: String,
    created_at: String,
    expires_at: String,
    last_used_at: Option<String>,
    paired_at: Option<String>,
    revoked_at: Option<String>,
    rate_limit_per_minute: i64,
    concurrency_limit: i64,
}

fn read_integration_row(row: &rusqlite::Row) -> McpIntegrationRow {
    McpIntegrationRow {
        integration_id: row.get(0).unwrap_or_default(),
        name: row.get(1).unwrap_or_default(),
        audience: row.get(2).unwrap_or_default(),
        client_kind: row.get(3).unwrap_or_default(),
        capabilities: row
            .get::<_, String>(4)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(json!([])),
        project_scope: row.get(5).unwrap_or_else(|_| "selected".into()),
        created_at: row.get(6).unwrap_or_default(),
        expires_at: row.get(7).unwrap_or_default(),
        last_used_at: row.get(8).unwrap_or(None),
        paired_at: row.get(9).unwrap_or(None),
        revoked_at: row.get(10).unwrap_or(None),
        rate_limit_per_minute: row.get(11).unwrap_or(0),
        concurrency_limit: row.get(12).unwrap_or(0),
    }
}

const INTEGRATION_COLS: &str = "integration_id, name, audience, client_kind, \
     capabilities_json, project_scope, created_at, expires_at, last_used_at, \
     paired_at, revoked_at, rate_limit_per_minute, concurrency_limit";

/// `toView`: allowedProjects resolves to the live project list for
/// `project_scope = 'all'`, else the stored grant ids that still exist.
fn integration_view(
    conn: &rusqlite::Connection,
    rec: &McpIntegrationRow,
    projects: &[(String, String)],
) -> Value {
    let allowed: Vec<Value> = if rec.project_scope == "all" {
        projects
            .iter()
            .map(|(id, title)| json!({"projectId": id, "title": title}))
            .collect()
    } else {
        let mut out = Vec::new();
        if let Ok(mut ps) = conn.prepare(
            "SELECT p.project_id, COALESCE(pr.title, p.project_id) \
             FROM external_mcp_integration_projects p \
             LEFT JOIN projection_projects pr ON pr.project_id = p.project_id \
             WHERE p.integration_id = ?1 ORDER BY p.project_id",
        ) {
            if let Ok(mapped) = ps.query_map(params![rec.integration_id.clone()], |r| {
                Ok(json!({
                    "projectId": r.get::<_, String>(0)?,
                    "title": r.get::<_, String>(1)?,
                }))
            }) {
                out = mapped.flatten().collect();
            }
        }
        out
    };
    json!({
        "integrationId": rec.integration_id,
        "name": rec.name,
        "audience": rec.audience,
        "capabilities": rec.capabilities,
        "projectScope": rec.project_scope,
        "allowedProjects": allowed,
        "createdAt": rec.created_at,
        "expiresAt": rec.expires_at,
        "lastUsedAt": rec.last_used_at,
        "pairedAt": rec.paired_at,
        "revokedAt": rec.revoked_at,
        "rateLimitPerMinute": rec.rate_limit_per_minute,
        "concurrencyLimit": rec.concurrency_limit,
        "clientKind": rec.client_kind,
        "stdio": mcp_serve_stdio(&rec.integration_id),
    })
}

fn integration_by_id(
    conn: &rusqlite::Connection,
    integration_id: &str,
) -> Result<Option<McpIntegrationRow>, WsErr> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {INTEGRATION_COLS} FROM external_mcp_integrations \
             WHERE integration_id = ?1"
        ))
        .map_err(|e| err(e.to_string()))?;
    let mut rows = stmt
        .query(params![integration_id])
        .map_err(|e| err(e.to_string()))?;
    match rows.next().map_err(|e| err(e.to_string()))? {
        Some(row) => Ok(Some(read_integration_row(row))),
        None => Ok(None),
    }
}

/// `server.listExternalMcpIntegrations`: rows from external_mcp_integrations
/// with their project grants.
fn list_external_mcp_integrations() -> Result<Value, WsErr> {
    let Some(copy) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &copy.conn;
    let projects = load_active_projects(conn);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {INTEGRATION_COLS} FROM external_mcp_integrations ORDER BY created_at"
        ))
        .map_err(|e| err(e.to_string()))?;
    let mut rows = stmt.query([]).map_err(|e| err(e.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
        let rec = read_integration_row(row);
        out.push(integration_view(conn, &rec, &projects));
    }
    Ok(Value::Array(out))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `makeSetup`: the create/refreshPairing success payload.
fn make_setup(
    conn: &rusqlite::Connection,
    rec: &McpIntegrationRow,
    pairing_code: &str,
    pairing_expires_at: &str,
) -> Result<Value, WsErr> {
    let projects = load_active_projects(conn);
    let pair = mcp_pair_stdio(pairing_code);
    Ok(json!({
        "integration": integration_view(conn, rec, &projects),
        "pairingCode": pairing_code,
        "pairingExpiresAt": pairing_expires_at,
        "setupCommand": mcp_shell_command(&pair),
        "stdio": pair,
    }))
}

/// `server.createExternalMcpIntegration`.
fn create_external_mcp_integration(payload: &Value) -> Result<Value, WsErr> {
    let project_scope = payload
        .get("projectScope")
        .and_then(Value::as_str)
        .unwrap_or("selected")
        .to_string();
    let mut project_ids: Vec<String> = Vec::new();
    if project_scope != "all" {
        for p in payload
            .get("projectIds")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            if let Some(id) = p.as_str() {
                if !project_ids.iter().any(|x| x == id) {
                    project_ids.push(id.to_string());
                }
            }
        }
    }
    let mut capabilities: Vec<String> = Vec::new();
    for c in payload
        .get("capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        if let Some(c) = c.as_str() {
            if !capabilities.iter().any(|x| x == c) {
                capabilities.push(c.to_string());
            }
        }
    }

    let conn = crate::orchestration_engine::open_state_rw()?;
    let active = load_active_projects(&conn);
    if project_scope == "selected" {
        if project_ids.is_empty() {
            return Err(err(
                "Select at least one project or grant access to all projects.",
            ));
        }
        if let Some(missing) = project_ids
            .iter()
            .find(|id| !active.iter().any(|(pid, _)| pid == *id))
        {
            return Err(err(format!("Project \"{missing}\" was not found.")));
        }
    }
    if capabilities.iter().any(|c| c == "tasks:create")
        && !capabilities.iter().any(|c| c == "projects:read")
    {
        return Err(err(
            "The \"tasks:create\" capability requires \"projects:read\".",
        ));
    }

    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let client_kind = payload
        .get("clientKind")
        .and_then(Value::as_str)
        .unwrap_or("other");
    let now = now_ms();
    let created_at = crate::todo::iso_from_ms(now);
    let expires_at = crate::todo::iso_from_ms(
        now + payload
            .get("expiresInDays")
            .and_then(Value::as_i64)
            .unwrap_or(DEFAULT_EXPIRY_DAYS)
            * 86_400_000,
    );
    let pairing_expires_at = crate::todo::iso_from_ms(now + PAIRING_TTL_MS);
    let integration_id = format!("mcp_int_{}", crate::orchestration_engine::uuid_v4());
    let pairing_code = random_opaque(PAIRING_PREFIX);
    let pairing_hash = hash_external_mcp_secret(&pairing_code);
    let caps_json = serde_json::to_string(&capabilities).unwrap_or_else(|_| "[]".into());

    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| err(format!("Could not create the external MCP integration. {e}")))?;
    let result = (|| -> Result<(), WsErr> {
        conn.execute(
            "INSERT INTO external_mcp_integrations (
                integration_id, name, client_kind, audience, credential_hash,
                capabilities_json, project_scope, created_at, expires_at,
                last_used_at, paired_at, revoked_at,
                rate_limit_per_minute, concurrency_limit
            ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, NULL, NULL, NULL, ?9, ?10)",
            params![
                integration_id,
                name,
                client_kind,
                EXTERNAL_MCP_AUDIENCE,
                caps_json,
                project_scope,
                created_at,
                expires_at,
                DEFAULT_RATE_LIMIT_PER_MINUTE,
                DEFAULT_ACTIVE_TASK_LIMIT,
            ],
        )
        .map_err(|e| err(format!("Could not create the external MCP integration. {e}")))?;
        for pid in &project_ids {
            conn.execute(
                "INSERT INTO external_mcp_integration_projects (integration_id, project_id) \
                 VALUES (?1, ?2)",
                params![integration_id, pid],
            )
            .map_err(|e| err(format!("Could not create the external MCP integration. {e}")))?;
        }
        conn.execute(
            "INSERT INTO external_mcp_pairing_codes (
                pairing_hash, integration_id, created_at, expires_at, consumed_at
            ) VALUES (?1, ?2, ?3, ?4, NULL)",
            params![pairing_hash, integration_id, created_at, pairing_expires_at],
        )
        .map_err(|e| err(format!("Could not create the external MCP integration. {e}")))?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT").ok();
        }
        Err(e) => {
            conn.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    }

    let Some(rec) = integration_by_id(&conn, &integration_id)? else {
        return Err(err("The new integration could not be read back."));
    };
    make_setup(&conn, &rec, &pairing_code, &pairing_expires_at)
}

/// `server.revokeExternalMcpIntegration` → `{ revoked: boolean }`.
fn revoke_external_mcp_integration(payload: &Value) -> Result<Value, WsErr> {
    let integration_id = payload
        .get("integrationId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("missing integrationId"))?;
    let conn = crate::orchestration_engine::open_state_rw()?;
    let n = conn
        .execute(
            "UPDATE external_mcp_integrations \
             SET revoked_at = ?2, credential_hash = NULL \
             WHERE integration_id = ?1 AND revoked_at IS NULL",
            params![integration_id, crate::todo::iso_now_ms()],
        )
        .map_err(|e| err(format!("Could not revoke the external MCP integration. {e}")))?;
    Ok(json!({"revoked": n > 0}))
}

/// `server.refreshExternalMcpPairing`.
fn refresh_external_mcp_pairing(payload: &Value) -> Result<Value, WsErr> {
    let integration_id = payload
        .get("integrationId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("missing integrationId"))?;
    let conn = crate::orchestration_engine::open_state_rw()?;
    let Some(rec) = integration_by_id(&conn, integration_id)? else {
        return Err(err(
            "Only an active, not-yet-paired integration can receive a new pairing code.",
        ));
    };
    let now = now_ms();
    let now_iso = crate::todo::iso_from_ms(now);
    if rec.revoked_at.is_some() || rec.expires_at <= now_iso {
        return Err(err(
            "Only an active, not-yet-paired integration can receive a new pairing code.",
        ));
    }
    let has_credential: bool = conn
        .query_row(
            "SELECT credential_hash IS NOT NULL FROM external_mcp_integrations \
             WHERE integration_id = ?1",
            params![integration_id],
            |r| r.get(0),
        )
        .unwrap_or(true);
    if has_credential {
        return Err(err(
            "Only an active, not-yet-paired integration can receive a new pairing code.",
        ));
    }
    let pairing_code = random_opaque(PAIRING_PREFIX);
    let pairing_hash = hash_external_mcp_secret(&pairing_code);
    let pairing_expires_at = crate::todo::iso_from_ms(now + PAIRING_TTL_MS);

    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| err(format!("Could not refresh pairing. {e}")))?;
    let refreshed = (|| -> Result<bool, WsErr> {
        let active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM external_mcp_integrations \
                 WHERE integration_id = ?1 AND credential_hash IS NULL \
                   AND paired_at IS NULL AND revoked_at IS NULL AND expires_at > ?2",
                params![integration_id, now_iso],
                |r| r.get(0),
            )
            .map_err(|e| err(format!("Could not refresh pairing. {e}")))?;
        if active == 0 {
            return Ok(false);
        }
        conn.execute(
            "DELETE FROM external_mcp_pairing_codes WHERE integration_id = ?1",
            params![integration_id],
        )
        .map_err(|e| err(format!("Could not refresh pairing. {e}")))?;
        conn.execute(
            "INSERT INTO external_mcp_pairing_codes (
                pairing_hash, integration_id, created_at, expires_at, consumed_at
            ) VALUES (?1, ?2, ?3, ?4, NULL)",
            params![pairing_hash, integration_id, now_iso, pairing_expires_at],
        )
        .map_err(|e| err(format!("Could not refresh pairing. {e}")))?;
        Ok(true)
    })();
    match refreshed {
        Ok(true) => {
            conn.execute_batch("COMMIT").ok();
        }
        Ok(false) => {
            conn.execute_batch("ROLLBACK").ok();
            return Err(err(
                "The integration was paired, revoked, or expired before pairing could be refreshed.",
            ));
        }
        Err(e) => {
            conn.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    }
    make_setup(&conn, &rec, &pairing_code, &pairing_expires_at)
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
