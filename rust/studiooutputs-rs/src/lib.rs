//! JS bindings for `apps/server/src/studioOutputs.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isStudioOutputRelativePath`.
#[napi(js_name = "isStudioOutputRelativePath")]
pub fn is_studio_output_relative_path(relative_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_studio_output_relative_path(relative_path)?;
        Ok(out)
    })
}

/// `collectFileChangeActivityPathCandidates`.
#[napi(js_name = "collectFileChangeActivityPathCandidates")]
pub fn collect_file_change_activity_path_candidates(
    payloads_newest_first: String,
) -> napi::Result<Vec<String>> {
    run(move || {
        let payloads_newest_first = crate::js::from_json(&payloads_newest_first)?;
        let out = port::collect_file_change_activity_path_candidates(payloads_newest_first)?;
        Ok(out)
    })
}

/// `collectThreadOutputRelativePaths`.
#[napi(js_name = "collectThreadOutputRelativePaths")]
pub fn collect_thread_output_relative_paths(checkpoints: String) -> napi::Result<Vec<String>> {
    run(move || {
        let checkpoints = crate::js::from_json(&checkpoints)?;
        let out = port::collect_thread_output_relative_paths(checkpoints)?;
        Ok(out)
    })
}

/// `diffStudioWorkspaceScans`.
#[napi(js_name = "diffStudioWorkspaceScans")]
pub fn diff_studio_workspace_scans(before: String, after: String) -> napi::Result<Vec<String>> {
    run(move || {
        let before = crate::js::from_json(&before)?;
        let after = crate::js::from_json(&after)?;
        let out = port::diff_studio_workspace_scans(before, after)?;
        Ok(out)
    })
}

/// `studioOutputsCapturedActivityPayload`.
#[napi(js_name = "studioOutputsCapturedActivityPayload")]
pub fn studio_outputs_captured_activity_payload(
    relative_paths: Vec<String>,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::studio_outputs_captured_activity_payload(relative_paths, options)?;
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

/// `isStudioOutputRelativePath` over the binary wire.
#[napi(js_name = "isStudioOutputRelativePathWire")]
pub fn is_studio_output_relative_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let relative_path: String = r.string16()?;
        let out = &port::is_studio_output_relative_path(relative_path)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COLLECT_FILE_CHANGE_ACTIVITY_PATH_CANDIDATES_0: wire::Schema = wire::Schema::Dyn;

/// `collectFileChangeActivityPathCandidates` over the binary wire.
#[napi(js_name = "collectFileChangeActivityPathCandidatesWire")]
pub fn collect_file_change_activity_path_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payloads_newest_first: serde_json::Value =
            wire::read_value(r, &S_COLLECT_FILE_CHANGE_ACTIVITY_PATH_CANDIDATES_0)?;
        let out = &port::collect_file_change_activity_path_candidates(payloads_newest_first)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_COLLECT_THREAD_OUTPUT_RELATIVE_PATHS_0: wire::Schema = wire::Schema::Dyn;

/// `collectThreadOutputRelativePaths` over the binary wire.
#[napi(js_name = "collectThreadOutputRelativePathsWire")]
pub fn collect_thread_output_relative_paths_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let checkpoints: serde_json::Value =
            wire::read_value(r, &S_COLLECT_THREAD_OUTPUT_RELATIVE_PATHS_0)?;
        let out = &port::collect_thread_output_relative_paths(checkpoints)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_DIFF_STUDIO_WORKSPACE_SCANS_0: wire::Schema = wire::Schema::Dyn;
static S_DIFF_STUDIO_WORKSPACE_SCANS_1: wire::Schema = wire::Schema::Dyn;

/// `diffStudioWorkspaceScans` over the binary wire.
#[napi(js_name = "diffStudioWorkspaceScansWire")]
pub fn diff_studio_workspace_scans_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let before: serde_json::Value = wire::read_value(r, &S_DIFF_STUDIO_WORKSPACE_SCANS_0)?;
        let after: serde_json::Value = wire::read_value(r, &S_DIFF_STUDIO_WORKSPACE_SCANS_1)?;
        let out = &port::diff_studio_workspace_scans(before, after)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_STUDIO_OUTPUTS_CAPTURED_ACTIVITY_PAYLOAD_OUT: wire::Schema = wire::Schema::Dyn;

/// `studioOutputsCapturedActivityPayload` over the binary wire.
#[napi(js_name = "studioOutputsCapturedActivityPayloadWire")]
pub fn studio_outputs_captured_activity_payload_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let relative_paths: Vec<String> = r.list(|r| r.string16())?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::studio_outputs_captured_activity_payload(relative_paths, options)?;
        wire::write_value(w, out, &S_STUDIO_OUTPUTS_CAPTURED_ACTIVITY_PAYLOAD_OUT)?;
        Ok(())
    })
}
