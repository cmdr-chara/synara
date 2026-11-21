//! JS bindings for `apps/server/src/remoteFeaturePolicy.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

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

/// `remoteConnectionsUnavailableReason`.
#[napi(js_name = "remoteConnectionsUnavailableReason")]
pub fn remote_connections_unavailable_reason(
    state_dir: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::remote_connections_unavailable_reason(state_dir)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `accountLiveActivityEnabled`.
#[napi(js_name = "accountLiveActivityEnabled")]
pub fn account_live_activity_enabled(state_dir: Option<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::account_live_activity_enabled(state_dir)?;
        Ok(out)
    })
}

/// `requireRemoteConnections`.
#[napi(js_name = "requireRemoteConnections")]
pub fn require_remote_connections(state_dir: Option<String>) -> napi::Result<()> {
    run(move || {
        port::require_remote_connections(state_dir)?;
        Ok(())
    })
}

/// `accountProfileSyncUnavailableReason`.
#[napi(js_name = "accountProfileSyncUnavailableReason")]
pub fn account_profile_sync_unavailable_reason()
-> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::account_profile_sync_unavailable_reason()?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `requireAccountProfileSync`.
#[napi(js_name = "requireAccountProfileSync")]
pub fn require_account_profile_sync() -> napi::Result<()> {
    run(move || {
        port::require_account_profile_sync()?;
        Ok(())
    })
}

/// `requireHostSecretsSync`.
#[napi(js_name = "requireHostSecretsSync")]
pub fn require_host_secrets_sync() -> napi::Result<()> {
    run(move || {
        port::require_host_secrets_sync()?;
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

/// `remoteConnectionsUnavailableReason` over the binary wire.
#[napi(js_name = "remoteConnectionsUnavailableReasonWire")]
pub fn remote_connections_unavailable_reason_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state_dir: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::remote_connections_unavailable_reason(state_dir)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

/// `accountLiveActivityEnabled` over the binary wire.
#[napi(js_name = "accountLiveActivityEnabledWire")]
pub fn account_live_activity_enabled_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state_dir: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::account_live_activity_enabled(state_dir)?;
        w.bool(*out);
        Ok(())
    })
}

/// `requireRemoteConnections` over the binary wire.
#[napi(js_name = "requireRemoteConnectionsWire")]
pub fn require_remote_connections_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let state_dir: Option<String> = r.opt_js(|r| r.string16())?;
        let () = port::require_remote_connections(state_dir)?;
        Ok(())
    })
}

/// `accountProfileSyncUnavailableReason` over the binary wire.
#[napi(js_name = "accountProfileSyncUnavailableReasonWire")]
pub fn account_profile_sync_unavailable_reason_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::account_profile_sync_unavailable_reason()?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

/// `requireAccountProfileSync` over the binary wire.
#[napi(js_name = "requireAccountProfileSyncWire")]
pub fn require_account_profile_sync_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::require_account_profile_sync()?;
        Ok(())
    })
}

/// `requireHostSecretsSync` over the binary wire.
#[napi(js_name = "requireHostSecretsSyncWire")]
pub fn require_host_secrets_sync_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::require_host_secrets_sync()?;
        Ok(())
    })
}
