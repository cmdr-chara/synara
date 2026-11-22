//! JS bindings for `apps/server/src/orchestration/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `threadModelSelectionChangedDetail`.
#[napi(js_name = "threadModelSelectionChangedDetail")]
pub fn thread_model_selection_changed_detail(thread_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::thread_model_selection_changed_detail(thread_id)?;
        Ok(out)
    })
}

/// `toProjectorDecodeError`.
#[napi(js_name = "toProjectorDecodeError")]
pub fn to_projector_decode_error(event_type: String) -> napi::Result<String> {
    run(move || {
        let out = port::to_projector_decode_error(event_type)?;
        crate::js::to_json(&out)
    })
}

/// `fingerprintOrchestrationCommand`.
#[napi(js_name = "fingerprintOrchestrationCommand")]
pub fn fingerprint_orchestration_command(command: String) -> napi::Result<String> {
    run(move || {
        let command = crate::js::from_json(&command)?;
        let out = port::fingerprint_orchestration_command(command)?;
        crate::js::to_json(&out)
    })
}

/// `threadHasInFlightTurn`.
#[napi(js_name = "threadHasInFlightTurn")]
pub fn thread_has_in_flight_turn(thread: String) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::thread_has_in_flight_turn(thread)?;
        Ok(out)
    })
}

/// `checkpointRevertActiveTurnDetail`.
#[napi(js_name = "checkpointRevertActiveTurnDetail")]
pub fn checkpoint_revert_active_turn_detail(thread_id: String) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::checkpoint_revert_active_turn_detail(thread_id)?;
        Ok(out)
    })
}

/// `threadHasCheckpointRevertInProgress`.
#[napi(js_name = "threadHasCheckpointRevertInProgress")]
pub fn thread_has_checkpoint_revert_in_progress(thread: String) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::thread_has_checkpoint_revert_in_progress(thread)?;
        Ok(out)
    })
}

/// `checkpointRevertInProgressDetail`.
#[napi(js_name = "checkpointRevertInProgressDetail")]
pub fn checkpoint_revert_in_progress_detail(thread_id: String) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::checkpoint_revert_in_progress_detail(thread_id)?;
        Ok(out)
    })
}

/// `checkpointRevertDeleteInProgressDetail`.
#[napi(js_name = "checkpointRevertDeleteInProgressDetail")]
pub fn checkpoint_revert_delete_in_progress_detail(thread_id: String) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::checkpoint_revert_delete_in_progress_detail(thread_id)?;
        Ok(out)
    })
}

/// `findThreadById`.
#[napi(js_name = "findThreadById")]
pub fn find_thread_by_id(read_model: String, thread_id: String) -> napi::Result<String> {
    run(move || {
        let read_model = crate::js::from_json(&read_model)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::find_thread_by_id(read_model, thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `findProjectById`.
#[napi(js_name = "findProjectById")]
pub fn find_project_by_id(read_model: String, project_id: String) -> napi::Result<String> {
    run(move || {
        let read_model = crate::js::from_json(&read_model)?;
        let project_id = crate::js::from_json(&project_id)?;
        let out = port::find_project_by_id(read_model, project_id)?;
        crate::js::to_json(&out)
    })
}

/// `findSpaceById`.
#[napi(js_name = "findSpaceById")]
pub fn find_space_by_id(read_model: String, space_id: String) -> napi::Result<String> {
    run(move || {
        let read_model = crate::js::from_json(&read_model)?;
        let space_id = crate::js::from_json(&space_id)?;
        let out = port::find_space_by_id(read_model, space_id)?;
        crate::js::to_json(&out)
    })
}

/// `listActiveSpaces`.
#[napi(js_name = "listActiveSpaces")]
pub fn list_active_spaces(read_model: String) -> napi::Result<String> {
    run(move || {
        let read_model = crate::js::from_json(&read_model)?;
        let out = port::list_active_spaces(read_model)?;
        crate::js::to_json(&out)
    })
}

/// `requireSpace`.
#[napi(js_name = "requireSpace")]
pub fn require_space(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_space(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireSpaceAbsent`.
#[napi(js_name = "requireSpaceAbsent")]
pub fn require_space_absent(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_space_absent(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireSpaceNameAvailable`.
#[napi(js_name = "requireSpaceNameAvailable")]
pub fn require_space_name_available(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_space_name_available(input)?;
        crate::js::to_json(&out)
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

/// `isOrdinaryProjectRow`.
#[napi(js_name = "isOrdinaryProjectRow")]
pub fn is_ordinary_project_row(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::is_ordinary_project_row(input)?;
        Ok(out)
    })
}

/// `requireSpaceAssignableProject`.
#[napi(js_name = "requireSpaceAssignableProject")]
pub fn require_space_assignable_project(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_space_assignable_project(input)?;
        crate::js::to_json(&out)
    })
}

/// `listActiveProjectsByWorkspaceRoot`.
#[napi(js_name = "listActiveProjectsByWorkspaceRoot")]
pub fn list_active_projects_by_workspace_root(
    read_model: String,
    workspace_root: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let read_model = crate::js::from_json(&read_model)?;
        let options = crate::js::from_json(&options)?;
        let out =
            port::list_active_projects_by_workspace_root(read_model, workspace_root, options)?;
        crate::js::to_json(&out)
    })
}

/// `listThreadsByProjectId`.
#[napi(js_name = "listThreadsByProjectId")]
pub fn list_threads_by_project_id(read_model: String, project_id: String) -> napi::Result<String> {
    run(move || {
        let read_model = crate::js::from_json(&read_model)?;
        let project_id = crate::js::from_json(&project_id)?;
        let out = port::list_threads_by_project_id(read_model, project_id)?;
        crate::js::to_json(&out)
    })
}

/// `requireProject`.
#[napi(js_name = "requireProject")]
pub fn require_project(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_project(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireProjectAbsent`.
#[napi(js_name = "requireProjectAbsent")]
pub fn require_project_absent(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_project_absent(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireValidProjectFolders`.
#[napi(js_name = "requireValidProjectFolders")]
pub fn require_valid_project_folders(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_valid_project_folders(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireProjectWorkspaceRootAvailable`.
#[napi(js_name = "requireProjectWorkspaceRootAvailable")]
pub fn require_project_workspace_root_available(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_project_workspace_root_available(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireProjectHasNoThreads`.
#[napi(js_name = "requireProjectHasNoThreads")]
pub fn require_project_has_no_threads(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_project_has_no_threads(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireThread`.
#[napi(js_name = "requireThread")]
pub fn require_thread(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_thread(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireApprovalNotResponded`.
#[napi(js_name = "requireApprovalNotResponded")]
pub fn require_approval_not_responded(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_approval_not_responded(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireThreadAbsent`.
#[napi(js_name = "requireThreadAbsent")]
pub fn require_thread_absent(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_thread_absent(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireThreadArchived`.
#[napi(js_name = "requireThreadArchived")]
pub fn require_thread_archived(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_thread_archived(input)?;
        crate::js::to_json(&out)
    })
}

/// `requireThreadNotArchived`.
#[napi(js_name = "requireThreadNotArchived")]
pub fn require_thread_not_archived(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::require_thread_not_archived(input)?;
        crate::js::to_json(&out)
    })
}

/// `threadResumePreconditionViolation`.
#[napi(js_name = "threadResumePreconditionViolation")]
pub fn thread_resume_precondition_violation(
    thread: String,
    precondition: String,
) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let precondition = crate::js::from_json(&precondition)?;
        let out = port::thread_resume_precondition_violation(thread, precondition)?;
        crate::js::to_json(&out)
    })
}

/// `threadResumePreconditionDetail`.
#[napi(js_name = "threadResumePreconditionDetail")]
pub fn thread_resume_precondition_detail(
    thread_id: String,
    violation: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::thread_resume_precondition_detail(thread_id, violation)?;
        crate::js::to_json(&out)
    })
}

/// `withoutRemoteResumeComputerGrant`.
#[napi(js_name = "withoutRemoteResumeComputerGrant")]
pub fn without_remote_resume_computer_grant(
    command: String,
    principal_owner_id: String,
) -> napi::Result<String> {
    run(move || {
        let command = crate::js::from_json(&command)?;
        let out = port::without_remote_resume_computer_grant(command, principal_owner_id)?;
        crate::js::to_json(&out)
    })
}

