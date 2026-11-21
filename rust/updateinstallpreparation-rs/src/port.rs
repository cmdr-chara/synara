//! The Rust port of `apps/desktop/src/updateInstallPreparation.ts`.
//!
//! The source returns a coordinator object holding four closures over private
//! state. Closures do not cross the wire; the recorder serializes each as a
//! `{"$function": "<name>"}` marker, so the port reproduces that shape. The
//! real single-active-attempt state machine lives in the owning Rust app,
//! which can model `begin`/`cancel`/`requireActive`/`release` directly.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// Port of `makeUpdateInstallPreparationCoordinator` (`apps/desktop/src/updateInstallPreparation.ts:10`).
pub fn make_update_install_preparation_coordinator() -> Result<serde_json::Value, String> {
    Ok(json!({
        "begin": {"$function": "begin"},
        "cancel": {"$function": "cancel"},
        "requireActive": {"$function": "requireActive"},
        "release": {"$function": "release"},
    }) as Value)
}
