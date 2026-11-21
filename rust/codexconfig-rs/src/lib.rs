//! JS bindings for `packages/shared/src/codexConfig.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parseCodexConfigModelProvider`.
#[napi(js_name = "parseCodexConfigModelProvider")]
pub fn parse_codex_config_model_provider(
    content: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::parse_codex_config_model_provider(content)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `parseCodexConfigProviderEnvKey`.
#[napi(js_name = "parseCodexConfigProviderEnvKey")]
pub fn parse_codex_config_provider_env_key(
    content: String,
    provider: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::parse_codex_config_provider_env_key(content, provider)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `parseCodexConfigActiveProviderEnvKey`.
#[napi(js_name = "parseCodexConfigActiveProviderEnvKey")]
pub fn parse_codex_config_active_provider_env_key(
    content: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::parse_codex_config_active_provider_env_key(content)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `resolveCodexHome`.
#[napi(js_name = "resolveCodexHome")]
pub fn resolve_codex_home(env: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::resolve_codex_home(env)?;
        Ok(out)
    })
}

/// `readCodexConfigContent`.
#[napi(js_name = "readCodexConfigContent")]
pub fn read_codex_config_content(
    env: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::read_codex_config_content(env)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `readActiveCodexProviderEnvKey`.
#[napi(js_name = "readActiveCodexProviderEnvKey")]
pub fn read_active_codex_provider_env_key(
    env: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::read_active_codex_provider_env_key(env)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
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

/// `parseCodexConfigModelProvider` over the binary wire.
#[napi(js_name = "parseCodexConfigModelProviderWire")]
pub fn parse_codex_config_model_provider_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let content: String = r.string16()?;
        let out = &port::parse_codex_config_model_provider(content)?;
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

/// `parseCodexConfigProviderEnvKey` over the binary wire.
#[napi(js_name = "parseCodexConfigProviderEnvKeyWire")]
pub fn parse_codex_config_provider_env_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let content: String = r.string16()?;
        let provider: String = r.string16()?;
        let out = &port::parse_codex_config_provider_env_key(content, provider)?;
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

/// `parseCodexConfigActiveProviderEnvKey` over the binary wire.
#[napi(js_name = "parseCodexConfigActiveProviderEnvKeyWire")]
pub fn parse_codex_config_active_provider_env_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let content: String = r.string16()?;
        let out = &port::parse_codex_config_active_provider_env_key(content)?;
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

static S_RESOLVE_CODEX_HOME_0: wire::Schema = wire::Schema::Dyn;

/// `resolveCodexHome` over the binary wire.
#[napi(js_name = "resolveCodexHomeWire")]
pub fn resolve_codex_home_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_RESOLVE_CODEX_HOME_0)?;
        let out = &port::resolve_codex_home(env)?;
        w.string16(out);
        Ok(())
    })
}

static S_READ_CODEX_CONFIG_CONTENT_0: wire::Schema = wire::Schema::Dyn;

/// `readCodexConfigContent` over the binary wire.
#[napi(js_name = "readCodexConfigContentWire")]
pub fn read_codex_config_content_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_READ_CODEX_CONFIG_CONTENT_0)?;
        let out = &port::read_codex_config_content(env)?;
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

static S_READ_ACTIVE_CODEX_PROVIDER_ENV_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `readActiveCodexProviderEnvKey` over the binary wire.
#[napi(js_name = "readActiveCodexProviderEnvKeyWire")]
pub fn read_active_codex_provider_env_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_READ_ACTIVE_CODEX_PROVIDER_ENV_KEY_0)?;
        let out = &port::read_active_codex_provider_env_key(env)?;
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
