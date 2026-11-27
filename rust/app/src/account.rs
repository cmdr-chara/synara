//! account.* backed by the hosted account service.
//!
//! The server owns the stored credential file, token rotation, OTP/SSO
//! sign-in, and every account-service round trip. OTP codes, authorization
//! codes, PKCE verifiers, access tokens, and refresh tokens are credentials:
//! they are never logged and only the token pair is persisted, in the private
//! account-credentials file.

use base64::Engine as _;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::todo::WsErr;

const DEFAULT_ACCOUNT_URL: &str = "https://api.synara.vrbty.dev";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const GRANT_TIMEOUT: Duration = Duration::from_secs(60);
const SSO_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAX_SSO_ATTEMPTS: usize = 4;

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

fn account_dir() -> PathBuf {
    let state = PathBuf::from(crate::todo::state_dir());
    if state.file_name().and_then(|name| name.to_str()) == Some("userdata") {
        state.parent().map(PathBuf::from).unwrap_or(state)
    } else {
        state
    }
}

fn credentials_path() -> PathBuf {
    account_dir().join("account-credentials.json")
}

fn read_account_file() -> Option<Value> {
    let text = std::fs::read_to_string(credentials_path()).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    if !value.is_object() || value.get("deviceToken").is_some() {
        return None;
    }
    for key in ["accountUrl", "workosClientId", "workosApiUrl"] {
        if value.get(key).and_then(Value::as_str).is_none() {
            return None;
        }
    }
    Some(value)
}

