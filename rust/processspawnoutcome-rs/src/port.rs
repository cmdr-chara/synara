//! The Rust port of `packages/shared/src/processSpawnOutcome.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `trackProcessSpawn` (`packages/shared/src/processSpawnOutcome.ts:15`).
pub fn track_process_spawn(child: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = &child;
    unimplemented!("port `trackProcessSpawn` from packages/shared/src/processSpawnOutcome.ts")
}

/// Port of `didProcessFailToSpawn` (`packages/shared/src/processSpawnOutcome.ts:28`).
pub fn did_process_fail_to_spawn(child: serde_json::Value) -> Result<bool, String> {
    let _ = &child;
    unimplemented!("port `didProcessFailToSpawn` from packages/shared/src/processSpawnOutcome.ts")
}
