//! JS bindings for `apps/web/src/storeNormalization.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod corpus;
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

/// `threadSessionsEqual`.
#[napi(js_name = "threadSessionsEqual")]
pub fn thread_sessions_equal(left: String, right: String) -> napi::Result<String> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::thread_sessions_equal(left, right)?;
        crate::js::to_json(&out)
    })
}

/// `resolveCreateBranchFlowCompletedMerge`.
#[napi(js_name = "resolveCreateBranchFlowCompletedMerge")]
pub fn resolve_create_branch_flow_completed_merge(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_create_branch_flow_completed_merge(input)?;
        crate::js::to_json(&out)
    })
}

/// `threadShellsEqual`.
#[napi(js_name = "threadShellsEqual")]
pub fn thread_shells_equal(left: String, right: String) -> napi::Result<String> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::thread_shells_equal(left, right)?;
        crate::js::to_json(&out)
    })
}

/// `threadTurnStatesEqual`.
#[napi(js_name = "threadTurnStatesEqual")]
pub fn thread_turn_states_equal(left: String, right: String) -> napi::Result<String> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::thread_turn_states_equal(left, right)?;
        crate::js::to_json(&out)
    })
}

/// `textSegmentArraysEqual`.
#[napi(js_name = "textSegmentArraysEqual")]
pub fn text_segment_arrays_equal(left: String, right: String) -> napi::Result<String> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::text_segment_arrays_equal(left, right)?;
        crate::js::to_json(&out)
    })
}

/// `providerReferenceArraysEqual`.
#[napi(js_name = "providerReferenceArraysEqual")]
pub fn provider_reference_arrays_equal(left: String, right: String) -> napi::Result<String> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::provider_reference_arrays_equal(left, right)?;
        crate::js::to_json(&out)
    })
}

/// `deepEqualJson`.
#[napi(js_name = "deepEqualJson")]
pub fn deep_equal_json(left: String, right: String) -> napi::Result<String> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::deep_equal_json(left, right)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeModelSelection`.
#[napi(js_name = "normalizeModelSelection")]
pub fn normalize_model_selection(value: String, previous: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::normalize_model_selection(value, previous)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeProject`.
#[napi(js_name = "normalizeProject")]
pub fn normalize_project(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::normalize_project(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeSpace`.
#[napi(js_name = "normalizeSpace")]
pub fn normalize_space(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::normalize_space(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `mapSpaces`.
#[napi(js_name = "mapSpaces")]
pub fn map_spaces(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::map_spaces(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeChatMessage`.
#[napi(js_name = "normalizeChatMessage")]
pub fn normalize_chat_message(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::normalize_chat_message(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `mergeReadModelThreadDetailWithLiveHotPath`.
#[napi(js_name = "mergeReadModelThreadDetailWithLiveHotPath")]
pub fn merge_read_model_thread_detail_with_live_hot_path(
    incoming: String,
    previous_thread: String,
    snapshot_sequence: String,
) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous_thread = crate::js::from_json(&previous_thread)?;
        let snapshot_sequence = crate::js::from_json(&snapshot_sequence)?;
        let out = port::merge_read_model_thread_detail_with_live_hot_path(
            incoming,
            previous_thread,
            snapshot_sequence,
        )?;
        crate::js::to_json(&out)
    })
}

/// `normalizeProposedPlans`.
#[napi(js_name = "normalizeProposedPlans")]
pub fn normalize_proposed_plans(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::normalize_proposed_plans(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeTurnDiffFiles`.
#[napi(js_name = "normalizeTurnDiffFiles")]
pub fn normalize_turn_diff_files(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::normalize_turn_diff_files(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeActivities`.
#[napi(js_name = "normalizeActivities")]
pub fn normalize_activities(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::normalize_activities(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `withOrchestrationEventSequence`.
#[napi(js_name = "withOrchestrationEventSequence")]
pub fn with_orchestration_event_sequence(
    activity: String,
    sequence: String,
) -> napi::Result<String> {
    run(move || {
        let activity = crate::js::from_json(&activity)?;
        let sequence = crate::js::from_json(&sequence)?;
        let out = port::with_orchestration_event_sequence(activity, sequence)?;
        crate::js::to_json(&out)
    })
}

/// `capThreadActivities`.
#[napi(js_name = "capThreadActivities")]
pub fn cap_thread_activities(activities: String) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let out = port::cap_thread_activities(activities)?;
        crate::js::to_json(&out)
    })
}

/// `dedupeActivitiesByIdAfterAppend`.
#[napi(js_name = "dedupeActivitiesByIdAfterAppend")]
pub fn dedupe_activities_by_id_after_append(
    activities: String,
    previous_activities: String,
    previous_activity_by_id: String,
) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let previous_activities = crate::js::from_json(&previous_activities)?;
        let previous_activity_by_id = crate::js::from_json(&previous_activity_by_id)?;
        let out = port::dedupe_activities_by_id_after_append(
            activities,
            previous_activities,
            previous_activity_by_id,
        )?;
        crate::js::to_json(&out)
    })
}

