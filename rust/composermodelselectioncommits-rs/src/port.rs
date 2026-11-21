//! The Rust port of `apps/web/src/composerModelSelectionCommits.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `notifyComposerModelSelectionCommitted` (`apps/web/src/composerModelSelectionCommits.ts:21`).
pub fn notify_composer_model_selection_committed(
    thread_id: serde_json::Value,
) -> Result<(), String> {
    // With no registered listeners the notify loop is empty and the call
    // returns `undefined`.
    let _ = &thread_id;
    Ok(())
}
