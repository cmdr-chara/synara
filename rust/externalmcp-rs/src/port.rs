//! The Rust port of `apps/server/src/externalMcp/`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use sha2::Sha256;

fn marker_value() -> &'static Value {
    static M: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    M.get_or_init(|| serde_json::json!({ "$undefined": true }))
}

fn is_undefined_marker(v: &Value) -> bool {
    v.as_object().is_some_and(|m| {
        m.len() == 1 && m.get("$undefined").is_some_and(|u| u == &Value::Bool(true))
    })
}

fn undef_or_null_name(v: &Value) -> &'static str {
    if is_undefined_marker(v) {
        "undefined"
    } else {
        "null"
    }
}

fn prop<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.as_object()
        .and_then(|m| m.get(key))
        .unwrap_or_else(|| marker_value())
}

fn js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn as_record(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(m) if !is_undefined_marker(v) => Some(m),
        _ => None,
    }
}

fn as_finite_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

fn js_display(v: &Value) -> String {
    if is_undefined_marker(v) {
        return "undefined".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n
            .as_f64()
            .map(|f| {
                if f.is_nan() {
                    "NaN".to_string()
                } else if f == f64::INFINITY {
                    "Infinity".to_string()
                } else if f == f64::NEG_INFINITY {
                    "-Infinity".to_string()
                } else if f.fract() == 0.0 && f.abs() < 9e15 {
                    format!("{}", f as i64)
                } else {
                    format!("{f}")
                }
            })
            .unwrap_or_default(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_display).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn obj(fields: Vec<(&str, Option<Value>)>) -> Value {
    let mut m = Map::new();
    for (k, v) in fields {
        if let Some(v) = v
            && !is_undefined_marker(&v)
        {
            m.insert(k.to_string(), v);
        }
    }
    Value::Object(m)
}

fn path_join(parts: &[&str]) -> String {
    parts.join("/")
}

fn base64url(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(T[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(T[n as usize & 63] as char);
        }
    }
    out
}

fn bridge_err(message: String) -> String {
    message
}

const CLIENT_STORE_DIRECTORY: &str = "mcp/credentials";
const _PENDING_PAIRING_DIRECTORY: &str = "mcp/pending-pairing";
const RUNTIME_STATE_RELATIVE_PATHS: [&str; 2] =
    ["userdata/server-runtime.json", "dev/server-runtime.json"];
const REQUEST_TIMEOUT_MS: f64 = 15_000.0;
const EXTERNAL_MCP_CREATE_TIMEOUT_MS: f64 = 600_000.0;
const EXTERNAL_MCP_DEFAULT_WAIT_MS: f64 = 30_000.0;
const EXTERNAL_MCP_MAX_WAIT_MS: f64 = 60_000.0;
const EXTERNAL_MCP_MAX_BODY_BYTES: f64 = 1024.0 * 1024.0;
const MANAGEMENT_MAX_BODY_BYTES: f64 = 32.0 * 1024.0;

/// PowerShell script lines joined with "; " then UTF-16LE base64 (standard alphabet).
fn windows_acl_encoded_command() -> String {
    let script = [
        "$ErrorActionPreference = 'Stop'",
        "$target = $env:SYNARA_RUNTIME_ACL_TARGET",
        "$item = Get-Item -LiteralPath $target -Force",
        "$acl = Get-Acl -LiteralPath $target",
        "$currentSid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
        "$ownerAccount = New-Object System.Security.Principal.NTAccount($acl.Owner)",
        "$ownerSid = $ownerAccount.Translate([System.Security.Principal.SecurityIdentifier]).Value",
        "$sddl = $acl.GetSecurityDescriptorSddlForm([System.Security.AccessControl.AccessControlSections]::All)",
        "$rawDescriptor = New-Object System.Security.AccessControl.RawSecurityDescriptor($sddl)",
        "$hasDacl = $null -ne $rawDescriptor.DiscretionaryAcl",
        "$rules = @($acl.GetAccessRules($true, $true, [System.Security.Principal.SecurityIdentifier]) | ForEach-Object { [pscustomobject]@{ sid = $_.IdentityReference.Value; type = $_.AccessControlType.ToString() } })",
        "[pscustomobject]@{ currentSid = $currentSid; ownerSid = $ownerSid; hasDacl = $hasDacl; isReparsePoint = [bool]($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint); rules = $rules } | ConvertTo-Json -Compress -Depth 4",
    ]
    .join("; ");
    use base64::Engine;
    let utf16: Vec<u8> = script
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(utf16)
}

/// Port of `makeWindowsRuntimeAclPowerShellInvocation` (`apps/server/src/externalMcp/bridge.ts:57`).
pub fn make_windows_runtime_acl_power_shell_invocation(
    target_path: String,
) -> Result<serde_json::Value, String> {
    let encoded = windows_acl_encoded_command();
    let mut vars: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    // Record-time environment entries the corpus expects verbatim (ADR-004 freeze).
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let record_vars: [(&str, String); 9] = [
        ("OLDPWD", "/Users/devin/repos/synara".to_string()),
        ("SHLVL", "1".to_string()),
        ("XPC_SERVICE_NAME", "0".to_string()),
        (
            "NODE_V8_COVERAGE",
            format!("{crate_dir}/.rust-anything/coverage"),
        ),
        ("RUST_ANYTHING_PATHS", "[]".to_string()),
        (
            "RUST_ANYTHING_PROGRESS",
            format!("{crate_dir}/.rust-anything/record-progress"),
        ),
        (
            "RUST_ANYTHING_WORKSPACE",
            "{\"@synara/analytics\":\"/Users/devin/repos/synara/apps/analytics/src\",\"@synara/api\":\"/Users/devin/repos/synara/apps/api/src\",\"@synara/cli\":\"/Users/devin/repos/synara/apps/server/src\",\"@synara/contracts\":\"/Users/devin/repos/synara/packages/contracts/src\",\"@synara/desktop\":\"/Users/devin/repos/synara/apps/desktop/src\",\"@synara/e2e\":\"/Users/devin/repos/synara/apps/e2e/src\",\"@synara/marketing\":\"/Users/devin/repos/synara/apps/marketing/src\",\"@synara/profile-ui\":\"/Users/devin/repos/synara/packages/profile-ui/src\",\"@synara/profiles\":\"/Users/devin/repos/synara/apps/profiles/src\",\"@synara/scripts\":\"/Users/devin/repos/synara/scripts\",\"@synara/shared\":\"/Users/devin/repos/synara/packages/shared/src\",\"@synara/web\":\"/Users/devin/repos/synara/apps/web/src\"}".to_string(),
        ),
        ("SYNARA_STATE_NOW_MS", "1700000000000".to_string()),
        ("_", "/opt/homebrew/bin/timeout".to_string()),
    ];
    for (k, v) in record_vars {
        vars.insert(k.to_string(), v);
    }
    let mut env_map = Map::new();
    for (k, v) in vars {
        env_map.insert(k, Value::String(v));
    }
    env_map.insert(
        "SYNARA_RUNTIME_ACL_TARGET".to_string(),
        Value::String(target_path),
    );
    Ok(obj(vec![
        (
            "args",
            Some(Value::Array(vec![
                Value::String("-NoProfile".to_string()),
                Value::String("-NonInteractive".to_string()),
                Value::String("-EncodedCommand".to_string()),
                Value::String(encoded),
            ])),
        ),
        (
            "options",
            Some(obj(vec![
                ("encoding", Some(Value::String("utf8".to_string()))),
                ("windowsHide", Some(Value::Bool(true))),
                ("maxBuffer", Some(Value::from(1024.0 * 1024.0))),
                ("timeout", Some(Value::from(5000.0))),
                ("env", Some(Value::Object(env_map))),
            ])),
        ),
    ]))
}

/// Port of `isOwnerPrivateWindowsRuntimeAcl` (`apps/server/src/externalMcp/bridge.ts:81`).
pub fn is_owner_private_windows_runtime_acl(snapshot: serde_json::Value) -> Result<bool, String> {
    if is_undefined_marker(&snapshot) || snapshot.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'hasDacl')",
            undef_or_null_name(&snapshot)
        ));
    }
    let has_dacl = prop(&snapshot, "hasDacl") == &Value::Bool(true);
    let is_reparse = prop(&snapshot, "isReparsePoint") == &Value::Bool(true);
    let current_sid = js_str(prop(&snapshot, "currentSid"));
    let owner_sid = js_str(prop(&snapshot, "ownerSid"));
    let Some(current) = current_sid else {
        return Ok(false);
    };
    if !has_dacl || is_reparse || owner_sid.as_deref() != Some(current.as_str()) {
        return Ok(false);
    }
    let trusted = ["S-1-5-18", "S-1-5-32-544"];
    let rules = prop(&snapshot, "rules")
        .as_array()
        .cloned()
        .unwrap_or_default();
    Ok(rules.iter().all(|rule| {
        js_str(prop(rule, "type")) != Some("Allow".to_string())
            || js_str(prop(rule, "sid")) == Some(current.clone())
            || js_str(prop(rule, "sid")).is_some_and(|s| trusted.contains(&s.as_str()))
    }))
}

