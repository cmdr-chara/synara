//! JS bindings for `apps/server/src/remoteTransport/tunnel.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod utf16;

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

/// `connectRemoteTls`.
#[napi(js_name = "connectRemoteTls")]
pub async fn connect_remote_tls(
    outer: String,
    anchor: String,
    signal: String,
    timeout_ms: String,
) -> napi::Result<String> {
    run_async(async move {
        let outer = crate::js::from_json(&outer)?;
        let anchor = crate::js::from_json(&anchor)?;
        let signal = crate::js::from_json(&signal)?;
        let timeout_ms = crate::js::from_json(&timeout_ms)?;
        let out = port::connect_remote_tls(outer, anchor, signal, timeout_ms).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `connectRemoteWebSocket`.
#[napi(js_name = "connectRemoteWebSocket")]
pub async fn connect_remote_web_socket(
    outer: String,
    anchor: String,
    path: String,
    signal: String,
) -> napi::Result<String> {
    run_async(async move {
        let outer = crate::js::from_json(&outer)?;
        let anchor = crate::js::from_json(&anchor)?;
        let path = crate::js::from_json(&path)?;
        let signal = crate::js::from_json(&signal)?;
        let out = port::connect_remote_web_socket(outer, anchor, path, signal).await?;
        crate::js::to_json(&out)
    })
    .await
}
