//! hosts.* — with no account backend the local lists that never need a
//! remote call answer empty, the rest answer the verbatim fallback
//! message upstream emits when its hosts service fails (wsHostsRpc).

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub fn handles(tag: &str) -> bool {
    tag.starts_with("hosts.")
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let err = |m: &str| crate::todo::WsErr(m.to_string(), None);
    match tag {
        "hosts.listSessions" => Ok(json!({"sessions": []})),
        "hosts.listConnections" => Ok(json!({
            "connections": crate::remote_connections::list(),
            "pairedHosts": paired_host_rows(),
            "desiredHosts": desired_host_rows(),
        })),
        "hosts.endSession" => Ok(json!({})),
        "hosts.remoteAccess" => remote_access(payload).map_err(|m| err(&m)),
        "hosts.list" => authed(reqwest::Method::GET, "/api/v1/hosts", None, "Failed to list hosts"),
        "hosts.update" => {
            let host_id = payload.get("hostId").and_then(Value::as_str).unwrap_or_default();
            let mut body = payload.clone();
            if let Some(object) = body.as_object_mut() { object.remove("hostId"); }
            let response = authed(reqwest::Method::PATCH, &format!("/api/v1/hosts/{host_id}"), Some(body), "Failed to update the host")?;
            if payload.get("discoverable").is_some()
                && let Some(mut stored) = crate::account::read_session_file()
                && let Some(object) = stored.as_object_mut()
            {
                let mut acknowledged = object.remove("discoverabilityAcknowledgedByHostId").unwrap_or(json!({}));
                if let Some(map) = acknowledged.as_object_mut() {
                    map.insert(host_id.to_string(), json!(true));
                }
                object.insert("discoverabilityAcknowledgedByHostId".into(), acknowledged);
                let _ = crate::account::write_account_file(&stored);
            }
            Ok(response.get("host").cloned().unwrap_or(Value::Null))
        }
        "hosts.delete" => {
            let host_id = payload.get("hostId").and_then(Value::as_str).unwrap_or_default();
            authed(reqwest::Method::DELETE, &format!("/api/v1/hosts/{host_id}"), None, "Failed to delete the host")
        }
        "hosts.listDevices" => authed(reqwest::Method::GET, "/api/v1/devices", None, "Failed to list devices"),
        "hosts.revokeDevice" => revoke_device(payload).map_err(|error| err(&error.0)),
        "hosts.approveDeviceLink" => authed(reqwest::Method::POST, "/api/v1/hosts/link/approve", Some(payload.clone()), "Failed to approve the device link"),
                "hosts.requestGrant" => {
            let host_id = payload.get("hostId").and_then(Value::as_str).unwrap_or_default();
            let user_id = crate::account::read_session_file()
                .and_then(|stored| stored.get("userId").and_then(Value::as_str).map(str::to_string))
                .ok_or_else(|| err("Failed to request a host grant"))?;
            crate::account::register_shell_device(&user_id)?;
            let device_jkt = crate::account::read_session_file()
                .and_then(|stored| stored.get("deviceJkt").and_then(Value::as_str).map(str::to_string))
                .ok_or_else(|| err("Failed to request a host grant"))?;
            authed(
                reqwest::Method::POST,
                &format!("/api/v1/hosts/{host_id}/grant"),
                Some(json!({"deviceJkt": device_jkt})),
                "Failed to request a host grant",
            )
        }
        "hosts.enrollment" => local_enrollment().map_err(|m| err(&m)),
        "hosts.unlinkLocalHost" => unlink_local_host(payload).map_err(|m| err(&m)),
        "hosts.beginSyncKeyPairing" => begin_sync_key_pairing().map_err(|error| err(&error.0)),
        "hosts.offerSyncKey" => offer_sync_key(payload).map_err(|error| err(&error.0)),
        "hosts.receiveSyncKey" => receive_sync_key().map_err(|error| err(&error.0)),
        "hosts.connect" => connect_host(payload).map_err(|m| err(&m)),
        "hosts.disconnect" => {
            let host_id = payload.get("hostId").and_then(Value::as_str).unwrap_or_default();
            if let Some(account) = crate::account::read_session_file()
                && let Some(environment_id) = crate::server_env::environment_id(&crate::todo::state_dir())
                && let Ok(binding) = controller_binding(&account, &environment_id)
                && let Ok(conn) = crate::remote_host_trust::open()
            {
                let _ = crate::remote_host_trust::set_desired(&conn, &binding, host_id, false);
            }
            crate::remote_connections::disconnect_blocking(host_id);
            Ok(json!({}))
        },
                "hosts.confirmSyncKey" => confirm_sync_key(payload).map_err(|error| err(&error.0)),
        _ => Err(err("unhandled native tag")),
    }
}

/// Owner-side remote pairing management, mirroring upstream
/// remotePairing/management.ts. Pairing previews are deliberately in-memory:
/// a restart requires redeeming a fresh public code.
fn controller_binding(account: &Value, environment_id: &str) -> Result<crate::remote_host_trust::Binding, String> {
    let account_url = account.get("accountUrl").and_then(Value::as_str).unwrap_or_default();
    let user_id = account.get("userId").and_then(Value::as_str).unwrap_or_default();
    let organization_id = account.get("organizationId").and_then(Value::as_str).unwrap_or_default();
    if account_url.is_empty() || user_id.is_empty() || organization_id.is_empty() {
        return Err("Remote access operation failed".to_string());
    }
    Ok(crate::remote_host_trust::Binding {
        controller_environment_id: environment_id.to_string(),
        account_authority: crate::account::account_api_issuer(account_url),
        user_id: user_id.to_string(),
        organization_id: organization_id.to_string(),
    })
}

fn pairing_previews() -> &'static Mutex<HashMap<String, Value>> {
    static PREVIEWS: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
    PREVIEWS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn prune_pairing_previews() {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    if let Ok(mut previews) = pairing_previews().lock() {
        previews.retain(|_, preview| {
            preview.pointer("/bundle/expiresAt")
                .and_then(Value::as_str)
                .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
                .is_some_and(|expires| expires.timestamp_millis() > now)
        });
    }
}

