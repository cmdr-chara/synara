//! JS bindings for `packages/shared/src/mcpConnectors.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `findMcpConnectorPreset`.
#[napi(js_name = "findMcpConnectorPreset")]
pub fn find_mcp_connector_preset(id: String) -> napi::Result<String> {
    run(move || {
        let out = port::find_mcp_connector_preset(id)?;
        crate::js::to_json(&out)
    })
}

/// `isMcpLoopbackHostname`.
#[napi(js_name = "isMcpLoopbackHostname")]
pub fn is_mcp_loopback_hostname(hostname: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_mcp_loopback_hostname(hostname)?;
        Ok(out)
    })
}

/// `normalizeMcpConnectorUrl`.
#[napi(js_name = "normalizeMcpConnectorUrl")]
pub fn normalize_mcp_connector_url(raw: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_mcp_connector_url(raw)?;
        crate::js::to_json(&out)
    })
}

/// `defaultMcpConnectorName`.
#[napi(js_name = "defaultMcpConnectorName")]
pub fn default_mcp_connector_name(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::default_mcp_connector_name(url)?;
        Ok(out)
    })
}

/// `mcpConnectorSlug`.
#[napi(js_name = "mcpConnectorSlug")]
pub fn mcp_connector_slug(name: String) -> napi::Result<String> {
    run(move || {
        let out = port::mcp_connector_slug(name)?;
        Ok(out)
    })
}

/// `mcpConnectorServerName`.
#[napi(js_name = "mcpConnectorServerName")]
pub fn mcp_connector_server_name(slug: String) -> napi::Result<String> {
    run(move || {
        let out = port::mcp_connector_server_name(slug)?;
        Ok(out)
    })
}

/// `mcpConnectorMentionPath`.
#[napi(js_name = "mcpConnectorMentionPath")]
pub fn mcp_connector_mention_path(server_name: String) -> napi::Result<String> {
    run(move || {
        let out = port::mcp_connector_mention_path(server_name)?;
        Ok(out)
    })
}

/// `isMcpConnectorMentionPath`.
#[napi(js_name = "isMcpConnectorMentionPath")]
pub fn is_mcp_connector_mention_path(path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_mcp_connector_mention_path(path)?;
        Ok(out)
    })
}

/// `serverNameFromMcpConnectorMentionPath`.
#[napi(js_name = "serverNameFromMcpConnectorMentionPath")]
pub fn server_name_from_mcp_connector_mention_path(path: String) -> napi::Result<String> {
    run(move || {
        let out = port::server_name_from_mcp_connector_mention_path(path)?;
        crate::js::to_json(&out)
    })
}

/// `providerSupportsMcpConnectors`.
#[napi(js_name = "providerSupportsMcpConnectors")]
pub fn provider_supports_mcp_connectors(provider: String) -> napi::Result<bool> {
    run(move || {
        let out = port::provider_supports_mcp_connectors(provider)?;
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

static S_FIND_MCP_CONNECTOR_PRESET_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("category", wire::Schema::Dyn),
    ("description", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("docsUrl", wire::Schema::Str16),
    ("requiresOwnApp", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("appSetupUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `findMcpConnectorPreset` over the binary wire.
#[napi(js_name = "findMcpConnectorPresetWire")]
pub fn find_mcp_connector_preset_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: String = r.string16()?;
        let out = &port::find_mcp_connector_preset(id)?;
        wire::write_value(w, out, &S_FIND_MCP_CONNECTOR_PRESET_OUT)?;
        Ok(())
    })
}

/// `isMcpLoopbackHostname` over the binary wire.
#[napi(js_name = "isMcpLoopbackHostnameWire")]
pub fn is_mcp_loopback_hostname_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let hostname: String = r.string16()?;
        let out = &port::is_mcp_loopback_hostname(hostname)?;
        w.bool(*out);
        Ok(())
    })
}

static S_NORMALIZE_MCP_CONNECTOR_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeMcpConnectorUrl` over the binary wire.
#[napi(js_name = "normalizeMcpConnectorUrlWire")]
pub fn normalize_mcp_connector_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::normalize_mcp_connector_url(raw)?;
        wire::write_value(w, out, &S_NORMALIZE_MCP_CONNECTOR_URL_OUT)?;
        Ok(())
    })
}

/// `defaultMcpConnectorName` over the binary wire.
#[napi(js_name = "defaultMcpConnectorNameWire")]
pub fn default_mcp_connector_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::default_mcp_connector_name(url)?;
        w.string16(out);
        Ok(())
    })
}

/// `mcpConnectorSlug` over the binary wire.
#[napi(js_name = "mcpConnectorSlugWire")]
pub fn mcp_connector_slug_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: String = r.string16()?;
        let out = &port::mcp_connector_slug(name)?;
        w.string16(out);
        Ok(())
    })
}

/// `mcpConnectorServerName` over the binary wire.
#[napi(js_name = "mcpConnectorServerNameWire")]
pub fn mcp_connector_server_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let slug: String = r.string16()?;
        let out = &port::mcp_connector_server_name(slug)?;
        w.string16(out);
        Ok(())
    })
}

/// `mcpConnectorMentionPath` over the binary wire.
#[napi(js_name = "mcpConnectorMentionPathWire")]
pub fn mcp_connector_mention_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let server_name: String = r.string16()?;
        let out = &port::mcp_connector_mention_path(server_name)?;
        w.string16(out);
        Ok(())
    })
}

/// `isMcpConnectorMentionPath` over the binary wire.
#[napi(js_name = "isMcpConnectorMentionPathWire")]
pub fn is_mcp_connector_mention_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let out = &port::is_mcp_connector_mention_path(path)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SERVER_NAME_FROM_MCP_CONNECTOR_MENTION_PATH_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `serverNameFromMcpConnectorMentionPath` over the binary wire.
#[napi(js_name = "serverNameFromMcpConnectorMentionPathWire")]
pub fn server_name_from_mcp_connector_mention_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let out = &port::server_name_from_mcp_connector_mention_path(path)?;
        wire::write_value(w, out, &S_SERVER_NAME_FROM_MCP_CONNECTOR_MENTION_PATH_OUT)?;
        Ok(())
    })
}

/// `providerSupportsMcpConnectors` over the binary wire.
#[napi(js_name = "providerSupportsMcpConnectorsWire")]
pub fn provider_supports_mcp_connectors_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: String = r.string16()?;
        let out = &port::provider_supports_mcp_connectors(provider)?;
        w.bool(*out);
        Ok(())
    })
}
