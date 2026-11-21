//! JS bindings for `packages/shared/src/pinnedMessages.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isMessagePinned`.
#[napi(js_name = "isMessagePinned")]
pub fn is_message_pinned(pins: String, message_id: String) -> napi::Result<bool> {
    run(move || {
        let pins = crate::js::from_json(&pins)?;
        let message_id = crate::js::from_json(&message_id)?;
        let out = port::is_message_pinned(pins, message_id)?;
        Ok(out)
    })
}

/// `addPinnedMessage`.
#[napi(js_name = "addPinnedMessage")]
pub fn add_pinned_message(pins: String, pin: String) -> napi::Result<String> {
    run(move || {
        let pins = crate::js::from_json(&pins)?;
        let pin = crate::js::from_json(&pin)?;
        let out = port::add_pinned_message(pins, pin)?;
        crate::js::to_json(&out)
    })
}

/// `removePinnedMessage`.
#[napi(js_name = "removePinnedMessage")]
pub fn remove_pinned_message(pins: String, message_id: String) -> napi::Result<String> {
    run(move || {
        let pins = crate::js::from_json(&pins)?;
        let message_id = crate::js::from_json(&message_id)?;
        let out = port::remove_pinned_message(pins, message_id)?;
        crate::js::to_json(&out)
    })
}

/// `setPinnedMessageDone`.
#[napi(js_name = "setPinnedMessageDone")]
pub fn set_pinned_message_done(
    pins: String,
    message_id: String,
    done: bool,
) -> napi::Result<String> {
    run(move || {
        let pins = crate::js::from_json(&pins)?;
        let message_id = crate::js::from_json(&message_id)?;
        let out = port::set_pinned_message_done(pins, message_id, done)?;
        crate::js::to_json(&out)
    })
}

/// `togglePinnedMessageDone`.
#[napi(js_name = "togglePinnedMessageDone")]
pub fn toggle_pinned_message_done(pins: String, message_id: String) -> napi::Result<String> {
    run(move || {
        let pins = crate::js::from_json(&pins)?;
        let message_id = crate::js::from_json(&message_id)?;
        let out = port::toggle_pinned_message_done(pins, message_id)?;
        crate::js::to_json(&out)
    })
}

