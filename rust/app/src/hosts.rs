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
        "hosts.revokeDevice" => {
            let device_id = payload.get("deviceId").and_then(Value::as_str).unwrap_or_default();
            let result = authed(reqwest::Method::DELETE, &format!("/api/v1/devices/{device_id}"), None, "Failed to revoke the device")?;
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
fn remote_access(payload: &Value) -> Result<Value, String> {
    use crate::remote_pairing as rp;
    let operation = payload.get("operation").and_then(Value::as_str).unwrap_or("");
    let state_dir = crate::todo::state_dir();
    let secrets_dir = format!("{state_dir}/secrets");
    let environment_id = crate::server_env::environment_id(&state_dir).unwrap_or_default();
    let conn = rp::open()?;
    let Some(scope) = rp::local_scope(&secrets_dir, &environment_id) else {
        return Err("Remote access operation failed".to_string());
    };
    match operation {
        "list" => Ok(json!({
            "kind": "host-state",
            "invitations": rp::list_invites(&conn, &scope)?,
            "activeSessions": crate::remote_sessions::list(),
            "devices": rp::list_devices(&conn, &scope)?,
            "rootFingerprint": scope.root_fingerprint,
            "allowConnections": rp::allows_connections(&conn, &scope)?,
        })),
        "create-invitation" => {
            let invite = rp::create_invite(&conn, &scope)?;
            Ok(json!({
                "kind": "invitation",
                "inviteId": invite.invite_id,
                "secret": invite.secret,
                "expiresAt": invite.expires_at,
            }))
        }
        "approve" => {
            let invite_id = payload.get("inviteId").and_then(Value::as_str).unwrap_or("");
            let device_jkt = payload.get("deviceJkt").and_then(Value::as_str).unwrap_or("");
            if !rp::approve(&conn, &scope, invite_id, device_jkt, "approval")? {
                return Err("The invitation or requested device changed. Review it again on this host.".to_string());
            }
            Ok(json!({"kind": "done"}))
        }
        "cancel-invitation" => {
            let invite_id = payload.get("inviteId").and_then(Value::as_str).unwrap_or("");
            rp::revoke(&conn, &scope, invite_id)?;
            Ok(json!({"kind": "done"}))
        }
        "set-allow-connections" => {
            let enabled = payload.get("enabled").and_then(Value::as_bool).unwrap_or(false);
            rp::set_allow_connections(&conn, &scope, enabled)?;
            Ok(json!({"kind": "done"}))
        }
        "revoke-device" => {
            let device_jkt = payload.get("deviceJkt").and_then(Value::as_str).unwrap_or("");
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
