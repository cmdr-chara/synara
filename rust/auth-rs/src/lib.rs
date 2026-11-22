//! JS bindings for `apps/server/src/auth/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `makeEffectAuthRequest`.
#[napi(js_name = "makeEffectAuthRequest")]
pub fn make_effect_auth_request(request: String) -> napi::Result<String> {
    run(move || {
        let request = crate::js::from_json(&request)?;
        let out = port::make_effect_auth_request(request)?;
        crate::js::to_json(&out)
    })
}

/// `authErrorResponse`.
#[napi(js_name = "authErrorResponse")]
pub fn auth_error_response(error: String) -> napi::Result<String> {
    run(move || {
        let error = crate::js::from_json(&error)?;
        let out = port::auth_error_response(error)?;
        crate::js::to_json(&out)
    })
}

/// `isTrustedLocalServerToken`.
#[napi(js_name = "isTrustedLocalServerToken")]
pub fn is_trusted_local_server_token(config: String, token: Option<String>) -> napi::Result<bool> {
    run(move || {
        let config = crate::js::from_json(&config)?;
        let out = port::is_trusted_local_server_token(config, token)?;
        Ok(out)
    })
}

/// `requireRemoteSessionAdmission`.
#[napi(js_name = "requireRemoteSessionAdmission")]
pub fn require_remote_session_admission(registry: String, session: String) -> napi::Result<String> {
    run(move || {
        let registry = crate::js::from_json(&registry)?;
        let session = crate::js::from_json(&session)?;
        let out = port::require_remote_session_admission(registry, session)?;
        crate::js::to_json(&out)
    })
}

/// `resolveSessionCookieName`.
#[napi(js_name = "resolveSessionCookieName")]
pub fn resolve_session_cookie_name(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_session_cookie_name(input)?;
        Ok(out)
    })
}

/// `base64UrlEncode`.
#[napi(js_name = "base64UrlEncode")]
pub fn base64_url_encode(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::base64_url_encode(input)?;
        Ok(out)
    })
}

/// `base64UrlDecodeUtf8`.
#[napi(js_name = "base64UrlDecodeUtf8")]
pub fn base64_url_decode_utf8(input: String) -> napi::Result<String> {
    run(move || {
        let out = port::base64_url_decode_utf8(input)?;
        Ok(out)
    })
}

/// `signPayload`.
#[napi(js_name = "signPayload")]
pub fn sign_payload(payload: String, secret: String) -> napi::Result<String> {
    run(move || {
        let secret = crate::js::from_json(&secret)?;
        let out = port::sign_payload(payload, secret)?;
        Ok(out)
    })
}

/// `timingSafeEqualBase64Url`.
#[napi(js_name = "timingSafeEqualBase64Url")]
pub fn timing_safe_equal_base64_url(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let out = port::timing_safe_equal_base64_url(left, right)?;
        Ok(out)
    })
}

/// `deriveAuthClientMetadata`.
#[napi(js_name = "deriveAuthClientMetadata")]
pub fn derive_auth_client_metadata(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_auth_client_metadata(input)?;
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

static S_MAKE_EFFECT_AUTH_REQUEST_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_EFFECT_AUTH_REQUEST_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "headers",
        wire::Schema::Map16(&wire::Schema::OptJs(&wire::Schema::Str16)),
    ),
    (
        "cookies",
        wire::Schema::Map16(&wire::Schema::OptJs(&wire::Schema::Str16)),
    ),
    ("url", wire::Schema::Dyn),
]);

/// `makeEffectAuthRequest` over the binary wire.
#[napi(js_name = "makeEffectAuthRequestWire")]
pub fn make_effect_auth_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let request: serde_json::Value = wire::read_value(r, &S_MAKE_EFFECT_AUTH_REQUEST_0)?;
        let out = &port::make_effect_auth_request(request)?;
        wire::write_value(w, out, &S_MAKE_EFFECT_AUTH_REQUEST_OUT)?;
        Ok(())
    })
}

static S_AUTH_ERROR_RESPONSE_0: wire::Schema = wire::Schema::Dyn;
static S_AUTH_ERROR_RESPONSE_OUT: wire::Schema = wire::Schema::Dyn;

