//! JS bindings for `packages/shared/src/appSnapShortcut.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isAppSnapShortcutModifier`.
#[napi(js_name = "isAppSnapShortcutModifier")]
pub fn is_app_snap_shortcut_modifier(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_app_snap_shortcut_modifier(value)?;
        Ok(out)
    })
}

/// `isAppSnapShortcutKey`.
#[napi(js_name = "isAppSnapShortcutKey")]
pub fn is_app_snap_shortcut_key(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_app_snap_shortcut_key(value)?;
        Ok(out)
    })
}

/// `isAppSnapShortcut`.
#[napi(js_name = "isAppSnapShortcut")]
pub fn is_app_snap_shortcut(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_app_snap_shortcut(value)?;
        Ok(out)
    })
}

/// `appSnapModifierFromEventCode`.
#[napi(js_name = "appSnapModifierFromEventCode")]
pub fn app_snap_modifier_from_event_code(
    code: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::app_snap_modifier_from_event_code(code)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `appSnapShortcutSystemConflict`.
#[napi(js_name = "appSnapShortcutSystemConflict")]
pub fn app_snap_shortcut_system_conflict(
    chord: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let chord = crate::js::from_json(&chord)?;
        let out = port::app_snap_shortcut_system_conflict(chord)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `appSnapShortcutLabels`.
#[napi(js_name = "appSnapShortcutLabels")]
pub fn app_snap_shortcut_labels(shortcut: String) -> napi::Result<String> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let out = port::app_snap_shortcut_labels(shortcut)?;
        crate::js::to_json(&out)
    })
}

/// `appSnapShortcutModifierLabel`.
#[napi(js_name = "appSnapShortcutModifierLabel")]
pub fn app_snap_shortcut_modifier_label(modifier: String) -> napi::Result<String> {
    run(move || {
        let modifier = crate::js::from_json(&modifier)?;
        let out = port::app_snap_shortcut_modifier_label(modifier)?;
        crate::js::to_json(&out)
    })
}

/// `appSnapShortcutKeyLabel`.
#[napi(js_name = "appSnapShortcutKeyLabel")]
pub fn app_snap_shortcut_key_label(code: String) -> napi::Result<String> {
    run(move || {
        let out = port::app_snap_shortcut_key_label(code)?;
        Ok(out)
    })
}

/// `appSnapShortcutAccelerator`.
#[napi(js_name = "appSnapShortcutAccelerator")]
pub fn app_snap_shortcut_accelerator(shortcut: String) -> napi::Result<String> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let out = port::app_snap_shortcut_accelerator(shortcut)?;
        Ok(out)
    })
}

