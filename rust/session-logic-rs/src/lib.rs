//! JS bindings for `apps/web/src/session-logic.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod corpus;
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

/// `formatClockDuration`.
#[napi(js_name = "formatClockDuration")]
pub fn format_clock_duration(duration_ms: f64) -> napi::Result<String> {
    run(move || {
        let out = port::format_clock_duration(duration_ms)?;
        Ok(out)
    })
}

/// `formatClockElapsed`.
#[napi(js_name = "formatClockElapsed")]
pub fn format_clock_elapsed(
    start_iso: String,
    end_iso: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::format_clock_elapsed(start_iso, end_iso)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `formatElapsed`.
#[napi(js_name = "formatElapsed")]
pub fn format_elapsed(
    start_iso: String,
    end_iso: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::format_elapsed(start_iso, end_iso)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isSessionOrLatestTurnRunning`.
#[napi(js_name = "isSessionOrLatestTurnRunning")]
pub fn is_session_or_latest_turn_running(
    orchestration_status: String,
    latest_turn_state: String,
) -> napi::Result<bool> {
    run(move || {
        let orchestration_status = crate::js::from_json(&orchestration_status)?;
        let latest_turn_state = crate::js::from_json(&latest_turn_state)?;
        let out = port::is_session_or_latest_turn_running(orchestration_status, latest_turn_state)?;
        Ok(out)
    })
}

/// `isSessionOrLatestTurnStartingOrRunning`.
#[napi(js_name = "isSessionOrLatestTurnStartingOrRunning")]
pub fn is_session_or_latest_turn_starting_or_running(
    orchestration_status: String,
    latest_turn_state: String,
) -> napi::Result<bool> {
    run(move || {
        let orchestration_status = crate::js::from_json(&orchestration_status)?;
        let latest_turn_state = crate::js::from_json(&latest_turn_state)?;
        let out = port::is_session_or_latest_turn_starting_or_running(
            orchestration_status,
            latest_turn_state,
        )?;
        Ok(out)
    })
}

/// `isSessionOpenOrLatestTurnRunning`.
#[napi(js_name = "isSessionOpenOrLatestTurnRunning")]
pub fn is_session_open_or_latest_turn_running(
    orchestration_status: String,
    latest_turn_state: String,
) -> napi::Result<bool> {
    run(move || {
        let orchestration_status = crate::js::from_json(&orchestration_status)?;
        let latest_turn_state = crate::js::from_json(&latest_turn_state)?;
        let out =
            port::is_session_open_or_latest_turn_running(orchestration_status, latest_turn_state)?;
        Ok(out)
    })
}

/// `runningSessionActiveTurnId`.
#[napi(js_name = "runningSessionActiveTurnId")]
pub fn running_session_active_turn_id(session: String) -> napi::Result<String> {
    run(move || {
        let session = crate::js::from_json(&session)?;
        let out = port::running_session_active_turn_id(session)?;
        crate::js::to_json(&out)
    })
}

/// `isLatestTurnSettled`.
#[napi(js_name = "isLatestTurnSettled")]
pub fn is_latest_turn_settled(latest_turn: String, session: String) -> napi::Result<bool> {
    run(move || {
        let latest_turn = crate::js::from_json(&latest_turn)?;
        let session = crate::js::from_json(&session)?;
        let out = port::is_latest_turn_settled(latest_turn, session)?;
        Ok(out)
    })
}

/// `hasLiveLatestTurn`.
#[napi(js_name = "hasLiveLatestTurn")]
pub fn has_live_latest_turn(latest_turn: String, session: String) -> napi::Result<bool> {
    run(move || {
        let latest_turn = crate::js::from_json(&latest_turn)?;
        let session = crate::js::from_json(&session)?;
        let out = port::has_live_latest_turn(latest_turn, session)?;
        Ok(out)
    })
}

/// `canSessionAnswerPendingRequests`.
#[napi(js_name = "canSessionAnswerPendingRequests")]
pub fn can_session_answer_pending_requests(session: String) -> napi::Result<bool> {
    run(move || {
        let session = crate::js::from_json(&session)?;
        let out = port::can_session_answer_pending_requests(session)?;
        Ok(out)
    })
}

/// `isSessionRuntimeRetired`.
#[napi(js_name = "isSessionRuntimeRetired")]
pub fn is_session_runtime_retired(status: String) -> napi::Result<bool> {
    run(move || {
        let status = crate::js::from_json(&status)?;
        let out = port::is_session_runtime_retired(status)?;
        Ok(out)
    })
}

/// `isSessionAliveForSubagents`.
#[napi(js_name = "isSessionAliveForSubagents")]
pub fn is_session_alive_for_subagents(status: String) -> napi::Result<bool> {
    run(move || {
        let status = crate::js::from_json(&status)?;
        let out = port::is_session_alive_for_subagents(status)?;
        Ok(out)
    })
}

/// `isSessionRunningTurn`.
#[napi(js_name = "isSessionRunningTurn")]
pub fn is_session_running_turn(session: String) -> napi::Result<bool> {
    run(move || {
        let session = crate::js::from_json(&session)?;
        let out = port::is_session_running_turn(session)?;
        Ok(out)
    })
}

/// `deriveActiveWorkStartedAt`.
#[napi(js_name = "deriveActiveWorkStartedAt")]
pub fn derive_active_work_started_at(
    latest_turn: String,
    session: String,
    send_started_at: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let latest_turn = crate::js::from_json(&latest_turn)?;
        let session = crate::js::from_json(&session)?;
        let out = port::derive_active_work_started_at(latest_turn, session, send_started_at)?;
        crate::js::to_json(&out)
    })
}

/// `deriveActiveTaskListState`.
#[napi(js_name = "deriveActiveTaskListState")]
pub fn derive_active_task_list_state(
    activities: String,
    latest_turn_id: String,
) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let latest_turn_id = crate::js::from_json(&latest_turn_id)?;
        let out = port::derive_active_task_list_state(activities, latest_turn_id)?;
        crate::js::to_json(&out)
    })
}

/// `deriveActiveBackgroundTasksState`.
#[napi(js_name = "deriveActiveBackgroundTasksState")]
pub fn derive_active_background_tasks_state(
    activities: String,
    latest_turn_id: String,
) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let latest_turn_id = crate::js::from_json(&latest_turn_id)?;
        let out = port::derive_active_background_tasks_state(activities, latest_turn_id)?;
        crate::js::to_json(&out)
    })
}

/// `derivePendingBackgroundWork`.
#[napi(js_name = "derivePendingBackgroundWork")]
pub fn derive_pending_background_work(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_pending_background_work(input)?;
        crate::js::to_json(&out)
    })
}

/// `deriveOutstandingBackgroundTaskIds`.
#[napi(js_name = "deriveOutstandingBackgroundTaskIds")]
pub fn derive_outstanding_background_task_ids(activities: String) -> napi::Result<Vec<String>> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let out = port::derive_outstanding_background_task_ids(activities)?;
        Ok(out)
    })
}

/// `countOutstandingBackgroundWork`.
#[napi(js_name = "countOutstandingBackgroundWork")]
pub fn count_outstanding_background_work(input: String) -> napi::Result<f64> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::count_outstanding_background_work(input)?;
        Ok(out)
    })
}

/// `deriveRunningSubagentTasks`.
#[napi(js_name = "deriveRunningSubagentTasks")]
pub fn derive_running_subagent_tasks(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_running_subagent_tasks(input)?;
        crate::js::to_json(&out)
    })
}

/// `hasLiveTurnTailWork`.
#[napi(js_name = "hasLiveTurnTailWork")]
pub fn has_live_turn_tail_work(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::has_live_turn_tail_work(input)?;
        Ok(out)
    })
}

/// `findLatestProposedPlan`.
#[napi(js_name = "findLatestProposedPlan")]
pub fn find_latest_proposed_plan(
    proposed_plans: String,
    latest_turn_id: String,
) -> napi::Result<String> {
    run(move || {
        let proposed_plans = crate::js::from_json(&proposed_plans)?;
        let latest_turn_id = crate::js::from_json(&latest_turn_id)?;
        let out = port::find_latest_proposed_plan(proposed_plans, latest_turn_id)?;
        crate::js::to_json(&out)
    })
}

/// `findSidebarProposedPlan`.
#[napi(js_name = "findSidebarProposedPlan")]
pub fn find_sidebar_proposed_plan(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::find_sidebar_proposed_plan(input)?;
        crate::js::to_json(&out)
    })
}

/// `hasActionableProposedPlan`.
#[napi(js_name = "hasActionableProposedPlan")]
pub fn has_actionable_proposed_plan(proposed_plan: String) -> napi::Result<bool> {
    run(move || {
        let proposed_plan = crate::js::from_json(&proposed_plan)?;
        let out = port::has_actionable_proposed_plan(proposed_plan)?;
        Ok(out)
    })
}

/// `buildSourceProposedPlanReference`.
#[napi(js_name = "buildSourceProposedPlanReference")]
pub fn build_source_proposed_plan_reference(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_source_proposed_plan_reference(input)?;
        crate::js::to_json(&out)
    })
}

/// `inferCheckpointTurnCountByTurnId`.
#[napi(js_name = "inferCheckpointTurnCountByTurnId")]
pub fn infer_checkpoint_turn_count_by_turn_id(summaries: String) -> napi::Result<String> {
    run(move || {
        let summaries = crate::js::from_json(&summaries)?;
        let out = port::infer_checkpoint_turn_count_by_turn_id(summaries)?;
        crate::js::to_json(&out)
    })
}

/// `derivePhase`.
#[napi(js_name = "derivePhase")]
pub fn derive_phase(session: String) -> napi::Result<String> {
    run(move || {
        let session = crate::js::from_json(&session)?;
        let out = port::derive_phase(session)?;
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

/// `formatClockDuration` over the binary wire.
#[napi(js_name = "formatClockDurationWire")]
pub fn format_clock_duration_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let duration_ms: f64 = r.f64()?;
        let out = &port::format_clock_duration(duration_ms)?;
        w.string16(out);
        Ok(())
    })
}

/// `formatClockElapsed` over the binary wire.
#[napi(js_name = "formatClockElapsedWire")]
pub fn format_clock_elapsed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let start_iso: String = r.string16()?;
        let end_iso: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::format_clock_elapsed(start_iso, end_iso)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `formatElapsed` over the binary wire.
#[napi(js_name = "formatElapsedWire")]
pub fn format_elapsed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let start_iso: String = r.string16()?;
        let end_iso: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::format_elapsed(start_iso, end_iso)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `isSessionOrLatestTurnRunning` over the binary wire.
#[napi(js_name = "isSessionOrLatestTurnRunningWire")]
pub fn is_session_or_latest_turn_running_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let orchestration_status: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let latest_turn_state: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out =
            &port::is_session_or_latest_turn_running(orchestration_status, latest_turn_state)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isSessionOrLatestTurnStartingOrRunning` over the binary wire.
#[napi(js_name = "isSessionOrLatestTurnStartingOrRunningWire")]
pub fn is_session_or_latest_turn_starting_or_running_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let orchestration_status: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let latest_turn_state: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_session_or_latest_turn_starting_or_running(
            orchestration_status,
            latest_turn_state,
        )?;
        w.bool(*out);
        Ok(())
    })
}