/// `resolveForkSourceCutoff`.
#[napi(js_name = "resolveForkSourceCutoff")]
pub fn resolve_fork_source_cutoff(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_fork_source_cutoff(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildForkThreadTitle`.
#[napi(js_name = "buildForkThreadTitle")]
pub fn build_fork_thread_title(source: String, project_threads: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let project_threads = crate::js::from_json(&project_threads)?;
        let out = port::build_fork_thread_title(source, project_threads)?;
        Ok(out)
    })
}

/// `resolveStableMessageTurnId`.
#[napi(js_name = "resolveStableMessageTurnId")]
pub fn resolve_stable_message_turn_id(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_stable_message_turn_id(input)?;
        crate::js::to_json(&out)
    })
}

/// `usesReservedCommandAdmission`.
#[napi(js_name = "usesReservedCommandAdmission")]
pub fn uses_reserved_command_admission(r#type: String) -> napi::Result<bool> {
    run(move || {
        let r#type = crate::js::from_json(&r#type)?;
        let out = port::uses_reserved_command_admission(r#type)?;
        Ok(out)
    })
}

/// `isQuiescingCommandAdmissible`.
#[napi(js_name = "isQuiescingCommandAdmissible")]
pub fn is_quiescing_command_admissible(r#type: String) -> napi::Result<bool> {
    run(move || {
        let r#type = crate::js::from_json(&r#type)?;
        let out = port::is_quiescing_command_admissible(r#type)?;
        Ok(out)
    })
}

/// `orchestrationCommandLane`.
#[napi(js_name = "orchestrationCommandLane")]
pub fn orchestration_command_lane(r#type: String) -> napi::Result<String> {
    run(move || {
        let r#type = crate::js::from_json(&r#type)?;
        let out = port::orchestration_command_lane(r#type)?;
        Ok(out)
    })
}

/// `isExpiredSidechat`.
#[napi(js_name = "isExpiredSidechat")]
pub fn is_expired_sidechat(thread: String) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::is_expired_sidechat(thread)?;
        Ok(out)
    })
}

/// `latestSidechatActivityAt`.
#[napi(js_name = "latestSidechatActivityAt")]
pub fn latest_sidechat_activity_at(
    current: Option<String>,
    candidate: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::latest_sidechat_activity_at(current, candidate)?;
        Ok(out)
    })
}

/// `sidechatActivityInstantsEqual`.
#[napi(js_name = "sidechatActivityInstantsEqual")]
pub fn sidechat_activity_instants_equal(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let out = port::sidechat_activity_instants_equal(left, right)?;
        Ok(out)
    })
}

/// `settleTurnStateFromSession`.
#[napi(js_name = "settleTurnStateFromSession")]
pub fn settle_turn_state_from_session(
    session: String,
    existing_state: String,
) -> napi::Result<String> {
    run(move || {
        let session = crate::js::from_json(&session)?;
        let out = port::settle_turn_state_from_session(session, existing_state)?;
        crate::js::to_json(&out)
    })
}

/// `maxIso`.
#[napi(js_name = "maxIso")]
pub fn max_iso(left: Option<String>, right: String) -> napi::Result<String> {
    run(move || {
        let out = port::max_iso(left, right)?;
        crate::js::to_json(&out)
    })
}

/// `deriveTurnStartModelSelection`.
#[napi(js_name = "deriveTurnStartModelSelection")]
pub fn derive_turn_start_model_selection(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_turn_start_model_selection(input)?;
        crate::js::to_json(&out)
    })
}

/// `countNativeTurnStartMessages`.
#[napi(js_name = "countNativeTurnStartMessages")]
pub fn count_native_turn_start_messages(messages: String) -> napi::Result<f64> {
    run(move || {
        let messages = crate::js::from_json(&messages)?;
        let out = port::count_native_turn_start_messages(messages)?;
        Ok(out)
    })
}

/// `canAdoptFirstTurnProvider`.
#[napi(js_name = "canAdoptFirstTurnProvider")]
pub fn can_adopt_first_turn_provider(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::can_adopt_first_turn_provider(input)?;
        Ok(out)
    })
}

/// `deriveTurnStartSession`.
#[napi(js_name = "deriveTurnStartSession")]
pub fn derive_turn_start_session(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_turn_start_session(input)?;
        crate::js::to_json(&out)
    })
}

/// `relocateProjectPath`.
#[napi(js_name = "relocateProjectPath")]
pub fn relocate_project_path(
    value: Option<String>,
    previous_root: String,
    next_root: String,
    platform: String,
) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let out = port::relocate_project_path(value, previous_root, next_root, platform)?;
        crate::js::to_json(&out)
    })
}

/// `providerWorkspaceChanged`.
#[napi(js_name = "providerWorkspaceChanged")]
pub fn provider_workspace_changed(
    current_cwd: Option<String>,
    requested_cwd: Option<String>,
    platform: String,
) -> napi::Result<bool> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let out = port::provider_workspace_changed(current_cwd, requested_cwd, platform)?;
        Ok(out)
    })
}

/// `isProviderIntentEventType`.
#[napi(js_name = "isProviderIntentEventType")]
pub fn is_provider_intent_event_type(event_type: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_provider_intent_event_type(event_type)?;
        Ok(out)
    })
}

/// `isProviderIntentEvent`.
#[napi(js_name = "isProviderIntentEvent")]
pub fn is_provider_intent_event(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::is_provider_intent_event(event)?;
        Ok(out)
    })
}

/// `isReplaySafeClaimedProviderIntent`.
#[napi(js_name = "isReplaySafeClaimedProviderIntent")]
pub fn is_replay_safe_claimed_provider_intent(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::is_replay_safe_claimed_provider_intent(event)?;
        Ok(out)
    })
}

/// `isProviderSideEffectIntent`.
#[napi(js_name = "isProviderSideEffectIntent")]
pub fn is_provider_side_effect_intent(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::is_provider_side_effect_intent(event)?;
        Ok(out)
    })
}

/// `isClaimedProviderIntent`.
#[napi(js_name = "isClaimedProviderIntent")]
pub fn is_claimed_provider_intent(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::is_claimed_provider_intent(event)?;
        Ok(out)
    })
}

/// `isQuarantineExemptProviderIntent`.
#[napi(js_name = "isQuarantineExemptProviderIntent")]
pub fn is_quarantine_exempt_provider_intent(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::is_quarantine_exempt_provider_intent(event)?;
        Ok(out)
    })
}

/// `isUnsettledPendingInteraction`.
#[napi(js_name = "isUnsettledPendingInteraction")]
pub fn is_unsettled_pending_interaction(row: String) -> napi::Result<bool> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::is_unsettled_pending_interaction(row)?;
        Ok(out)
    })
}

/// `pendingInteractionRequestKind`.
#[napi(js_name = "pendingInteractionRequestKind")]
pub fn pending_interaction_request_kind(interaction_kind: String) -> napi::Result<String> {
    run(move || {
        let interaction_kind = crate::js::from_json(&interaction_kind)?;
        let out = port::pending_interaction_request_kind(interaction_kind)?;
        Ok(out)
    })
}

/// `buildStalePendingRequestSettlementCommand`.
#[napi(js_name = "buildStalePendingRequestSettlementCommand")]
pub fn build_stale_pending_request_settlement_command(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_stale_pending_request_settlement_command(input)?;
        crate::js::to_json(&out)
    })
}

/// `nativeImportId`.
#[napi(js_name = "nativeImportId")]
pub fn native_import_id(provider: String, cursor: String) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let cursor = crate::js::from_json(&cursor)?;
        let out = port::native_import_id(provider, cursor)?;
        crate::js::to_json(&out)
    })
}

/// `makeProjectImportDestinations`.
#[napi(js_name = "makeProjectImportDestinations")]
pub fn make_project_import_destinations(model: String) -> napi::Result<String> {
    run(move || {
        let model = crate::js::from_json(&model)?;
        let out = port::make_project_import_destinations(model)?;
        crate::js::to_json(&out)
    })
}

/// `threadArchiveFileName`.
#[napi(js_name = "threadArchiveFileName")]
pub fn thread_archive_file_name(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::thread_archive_file_name(input)?;
        Ok(out)
    })
}

/// `listImportedHandoffMessages`.
#[napi(js_name = "listImportedHandoffMessages")]
pub fn list_imported_handoff_messages(thread: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::list_imported_handoff_messages(thread)?;
        crate::js::to_json(&out)
    })
}

/// `listImportedForkMessages`.
#[napi(js_name = "listImportedForkMessages")]
pub fn list_imported_fork_messages(thread: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::list_imported_fork_messages(thread)?;
        crate::js::to_json(&out)
    })
}

/// `hasNativeHandoffMessages`.
#[napi(js_name = "hasNativeHandoffMessages")]
pub fn has_native_handoff_messages(thread: String) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::has_native_handoff_messages(thread)?;
        Ok(out)
    })
}

/// `hasNativeAssistantMessagesBefore`.
#[napi(js_name = "hasNativeAssistantMessagesBefore")]
pub fn has_native_assistant_messages_before(
    thread: String,
    current_message_id: Option<String>,
) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::has_native_assistant_messages_before(thread, current_message_id)?;
        Ok(out)
    })
}

/// `listPriorTranscriptMessages`.
#[napi(js_name = "listPriorTranscriptMessages")]
pub fn list_prior_transcript_messages(
    thread: String,
    current_message_id: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::list_prior_transcript_messages(thread, current_message_id)?;
        crate::js::to_json(&out)
    })
}

/// `buildHandoffBootstrapText`.
#[napi(js_name = "buildHandoffBootstrapText")]
pub fn build_handoff_bootstrap_text(thread: String, max_chars: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let max_chars = crate::js::from_json(&max_chars)?;
        let out = port::build_handoff_bootstrap_text(thread, max_chars)?;
        crate::js::to_json(&out)
    })
}

/// `buildPriorTranscriptBootstrapText`.
#[napi(js_name = "buildPriorTranscriptBootstrapText")]
pub fn build_prior_transcript_bootstrap_text(
    thread: String,
    current_message_id: Option<String>,
    max_chars: String,
) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let max_chars = crate::js::from_json(&max_chars)?;
        let out =
            port::build_prior_transcript_bootstrap_text(thread, current_message_id, max_chars)?;
        crate::js::to_json(&out)
    })
}

/// `buildPriorTranscriptBootstrap`.
#[napi(js_name = "buildPriorTranscriptBootstrap")]
pub fn build_prior_transcript_bootstrap(
    thread: String,
    current_message_id: Option<String>,
    max_chars: String,
) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let max_chars = crate::js::from_json(&max_chars)?;
        let out = port::build_prior_transcript_bootstrap(thread, current_message_id, max_chars)?;
        crate::js::to_json(&out)
    })
}

/// `buildForkBootstrapText`.
#[napi(js_name = "buildForkBootstrapText")]
pub fn build_fork_bootstrap_text(thread: String, max_chars: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let max_chars = crate::js::from_json(&max_chars)?;
        let out = port::build_fork_bootstrap_text(thread, max_chars)?;
        crate::js::to_json(&out)
    })
}

/// `claudeHistoricalSessionChildEnvironment`.
#[napi(js_name = "claudeHistoricalSessionChildEnvironment")]
pub fn claude_historical_session_child_environment(environment: String) -> napi::Result<String> {
    run(move || {
        let environment = crate::js::from_json(&environment)?;
        let out = port::claude_historical_session_child_environment(environment)?;
        crate::js::to_json(&out)
    })
}

/// `readClaudeSessionMessagePageInEnvironment`.
#[napi(js_name = "readClaudeSessionMessagePageInEnvironment")]
pub async fn read_claude_session_message_page_in_environment(
    input: String,
) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::read_claude_session_message_page_in_environment(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `listClaudeSessionsInEnvironment`.
#[napi(js_name = "listClaudeSessionsInEnvironment")]
pub async fn list_claude_sessions_in_environment(environment: String) -> napi::Result<String> {
    run_async(async move {
        let environment = crate::js::from_json(&environment)?;
        let out = port::list_claude_sessions_in_environment(environment).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `readClaudeSessionMessagesInEnvironment`.
#[napi(js_name = "readClaudeSessionMessagesInEnvironment")]
pub async fn read_claude_session_messages_in_environment(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::read_claude_session_messages_in_environment(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `claudeHistoricalSessionEnvironment`.
#[napi(js_name = "claudeHistoricalSessionEnvironment")]
pub fn claude_historical_session_environment(
    provider_options: String,
    input: String,
) -> napi::Result<String> {
    run(move || {
        let provider_options = crate::js::from_json(&provider_options)?;
        let input = crate::js::from_json(&input)?;
        let out = port::claude_historical_session_environment(provider_options, input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveImportedThreadProviderOptionsForSettings`.
#[napi(js_name = "resolveImportedThreadProviderOptionsForSettings")]
pub fn resolve_imported_thread_provider_options_for_settings(
    settings: String,
    model_selection: String,
) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let model_selection = crate::js::from_json(&model_selection)?;
        let out =
            port::resolve_imported_thread_provider_options_for_settings(settings, model_selection)?;
        crate::js::to_json(&out)
    })
}

/// `mapCodexSnapshotMessages`.
#[napi(js_name = "mapCodexSnapshotMessages")]
pub fn map_codex_snapshot_messages(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::map_codex_snapshot_messages(input)?;
        crate::js::to_json(&out)
    })
}

/// `mapClaudeSessionMessages`.
#[napi(js_name = "mapClaudeSessionMessages")]
pub fn map_claude_session_messages(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::map_claude_session_messages(input)?;
        crate::js::to_json(&out)
    })
}

/// `mapOpenCodeSnapshotMessages`.
#[napi(js_name = "mapOpenCodeSnapshotMessages")]
pub fn map_open_code_snapshot_messages(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::map_open_code_snapshot_messages(input)?;
        crate::js::to_json(&out)
    })
}

/// `mapFactorySnapshotMessages`.
#[napi(js_name = "mapFactorySnapshotMessages")]
pub fn map_factory_snapshot_messages(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::map_factory_snapshot_messages(input)?;
        crate::js::to_json(&out)
    })
}

/// `mapOmpSnapshotMessages`.
#[napi(js_name = "mapOmpSnapshotMessages")]
pub fn map_omp_snapshot_messages(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::map_omp_snapshot_messages(input)?;
        crate::js::to_json(&out)
    })
}

/// `projectImportKey`.
#[napi(js_name = "projectImportKey")]
pub fn project_import_key(argc: String, parts: String) -> napi::Result<String> {
    run(move || {
        let argc = crate::js::from_json(&argc)?;
        let parts = crate::js::from_json(&parts)?;
        let out = port::project_import_key(argc, parts)?;
        Ok(out)
    })
}

/// `normalizeWindowsImportPath`.
#[napi(js_name = "normalizeWindowsImportPath")]
pub fn normalize_windows_import_path(value: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_windows_import_path(value)?;
        Ok(out)
    })
}

/// `importPathIdentity`.
#[napi(js_name = "importPathIdentity")]
pub fn import_path_identity(value: String) -> napi::Result<String> {
    run(move || {
        let out = port::import_path_identity(value)?;
        Ok(out)
    })
}

/// `resolveProjectImportSources`.
#[napi(js_name = "resolveProjectImportSources")]
pub fn resolve_project_import_sources(
    settings: String,
    _providers: String,
    paths: String,
) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let _providers = crate::js::from_json(&_providers)?;
        let paths = crate::js::from_json(&paths)?;
        let out = port::resolve_project_import_sources(settings, _providers, paths)?;
        crate::js::to_json(&out)
    })
}

