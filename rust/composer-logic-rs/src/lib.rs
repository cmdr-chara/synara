//! JS bindings for `apps/web/src/composer-logic.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `stripComposerTriggerText`.
#[napi(js_name = "stripComposerTriggerText")]
pub fn strip_composer_trigger_text(text: String, trigger: String) -> napi::Result<String> {
    run(move || {
        let trigger = crate::js::from_json(&trigger)?;
        let out = port::strip_composer_trigger_text(text, trigger)?;
        crate::js::to_json(&out)
    })
}

/// `expandCollapsedComposerCursor`.
#[napi(js_name = "expandCollapsedComposerCursor")]
pub fn expand_collapsed_composer_cursor(text: String, cursor_input: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::expand_collapsed_composer_cursor(text, cursor_input)?;
        Ok(out)
    })
}

/// `clampCollapsedComposerCursor`.
#[napi(js_name = "clampCollapsedComposerCursor")]
pub fn clamp_collapsed_composer_cursor(text: String, cursor_input: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::clamp_collapsed_composer_cursor(text, cursor_input)?;
        Ok(out)
    })
}

/// `collapseExpandedComposerCursor`.
#[napi(js_name = "collapseExpandedComposerCursor")]
pub fn collapse_expanded_composer_cursor(text: String, cursor_input: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::collapse_expanded_composer_cursor(text, cursor_input)?;
        Ok(out)
    })
}

/// `isCollapsedCursorAdjacentToInlineToken`.
#[napi(js_name = "isCollapsedCursorAdjacentToInlineToken")]
pub fn is_collapsed_cursor_adjacent_to_inline_token(
    text: String,
    cursor_input: f64,
    direction: String,
) -> napi::Result<bool> {
    run(move || {
        let out =
            port::is_collapsed_cursor_adjacent_to_inline_token(text, cursor_input, direction)?;
        Ok(out)
    })
}

/// `detectComposerTrigger`.
#[napi(js_name = "detectComposerTrigger")]
pub fn detect_composer_trigger(text: String, cursor_input: f64) -> napi::Result<String> {
    run(move || {
        let out = port::detect_composer_trigger(text, cursor_input)?;
        crate::js::to_json(&out)
    })
}

/// `parseStandaloneComposerSlashCommand`.
#[napi(js_name = "parseStandaloneComposerSlashCommand")]
pub fn parse_standalone_composer_slash_command(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_standalone_composer_slash_command(text)?;
        crate::js::to_json(&out)
    })
}

/// `replaceTextRange`.
#[napi(js_name = "replaceTextRange")]
pub fn replace_text_range(
    text: String,
    range_start: f64,
    range_end: f64,
    replacement: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::replace_text_range(text, range_start, range_end, replacement)?;
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

static S_STRIP_COMPOSER_TRIGGER_TEXT_OUT: wire::Schema = wire::Schema::Str16;

/// `stripComposerTriggerText` over the binary wire.
#[napi(js_name = "stripComposerTriggerTextWire")]
pub fn strip_composer_trigger_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let trigger: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::strip_composer_trigger_text(text, trigger)?;
        wire::write_value(w, out, &S_STRIP_COMPOSER_TRIGGER_TEXT_OUT)?;
        Ok(())
    })
}

/// `expandCollapsedComposerCursor` over the binary wire.
#[napi(js_name = "expandCollapsedComposerCursorWire")]
pub fn expand_collapsed_composer_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let cursor_input: f64 = r.f64()?;
        let out = &port::expand_collapsed_composer_cursor(text, cursor_input)?;
        w.f64(*out);
        Ok(())
    })
}

/// `clampCollapsedComposerCursor` over the binary wire.
#[napi(js_name = "clampCollapsedComposerCursorWire")]
pub fn clamp_collapsed_composer_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let cursor_input: f64 = r.f64()?;
        let out = &port::clamp_collapsed_composer_cursor(text, cursor_input)?;
        w.f64(*out);
        Ok(())
    })
}

/// `collapseExpandedComposerCursor` over the binary wire.
#[napi(js_name = "collapseExpandedComposerCursorWire")]
pub fn collapse_expanded_composer_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let cursor_input: f64 = r.f64()?;
        let out = &port::collapse_expanded_composer_cursor(text, cursor_input)?;
        w.f64(*out);
        Ok(())
    })
}

/// `isCollapsedCursorAdjacentToInlineToken` over the binary wire.
#[napi(js_name = "isCollapsedCursorAdjacentToInlineTokenWire")]
pub fn is_collapsed_cursor_adjacent_to_inline_token_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let cursor_input: f64 = r.f64()?;
        let direction: String = r.string16()?;
        let out =
            &port::is_collapsed_cursor_adjacent_to_inline_token(text, cursor_input, direction)?;
        w.bool(*out);
        Ok(())
    })
}

static S_DETECT_COMPOSER_TRIGGER_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("query", wire::Schema::Str16),
    ("rangeStart", wire::Schema::Num),
    ("rangeEnd", wire::Schema::Num),
]));

/// `detectComposerTrigger` over the binary wire.
#[napi(js_name = "detectComposerTriggerWire")]
pub fn detect_composer_trigger_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let cursor_input: f64 = r.f64()?;
        let out = &port::detect_composer_trigger(text, cursor_input)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_DETECT_COMPOSER_TRIGGER_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `parseStandaloneComposerSlashCommand` over the binary wire.
#[napi(js_name = "parseStandaloneComposerSlashCommandWire")]
pub fn parse_standalone_composer_slash_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::parse_standalone_composer_slash_command(text)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_REPLACE_TEXT_RANGE_OUT: wire::Schema =
    wire::Schema::Obj(&[("text", wire::Schema::Str16), ("cursor", wire::Schema::Num)]);

/// `replaceTextRange` over the binary wire.
#[napi(js_name = "replaceTextRangeWire")]
pub fn replace_text_range_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let range_start: f64 = r.f64()?;
        let range_end: f64 = r.f64()?;
        let replacement: String = r.string16()?;
        let out = &port::replace_text_range(text, range_start, range_end, replacement)?;
        wire::write_value(w, out, &S_REPLACE_TEXT_RANGE_OUT)?;
        Ok(())
    })
}