/// `dedupeActivitiesById`.
#[napi(js_name = "dedupeActivitiesById")]
pub fn dedupe_activities_by_id(activities: String) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let out = port::dedupe_activities_by_id(activities)?;
        crate::js::to_json(&out)
    })
}

/// `asActivityRecord`.
#[napi(js_name = "asActivityRecord")]
pub fn as_activity_record(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::as_activity_record(value)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeThreadErrorMessage`.
#[napi(js_name = "normalizeThreadErrorMessage")]
pub fn normalize_thread_error_message(message: String) -> napi::Result<String> {
    run(move || {
        let message = crate::js::from_json(&message)?;
        let out = port::normalize_thread_error_message(message)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeThreadSession`.
#[napi(js_name = "normalizeThreadSession")]
pub fn normalize_thread_session(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::normalize_thread_session(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeThreadFromReadModel`.
#[napi(js_name = "normalizeThreadFromReadModel")]
pub fn normalize_thread_from_read_model(
    incoming: String,
    previous: String,
    snapshot_sequence: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let snapshot_sequence = crate::js::from_json(&snapshot_sequence)?;
        let options = crate::js::from_json(&options)?;
        let out =
            port::normalize_thread_from_read_model(incoming, previous, snapshot_sequence, options)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeThreadShellSnapshot`.
#[napi(js_name = "normalizeThreadShellSnapshot")]
pub fn normalize_thread_shell_snapshot(
    incoming: String,
    previous: String,
    snapshot_sequence: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let snapshot_sequence = crate::js::from_json(&snapshot_sequence)?;
        let options = crate::js::from_json(&options)?;
        let out =
            port::normalize_thread_shell_snapshot(incoming, previous, snapshot_sequence, options)?;
        crate::js::to_json(&out)
    })
}

/// `mapProjects`.
#[napi(js_name = "mapProjects")]
pub fn map_projects(incoming: String, previous: String) -> napi::Result<String> {
    run(move || {
        let incoming = crate::js::from_json(&incoming)?;
        let previous = crate::js::from_json(&previous)?;
        let out = port::map_projects(incoming, previous)?;
        crate::js::to_json(&out)
    })
}

/// `toLegacyProvider`.
#[napi(js_name = "toLegacyProvider")]
pub fn to_legacy_provider(provider_name: String) -> napi::Result<String> {
    run(move || {
        let provider_name = crate::js::from_json(&provider_name)?;
        let out = port::to_legacy_provider(provider_name)?;
        crate::js::to_json(&out)
    })
}

/// `resolveThreadSidebarMetadata`.
#[napi(js_name = "resolveThreadSidebarMetadata")]
pub fn resolve_thread_sidebar_metadata(thread: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::resolve_thread_sidebar_metadata(thread)?;
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

static S_THREAD_SESSIONS_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_THREAD_SESSIONS_EQUAL_1: wire::Schema = wire::Schema::Dyn;
static S_THREAD_SESSIONS_EQUAL_OUT: wire::Schema = wire::Schema::Bool;

/// `threadSessionsEqual` over the binary wire.
#[napi(js_name = "threadSessionsEqualWire")]
pub fn thread_sessions_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_THREAD_SESSIONS_EQUAL_0)?;
        let right: Value = wire::read_value(r, &S_THREAD_SESSIONS_EQUAL_1)?;
        let out = &port::thread_sessions_equal(left, right)?;
        wire::write_value(w, out, &S_THREAD_SESSIONS_EQUAL_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_CREATE_BRANCH_FLOW_COMPLETED_MERGE_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_CREATE_BRANCH_FLOW_COMPLETED_MERGE_OUT: wire::Schema = wire::Schema::Bool;

/// `resolveCreateBranchFlowCompletedMerge` over the binary wire.
#[napi(js_name = "resolveCreateBranchFlowCompletedMergeWire")]
pub fn resolve_create_branch_flow_completed_merge_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_RESOLVE_CREATE_BRANCH_FLOW_COMPLETED_MERGE_0)?;
        let out = &port::resolve_create_branch_flow_completed_merge(input)?;
        wire::write_value(w, out, &S_RESOLVE_CREATE_BRANCH_FLOW_COMPLETED_MERGE_OUT)?;
        Ok(())
    })
}

static S_THREAD_SHELLS_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_THREAD_SHELLS_EQUAL_1: wire::Schema = wire::Schema::Dyn;
static S_THREAD_SHELLS_EQUAL_OUT: wire::Schema = wire::Schema::Bool;