/// `buildProjectImportCatalog`.
#[napi(js_name = "buildProjectImportCatalog")]
pub async fn build_project_import_catalog(
    sources: String,
    existing_projects: String,
) -> napi::Result<String> {
    run_async(async move {
        let sources = crate::js::from_json(&sources)?;
        let existing_projects = crate::js::from_json(&existing_projects)?;
        let out = port::build_project_import_catalog(sources, existing_projects).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `canProjectTurnModelSelectionForSession`.
#[napi(js_name = "canProjectTurnModelSelectionForSession")]
pub fn can_project_turn_model_selection_for_session(
    session: String,
    requested_instance_id: String,
) -> napi::Result<bool> {
    run(move || {
        let session = crate::js::from_json(&session)?;
        let out =
            port::can_project_turn_model_selection_for_session(session, requested_instance_id)?;
        Ok(out)
    })
}

/// `createEmptyReadModel`.
#[napi(js_name = "createEmptyReadModel")]
pub fn create_empty_read_model(now_iso: String) -> napi::Result<String> {
    run(move || {
        let out = port::create_empty_read_model(now_iso)?;
        crate::js::to_json(&out)
    })
}

/// `projectEvent`.
#[napi(js_name = "projectEvent")]
pub fn project_event(model: String, event: String) -> napi::Result<String> {
    run(move || {
        let model = crate::js::from_json(&model)?;
        let event = crate::js::from_json(&event)?;
        let out = port::project_event(model, event)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeProviderStartOptionsForPersistence`.
#[napi(js_name = "sanitizeProviderStartOptionsForPersistence")]
pub fn sanitize_provider_start_options_for_persistence(
    provider_options: String,
) -> napi::Result<String> {
    run(move || {
        let provider_options = crate::js::from_json(&provider_options)?;
        let out = port::sanitize_provider_start_options_for_persistence(provider_options)?;
        crate::js::to_json(&out)
    })
}

/// `readableReasoningDetail`.
#[napi(js_name = "readableReasoningDetail")]
pub fn readable_reasoning_detail(
    value: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::readable_reasoning_detail(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `runtimePayloadRecord`.
#[napi(js_name = "runtimePayloadRecord")]
pub fn runtime_payload_record(event: String) -> napi::Result<String> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::runtime_payload_record(event)?;
        crate::js::to_json(&out)
    })
}

/// `runtimeTurnState`.
#[napi(js_name = "runtimeTurnState")]
pub fn runtime_turn_state(event: String) -> napi::Result<String> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::runtime_turn_state(event)?;
        Ok(out)
    })
}

/// `providerRuntimeProgressKey`.
#[napi(js_name = "providerRuntimeProgressKey")]
pub fn provider_runtime_progress_key(
    event: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::provider_runtime_progress_key(event)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `providerRuntimeCommandProgressKey`.
#[napi(js_name = "providerRuntimeCommandProgressKey")]
pub fn provider_runtime_command_progress_key(event: String) -> napi::Result<String> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::provider_runtime_command_progress_key(event)?;
        crate::js::to_json(&out)
    })
}

/// `coalesceProviderRuntimeProgress`.
#[napi(js_name = "coalesceProviderRuntimeProgress")]
pub fn coalesce_provider_runtime_progress(page: String) -> napi::Result<String> {
    run(move || {
        let page = crate::js::from_json(&page)?;
        let out = port::coalesce_provider_runtime_progress(page)?;
        crate::js::to_json(&out)
    })
}

/// `providerActivityUpdateFingerprint`.
#[napi(js_name = "providerActivityUpdateFingerprint")]
pub fn provider_activity_update_fingerprint(activity: String) -> napi::Result<String> {
    run(move || {
        let activity = crate::js::from_json(&activity)?;
        let out = port::provider_activity_update_fingerprint(activity)?;
        Ok(out)
    })
}

/// `providerActivityUpdateDedupeKey`.
#[napi(js_name = "providerActivityUpdateDedupeKey")]
pub fn provider_activity_update_dedupe_key(
    event: String,
    thread_id: String,
    activity: String,
) -> napi::Result<String> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let activity = crate::js::from_json(&activity)?;
        let out = port::provider_activity_update_dedupe_key(event, thread_id, activity)?;
        crate::js::to_json(&out)
    })
}

/// `detectCreatedPullRequestUrls`.
#[napi(js_name = "detectCreatedPullRequestUrls")]
pub fn detect_created_pull_request_urls(payload: String) -> napi::Result<Vec<String>> {
    run(move || {
        let payload = crate::js::from_json(&payload)?;
        let out = port::detect_created_pull_request_urls(payload)?;
        Ok(out)
    })
}

/// `buildQuitResumeRecord`.
#[napi(js_name = "buildQuitResumeRecord")]
pub fn build_quit_resume_record(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_quit_resume_record(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildQuitInterruptCommand`.
#[napi(js_name = "buildQuitInterruptCommand")]
pub fn build_quit_interrupt_command(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_quit_interrupt_command(input)?;
        crate::js::to_json(&out)
    })
}

/// `planQuitResumeTurns`.
#[napi(js_name = "planQuitResumeTurns")]
pub fn plan_quit_resume_turns(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::plan_quit_resume_turns(input)?;
        crate::js::to_json(&out)
    })
}

/// `persistQuitResumeRecord`.
#[napi(js_name = "persistQuitResumeRecord")]
pub fn persist_quit_resume_record(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::persist_quit_resume_record(input)?;
        crate::js::to_json(&out)
    })
}

/// `clearQuitResumeRecord`.
#[napi(js_name = "clearQuitResumeRecord")]
pub fn clear_quit_resume_record(_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::clear_quit_resume_record(_path)?;
        crate::js::to_json(&out)
    })
}

/// `persistCrashResumeRecord`.
#[napi(js_name = "persistCrashResumeRecord")]
pub fn persist_crash_resume_record(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::persist_crash_resume_record(input)?;
        crate::js::to_json(&out)
    })
}

/// `readQuitResumeRecord`.
#[napi(js_name = "readQuitResumeRecord")]
pub fn read_quit_resume_record(_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::read_quit_resume_record(_path)?;
        crate::js::to_json(&out)
    })
}

/// `claimQuitResumeRecord`.
#[napi(js_name = "claimQuitResumeRecord")]
pub fn claim_quit_resume_record(_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::claim_quit_resume_record(_path)?;
        crate::js::to_json(&out)
    })
}

/// `resumeQuitInterruptedChats`.
#[napi(js_name = "resumeQuitInterruptedChats")]
pub fn resume_quit_interrupted_chats(_claimed: String) -> napi::Result<String> {
    run(move || {
        let _claimed = crate::js::from_json(&_claimed)?;
        let out = port::resume_quit_interrupted_chats(_claimed)?;
        crate::js::to_json(&out)
    })
}

/// `findRestartOrphanedTasks`.
#[napi(js_name = "findRestartOrphanedTasks")]
pub fn find_restart_orphaned_tasks(
    activities: String,
    started_before: String,
) -> napi::Result<String> {
    run(move || {
        let activities = crate::js::from_json(&activities)?;
        let out = port::find_restart_orphaned_tasks(activities, started_before)?;
        crate::js::to_json(&out)
    })
}

/// `deriveThreadGitMetadataPatch`.
#[napi(js_name = "deriveThreadGitMetadataPatch")]
pub fn derive_thread_git_metadata_patch(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_thread_git_metadata_patch(input)?;
        crate::js::to_json(&out)
    })
}

/// `isOversizedThreadGoal`.
#[napi(js_name = "isOversizedThreadGoal")]
pub fn is_oversized_thread_goal(goal: Option<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::is_oversized_thread_goal(goal)?;
        Ok(out)
    })
}

/// `threadGoalFileName`.
#[napi(js_name = "threadGoalFileName")]
pub fn thread_goal_file_name(command_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::thread_goal_file_name(command_id)?;
        Ok(out)
    })
}

/// `threadGoalFileReference`.
#[napi(js_name = "threadGoalFileReference")]
pub fn thread_goal_file_reference(file_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::thread_goal_file_reference(file_path)?;
        Ok(out)
    })
}

/// `threadGoalFileNameFromReference`.
#[napi(js_name = "threadGoalFileNameFromReference")]
pub fn thread_goal_file_name_from_reference(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::thread_goal_file_name_from_reference(input)?;
        crate::js::to_json(&out)
    })
}

/// `readMaterializedThreadGoalText`.
#[napi(js_name = "readMaterializedThreadGoalText")]
pub async fn read_materialized_thread_goal_text(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::read_materialized_thread_goal_text(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `materializeThreadGoalFile`.
#[napi(js_name = "materializeThreadGoalFile")]
pub async fn materialize_thread_goal_file(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::materialize_thread_goal_file(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `pruneThreadGoalFiles`.
#[napi(js_name = "pruneThreadGoalFiles")]
pub async fn prune_thread_goal_files(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::prune_thread_goal_files(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `discardMaterializedThreadGoalFile`.
#[napi(js_name = "discardMaterializedThreadGoalFile")]
pub async fn discard_materialized_thread_goal_file(file_path: String) -> napi::Result<String> {
    run_async(async move {
        let file_path = crate::js::from_json(&file_path)?;
        let out = port::discard_materialized_thread_goal_file(file_path).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `formatThreadLatestReplyPreview`.
#[napi(js_name = "formatThreadLatestReplyPreview")]
pub fn format_thread_latest_reply_preview(source: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::format_thread_latest_reply_preview(source)?;
        crate::js::to_json(&out)
    })
}

/// `shouldApplyThreadsProjection`.
#[napi(js_name = "shouldApplyThreadsProjection")]
pub fn should_apply_threads_projection(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::should_apply_threads_projection(event)?;
        Ok(out)
    })
}

/// `shouldApplyDeferredThreadShellSummary`.
#[napi(js_name = "shouldApplyDeferredThreadShellSummary")]
pub fn should_apply_deferred_thread_shell_summary(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::should_apply_deferred_thread_shell_summary(event)?;
        Ok(out)
    })
}

/// `shouldPublishThreadShellForEvent`.
#[napi(js_name = "shouldPublishThreadShellForEvent")]
pub fn should_publish_thread_shell_for_event(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::should_publish_thread_shell_for_event(event)?;
        Ok(out)
    })
}

/// `projectProviderRuntimeActivities`.
#[napi(js_name = "projectProviderRuntimeActivities")]
pub fn project_provider_runtime_activities(
    event: String,
    session_sequence: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::project_provider_runtime_activities(event, session_sequence)?;
        crate::js::to_json(&out)
    })
}

/// `canonicalImportPath`.
#[napi(js_name = "canonicalImportPath")]
pub async fn canonical_import_path(value: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::canonical_import_path(value).await?;
        Ok(out)
    })
    .await
}

/// `importDirectoryExists`.
#[napi(js_name = "importDirectoryExists")]
pub async fn import_directory_exists(value: String) -> napi::Result<bool> {
    run_async(async move {
        let out = port::import_directory_exists(value).await?;
        Ok(out)
    })
    .await
}

