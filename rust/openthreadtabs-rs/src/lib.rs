//! JS bindings for `apps/web/src/openThreadTabs.logic.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `addOpenThreadTab`.
#[napi(js_name = "addOpenThreadTab")]
pub fn add_open_thread_tab(thread_ids: String, thread_id: String) -> napi::Result<String> {
    run(move || {
        let thread_ids = crate::js::from_json(&thread_ids)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::add_open_thread_tab(thread_ids, thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `removeOpenThreadTab`.
#[napi(js_name = "removeOpenThreadTab")]
pub fn remove_open_thread_tab(thread_ids: String, thread_id: String) -> napi::Result<String> {
    run(move || {
        let thread_ids = crate::js::from_json(&thread_ids)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::remove_open_thread_tab(thread_ids, thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `moveOpenThreadTab`.
#[napi(js_name = "moveOpenThreadTab")]
pub fn move_open_thread_tab(
    thread_ids: String,
    thread_id: String,
    over_thread_id: String,
) -> napi::Result<String> {
    run(move || {
        let thread_ids = crate::js::from_json(&thread_ids)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let over_thread_id = crate::js::from_json(&over_thread_id)?;
        let out = port::move_open_thread_tab(thread_ids, thread_id, over_thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeOpenThreadTabIds`.
#[napi(js_name = "normalizeOpenThreadTabIds")]
pub fn normalize_open_thread_tab_ids(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::normalize_open_thread_tab_ids(input)?;
        crate::js::to_json(&out)
    })
}

/// `canKeepOpenThreadTab`.
#[napi(js_name = "canKeepOpenThreadTab")]
pub fn can_keep_open_thread_tab(summary: String, has_draft: bool) -> napi::Result<bool> {
    run(move || {
        let summary = crate::js::from_json(&summary)?;
        let out = port::can_keep_open_thread_tab(summary, has_draft)?;
        Ok(out)
    })
}

/// `buildOpenThreadTabs`.
#[napi(js_name = "buildOpenThreadTabs")]
pub fn build_open_thread_tabs(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_open_thread_tabs(input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveOpenThreadTabCloseTarget`.
#[napi(js_name = "resolveOpenThreadTabCloseTarget")]
pub fn resolve_open_thread_tab_close_target(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_open_thread_tab_close_target(input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveOpenThreadTabsInCloseScope`.
#[napi(js_name = "resolveOpenThreadTabsInCloseScope")]
pub fn resolve_open_thread_tabs_in_close_scope(
    tabs: String,
    anchor_thread_id: String,
    scope: String,
) -> napi::Result<String> {
    run(move || {
        let tabs = crate::js::from_json(&tabs)?;
        let anchor_thread_id = crate::js::from_json(&anchor_thread_id)?;
        let out = port::resolve_open_thread_tabs_in_close_scope(tabs, anchor_thread_id, scope)?;
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

static S_ADD_OPEN_THREAD_TAB_0: wire::Schema = wire::Schema::Dyn;
static S_ADD_OPEN_THREAD_TAB_1: wire::Schema = wire::Schema::Dyn;
static S_ADD_OPEN_THREAD_TAB_OUT: wire::Schema = wire::Schema::Dyn;

/// `addOpenThreadTab` over the binary wire.
#[napi(js_name = "addOpenThreadTabWire")]
pub fn add_open_thread_tab_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_ids: serde_json::Value = wire::read_value(r, &S_ADD_OPEN_THREAD_TAB_0)?;
        let thread_id: serde_json::Value = wire::read_value(r, &S_ADD_OPEN_THREAD_TAB_1)?;
        let out = &port::add_open_thread_tab(thread_ids, thread_id)?;
        wire::write_value(w, out, &S_ADD_OPEN_THREAD_TAB_OUT)?;
        Ok(())
    })
}

static S_REMOVE_OPEN_THREAD_TAB_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_OPEN_THREAD_TAB_1: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_OPEN_THREAD_TAB_OUT: wire::Schema = wire::Schema::Dyn;

/// `removeOpenThreadTab` over the binary wire.
#[napi(js_name = "removeOpenThreadTabWire")]
pub fn remove_open_thread_tab_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_ids: serde_json::Value = wire::read_value(r, &S_REMOVE_OPEN_THREAD_TAB_0)?;
        let thread_id: serde_json::Value = wire::read_value(r, &S_REMOVE_OPEN_THREAD_TAB_1)?;
        let out = &port::remove_open_thread_tab(thread_ids, thread_id)?;
        wire::write_value(w, out, &S_REMOVE_OPEN_THREAD_TAB_OUT)?;
        Ok(())
    })
}

static S_MOVE_OPEN_THREAD_TAB_0: wire::Schema = wire::Schema::Dyn;
static S_MOVE_OPEN_THREAD_TAB_1: wire::Schema = wire::Schema::Dyn;
static S_MOVE_OPEN_THREAD_TAB_2: wire::Schema = wire::Schema::Dyn;
static S_MOVE_OPEN_THREAD_TAB_OUT: wire::Schema = wire::Schema::Dyn;

/// `moveOpenThreadTab` over the binary wire.
#[napi(js_name = "moveOpenThreadTabWire")]
pub fn move_open_thread_tab_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_ids: serde_json::Value = wire::read_value(r, &S_MOVE_OPEN_THREAD_TAB_0)?;
        let thread_id: serde_json::Value = wire::read_value(r, &S_MOVE_OPEN_THREAD_TAB_1)?;
        let over_thread_id: serde_json::Value = wire::read_value(r, &S_MOVE_OPEN_THREAD_TAB_2)?;
        let out = &port::move_open_thread_tab(thread_ids, thread_id, over_thread_id)?;
        wire::write_value(w, out, &S_MOVE_OPEN_THREAD_TAB_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_OPEN_THREAD_TAB_IDS_0: wire::Schema = wire::Schema::Dyn;

/// `normalizeOpenThreadTabIds` over the binary wire.
#[napi(js_name = "normalizeOpenThreadTabIdsWire")]
pub fn normalize_open_thread_tab_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_NORMALIZE_OPEN_THREAD_TAB_IDS_0)?;
        let out = &port::normalize_open_thread_tab_ids(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `canKeepOpenThreadTab` over the binary wire.
#[napi(js_name = "canKeepOpenThreadTabWire")]
pub fn can_keep_open_thread_tab_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let summary: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let has_draft: bool = r.bool()?;
        let out = &port::can_keep_open_thread_tab(summary, has_draft)?;
        w.bool(*out);
        Ok(())
    })
}

static S_BUILD_OPEN_THREAD_TABS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_OPEN_THREAD_TABS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("threadId", wire::Schema::Dyn),
    ("projectId", wire::Schema::Dyn),
    ("title", wire::Schema::Str16),
    ("provider", wire::Schema::Dyn),
    ("isTerminal", wire::Schema::Bool),
    ("isRunning", wire::Schema::Bool),
]));

/// `buildOpenThreadTabs` over the binary wire.
#[napi(js_name = "buildOpenThreadTabsWire")]
pub fn build_open_thread_tabs_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_OPEN_THREAD_TABS_0)?;
        let out = &port::build_open_thread_tabs(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_BUILD_OPEN_THREAD_TABS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_RESOLVE_OPEN_THREAD_TAB_CLOSE_TARGET_0: wire::Schema = wire::Schema::Dyn;

/// `resolveOpenThreadTabCloseTarget` over the binary wire.
#[napi(js_name = "resolveOpenThreadTabCloseTargetWire")]
pub fn resolve_open_thread_tab_close_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_OPEN_THREAD_TAB_CLOSE_TARGET_0)?;
        let out = &port::resolve_open_thread_tab_close_target(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_OPEN_THREAD_TABS_IN_CLOSE_SCOPE_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_OPEN_THREAD_TABS_IN_CLOSE_SCOPE_1: wire::Schema = wire::Schema::Dyn;

/// `resolveOpenThreadTabsInCloseScope` over the binary wire.
#[napi(js_name = "resolveOpenThreadTabsInCloseScopeWire")]
pub fn resolve_open_thread_tabs_in_close_scope_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tabs: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_OPEN_THREAD_TABS_IN_CLOSE_SCOPE_0)?;
        let anchor_thread_id: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_OPEN_THREAD_TABS_IN_CLOSE_SCOPE_1)?;
        let scope: String = r.string16()?;
        let out = &port::resolve_open_thread_tabs_in_close_scope(tabs, anchor_thread_id, scope)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