/// `threadShellsEqual` over the binary wire.
#[napi(js_name = "threadShellsEqualWire")]
pub fn thread_shells_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_THREAD_SHELLS_EQUAL_0)?;
        let right: Value = wire::read_value(r, &S_THREAD_SHELLS_EQUAL_1)?;
        let out = &port::thread_shells_equal(left, right)?;
        wire::write_value(w, out, &S_THREAD_SHELLS_EQUAL_OUT)?;
        Ok(())
    })
}

static S_THREAD_TURN_STATES_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_THREAD_TURN_STATES_EQUAL_1: wire::Schema = wire::Schema::Dyn;
static S_THREAD_TURN_STATES_EQUAL_OUT: wire::Schema = wire::Schema::Bool;

/// `threadTurnStatesEqual` over the binary wire.
#[napi(js_name = "threadTurnStatesEqualWire")]
pub fn thread_turn_states_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_THREAD_TURN_STATES_EQUAL_0)?;
        let right: Value = wire::read_value(r, &S_THREAD_TURN_STATES_EQUAL_1)?;
        let out = &port::thread_turn_states_equal(left, right)?;
        wire::write_value(w, out, &S_THREAD_TURN_STATES_EQUAL_OUT)?;
        Ok(())
    })
}

static S_TEXT_SEGMENT_ARRAYS_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_TEXT_SEGMENT_ARRAYS_EQUAL_1: wire::Schema = wire::Schema::Dyn;
static S_TEXT_SEGMENT_ARRAYS_EQUAL_OUT: wire::Schema = wire::Schema::Bool;

/// `textSegmentArraysEqual` over the binary wire.
#[napi(js_name = "textSegmentArraysEqualWire")]
pub fn text_segment_arrays_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_TEXT_SEGMENT_ARRAYS_EQUAL_0)?;
        let right: Value = wire::read_value(r, &S_TEXT_SEGMENT_ARRAYS_EQUAL_1)?;
        let out = &port::text_segment_arrays_equal(left, right)?;
        wire::write_value(w, out, &S_TEXT_SEGMENT_ARRAYS_EQUAL_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_REFERENCE_ARRAYS_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_REFERENCE_ARRAYS_EQUAL_1: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_REFERENCE_ARRAYS_EQUAL_OUT: wire::Schema = wire::Schema::Bool;

/// `providerReferenceArraysEqual` over the binary wire.
#[napi(js_name = "providerReferenceArraysEqualWire")]
pub fn provider_reference_arrays_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_PROVIDER_REFERENCE_ARRAYS_EQUAL_0)?;
        let right: Value = wire::read_value(r, &S_PROVIDER_REFERENCE_ARRAYS_EQUAL_1)?;
        let out = &port::provider_reference_arrays_equal(left, right)?;
        wire::write_value(w, out, &S_PROVIDER_REFERENCE_ARRAYS_EQUAL_OUT)?;
        Ok(())
    })
}

static S_DEEP_EQUAL_JSON_0: wire::Schema = wire::Schema::Dyn;
static S_DEEP_EQUAL_JSON_1: wire::Schema = wire::Schema::Dyn;
static S_DEEP_EQUAL_JSON_OUT: wire::Schema = wire::Schema::Bool;

