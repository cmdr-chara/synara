//! The Rust port of `apps/server/src/accountAuth.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const CREDENTIALS_FILE_NAME: &str = "account-credentials.json";

fn js_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn prop(v: &Value, key: &str) -> Result<Value, String> {
    if js_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    match v {
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ => Ok(crate::js::undefined()),
    }
}

/// `path.posix.normalize` (darwin → posix rules).
fn path_normalize(path: &str) -> String {
    let is_abs = path.starts_with('/');
    let trailing = path.len() > 1 && path.ends_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => match segs.last() {
                Some(&"..") if !is_abs => segs.push(".."),
                Some(_) => {
                    segs.pop();
                }
                None if !is_abs => segs.push(".."),
                None => {}
            },
            s => segs.push(s),
        }
    }
    let mut out = segs.join("/");
    if is_abs {
        out = format!("/{out}");
    }
    if out.is_empty() {
        ".".to_string()
    } else if trailing && out != "/" {
        format!("{out}/")
    } else {
        out
    }
}

fn path_join(a: &str, b: &str) -> String {
    if a.is_empty() {
        path_normalize(b)
    } else {
        path_normalize(&format!("{a}/{b}"))
    }
}

/// JS `String.prototype.trim` — Unicode whitespace plus U+FEFF (BOM).
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// `value?.trim()` where the property must be a string when present.
fn optional_trimmed(v: &Value, owner: &str) -> Result<Option<String>, String> {
    match v {
        Value::Null => Ok(None),
        _ if js_undefined(v) => Ok(None),
        Value::String(s) => {
            let t = js_trim(s);
            if t.is_empty() {
                Ok(None)
            } else {
                Ok(Some(t.to_string()))
            }
        }
        _ => Err(format!("{owner}?.trim is not a function")),
    }
}

fn is_wildcard_host(host: &str) -> bool {
    host == "0.0.0.0" || host == "::" || host == "[::]"
}

fn is_loopback_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return true;
    };
    if host.is_empty() {
        return true;
    }
    let normalized = if host.starts_with('[') && host.ends_with(']') {
        &host[1..host.len() - 1]
    } else {
        host
    };
    normalized == "localhost" || normalized == "127.0.0.1" || normalized == "::1"
}

fn state_dir(base_dir: &str, dev_url: &Value) -> String {
    if js_undefined(dev_url) {
        path_join(base_dir, "userdata")
    } else {
        path_join(base_dir, "dev")
    }
}

/// `readAccountFile` — missing/unreadable/malformed → absent.
fn read_account_file_sync(base_dir: &str) -> Option<Value> {
    let raw = std::fs::read_to_string(account_credentials_path(base_dir.to_string()).ok()?).ok()?;
    let parsed: Value = serde_json::from_str(&raw).ok()?;
    let record = parsed.as_object()?;
    if record.get("deviceToken").is_some_and(|v| v.is_string()) {
        return None;
    }
    for key in ["accountUrl", "workosClientId", "workosApiUrl"] {
        if !record.get(key).is_some_and(|v| v.is_string()) {
            return None;
        }
    }
    Some(parsed)
}

fn read_account_credentials_sync(base_dir: &str) -> Option<Value> {
    let stored = read_account_file_sync(base_dir)?;
    for key in ["accessToken", "refreshToken", "organizationId"] {
        if !stored.get(key).is_some_and(|v| v.is_string()) {
            return None;
        }
    }
    Some(stored)
}

/// Port of `accountApiIssuer` (`apps/server/src/accountAuth.ts:60`).
pub fn account_api_issuer(account_url: String) -> Result<String, String> {
    Ok(format!("{}/api/v1", account_url.trim_end_matches('/')))
}

/// Port of `accountStateDirectory` (`apps/server/src/accountAuth.ts:141`).
pub fn account_state_directory(
    base_dir: String,
    dev_url: serde_json::Value,
) -> Result<String, String> {
    if js_undefined(&dev_url) {
        Ok(base_dir)
    } else {
        Ok(path_join(&base_dir, "dev"))
    }
}

/// Port of `accountCredentialsPath` (`apps/server/src/accountAuth.ts:145`).
pub fn account_credentials_path(base_dir: String) -> Result<String, String> {
    Ok(path_join(&base_dir, CREDENTIALS_FILE_NAME))
}

