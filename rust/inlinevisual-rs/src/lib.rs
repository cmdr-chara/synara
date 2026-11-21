//! JS bindings for `packages/shared/src/inlineVisual.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `clampInlineVisualHeight`.
#[napi(js_name = "clampInlineVisualHeight")]
pub fn clamp_inline_visual_height(height: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::clamp_inline_visual_height(height)?;
        Ok(out)
    })
}

/// `readInlineVisualFrameMessage`.
#[napi(js_name = "readInlineVisualFrameMessage")]
pub fn read_inline_visual_frame_message(data: String) -> napi::Result<String> {
    run(move || {
        let data = crate::js::from_json(&data)?;
        let out = port::read_inline_visual_frame_message(data)?;
        crate::js::to_json(&out)
    })
}

/// `inlineVisualLinkResult`.
#[napi(js_name = "inlineVisualLinkResult")]
pub fn inline_visual_link_result(id: String) -> napi::Result<String> {
    run(move || {
        let id = crate::js::from_json(&id)?;
        let out = port::inline_visual_link_result(id)?;
        crate::js::to_json(&out)
    })
}

/// `inlineVisualThemeMessage`.
#[napi(js_name = "inlineVisualThemeMessage")]
pub fn inline_visual_theme_message(theme: String) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::inline_visual_theme_message(theme)?;
        crate::js::to_json(&out)
    })
}

/// `buildInlineVisualDocument`.
#[napi(js_name = "buildInlineVisualDocument")]
pub fn build_inline_visual_document(html: String, theme: String) -> napi::Result<String> {
    run(move || {
        let theme = crate::js::from_json(&theme)?;
        let out = port::build_inline_visual_document(html, theme)?;
        Ok(out)
    })
}

/// `readInlineVisualEmbedId`.
#[napi(js_name = "readInlineVisualEmbedId")]
pub fn read_inline_visual_embed_id(
    body: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::read_inline_visual_embed_id(body)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `readInlineVisualEmbedIds`.
#[napi(js_name = "readInlineVisualEmbedIds")]
pub fn read_inline_visual_embed_ids(text: String) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::read_inline_visual_embed_ids(text)?;
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

/// `clampInlineVisualHeight` over the binary wire.
#[napi(js_name = "clampInlineVisualHeightWire")]
pub fn clamp_inline_visual_height_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let height: f64 = r.f64()?;
        let out = &port::clamp_inline_visual_height(height)?;
        w.f64(*out);
        Ok(())
    })
}

static S_READ_INLINE_VISUAL_FRAME_MESSAGE_0: wire::Schema = wire::Schema::Dyn;

/// `readInlineVisualFrameMessage` over the binary wire.
#[napi(js_name = "readInlineVisualFrameMessageWire")]
pub fn read_inline_visual_frame_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let data: serde_json::Value = wire::read_value(r, &S_READ_INLINE_VISUAL_FRAME_MESSAGE_0)?;
        let out = &port::read_inline_visual_frame_message(data)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_INLINE_VISUAL_LINK_RESULT_0: wire::Schema = wire::Schema::Dyn;
static S_INLINE_VISUAL_LINK_RESULT_OUT: wire::Schema = wire::Schema::Dyn;

/// `inlineVisualLinkResult` over the binary wire.
#[napi(js_name = "inlineVisualLinkResultWire")]
pub fn inline_visual_link_result_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: serde_json::Value = wire::read_value(r, &S_INLINE_VISUAL_LINK_RESULT_0)?;
        let out = &port::inline_visual_link_result(id)?;
        wire::write_value(w, out, &S_INLINE_VISUAL_LINK_RESULT_OUT)?;
        Ok(())
    })
}

static S_INLINE_VISUAL_THEME_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_INLINE_VISUAL_THEME_MESSAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `inlineVisualThemeMessage` over the binary wire.
#[napi(js_name = "inlineVisualThemeMessageWire")]
pub fn inline_visual_theme_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let theme: serde_json::Value = wire::read_value(r, &S_INLINE_VISUAL_THEME_MESSAGE_0)?;
        let out = &port::inline_visual_theme_message(theme)?;
        wire::write_value(w, out, &S_INLINE_VISUAL_THEME_MESSAGE_OUT)?;
        Ok(())
    })
}

static S_BUILD_INLINE_VISUAL_DOCUMENT_1: wire::Schema = wire::Schema::Dyn;

/// `buildInlineVisualDocument` over the binary wire.
#[napi(js_name = "buildInlineVisualDocumentWire")]
pub fn build_inline_visual_document_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: String = r.string16()?;
        let theme: serde_json::Value = wire::read_value(r, &S_BUILD_INLINE_VISUAL_DOCUMENT_1)?;
        let out = &port::build_inline_visual_document(html, theme)?;
        w.string16(out);
        Ok(())
    })
}

/// `readInlineVisualEmbedId` over the binary wire.
#[napi(js_name = "readInlineVisualEmbedIdWire")]
pub fn read_inline_visual_embed_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let body: String = r.string16()?;
        let out = &port::read_inline_visual_embed_id(body)?;
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

/// `readInlineVisualEmbedIds` over the binary wire.
#[napi(js_name = "readInlineVisualEmbedIdsWire")]
pub fn read_inline_visual_embed_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::read_inline_visual_embed_ids(text)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}
