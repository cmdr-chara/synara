//! JS bindings for `apps/desktop/src/windowsShellAppUserModel.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `windowsShellIconResource`.
#[napi(js_name = "windowsShellIconResource")]
pub fn windows_shell_icon_resource(icon_path: String, icon_index: String) -> napi::Result<String> {
    run(move || {
        let icon_index = crate::js::from_json(&icon_index)?;
        let out = port::windows_shell_icon_resource(icon_path, icon_index)?;
        Ok(out)
    })
}

/// `nativeWindowHandleToHwnd`.
#[napi(js_name = "nativeWindowHandleToHwnd")]
pub fn native_window_handle_to_hwnd(handle: String) -> napi::Result<String> {
    run(move || {
        let handle = crate::js::from_json(&handle)?;
        let out = port::native_window_handle_to_hwnd(handle)?;
        crate::js::to_json(&out)
    })
}

/// `windowsShellAppUserModelHelperName`.
#[napi(js_name = "windowsShellAppUserModelHelperName")]
pub fn windows_shell_app_user_model_helper_name() -> napi::Result<String> {
    run(move || {
        let out = port::windows_shell_app_user_model_helper_name()?;
        Ok(out)
    })
}

/// `cscCompilerCandidates`.
#[napi(js_name = "cscCompilerCandidates")]
pub fn csc_compiler_candidates() -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::csc_compiler_candidates()?;
        Ok(out)
    })
}

/// `ensureWindowsShellAppUserModelHelper`.
#[napi(js_name = "ensureWindowsShellAppUserModelHelper")]
pub fn ensure_windows_shell_app_user_model_helper(cache_directory: String) -> napi::Result<String> {
    run(move || {
        let out = port::ensure_windows_shell_app_user_model_helper(cache_directory)?;
        Ok(out)
    })
}

/// `applyWindowsShellAppUserModel`.
#[napi(js_name = "applyWindowsShellAppUserModel")]
pub fn apply_windows_shell_app_user_model(
    input: String,
    cache_directory: String,
    _options: String,
) -> napi::Result<()> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let _options = crate::js::from_json(&_options)?;
        port::apply_windows_shell_app_user_model(input, cache_directory, _options)?;
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

static S_WINDOWS_SHELL_ICON_RESOURCE_1: wire::Schema = wire::Schema::Dyn;

/// `windowsShellIconResource` over the binary wire.
#[napi(js_name = "windowsShellIconResourceWire")]
pub fn windows_shell_icon_resource_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let icon_path: String = r.string16()?;
        let icon_index: serde_json::Value = wire::read_value(r, &S_WINDOWS_SHELL_ICON_RESOURCE_1)?;
        let out = &port::windows_shell_icon_resource(icon_path, icon_index)?;
        w.string16(out);
        Ok(())
    })
}

static S_NATIVE_WINDOW_HANDLE_TO_HWND_0: wire::Schema = wire::Schema::Dyn;
static S_NATIVE_WINDOW_HANDLE_TO_HWND_OUT: wire::Schema = wire::Schema::Dyn;

/// `nativeWindowHandleToHwnd` over the binary wire.
#[napi(js_name = "nativeWindowHandleToHwndWire")]
pub fn native_window_handle_to_hwnd_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let handle: serde_json::Value = wire::read_value(r, &S_NATIVE_WINDOW_HANDLE_TO_HWND_0)?;
        let out = &port::native_window_handle_to_hwnd(handle)?;
        wire::write_value(w, out, &S_NATIVE_WINDOW_HANDLE_TO_HWND_OUT)?;
        Ok(())
    })
}

/// `windowsShellAppUserModelHelperName` over the binary wire.
#[napi(js_name = "windowsShellAppUserModelHelperNameWire")]
pub fn windows_shell_app_user_model_helper_name_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::windows_shell_app_user_model_helper_name()?;
        w.string16(out);
        Ok(())
    })
}

/// `cscCompilerCandidates` over the binary wire.
#[napi(js_name = "cscCompilerCandidatesWire")]
pub fn csc_compiler_candidates_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::csc_compiler_candidates()?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

/// `ensureWindowsShellAppUserModelHelper` over the binary wire.
#[napi(js_name = "ensureWindowsShellAppUserModelHelperWire")]
pub fn ensure_windows_shell_app_user_model_helper_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cache_directory: String = r.string16()?;
        let out = &port::ensure_windows_shell_app_user_model_helper(cache_directory)?;
        w.string16(out);
        Ok(())
    })
}

static S_APPLY_WINDOWS_SHELL_APP_USER_MODEL_0: wire::Schema = wire::Schema::Dyn;

/// `applyWindowsShellAppUserModel` over the binary wire.
#[napi(js_name = "applyWindowsShellAppUserModelWire")]
pub fn apply_windows_shell_app_user_model_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_APPLY_WINDOWS_SHELL_APP_USER_MODEL_0)?;
        let cache_directory: String = r.string16()?;
        let _options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let () = port::apply_windows_shell_app_user_model(input, cache_directory, _options)?;
        Ok(())
    })
}