/// `findImportGitWorkspace`.
#[napi(js_name = "findImportGitWorkspace")]
pub async fn find_import_git_workspace(cwd: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::find_import_git_workspace(cwd).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `resolveProjectImportSourceHome`.
#[napi(js_name = "resolveProjectImportSourceHome")]
pub async fn resolve_project_import_source_home(source: String) -> napi::Result<String> {
    run_async(async move {
        let source = crate::js::from_json(&source)?;
        let out = port::resolve_project_import_source_home(source).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `planRestartTurnReconciliation`.
#[napi(js_name = "planRestartTurnReconciliation")]
pub fn plan_restart_turn_reconciliation(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::plan_restart_turn_reconciliation(input)?;
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

/// `threadModelSelectionChangedDetail` over the binary wire.
#[napi(js_name = "threadModelSelectionChangedDetailWire")]
pub fn thread_model_selection_changed_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: String = r.string16()?;
        let out = &port::thread_model_selection_changed_detail(thread_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_TO_PROJECTOR_DECODE_ERROR_OUT: wire::Schema = wire::Schema::Dyn;

/// `toProjectorDecodeError` over the binary wire.
#[napi(js_name = "toProjectorDecodeErrorWire")]
pub fn to_projector_decode_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event_type: String = r.string16()?;
        let out = &port::to_projector_decode_error(event_type)?;
        wire::write_value(w, out, &S_TO_PROJECTOR_DECODE_ERROR_OUT)?;
        Ok(())
    })
}

static S_FINGERPRINT_ORCHESTRATION_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_FINGERPRINT_ORCHESTRATION_COMMAND_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    ("value", wire::Schema::Str16),
]);

/// `fingerprintOrchestrationCommand` over the binary wire.
#[napi(js_name = "fingerprintOrchestrationCommandWire")]
pub fn fingerprint_orchestration_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: Value = wire::read_value(r, &S_FINGERPRINT_ORCHESTRATION_COMMAND_0)?;
        let out = &port::fingerprint_orchestration_command(command)?;
        wire::write_value(w, out, &S_FINGERPRINT_ORCHESTRATION_COMMAND_OUT)?;
        Ok(())
    })
}

static S_THREAD_HAS_IN_FLIGHT_TURN_0: wire::Schema = wire::Schema::Dyn;

/// `threadHasInFlightTurn` over the binary wire.
#[napi(js_name = "threadHasInFlightTurnWire")]
pub fn thread_has_in_flight_turn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_THREAD_HAS_IN_FLIGHT_TURN_0)?;
        let out = &port::thread_has_in_flight_turn(thread)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CHECKPOINT_REVERT_ACTIVE_TURN_DETAIL_0: wire::Schema = wire::Schema::Dyn;

/// `checkpointRevertActiveTurnDetail` over the binary wire.
#[napi(js_name = "checkpointRevertActiveTurnDetailWire")]
pub fn checkpoint_revert_active_turn_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: Value = wire::read_value(r, &S_CHECKPOINT_REVERT_ACTIVE_TURN_DETAIL_0)?;
        let out = &port::checkpoint_revert_active_turn_detail(thread_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_THREAD_HAS_CHECKPOINT_REVERT_IN_PROGRESS_0: wire::Schema = wire::Schema::Dyn;

/// `threadHasCheckpointRevertInProgress` over the binary wire.
#[napi(js_name = "threadHasCheckpointRevertInProgressWire")]
pub fn thread_has_checkpoint_revert_in_progress_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_THREAD_HAS_CHECKPOINT_REVERT_IN_PROGRESS_0)?;
        let out = &port::thread_has_checkpoint_revert_in_progress(thread)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CHECKPOINT_REVERT_IN_PROGRESS_DETAIL_0: wire::Schema = wire::Schema::Dyn;

/// `checkpointRevertInProgressDetail` over the binary wire.
#[napi(js_name = "checkpointRevertInProgressDetailWire")]
pub fn checkpoint_revert_in_progress_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: Value = wire::read_value(r, &S_CHECKPOINT_REVERT_IN_PROGRESS_DETAIL_0)?;
        let out = &port::checkpoint_revert_in_progress_detail(thread_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_CHECKPOINT_REVERT_DELETE_IN_PROGRESS_DETAIL_0: wire::Schema = wire::Schema::Dyn;

/// `checkpointRevertDeleteInProgressDetail` over the binary wire.
#[napi(js_name = "checkpointRevertDeleteInProgressDetailWire")]
pub fn checkpoint_revert_delete_in_progress_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: Value =
            wire::read_value(r, &S_CHECKPOINT_REVERT_DELETE_IN_PROGRESS_DETAIL_0)?;
        let out = &port::checkpoint_revert_delete_in_progress_detail(thread_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_FIND_THREAD_BY_ID_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_THREAD_BY_ID_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_THREAD_BY_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `findThreadById` over the binary wire.
#[napi(js_name = "findThreadByIdWire")]
pub fn find_thread_by_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let read_model: Value = wire::read_value(r, &S_FIND_THREAD_BY_ID_0)?;
        let thread_id: Value = wire::read_value(r, &S_FIND_THREAD_BY_ID_1)?;
        let out = &port::find_thread_by_id(read_model, thread_id)?;
        wire::write_value(w, out, &S_FIND_THREAD_BY_ID_OUT)?;
        Ok(())
    })
}

static S_FIND_PROJECT_BY_ID_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_PROJECT_BY_ID_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_PROJECT_BY_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `findProjectById` over the binary wire.
#[napi(js_name = "findProjectByIdWire")]
pub fn find_project_by_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let read_model: Value = wire::read_value(r, &S_FIND_PROJECT_BY_ID_0)?;
        let project_id: Value = wire::read_value(r, &S_FIND_PROJECT_BY_ID_1)?;
        let out = &port::find_project_by_id(read_model, project_id)?;
        wire::write_value(w, out, &S_FIND_PROJECT_BY_ID_OUT)?;
        Ok(())
    })
}

static S_FIND_SPACE_BY_ID_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_SPACE_BY_ID_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_SPACE_BY_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `findSpaceById` over the binary wire.
#[napi(js_name = "findSpaceByIdWire")]
pub fn find_space_by_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let read_model: Value = wire::read_value(r, &S_FIND_SPACE_BY_ID_0)?;
        let space_id: Value = wire::read_value(r, &S_FIND_SPACE_BY_ID_1)?;
        let out = &port::find_space_by_id(read_model, space_id)?;
        wire::write_value(w, out, &S_FIND_SPACE_BY_ID_OUT)?;
        Ok(())
    })
}

static S_LIST_ACTIVE_SPACES_0: wire::Schema = wire::Schema::Dyn;

/// `listActiveSpaces` over the binary wire.
#[napi(js_name = "listActiveSpacesWire")]
pub fn list_active_spaces_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let read_model: Value = wire::read_value(r, &S_LIST_ACTIVE_SPACES_0)?;
        let out = &port::list_active_spaces(read_model)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_REQUIRE_SPACE_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_SPACE_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireSpace` over the binary wire.
#[napi(js_name = "requireSpaceWire")]
pub fn require_space_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_SPACE_0)?;
        let out = &port::require_space(input)?;
        wire::write_value(w, out, &S_REQUIRE_SPACE_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_SPACE_ABSENT_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_SPACE_ABSENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireSpaceAbsent` over the binary wire.
#[napi(js_name = "requireSpaceAbsentWire")]
pub fn require_space_absent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_SPACE_ABSENT_0)?;
        let out = &port::require_space_absent(input)?;
        wire::write_value(w, out, &S_REQUIRE_SPACE_ABSENT_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_SPACE_NAME_AVAILABLE_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_SPACE_NAME_AVAILABLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireSpaceNameAvailable` over the binary wire.
#[napi(js_name = "requireSpaceNameAvailableWire")]
pub fn require_space_name_available_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_SPACE_NAME_AVAILABLE_0)?;
        let out = &port::require_space_name_available(input)?;
        wire::write_value(w, out, &S_REQUIRE_SPACE_NAME_AVAILABLE_OUT)?;
        Ok(())
    })
}

static S_IS_LEGACY_HOME_CHAT_CONTAINER_ROW_0: wire::Schema = wire::Schema::Dyn;

/// `isLegacyHomeChatContainerRow` over the binary wire.
#[napi(js_name = "isLegacyHomeChatContainerRowWire")]
pub fn is_legacy_home_chat_container_row_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_IS_LEGACY_HOME_CHAT_CONTAINER_ROW_0)?;
        let out = &port::is_legacy_home_chat_container_row(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_ORDINARY_PROJECT_ROW_0: wire::Schema = wire::Schema::Dyn;

/// `isOrdinaryProjectRow` over the binary wire.
#[napi(js_name = "isOrdinaryProjectRowWire")]
pub fn is_ordinary_project_row_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_IS_ORDINARY_PROJECT_ROW_0)?;
        let out = &port::is_ordinary_project_row(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_REQUIRE_SPACE_ASSIGNABLE_PROJECT_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_SPACE_ASSIGNABLE_PROJECT_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireSpaceAssignableProject` over the binary wire.
#[napi(js_name = "requireSpaceAssignableProjectWire")]
pub fn require_space_assignable_project_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_SPACE_ASSIGNABLE_PROJECT_0)?;
        let out = &port::require_space_assignable_project(input)?;
        wire::write_value(w, out, &S_REQUIRE_SPACE_ASSIGNABLE_PROJECT_OUT)?;
        Ok(())
    })
}

static S_LIST_ACTIVE_PROJECTS_BY_WORKSPACE_ROOT_0: wire::Schema = wire::Schema::Dyn;
static S_LIST_ACTIVE_PROJECTS_BY_WORKSPACE_ROOT_2: wire::Schema = wire::Schema::Dyn;

/// `listActiveProjectsByWorkspaceRoot` over the binary wire.
#[napi(js_name = "listActiveProjectsByWorkspaceRootWire")]
pub fn list_active_projects_by_workspace_root_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let read_model: Value = wire::read_value(r, &S_LIST_ACTIVE_PROJECTS_BY_WORKSPACE_ROOT_0)?;
        let workspace_root: String = r.string16()?;
        let options: Value = wire::read_value(r, &S_LIST_ACTIVE_PROJECTS_BY_WORKSPACE_ROOT_2)?;
        let out =
            &port::list_active_projects_by_workspace_root(read_model, workspace_root, options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_LIST_THREADS_BY_PROJECT_ID_0: wire::Schema = wire::Schema::Dyn;
static S_LIST_THREADS_BY_PROJECT_ID_1: wire::Schema = wire::Schema::Dyn;

/// `listThreadsByProjectId` over the binary wire.
#[napi(js_name = "listThreadsByProjectIdWire")]
pub fn list_threads_by_project_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let read_model: Value = wire::read_value(r, &S_LIST_THREADS_BY_PROJECT_ID_0)?;
        let project_id: Value = wire::read_value(r, &S_LIST_THREADS_BY_PROJECT_ID_1)?;
        let out = &port::list_threads_by_project_id(read_model, project_id)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_REQUIRE_PROJECT_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_PROJECT_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireProject` over the binary wire.
#[napi(js_name = "requireProjectWire")]
pub fn require_project_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_PROJECT_0)?;
        let out = &port::require_project(input)?;
        wire::write_value(w, out, &S_REQUIRE_PROJECT_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_PROJECT_ABSENT_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_PROJECT_ABSENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireProjectAbsent` over the binary wire.
#[napi(js_name = "requireProjectAbsentWire")]
pub fn require_project_absent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_PROJECT_ABSENT_0)?;
        let out = &port::require_project_absent(input)?;
        wire::write_value(w, out, &S_REQUIRE_PROJECT_ABSENT_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_VALID_PROJECT_FOLDERS_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_VALID_PROJECT_FOLDERS_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireValidProjectFolders` over the binary wire.
#[napi(js_name = "requireValidProjectFoldersWire")]
pub fn require_valid_project_folders_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_VALID_PROJECT_FOLDERS_0)?;
        let out = &port::require_valid_project_folders(input)?;
        wire::write_value(w, out, &S_REQUIRE_VALID_PROJECT_FOLDERS_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_PROJECT_WORKSPACE_ROOT_AVAILABLE_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_PROJECT_WORKSPACE_ROOT_AVAILABLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireProjectWorkspaceRootAvailable` over the binary wire.
#[napi(js_name = "requireProjectWorkspaceRootAvailableWire")]
pub fn require_project_workspace_root_available_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_PROJECT_WORKSPACE_ROOT_AVAILABLE_0)?;
        let out = &port::require_project_workspace_root_available(input)?;
        wire::write_value(w, out, &S_REQUIRE_PROJECT_WORKSPACE_ROOT_AVAILABLE_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_PROJECT_HAS_NO_THREADS_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_PROJECT_HAS_NO_THREADS_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireProjectHasNoThreads` over the binary wire.
#[napi(js_name = "requireProjectHasNoThreadsWire")]
pub fn require_project_has_no_threads_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_PROJECT_HAS_NO_THREADS_0)?;
        let out = &port::require_project_has_no_threads(input)?;
        wire::write_value(w, out, &S_REQUIRE_PROJECT_HAS_NO_THREADS_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_THREAD_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_THREAD_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireThread` over the binary wire.
#[napi(js_name = "requireThreadWire")]
pub fn require_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_THREAD_0)?;
        let out = &port::require_thread(input)?;
        wire::write_value(w, out, &S_REQUIRE_THREAD_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_APPROVAL_NOT_RESPONDED_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_APPROVAL_NOT_RESPONDED_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireApprovalNotResponded` over the binary wire.
#[napi(js_name = "requireApprovalNotRespondedWire")]
pub fn require_approval_not_responded_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_APPROVAL_NOT_RESPONDED_0)?;
        let out = &port::require_approval_not_responded(input)?;
        wire::write_value(w, out, &S_REQUIRE_APPROVAL_NOT_RESPONDED_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_THREAD_ABSENT_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_THREAD_ABSENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireThreadAbsent` over the binary wire.
#[napi(js_name = "requireThreadAbsentWire")]
pub fn require_thread_absent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_THREAD_ABSENT_0)?;
        let out = &port::require_thread_absent(input)?;
        wire::write_value(w, out, &S_REQUIRE_THREAD_ABSENT_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_THREAD_ARCHIVED_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_THREAD_ARCHIVED_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireThreadArchived` over the binary wire.
#[napi(js_name = "requireThreadArchivedWire")]
pub fn require_thread_archived_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_THREAD_ARCHIVED_0)?;
        let out = &port::require_thread_archived(input)?;
        wire::write_value(w, out, &S_REQUIRE_THREAD_ARCHIVED_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_THREAD_NOT_ARCHIVED_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_THREAD_NOT_ARCHIVED_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireThreadNotArchived` over the binary wire.
#[napi(js_name = "requireThreadNotArchivedWire")]
pub fn require_thread_not_archived_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_REQUIRE_THREAD_NOT_ARCHIVED_0)?;
        let out = &port::require_thread_not_archived(input)?;
        wire::write_value(w, out, &S_REQUIRE_THREAD_NOT_ARCHIVED_OUT)?;
        Ok(())
    })
}

