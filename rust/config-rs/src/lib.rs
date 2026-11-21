//! JS bindings for `apps/api/src/config.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `loadApnsConfig`.
#[napi(js_name = "loadApnsConfig")]
pub fn load_apns_config(env: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::load_apns_config(env)?;
        crate::js::to_json(&out)
    })
}

/// `assertPolarBillingConfig`.
#[napi(js_name = "assertPolarBillingConfig")]
pub fn assert_polar_billing_config(config: String) -> napi::Result<()> {
    run(move || {
        let config = crate::js::from_json(&config)?;
        port::assert_polar_billing_config(config)?;
        Ok(())
    })
}

/// `loadPolarBillingConfig`.
#[napi(js_name = "loadPolarBillingConfig")]
pub fn load_polar_billing_config(env: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::load_polar_billing_config(env)?;
        crate::js::to_json(&out)
    })
}

/// `loadCloudflareTunnelConfig`.
#[napi(js_name = "loadCloudflareTunnelConfig")]
pub fn load_cloudflare_tunnel_config(env: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::load_cloudflare_tunnel_config(env)?;
        crate::js::to_json(&out)
    })
}

/// `assertDevIdentityAllowed`.
#[napi(js_name = "assertDevIdentityAllowed")]
pub fn assert_dev_identity_allowed(env: String) -> napi::Result<()> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        port::assert_dev_identity_allowed(env)?;
        Ok(())
    })
}

/// `loadAvatarStorageConfig`.
#[napi(js_name = "loadAvatarStorageConfig")]
pub fn load_avatar_storage_config(env: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::load_avatar_storage_config(env)?;
        crate::js::to_json(&out)
    })
}

/// `loadApiConfig`.
#[napi(js_name = "loadApiConfig")]
pub fn load_api_config(env: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::load_api_config(env)?;
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

static S_LOAD_APNS_CONFIG_0: wire::Schema = wire::Schema::Dyn;
static S_LOAD_APNS_CONFIG_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("privateKey", wire::Schema::Str16),
    ("keyId", wire::Schema::Str16),
    ("teamId", wire::Schema::Str16),
    ("bundleId", wire::Schema::Str16),
    ("environment", wire::Schema::Str16),
    (
        "productionCredentials",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("privateKey", wire::Schema::Str16),
            ("keyId", wire::Schema::Str16),
        ])),
    ),
]));

/// `loadApnsConfig` over the binary wire.
#[napi(js_name = "loadApnsConfigWire")]
pub fn load_apns_config_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_LOAD_APNS_CONFIG_0)?;
        let out = &port::load_apns_config(env)?;
        wire::write_value(w, out, &S_LOAD_APNS_CONFIG_OUT)?;
        Ok(())
    })
}

static S_ASSERT_POLAR_BILLING_CONFIG_0: wire::Schema = wire::Schema::Dyn;

/// `assertPolarBillingConfig` over the binary wire.
#[napi(js_name = "assertPolarBillingConfigWire")]
pub fn assert_polar_billing_config_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let config: serde_json::Value = wire::read_value(r, &S_ASSERT_POLAR_BILLING_CONFIG_0)?;
        let () = port::assert_polar_billing_config(config)?;
        Ok(())
    })
}

static S_LOAD_POLAR_BILLING_CONFIG_0: wire::Schema = wire::Schema::Dyn;
static S_LOAD_POLAR_BILLING_CONFIG_OUT: wire::Schema = wire::Schema::Dyn;

/// `loadPolarBillingConfig` over the binary wire.
#[napi(js_name = "loadPolarBillingConfigWire")]
pub fn load_polar_billing_config_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_LOAD_POLAR_BILLING_CONFIG_0)?;
        let out = &port::load_polar_billing_config(env)?;
        wire::write_value(w, out, &S_LOAD_POLAR_BILLING_CONFIG_OUT)?;
        Ok(())
    })
}

static S_LOAD_CLOUDFLARE_TUNNEL_CONFIG_0: wire::Schema = wire::Schema::Dyn;
static S_LOAD_CLOUDFLARE_TUNNEL_CONFIG_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("accountId", wire::Schema::Str16),
        ("zoneId", wire::Schema::Str16),
        ("apiToken", wire::Schema::Str16),
        ("domain", wire::Schema::Str16),
    ]));

/// `loadCloudflareTunnelConfig` over the binary wire.
#[napi(js_name = "loadCloudflareTunnelConfigWire")]
pub fn load_cloudflare_tunnel_config_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_LOAD_CLOUDFLARE_TUNNEL_CONFIG_0)?;
        let out = &port::load_cloudflare_tunnel_config(env)?;
        wire::write_value(w, out, &S_LOAD_CLOUDFLARE_TUNNEL_CONFIG_OUT)?;
        Ok(())
    })
}

static S_ASSERT_DEV_IDENTITY_ALLOWED_0: wire::Schema = wire::Schema::Dyn;

/// `assertDevIdentityAllowed` over the binary wire.
#[napi(js_name = "assertDevIdentityAllowedWire")]
pub fn assert_dev_identity_allowed_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let env: serde_json::Value = wire::read_value(r, &S_ASSERT_DEV_IDENTITY_ALLOWED_0)?;
        let () = port::assert_dev_identity_allowed(env)?;
        Ok(())
    })
}

static S_LOAD_AVATAR_STORAGE_CONFIG_0: wire::Schema = wire::Schema::Dyn;
static S_LOAD_AVATAR_STORAGE_CONFIG_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("endpoint", wire::Schema::Str16),
    ("bucket", wire::Schema::Str16),
    ("accessKeyId", wire::Schema::Str16),
    ("secretAccessKey", wire::Schema::Str16),
    ("publicBaseUrl", wire::Schema::Str16),
    ("region", wire::Schema::Str16),
]));

/// `loadAvatarStorageConfig` over the binary wire.
#[napi(js_name = "loadAvatarStorageConfigWire")]
pub fn load_avatar_storage_config_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_LOAD_AVATAR_STORAGE_CONFIG_0)?;
        let out = &port::load_avatar_storage_config(env)?;
        wire::write_value(w, out, &S_LOAD_AVATAR_STORAGE_CONFIG_OUT)?;
        Ok(())
    })
}

static S_LOAD_API_CONFIG_0: wire::Schema = wire::Schema::Dyn;
static S_LOAD_API_CONFIG_OUT: wire::Schema = wire::Schema::Dyn;

/// `loadApiConfig` over the binary wire.
#[napi(js_name = "loadApiConfigWire")]
pub fn load_api_config_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_LOAD_API_CONFIG_0)?;
        let out = &port::load_api_config(env)?;
        wire::write_value(w, out, &S_LOAD_API_CONFIG_OUT)?;
        Ok(())
    })
}
