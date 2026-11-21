//! JS bindings for `apps/web/src/pinning.logic.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `normalizePinnedIds`.
#[napi(js_name = "normalizePinnedIds")]
pub fn normalize_pinned_ids(ids: Vec<String>, options: String) -> napi::Result<Vec<String>> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::normalize_pinned_ids(ids, options)?;
        Ok(out)
    })
}

/// `pinId`.
#[napi(js_name = "pinId")]
pub fn pin_id(ids: Vec<String>, id: String, options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::pin_id(ids, id, options)?;
        crate::js::to_json(&out)
    })
}

/// `unpinId`.
#[napi(js_name = "unpinId")]
pub fn unpin_id(ids: Vec<String>, id: String) -> napi::Result<String> {
    run(move || {
        let out = port::unpin_id(ids, id)?;
        crate::js::to_json(&out)
    })
}

/// `prunePinnedIds`.
#[napi(js_name = "prunePinnedIds")]
pub fn prune_pinned_ids(ids: Vec<String>, allowed_ids: Vec<String>) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::prune_pinned_ids(ids, allowed_ids)?;
        Ok(out)
    })
}

/// `derivePinnedIds`.
#[napi(js_name = "derivePinnedIds")]
pub fn derive_pinned_ids(input: String) -> napi::Result<Vec<String>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_pinned_ids(input)?;
        Ok(out)
    })
}

/// `getPinnedItems`.
#[napi(js_name = "getPinnedItems")]
pub fn get_pinned_items(items: String, pinned_ids: Vec<String>) -> napi::Result<String> {
    run(move || {
        let items = crate::js::from_json(&items)?;
        let out = port::get_pinned_items(items, pinned_ids)?;
        crate::js::to_json(&out)
    })
}

/// `orderPinnedItemsFirst`.
#[napi(js_name = "orderPinnedItemsFirst")]
pub fn order_pinned_items_first(items: String, pinned_ids: Vec<String>) -> napi::Result<String> {
    run(move || {
        let items = crate::js::from_json(&items)?;
        let out = port::order_pinned_items_first(items, pinned_ids)?;
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

/// `normalizePinnedIds` over the binary wire.
#[napi(js_name = "normalizePinnedIdsWire")]
pub fn normalize_pinned_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ids: Vec<String> = r.list(|r| r.string16())?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::normalize_pinned_ids(ids, options)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_PIN_ID_OUT: wire::Schema = wire::Schema::Obj(&[
    ("pinnedIds", wire::Schema::List(&wire::Schema::Dyn)),
    ("changed", wire::Schema::Bool),
    ("rejected", wire::Schema::Bool),
]);

/// `pinId` over the binary wire.
#[napi(js_name = "pinIdWire")]
pub fn pin_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ids: Vec<String> = r.list(|r| r.string16())?;
        let id: String = r.string16()?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::pin_id(ids, id, options)?;
        wire::write_value(w, out, &S_PIN_ID_OUT)?;
        Ok(())
    })
}

static S_UNPIN_ID_OUT: wire::Schema = wire::Schema::Obj(&[
    ("pinnedIds", wire::Schema::List(&wire::Schema::Dyn)),
    ("changed", wire::Schema::Bool),
    ("rejected", wire::Schema::Bool),
]);

/// `unpinId` over the binary wire.
#[napi(js_name = "unpinIdWire")]
pub fn unpin_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ids: Vec<String> = r.list(|r| r.string16())?;
        let id: String = r.string16()?;
        let out = &port::unpin_id(ids, id)?;
        wire::write_value(w, out, &S_UNPIN_ID_OUT)?;
        Ok(())
    })
}

/// `prunePinnedIds` over the binary wire.
#[napi(js_name = "prunePinnedIdsWire")]
pub fn prune_pinned_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ids: Vec<String> = r.list(|r| r.string16())?;
        let allowed_ids: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::prune_pinned_ids(ids, allowed_ids)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_DERIVE_PINNED_IDS_0: wire::Schema = wire::Schema::Dyn;

/// `derivePinnedIds` over the binary wire.
#[napi(js_name = "derivePinnedIdsWire")]
pub fn derive_pinned_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DERIVE_PINNED_IDS_0)?;
        let out = &port::derive_pinned_ids(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_GET_PINNED_ITEMS_0: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Dyn)]));
static S_GET_PINNED_ITEMS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Dyn)]));

/// `getPinnedItems` over the binary wire.
#[napi(js_name = "getPinnedItemsWire")]
pub fn get_pinned_items_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let items: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_GET_PINNED_ITEMS_0.inner()))?;
        let pinned_ids: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::get_pinned_items(items, pinned_ids)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_GET_PINNED_ITEMS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_ORDER_PINNED_ITEMS_FIRST_0: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Dyn)]));
static S_ORDER_PINNED_ITEMS_FIRST_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Dyn)]));

/// `orderPinnedItemsFirst` over the binary wire.
#[napi(js_name = "orderPinnedItemsFirstWire")]
pub fn order_pinned_items_first_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let items: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_ORDER_PINNED_ITEMS_FIRST_0.inner()))?;
        let pinned_ids: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::order_pinned_items_first(items, pinned_ids)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_ORDER_PINNED_ITEMS_FIRST_OUT.inner())?;
        }
        Ok(())
    })
}
