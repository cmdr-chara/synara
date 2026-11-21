//! JS bindings for `apps/web/src/routeRestoreRefreshCoordinator.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod port;

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[allow(dead_code)]
fn run<T>(f: impl FnOnce() -> Result<T, String>) -> napi::Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

#[allow(dead_code)]
async fn run_async<T>(f: impl std::future::Future<Output = Result<T, String>>) -> napi::Result<T> {
    use futures::FutureExt as _;
    match std::panic::AssertUnwindSafe(f).catch_unwind().await {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `requestShellSnapshotRefresh`.
#[napi(js_name = "requestShellSnapshotRefresh")]
pub async fn request_shell_snapshot_refresh(min_sequence: f64) -> napi::Result<bool> {
    run_async(async move {
        let out = port::request_shell_snapshot_refresh(min_sequence).await?;
        Ok(out)
    })
    .await
}

/// `requestEmptyRouteRestoreRefresh`.
#[napi(js_name = "requestEmptyRouteRestoreRefresh")]
pub async fn request_empty_route_restore_refresh() -> napi::Result<String> {
    run_async(async move {
        let out = port::request_empty_route_restore_refresh().await?;
        Ok(out)
    })
    .await
}
