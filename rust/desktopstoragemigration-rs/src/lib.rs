//! JS bindings for `apps/desktop/src/desktopStorageMigration.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isSynaraStorageKey`.
#[napi(js_name = "isSynaraStorageKey")]
pub fn is_synara_storage_key(key: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_synara_storage_key(key)?;
        Ok(out)
    })
}

/// `validateSynaraStorageSnapshot`.
#[napi(js_name = "validateSynaraStorageSnapshot")]
pub fn validate_synara_storage_snapshot(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::validate_synara_storage_snapshot(value)?;
        crate::js::to_json(&out)
    })
}

/// `resolveSynaraStorageSnapshotPath`.
#[napi(js_name = "resolveSynaraStorageSnapshotPath")]
pub fn resolve_synara_storage_snapshot_path(user_data_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_synara_storage_snapshot_path(user_data_path)?;
        Ok(out)
    })
}

/// `readSynaraStorageSnapshot`.
#[napi(js_name = "readSynaraStorageSnapshot")]
pub fn read_synara_storage_snapshot(snapshot_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::read_synara_storage_snapshot(snapshot_path)?;
        crate::js::to_json(&out)
    })
}

/// `acknowledgeSynaraStorageSnapshot`.
#[napi(js_name = "acknowledgeSynaraStorageSnapshot")]
pub async fn acknowledge_synara_storage_snapshot(snapshot_path: String) -> napi::Result<()> {
    run_async(async move {
        port::acknowledge_synara_storage_snapshot(snapshot_path).await?;
        Ok(())
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

/// `isSynaraStorageKey` over the binary wire.
#[napi(js_name = "isSynaraStorageKeyWire")]
pub fn is_synara_storage_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let key: String = r.string16()?;
        let out = &port::is_synara_storage_key(key)?;
        w.bool(*out);
        Ok(())
    })
}

static S_VALIDATE_SYNARA_STORAGE_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_VALIDATE_SYNARA_STORAGE_SNAPSHOT_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("version", wire::Schema::Num),
        ("exportedAt", wire::Schema::Str16),
        ("entries", wire::Schema::Dyn),
    ]));

/// `validateSynaraStorageSnapshot` over the binary wire.
#[napi(js_name = "validateSynaraStorageSnapshotWire")]
pub fn validate_synara_storage_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_VALIDATE_SYNARA_STORAGE_SNAPSHOT_0)?;
        let out = &port::validate_synara_storage_snapshot(value)?;
        wire::write_value(w, out, &S_VALIDATE_SYNARA_STORAGE_SNAPSHOT_OUT)?;
        Ok(())
    })
}

/// `resolveSynaraStorageSnapshotPath` over the binary wire.
#[napi(js_name = "resolveSynaraStorageSnapshotPathWire")]
pub fn resolve_synara_storage_snapshot_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let user_data_path: String = r.string16()?;
        let out = &port::resolve_synara_storage_snapshot_path(user_data_path)?;
        w.string16(out);
        Ok(())
    })
}

static S_READ_SYNARA_STORAGE_SNAPSHOT_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("version", wire::Schema::Num),
        ("exportedAt", wire::Schema::Str16),
        ("entries", wire::Schema::Dyn),
    ]));

/// `readSynaraStorageSnapshot` over the binary wire.
#[napi(js_name = "readSynaraStorageSnapshotWire")]
pub fn read_synara_storage_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let snapshot_path: String = r.string16()?;
        let out = &port::read_synara_storage_snapshot(snapshot_path)?;
        wire::write_value(w, out, &S_READ_SYNARA_STORAGE_SNAPSHOT_OUT)?;
        Ok(())
    })
}
