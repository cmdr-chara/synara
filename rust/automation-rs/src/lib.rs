//! JS bindings for `apps/server/src/automation/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `automationProposalActivityId`.
#[napi(js_name = "automationProposalActivityId")]
pub fn automation_proposal_activity_id(automation_id: String) -> napi::Result<String> {
    run(move || {
        let automation_id = crate::js::from_json(&automation_id)?;
        let out = port::automation_proposal_activity_id(automation_id)?;
        crate::js::to_json(&out)
    })
}

/// `automationCadenceLabel`.
#[napi(js_name = "automationCadenceLabel")]
pub fn automation_cadence_label(schedule: String) -> napi::Result<String> {
    run(move || {
        let schedule = crate::js::from_json(&schedule)?;
        let out = port::automation_cadence_label(schedule)?;
        crate::js::to_json(&out)
    })
}

/// `buildAutomationProposalActivity`.
#[napi(js_name = "buildAutomationProposalActivity")]
pub fn build_automation_proposal_activity(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_automation_proposal_activity(input)?;
        crate::js::to_json(&out)
    })
}

/// `automationMemoryForEnvelope`.
#[napi(js_name = "automationMemoryForEnvelope")]
pub fn automation_memory_for_envelope(content: String) -> napi::Result<String> {
    run(move || {
        let out = port::automation_memory_for_envelope(content)?;
        Ok(out)
    })
}

/// `buildAutomationRunEnvelope`.
#[napi(js_name = "buildAutomationRunEnvelope")]
pub fn build_automation_run_envelope(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_automation_run_envelope(input)?;
        Ok(out)
    })
}

/// `automationRunResultSummary`.
#[napi(js_name = "automationRunResultSummary")]
pub fn automation_run_result_summary(
    value: Option<String>,
    fallback: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::automation_run_result_summary(value, fallback)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `automationRunResultSummaryWithNotice`.
#[napi(js_name = "automationRunResultSummaryWithNotice")]
pub fn automation_run_result_summary_with_notice(
    base_summary: Option<String>,
    notice: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::automation_run_result_summary_with_notice(base_summary, notice)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `normalizeAutomationCompletionReason`.
#[napi(js_name = "normalizeAutomationCompletionReason")]
pub fn normalize_automation_completion_reason(value: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_automation_completion_reason(value)?;
        Ok(out)
    })
}

/// `failedAutomationCompletionEvaluation`.
#[napi(js_name = "failedAutomationCompletionEvaluation")]
pub fn failed_automation_completion_evaluation(reason: String) -> napi::Result<String> {
    run(move || {
        let out = port::failed_automation_completion_evaluation(reason)?;
        crate::js::to_json(&out)
    })
}

/// `automationCompletionRunResult`.
#[napi(js_name = "automationCompletionRunResult")]
pub fn automation_completion_run_result(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::automation_completion_run_result(input)?;
        crate::js::to_json(&out)
    })
}

/// `deterministicAutomationJitterSeconds`.
#[napi(js_name = "deterministicAutomationJitterSeconds")]
pub fn deterministic_automation_jitter_seconds(input: String) -> napi::Result<f64> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::deterministic_automation_jitter_seconds(input)?;
        Ok(out)
    })
}

