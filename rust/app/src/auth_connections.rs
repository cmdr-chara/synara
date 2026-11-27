//! Authenticated websocket connection leases, port of the capacity half of
//! upstream SessionCredentialService. Local unauthenticated loopback sockets
//! have no auth-session row and are exempt; every explicit browser/client
//! session is limited independently so one saturated session cannot exhaust
//! another.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const MAX_AUTHENTICATED_CONNECTIONS_PER_SESSION: usize = 8;

static ACTIVE_CONNECTIONS: OnceLock<
    Mutex<HashMap<String, HashMap<String, tokio::sync::watch::Sender<()>>>>,
> = OnceLock::new();

fn active_connections() -> &'static Mutex<HashMap<String, HashMap<String, tokio::sync::watch::Sender<()>>>> {
    ACTIVE_CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug)]
pub(crate) struct ActiveConnectionLease {
    session_id: String,
    connection_id: String,
    close_rx: tokio::sync::watch::Receiver<()>,
}

impl ActiveConnectionLease {
    pub(crate) fn close_signal(&self) -> tokio::sync::watch::Receiver<()> {
        self.close_rx.clone()
    }
}

impl Drop for ActiveConnectionLease {
    fn drop(&mut self) {
        if let Ok(mut active) = active_connections().lock() {
            let empty = if let Some(connections) = active.get_mut(&self.session_id) {
                connections.remove(&self.connection_id);
                connections.is_empty()
            } else {
                false
            };
            if empty {
                active.remove(&self.session_id);
            }
        }
    }
}

pub(crate) fn register_authenticated_connection(
    session_id: &str,
) -> Result<ActiveConnectionLease, usize> {
    let mut active = active_connections().lock().map_err(|_| usize::MAX)?;
    let connections = active.entry(session_id.to_string()).or_default();
    let count = connections.len();
    if count >= MAX_AUTHENTICATED_CONNECTIONS_PER_SESSION {
        return Err(count);
    }
    let was_disconnected = count == 0;
    let connection_id = crate::orchestration_engine::uuid_v4();
    let (close_tx, close_rx) = tokio::sync::watch::channel(());
    connections.insert(connection_id.clone(), close_tx);
    drop(active);
    if was_disconnected {
        if let Ok(conn) = crate::orchestration_engine::open_state_rw() {
            let _ = conn.execute(
                "UPDATE auth_sessions SET last_connected_at = ?2 WHERE session_id = ?1",
                rusqlite::params![session_id, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)],
            );
        }
    }
    Ok(ActiveConnectionLease {
        session_id: session_id.to_string(),
        connection_id,
        close_rx,
    })
}

/// Interrupt every live authenticated socket for a revoked session. The
/// sender is process-local by design, mirroring upstream: revocation cannot
/// reach sockets owned by another server process, but every socket this
/// backend accepted is closed before the management RPC completes.
pub(crate) fn is_session_connected(session_id: &str) -> bool {
    active_connections()
        .lock()
        .map(|active| active.contains_key(session_id))
        .unwrap_or(false)
}

pub(crate) fn interrupt_sessions(session_ids: &[String]) {
    let Ok(active) = active_connections().lock() else { return };
    for session_id in session_ids {
        if let Some(connections) = active.get(session_id) {
            for close in connections.values() {
                let _ = close.send(());
            }
        }
    }
}

#[cfg(test)]
fn active_count(session_id: &str) -> usize {
    active_connections()
        .lock()
        .ok()
        .and_then(|active| active.get(session_id).map(std::collections::HashMap::len))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_drop_releases_capacity() {
        let session = format!("test-session-{}", crate::orchestration_engine::uuid_v4());
        let lease = register_authenticated_connection(&session).unwrap();
        assert_eq!(active_count(&session), 1);
        drop(lease);
        assert_eq!(active_count(&session), 0);
    }

    #[test]
    fn capacity_is_per_session() {
        let suffix = crate::orchestration_engine::uuid_v4();
        let saturated = format!("saturated-{suffix}");
        let independent = format!("independent-{suffix}");
        let leases: Vec<_> = (0..MAX_AUTHENTICATED_CONNECTIONS_PER_SESSION)
            .map(|_| register_authenticated_connection(&saturated).unwrap())
            .collect();
        assert_eq!(register_authenticated_connection(&saturated).unwrap_err(), MAX_AUTHENTICATED_CONNECTIONS_PER_SESSION);
        assert!(register_authenticated_connection(&independent).is_ok());
        drop(leases);
        assert!(register_authenticated_connection(&saturated).is_ok());
    }


    #[test]
    fn interrupt_sessions_signals_live_leases() {
        let session = format!("interrupt-{}", crate::orchestration_engine::uuid_v4());
        let lease = register_authenticated_connection(&session).unwrap();
        let mut signal = lease.close_signal();
        interrupt_sessions(&[session.clone()]);
        assert!(signal.has_changed().unwrap());
        drop(lease);
    }
}
