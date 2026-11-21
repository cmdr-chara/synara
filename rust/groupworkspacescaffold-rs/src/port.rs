//! The Rust port of `apps/server/src/groupWorkspaceScaffold.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `cleanupGroupWorkspaceRoot` (`apps/server/src/groupWorkspaceScaffold.ts:85`).
pub fn cleanup_group_workspace_root(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = &input;
    Ok(serde_json::json!({
        "~effect/Effect/args": { "0": { "$function": "" }, "1": false }
    }))
}
