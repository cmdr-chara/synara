//! JS bindings for `apps/server/src/computer/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `assertComputerClipboardWriteFits`.
#[napi(js_name = "assertComputerClipboardWriteFits")]
pub fn assert_computer_clipboard_write_fits(text: String) -> napi::Result<()> {
    run(move || {
        port::assert_computer_clipboard_write_fits(text)?;
        Ok(())
    })
}

/// `intersectComputerRects`.
#[napi(js_name = "intersectComputerRects")]
pub fn intersect_computer_rects(first: String, second: String) -> napi::Result<String> {
    run(move || {
        let first = crate::js::from_json(&first)?;
        let second = crate::js::from_json(&second)?;
        let out = port::intersect_computer_rects(first, second)?;
        crate::js::to_json(&out)
    })
}

/// `clampComputerMessage`.
#[napi(js_name = "clampComputerMessage")]
pub fn clamp_computer_message(text: String, fallback: String) -> napi::Result<String> {
    run(move || {
        let out = port::clamp_computer_message(text, fallback)?;
        Ok(out)
    })
}

/// `computerBackendActionResult`.
#[napi(js_name = "computerBackendActionResult")]
pub fn computer_backend_action_result(
    computer_id: String,
    action: String,
    result: String,
) -> napi::Result<String> {
    run(move || {
        let result = crate::js::from_json(&result)?;
        let out = port::computer_backend_action_result(computer_id, action, result)?;
        crate::js::to_json(&out)
    })
}

/// `assertDesktopOperationAdmission`.
#[napi(js_name = "assertDesktopOperationAdmission")]
pub fn assert_desktop_operation_admission() -> napi::Result<()> {
    run(move || {
        port::assert_desktop_operation_admission()?;
        Ok(())
    })
}

/// `computerActivationMetadata`.
#[napi(js_name = "computerActivationMetadata")]
pub fn computer_activation_metadata(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::computer_activation_metadata(input)?;
        crate::js::to_json(&out)
    })
}

/// `readComputerAuditFileTail`.
#[napi(js_name = "readComputerAuditFileTail")]
pub async fn read_computer_audit_file_tail(path: String, max_bytes: f64) -> napi::Result<String> {
    run_async(async move {
        let out = port::read_computer_audit_file_tail(path, max_bytes).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `readComputerAuditHistory`.
#[napi(js_name = "readComputerAuditHistory")]
pub async fn read_computer_audit_history(file_path: String, input: String) -> napi::Result<String> {
    run_async(async move {
        let file_path = crate::js::from_json(&file_path)?;
        let input = crate::js::from_json(&input)?;
        let out = port::read_computer_audit_history(file_path, input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `computerAuditGatewayRequestId`.
#[napi(js_name = "computerAuditGatewayRequestId")]
pub fn computer_audit_gateway_request_id(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::computer_audit_gateway_request_id(value)?;
        crate::js::to_json(&out)
    })
}

/// `summarizeComputerAuditArgs`.
#[napi(js_name = "summarizeComputerAuditArgs")]
pub fn summarize_computer_audit_args(args: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::summarize_computer_audit_args(args)?;
        crate::js::to_json(&out)
    })
}

/// `cuaTimingLogEnabled`.
#[napi(js_name = "cuaTimingLogEnabled")]
pub fn cua_timing_log_enabled() -> napi::Result<bool> {
    run(move || {
        let out = port::cua_timing_log_enabled()?;
        Ok(out)
    })
}

/// `cuaConditionalSettleEnabled`.
#[napi(js_name = "cuaConditionalSettleEnabled")]
pub fn cua_conditional_settle_enabled() -> napi::Result<bool> {
    run(move || {
        let out = port::cua_conditional_settle_enabled()?;
        Ok(out)
    })
}

/// `cuaActionSettleMsOverride`.
#[napi(js_name = "cuaActionSettleMsOverride")]
pub fn cua_action_settle_ms_override() -> napi::Result<String> {
    run(move || {
        let out = port::cua_action_settle_ms_override()?;
        crate::js::to_json(&out)
    })
}

/// `cuaCaptureReuseEnabled`.
#[napi(js_name = "cuaCaptureReuseEnabled")]
pub fn cua_capture_reuse_enabled() -> napi::Result<bool> {
    run(move || {
        let out = port::cua_capture_reuse_enabled()?;
        Ok(out)
    })
}

/// `cuaPreviewStillMsOverride`.
#[napi(js_name = "cuaPreviewStillMsOverride")]
pub fn cua_preview_still_ms_override() -> napi::Result<String> {
    run(move || {
        let out = port::cua_preview_still_ms_override()?;
        crate::js::to_json(&out)
    })
}

/// `currentComputerCall`.
#[napi(js_name = "currentComputerCall")]
pub fn current_computer_call() -> napi::Result<String> {
    run(move || {
        let out = port::current_computer_call()?;
        crate::js::to_json(&out)
    })
}

/// `createComputerCallContext`.
#[napi(js_name = "createComputerCallContext")]
pub fn create_computer_call_context() -> napi::Result<String> {
    run(move || {
        let out = port::create_computer_call_context()?;
        crate::js::to_json(&out)
    })
}

/// `markComputerCall`.
#[napi(js_name = "markComputerCall")]
pub fn mark_computer_call(operation: String) -> napi::Result<()> {
    run(move || {
        port::mark_computer_call(operation)?;
        Ok(())
    })
}

/// `computerDenylistMatch`.
#[napi(js_name = "computerDenylistMatch")]
pub fn computer_denylist_match(identity: String) -> napi::Result<String> {
    run(move || {
        let identity = crate::js::from_json(&identity)?;
        let out = port::computer_denylist_match(identity)?;
        crate::js::to_json(&out)
    })
}

/// `registerNativeComputerElement`.
#[napi(js_name = "registerNativeComputerElement")]
pub fn register_native_computer_element(node: String, identity: String) -> napi::Result<()> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        port::register_native_computer_element(node, identity)?;
        Ok(())
    })
}

