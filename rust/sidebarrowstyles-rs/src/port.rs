//! The Rust port of `apps/web/src/sidebarRowStyles.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `sidebarHoverRevealHideClassName` (`apps/web/src/sidebarRowStyles.ts:126`).
///
/// `HOVER_REVEAL_HIDE_CLASS_NAME[group]` over the `SidebarHoverRevealGroup`
/// keys (`"picker-row"` excluded); any other value resolves to `undefined`.
pub fn sidebar_hover_reveal_hide_class_name(
    group: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let key = match &group {
        serde_json::Value::String(s) => s.as_str(),
        _ => return Ok(crate::js::undefined()),
    };
    Ok(match key {
        "activity-row" => serde_json::Value::String(
            "transition-opacity group-hover/activity-row:pointer-events-none group-hover/activity-row:opacity-0 group-focus-within/activity-row:pointer-events-none group-focus-within/activity-row:opacity-0".to_string(),
        ),
        "project-header" => serde_json::Value::String(
            "transition-opacity group-hover/project-header:pointer-events-none group-hover/project-header:opacity-0 project-header-focus:pointer-events-none project-header-focus:opacity-0".to_string(),
        ),
        "thread-row" => serde_json::Value::String(
            "transition-opacity group-hover/thread-row:pointer-events-none group-hover/thread-row:opacity-0 group-focus-within/thread-row:pointer-events-none group-focus-within/thread-row:opacity-0".to_string(),
        ),
        _ => crate::js::undefined(),
    })
}