/// `sameAppSnapShortcut`.
#[napi(js_name = "sameAppSnapShortcut")]
pub fn same_app_snap_shortcut(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::same_app_snap_shortcut(left, right)?;
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

static S_IS_APP_SNAP_SHORTCUT_MODIFIER_0: wire::Schema = wire::Schema::Dyn;

/// `isAppSnapShortcutModifier` over the binary wire.
#[napi(js_name = "isAppSnapShortcutModifierWire")]
pub fn is_app_snap_shortcut_modifier_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_IS_APP_SNAP_SHORTCUT_MODIFIER_0)?;
        let out = &port::is_app_snap_shortcut_modifier(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_APP_SNAP_SHORTCUT_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `isAppSnapShortcutKey` over the binary wire.
#[napi(js_name = "isAppSnapShortcutKeyWire")]
pub fn is_app_snap_shortcut_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_IS_APP_SNAP_SHORTCUT_KEY_0)?;
        let out = &port::is_app_snap_shortcut_key(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_APP_SNAP_SHORTCUT_0: wire::Schema = wire::Schema::Dyn;

/// `isAppSnapShortcut` over the binary wire.
#[napi(js_name = "isAppSnapShortcutWire")]
pub fn is_app_snap_shortcut_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_IS_APP_SNAP_SHORTCUT_0)?;
        let out = &port::is_app_snap_shortcut(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `appSnapModifierFromEventCode` over the binary wire.
#[napi(js_name = "appSnapModifierFromEventCodeWire")]
pub fn app_snap_modifier_from_event_code_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let code: String = r.string16()?;
        let out = &port::app_snap_modifier_from_event_code(code)?;
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

static S_APP_SNAP_SHORTCUT_SYSTEM_CONFLICT_0: wire::Schema = wire::Schema::Dyn;

/// `appSnapShortcutSystemConflict` over the binary wire.
#[napi(js_name = "appSnapShortcutSystemConflictWire")]
pub fn app_snap_shortcut_system_conflict_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let chord: serde_json::Value = wire::read_value(r, &S_APP_SNAP_SHORTCUT_SYSTEM_CONFLICT_0)?;
        let out = &port::app_snap_shortcut_system_conflict(chord)?;
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

static S_APP_SNAP_SHORTCUT_LABELS_0: wire::Schema = wire::Schema::Dyn;
static S_APP_SNAP_SHORTCUT_LABELS_OUT: wire::Schema = wire::Schema::Dyn;

/// `appSnapShortcutLabels` over the binary wire.
#[napi(js_name = "appSnapShortcutLabelsWire")]
pub fn app_snap_shortcut_labels_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: serde_json::Value = wire::read_value(r, &S_APP_SNAP_SHORTCUT_LABELS_0)?;
        let out = &port::app_snap_shortcut_labels(shortcut)?;
        wire::write_value(w, out, &S_APP_SNAP_SHORTCUT_LABELS_OUT)?;
        Ok(())
    })
}

static S_APP_SNAP_SHORTCUT_MODIFIER_LABEL_0: wire::Schema = wire::Schema::Dyn;
static S_APP_SNAP_SHORTCUT_MODIFIER_LABEL_OUT: wire::Schema = wire::Schema::Str16;

/// `appSnapShortcutModifierLabel` over the binary wire.
#[napi(js_name = "appSnapShortcutModifierLabelWire")]
pub fn app_snap_shortcut_modifier_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let modifier: serde_json::Value =
            wire::read_value(r, &S_APP_SNAP_SHORTCUT_MODIFIER_LABEL_0)?;
        let out = &port::app_snap_shortcut_modifier_label(modifier)?;
        wire::write_value(w, out, &S_APP_SNAP_SHORTCUT_MODIFIER_LABEL_OUT)?;
        Ok(())
    })
}

/// `appSnapShortcutKeyLabel` over the binary wire.
#[napi(js_name = "appSnapShortcutKeyLabelWire")]
pub fn app_snap_shortcut_key_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let code: String = r.string16()?;
        let out = &port::app_snap_shortcut_key_label(code)?;
        w.string16(out);
        Ok(())
    })
}

static S_APP_SNAP_SHORTCUT_ACCELERATOR_0: wire::Schema = wire::Schema::Dyn;

/// `appSnapShortcutAccelerator` over the binary wire.
#[napi(js_name = "appSnapShortcutAcceleratorWire")]
pub fn app_snap_shortcut_accelerator_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: serde_json::Value = wire::read_value(r, &S_APP_SNAP_SHORTCUT_ACCELERATOR_0)?;
        let out = &port::app_snap_shortcut_accelerator(shortcut)?;
        w.string16(out);
        Ok(())
    })
}

static S_SAME_APP_SNAP_SHORTCUT_0: wire::Schema = wire::Schema::Dyn;
static S_SAME_APP_SNAP_SHORTCUT_1: wire::Schema = wire::Schema::Dyn;

/// `sameAppSnapShortcut` over the binary wire.
#[napi(js_name = "sameAppSnapShortcutWire")]
pub fn same_app_snap_shortcut_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: serde_json::Value = wire::read_value(r, &S_SAME_APP_SNAP_SHORTCUT_0)?;
        let right: serde_json::Value = wire::read_value(r, &S_SAME_APP_SNAP_SHORTCUT_1)?;
        let out = &port::same_app_snap_shortcut(left, right)?;
        w.bool(*out);
        Ok(())
    })
}
