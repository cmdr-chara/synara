//! JS bindings for `packages/shared/src/approvalSessionGrant.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `approvalSessionGrantWidensSessionPolicy`.
#[napi(js_name = "approvalSessionGrantWidensSessionPolicy")]
pub fn approval_session_grant_widens_session_policy(
    request_kind: String,
) -> napi::Result<napi::bindgen_prelude::Either<bool, ()>> {
    run(move || {
        let request_kind = crate::js::from_json(&request_kind)?;
        let out = port::approval_session_grant_widens_session_policy(request_kind)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}
