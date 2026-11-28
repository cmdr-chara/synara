//! Active remote session registry, the local subset of upstream
//! remoteSessions/sessionRegistry.ts. Upstream interleaves billing policy
//! enforcement and account-mediated admission guards; neither exists in the
//! local build, so this registry keeps exactly what the owner experience
//! needs: which devices hold live sessions, since when, and the ability to
//! drop them — individually or wholesale when trust is revoked.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const HOST_SESSION_CLOSE_REVOKED: u16 = 4403;

pub struct RemoteSession {
    pub id: String,
    pub user_id: String,
    pub device_jkt: String,
    pub started_at: String,
    pub via: String,
    close_signal: tokio::sync::mpsc::UnboundedSender<u16>,
}

fn registry() -> &'static Mutex<HashMap<String, RemoteSession>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, RemoteSession>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Registers a live session and returns the removal closure's receiver half:
/// the connection task watches it for close signals. Removal is explicit via
/// `remove` when the socket ends for any reason.
pub fn add(
    device_jkt: &str,
    user_id: &str,
    via: &str,
) -> (String, tokio::sync::mpsc::UnboundedReceiver<u16>) {
    let id = crate::orchestration_engine::uuid_v4();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let started_at = chrono::DateTime::from_timestamp_millis(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
    )
    .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    .unwrap_or_default();
    let session = RemoteSession {
        id: id.clone(),
        user_id: user_id.to_string(),
        device_jkt: device_jkt.to_string(),
        started_at,
        via: via.to_string(),
        close_signal: tx,
    };
    registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id.clone(), session);
    (id, rx)
}

pub fn remove(id: &str) {
    registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(id);
}

/// Closes every live session a device holds — the revoke path calls this so a
/// trust change reaches open sockets immediately instead of at credential
/// expiry, exactly upstream's closeForDevice.
pub fn close_for_device(device_jkt: &str) {
    let sessions = registry().lock().unwrap_or_else(|e| e.into_inner());
    for session in sessions.values() {
        if session.device_jkt == device_jkt {
            let _ = session.close_signal.send(HOST_SESSION_CLOSE_REVOKED);
        }
    }
}

/// Snapshot for the management surface: no close handles, just the facts.
pub fn list() -> Vec<serde_json::Value> {
    registry()
        .lock()
        .unwrap()
        .values()
        .map(|session| {
            serde_json::json!({
                "id": session.id,
                "userId": session.user_id,
                "deviceJkt": session.device_jkt,
                "startedAt": session.started_at,
                "via": session.via,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_remove_cycle() {
        let (id, _rx) = add("jkt-1", "user-1", "direct");
        let sessions = list();
        assert!(sessions.iter().any(|s| s["id"] == id));
        remove(&id);
        assert!(!list().iter().any(|s| s["id"] == id));
    }

    #[test]
    fn close_for_device_signals_only_matching() {
        let (_id_a, mut rx_a) = add("jkt-match", "", "direct");
        let (_id_b, mut rx_b) = add("jkt-other", "", "direct");
        close_for_device("jkt-match");
        assert_eq!(rx_a.try_recv().ok(), Some(HOST_SESSION_CLOSE_REVOKED));
        assert!(rx_b.try_recv().is_err());
    }
}