pub(crate) fn write_account_file(value: &Value) -> Result<(), WsErr> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| err(format!("Failed to save the account session: {e}")))?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = format!("{}\n", serde_json::to_string_pretty(value).map_err(|e| err(e.to_string()))?);
    std::fs::write(&tmp, body).map_err(|e| err(format!("Failed to save the account session: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &path).map_err(|e| err(format!("Failed to save the account session: {e}")))?;
    Ok(())
}

pub(crate) fn read_session_file() -> Option<Value> {
    let value = read_account_file()?;
    for key in ["accessToken", "refreshToken", "organizationId"] {
        value.get(key).and_then(Value::as_str)?;
    }
    Some(value)
}

fn clear_user_session(mut value: Value) {
    if let Some(object) = value.as_object_mut() {
        for key in ["accessToken", "refreshToken", "organizationId", "userId", "refreshPendingTokenHash"] {
            object.remove(key);
        }
    }
    let _ = write_account_file(&value);
}

fn configured_account_url() -> String {
    std::env::var("SYNARA_ACCOUNT_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ACCOUNT_URL.to_string())
}

fn account_api_issuer(account_url: &str) -> String {
    format!("{}/api/v1", account_url.trim_end_matches('/'))
}

#[derive(Debug, Clone)]
pub(crate) struct AccountError {
    status: u16,
    code: String,
    message: String,
    organizations: Vec<Value>,
}

impl AccountError {
    fn unauthorized(&self) -> bool {
        self.status == 401 || self.code == "token_revoked"
    }
    fn organization_required(&self) -> bool {
        self.code == "organization_required" && !self.organizations.is_empty()
    }
}

#[derive(Clone)]
pub(crate) struct AccountClient {
    base_url: String,
}

impl AccountClient {
    fn new(base_url: &str) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_string() }
    }

    pub(crate) fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        bearer: Option<&str>,
        body: Option<Value>,
        raw_body: Option<(Vec<u8>, String)>,
        timeout: Duration,
    ) -> Result<Value, AccountError> {
        let client = reqwest::blocking::Client::builder()
            .build()
            .map_err(|error| AccountError {
                status: 0,
                code: "internal_error".into(),
                message: error.to_string(),
                organizations: Vec::new(),
            })?;
        let mut request = client.request(method, format!("{}{path}", self.base_url)).timeout(timeout);
        if let Some(token) = bearer {
            request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some((bytes, content_type)) = raw_body {
            request = request.header(reqwest::header::CONTENT_TYPE, content_type).body(bytes);
        } else if let Some(body) = body {
            request = request.header(reqwest::header::CONTENT_TYPE, "application/json").body(body.to_string());
        }
        let response = match request.send() {
            Ok(response) => response,
            Err(error) => {
                return Err(AccountError {
                    status: if error.is_timeout() { 408 } else { 0 },
                    code: "internal_error".into(),
                    message: format!("Request to {path} failed"),
                    organizations: Vec::new(),
                });
            }
        };
        let status = response.status().as_u16();
        let text = response.text().unwrap_or_default();
        let value = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
        if (200..300).contains(&status) {
            return Ok(value);
        }
        let organizations = value
            .get("organizations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let code = value.get("error").and_then(Value::as_str).unwrap_or("internal_error").to_string();
        let message = value
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| value.get("error_description").and_then(Value::as_str))
            .unwrap_or("The account service request failed.")
            .to_string();
        Err(AccountError { status, code, message, organizations })
    }

    fn get(&self, path: &str, bearer: Option<&str>) -> Result<Value, AccountError> {
        self.request(reqwest::Method::GET, path, bearer, None, None, REQUEST_TIMEOUT)
    }

    fn post(&self, path: &str, bearer: Option<&str>, body: Value) -> Result<Value, AccountError> {
        self.request(reqwest::Method::POST, path, bearer, Some(body), None, REQUEST_TIMEOUT)
    }

    fn post_grant(&self, path: &str, body: Value) -> Result<Value, AccountError> {
        self.request(reqwest::Method::POST, path, None, Some(body), None, GRANT_TIMEOUT)
    }
}

fn service_error(error: &AccountError, fallback: &str) -> WsErr {
    if error.message.trim().is_empty() { err(fallback) } else { err(error.message.clone()) }
}

fn session_expired() -> WsErr {
    err("Session expired — sign in again from the Synara app.")
}

fn refresh_session(mut stored: Value, organization_override: Option<&str>) -> Result<Value, WsErr> {
    let account_url = stored["accountUrl"].as_str().unwrap_or_default().to_string();
    let refresh_token = stored["refreshToken"].as_str().unwrap_or_default().to_string();
    let organization_id = organization_override
        .map(str::to_string)
        .or_else(|| stored["organizationId"].as_str().map(str::to_string));
    let client = AccountClient::new(&account_url);
    let mut body = json!({"refreshToken": refresh_token});
    if let Some(organization_id) = organization_id {
        body["organizationId"] = json!(organization_id);
    }
    match client.post_grant("/api/v1/auth/refresh", body) {
        Ok(tokens) => {
            if let Some(object) = stored.as_object_mut() {
                object.insert("accessToken".into(), tokens["accessToken"].clone());
                object.insert("refreshToken".into(), tokens["refreshToken"].clone());
                if let Some(user) = tokens.get("user").and_then(|user| user.get("id")).and_then(Value::as_str) {
                    object.insert("userId".into(), json!(user));
                }
                if let Some(organization) = tokens.get("organizationId").and_then(Value::as_str) {
                    object.insert("organizationId".into(), json!(organization));
                }
            }
            write_account_file(&stored)?;
            Ok(stored)
        }
        Err(error) => {
            if error.unauthorized() {
                clear_user_session(stored);
                return Err(session_expired());
            }
            Err(service_error(&error, "Failed to refresh the account session"))
        }
    }
}

pub(crate) fn authenticated_request(
    fallback: &str,
    operation: impl Fn(&AccountClient, &str) -> Result<Value, AccountError>,
) -> Result<Value, WsErr> {
    let stored = read_session_file().ok_or_else(session_expired)?;
    let account_url = stored["accountUrl"].as_str().unwrap_or_default().to_string();
    let client = AccountClient::new(&account_url);
    let access = stored["accessToken"].as_str().unwrap_or_default().to_string();
    match operation(&client, &access) {
        Ok(value) => Ok(value),
        Err(first) => {
            let organization_override = if first.organization_required() {
                if first.organizations.len() > 1 {
                    return Err(err("Multiple workspaces aren't supported yet"));
                }
                first.organizations.first().and_then(|org| org.get("id")).and_then(Value::as_str).map(str::to_string)
            } else {
                None
            };
            if !first.unauthorized() && !first.organization_required() {
                return Err(service_error(&first, fallback));
            }
            if first.organization_required() && organization_override.is_none() {
                return Err(service_error(&first, fallback));
            }
            let refreshed = refresh_session(stored, organization_override.as_deref())?;
            let access = refreshed["accessToken"].as_str().unwrap_or_default().to_string();
            operation(&client, &access).map_err(|error| service_error(&error, fallback))
        }
    }
}

fn signed_in_status(me: Value) -> Result<Value, WsErr> {
    let Some(stored) = read_session_file() else {
        return Ok(json!({"state": "signed-out"}));
    };
    let user_id = me.get("id").and_then(Value::as_str).unwrap_or_default();
    let organization_id = me.pointer("/organization/id").and_then(Value::as_str).unwrap_or_default();
    if stored.get("userId").and_then(Value::as_str) != Some(user_id)
        || stored.get("organizationId").and_then(Value::as_str) != Some(organization_id)
    {
        return Ok(json!({"state": "signed-out"}));
    }
    Ok(json!({
        "state": "signed-in",
        "me": me,
        "accountAuthority": account_api_issuer(stored["accountUrl"].as_str().unwrap_or_default()),
    }))
}

fn scope_token_to_workspace(client: &AccountClient, token: &Value) -> Result<Value, WsErr> {
    let access = token["accessToken"].as_str().unwrap_or_default();
    let refresh = token["refreshToken"].as_str().unwrap_or_default();
    match client.get("/api/v1/me", Some(access)) {
        Ok(me) => {
            let organization_id = me.pointer("/organization/id").and_then(Value::as_str).unwrap_or_default();
            let user_id = me.get("id").and_then(Value::as_str).unwrap_or_default();
            if organization_id.is_empty() || user_id.is_empty() {
                return Err(err("The account service returned an incomplete workspace session."));
            }
            Ok(json!({
                "accessToken": access,
                "refreshToken": refresh,
                "organizationId": organization_id,
                "userId": user_id,
            }))
        }
        Err(error) if error.organization_required() => {
            if error.organizations.is_empty() {
                return Err(err("Your account has no workspace to sign in to. Contact your administrator, or create one in the WorkOS dashboard."));
            }
            if error.organizations.len() > 1 {
                return Err(err("Multiple workspaces aren't supported yet"));
            }
            let organization_id = error.organizations[0].get("id").and_then(Value::as_str).unwrap_or_default();
            let refreshed = client
                .post_grant("/api/v1/auth/refresh", json!({"refreshToken": refresh, "organizationId": organization_id}))
                .map_err(|error| service_error(&error, "Failed to scope the account session"))?;
            let user_id = refreshed.pointer("/user/id").and_then(Value::as_str).unwrap_or_default();
            Ok(json!({
                "accessToken": refreshed["accessToken"].clone(),
                "refreshToken": refreshed["refreshToken"].clone(),
                "organizationId": organization_id,
                "userId": user_id,
            }))
        }
        Err(error) => Err(service_error(&error, "Failed to read the account workspace")),
    }
}


fn secrets_dir() -> PathBuf {
    PathBuf::from(crate::todo::state_dir()).join("secrets")
}

fn device_identity_path() -> PathBuf {
    secrets_dir().join("device-identity.json")
}

pub(crate) fn host_identity_file_path() -> PathBuf {
    crate::host_identity::host_identity_path(secrets_dir().to_string_lossy().as_ref())
}

fn account_platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        other => other,
    }
}