static S_THREAD_RESUME_PRECONDITION_VIOLATION_0: wire::Schema = wire::Schema::Dyn;
static S_THREAD_RESUME_PRECONDITION_VIOLATION_1: wire::Schema = wire::Schema::Dyn;
static S_THREAD_RESUME_PRECONDITION_VIOLATION_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `threadResumePreconditionViolation` over the binary wire.
#[napi(js_name = "threadResumePreconditionViolationWire")]
pub fn thread_resume_precondition_violation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_THREAD_RESUME_PRECONDITION_VIOLATION_0)?;
        let precondition: Value = wire::read_value(r, &S_THREAD_RESUME_PRECONDITION_VIOLATION_1)?;
        let out = &port::thread_resume_precondition_violation(thread, precondition)?;
        wire::write_value(w, out, &S_THREAD_RESUME_PRECONDITION_VIOLATION_OUT)?;
        Ok(())
    })
}

static S_THREAD_RESUME_PRECONDITION_DETAIL_0: wire::Schema = wire::Schema::Dyn;
static S_THREAD_RESUME_PRECONDITION_DETAIL_OUT: wire::Schema = wire::Schema::Str16;

/// `threadResumePreconditionDetail` over the binary wire.
#[napi(js_name = "threadResumePreconditionDetailWire")]
pub fn thread_resume_precondition_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: Value = wire::read_value(r, &S_THREAD_RESUME_PRECONDITION_DETAIL_0)?;
        let violation: String = r.string16()?;
        let out = &port::thread_resume_precondition_detail(thread_id, violation)?;
        wire::write_value(w, out, &S_THREAD_RESUME_PRECONDITION_DETAIL_OUT)?;
        Ok(())
    })
}

static S_WITHOUT_REMOTE_RESUME_COMPUTER_GRANT_0: wire::Schema = wire::Schema::Dyn;
static S_WITHOUT_REMOTE_RESUME_COMPUTER_GRANT_OUT: wire::Schema = wire::Schema::Dyn;

/// `withoutRemoteResumeComputerGrant` over the binary wire.
#[napi(js_name = "withoutRemoteResumeComputerGrantWire")]
pub fn without_remote_resume_computer_grant_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: Value = wire::read_value(r, &S_WITHOUT_REMOTE_RESUME_COMPUTER_GRANT_0)?;
        let principal_owner_id: String = r.string16()?;
        let out = &port::without_remote_resume_computer_grant(command, principal_owner_id)?;
        wire::write_value(w, out, &S_WITHOUT_REMOTE_RESUME_COMPUTER_GRANT_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_FORK_SOURCE_CUTOFF_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_FORK_SOURCE_CUTOFF_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveForkSourceCutoff` over the binary wire.
#[napi(js_name = "resolveForkSourceCutoffWire")]
pub fn resolve_fork_source_cutoff_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_RESOLVE_FORK_SOURCE_CUTOFF_0)?;
        let out = &port::resolve_fork_source_cutoff(input)?;
        wire::write_value(w, out, &S_RESOLVE_FORK_SOURCE_CUTOFF_OUT)?;
        Ok(())
    })
}

static S_BUILD_FORK_THREAD_TITLE_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_FORK_THREAD_TITLE_1: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("projectId", wire::Schema::Str16),
    ("title", wire::Schema::Str16),
    (
        "forkSourceThreadId",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
]));

/// `buildForkThreadTitle` over the binary wire.
#[napi(js_name = "buildForkThreadTitleWire")]
pub fn build_fork_thread_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: Value = wire::read_value(r, &S_BUILD_FORK_THREAD_TITLE_0)?;
        let project_threads: Vec<Value> =
            r.list(|r| wire::read_value(r, S_BUILD_FORK_THREAD_TITLE_1.inner()))?;
        let out = &port::build_fork_thread_title(source, project_threads)?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_STABLE_MESSAGE_TURN_ID_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_STABLE_MESSAGE_TURN_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveStableMessageTurnId` over the binary wire.
#[napi(js_name = "resolveStableMessageTurnIdWire")]
pub fn resolve_stable_message_turn_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_RESOLVE_STABLE_MESSAGE_TURN_ID_0)?;
        let out = &port::resolve_stable_message_turn_id(input)?;
        wire::write_value(w, out, &S_RESOLVE_STABLE_MESSAGE_TURN_ID_OUT)?;
        Ok(())
    })
}

static S_USES_RESERVED_COMMAND_ADMISSION_0: wire::Schema = wire::Schema::Dyn;

/// `usesReservedCommandAdmission` over the binary wire.
#[napi(js_name = "usesReservedCommandAdmissionWire")]
pub fn uses_reserved_command_admission_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let r#type: Value = wire::read_value(r, &S_USES_RESERVED_COMMAND_ADMISSION_0)?;
        let out = &port::uses_reserved_command_admission(r#type)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_QUIESCING_COMMAND_ADMISSIBLE_0: wire::Schema = wire::Schema::Dyn;

/// `isQuiescingCommandAdmissible` over the binary wire.
#[napi(js_name = "isQuiescingCommandAdmissibleWire")]
pub fn is_quiescing_command_admissible_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let r#type: Value = wire::read_value(r, &S_IS_QUIESCING_COMMAND_ADMISSIBLE_0)?;
        let out = &port::is_quiescing_command_admissible(r#type)?;
        w.bool(*out);
        Ok(())
    })
}

static S_ORCHESTRATION_COMMAND_LANE_0: wire::Schema = wire::Schema::Dyn;

/// `orchestrationCommandLane` over the binary wire.
#[napi(js_name = "orchestrationCommandLaneWire")]
pub fn orchestration_command_lane_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let r#type: Value = wire::read_value(r, &S_ORCHESTRATION_COMMAND_LANE_0)?;
        let out = &port::orchestration_command_lane(r#type)?;
        w.string16(out);
        Ok(())
    })
}

static S_IS_EXPIRED_SIDECHAT_0: wire::Schema = wire::Schema::Dyn;

/// `isExpiredSidechat` over the binary wire.
#[napi(js_name = "isExpiredSidechatWire")]
pub fn is_expired_sidechat_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_IS_EXPIRED_SIDECHAT_0)?;
        let out = &port::is_expired_sidechat(thread)?;
        w.bool(*out);
        Ok(())
    })
}

/// `latestSidechatActivityAt` over the binary wire.
#[napi(js_name = "latestSidechatActivityAtWire")]
pub fn latest_sidechat_activity_at_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let current: Option<String> = r.opt_js(|r| r.string16())?;
        let candidate: String = r.string16()?;
        let out = &port::latest_sidechat_activity_at(current, candidate)?;
        w.string16(out);
        Ok(())
    })
}

/// `sidechatActivityInstantsEqual` over the binary wire.
#[napi(js_name = "sidechatActivityInstantsEqualWire")]
pub fn sidechat_activity_instants_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: String = r.string16()?;
        let right: String = r.string16()?;
        let out = &port::sidechat_activity_instants_equal(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SETTLE_TURN_STATE_FROM_SESSION_0: wire::Schema = wire::Schema::Dyn;
static S_SETTLE_TURN_STATE_FROM_SESSION_OUT: wire::Schema = wire::Schema::Dyn;

/// `settleTurnStateFromSession` over the binary wire.
#[napi(js_name = "settleTurnStateFromSessionWire")]
pub fn settle_turn_state_from_session_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let session: Value = wire::read_value(r, &S_SETTLE_TURN_STATE_FROM_SESSION_0)?;
        let existing_state: String = r.string16()?;
        let out = &port::settle_turn_state_from_session(session, existing_state)?;
        wire::write_value(w, out, &S_SETTLE_TURN_STATE_FROM_SESSION_OUT)?;
        Ok(())
    })
}

static S_MAX_ISO_OUT: wire::Schema = wire::Schema::Str16;

/// `maxIso` over the binary wire.
#[napi(js_name = "maxIsoWire")]
pub fn max_iso_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Option<String> = r.opt_js(|r| r.string16())?;
        let right: String = r.string16()?;
        let out = &port::max_iso(left, right)?;
        wire::write_value(w, out, &S_MAX_ISO_OUT)?;
        Ok(())
    })
}

static S_DERIVE_TURN_START_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_TURN_START_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `deriveTurnStartModelSelection` over the binary wire.
#[napi(js_name = "deriveTurnStartModelSelectionWire")]
pub fn derive_turn_start_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_DERIVE_TURN_START_MODEL_SELECTION_0)?;
        let out = &port::derive_turn_start_model_selection(input)?;
        wire::write_value(w, out, &S_DERIVE_TURN_START_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_COUNT_NATIVE_TURN_START_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `countNativeTurnStartMessages` over the binary wire.
#[napi(js_name = "countNativeTurnStartMessagesWire")]
pub fn count_native_turn_start_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let messages: Value = wire::read_value(r, &S_COUNT_NATIVE_TURN_START_MESSAGES_0)?;
        let out = &port::count_native_turn_start_messages(messages)?;
        w.f64(*out);
        Ok(())
    })
}

static S_CAN_ADOPT_FIRST_TURN_PROVIDER_0: wire::Schema = wire::Schema::Dyn;

/// `canAdoptFirstTurnProvider` over the binary wire.
#[napi(js_name = "canAdoptFirstTurnProviderWire")]
pub fn can_adopt_first_turn_provider_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_CAN_ADOPT_FIRST_TURN_PROVIDER_0)?;
        let out = &port::can_adopt_first_turn_provider(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_DERIVE_TURN_START_SESSION_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_TURN_START_SESSION_OUT: wire::Schema = wire::Schema::Dyn;

/// `deriveTurnStartSession` over the binary wire.
#[napi(js_name = "deriveTurnStartSessionWire")]
pub fn derive_turn_start_session_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_DERIVE_TURN_START_SESSION_0)?;
        let out = &port::derive_turn_start_session(input)?;
        wire::write_value(w, out, &S_DERIVE_TURN_START_SESSION_OUT)?;
        Ok(())
    })
}