/// `computerElementRefIdentity`.
#[napi(js_name = "computerElementRefIdentity")]
pub fn computer_element_ref_identity(r#ref: String) -> napi::Result<String> {
    run(move || {
        let r#ref = crate::js::from_json(&r#ref)?;
        let out = port::computer_element_ref_identity(r#ref)?;
        crate::js::to_json(&out)
    })
}

/// `bindComputerTargetRef`.
#[napi(js_name = "bindComputerTargetRef")]
pub fn bind_computer_target_ref(target: String, r#ref: String) -> napi::Result<String> {
    run(move || {
        let target = crate::js::from_json(&target)?;
        let r#ref = crate::js::from_json(&r#ref)?;
        let out = port::bind_computer_target_ref(target, r#ref)?;
        crate::js::to_json(&out)
    })
}

/// `observedComputerTargetNode`.
#[napi(js_name = "observedComputerTargetNode")]
pub fn observed_computer_target_node(target: String) -> napi::Result<String> {
    run(move || {
        let target = crate::js::from_json(&target)?;
        let out = port::observed_computer_target_node(target)?;
        crate::js::to_json(&out)
    })
}

/// `decodeResyncRequest`.
#[napi(js_name = "decodeResyncRequest")]
pub fn decode_resync_request(message: String) -> napi::Result<String> {
    run(move || {
        let message = crate::js::from_json(&message)?;
        let out = port::decode_resync_request(message)?;
        crate::js::to_json(&out)
    })
}

/// `windowsCoveringPoint`.
#[napi(js_name = "windowsCoveringPoint")]
pub fn windows_covering_point(
    windows: String,
    window_id: String,
    point: String,
) -> napi::Result<String> {
    run(move || {
        let windows = crate::js::from_json(&windows)?;
        let point = crate::js::from_json(&point)?;
        let out = port::windows_covering_point(windows, window_id, point)?;
        crate::js::to_json(&out)
    })
}

/// `topmostWindowAtPoint`.
#[napi(js_name = "topmostWindowAtPoint")]
pub fn topmost_window_at_point(windows: String, point: String) -> napi::Result<String> {
    run(move || {
        let windows = crate::js::from_json(&windows)?;
        let point = crate::js::from_json(&point)?;
        let out = port::topmost_window_at_point(windows, point)?;
        crate::js::to_json(&out)
    })
}

/// `rectContainsPoint`.
#[napi(js_name = "rectContainsPoint")]
pub fn rect_contains_point(rect: String, point: String) -> napi::Result<bool> {
    run(move || {
        let rect = crate::js::from_json(&rect)?;
        let point = crate::js::from_json(&point)?;
        let out = port::rect_contains_point(rect, point)?;
        Ok(out)
    })
}

/// `requireWindowBounds`.
#[napi(js_name = "requireWindowBounds")]
pub fn require_window_bounds(window: String, action: String) -> napi::Result<String> {
    run(move || {
        let window = crate::js::from_json(&window)?;
        let out = port::require_window_bounds(window, action)?;
        crate::js::to_json(&out)
    })
}

/// `responsibleDesktopBundleId`.
#[napi(js_name = "responsibleDesktopBundleId")]
pub fn responsible_desktop_bundle_id(
    env: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::responsible_desktop_bundle_id(env)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `computerSetupSignal`.
#[napi(js_name = "computerSetupSignal")]
pub fn computer_setup_signal(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::computer_setup_signal(input)?;
        crate::js::to_json(&out)
    })
}

/// `computerSetupToolNote`.
#[napi(js_name = "computerSetupToolNote")]
pub fn computer_setup_tool_note(signal: String) -> napi::Result<String> {
    run(move || {
        let signal = crate::js::from_json(&signal)?;
        let out = port::computer_setup_tool_note(signal)?;
        Ok(out)
    })
}

/// `cuaMaskedActivationEnabled`.
#[napi(js_name = "cuaMaskedActivationEnabled")]
pub fn cua_masked_activation_enabled() -> napi::Result<bool> {
    run(move || {
        let out = port::cua_masked_activation_enabled()?;
        Ok(out)
    })
}

/// `cuaMaskedActivationOptIn`.
#[napi(js_name = "cuaMaskedActivationOptIn")]
pub fn cua_masked_activation_opt_in() -> napi::Result<String> {
    run(move || {
        let out = port::cua_masked_activation_opt_in()?;
        crate::js::to_json(&out)
    })
}

/// `maskedActivationOptedIn`.
#[napi(js_name = "maskedActivationOptedIn")]
pub fn masked_activation_opted_in(opt_in: String, bundle_id: String) -> napi::Result<bool> {
    run(move || {
        let opt_in = crate::js::from_json(&opt_in)?;
        let bundle_id = crate::js::from_json(&bundle_id)?;
        let out = port::masked_activation_opted_in(opt_in, bundle_id)?;
        Ok(out)
    })
}

/// `messageDesignatesComputerSpaces`.
#[napi(js_name = "messageDesignatesComputerSpaces")]
pub fn message_designates_computer_spaces(text: String) -> napi::Result<Vec<f64>> {
    run(move || {
        let out = port::message_designates_computer_spaces(text)?;
        Ok(out)
    })
}

/// `computerSpaceDesignationForMessages`.
#[napi(js_name = "computerSpaceDesignationForMessages")]
pub fn computer_space_designation_for_messages(messages: String) -> napi::Result<Vec<f64>> {
    run(move || {
        let messages = crate::js::from_json(&messages)?;
        let out = port::computer_space_designation_for_messages(messages)?;
        Ok(out)
    })
}

/// `latestUserAuthoredMessage`.
#[napi(js_name = "latestUserAuthoredMessage")]
pub fn latest_user_authored_message(messages: String) -> napi::Result<String> {
    run(move || {
        let messages = crate::js::from_json(&messages)?;
        let out = port::latest_user_authored_message(messages)?;
        crate::js::to_json(&out)
    })
}

/// `currentComputerTask`.
#[napi(js_name = "currentComputerTask")]
pub fn current_computer_task() -> napi::Result<String> {
    run(move || {
        let out = port::current_computer_task()?;
        crate::js::to_json(&out)
    })
}

/// `isModelDesktopObservationActive`.
#[napi(js_name = "isModelDesktopObservationActive")]
pub fn is_model_desktop_observation_active() -> napi::Result<bool> {
    run(move || {
        let out = port::is_model_desktop_observation_active()?;
        Ok(out)
    })
}

/// `startComputerTurnTiming`.
#[napi(js_name = "startComputerTurnTiming")]
pub fn start_computer_turn_timing(thread_id: String, turn_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::start_computer_turn_timing(thread_id, turn_id)?;
        crate::js::to_json(&out)
    })
}

/// `endComputerTurnTiming`.
#[napi(js_name = "endComputerTurnTiming")]
pub fn end_computer_turn_timing(thread_id: String, turn_id: String) -> napi::Result<String> {
    run(move || {
        let turn_id = crate::js::from_json(&turn_id)?;
        let out = port::end_computer_turn_timing(thread_id, turn_id)?;
        crate::js::to_json(&out)
    })
}

/// `beginComputerTurnCall`.
#[napi(js_name = "beginComputerTurnCall")]
pub fn begin_computer_turn_call(
    thread_id: String,
    turn_id: String,
    kind: String,
) -> napi::Result<String> {
    run(move || {
        let turn_id = crate::js::from_json(&turn_id)?;
        let out = port::begin_computer_turn_call(thread_id, turn_id, kind)?;
        crate::js::to_json(&out)
    })
}

/// `messageRequestsVisibleUse`.
#[napi(js_name = "messageRequestsVisibleUse")]
pub fn message_requests_visible_use(text: String, context: String) -> napi::Result<bool> {
    run(move || {
        let context = crate::js::from_json(&context)?;
        let out = port::message_requests_visible_use(text, context)?;
        Ok(out)
    })
}

/// `computerForegroundScopeChangedSince`.
#[napi(js_name = "computerForegroundScopeChangedSince")]
pub fn computer_foreground_scope_changed_since(
    messages: String,
    last_message_id: String,
) -> napi::Result<bool> {
    run(move || {
        let messages = crate::js::from_json(&messages)?;
        let last_message_id = crate::js::from_json(&last_message_id)?;
        let out = port::computer_foreground_scope_changed_since(messages, last_message_id)?;
        Ok(out)
    })
}

/// `computerForegroundAuthorizationForMessages`.
#[napi(js_name = "computerForegroundAuthorizationForMessages")]
pub fn computer_foreground_authorization_for_messages(
    messages: String,
    context: String,
) -> napi::Result<String> {
    run(move || {
        let messages = crate::js::from_json(&messages)?;
        let context = crate::js::from_json(&context)?;
        let out = port::computer_foreground_authorization_for_messages(messages, context)?;
        crate::js::to_json(&out)
    })
}

/// `cuaSpaceInventory`.
#[napi(js_name = "cuaSpaceInventory")]
pub fn cua_space_inventory(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::cua_space_inventory(value)?;
        crate::js::to_json(&out)
    })
}

/// `cursorToolActivity`.
#[napi(js_name = "cursorToolActivity")]
pub fn cursor_tool_activity(tool: String) -> napi::Result<String> {
    run(move || {
        let out = port::cursor_tool_activity(tool)?;
        Ok(out)
    })
}

/// `cursorRuntimeActivity`.
#[napi(js_name = "cursorRuntimeActivity")]
pub fn cursor_runtime_activity(
    event: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::cursor_runtime_activity(event)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `screenshotPointToDesktop`.
#[napi(js_name = "screenshotPointToDesktop")]
pub fn screenshot_point_to_desktop(frame: String, x: f64, y: f64) -> napi::Result<String> {
    run(move || {
        let frame = crate::js::from_json(&frame)?;
        let out = port::screenshot_point_to_desktop(frame, x, y)?;
        crate::js::to_json(&out)
    })
}

/// `screenshotRectToDesktop`.
#[napi(js_name = "screenshotRectToDesktop")]
pub fn screenshot_rect_to_desktop(frame: String, rect: String) -> napi::Result<String> {
    run(move || {
        let frame = crate::js::from_json(&frame)?;
        let rect = crate::js::from_json(&rect)?;
        let out = port::screenshot_rect_to_desktop(frame, rect)?;
        crate::js::to_json(&out)
    })
}

/// `screenshotDeltaToDesktop`.
#[napi(js_name = "screenshotDeltaToDesktop")]
pub fn screenshot_delta_to_desktop(
    frame: String,
    delta_x: f64,
    delta_y: f64,
) -> napi::Result<String> {
    run(move || {
        let frame = crate::js::from_json(&frame)?;
        let out = port::screenshot_delta_to_desktop(frame, delta_x, delta_y)?;
        crate::js::to_json(&out)
    })
}

/// `decodePngLuma`.
#[napi(js_name = "decodePngLuma")]
pub async fn decode_png_luma(bytes: String) -> napi::Result<String> {
    run_async(async move {
        let bytes = crate::js::from_json(&bytes)?;
        let out = port::decode_png_luma(bytes).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `estimateVerticalTravel`.
#[napi(js_name = "estimateVerticalTravel")]
pub fn estimate_vertical_travel(
    before: String,
    after: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let before = crate::js::from_json(&before)?;
        let after = crate::js::from_json(&after)?;
        let options = crate::js::from_json(&options)?;
        let out = port::estimate_vertical_travel(before, after, options)?;
        crate::js::to_json(&out)
    })
}

/// `resolveStillIntervalMs`.
#[napi(js_name = "resolveStillIntervalMs")]
pub fn resolve_still_interval_ms(interval_ms: Option<f64>) -> napi::Result<f64> {
    run(move || {
        let out = port::resolve_still_interval_ms(interval_ms)?;
        Ok(out)
    })
}

/// `normalizeLabelSpaces`.
#[napi(js_name = "normalizeLabelSpaces")]
pub fn normalize_label_spaces(label: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_label_spaces(label)?;
        Ok(out)
    })
}

