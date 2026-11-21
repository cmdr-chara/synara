//! JS bindings for `apps/web/src/appSnapIntake.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
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

/// `sourceWithCachedIcon`.
#[napi(js_name = "sourceWithCachedIcon")]
pub async fn source_with_cached_icon(source: String) -> napi::Result<String> {
    run_async(async move {
        let source = crate::js::from_json(&source)?;
        let out = port::source_with_cached_icon(source).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `insertAppSnapCaptureIntoDraft`.
#[napi(js_name = "insertAppSnapCaptureIntoDraft")]
pub async fn insert_app_snap_capture_into_draft(
    thread_id: String,
    capture: String,
) -> napi::Result<String> {
    run_async(async move {
        let thread_id = crate::js::from_json(&thread_id)?;
        let capture = crate::js::from_json(&capture)?;
        let out = port::insert_app_snap_capture_into_draft(thread_id, capture).await?;
        Ok(out)
    })
    .await
}
