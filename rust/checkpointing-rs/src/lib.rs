//! JS bindings for `apps/server/src/checkpointing/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parseCheckpointFilesFromUnifiedDiff`.
#[napi(js_name = "parseCheckpointFilesFromUnifiedDiff")]
pub fn parse_checkpoint_files_from_unified_diff(diff: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_checkpoint_files_from_unified_diff(diff)?;
        crate::js::to_json(&out)
    })
}

/// `parseManagedCheckpointRef`.
#[napi(js_name = "parseManagedCheckpointRef")]
pub fn parse_managed_checkpoint_ref(value: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_managed_checkpoint_ref(value)?;
        crate::js::to_json(&out)
    })
}

/// `isManagedCheckpointRefForThread`.
#[napi(js_name = "isManagedCheckpointRefForThread")]
pub fn is_managed_checkpoint_ref_for_thread(
    value: String,
    thread_id: String,
) -> napi::Result<bool> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::is_managed_checkpoint_ref_for_thread(value, thread_id)?;
        Ok(out)
    })
}

/// `checkpointRefForThreadTurn`.
#[napi(js_name = "checkpointRefForThreadTurn")]
pub fn checkpoint_ref_for_thread_turn(thread_id: String, turn_count: f64) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::checkpoint_ref_for_thread_turn(thread_id, turn_count)?;
        crate::js::to_json(&out)
    })
}

/// `checkpointRefForThreadTurnInManagedFamily`.
#[napi(js_name = "checkpointRefForThreadTurnInManagedFamily")]
pub fn checkpoint_ref_for_thread_turn_in_managed_family(
    managed_ref: String,
    thread_id: String,
    turn_count: f64,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::checkpoint_ref_for_thread_turn_in_managed_family(
            managed_ref,
            thread_id,
            turn_count,
        )?;
        crate::js::to_json(&out)
    })
}

/// `checkpointRefForThreadMessageStart`.
#[napi(js_name = "checkpointRefForThreadMessageStart")]
pub fn checkpoint_ref_for_thread_message_start(
    thread_id: String,
    message_id: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let message_id = crate::js::from_json(&message_id)?;
        let out = port::checkpoint_ref_for_thread_message_start(thread_id, message_id)?;
        crate::js::to_json(&out)
    })
}

/// `checkpointRefForThreadTurnStart`.
#[napi(js_name = "checkpointRefForThreadTurnStart")]
pub fn checkpoint_ref_for_thread_turn_start(
    thread_id: String,
    turn_id: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let turn_id = crate::js::from_json(&turn_id)?;
        let out = port::checkpoint_ref_for_thread_turn_start(thread_id, turn_id)?;
        crate::js::to_json(&out)
    })
}

/// `checkpointRefForThreadTurnStartInManagedFamily`.
#[napi(js_name = "checkpointRefForThreadTurnStartInManagedFamily")]
pub fn checkpoint_ref_for_thread_turn_start_in_managed_family(
    managed_ref: String,
    thread_id: String,
    turn_id: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let turn_id = crate::js::from_json(&turn_id)?;
        let out = port::checkpoint_ref_for_thread_turn_start_in_managed_family(
            managed_ref,
            thread_id,
            turn_id,
        )?;
        crate::js::to_json(&out)
    })
}

/// `checkpointRefForThreadTurnLive`.
#[napi(js_name = "checkpointRefForThreadTurnLive")]
pub fn checkpoint_ref_for_thread_turn_live(
    thread_id: String,
    turn_id: String,
    revision: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let turn_id = crate::js::from_json(&turn_id)?;
        let out = port::checkpoint_ref_for_thread_turn_live(thread_id, turn_id, revision)?;
        crate::js::to_json(&out)
    })
}

/// `checkpointRefForThreadRevertRescue`.
#[napi(js_name = "checkpointRefForThreadRevertRescue")]
pub fn checkpoint_ref_for_thread_revert_rescue(
    thread_id: String,
    token: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::checkpoint_ref_for_thread_revert_rescue(thread_id, token)?;
        crate::js::to_json(&out)
    })
}