fn local_host_proof(account: &Value, environment_id: &str) -> Result<String, String> {
    let host_id = account.get("hostId").and_then(Value::as_str).unwrap_or_default();
    let key_generation = account.get("hostKeyGeneration").and_then(Value::as_u64).unwrap_or(0);
    if host_id.is_empty() || key_generation == 0 {
        return Err("Link this host to your own account first".to_string());
    }
    let secrets_dir = format!("{}/secrets", crate::todo::state_dir());
    let identity_path = crate::host_identity::host_identity_path(&secrets_dir);
    let identity = crate::host_identity::read_host_identity(&identity_path)
        .map_err(|_| "Remote access operation failed".to_string())?
        .ok_or_else(|| "Link this host to your own account first".to_string())?;
    let account_url = account.get("accountUrl").and_then(Value::as_str).unwrap_or_default();
    crate::host_identity::mint_host_proof(
        &identity,
        &crate::account::account_api_issuer(account_url),
        environment_id,
        host_id,
        key_generation,
    )
}

fn remote_endpoints(host: &Value) -> Vec<crate::remote_client::RemoteEndpoint> {
    host.get("endpoints")
        .and_then(Value::as_array)
        .map(|endpoints| endpoints.iter().filter_map(|endpoint| {
            let transport = endpoint.get("transport").and_then(Value::as_str)?.to_string();
            let url = endpoint.get("url").and_then(Value::as_str)?.to_string();
            if transport.is_empty() || url.is_empty() { None } else { Some(crate::remote_client::RemoteEndpoint { transport, url }) }
        }).collect())
        .unwrap_or_default()
}

fn preview_matches_binding(preview: &Value, account: &Value) -> bool {
    let account_url = account.get("accountUrl").and_then(Value::as_str).unwrap_or_default();
    let authority = crate::account::account_api_issuer(account_url);
    preview.pointer("/binding/accountAuthority").and_then(Value::as_str) == Some(authority.as_str())
        && preview.pointer("/binding/userId").and_then(Value::as_str) == account.get("userId").and_then(Value::as_str)
        && preview.pointer("/binding/organizationId").and_then(Value::as_str) == account.get("organizationId").and_then(Value::as_str)
}

pub(crate) fn request_host_grant(host_id: &str, user_id: &str) -> Result<String, String> {
    crate::account::register_shell_device(user_id).map_err(|error| error.0)?;
    let device_jkt = crate::account::read_session_file()
        .and_then(|stored| stored.get("deviceJkt").and_then(Value::as_str).map(str::to_string))
        .ok_or_else(|| "Failed to request a host grant".to_string())?;
    let response = authed(
        reqwest::Method::POST,
        &format!("/api/v1/hosts/{host_id}/grant"),
        Some(json!({"deviceJkt": device_jkt})),
        "Failed to request a host grant",
    ).map_err(|error| error.0)?;
    response.get("grant").and_then(Value::as_str).map(str::to_string)
        .ok_or_else(|| "Failed to request a host grant".to_string())
}

fn run_remote_pairing(
    endpoints: Vec<crate::remote_client::RemoteEndpoint>,
    bundle: Value,
    secrets_dir: String,
    label: String,
    host_id: String,
    user_id: String,
) -> Result<crate::remote_client::PairingOutcome, String> {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("Remote pairing runtime unavailable: {error}"))?;
        runtime.block_on(async move {
            let identity = crate::device_identity::load_or_create(&crate::device_identity::device_identity_path(&secrets_dir))
                .map_err(|_| "Remote access operation failed".to_string())?;
            let grant_host_id = host_id.clone();
            let grant_user_id = user_id.clone();
            crate::remote_client::pair_remote_host(
                endpoints,
                &bundle,
                &identity,
                &label,
                move || request_host_grant(&grant_host_id, &grant_user_id),
            ).await
        })
    })
    .join()
    .map_err(|_| "Remote pairing worker failed".to_string())?
}

fn connect_host(payload: &Value) -> Result<Value, String> {
    let host_id = payload.get("hostId").and_then(Value::as_str).unwrap_or_default();
    if host_id.is_empty() {
        return Err("Failed to connect to the host".to_string());
    }
    let account = crate::account::read_session_file().ok_or_else(|| "Connect this installation to your account first".to_string())?;
    let state_dir = crate::todo::state_dir();
    let controller_environment = crate::server_env::environment_id(&state_dir).unwrap_or_default();
    let binding = controller_binding(&account, &controller_environment)?;
    let listed = authed(reqwest::Method::GET, "/api/v1/hosts", None, "Failed to list hosts").map_err(|error| error.0)?;
    let host = listed.get("hosts").and_then(Value::as_array).and_then(|hosts| {
        hosts.iter().find(|candidate| candidate.get("id").and_then(Value::as_str) == Some(host_id)).cloned()
    }).ok_or_else(|| "That host is not on your account".to_string())?;
    if host.get("linked").and_then(Value::as_bool) != Some(true) {
        return Err("That host has not completed its key exchange yet".to_string());
    }
    let target_environment = host.get("environmentId").and_then(Value::as_str).unwrap_or_default().to_string();
    if target_environment.is_empty() {
        return Err("Remote host identity changed".to_string());
    }
    let trust_conn = crate::remote_host_trust::open()?;
    let trusted = crate::remote_host_trust::get(&trust_conn, &binding, &target_environment)?
        .filter(|trusted| trusted.paired_at.is_some())
        .ok_or_else(|| "Pair this device on the host before connecting.".to_string())?;
    if trusted.host_id != host_id {
        return Err("Remote host identity changed".to_string());
    }
    let endpoints = remote_endpoints(&host);
    if endpoints.is_empty() {
        return Err("The host published no reachable remote endpoints".to_string());
    }
    crate::remote_host_trust::set_desired(&trust_conn, &binding, host_id, true)?;
    let execution_scope = json!({
        "environmentId": trusted.environment_id,
        "accountAuthority": binding.account_authority,
        "userId": binding.user_id,
        "organizationId": binding.organization_id,
        "channel": trusted.channel,
    });
    crate::remote_connections::set_desired(crate::remote_connections::DesiredHost {
        host_id: host_id.to_string(),
        host_name: host.get("name").and_then(Value::as_str).unwrap_or(&trusted.label).to_string(),
        environment_id: target_environment,
        endpoints,
        root_certificate: trusted.root_certificate,
        execution_scope,
        state: "connecting",
        transport: None,
        started_at: None,
        credential_expires_at: None,
    });
    if let Err(error) = crate::remote_connections::probe_blocking(host_id, Value::Null) {
        crate::remote_connections::mark_state(host_id, "reconnecting");
        return Err(error);
    }
    Ok(crate::remote_connections::list()
        .into_iter()
        .find(|connection| connection.get("hostId").and_then(Value::as_str) == Some(host_id))
        .ok_or_else(|| "Remote connection closed during startup".to_string())?)
}

