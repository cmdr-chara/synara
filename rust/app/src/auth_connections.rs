//! Authenticated websocket connection leases, port of the capacity half of
//! upstream SessionCredentialService. Local unauthenticated loopback sockets
//! have no auth-session row and are exempt; every explicit browser/client
//! session is limited independently so one saturated session cannot exhaust
//! another.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

pub const MAX_AUTHENTICATED_CONNECTIONS_PER_SESSION: usize = 8;

static ACTIVE_CONNECTIONS: OnceLock<Mutex<HashMap<String, HashSet<String>>>> = OnceLock::new();

fn active_connections() -> &'static Mutex<HashMap<String, HashSet<String>>> {
    ACTIVE_CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug)]
pub(crate) struct ActiveConnectionLease {
    session_id: String,
    connection_id: String,
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
    let connection_id = crate::orchestration_engine::uuid_v4();
    connections.insert(connection_id.clone());
    Ok(ActiveConnectionLease {
        session_id: session_id.to_string(),
        connection_id,
    })
}

#[cfg(test)]
fn active_count(session_id: &str) -> usize {
    active_connections()
        .lock()
        .ok()
        .and_then(|active| active.get(session_id).map(HashSet::len))
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
}