fn display_hostname() -> String {
    let output = std::process::Command::new("hostname").output().ok();
    output
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|name| name.trim().chars().take(200).collect())
        .filter(|name: &String| !name.is_empty())
        .unwrap_or_else(|| "Synara host".to_string())
}

fn sign_device_registration(
    identity: &crate::device_identity::DeviceIdentity,
    user_id: &str,
    audience: &str,
) -> Result<(String, String), WsErr> {
    let proof = identity
        .sign_registration(user_id, &display_hostname(), account_platform(), audience)
        .map_err(err)?;
    Ok((proof, identity.jkt.clone()))
}

pub(crate) fn register_shell_device(user_id: &str) -> Result<(), WsErr> {
    let stored = read_session_file().ok_or_else(session_expired)?;
    let account_url = stored["accountUrl"].as_str().unwrap_or_default().to_string();
    let identity = crate::device_identity::load_or_create(&crate::device_identity::device_identity_path(
        secrets_dir().to_string_lossy().as_ref(),
    )).map_err(err)?;
    let (proof, jkt) = sign_device_registration(&identity, user_id, &account_api_issuer(&account_url))?;
    let response = authenticated_request("Failed to register this device", |client, access| {
        client.post("/api/v1/devices", Some(access), json!({"proof": proof}))
    })?;
    let device = response.get("device").cloned().unwrap_or(Value::Null);
    if device.get("jkt").and_then(Value::as_str) != Some(jkt.as_str()) || !device.get("revokedAt").is_some_and(Value::is_null) {
        return Err(err("The account did not register this shell's device key"));
    }
    let mut current = read_session_file().ok_or_else(session_expired)?;
    if let Some(object) = current.as_object_mut() {
        object.insert("deviceId".into(), device.get("id").cloned().unwrap_or(Value::Null));
        object.insert("deviceJkt".into(), json!(jkt));
        object.insert("userId".into(), json!(user_id));
    }
    write_account_file(&current)
}

fn link_local_host() -> Result<(), WsErr> {
    let stored = read_session_file().ok_or_else(session_expired)?;
    if stored.get("hostId").and_then(Value::as_str).is_some()
        && stored.get("hostOwnerUserId").and_then(Value::as_str) == stored.get("userId").and_then(Value::as_str)
        && stored.get("hostKeyGeneration").is_some()
        && crate::host_identity::read_host_identity(&host_identity_file_path()).ok().flatten().is_some()
    {
        let _ = publish_local_host_endpoints();
        return Ok(());
    }
    let state_dir = crate::todo::state_dir();
    let environment_id = crate::server_env::environment_id(&state_dir).ok_or_else(|| err("Failed to resolve the environment id"))?;
    let name = display_hostname();
    let platform = account_platform();
    let challenge = authenticated_request("Failed to link this host", |client, access| {
        client.post("/api/v1/hosts/link/start", Some(access), json!({
            "environmentId": environment_id,
            "name": name,
            "platform": platform,
            "kind": "local",
        }))
    })?;
    if let Some(parent) = host_identity_file_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let identity = crate::host_identity::generate_and_persist_host_identity(&host_identity_file_path()).map_err(err)?;
    let proof = crate::host_identity::mint_host_link_proof(
        &identity,
        &account_api_issuer(stored["accountUrl"].as_str().unwrap_or_default()),
        &environment_id,
        challenge.get("challengeId").and_then(Value::as_str).unwrap_or_default(),
        challenge.get("nonce").and_then(Value::as_str).unwrap_or_default(),
        &name,
        platform,
        Some(env!("CARGO_PKG_VERSION")),
    ).map_err(err)?;
    let linked = AccountClient::new(stored["accountUrl"].as_str().unwrap_or_default())
        .post("/api/v1/hosts/link/complete", None, json!({
            "challengeId": challenge.get("challengeId").and_then(Value::as_str).unwrap_or_default(),
            "proof": proof,
        }))
        .map_err(|error| service_error(&error, "Failed to link this host"))?;
    let host = linked.get("host").cloned().unwrap_or(Value::Null);
    let host_id = host.get("id").and_then(Value::as_str).unwrap_or_default();
    let owner = host.get("ownerUserId").and_then(Value::as_str).unwrap_or_default();
    if host_id.is_empty() || owner.is_empty() {
        return Err(err("The account service returned an incomplete host link"));
    }
    let mut current = read_session_file().ok_or_else(session_expired)?;
    if current.get("userId").and_then(Value::as_str) != Some(owner) {
        return Err(err("Your workspace access changed — sign in again from the Synara app."));
    }
    if let Some(object) = current.as_object_mut() {
        object.insert("hostId".into(), json!(host_id));
        object.insert("hostOwnerUserId".into(), json!(owner));
        object.insert("hostKeyGeneration".into(), host.get("keyGeneration").cloned().unwrap_or(json!(0)));
    }
    write_account_file(&current)
}