fn desired_host_rows() -> Vec<Value> {
    let Some(account) = crate::account::read_session_file() else { return Vec::new() };
    let state_dir = crate::todo::state_dir();
    let Some(environment_id) = crate::server_env::environment_id(&state_dir) else { return Vec::new() };
    let Ok(binding) = controller_binding(&account, &environment_id) else { return Vec::new() };
    let Ok(conn) = crate::remote_host_trust::open() else { return Vec::new() };
    crate::remote_host_trust::list_desired(&conn, &binding)
        .unwrap_or_default()
        .into_iter()
        .map(|host| json!({
            "hostId": host.host_id,
            "environmentId": host.environment_id,
            "label": host.label,
            "state": crate::remote_connections::state(&host.host_id),
        }))
        .collect()
}

fn paired_host_rows() -> Vec<Value> {
    let Some(account) = crate::account::read_session_file() else { return Vec::new() };
    let state_dir = crate::todo::state_dir();
    let Some(environment_id) = crate::server_env::environment_id(&state_dir) else { return Vec::new() };
    let Ok(binding) = controller_binding(&account, &environment_id) else { return Vec::new() };
    let Ok(conn) = crate::remote_host_trust::open() else { return Vec::new() };
    crate::remote_host_trust::list_paired(&conn, &binding)
        .unwrap_or_default()
        .into_iter()
        .map(|host| json!({"hostId": host.host_id, "environmentId": host.environment_id}))
        .collect()
}

fn cancel_published_pairing_code(invite_id: &str) {
    let Some(account) = crate::account::read_session_file() else { return };
    let state_dir = crate::todo::state_dir();
    let Some(environment_id) = crate::server_env::environment_id(&state_dir) else { return };
    let Ok(proof) = local_host_proof(&account, &environment_id) else { return };
    let account_url = account.get("accountUrl").and_then(Value::as_str).unwrap_or_default();
    let host_id = account.get("hostId").and_then(Value::as_str).unwrap_or_default();
    let _ = crate::account::AccountClient::new(account_url).request_host_proof(
        reqwest::Method::DELETE,
        &format!("/api/v1/hosts/{host_id}/pairing-codes/{invite_id}"),
        &proof,
        None,
    );
}

