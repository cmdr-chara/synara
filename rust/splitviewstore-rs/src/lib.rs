//! JS bindings for `apps/web/src/splitViewStore.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveSplitViewFocusedThreadId`.
#[napi(js_name = "resolveSplitViewFocusedThreadId")]
pub fn resolve_split_view_focused_thread_id(split_view: String) -> napi::Result<String> {
    run(move || {
        let split_view = crate::js::from_json(&split_view)?;
        let out = port::resolve_split_view_focused_thread_id(split_view)?;
        crate::js::to_json(&out)
    })
}

/// `resolveSplitViewFocusedPaneThreadId`.
#[napi(js_name = "resolveSplitViewFocusedPaneThreadId")]
pub fn resolve_split_view_focused_pane_thread_id(split_view: String) -> napi::Result<String> {
    run(move || {
        let split_view = crate::js::from_json(&split_view)?;
        let out = port::resolve_split_view_focused_pane_thread_id(split_view)?;
        crate::js::to_json(&out)
    })
}

/// `resolveSplitViewThreadIds`.
#[napi(js_name = "resolveSplitViewThreadIds")]
pub fn resolve_split_view_thread_ids(split_view: String) -> napi::Result<String> {
    run(move || {
        let split_view = crate::js::from_json(&split_view)?;
        let out = port::resolve_split_view_thread_ids(split_view)?;
        crate::js::to_json(&out)
    })
}

/// `resolveSplitViewPaneIdForThread`.
#[napi(js_name = "resolveSplitViewPaneIdForThread")]
pub fn resolve_split_view_pane_id_for_thread(
    split_view: String,
    thread_id: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let split_view = crate::js::from_json(&split_view)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::resolve_split_view_pane_id_for_thread(split_view, thread_id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `selectSplitView`.
#[napi(js_name = "selectSplitView")]
pub fn select_split_view(split_view_id: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::select_split_view(split_view_id)?;
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

static S_RESOLVE_SPLIT_VIEW_FOCUSED_THREAD_ID_0: wire::Schema = wire::Schema::Dyn;

/// `resolveSplitViewFocusedThreadId` over the binary wire.
#[napi(js_name = "resolveSplitViewFocusedThreadIdWire")]
pub fn resolve_split_view_focused_thread_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let split_view: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_SPLIT_VIEW_FOCUSED_THREAD_ID_0)?;
        let out = &port::resolve_split_view_focused_thread_id(split_view)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_SPLIT_VIEW_FOCUSED_PANE_THREAD_ID_0: wire::Schema = wire::Schema::Dyn;

/// `resolveSplitViewFocusedPaneThreadId` over the binary wire.
#[napi(js_name = "resolveSplitViewFocusedPaneThreadIdWire")]
pub fn resolve_split_view_focused_pane_thread_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let split_view: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_SPLIT_VIEW_FOCUSED_PANE_THREAD_ID_0)?;
        let out = &port::resolve_split_view_focused_pane_thread_id(split_view)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_SPLIT_VIEW_THREAD_IDS_0: wire::Schema = wire::Schema::Dyn;

/// `resolveSplitViewThreadIds` over the binary wire.
#[napi(js_name = "resolveSplitViewThreadIdsWire")]
pub fn resolve_split_view_thread_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let split_view: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_SPLIT_VIEW_THREAD_IDS_0)?;
        let out = &port::resolve_split_view_thread_ids(split_view)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_SPLIT_VIEW_PANE_ID_FOR_THREAD_0: wire::Schema = wire::Schema::Dyn;

/// `resolveSplitViewPaneIdForThread` over the binary wire.
#[napi(js_name = "resolveSplitViewPaneIdForThreadWire")]
pub fn resolve_split_view_pane_id_for_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let split_view: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_SPLIT_VIEW_PANE_ID_FOR_THREAD_0)?;
        let thread_id: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_split_view_pane_id_for_thread(split_view, thread_id)?;
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

static S_SELECT_SPLIT_VIEW_OUT: wire::Schema = wire::Schema::Dyn;

/// `selectSplitView` over the binary wire.
#[napi(js_name = "selectSplitViewWire")]
pub fn select_split_view_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let split_view_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::select_split_view(split_view_id)?;
        wire::write_value(w, out, &S_SELECT_SPLIT_VIEW_OUT)?;
        Ok(())
    })
}