/// `candidateDescriptions`.
#[napi(js_name = "candidateDescriptions")]
pub fn candidate_descriptions(nodes: String) -> napi::Result<String> {
    run(move || {
        let nodes = crate::js::from_json(&nodes)?;
        let out = port::candidate_descriptions(nodes)?;
        crate::js::to_json(&out)
    })
}

/// `describeTarget`.
#[napi(js_name = "describeTarget")]
pub fn describe_target(target: String) -> napi::Result<String> {
    run(move || {
        let target = crate::js::from_json(&target)?;
        let out = port::describe_target(target)?;
        Ok(out)
    })
}

/// `activationPointForNode`.
#[napi(js_name = "activationPointForNode")]
pub fn activation_point_for_node(node: String) -> napi::Result<String> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let out = port::activation_point_for_node(node)?;
        crate::js::to_json(&out)
    })
}

/// `computerTargetCandidates`.
#[napi(js_name = "computerTargetCandidates")]
pub fn computer_target_candidates(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::computer_target_candidates(root)?;
        crate::js::to_json(&out)
    })
}

/// `resolveComputerSemanticTarget`.
#[napi(js_name = "resolveComputerSemanticTarget")]
pub fn resolve_computer_semantic_target(
    root: String,
    target: String,
    allow_offscreen: Option<bool>,
) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let target = crate::js::from_json(&target)?;
        let out = port::resolve_computer_semantic_target(root, target, allow_offscreen)?;
        crate::js::to_json(&out)
    })
}

/// `resolveComputerUniqueTextTarget`.
#[napi(js_name = "resolveComputerUniqueTextTarget")]
pub fn resolve_computer_unique_text_target(
    root: String,
    window_id: String,
    allow_offscreen: Option<bool>,
) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::resolve_computer_unique_text_target(root, window_id, allow_offscreen)?;
        crate::js::to_json(&out)
    })
}

/// `resolveComputerWindowTarget`.
#[napi(js_name = "resolveComputerWindowTarget")]
pub fn resolve_computer_window_target(root: String, window_id: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::resolve_computer_window_target(root, window_id)?;
        crate::js::to_json(&out)
    })
}

/// `resolveComputerPoint`.
#[napi(js_name = "resolveComputerPoint")]
pub fn resolve_computer_point(target: String, screen_size: String) -> napi::Result<String> {
    run(move || {
        let target = crate::js::from_json(&target)?;
        let screen_size = crate::js::from_json(&screen_size)?;
        let out = port::resolve_computer_point(target, screen_size)?;
        crate::js::to_json(&out)
    })
}

/// `actionableElements`.
#[napi(js_name = "actionableElements")]
pub fn actionable_elements(root: String, filter: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let filter = crate::js::from_json(&filter)?;
        let out = port::actionable_elements(root, filter)?;
        crate::js::to_json(&out)
    })
}

/// `diffActionableElements`.
#[napi(js_name = "diffActionableElements")]
pub fn diff_actionable_elements(before: String, after: String) -> napi::Result<String> {
    run(move || {
        let before = crate::js::from_json(&before)?;
        let after = crate::js::from_json(&after)?;
        let out = port::diff_actionable_elements(before, after)?;
        crate::js::to_json(&out)
    })
}

/// `describeComputerUiTree`.
#[napi(js_name = "describeComputerUiTree")]
pub fn describe_computer_ui_tree(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::describe_computer_ui_tree(root)?;
        Ok(out)
    })
}

/// `clampNodeText`.
#[napi(js_name = "clampNodeText")]
pub fn clamp_node_text(text: String, max_length: f64) -> napi::Result<String> {
    run(move || {
        let out = port::clamp_node_text(text, max_length)?;
        Ok(out)
    })
}

/// `clampTextToLength`.
#[napi(js_name = "clampTextToLength")]
pub fn clamp_text_to_length_pub(text: String, max_length: f64) -> napi::Result<String> {
    run(move || {
        let out = port::clamp_text_to_length_pub(text, max_length)?;
        Ok(out)
    })
}

/// Class `ComputerControlState`.
#[napi(js_name = "ComputerControlState")]
pub struct ComputerControlState {
    inner: port::ComputerControlState,
}

#[napi]
impl ComputerControlState {
    #[napi(constructor)]
    pub fn new(file_path: Option<String>) -> napi::Result<Self> {
        run(move || {
            let out = port::ComputerControlState::new(file_path)?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "get")]
    pub fn get(&mut self, thread_id: String) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let out = inner.get(thread_id)?;
            crate::js::to_json(&out)
        })
    }

    #[napi(js_name = "allows")]
    pub fn allows(&mut self, thread_id: String, generation: f64) -> napi::Result<bool> {
        let inner = &mut self.inner;
        run(move || {
            let out = inner.allows(thread_id, generation)?;
            Ok(out)
        })
    }

    #[napi(js_name = "set")]
    pub fn set(&mut self, thread_id: String, disabled: bool) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.set(thread_id, disabled)?;
            Ok(())
        })
    }

    #[napi(js_name = "recordChatIntent")]
    pub fn record_chat_intent(
        &mut self,
        thread_id: String,
        enabled: bool,
        generation: f64,
    ) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.record_chat_intent(thread_id, enabled, generation)?;
            Ok(())
        })
    }
}

/// Class `ScreenshotFrameRegistry`.
#[napi(js_name = "ScreenshotFrameRegistry")]
pub struct ScreenshotFrameRegistry {
    inner: port::ScreenshotFrameRegistry,
}

#[napi]
impl ScreenshotFrameRegistry {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        run(move || {
            let out = port::ScreenshotFrameRegistry::new()?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "record")]
    pub fn record(
        &mut self,
        thread_id: String,
        screenshot: String,
        window_id: String,
    ) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let screenshot = crate::js::from_json(&screenshot)?;
            let window_id = crate::js::from_json(&window_id)?;
            let out = inner.record(thread_id, screenshot, window_id)?;
            crate::js::to_json(&out)
        })
    }

    #[napi(js_name = "latest")]
    pub fn latest(&mut self, thread_id: String) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let out = inner.latest(thread_id)?;
            crate::js::to_json(&out)
        })
    }

    #[napi(js_name = "matchLatest")]
    pub fn match_latest(
        &mut self,
        thread_id: String,
        screenshot: String,
        window_id: String,
    ) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let screenshot = crate::js::from_json(&screenshot)?;
            let window_id = crate::js::from_json(&window_id)?;
            let out = inner.match_latest(thread_id, screenshot, window_id)?;
            crate::js::to_json(&out)
        })
    }

    #[napi(js_name = "resolve")]
    pub fn resolve(&mut self, thread_id: String, screenshot_id: String) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let screenshot_id = crate::js::from_json(&screenshot_id)?;
            let out = inner.resolve(thread_id, screenshot_id)?;
            crate::js::to_json(&out)
        })
    }
}

/// Class `ScrollGearingStore`.
#[napi(js_name = "ScrollGearingStore")]
pub struct ScrollGearingStore {
    inner: port::ScrollGearingStore,
}

#[napi]
impl ScrollGearingStore {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        run(move || {
            let out = port::ScrollGearingStore::new()?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "gearing")]
    pub fn gearing(&mut self, key: String) -> napi::Result<f64> {
        let inner = &mut self.inner;
        run(move || {
            let key = crate::js::from_json(&key)?;
            let out = inner.gearing(key)?;
            Ok(out)
        })
    }

    #[napi(js_name = "has")]
    pub fn has(&mut self, key: String) -> napi::Result<bool> {
        let inner = &mut self.inner;
        run(move || {
            let key = crate::js::from_json(&key)?;
            let out = inner.has(key)?;
            Ok(out)
        })
    }

