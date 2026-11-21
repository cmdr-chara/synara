//! JS bindings for `apps/web/src/workLog.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isFileChangeWorkLogEntry`.
#[napi(js_name = "isFileChangeWorkLogEntry")]
pub fn is_file_change_work_log_entry(work_entry: String) -> napi::Result<bool> {
    run(move || {
        let work_entry = crate::js::from_json(&work_entry)?;
        let out = port::is_file_change_work_log_entry(work_entry)?;
        Ok(out)
    })
}

/// `isProviderFileEditWorkLogEntry`.
#[napi(js_name = "isProviderFileEditWorkLogEntry")]
pub fn is_provider_file_edit_work_log_entry(work_entry: String) -> napi::Result<bool> {
    run(move || {
        let work_entry = crate::js::from_json(&work_entry)?;
        let out = port::is_provider_file_edit_work_log_entry(work_entry)?;
        Ok(out)
    })
}

/// `orderedActivities`.
#[napi(js_name = "orderedActivities")]
pub fn ordered_activities(activities: String) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let out = port::ordered_activities(activities)?;
        crate::js::to_json(&out)
    })
}

/// `isRoutedSubagentWorkEntry`.
#[napi(js_name = "isRoutedSubagentWorkEntry")]
pub fn is_routed_subagent_work_entry(entry: String) -> napi::Result<bool> {
    run(move || {
        let entry = crate::js::from_json(&entry)?;
        let out = port::is_routed_subagent_work_entry(entry)?;
        Ok(out)
    })
}

/// `isSubagentStateOnlyWorkEntry`.
#[napi(js_name = "isSubagentStateOnlyWorkEntry")]
pub fn is_subagent_state_only_work_entry(entry: String) -> napi::Result<bool> {
    run(move || {
        let entry = crate::js::from_json(&entry)?;
        let out = port::is_subagent_state_only_work_entry(entry)?;
        Ok(out)
    })
}

/// `omitRoutedSubagentWorkEntries`.
#[napi(js_name = "omitRoutedSubagentWorkEntries")]
pub fn omit_routed_subagent_work_entries(entries: String) -> napi::Result<String> {
    run(move || {
        let entries = crate::js::from_json(&entries)?;
        let out = port::omit_routed_subagent_work_entries(entries)?;
        crate::js::to_json(&out)
    })
}

/// `isMissingCheckpointBaselineActivity`.
#[napi(js_name = "isMissingCheckpointBaselineActivity")]
pub fn is_missing_checkpoint_baseline_activity(activity: String) -> napi::Result<bool> {
    run(move || {
        let activity = crate::js::from_json(&activity)?;
        let out = port::is_missing_checkpoint_baseline_activity(activity)?;
        Ok(out)
    })
}

/// `isCodexRuntimeWarningType`.
#[napi(js_name = "isCodexRuntimeWarningType")]
pub fn is_codex_runtime_warning_type(native_event_type: String) -> napi::Result<bool> {
    run(move || {
        let native_event_type = crate::js::from_json(&native_event_type)?;
        let out = port::is_codex_runtime_warning_type(native_event_type)?;
        Ok(out)
    })
}

/// `parseTaskListTasks`.
#[napi(js_name = "parseTaskListTasks")]
pub fn parse_task_list_tasks(payload: String) -> napi::Result<String> {
    run(move || {
        let payload = crate::js::from_json(&payload)?;
        let out = port::parse_task_list_tasks(payload)?;
        crate::js::to_json(&out)
    })
}

/// `extractProviderHandoffInfo`.
#[napi(js_name = "extractProviderHandoffInfo")]
pub fn extract_provider_handoff_info(payload: String, status: String) -> napi::Result<String> {
    run(move || {
        let payload = crate::js::from_json(&payload)?;
        let status = crate::js::from_json(&status)?;
        let out = port::extract_provider_handoff_info(payload, status)?;
        crate::js::to_json(&out)
    })
}

/// `parseUserInputQuestions`.
#[napi(js_name = "parseUserInputQuestions")]
pub fn parse_user_input_questions(payload: String) -> napi::Result<String> {
    run(move || {
        let payload = crate::js::from_json(&payload)?;
        let out = port::parse_user_input_questions(payload)?;
        crate::js::to_json(&out)
    })
}

