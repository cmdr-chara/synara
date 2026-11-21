//! JS bindings for `packages/shared/src/path.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isWindowsDrivePath`.
#[napi(js_name = "isWindowsDrivePath")]
pub fn is_windows_drive_path(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_windows_drive_path(value)?;
        Ok(out)
    })
}

/// `isUncPath`.
#[napi(js_name = "isUncPath")]
pub fn is_unc_path(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_unc_path(value)?;
        Ok(out)
    })
}

/// `isWindowsAbsolutePath`.
#[napi(js_name = "isWindowsAbsolutePath")]
pub fn is_windows_absolute_path(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_windows_absolute_path(value)?;
        Ok(out)
    })
}

/// `isLocalAbsolutePath`.
#[napi(js_name = "isLocalAbsolutePath")]
pub fn is_local_absolute_path(value: String, options: String) -> napi::Result<bool> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::is_local_absolute_path(value, options)?;
        Ok(out)
    })
}

/// `isExplicitRelativePath`.
#[napi(js_name = "isExplicitRelativePath")]
pub fn is_explicit_relative_path(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_explicit_relative_path(value)?;
        Ok(out)
    })
}

/// `isNormalizedWindowsAbsolutePath`.
#[napi(js_name = "isNormalizedWindowsAbsolutePath")]
pub fn is_normalized_windows_absolute_path(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_normalized_windows_absolute_path(value)?;
        Ok(out)
    })
}

/// `localPathsEqual`.
#[napi(js_name = "localPathsEqual")]
pub fn local_paths_equal(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let out = port::local_paths_equal(left, right)?;
        Ok(out)
    })
}

/// `workspaceRelativePathOf`.
#[napi(js_name = "workspaceRelativePathOf")]
pub fn workspace_relative_path_of(
    target_path: String,
    workspace_root: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::workspace_relative_path_of(target_path, workspace_root)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `joinWorkspaceRelativePath`.
#[napi(js_name = "joinWorkspaceRelativePath")]
pub fn join_workspace_relative_path(
    workspace_root: String,
    relative_path: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::join_workspace_relative_path(workspace_root, relative_path)?;
        Ok(out)
    })
}

/// `isWorkspaceRelativePathSafe`.
#[napi(js_name = "isWorkspaceRelativePathSafe")]
pub fn is_workspace_relative_path_safe(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_workspace_relative_path_safe(value)?;
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

/// `isWindowsDrivePath` over the binary wire.
#[napi(js_name = "isWindowsDrivePathWire")]
pub fn is_windows_drive_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_windows_drive_path(value)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isUncPath` over the binary wire.
#[napi(js_name = "isUncPathWire")]
pub fn is_unc_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_unc_path(value)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isWindowsAbsolutePath` over the binary wire.
#[napi(js_name = "isWindowsAbsolutePathWire")]
pub fn is_windows_absolute_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_windows_absolute_path(value)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isLocalAbsolutePath` over the binary wire.
#[napi(js_name = "isLocalAbsolutePathWire")]
pub fn is_local_absolute_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let options: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::is_local_absolute_path(value, options)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isExplicitRelativePath` over the binary wire.
#[napi(js_name = "isExplicitRelativePathWire")]
pub fn is_explicit_relative_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_explicit_relative_path(value)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isNormalizedWindowsAbsolutePath` over the binary wire.
#[napi(js_name = "isNormalizedWindowsAbsolutePathWire")]
pub fn is_normalized_windows_absolute_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_normalized_windows_absolute_path(value)?;
        w.bool(*out);
        Ok(())
    })
}

/// `localPathsEqual` over the binary wire.
#[napi(js_name = "localPathsEqualWire")]
pub fn local_paths_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: String = r.string16()?;
        let right: String = r.string16()?;
        let out = &port::local_paths_equal(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

/// `workspaceRelativePathOf` over the binary wire.
#[napi(js_name = "workspaceRelativePathOfWire")]
pub fn workspace_relative_path_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let target_path: String = r.string16()?;
        let workspace_root: String = r.string16()?;
        let out = &port::workspace_relative_path_of(target_path, workspace_root)?;
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

/// `joinWorkspaceRelativePath` over the binary wire.
#[napi(js_name = "joinWorkspaceRelativePathWire")]
pub fn join_workspace_relative_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let workspace_root: String = r.string16()?;
        let relative_path: String = r.string16()?;
        let out = &port::join_workspace_relative_path(workspace_root, relative_path)?;
        w.string16(out);
        Ok(())
    })
}

/// `isWorkspaceRelativePathSafe` over the binary wire.
#[napi(js_name = "isWorkspaceRelativePathSafeWire")]
pub fn is_workspace_relative_path_safe_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_workspace_relative_path_safe(value)?;
        w.bool(*out);
        Ok(())
    })
}