/// `deepEqualJson` over the binary wire.
#[napi(js_name = "deepEqualJsonWire")]
pub fn deep_equal_json_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_DEEP_EQUAL_JSON_0)?;
        let right: Value = wire::read_value(r, &S_DEEP_EQUAL_JSON_1)?;
        let out = &port::deep_equal_json(left, right)?;
        wire::write_value(w, out, &S_DEEP_EQUAL_JSON_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_MODEL_SELECTION_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeModelSelection` over the binary wire.
#[napi(js_name = "normalizeModelSelectionWire")]
pub fn normalize_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_MODEL_SELECTION_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_MODEL_SELECTION_1)?;
        let out = &port::normalize_model_selection(value, previous)?;
        wire::write_value(w, out, &S_NORMALIZE_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_PROJECT_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PROJECT_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PROJECT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("id", wire::Schema::Dyn),
    ("kind", wire::Schema::Dyn),
    ("name", wire::Schema::Str16),
    ("remoteName", wire::Schema::Str16),
    ("folderName", wire::Schema::Str16),
    ("localName", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("appearance", wire::Schema::Dyn),
    ("cwd", wire::Schema::Str16),
    ("defaultModelSelection", wire::Schema::Dyn),
    ("expanded", wire::Schema::Bool),
    ("isPinned", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("spaceId", wire::Schema::Dyn),
    ("createdAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("scripts", wire::Schema::List(&wire::Schema::Dyn)),
    (
        "additionalFolders",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16)),
    ),
]);

/// `normalizeProject` over the binary wire.
#[napi(js_name = "normalizeProjectWire")]
pub fn normalize_project_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_PROJECT_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_PROJECT_1)?;
        let out = &port::normalize_project(incoming, previous)?;
        wire::write_value(w, out, &S_NORMALIZE_PROJECT_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_SPACE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_SPACE_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_SPACE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("id", wire::Schema::Dyn),
    ("name", wire::Schema::Str16),
    ("icon", wire::Schema::Dyn),
    ("sortOrder", wire::Schema::Num),
    ("createdAt", wire::Schema::Str16),
    ("updatedAt", wire::Schema::Str16),
]);

/// `normalizeSpace` over the binary wire.
#[napi(js_name = "normalizeSpaceWire")]
pub fn normalize_space_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_SPACE_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_SPACE_1)?;
        let out = &port::normalize_space(incoming, previous)?;
        wire::write_value(w, out, &S_NORMALIZE_SPACE_OUT)?;
        Ok(())
    })
}

static S_MAP_SPACES_0: wire::Schema = wire::Schema::Dyn;
static S_MAP_SPACES_1: wire::Schema = wire::Schema::Dyn;
static S_MAP_SPACES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Dyn),
    ("name", wire::Schema::Str16),
    ("icon", wire::Schema::Dyn),
    ("sortOrder", wire::Schema::Num),
    ("createdAt", wire::Schema::Str16),
    ("updatedAt", wire::Schema::Str16),
]));

/// `mapSpaces` over the binary wire.
#[napi(js_name = "mapSpacesWire")]
pub fn map_spaces_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_MAP_SPACES_0)?;
        let previous: Value = wire::read_value(r, &S_MAP_SPACES_1)?;
        let out = &port::map_spaces(incoming, previous)?;
        wire::write_value(w, out, &S_MAP_SPACES_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_CHAT_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_CHAT_MESSAGE_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_CHAT_MESSAGE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("id", wire::Schema::Dyn),
    ("role", wire::Schema::Str16),
    ("text", wire::Schema::Str16),
    (
        "textSegments",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    ("asyncUserInput", wire::Schema::Dyn),
    (
        "attachments",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    (
        "skills",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    (
        "mentions",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    ("dispatchMode", wire::Schema::Dyn),
    ("dispatchOrigin", wire::Schema::Dyn),
    ("startsNewTurn", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("turnId", wire::Schema::Dyn),
    ("createdAt", wire::Schema::Str16),
    ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("completedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("streaming", wire::Schema::Bool),
    ("source", wire::Schema::Dyn),
]);

/// `normalizeChatMessage` over the binary wire.
#[napi(js_name = "normalizeChatMessageWire")]
pub fn normalize_chat_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_CHAT_MESSAGE_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_CHAT_MESSAGE_1)?;
        let out = &port::normalize_chat_message(incoming, previous)?;
        wire::write_value(w, out, &S_NORMALIZE_CHAT_MESSAGE_OUT)?;
        Ok(())
    })
}

static S_MERGE_READ_MODEL_THREAD_DETAIL_WITH_LIVE_HOT_PATH_0: wire::Schema = wire::Schema::Dyn;
static S_MERGE_READ_MODEL_THREAD_DETAIL_WITH_LIVE_HOT_PATH_1: wire::Schema = wire::Schema::Dyn;
static S_MERGE_READ_MODEL_THREAD_DETAIL_WITH_LIVE_HOT_PATH_2: wire::Schema = wire::Schema::Dyn;
static S_MERGE_READ_MODEL_THREAD_DETAIL_WITH_LIVE_HOT_PATH_OUT: wire::Schema = wire::Schema::Dyn;

/// `mergeReadModelThreadDetailWithLiveHotPath` over the binary wire.
#[napi(js_name = "mergeReadModelThreadDetailWithLiveHotPathWire")]
pub fn merge_read_model_thread_detail_with_live_hot_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value =
            wire::read_value(r, &S_MERGE_READ_MODEL_THREAD_DETAIL_WITH_LIVE_HOT_PATH_0)?;
        let previous_thread: Value =
            wire::read_value(r, &S_MERGE_READ_MODEL_THREAD_DETAIL_WITH_LIVE_HOT_PATH_1)?;
        let snapshot_sequence: Value =
            wire::read_value(r, &S_MERGE_READ_MODEL_THREAD_DETAIL_WITH_LIVE_HOT_PATH_2)?;
        let out = &port::merge_read_model_thread_detail_with_live_hot_path(
            incoming,
            previous_thread,
            snapshot_sequence,
        )?;
        wire::write_value(
            w,
            out,
            &S_MERGE_READ_MODEL_THREAD_DETAIL_WITH_LIVE_HOT_PATH_OUT,
        )?;
        Ok(())
    })
}

static S_NORMALIZE_PROPOSED_PLANS_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PROPOSED_PLANS_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PROPOSED_PLANS_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeProposedPlans` over the binary wire.
#[napi(js_name = "normalizeProposedPlansWire")]
pub fn normalize_proposed_plans_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_PROPOSED_PLANS_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_PROPOSED_PLANS_1)?;
        let out = &port::normalize_proposed_plans(incoming, previous)?;
        wire::write_value(w, out, &S_NORMALIZE_PROPOSED_PLANS_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_TURN_DIFF_FILES_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_TURN_DIFF_FILES_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_TURN_DIFF_FILES_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeTurnDiffFiles` over the binary wire.
#[napi(js_name = "normalizeTurnDiffFilesWire")]
pub fn normalize_turn_diff_files_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_TURN_DIFF_FILES_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_TURN_DIFF_FILES_1)?;
        let out = &port::normalize_turn_diff_files(incoming, previous)?;
        wire::write_value(w, out, &S_NORMALIZE_TURN_DIFF_FILES_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_ACTIVITIES_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_ACTIVITIES_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_ACTIVITIES_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeActivities` over the binary wire.
#[napi(js_name = "normalizeActivitiesWire")]
pub fn normalize_activities_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_ACTIVITIES_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_ACTIVITIES_1)?;
        let out = &port::normalize_activities(incoming, previous)?;
        wire::write_value(w, out, &S_NORMALIZE_ACTIVITIES_OUT)?;
        Ok(())
    })
}

