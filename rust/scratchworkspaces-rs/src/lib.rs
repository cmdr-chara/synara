//! JS bindings for `apps/server/src/scratchWorkspaces.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveScratchWorkspacesRoot`.
#[napi(js_name = "resolveScratchWorkspacesRoot")]
pub fn resolve_scratch_workspaces_root(options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::resolve_scratch_workspaces_root(options)?;
        Ok(out)
    })
}

/// `ensureIsolatedScratchWorkspace`.
#[napi(js_name = "ensureIsolatedScratchWorkspace")]
pub fn ensure_isolated_scratch_workspace(
    thread_id: String,
    workspace_root: String,
    legacy_workspace_root: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let workspace_root = crate::js::from_json(&workspace_root)?;
        let legacy_workspace_root = crate::js::from_json(&legacy_workspace_root)?;
        let out = port::ensure_isolated_scratch_workspace(
            thread_id,
            workspace_root,
            legacy_workspace_root,
        )?;
        Ok(out)
    })
}

/// `resolveScratchWorkspaceCwd`.
#[napi(js_name = "resolveScratchWorkspaceCwd")]
pub fn resolve_scratch_workspace_cwd(
    thread_id: String,
    persisted_cwd: String,
    roots: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let persisted_cwd = crate::js::from_json(&persisted_cwd)?;
        let roots = crate::js::from_json(&roots)?;
        let out = port::resolve_scratch_workspace_cwd(thread_id, persisted_cwd, roots)?;
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

static S_RESOLVE_SCRATCH_WORKSPACES_ROOT_0: wire::Schema = wire::Schema::Dyn;

/// `resolveScratchWorkspacesRoot` over the binary wire.
#[napi(js_name = "resolveScratchWorkspacesRootWire")]
pub fn resolve_scratch_workspaces_root_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let options: serde_json::Value = wire::read_value(r, &S_RESOLVE_SCRATCH_WORKSPACES_ROOT_0)?;
        let out = &port::resolve_scratch_workspaces_root(options)?;
        w.string16(out);
        Ok(())
    })
}

static S_ENSURE_ISOLATED_SCRATCH_WORKSPACE_0: wire::Schema = wire::Schema::Dyn;
static S_ENSURE_ISOLATED_SCRATCH_WORKSPACE_1: wire::Schema = wire::Schema::Dyn;
static S_ENSURE_ISOLATED_SCRATCH_WORKSPACE_2: wire::Schema = wire::Schema::Dyn;

/// `ensureIsolatedScratchWorkspace` over the binary wire.
#[napi(js_name = "ensureIsolatedScratchWorkspaceWire")]
pub fn ensure_isolated_scratch_workspace_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_ENSURE_ISOLATED_SCRATCH_WORKSPACE_0)?;
        let workspace_root: serde_json::Value =
            wire::read_value(r, &S_ENSURE_ISOLATED_SCRATCH_WORKSPACE_1)?;
        let legacy_workspace_root: serde_json::Value =
            wire::read_value(r, &S_ENSURE_ISOLATED_SCRATCH_WORKSPACE_2)?;
        let out = &port::ensure_isolated_scratch_workspace(
            thread_id,
            workspace_root,
            legacy_workspace_root,
        )?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_SCRATCH_WORKSPACE_CWD_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_SCRATCH_WORKSPACE_CWD_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_SCRATCH_WORKSPACE_CWD_2: wire::Schema = wire::Schema::Dyn;

/// `resolveScratchWorkspaceCwd` over the binary wire.
#[napi(js_name = "resolveScratchWorkspaceCwdWire")]
pub fn resolve_scratch_workspace_cwd_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value = wire::read_value(r, &S_RESOLVE_SCRATCH_WORKSPACE_CWD_0)?;
        let persisted_cwd: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_SCRATCH_WORKSPACE_CWD_1)?;
        let roots: serde_json::Value = wire::read_value(r, &S_RESOLVE_SCRATCH_WORKSPACE_CWD_2)?;
        let out = &port::resolve_scratch_workspace_cwd(thread_id, persisted_cwd, roots)?;
        w.string16(out);
        Ok(())
    })
}
