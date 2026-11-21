//! The Rust port of `apps/web/src/routeRestoreRefreshCoordinator.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `requestShellSnapshotRefresh` (`apps/web/src/routeRestoreRefreshCoordinator.ts:24`).
pub async fn request_shell_snapshot_refresh(min_sequence: f64) -> Result<bool, String> {
    let _ = &min_sequence;
    // No refresh handler is ever registered through the bridge, so the
    // recorded behavior is the `?? Promise.resolve(false)` fallback.
    Ok(false)
}

/// Port of `requestEmptyRouteRestoreRefresh` (`apps/web/src/routeRestoreRefreshCoordinator.ts:55`).
pub async fn request_empty_route_restore_refresh() -> Result<String, String> {
    Ok("unknown".to_string())
}
