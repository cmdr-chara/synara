//! JS bindings for `apps/desktop/src/backendSupervisionPolicy.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;
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

/// `backendRestartDelayMs`.
#[napi(js_name = "backendRestartDelayMs")]
pub fn backend_restart_delay_ms(attempt: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::backend_restart_delay_ms(attempt)?;
        Ok(out)
    })
}

/// `summarizeBackendFailureOutput`.
#[napi(js_name = "summarizeBackendFailureOutput")]
pub fn summarize_backend_failure_output(output: String, max_lines: String) -> napi::Result<String> {
    run(move || {
        let max_lines = crate::js::from_json(&max_lines)?;
        let out = port::summarize_backend_failure_output(output, max_lines)?;
        Ok(out)
    })
}

/// Class `BackendOutputTailDetector`.
#[napi(js_name = "BackendOutputTailDetector")]
pub struct BackendOutputTailDetector {
    inner: port::BackendOutputTailDetector,
}

#[napi]
impl BackendOutputTailDetector {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        run(move || {
            let out = port::BackendOutputTailDetector::new()?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "push")]
    pub fn push(&mut self, chunk: String) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            let chunk = crate::js::from_json(&chunk)?;
            inner.push(chunk)?;
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

// ---- the binary wire ------------------------------------------------------------------------

type WireBuffers = (
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint32Array,
);

thread_local! {
    static WIRE: std::cell::RefCell<Option<WireBuffers>> = const { std::cell::RefCell::new(None) };
    static PENDING: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static SCRATCH: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers the argument, result and argument-length buffers `index.cjs` uses.
#[napi(js_name = "rustAnythingWireInit")]
pub fn rust_anything_wire_init(
    input: napi::bindgen_prelude::Uint8Array,
    output: napi::bindgen_prelude::Uint8Array,
    len: napi::bindgen_prelude::Uint32Array,
) {
    WIRE.with(|w| *w.borrow_mut() = Some((input, output, len)));
}

/// Copies a result that did not fit into the (now larger) result buffer.
#[napi(js_name = "rustAnythingWireFlush")]
pub fn rust_anything_wire_flush() -> f64 {
    let bytes = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    WIRE.with(|w| {
        let mut w = w.borrow_mut();
        let Some((_, output, _)) = w.as_mut() else {
            return -1.0;
        };
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        n as f64
    })
}

/// Reads the argument bytes (`wLen[0]` of them), runs `f`, and writes its result; a negative
/// return is the size of a result that did not fit, kept for `rustAnythingWireFlush`.
fn wire_io(
    f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>,
) -> napi::Result<f64> {
    WIRE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some((input, output, len)) = guard.as_mut() else {
            return Err(napi::Error::from_reason(
                "rust-anything: the wire buffers are not registered",
            ));
        };
        let len = (len.first().copied().unwrap_or(0) as usize).min(input.len());
        let mut w = wire::Writer(SCRATCH.with(|s| std::mem::take(&mut *s.borrow_mut())));
        w.0.clear();
        let result = run(|| f(&mut wire::Reader::new(&input[..len]), &mut w));
        let keep = |w: wire::Writer| SCRATCH.with(|s| *s.borrow_mut() = w.0);
        match result {
            // A result of another shape than the TS type: JS runs the call through JSON.
            Err(e) if e.reason.starts_with(wire::MISMATCH) => {
                keep(w);
                return Ok(f64::NAN);
            }
            Err(e) => {
                keep(w);
                let mut m = wire::Writer::default();
                m.string16(&e.reason);
                // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
                let out = unsafe { output.as_mut() };
                if m.0.len() > out.len() {
                    return Err(e);
                }
                out[..m.0.len()].copy_from_slice(&m.0);
                return Ok(f64::INFINITY);
            }
            Ok(()) => {}
        }
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = w.0.len();
        if n > out.len() {
            PENDING.with(|p| *p.borrow_mut() = w.0);
            return Ok(-(n as f64));
        }
        out[..n].copy_from_slice(&w.0);
        keep(w);
        Ok(n as f64)
    })
}

/// `backendRestartDelayMs` over the binary wire.
#[napi(js_name = "backendRestartDelayMsWire")]
pub fn backend_restart_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let attempt: f64 = r.f64()?;
        let out = &port::backend_restart_delay_ms(attempt)?;
        w.f64(*out);
        Ok(())
    })
}

static S_SUMMARIZE_BACKEND_FAILURE_OUTPUT_1: wire::Schema = wire::Schema::Dyn;

/// `summarizeBackendFailureOutput` over the binary wire.
#[napi(js_name = "summarizeBackendFailureOutputWire")]
pub fn summarize_backend_failure_output_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let output: String = r.string16()?;
        let max_lines: serde_json::Value =
            wire::read_value(r, &S_SUMMARIZE_BACKEND_FAILURE_OUTPUT_1)?;
        let out = &port::summarize_backend_failure_output(output, max_lines)?;
        w.string16(out);
        Ok(())
    })
}