/// `isSessionOpenOrLatestTurnRunning` over the binary wire.
#[napi(js_name = "isSessionOpenOrLatestTurnRunningWire")]
pub fn is_session_open_or_latest_turn_running_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let orchestration_status: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let latest_turn_state: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out =
            &port::is_session_open_or_latest_turn_running(orchestration_status, latest_turn_state)?;
        w.bool(*out);
        Ok(())
    })
}

/// `runningSessionActiveTurnId` over the binary wire.
#[napi(js_name = "runningSessionActiveTurnIdWire")]
pub fn running_session_active_turn_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::running_session_active_turn_id(session)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `isLatestTurnSettled` over the binary wire.
#[napi(js_name = "isLatestTurnSettledWire")]
pub fn is_latest_turn_settled_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let latest_turn: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_latest_turn_settled(latest_turn, session)?;
        w.bool(*out);
        Ok(())
    })
}

/// `hasLiveLatestTurn` over the binary wire.
#[napi(js_name = "hasLiveLatestTurnWire")]
pub fn has_live_latest_turn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let latest_turn: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::has_live_latest_turn(latest_turn, session)?;
        w.bool(*out);
        Ok(())
    })
}

/// `canSessionAnswerPendingRequests` over the binary wire.
#[napi(js_name = "canSessionAnswerPendingRequestsWire")]
pub fn can_session_answer_pending_requests_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::can_session_answer_pending_requests(session)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isSessionRuntimeRetired` over the binary wire.
#[napi(js_name = "isSessionRuntimeRetiredWire")]
pub fn is_session_runtime_retired_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let status: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_session_runtime_retired(status)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isSessionAliveForSubagents` over the binary wire.
#[napi(js_name = "isSessionAliveForSubagentsWire")]
pub fn is_session_alive_for_subagents_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let status: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_session_alive_for_subagents(status)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isSessionRunningTurn` over the binary wire.
#[napi(js_name = "isSessionRunningTurnWire")]
pub fn is_session_running_turn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_session_running_turn(session)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_DERIVE_ACTIVE_WORK_STARTED_AT_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `deriveActiveWorkStartedAt` over the binary wire.
#[napi(js_name = "deriveActiveWorkStartedAtWire")]
pub fn derive_active_work_started_at_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let latest_turn: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let send_started_at: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::derive_active_work_started_at(latest_turn, session, send_started_at)?;
        wire::write_value(w, out, &S_DERIVE_ACTIVE_WORK_STARTED_AT_OUT)?;
        Ok(())
    })
}

static S_DERIVE_ACTIVE_TASK_LIST_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_ACTIVE_TASK_LIST_STATE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("createdAt", wire::Schema::Str16),
        ("turnId", wire::Schema::Dyn),
        ("explanation", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "tasks",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("task", wire::Schema::Str16),
                ("status", wire::Schema::Str16),
            ])),
        ),
    ]));

/// `deriveActiveTaskListState` over the binary wire.
#[napi(js_name = "deriveActiveTaskListStateWire")]
pub fn derive_active_task_list_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: serde_json::Value =
            wire::read_value(r, &S_DERIVE_ACTIVE_TASK_LIST_STATE_0)?;
        let latest_turn_id: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::derive_active_task_list_state(activities, latest_turn_id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_DERIVE_ACTIVE_TASK_LIST_STATE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_DERIVE_ACTIVE_BACKGROUND_TASKS_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_ACTIVE_BACKGROUND_TASKS_STATE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("activeCount", wire::Schema::Num),
        ("taskIds", wire::Schema::List(&wire::Schema::Str16)),
    ]));

/// `deriveActiveBackgroundTasksState` over the binary wire.
#[napi(js_name = "deriveActiveBackgroundTasksStateWire")]
pub fn derive_active_background_tasks_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: serde_json::Value =
            wire::read_value(r, &S_DERIVE_ACTIVE_BACKGROUND_TASKS_STATE_0)?;
        let latest_turn_id: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::derive_active_background_tasks_state(activities, latest_turn_id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_DERIVE_ACTIVE_BACKGROUND_TASKS_STATE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_DERIVE_PENDING_BACKGROUND_WORK_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_PENDING_BACKGROUND_WORK_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("count", wire::Schema::Num),
        ("taskIds", wire::Schema::List(&wire::Schema::Str16)),
    ]));

/// `derivePendingBackgroundWork` over the binary wire.
#[napi(js_name = "derivePendingBackgroundWorkWire")]
pub fn derive_pending_background_work_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DERIVE_PENDING_BACKGROUND_WORK_0)?;
        let out = &port::derive_pending_background_work(input)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_DERIVE_PENDING_BACKGROUND_WORK_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_DERIVE_OUTSTANDING_BACKGROUND_TASK_IDS_0: wire::Schema = wire::Schema::Dyn;

/// `deriveOutstandingBackgroundTaskIds` over the binary wire.
#[napi(js_name = "deriveOutstandingBackgroundTaskIdsWire")]
pub fn derive_outstanding_background_task_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: serde_json::Value =
            wire::read_value(r, &S_DERIVE_OUTSTANDING_BACKGROUND_TASK_IDS_0)?;
        let out = &port::derive_outstanding_background_task_ids(activities)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_COUNT_OUTSTANDING_BACKGROUND_WORK_0: wire::Schema = wire::Schema::Dyn;

/// `countOutstandingBackgroundWork` over the binary wire.
#[napi(js_name = "countOutstandingBackgroundWorkWire")]
pub fn count_outstanding_background_work_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_COUNT_OUTSTANDING_BACKGROUND_WORK_0)?;
        let out = &port::count_outstanding_background_work(input)?;
        w.f64(*out);
        Ok(())
    })
}

static S_DERIVE_RUNNING_SUBAGENT_TASKS_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_RUNNING_SUBAGENT_TASKS_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "running",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("taskId", wire::Schema::Str16),
            ("toolUseId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("resumed", wire::Schema::Bool),
        ])),
    ),
    ("trackedKeys", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `deriveRunningSubagentTasks` over the binary wire.
#[napi(js_name = "deriveRunningSubagentTasksWire")]
pub fn derive_running_subagent_tasks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DERIVE_RUNNING_SUBAGENT_TASKS_0)?;
        let out = &port::derive_running_subagent_tasks(input)?;
        wire::write_value(w, out, &S_DERIVE_RUNNING_SUBAGENT_TASKS_OUT)?;
        Ok(())
    })
}

static S_HAS_LIVE_TURN_TAIL_WORK_0: wire::Schema = wire::Schema::Dyn;

/// `hasLiveTurnTailWork` over the binary wire.
#[napi(js_name = "hasLiveTurnTailWorkWire")]
pub fn has_live_turn_tail_work_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_HAS_LIVE_TURN_TAIL_WORK_0)?;
        let out = &port::has_live_turn_tail_work(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_FIND_LATEST_PROPOSED_PLAN_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_LATEST_PROPOSED_PLAN_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_LATEST_PROPOSED_PLAN_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("id", wire::Schema::Dyn),
    ("createdAt", wire::Schema::Str16),
    ("updatedAt", wire::Schema::Str16),
    ("turnId", wire::Schema::Dyn),
    ("planMarkdown", wire::Schema::Str16),
    ("implementedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("implementationThreadId", wire::Schema::Dyn),
]));

/// `findLatestProposedPlan` over the binary wire.
#[napi(js_name = "findLatestProposedPlanWire")]
pub fn find_latest_proposed_plan_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let proposed_plans: serde_json::Value =
            wire::read_value(r, &S_FIND_LATEST_PROPOSED_PLAN_0)?;
        let latest_turn_id: serde_json::Value =
            wire::read_value(r, &S_FIND_LATEST_PROPOSED_PLAN_1)?;
        let out = &port::find_latest_proposed_plan(proposed_plans, latest_turn_id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_FIND_LATEST_PROPOSED_PLAN_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_FIND_SIDEBAR_PROPOSED_PLAN_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_SIDEBAR_PROPOSED_PLAN_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("id", wire::Schema::Dyn),
    ("createdAt", wire::Schema::Str16),
    ("updatedAt", wire::Schema::Str16),
    ("turnId", wire::Schema::Dyn),
    ("planMarkdown", wire::Schema::Str16),
    ("implementedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("implementationThreadId", wire::Schema::Dyn),
]));

/// `findSidebarProposedPlan` over the binary wire.
#[napi(js_name = "findSidebarProposedPlanWire")]
pub fn find_sidebar_proposed_plan_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_FIND_SIDEBAR_PROPOSED_PLAN_0)?;
        let out = &port::find_sidebar_proposed_plan(input)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_FIND_SIDEBAR_PROPOSED_PLAN_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_HAS_ACTIONABLE_PROPOSED_PLAN_0: wire::Schema = wire::Schema::Dyn;

/// `hasActionableProposedPlan` over the binary wire.
#[napi(js_name = "hasActionableProposedPlanWire")]
pub fn has_actionable_proposed_plan_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let proposed_plan: serde_json::Value =
            wire::read_value(r, &S_HAS_ACTIONABLE_PROPOSED_PLAN_0)?;
        let out = &port::has_actionable_proposed_plan(proposed_plan)?;
        w.bool(*out);
        Ok(())
    })
}

static S_BUILD_SOURCE_PROPOSED_PLAN_REFERENCE_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_SOURCE_PROPOSED_PLAN_REFERENCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildSourceProposedPlanReference` over the binary wire.
#[napi(js_name = "buildSourceProposedPlanReferenceWire")]
pub fn build_source_proposed_plan_reference_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_BUILD_SOURCE_PROPOSED_PLAN_REFERENCE_0)?;
        let out = &port::build_source_proposed_plan_reference(input)?;
        wire::write_value(w, out, &S_BUILD_SOURCE_PROPOSED_PLAN_REFERENCE_OUT)?;
        Ok(())
    })
}

static S_INFER_CHECKPOINT_TURN_COUNT_BY_TURN_ID_0: wire::Schema = wire::Schema::Dyn;
static S_INFER_CHECKPOINT_TURN_COUNT_BY_TURN_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `inferCheckpointTurnCountByTurnId` over the binary wire.
#[napi(js_name = "inferCheckpointTurnCountByTurnIdWire")]
pub fn infer_checkpoint_turn_count_by_turn_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let summaries: serde_json::Value =
            wire::read_value(r, &S_INFER_CHECKPOINT_TURN_COUNT_BY_TURN_ID_0)?;
        let out = &port::infer_checkpoint_turn_count_by_turn_id(summaries)?;
        wire::write_value(w, out, &S_INFER_CHECKPOINT_TURN_COUNT_BY_TURN_ID_OUT)?;
        Ok(())
    })
}

/// `derivePhase` over the binary wire.
#[napi(js_name = "derivePhaseWire")]
pub fn derive_phase_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::derive_phase(session)?;
        w.string16(out);
        Ok(())
    })
}