fn host_is_wildcard(host: &str) -> bool {
    matches!(host, "0.0.0.0" | "::" | "*")
}

fn host_is_loopback(host: &str) -> bool {
    let normalized = host.trim_matches(['[', ']']).trim_end_matches('.').to_ascii_lowercase();
    normalized == "localhost" || normalized == "::1" || normalized.strip_prefix("127.").is_some()
}

fn tailscale_endpoint(local_port: u16) -> Option<Value> {
    let status = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let serve = std::process::Command::new("tailscale")
        .args(["serve", "status", "--json"])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let status: Value = serde_json::from_slice(&status.stdout).ok()?;
    let serve: Value = serde_json::from_slice(&serve.stdout).ok()?;
    if status.get("BackendState").and_then(Value::as_str) != Some("Running")
        || status.pointer("/Self/Online").and_then(Value::as_bool) == Some(false)
    {
        return None;
    }
    let dns_name = status
        .pointer("/Self/DNSName")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_string();
    if dns_name.is_empty()
        || !status.get("CertDomains").and_then(Value::as_array).is_some_and(|domains| {
            domains.iter().any(|domain| domain.as_str().unwrap_or_default().trim_end_matches('.') == dns_name)
        })
    {
        return None;
    }
    for (host_port, entry) in serve.get("Web").and_then(Value::as_object)? {
        for handler in entry.get("Handlers").and_then(Value::as_object).map(|handlers| handlers.values()).into_iter().flatten() {
            let proxy = handler.get("Proxy").and_then(Value::as_str).unwrap_or_default();
            let Ok(url) = reqwest::Url::parse(proxy) else { continue };
            if url.port_or_known_default() != Some(local_port) { continue; }
            let port = host_port.rsplit(':').next().and_then(|part| part.parse::<u16>().ok()).unwrap_or(443);
            return Some(json!({
                "url": if port == 443 { format!("https://{dns_name}") } else { format!("https://{dns_name}:{port}") },
                "transport": "tailscale",
            }));
        }
    }
    None
}

fn local_endpoints() -> Vec<Value> {
    let state_path = PathBuf::from(crate::todo::state_dir()).join("server-runtime.json");
    let state: Value = std::fs::read_to_string(state_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null);
    let mut endpoints = Vec::new();
    let host = state.get("host").and_then(Value::as_str).unwrap_or_default();
    let origin = state.get("origin").and_then(Value::as_str).unwrap_or_default();
    if !host.is_empty() && !host_is_wildcard(host) && !host_is_loopback(host) && !origin.is_empty() {
        endpoints.push(json!({"url": origin, "transport": "lan"}));
    }
    if let Some(port) = state.get("port").and_then(Value::as_u64).filter(|port| *port <= u16::MAX as u64)
        && let Some(endpoint) = tailscale_endpoint(port as u16)
    {
        endpoints.push(endpoint);
    }
    endpoints.truncate(8);
    endpoints
}

