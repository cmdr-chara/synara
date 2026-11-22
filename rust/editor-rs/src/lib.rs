//! JS bindings for `apps/server/src/editor/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `dapObject`.
#[napi(js_name = "dapObject")]
pub fn dap_object(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::dap_object(value);
        crate::js::to_json(&out)
    })
}

/// `dapNumber`.
#[napi(js_name = "dapNumber")]
pub fn dap_number(value: String, fallback: Option<f64>) -> napi::Result<f64> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::dap_number(value, fallback);
        Ok(out)
    })
}

/// `dapString`.
#[napi(js_name = "dapString")]
pub fn dap_string(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::dap_string(value);
        Ok(out)
    })
}

/// `dapArray`.
#[napi(js_name = "dapArray")]
pub fn dap_array(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::dap_array(value);
        crate::js::to_json(&out)
    })
}

/// `ensureDebugAdapter`.
#[napi(js_name = "ensureDebugAdapter")]
pub async fn ensure_debug_adapter(state_dir: String, signal: String) -> napi::Result<String> {
    run_async(async move {
        let signal = crate::js::from_json(&signal)?;
        let out = port::ensure_debug_adapter(state_dir, signal).await?;
        Ok(out)
    })
    .await
}

/// `nativeEditorUnavailableError`.
#[napi(js_name = "nativeEditorUnavailableError")]
pub fn native_editor_unavailable_error(flavor: String) -> napi::Result<String> {
    run(move || {
        let flavor = crate::js::from_json(&flavor)?;
        let out = port::native_editor_unavailable_error(flavor);
        crate::js::to_json(&out)
    })
}

/// `verifyEditorLanguageRuntime`.
#[napi(js_name = "verifyEditorLanguageRuntime")]
pub async fn verify_editor_language_runtime() -> napi::Result<()> {
    run_async(async move {
        port::verify_editor_language_runtime().await;
        Ok(())
    })
    .await
}

/// `createTypeScriptService`.
#[napi(js_name = "createTypeScriptService")]
pub fn create_type_script_service(workspace_root: String) -> napi::Result<String> {
    run(move || {
        let out = port::create_type_script_service(workspace_root);
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

static S_DAP_OBJECT_0: wire::Schema = wire::Schema::Dyn;
static S_DAP_OBJECT_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Dyn);

/// `dapObject` over the binary wire.
#[napi(js_name = "dapObjectWire")]
pub fn dap_object_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_DAP_OBJECT_0)?;
        let out = &port::dap_object(value);
        wire::write_value(w, out, &S_DAP_OBJECT_OUT)?;
        Ok(())
    })
}

static S_DAP_NUMBER_0: wire::Schema = wire::Schema::Dyn;

/// `dapNumber` over the binary wire.
#[napi(js_name = "dapNumberWire")]
pub fn dap_number_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_DAP_NUMBER_0)?;
        let fallback: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::dap_number(value, fallback);
        w.f64(*out);
        Ok(())
    })
}

static S_DAP_STRING_0: wire::Schema = wire::Schema::Dyn;

/// `dapString` over the binary wire.
#[napi(js_name = "dapStringWire")]
pub fn dap_string_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_DAP_STRING_0)?;
        let out = &port::dap_string(value);
        w.string16(out);
        Ok(())
    })
}

static S_DAP_ARRAY_0: wire::Schema = wire::Schema::Dyn;
static S_DAP_ARRAY_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Map16(&wire::Schema::Dyn));

/// `dapArray` over the binary wire.
#[napi(js_name = "dapArrayWire")]
pub fn dap_array_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_DAP_ARRAY_0)?;
        let out = &port::dap_array(value);
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_DAP_ARRAY_OUT.inner())?;
        }
        Ok(())
    })
}

static S_NATIVE_EDITOR_UNAVAILABLE_ERROR_0: wire::Schema = wire::Schema::Dyn;

/// `nativeEditorUnavailableError` over the binary wire.
#[napi(js_name = "nativeEditorUnavailableErrorWire")]
pub fn native_editor_unavailable_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let flavor: serde_json::Value = wire::read_value(r, &S_NATIVE_EDITOR_UNAVAILABLE_ERROR_0)?;
        let out = &port::native_editor_unavailable_error(flavor);
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_CREATE_TYPE_SCRIPT_SERVICE_OUT: wire::Schema = wire::Schema::Dyn;

/// `createTypeScriptService` over the binary wire.
#[napi(js_name = "createTypeScriptServiceWire")]
pub fn create_type_script_service_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let workspace_root: String = r.string16()?;
        let out = &port::create_type_script_service(workspace_root);
        wire::write_value(w, out, &S_CREATE_TYPE_SCRIPT_SERVICE_OUT)?;
        Ok(())
    })
}