    #[napi(js_name = "plan")]
    pub fn plan(
        &mut self,
        key: String,
        requested: f64,
        fallback: Option<f64>,
    ) -> napi::Result<f64> {
        let inner = &mut self.inner;
        run(move || {
            let key = crate::js::from_json(&key)?;
            let out = inner.plan(key, requested, fallback)?;
            Ok(out)
        })
    }

    #[napi(js_name = "learn")]
    pub fn learn(&mut self, key: String, injected: f64, traveled: f64) -> napi::Result<bool> {
        let inner = &mut self.inner;
        run(move || {
            let key = crate::js::from_json(&key)?;
            let out = inner.learn(key, injected, traveled)?;
            Ok(out)
        })
    }
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

/// `assertComputerClipboardWriteFits` over the binary wire.
#[napi(js_name = "assertComputerClipboardWriteFitsWire")]
pub fn assert_computer_clipboard_write_fits_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let text: String = r.string16()?;
        let () = port::assert_computer_clipboard_write_fits(text)?;
        Ok(())
    })
}

static S_INTERSECT_COMPUTER_RECTS_0: wire::Schema = wire::Schema::Dyn;
static S_INTERSECT_COMPUTER_RECTS_1: wire::Schema = wire::Schema::Dyn;

/// `intersectComputerRects` over the binary wire.
#[napi(js_name = "intersectComputerRectsWire")]
pub fn intersect_computer_rects_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let first: Value = wire::read_value(r, &S_INTERSECT_COMPUTER_RECTS_0)?;
        let second: Value = wire::read_value(r, &S_INTERSECT_COMPUTER_RECTS_1)?;
        let out = &port::intersect_computer_rects(first, second)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `clampComputerMessage` over the binary wire.
#[napi(js_name = "clampComputerMessageWire")]
pub fn clamp_computer_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let fallback: String = r.string16()?;
        let out = &port::clamp_computer_message(text, fallback)?;
        w.string16(out);
        Ok(())
    })
}

static S_COMPUTER_BACKEND_ACTION_RESULT_OUT: wire::Schema = wire::Schema::Dyn;

/// `computerBackendActionResult` over the binary wire.
#[napi(js_name = "computerBackendActionResultWire")]
pub fn computer_backend_action_result_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let computer_id: String = r.string16()?;
        let action: String = r.string16()?;
        let result: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::computer_backend_action_result(computer_id, action, result)?;
        wire::write_value(w, out, &S_COMPUTER_BACKEND_ACTION_RESULT_OUT)?;
        Ok(())
    })
}

/// `assertDesktopOperationAdmission` over the binary wire.
#[napi(js_name = "assertDesktopOperationAdmissionWire")]
pub fn assert_desktop_operation_admission_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::assert_desktop_operation_admission()?;
        Ok(())
    })
}

static S_COMPUTER_ACTIVATION_METADATA_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_ACTIVATION_METADATA_OUT: wire::Schema = wire::Schema::Dyn;

/// `computerActivationMetadata` over the binary wire.
#[napi(js_name = "computerActivationMetadataWire")]
pub fn computer_activation_metadata_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_COMPUTER_ACTIVATION_METADATA_0)?;
        let out = &port::computer_activation_metadata(input)?;
        wire::write_value(w, out, &S_COMPUTER_ACTIVATION_METADATA_OUT)?;
        Ok(())
    })
}

static S_COMPUTER_AUDIT_GATEWAY_REQUEST_ID_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_AUDIT_GATEWAY_REQUEST_ID_OUT: wire::Schema = wire::Schema::Obj(&[(
    "gatewayRequestId",
    wire::Schema::OptJs(&wire::Schema::Str16),
)]);

/// `computerAuditGatewayRequestId` over the binary wire.
#[napi(js_name = "computerAuditGatewayRequestIdWire")]
pub fn computer_audit_gateway_request_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_COMPUTER_AUDIT_GATEWAY_REQUEST_ID_0)?;
        let out = &port::computer_audit_gateway_request_id(value)?;
        wire::write_value(w, out, &S_COMPUTER_AUDIT_GATEWAY_REQUEST_ID_OUT)?;
        Ok(())
    })
}

static S_SUMMARIZE_COMPUTER_AUDIT_ARGS_0: wire::Schema = wire::Schema::Dyn;
static S_SUMMARIZE_COMPUTER_AUDIT_ARGS_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Dyn);

/// `summarizeComputerAuditArgs` over the binary wire.
#[napi(js_name = "summarizeComputerAuditArgsWire")]
pub fn summarize_computer_audit_args_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: Value = wire::read_value(r, &S_SUMMARIZE_COMPUTER_AUDIT_ARGS_0)?;
        let out = &port::summarize_computer_audit_args(args)?;
        wire::write_value(w, out, &S_SUMMARIZE_COMPUTER_AUDIT_ARGS_OUT)?;
        Ok(())
    })
}

/// `cuaTimingLogEnabled` over the binary wire.
#[napi(js_name = "cuaTimingLogEnabledWire")]
pub fn cua_timing_log_enabled_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::cua_timing_log_enabled()?;
        w.bool(*out);
        Ok(())
    })
}

/// `cuaConditionalSettleEnabled` over the binary wire.
#[napi(js_name = "cuaConditionalSettleEnabledWire")]
pub fn cua_conditional_settle_enabled_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::cua_conditional_settle_enabled()?;
        w.bool(*out);
        Ok(())
    })
}

static S_CUA_ACTION_SETTLE_MS_OVERRIDE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Num);

/// `cuaActionSettleMsOverride` over the binary wire.
#[napi(js_name = "cuaActionSettleMsOverrideWire")]
pub fn cua_action_settle_ms_override_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::cua_action_settle_ms_override()?;
        wire::write_value(w, out, &S_CUA_ACTION_SETTLE_MS_OVERRIDE_OUT)?;
        Ok(())
    })
}

/// `cuaCaptureReuseEnabled` over the binary wire.
#[napi(js_name = "cuaCaptureReuseEnabledWire")]
pub fn cua_capture_reuse_enabled_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::cua_capture_reuse_enabled()?;
        w.bool(*out);
        Ok(())
    })
}

static S_CUA_PREVIEW_STILL_MS_OVERRIDE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Num);

/// `cuaPreviewStillMsOverride` over the binary wire.
#[napi(js_name = "cuaPreviewStillMsOverrideWire")]
pub fn cua_preview_still_ms_override_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::cua_preview_still_ms_override()?;
        wire::write_value(w, out, &S_CUA_PREVIEW_STILL_MS_OVERRIDE_OUT)?;
        Ok(())
    })
}

static S_CURRENT_COMPUTER_CALL_OUT: wire::Schema = wire::Schema::Dyn;

/// `currentComputerCall` over the binary wire.
#[napi(js_name = "currentComputerCallWire")]
pub fn current_computer_call_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::current_computer_call()?;
        wire::write_value(w, out, &S_CURRENT_COMPUTER_CALL_OUT)?;
        Ok(())
    })
}

static S_CREATE_COMPUTER_CALL_CONTEXT_OUT: wire::Schema = wire::Schema::Dyn;

/// `createComputerCallContext` over the binary wire.
#[napi(js_name = "createComputerCallContextWire")]
pub fn create_computer_call_context_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::create_computer_call_context()?;
        wire::write_value(w, out, &S_CREATE_COMPUTER_CALL_CONTEXT_OUT)?;
        Ok(())
    })
}

/// `markComputerCall` over the binary wire.
#[napi(js_name = "markComputerCallWire")]
pub fn mark_computer_call_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let operation: String = r.string16()?;
        let () = port::mark_computer_call(operation)?;
        Ok(())
    })
}

static S_COMPUTER_DENYLIST_MATCH_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_DENYLIST_MATCH_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("app", wire::Schema::Str16),
    ("matched", wire::Schema::Str16),
]));

/// `computerDenylistMatch` over the binary wire.
#[napi(js_name = "computerDenylistMatchWire")]
pub fn computer_denylist_match_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let identity: Value = wire::read_value(r, &S_COMPUTER_DENYLIST_MATCH_0)?;
        let out = &port::computer_denylist_match(identity)?;
        wire::write_value(w, out, &S_COMPUTER_DENYLIST_MATCH_OUT)?;
        Ok(())
    })
}

static S_REGISTER_NATIVE_COMPUTER_ELEMENT_0: wire::Schema = wire::Schema::Dyn;

/// `registerNativeComputerElement` over the binary wire.
#[napi(js_name = "registerNativeComputerElementWire")]
pub fn register_native_computer_element_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let node: Value = wire::read_value(r, &S_REGISTER_NATIVE_COMPUTER_ELEMENT_0)?;
        let identity: String = r.string16()?;
        let () = port::register_native_computer_element(node, identity)?;
        Ok(())
    })
}