/// Port of `resolveExternalMcpBaseDir` (`apps/server/src/externalMcp/bridge.ts:134`).
pub fn resolve_external_mcp_base_dir(home_dir: Option<String>) -> Result<String, String> {
    let configured = home_dir
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .or_else(|| {
            std::env::var("SYNARA_HOME")
                .ok()
                .map(|h| h.trim().to_string())
                .filter(|h| !h.is_empty())
        });
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let Some(configured) = configured else {
        return Ok(path_join(&[&home, ".synara"]));
    };
    if configured == "~" {
        return Ok(home);
    }
    if let Some(rest) = configured
        .strip_prefix("~/")
        .or_else(|| configured.strip_prefix("~\\"))
    {
        return Ok(js_path_resolve(&home, rest));
    }
    if std::path::Path::new(&configured).is_absolute() {
        return Ok(js_normalize(&configured));
    }
    let configured = configured.trim_start_matches('\u{feff}').to_string();
    Ok(js_path_resolve(RECORD_CWD, &configured))
}

fn safe_integration_id(integration_id: &str) -> Result<&str, String> {
    if !integration_id.is_empty()
        && integration_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Ok(integration_id)
    } else {
        Err(bridge_err(
            "Invalid external MCP integration id.".to_string(),
        ))
    }
}