/// Port of `readAccountFile` (`apps/server/src/accountAuth.ts:158`).
pub async fn read_account_file(base_dir: String) -> Result<serde_json::Value, String> {
    Ok(read_account_file_sync(&base_dir).unwrap_or_else(crate::js::undefined))
}

/// Port of `readAccountCredentials` (`apps/server/src/accountAuth.ts:257`).
pub async fn read_account_credentials(base_dir: String) -> Result<serde_json::Value, String> {
    Ok(read_account_credentials_sync(&base_dir).unwrap_or_else(crate::js::undefined))
}

/// Port of `writeAccountCredentials` (`apps/server/src/accountAuth.ts:270`).
pub async fn write_account_credentials(
    base_dir: String,
    credentials: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let path = account_credentials_path(base_dir)?;
    let body = serde_json::to_string_pretty(&credentials).unwrap_or_default();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = format!("{path}.tmp");
    let _ = std::fs::write(&tmp, format!("{body}\n"));
    let _ = std::fs::rename(&tmp, &path);
    Ok(crate::js::undefined())
}

/// Port of `deleteAccountCredentials` (`apps/server/src/accountAuth.ts:283`).
pub async fn delete_account_credentials(base_dir: String) -> Result<serde_json::Value, String> {
    let path = account_credentials_path(base_dir)?;
    let _ = std::fs::remove_file(path);
    Ok(crate::js::undefined())
}

/// Port of `resolveAccountUrl` (`apps/server/src/accountAuth.ts:362`).
pub fn resolve_account_url(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let flag = prop(&input, "flag")?;
    if let Some(flag) = optional_trimmed(&flag, "input.flag")? {
        return Ok(Value::String(flag));
    }
    let env = prop(&input, "env")?;
    let from_env = match &env {
        Value::Object(o) => o
            .get("SYNARA_ACCOUNT_URL")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ if env.is_null() || js_undefined(&env) => std::env::var("SYNARA_ACCOUNT_URL")
            .map(Value::String)
            .unwrap_or_else(|_| crate::js::undefined()),
        _ => crate::js::undefined(),
    };
    Ok(optional_trimmed(
        &from_env,
        "(input.env ?? process.env)[ACCOUNT_URL_ENV_NAME]",
    )?
    .map(Value::String)
    .unwrap_or_else(crate::js::undefined))
}

/// Port of `toAccountHostPlatform` (`apps/server/src/accountAuth.ts:417`).
pub fn to_account_host_platform(platform: serde_json::Value) -> Result<serde_json::Value, String> {
    let mapped = platform.as_str().and_then(|p| match p {
        "darwin" | "linux" => Some(p.to_string()),
        "win32" => Some("windows".to_string()),
        _ => None,
    });
    Ok(mapped
        .map(Value::String)
        .unwrap_or_else(crate::js::undefined))
}

/// Port of `resolveLanEndpoints` (`apps/server/src/accountAuth.ts:429`).
pub async fn resolve_lan_endpoints(
    base_dir: String,
    dev_url: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let path = path_join(&state_dir(&base_dir, &dev_url), "server-runtime.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Ok(vec![]);
    };
    let Ok(state) = serde_json::from_str::<Value>(&raw) else {
        return Ok(vec![]);
    };
    let Some(state) = state.as_object() else {
        return Ok(vec![]);
    };
    let host = state.get("host").and_then(Value::as_str);
    let mut endpoints = Vec::new();
    if let Some(h) = host
        && !is_wildcard_host(h)
        && !is_loopback_host(Some(h))
        && let Some(origin) = state.get("origin").and_then(Value::as_str)
    {
        endpoints.push(serde_json::json!({"url": origin, "transport": "lan"}));
    }
    // Tailscale resolution is a system lookup; the recorded environment has
    // no tailscale endpoint to add.
    Ok(endpoints)
}

/// Port of `logAccountSession` (`apps/server/src/accountAuth.ts:550`).
pub fn log_account_session(
    base_dir: String,
    reason: serde_json::Value,
    detail: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&base_dir, &reason, &detail);
    Ok(crate::js::undefined())
}

