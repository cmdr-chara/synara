//! JS bindings for `apps/server/src/remoteTransport/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `remoteTlsIdentityPath`.
#[napi(js_name = "remoteTlsIdentityPath")]
pub fn remote_tls_identity_path(secrets_dir: String) -> napi::Result<String> {
    run(move || {
        let out = port::remote_tls_identity_path(secrets_dir)?;
        Ok(out)
    })
}

/// `remoteTlsServerName`.
#[napi(js_name = "remoteTlsServerName")]
pub fn remote_tls_server_name(environment_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::remote_tls_server_name(environment_id)?;
        Ok(out)
    })
}

/// `remoteTlsAnchor`.
#[napi(js_name = "remoteTlsAnchor")]
pub fn remote_tls_anchor(identity: String) -> napi::Result<String> {
    run(move || {
        let identity = crate::js::from_json(&identity)?;
        let out = port::remote_tls_anchor(identity)?;
        crate::js::to_json(&out)
    })
}

/// `validateRemoteTlsAnchor`.
#[napi(js_name = "validateRemoteTlsAnchor")]
pub fn validate_remote_tls_anchor(anchor: String) -> napi::Result<()> {
    run(move || {
        let anchor = crate::js::from_json(&anchor)?;
        port::validate_remote_tls_anchor(anchor)?;
        Ok(())
    })
}

