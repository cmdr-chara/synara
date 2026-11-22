//! JS bindings for `apps/server/src/providerUsage/providers/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parseAntigravityQuotaSummary`.
#[napi(js_name = "parseAntigravityQuotaSummary")]
pub fn parse_antigravity_quota_summary(quota: String) -> napi::Result<String> {
    run(move || {
        let quota = crate::js::from_json(&quota)?;
        let out = port::parse_antigravity_quota_summary(quota)?;
        crate::js::to_json(&out)
    })
}

/// `parseAntigravityQuota`.
#[napi(js_name = "parseAntigravityQuota")]
pub fn parse_antigravity_quota(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_antigravity_quota(input)?;
        crate::js::to_json(&out)
    })
}

/// `parseClaudeUsage`.
#[napi(js_name = "parseClaudeUsage")]
pub fn parse_claude_usage(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_claude_usage(input)?;
        crate::js::to_json(&out)
    })
}

/// `__resetClaudeUsageRateLimitState`.
#[napi(js_name = "__resetClaudeUsageRateLimitState")]
pub fn __reset_claude_usage_rate_limit_state() -> napi::Result<()> {
    run(move || {
        port::__reset_claude_usage_rate_limit_state()?;
        Ok(())
    })
}

/// `parseCodexUsage`.
#[napi(js_name = "parseCodexUsage")]
pub fn parse_codex_usage(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_codex_usage(input)?;
        crate::js::to_json(&out)
    })
}

/// `cursorStateDbPaths`.
#[napi(js_name = "cursorStateDbPaths")]
pub fn cursor_state_db_paths(ctx: String) -> napi::Result<Vec<String>> {
    run(move || {
        let ctx = crate::js::from_json(&ctx)?;
        let out = port::cursor_state_db_paths(ctx)?;
        Ok(out)
    })
}

/// `parseCursorUsage`.
#[napi(js_name = "parseCursorUsage")]
pub fn parse_cursor_usage(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_cursor_usage(input)?;
        crate::js::to_json(&out)
    })
}

/// `isCursorSafeStorageValue`.
#[napi(js_name = "isCursorSafeStorageValue")]
pub fn is_cursor_safe_storage_value(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_cursor_safe_storage_value(value)?;
        Ok(out)
    })
}

/// `unsealCursorSafeStorageValue`.
#[napi(js_name = "unsealCursorSafeStorageValue")]
pub async fn unseal_cursor_safe_storage_value(ctx: String, stored: String) -> napi::Result<String> {
    run_async(async move {
        let ctx = crate::js::from_json(&ctx)?;
        let out = port::unseal_cursor_safe_storage_value(ctx, stored).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `parseDevinUsage`.
#[napi(js_name = "parseDevinUsage")]
pub fn parse_devin_usage(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_devin_usage(input)?;
        crate::js::to_json(&out)
    })
}

/// `devinUsageCacheKey`.
#[napi(js_name = "devinUsageCacheKey")]
pub fn devin_usage_cache_key(ctx: String, auth: String) -> napi::Result<String> {
    run(move || {
        let ctx = crate::js::from_json(&ctx)?;
        let auth = crate::js::from_json(&auth)?;
        let out = port::devin_usage_cache_key(ctx, auth)?;
        Ok(out)
    })
}

/// `parseDroidUsage`.
#[napi(js_name = "parseDroidUsage")]
pub fn parse_droid_usage(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_droid_usage(input)?;
        crate::js::to_json(&out)
    })
}

/// `__resetDroidUsageRateLimitState`.
#[napi(js_name = "__resetDroidUsageRateLimitState")]
pub fn __reset_droid_usage_rate_limit_state() -> napi::Result<()> {
    run(move || {
        port::__reset_droid_usage_rate_limit_state()?;
        Ok(())
    })
}

/// `decryptDroidCredentialFile`.
#[napi(js_name = "decryptDroidCredentialFile")]
pub fn decrypt_droid_credential_file(contents: String, key: String) -> napi::Result<String> {
    run(move || {
        let key = crate::js::from_json(&key)?;
        let out = port::decrypt_droid_credential_file(contents, key)?;
        crate::js::to_json(&out)
    })
}