/// `deriveSubagentTaskEnds`.
#[napi(js_name = "deriveSubagentTaskEnds")]
pub fn derive_subagent_task_ends(activities: String) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let out = port::derive_subagent_task_ends(activities)?;
        crate::js::to_json(&out)
    })
}

/// `deriveThreadBackgroundTasks`.
#[napi(js_name = "deriveThreadBackgroundTasks")]
pub fn derive_thread_background_tasks(activities: String) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let out = port::derive_thread_background_tasks(activities)?;
        crate::js::to_json(&out)
    })
}

/// `deriveWorkLogEntries`.
#[napi(js_name = "deriveWorkLogEntries")]
pub fn derive_work_log_entries(
    activities: String,
    latest_turn_id: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let latest_turn_id = crate::js::from_json(&latest_turn_id)?;
        let options = crate::js::from_json(&options)?;
        let out = port::derive_work_log_entries(activities, latest_turn_id, options)?;
        crate::js::to_json(&out)
    })
}

/// `deriveTimelineEntries`.
#[napi(js_name = "deriveTimelineEntries")]
pub fn derive_timeline_entries(
    messages: String,
    proposed_plans: String,
    work_entries: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let messages = crate::js::from_json(&messages)?;
        let proposed_plans = crate::js::from_json(&proposed_plans)?;
        let work_entries = crate::js::from_json(&work_entries)?;
        let options = crate::js::from_json(&options)?;
        let out = port::derive_timeline_entries(messages, proposed_plans, work_entries, options)?;
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

static S_IS_FILE_CHANGE_WORK_LOG_ENTRY_0: wire::Schema = wire::Schema::Dyn;

/// `isFileChangeWorkLogEntry` over the binary wire.
#[napi(js_name = "isFileChangeWorkLogEntryWire")]
pub fn is_file_change_work_log_entry_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let work_entry: Value = wire::read_value(r, &S_IS_FILE_CHANGE_WORK_LOG_ENTRY_0)?;
        let out = &port::is_file_change_work_log_entry(work_entry)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_PROVIDER_FILE_EDIT_WORK_LOG_ENTRY_0: wire::Schema = wire::Schema::Dyn;

/// `isProviderFileEditWorkLogEntry` over the binary wire.
#[napi(js_name = "isProviderFileEditWorkLogEntryWire")]
pub fn is_provider_file_edit_work_log_entry_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let work_entry: Value = wire::read_value(r, &S_IS_PROVIDER_FILE_EDIT_WORK_LOG_ENTRY_0)?;
        let out = &port::is_provider_file_edit_work_log_entry(work_entry)?;
        w.bool(*out);
        Ok(())
    })
}

static S_ORDERED_ACTIVITIES_0: wire::Schema = wire::Schema::Dyn;
static S_ORDERED_ACTIVITIES_OUT: wire::Schema = wire::Schema::Dyn;

/// `orderedActivities` over the binary wire.
#[napi(js_name = "orderedActivitiesWire")]
pub fn ordered_activities_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: Value = wire::read_value(r, &S_ORDERED_ACTIVITIES_0)?;
        let out = &port::ordered_activities(activities)?;
        wire::write_value(w, out, &S_ORDERED_ACTIVITIES_OUT)?;
        Ok(())
    })
}

static S_IS_ROUTED_SUBAGENT_WORK_ENTRY_0: wire::Schema = wire::Schema::Dyn;

/// `isRoutedSubagentWorkEntry` over the binary wire.
#[napi(js_name = "isRoutedSubagentWorkEntryWire")]
pub fn is_routed_subagent_work_entry_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let entry: Value = wire::read_value(r, &S_IS_ROUTED_SUBAGENT_WORK_ENTRY_0)?;
        let out = &port::is_routed_subagent_work_entry(entry)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_SUBAGENT_STATE_ONLY_WORK_ENTRY_0: wire::Schema = wire::Schema::Dyn;

