//! JS bindings for `apps/web/src/threadDetailResumeCursors.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `advanceThreadDetailResumeCursor`.
#[napi(js_name = "advanceThreadDetailResumeCursor")]
pub fn advance_thread_detail_resume_cursor(thread_id: String, sequence: f64) -> napi::Result<()> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        port::advance_thread_detail_resume_cursor(thread_id, sequence)?;
        Ok(())
    })
}

/// `setThreadDetailResumeCursor`.
#[napi(js_name = "setThreadDetailResumeCursor")]
pub fn set_thread_detail_resume_cursor(thread_id: String, sequence: f64) -> napi::Result<()> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        port::set_thread_detail_resume_cursor(thread_id, sequence)?;
        Ok(())
    })
}

/// `getThreadDetailResumeCursor`.
#[napi(js_name = "getThreadDetailResumeCursor")]
pub fn get_thread_detail_resume_cursor(
    thread_id: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::get_thread_detail_resume_cursor(thread_id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `hasThreadDetailResumeCursor`.
#[napi(js_name = "hasThreadDetailResumeCursor")]
pub fn has_thread_detail_resume_cursor(thread_id: String) -> napi::Result<bool> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::has_thread_detail_resume_cursor(thread_id)?;
        Ok(out)
    })
}

/// `clearThreadDetailResumeCursor`.
#[napi(js_name = "clearThreadDetailResumeCursor")]
pub fn clear_thread_detail_resume_cursor(thread_id: String) -> napi::Result<()> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        port::clear_thread_detail_resume_cursor(thread_id)?;
        Ok(())
    })
}

/// `retainThreadDetailResumeCursors`.
#[napi(js_name = "retainThreadDetailResumeCursors")]
pub fn retain_thread_detail_resume_cursors(thread_ids: String) -> napi::Result<()> {
    run(move || {
        let thread_ids = crate::js::from_json(&thread_ids)?;
        port::retain_thread_detail_resume_cursors(thread_ids)?;
        Ok(())
    })
}

/// `resetThreadDetailResumeCursors`.
#[napi(js_name = "resetThreadDetailResumeCursors")]
pub fn reset_thread_detail_resume_cursors() -> napi::Result<()> {
    run(move || {
        port::reset_thread_detail_resume_cursors()?;
        Ok(())
    })
}

/// `buildThreadSubscribeInput`.
#[napi(js_name = "buildThreadSubscribeInput")]
pub fn build_thread_subscribe_input(thread_id: String) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::build_thread_subscribe_input(thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `resetThreadDetailResumeCursorsForTests`.
#[napi(js_name = "resetThreadDetailResumeCursorsForTests")]
pub fn reset_thread_detail_resume_cursors_for_tests() -> napi::Result<()> {
    run(move || {
        port::reset_thread_detail_resume_cursors_for_tests()?;
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

static S_ADVANCE_THREAD_DETAIL_RESUME_CURSOR_0: wire::Schema = wire::Schema::Dyn;

/// `advanceThreadDetailResumeCursor` over the binary wire.
#[napi(js_name = "advanceThreadDetailResumeCursorWire")]
pub fn advance_thread_detail_resume_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_ADVANCE_THREAD_DETAIL_RESUME_CURSOR_0)?;
        let sequence: f64 = r.f64()?;
        let () = port::advance_thread_detail_resume_cursor(thread_id, sequence)?;
        Ok(())
    })
}

static S_SET_THREAD_DETAIL_RESUME_CURSOR_0: wire::Schema = wire::Schema::Dyn;

/// `setThreadDetailResumeCursor` over the binary wire.
#[napi(js_name = "setThreadDetailResumeCursorWire")]
pub fn set_thread_detail_resume_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_SET_THREAD_DETAIL_RESUME_CURSOR_0)?;
        let sequence: f64 = r.f64()?;
        let () = port::set_thread_detail_resume_cursor(thread_id, sequence)?;
        Ok(())
    })
}

static S_GET_THREAD_DETAIL_RESUME_CURSOR_0: wire::Schema = wire::Schema::Dyn;

/// `getThreadDetailResumeCursor` over the binary wire.
#[napi(js_name = "getThreadDetailResumeCursorWire")]
pub fn get_thread_detail_resume_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_GET_THREAD_DETAIL_RESUME_CURSOR_0)?;
        let out = &port::get_thread_detail_resume_cursor(thread_id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_HAS_THREAD_DETAIL_RESUME_CURSOR_0: wire::Schema = wire::Schema::Dyn;

/// `hasThreadDetailResumeCursor` over the binary wire.
#[napi(js_name = "hasThreadDetailResumeCursorWire")]
pub fn has_thread_detail_resume_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_HAS_THREAD_DETAIL_RESUME_CURSOR_0)?;
        let out = &port::has_thread_detail_resume_cursor(thread_id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CLEAR_THREAD_DETAIL_RESUME_CURSOR_0: wire::Schema = wire::Schema::Dyn;

/// `clearThreadDetailResumeCursor` over the binary wire.
#[napi(js_name = "clearThreadDetailResumeCursorWire")]
pub fn clear_thread_detail_resume_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_CLEAR_THREAD_DETAIL_RESUME_CURSOR_0)?;
        let () = port::clear_thread_detail_resume_cursor(thread_id)?;
        Ok(())
    })
}

static S_RETAIN_THREAD_DETAIL_RESUME_CURSORS_0: wire::Schema = wire::Schema::Dyn;

/// `retainThreadDetailResumeCursors` over the binary wire.
#[napi(js_name = "retainThreadDetailResumeCursorsWire")]
pub fn retain_thread_detail_resume_cursors_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let thread_ids: serde_json::Value =
            wire::read_value(r, &S_RETAIN_THREAD_DETAIL_RESUME_CURSORS_0)?;
        let () = port::retain_thread_detail_resume_cursors(thread_ids)?;
        Ok(())
    })
}

/// `resetThreadDetailResumeCursors` over the binary wire.
#[napi(js_name = "resetThreadDetailResumeCursorsWire")]
pub fn reset_thread_detail_resume_cursors_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::reset_thread_detail_resume_cursors()?;
        Ok(())
    })
}

static S_BUILD_THREAD_SUBSCRIBE_INPUT_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_THREAD_SUBSCRIBE_INPUT_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildThreadSubscribeInput` over the binary wire.
#[napi(js_name = "buildThreadSubscribeInputWire")]
pub fn build_thread_subscribe_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value = wire::read_value(r, &S_BUILD_THREAD_SUBSCRIBE_INPUT_0)?;
        let out = &port::build_thread_subscribe_input(thread_id)?;
        wire::write_value(w, out, &S_BUILD_THREAD_SUBSCRIBE_INPUT_OUT)?;
        Ok(())
    })
}

/// `resetThreadDetailResumeCursorsForTests` over the binary wire.
#[napi(js_name = "resetThreadDetailResumeCursorsForTestsWire")]
pub fn reset_thread_detail_resume_cursors_for_tests_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::reset_thread_detail_resume_cursors_for_tests()?;
        Ok(())
    })
}
