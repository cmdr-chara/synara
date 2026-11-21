//! JS bindings for `packages/shared/src/logging.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// Class `RotatingFileSink`.
#[napi(js_name = "RotatingFileSink")]
pub struct RotatingFileSink {
    inner: port::RotatingFileSink,
}

#[napi]
impl RotatingFileSink {
    #[napi(constructor)]
    pub fn new(options: String) -> napi::Result<Self> {
        run(move || {
            let options = crate::js::from_json(&options)?;
            let out = port::RotatingFileSink::new(options)?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "write")]
    pub fn write(&mut self, chunk: String) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            let chunk = crate::js::from_json(&chunk)?;
            inner.write(chunk)?;
            Ok(())
        })
    }

    #[napi(js_name = "close")]
    pub fn close(&mut self) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.close()?;
            Ok(())
        })
    }
}