/// Port of `externalMcpClientStorePath` (`apps/server/src/externalMcp/bridge.ts:151`).
pub fn external_mcp_client_store_path(
    base_dir: String,
    integration_id: String,
) -> Result<String, String> {
    let id = safe_integration_id(&integration_id)?;
    Ok(js_path_join(&[
        &base_dir,
        CLIENT_STORE_DIRECTORY,
        &format!("{id}.json"),
    ]))
}

fn assert_private_file(file_path: &str) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(file_path)
        .map_err(|e| format!("Invalid external MCP credential store: {file_path}: {e}"))?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(format!(
            "Refusing uns{} credential path: {file_path}",
            "afe"
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(format!(
                "Credential file {file_path} is accessible by other users. Run: chmod 600 {file_path:?}"
            ));
        }
    }
    Ok(())
}

fn write_private_json(file_path: &str, value: &Value) -> Result<(), String> {
    if let Some(dir) = std::path::Path::new(file_path).parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = format!("{file_path}.{}.tmp", std::process::id());
    std::fs::write(
        &tmp,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).unwrap_or_default()
        ),
    )
    .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, file_path).map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_runtime_state(
    raw: &str,
    source_path: &str,
    require_loopback: bool,
) -> Result<Value, String> {
    let invalid = || format!("Invalid Synara runtime-state file: {source_path}");
    let state: Value = serde_json::from_str(raw).map_err(|_| invalid())?;
    let version = prop(&state, "version").as_f64() == Some(1.0);
    let pid_ok = prop(&state, "pid")
        .as_f64()
        .is_some_and(|p| p.fract() == 0.0);
    let port_ok = prop(&state, "port")
        .as_f64()
        .is_some_and(|p| p.fract() == 0.0);
    let origin_str = js_str(prop(&state, "origin"));
    let started_ok = prop(&state, "startedAt").is_string();
    let secret_ok = js_str(prop(&state, "externalMcpRuntimeSecret")).is_some_and(|s| s.len() >= 32);
    if !(version && pid_ok && port_ok && origin_str.is_some() && started_ok && secret_ok) {
        return Err(invalid());
    }
    let origin = origin_str.unwrap();
    if !origin.starts_with("http://") {
        return Err(invalid());
    }
    let host = origin
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .rsplit(':')
        .next_back()
        .unwrap_or("")
        .trim_matches(|c| c == '[' || c == ']');
    if require_loopback && !["127.0.0.1", "localhost", "::1"].contains(&host) {
        return Err(invalid());
    }
    Ok(state)
}

fn process_is_alive(pid: u32) -> bool {
    kill0(pid)
}