fn remote_access(payload: &Value) -> Result<Value, String> {
    use crate::remote_pairing as rp;
    // The RPC envelope is `{request: ...}`; accepting the bare request keeps
    // internal/test callers compatible without weakening the wire shape.
    let request = payload.get("request").unwrap_or(payload);
    let operation = request.get("operation").and_then(Value::as_str).unwrap_or("");
    let state_dir = crate::todo::state_dir();
    let secrets_dir = format!("{state_dir}/secrets");
    let environment_id = crate::server_env::environment_id(&state_dir).unwrap_or_default();
    let conn = rp::open()?;
    let tls_path = crate::remote_tls::remote_tls_identity_path(&secrets_dir);
    let mut tls_identity = crate::remote_tls::load_remote_tls_identity(&tls_path, &environment_id).ok();
    let mut scope = rp::local_scope(&secrets_dir, &environment_id);
    if matches!(operation, "create-invitation" | "create-code") && scope.is_none() && !tls_path.exists() {
        if rp::has_identity(&conn, &environment_id)? {
            return Err("The host root is missing. Restore it or explicitly reset remote trust before pairing again.".to_string());
        }
        tls_identity = crate::remote_tls::initialize_remote_tls_identity(&tls_path, &environment_id).ok();
        scope = rp::local_scope(&secrets_dir, &environment_id);
    }
    if operation == "list" && tls_identity.is_none() {
        return Ok(json!({
            "kind": "host-state",
            "invitations": [],
            "devices": [],
            "rootExpiresAt": Value::Null,
            "rootNeedsRepair": rp::has_identity(&conn, &environment_id)?,
        }));
    }
    // Controller operations (redeem/forget/device-info) must not depend on
    // this installation already being a host with a TLS root. Only the
    // host-side invitation/device operations require a local trust scope.
    let requires_scope = matches!(
        operation,
        "list" | "create-invitation" | "create-code" | "approve" | "cancel-invitation" | "set-allow-connections" | "revoke-device"
    );
    if requires_scope && scope.is_none() {
        return Err("Remote access operation failed".to_string());
    }
    let scope = scope.unwrap_or(crate::remote_pairing::TrustScope {
        environment_id: environment_id.clone(),
        root_fingerprint: String::new(),
        account_authority: String::new(),
        user_id: String::new(),
        organization_id: String::new(),
    });
    match operation {
        "list" => {
            let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
            let root_expires = tls_identity.as_ref().and_then(|identity| crate::remote_tls::root_validity(identity).ok()).map(|(_, end)| end);
            let root_expires = root_expires.and_then(|ms| chrono::DateTime::from_timestamp_millis(ms)).map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            Ok(json!({
                "kind": "host-state",
                "invitations": rp::list_invites(&conn, &scope)?,
                "devices": rp::list_devices(&conn, &scope)?,
                "rootFingerprint": scope.root_fingerprint,
                "rootExpiresAt": root_expires,
                "rootNeedsRepair": tls_identity.as_ref().is_some_and(|identity| crate::remote_tls::root_needs_repair(identity, now_ms)),
                "allowConnections": rp::allows_connections(&conn, &scope)?,
            }))
        }
        "pair" | "confirm-code" => {
            prune_pairing_previews();
            let account = crate::account::read_session_file().ok_or_else(|| "Remote access operation failed".to_string())?;
            let binding = controller_binding(&account, &environment_id)?;
            let identity = crate::device_identity::load_or_create(&crate::device_identity::device_identity_path(&secrets_dir))
                .map_err(|_| "Remote access operation failed".to_string())?;
            let bundle = if operation == "pair" {
                request.get("bundle").cloned().unwrap_or(Value::Null)
            } else {
                let invite_id = request.get("inviteId").and_then(Value::as_str).unwrap_or_default();
                let preview = pairing_previews().lock().ok().and_then(|previews| previews.get(invite_id).cloned())
                    .ok_or_else(|| "Review a fresh pairing code and compare the host fingerprint".to_string())?;
                if !preview_matches_binding(&preview, &account)
                    || preview.get("deviceJkt").and_then(Value::as_str) != Some(identity.jkt.as_str())
                    || request.get("rootFingerprint").and_then(Value::as_str) != preview.pointer("/bundle/rootFingerprint").and_then(Value::as_str)
                {
                    return Err("Review a fresh pairing code and compare the host fingerprint".to_string());
                }
                preview.get("bundle").cloned().unwrap_or(Value::Null)
            };
            if bundle.get("v").and_then(Value::as_u64) != Some(2)
                || bundle.get("accountAuthority").and_then(Value::as_str) != Some(binding.account_authority.as_str())
                || bundle.get("userId").and_then(Value::as_str) != Some(binding.user_id.as_str())
                || bundle.get("organizationId").and_then(Value::as_str) != Some(binding.organization_id.as_str())
            {
                return Err("Pairing belongs to another account".to_string());
            }
            let host_id = bundle.get("hostId").and_then(Value::as_str).unwrap_or_default().to_string();
            let target_environment = bundle.get("environmentId").and_then(Value::as_str).unwrap_or_default().to_string();
            if host_id.is_empty() || target_environment.is_empty() {
                return Err("Review a fresh pairing code and compare the host fingerprint".to_string());
            }
            let trust_conn = crate::remote_host_trust::open()?;
            crate::remote_host_trust::import_invitation(&trust_conn, &binding, &bundle)?;
            let listed = authed(reqwest::Method::GET, "/api/v1/hosts", None, "Failed to list hosts").map_err(|error| error.0)?;
            let host = listed.get("hosts").and_then(Value::as_array).and_then(|hosts| {
                hosts.iter().find(|candidate| {
                    candidate.get("id").and_then(Value::as_str) == Some(host_id.as_str())
                        && candidate.get("environmentId").and_then(Value::as_str) == Some(target_environment.as_str())
                }).cloned()
            }).ok_or_else(|| "The invitation host is not available in this account".to_string())?;
            let endpoints = remote_endpoints(&host);
            let outcome = run_remote_pairing(
                endpoints,
                bundle.clone(),
                secrets_dir.clone(),
                crate::account::display_hostname(),
                host_id.clone(),
                binding.user_id.clone(),
            )?;
            let current = crate::account::read_session_file().ok_or_else(|| "Account changed during pairing".to_string())?;
            let current_binding = controller_binding(&current, &environment_id)?;
            if current_binding.account_authority != binding.account_authority
                || current_binding.user_id != binding.user_id
                || current_binding.organization_id != binding.organization_id
            {
                return Err("Account changed during pairing".to_string());
            }
            if !crate::remote_host_trust::confirm(
                &trust_conn,
                &binding,
                &target_environment,
                bundle.get("rootFingerprint").and_then(Value::as_str).unwrap_or_default(),
                &crate::todo::iso_now_ms(),
            )? {
                return Err("Local trust changed during pairing".to_string());
            }
            if let Ok(mut previews) = pairing_previews().lock() {
                previews.remove(bundle.get("inviteId").and_then(Value::as_str).unwrap_or_default());
            }
            let _ = outcome;
            Ok(json!({"kind":"paired","environmentId":target_environment,"hostId":host_id}))
        }
        "forget-host" => {
            let target_environment = request.get("environmentId").and_then(Value::as_str).unwrap_or_default();
            if target_environment.is_empty() {
                return Err("Remote access operation failed".to_string());
            }
            let account = crate::account::read_session_file().ok_or_else(|| "Remote access operation failed".to_string())?;
            let binding = controller_binding(&account, &environment_id)?;
            let trust_conn = crate::remote_host_trust::open()?;
            crate::remote_host_trust::forget(&trust_conn, &binding, target_environment)?;
            if let Ok(mut previews) = pairing_previews().lock() {
                previews.retain(|_, preview| {
                    preview.pointer("/bundle/environmentId").and_then(Value::as_str) != Some(target_environment)
                });
            }
            Ok(json!({"kind":"done"}))
        }
        "redeem-code" => {
            prune_pairing_previews();
            if pairing_previews().lock().map(|previews| previews.len()).unwrap_or(0) >= 8 {
                return Err("Finish or cancel the pending pairing first".to_string());
            }
            let raw_code = request.get("code").and_then(Value::as_str).unwrap_or_default().to_uppercase();
            let filtered: String = raw_code.chars().filter(|c| c.is_ascii_uppercase() || ('2'..='9').contains(c)).collect();
            let code = if filtered.len() == 8 {
                format!("{}-{}", &filtered[..4], &filtered[4..])
            } else {
                filtered
            };
            let account = crate::account::read_session_file().ok_or_else(|| "Remote access operation failed".to_string())?;
            let account_url = account.get("accountUrl").and_then(Value::as_str).unwrap_or_default();
            let user_id = account.get("userId").and_then(Value::as_str).unwrap_or_default();
            let organization_id = account.get("organizationId").and_then(Value::as_str).unwrap_or_default();
            if account_url.is_empty() || user_id.is_empty() || organization_id.is_empty() {
                return Err("Remote access operation failed".to_string());
            }
            let authority = crate::account::account_api_issuer(account_url);
            let identity = crate::device_identity::load_or_create(&crate::device_identity::device_identity_path(&secrets_dir))
                .map_err(|_| "Remote access operation failed".to_string())?;
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let proof = identity.sign_jwt(
                json!({"typ":"synara-pairing-code+jwt"}),
                json!({
                    "code": code,
                    "iss": "synara-device",
                    "sub": user_id,
                    "aud": authority,
                    "iat": now,
                    "exp": now + 60,
                    "jti": crate::orchestration_engine::uuid_v4(),
                }),
            ).map_err(|_| "Remote access operation failed".to_string())?;
            let bundle = authed(
                reqwest::Method::POST,
                "/api/v1/remote/pairing-codes/redeem",
                Some(json!({"code": code, "deviceJkt": identity.jkt, "proof": proof})),
                "Remote access operation failed",
            ).map_err(|error| error.0)?;
            if bundle.get("accountAuthority").and_then(Value::as_str) != Some(authority.as_str())
                || bundle.get("userId").and_then(Value::as_str) != Some(user_id)
                || bundle.get("organizationId").and_then(Value::as_str) != Some(organization_id)
            {
                return Err("Pairing belongs to another account".to_string());
            }
            let invite_id = bundle.get("inviteId").and_then(Value::as_str).unwrap_or_default();
            if invite_id.is_empty() {
                return Err("Remote access operation failed".to_string());
            }
            if let Ok(mut previews) = pairing_previews().lock() {
                previews.insert(invite_id.to_string(), json!({
                    "bundle": bundle,
                    "deviceJkt": identity.jkt,
                    "binding": {"accountAuthority": authority, "userId": user_id, "organizationId": organization_id},
                }));
            }
            Ok(json!({
                "kind": "pairing-preview",
                "inviteId": invite_id,
                "environmentId": bundle.get("environmentId").cloned().unwrap_or(Value::Null),
                "label": bundle.get("label").cloned().unwrap_or(Value::Null),
                "rootFingerprint": bundle.get("rootFingerprint").cloned().unwrap_or(Value::Null),
                "expiresAt": bundle.get("expiresAt").cloned().unwrap_or(Value::Null),
                "deviceJkt": identity.jkt,
            }))
        }
        "device-info" => {
            let identity = crate::device_identity::load_or_create(&crate::device_identity::device_identity_path(&secrets_dir))
                .map_err(|_| "Remote access operation failed".to_string())?;
            Ok(json!({"kind":"device-info","deviceJkt":identity.jkt,"label":crate::account::display_hostname()}))
        }
        "reset-identity" => {
            if request.get("environmentId").and_then(Value::as_str) != Some(environment_id.as_str()) {
                return Err("The environment changed. Review the reset on this computer.".to_string());
            }
            rp::reset_environment(&conn, &environment_id, &crate::todo::iso_now_ms())?;
            crate::remote_tls::reset_remote_tls_identity(&tls_path, &environment_id)?;
            Ok(json!({"kind":"done"}))
        }
        "create-code" => {
            let Some(identity) = tls_identity.as_ref() else {
                return Err("The host root is missing. Restore it or explicitly reset remote trust before pairing again.".to_string());
            };
            let account = crate::account::read_session_file().ok_or_else(|| "Link this host to your own account first".to_string())?;
            let host_id = account.get("hostId").and_then(Value::as_str).unwrap_or_default();
            if host_id.is_empty() {
                return Err("Link this host to your own account first".to_string());
            }
            for old in rp::list_invites(&conn, &scope)? {
                let open = old.get("approved").and_then(Value::as_bool) == Some(false)
                    && old.get("revoked").and_then(Value::as_bool) == Some(false);
                if open {
                    if let Some(id) = old.get("inviteId").and_then(Value::as_str) {
                        let _ = rp::revoke(&conn, &scope, id);
                    }
                }
            }
            let invite = rp::create_invite(&conn, &scope)?;
            let channel = match std::env::var("SYNARA_DESKTOP_BUNDLE_ID").ok().as_deref().map(str::trim) {
                Some("com.emanueledipietro.synara.beta") => "beta",
                Some("com.emanueledipietro.synara.canary") => "canary",
                Some("com.emanueledipietro.synara") => "stable",
                _ => "dev",
            };
            let bundle = json!({
                "v": 2,
                "environmentId": scope.environment_id,
                "rootFingerprint": scope.root_fingerprint,
                "accountAuthority": scope.account_authority,
                "userId": scope.user_id,
                "organizationId": scope.organization_id,
                "channel": channel,
                "rootCertificate": identity.root_certificate,
                "hostId": host_id,
                "label": crate::account::display_hostname(),
                "inviteId": invite.invite_id,
                "secret": invite.secret,
                "expiresAt": invite.expires_at,
            });
            let published = (|| -> Result<Value, String> {
                let proof = local_host_proof(&account, &environment_id)?;
                let account_url = account.get("accountUrl").and_then(Value::as_str).unwrap_or_default();
                crate::account::AccountClient::new(account_url)
                    .request_host_proof(
                        reqwest::Method::POST,
                        &format!("/api/v1/hosts/{host_id}/pairing-codes"),
                        &proof,
                        Some(bundle),
                    )
                    .map_err(|error| crate::account::service_error(&error, "Remote access operation failed").0)
            })();
            let published = match published {
                Ok(value) => value,
                Err(error) => {
                    let _ = rp::revoke(&conn, &scope, &invite.invite_id);
                    return Err(error);
                }
            };
            let published_invite = published.get("inviteId").and_then(Value::as_str).unwrap_or_default();
            if published_invite != invite.invite_id {
                let _ = rp::revoke(&conn, &scope, &invite.invite_id);
                return Err("Remote access operation failed".to_string());
            }
            let result = json!({
                "kind": "pairing-code",
                "code": published.get("code").cloned().unwrap_or(Value::Null),
                "inviteId": published_invite,
                "expiresAt": published.get("expiresAt").cloned().unwrap_or(json!(invite.expires_at)),
                "rootFingerprint": scope.root_fingerprint,
            });
            if result.get("code").is_none_or(Value::is_null) {
                let _ = rp::revoke(&conn, &scope, &invite.invite_id);
                return Err("Remote access operation failed".to_string());
            }
            Ok(result)
        }
        "create-invitation" => {
            let Some(identity) = tls_identity.as_ref() else {
                return Err("The host root is missing. Restore it or explicitly reset remote trust before pairing again.".to_string());
            };
            let account = crate::account::read_session_file().ok_or_else(|| "Link this host to your own account first".to_string())?;
            let host_id = account.get("hostId").and_then(Value::as_str).unwrap_or_default();
            if host_id.is_empty() {
                return Err("Link this host to your own account first".to_string());
            }
            let channel = match std::env::var("SYNARA_DESKTOP_BUNDLE_ID").ok().as_deref().map(str::trim) {
                Some("com.emanueledipietro.synara.beta") => "beta",
                Some("com.emanueledipietro.synara.canary") => "canary",
                Some("com.emanueledipietro.synara") => "stable",
                _ => "dev",
            };
            let invite = rp::create_invite(&conn, &scope)?;
            let bundle = json!({
                "v": 2,
                "environmentId": scope.environment_id,
                "rootFingerprint": scope.root_fingerprint,
                "accountAuthority": scope.account_authority,
                "userId": scope.user_id,
                "organizationId": scope.organization_id,
                "channel": channel,
                "rootCertificate": identity.root_certificate,
                "hostId": host_id,
                "label": crate::account::display_hostname(),
                "inviteId": invite.invite_id,
                "secret": invite.secret,
                "expiresAt": invite.expires_at,
            });
            Ok(json!({"kind": "invitation", "bundle": bundle}))
        }
        "approve" => {
            let invite_id = request.get("inviteId").and_then(Value::as_str).unwrap_or("");
            let device_jkt = request.get("deviceJkt").and_then(Value::as_str).unwrap_or("");
            if !rp::approve(&conn, &scope, invite_id, device_jkt, "approval")? {
                return Err("The invitation or requested device changed. Review it again on this host.".to_string());
            }
            Ok(json!({"kind": "done"}))
        }
        "cancel-invitation" => {
            let invite_id = request.get("inviteId").and_then(Value::as_str).unwrap_or("");
            rp::revoke(&conn, &scope, invite_id)?;
            cancel_published_pairing_code(invite_id);
            Ok(json!({"kind": "done"}))
        }
        "set-allow-connections" => {
            let enabled = request.get("enabled").and_then(Value::as_bool).unwrap_or(false);
            rp::set_allow_connections(&conn, &scope, enabled)?;
            Ok(json!({"kind": "done"}))
        }
        "revoke-device" => {
            let device_jkt = request.get("deviceJkt").and_then(Value::as_str).unwrap_or("");
            rp::revoke_device(&conn, &scope, device_jkt)?;
            Ok(json!({"kind": "done"}))
        }
        _ => Err("Remote access operation failed".to_string()),
    }
}

