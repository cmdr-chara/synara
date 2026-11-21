//! JS bindings for `apps/web/src/threadGoal.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `dispatchThreadGoal`.
#[napi(js_name = "dispatchThreadGoal")]
pub async fn dispatch_thread_goal(
    thread_id: String,
    goal: String,
    options: String,
) -> napi::Result<()> {
    run_async(async move {
        let thread_id = crate::js::from_json(&thread_id)?;
        let options = crate::js::from_json(&options)?;
        port::dispatch_thread_goal(thread_id, goal, options).await?;
        Ok(())
    })
    .await
}

/// `dispatchThreadGoalPaused`.
#[napi(js_name = "dispatchThreadGoalPaused")]
pub async fn dispatch_thread_goal_paused(thread_id: String, paused: bool) -> napi::Result<()> {
    run_async(async move {
        let thread_id = crate::js::from_json(&thread_id)?;
        port::dispatch_thread_goal_paused(thread_id, paused).await?;
        Ok(())
    })
    .await
}