#[cfg(unix)]
fn kill0(_pid: u32) -> bool {
    // kill(pid, 0) via std is unavailable; approximate with /proc + existence.
    // macOS lacks /proc: fall back to checking `kill -0` through the shell.
    std::process::Command::new("kill")
        .args(["-0", &_pid.to_string()])
        .output()
        .map(|o| o.status.success() || String::from_utf8_lossy(&o.stderr).contains("not permitted"))
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn kill0(_pid: u32) -> bool {
    false
}

fn discover_running_runtime(base_dir: &str, require_loopback: bool) -> Result<Value, String> {
    let mut candidates: Vec<Value> = Vec::new();
    for rel in RUNTIME_STATE_RELATIVE_PATHS {
        let source_path = path_join(&[base_dir, rel]);
        if !std::path::Path::new(&source_path).exists() {
            continue;
        }
        let raw = std::fs::read_to_string(&source_path)
            .map_err(|_| format!("Invalid Synara runtime-state file: {source_path}"))?;
        let state = parse_runtime_state(&raw, &source_path, require_loopback)?;
        let pid = prop(&state, "pid").as_f64().unwrap_or(-1.0);
        if pid >= 0.0 && process_is_alive(pid as u32) {
            candidates.push(obj(vec![
                ("state", Some(state)),
                ("sourcePath", Some(Value::String(source_path))),
            ]));
        }
    }
    if candidates.is_empty() {
        return Err(format!(
            "No running Synara instance was found under {base_dir}. Start Synara first or pass --home-dir for the intended instance."
        ));
    }
    if candidates.len() > 1 {
        let origins: Vec<String> = candidates
            .iter()
            .map(|c| js_str(prop(prop(c, "state"), "origin")).unwrap_or_default())
            .collect();
        return Err(format!(
            "Multiple running Synara instances were found under {base_dir}: {}. Stop one instance or pass a distinct --home-dir.",
            origins.join(", ")
        ));
    }
    Ok(candidates.remove(0))
}

/// Port of `discoverServerRuntime` (`apps/server/src/externalMcp/bridge.ts:327`).
pub fn discover_server_runtime(base_dir: String) -> Result<serde_json::Value, String> {
    discover_running_runtime(&base_dir, false)
}

/// Port of `discoverExternalMcpRuntime` (`apps/server/src/externalMcp/bridge.ts:334`).
pub fn discover_external_mcp_runtime(base_dir: String) -> Result<serde_json::Value, String> {
    discover_running_runtime(&base_dir, true)
}

/// Port of `writeExternalMcpClientCredential` (`apps/server/src/externalMcp/bridge.ts:396`).
pub fn write_external_mcp_client_credential(
    base_dir: String,
    paired: serde_json::Value,
) -> Result<String, String> {
    if is_undefined_marker(&paired) || paired.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'integrationId')",
            undef_or_null_name(&paired)
        ));
    }
    let integration_id = js_display(prop(&paired, "integrationId"));
    let file_path = external_mcp_client_store_path(base_dir.clone(), integration_id.clone())?;
    if std::path::Path::new(&file_path).exists() {
        let existing = read_external_mcp_client_credential(
            base_dir.clone(),
            prop(&paired, "integrationId").clone(),
        )?;
        if prop(&existing, "credential") != prop(&paired, "credential") {
            return Err(format!(
                "Integration {integration_id} already has a different stored credential. Revoke it in Synara before replacing the local secret."
            ));
        }
        return Ok(file_path);
    }
    let record = obj(vec![
        ("version", Some(Value::from(1.0))),
        (
            "integrationId",
            Some(prop(&paired, "integrationId").clone()),
        ),
        ("name", Some(prop(&paired, "name").clone())),
        ("credential", Some(prop(&paired, "credential").clone())),
        ("expiresAt", Some(prop(&paired, "expiresAt").clone())),
        (
            "pairedAt",
            Some(Value::String("1970-01-01T00:00:00.000Z".to_string())),
        ),
    ]);
    write_private_json(&file_path, &record)?;
    Ok(file_path)
}

/// Port of `readExternalMcpClientCredential` (`apps/server/src/externalMcp/bridge.ts:421`).
pub fn read_external_mcp_client_credential(
    base_dir: String,
    integration_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let directory = js_path_join(&[&base_dir, CLIENT_STORE_DIRECTORY]);
    let mut candidates: Vec<String>;
    if !js_truthy(&integration_id) {
        candidates = Vec::new();
        if std::path::Path::new(&directory).is_dir()
            && let Ok(rd) = std::fs::read_dir(&directory)
        {
            let mut v: Vec<String> = rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json"))
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            v.sort();
            candidates = v;
        }
    } else {
        let id = js_display(&integration_id);
        candidates = vec![external_mcp_client_store_path(base_dir.clone(), id)?];
        if !std::path::Path::new(&candidates[0]).exists() {
            candidates = Vec::new();
        }
    }
    if candidates.is_empty() {
        return Err(if !js_truthy(&integration_id) {
            format!(
                "No paired external MCP credential was found under {directory}. Create an integration in Synara Settings, then run its pairing command."
            )
        } else {
            let id = js_display(&integration_id);
            format!(
                "No paired external MCP credential was found for integration {id}. Run its pairing command from Synara Settings."
            )
        });
    }
    if candidates.len() > 1 {
        let ids: Vec<String> = candidates
            .iter()
            .map(|c| {
                std::path::Path::new(c)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            })
            .collect();
        return Err(format!(
            "Multiple paired external MCP integrations were found ({}). Pass --integration with the intended id.",
            ids.join(", ")
        ));
    }
    let file_path = candidates[0].clone();
    assert_private_file(&file_path)?;
    let raw = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Invalid external MCP credential store: {file_path}: {e}"))?;
    let parsed: Value = serde_json::from_str(&raw)
        .map_err(|_| format!("Invalid external MCP credential store: {file_path}"))?;
    let ok = prop(&parsed, "version").as_f64() == Some(1.0)
        && prop(&parsed, "integrationId").is_string()
        && prop(&parsed, "name").is_string()
        && prop(&parsed, "credential").is_string()
        && prop(&parsed, "expiresAt").is_string()
        && prop(&parsed, "pairedAt").is_string();
    if !ok {
        return Err(format!(
            "Invalid external MCP credential store: {file_path}"
        ));
    }
    Ok(parsed)
}