fn authed(
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
    fallback: &str,
) -> Result<Value, crate::todo::WsErr> {
    crate::account::authenticated_request(fallback, |client, access| {
        client.request(method.clone(), path, Some(access), body.clone(), None, std::time::Duration::from_secs(15))
    })
}

fn local_enrollment() -> Result<Value, String> {
    let stored = crate::account::read_session_file().ok_or_else(|| "Failed to read local host enrollment".to_string())?;
    let listed = authed(reqwest::Method::GET, "/api/v1/hosts", None, "Failed to read local host enrollment")
        .map_err(|error| error.0)?;
    let member_count = authed(reqwest::Method::GET, "/api/v1/organization/member-count", None, "Failed to read local host enrollment")
        .ok()
        .and_then(|value| value.get("count").cloned())
        .unwrap_or(Value::Null);
    let host = stored
        .get("hostId")
        .and_then(Value::as_str)
        .and_then(|host_id| {
            listed.get("hosts").and_then(Value::as_array).and_then(|hosts| {
                hosts.iter().find(|host| host.get("id").and_then(Value::as_str) == Some(host_id)).cloned()
            })
        })
        .unwrap_or(Value::Null);
    let acknowledged = stored
        .get("hostId")
        .and_then(Value::as_str)
        .is_some_and(|host_id| stored.pointer(&format!("/discoverabilityAcknowledgedByHostId/{host_id}")).and_then(Value::as_bool) == Some(true));
    Ok(json!({
        "host": host,
        "organizationMemberCount": member_count,
        "discoverabilityAcknowledged": acknowledged,
    }))
}

