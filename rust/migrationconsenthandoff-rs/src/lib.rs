//! JS bindings for `apps/desktop/src/migrationConsentHandoff.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// Class `MigrationConsentHandoff`.
#[napi(js_name = "MigrationConsentHandoff")]
pub struct MigrationConsentHandoff {
    inner: port::MigrationConsentHandoff,
}

#[napi]
impl MigrationConsentHandoff {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        run(move || {
            let out = port::MigrationConsentHandoff::new()?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "approve")]
    pub fn approve(&mut self, consent_token: String) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.approve(consent_token)?;
            Ok(())
        })
    }

    #[napi(js_name = "take")]
    pub fn take(&mut self) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let out = inner.take()?;
            crate::js::to_json(&out)
        })
    }
}
