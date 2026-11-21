//! JS bindings for `packages/shared/src/computerGrants.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `missingComputerAppSnapPermissions`.
#[napi(js_name = "missingComputerAppSnapPermissions")]
pub fn missing_computer_app_snap_permissions(state: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::missing_computer_app_snap_permissions(state)?;
        crate::js::to_json(&out)
    })
}

/// `computerGrantsBlockControl`.
#[napi(js_name = "computerGrantsBlockControl")]
pub fn computer_grants_block_control(permissions: String) -> napi::Result<bool> {
    run(move || {
        let permissions = crate::js::from_json(&permissions)?;
        let out = port::computer_grants_block_control(permissions)?;
        Ok(out)
    })
}

/// `sortComputerPermissions`.
#[napi(js_name = "sortComputerPermissions")]
pub fn sort_computer_permissions(permissions: String) -> napi::Result<String> {
    run(move || {
        let permissions = crate::js::from_json(&permissions)?;
        let out = port::sort_computer_permissions(permissions)?;
        crate::js::to_json(&out)
    })
}

/// `listComputerPermissions`.
#[napi(js_name = "listComputerPermissions")]
pub fn list_computer_permissions(permissions: String) -> napi::Result<String> {
    run(move || {
        let permissions = crate::js::from_json(&permissions)?;
        let out = port::list_computer_permissions(permissions)?;
        Ok(out)
    })
}

/// `computerStaleGrantAdvice`.
#[napi(js_name = "computerStaleGrantAdvice")]
pub fn computer_stale_grant_advice(
    permissions: String,
    build_signature: String,
    bundle_id: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let permissions = crate::js::from_json(&permissions)?;
        let build_signature = crate::js::from_json(&build_signature)?;
        let bundle_id = crate::js::from_json(&bundle_id)?;
        let out = port::computer_stale_grant_advice(permissions, build_signature, bundle_id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `computerPermissionSetupMessage`.
#[napi(js_name = "computerPermissionSetupMessage")]
pub fn computer_permission_setup_message(
    permissions: String,
    build_signature: String,
    bundle_id: String,
) -> napi::Result<String> {
    run(move || {
        let permissions = crate::js::from_json(&permissions)?;
        let build_signature = crate::js::from_json(&build_signature)?;
        let bundle_id = crate::js::from_json(&bundle_id)?;
        let out = port::computer_permission_setup_message(permissions, build_signature, bundle_id)?;
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

static S_MISSING_COMPUTER_APP_SNAP_PERMISSIONS_0: wire::Schema = wire::Schema::Dyn;

/// `missingComputerAppSnapPermissions` over the binary wire.
#[napi(js_name = "missingComputerAppSnapPermissionsWire")]
pub fn missing_computer_app_snap_permissions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_MISSING_COMPUTER_APP_SNAP_PERMISSIONS_0)?;
        let out = &port::missing_computer_app_snap_permissions(state)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_COMPUTER_GRANTS_BLOCK_CONTROL_0: wire::Schema = wire::Schema::Dyn;

/// `computerGrantsBlockControl` over the binary wire.
#[napi(js_name = "computerGrantsBlockControlWire")]
pub fn computer_grants_block_control_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let permissions: serde_json::Value =
            wire::read_value(r, &S_COMPUTER_GRANTS_BLOCK_CONTROL_0)?;
        let out = &port::computer_grants_block_control(permissions)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SORT_COMPUTER_PERMISSIONS_0: wire::Schema = wire::Schema::Dyn;

/// `sortComputerPermissions` over the binary wire.
#[napi(js_name = "sortComputerPermissionsWire")]
pub fn sort_computer_permissions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let permissions: serde_json::Value = wire::read_value(r, &S_SORT_COMPUTER_PERMISSIONS_0)?;
        let out = &port::sort_computer_permissions(permissions)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_LIST_COMPUTER_PERMISSIONS_0: wire::Schema = wire::Schema::Dyn;

/// `listComputerPermissions` over the binary wire.
#[napi(js_name = "listComputerPermissionsWire")]
pub fn list_computer_permissions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let permissions: serde_json::Value = wire::read_value(r, &S_LIST_COMPUTER_PERMISSIONS_0)?;
        let out = &port::list_computer_permissions(permissions)?;
        w.string16(out);
        Ok(())
    })
}

static S_COMPUTER_STALE_GRANT_ADVICE_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_STALE_GRANT_ADVICE_1: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_STALE_GRANT_ADVICE_2: wire::Schema = wire::Schema::Dyn;

/// `computerStaleGrantAdvice` over the binary wire.
#[napi(js_name = "computerStaleGrantAdviceWire")]
pub fn computer_stale_grant_advice_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let permissions: serde_json::Value = wire::read_value(r, &S_COMPUTER_STALE_GRANT_ADVICE_0)?;
        let build_signature: serde_json::Value =
            wire::read_value(r, &S_COMPUTER_STALE_GRANT_ADVICE_1)?;
        let bundle_id: serde_json::Value = wire::read_value(r, &S_COMPUTER_STALE_GRANT_ADVICE_2)?;
        let out = &port::computer_stale_grant_advice(permissions, build_signature, bundle_id)?;
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

static S_COMPUTER_PERMISSION_SETUP_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_PERMISSION_SETUP_MESSAGE_1: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_PERMISSION_SETUP_MESSAGE_2: wire::Schema = wire::Schema::Dyn;

/// `computerPermissionSetupMessage` over the binary wire.
#[napi(js_name = "computerPermissionSetupMessageWire")]
pub fn computer_permission_setup_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let permissions: serde_json::Value =
            wire::read_value(r, &S_COMPUTER_PERMISSION_SETUP_MESSAGE_0)?;
        let build_signature: serde_json::Value =
            wire::read_value(r, &S_COMPUTER_PERMISSION_SETUP_MESSAGE_1)?;
        let bundle_id: serde_json::Value =
            wire::read_value(r, &S_COMPUTER_PERMISSION_SETUP_MESSAGE_2)?;
        let out =
            &port::computer_permission_setup_message(permissions, build_signature, bundle_id)?;
        w.string16(out);
        Ok(())
    })
}
