//! JS bindings for `apps/desktop/src/menuShortcuts.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveDesktopPhysicalZoomAction`.
#[napi(js_name = "resolveDesktopPhysicalZoomAction")]
pub fn resolve_desktop_physical_zoom_action(
    platform: String,
    input: String,
) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_desktop_physical_zoom_action(platform, input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveDesktopZoomShortcutAction`.
#[napi(js_name = "resolveDesktopZoomShortcutAction")]
pub fn resolve_desktop_zoom_shortcut_action(
    platform: String,
    input: String,
) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_desktop_zoom_shortcut_action(platform, input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveDesktopMenuAccelerator`.
#[napi(js_name = "resolveDesktopMenuAccelerator")]
pub fn resolve_desktop_menu_accelerator(
    platform: String,
    accelerator: String,
) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let accelerator = crate::js::from_json(&accelerator)?;
        let out = port::resolve_desktop_menu_accelerator(platform, accelerator)?;
        crate::js::to_json(&out)
    })
}

/// `keybindingShortcutToAccelerator`.
#[napi(js_name = "keybindingShortcutToAccelerator")]
pub fn keybinding_shortcut_to_accelerator(
    shortcut: String,
    platform: String,
) -> napi::Result<String> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::keybinding_shortcut_to_accelerator(shortcut, platform)?;
        crate::js::to_json(&out)
    })
}

/// `resolveReportedMenuAccelerators`.
#[napi(js_name = "resolveReportedMenuAccelerators")]
pub fn resolve_reported_menu_accelerators(
    payload: String,
    platform: String,
) -> napi::Result<String> {
    run(move || {
        let payload = crate::js::from_json(&payload)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::resolve_reported_menu_accelerators(payload, platform)?;
        crate::js::to_json(&out)
    })
}

/// `sameDesktopMenuAccelerators`.
#[napi(js_name = "sameDesktopMenuAccelerators")]
pub fn same_desktop_menu_accelerators(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::same_desktop_menu_accelerators(left, right)?;
        Ok(out)
    })
}

/// `shouldUseNativeZoomMenuRoles`.
#[napi(js_name = "shouldUseNativeZoomMenuRoles")]
pub fn should_use_native_zoom_menu_roles(platform: String) -> napi::Result<bool> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let out = port::should_use_native_zoom_menu_roles(platform)?;
        Ok(out)
    })
}

/// `resolveKeyboardShortcutsMenuAccelerator`.
#[napi(js_name = "resolveKeyboardShortcutsMenuAccelerator")]
pub fn resolve_keyboard_shortcuts_menu_accelerator(platform: String) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let out = port::resolve_keyboard_shortcuts_menu_accelerator(platform)?;
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

static S_RESOLVE_DESKTOP_PHYSICAL_ZOOM_ACTION_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DESKTOP_PHYSICAL_ZOOM_ACTION_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DESKTOP_PHYSICAL_ZOOM_ACTION_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveDesktopPhysicalZoomAction` over the binary wire.
#[napi(js_name = "resolveDesktopPhysicalZoomActionWire")]
pub fn resolve_desktop_physical_zoom_action_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: Value = wire::read_value(r, &S_RESOLVE_DESKTOP_PHYSICAL_ZOOM_ACTION_0)?;
        let input: Value = wire::read_value(r, &S_RESOLVE_DESKTOP_PHYSICAL_ZOOM_ACTION_1)?;
        let out = &port::resolve_desktop_physical_zoom_action(platform, input)?;
        wire::write_value(w, out, &S_RESOLVE_DESKTOP_PHYSICAL_ZOOM_ACTION_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_DESKTOP_ZOOM_SHORTCUT_ACTION_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DESKTOP_ZOOM_SHORTCUT_ACTION_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DESKTOP_ZOOM_SHORTCUT_ACTION_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveDesktopZoomShortcutAction` over the binary wire.
