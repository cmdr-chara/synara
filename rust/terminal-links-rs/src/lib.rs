//! JS bindings for `apps/web/src/terminal-links.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `extractTerminalLinks`.
#[napi(js_name = "extractTerminalLinks")]
pub fn extract_terminal_links(line: String) -> napi::Result<String> {
    run(move || {
        let out = port::extract_terminal_links(line)?;
        crate::js::to_json(&out)
    })
}

/// `resolveWrappedTerminalLinkRange`.
#[napi(js_name = "resolveWrappedTerminalLinkRange")]
pub fn resolve_wrapped_terminal_link_range(
    wrapped_line: String,
    r#match: String,
) -> napi::Result<String> {
    run(move || {
        let wrapped_line = crate::js::from_json(&wrapped_line)?;
        let r#match = crate::js::from_json(&r#match)?;
        let out = port::resolve_wrapped_terminal_link_range(wrapped_line, r#match)?;
        crate::js::to_json(&out)
    })
}

/// `wrappedTerminalLinkRangeIntersectsBufferLine`.
#[napi(js_name = "wrappedTerminalLinkRangeIntersectsBufferLine")]
pub fn wrapped_terminal_link_range_intersects_buffer_line(
    range: String,
    buffer_line_number: f64,
) -> napi::Result<bool> {
    run(move || {
        let range = crate::js::from_json(&range)?;
        let out =
            port::wrapped_terminal_link_range_intersects_buffer_line(range, buffer_line_number)?;
        Ok(out)
    })
}

/// `isTerminalLinkActivation`.
#[napi(js_name = "isTerminalLinkActivation")]
pub fn is_terminal_link_activation(event: String, platform: String) -> napi::Result<String> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::is_terminal_link_activation(event, platform)?;
        crate::js::to_json(&out)
    })
}

/// `resolvePathLinkTarget`.
#[napi(js_name = "resolvePathLinkTarget")]
pub fn resolve_path_link_target(raw_path: String, cwd: String) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_path_link_target(raw_path, cwd)?;
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

static S_EXTRACT_TERMINAL_LINKS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("text", wire::Schema::Str16),
    ("start", wire::Schema::Num),
    ("end", wire::Schema::Num),
]));

/// `extractTerminalLinks` over the binary wire.
#[napi(js_name = "extractTerminalLinksWire")]
pub fn extract_terminal_links_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let out = &port::extract_terminal_links(line)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_EXTRACT_TERMINAL_LINKS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_RESOLVE_WRAPPED_TERMINAL_LINK_RANGE_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_WRAPPED_TERMINAL_LINK_RANGE_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_WRAPPED_TERMINAL_LINK_RANGE_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "start",
        wire::Schema::Obj(&[("x", wire::Schema::Num), ("y", wire::Schema::Num)]),
    ),
    (
        "end",
        wire::Schema::Obj(&[("x", wire::Schema::Num), ("y", wire::Schema::Num)]),
    ),
]);

/// `resolveWrappedTerminalLinkRange` over the binary wire.
#[napi(js_name = "resolveWrappedTerminalLinkRangeWire")]
pub fn resolve_wrapped_terminal_link_range_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let wrapped_line: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_WRAPPED_TERMINAL_LINK_RANGE_0)?;
        let r#match: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_WRAPPED_TERMINAL_LINK_RANGE_1)?;
        let out = &port::resolve_wrapped_terminal_link_range(wrapped_line, r#match)?;
        wire::write_value(w, out, &S_RESOLVE_WRAPPED_TERMINAL_LINK_RANGE_OUT)?;
        Ok(())
    })
}

static S_WRAPPED_TERMINAL_LINK_RANGE_INTERSECTS_BUFFER_LINE_0: wire::Schema = wire::Schema::Dyn;

/// `wrappedTerminalLinkRangeIntersectsBufferLine` over the binary wire.
#[napi(js_name = "wrappedTerminalLinkRangeIntersectsBufferLineWire")]
pub fn wrapped_terminal_link_range_intersects_buffer_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let range: serde_json::Value =
            wire::read_value(r, &S_WRAPPED_TERMINAL_LINK_RANGE_INTERSECTS_BUFFER_LINE_0)?;
        let buffer_line_number: f64 = r.f64()?;
        let out =
            &port::wrapped_terminal_link_range_intersects_buffer_line(range, buffer_line_number)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_TERMINAL_LINK_ACTIVATION_0: wire::Schema = wire::Schema::Dyn;
static S_IS_TERMINAL_LINK_ACTIVATION_1: wire::Schema = wire::Schema::Dyn;
static S_IS_TERMINAL_LINK_ACTIVATION_OUT: wire::Schema = wire::Schema::Bool;

/// `isTerminalLinkActivation` over the binary wire.
#[napi(js_name = "isTerminalLinkActivationWire")]
pub fn is_terminal_link_activation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_IS_TERMINAL_LINK_ACTIVATION_0)?;
        let platform: serde_json::Value = wire::read_value(r, &S_IS_TERMINAL_LINK_ACTIVATION_1)?;
        let out = &port::is_terminal_link_activation(event, platform)?;
        wire::write_value(w, out, &S_IS_TERMINAL_LINK_ACTIVATION_OUT)?;
        Ok(())
    })
}

/// `resolvePathLinkTarget` over the binary wire.
#[napi(js_name = "resolvePathLinkTargetWire")]
pub fn resolve_path_link_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_path: String = r.string16()?;
        let cwd: String = r.string16()?;
        let out = &port::resolve_path_link_target(raw_path, cwd)?;
        w.string16(out);
        Ok(())
    })
}