/// `droidLocalIdentity`.
#[napi(js_name = "droidLocalIdentity")]
pub fn droid_local_identity(credential: String) -> napi::Result<String> {
    run(move || {
        let credential = crate::js::from_json(&credential)?;
        let out = port::droid_local_identity(credential)?;
        Ok(out)
    })
}

/// `droidCredentialCacheKey`.
#[napi(js_name = "droidCredentialCacheKey")]
pub fn droid_credential_cache_key(
    ctx: String,
    resolution: String,
    api_key: String,
) -> napi::Result<String> {
    run(move || {
        let ctx = crate::js::from_json(&ctx)?;
        let resolution = crate::js::from_json(&resolution)?;
        let api_key = crate::js::from_json(&api_key)?;
        let out = port::droid_credential_cache_key(ctx, resolution, api_key)?;
        Ok(out)
    })
}

/// `readDroidSecureKey`.
#[napi(js_name = "readDroidSecureKey")]
pub async fn read_droid_secure_key(ctx: String, source: String) -> napi::Result<String> {
    run_async(async move {
        let ctx = crate::js::from_json(&ctx)?;
        let out = port::read_droid_secure_key(ctx, source).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `parseGrokAuthRecord`.
#[napi(js_name = "parseGrokAuthRecord")]
pub fn parse_grok_auth_record(value: String, path: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let path = crate::js::from_json(&path)?;
        let out = port::parse_grok_auth_record(value, path)?;
        crate::js::to_json(&out)
    })
}

/// `readGrokCachedLogin`.
#[napi(js_name = "readGrokCachedLogin")]
pub async fn read_grok_cached_login(
    env: String,
    home_dir: String,
    now_ms: String,
) -> napi::Result<String> {
    run_async(async move {
        let env = crate::js::from_json(&env)?;
        let home_dir = crate::js::from_json(&home_dir)?;
        let now_ms = crate::js::from_json(&now_ms)?;
        let out = port::read_grok_cached_login(env, home_dir, now_ms).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `parseGrokBilling`.
#[napi(js_name = "parseGrokBilling")]
pub fn parse_grok_billing(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_grok_billing(input)?;
        crate::js::to_json(&out)
    })
}

/// `parseGrokApiKeyIdentity`.
#[napi(js_name = "parseGrokApiKeyIdentity")]
pub fn parse_grok_api_key_identity(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_grok_api_key_identity(input)?;
        crate::js::to_json(&out)
    })
}

/// `parseOpenCodeGoUsage`.
#[napi(js_name = "parseOpenCodeGoUsage")]
pub fn parse_open_code_go_usage(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_open_code_go_usage(input)?;
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

static S_PARSE_ANTIGRAVITY_QUOTA_SUMMARY_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_ANTIGRAVITY_QUOTA_SUMMARY_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseAntigravityQuotaSummary` over the binary wire.
#[napi(js_name = "parseAntigravityQuotaSummaryWire")]
pub fn parse_antigravity_quota_summary_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let quota: serde_json::Value = wire::read_value(r, &S_PARSE_ANTIGRAVITY_QUOTA_SUMMARY_0)?;
        let out = &port::parse_antigravity_quota_summary(quota)?;
        wire::write_value(w, out, &S_PARSE_ANTIGRAVITY_QUOTA_SUMMARY_OUT)?;
        Ok(())
    })
}

static S_PARSE_ANTIGRAVITY_QUOTA_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_ANTIGRAVITY_QUOTA_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseAntigravityQuota` over the binary wire.
#[napi(js_name = "parseAntigravityQuotaWire")]
pub fn parse_antigravity_quota_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_ANTIGRAVITY_QUOTA_0)?;
        let out = &port::parse_antigravity_quota(input)?;
        wire::write_value(w, out, &S_PARSE_ANTIGRAVITY_QUOTA_OUT)?;
        Ok(())
    })
}