fn unlink_local_host(payload: &Value) -> Result<Value, String> {
    let host_id = payload
        .get("hostId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| crate::account::read_session_file().and_then(|stored| stored.get("hostId").and_then(Value::as_str).map(str::to_string)))
        .ok_or_else(|| "Failed to unlink the local host".to_string())?;
    authed(reqwest::Method::DELETE, &format!("/api/v1/hosts/{host_id}"), None, "Failed to unlink the local host")
        .map_err(|error| error.0)?;
    if let Some(mut stored) = crate::account::read_session_file()
        && let Some(object) = stored.as_object_mut()
    {
        for key in ["hostId", "hostOwnerUserId", "hostKeyGeneration", "remoteBillingPolicy", "discoverabilityAcknowledgedByHostId"] {
            object.remove(key);
        }
        let _ = crate::account::write_account_file(&stored);
    }
    crate::host_identity::delete_host_identity(&crate::account::host_identity_file_path());
    Ok(json!({}))
}


fn account_identity() -> Result<(String, String, String), crate::todo::WsErr> {
    let stored = crate::account::read_session_file()
        .ok_or_else(|| crate::todo::WsErr("Session expired — sign in again from the Synara app.".into(), None))?;
    let account_url = stored.get("accountUrl").and_then(Value::as_str).unwrap_or_default().to_string();
    let user_id = stored.get("userId").and_then(Value::as_str).unwrap_or_default().to_string();
    let device_id = stored.get("deviceId").and_then(Value::as_str).unwrap_or_default().to_string();
    if account_url.is_empty() || user_id.is_empty() || device_id.is_empty() {
        return Err(crate::todo::WsErr("This device is not registered with the account".into(), None));
    }
    Ok((account_url, user_id, device_id))
}

fn begin_sync_key_pairing() -> Result<Value, crate::todo::WsErr> {
    let (account_url, user_id, device_id) = account_identity()?;
    crate::host_secrets::begin_pairing(&device_id, &account_url, &user_id)
}

fn offer_sync_key(payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let (account_url, user_id, _device_id) = account_identity()?;
    let recipient_device_id = payload.get("recipientDeviceId").and_then(Value::as_str).unwrap_or_default();
    let recipient_public_jwk = payload.get("recipientPublicJwk").cloned().unwrap_or(Value::Null);
    if recipient_device_id.is_empty() || recipient_public_jwk.is_null() {
        return Err(crate::todo::WsErr("Failed to offer the Sync Key".into(), None));
    }
    let key = match crate::host_secrets::read_sync_key(&account_url, &user_id)? {
        Some(key) => key,
        None => {
            let key = crate::host_secrets::generate_sync_key();
            crate::host_secrets::write_sync_key(&account_url, &user_id, &key)?;
            key
        }
    };
    let wrap = crate::host_secrets::wrap_sync_key(&key, &recipient_public_jwk)?;
    let code = crate::host_secrets::pairing_verification_for_wrap(&wrap)?;
    authed(
        reqwest::Method::PUT,
        "/api/v1/sync-key-wraps",
        Some(json!({"recipientDeviceId": recipient_device_id, "wrap": wrap})),
        "Failed to offer the Sync Key",
    )?;
    Ok(json!({"verificationCode": code}))
}

fn receive_sync_key() -> Result<Value, crate::todo::WsErr> {
    let (_account_url, _user_id, device_id) = account_identity()?;
    let response = authed(
        reqwest::Method::GET,
        &format!("/api/v1/sync-key-wraps/{device_id}"),
        None,
        "Failed to receive the Sync Key",
    )?;
    let wrap = response.get("wrap").cloned().unwrap_or(Value::Null);
    if wrap.is_null() {
        return Err(crate::todo::WsErr("Failed to receive the Sync Key".into(), None));
    }
    let code = crate::host_secrets::pairing_verification_for_wrap(&wrap)?;
    crate::host_secrets::remember_received_wrap(wrap, code.clone());
    Ok(json!({"verificationCode": code}))
}

fn confirm_sync_key(payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let (account_url, user_id, _device_id) = account_identity()?;
    let code = payload.get("verificationCode").and_then(Value::as_str).unwrap_or_default();
    crate::host_secrets::confirm_received_wrap(code, &account_url, &user_id)?;
    Ok(json!({}))
}


fn delete_device_remote(device_id: &str) -> Result<Value, crate::todo::WsErr> {
    let result = authed(
        reqwest::Method::DELETE,
        &format!("/api/v1/devices/{device_id}"),
        None,
        "Failed to revoke the device",
    )?;
    if let Some(mut stored) = crate::account::read_session_file()
        && stored.get("deviceId").and_then(Value::as_str) == Some(device_id)
        && let Some(object) = stored.as_object_mut()
    {
        object.remove("deviceId");
        object.remove("deviceJkt");
        let _ = crate::account::write_account_file(&stored);
    }
    Ok(result)
}

fn get_host_secret(host_id: &str) -> Result<Value, crate::todo::WsErr> {
    authed(
        reqwest::Method::GET,
        &format!("/api/v1/hosts/{host_id}/secrets"),
        None,
        "Failed to read the host secret",
    )
}

fn put_host_secret(host_id: &str, expected_version: i64, envelope: &Value) -> Result<(), crate::todo::WsErr> {
    authed(
        reqwest::Method::PUT,
        &format!("/api/v1/hosts/{host_id}/secrets"),
        Some(json!({"expectedVersion": expected_version, "envelope": envelope})),
        "Failed to rotate Host Secrets",
    )?;
    Ok(())
}

fn owned_host_secret_entries(user_id: &str) -> Result<Vec<Value>, crate::todo::WsErr> {
    let listed = authed(reqwest::Method::GET, "/api/v1/hosts", None, "Failed to rotate Host Secrets")?;
    let mut entries = Vec::new();
    for host in listed.get("hosts").and_then(Value::as_array).cloned().unwrap_or_default() {
        if host.get("mine").and_then(Value::as_bool) != Some(true)
            || host.get("ownerUserId").and_then(Value::as_str) != Some(user_id)
        {
            continue;
        }
        let host_id = host.get("id").and_then(Value::as_str).unwrap_or_default();
        if host_id.is_empty() { continue; }
        let response = get_host_secret(host_id)?;
        let Some(secret) = response.get("secret").filter(|secret| !secret.is_null()) else { continue };
        entries.push(json!({
            "hostId": host_id,
            "ownerUserId": host.get("ownerUserId").cloned().unwrap_or(json!(user_id)),
            "envelope": {
                "ciphertext": secret.get("ciphertext").cloned().unwrap_or(Value::Null),
                "iv": secret.get("iv").cloned().unwrap_or(Value::Null),
                "version": secret.get("version").cloned().unwrap_or(Value::Null),
            },
        }));
    }
    Ok(entries)
}

fn device_is_active(device_id: &str) -> Result<bool, crate::todo::WsErr> {
    let listed = authed(reqwest::Method::GET, "/api/v1/devices", None, "Failed to rotate Host Secrets")?;
    Ok(listed.get("devices").and_then(Value::as_array).is_some_and(|devices| {
        devices.iter().any(|device| {
            device.get("id").and_then(Value::as_str) == Some(device_id)
                && device.get("revokedAt").is_some_and(Value::is_null)
        })
    }))
}

fn persist_pending(
    account_url: &str,
    user_id: &str,
    old_key: &[u8; 32],
    pending: &Value,
) -> Result<(), crate::todo::WsErr> {
    crate::host_secrets::write_pending_rotation(account_url, user_id, old_key, pending)
}

fn continue_rotation(
    account_url: &str,
    user_id: &str,
    old_key: &[u8; 32],
    mut pending: Value,
) -> Result<(), crate::todo::WsErr> {
    if pending.get("revocationCompleted").and_then(Value::as_bool) != Some(true) {
        let device_id = pending.get("deviceId").and_then(Value::as_str).unwrap_or_default();
        if device_is_active(device_id)? {
            authed(
                reqwest::Method::DELETE,
                &format!("/api/v1/devices/{device_id}"),
                None,
                "Failed to revoke the device",
            )?;
        }
        pending["revocationCompleted"] = json!(true);
        persist_pending(account_url, user_id, old_key, &pending)?;
    }

    loop {
        let entries = pending.get("entries").and_then(Value::as_array).cloned().unwrap_or_default();
        let Some(entry) = entries.first().cloned() else { break };
        let remaining = json!(entries.iter().skip(1).cloned().collect::<Vec<_>>());
        let host_id = entry.get("hostId").and_then(Value::as_str).unwrap_or_default().to_string();
        let owner = entry.get("ownerUserId").and_then(Value::as_str).unwrap_or_default().to_string();
        let current = get_host_secret(&host_id)?.get("secret").cloned().unwrap_or(Value::Null);
        if current.is_null() {
            pending["entries"] = remaining;
            persist_pending(account_url, user_id, old_key, &pending)?;
            continue;
        }
        let envelope = entry.get("envelope").cloned().unwrap_or(Value::Null);
        let already_uploaded = current.get("version") == envelope.get("version")
            && current.get("ciphertext") == envelope.get("ciphertext")
            && current.get("iv") == envelope.get("iv");
        if !already_uploaded {
            let mut upload_entry = entry.clone();
            let mut expected_version = envelope.get("version").and_then(Value::as_i64).unwrap_or(1) - 1;
            let current_version = current.get("version").and_then(Value::as_i64).unwrap_or(0);
            if current_version != expected_version {
                let next_key_bytes = crate::host_secrets::read_pending_rotation(account_url, user_id)?
                    .and_then(|pending| pending.get("nextKey").and_then(Value::as_str).map(str::to_string))
                    .and_then(|encoded| base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, encoded).ok())
                    .filter(|bytes: &Vec<u8>| bytes.len() == 32)
                    .map(|bytes| {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&bytes);
                        key
                    })
                    .ok_or_else(|| crate::todo::WsErr("The Host Secrets rotation journal has no next key".into(), None))?;
                let secret = crate::host_secrets::open_host_secret(
                    old_key,
                    &host_id,
                    &owner,
                    current_version,
                    &current,
                )?;
                let envelope = crate::host_secrets::seal_host_secret(
                    &next_key_bytes,
                    &host_id,
                    &owner,
                    current_version + 1,
                    &secret,
                )?;
                upload_entry["envelope"] = envelope;
                expected_version = current_version;
                let mut rewritten_entries = vec![upload_entry.clone()];
                rewritten_entries.extend(entries.iter().skip(1).cloned());
                pending["entries"] = json!(rewritten_entries);
                persist_pending(account_url, user_id, old_key, &pending)?;
            }
            let upload_envelope = upload_entry.get("envelope").cloned().unwrap_or(Value::Null);
            put_host_secret(&host_id, expected_version, &upload_envelope)?;
        }
        pending["entries"] = remaining;
        persist_pending(account_url, user_id, old_key, &pending)?;
    }

    let next_key = pending
        .get("nextKey")
        .and_then(Value::as_str)
        .and_then(|encoded| base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, encoded).ok())
        .filter(|bytes: &Vec<u8>| bytes.len() == 32)
        .map(|bytes| {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            key
        })
        .ok_or_else(|| crate::todo::WsErr("The Host Secrets rotation journal has no next key".into(), None))?;
    crate::host_secrets::write_sync_key(account_url, user_id, &next_key)
}