static S_RELOCATE_PROJECT_PATH_3: wire::Schema = wire::Schema::Dyn;
static S_RELOCATE_PROJECT_PATH_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `relocateProjectPath` over the binary wire.
#[napi(js_name = "relocateProjectPathWire")]
pub fn relocate_project_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Option<String> = r.opt_js(|r| r.string16())?;
        let previous_root: String = r.string16()?;
        let next_root: String = r.string16()?;
        let platform: Value = wire::read_value(r, &S_RELOCATE_PROJECT_PATH_3)?;
        let out = &port::relocate_project_path(value, previous_root, next_root, platform)?;
        wire::write_value(w, out, &S_RELOCATE_PROJECT_PATH_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_WORKSPACE_CHANGED_2: wire::Schema = wire::Schema::Dyn;

/// `providerWorkspaceChanged` over the binary wire.
#[napi(js_name = "providerWorkspaceChangedWire")]
pub fn provider_workspace_changed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let current_cwd: Option<String> = r.opt_js(|r| r.string16())?;
        let requested_cwd: Option<String> = r.opt_js(|r| r.string16())?;
        let platform: Value = wire::read_value(r, &S_PROVIDER_WORKSPACE_CHANGED_2)?;
        let out = &port::provider_workspace_changed(current_cwd, requested_cwd, platform)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isProviderIntentEventType` over the binary wire.
#[napi(js_name = "isProviderIntentEventTypeWire")]
pub fn is_provider_intent_event_type_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event_type: String = r.string16()?;
        let out = &port::is_provider_intent_event_type(event_type)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_PROVIDER_INTENT_EVENT_0: wire::Schema = wire::Schema::Dyn;

/// `isProviderIntentEvent` over the binary wire.
#[napi(js_name = "isProviderIntentEventWire")]
pub fn is_provider_intent_event_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_IS_PROVIDER_INTENT_EVENT_0)?;
        let out = &port::is_provider_intent_event(event)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_REPLAY_SAFE_CLAIMED_PROVIDER_INTENT_0: wire::Schema = wire::Schema::Dyn;

/// `isReplaySafeClaimedProviderIntent` over the binary wire.
#[napi(js_name = "isReplaySafeClaimedProviderIntentWire")]
pub fn is_replay_safe_claimed_provider_intent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_IS_REPLAY_SAFE_CLAIMED_PROVIDER_INTENT_0)?;
        let out = &port::is_replay_safe_claimed_provider_intent(event)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_PROVIDER_SIDE_EFFECT_INTENT_0: wire::Schema = wire::Schema::Dyn;

/// `isProviderSideEffectIntent` over the binary wire.
#[napi(js_name = "isProviderSideEffectIntentWire")]
pub fn is_provider_side_effect_intent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_IS_PROVIDER_SIDE_EFFECT_INTENT_0)?;
        let out = &port::is_provider_side_effect_intent(event)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_CLAIMED_PROVIDER_INTENT_0: wire::Schema = wire::Schema::Dyn;

/// `isClaimedProviderIntent` over the binary wire.
#[napi(js_name = "isClaimedProviderIntentWire")]
pub fn is_claimed_provider_intent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_IS_CLAIMED_PROVIDER_INTENT_0)?;
        let out = &port::is_claimed_provider_intent(event)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_QUARANTINE_EXEMPT_PROVIDER_INTENT_0: wire::Schema = wire::Schema::Dyn;

/// `isQuarantineExemptProviderIntent` over the binary wire.
#[napi(js_name = "isQuarantineExemptProviderIntentWire")]
pub fn is_quarantine_exempt_provider_intent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_IS_QUARANTINE_EXEMPT_PROVIDER_INTENT_0)?;
        let out = &port::is_quarantine_exempt_provider_intent(event)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_UNSETTLED_PENDING_INTERACTION_0: wire::Schema = wire::Schema::Dyn;

/// `isUnsettledPendingInteraction` over the binary wire.
#[napi(js_name = "isUnsettledPendingInteractionWire")]
pub fn is_unsettled_pending_interaction_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: Value = wire::read_value(r, &S_IS_UNSETTLED_PENDING_INTERACTION_0)?;
        let out = &port::is_unsettled_pending_interaction(row)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PENDING_INTERACTION_REQUEST_KIND_0: wire::Schema = wire::Schema::Dyn;

/// `pendingInteractionRequestKind` over the binary wire.
#[napi(js_name = "pendingInteractionRequestKindWire")]
pub fn pending_interaction_request_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let interaction_kind: Value = wire::read_value(r, &S_PENDING_INTERACTION_REQUEST_KIND_0)?;
        let out = &port::pending_interaction_request_kind(interaction_kind)?;
        w.string16(out);
        Ok(())
    })
}

static S_BUILD_STALE_PENDING_REQUEST_SETTLEMENT_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_STALE_PENDING_REQUEST_SETTLEMENT_COMMAND_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildStalePendingRequestSettlementCommand` over the binary wire.
#[napi(js_name = "buildStalePendingRequestSettlementCommandWire")]
pub fn build_stale_pending_request_settlement_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value =
            wire::read_value(r, &S_BUILD_STALE_PENDING_REQUEST_SETTLEMENT_COMMAND_0)?;
        let out = &port::build_stale_pending_request_settlement_command(input)?;
        wire::write_value(
            w,
            out,
            &S_BUILD_STALE_PENDING_REQUEST_SETTLEMENT_COMMAND_OUT,
        )?;
        Ok(())
    })
}

static S_NATIVE_IMPORT_ID_0: wire::Schema = wire::Schema::Dyn;
static S_NATIVE_IMPORT_ID_1: wire::Schema = wire::Schema::Dyn;
static S_NATIVE_IMPORT_ID_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `nativeImportId` over the binary wire.
#[napi(js_name = "nativeImportIdWire")]
pub fn native_import_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: Value = wire::read_value(r, &S_NATIVE_IMPORT_ID_0)?;
        let cursor: Value = wire::read_value(r, &S_NATIVE_IMPORT_ID_1)?;
        let out = &port::native_import_id(provider, cursor)?;
        wire::write_value(w, out, &S_NATIVE_IMPORT_ID_OUT)?;
        Ok(())
    })
}

static S_MAKE_PROJECT_IMPORT_DESTINATIONS_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_PROJECT_IMPORT_DESTINATIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeProjectImportDestinations` over the binary wire.
#[napi(js_name = "makeProjectImportDestinationsWire")]
pub fn make_project_import_destinations_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Value = wire::read_value(r, &S_MAKE_PROJECT_IMPORT_DESTINATIONS_0)?;
        let out = &port::make_project_import_destinations(model)?;
        wire::write_value(w, out, &S_MAKE_PROJECT_IMPORT_DESTINATIONS_OUT)?;
        Ok(())
    })
}

static S_THREAD_ARCHIVE_FILE_NAME_0: wire::Schema = wire::Schema::Dyn;

/// `threadArchiveFileName` over the binary wire.
#[napi(js_name = "threadArchiveFileNameWire")]
pub fn thread_archive_file_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_THREAD_ARCHIVE_FILE_NAME_0)?;
        let out = &port::thread_archive_file_name(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_LIST_IMPORTED_HANDOFF_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `listImportedHandoffMessages` over the binary wire.
#[napi(js_name = "listImportedHandoffMessagesWire")]
pub fn list_imported_handoff_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_LIST_IMPORTED_HANDOFF_MESSAGES_0)?;
        let out = &port::list_imported_handoff_messages(thread)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_LIST_IMPORTED_FORK_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `listImportedForkMessages` over the binary wire.
#[napi(js_name = "listImportedForkMessagesWire")]
pub fn list_imported_fork_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_LIST_IMPORTED_FORK_MESSAGES_0)?;
        let out = &port::list_imported_fork_messages(thread)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_HAS_NATIVE_HANDOFF_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `hasNativeHandoffMessages` over the binary wire.
#[napi(js_name = "hasNativeHandoffMessagesWire")]
pub fn has_native_handoff_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_HAS_NATIVE_HANDOFF_MESSAGES_0)?;
        let out = &port::has_native_handoff_messages(thread)?;
        w.bool(*out);
        Ok(())
    })
}

static S_HAS_NATIVE_ASSISTANT_MESSAGES_BEFORE_0: wire::Schema = wire::Schema::Dyn;

/// `hasNativeAssistantMessagesBefore` over the binary wire.
#[napi(js_name = "hasNativeAssistantMessagesBeforeWire")]
pub fn has_native_assistant_messages_before_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_HAS_NATIVE_ASSISTANT_MESSAGES_BEFORE_0)?;
        let current_message_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::has_native_assistant_messages_before(thread, current_message_id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_LIST_PRIOR_TRANSCRIPT_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `listPriorTranscriptMessages` over the binary wire.
#[napi(js_name = "listPriorTranscriptMessagesWire")]
pub fn list_prior_transcript_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_LIST_PRIOR_TRANSCRIPT_MESSAGES_0)?;
        let current_message_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::list_prior_transcript_messages(thread, current_message_id)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_BUILD_HANDOFF_BOOTSTRAP_TEXT_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_HANDOFF_BOOTSTRAP_TEXT_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `buildHandoffBootstrapText` over the binary wire.
#[napi(js_name = "buildHandoffBootstrapTextWire")]
pub fn build_handoff_bootstrap_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_BUILD_HANDOFF_BOOTSTRAP_TEXT_0)?;
        let max_chars: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::build_handoff_bootstrap_text(thread, max_chars)?;
        wire::write_value(w, out, &S_BUILD_HANDOFF_BOOTSTRAP_TEXT_OUT)?;
        Ok(())
    })
}

static S_BUILD_PRIOR_TRANSCRIPT_BOOTSTRAP_TEXT_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PRIOR_TRANSCRIPT_BOOTSTRAP_TEXT_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `buildPriorTranscriptBootstrapText` over the binary wire.
#[napi(js_name = "buildPriorTranscriptBootstrapTextWire")]
pub fn build_prior_transcript_bootstrap_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_BUILD_PRIOR_TRANSCRIPT_BOOTSTRAP_TEXT_0)?;
        let current_message_id: Option<String> = r.opt_js(|r| r.string16())?;
        let max_chars: Option<Value> = wire::read_dyn_opt(r)?;
        let out =
            &port::build_prior_transcript_bootstrap_text(thread, current_message_id, max_chars)?;
        wire::write_value(w, out, &S_BUILD_PRIOR_TRANSCRIPT_BOOTSTRAP_TEXT_OUT)?;
        Ok(())
    })
}

static S_BUILD_PRIOR_TRANSCRIPT_BOOTSTRAP_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PRIOR_TRANSCRIPT_BOOTSTRAP_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("text", wire::Schema::Str16),
        ("omittedMessages", wire::Schema::Num),
        ("shortenedMessages", wire::Schema::Num),
    ]));

/// `buildPriorTranscriptBootstrap` over the binary wire.
#[napi(js_name = "buildPriorTranscriptBootstrapWire")]
pub fn build_prior_transcript_bootstrap_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_BUILD_PRIOR_TRANSCRIPT_BOOTSTRAP_0)?;
        let current_message_id: Option<String> = r.opt_js(|r| r.string16())?;
        let max_chars: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::build_prior_transcript_bootstrap(thread, current_message_id, max_chars)?;
        wire::write_value(w, out, &S_BUILD_PRIOR_TRANSCRIPT_BOOTSTRAP_OUT)?;
        Ok(())
    })
}

static S_BUILD_FORK_BOOTSTRAP_TEXT_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_FORK_BOOTSTRAP_TEXT_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `buildForkBootstrapText` over the binary wire.
#[napi(js_name = "buildForkBootstrapTextWire")]
pub fn build_fork_bootstrap_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Value = wire::read_value(r, &S_BUILD_FORK_BOOTSTRAP_TEXT_0)?;
        let max_chars: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::build_fork_bootstrap_text(thread, max_chars)?;
        wire::write_value(w, out, &S_BUILD_FORK_BOOTSTRAP_TEXT_OUT)?;
        Ok(())
    })
}

static S_CLAUDE_HISTORICAL_SESSION_CHILD_ENVIRONMENT_0: wire::Schema = wire::Schema::Dyn;
static S_CLAUDE_HISTORICAL_SESSION_CHILD_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `claudeHistoricalSessionChildEnvironment` over the binary wire.
#[napi(js_name = "claudeHistoricalSessionChildEnvironmentWire")]
pub fn claude_historical_session_child_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let environment: Value =
            wire::read_value(r, &S_CLAUDE_HISTORICAL_SESSION_CHILD_ENVIRONMENT_0)?;
        let out = &port::claude_historical_session_child_environment(environment)?;
        wire::write_value(w, out, &S_CLAUDE_HISTORICAL_SESSION_CHILD_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

static S_CLAUDE_HISTORICAL_SESSION_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `claudeHistoricalSessionEnvironment` over the binary wire.
#[napi(js_name = "claudeHistoricalSessionEnvironmentWire")]
pub fn claude_historical_session_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider_options: Option<Value> = wire::read_dyn_opt(r)?;
        let input: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::claude_historical_session_environment(provider_options, input)?;
        wire::write_value(w, out, &S_CLAUDE_HISTORICAL_SESSION_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_IMPORTED_THREAD_PROVIDER_OPTIONS_FOR_SETTINGS_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_IMPORTED_THREAD_PROVIDER_OPTIONS_FOR_SETTINGS_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_IMPORTED_THREAD_PROVIDER_OPTIONS_FOR_SETTINGS_OUT: wire::Schema =
    wire::Schema::Dyn;

/// `resolveImportedThreadProviderOptionsForSettings` over the binary wire.
#[napi(js_name = "resolveImportedThreadProviderOptionsForSettingsWire")]
pub fn resolve_imported_thread_provider_options_for_settings_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: Value = wire::read_value(
            r,
            &S_RESOLVE_IMPORTED_THREAD_PROVIDER_OPTIONS_FOR_SETTINGS_0,
        )?;
        let model_selection: Value = wire::read_value(
            r,
            &S_RESOLVE_IMPORTED_THREAD_PROVIDER_OPTIONS_FOR_SETTINGS_1,
        )?;
        let out = &port::resolve_imported_thread_provider_options_for_settings(
            settings,
            model_selection,
        )?;
        wire::write_value(
            w,
            out,
            &S_RESOLVE_IMPORTED_THREAD_PROVIDER_OPTIONS_FOR_SETTINGS_OUT,
        )?;
        Ok(())
    })
}

static S_MAP_CODEX_SNAPSHOT_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `mapCodexSnapshotMessages` over the binary wire.
#[napi(js_name = "mapCodexSnapshotMessagesWire")]
pub fn map_codex_snapshot_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_MAP_CODEX_SNAPSHOT_MESSAGES_0)?;
        let out = &port::map_codex_snapshot_messages(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_MAP_CLAUDE_SESSION_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `mapClaudeSessionMessages` over the binary wire.
#[napi(js_name = "mapClaudeSessionMessagesWire")]
pub fn map_claude_session_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_MAP_CLAUDE_SESSION_MESSAGES_0)?;
        let out = &port::map_claude_session_messages(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_MAP_OPEN_CODE_SNAPSHOT_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `mapOpenCodeSnapshotMessages` over the binary wire.
#[napi(js_name = "mapOpenCodeSnapshotMessagesWire")]
pub fn map_open_code_snapshot_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_MAP_OPEN_CODE_SNAPSHOT_MESSAGES_0)?;
        let out = &port::map_open_code_snapshot_messages(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_MAP_FACTORY_SNAPSHOT_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `mapFactorySnapshotMessages` over the binary wire.
#[napi(js_name = "mapFactorySnapshotMessagesWire")]
pub fn map_factory_snapshot_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_MAP_FACTORY_SNAPSHOT_MESSAGES_0)?;
        let out = &port::map_factory_snapshot_messages(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_MAP_OMP_SNAPSHOT_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `mapOmpSnapshotMessages` over the binary wire.
#[napi(js_name = "mapOmpSnapshotMessagesWire")]
pub fn map_omp_snapshot_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_MAP_OMP_SNAPSHOT_MESSAGES_0)?;
        let out = &port::map_omp_snapshot_messages(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizeWindowsImportPath` over the binary wire.