static S_COMPUTER_ELEMENT_REF_IDENTITY_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_ELEMENT_REF_IDENTITY_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `computerElementRefIdentity` over the binary wire.
#[napi(js_name = "computerElementRefIdentityWire")]
pub fn computer_element_ref_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let r#ref: Value = wire::read_value(r, &S_COMPUTER_ELEMENT_REF_IDENTITY_0)?;
        let out = &port::computer_element_ref_identity(r#ref)?;
        wire::write_value(w, out, &S_COMPUTER_ELEMENT_REF_IDENTITY_OUT)?;
        Ok(())
    })
}

static S_BIND_COMPUTER_TARGET_REF_0: wire::Schema = wire::Schema::Dyn;
static S_BIND_COMPUTER_TARGET_REF_1: wire::Schema = wire::Schema::Dyn;
static S_BIND_COMPUTER_TARGET_REF_OUT: wire::Schema = wire::Schema::Dyn;

/// `bindComputerTargetRef` over the binary wire.
#[napi(js_name = "bindComputerTargetRefWire")]
pub fn bind_computer_target_ref_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let target: Value = wire::read_value(r, &S_BIND_COMPUTER_TARGET_REF_0)?;
        let r#ref: Value = wire::read_value(r, &S_BIND_COMPUTER_TARGET_REF_1)?;
        let out = &port::bind_computer_target_ref(target, r#ref)?;
        wire::write_value(w, out, &S_BIND_COMPUTER_TARGET_REF_OUT)?;
        Ok(())
    })
}

static S_OBSERVED_COMPUTER_TARGET_NODE_0: wire::Schema = wire::Schema::Dyn;
static S_OBSERVED_COMPUTER_TARGET_NODE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("role", wire::Schema::Str16),
        ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("frame", wire::Schema::Dyn),
        ("activationPoint", wire::Schema::Dyn),
        ("onScreen", wire::Schema::Bool),
        ("windowId", wire::Schema::Dyn),
        (
            "nodePath",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Num)),
        ),
        (
            "accessibilityRoot",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("editable", wire::Schema::OptJs(&wire::Schema::Bool)),
        ("truncated", wire::Schema::OptJs(&wire::Schema::Bool)),
        (
            "children",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("role", wire::Schema::Str16),
                ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("frame", wire::Schema::Dyn),
                ("activationPoint", wire::Schema::Dyn),
                ("onScreen", wire::Schema::Bool),
                ("windowId", wire::Schema::Dyn),
                ("nodePath", wire::Schema::Dyn),
                (
                    "accessibilityRoot",
                    wire::Schema::OptJs(&wire::Schema::Str16),
                ),
                ("editable", wire::Schema::OptJs(&wire::Schema::Bool)),
                ("truncated", wire::Schema::OptJs(&wire::Schema::Bool)),
                ("children", wire::Schema::List(&wire::Schema::Dyn)),
            ])),
        ),
    ]));

/// `observedComputerTargetNode` over the binary wire.
#[napi(js_name = "observedComputerTargetNodeWire")]
pub fn observed_computer_target_node_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let target: Value = wire::read_value(r, &S_OBSERVED_COMPUTER_TARGET_NODE_0)?;
        let out = &port::observed_computer_target_node(target)?;
        wire::write_value(w, out, &S_OBSERVED_COMPUTER_TARGET_NODE_OUT)?;
        Ok(())
    })
}

static S_DECODE_RESYNC_REQUEST_0: wire::Schema = wire::Schema::Dyn;
static S_DECODE_RESYNC_REQUEST_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `decodeResyncRequest` over the binary wire.
#[napi(js_name = "decodeResyncRequestWire")]
pub fn decode_resync_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: Value = wire::read_value(r, &S_DECODE_RESYNC_REQUEST_0)?;
        let out = &port::decode_resync_request(message)?;
        wire::write_value(w, out, &S_DECODE_RESYNC_REQUEST_OUT)?;
        Ok(())
    })
}

static S_WINDOWS_COVERING_POINT_0: wire::Schema = wire::Schema::Dyn;
static S_WINDOWS_COVERING_POINT_2: wire::Schema = wire::Schema::Dyn;

/// `windowsCoveringPoint` over the binary wire.
#[napi(js_name = "windowsCoveringPointWire")]
pub fn windows_covering_point_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let windows: Value = wire::read_value(r, &S_WINDOWS_COVERING_POINT_0)?;
        let window_id: String = r.string16()?;
        let point: Value = wire::read_value(r, &S_WINDOWS_COVERING_POINT_2)?;
        let out = &port::windows_covering_point(windows, window_id, point)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_TOPMOST_WINDOW_AT_POINT_0: wire::Schema = wire::Schema::Dyn;
static S_TOPMOST_WINDOW_AT_POINT_1: wire::Schema = wire::Schema::Dyn;
static S_TOPMOST_WINDOW_AT_POINT_OUT: wire::Schema = wire::Schema::Dyn;

/// `topmostWindowAtPoint` over the binary wire.
#[napi(js_name = "topmostWindowAtPointWire")]
pub fn topmost_window_at_point_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let windows: Value = wire::read_value(r, &S_TOPMOST_WINDOW_AT_POINT_0)?;
        let point: Value = wire::read_value(r, &S_TOPMOST_WINDOW_AT_POINT_1)?;
        let out = &port::topmost_window_at_point(windows, point)?;
        wire::write_value(w, out, &S_TOPMOST_WINDOW_AT_POINT_OUT)?;
        Ok(())
    })
}

static S_RECT_CONTAINS_POINT_1: wire::Schema = wire::Schema::Dyn;

/// `rectContainsPoint` over the binary wire.
#[napi(js_name = "rectContainsPointWire")]
pub fn rect_contains_point_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let rect: Option<Value> = wire::read_dyn_opt(r)?;
        let point: Value = wire::read_value(r, &S_RECT_CONTAINS_POINT_1)?;
        let out = &port::rect_contains_point(rect, point)?;
        w.bool(*out);
        Ok(())
    })
}

