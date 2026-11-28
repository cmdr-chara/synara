//! Application state shared across the HTTP/websocket routers.
//!
//! During the port this was the shadow-testing harness (`original.rs`): it
//! spawned the TypeScript server and relayed unported calls to it. The port is
//! complete and the TS tree is gone, so what remains is only the genuinely
//! shared state.

use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    /// Native-owned `todo.*` WS-RPC store, shared by every socket serving path.
    pub todos: Arc<crate::todo::TodoStore>,
}

impl AppState {
    pub fn open() -> Self {
        Self {
            todos: crate::todo::TodoStore::open(&crate::todo::state_dir()),
        }
    }
}

/// Ctrl-C or SIGTERM: stop taking requests and finish the open ones.
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("a SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
