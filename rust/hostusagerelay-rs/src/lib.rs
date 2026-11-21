//! JS bindings for `apps/desktop/src/hostUsageRelay.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// Class `HostUsageRelay`.
#[napi(js_name = "HostUsageRelay")]
pub struct HostUsageRelay {
    inner: port::HostUsageRelay,
}

#[napi]
impl HostUsageRelay {
    #[napi(constructor)]
    pub fn new(home_dir: String, channel: String) -> napi::Result<Self> {
        run(move || {
            let out = port::HostUsageRelay::new(home_dir, channel)?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "enable")]
    pub fn enable(&mut self) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.enable()?;
            Ok(())
        })
    }

    #[napi(js_name = "clear")]
    pub fn clear(&mut self) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.clear()?;
            Ok(())
        })
    }

    #[napi(js_name = "peek")]
    pub fn peek(&mut self, install_id: String) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let out = inner.peek(install_id)?;
            crate::js::to_json(&out)
        })
    }

    #[napi(js_name = "acknowledge")]
    pub fn acknowledge(&mut self, window: String) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            let window = crate::js::from_json(&window)?;
            inner.acknowledge(window)?;
            Ok(())
        })
    }
}
