//! Explicit command-backed desktop notification transport. The OS remains the
//! authority on delivery and permissions. No notification payload is logged.
use crate::{NotificationRequest, RuntimeError};
use std::path::Path;
use tokio_util::sync::CancellationToken;

pub fn desktop_notification_status() -> &'static str {
    if cfg!(target_os = "linux")
        && Path::new("/usr/bin/notify-send").is_file()
        && std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
    {
        "Desktop notification transport available. Delivery and permission must be tested."
    } else if cfg!(target_os = "macos") && Path::new("/usr/bin/osascript").is_file() {
        "macOS notification transport available. OS permission and delivery are not yet verified."
    } else {
        "Desktop notifications unavailable. Linux needs notify-send and a desktop D-Bus session. The Windows adapter is not implemented."
    }
}
pub fn desktop_notifications_available() -> bool {
    (cfg!(target_os = "linux")
        && Path::new("/usr/bin/notify-send").is_file()
        && std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some())
        || (cfg!(target_os = "macos") && Path::new("/usr/bin/osascript").is_file())
}
pub async fn desktop_notify(request: NotificationRequest) -> Result<(), RuntimeError> {
    request.validate()?;
    if !desktop_notifications_available() {
        return Err(RuntimeError::Unsupported(
            desktop_notification_status().into(),
        ));
    }
    let (command, args) = if cfg!(target_os = "macos") {
        // Only the fixed program is interpreted. User data is passed as argv,
        // never concatenated into AppleScript or a shell command.
        ("/usr/bin/osascript", vec!["-e".into(), "on run argv\ndisplay notification (item 2 of argv) with title (item 1 of argv)\nend run".into(), "--".into(), request.title, request.body])
    } else {
        (
            "/usr/bin/notify-send",
            vec![
                "--app-name=Synara".into(),
                "--".into(),
                request.title,
                request.body,
            ],
        )
    };
    crate::device_tools::command::run(Path::new(command), args, 4096, &CancellationToken::new())
        .await?;
    Ok(())
}