/// `readRemoteTlsTrustAnchor`.
#[napi(js_name = "readRemoteTlsTrustAnchor")]
pub async fn read_remote_tls_trust_anchor(
    file_path: String,
    environment_id: String,
    now: String,
) -> napi::Result<String> {
    run_async(async move {
        let now = crate::js::from_json(&now)?;
        let out = port::read_remote_tls_trust_anchor(file_path, environment_id, now).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `remoteTlsRootNeedsRepair`.
#[napi(js_name = "remoteTlsRootNeedsRepair")]
pub fn remote_tls_root_needs_repair(identity: String, now: String) -> napi::Result<bool> {
    run(move || {
        let identity = crate::js::from_json(&identity)?;
        let now = crate::js::from_json(&now)?;
        let out = port::remote_tls_root_needs_repair(identity, now)?;
        Ok(out)
    })
}

/// `initializeRemoteTlsIdentity`.
#[napi(js_name = "initializeRemoteTlsIdentity")]
pub async fn initialize_remote_tls_identity(
    file_path: String,
    environment_id: String,
    now: String,
) -> napi::Result<String> {
    run_async(async move {
        let now = crate::js::from_json(&now)?;
        let out = port::initialize_remote_tls_identity(file_path, environment_id, now).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `loadRemoteTlsIdentity`.
#[napi(js_name = "loadRemoteTlsIdentity")]
pub async fn load_remote_tls_identity(
    file_path: String,
    environment_id: String,
    now: String,
) -> napi::Result<String> {
    run_async(async move {
        let now = crate::js::from_json(&now)?;
        let out = port::load_remote_tls_identity(file_path, environment_id, now).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `resetRemoteTlsIdentity`.
#[napi(js_name = "resetRemoteTlsIdentity")]
pub async fn reset_remote_tls_identity(
    file_path: String,
    environment_id: String,
) -> napi::Result<String> {
    run_async(async move {
        let out = port::reset_remote_tls_identity(file_path, environment_id).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `resourceRequestHeaders`.
#[napi(js_name = "resourceRequestHeaders")]
pub fn resource_request_headers(headers: String, max_body_bytes: f64) -> napi::Result<String> {
    run(move || {
        let headers = crate::js::from_json(&headers)?;
        let out = port::resource_request_headers(headers, max_body_bytes)?;
        crate::js::to_json(&out)
    })
}

/// `verifyRemoteTlsRuntime`.
#[napi(js_name = "verifyRemoteTlsRuntime")]
pub async fn verify_remote_tls_runtime() -> napi::Result<()> {
    run_async(async move {
        port::verify_remote_tls_runtime().await?;
        Ok(())
    })
    .await
}

/// `connectRemoteTls`.
#[napi(js_name = "connectRemoteTls")]
pub async fn connect_remote_tls(
    outer: String,
    anchor: String,
    signal: String,
    timeout_ms: String,
) -> napi::Result<String> {
    run_async(async move {
        let outer = crate::js::from_json(&outer)?;
        let anchor = crate::js::from_json(&anchor)?;
        let signal = crate::js::from_json(&signal)?;
        let timeout_ms = crate::js::from_json(&timeout_ms)?;
        let out = port::connect_remote_tls(outer, anchor, signal, timeout_ms).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `connectRemoteWebSocket`.
#[napi(js_name = "connectRemoteWebSocket")]
pub async fn connect_remote_web_socket(
    outer: String,
    anchor: String,
    path: String,
    signal: String,
) -> napi::Result<String> {
    run_async(async move {
        let outer = crate::js::from_json(&outer)?;
        let anchor = crate::js::from_json(&anchor)?;
        let path = crate::js::from_json(&path)?;
        let signal = crate::js::from_json(&signal)?;
        let out = port::connect_remote_web_socket(outer, anchor, path, signal).await?;
        crate::js::to_json(&out)
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

/// `remoteTlsIdentityPath` over the binary wire.
#[napi(js_name = "remoteTlsIdentityPathWire")]
pub fn remote_tls_identity_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let secrets_dir: String = r.string16()?;
        let out = &port::remote_tls_identity_path(secrets_dir)?;
        w.string16(out);
        Ok(())
    })
}

/// `remoteTlsServerName` over the binary wire.
#[napi(js_name = "remoteTlsServerNameWire")]
pub fn remote_tls_server_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let environment_id: String = r.string16()?;
        let out = &port::remote_tls_server_name(environment_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_REMOTE_TLS_ANCHOR_0: wire::Schema = wire::Schema::Dyn;
static S_REMOTE_TLS_ANCHOR_OUT: wire::Schema = wire::Schema::Obj(&[
    ("environmentId", wire::Schema::Str16),
    ("rootCertificate", wire::Schema::Str16),
    ("rootFingerprint", wire::Schema::Str16),
]);

/// `remoteTlsAnchor` over the binary wire.
#[napi(js_name = "remoteTlsAnchorWire")]
pub fn remote_tls_anchor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let identity: serde_json::Value = wire::read_value(r, &S_REMOTE_TLS_ANCHOR_0)?;
        let out = &port::remote_tls_anchor(identity)?;
        wire::write_value(w, out, &S_REMOTE_TLS_ANCHOR_OUT)?;
        Ok(())
    })
}

static S_VALIDATE_REMOTE_TLS_ANCHOR_0: wire::Schema = wire::Schema::Dyn;

/// `validateRemoteTlsAnchor` over the binary wire.
#[napi(js_name = "validateRemoteTlsAnchorWire")]
pub fn validate_remote_tls_anchor_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let anchor: serde_json::Value = wire::read_value(r, &S_VALIDATE_REMOTE_TLS_ANCHOR_0)?;
        let () = port::validate_remote_tls_anchor(anchor)?;
        Ok(())
    })
}

static S_REMOTE_TLS_ROOT_NEEDS_REPAIR_0: wire::Schema = wire::Schema::Dyn;

/// `remoteTlsRootNeedsRepair` over the binary wire.
#[napi(js_name = "remoteTlsRootNeedsRepairWire")]
pub fn remote_tls_root_needs_repair_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let identity: serde_json::Value = wire::read_value(r, &S_REMOTE_TLS_ROOT_NEEDS_REPAIR_0)?;
        let now: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::remote_tls_root_needs_repair(identity, now)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOURCE_REQUEST_HEADERS_0: wire::Schema = wire::Schema::Dyn;
static S_RESOURCE_REQUEST_HEADERS_OUT: wire::Schema = wire::Schema::Dyn;

/// `resourceRequestHeaders` over the binary wire.
#[napi(js_name = "resourceRequestHeadersWire")]
pub fn resource_request_headers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let headers: serde_json::Value = wire::read_value(r, &S_RESOURCE_REQUEST_HEADERS_0)?;
        let max_body_bytes: f64 = r.f64()?;
        let out = &port::resource_request_headers(headers, max_body_bytes)?;
        wire::write_value(w, out, &S_RESOURCE_REQUEST_HEADERS_OUT)?;
        Ok(())
    })
}