static S_PARSE_CLAUDE_USAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_CLAUDE_USAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseClaudeUsage` over the binary wire.
#[napi(js_name = "parseClaudeUsageWire")]
pub fn parse_claude_usage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_CLAUDE_USAGE_0)?;
        let out = &port::parse_claude_usage(input)?;
        wire::write_value(w, out, &S_PARSE_CLAUDE_USAGE_OUT)?;
        Ok(())
    })
}

/// `__resetClaudeUsageRateLimitState` over the binary wire.
#[napi(js_name = "__resetClaudeUsageRateLimitStateWire")]
pub fn __reset_claude_usage_rate_limit_state_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::__reset_claude_usage_rate_limit_state()?;
        Ok(())
    })
}

static S_PARSE_CODEX_USAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_CODEX_USAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseCodexUsage` over the binary wire.
#[napi(js_name = "parseCodexUsageWire")]
pub fn parse_codex_usage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_CODEX_USAGE_0)?;
        let out = &port::parse_codex_usage(input)?;
        wire::write_value(w, out, &S_PARSE_CODEX_USAGE_OUT)?;
        Ok(())
    })
}

static S_CURSOR_STATE_DB_PATHS_0: wire::Schema = wire::Schema::Dyn;

/// `cursorStateDbPaths` over the binary wire.
#[napi(js_name = "cursorStateDbPathsWire")]
pub fn cursor_state_db_paths_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ctx: serde_json::Value = wire::read_value(r, &S_CURSOR_STATE_DB_PATHS_0)?;
        let out = &port::cursor_state_db_paths(ctx)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_PARSE_CURSOR_USAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_CURSOR_USAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseCursorUsage` over the binary wire.
#[napi(js_name = "parseCursorUsageWire")]
pub fn parse_cursor_usage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_CURSOR_USAGE_0)?;
        let out = &port::parse_cursor_usage(input)?;
        wire::write_value(w, out, &S_PARSE_CURSOR_USAGE_OUT)?;
        Ok(())
    })
}

/// `isCursorSafeStorageValue` over the binary wire.
#[napi(js_name = "isCursorSafeStorageValueWire")]
pub fn is_cursor_safe_storage_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_cursor_safe_storage_value(value)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PARSE_DEVIN_USAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_DEVIN_USAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseDevinUsage` over the binary wire.
#[napi(js_name = "parseDevinUsageWire")]
pub fn parse_devin_usage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_DEVIN_USAGE_0)?;
        let out = &port::parse_devin_usage(input)?;
        wire::write_value(w, out, &S_PARSE_DEVIN_USAGE_OUT)?;
        Ok(())
    })
}

static S_DEVIN_USAGE_CACHE_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_DEVIN_USAGE_CACHE_KEY_1: wire::Schema = wire::Schema::Dyn;

/// `devinUsageCacheKey` over the binary wire.
#[napi(js_name = "devinUsageCacheKeyWire")]
pub fn devin_usage_cache_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ctx: serde_json::Value = wire::read_value(r, &S_DEVIN_USAGE_CACHE_KEY_0)?;
        let auth: serde_json::Value = wire::read_value(r, &S_DEVIN_USAGE_CACHE_KEY_1)?;
        let out = &port::devin_usage_cache_key(ctx, auth)?;
        w.string16(out);
        Ok(())
    })
}