static S_WITH_ORCHESTRATION_EVENT_SEQUENCE_0: wire::Schema = wire::Schema::Dyn;
static S_WITH_ORCHESTRATION_EVENT_SEQUENCE_1: wire::Schema = wire::Schema::Dyn;
static S_WITH_ORCHESTRATION_EVENT_SEQUENCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `withOrchestrationEventSequence` over the binary wire.
#[napi(js_name = "withOrchestrationEventSequenceWire")]
pub fn with_orchestration_event_sequence_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activity: Value = wire::read_value(r, &S_WITH_ORCHESTRATION_EVENT_SEQUENCE_0)?;
        let sequence: Value = wire::read_value(r, &S_WITH_ORCHESTRATION_EVENT_SEQUENCE_1)?;
        let out = &port::with_orchestration_event_sequence(activity, sequence)?;
        wire::write_value(w, out, &S_WITH_ORCHESTRATION_EVENT_SEQUENCE_OUT)?;
        Ok(())
    })
}

static S_CAP_THREAD_ACTIVITIES_0: wire::Schema = wire::Schema::Dyn;
static S_CAP_THREAD_ACTIVITIES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `capThreadActivities` over the binary wire.
#[napi(js_name = "capThreadActivitiesWire")]
pub fn cap_thread_activities_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: Value = wire::read_value(r, &S_CAP_THREAD_ACTIVITIES_0)?;
        let out = &port::cap_thread_activities(activities)?;
        wire::write_value(w, out, &S_CAP_THREAD_ACTIVITIES_OUT)?;
        Ok(())
    })
}

static S_DEDUPE_ACTIVITIES_BY_ID_AFTER_APPEND_0: wire::Schema = wire::Schema::Dyn;
static S_DEDUPE_ACTIVITIES_BY_ID_AFTER_APPEND_1: wire::Schema = wire::Schema::Dyn;
static S_DEDUPE_ACTIVITIES_BY_ID_AFTER_APPEND_2: wire::Schema = wire::Schema::Dyn;
static S_DEDUPE_ACTIVITIES_BY_ID_AFTER_APPEND_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Dyn);

/// `dedupeActivitiesByIdAfterAppend` over the binary wire.
#[napi(js_name = "dedupeActivitiesByIdAfterAppendWire")]
pub fn dedupe_activities_by_id_after_append_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: Value = wire::read_value(r, &S_DEDUPE_ACTIVITIES_BY_ID_AFTER_APPEND_0)?;
        let previous_activities: Value =
            wire::read_value(r, &S_DEDUPE_ACTIVITIES_BY_ID_AFTER_APPEND_1)?;
        let previous_activity_by_id: Value =
            wire::read_value(r, &S_DEDUPE_ACTIVITIES_BY_ID_AFTER_APPEND_2)?;
        let out = &port::dedupe_activities_by_id_after_append(
            activities,
            previous_activities,
            previous_activity_by_id,
        )?;
        wire::write_value(w, out, &S_DEDUPE_ACTIVITIES_BY_ID_AFTER_APPEND_OUT)?;
        Ok(())
    })
}