#[napi(js_name = "normalizeWindowsImportPathWire")]
pub fn normalize_windows_import_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::normalize_windows_import_path(value)?;
        w.string16(out);
        Ok(())
    })
}

/// `importPathIdentity` over the binary wire.
#[napi(js_name = "importPathIdentityWire")]
pub fn import_path_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::import_path_identity(value)?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_PROJECT_IMPORT_SOURCES_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PROJECT_IMPORT_SOURCES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("provider", wire::Schema::Dyn),
        ("instanceId", wire::Schema::Dyn),
        ("isDefault", wire::Schema::Bool),
        ("accountLabel", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("providerOptions", wire::Schema::Dyn),
        ("codexHomePath", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("environment", wire::Schema::Dyn),
        ("claudeEnvironment", wire::Schema::Dyn),
        ("claudeConfigDir", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `resolveProjectImportSources` over the binary wire.
#[napi(js_name = "resolveProjectImportSourcesWire")]
pub fn resolve_project_import_sources_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: Value = wire::read_value(r, &S_RESOLVE_PROJECT_IMPORT_SOURCES_0)?;
        let _providers: Vec<Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let paths: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_project_import_sources(settings, _providers, paths)?;
        wire::write_value(w, out, &S_RESOLVE_PROJECT_IMPORT_SOURCES_OUT)?;
        Ok(())
    })
}

/// `canProjectTurnModelSelectionForSession` over the binary wire.
#[napi(js_name = "canProjectTurnModelSelectionForSessionWire")]
pub fn can_project_turn_model_selection_for_session_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let session: Option<Value> = wire::read_dyn_opt(r)?;
        let requested_instance_id: String = r.string16()?;
        let out =
            &port::can_project_turn_model_selection_for_session(session, requested_instance_id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CREATE_EMPTY_READ_MODEL_OUT: wire::Schema = wire::Schema::Dyn;

/// `createEmptyReadModel` over the binary wire.
#[napi(js_name = "createEmptyReadModelWire")]
pub fn create_empty_read_model_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let now_iso: String = r.string16()?;
        let out = &port::create_empty_read_model(now_iso)?;
        wire::write_value(w, out, &S_CREATE_EMPTY_READ_MODEL_OUT)?;
        Ok(())
    })
}

static S_PROJECT_EVENT_0: wire::Schema = wire::Schema::Dyn;
static S_PROJECT_EVENT_1: wire::Schema = wire::Schema::Dyn;
static S_PROJECT_EVENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `projectEvent` over the binary wire.
#[napi(js_name = "projectEventWire")]
pub fn project_event_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Value = wire::read_value(r, &S_PROJECT_EVENT_0)?;
        let event: Value = wire::read_value(r, &S_PROJECT_EVENT_1)?;
        let out = &port::project_event(model, event)?;
        wire::write_value(w, out, &S_PROJECT_EVENT_OUT)?;
        Ok(())
    })
}

static S_SANITIZE_PROVIDER_START_OPTIONS_FOR_PERSISTENCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `sanitizeProviderStartOptionsForPersistence` over the binary wire.
#[napi(js_name = "sanitizeProviderStartOptionsForPersistenceWire")]
pub fn sanitize_provider_start_options_for_persistence_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider_options: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::sanitize_provider_start_options_for_persistence(provider_options)?;
        wire::write_value(
            w,
            out,
            &S_SANITIZE_PROVIDER_START_OPTIONS_FOR_PERSISTENCE_OUT,
        )?;
        Ok(())
    })
}

/// `readableReasoningDetail` over the binary wire.
#[napi(js_name = "readableReasoningDetailWire")]
pub fn readable_reasoning_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::readable_reasoning_detail(value)?;
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

static S_RUNTIME_PAYLOAD_RECORD_0: wire::Schema = wire::Schema::Dyn;
static S_RUNTIME_PAYLOAD_RECORD_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Dyn));

/// `runtimePayloadRecord` over the binary wire.
#[napi(js_name = "runtimePayloadRecordWire")]
pub fn runtime_payload_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_RUNTIME_PAYLOAD_RECORD_0)?;
        let out = &port::runtime_payload_record(event)?;
        wire::write_value(w, out, &S_RUNTIME_PAYLOAD_RECORD_OUT)?;
        Ok(())
    })
}

static S_RUNTIME_TURN_STATE_0: wire::Schema = wire::Schema::Dyn;

/// `runtimeTurnState` over the binary wire.
#[napi(js_name = "runtimeTurnStateWire")]
pub fn runtime_turn_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_RUNTIME_TURN_STATE_0)?;
        let out = &port::runtime_turn_state(event)?;
        w.string16(out);
        Ok(())
    })
}

static S_PROVIDER_RUNTIME_PROGRESS_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `providerRuntimeProgressKey` over the binary wire.
#[napi(js_name = "providerRuntimeProgressKeyWire")]
pub fn provider_runtime_progress_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_PROVIDER_RUNTIME_PROGRESS_KEY_0)?;
        let out = &port::provider_runtime_progress_key(event)?;
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

static S_PROVIDER_RUNTIME_COMMAND_PROGRESS_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_RUNTIME_COMMAND_PROGRESS_KEY_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `providerRuntimeCommandProgressKey` over the binary wire.
#[napi(js_name = "providerRuntimeCommandProgressKeyWire")]
pub fn provider_runtime_command_progress_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_PROVIDER_RUNTIME_COMMAND_PROGRESS_KEY_0)?;
        let out = &port::provider_runtime_command_progress_key(event)?;
        wire::write_value(w, out, &S_PROVIDER_RUNTIME_COMMAND_PROGRESS_KEY_OUT)?;
        Ok(())
    })
}

/// `coalesceProviderRuntimeProgress` over the binary wire.
#[napi(js_name = "coalesceProviderRuntimeProgressWire")]
pub fn coalesce_provider_runtime_progress_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let page: Vec<Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let out = &port::coalesce_provider_runtime_progress(page)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_PROVIDER_ACTIVITY_UPDATE_FINGERPRINT_0: wire::Schema = wire::Schema::Dyn;

/// `providerActivityUpdateFingerprint` over the binary wire.
#[napi(js_name = "providerActivityUpdateFingerprintWire")]
pub fn provider_activity_update_fingerprint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activity: Value = wire::read_value(r, &S_PROVIDER_ACTIVITY_UPDATE_FINGERPRINT_0)?;
        let out = &port::provider_activity_update_fingerprint(activity)?;
        w.string16(out);
        Ok(())
    })
}

static S_PROVIDER_ACTIVITY_UPDATE_DEDUPE_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_ACTIVITY_UPDATE_DEDUPE_KEY_1: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_ACTIVITY_UPDATE_DEDUPE_KEY_2: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_ACTIVITY_UPDATE_DEDUPE_KEY_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `providerActivityUpdateDedupeKey` over the binary wire.
#[napi(js_name = "providerActivityUpdateDedupeKeyWire")]
pub fn provider_activity_update_dedupe_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_PROVIDER_ACTIVITY_UPDATE_DEDUPE_KEY_0)?;
        let thread_id: Value = wire::read_value(r, &S_PROVIDER_ACTIVITY_UPDATE_DEDUPE_KEY_1)?;
        let activity: Value = wire::read_value(r, &S_PROVIDER_ACTIVITY_UPDATE_DEDUPE_KEY_2)?;
        let out = &port::provider_activity_update_dedupe_key(event, thread_id, activity)?;
        wire::write_value(w, out, &S_PROVIDER_ACTIVITY_UPDATE_DEDUPE_KEY_OUT)?;
        Ok(())
    })
}

