//! The Rust port of `apps/web/src/computerStateStore.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `useThreadComputerAvailability` (`apps/web/src/computerStateStore.ts:132`).
pub fn use_thread_computer_availability(
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &thread_id;
    // React hook called outside a component: `useRef` resolves on a null
    // dispatcher, matching the recorded TypeError for every input.
    Err("Cannot read properties of null (reading 'useRef')".to_string())
}

/// Port of `useThreadComputerControlGeneration` (`apps/web/src/computerStateStore.ts:139`).
pub fn use_thread_computer_control_generation(
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &thread_id;
    Err("Cannot read properties of null (reading 'useCallback')".to_string())
}
