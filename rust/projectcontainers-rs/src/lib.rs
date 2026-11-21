//! JS bindings for `packages/shared/src/projectContainers.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveChatContainerWorkspaceRoot`.
#[napi(js_name = "resolveChatContainerWorkspaceRoot")]
pub fn resolve_chat_container_workspace_root(paths: String) -> napi::Result<String> {
    run(move || {
        let paths = crate::js::from_json(&paths)?;
        let out = port::resolve_chat_container_workspace_root(paths)?;
        crate::js::to_json(&out)
    })
}

/// `matchesLegacyHomeChatWorkspaceRoot`.
#[napi(js_name = "matchesLegacyHomeChatWorkspaceRoot")]
pub fn matches_legacy_home_chat_workspace_root(
    workspace_root: String,
    paths: String,
    options: String,
) -> napi::Result<bool> {
    run(move || {
        let workspace_root = crate::js::from_json(&workspace_root)?;
        let paths = crate::js::from_json(&paths)?;
        let options = crate::js::from_json(&options)?;
        let out = port::matches_legacy_home_chat_workspace_root(workspace_root, paths, options)?;
        Ok(out)
    })
}

/// `isLegacyHomeChatContainerRow`.
#[napi(js_name = "isLegacyHomeChatContainerRow")]
pub fn is_legacy_home_chat_container_row(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::is_legacy_home_chat_container_row(input)?;
        Ok(out)
    })
}

/// `isGroupContainerKind`.
#[napi(js_name = "isGroupContainerKind")]
pub fn is_group_container_kind(kind: String) -> napi::Result<bool> {
    run(move || {
        let kind = crate::js::from_json(&kind)?;
        let out = port::is_group_container_kind(kind)?;
        Ok(out)
    })
}

/// `isOrdinaryProjectRow`.
#[napi(js_name = "isOrdinaryProjectRow")]
pub fn is_ordinary_project_row(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::is_ordinary_project_row(input)?;
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

static S_RESOLVE_CHAT_CONTAINER_WORKSPACE_ROOT_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_CHAT_CONTAINER_WORKSPACE_ROOT_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveChatContainerWorkspaceRoot` over the binary wire.
#[napi(js_name = "resolveChatContainerWorkspaceRootWire")]
pub fn resolve_chat_container_workspace_root_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let paths: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_CHAT_CONTAINER_WORKSPACE_ROOT_0)?;
        let out = &port::resolve_chat_container_workspace_root(paths)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_RESOLVE_CHAT_CONTAINER_WORKSPACE_ROOT_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_MATCHES_LEGACY_HOME_CHAT_WORKSPACE_ROOT_0: wire::Schema = wire::Schema::Dyn;
static S_MATCHES_LEGACY_HOME_CHAT_WORKSPACE_ROOT_1: wire::Schema = wire::Schema::Dyn;
static S_MATCHES_LEGACY_HOME_CHAT_WORKSPACE_ROOT_2: wire::Schema = wire::Schema::Dyn;

/// `matchesLegacyHomeChatWorkspaceRoot` over the binary wire.
#[napi(js_name = "matchesLegacyHomeChatWorkspaceRootWire")]
pub fn matches_legacy_home_chat_workspace_root_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let workspace_root: serde_json::Value =
            wire::read_value(r, &S_MATCHES_LEGACY_HOME_CHAT_WORKSPACE_ROOT_0)?;
        let paths: serde_json::Value =
            wire::read_value(r, &S_MATCHES_LEGACY_HOME_CHAT_WORKSPACE_ROOT_1)?;
        let options: serde_json::Value =
            wire::read_value(r, &S_MATCHES_LEGACY_HOME_CHAT_WORKSPACE_ROOT_2)?;
        let out = &port::matches_legacy_home_chat_workspace_root(workspace_root, paths, options)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_LEGACY_HOME_CHAT_CONTAINER_ROW_0: wire::Schema = wire::Schema::Dyn;

/// `isLegacyHomeChatContainerRow` over the binary wire.
#[napi(js_name = "isLegacyHomeChatContainerRowWire")]
pub fn is_legacy_home_chat_container_row_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_IS_LEGACY_HOME_CHAT_CONTAINER_ROW_0)?;
        let out = &port::is_legacy_home_chat_container_row(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_GROUP_CONTAINER_KIND_0: wire::Schema = wire::Schema::Dyn;

/// `isGroupContainerKind` over the binary wire.
#[napi(js_name = "isGroupContainerKindWire")]
pub fn is_group_container_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let kind: serde_json::Value = wire::read_value(r, &S_IS_GROUP_CONTAINER_KIND_0)?;
        let out = &port::is_group_container_kind(kind)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_ORDINARY_PROJECT_ROW_0: wire::Schema = wire::Schema::Dyn;

/// `isOrdinaryProjectRow` over the binary wire.
#[napi(js_name = "isOrdinaryProjectRowWire")]
pub fn is_ordinary_project_row_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_IS_ORDINARY_PROJECT_ROW_0)?;
        let out = &port::is_ordinary_project_row(input)?;
        w.bool(*out);
        Ok(())
    })
}
