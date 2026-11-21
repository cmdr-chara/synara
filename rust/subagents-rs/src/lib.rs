//! JS bindings for `packages/shared/src/subagents.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isWorkerTierSubagentRole`.
#[napi(js_name = "isWorkerTierSubagentRole")]
pub fn is_worker_tier_subagent_role(role: Option<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::is_worker_tier_subagent_role(role)?;
        Ok(out)
    })
}

/// `decodeSubagentReceiverThreadIds`.
#[napi(js_name = "decodeSubagentReceiverThreadIds")]
pub fn decode_subagent_receiver_thread_ids(item: String) -> napi::Result<Vec<String>> {
    run(move || {
        let item = crate::js::from_json(&item)?;
        let out = port::decode_subagent_receiver_thread_ids(item)?;
        Ok(out)
    })
}

/// `decodeSubagentReceiverAgents`.
#[napi(js_name = "decodeSubagentReceiverAgents")]
pub fn decode_subagent_receiver_agents(
    item: String,
    fallback_thread_ids: Vec<String>,
) -> napi::Result<String> {
    run(move || {
        let item = crate::js::from_json(&item)?;
        let out = port::decode_subagent_receiver_agents(item, fallback_thread_ids)?;
        crate::js::to_json(&out)
    })
}

/// `decodeSubagentAgentStates`.
#[napi(js_name = "decodeSubagentAgentStates")]
pub fn decode_subagent_agent_states(item: String) -> napi::Result<String> {
    run(move || {
        let item = crate::js::from_json(&item)?;
        let out = port::decode_subagent_agent_states(item)?;
        crate::js::to_json(&out)
    })
}

/// `collectSubagentProviderThreadIds`.
#[napi(js_name = "collectSubagentProviderThreadIds")]
pub fn collect_subagent_provider_thread_ids(item: String) -> napi::Result<Vec<String>> {
    run(move || {
        let item = crate::js::from_json(&item)?;
        let out = port::collect_subagent_provider_thread_ids(item)?;
        Ok(out)
    })
}

/// `extractSubagentIdentityHints`.
#[napi(js_name = "extractSubagentIdentityHints")]
pub fn extract_subagent_identity_hints(item: String) -> napi::Result<String> {
    run(move || {
        let item = crate::js::from_json(&item)?;
        let out = port::extract_subagent_identity_hints(item)?;
        crate::js::to_json(&out)
    })
}

/// `buildSubagentIdentityDirectory`.
#[napi(js_name = "buildSubagentIdentityDirectory")]
pub fn build_subagent_identity_directory(hints: String) -> napi::Result<String> {
    run(move || {
        let hints = crate::js::from_json(&hints)?;
        let out = port::build_subagent_identity_directory(hints)?;
        crate::js::to_json(&out)
    })
}

/// `resolveSubagentIdentityFromDirectory`.
#[napi(js_name = "resolveSubagentIdentityFromDirectory")]
pub fn resolve_subagent_identity_from_directory(
    directory: String,
    input: String,
) -> napi::Result<String> {
    run(move || {
        let directory = crate::js::from_json(&directory)?;
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_subagent_identity_from_directory(directory, input)?;
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

/// `isWorkerTierSubagentRole` over the binary wire.
#[napi(js_name = "isWorkerTierSubagentRoleWire")]
pub fn is_worker_tier_subagent_role_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let role: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_worker_tier_subagent_role(role)?;
        w.bool(*out);
        Ok(())
    })
}

/// `decodeSubagentReceiverThreadIds` over the binary wire.
#[napi(js_name = "decodeSubagentReceiverThreadIdsWire")]
pub fn decode_subagent_receiver_thread_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let item: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::decode_subagent_receiver_thread_ids(item)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_DECODE_SUBAGENT_RECEIVER_AGENTS_0: wire::Schema = wire::Schema::Dyn;
static S_DECODE_SUBAGENT_RECEIVER_AGENTS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("providerThreadId", wire::Schema::Str16),
        ("agentId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("nickname", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("role", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("model", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("effort", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("background", wire::Schema::OptJs(&wire::Schema::Bool)),
        ("prompt", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "modelIsRequestedHint",
            wire::Schema::OptJs(&wire::Schema::Bool),
        ),
    ]));

