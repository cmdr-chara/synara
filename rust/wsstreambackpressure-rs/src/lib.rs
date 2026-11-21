//! JS bindings for `apps/server/src/wsStreamBackpressure.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

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

/// `makeLiveUiStreamLagState`.
#[napi(js_name = "makeLiveUiStreamLagState")]
pub fn make_live_ui_stream_lag_state() -> napi::Result<String> {
    run(move || {
        let out = port::make_live_ui_stream_lag_state()?;
        crate::js::to_json(&out)
    })
}

/// `normalizeLiveUiStreamBufferCapacity`.
#[napi(js_name = "normalizeLiveUiStreamBufferCapacity")]
pub fn normalize_live_ui_stream_buffer_capacity(capacity: String) -> napi::Result<String> {
    run(move || {
        let capacity = crate::js::from_json(&capacity)?;
        let out = port::normalize_live_ui_stream_buffer_capacity(capacity)?;
        crate::js::to_json(&out)
    })
}

/// `recordLiveUiStreamIngress`.
#[napi(js_name = "recordLiveUiStreamIngress")]
pub fn record_live_ui_stream_ingress(
    state: String,
    capacity: String,
    report_growth_step: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let capacity = crate::js::from_json(&capacity)?;
        let report_growth_step = crate::js::from_json(&report_growth_step)?;
        let out = port::record_live_ui_stream_ingress(state, capacity, report_growth_step)?;
        crate::js::to_json(&out)
    })
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

static S_MAKE_LIVE_UI_STREAM_LAG_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("ingressCount", wire::Schema::Num),
    ("egressCount", wire::Schema::Num),
    ("reportedDroppedAtLeast", wire::Schema::Num),
]);

/// `makeLiveUiStreamLagState` over the binary wire.
#[napi(js_name = "makeLiveUiStreamLagStateWire")]
pub fn make_live_ui_stream_lag_state_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::make_live_ui_stream_lag_state()?;
        wire::write_value(w, out, &S_MAKE_LIVE_UI_STREAM_LAG_STATE_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_LIVE_UI_STREAM_BUFFER_CAPACITY_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_LIVE_UI_STREAM_BUFFER_CAPACITY_OUT: wire::Schema = wire::Schema::Num;

/// `normalizeLiveUiStreamBufferCapacity` over the binary wire.
#[napi(js_name = "normalizeLiveUiStreamBufferCapacityWire")]
pub fn normalize_live_ui_stream_buffer_capacity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let capacity: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_LIVE_UI_STREAM_BUFFER_CAPACITY_0)?;
        let out = &port::normalize_live_ui_stream_buffer_capacity(capacity)?;
        wire::write_value(w, out, &S_NORMALIZE_LIVE_UI_STREAM_BUFFER_CAPACITY_OUT)?;
        Ok(())
    })
}

static S_RECORD_LIVE_UI_STREAM_INGRESS_0: wire::Schema = wire::Schema::Dyn;
static S_RECORD_LIVE_UI_STREAM_INGRESS_1: wire::Schema = wire::Schema::Dyn;
static S_RECORD_LIVE_UI_STREAM_INGRESS_2: wire::Schema = wire::Schema::Dyn;
static S_RECORD_LIVE_UI_STREAM_INGRESS_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Num);

/// `recordLiveUiStreamIngress` over the binary wire.
#[napi(js_name = "recordLiveUiStreamIngressWire")]
pub fn record_live_ui_stream_ingress_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_RECORD_LIVE_UI_STREAM_INGRESS_0)?;
        let capacity: serde_json::Value = wire::read_value(r, &S_RECORD_LIVE_UI_STREAM_INGRESS_1)?;
        let report_growth_step: serde_json::Value =
            wire::read_value(r, &S_RECORD_LIVE_UI_STREAM_INGRESS_2)?;
        let out = &port::record_live_ui_stream_ingress(state, capacity, report_growth_step)?;
        wire::write_value(w, out, &S_RECORD_LIVE_UI_STREAM_INGRESS_OUT)?;
        Ok(())
    })
}
