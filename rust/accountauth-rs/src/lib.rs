//! JS bindings for `apps/server/src/accountAuth.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `accountApiIssuer`.
#[napi(js_name = "accountApiIssuer")]
pub fn account_api_issuer(account_url: String) -> napi::Result<String> {
    run(move || {
        let out = port::account_api_issuer(account_url)?;
        Ok(out)
    })
}

/// `accountStateDirectory`.
#[napi(js_name = "accountStateDirectory")]
pub fn account_state_directory(base_dir: String, dev_url: String) -> napi::Result<String> {
    run(move || {
        let dev_url = crate::js::from_json(&dev_url)?;
        let out = port::account_state_directory(base_dir, dev_url)?;
        Ok(out)
    })
}

/// `accountCredentialsPath`.
#[napi(js_name = "accountCredentialsPath")]
pub fn account_credentials_path(base_dir: String) -> napi::Result<String> {
    run(move || {
        let out = port::account_credentials_path(base_dir)?;
        Ok(out)
    })
}

/// `readAccountFile`.
#[napi(js_name = "readAccountFile")]
pub async fn read_account_file(base_dir: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::read_account_file(base_dir).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `readAccountCredentials`.
#[napi(js_name = "readAccountCredentials")]
pub async fn read_account_credentials(base_dir: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::read_account_credentials(base_dir).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `writeAccountCredentials`.
#[napi(js_name = "writeAccountCredentials")]
pub async fn write_account_credentials(
    base_dir: String,
    credentials: String,
) -> napi::Result<String> {
    run_async(async move {
        let credentials = crate::js::from_json(&credentials)?;
        let out = port::write_account_credentials(base_dir, credentials).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `deleteAccountCredentials`.
#[napi(js_name = "deleteAccountCredentials")]
pub async fn delete_account_credentials(base_dir: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::delete_account_credentials(base_dir).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `resolveAccountUrl`.
#[napi(js_name = "resolveAccountUrl")]
pub fn resolve_account_url(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_account_url(input)?;
        crate::js::to_json(&out)
    })
}

/// `toAccountHostPlatform`.
#[napi(js_name = "toAccountHostPlatform")]
pub fn to_account_host_platform(platform: String) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let out = port::to_account_host_platform(platform)?;
        crate::js::to_json(&out)
    })
}

/// `resolveLanEndpoints`.
#[napi(js_name = "resolveLanEndpoints")]
pub async fn resolve_lan_endpoints(base_dir: String, dev_url: String) -> napi::Result<String> {
    run_async(async move {
        let dev_url = crate::js::from_json(&dev_url)?;
        let out = port::resolve_lan_endpoints(base_dir, dev_url).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `logAccountSession`.
#[napi(js_name = "logAccountSession")]
pub fn log_account_session(
    base_dir: String,
    reason: String,
    detail: String,
) -> napi::Result<String> {
    run(move || {
        let reason = crate::js::from_json(&reason)?;
        let detail = crate::js::from_json(&detail)?;
        let out = port::log_account_session(base_dir, reason, detail)?;
        crate::js::to_json(&out)
    })
}

/// `recordRemoteBillingPolicy`.
#[napi(js_name = "recordRemoteBillingPolicy")]
pub async fn record_remote_billing_policy(
    base_dir: String,
    expected: String,
    enforced: bool,
) -> napi::Result<bool> {
    run_async(async move {
        let expected = crate::js::from_json(&expected)?;
        let out = port::record_remote_billing_policy(base_dir, expected, enforced).await?;
        Ok(out)
    })
    .await
}

/// `selectOrganization`.
#[napi(js_name = "selectOrganization")]
pub async fn select_organization(organizations: String) -> napi::Result<String> {
    run_async(async move {
        let organizations = crate::js::from_json(&organizations)?;
        let out = port::select_organization(organizations).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `resolveAuthLoginAccountUrl`.
#[napi(js_name = "resolveAuthLoginAccountUrl")]
pub async fn resolve_auth_login_account_url(options: String) -> napi::Result<String> {
    run_async(async move {
        let options = crate::js::from_json(&options)?;
        let out = port::resolve_auth_login_account_url(options).await?;
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

/// `accountApiIssuer` over the binary wire.
#[napi(js_name = "accountApiIssuerWire")]
pub fn account_api_issuer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let account_url: String = r.string16()?;
        let out = &port::account_api_issuer(account_url)?;
        w.string16(out);
        Ok(())
    })
}

static S_ACCOUNT_STATE_DIRECTORY_1: wire::Schema = wire::Schema::Dyn;

/// `accountStateDirectory` over the binary wire.
#[napi(js_name = "accountStateDirectoryWire")]
pub fn account_state_directory_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_dir: String = r.string16()?;
        let dev_url: serde_json::Value = wire::read_value(r, &S_ACCOUNT_STATE_DIRECTORY_1)?;
        let out = &port::account_state_directory(base_dir, dev_url)?;
        w.string16(out);
        Ok(())
    })
}

/// `accountCredentialsPath` over the binary wire.
#[napi(js_name = "accountCredentialsPathWire")]
pub fn account_credentials_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_dir: String = r.string16()?;
        let out = &port::account_credentials_path(base_dir)?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_ACCOUNT_URL_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_ACCOUNT_URL_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveAccountUrl` over the binary wire.
#[napi(js_name = "resolveAccountUrlWire")]
pub fn resolve_account_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_ACCOUNT_URL_0)?;
        let out = &port::resolve_account_url(input)?;
        wire::write_value(w, out, &S_RESOLVE_ACCOUNT_URL_OUT)?;
        Ok(())
    })
}

static S_TO_ACCOUNT_HOST_PLATFORM_0: wire::Schema = wire::Schema::Dyn;
static S_TO_ACCOUNT_HOST_PLATFORM_OUT: wire::Schema = wire::Schema::Dyn;

/// `toAccountHostPlatform` over the binary wire.
#[napi(js_name = "toAccountHostPlatformWire")]
pub fn to_account_host_platform_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: serde_json::Value = wire::read_value(r, &S_TO_ACCOUNT_HOST_PLATFORM_0)?;
        let out = &port::to_account_host_platform(platform)?;
        wire::write_value(w, out, &S_TO_ACCOUNT_HOST_PLATFORM_OUT)?;
        Ok(())
    })
}

static S_LOG_ACCOUNT_SESSION_1: wire::Schema = wire::Schema::Dyn;
static S_LOG_ACCOUNT_SESSION_OUT: wire::Schema = wire::Schema::Dyn;

/// `logAccountSession` over the binary wire.
#[napi(js_name = "logAccountSessionWire")]
pub fn log_account_session_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_dir: String = r.string16()?;
        let reason: serde_json::Value = wire::read_value(r, &S_LOG_ACCOUNT_SESSION_1)?;
        let detail: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::log_account_session(base_dir, reason, detail)?;
        wire::write_value(w, out, &S_LOG_ACCOUNT_SESSION_OUT)?;
        Ok(())
    })
}
