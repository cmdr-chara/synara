//! JS bindings for `apps/server/src/privatePathPermissions.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `ensurePrivateDirectorySync`.
#[napi(js_name = "ensurePrivateDirectorySync")]
pub fn ensure_private_directory_sync(directory_path: String, platform: String) -> napi::Result<()> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        port::ensure_private_directory_sync(directory_path, platform)?;
        Ok(())
    })
}

/// `repairPrivateFileSync`.
#[napi(js_name = "repairPrivateFileSync")]
pub fn repair_private_file_sync(file_path: String, options: String) -> napi::Result<()> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        port::repair_private_file_sync(file_path, options)?;
        Ok(())
    })
}

/// `ensurePrivateFileSync`.
#[napi(js_name = "ensurePrivateFileSync")]
pub fn ensure_private_file_sync(file_path: String, options: String) -> napi::Result<()> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        port::ensure_private_file_sync(file_path, options)?;
        Ok(())
    })
}

/// `repairPrivateFile`.
#[napi(js_name = "repairPrivateFile")]
pub async fn repair_private_file(file_path: String, options: String) -> napi::Result<()> {
    run_async(async move {
        let options = crate::js::from_json(&options)?;
        port::repair_private_file(file_path, options).await?;
        Ok(())
    })
    .await
}

/// `repairPrivateTreeSync`.
#[napi(js_name = "repairPrivateTreeSync")]
pub fn repair_private_tree_sync(root_path: String, platform: String) -> napi::Result<()> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        port::repair_private_tree_sync(root_path, platform)?;
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

/// `ensurePrivateDirectorySync` over the binary wire.
#[napi(js_name = "ensurePrivateDirectorySyncWire")]
pub fn ensure_private_directory_sync_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let directory_path: String = r.string16()?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let () = port::ensure_private_directory_sync(directory_path, platform)?;
        Ok(())
    })
}

static S_REPAIR_PRIVATE_FILE_SYNC_1: wire::Schema = wire::Schema::Dyn;

/// `repairPrivateFileSync` over the binary wire.
#[napi(js_name = "repairPrivateFileSyncWire")]
pub fn repair_private_file_sync_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let options: serde_json::Value = wire::read_value(r, &S_REPAIR_PRIVATE_FILE_SYNC_1)?;
        let () = port::repair_private_file_sync(file_path, options)?;
        Ok(())
    })
}

static S_ENSURE_PRIVATE_FILE_SYNC_1: wire::Schema = wire::Schema::Dyn;

/// `ensurePrivateFileSync` over the binary wire.
#[napi(js_name = "ensurePrivateFileSyncWire")]
pub fn ensure_private_file_sync_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let options: serde_json::Value = wire::read_value(r, &S_ENSURE_PRIVATE_FILE_SYNC_1)?;
        let () = port::ensure_private_file_sync(file_path, options)?;
        Ok(())
    })
}

/// `repairPrivateTreeSync` over the binary wire.
#[napi(js_name = "repairPrivateTreeSyncWire")]
pub fn repair_private_tree_sync_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root_path: String = r.string16()?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let () = port::repair_private_tree_sync(root_path, platform)?;
        Ok(())
    })
}
