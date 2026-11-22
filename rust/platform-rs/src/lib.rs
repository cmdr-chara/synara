//! JS bindings for `apps/server/src/platform/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `makeEffectProcessCommand`.
#[napi(js_name = "makeEffectProcessCommand")]
pub fn make_effect_process_command(
    command: String,
    args: Vec<String>,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::make_effect_process_command(command, args, options)?;
        crate::js::to_json(&out)
    })
}

/// `parseProcessChildrenMap`.
#[napi(js_name = "parseProcessChildrenMap")]
pub fn parse_process_children_map(
    ps_output: String,
    include_start_time: Option<bool>,
) -> napi::Result<String> {
    run(move || {
        let out = port::parse_process_children_map(ps_output, include_start_time)?;
        crate::js::to_json(&out)
    })
}

/// `collectDescendantProcesses`.
#[napi(js_name = "collectDescendantProcesses")]
pub fn collect_descendant_processes(
    parent_pid: f64,
    children_by_parent_pid: String,
) -> napi::Result<String> {
    run(move || {
        let children_by_parent_pid = crate::js::from_json(&children_by_parent_pid)?;
        let out = port::collect_descendant_processes(parent_pid, children_by_parent_pid)?;
        crate::js::to_json(&out)
    })
}

/// `signalOwnedChildProcess`.
#[napi(js_name = "signalOwnedChildProcess")]
pub fn signal_owned_child_process(
    child: String,
    signal: String,
    platform: String,
) -> napi::Result<()> {
    run(move || {
        let child = crate::js::from_json(&child)?;
        let platform = crate::js::from_json(&platform)?;
        port::signal_owned_child_process(child, signal, platform)?;
        Ok(())
    })
}

/// `teardownEffectProcessTree`.
#[napi(js_name = "teardownEffectProcessTree")]
pub async fn teardown_effect_process_tree(
    process: String,
    teardown_process_tree: String,
) -> napi::Result<String> {
    run_async(async move {
        let process = crate::js::from_json(&process)?;
        let teardown_process_tree = crate::js::from_json(&teardown_process_tree)?;
        let out = port::teardown_effect_process_tree(process, teardown_process_tree).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `parseWindowsProcessSnapshotLine`.
#[napi(js_name = "parseWindowsProcessSnapshotLine")]
pub fn parse_windows_process_snapshot_line(line: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_windows_process_snapshot_line(line)?;
        crate::js::to_json(&out)
    })
}

/// `captureWindowsProcessChildrenMap`.
#[napi(js_name = "captureWindowsProcessChildrenMap")]
pub async fn capture_windows_process_children_map() -> napi::Result<String> {
    run_async(async move {
        let out = port::capture_windows_process_children_map().await?;
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

static S_MAKE_EFFECT_PROCESS_COMMAND_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeEffectProcessCommand` over the binary wire.
#[napi(js_name = "makeEffectProcessCommandWire")]
pub fn make_effect_process_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let args: Vec<String> = r.list(|r| r.string16())?;
        let options: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::make_effect_process_command(command, args, options)?;
        wire::write_value(w, out, &S_MAKE_EFFECT_PROCESS_COMMAND_OUT)?;
        Ok(())
    })
}

static S_PARSE_PROCESS_CHILDREN_MAP_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseProcessChildrenMap` over the binary wire.
#[napi(js_name = "parseProcessChildrenMapWire")]
pub fn parse_process_children_map_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ps_output: String = r.string16()?;
        let include_start_time: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::parse_process_children_map(ps_output, include_start_time)?;
        wire::write_value(w, out, &S_PARSE_PROCESS_CHILDREN_MAP_OUT)?;
        Ok(())
    })
}

static S_COLLECT_DESCENDANT_PROCESSES_1: wire::Schema = wire::Schema::Dyn;
static S_COLLECT_DESCENDANT_PROCESSES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("pid", wire::Schema::Num),
        ("command", wire::Schema::Str16),
        ("startedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `collectDescendantProcesses` over the binary wire.
#[napi(js_name = "collectDescendantProcessesWire")]
pub fn collect_descendant_processes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let parent_pid: f64 = r.f64()?;
        let children_by_parent_pid: serde_json::Value =
            wire::read_value(r, &S_COLLECT_DESCENDANT_PROCESSES_1)?;
        let out = &port::collect_descendant_processes(parent_pid, children_by_parent_pid)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_COLLECT_DESCENDANT_PROCESSES_OUT.inner())?;
        }
        Ok(())
    })
}

static S_SIGNAL_OWNED_CHILD_PROCESS_0: wire::Schema = wire::Schema::Dyn;

/// `signalOwnedChildProcess` over the binary wire.
#[napi(js_name = "signalOwnedChildProcessWire")]
pub fn signal_owned_child_process_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let child: serde_json::Value = wire::read_value(r, &S_SIGNAL_OWNED_CHILD_PROCESS_0)?;
        let signal: String = r.string16()?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let () = port::signal_owned_child_process(child, signal, platform)?;
        Ok(())
    })
}

static S_PARSE_WINDOWS_PROCESS_SNAPSHOT_LINE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("pid", wire::Schema::Num),
        ("ppid", wire::Schema::Num),
        ("command", wire::Schema::Str16),
        ("startedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `parseWindowsProcessSnapshotLine` over the binary wire.
#[napi(js_name = "parseWindowsProcessSnapshotLineWire")]
pub fn parse_windows_process_snapshot_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let out = &port::parse_windows_process_snapshot_line(line)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PARSE_WINDOWS_PROCESS_SNAPSHOT_LINE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}
