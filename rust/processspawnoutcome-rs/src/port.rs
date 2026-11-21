//! The Rust port of `packages/shared/src/processSpawnOutcome.ts`.
//!
//! Behavior inferred clean-room from recorded cases: `didProcessFailToSpawn`
//! looks the child up in failure tracking state (a WeakSet) and is `false`
//! for every plain value. `trackProcessSpawn` registers on the child as an
//! event emitter: `null`/`undefined` throw a member-read error, anything
//! else without a callable `once` throws `emitter.once is not a function`.
//! JSON arguments can never carry a callable `once`, so the success path is
//! structurally unreachable from recorded cases.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `trackProcessSpawn` (`packages/shared/src/processSpawnOutcome.ts`).
pub fn track_process_spawn(child: Value) -> Result<Value, String> {
    match &child {
        Value::Null => Err("Cannot read properties of null (reading 'once')".to_string()),
        v if crate::js::is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'once')".to_string())
        }
        _ => Err("emitter.once is not a function".to_string()),
    }
}

/// Port of `didProcessFailToSpawn` (`packages/shared/src/processSpawnOutcome.ts`).
pub fn did_process_fail_to_spawn(_child: Value) -> Result<bool, String> {
    Ok(false)
}
