//! JS bindings for `packages/shared/src/frameTransport.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `encodeFrameEnvelope`.
#[napi(js_name = "encodeFrameEnvelope")]
pub fn encode_frame_envelope(config: String, frame: String) -> napi::Result<String> {
    run(move || {
        let config = crate::js::from_json(&config)?;
        let frame = crate::js::from_json(&frame)?;
        let out = port::encode_frame_envelope(config, frame)?;
        crate::js::to_json(&out)
    })
}

/// `decodeFrameEnvelope`.
#[napi(js_name = "decodeFrameEnvelope")]
pub fn decode_frame_envelope(config: String, bytes: String) -> napi::Result<String> {
    run(move || {
        let config = crate::js::from_json(&config)?;
        let bytes = crate::js::from_json(&bytes)?;
        let out = port::decode_frame_envelope(config, bytes)?;
        crate::js::to_json(&out)
    })
}

/// `peekFrameHeader`.
#[napi(js_name = "peekFrameHeader")]
pub fn peek_frame_header(config: String, bytes: String) -> napi::Result<String> {
    run(move || {
        let config = crate::js::from_json(&config)?;
        let bytes = crate::js::from_json(&bytes)?;
        let out = port::peek_frame_header(config, bytes)?;
        crate::js::to_json(&out)
    })
}

/// `classifyByFrameFlags`.
#[napi(js_name = "classifyByFrameFlags")]
pub fn classify_by_frame_flags(frame: String) -> napi::Result<String> {
    run(move || {
        let frame = crate::js::from_json(&frame)?;
        let out = port::classify_by_frame_flags(frame)?;
        crate::js::to_json(&out)
    })
}

/// `decodeFrameResyncRequest`.
#[napi(js_name = "decodeFrameResyncRequest")]
pub fn decode_frame_resync_request(
    message: String,
    resync_message: String,
    max_bytes: String,
) -> napi::Result<String> {
    run(move || {
        let message = crate::js::from_json(&message)?;
        let max_bytes = crate::js::from_json(&max_bytes)?;
        let out = port::decode_frame_resync_request(message, resync_message, max_bytes)?;
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

static S_ENCODE_FRAME_ENVELOPE_0: wire::Schema = wire::Schema::Dyn;
static S_ENCODE_FRAME_ENVELOPE_1: wire::Schema = wire::Schema::Dyn;
static S_ENCODE_FRAME_ENVELOPE_OUT: wire::Schema = wire::Schema::Dyn;

/// `encodeFrameEnvelope` over the binary wire.
#[napi(js_name = "encodeFrameEnvelopeWire")]
pub fn encode_frame_envelope_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: serde_json::Value = wire::read_value(r, &S_ENCODE_FRAME_ENVELOPE_0)?;
        let frame: serde_json::Value = wire::read_value(r, &S_ENCODE_FRAME_ENVELOPE_1)?;
        let out = &port::encode_frame_envelope(config, frame)?;
        wire::write_value(w, out, &S_ENCODE_FRAME_ENVELOPE_OUT)?;
        Ok(())
    })
}

static S_DECODE_FRAME_ENVELOPE_0: wire::Schema = wire::Schema::Dyn;
static S_DECODE_FRAME_ENVELOPE_1: wire::Schema = wire::Schema::Dyn;
static S_DECODE_FRAME_ENVELOPE_OUT: wire::Schema = wire::Schema::Dyn;

/// `decodeFrameEnvelope` over the binary wire.
#[napi(js_name = "decodeFrameEnvelopeWire")]
pub fn decode_frame_envelope_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: serde_json::Value = wire::read_value(r, &S_DECODE_FRAME_ENVELOPE_0)?;
        let bytes: serde_json::Value = wire::read_value(r, &S_DECODE_FRAME_ENVELOPE_1)?;
        let out = &port::decode_frame_envelope(config, bytes)?;
        wire::write_value(w, out, &S_DECODE_FRAME_ENVELOPE_OUT)?;
        Ok(())
    })
}

static S_PEEK_FRAME_HEADER_0: wire::Schema = wire::Schema::Dyn;
static S_PEEK_FRAME_HEADER_1: wire::Schema = wire::Schema::Dyn;
static S_PEEK_FRAME_HEADER_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("streamId", wire::Schema::Str16),
    ("sequence", wire::Schema::Num),
    ("timestampMs", wire::Schema::Num),
    ("keyframe", wire::Schema::Bool),
    ("codecConfig", wire::Schema::Bool),
]));

/// `peekFrameHeader` over the binary wire.
#[napi(js_name = "peekFrameHeaderWire")]
pub fn peek_frame_header_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: serde_json::Value = wire::read_value(r, &S_PEEK_FRAME_HEADER_0)?;
        let bytes: serde_json::Value = wire::read_value(r, &S_PEEK_FRAME_HEADER_1)?;
        let out = &port::peek_frame_header(config, bytes)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PEEK_FRAME_HEADER_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_CLASSIFY_BY_FRAME_FLAGS_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_BY_FRAME_FLAGS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("keyframe", wire::Schema::Bool),
    ("codecConfig", wire::Schema::Bool),
]);

/// `classifyByFrameFlags` over the binary wire.
#[napi(js_name = "classifyByFrameFlagsWire")]
pub fn classify_by_frame_flags_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let frame: serde_json::Value = wire::read_value(r, &S_CLASSIFY_BY_FRAME_FLAGS_0)?;
        let out = &port::classify_by_frame_flags(frame)?;
        wire::write_value(w, out, &S_CLASSIFY_BY_FRAME_FLAGS_OUT)?;
        Ok(())
    })
}

static S_DECODE_FRAME_RESYNC_REQUEST_0: wire::Schema = wire::Schema::Dyn;
static S_DECODE_FRAME_RESYNC_REQUEST_2: wire::Schema = wire::Schema::Dyn;
static S_DECODE_FRAME_RESYNC_REQUEST_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `decodeFrameResyncRequest` over the binary wire.
#[napi(js_name = "decodeFrameResyncRequestWire")]
pub fn decode_frame_resync_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: serde_json::Value = wire::read_value(r, &S_DECODE_FRAME_RESYNC_REQUEST_0)?;
        let resync_message: String = r.string16()?;
        let max_bytes: serde_json::Value = wire::read_value(r, &S_DECODE_FRAME_RESYNC_REQUEST_2)?;
        let out = &port::decode_frame_resync_request(message, resync_message, max_bytes)?;
        wire::write_value(w, out, &S_DECODE_FRAME_RESYNC_REQUEST_OUT)?;
        Ok(())
    })
}
