//! The Rust port of `apps/desktop/src/notificationRetention.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// Port of `createDesktopNotificationRetainer` (`apps/desktop/src/notificationRetention.ts:21`).
///
/// The retainer object closes over a notification set; at the wire boundary it
/// is a function plus the current `size` (0 on creation).
pub fn create_desktop_notification_retainer(_limit: Option<f64>) -> Result<Value, String> {
    Ok(json!({
        "retain": {"$function": "retain"},
        "size": 0,
    }))
}
