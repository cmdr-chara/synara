//! JS bindings for `apps/server/src/remotePairing/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `requestLocalRemoteAccess`.
#[napi(js_name = "requestLocalRemoteAccess")]
pub async fn request_local_remote_access(
    base_dir: String,
    request: String,
) -> napi::Result<String> {
    run_async(async move {
        let request = crate::js::from_json(&request)?;
        let out = port::request_local_remote_access(base_dir, request).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `saveRemoteInvitation`.
#[napi(js_name = "saveRemoteInvitation")]
pub async fn save_remote_invitation(file: String, result: String) -> napi::Result<()> {
    run_async(async move {
        let result = crate::js::from_json(&result)?;
        port::save_remote_invitation(file, result).await?;
        Ok(())
    })
    .await
}