/// `authErrorResponse` over the binary wire.
#[napi(js_name = "authErrorResponseWire")]
pub fn auth_error_response_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let error: serde_json::Value = wire::read_value(r, &S_AUTH_ERROR_RESPONSE_0)?;
        let out = &port::auth_error_response(error)?;
        wire::write_value(w, out, &S_AUTH_ERROR_RESPONSE_OUT)?;
        Ok(())
    })
}

static S_IS_TRUSTED_LOCAL_SERVER_TOKEN_0: wire::Schema = wire::Schema::Dyn;

/// `isTrustedLocalServerToken` over the binary wire.
#[napi(js_name = "isTrustedLocalServerTokenWire")]
pub fn is_trusted_local_server_token_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: serde_json::Value = wire::read_value(r, &S_IS_TRUSTED_LOCAL_SERVER_TOKEN_0)?;
        let token: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_trusted_local_server_token(config, token)?;
        w.bool(*out);
        Ok(())
    })
}

static S_REQUIRE_REMOTE_SESSION_ADMISSION_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_REMOTE_SESSION_ADMISSION_1: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_REMOTE_SESSION_ADMISSION_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireRemoteSessionAdmission` over the binary wire.
#[napi(js_name = "requireRemoteSessionAdmissionWire")]
pub fn require_remote_session_admission_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let registry: serde_json::Value =
            wire::read_value(r, &S_REQUIRE_REMOTE_SESSION_ADMISSION_0)?;
        let session: serde_json::Value =
            wire::read_value(r, &S_REQUIRE_REMOTE_SESSION_ADMISSION_1)?;
        let out = &port::require_remote_session_admission(registry, session)?;
        wire::write_value(w, out, &S_REQUIRE_REMOTE_SESSION_ADMISSION_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_SESSION_COOKIE_NAME_0: wire::Schema = wire::Schema::Dyn;

/// `resolveSessionCookieName` over the binary wire.
#[napi(js_name = "resolveSessionCookieNameWire")]
pub fn resolve_session_cookie_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_SESSION_COOKIE_NAME_0)?;
        let out = &port::resolve_session_cookie_name(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_BASE64_URL_ENCODE_0: wire::Schema = wire::Schema::Dyn;

/// `base64UrlEncode` over the binary wire.
#[napi(js_name = "base64UrlEncodeWire")]
pub fn base64_url_encode_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BASE64_URL_ENCODE_0)?;
        let out = &port::base64_url_encode(input)?;
        w.string16(out);
        Ok(())
    })
}

/// `base64UrlDecodeUtf8` over the binary wire.
#[napi(js_name = "base64UrlDecodeUtf8Wire")]
pub fn base64_url_decode_utf8_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: String = r.string16()?;
        let out = &port::base64_url_decode_utf8(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_SIGN_PAYLOAD_1: wire::Schema = wire::Schema::Dyn;

/// `signPayload` over the binary wire.
#[napi(js_name = "signPayloadWire")]
pub fn sign_payload_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: String = r.string16()?;
        let secret: serde_json::Value = wire::read_value(r, &S_SIGN_PAYLOAD_1)?;
        let out = &port::sign_payload(payload, secret)?;
        w.string16(out);
        Ok(())
    })
}

/// `timingSafeEqualBase64Url` over the binary wire.
#[napi(js_name = "timingSafeEqualBase64UrlWire")]
pub fn timing_safe_equal_base64_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: String = r.string16()?;
        let right: String = r.string16()?;
        let out = &port::timing_safe_equal_base64_url(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

static S_DERIVE_AUTH_CLIENT_METADATA_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_AUTH_CLIENT_METADATA_OUT: wire::Schema = wire::Schema::Dyn;

/// `deriveAuthClientMetadata` over the binary wire.
#[napi(js_name = "deriveAuthClientMetadataWire")]
pub fn derive_auth_client_metadata_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DERIVE_AUTH_CLIENT_METADATA_0)?;
        let out = &port::derive_auth_client_metadata(input)?;
        wire::write_value(w, out, &S_DERIVE_AUTH_CLIENT_METADATA_OUT)?;
        Ok(())
    })
}
