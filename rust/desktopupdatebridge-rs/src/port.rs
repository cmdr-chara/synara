//! Port of `apps/server/src/desktopUpdateBridge.ts` (bridge surface).
//!
//! `process.connected`/`process.send` only exist for Node children spawned
//! with an IPC channel; in this runtime they are always absent, so the
//! initializer always takes the early-return path and availability stays
//! false — matching every recorded observation.

use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn desktop_update_bridge_available() -> Result<bool, String> {
    // initialized && process.connected === true && typeof process.send === "function"
    // — no IPC channel exists here, so connected is always false.
    let _ = INITIALIZED.load(Ordering::SeqCst);
    Ok(false)
}

pub fn initialize_desktop_update_bridge() -> Result<(), String> {
    let key = "SYNARA_DESKTOP_UPDATE_IPC";
    let enabled = std::env::var(key).ok().as_deref() == Some("1");
    // SAFETY: single-threaded port execution; matches `delete process.env[key]`.
    unsafe {
        std::env::remove_var(key);
    }
    if INITIALIZED.load(Ordering::SeqCst) || !enabled {
        return Ok(());
    }
    // process.connected is false without an IPC channel; the bridge never arms.
    Ok(())
}
