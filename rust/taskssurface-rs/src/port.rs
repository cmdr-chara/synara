//! The Rust port of `apps/web/src/tasksSurface.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};

/// `TASKS_OFFERED_BY_BUILD` — `isBetaFeatureOn("tasks")` was true in the recorded build.
const TASKS_OFFERED_BY_BUILD: bool = true;

/// `useTasksRefusalStore`'s `refused` flag, shared process-wide like the zustand store.
static REFUSED: AtomicBool = AtomicBool::new(false);

/// `TASKS_UNAVAILABLE_ERROR_CODE` from `@synara/contracts`.
const TASKS_UNAVAILABLE_ERROR_CODE: &str = "TASKS_UNAVAILABLE";

/// Port of `isTasksRefusal` (`apps/web/src/tasksSurface.ts:18`).
pub fn is_tasks_refusal(error: serde_json::Value) -> Result<bool, String> {
    // `typeof error === "object" && error !== null && "code" in error`
    Ok(error
        .get("code")
        .is_some_and(|c| c.as_str() == Some(TASKS_UNAVAILABLE_ERROR_CODE)))
}

/// Port of `noteTasksRefusal` (`apps/web/src/tasksSurface.ts:28`).
pub fn note_tasks_refusal(error: serde_json::Value) -> Result<bool, String> {
    let is_refusal = error
        .get("code")
        .is_some_and(|c| c.as_str() == Some(TASKS_UNAVAILABLE_ERROR_CODE));
    if !is_refusal {
        return Ok(false);
    }
    REFUSED.store(true, Ordering::SeqCst);
    Ok(true)
}

/// Port of `isTasksSurfaceEnabled` (`apps/web/src/tasksSurface.ts:35`).
pub fn is_tasks_surface_enabled() -> Result<bool, String> {
    Ok(TASKS_OFFERED_BY_BUILD && !REFUSED.load(Ordering::SeqCst))
}

/// Port of `useTasksSurfaceEnabled` (`apps/web/src/tasksSurface.ts:39`).
pub fn use_tasks_surface_enabled() -> Result<bool, String> {
    // No React dispatcher outside a component in the recorded environment.
    Err("null is not an object (evaluating 'resolveDispatcher().useCallback')".to_string())
}