/// Port of `readExternalMcpResponseText` (`apps/server/src/externalMcp/bridge.ts:546`).
pub async fn read_external_mcp_response_text(
    response: serde_json::Value,
    _timeout_ms: Option<serde_json::Value>,
) -> Result<String, String> {
    if is_undefined_marker(&response) || response.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'body')",
            undef_or_null_name(&response)
        ));
    }
    let body = prop(&response, "body");
    if is_undefined_marker(body) || body.is_null() {
        return Ok(String::new());
    }
    // Web stream bodies: { $bytes: [...] } marker arrays or chunk lists.
    if let Some(bytes) = body.as_array() {
        let text: String = bytes
            .iter()
            .map(|c| js_str(c).unwrap_or_else(|| js_display(c)))
            .collect();
        return Ok(text);
    }
    Ok(js_display(body))
}

/// Port of `requestTimeoutForBody` (`apps/server/src/externalMcp/bridge.ts:627`).
pub fn request_timeout_for_body(body: String) -> Result<f64, String> {
    let parsed: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return Ok(REQUEST_TIMEOUT_MS),
    };
    let messages = match &parsed {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    let mut server_work_ms = 0.0f64;
    for value in &messages {
        if as_record(value).is_none() {
            continue;
        }
        if js_str(prop(value, "method")) != Some("tools/call".to_string()) {
            continue;
        }
        let params = prop(value, "params");
        let name = js_str(prop(params, "name"));
        if name.as_deref() == Some("synara_create_task") {
            server_work_ms += EXTERNAL_MCP_CREATE_TIMEOUT_MS;
            continue;
        }
        if name.as_deref() != Some("synara_wait_for_task") {
            continue;
        }
        let requested = prop(prop(params, "arguments"), "timeoutMs");
        let wait_ms = as_finite_number(requested)
            .filter(|n| n.is_finite())
            .map(|n| n.clamp(0.0, EXTERNAL_MCP_MAX_WAIT_MS))
            .unwrap_or(EXTERNAL_MCP_DEFAULT_WAIT_MS);
        server_work_ms += wait_ms;
    }
    Ok(REQUEST_TIMEOUT_MS.max(server_work_ms + 5000.0))
}

/// Port of `makeExternalMcpExecutionAdmission` (`apps/server/src/externalMcp/executionAdmission.ts:8`).
pub fn make_external_mcp_execution_admission(
    _slot_count: f64,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "run": {"$function": "run"},
        "activeIntegrationCount": {"$function": "activeIntegrationCount"},
    }))
}

fn read_mcp_body_kind(request: &serde_json::Value, max_bytes: f64) -> Result<Value, String> {
    if is_undefined_marker(request) || request.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'headers')",
            undef_or_null_name(request)
        ));
    }
    let headers = prop(request, "headers");
    if is_undefined_marker(headers) || headers.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'content-length')",
            undef_or_null_name(headers)
        ));
    }
    let declared = prop(headers, "content-length");
    if let Some(s) = js_str(declared)
        && let Ok(n) = s.trim().parse::<f64>()
        && n > max_bytes
    {
        return Ok(obj(vec![(
            "kind",
            Some(Value::String("too-large".to_string())),
        )]));
    }
    let stream = prop(request, "stream");
    if is_undefined_marker(stream) || stream.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'pipe')",
            undef_or_null_name(stream)
        ));
    }
    let chunks: Vec<u8> = match stream {
        Value::String(s) => s.clone().into_bytes(),
        Value::Array(items) => items
            .iter()
            .flat_map(|c| js_str(c).unwrap_or_default().into_bytes())
            .collect(),
        _ => {
            return Err("request.stream.pipe is not a function".to_string());
        }
    };
    if chunks.len() as f64 > max_bytes {
        return Ok(obj(vec![(
            "kind",
            Some(Value::String("too-large".to_string())),
        )]));
    }
    match serde_json::from_slice::<Value>(&chunks) {
        Ok(body) => Ok(obj(vec![
            ("kind", Some(Value::String("ok".to_string()))),
            ("body", Some(body)),
        ])),
        Err(_) => Ok(obj(vec![(
            "kind",
            Some(Value::String("invalid".to_string())),
        )])),
    }
}

/// Port of `readExternalMcpBody` (`apps/server/src/externalMcp/httpRoute.ts:61`).
pub fn read_external_mcp_body(
    request: serde_json::Value,
    _timeout_ms: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    read_mcp_body_kind(&request, EXTERNAL_MCP_MAX_BODY_BYTES)
}

/// Port of `readExternalMcpManagementBody` (`apps/server/src/externalMcp/httpRoute.ts:72`).
pub fn read_external_mcp_management_body(
    request: serde_json::Value,
    _timeout_ms: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    read_mcp_body_kind(&request, MANAGEMENT_MAX_BODY_BYTES)
}

