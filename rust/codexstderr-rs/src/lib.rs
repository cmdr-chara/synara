//! JS bindings for `packages/shared/src/codexStderr.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `normalizeCodexProcessLine`.
#[napi(js_name = "normalizeCodexProcessLine")]
pub fn normalize_codex_process_line(line: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_codex_process_line(line)?;
        Ok(out)
    })
}

/// `isIgnorableCodexProcessLine`.
#[napi(js_name = "isIgnorableCodexProcessLine")]
pub fn is_ignorable_codex_process_line(line: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_ignorable_codex_process_line(line)?;
        Ok(out)
    })
}

/// `isCodexUnstableFeaturesWarning`.
#[napi(js_name = "isCodexUnstableFeaturesWarning")]
pub fn is_codex_unstable_features_warning(message: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_codex_unstable_features_warning(message)?;
        Ok(out)
    })
}

/// `isCodexMultiAgentDeprecationWarning`.
#[napi(js_name = "isCodexMultiAgentDeprecationWarning")]
pub fn is_codex_multi_agent_deprecation_warning(message: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_codex_multi_agent_deprecation_warning(message)?;
        Ok(out)
    })
}

/// `isCodexUserMcpOAuthStartupError`.
#[napi(js_name = "isCodexUserMcpOAuthStartupError")]
pub fn is_codex_user_mcp_o_auth_startup_error(server: String, error: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_codex_user_mcp_o_auth_startup_error(server, error)?;
        Ok(out)
    })
}

/// `readCodexMcpSubscriptionError`.
#[napi(js_name = "readCodexMcpSubscriptionError")]
pub fn read_codex_mcp_subscription_error(
    message: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::read_codex_mcp_subscription_error(message)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `normalizeCodexUserVisibleErrorMessage`.
#[napi(js_name = "normalizeCodexUserVisibleErrorMessage")]
pub fn normalize_codex_user_visible_error_message(raw_message: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_codex_user_visible_error_message(raw_message)?;
        Ok(out)
    })
}

/// `classifyCodexStderr`.
#[napi(js_name = "classifyCodexStderr")]
pub fn classify_codex_stderr(
    line: String,
    mirrored_user_config_path: Option<String>,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::classify_codex_stderr(line, mirrored_user_config_path, options)?;
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

/// `normalizeCodexProcessLine` over the binary wire.
#[napi(js_name = "normalizeCodexProcessLineWire")]
pub fn normalize_codex_process_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let out = &port::normalize_codex_process_line(line)?;
        w.string16(out);
        Ok(())
    })
}

/// `isIgnorableCodexProcessLine` over the binary wire.
#[napi(js_name = "isIgnorableCodexProcessLineWire")]
pub fn is_ignorable_codex_process_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let out = &port::is_ignorable_codex_process_line(line)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isCodexUnstableFeaturesWarning` over the binary wire.
#[napi(js_name = "isCodexUnstableFeaturesWarningWire")]
pub fn is_codex_unstable_features_warning_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: String = r.string16()?;
        let out = &port::is_codex_unstable_features_warning(message)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isCodexMultiAgentDeprecationWarning` over the binary wire.
#[napi(js_name = "isCodexMultiAgentDeprecationWarningWire")]
pub fn is_codex_multi_agent_deprecation_warning_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: String = r.string16()?;
        let out = &port::is_codex_multi_agent_deprecation_warning(message)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isCodexUserMcpOAuthStartupError` over the binary wire.
#[napi(js_name = "isCodexUserMcpOAuthStartupErrorWire")]
pub fn is_codex_user_mcp_o_auth_startup_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let server: String = r.string16()?;
        let error: String = r.string16()?;
        let out = &port::is_codex_user_mcp_o_auth_startup_error(server, error)?;
        w.bool(*out);
        Ok(())
    })
}

/// `readCodexMcpSubscriptionError` over the binary wire.
#[napi(js_name = "readCodexMcpSubscriptionErrorWire")]
pub fn read_codex_mcp_subscription_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: String = r.string16()?;
        let out = &port::read_codex_mcp_subscription_error(message)?;
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

/// `normalizeCodexUserVisibleErrorMessage` over the binary wire.
#[napi(js_name = "normalizeCodexUserVisibleErrorMessageWire")]
pub fn normalize_codex_user_visible_error_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_message: String = r.string16()?;
        let out = &port::normalize_codex_user_visible_error_message(raw_message)?;
        w.string16(out);
        Ok(())
    })
}

static S_CLASSIFY_CODEX_STDERR_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifyCodexStderr` over the binary wire.
#[napi(js_name = "classifyCodexStderrWire")]
pub fn classify_codex_stderr_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let mirrored_user_config_path: Option<String> = r.opt_js(|r| r.string16())?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::classify_codex_stderr(line, mirrored_user_config_path, options)?;
        wire::write_value(w, out, &S_CLASSIFY_CODEX_STDERR_OUT)?;
        Ok(())
    })
}
