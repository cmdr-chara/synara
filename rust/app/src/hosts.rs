//! hosts.* — with no account backend the local lists that never need a
//! remote call answer empty, the rest answer the verbatim fallback
//! message upstream emits when its hosts service fails (wsHostsRpc).

use serde_json::{json, Value};

pub fn handles(tag: &str) -> bool {
    tag.starts_with("hosts.")
}

pub fn handle(tag: &str, _payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let err = |m: &str| crate::todo::WsErr(m.to_string(), None);
    match tag {
        "hosts.listSessions" => Ok(json!({"sessions": []})),
        "hosts.listConnections" => Ok(json!({"connections": []})),
        "hosts.endSession" => Ok(json!({})),
        "hosts.remoteAccess" => Err(err("Remote access operation failed")),
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