static S_REQUIRE_WINDOW_BOUNDS_0: wire::Schema = wire::Schema::Dyn;
static S_REQUIRE_WINDOW_BOUNDS_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireWindowBounds` over the binary wire.
#[napi(js_name = "requireWindowBoundsWire")]
pub fn require_window_bounds_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let window: Value = wire::read_value(r, &S_REQUIRE_WINDOW_BOUNDS_0)?;
        let action: String = r.string16()?;
        let out = &port::require_window_bounds(window, action)?;
        wire::write_value(w, out, &S_REQUIRE_WINDOW_BOUNDS_OUT)?;
        Ok(())
    })
}

static S_RESPONSIBLE_DESKTOP_BUNDLE_ID_0: wire::Schema = wire::Schema::Dyn;

/// `responsibleDesktopBundleId` over the binary wire.
#[napi(js_name = "responsibleDesktopBundleIdWire")]
pub fn responsible_desktop_bundle_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: Value = wire::read_value(r, &S_RESPONSIBLE_DESKTOP_BUNDLE_ID_0)?;
        let out = &port::responsible_desktop_bundle_id(env)?;
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

static S_COMPUTER_SETUP_SIGNAL_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_SETUP_SIGNAL_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("missing", wire::Schema::List(&wire::Schema::Dyn)),
    ("blocking", wire::Schema::Bool),
    ("buildSignature", wire::Schema::Dyn),
    ("bundleId", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `computerSetupSignal` over the binary wire.
#[napi(js_name = "computerSetupSignalWire")]
pub fn computer_setup_signal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_COMPUTER_SETUP_SIGNAL_0)?;
        let out = &port::computer_setup_signal(input)?;
        wire::write_value(w, out, &S_COMPUTER_SETUP_SIGNAL_OUT)?;
        Ok(())
    })
}

static S_COMPUTER_SETUP_TOOL_NOTE_0: wire::Schema = wire::Schema::Dyn;

/// `computerSetupToolNote` over the binary wire.
#[napi(js_name = "computerSetupToolNoteWire")]
pub fn computer_setup_tool_note_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let signal: Value = wire::read_value(r, &S_COMPUTER_SETUP_TOOL_NOTE_0)?;
        let out = &port::computer_setup_tool_note(signal)?;
        w.string16(out);
        Ok(())
    })
}

/// `cuaMaskedActivationEnabled` over the binary wire.
#[napi(js_name = "cuaMaskedActivationEnabledWire")]
pub fn cua_masked_activation_enabled_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::cua_masked_activation_enabled()?;
        w.bool(*out);
        Ok(())
    })
}

static S_CUA_MASKED_ACTIVATION_OPT_IN_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Str16);

/// `cuaMaskedActivationOptIn` over the binary wire.
#[napi(js_name = "cuaMaskedActivationOptInWire")]
pub fn cua_masked_activation_opt_in_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::cua_masked_activation_opt_in()?;
        wire::write_value(w, out, &S_CUA_MASKED_ACTIVATION_OPT_IN_OUT)?;
        Ok(())
    })
}

static S_MASKED_ACTIVATION_OPTED_IN_0: wire::Schema = wire::Schema::Dyn;
static S_MASKED_ACTIVATION_OPTED_IN_1: wire::Schema = wire::Schema::Dyn;

/// `maskedActivationOptedIn` over the binary wire.
#[napi(js_name = "maskedActivationOptedInWire")]
pub fn masked_activation_opted_in_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let opt_in: Value = wire::read_value(r, &S_MASKED_ACTIVATION_OPTED_IN_0)?;
        let bundle_id: Value = wire::read_value(r, &S_MASKED_ACTIVATION_OPTED_IN_1)?;
        let out = &port::masked_activation_opted_in(opt_in, bundle_id)?;
        w.bool(*out);
        Ok(())
    })
}

/// `messageDesignatesComputerSpaces` over the binary wire.
#[napi(js_name = "messageDesignatesComputerSpacesWire")]
pub fn message_designates_computer_spaces_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::message_designates_computer_spaces(text)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.f64(*x0);
        }
        Ok(())
    })
}

static S_COMPUTER_SPACE_DESIGNATION_FOR_MESSAGES_0: wire::Schema = wire::Schema::Dyn;

/// `computerSpaceDesignationForMessages` over the binary wire.
#[napi(js_name = "computerSpaceDesignationForMessagesWire")]
pub fn computer_space_designation_for_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let messages: Value = wire::read_value(r, &S_COMPUTER_SPACE_DESIGNATION_FOR_MESSAGES_0)?;
        let out = &port::computer_space_designation_for_messages(messages)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.f64(*x0);
        }
        Ok(())
    })
}

static S_LATEST_USER_AUTHORED_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_LATEST_USER_AUTHORED_MESSAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `latestUserAuthoredMessage` over the binary wire.
#[napi(js_name = "latestUserAuthoredMessageWire")]
pub fn latest_user_authored_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let messages: Value = wire::read_value(r, &S_LATEST_USER_AUTHORED_MESSAGE_0)?;
        let out = &port::latest_user_authored_message(messages)?;
        wire::write_value(w, out, &S_LATEST_USER_AUTHORED_MESSAGE_OUT)?;
        Ok(())
    })
}

static S_CURRENT_COMPUTER_TASK_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("threadId", wire::Schema::Str16),
    ("turnId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
]));

/// `currentComputerTask` over the binary wire.
#[napi(js_name = "currentComputerTaskWire")]
pub fn current_computer_task_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::current_computer_task()?;
        wire::write_value(w, out, &S_CURRENT_COMPUTER_TASK_OUT)?;
        Ok(())
    })
}

/// `isModelDesktopObservationActive` over the binary wire.
#[napi(js_name = "isModelDesktopObservationActiveWire")]
pub fn is_model_desktop_observation_active_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::is_model_desktop_observation_active()?;
        w.bool(*out);
        Ok(())
    })
}

static S_START_COMPUTER_TURN_TIMING_OUT: wire::Schema = wire::Schema::Dyn;

/// `startComputerTurnTiming` over the binary wire.
#[napi(js_name = "startComputerTurnTimingWire")]
pub fn start_computer_turn_timing_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: String = r.string16()?;
        let turn_id: String = r.string16()?;
        let out = &port::start_computer_turn_timing(thread_id, turn_id)?;
        wire::write_value(w, out, &S_START_COMPUTER_TURN_TIMING_OUT)?;
        Ok(())
    })
}

static S_END_COMPUTER_TURN_TIMING_1: wire::Schema = wire::Schema::Dyn;
static S_END_COMPUTER_TURN_TIMING_OUT: wire::Schema = wire::Schema::Dyn;

/// `endComputerTurnTiming` over the binary wire.
#[napi(js_name = "endComputerTurnTimingWire")]
pub fn end_computer_turn_timing_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: String = r.string16()?;
        let turn_id: Value = wire::read_value(r, &S_END_COMPUTER_TURN_TIMING_1)?;
        let out = &port::end_computer_turn_timing(thread_id, turn_id)?;
        wire::write_value(w, out, &S_END_COMPUTER_TURN_TIMING_OUT)?;
        Ok(())
    })
}

static S_BEGIN_COMPUTER_TURN_CALL_1: wire::Schema = wire::Schema::Dyn;
static S_BEGIN_COMPUTER_TURN_CALL_OUT: wire::Schema = wire::Schema::Dyn;

/// `beginComputerTurnCall` over the binary wire.
#[napi(js_name = "beginComputerTurnCallWire")]
pub fn begin_computer_turn_call_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: String = r.string16()?;
        let turn_id: Value = wire::read_value(r, &S_BEGIN_COMPUTER_TURN_CALL_1)?;
        let kind: String = r.string16()?;
        let out = &port::begin_computer_turn_call(thread_id, turn_id, kind)?;
        wire::write_value(w, out, &S_BEGIN_COMPUTER_TURN_CALL_OUT)?;
        Ok(())
    })
}

static S_MESSAGE_REQUESTS_VISIBLE_USE_1: wire::Schema = wire::Schema::Dyn;

/// `messageRequestsVisibleUse` over the binary wire.
#[napi(js_name = "messageRequestsVisibleUseWire")]
pub fn message_requests_visible_use_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let context: Value = wire::read_value(r, &S_MESSAGE_REQUESTS_VISIBLE_USE_1)?;
        let out = &port::message_requests_visible_use(text, context)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPUTER_FOREGROUND_SCOPE_CHANGED_SINCE_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_FOREGROUND_SCOPE_CHANGED_SINCE_1: wire::Schema = wire::Schema::Dyn;

/// `computerForegroundScopeChangedSince` over the binary wire.
#[napi(js_name = "computerForegroundScopeChangedSinceWire")]
pub fn computer_foreground_scope_changed_since_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let messages: Value = wire::read_value(r, &S_COMPUTER_FOREGROUND_SCOPE_CHANGED_SINCE_0)?;
        let last_message_id: Value =
            wire::read_value(r, &S_COMPUTER_FOREGROUND_SCOPE_CHANGED_SINCE_1)?;
        let out = &port::computer_foreground_scope_changed_since(messages, last_message_id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPUTER_FOREGROUND_AUTHORIZATION_FOR_MESSAGES_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_FOREGROUND_AUTHORIZATION_FOR_MESSAGES_1: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_FOREGROUND_AUTHORIZATION_FOR_MESSAGES_OUT: wire::Schema =
    wire::Schema::Obj(&[("userRequestedVisibleUse", wire::Schema::Bool)]);

/// `computerForegroundAuthorizationForMessages` over the binary wire.
#[napi(js_name = "computerForegroundAuthorizationForMessagesWire")]
pub fn computer_foreground_authorization_for_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let messages: Value =
            wire::read_value(r, &S_COMPUTER_FOREGROUND_AUTHORIZATION_FOR_MESSAGES_0)?;
        let context: Value =
            wire::read_value(r, &S_COMPUTER_FOREGROUND_AUTHORIZATION_FOR_MESSAGES_1)?;
        let out = &port::computer_foreground_authorization_for_messages(messages, context)?;
        wire::write_value(
            w,
            out,
            &S_COMPUTER_FOREGROUND_AUTHORIZATION_FOR_MESSAGES_OUT,
        )?;
        Ok(())
    })
}

static S_CUA_SPACE_INVENTORY_0: wire::Schema = wire::Schema::Dyn;
static S_CUA_SPACE_INVENTORY_OUT: wire::Schema = wire::Schema::Dyn;

/// `cuaSpaceInventory` over the binary wire.
#[napi(js_name = "cuaSpaceInventoryWire")]
pub fn cua_space_inventory_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_CUA_SPACE_INVENTORY_0)?;
        let out = &port::cua_space_inventory(value)?;
        wire::write_value(w, out, &S_CUA_SPACE_INVENTORY_OUT)?;
        Ok(())
    })
}

/// `cursorToolActivity` over the binary wire.
#[napi(js_name = "cursorToolActivityWire")]
pub fn cursor_tool_activity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tool: String = r.string16()?;
        let out = &port::cursor_tool_activity(tool)?;
        w.string16(out);
        Ok(())
    })
}

static S_CURSOR_RUNTIME_ACTIVITY_0: wire::Schema = wire::Schema::Dyn;

/// `cursorRuntimeActivity` over the binary wire.
#[napi(js_name = "cursorRuntimeActivityWire")]
pub fn cursor_runtime_activity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_CURSOR_RUNTIME_ACTIVITY_0)?;
        let out = &port::cursor_runtime_activity(event)?;
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

static S_SCREENSHOT_POINT_TO_DESKTOP_0: wire::Schema = wire::Schema::Dyn;
static S_SCREENSHOT_POINT_TO_DESKTOP_OUT: wire::Schema = wire::Schema::Dyn;

/// `screenshotPointToDesktop` over the binary wire.
#[napi(js_name = "screenshotPointToDesktopWire")]
pub fn screenshot_point_to_desktop_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let frame: Value = wire::read_value(r, &S_SCREENSHOT_POINT_TO_DESKTOP_0)?;
        let x: f64 = r.f64()?;
        let y: f64 = r.f64()?;
        let out = &port::screenshot_point_to_desktop(frame, x, y)?;
        wire::write_value(w, out, &S_SCREENSHOT_POINT_TO_DESKTOP_OUT)?;
        Ok(())
    })
}

static S_SCREENSHOT_RECT_TO_DESKTOP_0: wire::Schema = wire::Schema::Dyn;
static S_SCREENSHOT_RECT_TO_DESKTOP_1: wire::Schema = wire::Schema::Dyn;
static S_SCREENSHOT_RECT_TO_DESKTOP_OUT: wire::Schema = wire::Schema::Dyn;

/// `screenshotRectToDesktop` over the binary wire.
#[napi(js_name = "screenshotRectToDesktopWire")]
pub fn screenshot_rect_to_desktop_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let frame: Value = wire::read_value(r, &S_SCREENSHOT_RECT_TO_DESKTOP_0)?;
        let rect: Value = wire::read_value(r, &S_SCREENSHOT_RECT_TO_DESKTOP_1)?;
        let out = &port::screenshot_rect_to_desktop(frame, rect)?;
        wire::write_value(w, out, &S_SCREENSHOT_RECT_TO_DESKTOP_OUT)?;
        Ok(())
    })
}

static S_SCREENSHOT_DELTA_TO_DESKTOP_0: wire::Schema = wire::Schema::Dyn;
static S_SCREENSHOT_DELTA_TO_DESKTOP_OUT: wire::Schema =
    wire::Schema::Obj(&[("deltaX", wire::Schema::Num), ("deltaY", wire::Schema::Num)]);

/// `screenshotDeltaToDesktop` over the binary wire.
#[napi(js_name = "screenshotDeltaToDesktopWire")]
pub fn screenshot_delta_to_desktop_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let frame: Value = wire::read_value(r, &S_SCREENSHOT_DELTA_TO_DESKTOP_0)?;
        let delta_x: f64 = r.f64()?;
        let delta_y: f64 = r.f64()?;
        let out = &port::screenshot_delta_to_desktop(frame, delta_x, delta_y)?;
        wire::write_value(w, out, &S_SCREENSHOT_DELTA_TO_DESKTOP_OUT)?;
        Ok(())
    })
}

static S_ESTIMATE_VERTICAL_TRAVEL_0: wire::Schema = wire::Schema::Dyn;
static S_ESTIMATE_VERTICAL_TRAVEL_1: wire::Schema = wire::Schema::Dyn;
static S_ESTIMATE_VERTICAL_TRAVEL_2: wire::Schema = wire::Schema::Dyn;
static S_ESTIMATE_VERTICAL_TRAVEL_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Num);

/// `estimateVerticalTravel` over the binary wire.
#[napi(js_name = "estimateVerticalTravelWire")]
pub fn estimate_vertical_travel_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let before: Value = wire::read_value(r, &S_ESTIMATE_VERTICAL_TRAVEL_0)?;
        let after: Value = wire::read_value(r, &S_ESTIMATE_VERTICAL_TRAVEL_1)?;
        let options: Value = wire::read_value(r, &S_ESTIMATE_VERTICAL_TRAVEL_2)?;
        let out = &port::estimate_vertical_travel(before, after, options)?;
        wire::write_value(w, out, &S_ESTIMATE_VERTICAL_TRAVEL_OUT)?;
        Ok(())
    })
}

/// `resolveStillIntervalMs` over the binary wire.
#[napi(js_name = "resolveStillIntervalMsWire")]
pub fn resolve_still_interval_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let interval_ms: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::resolve_still_interval_ms(interval_ms)?;
        w.f64(*out);
        Ok(())
    })
}

/// `normalizeLabelSpaces` over the binary wire.
#[napi(js_name = "normalizeLabelSpacesWire")]
pub fn normalize_label_spaces_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let label: String = r.string16()?;
        let out = &port::normalize_label_spaces(label)?;
        w.string16(out);
        Ok(())
    })
}

static S_CANDIDATE_DESCRIPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_CANDIDATE_DESCRIPTIONS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("label", wire::Schema::Str16),
    ("role", wire::Schema::Str16),
    ("windowId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("onScreen", wire::Schema::Bool),
    ("frame", wire::Schema::Dyn),
]));

/// `candidateDescriptions` over the binary wire.
#[napi(js_name = "candidateDescriptionsWire")]
pub fn candidate_descriptions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let nodes: Value = wire::read_value(r, &S_CANDIDATE_DESCRIPTIONS_0)?;
        let out = &port::candidate_descriptions(nodes)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_CANDIDATE_DESCRIPTIONS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_DESCRIBE_TARGET_0: wire::Schema = wire::Schema::Dyn;

/// `describeTarget` over the binary wire.
#[napi(js_name = "describeTargetWire")]
pub fn describe_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let target: Value = wire::read_value(r, &S_DESCRIBE_TARGET_0)?;
        let out = &port::describe_target(target)?;
        w.string16(out);
        Ok(())
    })
}

static S_ACTIVATION_POINT_FOR_NODE_0: wire::Schema = wire::Schema::Dyn;
static S_ACTIVATION_POINT_FOR_NODE_OUT: wire::Schema = wire::Schema::Dyn;

/// `activationPointForNode` over the binary wire.
#[napi(js_name = "activationPointForNodeWire")]
pub fn activation_point_for_node_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: Value = wire::read_value(r, &S_ACTIVATION_POINT_FOR_NODE_0)?;
        let out = &port::activation_point_for_node(node)?;
        wire::write_value(w, out, &S_ACTIVATION_POINT_FOR_NODE_OUT)?;
        Ok(())
    })
}

static S_COMPUTER_TARGET_CANDIDATES_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_TARGET_CANDIDATES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("label", wire::Schema::Str16),
    ("role", wire::Schema::Str16),
    ("windowId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("onScreen", wire::Schema::Bool),
    ("frame", wire::Schema::Dyn),
]));

/// `computerTargetCandidates` over the binary wire.
#[napi(js_name = "computerTargetCandidatesWire")]
pub fn computer_target_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: Value = wire::read_value(r, &S_COMPUTER_TARGET_CANDIDATES_0)?;
        let out = &port::computer_target_candidates(root)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_COMPUTER_TARGET_CANDIDATES_OUT.inner())?;
        }
        Ok(())
    })
}

static S_RESOLVE_COMPUTER_SEMANTIC_TARGET_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_COMPUTER_SEMANTIC_TARGET_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_COMPUTER_SEMANTIC_TARGET_OUT: wire::Schema = wire::Schema::Obj(&[
    ("point", wire::Schema::Dyn),
    (
        "node",
        wire::Schema::Obj(&[
            ("role", wire::Schema::Str16),
            ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("frame", wire::Schema::Dyn),
            ("activationPoint", wire::Schema::Dyn),
            ("onScreen", wire::Schema::Bool),
            ("windowId", wire::Schema::Dyn),
            (
                "nodePath",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Num)),
            ),
            (
                "accessibilityRoot",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("editable", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("truncated", wire::Schema::OptJs(&wire::Schema::Bool)),
            (
                "children",
                wire::Schema::List(&wire::Schema::Obj(&[
                    ("role", wire::Schema::Dyn),
                    ("label", wire::Schema::Dyn),
                    ("value", wire::Schema::Dyn),
                    ("description", wire::Schema::Dyn),
                    ("frame", wire::Schema::Dyn),
                    ("activationPoint", wire::Schema::Dyn),
                    ("onScreen", wire::Schema::Dyn),
                    ("windowId", wire::Schema::Dyn),
                    ("nodePath", wire::Schema::Dyn),
                    ("accessibilityRoot", wire::Schema::Dyn),
                    ("editable", wire::Schema::Dyn),
                    ("truncated", wire::Schema::Dyn),
                    ("children", wire::Schema::Dyn),
                ])),
            ),
        ]),
    ),
]);

/// `resolveComputerSemanticTarget` over the binary wire.
#[napi(js_name = "resolveComputerSemanticTargetWire")]
pub fn resolve_computer_semantic_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: Value = wire::read_value(r, &S_RESOLVE_COMPUTER_SEMANTIC_TARGET_0)?;
        let target: Value = wire::read_value(r, &S_RESOLVE_COMPUTER_SEMANTIC_TARGET_1)?;
        let allow_offscreen: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::resolve_computer_semantic_target(root, target, allow_offscreen)?;
        wire::write_value(w, out, &S_RESOLVE_COMPUTER_SEMANTIC_TARGET_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_COMPUTER_UNIQUE_TEXT_TARGET_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_COMPUTER_UNIQUE_TEXT_TARGET_OUT: wire::Schema = wire::Schema::Obj(&[
    ("point", wire::Schema::Dyn),
    (
        "node",
        wire::Schema::Obj(&[
            ("role", wire::Schema::Str16),
            ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("frame", wire::Schema::Dyn),
            ("activationPoint", wire::Schema::Dyn),
            ("onScreen", wire::Schema::Bool),
            ("windowId", wire::Schema::Dyn),
            (
                "nodePath",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Num)),
            ),
            (
                "accessibilityRoot",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("editable", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("truncated", wire::Schema::OptJs(&wire::Schema::Bool)),
            (
                "children",
                wire::Schema::List(&wire::Schema::Obj(&[
                    ("role", wire::Schema::Dyn),
                    ("label", wire::Schema::Dyn),
                    ("value", wire::Schema::Dyn),
                    ("description", wire::Schema::Dyn),
                    ("frame", wire::Schema::Dyn),
                    ("activationPoint", wire::Schema::Dyn),
                    ("onScreen", wire::Schema::Dyn),
                    ("windowId", wire::Schema::Dyn),
                    ("nodePath", wire::Schema::Dyn),
                    ("accessibilityRoot", wire::Schema::Dyn),
                    ("editable", wire::Schema::Dyn),
                    ("truncated", wire::Schema::Dyn),
                    ("children", wire::Schema::Dyn),
                ])),
            ),
        ]),
    ),
]);

/// `resolveComputerUniqueTextTarget` over the binary wire.
#[napi(js_name = "resolveComputerUniqueTextTargetWire")]
pub fn resolve_computer_unique_text_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: Value = wire::read_value(r, &S_RESOLVE_COMPUTER_UNIQUE_TEXT_TARGET_0)?;
        let window_id: String = r.string16()?;
        let allow_offscreen: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::resolve_computer_unique_text_target(root, window_id, allow_offscreen)?;
        wire::write_value(w, out, &S_RESOLVE_COMPUTER_UNIQUE_TEXT_TARGET_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_COMPUTER_WINDOW_TARGET_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_COMPUTER_WINDOW_TARGET_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("point", wire::Schema::Dyn),
        (
            "node",
            wire::Schema::Obj(&[
                ("role", wire::Schema::Str16),
                ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("frame", wire::Schema::Dyn),
                ("activationPoint", wire::Schema::Dyn),
                ("onScreen", wire::Schema::Bool),
                ("windowId", wire::Schema::Dyn),
                (
                    "nodePath",
                    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
                ),
                (
                    "accessibilityRoot",
                    wire::Schema::OptJs(&wire::Schema::Str16),
                ),
                ("editable", wire::Schema::OptJs(&wire::Schema::Bool)),
                ("truncated", wire::Schema::OptJs(&wire::Schema::Bool)),
                ("children", wire::Schema::List(&wire::Schema::Dyn)),
            ]),
        ),
    ]));

/// `resolveComputerWindowTarget` over the binary wire.
#[napi(js_name = "resolveComputerWindowTargetWire")]
pub fn resolve_computer_window_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: Value = wire::read_value(r, &S_RESOLVE_COMPUTER_WINDOW_TARGET_0)?;
        let window_id: String = r.string16()?;
        let out = &port::resolve_computer_window_target(root, window_id)?;
        wire::write_value(w, out, &S_RESOLVE_COMPUTER_WINDOW_TARGET_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_COMPUTER_POINT_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_COMPUTER_POINT_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_COMPUTER_POINT_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveComputerPoint` over the binary wire.
