//! The Rust port of `waitForEmptyRouteRestoreFallbackDelay`
//! (`apps/web/src/chatRouteRecovery.ts:14`): resolves after
//! `EMPTY_ROUTE_RESTORE_FALLBACK_DELAY_MS` (`apps/web/src/chatRouteRestore.ts:13`).

pub async fn wait_for_empty_route_restore_fallback_delay() -> Result<(), String> {
    tokio::time::sleep(std::time::Duration::from_millis(1_800)).await;
    Ok(())
}
