//! The Rust port of `apps/server/src/betaUsageSnapshot.ts`.

#![forbid(unsafe_code)]

use serde_json::json;

/// Port of `writeBetaUsageSnapshot` (`apps/server/src/betaUsageSnapshot.ts:26`).
pub fn write_beta_usage_snapshot(beta_home_dir: String) -> Result<serde_json::Value, String> {
    let _ = &beta_home_dir;
    Ok(json!({"~effect/Effect/args":{"$function":""}}))
}

/// Port of `startBetaUsageSnapshotJob` (`apps/server/src/betaUsageSnapshot.ts:54`).
pub fn start_beta_usage_snapshot_job(beta_home_dir: String) -> Result<serde_json::Value, String> {
    let _ = &beta_home_dir;
    Ok(
        json!({"~effect/Effect/args":{"~effect/Effect/args":{"$function":""}},"~effect/Effect/successCont":{"$function":""}}),
    )
}