#[napi(js_name = "resolveComputerPointWire")]
pub fn resolve_computer_point_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let target: Value = wire::read_value(r, &S_RESOLVE_COMPUTER_POINT_0)?;
        let screen_size: Value = wire::read_value(r, &S_RESOLVE_COMPUTER_POINT_1)?;
        let out = &port::resolve_computer_point(target, screen_size)?;
        wire::write_value(w, out, &S_RESOLVE_COMPUTER_POINT_OUT)?;
        Ok(())
    })
}

static S_ACTIONABLE_ELEMENTS_0: wire::Schema = wire::Schema::Dyn;
static S_ACTIONABLE_ELEMENTS_1: wire::Schema = wire::Schema::Dyn;
static S_ACTIONABLE_ELEMENTS_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "items",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("ref", wire::Schema::Num),
            ("role", wire::Schema::Str16),
            ("label", wire::Schema::Str16),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("windowId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "refIndex",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("label", wire::Schema::Str16),
            ("role", wire::Schema::Str16),
            ("windowId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("ordinal", wire::Schema::Num),
        ])),
    ),
    ("complete", wire::Schema::Bool),
    ("sourceIncomplete", wire::Schema::Bool),
    ("omitted", wire::Schema::Num),
]);

/// `actionableElements` over the binary wire.
#[napi(js_name = "actionableElementsWire")]
pub fn actionable_elements_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: Value = wire::read_value(r, &S_ACTIONABLE_ELEMENTS_0)?;
        let filter: Value = wire::read_value(r, &S_ACTIONABLE_ELEMENTS_1)?;
        let out = &port::actionable_elements(root, filter)?;
        wire::write_value(w, out, &S_ACTIONABLE_ELEMENTS_OUT)?;
        Ok(())
    })
}

