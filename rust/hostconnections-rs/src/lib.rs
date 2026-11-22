//! JS bindings for `apps/server/src/hostConnections/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `buildDialCandidates`.
#[napi(js_name = "buildDialCandidates")]
pub fn build_dial_candidates(host: String) -> napi::Result<String> {
    run(move || {
        let host = crate::js::from_json(&host)?;
        let out = port::build_dial_candidates(host)?;
        crate::js::to_json(&out)
    })
}

/// `exchangeRemoteFrame`.
#[napi(js_name = "exchangeRemoteFrame")]
pub async fn exchange_remote_frame(
    socket: String,
    request: String,
    response_type: String,
    signal: String,
) -> napi::Result<String> {
    run_async(async move {
        let socket = crate::js::from_json(&socket)?;
        let request = crate::js::from_json(&request)?;
        let signal = crate::js::from_json(&signal)?;
        let out = port::exchange_remote_frame(socket, request, response_type, signal).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `backgroundRetryDelayMs`.
#[napi(js_name = "backgroundRetryDelayMs")]
pub fn background_retry_delay_ms(failures: f64, error: String) -> napi::Result<f64> {
    run(move || {
        let error = crate::js::from_json(&error)?;
        let out = port::background_retry_delay_ms(failures, error)?;
        Ok(out)
    })
}

/// `attentionRetryDelayMs`.
#[napi(js_name = "attentionRetryDelayMs")]
pub fn attention_retry_delay_ms(failures: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::attention_retry_delay_ms(failures)?;
        Ok(out)
    })
}

/// `remoteUnaryRpc`.
#[napi(js_name = "remoteUnaryRpc")]
pub async fn remote_unary_rpc(
    socket: String,
    tag: String,
    payload: String,
    signal: String,
) -> napi::Result<String> {
    run_async(async move {
        let socket = crate::js::from_json(&socket)?;
        let payload = crate::js::from_json(&payload)?;
        let signal = crate::js::from_json(&signal)?;
        let out = port::remote_unary_rpc(socket, tag, payload, signal).await?;
        crate::js::to_json(&out)
    })
    .await
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

static S_BUILD_DIAL_CANDIDATES_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_DIAL_CANDIDATES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `buildDialCandidates` over the binary wire.
#[napi(js_name = "buildDialCandidatesWire")]
pub fn build_dial_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let host: serde_json::Value = wire::read_value(r, &S_BUILD_DIAL_CANDIDATES_0)?;
        let out = &port::build_dial_candidates(host)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_BUILD_DIAL_CANDIDATES_OUT.inner())?;
        }
        Ok(())
    })
}

static S_BACKGROUND_RETRY_DELAY_MS_1: wire::Schema = wire::Schema::Dyn;

/// `backgroundRetryDelayMs` over the binary wire.
#[napi(js_name = "backgroundRetryDelayMsWire")]
pub fn background_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let failures: f64 = r.f64()?;
        let error: serde_json::Value = wire::read_value(r, &S_BACKGROUND_RETRY_DELAY_MS_1)?;
        let out = &port::background_retry_delay_ms(failures, error)?;
        w.f64(*out);
        Ok(())
    })
}

/// `attentionRetryDelayMs` over the binary wire.
#[napi(js_name = "attentionRetryDelayMsWire")]
pub fn attention_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let failures: f64 = r.f64()?;
        let out = &port::attention_retry_delay_ms(failures)?;
        w.f64(*out);
        Ok(())
    })
}
