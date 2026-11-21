//! JS bindings for `packages/shared/src/asyncUserInput.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `mergeAsyncUserInput`.
#[napi(js_name = "mergeAsyncUserInput")]
pub fn merge_async_user_input(previous: String, incoming: String) -> napi::Result<String> {
    run(move || {
        let previous = crate::js::from_json(&previous)?;
        let incoming = crate::js::from_json(&incoming)?;
        let out = port::merge_async_user_input(previous, incoming)?;
        crate::js::to_json(&out)
    })
}

/// `clearRemovedAsyncUserInputResponses`.
#[napi(js_name = "clearRemovedAsyncUserInputResponses")]
pub fn clear_removed_async_user_input_responses(
    messages: String,
    retained_message_ids: String,
    sequence: f64,
) -> napi::Result<String> {
    run(move || {
        let messages = crate::js::from_json(&messages)?;
        let retained_message_ids = crate::js::from_json(&retained_message_ids)?;
        let out = port::clear_removed_async_user_input_responses(
            messages,
            retained_message_ids,
            sequence,
        )?;
        crate::js::to_json(&out)
    })
}

/// `formatAsyncUserInputResponse`.
#[napi(js_name = "formatAsyncUserInputResponse")]
pub fn format_async_user_input_response(
    questions: String,
    answers: String,
) -> napi::Result<String> {
    run(move || {
        let questions = crate::js::from_json(&questions)?;
        let answers = crate::js::from_json(&answers)?;
        let out = port::format_async_user_input_response(questions, answers)?;
        Ok(out)
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

static S_MERGE_ASYNC_USER_INPUT_0: wire::Schema = wire::Schema::Dyn;
static S_MERGE_ASYNC_USER_INPUT_1: wire::Schema = wire::Schema::Dyn;
static S_MERGE_ASYNC_USER_INPUT_OUT: wire::Schema = wire::Schema::Dyn;

/// `mergeAsyncUserInput` over the binary wire.
#[napi(js_name = "mergeAsyncUserInputWire")]
pub fn merge_async_user_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let previous: serde_json::Value = wire::read_value(r, &S_MERGE_ASYNC_USER_INPUT_0)?;
        let incoming: serde_json::Value = wire::read_value(r, &S_MERGE_ASYNC_USER_INPUT_1)?;
        let out = &port::merge_async_user_input(previous, incoming)?;
        wire::write_value(w, out, &S_MERGE_ASYNC_USER_INPUT_OUT)?;
        Ok(())
    })
}

static S_CLEAR_REMOVED_ASYNC_USER_INPUT_RESPONSES_0: wire::Schema = wire::Schema::Dyn;
static S_CLEAR_REMOVED_ASYNC_USER_INPUT_RESPONSES_1: wire::Schema = wire::Schema::Dyn;
static S_CLEAR_REMOVED_ASYNC_USER_INPUT_RESPONSES_OUT: wire::Schema = wire::Schema::Dyn;

/// `clearRemovedAsyncUserInputResponses` over the binary wire.
#[napi(js_name = "clearRemovedAsyncUserInputResponsesWire")]
pub fn clear_removed_async_user_input_responses_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let messages: serde_json::Value =
            wire::read_value(r, &S_CLEAR_REMOVED_ASYNC_USER_INPUT_RESPONSES_0)?;
        let retained_message_ids: serde_json::Value =
            wire::read_value(r, &S_CLEAR_REMOVED_ASYNC_USER_INPUT_RESPONSES_1)?;
        let sequence: f64 = r.f64()?;
        let out = &port::clear_removed_async_user_input_responses(
            messages,
            retained_message_ids,
            sequence,
        )?;
        wire::write_value(w, out, &S_CLEAR_REMOVED_ASYNC_USER_INPUT_RESPONSES_OUT)?;
        Ok(())
    })
}

static S_FORMAT_ASYNC_USER_INPUT_RESPONSE_0: wire::Schema = wire::Schema::Dyn;
static S_FORMAT_ASYNC_USER_INPUT_RESPONSE_1: wire::Schema = wire::Schema::Dyn;

/// `formatAsyncUserInputResponse` over the binary wire.
#[napi(js_name = "formatAsyncUserInputResponseWire")]
pub fn format_async_user_input_response_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let questions: serde_json::Value =
            wire::read_value(r, &S_FORMAT_ASYNC_USER_INPUT_RESPONSE_0)?;
        let answers: serde_json::Value =
            wire::read_value(r, &S_FORMAT_ASYNC_USER_INPUT_RESPONSE_1)?;
        let out = &port::format_async_user_input_response(questions, answers)?;
        w.string16(out);
        Ok(())
    })
}