pub(crate) fn publish_local_host_endpoints() -> Result<(), WsErr> {
    let stored = read_session_file().ok_or_else(session_expired)?;
    let host_id = stored.get("hostId").and_then(Value::as_str).unwrap_or_default();
    let owner = stored.get("hostOwnerUserId").and_then(Value::as_str).unwrap_or_default();
    let key_generation = stored.get("hostKeyGeneration").and_then(Value::as_u64).unwrap_or(0);
    if host_id.is_empty() || owner.is_empty() {
        return Ok(());
    }
    let endpoints = local_endpoints();
    if endpoints.is_empty() {
        return Ok(());
    }
    let identity = crate::host_identity::read_host_identity(&host_identity_file_path())
        .map_err(err)?
        .ok_or_else(|| err("The host identity is missing"))?;
    let state_dir = crate::todo::state_dir();
    let environment_id = crate::server_env::environment_id(&state_dir)
        .ok_or_else(|| err("Failed to resolve the environment id"))?;
    let proof = crate::host_identity::mint_host_proof(
        &identity,
        &account_api_issuer(stored["accountUrl"].as_str().unwrap_or_default()),
        &environment_id,
        host_id,
        key_generation,
    ).map_err(err)?;
    let client = reqwest::blocking::Client::builder().build().map_err(|e| err(e.to_string()))?;
    let response = client
        .put(format!("{}/api/v1/hosts/{host_id}/endpoints", stored["accountUrl"].as_str().unwrap_or_default().trim_end_matches('/')))
        .header(reqwest::header::AUTHORIZATION, format!("HostProof {proof}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(json!({"endpoints": endpoints, "appVersion": env!("CARGO_PKG_VERSION")}).to_string())
        .timeout(REQUEST_TIMEOUT)
        .send()
        .map_err(|_| err("Failed to publish host endpoints"))?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Err(err("Failed to publish host endpoints"));
    }
    Ok(())
}

fn enroll_desktop(user_id: &str) {
    // Enrollment is best-effort and retried by later account operations; the
    // durable user session must remain useful when device/host registration is
    // temporarily unavailable.
    let _ = register_shell_device(user_id);
    let _ = link_local_host();
}

fn establish_session(token: Value) -> Result<Value, WsErr> {
    let account_url = configured_account_url();
    let client = AccountClient::new(&account_url);
    let instance = client.get("/api/v1/instance", None)
        .map_err(|error| service_error(&error, "Failed to read the account service"))?;
    let scoped = scope_token_to_workspace(&client, &token)?;
    let previous = read_account_file();
    let same_account_and_workspace = previous.as_ref().is_some_and(|previous| {
        previous.get("accountUrl").and_then(Value::as_str) == Some(account_url.as_str())
            && previous.get("hostOwnerUserId").and_then(Value::as_str) == scoped.get("userId").and_then(Value::as_str)
            && (previous.get("organizationId").and_then(Value::as_str) == scoped.get("organizationId").and_then(Value::as_str)
                || (previous.get("organizationId").is_none() && previous.get("hostId").is_some()))
    });
    let mut stored = json!({
        "accountUrl": account_url,
        "workosClientId": instance.get("clientId").and_then(Value::as_str).unwrap_or_default(),
        "workosApiUrl": instance.get("workosApiUrl").and_then(Value::as_str).unwrap_or_default(),
        "organizationId": scoped["organizationId"].clone(),
        "userId": scoped["userId"].clone(),
        "accessToken": scoped["accessToken"].clone(),
        "refreshToken": scoped["refreshToken"].clone(),
    });
    if same_account_and_workspace {
        if let (Some(object), Some(previous)) = (stored.as_object_mut(), previous.as_ref()) {
            for key in ["hostId", "hostOwnerUserId", "hostKeyGeneration", "discoverabilityAcknowledgedByHostId"] {
                if let Some(value) = previous.get(key) {
                    object.insert(key.into(), value.clone());
                }
            }
        }
    }
    write_account_file(&stored)?;
    enroll_desktop(scoped["userId"].as_str().unwrap_or_default());
    let me = authenticated_request("Failed to read the signed-in account", |client, access| client.get("/api/v1/me", Some(access)))?;
    signed_in_status(me)
}

fn random_base64url(bytes: usize) -> String {
    let mut data = vec![0u8; bytes];
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut data);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn create_pkce_pair() -> (String, String) {
    let verifier = random_base64url(32);
    let digest = <sha2::Sha256 as sha2::Digest>::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    (verifier, challenge)
}

struct SsoAttempt {
    authorize_url: String,
    verifier: String,
    outcome: Arc<(Mutex<Option<Result<String, String>>>, Condvar)>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

fn sso_attempts() -> &'static Mutex<HashMap<String, SsoAttempt>> {
    static ATTEMPTS: OnceLock<Mutex<HashMap<String, SsoAttempt>>> = OnceLock::new();
    ATTEMPTS.get_or_init(|| Mutex::new(HashMap::new()))
}

const SSO_SUCCESS_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Synara</title>\n<body style=\"font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\">\n<div style=\"text-align:center\"><h1 style=\"font-size:1.2rem\">Signed in</h1>\n<p>You can close this window and return to Synara.</p></div></body>";
const SSO_FAILURE_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Synara</title>\n<body style=\"font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\">\n<div style=\"text-align:center\"><h1 style=\"font-size:1.2rem\">Sign-in not completed</h1>\n<p>You can close this window and try again from Synara.</p></div></body>";

fn decode_query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        if parts.next()? != key {
            continue;
        }
        let raw = parts.next().unwrap_or_default();
        let mut out = Vec::new();
        let bytes = raw.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' && index + 2 < bytes.len() {
                if let Ok(value) = u8::from_str_radix(&raw[index + 1..index + 3], 16) {
                    out.push(value);
                    index += 3;
                    continue;
                }
            }
            out.push(if bytes[index] == b'+' { b' ' } else { bytes[index] });
            index += 1;
        }
        return Some(String::from_utf8_lossy(&out).into_owned());
    }
    None
}