static S_PARSE_DROID_USAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_DROID_USAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseDroidUsage` over the binary wire.
#[napi(js_name = "parseDroidUsageWire")]
pub fn parse_droid_usage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_DROID_USAGE_0)?;
        let out = &port::parse_droid_usage(input)?;
        wire::write_value(w, out, &S_PARSE_DROID_USAGE_OUT)?;
        Ok(())
    })
}

/// `__resetDroidUsageRateLimitState` over the binary wire.
#[napi(js_name = "__resetDroidUsageRateLimitStateWire")]
pub fn __reset_droid_usage_rate_limit_state_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::__reset_droid_usage_rate_limit_state()?;
        Ok(())
    })
}

static S_DECRYPT_DROID_CREDENTIAL_FILE_1: wire::Schema = wire::Schema::Dyn;
static S_DECRYPT_DROID_CREDENTIAL_FILE_OUT: wire::Schema = wire::Schema::Dyn;

/// `decryptDroidCredentialFile` over the binary wire.
#[napi(js_name = "decryptDroidCredentialFileWire")]
pub fn decrypt_droid_credential_file_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let contents: String = r.string16()?;
        let key: serde_json::Value = wire::read_value(r, &S_DECRYPT_DROID_CREDENTIAL_FILE_1)?;
        let out = &port::decrypt_droid_credential_file(contents, key)?;
        wire::write_value(w, out, &S_DECRYPT_DROID_CREDENTIAL_FILE_OUT)?;
        Ok(())
    })
}

static S_DROID_LOCAL_IDENTITY_0: wire::Schema = wire::Schema::Dyn;

/// `droidLocalIdentity` over the binary wire.
#[napi(js_name = "droidLocalIdentityWire")]
pub fn droid_local_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let credential: serde_json::Value = wire::read_value(r, &S_DROID_LOCAL_IDENTITY_0)?;
        let out = &port::droid_local_identity(credential)?;
        w.string16(out);
        Ok(())
    })
}

static S_DROID_CREDENTIAL_CACHE_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_DROID_CREDENTIAL_CACHE_KEY_1: wire::Schema = wire::Schema::Dyn;
static S_DROID_CREDENTIAL_CACHE_KEY_2: wire::Schema = wire::Schema::Dyn;

/// `droidCredentialCacheKey` over the binary wire.
#[napi(js_name = "droidCredentialCacheKeyWire")]
pub fn droid_credential_cache_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ctx: serde_json::Value = wire::read_value(r, &S_DROID_CREDENTIAL_CACHE_KEY_0)?;
        let resolution: serde_json::Value = wire::read_value(r, &S_DROID_CREDENTIAL_CACHE_KEY_1)?;
        let api_key: serde_json::Value = wire::read_value(r, &S_DROID_CREDENTIAL_CACHE_KEY_2)?;
        let out = &port::droid_credential_cache_key(ctx, resolution, api_key)?;
        w.string16(out);
        Ok(())
    })
}

static S_PARSE_GROK_AUTH_RECORD_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_GROK_AUTH_RECORD_1: wire::Schema = wire::Schema::Dyn;
static S_PARSE_GROK_AUTH_RECORD_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("accessToken", wire::Schema::Str16),
    ("email", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("plan", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("expiresAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("principalType", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("refreshToken", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("clientId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("path", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("scope", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "root",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Dyn)),
    ),
]));

/// `parseGrokAuthRecord` over the binary wire.
#[napi(js_name = "parseGrokAuthRecordWire")]
pub fn parse_grok_auth_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_PARSE_GROK_AUTH_RECORD_0)?;
        let path: serde_json::Value = wire::read_value(r, &S_PARSE_GROK_AUTH_RECORD_1)?;
        let out = &port::parse_grok_auth_record(value, path)?;
        wire::write_value(w, out, &S_PARSE_GROK_AUTH_RECORD_OUT)?;
        Ok(())
    })
}

static S_PARSE_GROK_BILLING_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_GROK_BILLING_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseGrokBilling` over the binary wire.
#[napi(js_name = "parseGrokBillingWire")]
pub fn parse_grok_billing_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_GROK_BILLING_0)?;
        let out = &port::parse_grok_billing(input)?;
        wire::write_value(w, out, &S_PARSE_GROK_BILLING_OUT)?;
        Ok(())
    })
}

static S_PARSE_GROK_API_KEY_IDENTITY_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_GROK_API_KEY_IDENTITY_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseGrokApiKeyIdentity` over the binary wire.
#[napi(js_name = "parseGrokApiKeyIdentityWire")]
pub fn parse_grok_api_key_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_GROK_API_KEY_IDENTITY_0)?;
        let out = &port::parse_grok_api_key_identity(input)?;
        wire::write_value(w, out, &S_PARSE_GROK_API_KEY_IDENTITY_OUT)?;
        Ok(())
    })
}

static S_PARSE_OPEN_CODE_GO_USAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_OPEN_CODE_GO_USAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseOpenCodeGoUsage` over the binary wire.
#[napi(js_name = "parseOpenCodeGoUsageWire")]
pub fn parse_open_code_go_usage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_OPEN_CODE_GO_USAGE_0)?;
        let out = &port::parse_open_code_go_usage(input)?;
        wire::write_value(w, out, &S_PARSE_OPEN_CODE_GO_USAGE_OUT)?;
        Ok(())
    })
}