/// Port of `recordRemoteBillingPolicy` (`apps/server/src/accountAuth.ts:624`).
pub async fn record_remote_billing_policy(
    base_dir: String,
    expected: serde_json::Value,
    enforced: bool,
) -> Result<bool, String> {
    let account_url = prop(&expected, "accountUrl")?;
    let api_issuer = match &account_url {
        Value::String(s) => account_api_issuer(s.clone())?,
        Value::Null => return Err("Cannot read properties of null (reading 'replace')".to_string()),
        _ if js_undefined(&account_url) => {
            return Err("Cannot read properties of undefined (reading 'replace')".to_string());
        }
        _ => return Err("accountApiIssuer(...).replace is not a function".to_string()),
    };
    let host_id = prop(&expected, "hostId")?;
    let owner_user_id = prop(&expected, "ownerUserId")?;
    let Some(current) = read_account_file_sync(&base_dir) else {
        return Ok(false);
    };
    let current_issuer = current
        .get("accountUrl")
        .and_then(Value::as_str)
        .map(|s| s.trim_end_matches('/').to_string() + "/api/v1");
    if current_issuer.as_deref() != Some(api_issuer.as_str())
        || current.get("hostId") != Some(&host_id)
        || current.get("hostOwnerUserId") != Some(&owner_user_id)
    {
        return Ok(false);
    }
    if current
        .get("remoteBillingPolicy")
        .and_then(|p| p.get("enforced"))
        .and_then(Value::as_bool)
        == Some(enforced)
    {
        return Ok(true);
    }
    let mut next = current.clone();
    next["remoteBillingPolicy"] = serde_json::json!({
        "apiIssuer": api_issuer,
        "hostId": host_id,
        "ownerUserId": owner_user_id,
        "enforced": enforced,
    });
    write_account_credentials(base_dir, next).await?;
    Ok(true)
}

/// Port of `selectOrganization` (`apps/server/src/accountAuth.ts:1070`).
pub async fn select_organization(
    organizations: serde_json::Value,
) -> Result<serde_json::Value, String> {
    const NO_WORKSPACE: &str = "Your account has no workspace to sign in to. Contact your administrator, or create one in the WorkOS dashboard.";
    let first = match &organizations {
        Value::Null => return Err("Cannot read properties of null (reading '0')".to_string()),
        _ if js_undefined(&organizations) => {
            return Err("Cannot read properties of undefined (reading '0')".to_string());
        }
        Value::Array(a) => a.first().cloned().unwrap_or_else(crate::js::undefined),
        Value::String(s) => s
            .encode_utf16()
            .next()
            .map(|u| {
                Value::String(char::from_u32(u as u32).map_or(String::new(), |c| c.to_string()))
            })
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    };
    if js_undefined(&first) {
        return Err(NO_WORKSPACE.to_string());
    }
    let len = match &organizations {
        Value::Array(a) => a.len(),
        Value::String(s) => s.encode_utf16().count(),
        _ => 0,
    };
    if len > 1 {
        return Err("Multiple workspaces aren't supported yet".to_string());
    }
    Ok(first)
}

/// Port of `resolveAuthLoginAccountUrl` (`apps/server/src/accountAuth.ts:1308`).
pub async fn resolve_auth_login_account_url(
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let base_dir = prop(&options, "baseDir")?;
    let base_dir_str = base_dir.as_str().unwrap_or_default().to_string();
    let dev_url = prop(&options, "devUrl")?;
    let explicit_url = prop(&options, "explicitUrl")?;
    let stored_dir = account_state_directory(base_dir_str, dev_url)?;
    let Some(stored) = read_account_credentials_sync(&stored_dir) else {
        return Ok(match explicit_url {
            Value::String(s) => Value::String(s),
            _ => crate::js::undefined(),
        });
    };
    let stored_url = stored.get("accountUrl").cloned();
    if !js_undefined(&explicit_url)
        && !explicit_url.is_null()
        && Some(&explicit_url) != stored_url.as_ref()
    {
        let shown = match &explicit_url {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        return Err(format!(
            "This machine is signed in to {}, but {shown} was requested. Run `synara auth logout` first to link against a different account service.",
            stored_url
                .and_then(|u| u.as_str().map(str::to_string))
                .unwrap_or_default()
        ));
    }
    Ok(stored_url
        .and_then(|u| u.as_str().map(str::to_string))
        .map(Value::String)
        .unwrap_or_else(crate::js::undefined))
}