/// `isSubagentStateOnlyWorkEntry` over the binary wire.
#[napi(js_name = "isSubagentStateOnlyWorkEntryWire")]
pub fn is_subagent_state_only_work_entry_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let entry: Value = wire::read_value(r, &S_IS_SUBAGENT_STATE_ONLY_WORK_ENTRY_0)?;
        let out = &port::is_subagent_state_only_work_entry(entry)?;
        w.bool(*out);
        Ok(())
    })
}

static S_OMIT_ROUTED_SUBAGENT_WORK_ENTRIES_0: wire::Schema = wire::Schema::Dyn;
static S_OMIT_ROUTED_SUBAGENT_WORK_ENTRIES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Str16),
        ("createdAt", wire::Schema::Str16),
        ("sequence", wire::Schema::OptJs(&wire::Schema::Num)),
        ("turnId", wire::Schema::Dyn),
        ("label", wire::Schema::Str16),
        ("detail", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("command", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("rawCommand", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("preview", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "changedFiles",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16)),
        ),
        ("tone", wire::Schema::Str16),
        ("toolTitle", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "displayToolTitle",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("toolName", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("mcpServer", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("toolCallId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("toolStatus", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "liveActivity",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("state", wire::Schema::Dyn),
                ("label", wire::Schema::Str16),
                ("startedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("lastActivityAt", wire::Schema::Str16),
                ("detail", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("progress", wire::Schema::OptJs(&wire::Schema::Num)),
                ("elapsedSeconds", wire::Schema::OptJs(&wire::Schema::Num)),
            ])),
        ),
        (
            "toolDetails",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("kind", wire::Schema::Str16),
                ("title", wire::Schema::Str16),
                ("command", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("output", wire::Schema::Dyn),
                ("diff", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("content", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("edits", wire::Schema::Dyn),
                ("files", wire::Schema::Dyn),
            ])),
        ),
        ("itemType", wire::Schema::Dyn),
        ("requestKind", wire::Schema::Dyn),
        (
            "grantedCommandApproval",
            wire::Schema::OptJs(&wire::Schema::Bool),
        ),
        (
            "subagents",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
        ),
        (
            "subagentAction",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("tool", wire::Schema::Str16),
                ("status", wire::Schema::Str16),
                ("summaryText", wire::Schema::Str16),
                ("model", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("prompt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ])),
        ),
        (
            "subagentRun",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("entryIds", wire::Schema::Dyn),
                ("members", wire::Schema::Dyn),
            ])),
        ),
        (
            "automation",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("id", wire::Schema::Str16),
                ("name", wire::Schema::Str16),
                ("cadenceLabel", wire::Schema::Str16),
                ("proposalState", wire::Schema::OptJs(&wire::Schema::Str16)),
            ])),
        ),
        (
            "connectorProposalId",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        (
            "synaraThreadCreation",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("operationId", wire::Schema::Str16),
                ("requestedCount", wire::Schema::Num),
                ("createdCount", wire::Schema::Num),
                ("threads", wire::Schema::Dyn),
                ("environmentId", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("hostName", wire::Schema::OptJs(&wire::Schema::Str16)),
            ])),
        ),
        (
            "synaraWorkerNotice",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("kind", wire::Schema::Str16),
                ("marker", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("phrase", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("threads", wire::Schema::Dyn),
                ("actions", wire::Schema::Dyn),
            ])),
        ),
        (
            "monitorNotification",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("taskId", wire::Schema::Str16),
                ("name", wire::Schema::Str16),
                ("output", wire::Schema::Str16),
                ("outcome", wire::Schema::Str16),
                ("midTurn", wire::Schema::OptJs(&wire::Schema::Bool)),
            ])),
        ),
        (
            "backgroundTaskCompletion",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("taskId", wire::Schema::Str16),
                ("taskType", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("outcome", wire::Schema::Dyn),
            ])),
        ),
        (
            "backgroundTask",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("taskId", wire::Schema::Str16),
                ("taskType", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("command", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("status", wire::Schema::Dyn),
                ("startedAt", wire::Schema::Str16),
                ("completedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
                (
                    "completionActivityId",
                    wire::Schema::OptJs(&wire::Schema::Str16),
                ),
                ("exitCode", wire::Schema::OptJs(&wire::Schema::Num)),
            ])),
        ),
        (
            "subagentProgress",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("toolUseId", wire::Schema::Str16),
                ("invocationId", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("title", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("outcome", wire::Schema::OptJs(&wire::Schema::Str16)),
            ])),
        ),
        (
            "computerControlDenied",
            wire::Schema::OptJs(&wire::Schema::Obj(&[(
                "toolName",
                wire::Schema::OptJs(&wire::Schema::Str16),
            )])),
        ),
        (
            "computerSetupRequired",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("missing", wire::Schema::List(&wire::Schema::Dyn)),
                ("buildSignature", wire::Schema::Dyn),
                ("bundleId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ])),
        ),
        (
            "providerContextLifecycle",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("provider", wire::Schema::Dyn),
                ("nativeHistory", wire::Schema::Str16),
                ("restartReason", wire::Schema::Dyn),
                ("sessionRestarted", wire::Schema::Bool),
                ("recapInjected", wire::Schema::Bool),
                ("recapCharacters", wire::Schema::Num),
                ("recapPreview", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("recapPreviewTruncated", wire::Schema::Bool),
            ])),
        ),
        (
            "providerHandoff",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("status", wire::Schema::Str16),
                ("sourceProvider", wire::Schema::Dyn),
                ("sourceModel", wire::Schema::Str16),
                ("targetProvider", wire::Schema::Dyn),
                ("targetModel", wire::Schema::Str16),
                ("sourceModelSelection", wire::Schema::Dyn),
                ("targetModelSelection", wire::Schema::Dyn),
                ("contextText", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("failureDetail", wire::Schema::OptJs(&wire::Schema::Str16)),
                (
                    "blockedByBackgroundTasks",
                    wire::Schema::OptJs(&wire::Schema::Bool),
                ),
                ("sourceFastModeNotice", wire::Schema::Dyn),
                ("targetFastModeNotice", wire::Schema::Dyn),
            ])),
        ),
        (
            "turnFailure",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("cause", wire::Schema::Str16),
                ("message", wire::Schema::Str16),
                ("errorCode", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("provider", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("usageLimit", wire::Schema::OptJs(&wire::Schema::Bool)),
            ])),
        ),
        (
            "userInputExchange",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
        ),
        ("inlineVisual", wire::Schema::Dyn),
        ("activityKind", wire::Schema::Dyn),
        ("codexReasoning", wire::Schema::OptJs(&wire::Schema::Bool)),
        ("nativeEventType", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `omitRoutedSubagentWorkEntries` over the binary wire.
#[napi(js_name = "omitRoutedSubagentWorkEntriesWire")]
pub fn omit_routed_subagent_work_entries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let entries: Value = wire::read_value(r, &S_OMIT_ROUTED_SUBAGENT_WORK_ENTRIES_0)?;
        let out = &port::omit_routed_subagent_work_entries(entries)?;
        wire::write_value(w, out, &S_OMIT_ROUTED_SUBAGENT_WORK_ENTRIES_OUT)?;
        Ok(())
    })
}