static S_DEDUPE_ACTIVITIES_BY_ID_0: wire::Schema = wire::Schema::Dyn;
static S_DEDUPE_ACTIVITIES_BY_ID_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `dedupeActivitiesById` over the binary wire.
#[napi(js_name = "dedupeActivitiesByIdWire")]
pub fn dedupe_activities_by_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: Value = wire::read_value(r, &S_DEDUPE_ACTIVITIES_BY_ID_0)?;
        let out = &port::dedupe_activities_by_id(activities)?;
        wire::write_value(w, out, &S_DEDUPE_ACTIVITIES_BY_ID_OUT)?;
        Ok(())
    })
}

static S_AS_ACTIVITY_RECORD_0: wire::Schema = wire::Schema::Dyn;
static S_AS_ACTIVITY_RECORD_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Dyn));

/// `asActivityRecord` over the binary wire.
#[napi(js_name = "asActivityRecordWire")]
pub fn as_activity_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_AS_ACTIVITY_RECORD_0)?;
        let out = &port::as_activity_record(value)?;
        wire::write_value(w, out, &S_AS_ACTIVITY_RECORD_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_THREAD_ERROR_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_ERROR_MESSAGE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `normalizeThreadErrorMessage` over the binary wire.
#[napi(js_name = "normalizeThreadErrorMessageWire")]
pub fn normalize_thread_error_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: Value = wire::read_value(r, &S_NORMALIZE_THREAD_ERROR_MESSAGE_0)?;
        let out = &port::normalize_thread_error_message(message)?;
        wire::write_value(w, out, &S_NORMALIZE_THREAD_ERROR_MESSAGE_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_THREAD_SESSION_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_SESSION_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_SESSION_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeThreadSession` over the binary wire.
#[napi(js_name = "normalizeThreadSessionWire")]
pub fn normalize_thread_session_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_THREAD_SESSION_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_THREAD_SESSION_1)?;
        let out = &port::normalize_thread_session(incoming, previous)?;
        wire::write_value(w, out, &S_NORMALIZE_THREAD_SESSION_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_THREAD_FROM_READ_MODEL_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_FROM_READ_MODEL_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_FROM_READ_MODEL_2: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_FROM_READ_MODEL_3: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_FROM_READ_MODEL_OUT: wire::Schema = wire::Schema::Obj(&[
    ("isProjectImport", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("id", wire::Schema::Dyn),
    ("codexThreadId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("projectId", wire::Schema::Dyn),
    ("title", wire::Schema::Str16),
    ("modelSelection", wire::Schema::Dyn),
    ("runtimeMode", wire::Schema::Dyn),
    ("interactionMode", wire::Schema::Dyn),
    (
        "session",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("provider", wire::Schema::Dyn),
            ("runtimeMode", wire::Schema::Dyn),
            ("providerInstanceId", wire::Schema::Dyn),
            ("status", wire::Schema::Dyn),
            ("activeTurnId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::Str16),
            ("updatedAt", wire::Schema::Str16),
            ("lastError", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("orchestrationStatus", wire::Schema::Dyn),
        ])),
    ),
    (
        "messages",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("role", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
            (
                "textSegments",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            ("asyncUserInput", wire::Schema::Dyn),
            (
                "attachments",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            (
                "skills",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            (
                "mentions",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            ("dispatchMode", wire::Schema::Dyn),
            ("dispatchOrigin", wire::Schema::Dyn),
            ("startsNewTurn", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("turnId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::Str16),
            ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("completedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("streaming", wire::Schema::Bool),
            ("source", wire::Schema::Dyn),
        ])),
    ),
    (
        "proposedPlans",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("turnId", wire::Schema::Dyn),
            ("planMarkdown", wire::Schema::Str16),
            ("implementedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("implementationThreadId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::Str16),
            ("updatedAt", wire::Schema::Str16),
        ])),
    ),
    ("error", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("createdAt", wire::Schema::Str16),
    ("archivedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("settledAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("snoozedUntil", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "snoozeReminderAt",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("isPinned", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "pinnedMessages",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    ("notes", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("goal", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("goalStartedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("goalPausedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "goalAchievements",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    ("latestTurn", wire::Schema::Dyn),
    ("pendingSourceProposedPlan", wire::Schema::Dyn),
    ("pendingTurnStartMessageId", wire::Schema::Dyn),
    ("lastVisitedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("parentThreadId", wire::Schema::Dyn),
    ("creationSource", wire::Schema::Dyn),
    ("sourceThreadId", wire::Schema::Dyn),
    ("subagentAgentId", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "subagentNickname",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("subagentRole", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("forkSourceThreadId", wire::Schema::Dyn),
    ("sidechatSourceThreadId", wire::Schema::Dyn),
    ("sidechatContext", wire::Schema::Dyn),
    (
        "sidechatLastActivityAt",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "sidechatExpiredAt",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("handoff", wire::Schema::Dyn),
    ("usageLimitRecovery", wire::Schema::Dyn),
    (
        "usageLimitRecoverySequence",
        wire::Schema::OptJs(&wire::Schema::Num),
    ),
    ("claudeCacheReview", wire::Schema::Dyn),
    (
        "claudeCacheReviewSequence",
        wire::Schema::OptJs(&wire::Schema::Num),
    ),
    ("snoozeSequence", wire::Schema::OptJs(&wire::Schema::Num)),
    ("lastKnownPr", wire::Schema::Dyn),
    (
        "pullRequests",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    (
        "latestUserMessageAt",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "latestHumanMessageAt",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "hasPendingApprovals",
        wire::Schema::OptJs(&wire::Schema::Bool),
    ),
    (
        "hasPendingUserInput",
        wire::Schema::OptJs(&wire::Schema::Bool),
    ),
    (
        "hasActionableProposedPlan",
        wire::Schema::OptJs(&wire::Schema::Bool),
    ),
    (
        "pendingBackgroundTaskCount",
        wire::Schema::OptJs(&wire::Schema::Num),
    ),
    (
        "latestReplyPreview",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "pendingInteractions",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    (
        "turnDiffSummaries",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("turnId", wire::Schema::Dyn),
            ("startedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("completedAt", wire::Schema::Str16),
            ("status", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("files", wire::Schema::List(&wire::Schema::Dyn)),
            ("checkpointRef", wire::Schema::Dyn),
            ("assistantMessageId", wire::Schema::Dyn),
            (
                "checkpointTurnCount",
                wire::Schema::OptJs(&wire::Schema::Num),
            ),
            (
                "checkpointTurnCounts",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
        ])),
    ),
    ("activities", wire::Schema::List(&wire::Schema::Dyn)),
]);

/// `normalizeThreadFromReadModel` over the binary wire.
#[napi(js_name = "normalizeThreadFromReadModelWire")]
pub fn normalize_thread_from_read_model_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_THREAD_FROM_READ_MODEL_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_THREAD_FROM_READ_MODEL_1)?;
        let snapshot_sequence: Value = wire::read_value(r, &S_NORMALIZE_THREAD_FROM_READ_MODEL_2)?;
        let options: Value = wire::read_value(r, &S_NORMALIZE_THREAD_FROM_READ_MODEL_3)?;
        let out = &port::normalize_thread_from_read_model(
            incoming,
            previous,
            snapshot_sequence,
            options,
        )?;
        wire::write_value(w, out, &S_NORMALIZE_THREAD_FROM_READ_MODEL_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_THREAD_SHELL_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_SHELL_SNAPSHOT_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_SHELL_SNAPSHOT_2: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_SHELL_SNAPSHOT_3: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THREAD_SHELL_SNAPSHOT_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "shell",
        wire::Schema::Obj(&[
            ("isProjectImport", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("id", wire::Schema::Dyn),
            ("codexThreadId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("projectId", wire::Schema::Dyn),
            ("title", wire::Schema::Str16),
            ("modelSelection", wire::Schema::Dyn),
            ("runtimeMode", wire::Schema::Dyn),
            ("interactionMode", wire::Schema::Dyn),
            ("error", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("createdAt", wire::Schema::Str16),
            ("archivedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("settledAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("snoozedUntil", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "snoozeReminderAt",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("isPinned", wire::Schema::OptJs(&wire::Schema::Bool)),
            (
                "pinnedMessages",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            ("notes", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("goal", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("goalStartedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("goalPausedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "goalAchievements",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            ("parentThreadId", wire::Schema::Dyn),
            ("creationSource", wire::Schema::Dyn),
            ("sourceThreadId", wire::Schema::Dyn),
            ("subagentAgentId", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "subagentNickname",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("subagentRole", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("forkSourceThreadId", wire::Schema::Dyn),
            ("sidechatSourceThreadId", wire::Schema::Dyn),
            ("sidechatContext", wire::Schema::Dyn),
            (
                "sidechatLastActivityAt",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "sidechatExpiredAt",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("handoff", wire::Schema::Dyn),
            ("usageLimitRecovery", wire::Schema::Dyn),
            (
                "usageLimitRecoverySequence",
                wire::Schema::OptJs(&wire::Schema::Num),
            ),
            ("claudeCacheReview", wire::Schema::Dyn),
            (
                "claudeCacheReviewSequence",
                wire::Schema::OptJs(&wire::Schema::Num),
            ),
            ("snoozeSequence", wire::Schema::OptJs(&wire::Schema::Num)),
            ("lastKnownPr", wire::Schema::Dyn),
            (
                "pullRequests",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            (
                "latestUserMessageAt",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "latestHumanMessageAt",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "hasPendingApprovals",
                wire::Schema::OptJs(&wire::Schema::Bool),
            ),
            (
                "hasPendingUserInput",
                wire::Schema::OptJs(&wire::Schema::Bool),
            ),
            (
                "hasActionableProposedPlan",
                wire::Schema::OptJs(&wire::Schema::Bool),
            ),
            (
                "pendingBackgroundTaskCount",
                wire::Schema::OptJs(&wire::Schema::Num),
            ),
            (
                "latestReplyPreview",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "pendingInteractions",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            ("lastVisitedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ]),
    ),
    (
        "session",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("provider", wire::Schema::Dyn),
            ("runtimeMode", wire::Schema::Dyn),
            ("providerInstanceId", wire::Schema::Dyn),
            ("status", wire::Schema::Dyn),
            ("activeTurnId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::Str16),
            ("updatedAt", wire::Schema::Str16),
            ("lastError", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("orchestrationStatus", wire::Schema::Dyn),
        ])),
    ),
    (
        "turnState",
        wire::Schema::Obj(&[
            ("latestTurn", wire::Schema::Dyn),
            ("pendingSourceProposedPlan", wire::Schema::Dyn),
            ("pendingTurnStartMessageId", wire::Schema::Dyn),
        ]),
    ),
]);

/// `normalizeThreadShellSnapshot` over the binary wire.
#[napi(js_name = "normalizeThreadShellSnapshotWire")]
pub fn normalize_thread_shell_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_NORMALIZE_THREAD_SHELL_SNAPSHOT_0)?;
        let previous: Value = wire::read_value(r, &S_NORMALIZE_THREAD_SHELL_SNAPSHOT_1)?;
        let snapshot_sequence: Value = wire::read_value(r, &S_NORMALIZE_THREAD_SHELL_SNAPSHOT_2)?;
        let options: Value = wire::read_value(r, &S_NORMALIZE_THREAD_SHELL_SNAPSHOT_3)?;
        let out =
            &port::normalize_thread_shell_snapshot(incoming, previous, snapshot_sequence, options)?;
        wire::write_value(w, out, &S_NORMALIZE_THREAD_SHELL_SNAPSHOT_OUT)?;
        Ok(())
    })
}

static S_MAP_PROJECTS_0: wire::Schema = wire::Schema::Dyn;
static S_MAP_PROJECTS_1: wire::Schema = wire::Schema::Dyn;
static S_MAP_PROJECTS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Dyn),
    ("kind", wire::Schema::Dyn),
    ("name", wire::Schema::Str16),
    ("remoteName", wire::Schema::Str16),
    ("folderName", wire::Schema::Str16),
    ("localName", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("appearance", wire::Schema::Dyn),
    ("cwd", wire::Schema::Str16),
    ("defaultModelSelection", wire::Schema::Dyn),
    ("expanded", wire::Schema::Bool),
    ("isPinned", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("spaceId", wire::Schema::Dyn),
    ("createdAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("scripts", wire::Schema::List(&wire::Schema::Dyn)),
    (
        "additionalFolders",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16)),
    ),
]));

/// `mapProjects` over the binary wire.
#[napi(js_name = "mapProjectsWire")]
pub fn map_projects_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let incoming: Value = wire::read_value(r, &S_MAP_PROJECTS_0)?;
        let previous: Value = wire::read_value(r, &S_MAP_PROJECTS_1)?;
        let out = &port::map_projects(incoming, previous)?;
        wire::write_value(w, out, &S_MAP_PROJECTS_OUT)?;
        Ok(())
    })
}

static S_TO_LEGACY_PROVIDER_0: wire::Schema = wire::Schema::Dyn;
static S_TO_LEGACY_PROVIDER_OUT: wire::Schema = wire::Schema::Dyn;

/// `toLegacyProvider` over the binary wire.
#[napi(js_name = "toLegacyProviderWire")]
pub fn to_legacy_provider_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider_name: Value = wire::read_value(r, &S_TO_LEGACY_PROVIDER_0)?;
        let out = &port::to_legacy_provider(provider_name)?;
        wire::write_value(w, out, &S_TO_LEGACY_PROVIDER_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_THREAD_SIDEBAR_METADATA_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_THREAD_SIDEBAR_METADATA_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveThreadSidebarMetadata` over the binary wire.
#[napi(js_name = "resolveThreadSidebarMetadataWire")]
pub fn resolve_thread_sidebar_metadata_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_RESOLVE_THREAD_SIDEBAR_METADATA_0)?;
        let out = &port::resolve_thread_sidebar_metadata(thread)?;
        wire::write_value(w, out, &S_RESOLVE_THREAD_SIDEBAR_METADATA_OUT)?;
        Ok(())
    })
}
