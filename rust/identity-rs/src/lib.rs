//! JS bindings for `apps/api/src/identity/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `hostEnvironmentLockKey`.
#[napi(js_name = "hostEnvironmentLockKey")]
pub fn host_environment_lock_key(environment_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::host_environment_lock_key(environment_id)?;
        Ok(out)
    })
}

/// `createHostGrantIssuer`.
#[napi(js_name = "createHostGrantIssuer")]
pub fn create_host_grant_issuer(signing: String) -> napi::Result<String> {
    run(move || {
        let signing = crate::js::from_json(&signing)?;
        let out = port::create_host_grant_issuer(signing)?;
        crate::js::to_json(&out)
    })
}

/// `hostRecordFromRow`.
#[napi(js_name = "hostRecordFromRow")]
pub fn host_record_from_row(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::host_record_from_row(row)?;
        crate::js::to_json(&out)
    })
}

/// `isHostProofAuthorization`.
#[napi(js_name = "isHostProofAuthorization")]
pub fn is_host_proof_authorization(authorization: Option<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::is_host_proof_authorization(authorization)?;
        Ok(out)
    })
}

/// `toAccountHost`.
#[napi(js_name = "toAccountHost")]
pub fn to_account_host(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::to_account_host(row)?;
        crate::js::to_json(&out)
    })
}

/// `deleteHostSecretRows`.
#[napi(js_name = "deleteHostSecretRows")]
pub async fn delete_host_secret_rows(writer: String, host_id: String) -> napi::Result<()> {
    run_async(async move {
        let writer = crate::js::from_json(&writer)?;
        port::delete_host_secret_rows(writer, host_id).await?;
        Ok(())
    })
    .await
}

/// `createHostSecretStore`.
#[napi(js_name = "createHostSecretStore")]
pub fn create_host_secret_store(db: String) -> napi::Result<String> {
    run(move || {
        let db = crate::js::from_json(&db)?;
        let out = port::create_host_secret_store(db)?;
        crate::js::to_json(&out)
    })
}

/// `clearOrgCache`.
#[napi(js_name = "clearOrgCache")]
pub fn clear_org_cache() -> napi::Result<()> {
    run(move || {
        port::clear_org_cache()?;
        Ok(())
    })
}

/// `invalidateOrgCacheForOrganization`.
#[napi(js_name = "invalidateOrgCacheForOrganization")]
pub fn invalidate_org_cache_for_organization(org_id: String) -> napi::Result<()> {
    run(move || {
        port::invalidate_org_cache_for_organization(org_id)?;
        Ok(())
    })
}

/// `personalOrgName`.
#[napi(js_name = "personalOrgName")]
pub fn personal_org_name(email: String) -> napi::Result<String> {
    run(move || {
        let out = port::personal_org_name(email)?;
        Ok(out)
    })
}

/// `writeRevocationEvents`.
#[napi(js_name = "writeRevocationEvents")]
pub async fn write_revocation_events(writer: String, values: String) -> napi::Result<()> {
    run_async(async move {
        let writer = crate::js::from_json(&writer)?;
        let values = crate::js::from_json(&values)?;
        port::write_revocation_events(writer, values).await?;
        Ok(())
    })
    .await
}

