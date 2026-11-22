//! JS bindings for `apps/server/src/workspace/`: generated from the signatures in `port.rs` by rust-anything.
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

#[allow(dead_code)]
async fn run_async<T>(f: impl std::future::Future<Output = Result<T, String>>) -> napi::Result<T> {
    use futures::FutureExt as _;
    match std::panic::AssertUnwindSafe(f).catch_unwind().await {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `listLocalPreviewWorktreeRoots`.
#[napi(js_name = "listLocalPreviewWorktreeRoots")]
pub fn list_local_preview_worktree_roots(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::list_local_preview_worktree_roots(input)?;
        crate::js::to_json(&out)
    })
}

/// `parseManagedWorktreeWorkspaceRoot`.
#[napi(js_name = "parseManagedWorktreeWorkspaceRoot")]
pub fn parse_managed_worktree_workspace_root(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_managed_worktree_workspace_root(input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveOutOfRootFileReference` — raw result: the resolved path or null.
#[napi(js_name = "resolveOutOfRootFileReference")]
pub async fn resolve_out_of_root_file_reference(input: String) -> napi::Result<Option<String>> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_out_of_root_file_reference(input).await?;
        raw_nullable_string(out)
    })
    .await
}

/// `string | null` results on the raw wire: `Null` → `None`, `String` → `Some`.
fn raw_nullable_string(out: serde_json::Value) -> Result<Option<String>, String> {
    match out {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => Ok(Some(s)),
        other => Err(format!("unexpected result shape: {other}")),
    }
}

/// `isContainedPath`.
#[napi(js_name = "isContainedPath")]
pub fn is_contained_path(real_root: String, candidate_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_contained_path(real_root, candidate_path)?;
        Ok(out)
    })
}

/// `resolveRealPathWithinRoot`.
#[napi(js_name = "resolveRealPathWithinRoot")]
pub async fn resolve_real_path_within_root(
    workspace_root: String,
    absolute_path: String,
) -> napi::Result<Option<String>> {
    run_async(async move {
        let out = port::resolve_real_path_within_root(workspace_root, absolute_path).await?;
        raw_nullable_string(out)
    })
    .await
}

/// `resolveRealPathForCreateWithinRoot`.
#[napi(js_name = "resolveRealPathForCreateWithinRoot")]
pub async fn resolve_real_path_for_create_within_root(
    workspace_root: String,
    absolute_path: String,
) -> napi::Result<Option<String>> {
    run_async(async move {
        let out =
            port::resolve_real_path_for_create_within_root(workspace_root, absolute_path).await?;
        raw_nullable_string(out)
    })
    .await
}

/// `prepareRealPathForWriteWithinRoot`.
#[napi(js_name = "prepareRealPathForWriteWithinRoot")]
pub async fn prepare_real_path_for_write_within_root(
    workspace_root: String,
    absolute_path: String,
) -> napi::Result<Option<String>> {
    run_async(async move {
        let out =
            port::prepare_real_path_for_write_within_root(workspace_root, absolute_path).await?;
        raw_nullable_string(out)
    })
    .await
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

static S_LIST_LOCAL_PREVIEW_WORKTREE_ROOTS_0: wire::Schema = wire::Schema::Dyn;
static S_LIST_LOCAL_PREVIEW_WORKTREE_ROOTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `listLocalPreviewWorktreeRoots` over the binary wire.
#[napi(js_name = "listLocalPreviewWorktreeRootsWire")]
pub fn list_local_preview_worktree_roots_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_LIST_LOCAL_PREVIEW_WORKTREE_ROOTS_0)?;
        let out = &port::list_local_preview_worktree_roots(input)?;
        wire::write_value(w, out, &S_LIST_LOCAL_PREVIEW_WORKTREE_ROOTS_OUT)?;
        Ok(())
    })
}

static S_PARSE_MANAGED_WORKTREE_WORKSPACE_ROOT_0: wire::Schema = wire::Schema::Dyn;

/// `parseManagedWorktreeWorkspaceRoot` over the binary wire.
#[napi(js_name = "parseManagedWorktreeWorkspaceRootWire")]
pub fn parse_managed_worktree_workspace_root_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_PARSE_MANAGED_WORKTREE_WORKSPACE_ROOT_0)?;
        let out = &port::parse_managed_worktree_workspace_root(input)?;
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

/// `isContainedPath` over the binary wire.
#[napi(js_name = "isContainedPathWire")]
pub fn is_contained_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let real_root: String = r.string16()?;
        let candidate_path: String = r.string16()?;
        let out = &port::is_contained_path(real_root, candidate_path)?;
        w.bool(*out);
        Ok(())
    })
}
