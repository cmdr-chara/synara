//! JS bindings for `apps/web/src/appRail.logic.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isRailOrderableItemId`.
#[napi(js_name = "isRailOrderableItemId")]
pub fn is_rail_orderable_item_id(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_rail_orderable_item_id(value)?;
        Ok(out)
    })
}

/// `railItemCanHide`.
#[napi(js_name = "railItemCanHide")]
pub fn rail_item_can_hide(id: String) -> napi::Result<bool> {
    run(move || {
        let id = crate::js::from_json(&id)?;
        let out = port::rail_item_can_hide(id)?;
        Ok(out)
    })
}

/// `normalizeRailItemOrder`.
#[napi(js_name = "normalizeRailItemOrder")]
pub fn normalize_rail_item_order(order: Vec<String>) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_rail_item_order(order)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeHiddenRailItems`.
#[napi(js_name = "normalizeHiddenRailItems")]
pub fn normalize_hidden_rail_items(hidden: Vec<String>) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_hidden_rail_items(hidden)?;
        crate::js::to_json(&out)
    })
}

/// `isRailItemAvailable`.
#[napi(js_name = "isRailItemAvailable")]
pub fn is_rail_item_available(id: String, availability: String) -> napi::Result<String> {
    run(move || {
        let id = crate::js::from_json(&id)?;
        let availability = crate::js::from_json(&availability)?;
        let out = port::is_rail_item_available(id, availability)?;
        crate::js::to_json(&out)
    })
}

/// `buildRailItemOrder`.
#[napi(js_name = "buildRailItemOrder")]
pub fn build_rail_item_order(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_rail_item_order(input)?;
        crate::js::to_json(&out)
    })
}

/// `railSpaceShortcutKey`.
#[napi(js_name = "railSpaceShortcutKey")]
pub fn rail_space_shortcut_key(space_id: String) -> napi::Result<String> {
    run(move || {
        let space_id = crate::js::from_json(&space_id)?;
        let out = port::rail_space_shortcut_key(space_id)?;
        Ok(out)
    })
}

/// `railProjectShortcutKey`.
#[napi(js_name = "railProjectShortcutKey")]
pub fn rail_project_shortcut_key(project_id: String) -> napi::Result<String> {
    run(move || {
        let project_id = crate::js::from_json(&project_id)?;
        let out = port::rail_project_shortcut_key(project_id)?;
        Ok(out)
    })
}

/// `resolveRailShortcuts`.
#[napi(js_name = "resolveRailShortcuts")]
pub fn resolve_rail_shortcuts(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_rail_shortcuts(input)?;
        crate::js::to_json(&out)
    })
}

/// `toggleRailShortcutKey`.
#[napi(js_name = "toggleRailShortcutKey")]
pub fn toggle_rail_shortcut_key(keys: Vec<String>, key: String) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::toggle_rail_shortcut_key(keys, key)?;
        Ok(out)
    })
}

/// `resolveActiveRailShortcutKey`.
#[napi(js_name = "resolveActiveRailShortcutKey")]
pub fn resolve_active_rail_shortcut_key(
    input: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_active_rail_shortcut_key(input)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `railItemShowsPanel`.
#[napi(js_name = "railItemShowsPanel")]
pub fn rail_item_shows_panel(id: String) -> napi::Result<bool> {
    run(move || {
        let id = crate::js::from_json(&id)?;
        let out = port::rail_item_shows_panel(id)?;
        Ok(out)
    })
}

/// `railItemForPathname`.
#[napi(js_name = "railItemForPathname")]
pub fn rail_item_for_pathname(pathname: String) -> napi::Result<String> {
    run(move || {
        let out = port::rail_item_for_pathname(pathname)?;
        crate::js::to_json(&out)
    })
}

/// `reconcileActiveRailItem`.
#[napi(js_name = "reconcileActiveRailItem")]
pub fn reconcile_active_rail_item(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::reconcile_active_rail_item(input)?;
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

/// `isRailOrderableItemId` over the binary wire.
#[napi(js_name = "isRailOrderableItemIdWire")]
pub fn is_rail_orderable_item_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_rail_orderable_item_id(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RAIL_ITEM_CAN_HIDE_0: wire::Schema = wire::Schema::Dyn;

/// `railItemCanHide` over the binary wire.
#[napi(js_name = "railItemCanHideWire")]
pub fn rail_item_can_hide_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: serde_json::Value = wire::read_value(r, &S_RAIL_ITEM_CAN_HIDE_0)?;
        let out = &port::rail_item_can_hide(id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_NORMALIZE_RAIL_ITEM_ORDER_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `normalizeRailItemOrder` over the binary wire.
#[napi(js_name = "normalizeRailItemOrderWire")]
pub fn normalize_rail_item_order_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let order: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::normalize_rail_item_order(order)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_NORMALIZE_RAIL_ITEM_ORDER_OUT.inner())?;
        }
        Ok(())
    })
}

static S_NORMALIZE_HIDDEN_RAIL_ITEMS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `normalizeHiddenRailItems` over the binary wire.
#[napi(js_name = "normalizeHiddenRailItemsWire")]
pub fn normalize_hidden_rail_items_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let hidden: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::normalize_hidden_rail_items(hidden)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_NORMALIZE_HIDDEN_RAIL_ITEMS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_IS_RAIL_ITEM_AVAILABLE_0: wire::Schema = wire::Schema::Dyn;
static S_IS_RAIL_ITEM_AVAILABLE_1: wire::Schema = wire::Schema::Dyn;
static S_IS_RAIL_ITEM_AVAILABLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `isRailItemAvailable` over the binary wire.
#[napi(js_name = "isRailItemAvailableWire")]
pub fn is_rail_item_available_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: serde_json::Value = wire::read_value(r, &S_IS_RAIL_ITEM_AVAILABLE_0)?;
        let availability: serde_json::Value = wire::read_value(r, &S_IS_RAIL_ITEM_AVAILABLE_1)?;
        let out = &port::is_rail_item_available(id, availability)?;
        wire::write_value(w, out, &S_IS_RAIL_ITEM_AVAILABLE_OUT)?;
        Ok(())
    })
}

static S_BUILD_RAIL_ITEM_ORDER_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_RAIL_ITEM_ORDER_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `buildRailItemOrder` over the binary wire.
#[napi(js_name = "buildRailItemOrderWire")]
pub fn build_rail_item_order_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_RAIL_ITEM_ORDER_0)?;
        let out = &port::build_rail_item_order(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_BUILD_RAIL_ITEM_ORDER_OUT.inner())?;
        }
        Ok(())
    })
}

/// `railSpaceShortcutKey` over the binary wire.
#[napi(js_name = "railSpaceShortcutKeyWire")]
pub fn rail_space_shortcut_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let space_id: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::rail_space_shortcut_key(space_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_RAIL_PROJECT_SHORTCUT_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `railProjectShortcutKey` over the binary wire.
#[napi(js_name = "railProjectShortcutKeyWire")]
pub fn rail_project_shortcut_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let project_id: serde_json::Value = wire::read_value(r, &S_RAIL_PROJECT_SHORTCUT_KEY_0)?;
        let out = &port::rail_project_shortcut_key(project_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_RAIL_SHORTCUTS_0: wire::Schema = wire::Schema::Dyn;

/// `resolveRailShortcuts` over the binary wire.
#[napi(js_name = "resolveRailShortcutsWire")]
pub fn resolve_rail_shortcuts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_RAIL_SHORTCUTS_0)?;
        let out = &port::resolve_rail_shortcuts(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `toggleRailShortcutKey` over the binary wire.
#[napi(js_name = "toggleRailShortcutKeyWire")]
pub fn toggle_rail_shortcut_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let keys: Vec<String> = r.list(|r| r.string16())?;
        let key: String = r.string16()?;
        let out = &port::toggle_rail_shortcut_key(keys, key)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_RESOLVE_ACTIVE_RAIL_SHORTCUT_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `resolveActiveRailShortcutKey` over the binary wire.
#[napi(js_name = "resolveActiveRailShortcutKeyWire")]
pub fn resolve_active_rail_shortcut_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_ACTIVE_RAIL_SHORTCUT_KEY_0)?;
        let out = &port::resolve_active_rail_shortcut_key(input)?;
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

static S_RAIL_ITEM_SHOWS_PANEL_0: wire::Schema = wire::Schema::Dyn;

/// `railItemShowsPanel` over the binary wire.
#[napi(js_name = "railItemShowsPanelWire")]
pub fn rail_item_shows_panel_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: serde_json::Value = wire::read_value(r, &S_RAIL_ITEM_SHOWS_PANEL_0)?;
        let out = &port::rail_item_shows_panel(id)?;
        w.bool(*out);
        Ok(())
    })
}

/// `railItemForPathname` over the binary wire.
#[napi(js_name = "railItemForPathnameWire")]
pub fn rail_item_for_pathname_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pathname: String = r.string16()?;
        let out = &port::rail_item_for_pathname(pathname)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RECONCILE_ACTIVE_RAIL_ITEM_0: wire::Schema = wire::Schema::Dyn;
static S_RECONCILE_ACTIVE_RAIL_ITEM_OUT: wire::Schema = wire::Schema::Dyn;

/// `reconcileActiveRailItem` over the binary wire.
#[napi(js_name = "reconcileActiveRailItemWire")]
pub fn reconcile_active_rail_item_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RECONCILE_ACTIVE_RAIL_ITEM_0)?;
        let out = &port::reconcile_active_rail_item(input)?;
        wire::write_value(w, out, &S_RECONCILE_ACTIVE_RAIL_ITEM_OUT)?;
        Ok(())
    })
}
