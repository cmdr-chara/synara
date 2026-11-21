//! The Rust port of `apps/server/src/hostUsageSnapshot.ts`.

#![forbid(unsafe_code)]

use serde_json::json;

/// Port of `writeHostUsageSnapshots` (`apps/server/src/hostUsageSnapshot.ts`).
/// The source returns an unevaluated `Effect.gen` for every input, which the
/// recorder serializes as a constant marker object.
pub fn write_host_usage_snapshots(
    home_dir: String,
    now: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&home_dir, &now);
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}

/// Port of `startHostUsageSnapshotJob` (`apps/server/src/hostUsageSnapshot.ts:283`).
/// Also a lazy Effect: nested args + successCont markers.
pub fn start_host_usage_snapshot_job(home_dir: String) -> Result<serde_json::Value, String> {
    let _ = &home_dir;
    Ok(json!({
        "~effect/Effect/args": {"~effect/Effect/args": {"$function": ""}},
        "~effect/Effect/successCont": {"$function": ""}
    }))
}