/// `resolveProjectCwdForKind`.
#[napi(js_name = "resolveProjectCwdForKind")]
pub fn resolve_project_cwd_for_kind(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_project_cwd_for_kind(input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveThreadWorkspaceCwd`.
#[napi(js_name = "resolveThreadWorkspaceCwd")]
pub fn resolve_thread_workspace_cwd(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_thread_workspace_cwd(input)?;
        crate::js::to_json(&out)
    })
}

/// `formatCheckpointHeadTrailer`.
#[napi(js_name = "formatCheckpointHeadTrailer")]
pub fn format_checkpoint_head_trailer(
    head_commit: String,
    reflog_entry: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::format_checkpoint_head_trailer(head_commit, reflog_entry)?;
        Ok(out)
    })
}

/// `checkpointReflogAnchor`.
#[napi(js_name = "checkpointReflogAnchor")]
pub fn checkpoint_reflog_anchor(entry: String) -> napi::Result<String> {
    run(move || {
        let out = port::checkpoint_reflog_anchor(entry)?;
        Ok(out)
    })
}

/// `parseCheckpointCommit`.
#[napi(js_name = "parseCheckpointCommit")]
pub fn parse_checkpoint_commit(raw: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_checkpoint_commit(raw)?;
        crate::js::to_json(&out)
    })
}

/// `checkpointHeadTransitions`.
#[napi(js_name = "checkpointHeadTransitions")]
pub fn checkpoint_head_transitions(raw: String, from: String, to: String) -> napi::Result<String> {
    run(move || {
        let from = crate::js::from_json(&from)?;
        let to = crate::js::from_json(&to)?;
        let out = port::checkpoint_head_transitions(raw, from, to)?;
        crate::js::to_json(&out)
    })
}

/// `parseRawDiff`.
#[napi(js_name = "parseRawDiff")]
pub fn parse_raw_diff(stdout: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_raw_diff(stdout)?;
        crate::js::to_json(&out)
    })
}

/// `parseNameOnlyPaths`.
#[napi(js_name = "parseNameOnlyPaths")]
pub fn parse_name_only_paths(stdout: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_name_only_paths(stdout)?;
        crate::js::to_json(&out)
    })
}

/// `chunkArgumentGroups`.
#[napi(js_name = "chunkArgumentGroups")]
pub fn chunk_argument_groups(groups: String, max_characters: Option<f64>) -> napi::Result<String> {
    run(move || {
        let groups = crate::js::from_json(&groups)?;
        let out = port::chunk_argument_groups(groups, max_characters)?;
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

static S_PARSE_CHECKPOINT_FILES_FROM_UNIFIED_DIFF_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseCheckpointFilesFromUnifiedDiff` over the binary wire.
#[napi(js_name = "parseCheckpointFilesFromUnifiedDiffWire")]
pub fn parse_checkpoint_files_from_unified_diff_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let diff: String = r.string16()?;
        let out = &port::parse_checkpoint_files_from_unified_diff(diff)?;
        wire::write_value(w, out, &S_PARSE_CHECKPOINT_FILES_FROM_UNIFIED_DIFF_OUT)?;
        Ok(())
    })
}

static S_PARSE_MANAGED_CHECKPOINT_REF_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("namespace", wire::Schema::Str16),
        ("threadToken", wire::Schema::Str16),
        ("kind", wire::Schema::Str16),
        ("valueToken", wire::Schema::Str16),
        ("familyPrefix", wire::Schema::Str16),
    ]));

/// `parseManagedCheckpointRef` over the binary wire.
#[napi(js_name = "parseManagedCheckpointRefWire")]
pub fn parse_managed_checkpoint_ref_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::parse_managed_checkpoint_ref(value)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PARSE_MANAGED_CHECKPOINT_REF_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_IS_MANAGED_CHECKPOINT_REF_FOR_THREAD_1: wire::Schema = wire::Schema::Dyn;

/// `isManagedCheckpointRefForThread` over the binary wire.
#[napi(js_name = "isManagedCheckpointRefForThreadWire")]
pub fn is_managed_checkpoint_ref_for_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_IS_MANAGED_CHECKPOINT_REF_FOR_THREAD_1)?;
        let out = &port::is_managed_checkpoint_ref_for_thread(value, thread_id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CHECKPOINT_REF_FOR_THREAD_TURN_0: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_TURN_OUT: wire::Schema = wire::Schema::Dyn;

/// `checkpointRefForThreadTurn` over the binary wire.
#[napi(js_name = "checkpointRefForThreadTurnWire")]
pub fn checkpoint_ref_for_thread_turn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_TURN_0)?;
        let turn_count: f64 = r.f64()?;
        let out = &port::checkpoint_ref_for_thread_turn(thread_id, turn_count)?;
        wire::write_value(w, out, &S_CHECKPOINT_REF_FOR_THREAD_TURN_OUT)?;
        Ok(())
    })
}

static S_CHECKPOINT_REF_FOR_THREAD_TURN_IN_MANAGED_FAMILY_1: wire::Schema = wire::Schema::Dyn;