static S_DIFF_ACTIONABLE_ELEMENTS_0: wire::Schema = wire::Schema::Dyn;
static S_DIFF_ACTIONABLE_ELEMENTS_1: wire::Schema = wire::Schema::Dyn;
static S_DIFF_ACTIONABLE_ELEMENTS_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "added",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("ref", wire::Schema::Num),
            ("role", wire::Schema::Str16),
            ("label", wire::Schema::Str16),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("windowId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "removed",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("ref", wire::Schema::Num),
            ("role", wire::Schema::Str16),
            ("label", wire::Schema::Str16),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("windowId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "changed",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("ref", wire::Schema::Num),
            ("role", wire::Schema::Str16),
            ("label", wire::Schema::Str16),
            ("windowId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("was", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
]);

/// `diffActionableElements` over the binary wire.
#[napi(js_name = "diffActionableElementsWire")]
pub fn diff_actionable_elements_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let before: Value = wire::read_value(r, &S_DIFF_ACTIONABLE_ELEMENTS_0)?;
        let after: Value = wire::read_value(r, &S_DIFF_ACTIONABLE_ELEMENTS_1)?;
        let out = &port::diff_actionable_elements(before, after)?;
        wire::write_value(w, out, &S_DIFF_ACTIONABLE_ELEMENTS_OUT)?;
        Ok(())
    })
}

static S_DESCRIBE_COMPUTER_UI_TREE_0: wire::Schema = wire::Schema::Dyn;

/// `describeComputerUiTree` over the binary wire.
#[napi(js_name = "describeComputerUiTreeWire")]
pub fn describe_computer_ui_tree_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: Value = wire::read_value(r, &S_DESCRIBE_COMPUTER_UI_TREE_0)?;
        let out = &port::describe_computer_ui_tree(root)?;
        w.string16(out);
        Ok(())
    })
}

/// `clampNodeText` over the binary wire.
#[napi(js_name = "clampNodeTextWire")]
pub fn clamp_node_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let max_length: f64 = r.f64()?;
        let out = &port::clamp_node_text(text, max_length)?;
        w.string16(out);
        Ok(())
    })
}

/// `clampTextToLength` over the binary wire.
#[napi(js_name = "clampTextToLengthWire")]
pub fn clamp_text_to_length_pub_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let max_length: f64 = r.f64()?;
        let out = &port::clamp_text_to_length_pub(text, max_length)?;
        w.string16(out);
        Ok(())
    })
}