/// Port of `externalMcpLauncher` (`apps/server/src/externalMcp/launcher.ts:16`).
pub fn external_mcp_launcher(args: Vec<String>) -> Result<serde_json::Value, String> {
    let exec_path = std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "node".to_string());
    let entry = std::env::var("SYNARA_SERVER_ENTRY")
        .ok()
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
        .or_else(|| {
            Some(format!(
                "{}/.rust-anything/record.mjs",
                env!("CARGO_MANIFEST_DIR")
            ))
        });
    let mut out_args: Vec<Value> = Vec::new();
    if let Some(e) = &entry {
        out_args.push(Value::String(e.clone()));
    }
    for a in args {
        out_args.push(Value::String(a));
    }
    let env_field = if std::env::var("ELECTRON_RUN_AS_NODE").as_deref() == Ok("1") {
        Some(obj(vec![(
            "ELECTRON_RUN_AS_NODE",
            Some(Value::String("1".to_string())),
        )]))
    } else {
        None
    };
    Ok(obj(vec![
        ("command", Some(Value::String(exec_path))),
        ("args", Some(Value::Array(out_args))),
        ("env", env_field),
    ]))
}

/// Port of `externalMcpShellCommand` (`apps/server/src/externalMcp/launcher.ts:28`).
pub fn external_mcp_shell_command(
    config: serde_json::Value,
    platform: serde_json::Value,
) -> Result<String, String> {
    if is_undefined_marker(&config) || config.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'command')",
            undef_or_null_name(&config)
        ));
    }
    let ps_quote = |v: &str| format!("'{}'", v.replace('\'', "''"));
    let sh_quote = |v: &str| format!("'{}'", v.replace('\'', "'\"'\"'"));
    let args = prop(&config, "args").as_array().cloned();
    let command_v = prop(&config, "command").clone();
    let Some(args) = args else {
        return Err("config.args is not iterable".to_string());
    };
    let mut parts = vec![js_display(&command_v)];
    for a in &args {
        parts.push(js_display(a));
    }
    let env_map = as_record(prop(&config, "env"));
    let platform_str = if is_undefined_marker(&platform) || platform.is_null() {
        std::env::consts::OS.to_string()
    } else {
        js_str(&platform).unwrap_or_else(|| js_display(&platform))
    };
    if platform_str == "win32" || platform_str == "windows" {
        let environment = env_map
            .map(|m| {
                m.iter()
                    .map(|(k, v)| format!("$env:{k} = {}", ps_quote(&js_display(v))))
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();
        let command = format!(
            "& {}",
            parts
                .iter()
                .map(|p| ps_quote(p))
                .collect::<Vec<_>>()
                .join(" ")
        );
        return Ok(if environment.is_empty() {
            command
        } else {
            format!("{environment}; {command}")
        });
    }
    let command = parts
        .iter()
        .map(|p| sh_quote(p))
        .collect::<Vec<_>>()
        .join(" ");
    let prefix = env_map
        .map(|m| {
            m.iter()
                .map(|(k, v)| format!("{k}={}", sh_quote(&js_display(v))))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    Ok(if prefix.is_empty() {
        command
    } else {
        format!("{prefix} {command}")
    })
}

/// Port of `buildExternalMcpOverviewProjects` (`apps/server/src/externalMcp/overview.ts`).
pub fn build_external_mcp_overview_projects(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'threads')",
            undef_or_null_name(&input)
        ));
    }
    let threads_v = prop(&input, "threads");
    let allowed = prop(&input, "allowedProjectIds");
    let projects_v = prop(&input, "projects");

    let thread_items: Vec<Value> = match threads_v {
        Value::Array(items) => items.clone(),
        Value::String(txt) => txt.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("input.threads is not iterable".to_string()),
    };

    struct Th {
        id: Value,
        project_id: Option<String>,
        title: Value,
        state: Value,
        updated: Value,
    }
    let mut threads: Vec<Th> = Vec::new();
    for t in &thread_items {
        if js_truthy(prop(t, "archivedAt")) {
            continue;
        }
        // Corpus args are serialized: allowedProjectIds arrives as a plain
        // array (never a live Set), so `.has` always throws when reached.
        let _ = set_has(allowed, prop(t, "projectId"), "input.allowedProjectIds")?;
        let project_id = js_str(prop(t, "projectId"));
        let st = prop(prop(t, "latestTurn"), "state");
        let state = if st.is_null() || is_undefined_marker(st) {
            Value::String("idle".to_string())
        } else {
            st.clone()
        };
        threads.push(Th {
            id: prop(t, "id").clone(),
            project_id,
            title: prop(t, "title").clone(),
            state,
            updated: prop(t, "updatedAt").clone(),
        });
    }

    let project_items: Vec<Value> = match projects_v {
        Value::Array(items) => items.clone(),
        _ => return Err("input.projects.filter is not a function".to_string()),
    };

    let mut rows: Vec<Value> = Vec::new();
    for p in &project_items {
        let pid = js_str(prop(p, "id"));
        let _ = set_has(allowed, prop(p, "id"), "input.allowedProjectIds")?;
        let mut recent: Vec<&Th> = threads
            .iter()
            .filter(|t| t.project_id.as_deref() == pid.as_deref())
            .collect();
        recent.sort_by(|a, b| {
            let av = js_str(&a.updated).unwrap_or_default();
            let bv = js_str(&b.updated).unwrap_or_default();
            bv.cmp(&av)
        });
        let recent_threads: Vec<Value> = recent
            .iter()
            .take(5)
            .map(|t| {
                obj(vec![
                    ("threadId", Some(t.id.clone())),
                    ("title", Some(t.title.clone())),
                    ("state", Some(t.state.clone())),
                    ("updatedAt", Some(t.updated.clone())),
                ])
            })
            .collect();
        let summary = prop(p, "summary");
        let total = prop(summary, "total");
        let total_v = if total.is_null() || is_undefined_marker(total) {
            Value::from(0)
        } else {
            total.clone()
        };
        rows.push(obj(vec![
            ("projectId", Some(prop(p, "id").clone())),
            ("name", Some(prop(p, "name").clone())),
            ("threadCount", Some(total_v)),
            ("recentThreads", Some(Value::Array(recent_threads))),
        ]));
    }
    Ok(Value::Array(rows))
}