#[napi(js_name = "resolveDesktopZoomShortcutActionWire")]
pub fn resolve_desktop_zoom_shortcut_action_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: Value = wire::read_value(r, &S_RESOLVE_DESKTOP_ZOOM_SHORTCUT_ACTION_0)?;
        let input: Value = wire::read_value(r, &S_RESOLVE_DESKTOP_ZOOM_SHORTCUT_ACTION_1)?;
        let out = &port::resolve_desktop_zoom_shortcut_action(platform, input)?;
        wire::write_value(w, out, &S_RESOLVE_DESKTOP_ZOOM_SHORTCUT_ACTION_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_DESKTOP_MENU_ACCELERATOR_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DESKTOP_MENU_ACCELERATOR_1: wire::Schema = wire::Schema::Dyn;

/// `resolveDesktopMenuAccelerator` over the binary wire.
#[napi(js_name = "resolveDesktopMenuAcceleratorWire")]
pub fn resolve_desktop_menu_accelerator_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: Value = wire::read_value(r, &S_RESOLVE_DESKTOP_MENU_ACCELERATOR_0)?;
        let accelerator: Value = wire::read_value(r, &S_RESOLVE_DESKTOP_MENU_ACCELERATOR_1)?;
        let out = &port::resolve_desktop_menu_accelerator(platform, accelerator)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_KEYBINDING_SHORTCUT_TO_ACCELERATOR_0: wire::Schema = wire::Schema::Dyn;
static S_KEYBINDING_SHORTCUT_TO_ACCELERATOR_1: wire::Schema = wire::Schema::Dyn;
static S_KEYBINDING_SHORTCUT_TO_ACCELERATOR_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `keybindingShortcutToAccelerator` over the binary wire.
#[napi(js_name = "keybindingShortcutToAcceleratorWire")]
pub fn keybinding_shortcut_to_accelerator_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: Value = wire::read_value(r, &S_KEYBINDING_SHORTCUT_TO_ACCELERATOR_0)?;
        let platform: Value = wire::read_value(r, &S_KEYBINDING_SHORTCUT_TO_ACCELERATOR_1)?;
        let out = &port::keybinding_shortcut_to_accelerator(shortcut, platform)?;
        wire::write_value(w, out, &S_KEYBINDING_SHORTCUT_TO_ACCELERATOR_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_REPORTED_MENU_ACCELERATORS_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_REPORTED_MENU_ACCELERATORS_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_REPORTED_MENU_ACCELERATORS_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveReportedMenuAccelerators` over the binary wire.
#[napi(js_name = "resolveReportedMenuAcceleratorsWire")]
pub fn resolve_reported_menu_accelerators_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: Value = wire::read_value(r, &S_RESOLVE_REPORTED_MENU_ACCELERATORS_0)?;
        let platform: Value = wire::read_value(r, &S_RESOLVE_REPORTED_MENU_ACCELERATORS_1)?;
        let out = &port::resolve_reported_menu_accelerators(payload, platform)?;
        wire::write_value(w, out, &S_RESOLVE_REPORTED_MENU_ACCELERATORS_OUT)?;
        Ok(())
    })
}

static S_SAME_DESKTOP_MENU_ACCELERATORS_0: wire::Schema = wire::Schema::Dyn;
static S_SAME_DESKTOP_MENU_ACCELERATORS_1: wire::Schema = wire::Schema::Dyn;

/// `sameDesktopMenuAccelerators` over the binary wire.
#[napi(js_name = "sameDesktopMenuAcceleratorsWire")]
pub fn same_desktop_menu_accelerators_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_SAME_DESKTOP_MENU_ACCELERATORS_0)?;
        let right: Value = wire::read_value(r, &S_SAME_DESKTOP_MENU_ACCELERATORS_1)?;
        let out = &port::same_desktop_menu_accelerators(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_USE_NATIVE_ZOOM_MENU_ROLES_0: wire::Schema = wire::Schema::Dyn;

/// `shouldUseNativeZoomMenuRoles` over the binary wire.
#[napi(js_name = "shouldUseNativeZoomMenuRolesWire")]
pub fn should_use_native_zoom_menu_roles_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: Value = wire::read_value(r, &S_SHOULD_USE_NATIVE_ZOOM_MENU_ROLES_0)?;
        let out = &port::should_use_native_zoom_menu_roles(platform)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_KEYBOARD_SHORTCUTS_MENU_ACCELERATOR_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_KEYBOARD_SHORTCUTS_MENU_ACCELERATOR_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveKeyboardShortcutsMenuAccelerator` over the binary wire.
#[napi(js_name = "resolveKeyboardShortcutsMenuAcceleratorWire")]
pub fn resolve_keyboard_shortcuts_menu_accelerator_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: Value =
            wire::read_value(r, &S_RESOLVE_KEYBOARD_SHORTCUTS_MENU_ACCELERATOR_0)?;
        let out = &port::resolve_keyboard_shortcuts_menu_accelerator(platform)?;
        wire::write_value(w, out, &S_RESOLVE_KEYBOARD_SHORTCUTS_MENU_ACCELERATOR_OUT)?;
        Ok(())
    })
}