/// `computeNextAutomationRunAt`.
#[napi(js_name = "computeNextAutomationRunAt")]
pub fn compute_next_automation_run_at(
    schedule: String,
    from_iso: String,
    jitter_context: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let schedule = crate::js::from_json(&schedule)?;
        let jitter_context = crate::js::from_json(&jitter_context)?;
        let out = port::compute_next_automation_run_at(schedule, from_iso, jitter_context)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `computeNextAutomationRunAtAfter`.
#[napi(js_name = "computeNextAutomationRunAtAfter")]
pub fn compute_next_automation_run_at_after(
    schedule: String,
    from_iso: String,
    not_before_iso: String,
    jitter_context: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let schedule = crate::js::from_json(&schedule)?;
        let jitter_context = crate::js::from_json(&jitter_context)?;
        let out = port::compute_next_automation_run_at_after(
            schedule,
            from_iso,
            not_before_iso,
            jitter_context,
        )?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `computeAutomationScheduleSpacingSeconds`.
#[napi(js_name = "computeAutomationScheduleSpacingSeconds")]
pub fn compute_automation_schedule_spacing_seconds(
    schedule: String,
    from_iso: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let schedule = crate::js::from_json(&schedule)?;
        let out = port::compute_automation_schedule_spacing_seconds(schedule, from_iso)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `resolveAutomationStopPolicy`.
#[napi(js_name = "resolveAutomationStopPolicy")]
pub fn resolve_automation_stop_policy(input: String, fallback: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let fallback = crate::js::from_json(&fallback)?;
        let out = port::resolve_automation_stop_policy(input, fallback)?;
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

static S_AUTOMATION_PROPOSAL_ACTIVITY_ID_0: wire::Schema = wire::Schema::Dyn;
static S_AUTOMATION_PROPOSAL_ACTIVITY_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `automationProposalActivityId` over the binary wire.
#[napi(js_name = "automationProposalActivityIdWire")]
pub fn automation_proposal_activity_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let automation_id: serde_json::Value =
            wire::read_value(r, &S_AUTOMATION_PROPOSAL_ACTIVITY_ID_0)?;
        let out = &port::automation_proposal_activity_id(automation_id)?;
        wire::write_value(w, out, &S_AUTOMATION_PROPOSAL_ACTIVITY_ID_OUT)?;
        Ok(())
    })
}

static S_AUTOMATION_CADENCE_LABEL_0: wire::Schema = wire::Schema::Dyn;
static S_AUTOMATION_CADENCE_LABEL_OUT: wire::Schema = wire::Schema::Str16;

/// `automationCadenceLabel` over the binary wire.
#[napi(js_name = "automationCadenceLabelWire")]
pub fn automation_cadence_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let schedule: serde_json::Value = wire::read_value(r, &S_AUTOMATION_CADENCE_LABEL_0)?;
        let out = &port::automation_cadence_label(schedule)?;
        wire::write_value(w, out, &S_AUTOMATION_CADENCE_LABEL_OUT)?;
        Ok(())
    })
}