static S_IS_MISSING_CHECKPOINT_BASELINE_ACTIVITY_0: wire::Schema = wire::Schema::Dyn;

/// `isMissingCheckpointBaselineActivity` over the binary wire.
#[napi(js_name = "isMissingCheckpointBaselineActivityWire")]
pub fn is_missing_checkpoint_baseline_activity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activity: Value = wire::read_value(r, &S_IS_MISSING_CHECKPOINT_BASELINE_ACTIVITY_0)?;
        let out = &port::is_missing_checkpoint_baseline_activity(activity)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_CODEX_RUNTIME_WARNING_TYPE_0: wire::Schema = wire::Schema::Dyn;

/// `isCodexRuntimeWarningType` over the binary wire.
#[napi(js_name = "isCodexRuntimeWarningTypeWire")]
pub fn is_codex_runtime_warning_type_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let native_event_type: Value = wire::read_value(r, &S_IS_CODEX_RUNTIME_WARNING_TYPE_0)?;
        let out = &port::is_codex_runtime_warning_type(native_event_type)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PARSE_TASK_LIST_TASKS_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_TASK_LIST_TASKS_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
        ("task", wire::Schema::Str16),
        ("status", wire::Schema::Str16),
    ])));

/// `parseTaskListTasks` over the binary wire.
#[napi(js_name = "parseTaskListTasksWire")]
pub fn parse_task_list_tasks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: Value = wire::read_value(r, &S_PARSE_TASK_LIST_TASKS_0)?;
        let out = &port::parse_task_list_tasks(payload)?;
        wire::write_value(w, out, &S_PARSE_TASK_LIST_TASKS_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_PROVIDER_HANDOFF_INFO_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_PROVIDER_HANDOFF_INFO_1: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_PROVIDER_HANDOFF_INFO_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("status", wire::Schema::Str16),
        ("sourceProvider", wire::Schema::Dyn),
        ("sourceModel", wire::Schema::Str16),
        ("targetProvider", wire::Schema::Dyn),
        ("targetModel", wire::Schema::Str16),
        ("sourceModelSelection", wire::Schema::Dyn),
        ("targetModelSelection", wire::Schema::Dyn),
        ("contextText", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("failureDetail", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "blockedByBackgroundTasks",
            wire::Schema::OptJs(&wire::Schema::Bool),
        ),
        (
            "sourceFastModeNotice",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("kind", wire::Schema::Str16),
                ("label", wire::Schema::Str16),
                ("detail", wire::Schema::Str16),
            ])),
        ),
        (
            "targetFastModeNotice",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("kind", wire::Schema::Str16),
                ("label", wire::Schema::Str16),
                ("detail", wire::Schema::Str16),
            ])),
        ),
    ]));