/// `decodeSubagentReceiverAgents` over the binary wire.
#[napi(js_name = "decodeSubagentReceiverAgentsWire")]
pub fn decode_subagent_receiver_agents_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let item: serde_json::Value = wire::read_value(r, &S_DECODE_SUBAGENT_RECEIVER_AGENTS_0)?;
        let fallback_thread_ids: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::decode_subagent_receiver_agents(item, fallback_thread_ids)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_DECODE_SUBAGENT_RECEIVER_AGENTS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_DECODE_SUBAGENT_AGENT_STATES_OUT: wire::Schema =
    wire::Schema::Map16(&wire::Schema::Obj(&[
        ("threadId", wire::Schema::Str16),
        ("agentId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("nickname", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("role", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("model", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("prompt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("status", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `decodeSubagentAgentStates` over the binary wire.
#[napi(js_name = "decodeSubagentAgentStatesWire")]
pub fn decode_subagent_agent_states_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let item: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::decode_subagent_agent_states(item)?;
        wire::write_value(w, out, &S_DECODE_SUBAGENT_AGENT_STATES_OUT)?;
        Ok(())
    })
}

static S_COLLECT_SUBAGENT_PROVIDER_THREAD_IDS_0: wire::Schema = wire::Schema::Dyn;

/// `collectSubagentProviderThreadIds` over the binary wire.
#[napi(js_name = "collectSubagentProviderThreadIdsWire")]
pub fn collect_subagent_provider_thread_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let item: serde_json::Value =
            wire::read_value(r, &S_COLLECT_SUBAGENT_PROVIDER_THREAD_IDS_0)?;
        let out = &port::collect_subagent_provider_thread_ids(item)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_EXTRACT_SUBAGENT_IDENTITY_HINTS_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_SUBAGENT_IDENTITY_HINTS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        (
            "providerThreadId",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("agentId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("nickname", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("role", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("model", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("effort", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("background", wire::Schema::OptJs(&wire::Schema::Bool)),
        ("prompt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("status", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "modelIsRequestedHint",
            wire::Schema::OptJs(&wire::Schema::Bool),
        ),
    ]));

/// `extractSubagentIdentityHints` over the binary wire.
#[napi(js_name = "extractSubagentIdentityHintsWire")]
pub fn extract_subagent_identity_hints_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let item: serde_json::Value = wire::read_value(r, &S_EXTRACT_SUBAGENT_IDENTITY_HINTS_0)?;
        let out = &port::extract_subagent_identity_hints(item)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_EXTRACT_SUBAGENT_IDENTITY_HINTS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_BUILD_SUBAGENT_IDENTITY_DIRECTORY_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_SUBAGENT_IDENTITY_DIRECTORY_OUT: wire::Schema = wire::Schema::Obj(&[
    ("byProviderThreadId", wire::Schema::Dyn),
    ("byAgentId", wire::Schema::Dyn),
]);

/// `buildSubagentIdentityDirectory` over the binary wire.
#[napi(js_name = "buildSubagentIdentityDirectoryWire")]
pub fn build_subagent_identity_directory_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let hints: serde_json::Value = wire::read_value(r, &S_BUILD_SUBAGENT_IDENTITY_DIRECTORY_0)?;
        let out = &port::build_subagent_identity_directory(hints)?;
        wire::write_value(w, out, &S_BUILD_SUBAGENT_IDENTITY_DIRECTORY_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_SUBAGENT_IDENTITY_FROM_DIRECTORY_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_SUBAGENT_IDENTITY_FROM_DIRECTORY_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_SUBAGENT_IDENTITY_FROM_DIRECTORY_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        (
            "providerThreadId",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("agentId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("nickname", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("role", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("model", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("effort", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("background", wire::Schema::OptJs(&wire::Schema::Bool)),
        ("prompt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("status", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "modelIsRequestedHint",
            wire::Schema::OptJs(&wire::Schema::Bool),
        ),
    ]));

/// `resolveSubagentIdentityFromDirectory` over the binary wire.
#[napi(js_name = "resolveSubagentIdentityFromDirectoryWire")]
pub fn resolve_subagent_identity_from_directory_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let directory: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_SUBAGENT_IDENTITY_FROM_DIRECTORY_0)?;
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_SUBAGENT_IDENTITY_FROM_DIRECTORY_1)?;
        let out = &port::resolve_subagent_identity_from_directory(directory, input)?;
        wire::write_value(w, out, &S_RESOLVE_SUBAGENT_IDENTITY_FROM_DIRECTORY_OUT)?;
        Ok(())
    })
}
