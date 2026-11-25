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
        "hosts.list" => Err(err("Failed to list hosts")),
        "hosts.update" => Err(err("Failed to update the host")),
        "hosts.delete" => Err(err("Failed to delete the host")),
        "hosts.listDevices" => Err(err("Failed to list devices")),
        "hosts.revokeDevice" => Err(err("Failed to revoke the device")),
        "hosts.approveDeviceLink" => Err(err("Failed to approve the device link")),
        "hosts.requestGrant" => Err(err("Failed to request a host grant")),
        "hosts.enrollment" => Err(err("Failed to read local host enrollment")),
        "hosts.unlinkLocalHost" => Err(err("Failed to unlink the local host")),
        "hosts.beginSyncKeyPairing" => Err(err("Failed to start Sync-Key pairing")),
        "hosts.offerSyncKey" => Err(err("Failed to offer the Sync Key")),
        "hosts.receiveSyncKey" => Err(err("Failed to receive the Sync Key")),
        "hosts.connect" => Err(err("Failed to connect to the host")),
        "hosts.disconnect" => Err(err("Failed to disconnect from the host")),
        "hosts.confirmSyncKey" => Err(err("Failed to confirm Sync-Key pairing")),
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