static S_BUILD_AUTOMATION_PROPOSAL_ACTIVITY_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_AUTOMATION_PROPOSAL_ACTIVITY_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildAutomationProposalActivity` over the binary wire.
#[napi(js_name = "buildAutomationProposalActivityWire")]
pub fn build_automation_proposal_activity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_BUILD_AUTOMATION_PROPOSAL_ACTIVITY_0)?;
        let out = &port::build_automation_proposal_activity(input)?;
        wire::write_value(w, out, &S_BUILD_AUTOMATION_PROPOSAL_ACTIVITY_OUT)?;
        Ok(())
    })
}

/// `automationMemoryForEnvelope` over the binary wire.
#[napi(js_name = "automationMemoryForEnvelopeWire")]
pub fn automation_memory_for_envelope_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let content: String = r.string16()?;
        let out = &port::automation_memory_for_envelope(content)?;
        w.string16(out);
        Ok(())
    })
}

static S_BUILD_AUTOMATION_RUN_ENVELOPE_0: wire::Schema = wire::Schema::Dyn;

/// `buildAutomationRunEnvelope` over the binary wire.
#[napi(js_name = "buildAutomationRunEnvelopeWire")]
pub fn build_automation_run_envelope_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_AUTOMATION_RUN_ENVELOPE_0)?;
        let out = &port::build_automation_run_envelope(input)?;
        w.string16(out);
        Ok(())
    })
}

/// `automationRunResultSummary` over the binary wire.
#[napi(js_name = "automationRunResultSummaryWire")]
pub fn automation_run_result_summary_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Option<String> = r.opt_js(|r| r.string16())?;
        let fallback: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::automation_run_result_summary(value, fallback)?;
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

/// `automationRunResultSummaryWithNotice` over the binary wire.
#[napi(js_name = "automationRunResultSummaryWithNoticeWire")]
pub fn automation_run_result_summary_with_notice_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_summary: Option<String> = r.opt_js(|r| r.string16())?;
        let notice: String = r.string16()?;
        let out = &port::automation_run_result_summary_with_notice(base_summary, notice)?;
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

/// `normalizeAutomationCompletionReason` over the binary wire.
#[napi(js_name = "normalizeAutomationCompletionReasonWire")]
pub fn normalize_automation_completion_reason_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::normalize_automation_completion_reason(value)?;
        w.string16(out);
        Ok(())
    })
}

static S_FAILED_AUTOMATION_COMPLETION_EVALUATION_OUT: wire::Schema = wire::Schema::Dyn;

/// `failedAutomationCompletionEvaluation` over the binary wire.
#[napi(js_name = "failedAutomationCompletionEvaluationWire")]
pub fn failed_automation_completion_evaluation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let reason: String = r.string16()?;
        let out = &port::failed_automation_completion_evaluation(reason)?;
        wire::write_value(w, out, &S_FAILED_AUTOMATION_COMPLETION_EVALUATION_OUT)?;
        Ok(())
    })
}

static S_AUTOMATION_COMPLETION_RUN_RESULT_0: wire::Schema = wire::Schema::Dyn;
static S_AUTOMATION_COMPLETION_RUN_RESULT_OUT: wire::Schema = wire::Schema::Dyn;

/// `automationCompletionRunResult` over the binary wire.
#[napi(js_name = "automationCompletionRunResultWire")]
pub fn automation_completion_run_result_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_AUTOMATION_COMPLETION_RUN_RESULT_0)?;
        let out = &port::automation_completion_run_result(input)?;
        wire::write_value(w, out, &S_AUTOMATION_COMPLETION_RUN_RESULT_OUT)?;
        Ok(())
    })
}

static S_DETERMINISTIC_AUTOMATION_JITTER_SECONDS_0: wire::Schema = wire::Schema::Dyn;

/// `deterministicAutomationJitterSeconds` over the binary wire.
#[napi(js_name = "deterministicAutomationJitterSecondsWire")]
pub fn deterministic_automation_jitter_seconds_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_DETERMINISTIC_AUTOMATION_JITTER_SECONDS_0)?;
        let out = &port::deterministic_automation_jitter_seconds(input)?;
        w.f64(*out);
        Ok(())
    })
}

static S_COMPUTE_NEXT_AUTOMATION_RUN_AT_0: wire::Schema = wire::Schema::Dyn;

/// `computeNextAutomationRunAt` over the binary wire.
#[napi(js_name = "computeNextAutomationRunAtWire")]
pub fn compute_next_automation_run_at_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let schedule: serde_json::Value = wire::read_value(r, &S_COMPUTE_NEXT_AUTOMATION_RUN_AT_0)?;
        let from_iso: String = r.string16()?;
        let jitter_context: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::compute_next_automation_run_at(schedule, from_iso, jitter_context)?;
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

static S_COMPUTE_NEXT_AUTOMATION_RUN_AT_AFTER_0: wire::Schema = wire::Schema::Dyn;

/// `computeNextAutomationRunAtAfter` over the binary wire.
#[napi(js_name = "computeNextAutomationRunAtAfterWire")]
pub fn compute_next_automation_run_at_after_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let schedule: serde_json::Value =
            wire::read_value(r, &S_COMPUTE_NEXT_AUTOMATION_RUN_AT_AFTER_0)?;
        let from_iso: String = r.string16()?;
        let not_before_iso: String = r.string16()?;
        let jitter_context: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::compute_next_automation_run_at_after(
            schedule,
            from_iso,
            not_before_iso,
            jitter_context,
        )?;
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

static S_COMPUTE_AUTOMATION_SCHEDULE_SPACING_SECONDS_0: wire::Schema = wire::Schema::Dyn;

/// `computeAutomationScheduleSpacingSeconds` over the binary wire.
#[napi(js_name = "computeAutomationScheduleSpacingSecondsWire")]
pub fn compute_automation_schedule_spacing_seconds_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let schedule: serde_json::Value =
            wire::read_value(r, &S_COMPUTE_AUTOMATION_SCHEDULE_SPACING_SECONDS_0)?;
        let from_iso: String = r.string16()?;
        let out = &port::compute_automation_schedule_spacing_seconds(schedule, from_iso)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_RESOLVE_AUTOMATION_STOP_POLICY_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_AUTOMATION_STOP_POLICY_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_AUTOMATION_STOP_POLICY_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Num);

/// `resolveAutomationStopPolicy` over the binary wire.
#[napi(js_name = "resolveAutomationStopPolicyWire")]
pub fn resolve_automation_stop_policy_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_AUTOMATION_STOP_POLICY_0)?;
        let fallback: serde_json::Value = wire::read_value(r, &S_RESOLVE_AUTOMATION_STOP_POLICY_1)?;
        let out = &port::resolve_automation_stop_policy(input, fallback)?;
        wire::write_value(w, out, &S_RESOLVE_AUTOMATION_STOP_POLICY_OUT)?;
        Ok(())
    })
}