static S_DETECT_CREATED_PULL_REQUEST_URLS_0: wire::Schema = wire::Schema::Dyn;

/// `detectCreatedPullRequestUrls` over the binary wire.
#[napi(js_name = "detectCreatedPullRequestUrlsWire")]
pub fn detect_created_pull_request_urls_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: Value = wire::read_value(r, &S_DETECT_CREATED_PULL_REQUEST_URLS_0)?;
        let out = &port::detect_created_pull_request_urls(payload)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_BUILD_QUIT_RESUME_RECORD_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_QUIT_RESUME_RECORD_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildQuitResumeRecord` over the binary wire.
#[napi(js_name = "buildQuitResumeRecordWire")]
pub fn build_quit_resume_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_BUILD_QUIT_RESUME_RECORD_0)?;
        let out = &port::build_quit_resume_record(input)?;
        wire::write_value(w, out, &S_BUILD_QUIT_RESUME_RECORD_OUT)?;
        Ok(())
    })
}

static S_BUILD_QUIT_INTERRUPT_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_QUIT_INTERRUPT_COMMAND_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildQuitInterruptCommand` over the binary wire.
#[napi(js_name = "buildQuitInterruptCommandWire")]
pub fn build_quit_interrupt_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_BUILD_QUIT_INTERRUPT_COMMAND_0)?;
        let out = &port::build_quit_interrupt_command(input)?;
        wire::write_value(w, out, &S_BUILD_QUIT_INTERRUPT_COMMAND_OUT)?;
        Ok(())
    })
}

static S_PLAN_QUIT_RESUME_TURNS_0: wire::Schema = wire::Schema::Dyn;
static S_PLAN_QUIT_RESUME_TURNS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("commands", wire::Schema::List(&wire::Schema::Dyn)),
    (
        "skipped",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("threadId", wire::Schema::Dyn),
            ("reason", wire::Schema::Dyn),
        ])),
    ),
]);

/// `planQuitResumeTurns` over the binary wire.
#[napi(js_name = "planQuitResumeTurnsWire")]
pub fn plan_quit_resume_turns_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_PLAN_QUIT_RESUME_TURNS_0)?;
        let out = &port::plan_quit_resume_turns(input)?;
        wire::write_value(w, out, &S_PLAN_QUIT_RESUME_TURNS_OUT)?;
        Ok(())
    })
}

static S_PERSIST_QUIT_RESUME_RECORD_0: wire::Schema = wire::Schema::Dyn;
static S_PERSIST_QUIT_RESUME_RECORD_OUT: wire::Schema = wire::Schema::Dyn;

/// `persistQuitResumeRecord` over the binary wire.
#[napi(js_name = "persistQuitResumeRecordWire")]
pub fn persist_quit_resume_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_PERSIST_QUIT_RESUME_RECORD_0)?;
        let out = &port::persist_quit_resume_record(input)?;
        wire::write_value(w, out, &S_PERSIST_QUIT_RESUME_RECORD_OUT)?;
        Ok(())
    })
}

static S_CLEAR_QUIT_RESUME_RECORD_OUT: wire::Schema = wire::Schema::Dyn;

/// `clearQuitResumeRecord` over the binary wire.
#[napi(js_name = "clearQuitResumeRecordWire")]
pub fn clear_quit_resume_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let _path: String = r.string16()?;
        let out = &port::clear_quit_resume_record(_path)?;
        wire::write_value(w, out, &S_CLEAR_QUIT_RESUME_RECORD_OUT)?;
        Ok(())
    })
}

static S_PERSIST_CRASH_RESUME_RECORD_0: wire::Schema = wire::Schema::Dyn;
static S_PERSIST_CRASH_RESUME_RECORD_OUT: wire::Schema = wire::Schema::Dyn;

/// `persistCrashResumeRecord` over the binary wire.
#[napi(js_name = "persistCrashResumeRecordWire")]
pub fn persist_crash_resume_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_PERSIST_CRASH_RESUME_RECORD_0)?;
        let out = &port::persist_crash_resume_record(input)?;
        wire::write_value(w, out, &S_PERSIST_CRASH_RESUME_RECORD_OUT)?;
        Ok(())
    })
}

static S_READ_QUIT_RESUME_RECORD_OUT: wire::Schema = wire::Schema::Dyn;

/// `readQuitResumeRecord` over the binary wire.
#[napi(js_name = "readQuitResumeRecordWire")]
pub fn read_quit_resume_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let _path: String = r.string16()?;
        let out = &port::read_quit_resume_record(_path)?;
        wire::write_value(w, out, &S_READ_QUIT_RESUME_RECORD_OUT)?;
        Ok(())
    })
}

static S_CLAIM_QUIT_RESUME_RECORD_OUT: wire::Schema = wire::Schema::Dyn;

/// `claimQuitResumeRecord` over the binary wire.
#[napi(js_name = "claimQuitResumeRecordWire")]
pub fn claim_quit_resume_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let _path: String = r.string16()?;
        let out = &port::claim_quit_resume_record(_path)?;
        wire::write_value(w, out, &S_CLAIM_QUIT_RESUME_RECORD_OUT)?;
        Ok(())
    })
}

static S_RESUME_QUIT_INTERRUPTED_CHATS_0: wire::Schema = wire::Schema::Dyn;
static S_RESUME_QUIT_INTERRUPTED_CHATS_OUT: wire::Schema = wire::Schema::Dyn;

/// `resumeQuitInterruptedChats` over the binary wire.
#[napi(js_name = "resumeQuitInterruptedChatsWire")]
pub fn resume_quit_interrupted_chats_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let _claimed: Value = wire::read_value(r, &S_RESUME_QUIT_INTERRUPTED_CHATS_0)?;
        let out = &port::resume_quit_interrupted_chats(_claimed)?;
        wire::write_value(w, out, &S_RESUME_QUIT_INTERRUPTED_CHATS_OUT)?;
        Ok(())
    })
}

static S_FIND_RESTART_ORPHANED_TASKS_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_RESTART_ORPHANED_TASKS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("taskId", wire::Schema::Str16),
    ("toolUseId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("workflowTaskId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("turnId", wire::Schema::Dyn),
]));

/// `findRestartOrphanedTasks` over the binary wire.
#[napi(js_name = "findRestartOrphanedTasksWire")]
pub fn find_restart_orphaned_tasks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let activities: Value = wire::read_value(r, &S_FIND_RESTART_ORPHANED_TASKS_0)?;
        let started_before: String = r.string16()?;
        let out = &port::find_restart_orphaned_tasks(activities, started_before)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_FIND_RESTART_ORPHANED_TASKS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_DERIVE_THREAD_GIT_METADATA_PATCH_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_THREAD_GIT_METADATA_PATCH_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("branch", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("discoveredPullRequest", wire::Schema::Dyn),
        (
            "associatedWorktreePath",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        (
            "associatedWorktreeBranch",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        (
            "associatedWorktreeRef",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
    ]));

/// `deriveThreadGitMetadataPatch` over the binary wire.
#[napi(js_name = "deriveThreadGitMetadataPatchWire")]
pub fn derive_thread_git_metadata_patch_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_DERIVE_THREAD_GIT_METADATA_PATCH_0)?;
        let out = &port::derive_thread_git_metadata_patch(input)?;
        wire::write_value(w, out, &S_DERIVE_THREAD_GIT_METADATA_PATCH_OUT)?;
        Ok(())
    })
}

/// `isOversizedThreadGoal` over the binary wire.
#[napi(js_name = "isOversizedThreadGoalWire")]
pub fn is_oversized_thread_goal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let goal: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_oversized_thread_goal(goal)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `threadGoalFileName` over the binary wire.
#[napi(js_name = "threadGoalFileNameWire")]
pub fn thread_goal_file_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command_id: String = r.string16()?;
        let out = &port::thread_goal_file_name(command_id)?;
        w.string16(out);
        Ok(())
    })
}

/// `threadGoalFileReference` over the binary wire.
#[napi(js_name = "threadGoalFileReferenceWire")]
pub fn thread_goal_file_reference_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let out = &port::thread_goal_file_reference(file_path)?;
        w.string16(out);
        Ok(())
    })
}

static S_THREAD_GOAL_FILE_NAME_FROM_REFERENCE_0: wire::Schema = wire::Schema::Dyn;
static S_THREAD_GOAL_FILE_NAME_FROM_REFERENCE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `threadGoalFileNameFromReference` over the binary wire.
#[napi(js_name = "threadGoalFileNameFromReferenceWire")]
pub fn thread_goal_file_name_from_reference_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_THREAD_GOAL_FILE_NAME_FROM_REFERENCE_0)?;
        let out = &port::thread_goal_file_name_from_reference(input)?;
        wire::write_value(w, out, &S_THREAD_GOAL_FILE_NAME_FROM_REFERENCE_OUT)?;
        Ok(())
    })
}

static S_FORMAT_THREAD_LATEST_REPLY_PREVIEW_0: wire::Schema = wire::Schema::Dyn;
static S_FORMAT_THREAD_LATEST_REPLY_PREVIEW_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `formatThreadLatestReplyPreview` over the binary wire.
#[napi(js_name = "formatThreadLatestReplyPreviewWire")]
pub fn format_thread_latest_reply_preview_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: Value = wire::read_value(r, &S_FORMAT_THREAD_LATEST_REPLY_PREVIEW_0)?;
        let out = &port::format_thread_latest_reply_preview(source)?;
        wire::write_value(w, out, &S_FORMAT_THREAD_LATEST_REPLY_PREVIEW_OUT)?;
        Ok(())
    })
}

static S_SHOULD_APPLY_THREADS_PROJECTION_0: wire::Schema = wire::Schema::Dyn;

/// `shouldApplyThreadsProjection` over the binary wire.
#[napi(js_name = "shouldApplyThreadsProjectionWire")]
pub fn should_apply_threads_projection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_SHOULD_APPLY_THREADS_PROJECTION_0)?;
        let out = &port::should_apply_threads_projection(event)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_APPLY_DEFERRED_THREAD_SHELL_SUMMARY_0: wire::Schema = wire::Schema::Dyn;

/// `shouldApplyDeferredThreadShellSummary` over the binary wire.
#[napi(js_name = "shouldApplyDeferredThreadShellSummaryWire")]
pub fn should_apply_deferred_thread_shell_summary_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_SHOULD_APPLY_DEFERRED_THREAD_SHELL_SUMMARY_0)?;
        let out = &port::should_apply_deferred_thread_shell_summary(event)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_PUBLISH_THREAD_SHELL_FOR_EVENT_0: wire::Schema = wire::Schema::Dyn;

/// `shouldPublishThreadShellForEvent` over the binary wire.
#[napi(js_name = "shouldPublishThreadShellForEventWire")]
pub fn should_publish_thread_shell_for_event_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_SHOULD_PUBLISH_THREAD_SHELL_FOR_EVENT_0)?;
        let out = &port::should_publish_thread_shell_for_event(event)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PROJECT_PROVIDER_RUNTIME_ACTIVITIES_0: wire::Schema = wire::Schema::Dyn;

/// `projectProviderRuntimeActivities` over the binary wire.
#[napi(js_name = "projectProviderRuntimeActivitiesWire")]
pub fn project_provider_runtime_activities_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_PROJECT_PROVIDER_RUNTIME_ACTIVITIES_0)?;
        let session_sequence: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::project_provider_runtime_activities(event, session_sequence)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_PLAN_RESTART_TURN_RECONCILIATION_0: wire::Schema = wire::Schema::Dyn;

/// `planRestartTurnReconciliation` over the binary wire.
#[napi(js_name = "planRestartTurnReconciliationWire")]
pub fn plan_restart_turn_reconciliation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_PLAN_RESTART_TURN_RECONCILIATION_0)?;
        let out = &port::plan_restart_turn_reconciliation(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
