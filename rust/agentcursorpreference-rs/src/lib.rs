//! JS bindings for `apps/desktop/src/agentCursorPreference.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `normalizeAgentCursorStylePreference`.
#[napi(js_name = "normalizeAgentCursorStylePreference")]
pub fn normalize_agent_cursor_style_preference(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_agent_cursor_style_preference(value)?;
        crate::js::to_json(&out)
    })
}

/// `parseAgentCursorPreference`.
#[napi(js_name = "parseAgentCursorPreference")]
pub fn parse_agent_cursor_preference(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::parse_agent_cursor_preference(value)?;
        crate::js::to_json(&out)
    })
}

/// `readAgentCursorPreference`.
#[napi(js_name = "readAgentCursorPreference")]
pub fn read_agent_cursor_preference(file_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::read_agent_cursor_preference(file_path)?;
        crate::js::to_json(&out)
    })
}

/// `writeAgentCursorPreference`.
#[napi(js_name = "writeAgentCursorPreference")]
pub fn write_agent_cursor_preference(file_path: String, style: String) -> napi::Result<()> {
    run(move || {
        let style = crate::js::from_json(&style)?;
        port::write_agent_cursor_preference(file_path, style)?;
        Ok(())
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

static S_NORMALIZE_AGENT_CURSOR_STYLE_PREFERENCE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_AGENT_CURSOR_STYLE_PREFERENCE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("fill", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("rim", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("shadow", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `normalizeAgentCursorStylePreference` over the binary wire.
#[napi(js_name = "normalizeAgentCursorStylePreferenceWire")]
pub fn normalize_agent_cursor_style_preference_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_AGENT_CURSOR_STYLE_PREFERENCE_0)?;
        let out = &port::normalize_agent_cursor_style_preference(value)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_NORMALIZE_AGENT_CURSOR_STYLE_PREFERENCE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_PARSE_AGENT_CURSOR_PREFERENCE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_AGENT_CURSOR_PREFERENCE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("version", wire::Schema::Num),
        (
            "style",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("fill", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("rim", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("shadow", wire::Schema::OptJs(&wire::Schema::Str16)),
            ])),
        ),
    ]));

/// `parseAgentCursorPreference` over the binary wire.
#[napi(js_name = "parseAgentCursorPreferenceWire")]
pub fn parse_agent_cursor_preference_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_PARSE_AGENT_CURSOR_PREFERENCE_0)?;
        let out = &port::parse_agent_cursor_preference(value)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PARSE_AGENT_CURSOR_PREFERENCE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_READ_AGENT_CURSOR_PREFERENCE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("fill", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("rim", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("shadow", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `readAgentCursorPreference` over the binary wire.
#[napi(js_name = "readAgentCursorPreferenceWire")]
pub fn read_agent_cursor_preference_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let out = &port::read_agent_cursor_preference(file_path)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_READ_AGENT_CURSOR_PREFERENCE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `writeAgentCursorPreference` over the binary wire.
#[napi(js_name = "writeAgentCursorPreferenceWire")]
pub fn write_agent_cursor_preference_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let file_path: String = r.string16()?;
        let style: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let () = port::write_agent_cursor_preference(file_path, style)?;
        Ok(())
    })
}