fn revoke_device(payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let device_id = payload.get("deviceId").and_then(Value::as_str).unwrap_or_default();
    let (account_url, user_id, own_device_id) = account_identity()?;
    if device_id == own_device_id {
        return Err(crate::todo::WsErr("Revoke this device from another device that is paired so it can rotate".into(), None));
    }
    let mut old_key = crate::host_secrets::read_sync_key(&account_url, &user_id)?;
    if let Some(pending) = crate::host_secrets::read_pending_rotation(&account_url, &user_id)? {
        let Some(key) = old_key else {
            return Err(crate::todo::WsErr("The Host Secrets rotation journal has no current key".into(), None));
        };
        let pending_device = pending.get("deviceId").and_then(Value::as_str).unwrap_or_default().to_string();
        continue_rotation(&account_url, &user_id, &key, pending)?;
        if pending_device == device_id {
            return Ok(json!({}));
        }
        old_key = crate::host_secrets::read_sync_key(&account_url, &user_id)?;
    }

    let entries = owned_host_secret_entries(&user_id)?;
    if entries.is_empty() {
        return delete_device_remote(device_id);
    }
    let Some(old_key) = old_key else {
        return Err(crate::todo::WsErr("Pair this device before revoking another device with Host Secrets".into(), None));
    };
    if !device_is_active(device_id)? {
        return Err(crate::todo::WsErr("Device not found or already revoked".into(), None));
    }
    let new_key = crate::host_secrets::generate_sync_key();
    let rotated = crate::host_secrets::rotate_host_secret_entries(&old_key, &new_key, &entries)?;
    let pending = json!({
        "deviceId": device_id,
        "revocationCompleted": false,
        "nextKey": base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, new_key),
        "entries": rotated,
    });
    persist_pending(&account_url, &user_id, &old_key, &pending)?;
    continue_rotation(&account_url, &user_id, &old_key, pending)?;
    Ok(json!({}))
}