fn start_sso_listener(state: String) -> Result<(String, Arc<(Mutex<Option<Result<String, String>>>, Condvar)>, Arc<std::sync::atomic::AtomicBool>), WsErr> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|error| err(format!("Failed to start sign-in listener: {error}")))?;
    let port = listener.local_addr().map_err(|error| err(error.to_string()))?.port();
    listener.set_nonblocking(true).map_err(|error| err(error.to_string()))?;
    let outcome = Arc::new((Mutex::new(None), Condvar::new()));
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let thread_outcome = Arc::clone(&outcome);
    let thread_cancel = Arc::clone(&cancel);
    std::thread::spawn(move || {
        let deadline = Instant::now() + SSO_TIMEOUT;
        let finish = |result: Result<String, String>| {
            let (lock, cvar) = &*thread_outcome;
            if let Ok(mut guard) = lock.lock() {
                if guard.is_none() {
                    *guard = Some(result);
                }
            }
            cvar.notify_all();
        };
        loop {
            if thread_cancel.load(std::sync::atomic::Ordering::SeqCst) {
                finish(Err("The sign-in attempt was cancelled.".into()));
                return;
            }
            if Instant::now() >= deadline {
                finish(Err("The sign-in attempt timed out. Start a new one.".into()));
                return;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                    let mut buffer = [0u8; 8192];
                    let size = stream.read(&mut buffer).unwrap_or(0);
                    let request = String::from_utf8_lossy(&buffer[..size]);
                    let target = request.lines().next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or("/");
                    let (path, query) = target.split_once('?').unwrap_or((target, ""));
                    if path != "/callback" {
                        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\ncontent-type: text/plain\r\ncontent-length: 9\r\nconnection: close\r\n\r\nNot found");
                        continue;
                    }
                    let result = if decode_query_param(query, "error").is_some() {
                        (400, Err("The sign-in was cancelled or denied in the browser.".to_string()))
                    } else if decode_query_param(query, "state").as_deref() != Some(state.as_str()) {
                        (400, Err("The sign-in callback did not match this attempt. Start a new sign-in.".to_string()))
                    } else if let Some(code) = decode_query_param(query, "code") {
                        (200, Ok(code))
                    } else {
                        (400, Err("The sign-in callback carried no authorization code. Start a new sign-in.".to_string()))
                    };
                    let (status, result) = result;
                    let page = if result.is_ok() { SSO_SUCCESS_PAGE } else { SSO_FAILURE_PAGE };
                    let response = format!(
                        "HTTP/1.1 {} {}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        status,
                        if status == 200 { "OK" } else { "Bad Request" },
                        page.len(),
                        page,
                    );
                    let _ = stream.write_all(response.as_bytes());
                    finish(result);
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(40));
                }
                Err(_) => {
                    finish(Err("The sign-in listener failed. Start a new sign-in.".into()));
                    return;
                }
            }
        }
    });
    Ok((format!("http://127.0.0.1:{port}/callback"), outcome, cancel))
}

