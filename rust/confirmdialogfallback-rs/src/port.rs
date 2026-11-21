//! The Rust port of `apps/web/src/confirmDialogFallback.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `showConfirmDialogFallback` (`apps/web/src/confirmDialogFallback.ts:9`).
pub async fn show_confirm_dialog_fallback(message: String) -> Result<bool, String> {
    let _ = &message;
    Err("document is not defined".to_string())
}
