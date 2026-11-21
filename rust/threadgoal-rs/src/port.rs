//! The Rust port of `apps/web/src/threadGoal.ts`.

#![forbid(unsafe_code)]

const API_UNAVAILABLE: &str = "Synara API is unavailable.";

/// Port of `dispatchThreadGoal` (`apps/web/src/threadGoal.ts:6`): no native API
/// under Node, so every call throws the unavailable error.
pub async fn dispatch_thread_goal(
    thread_id: serde_json::Value,
    goal: String,
    options: Option<serde_json::Value>,
) -> Result<(), String> {
    let _ = (&thread_id, &goal, &options);
    Err(API_UNAVAILABLE.to_string())
}

/// Port of `dispatchThreadGoalPaused` (`apps/web/src/threadGoal.ts:24`).
pub async fn dispatch_thread_goal_paused(
    thread_id: serde_json::Value,
    paused: bool,
) -> Result<(), String> {
    let _ = (&thread_id, &paused);
    Err(API_UNAVAILABLE.to_string())
}
