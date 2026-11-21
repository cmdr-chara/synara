//! The Rust port of `apps/web/src/contextMenuFallback.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `showContextMenuFallback` (`apps/web/src/contextMenuFallback.ts:40`).
pub async fn show_context_menu_fallback(
    items: Vec<serde_json::Value>,
    position: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let _ = (&items, &position);
    // Imperative DOM menu — outside a browser `document.createElement` throws immediately.
    Err("document is not defined".into())
}