/// `checkpointRefForThreadTurnInManagedFamily` over the binary wire.
#[napi(js_name = "checkpointRefForThreadTurnInManagedFamilyWire")]
pub fn checkpoint_ref_for_thread_turn_in_managed_family_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let managed_ref: String = r.string16()?;
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_TURN_IN_MANAGED_FAMILY_1)?;
        let turn_count: f64 = r.f64()?;
        let out = &port::checkpoint_ref_for_thread_turn_in_managed_family(
            managed_ref,
            thread_id,
            turn_count,
        )?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_CHECKPOINT_REF_FOR_THREAD_MESSAGE_START_0: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_MESSAGE_START_1: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_MESSAGE_START_OUT: wire::Schema = wire::Schema::Dyn;

/// `checkpointRefForThreadMessageStart` over the binary wire.
#[napi(js_name = "checkpointRefForThreadMessageStartWire")]
pub fn checkpoint_ref_for_thread_message_start_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_MESSAGE_START_0)?;
        let message_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_MESSAGE_START_1)?;
        let out = &port::checkpoint_ref_for_thread_message_start(thread_id, message_id)?;
        wire::write_value(w, out, &S_CHECKPOINT_REF_FOR_THREAD_MESSAGE_START_OUT)?;
        Ok(())
    })
}

static S_CHECKPOINT_REF_FOR_THREAD_TURN_START_0: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_TURN_START_1: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_TURN_START_OUT: wire::Schema = wire::Schema::Dyn;

/// `checkpointRefForThreadTurnStart` over the binary wire.
#[napi(js_name = "checkpointRefForThreadTurnStartWire")]
pub fn checkpoint_ref_for_thread_turn_start_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_TURN_START_0)?;
        let turn_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_TURN_START_1)?;
        let out = &port::checkpoint_ref_for_thread_turn_start(thread_id, turn_id)?;
        wire::write_value(w, out, &S_CHECKPOINT_REF_FOR_THREAD_TURN_START_OUT)?;
        Ok(())
    })
}

static S_CHECKPOINT_REF_FOR_THREAD_TURN_START_IN_MANAGED_FAMILY_1: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_TURN_START_IN_MANAGED_FAMILY_2: wire::Schema = wire::Schema::Dyn;

/// `checkpointRefForThreadTurnStartInManagedFamily` over the binary wire.
#[napi(js_name = "checkpointRefForThreadTurnStartInManagedFamilyWire")]
pub fn checkpoint_ref_for_thread_turn_start_in_managed_family_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let managed_ref: String = r.string16()?;
        let thread_id: serde_json::Value = wire::read_value(
            r,
            &S_CHECKPOINT_REF_FOR_THREAD_TURN_START_IN_MANAGED_FAMILY_1,
        )?;
        let turn_id: serde_json::Value = wire::read_value(
            r,
            &S_CHECKPOINT_REF_FOR_THREAD_TURN_START_IN_MANAGED_FAMILY_2,
        )?;
        let out = &port::checkpoint_ref_for_thread_turn_start_in_managed_family(
            managed_ref,
            thread_id,
            turn_id,
        )?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_CHECKPOINT_REF_FOR_THREAD_TURN_LIVE_0: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_TURN_LIVE_1: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_TURN_LIVE_OUT: wire::Schema = wire::Schema::Dyn;

/// `checkpointRefForThreadTurnLive` over the binary wire.
#[napi(js_name = "checkpointRefForThreadTurnLiveWire")]
pub fn checkpoint_ref_for_thread_turn_live_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_TURN_LIVE_0)?;
        let turn_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_TURN_LIVE_1)?;
        let revision: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::checkpoint_ref_for_thread_turn_live(thread_id, turn_id, revision)?;
        wire::write_value(w, out, &S_CHECKPOINT_REF_FOR_THREAD_TURN_LIVE_OUT)?;
        Ok(())
    })
}

static S_CHECKPOINT_REF_FOR_THREAD_REVERT_RESCUE_0: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_REF_FOR_THREAD_REVERT_RESCUE_OUT: wire::Schema = wire::Schema::Dyn;