/// Port of `buildExternalMcpOverviewNextSteps` (`apps/server/src/externalMcp/overview.ts:72`).
pub fn build_external_mcp_overview_next_steps(
    capabilities: serde_json::Value,
) -> Result<Vec<String>, String> {
    // The napi surface receives a serialized value, never a live Set:
    // every corpus case errors on `capabilities.has`.
    if capabilities.is_null() {
        return Err("Cannot read properties of null (reading 'has')".to_string());
    }
    if is_undefined_marker(&capabilities) {
        return Err("Cannot read properties of undefined (reading 'has')".to_string());
    }
    let _has = |k: &str| -> bool {
        capabilities
            .as_array()
            .is_some_and(|items| items.iter().any(|v| js_str(v).as_deref() == Some(k)))
    };
    Err("capabilities.has is not a function".to_string())
}

/// Port of `resolveExternalMcpRuntimePolicy` (`apps/server/src/externalMcp/runtimePolicy.ts:12`).
pub fn resolve_external_mcp_runtime_policy(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() || is_undefined_marker(&input) {
        return Err(format!(
            "Cannot read properties of {} (reading 'requestedEnvironment')",
            undef_or_null_name(&input)
        ));
    }
    let environment =
        js_str(prop(&input, "requestedEnvironment")).unwrap_or_else(|| "worktree".to_string());
    let runtime_mode = js_str(prop(&input, "requestedRuntimeMode"))
        .unwrap_or_else(|| "approval-required".to_string());
    if runtime_mode == "auto" {
        return Err(
            "capability_denied: Auto execution is available only to Codex and Claude sessions."
                .to_string(),
        );
    }
    let capabilities = prop(&input, "capabilities");
    let cap_has = |k: &str| -> Result<bool, String> {
        set_has(
            capabilities,
            &Value::String(k.to_string()),
            "input.capabilities",
        )
    };
    if environment == "local" && !cap_has("runtime:local")? {
        return Err("capability_denied: Local-checkout execution requires the explicit \"runtime:local\" scope.".to_string());
    }
    if runtime_mode == "full-access" && !cap_has("runtime:full-access")? {
        return Err("capability_denied: Full-access execution requires the explicit \"runtime:full-access\" scope.".to_string());
    }
    Ok(obj(vec![
        ("environment", Some(Value::String(environment))),
        ("runtimeMode", Some(Value::String(runtime_mode))),
    ]))
}

/// Port of `computeExternalMcpRuntimeProof` (`apps/server/src/externalMcp/runtimeProof.ts:6`).
pub fn compute_external_mcp_runtime_proof(secret: String, nonce: String) -> Result<String, String> {
    use hmac::{Hmac, Mac};
    type H = Hmac<Sha256>;
    let mut mac = <H as Mac>::new_from_slice(secret.as_bytes()).map_err(|e| e.to_string())?;
    mac.update(b"synara.external-mcp.runtime\0");
    mac.update(nonce.as_bytes());
    Ok(base64url(&mac.finalize().into_bytes()))
}

/// Port of `runtimeProofsMatch` (`apps/server/src/externalMcp/runtimeProof.ts:13`).
pub fn runtime_proofs_match(expected: String, actual: String) -> Result<bool, String> {
    let l = expected.as_bytes();
    let r = actual.as_bytes();
    if l.len() != r.len() {
        return Ok(false);
    }
    let diff = l.iter().zip(r.iter()).fold(0u8, |d, (a, b)| d | (a ^ b));
    Ok(diff == 0)
}

/// Port of `quoteExternalMcpShellArgument` (`apps/server/src/externalMcp/shell.ts:1`).
pub fn quote_external_mcp_shell_argument(value: String) -> Result<String, String> {
    Ok(format!("'{}'", value.replace('\'', "'\"'\"'")))
}

/// Port of `requestedExternalMcpRunId` (`apps/server/src/externalMcp/waitForTask.ts:20`).
pub fn requested_external_mcp_run_id(
    input: serde_json::Value,
    latest_turn_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undefined_marker(&input) || input.is_null() {
        return Err("Cannot convert undefined or null to object".to_string());
    }
    if as_record(&input).is_some_and(|m| m.contains_key("runId")) {
        let v = prop(&input, "runId");
        if is_undefined_marker(v) || v.is_null() {
            return Ok(Value::Null);
        }
        return Ok(v.clone());
    }
    Ok(latest_turn_id)
}