fn begin_sso(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload.get("provider").and_then(Value::as_str).unwrap_or_default();
    if !matches!(provider, "google" | "apple" | "github") {
        return Err(err("Unsupported sign-in provider"));
    }
    let (verifier, challenge) = create_pkce_pair();
    let state = random_base64url(16);
    let (redirect_uri, outcome, cancel) = start_sso_listener(state.clone())?;
    let client = AccountClient::new(&configured_account_url());
    let response = client
        .post("/api/v1/auth/authorize", None, json!({
            "provider": provider,
            "redirectUri": redirect_uri,
            "codeChallenge": challenge,
            "state": state,
        }))
        .map_err(|error| service_error(&error, "Failed to start sign-in"))?;
    let authorize_url = response.get("authorizeUrl").and_then(Value::as_str).unwrap_or_default().to_string();
    if authorize_url.is_empty() {
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        return Err(err("Failed to start sign-in"));
    }
    let sso_id = crate::orchestration_engine::uuid_v4();
    let mut attempts = sso_attempts().lock().map_err(|_| err("Failed to start sign-in"))?;
    while attempts.len() >= MAX_SSO_ATTEMPTS {
        if let Some(oldest) = attempts.keys().next().cloned() {
            if let Some(old) = attempts.remove(&oldest) {
                old.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        } else {
            break;
        }
    }
    attempts.insert(sso_id.clone(), SsoAttempt { authorize_url: authorize_url.clone(), verifier, outcome, cancel });
    Ok(json!({"ssoId": sso_id, "authorizeUrl": authorize_url}))
}

fn wait_for_sso_code(attempt: &SsoAttempt) -> Result<String, WsErr> {
    let outcome = Arc::clone(&attempt.outcome);
    let (lock, cvar) = &*outcome;
    let mut guard = lock.lock().map_err(|_| err("Failed to finish signing in"))?;
    let deadline = Instant::now() + SSO_TIMEOUT;
    loop {
        if let Some(result) = guard.take() {
            return result.map_err(err);
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(err("The sign-in attempt timed out. Start a new one."));
        }
        let remaining = deadline.saturating_duration_since(now).min(Duration::from_millis(250));
        let (next, _) = cvar.wait_timeout(guard, remaining).map_err(|_| err("Failed to finish signing in"))?;
        guard = next;
    }
}

fn complete_sso(payload: &Value) -> Result<Value, WsErr> {
    let sso_id = payload.get("ssoId").and_then(Value::as_str).unwrap_or_default();
    let attempt = {
        let attempts = sso_attempts().lock().map_err(|_| err("Failed to finish signing in"))?;
        attempts.get(sso_id).map(|attempt| SsoAttempt {
            authorize_url: attempt.authorize_url.clone(),
            verifier: attempt.verifier.clone(),
            outcome: Arc::clone(&attempt.outcome),
            cancel: Arc::clone(&attempt.cancel),
        })
    };
    let Some(attempt) = attempt else {
        return Err(err("This sign-in attempt has expired. Start a new one."));
    };
    let code = wait_for_sso_code(&attempt)?;
    let client = AccountClient::new(&configured_account_url());
    let token = client
        .post_grant("/api/v1/auth/authorize/token", json!({"code": code, "codeVerifier": attempt.verifier}))
        .map_err(|error| service_error(&error, "Failed to finish signing in"))?;
    let status = establish_session(token)?;
    if let Ok(mut attempts) = sso_attempts().lock() {
        attempts.remove(sso_id);
    }
    attempt.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(status)
}

fn cancel_sso(payload: &Value) -> Result<Value, WsErr> {
    let sso_id = payload.get("ssoId").and_then(Value::as_str).unwrap_or_default();
    if let Ok(mut attempts) = sso_attempts().lock()
        && let Some(attempt) = attempts.remove(sso_id)
    {
        attempt.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        let (lock, cvar) = &*attempt.outcome;
        if let Ok(mut guard) = lock.lock()
            && guard.is_none()
        {
            *guard = Some(Err("The sign-in attempt was cancelled.".into()));
        }
        cvar.notify_all();
    }
    Ok(json!({}))
}

fn verification_url_allowed(payload: &Value) -> Result<Value, WsErr> {
    let url = payload.get("url").and_then(Value::as_str).unwrap_or_default();
    let allowed = sso_attempts()
        .lock()
        .map(|attempts| attempts.values().any(|attempt| attempt.authorize_url == url))
        .unwrap_or(false);
    if allowed {
        #[cfg(target_os = "macos")]
        let mut command = std::process::Command::new("open");
        #[cfg(target_os = "linux")]
        let mut command = std::process::Command::new("xdg-open");
        #[cfg(target_os = "windows")]
        let mut command = {
            let mut command = std::process::Command::new("rundll32");
            command.arg("url.dll,FileProtocolHandler");
            command
        };
        let _ = command.arg(url).spawn();
        Ok(json!({}))
    } else {
        Err(err("That link is not a sign-in verification URL from this server."))
    }
}

fn sign_out() -> Result<Value, WsErr> {
    if let Some(mut stored) = read_account_file() {
        if let Some(object) = stored.as_object_mut() {
            for key in ["accessToken", "refreshToken", "organizationId", "userId", "refreshPendingTokenHash"] {
                object.remove(key);
            }
        }
        write_account_file(&stored)?;
    }
    Ok(json!({}))
}

fn account_status() -> Result<Value, WsErr> {
    if read_session_file().is_none() {
        return Ok(json!({"state": "signed-out"}));
    }
    match authenticated_request("Failed to read the account", |client, access| client.get("/api/v1/me", Some(access))) {
        Ok(me) => {
            if let Some(user_id) = me.get("id").and_then(Value::as_str) {
                let incomplete = read_session_file().is_some_and(|stored| {
                    stored.get("deviceId").is_none()
                        || stored.get("hostId").is_none()
                        || stored.get("hostKeyGeneration").is_none()
                });
                if incomplete {
                    enroll_desktop(user_id);
                }
            }
            signed_in_status(me)
        }
        Err(error) if error.0.contains("Session expired") => Ok(json!({"state": "signed-out"})),
        Err(error) => Err(error),
    }
}

fn send_otp(payload: &Value) -> Result<Value, WsErr> {
    let email = payload.get("email").and_then(Value::as_str).unwrap_or_default();
    if email.trim().is_empty() || email.len() > 320 {
        return Err(err("Could not send the code. Try again in a minute."));
    }
    AccountClient::new(&configured_account_url())
        .post("/api/v1/auth/otp/send", None, json!({"email": email}))
        .map_err(|error| service_error(&error, "Could not send the code. Try again in a minute."))
}

fn authenticate_otp(payload: &Value) -> Result<Value, WsErr> {
    let email = payload.get("email").and_then(Value::as_str).unwrap_or_default();
    let code = payload.get("code").and_then(Value::as_str).unwrap_or_default();
    if email.trim().is_empty() || !(code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit())) {
        return Err(err("Could not sign in. Check the code and try again."));
    }
    let token = AccountClient::new(&configured_account_url())
        .post_grant("/api/v1/auth/otp/authenticate", json!({"email": email, "code": code}))
        .map_err(|error| service_error(&error, "Could not sign in. Check the code and try again."))?;
    establish_session(token)
}