/// `checkpointRefForThreadRevertRescue` over the binary wire.
#[napi(js_name = "checkpointRefForThreadRevertRescueWire")]
pub fn checkpoint_ref_for_thread_revert_rescue_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_CHECKPOINT_REF_FOR_THREAD_REVERT_RESCUE_0)?;
        let token: String = r.string16()?;
        let out = &port::checkpoint_ref_for_thread_revert_rescue(thread_id, token)?;
        wire::write_value(w, out, &S_CHECKPOINT_REF_FOR_THREAD_REVERT_RESCUE_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_PROJECT_CWD_FOR_KIND_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PROJECT_CWD_FOR_KIND_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveProjectCwdForKind` over the binary wire.
#[napi(js_name = "resolveProjectCwdForKindWire")]
pub fn resolve_project_cwd_for_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_PROJECT_CWD_FOR_KIND_0)?;
        let out = &port::resolve_project_cwd_for_kind(input)?;
        wire::write_value(w, out, &S_RESOLVE_PROJECT_CWD_FOR_KIND_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_THREAD_WORKSPACE_CWD_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_THREAD_WORKSPACE_CWD_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveThreadWorkspaceCwd` over the binary wire.
#[napi(js_name = "resolveThreadWorkspaceCwdWire")]
pub fn resolve_thread_workspace_cwd_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_THREAD_WORKSPACE_CWD_0)?;
        let out = &port::resolve_thread_workspace_cwd(input)?;
        wire::write_value(w, out, &S_RESOLVE_THREAD_WORKSPACE_CWD_OUT)?;
        Ok(())
    })
}

/// `formatCheckpointHeadTrailer` over the binary wire.
#[napi(js_name = "formatCheckpointHeadTrailerWire")]
pub fn format_checkpoint_head_trailer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let head_commit: String = r.string16()?;
        let reflog_entry: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::format_checkpoint_head_trailer(head_commit, reflog_entry)?;
        w.string16(out);
        Ok(())
    })
}

/// `checkpointReflogAnchor` over the binary wire.
#[napi(js_name = "checkpointReflogAnchorWire")]
pub fn checkpoint_reflog_anchor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let entry: String = r.string16()?;
        let out = &port::checkpoint_reflog_anchor(entry)?;
        w.string16(out);
        Ok(())
    })
}

static S_PARSE_CHECKPOINT_COMMIT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("head", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("reflogAnchor", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `parseCheckpointCommit` over the binary wire.
#[napi(js_name = "parseCheckpointCommitWire")]
pub fn parse_checkpoint_commit_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::parse_checkpoint_commit(raw)?;
        wire::write_value(w, out, &S_PARSE_CHECKPOINT_COMMIT_OUT)?;
        Ok(())
    })
}

static S_CHECKPOINT_HEAD_TRANSITIONS_1: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_HEAD_TRANSITIONS_2: wire::Schema = wire::Schema::Dyn;
static S_CHECKPOINT_HEAD_TRANSITIONS_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
        ("head", wire::Schema::Str16),
        ("action", wire::Schema::Str16),
        ("anchor", wire::Schema::Str16),
    ])));

/// `checkpointHeadTransitions` over the binary wire.
#[napi(js_name = "checkpointHeadTransitionsWire")]
pub fn checkpoint_head_transitions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let from: serde_json::Value = wire::read_value(r, &S_CHECKPOINT_HEAD_TRANSITIONS_1)?;
        let to: serde_json::Value = wire::read_value(r, &S_CHECKPOINT_HEAD_TRANSITIONS_2)?;
        let out = &port::checkpoint_head_transitions(raw, from, to)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.tag(1);
                w.count(x0.len());
                for x1 in x0.iter() {
                    wire::write_value(w, x1, S_CHECKPOINT_HEAD_TRANSITIONS_OUT.inner().inner())?;
                }
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_PARSE_RAW_DIFF_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("path", wire::Schema::Str16),
    ("sourceMode", wire::Schema::Str16),
    ("sourceObject", wire::Schema::Str16),
    ("status", wire::Schema::Str16),
]));

/// `parseRawDiff` over the binary wire.
#[napi(js_name = "parseRawDiffWire")]
pub fn parse_raw_diff_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let stdout: String = r.string16()?;
        let out = &port::parse_raw_diff(stdout)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_RAW_DIFF_OUT.inner())?;
        }
        Ok(())
    })
}

static S_PARSE_NAME_ONLY_PATHS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Str16);

/// `parseNameOnlyPaths` over the binary wire.
#[napi(js_name = "parseNameOnlyPathsWire")]
pub fn parse_name_only_paths_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let stdout: String = r.string16()?;
        let out = &port::parse_name_only_paths(stdout)?;
        wire::write_value(w, out, &S_PARSE_NAME_ONLY_PATHS_OUT)?;
        Ok(())
    })
}

/// `chunkArgumentGroups` over the binary wire.
#[napi(js_name = "chunkArgumentGroupsWire")]
pub fn chunk_argument_groups_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let groups: Vec<Vec<String>> = r.list(|r| r.list(|r| r.string16()))?;
        let max_characters: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::chunk_argument_groups(groups, max_characters)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.tag(1);
            w.count(x0.len());
            for x1 in x0.iter() {
                w.string16(x1);
            }
        }
        Ok(())
    })
}