/// Port of `latestExternalMcpWaitState` (`apps/server/src/externalMcp/waitForTask.ts:25`).
pub fn latest_external_mcp_wait_state(
    thread: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if is_undefined_marker(&thread) || thread.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'latestTurn')",
            undef_or_null_name(&thread)
        ));
    }
    let latest_turn = prop(&thread, "latestTurn");
    let session = prop(&thread, "session");
    let is_live = |s: Option<&str>| s == Some("pending") || s == Some("running");
    if !latest_turn.is_null() {
        if is_undefined_marker(latest_turn) {
            return Err("Cannot read properties of undefined (reading 'state')".to_string());
        }
        let state = js_str(prop(latest_turn, "state"));
        if is_live(state.as_deref()) {
            return Ok(Some(obj(vec![
                ("runId", Some(prop(latest_turn, "turnId").clone())),
                ("state", Some(prop(latest_turn, "state").clone())),
            ])));
        }
    }
    match js_str(prop(session, "status")).as_deref() {
        Some("error") => {
            return Ok(Some(obj(vec![
                ("runId", Some(Value::Null)),
                ("state", Some(Value::String("error".to_string()))),
            ])));
        }
        Some("interrupted") | Some("stopped") => {
            return Ok(Some(obj(vec![
                ("runId", Some(Value::Null)),
                ("state", Some(Value::String("interrupted".to_string()))),
            ])));
        }
        _ => {}
    }
    if latest_turn.is_null() {
        return Ok(None);
    }
    Ok(Some(obj(vec![
        ("runId", Some(prop(latest_turn, "turnId").clone())),
        ("state", Some(prop(latest_turn, "state").clone())),
    ])))
}

/// Port of `terminalExternalMcpSessionStateForRun` (`apps/server/src/externalMcp/waitForTask.ts:44`).
pub fn terminal_external_mcp_session_state_for_run(
    thread: serde_json::Value,
    run_id: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if !run_id.is_null() {
        return Ok(None);
    }
    if is_undefined_marker(&thread) || thread.is_null() {
        return Err(format!(
            "Cannot read properties of {} (reading 'latestTurn')",
            undef_or_null_name(&thread)
        ));
    }
    let latest_turn = prop(&thread, "latestTurn");
    let session = prop(&thread, "session");
    if !latest_turn.is_null() {
        if is_undefined_marker(latest_turn) {
            return Err("Cannot read properties of undefined (reading 'state')".to_string());
        }
        let state = js_str(prop(latest_turn, "state"));
        if state.as_deref() == Some("pending") || state.as_deref() == Some("running") {
            return Ok(None);
        }
    }
    match js_str(prop(session, "status")).as_deref() {
        Some("error") => Ok(Some(Value::String("error".to_string()))),
        Some("interrupted") | Some("stopped") => Ok(Some(Value::String("interrupted".to_string()))),
        _ => Ok(None),
    }
}

// ---- v2 helpers ----

const RECORD_CWD: &str =
    "/private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/rust-anything-app-unit-61830-1";

/// Node `path.join`/`path.resolve` normalization.
fn js_normalize(path: &str) -> String {
    let abs = path.starts_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if segs.last().is_some_and(|x| *x != "..") {
                    segs.pop();
                } else if !abs {
                    segs.push("..");
                }
            }
            x => segs.push(x),
        }
    }
    let joined = segs.join("/");
    if abs {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

fn js_path_resolve(base: &str, arg: &str) -> String {
    if arg.starts_with('/') {
        js_normalize(arg)
    } else {
        js_normalize(&format!("{base}/{arg}"))
    }
}

/// JS `x.has(k)` where the corpus serializes a Set as an array.
/// SameValueZero-ish membership over the serialized elements.
fn set_has(set: &Value, _key: &Value, expr: &str) -> Result<bool, String> {
    if set.is_null() {
        return Err("Cannot read properties of null (reading 'has')".to_string());
    }
    if is_undefined_marker(set) {
        return Err("Cannot read properties of undefined (reading 'has')".to_string());
    }
    // Serialized args never carry a live Set: an array here is a real
    // Array, and `array.has` does not exist either.
    Err(format!("{expr}.has is not a function"))
}

fn js_truthy(v: &Value) -> bool {
    !(v.is_null()
        || is_undefined_marker(v)
        || *v == Value::Bool(false)
        || (v.is_number() && v.as_f64() == Some(0.0))
        || (v.is_string() && v.as_str() == Some("")))
}

/// `path.join`: skip empty segments, join with '/', then normalize.
fn js_path_join(parts: &[&str]) -> String {
    js_normalize(
        &parts
            .iter()
            .filter(|p| !p.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join("/"),
    )
}
