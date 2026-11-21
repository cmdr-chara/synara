//! JS bindings for `apps/server/src/remoteBillingPolicy.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `recordRemoteBillingPolicyHint`.
#[napi(js_name = "recordRemoteBillingPolicyHint")]
pub async fn record_remote_billing_policy_hint(
    state_dir: String,
    account_url: String,
    enforced: bool,
    checked_at: String,
    requested_at: Option<f64>,
) -> napi::Result<()> {
    run_async(async move {
        let checked_at = crate::js::from_json(&checked_at)?;
        port::record_remote_billing_policy_hint(
            state_dir,
            account_url,
            enforced,
            checked_at,
            requested_at,
        )
        .await?;
        Ok(())
    })
    .await
}
