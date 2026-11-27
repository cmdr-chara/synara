//! hosts.* — with no account backend the local lists that never need a
//! remote call answer empty, the rest answer the verbatim fallback
//! message upstream emits when its hosts service fails (wsHostsRpc).

use serde_json::{json, Value};

pub fn handles(tag: &str) -> bool {
    tag.starts_with("hosts.")
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let err = |m: &str| crate::todo::WsErr(m.to_string(), None);
    match tag {
        "hosts.listSessions" => Ok(json!({"sessions": []})),
        "hosts.listConnections" => Ok(json!({"connections": []})),
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
        "hosts.connect" => Err(err("Failed to connect to the host")),
        "hosts.disconnect" => Err(err("Failed to disconnect from the host")),
                "hosts.confirmSyncKey" => confirm_sync_key(payload).map_err(|error| err(&error.0)),
        _ => Err(err("unhandled native tag")),
    }
}

/// Local subset of upstream remotePairing/management.ts operations: invite
/// and device-trust management needs no account connectivity. Operations that
/// publish to or redeem through the account API still answer the fallback.
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
    let Some(scope) = scope else {
        return Err("Remote access operation failed".to_string());
    };
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
