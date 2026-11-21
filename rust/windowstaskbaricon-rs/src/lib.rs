//! JS bindings for `apps/desktop/src/windowsTaskbarIcon.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `windowsShellIconCachePath`.
#[napi(js_name = "windowsShellIconCachePath")]
pub fn windows_shell_icon_cache_path(
    cache_directory: String,
    icon_key: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::windows_shell_icon_cache_path(cache_directory, icon_key)?;
        Ok(out)
    })
}

/// `windowsShellIconContentKey`.
#[napi(js_name = "windowsShellIconContentKey")]
pub fn windows_shell_icon_content_key(icon_key: String, ico: String) -> napi::Result<String> {
    run(move || {
        let ico = crate::js::from_json(&ico)?;
        let out = port::windows_shell_icon_content_key(icon_key, ico)?;
        Ok(out)
    })
}

/// `resolveWindowsShellIconCacheDirectory`.
#[napi(js_name = "resolveWindowsShellIconCacheDirectory")]
pub fn resolve_windows_shell_icon_cache_directory(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_windows_shell_icon_cache_directory(input)?;
        Ok(out)
    })
}

/// `nextWindowsShellIconCacheKey`.
#[napi(js_name = "nextWindowsShellIconCacheKey")]
pub fn next_windows_shell_icon_cache_key(icon_key: String) -> napi::Result<String> {
    run(move || {
        let out = port::next_windows_shell_icon_cache_key(icon_key)?;
        Ok(out)
    })
}

/// `resetWindowsShellIconGenerationForTests`.
#[napi(js_name = "resetWindowsShellIconGenerationForTests")]
pub fn reset_windows_shell_icon_generation_for_tests() -> napi::Result<()> {
    run(move || {
        port::reset_windows_shell_icon_generation_for_tests()?;
        Ok(())
    })
}

/// `isWindowsTaskbarIconRefreshPending`.
#[napi(js_name = "isWindowsTaskbarIconRefreshPending")]
pub fn is_windows_taskbar_icon_refresh_pending() -> napi::Result<bool> {
    run(move || {
        let out = port::is_windows_taskbar_icon_refresh_pending()?;
        Ok(out)
    })
}

/// `clearWindowsTaskbarIconRefresh`.
#[napi(js_name = "clearWindowsTaskbarIconRefresh")]
pub fn clear_windows_taskbar_icon_refresh() -> napi::Result<()> {
    run(move || {
        port::clear_windows_taskbar_icon_refresh()?;
        Ok(())
    })
}

/// `windowsTaskbarIconPropertyUpdates`.
#[napi(js_name = "windowsTaskbarIconPropertyUpdates")]
pub fn windows_taskbar_icon_property_updates(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::windows_taskbar_icon_property_updates(input)?;
        crate::js::to_json(&out)
    })
}

/// `applyWindowsTaskbarIcon`.
#[napi(js_name = "applyWindowsTaskbarIcon")]
pub fn apply_windows_taskbar_icon(input: String) -> napi::Result<()> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        port::apply_windows_taskbar_icon(input)?;
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

/// `windowsShellIconCachePath` over the binary wire.
#[napi(js_name = "windowsShellIconCachePathWire")]
pub fn windows_shell_icon_cache_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cache_directory: String = r.string16()?;
        let icon_key: String = r.string16()?;
        let out = &port::windows_shell_icon_cache_path(cache_directory, icon_key)?;
        w.string16(out);
        Ok(())
    })
}

static S_WINDOWS_SHELL_ICON_CONTENT_KEY_1: wire::Schema = wire::Schema::Dyn;

/// `windowsShellIconContentKey` over the binary wire.
#[napi(js_name = "windowsShellIconContentKeyWire")]
pub fn windows_shell_icon_content_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let icon_key: String = r.string16()?;
        let ico: serde_json::Value = wire::read_value(r, &S_WINDOWS_SHELL_ICON_CONTENT_KEY_1)?;
        let out = &port::windows_shell_icon_content_key(icon_key, ico)?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_WINDOWS_SHELL_ICON_CACHE_DIRECTORY_0: wire::Schema = wire::Schema::Dyn;

