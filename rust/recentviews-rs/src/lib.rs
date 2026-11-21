//! JS bindings for `apps/web/src/recentViews.logic.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `recentViewKey`.
#[napi(js_name = "recentViewKey")]
pub fn recent_view_key(view: String) -> napi::Result<String> {
    run(move || {
        let view = crate::js::from_json(&view)?;
        let out = port::recent_view_key(view)?;
        crate::js::to_json(&out)
    })
}

/// `deriveCurrentRecentView`.
#[napi(js_name = "deriveCurrentRecentView")]
pub fn derive_current_recent_view(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_current_recent_view(input)?;
        crate::js::to_json(&out)
    })
}

/// `upsertRecentView`.
#[napi(js_name = "upsertRecentView")]
pub fn upsert_recent_view(
    recent_views: String,
    view: String,
    limit: String,
) -> napi::Result<String> {
    run(move || {
        let recent_views = crate::js::from_json(&recent_views)?;
        let view = crate::js::from_json(&view)?;
        let limit = crate::js::from_json(&limit)?;
        let out = port::upsert_recent_view(recent_views, view, limit)?;
        crate::js::to_json(&out)
    })
}

/// `pruneRecentViews`.
#[napi(js_name = "pruneRecentViews")]
pub fn prune_recent_views(
    recent_views: String,
    availability: String,
    limit: String,
) -> napi::Result<String> {
    run(move || {
        let recent_views = crate::js::from_json(&recent_views)?;
        let availability = crate::js::from_json(&availability)?;
        let limit = crate::js::from_json(&limit)?;
        let out = port::prune_recent_views(recent_views, availability, limit)?;
        crate::js::to_json(&out)
    })
}

/// `resolveRecentViewNavigationIndex`.
#[napi(js_name = "resolveRecentViewNavigationIndex")]
pub fn resolve_recent_view_navigation_index(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_recent_view_navigation_index(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildRecentViewDisplayEntries`.
#[napi(js_name = "buildRecentViewDisplayEntries")]
pub fn build_recent_view_display_entries(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_recent_view_display_entries(input)?;
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

static S_RECENT_VIEW_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_RECENT_VIEW_KEY_OUT: wire::Schema = wire::Schema::Str16;

/// `recentViewKey` over the binary wire.
#[napi(js_name = "recentViewKeyWire")]
pub fn recent_view_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let view: serde_json::Value = wire::read_value(r, &S_RECENT_VIEW_KEY_0)?;
        let out = &port::recent_view_key(view)?;
        wire::write_value(w, out, &S_RECENT_VIEW_KEY_OUT)?;
        Ok(())
    })
}

static S_DERIVE_CURRENT_RECENT_VIEW_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_CURRENT_RECENT_VIEW_OUT: wire::Schema = wire::Schema::Dyn;

/// `deriveCurrentRecentView` over the binary wire.
#[napi(js_name = "deriveCurrentRecentViewWire")]
pub fn derive_current_recent_view_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DERIVE_CURRENT_RECENT_VIEW_0)?;
        let out = &port::derive_current_recent_view(input)?;
        wire::write_value(w, out, &S_DERIVE_CURRENT_RECENT_VIEW_OUT)?;
        Ok(())
    })
}

static S_UPSERT_RECENT_VIEW_0: wire::Schema = wire::Schema::Dyn;
static S_UPSERT_RECENT_VIEW_1: wire::Schema = wire::Schema::Dyn;
static S_UPSERT_RECENT_VIEW_2: wire::Schema = wire::Schema::Dyn;

/// `upsertRecentView` over the binary wire.
#[napi(js_name = "upsertRecentViewWire")]
pub fn upsert_recent_view_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let recent_views: serde_json::Value = wire::read_value(r, &S_UPSERT_RECENT_VIEW_0)?;
        let view: serde_json::Value = wire::read_value(r, &S_UPSERT_RECENT_VIEW_1)?;
        let limit: serde_json::Value = wire::read_value(r, &S_UPSERT_RECENT_VIEW_2)?;
        let out = &port::upsert_recent_view(recent_views, view, limit)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_PRUNE_RECENT_VIEWS_0: wire::Schema = wire::Schema::Dyn;
static S_PRUNE_RECENT_VIEWS_1: wire::Schema = wire::Schema::Dyn;
static S_PRUNE_RECENT_VIEWS_2: wire::Schema = wire::Schema::Dyn;

/// `pruneRecentViews` over the binary wire.
#[napi(js_name = "pruneRecentViewsWire")]
pub fn prune_recent_views_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let recent_views: serde_json::Value = wire::read_value(r, &S_PRUNE_RECENT_VIEWS_0)?;
        let availability: serde_json::Value = wire::read_value(r, &S_PRUNE_RECENT_VIEWS_1)?;
        let limit: serde_json::Value = wire::read_value(r, &S_PRUNE_RECENT_VIEWS_2)?;
        let out = &port::prune_recent_views(recent_views, availability, limit)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_RECENT_VIEW_NAVIGATION_INDEX_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_RECENT_VIEW_NAVIGATION_INDEX_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Num);

/// `resolveRecentViewNavigationIndex` over the binary wire.
#[napi(js_name = "resolveRecentViewNavigationIndexWire")]
pub fn resolve_recent_view_navigation_index_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_RECENT_VIEW_NAVIGATION_INDEX_0)?;
        let out = &port::resolve_recent_view_navigation_index(input)?;
        wire::write_value(w, out, &S_RESOLVE_RECENT_VIEW_NAVIGATION_INDEX_OUT)?;
        Ok(())
    })
}

static S_BUILD_RECENT_VIEW_DISPLAY_ENTRIES_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_RECENT_VIEW_DISPLAY_ENTRIES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("key", wire::Schema::Str16),
        ("view", wire::Schema::Dyn),
        ("kind", wire::Schema::Dyn),
        ("icon", wire::Schema::Dyn),
        ("title", wire::Schema::Str16),
        ("subtitle", wire::Schema::Str16),
        ("isCurrent", wire::Schema::Bool),
        ("isPinned", wire::Schema::Bool),
        ("isSplit", wire::Schema::Bool),
        ("isTerminal", wire::Schema::Bool),
        ("provider", wire::Schema::Dyn),
        (
            "terminalVisualIdentity",
            wire::Schema::OptJs(&wire::Schema::Obj(&[("state", wire::Schema::Dyn)])),
        ),
    ]));

/// `buildRecentViewDisplayEntries` over the binary wire.
#[napi(js_name = "buildRecentViewDisplayEntriesWire")]
pub fn build_recent_view_display_entries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_RECENT_VIEW_DISPLAY_ENTRIES_0)?;
        let out = &port::build_recent_view_display_entries(input)?;
        wire::write_value(w, out, &S_BUILD_RECENT_VIEW_DISPLAY_ENTRIES_OUT)?;
        Ok(())
    })
}
