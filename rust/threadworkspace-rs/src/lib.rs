//! JS bindings for `packages/shared/src/threadWorkspace.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `normalizeWorkspaceRootForComparison`.
#[napi(js_name = "normalizeWorkspaceRootForComparison")]
pub fn normalize_workspace_root_for_comparison(
    value: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::normalize_workspace_root_for_comparison(value, options)?;
        Ok(out)
    })
}

/// `workspaceRootsEqual`.
#[napi(js_name = "workspaceRootsEqual")]
pub fn workspace_roots_equal(left: String, right: String, options: String) -> napi::Result<bool> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::workspace_roots_equal(left, right, options)?;
        Ok(out)
    })
}

/// `isWorkspaceRootWithin`.
#[napi(js_name = "isWorkspaceRootWithin")]
pub fn is_workspace_root_within(
    candidate: String,
    ancestor_root: String,
    options: String,
) -> napi::Result<bool> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::is_workspace_root_within(candidate, ancestor_root, options)?;
        Ok(out)
    })
}

/// `isScratchWorkspacePath`.
#[napi(js_name = "isScratchWorkspacePath")]
pub fn is_scratch_workspace_path(file_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_scratch_workspace_path(file_path)?;
        Ok(out)
    })
}

/// `deriveAssociatedWorktreeMetadata`.
#[napi(js_name = "deriveAssociatedWorktreeMetadata")]
pub fn derive_associated_worktree_metadata(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_associated_worktree_metadata(input)?;
        crate::js::to_json(&out)
    })
}

/// `deriveAssociatedWorktreeMetadataPatch`.
#[napi(js_name = "deriveAssociatedWorktreeMetadataPatch")]
pub fn derive_associated_worktree_metadata_patch(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_associated_worktree_metadata_patch(input)?;
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

/// `normalizeWorkspaceRootForComparison` over the binary wire.
#[napi(js_name = "normalizeWorkspaceRootForComparisonWire")]
pub fn normalize_workspace_root_for_comparison_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::normalize_workspace_root_for_comparison(value, options)?;
        w.string16(out);
        Ok(())
    })
}

/// `workspaceRootsEqual` over the binary wire.
#[napi(js_name = "workspaceRootsEqualWire")]
pub fn workspace_roots_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: String = r.string16()?;
        let right: String = r.string16()?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::workspace_roots_equal(left, right, options)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isWorkspaceRootWithin` over the binary wire.
#[napi(js_name = "isWorkspaceRootWithinWire")]
pub fn is_workspace_root_within_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let candidate: String = r.string16()?;
        let ancestor_root: String = r.string16()?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_workspace_root_within(candidate, ancestor_root, options)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isScratchWorkspacePath` over the binary wire.
#[napi(js_name = "isScratchWorkspacePathWire")]
pub fn is_scratch_workspace_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let out = &port::is_scratch_workspace_path(file_path)?;
        w.bool(*out);
        Ok(())
    })
}

static S_DERIVE_ASSOCIATED_WORKTREE_METADATA_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_ASSOCIATED_WORKTREE_METADATA_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "associatedWorktreePath",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "associatedWorktreeBranch",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "associatedWorktreeRef",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
]);

/// `deriveAssociatedWorktreeMetadata` over the binary wire.
#[napi(js_name = "deriveAssociatedWorktreeMetadataWire")]
pub fn derive_associated_worktree_metadata_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_DERIVE_ASSOCIATED_WORKTREE_METADATA_0)?;
        let out = &port::derive_associated_worktree_metadata(input)?;
        wire::write_value(w, out, &S_DERIVE_ASSOCIATED_WORKTREE_METADATA_OUT)?;
        Ok(())
    })
}

static S_DERIVE_ASSOCIATED_WORKTREE_METADATA_PATCH_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_ASSOCIATED_WORKTREE_METADATA_PATCH_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "associatedWorktreePath",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "associatedWorktreeBranch",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "associatedWorktreeRef",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
]);

/// `deriveAssociatedWorktreeMetadataPatch` over the binary wire.
#[napi(js_name = "deriveAssociatedWorktreeMetadataPatchWire")]
pub fn derive_associated_worktree_metadata_patch_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_DERIVE_ASSOCIATED_WORKTREE_METADATA_PATCH_0)?;
        let out = &port::derive_associated_worktree_metadata_patch(input)?;
        wire::write_value(w, out, &S_DERIVE_ASSOCIATED_WORKTREE_METADATA_PATCH_OUT)?;
        Ok(())
    })
}