/// `resolveWindowsShellIconCacheDirectory` over the binary wire.
#[napi(js_name = "resolveWindowsShellIconCacheDirectoryWire")]
pub fn resolve_windows_shell_icon_cache_directory_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_WINDOWS_SHELL_ICON_CACHE_DIRECTORY_0)?;
        let out = &port::resolve_windows_shell_icon_cache_directory(input)?;
        w.string16(out);
        Ok(())
    })
}

/// `nextWindowsShellIconCacheKey` over the binary wire.
#[napi(js_name = "nextWindowsShellIconCacheKeyWire")]
pub fn next_windows_shell_icon_cache_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let icon_key: String = r.string16()?;
        let out = &port::next_windows_shell_icon_cache_key(icon_key)?;
        w.string16(out);
        Ok(())
    })
}

/// `resetWindowsShellIconGenerationForTests` over the binary wire.
#[napi(js_name = "resetWindowsShellIconGenerationForTestsWire")]
pub fn reset_windows_shell_icon_generation_for_tests_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::reset_windows_shell_icon_generation_for_tests()?;
        Ok(())
    })
}

/// `isWindowsTaskbarIconRefreshPending` over the binary wire.
#[napi(js_name = "isWindowsTaskbarIconRefreshPendingWire")]
pub fn is_windows_taskbar_icon_refresh_pending_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::is_windows_taskbar_icon_refresh_pending()?;
        w.bool(*out);
        Ok(())
    })
}

/// `clearWindowsTaskbarIconRefresh` over the binary wire.
#[napi(js_name = "clearWindowsTaskbarIconRefreshWire")]
pub fn clear_windows_taskbar_icon_refresh_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::clear_windows_taskbar_icon_refresh()?;
        Ok(())
    })
}

static S_WINDOWS_TASKBAR_ICON_PROPERTY_UPDATES_0: wire::Schema = wire::Schema::Dyn;
static S_WINDOWS_TASKBAR_ICON_PROPERTY_UPDATES_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "iconOnly",
        wire::Schema::Obj(&[
            ("appIconPath", wire::Schema::Str16),
            ("appIconIndex", wire::Schema::Num),
            ("relaunchCommand", wire::Schema::Str16),
            ("relaunchDisplayName", wire::Schema::Str16),
        ]),
    ),
    (
        "withAppId",
        wire::Schema::Obj(&[
            ("appId", wire::Schema::Str16),
            ("appIconPath", wire::Schema::Str16),
            ("appIconIndex", wire::Schema::Num),
            ("relaunchCommand", wire::Schema::Str16),
            ("relaunchDisplayName", wire::Schema::Str16),
        ]),
    ),
]);

/// `windowsTaskbarIconPropertyUpdates` over the binary wire.
#[napi(js_name = "windowsTaskbarIconPropertyUpdatesWire")]
pub fn windows_taskbar_icon_property_updates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_WINDOWS_TASKBAR_ICON_PROPERTY_UPDATES_0)?;
        let out = &port::windows_taskbar_icon_property_updates(input)?;
        wire::write_value(w, out, &S_WINDOWS_TASKBAR_ICON_PROPERTY_UPDATES_OUT)?;
        Ok(())
    })
}

static S_APPLY_WINDOWS_TASKBAR_ICON_0: wire::Schema = wire::Schema::Dyn;

/// `applyWindowsTaskbarIcon` over the binary wire.
#[napi(js_name = "applyWindowsTaskbarIconWire")]
pub fn apply_windows_taskbar_icon_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let input: serde_json::Value = wire::read_value(r, &S_APPLY_WINDOWS_TASKBAR_ICON_0)?;
        let () = port::apply_windows_taskbar_icon(input)?;
        Ok(())
    })
}
