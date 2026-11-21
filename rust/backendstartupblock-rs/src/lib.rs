//! JS bindings for `apps/desktop/src/backendStartupBlock.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// Class `BackendStartupBlockDetector`.
#[napi(js_name = "BackendStartupBlockDetector")]
pub struct BackendStartupBlockDetector {
    inner: port::BackendStartupBlockDetector,
}

#[napi]
impl BackendStartupBlockDetector {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        run(move || {
            let out = port::BackendStartupBlockDetector::new()?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "push")]
    pub fn push(&mut self, chunk: String, source: Option<String>) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            let chunk = crate::js::from_json(&chunk)?;
            inner.push(chunk, source)?;
            Ok(())
        })
    }

    #[napi(js_name = "end")]
    pub fn end(&mut self, source: String) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.end(source)?;
            Ok(())
        })
    }

    #[napi(js_name = "freeze")]
    pub fn freeze(&mut self) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.freeze()?;
            Ok(())
        })
    }

    #[napi(js_name = "read")]
    pub fn read(&mut self) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let out = inner.read()?;
            crate::js::to_json(&out)
        })
    }
}