/// `extractProviderHandoffInfo` over the binary wire.
#[napi(js_name = "extractProviderHandoffInfoWire")]
pub fn extract_provider_handoff_info_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: Value = wire::read_value(r, &S_EXTRACT_PROVIDER_HANDOFF_INFO_0)?;
        let status: Value = wire::read_value(r, &S_EXTRACT_PROVIDER_HANDOFF_INFO_1)?;
        let out = &port::extract_provider_handoff_info(payload, status)?;
        wire::write_value(w, out, &S_EXTRACT_PROVIDER_HANDOFF_INFO_OUT)?;
        Ok(())
    })
}

static S_PARSE_USER_INPUT_QUESTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_USER_INPUT_QUESTIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseUserInputQuestions` over the binary wire.
#[napi(js_name = "parseUserInputQuestionsWire")]
pub fn parse_user_input_questions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: Value = wire::read_value(r, &S_PARSE_USER_INPUT_QUESTIONS_0)?;
        let out = &port::parse_user_input_questions(payload)?;
        wire::write_value(w, out, &S_PARSE_USER_INPUT_QUESTIONS_OUT)?;
        Ok(())
    })
}

static S_DERIVE_SUBAGENT_TASK_ENDS_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_SUBAGENT_TASK_ENDS_OUT: wire::Schema = wire::Schema::Dyn;

/// `deriveSubagentTaskEnds` over the binary wire.
#[napi(js_name = "deriveSubagentTaskEndsWire")]
pub fn derive_subagent_task_ends_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: Value = wire::read_value(r, &S_DERIVE_SUBAGENT_TASK_ENDS_0)?;
        let out = &port::derive_subagent_task_ends(activities)?;
        wire::write_value(w, out, &S_DERIVE_SUBAGENT_TASK_ENDS_OUT)?;
        Ok(())
    })
}

static S_DERIVE_THREAD_BACKGROUND_TASKS_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_THREAD_BACKGROUND_TASKS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("taskId", wire::Schema::Str16),
        ("taskType", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("command", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("status", wire::Schema::Dyn),
        ("startedAt", wire::Schema::Str16),
        ("completedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "completionActivityId",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("exitCode", wire::Schema::OptJs(&wire::Schema::Num)),
    ]));

/// `deriveThreadBackgroundTasks` over the binary wire.
#[napi(js_name = "deriveThreadBackgroundTasksWire")]
pub fn derive_thread_background_tasks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: Value = wire::read_value(r, &S_DERIVE_THREAD_BACKGROUND_TASKS_0)?;
        let out = &port::derive_thread_background_tasks(activities)?;
        wire::write_value(w, out, &S_DERIVE_THREAD_BACKGROUND_TASKS_OUT)?;
        Ok(())
    })
}

static S_DERIVE_WORK_LOG_ENTRIES_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_WORK_LOG_ENTRIES_1: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_WORK_LOG_ENTRIES_2: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_WORK_LOG_ENTRIES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("createdAt", wire::Schema::Str16),
    ("sequence", wire::Schema::OptJs(&wire::Schema::Num)),
    ("turnId", wire::Schema::Dyn),
    ("label", wire::Schema::Str16),
    ("detail", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("command", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("rawCommand", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("preview", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "changedFiles",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16)),
    ),
    ("tone", wire::Schema::Str16),
    ("toolTitle", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "displayToolTitle",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("toolName", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("mcpServer", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("toolCallId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("toolStatus", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "liveActivity",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("state", wire::Schema::Str16),
            ("label", wire::Schema::Str16),
            ("startedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("lastActivityAt", wire::Schema::Str16),
            ("detail", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("progress", wire::Schema::OptJs(&wire::Schema::Num)),
            ("elapsedSeconds", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
    (
        "toolDetails",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("kind", wire::Schema::Str16),
            ("title", wire::Schema::Str16),
            ("command", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "output",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("output", wire::Schema::Dyn),
                    ("stdout", wire::Schema::Dyn),
                    ("stderr", wire::Schema::Dyn),
                    ("exitCode", wire::Schema::Dyn),
                    ("truncated", wire::Schema::Dyn),
                ])),
            ),
            ("diff", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("content", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "edits",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            (
                "files",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
        ])),
    ),
    ("itemType", wire::Schema::Dyn),
    ("requestKind", wire::Schema::Dyn),
    (
        "grantedCommandApproval",
        wire::Schema::OptJs(&wire::Schema::Bool),
    ),
    (
        "subagents",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
            ("threadId", wire::Schema::Dyn),
            ("providerThreadId", wire::Schema::Dyn),
            ("resolvedThreadId", wire::Schema::Dyn),
            ("agentId", wire::Schema::Dyn),
            ("nickname", wire::Schema::Dyn),
            ("role", wire::Schema::Dyn),
            ("model", wire::Schema::Dyn),
            ("effort", wire::Schema::Dyn),
            ("background", wire::Schema::Dyn),
            ("prompt", wire::Schema::Dyn),
            ("rawStatus", wire::Schema::Dyn),
            ("latestUpdate", wire::Schema::Dyn),
            ("title", wire::Schema::Dyn),
            ("statusLabel", wire::Schema::Dyn),
            ("isActive", wire::Schema::Dyn),
        ]))),
    ),
    (
        "subagentAction",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("tool", wire::Schema::Str16),
            ("status", wire::Schema::Str16),
            ("summaryText", wire::Schema::Str16),
            ("model", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("prompt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "subagentRun",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "entryIds",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
            ("members", wire::Schema::List(&wire::Schema::Dyn)),
        ])),
    ),
    (
        "automation",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("cadenceLabel", wire::Schema::Str16),
            ("proposalState", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "connectorProposalId",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "synaraThreadCreation",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("operationId", wire::Schema::Str16),
            ("requestedCount", wire::Schema::Num),
            ("createdCount", wire::Schema::Num),
            ("threads", wire::Schema::List(&wire::Schema::Dyn)),
            ("environmentId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("hostName", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "synaraWorkerNotice",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("kind", wire::Schema::Str16),
            ("marker", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("phrase", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("threads", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "actions",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
        ])),
    ),
    (
        "monitorNotification",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("taskId", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("output", wire::Schema::Str16),
            ("outcome", wire::Schema::Str16),
            ("midTurn", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "backgroundTaskCompletion",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("taskId", wire::Schema::Str16),
            ("taskType", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("outcome", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "backgroundTask",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("taskId", wire::Schema::Str16),
            ("taskType", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("command", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("status", wire::Schema::Dyn),
            ("startedAt", wire::Schema::Str16),
            ("completedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "completionActivityId",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("exitCode", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
    (
        "subagentProgress",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("toolUseId", wire::Schema::Str16),
            ("invocationId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("title", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("outcome", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "computerControlDenied",
        wire::Schema::OptJs(&wire::Schema::Obj(&[(
            "toolName",
            wire::Schema::OptJs(&wire::Schema::Str16),
        )])),
    ),
    (
        "computerSetupRequired",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("missing", wire::Schema::List(&wire::Schema::Dyn)),
            ("buildSignature", wire::Schema::Dyn),
            ("bundleId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "providerContextLifecycle",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("provider", wire::Schema::Dyn),
            ("nativeHistory", wire::Schema::Str16),
            ("restartReason", wire::Schema::Str16),
            ("sessionRestarted", wire::Schema::Bool),
            ("recapInjected", wire::Schema::Bool),
            ("recapCharacters", wire::Schema::Num),
            ("recapPreview", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("recapPreviewTruncated", wire::Schema::Bool),
        ])),
    ),
    (
        "providerHandoff",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("status", wire::Schema::Str16),
            ("sourceProvider", wire::Schema::Dyn),
            ("sourceModel", wire::Schema::Str16),
            ("targetProvider", wire::Schema::Dyn),
            ("targetModel", wire::Schema::Str16),
            ("sourceModelSelection", wire::Schema::Dyn),
            ("targetModelSelection", wire::Schema::Dyn),
            ("contextText", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("failureDetail", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "blockedByBackgroundTasks",
                wire::Schema::OptJs(&wire::Schema::Bool),
            ),
            ("sourceFastModeNotice", wire::Schema::Dyn),
            ("targetFastModeNotice", wire::Schema::Dyn),
        ])),
    ),
    (
        "turnFailure",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("cause", wire::Schema::Str16),
            ("message", wire::Schema::Str16),
            ("errorCode", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("provider", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("usageLimit", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "userInputExchange",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("header", wire::Schema::Dyn),
            ("question", wire::Schema::Dyn),
            ("options", wire::Schema::Dyn),
            ("answer", wire::Schema::Dyn),
        ]))),
    ),
    ("inlineVisual", wire::Schema::Dyn),
    ("activityKind", wire::Schema::Dyn),
    ("codexReasoning", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("nativeEventType", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `deriveWorkLogEntries` over the binary wire.
#[napi(js_name = "deriveWorkLogEntriesWire")]
pub fn derive_work_log_entries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: Value = wire::read_value(r, &S_DERIVE_WORK_LOG_ENTRIES_0)?;
        let latest_turn_id: Value = wire::read_value(r, &S_DERIVE_WORK_LOG_ENTRIES_1)?;
        let options: Value = wire::read_value(r, &S_DERIVE_WORK_LOG_ENTRIES_2)?;
        let out = &port::derive_work_log_entries(activities, latest_turn_id, options)?;
        wire::write_value(w, out, &S_DERIVE_WORK_LOG_ENTRIES_OUT)?;
        Ok(())
    })
}

static S_DERIVE_TIMELINE_ENTRIES_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_TIMELINE_ENTRIES_1: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_TIMELINE_ENTRIES_2: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_TIMELINE_ENTRIES_3: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_TIMELINE_ENTRIES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `deriveTimelineEntries` over the binary wire.
#[napi(js_name = "deriveTimelineEntriesWire")]
pub fn derive_timeline_entries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let messages: Value = wire::read_value(r, &S_DERIVE_TIMELINE_ENTRIES_0)?;
        let proposed_plans: Value = wire::read_value(r, &S_DERIVE_TIMELINE_ENTRIES_1)?;
        let work_entries: Value = wire::read_value(r, &S_DERIVE_TIMELINE_ENTRIES_2)?;
        let options: Value = wire::read_value(r, &S_DERIVE_TIMELINE_ENTRIES_3)?;
        let out = &port::derive_timeline_entries(messages, proposed_plans, work_entries, options)?;
        wire::write_value(w, out, &S_DERIVE_TIMELINE_ENTRIES_OUT)?;
        Ok(())
    })
}
