//! JS bindings for `packages/shared/src/text.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `normalizeLineEndings`.
#[napi(js_name = "normalizeLineEndings")]
pub fn normalize_line_endings(contents: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_line_endings(contents)?;
        Ok(out)
    })
}

/// `splitsSurrogatePair`.
#[napi(js_name = "splitsSurrogatePair")]
pub fn splits_surrogate_pair(text: String, offset_chars: f64) -> napi::Result<bool> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let out = port::splits_surrogate_pair(text, offset_chars)?;
        Ok(out)
    })
}

/// `unicodeSafeEndOffset`.
#[napi(js_name = "unicodeSafeEndOffset")]
pub fn unicode_safe_end_offset(text: String, requested_end_offset_chars: f64) -> napi::Result<f64> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let out = port::unicode_safe_end_offset(text, requested_end_offset_chars)?;
        Ok(out)
    })
}

/// `nonEmptyTrimmed`.
#[napi(js_name = "nonEmptyTrimmed")]
pub fn non_empty_trimmed(value: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::non_empty_trimmed(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `stripTerminalControlSequences`.
#[napi(js_name = "stripTerminalControlSequences")]
pub fn strip_terminal_control_sequences(value: String) -> napi::Result<String> {
    run(move || {
        let out = port::strip_terminal_control_sequences(value)?;
        Ok(out)
    })
}

/// `pluralize`.
#[napi(js_name = "pluralize")]
pub fn pluralize(count: String, singular: String, plural: String) -> napi::Result<String> {
    run(move || {
        let count = crate::js::from_json(&count)?;
        let singular = crate::js::from_json(&singular)?;
        let plural = crate::js::from_json(&plural)?;
        let out = port::pluralize(count, singular, plural)?;
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

/// `normalizeLineEndings` over the binary wire.
#[napi(js_name = "normalizeLineEndingsWire")]
pub fn normalize_line_endings_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let contents: String = r.string16()?;
        let out = &port::normalize_line_endings(contents)?;
        w.string16(out);
        Ok(())
    })
}

static S_SPLITS_SURROGATE_PAIR_0: wire::Schema = wire::Schema::Dyn;

/// `splitsSurrogatePair` over the binary wire.
#[napi(js_name = "splitsSurrogatePairWire")]
pub fn splits_surrogate_pair_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_SPLITS_SURROGATE_PAIR_0)?;
        let offset_chars: f64 = r.f64()?;
        let out = &port::splits_surrogate_pair(text, offset_chars)?;
        w.bool(*out);
        Ok(())
    })
}

static S_UNICODE_SAFE_END_OFFSET_0: wire::Schema = wire::Schema::Dyn;

/// `unicodeSafeEndOffset` over the binary wire.
#[napi(js_name = "unicodeSafeEndOffsetWire")]
pub fn unicode_safe_end_offset_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_UNICODE_SAFE_END_OFFSET_0)?;
        let requested_end_offset_chars: f64 = r.f64()?;
        let out = &port::unicode_safe_end_offset(text, requested_end_offset_chars)?;
        w.f64(*out);
        Ok(())
    })
}

static S_NON_EMPTY_TRIMMED_0: wire::Schema = wire::Schema::Dyn;

/// `nonEmptyTrimmed` over the binary wire.
#[napi(js_name = "nonEmptyTrimmedWire")]
pub fn non_empty_trimmed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NON_EMPTY_TRIMMED_0)?;
        let out = &port::non_empty_trimmed(value)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

/// `stripTerminalControlSequences` over the binary wire.
#[napi(js_name = "stripTerminalControlSequencesWire")]
pub fn strip_terminal_control_sequences_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::strip_terminal_control_sequences(value)?;
        w.string16(out);
        Ok(())
    })
}

static S_PLURALIZE_0: wire::Schema = wire::Schema::Dyn;
static S_PLURALIZE_1: wire::Schema = wire::Schema::Dyn;
static S_PLURALIZE_2: wire::Schema = wire::Schema::Dyn;
static S_PLURALIZE_OUT: wire::Schema = wire::Schema::Str16;

/// `pluralize` over the binary wire.
#[napi(js_name = "pluralizeWire")]
pub fn pluralize_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let count: Value = wire::read_value(r, &S_PLURALIZE_0)?;
        let singular: Value = wire::read_value(r, &S_PLURALIZE_1)?;
        let plural: Value = wire::read_value(r, &S_PLURALIZE_2)?;
        let out = &port::pluralize(count, singular, plural)?;
        wire::write_value(w, out, &S_PLURALIZE_OUT)?;
        Ok(())
    })
}