/// `verifyBoundedJwt`.
#[napi(js_name = "verifyBoundedJwt")]
pub async fn verify_bounded_jwt(
    token: String,
    key: String,
    options: String,
) -> napi::Result<String> {
    run_async(async move {
        let key = crate::js::from_json(&key)?;
        let options = crate::js::from_json(&options)?;
        let out = port::verify_bounded_jwt(token, key, options).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `verifyJwtWithEmbeddedJwk`.
#[napi(js_name = "verifyJwtWithEmbeddedJwk")]
pub async fn verify_jwt_with_embedded_jwk(token: String, options: String) -> napi::Result<String> {
    run_async(async move {
        let options = crate::js::from_json(&options)?;
        let out = port::verify_jwt_with_embedded_jwk(token, options).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `publicJwkThumbprint`.
#[napi(js_name = "publicJwkThumbprint")]
pub async fn public_jwk_thumbprint(jwk: String) -> napi::Result<String> {
    run_async(async move {
        let jwk = crate::js::from_json(&jwk)?;
        let out = port::public_jwk_thumbprint(jwk).await?;
        Ok(out)
    })
    .await
}

/// `createApiSigningService`.
#[napi(js_name = "createApiSigningService")]
pub async fn create_api_signing_service(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::create_api_signing_service(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `clearUserCache`.
#[napi(js_name = "clearUserCache")]
pub fn clear_user_cache() -> napi::Result<()> {
    run(move || {
        port::clear_user_cache()?;
        Ok(())
    })
}

/// `invalidateUserCache`.
#[napi(js_name = "invalidateUserCache")]
pub fn invalidate_user_cache(user_id: String) -> napi::Result<()> {
    run(move || {
        port::invalidate_user_cache(user_id)?;
        Ok(())
    })
}

/// `userCacheSize`.
#[napi(js_name = "userCacheSize")]
pub fn user_cache_size() -> napi::Result<f64> {
    run(move || {
        let out = port::user_cache_size()?;
        Ok(out)
    })
}

/// `classifyMagicAuthFailure`.
#[napi(js_name = "classifyMagicAuthFailure")]
pub fn classify_magic_auth_failure(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::classify_magic_auth_failure(raw)?;
        crate::js::to_json(&out)
    })
}

/// `classifyAuthorizationCodeFailure`.
#[napi(js_name = "classifyAuthorizationCodeFailure")]
pub fn classify_authorization_code_failure(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::classify_authorization_code_failure(raw)?;
        crate::js::to_json(&out)
    })
}

/// `parseWorkosIdentityMap`.
#[napi(js_name = "parseWorkosIdentityMap")]
pub fn parse_workos_identity_map(raw: String, client_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_workos_identity_map(raw, client_id)?;
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

/// `hostEnvironmentLockKey` over the binary wire.
#[napi(js_name = "hostEnvironmentLockKeyWire")]
pub fn host_environment_lock_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let environment_id: String = r.string16()?;
        let out = &port::host_environment_lock_key(environment_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_CREATE_HOST_GRANT_ISSUER_0: wire::Schema = wire::Schema::Dyn;
static S_CREATE_HOST_GRANT_ISSUER_OUT: wire::Schema = wire::Schema::Dyn;

/// `createHostGrantIssuer` over the binary wire.
#[napi(js_name = "createHostGrantIssuerWire")]
pub fn create_host_grant_issuer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let signing: serde_json::Value = wire::read_value(r, &S_CREATE_HOST_GRANT_ISSUER_0)?;
        let out = &port::create_host_grant_issuer(signing)?;
        wire::write_value(w, out, &S_CREATE_HOST_GRANT_ISSUER_OUT)?;
        Ok(())
    })
}

static S_HOST_RECORD_FROM_ROW_0: wire::Schema = wire::Schema::Dyn;
static S_HOST_RECORD_FROM_ROW_OUT: wire::Schema = wire::Schema::Dyn;

/// `hostRecordFromRow` over the binary wire.
#[napi(js_name = "hostRecordFromRowWire")]
pub fn host_record_from_row_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_HOST_RECORD_FROM_ROW_0)?;
        let out = &port::host_record_from_row(row)?;
        wire::write_value(w, out, &S_HOST_RECORD_FROM_ROW_OUT)?;
        Ok(())
    })
}

/// `isHostProofAuthorization` over the binary wire.
#[napi(js_name = "isHostProofAuthorizationWire")]
pub fn is_host_proof_authorization_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let authorization: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_host_proof_authorization(authorization)?;
        w.bool(*out);
        Ok(())
    })
}

static S_TO_ACCOUNT_HOST_0: wire::Schema = wire::Schema::Dyn;
static S_TO_ACCOUNT_HOST_OUT: wire::Schema = wire::Schema::Dyn;