/// `normalizePinLabel`.
#[napi(js_name = "normalizePinLabel")]
pub fn normalize_pin_label(
    label: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let label = crate::js::from_json(&label)?;
        let out = port::normalize_pin_label(label)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `setPinnedMessageLabel`.
#[napi(js_name = "setPinnedMessageLabel")]
pub fn set_pinned_message_label(
    pins: String,
    message_id: String,
    label: String,
) -> napi::Result<String> {
    run(move || {
        let pins = crate::js::from_json(&pins)?;
        let message_id = crate::js::from_json(&message_id)?;
        let label = crate::js::from_json(&label)?;
        let out = port::set_pinned_message_label(pins, message_id, label)?;
        crate::js::to_json(&out)
    })
}

/// `clampThreadNotes`.
#[napi(js_name = "clampThreadNotes")]
pub fn clamp_thread_notes(notes: String) -> napi::Result<String> {
    run(move || {
        let out = port::clamp_thread_notes(notes)?;
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

static S_IS_MESSAGE_PINNED_0: wire::Schema = wire::Schema::Dyn;
static S_IS_MESSAGE_PINNED_1: wire::Schema = wire::Schema::Dyn;

/// `isMessagePinned` over the binary wire.
#[napi(js_name = "isMessagePinnedWire")]
pub fn is_message_pinned_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pins: serde_json::Value = wire::read_value(r, &S_IS_MESSAGE_PINNED_0)?;
        let message_id: serde_json::Value = wire::read_value(r, &S_IS_MESSAGE_PINNED_1)?;
        let out = &port::is_message_pinned(pins, message_id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_ADD_PINNED_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_ADD_PINNED_MESSAGE_1: wire::Schema = wire::Schema::Dyn;

/// `addPinnedMessage` over the binary wire.
#[napi(js_name = "addPinnedMessageWire")]
pub fn add_pinned_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pins: serde_json::Value = wire::read_value(r, &S_ADD_PINNED_MESSAGE_0)?;
        let pin: serde_json::Value = wire::read_value(r, &S_ADD_PINNED_MESSAGE_1)?;
        let out = &port::add_pinned_message(pins, pin)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_REMOVE_PINNED_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_PINNED_MESSAGE_1: wire::Schema = wire::Schema::Dyn;

/// `removePinnedMessage` over the binary wire.
#[napi(js_name = "removePinnedMessageWire")]
pub fn remove_pinned_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pins: serde_json::Value = wire::read_value(r, &S_REMOVE_PINNED_MESSAGE_0)?;
        let message_id: serde_json::Value = wire::read_value(r, &S_REMOVE_PINNED_MESSAGE_1)?;
        let out = &port::remove_pinned_message(pins, message_id)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SET_PINNED_MESSAGE_DONE_0: wire::Schema = wire::Schema::Dyn;
static S_SET_PINNED_MESSAGE_DONE_1: wire::Schema = wire::Schema::Dyn;

/// `setPinnedMessageDone` over the binary wire.
#[napi(js_name = "setPinnedMessageDoneWire")]
pub fn set_pinned_message_done_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pins: serde_json::Value = wire::read_value(r, &S_SET_PINNED_MESSAGE_DONE_0)?;
        let message_id: serde_json::Value = wire::read_value(r, &S_SET_PINNED_MESSAGE_DONE_1)?;
        let done: bool = r.bool()?;
        let out = &port::set_pinned_message_done(pins, message_id, done)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_TOGGLE_PINNED_MESSAGE_DONE_0: wire::Schema = wire::Schema::Dyn;
static S_TOGGLE_PINNED_MESSAGE_DONE_1: wire::Schema = wire::Schema::Dyn;

/// `togglePinnedMessageDone` over the binary wire.
#[napi(js_name = "togglePinnedMessageDoneWire")]
pub fn toggle_pinned_message_done_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pins: serde_json::Value = wire::read_value(r, &S_TOGGLE_PINNED_MESSAGE_DONE_0)?;
        let message_id: serde_json::Value = wire::read_value(r, &S_TOGGLE_PINNED_MESSAGE_DONE_1)?;
        let out = &port::toggle_pinned_message_done(pins, message_id)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_NORMALIZE_PIN_LABEL_0: wire::Schema = wire::Schema::Dyn;

/// `normalizePinLabel` over the binary wire.
#[napi(js_name = "normalizePinLabelWire")]
pub fn normalize_pin_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let label: serde_json::Value = wire::read_value(r, &S_NORMALIZE_PIN_LABEL_0)?;
        let out = &port::normalize_pin_label(label)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_SET_PINNED_MESSAGE_LABEL_0: wire::Schema = wire::Schema::Dyn;
static S_SET_PINNED_MESSAGE_LABEL_1: wire::Schema = wire::Schema::Dyn;
static S_SET_PINNED_MESSAGE_LABEL_2: wire::Schema = wire::Schema::Dyn;

/// `setPinnedMessageLabel` over the binary wire.
#[napi(js_name = "setPinnedMessageLabelWire")]
pub fn set_pinned_message_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pins: serde_json::Value = wire::read_value(r, &S_SET_PINNED_MESSAGE_LABEL_0)?;
        let message_id: serde_json::Value = wire::read_value(r, &S_SET_PINNED_MESSAGE_LABEL_1)?;
        let label: serde_json::Value = wire::read_value(r, &S_SET_PINNED_MESSAGE_LABEL_2)?;
        let out = &port::set_pinned_message_label(pins, message_id, label)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `clampThreadNotes` over the binary wire.
#[napi(js_name = "clampThreadNotesWire")]
pub fn clamp_thread_notes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let notes: String = r.string16()?;
        let out = &port::clamp_thread_notes(notes)?;
        w.string16(out);
        Ok(())
    })
}