fn authed_json(method: reqwest::Method, path: &str, body: Option<Value>, fallback: &str) -> Result<Value, WsErr> {
    authenticated_request(fallback, |client, access| {
        client.request(method.clone(), path, Some(access), body.clone(), None, REQUEST_TIMEOUT)
    })
}

fn update_profile(payload: &Value) -> Result<Value, WsErr> {
    let workspace_name = payload.get("workspaceName").and_then(Value::as_str).map(str::to_string);
    let mut profile = payload.clone();
    if let Some(object) = profile.as_object_mut() {
        object.remove("workspaceName");
    }
    if let Some(name) = workspace_name {
        authed_json(reqwest::Method::PATCH, "/api/v1/organization", Some(json!({"name": name})), "Failed to save your workspace")?;
    }
    authed_json(reqwest::Method::PUT, "/api/v1/profile", Some(profile), "Failed to save your profile")
}

fn upload_avatar(payload: &Value) -> Result<Value, WsErr> {
    let encoded = payload.get("bytes").and_then(Value::as_str).unwrap_or_default();
    let content_type = payload.get("contentType").and_then(Value::as_str).unwrap_or_default();
    if encoded.is_empty() || encoded.len() > 400_000 || !matches!(content_type, "image/webp" | "image/jpeg" | "image/png") {
        return Err(err("Failed to upload your avatar"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| err("Failed to upload your avatar"))?;
    authenticated_request("Failed to upload your avatar", |client, access| {
        client.request(reqwest::Method::PUT, "/api/v1/profile/avatar", Some(access), None, Some((bytes.clone(), content_type.to_string())), REQUEST_TIMEOUT)
    })
}

pub fn handles(tag: &str) -> bool {
    tag.starts_with("account.")
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "account.status" => account_status(),
        "account.signOut" => sign_out(),
        "account.sendOtp" => send_otp(payload),
        "account.authenticateOtp" => authenticate_otp(payload),
        "account.beginSso" => begin_sso(payload),
        "account.completeSso" => complete_sso(payload),
        "account.cancelSso" => cancel_sso(payload),
        "account.openVerificationUrl" => verification_url_allowed(payload),
        "account.usageSummary" => {
            let offset = payload.get("utcOffsetMinutes").and_then(Value::as_i64).unwrap_or(0);
            authed_json(reqwest::Method::GET, &format!("/api/v1/usage/summary?utcOffsetMinutes={offset}"), None, "Failed to load account usage")
        }
        "account.saveInboxRecap" => authed_json(reqwest::Method::PUT, "/api/v1/inbox/recaps", Some(payload.clone()), "Failed to save the private recap"),
        "account.listInboxRecaps" => {
            let mut query = Vec::new();
            if let Some(limit) = payload.get("limit").and_then(Value::as_i64) { query.push(format!("limit={limit}")); }
            if let Some(cursor) = payload.get("cursor").and_then(Value::as_str) { query.push(format!("cursor={}", cursor)); }
            let suffix = if query.is_empty() { String::new() } else { format!("?{}", query.join("&")) };
            authed_json(reqwest::Method::GET, &format!("/api/v1/inbox/recaps{suffix}"), None, "Failed to load saved recaps")
        }
        "account.deleteInboxRecap" => {
            let id = payload.get("id").and_then(Value::as_str).unwrap_or_default();
            authed_json(reqwest::Method::DELETE, &format!("/api/v1/inbox/recaps/{id}"), None, "Failed to delete the saved recap")
        }
        "account.updateProfile" => update_profile(payload),
        "account.uploadAvatar" => upload_avatar(payload),
        "account.deleteAvatar" => authed_json(reqwest::Method::DELETE, "/api/v1/profile/avatar", None, "Failed to remove your avatar"),
        "account.delete" => {
            let result = authed_json(reqwest::Method::DELETE, "/api/v1/me", Some(payload.clone()), "Could not confirm account deletion")?;
            let path = credentials_path();
            let _ = std::fs::remove_file(path);
            Ok(result)
        }
        "account.billingStatus" => authed_json(reqwest::Method::GET, "/api/v1/billing/status", None, "Could not load the subscription"),
        "account.billingCheckout" => authed_json(reqwest::Method::POST, "/api/v1/billing/checkout", Some(payload.clone()), "Could not open the test checkout"),
        "account.billingPortal" => authed_json(reqwest::Method::POST, "/api/v1/billing/portal", None, "Could not open subscription settings"),
        _ => Err(err("unhandled native tag")),
    }
}