/// `toAccountHost` over the binary wire.
#[napi(js_name = "toAccountHostWire")]
pub fn to_account_host_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_TO_ACCOUNT_HOST_0)?;
        let out = &port::to_account_host(row)?;
        wire::write_value(w, out, &S_TO_ACCOUNT_HOST_OUT)?;
        Ok(())
    })
}

static S_CREATE_HOST_SECRET_STORE_0: wire::Schema = wire::Schema::Dyn;
static S_CREATE_HOST_SECRET_STORE_OUT: wire::Schema = wire::Schema::Dyn;

/// `createHostSecretStore` over the binary wire.
#[napi(js_name = "createHostSecretStoreWire")]
pub fn create_host_secret_store_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db: serde_json::Value = wire::read_value(r, &S_CREATE_HOST_SECRET_STORE_0)?;
        let out = &port::create_host_secret_store(db)?;
        wire::write_value(w, out, &S_CREATE_HOST_SECRET_STORE_OUT)?;
        Ok(())
    })
}

/// `clearOrgCache` over the binary wire.
#[napi(js_name = "clearOrgCacheWire")]
pub fn clear_org_cache_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::clear_org_cache()?;
        Ok(())
    })
}

/// `invalidateOrgCacheForOrganization` over the binary wire.
#[napi(js_name = "invalidateOrgCacheForOrganizationWire")]
pub fn invalidate_org_cache_for_organization_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let org_id: String = r.string16()?;
        let () = port::invalidate_org_cache_for_organization(org_id)?;
        Ok(())
    })
}

/// `personalOrgName` over the binary wire.
#[napi(js_name = "personalOrgNameWire")]
pub fn personal_org_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let email: String = r.string16()?;
        let out = &port::personal_org_name(email)?;
        w.string16(out);
        Ok(())
    })
}

/// `clearUserCache` over the binary wire.
#[napi(js_name = "clearUserCacheWire")]
pub fn clear_user_cache_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::clear_user_cache()?;
        Ok(())
    })
}

/// `invalidateUserCache` over the binary wire.
#[napi(js_name = "invalidateUserCacheWire")]
pub fn invalidate_user_cache_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let user_id: String = r.string16()?;
        let () = port::invalidate_user_cache(user_id)?;
        Ok(())
    })
}

/// `userCacheSize` over the binary wire.
#[napi(js_name = "userCacheSizeWire")]
pub fn user_cache_size_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::user_cache_size()?;
        w.f64(*out);
        Ok(())
    })
}

static S_CLASSIFY_MAGIC_AUTH_FAILURE_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_MAGIC_AUTH_FAILURE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `classifyMagicAuthFailure` over the binary wire.
#[napi(js_name = "classifyMagicAuthFailureWire")]
pub fn classify_magic_auth_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: serde_json::Value = wire::read_value(r, &S_CLASSIFY_MAGIC_AUTH_FAILURE_0)?;
        let out = &port::classify_magic_auth_failure(raw)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_CLASSIFY_MAGIC_AUTH_FAILURE_OUT.inner())?;
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_CLASSIFY_AUTHORIZATION_CODE_FAILURE_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_AUTHORIZATION_CODE_FAILURE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `classifyAuthorizationCodeFailure` over the binary wire.
#[napi(js_name = "classifyAuthorizationCodeFailureWire")]
pub fn classify_authorization_code_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: serde_json::Value = wire::read_value(r, &S_CLASSIFY_AUTHORIZATION_CODE_FAILURE_0)?;
        let out = &port::classify_authorization_code_failure(raw)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_CLASSIFY_AUTHORIZATION_CODE_FAILURE_OUT.inner())?;
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_PARSE_WORKOS_IDENTITY_MAP_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseWorkosIdentityMap` over the binary wire.
#[napi(js_name = "parseWorkosIdentityMapWire")]
pub fn parse_workos_identity_map_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let client_id: String = r.string16()?;
        let out = &port::parse_workos_identity_map(raw, client_id)?;
        wire::write_value(w, out, &S_PARSE_WORKOS_IDENTITY_MAP_OUT)?;
        Ok(())
    })
}
