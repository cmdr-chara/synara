//! JS bindings for `apps/server/src/agentGateway/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `extractBearerToken`.
#[napi(js_name = "extractBearerToken")]
pub fn extract_bearer_token(
    authorization_header: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::extract_bearer_token(authorization_header)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `saveBrowserProof`.
#[napi(js_name = "saveBrowserProof")]
pub async fn save_browser_proof(
    thread_id: String,
    data: String,
    root: String,
    category: Option<String>,
) -> napi::Result<String> {
    run_async(async move {
        let root = crate::js::from_json(&root)?;
        let out = port::save_browser_proof(thread_id, data, root, category).await?;
        Ok(out)
    })
    .await
}

/// `normalizeGatewayBrowserArguments`.
#[napi(js_name = "normalizeGatewayBrowserArguments")]
pub fn normalize_gateway_browser_arguments(
    name: String,
    arguments_value: String,
) -> napi::Result<String> {
    run(move || {
        let name = crate::js::from_json(&name)?;
        let arguments_value = crate::js::from_json(&arguments_value)?;
        let out = port::normalize_gateway_browser_arguments(name, arguments_value)?;
        crate::js::to_json(&out)
    })
}

/// `computerApprovalDisplayArgs`.
#[napi(js_name = "computerApprovalDisplayArgs")]
pub fn computer_approval_display_args(args: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::computer_approval_display_args(args)?;
        Ok(out)
    })
}

/// `computerBrowserEffect`.
#[napi(js_name = "computerBrowserEffect")]
pub fn computer_browser_effect(name: String, args: String, result: String) -> napi::Result<String> {
    run(move || {
        let name = crate::js::from_json(&name)?;
        let args = crate::js::from_json(&args)?;
        let result = crate::js::from_json(&result)?;
        let out = port::computer_browser_effect(name, args, result)?;
        crate::js::to_json(&out)
    })
}

/// `computerBrowserFieldReadback`.
#[napi(js_name = "computerBrowserFieldReadback")]
pub fn computer_browser_field_readback(
    name: String,
    args: String,
    result: String,
) -> napi::Result<bool> {
    run(move || {
        let name = crate::js::from_json(&name)?;
        let args = crate::js::from_json(&args)?;
        let result = crate::js::from_json(&result)?;
        let out = port::computer_browser_field_readback(name, args, result)?;
        Ok(out)
    })
}

/// `computerBrowserToolRequiresApproval`.
#[napi(js_name = "computerBrowserToolRequiresApproval")]
pub fn computer_browser_tool_requires_approval(name: String, args: String) -> napi::Result<bool> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::computer_browser_tool_requires_approval(name, args)?;
        Ok(out)
    })
}

/// `computerToolInstructions`.
#[napi(js_name = "computerToolInstructions")]
pub fn computer_tool_instructions() -> napi::Result<String> {
    run(move || {
        let out = port::computer_tool_instructions()?;
        Ok(out)
    })
}

/// `readComputerPastePayload`.
#[napi(js_name = "readComputerPastePayload")]
pub fn read_computer_paste_payload(args: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_computer_paste_payload(args)?;
        crate::js::to_json(&out)
    })
}

/// `canonicalSynaraComputerToolName`.
#[napi(js_name = "canonicalSynaraComputerToolName")]
pub fn canonical_synara_computer_tool_name(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::canonical_synara_computer_tool_name(value)?;
        crate::js::to_json(&out)
    })
}

/// `qualifiedSynaraComputerToolName`.
#[napi(js_name = "qualifiedSynaraComputerToolName")]
pub fn qualified_synara_computer_tool_name(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::qualified_synara_computer_tool_name(value)?;
        crate::js::to_json(&out)
    })
}

/// `isSynaraComputerToolFamilyName`.
#[napi(js_name = "isSynaraComputerToolFamilyName")]
pub fn is_synara_computer_tool_family_name(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_synara_computer_tool_family_name(value)?;
        Ok(out)
    })
}

/// `computerToolNameFromProviderPermission`.
#[napi(js_name = "computerToolNameFromProviderPermission")]
pub fn computer_tool_name_from_provider_permission(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::computer_tool_name_from_provider_permission(input)?;
        crate::js::to_json(&out)
    })
}

/// `isSynaraGatewayToolName`.
#[napi(js_name = "isSynaraGatewayToolName")]
pub fn is_synara_gateway_tool_name(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_synara_gateway_tool_name(value)?;
        Ok(out)
    })
}

/// `isSynaraGatewayToolCall`.
#[napi(js_name = "isSynaraGatewayToolCall")]
pub fn is_synara_gateway_tool_call(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::is_synara_gateway_tool_call(input)?;
        Ok(out)
    })
}

/// `shouldAllowSynaraComputerProviderTool`.
#[napi(js_name = "shouldAllowSynaraComputerProviderTool")]
pub fn should_allow_synara_computer_provider_tool(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::should_allow_synara_computer_provider_tool(input)?;
        Ok(out)
    })
}

/// `computerToolRequiresApproval`.
#[napi(js_name = "computerToolRequiresApproval")]
pub fn computer_tool_requires_approval(name: String) -> napi::Result<bool> {
    run(move || {
        let out = port::computer_tool_requires_approval(name)?;
        Ok(out)
    })
}

/// `computerApprovalRequiredError`.
#[napi(js_name = "computerApprovalRequiredError")]
pub fn computer_approval_required_error(name: String) -> napi::Result<String> {
    run(move || {
        let out = port::computer_approval_required_error(name)?;
        crate::js::to_json(&out)
    })
}

/// `cuaActionErrorPayload`.
#[napi(js_name = "cuaActionErrorPayload")]
pub fn cua_action_error_payload(error: String) -> napi::Result<String> {
    run(move || {
        let error = crate::js::from_json(&error)?;
        let out = port::cua_action_error_payload(error)?;
        crate::js::to_json(&out)
    })
}

/// `boundedWorkspacePath`.
#[napi(js_name = "boundedWorkspacePath")]
pub async fn bounded_workspace_path(
    raw: String,
    workspace_root: String,
    field: String,
) -> napi::Result<String> {
    run_async(async move {
        let raw = crate::js::from_json(&raw)?;
        let out = port::bounded_workspace_path(raw, workspace_root, field).await?;
        Ok(out)
    })
    .await
}

/// `gatewayIsoNow`.
#[napi(js_name = "gatewayIsoNow")]
pub fn gateway_iso_now() -> napi::Result<String> {
    run(move || {
        let out = port::gateway_iso_now()?;
        Ok(out)
    })
}

/// `canonicalJson`.
#[napi(js_name = "canonicalJson")]
pub fn canonical_json(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::canonical_json(value)?;
        Ok(out)
    })
}

/// `stableGatewayDigest`.
#[napi(js_name = "stableGatewayDigest")]
pub fn stable_gateway_digest(value: String, length: Option<f64>) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::stable_gateway_digest(value, length)?;
        Ok(out)
    })
}

/// `makeAgentCreationIds`.
#[napi(js_name = "makeAgentCreationIds")]
pub fn make_agent_creation_ids(operation_id: String, index: f64) -> napi::Result<String> {
    run(move || {
        let out = port::make_agent_creation_ids(operation_id, index)?;
        crate::js::to_json(&out)
    })
}

/// `deviceToolRequiresApproval`.
#[napi(js_name = "deviceToolRequiresApproval")]
pub fn device_tool_requires_approval(name: String) -> napi::Result<bool> {
    run(move || {
        let out = port::device_tool_requires_approval(name)?;
        Ok(out)
    })
}

/// `edgeForSwipeStart`.
#[napi(js_name = "edgeForSwipeStart")]
pub fn edge_for_swipe_start(
    point: String,
    screen: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let point = crate::js::from_json(&point)?;
        let screen = crate::js::from_json(&screen)?;
        let out = port::edge_for_swipe_start(point, screen)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `diagnosticFilterFingerprint`.
#[napi(js_name = "diagnosticFilterFingerprint")]
pub fn diagnostic_filter_fingerprint(filters: String) -> napi::Result<String> {
    run(move || {
        let filters = crate::js::from_json(&filters)?;
        let out = port::diagnostic_filter_fingerprint(filters)?;
        Ok(out)
    })
}

/// `encodeDiagnosticCursor`.
#[napi(js_name = "encodeDiagnosticCursor")]
pub fn encode_diagnostic_cursor(cursor: String) -> napi::Result<String> {
    run(move || {
        let cursor = crate::js::from_json(&cursor)?;
        let out = port::encode_diagnostic_cursor(cursor)?;
        Ok(out)
    })
}

/// `decodeDiagnosticCursor`.
#[napi(js_name = "decodeDiagnosticCursor")]
pub fn decode_diagnostic_cursor(raw: Option<String>, expected: String) -> napi::Result<String> {
    run(move || {
        let expected = crate::js::from_json(&expected)?;
        let out = port::decode_diagnostic_cursor(raw, expected)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeDiagnosticValue`.
#[napi(js_name = "sanitizeDiagnosticValue")]
pub fn sanitize_diagnostic_value(value: String, depth: Option<f64>) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::sanitize_diagnostic_value(value, depth)?;
        crate::js::to_json(&out)
    })
}

/// `renderSynaraHarnessPolicy`.
#[napi(js_name = "renderSynaraHarnessPolicy")]
pub fn render_synara_harness_policy(capabilities: String) -> napi::Result<String> {
    run(move || {
        let capabilities = crate::js::from_json(&capabilities)?;
        let out = port::render_synara_harness_policy(capabilities)?;
        Ok(out)
    })
}

/// `providerHasSynaraGatewayControl`.
#[napi(js_name = "providerHasSynaraGatewayControl")]
pub fn provider_has_synara_gateway_control(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::provider_has_synara_gateway_control(input)?;
        Ok(out)
    })
}

/// `takeSynaraHarnessPolicyForSession`.
#[napi(js_name = "takeSynaraHarnessPolicyForSession")]
pub fn take_synara_harness_policy_for_session(
    state: String,
    capabilities: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let capabilities = crate::js::from_json(&capabilities)?;
        let out = port::take_synara_harness_policy_for_session(state, capabilities)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `takeSynaraHarnessPolicyForProviderSession`.
#[napi(js_name = "takeSynaraHarnessPolicyForProviderSession")]
pub fn take_synara_harness_policy_for_provider_session(
    state: String,
    input: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let input = crate::js::from_json(&input)?;
        let out = port::take_synara_harness_policy_for_provider_session(state, input)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `takeSynaraHarnessPolicyTextPartForProviderSession`.
#[napi(js_name = "takeSynaraHarnessPolicyTextPartForProviderSession")]
pub fn take_synara_harness_policy_text_part_for_provider_session(
    state: String,
    input: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let input = crate::js::from_json(&input)?;
        let out = port::take_synara_harness_policy_text_part_for_provider_session(state, input)?;
        crate::js::to_json(&out)
    })
}

/// `readMcpJsonBody`.
#[napi(js_name = "readMcpJsonBody")]
pub fn read_mcp_json_body(request: String, max_bytes: String) -> napi::Result<String> {
    run(move || {
        let request = crate::js::from_json(&request)?;
        let max_bytes = crate::js::from_json(&max_bytes)?;
        let out = port::read_mcp_json_body(request, max_bytes)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeHubTaskKey`.
#[napi(js_name = "normalizeHubTaskKey")]
pub fn normalize_hub_task_key(
    value: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::normalize_hub_task_key(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `hubWorkWorkItemId`.
#[napi(js_name = "hubWorkWorkItemId")]
pub fn hub_work_work_item_id(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::hub_work_work_item_id(input)?;
        Ok(out)
    })
}

/// `hubWorkTaskFingerprint`.
#[napi(js_name = "hubWorkTaskFingerprint")]
pub fn hub_work_task_fingerprint(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::hub_work_task_fingerprint(input)?;
        Ok(out)
    })
}

/// `hubWorkRequestFingerprint`.
#[napi(js_name = "hubWorkRequestFingerprint")]
pub fn hub_work_request_fingerprint(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::hub_work_request_fingerprint(input)?;
        Ok(out)
    })
}

/// `renderHubWorkPrompt`.
#[napi(js_name = "renderHubWorkPrompt")]
pub fn render_hub_work_prompt(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::render_hub_work_prompt(input)?;
        Ok(out)
    })
}

/// `buildCodexMcpConfigToml`.
#[napi(js_name = "buildCodexMcpConfigToml")]
pub fn build_codex_mcp_config_toml(
    endpoint_url: String,
    credential_transport: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::build_codex_mcp_config_toml(endpoint_url, credential_transport)?;
        Ok(out)
    })
}

/// `buildCodexMcpThreadConfig`.
#[napi(js_name = "buildCodexMcpThreadConfig")]
pub fn build_codex_mcp_thread_config(connection: String) -> napi::Result<String> {
    run(move || {
        let connection = crate::js::from_json(&connection)?;
        let out = port::build_codex_mcp_thread_config(connection)?;
        crate::js::to_json(&out)
    })
}

/// `buildOpenCodeMcpServer`.
#[napi(js_name = "buildOpenCodeMcpServer")]
pub fn build_open_code_mcp_server(connection: String) -> napi::Result<String> {
    run(move || {
        let connection = crate::js::from_json(&connection)?;
        let out = port::build_open_code_mcp_server(connection)?;
        crate::js::to_json(&out)
    })
}

/// `buildOpenCodeConnectorMcpServers`.
#[napi(js_name = "buildOpenCodeConnectorMcpServers")]
pub fn build_open_code_connector_mcp_servers(connection: String) -> napi::Result<String> {
    run(move || {
        let connection = crate::js::from_json(&connection)?;
        let out = port::build_open_code_connector_mcp_servers(connection)?;
        crate::js::to_json(&out)
    })
}

/// `buildClaudeMcpServers`.
#[napi(js_name = "buildClaudeMcpServers")]
pub fn build_claude_mcp_servers(connection: String) -> napi::Result<String> {
    run(move || {
        let connection = crate::js::from_json(&connection)?;
        let out = port::build_claude_mcp_servers(connection)?;
        crate::js::to_json(&out)
    })
}

/// `buildAcpSynaraMcpServers`.
#[napi(js_name = "buildAcpSynaraMcpServers")]
pub fn build_acp_synara_mcp_servers(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_acp_synara_mcp_servers(input)?;
        crate::js::to_json(&out)
    })
}

/// `parseRecoverableCreationPlan`.
#[napi(js_name = "parseRecoverableCreationPlan")]
pub fn parse_recoverable_creation_plan(
    plan_json: String,
    operation_id: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::parse_recoverable_creation_plan(plan_json, operation_id)?;
        crate::js::to_json(&out)
    })
}

/// `recordCreatedWorktreeInPlan`.
#[napi(js_name = "recordCreatedWorktreeInPlan")]
pub fn record_created_worktree_in_plan(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::record_created_worktree_in_plan(input)?;
        Ok(out)
    })
}

/// `redactCreationPlanForPurgedCaller`.
#[napi(js_name = "redactCreationPlanForPurgedCaller")]
pub fn redact_creation_plan_for_purged_caller(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::redact_creation_plan_for_purged_caller(input)?;
        Ok(out)
    })
}

/// `mcpToolResultError`.
#[napi(js_name = "mcpToolResultError")]
pub fn mcp_tool_result_error(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::mcp_tool_result_error(text)?;
        crate::js::to_json(&out)
    })
}

/// `mcpToolResultJson`.
#[napi(js_name = "mcpToolResultJson")]
pub fn mcp_tool_result_json(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::mcp_tool_result_json(value)?;
        crate::js::to_json(&out)
    })
}

/// `jsonRpcResult`.
#[napi(js_name = "jsonRpcResult")]
pub fn json_rpc_result(id: String, result: String) -> napi::Result<String> {
    run(move || {
        let id = crate::js::from_json(&id)?;
        let result = crate::js::from_json(&result)?;
        let out = port::json_rpc_result(id, result)?;
        crate::js::to_json(&out)
    })
}

/// `jsonRpcError`.
#[napi(js_name = "jsonRpcError")]
pub fn json_rpc_error(id: String, code: f64, message: String) -> napi::Result<String> {
    run(move || {
        let id = crate::js::from_json(&id)?;
        let out = port::json_rpc_error(id, code, message)?;
        crate::js::to_json(&out)
    })
}

/// `parseMcpMessage`.
#[napi(js_name = "parseMcpMessage")]
pub fn parse_mcp_message(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::parse_mcp_message(raw)?;
        crate::js::to_json(&out)
    })
}

/// `negotiateMcpProtocolVersion`.
#[napi(js_name = "negotiateMcpProtocolVersion")]
pub fn negotiate_mcp_protocol_version(requested: String) -> napi::Result<String> {
    run(move || {
        let requested = crate::js::from_json(&requested)?;
        let out = port::negotiate_mcp_protocol_version(requested)?;
        Ok(out)
    })
}

/// `buildMcpInitializeResult`.
#[napi(js_name = "buildMcpInitializeResult")]
pub fn build_mcp_initialize_result(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_mcp_initialize_result(input)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeToolInputSchema`.
#[napi(js_name = "sanitizeToolInputSchema")]
pub fn sanitize_tool_input_schema(schema: String) -> napi::Result<String> {
    run(move || {
        let schema = crate::js::from_json(&schema)?;
        let out = port::sanitize_tool_input_schema(schema)?;
        crate::js::to_json(&out)
    })
}

/// `isJsonRecord`.
#[napi(js_name = "isJsonRecord")]
pub fn is_json_record(node: String) -> napi::Result<bool> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let out = port::is_json_record(node)?;
        Ok(out)
    })
}

/// `countSchemaKeyOccurrences`.
#[napi(js_name = "countSchemaKeyOccurrences")]
pub fn count_schema_key_occurrences(node: String, key: String) -> napi::Result<f64> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let out = port::count_schema_key_occurrences(node, key)?;
        Ok(out)
    })
}

/// `agentGatewayCapabilitiesFor`.
#[napi(js_name = "agentGatewayCapabilitiesFor")]
pub fn agent_gateway_capabilities_for(input: String) -> napi::Result<Vec<String>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::agent_gateway_capabilities_for(input)?;
        Ok(out)
    })
}

/// `agentGatewayProviderCapabilitiesFor`.
#[napi(js_name = "agentGatewayProviderCapabilitiesFor")]
pub fn agent_gateway_provider_capabilities_for(provider: String) -> napi::Result<Vec<String>> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::agent_gateway_provider_capabilities_for(provider)?;
        Ok(out)
    })
}

/// `agentGatewaySessionLeaseOptionsFor`.
#[napi(js_name = "agentGatewaySessionLeaseOptionsFor")]
pub fn agent_gateway_session_lease_options_for(
    input: String,
    provider: String,
) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let provider = crate::js::from_json(&provider)?;
        let out = port::agent_gateway_session_lease_options_for(input, provider)?;
        crate::js::to_json(&out)
    })
}

/// `captureAgentGatewayCapabilityInput`.
#[napi(js_name = "captureAgentGatewayCapabilityInput")]
pub fn capture_agent_gateway_capability_input(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::capture_agent_gateway_capability_input(input)?;
        crate::js::to_json(&out)
    })
}

/// `agentGatewayTargetOptionGuidance`.
#[napi(js_name = "agentGatewayTargetOptionGuidance")]
pub fn agent_gateway_target_option_guidance(catalog: String) -> napi::Result<String> {
    run(move || {
        let catalog = crate::js::from_json(&catalog)?;
        let out = port::agent_gateway_target_option_guidance(catalog)?;
        crate::js::to_json(&out)
    })
}

/// `readDiagnosticPageLimit`.
#[napi(js_name = "readDiagnosticPageLimit")]
pub fn read_diagnostic_page_limit(args: String) -> napi::Result<f64> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_diagnostic_page_limit(args)?;
        Ok(out)
    })
}

/// `diagnosticEventMessageId`.
#[napi(js_name = "diagnosticEventMessageId")]
pub fn diagnostic_event_message_id(
    event: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::diagnostic_event_message_id(event)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `groupDiagnosticEvents`.
#[napi(js_name = "groupDiagnosticEvents")]
pub fn group_diagnostic_events(events_newest_first: String) -> napi::Result<String> {
    run(move || {
        let events_newest_first = crate::js::from_json(&events_newest_first)?;
        let out = port::group_diagnostic_events(events_newest_first)?;
        crate::js::to_json(&out)
    })
}

/// `shapeDiagnosticEvents`.
#[napi(js_name = "shapeDiagnosticEvents")]
pub fn shape_diagnostic_events(
    events_newest_first: String,
    payload_mode: String,
) -> napi::Result<String> {
    run(move || {
        let events_newest_first = crate::js::from_json(&events_newest_first)?;
        let out = port::shape_diagnostic_events(events_newest_first, payload_mode)?;
        crate::js::to_json(&out)
    })
}

/// `deriveAgentThreadStatus`.
#[napi(js_name = "deriveAgentThreadStatus")]
pub fn derive_agent_thread_status(thread: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::derive_agent_thread_status(thread)?;
        Ok(out)
    })
}

/// `summarizeThreadShell`.
#[napi(js_name = "summarizeThreadShell")]
pub fn summarize_thread_shell(thread: String, caller_thread_id: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::summarize_thread_shell(thread, caller_thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `summarizeWaitThreadText`.
#[napi(js_name = "summarizeWaitThreadText")]
pub fn summarize_wait_thread_text(text: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::summarize_wait_thread_text(text)?;
        crate::js::to_json(&out)
    })
}

/// `paginateThreadMessages`.
#[napi(js_name = "paginateThreadMessages")]
pub fn paginate_thread_messages(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::paginate_thread_messages(input)?;
        crate::js::to_json(&out)
    })
}

/// `summarizeThreadDetail`.
#[napi(js_name = "summarizeThreadDetail")]
pub fn summarize_thread_detail(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::summarize_thread_detail(input)?;
        crate::js::to_json(&out)
    })
}

/// `readStringArg`.
#[napi(js_name = "readStringArg")]
pub fn read_string_arg(
    args: String,
    name: String,
    options: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let options = crate::js::from_json(&options)?;
        let out = port::read_string_arg(args, name, options)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `readVerbatimStringArg`.
#[napi(js_name = "readVerbatimStringArg")]
pub fn read_verbatim_string_arg(
    args: String,
    name: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_verbatim_string_arg(args, name)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `readNumberArg`.
#[napi(js_name = "readNumberArg")]
pub fn read_number_arg(
    args: String,
    name: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_number_arg(args, name)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `readBooleanArg`.
#[napi(js_name = "readBooleanArg")]
pub fn read_boolean_arg(
    args: String,
    name: String,
) -> napi::Result<napi::bindgen_prelude::Either<bool, ()>> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_boolean_arg(args, name)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `readIsoTimestampArg`.
#[napi(js_name = "readIsoTimestampArg")]
pub fn read_iso_timestamp_arg(
    args: String,
    name: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_iso_timestamp_arg(args, name)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `readRecordArg`.
#[napi(js_name = "readRecordArg")]
pub fn read_record_arg(args: String, name: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_record_arg(args, name)?;
        crate::js::to_json(&out)
    })
}

/// `readStringArrayArg`.
#[napi(js_name = "readStringArrayArg")]
pub fn read_string_array_arg(args: String, name: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_string_array_arg(args, name)?;
        crate::js::to_json(&out)
    })
}

/// `parseProviderKind`.
#[napi(js_name = "parseProviderKind")]
pub fn parse_provider_kind(raw: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_provider_kind(raw)?;
        crate::js::to_json(&out)
    })
}

/// `readModelSelectionArg`.
#[napi(js_name = "readModelSelectionArg")]
pub fn read_model_selection_arg(args: String, name: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::read_model_selection_arg(args, name)?;
        crate::js::to_json(&out)
    })
}

/// `buildModelSelection`.
#[napi(js_name = "buildModelSelection")]
pub fn build_model_selection(
    provider: String,
    model: Option<String>,
    fallback_selection: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let fallback_selection = crate::js::from_json(&fallback_selection)?;
        let out = port::build_model_selection(provider, model, fallback_selection)?;
        crate::js::to_json(&out)
    })
}

/// `decodeCreateThreadsInput`.
#[napi(js_name = "decodeCreateThreadsInput")]
pub fn decode_create_threads_input(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::decode_create_threads_input(value)?;
        crate::js::to_json(&out)
    })
}

/// `decodeWaitForThreadsInput`.
#[napi(js_name = "decodeWaitForThreadsInput")]
pub fn decode_wait_for_threads_input(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::decode_wait_for_threads_input(value)?;
        crate::js::to_json(&out)
    })
}

/// `filterToolsByCapability`.
#[napi(js_name = "filterToolsByCapability")]
pub fn filter_tools_by_capability(tools: String, capabilities: String) -> napi::Result<String> {
    run(move || {
        let tools = crate::js::from_json(&tools)?;
        let capabilities = crate::js::from_json(&capabilities)?;
        let out = port::filter_tools_by_capability(tools, capabilities)?;
        crate::js::to_json(&out)
    })
}

/// `gatewayToolErrorResult`.
#[napi(js_name = "gatewayToolErrorResult")]
pub fn gateway_tool_error_result(error: String) -> napi::Result<String> {
    run(move || {
        let error = crate::js::from_json(&error)?;
        let out = port::gateway_tool_error_result(error)?;
        crate::js::to_json(&out)
    })
}

/// `parseTurnPlanSteps`.
#[napi(js_name = "parseTurnPlanSteps")]
pub fn parse_turn_plan_steps(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::parse_turn_plan_steps(input)?;
        crate::js::to_json(&out)
    })
}

/// Class `ComputerProgressGuard`.
#[napi(js_name = "ComputerProgressGuard")]
pub struct ComputerProgressGuard {
    inner: port::ComputerProgressGuard,
}

#[napi]
impl ComputerProgressGuard {
    #[napi(constructor)]
    pub fn new(maximum_scopes: String, maximum_targets_per_scope: String) -> napi::Result<Self> {
        run(move || {
            let maximum_scopes = crate::js::from_json(&maximum_scopes)?;
            let maximum_targets_per_scope = crate::js::from_json(&maximum_targets_per_scope)?;
            let out = port::ComputerProgressGuard::new(maximum_scopes, maximum_targets_per_scope)?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "check")]
    pub fn check(&mut self, action: String) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let action = crate::js::from_json(&action)?;
            let out = inner.check(action)?;
            crate::js::to_json(&out)
        })
    }

    #[napi(js_name = "record")]
    pub fn record(&mut self, action: String, outcome: String) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            let action = crate::js::from_json(&action)?;
            let outcome = crate::js::from_json(&outcome)?;
            inner.record(action, outcome)?;
            Ok(())
        })
    }

    #[napi(js_name = "recordVerifiedProgress")]
    pub fn record_verified_progress(
        &mut self,
        scope: String,
        target_key: String,
    ) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            let scope = crate::js::from_json(&scope)?;
            inner.record_verified_progress(scope, target_key)?;
            Ok(())
        })
    }
}

/// Class `ToolGuidanceCadence`.
#[napi(js_name = "ToolGuidanceCadence")]
pub struct ToolGuidanceCadence {
    inner: port::ToolGuidanceCadence,
}

#[napi]
impl ToolGuidanceCadence {
    #[napi(constructor)]
    pub fn new(interval: f64, maximum_threads: f64) -> napi::Result<Self> {
        run(move || {
            let out = port::ToolGuidanceCadence::new(interval, maximum_threads)?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "shouldRefresh")]
    pub fn should_refresh(&mut self, thread_id: String) -> napi::Result<bool> {
        let inner = &mut self.inner;
        run(move || {
            let out = inner.should_refresh(thread_id)?;
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

/// `extractBearerToken` over the binary wire.
#[napi(js_name = "extractBearerTokenWire")]
pub fn extract_bearer_token_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let authorization_header: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::extract_bearer_token(authorization_header)?;
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

static S_NORMALIZE_GATEWAY_BROWSER_ARGUMENTS_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_GATEWAY_BROWSER_ARGUMENTS_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_GATEWAY_BROWSER_ARGUMENTS_OUT: wire::Schema =
    wire::Schema::Map16(&wire::Schema::Dyn);

/// `normalizeGatewayBrowserArguments` over the binary wire.
#[napi(js_name = "normalizeGatewayBrowserArgumentsWire")]
pub fn normalize_gateway_browser_arguments_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_GATEWAY_BROWSER_ARGUMENTS_0)?;
        let arguments_value: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_GATEWAY_BROWSER_ARGUMENTS_1)?;
        let out = &port::normalize_gateway_browser_arguments(name, arguments_value)?;
        wire::write_value(w, out, &S_NORMALIZE_GATEWAY_BROWSER_ARGUMENTS_OUT)?;
        Ok(())
    })
}

static S_COMPUTER_APPROVAL_DISPLAY_ARGS_0: wire::Schema = wire::Schema::Dyn;

/// `computerApprovalDisplayArgs` over the binary wire.
#[napi(js_name = "computerApprovalDisplayArgsWire")]
pub fn computer_approval_display_args_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_COMPUTER_APPROVAL_DISPLAY_ARGS_0)?;
        let out = &port::computer_approval_display_args(args)?;
        w.string16(out);
        Ok(())
    })
}

static S_COMPUTER_BROWSER_EFFECT_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_BROWSER_EFFECT_1: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_BROWSER_EFFECT_2: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_BROWSER_EFFECT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("effect", wire::Schema::Dyn),
    ("code", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `computerBrowserEffect` over the binary wire.
#[napi(js_name = "computerBrowserEffectWire")]
pub fn computer_browser_effect_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: serde_json::Value = wire::read_value(r, &S_COMPUTER_BROWSER_EFFECT_0)?;
        let args: serde_json::Value = wire::read_value(r, &S_COMPUTER_BROWSER_EFFECT_1)?;
        let result: serde_json::Value = wire::read_value(r, &S_COMPUTER_BROWSER_EFFECT_2)?;
        let out = &port::computer_browser_effect(name, args, result)?;
        wire::write_value(w, out, &S_COMPUTER_BROWSER_EFFECT_OUT)?;
        Ok(())
    })
}

static S_COMPUTER_BROWSER_FIELD_READBACK_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_BROWSER_FIELD_READBACK_1: wire::Schema = wire::Schema::Dyn;
static S_COMPUTER_BROWSER_FIELD_READBACK_2: wire::Schema = wire::Schema::Dyn;

/// `computerBrowserFieldReadback` over the binary wire.
#[napi(js_name = "computerBrowserFieldReadbackWire")]
pub fn computer_browser_field_readback_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: serde_json::Value = wire::read_value(r, &S_COMPUTER_BROWSER_FIELD_READBACK_0)?;
        let args: serde_json::Value = wire::read_value(r, &S_COMPUTER_BROWSER_FIELD_READBACK_1)?;
        let result: serde_json::Value = wire::read_value(r, &S_COMPUTER_BROWSER_FIELD_READBACK_2)?;
        let out = &port::computer_browser_field_readback(name, args, result)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPUTER_BROWSER_TOOL_REQUIRES_APPROVAL_1: wire::Schema = wire::Schema::Dyn;

/// `computerBrowserToolRequiresApproval` over the binary wire.
#[napi(js_name = "computerBrowserToolRequiresApprovalWire")]
pub fn computer_browser_tool_requires_approval_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: String = r.string16()?;
        let args: serde_json::Value =
            wire::read_value(r, &S_COMPUTER_BROWSER_TOOL_REQUIRES_APPROVAL_1)?;
        let out = &port::computer_browser_tool_requires_approval(name, args)?;
        w.bool(*out);
        Ok(())
    })
}

/// `computerToolInstructions` over the binary wire.
#[napi(js_name = "computerToolInstructionsWire")]
pub fn computer_tool_instructions_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::computer_tool_instructions()?;
        w.string16(out);
        Ok(())
    })
}

static S_READ_COMPUTER_PASTE_PAYLOAD_0: wire::Schema = wire::Schema::Dyn;
static S_READ_COMPUTER_PASTE_PAYLOAD_OUT: wire::Schema = wire::Schema::Dyn;

/// `readComputerPastePayload` over the binary wire.
#[napi(js_name = "readComputerPastePayloadWire")]
pub fn read_computer_paste_payload_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_COMPUTER_PASTE_PAYLOAD_0)?;
        let out = &port::read_computer_paste_payload(args)?;
        wire::write_value(w, out, &S_READ_COMPUTER_PASTE_PAYLOAD_OUT)?;
        Ok(())
    })
}

static S_CANONICAL_SYNARA_COMPUTER_TOOL_NAME_0: wire::Schema = wire::Schema::Dyn;

/// `canonicalSynaraComputerToolName` over the binary wire.
#[napi(js_name = "canonicalSynaraComputerToolNameWire")]
pub fn canonical_synara_computer_tool_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value =
            wire::read_value(r, &S_CANONICAL_SYNARA_COMPUTER_TOOL_NAME_0)?;
        let out = &port::canonical_synara_computer_tool_name(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_QUALIFIED_SYNARA_COMPUTER_TOOL_NAME_0: wire::Schema = wire::Schema::Dyn;

/// `qualifiedSynaraComputerToolName` over the binary wire.
#[napi(js_name = "qualifiedSynaraComputerToolNameWire")]
pub fn qualified_synara_computer_tool_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value =
            wire::read_value(r, &S_QUALIFIED_SYNARA_COMPUTER_TOOL_NAME_0)?;
        let out = &port::qualified_synara_computer_tool_name(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_SYNARA_COMPUTER_TOOL_FAMILY_NAME_0: wire::Schema = wire::Schema::Dyn;

/// `isSynaraComputerToolFamilyName` over the binary wire.
#[napi(js_name = "isSynaraComputerToolFamilyNameWire")]
pub fn is_synara_computer_tool_family_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value =
            wire::read_value(r, &S_IS_SYNARA_COMPUTER_TOOL_FAMILY_NAME_0)?;
        let out = &port::is_synara_computer_tool_family_name(value)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPUTER_TOOL_NAME_FROM_PROVIDER_PERMISSION_0: wire::Schema = wire::Schema::Dyn;

/// `computerToolNameFromProviderPermission` over the binary wire.
#[napi(js_name = "computerToolNameFromProviderPermissionWire")]
pub fn computer_tool_name_from_provider_permission_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_COMPUTER_TOOL_NAME_FROM_PROVIDER_PERMISSION_0)?;
        let out = &port::computer_tool_name_from_provider_permission(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_SYNARA_GATEWAY_TOOL_NAME_0: wire::Schema = wire::Schema::Dyn;

/// `isSynaraGatewayToolName` over the binary wire.
#[napi(js_name = "isSynaraGatewayToolNameWire")]
pub fn is_synara_gateway_tool_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_IS_SYNARA_GATEWAY_TOOL_NAME_0)?;
        let out = &port::is_synara_gateway_tool_name(value)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_SYNARA_GATEWAY_TOOL_CALL_0: wire::Schema = wire::Schema::Dyn;

/// `isSynaraGatewayToolCall` over the binary wire.
#[napi(js_name = "isSynaraGatewayToolCallWire")]
pub fn is_synara_gateway_tool_call_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_IS_SYNARA_GATEWAY_TOOL_CALL_0)?;
        let out = &port::is_synara_gateway_tool_call(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_ALLOW_SYNARA_COMPUTER_PROVIDER_TOOL_0: wire::Schema = wire::Schema::Dyn;

/// `shouldAllowSynaraComputerProviderTool` over the binary wire.
#[napi(js_name = "shouldAllowSynaraComputerProviderToolWire")]
pub fn should_allow_synara_computer_provider_tool_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_SHOULD_ALLOW_SYNARA_COMPUTER_PROVIDER_TOOL_0)?;
        let out = &port::should_allow_synara_computer_provider_tool(input)?;
        w.bool(*out);
        Ok(())
    })
}

/// `computerToolRequiresApproval` over the binary wire.
#[napi(js_name = "computerToolRequiresApprovalWire")]
pub fn computer_tool_requires_approval_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: String = r.string16()?;
        let out = &port::computer_tool_requires_approval(name)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPUTER_APPROVAL_REQUIRED_ERROR_OUT: wire::Schema = wire::Schema::Obj(&[
    ("code", wire::Schema::Str16),
    ("message", wire::Schema::Str16),
]);

/// `computerApprovalRequiredError` over the binary wire.
#[napi(js_name = "computerApprovalRequiredErrorWire")]
pub fn computer_approval_required_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: String = r.string16()?;
        let out = &port::computer_approval_required_error(name)?;
        wire::write_value(w, out, &S_COMPUTER_APPROVAL_REQUIRED_ERROR_OUT)?;
        Ok(())
    })
}

static S_CUA_ACTION_ERROR_PAYLOAD_0: wire::Schema = wire::Schema::Dyn;
static S_CUA_ACTION_ERROR_PAYLOAD_OUT: wire::Schema = wire::Schema::Obj(&[
    ("error", wire::Schema::Str16),
    ("effect", wire::Schema::Str16),
    ("message", wire::Schema::Str16),
    ("retryAllowed", wire::Schema::Bool),
    ("diagnostics", wire::Schema::Dyn),
    ("layer", wire::Schema::Dyn),
    ("wait_seconds", wire::Schema::OptJs(&wire::Schema::Num)),
    ("requery_hint", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `cuaActionErrorPayload` over the binary wire.
#[napi(js_name = "cuaActionErrorPayloadWire")]
pub fn cua_action_error_payload_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let error: serde_json::Value = wire::read_value(r, &S_CUA_ACTION_ERROR_PAYLOAD_0)?;
        let out = &port::cua_action_error_payload(error)?;
        wire::write_value(w, out, &S_CUA_ACTION_ERROR_PAYLOAD_OUT)?;
        Ok(())
    })
}

/// `gatewayIsoNow` over the binary wire.
#[napi(js_name = "gatewayIsoNowWire")]
pub fn gateway_iso_now_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::gateway_iso_now()?;
        w.string16(out);
        Ok(())
    })
}

static S_CANONICAL_JSON_0: wire::Schema = wire::Schema::Dyn;

/// `canonicalJson` over the binary wire.
#[napi(js_name = "canonicalJsonWire")]
pub fn canonical_json_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_CANONICAL_JSON_0)?;
        let out = &port::canonical_json(value)?;
        w.string16(out);
        Ok(())
    })
}

static S_STABLE_GATEWAY_DIGEST_0: wire::Schema = wire::Schema::Dyn;

/// `stableGatewayDigest` over the binary wire.
#[napi(js_name = "stableGatewayDigestWire")]
pub fn stable_gateway_digest_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_STABLE_GATEWAY_DIGEST_0)?;
        let length: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::stable_gateway_digest(value, length)?;
        w.string16(out);
        Ok(())
    })
}

static S_MAKE_AGENT_CREATION_IDS_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeAgentCreationIds` over the binary wire.
#[napi(js_name = "makeAgentCreationIdsWire")]
pub fn make_agent_creation_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let operation_id: String = r.string16()?;
        let index: f64 = r.f64()?;
        let out = &port::make_agent_creation_ids(operation_id, index)?;
        wire::write_value(w, out, &S_MAKE_AGENT_CREATION_IDS_OUT)?;
        Ok(())
    })
}

/// `deviceToolRequiresApproval` over the binary wire.
#[napi(js_name = "deviceToolRequiresApprovalWire")]
pub fn device_tool_requires_approval_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: String = r.string16()?;
        let out = &port::device_tool_requires_approval(name)?;
        w.bool(*out);
        Ok(())
    })
}

static S_EDGE_FOR_SWIPE_START_0: wire::Schema = wire::Schema::Dyn;

/// `edgeForSwipeStart` over the binary wire.
#[napi(js_name = "edgeForSwipeStartWire")]
pub fn edge_for_swipe_start_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let point: serde_json::Value = wire::read_value(r, &S_EDGE_FOR_SWIPE_START_0)?;
        let screen: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::edge_for_swipe_start(point, screen)?;
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

static S_DIAGNOSTIC_FILTER_FINGERPRINT_0: wire::Schema = wire::Schema::Dyn;

/// `diagnosticFilterFingerprint` over the binary wire.
#[napi(js_name = "diagnosticFilterFingerprintWire")]
pub fn diagnostic_filter_fingerprint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let filters: serde_json::Value = wire::read_value(r, &S_DIAGNOSTIC_FILTER_FINGERPRINT_0)?;
        let out = &port::diagnostic_filter_fingerprint(filters)?;
        w.string16(out);
        Ok(())
    })
}

static S_ENCODE_DIAGNOSTIC_CURSOR_0: wire::Schema = wire::Schema::Dyn;

/// `encodeDiagnosticCursor` over the binary wire.
#[napi(js_name = "encodeDiagnosticCursorWire")]
pub fn encode_diagnostic_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cursor: serde_json::Value = wire::read_value(r, &S_ENCODE_DIAGNOSTIC_CURSOR_0)?;
        let out = &port::encode_diagnostic_cursor(cursor)?;
        w.string16(out);
        Ok(())
    })
}

static S_DECODE_DIAGNOSTIC_CURSOR_1: wire::Schema = wire::Schema::Dyn;
static S_DECODE_DIAGNOSTIC_CURSOR_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    ("kind", wire::Schema::Str16),
    ("threadId", wire::Schema::Str16),
    ("filterFingerprint", wire::Schema::Str16),
    ("highWaterSequence", wire::Schema::Num),
    ("beforeSequence", wire::Schema::Num),
]));

/// `decodeDiagnosticCursor` over the binary wire.
#[napi(js_name = "decodeDiagnosticCursorWire")]
pub fn decode_diagnostic_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Option<String> = r.opt_js(|r| r.string16())?;
        let expected: serde_json::Value = wire::read_value(r, &S_DECODE_DIAGNOSTIC_CURSOR_1)?;
        let out = &port::decode_diagnostic_cursor(raw, expected)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_DECODE_DIAGNOSTIC_CURSOR_OUT.inner())?;
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_SANITIZE_DIAGNOSTIC_VALUE_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_DIAGNOSTIC_VALUE_OUT: wire::Schema = wire::Schema::Dyn;

/// `sanitizeDiagnosticValue` over the binary wire.
#[napi(js_name = "sanitizeDiagnosticValueWire")]
pub fn sanitize_diagnostic_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_SANITIZE_DIAGNOSTIC_VALUE_0)?;
        let depth: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::sanitize_diagnostic_value(value, depth)?;
        wire::write_value(w, out, &S_SANITIZE_DIAGNOSTIC_VALUE_OUT)?;
        Ok(())
    })
}

static S_RENDER_SYNARA_HARNESS_POLICY_0: wire::Schema = wire::Schema::Dyn;

/// `renderSynaraHarnessPolicy` over the binary wire.
#[napi(js_name = "renderSynaraHarnessPolicyWire")]
pub fn render_synara_harness_policy_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let capabilities: serde_json::Value =
            wire::read_value(r, &S_RENDER_SYNARA_HARNESS_POLICY_0)?;
        let out = &port::render_synara_harness_policy(capabilities)?;
        w.string16(out);
        Ok(())
    })
}

static S_PROVIDER_HAS_SYNARA_GATEWAY_CONTROL_0: wire::Schema = wire::Schema::Dyn;

/// `providerHasSynaraGatewayControl` over the binary wire.
#[napi(js_name = "providerHasSynaraGatewayControlWire")]
pub fn provider_has_synara_gateway_control_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_PROVIDER_HAS_SYNARA_GATEWAY_CONTROL_0)?;
        let out = &port::provider_has_synara_gateway_control(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_TAKE_SYNARA_HARNESS_POLICY_FOR_SESSION_0: wire::Schema = wire::Schema::Dyn;
static S_TAKE_SYNARA_HARNESS_POLICY_FOR_SESSION_1: wire::Schema = wire::Schema::Dyn;

/// `takeSynaraHarnessPolicyForSession` over the binary wire.
#[napi(js_name = "takeSynaraHarnessPolicyForSessionWire")]
pub fn take_synara_harness_policy_for_session_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_TAKE_SYNARA_HARNESS_POLICY_FOR_SESSION_0)?;
        let capabilities: serde_json::Value =
            wire::read_value(r, &S_TAKE_SYNARA_HARNESS_POLICY_FOR_SESSION_1)?;
        let out = &port::take_synara_harness_policy_for_session(state, capabilities)?;
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

static S_TAKE_SYNARA_HARNESS_POLICY_FOR_PROVIDER_SESSION_0: wire::Schema = wire::Schema::Dyn;
static S_TAKE_SYNARA_HARNESS_POLICY_FOR_PROVIDER_SESSION_1: wire::Schema = wire::Schema::Dyn;

/// `takeSynaraHarnessPolicyForProviderSession` over the binary wire.
#[napi(js_name = "takeSynaraHarnessPolicyForProviderSessionWire")]
pub fn take_synara_harness_policy_for_provider_session_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_TAKE_SYNARA_HARNESS_POLICY_FOR_PROVIDER_SESSION_0)?;
        let input: serde_json::Value =
            wire::read_value(r, &S_TAKE_SYNARA_HARNESS_POLICY_FOR_PROVIDER_SESSION_1)?;
        let out = &port::take_synara_harness_policy_for_provider_session(state, input)?;
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

static S_TAKE_SYNARA_HARNESS_POLICY_TEXT_PART_FOR_PROVIDER_SESSION_0: wire::Schema =
    wire::Schema::Dyn;
static S_TAKE_SYNARA_HARNESS_POLICY_TEXT_PART_FOR_PROVIDER_SESSION_1: wire::Schema =
    wire::Schema::Dyn;
static S_TAKE_SYNARA_HARNESS_POLICY_TEXT_PART_FOR_PROVIDER_SESSION_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("type", wire::Schema::Str16),
        ("text", wire::Schema::Str16),
    ]));

/// `takeSynaraHarnessPolicyTextPartForProviderSession` over the binary wire.
#[napi(js_name = "takeSynaraHarnessPolicyTextPartForProviderSessionWire")]
pub fn take_synara_harness_policy_text_part_for_provider_session_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(
            r,
            &S_TAKE_SYNARA_HARNESS_POLICY_TEXT_PART_FOR_PROVIDER_SESSION_0,
        )?;
        let input: serde_json::Value = wire::read_value(
            r,
            &S_TAKE_SYNARA_HARNESS_POLICY_TEXT_PART_FOR_PROVIDER_SESSION_1,
        )?;
        let out = &port::take_synara_harness_policy_text_part_for_provider_session(state, input)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(
                    w,
                    x0,
                    S_TAKE_SYNARA_HARNESS_POLICY_TEXT_PART_FOR_PROVIDER_SESSION_OUT.inner(),
                )?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_READ_MCP_JSON_BODY_0: wire::Schema = wire::Schema::Dyn;
static S_READ_MCP_JSON_BODY_OUT: wire::Schema = wire::Schema::Dyn;

/// `readMcpJsonBody` over the binary wire.
#[napi(js_name = "readMcpJsonBodyWire")]
pub fn read_mcp_json_body_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let request: serde_json::Value = wire::read_value(r, &S_READ_MCP_JSON_BODY_0)?;
        let max_bytes: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::read_mcp_json_body(request, max_bytes)?;
        wire::write_value(w, out, &S_READ_MCP_JSON_BODY_OUT)?;
        Ok(())
    })
}

/// `normalizeHubTaskKey` over the binary wire.
#[napi(js_name = "normalizeHubTaskKeyWire")]
pub fn normalize_hub_task_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::normalize_hub_task_key(value)?;
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

static S_HUB_WORK_WORK_ITEM_ID_0: wire::Schema = wire::Schema::Dyn;

/// `hubWorkWorkItemId` over the binary wire.
#[napi(js_name = "hubWorkWorkItemIdWire")]
pub fn hub_work_work_item_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_HUB_WORK_WORK_ITEM_ID_0)?;
        let out = &port::hub_work_work_item_id(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_HUB_WORK_TASK_FINGERPRINT_0: wire::Schema = wire::Schema::Dyn;

/// `hubWorkTaskFingerprint` over the binary wire.
#[napi(js_name = "hubWorkTaskFingerprintWire")]
pub fn hub_work_task_fingerprint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_HUB_WORK_TASK_FINGERPRINT_0)?;
        let out = &port::hub_work_task_fingerprint(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_HUB_WORK_REQUEST_FINGERPRINT_0: wire::Schema = wire::Schema::Dyn;

/// `hubWorkRequestFingerprint` over the binary wire.
#[napi(js_name = "hubWorkRequestFingerprintWire")]
pub fn hub_work_request_fingerprint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_HUB_WORK_REQUEST_FINGERPRINT_0)?;
        let out = &port::hub_work_request_fingerprint(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_RENDER_HUB_WORK_PROMPT_0: wire::Schema = wire::Schema::Dyn;

/// `renderHubWorkPrompt` over the binary wire.
#[napi(js_name = "renderHubWorkPromptWire")]
pub fn render_hub_work_prompt_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RENDER_HUB_WORK_PROMPT_0)?;
        let out = &port::render_hub_work_prompt(input)?;
        w.string16(out);
        Ok(())
    })
}

/// `buildCodexMcpConfigToml` over the binary wire.
#[napi(js_name = "buildCodexMcpConfigTomlWire")]
pub fn build_codex_mcp_config_toml_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let endpoint_url: String = r.string16()?;
        let credential_transport: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::build_codex_mcp_config_toml(endpoint_url, credential_transport)?;
        w.string16(out);
        Ok(())
    })
}

static S_BUILD_CODEX_MCP_THREAD_CONFIG_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_CODEX_MCP_THREAD_CONFIG_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildCodexMcpThreadConfig` over the binary wire.
#[napi(js_name = "buildCodexMcpThreadConfigWire")]
pub fn build_codex_mcp_thread_config_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let connection: serde_json::Value =
            wire::read_value(r, &S_BUILD_CODEX_MCP_THREAD_CONFIG_0)?;
        let out = &port::build_codex_mcp_thread_config(connection)?;
        wire::write_value(w, out, &S_BUILD_CODEX_MCP_THREAD_CONFIG_OUT)?;
        Ok(())
    })
}

static S_BUILD_OPEN_CODE_MCP_SERVER_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_OPEN_CODE_MCP_SERVER_OUT: wire::Schema = wire::Schema::Obj(&[
    ("type", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("enabled", wire::Schema::Bool),
    ("headers", wire::Schema::Map16(&wire::Schema::Str16)),
    ("oauth", wire::Schema::Bool),
]);

/// `buildOpenCodeMcpServer` over the binary wire.
#[napi(js_name = "buildOpenCodeMcpServerWire")]
pub fn build_open_code_mcp_server_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let connection: serde_json::Value = wire::read_value(r, &S_BUILD_OPEN_CODE_MCP_SERVER_0)?;
        let out = &port::build_open_code_mcp_server(connection)?;
        wire::write_value(w, out, &S_BUILD_OPEN_CODE_MCP_SERVER_OUT)?;
        Ok(())
    })
}

static S_BUILD_OPEN_CODE_CONNECTOR_MCP_SERVERS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_OPEN_CODE_CONNECTOR_MCP_SERVERS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("name", wire::Schema::Str16),
        (
            "config",
            wire::Schema::Obj(&[
                ("type", wire::Schema::Str16),
                ("url", wire::Schema::Str16),
                ("enabled", wire::Schema::Bool),
                ("headers", wire::Schema::Map16(&wire::Schema::Dyn)),
                ("oauth", wire::Schema::Bool),
            ]),
        ),
    ]));

/// `buildOpenCodeConnectorMcpServers` over the binary wire.
#[napi(js_name = "buildOpenCodeConnectorMcpServersWire")]
pub fn build_open_code_connector_mcp_servers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let connection: serde_json::Value =
            wire::read_value(r, &S_BUILD_OPEN_CODE_CONNECTOR_MCP_SERVERS_0)?;
        let out = &port::build_open_code_connector_mcp_servers(connection)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_BUILD_OPEN_CODE_CONNECTOR_MCP_SERVERS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_BUILD_CLAUDE_MCP_SERVERS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_CLAUDE_MCP_SERVERS_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Obj(&[
    ("type", wire::Schema::Str16),
    ("url", wire::Schema::Str16),
    ("headers", wire::Schema::Map16(&wire::Schema::Str16)),
]));

/// `buildClaudeMcpServers` over the binary wire.
#[napi(js_name = "buildClaudeMcpServersWire")]
pub fn build_claude_mcp_servers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let connection: serde_json::Value = wire::read_value(r, &S_BUILD_CLAUDE_MCP_SERVERS_0)?;
        let out = &port::build_claude_mcp_servers(connection)?;
        wire::write_value(w, out, &S_BUILD_CLAUDE_MCP_SERVERS_OUT)?;
        Ok(())
    })
}

static S_BUILD_ACP_SYNARA_MCP_SERVERS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_ACP_SYNARA_MCP_SERVERS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `buildAcpSynaraMcpServers` over the binary wire.
#[napi(js_name = "buildAcpSynaraMcpServersWire")]
pub fn build_acp_synara_mcp_servers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_ACP_SYNARA_MCP_SERVERS_0)?;
        let out = &port::build_acp_synara_mcp_servers(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_BUILD_ACP_SYNARA_MCP_SERVERS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_PARSE_RECOVERABLE_CREATION_PLAN_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("workspaceRoot", wire::Schema::Str16),
        ("environment", wire::Schema::Str16),
        ("worktreeRef", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("newBranch", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "plannedWorktreePath",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("ownershipPreflightPassed", wire::Schema::Bool),
        (
            "worktreeOwnership",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("operationId", wire::Schema::Str16),
                ("path", wire::Schema::Str16),
                ("branch", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("token", wire::Schema::Str16),
                ("gitDir", wire::Schema::Str16),
                ("head", wire::Schema::Str16),
                ("stateHash", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("recordedAt", wire::Schema::Str16),
            ])),
        ),
        (
            "ids",
            wire::Schema::Obj(&[
                ("threadId", wire::Schema::Str16),
                ("compensateCommandId", wire::Schema::Str16),
            ]),
        ),
    ]));

/// `parseRecoverableCreationPlan` over the binary wire.
#[napi(js_name = "parseRecoverableCreationPlanWire")]
pub fn parse_recoverable_creation_plan_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let plan_json: String = r.string16()?;
        let operation_id: String = r.string16()?;
        let out = &port::parse_recoverable_creation_plan(plan_json, operation_id)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_RECOVERABLE_CREATION_PLAN_OUT.inner())?;
        }
        Ok(())
    })
}

static S_RECORD_CREATED_WORKTREE_IN_PLAN_0: wire::Schema = wire::Schema::Dyn;

/// `recordCreatedWorktreeInPlan` over the binary wire.
#[napi(js_name = "recordCreatedWorktreeInPlanWire")]
pub fn record_created_worktree_in_plan_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RECORD_CREATED_WORKTREE_IN_PLAN_0)?;
        let out = &port::record_created_worktree_in_plan(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_REDACT_CREATION_PLAN_FOR_PURGED_CALLER_0: wire::Schema = wire::Schema::Dyn;

/// `redactCreationPlanForPurgedCaller` over the binary wire.
#[napi(js_name = "redactCreationPlanForPurgedCallerWire")]
pub fn redact_creation_plan_for_purged_caller_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_REDACT_CREATION_PLAN_FOR_PURGED_CALLER_0)?;
        let out = &port::redact_creation_plan_for_purged_caller(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_MCP_TOOL_RESULT_ERROR_OUT: wire::Schema = wire::Schema::Obj(&[
    ("content", wire::Schema::List(&wire::Schema::Dyn)),
    ("isError", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "structuredContent",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Dyn)),
    ),
]);

/// `mcpToolResultError` over the binary wire.
#[napi(js_name = "mcpToolResultErrorWire")]
pub fn mcp_tool_result_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::mcp_tool_result_error(text)?;
        wire::write_value(w, out, &S_MCP_TOOL_RESULT_ERROR_OUT)?;
        Ok(())
    })
}

static S_MCP_TOOL_RESULT_JSON_0: wire::Schema = wire::Schema::Dyn;
static S_MCP_TOOL_RESULT_JSON_OUT: wire::Schema = wire::Schema::Obj(&[
    ("content", wire::Schema::List(&wire::Schema::Dyn)),
    ("isError", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "structuredContent",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Dyn)),
    ),
]);

/// `mcpToolResultJson` over the binary wire.
#[napi(js_name = "mcpToolResultJsonWire")]
pub fn mcp_tool_result_json_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_MCP_TOOL_RESULT_JSON_0)?;
        let out = &port::mcp_tool_result_json(value)?;
        wire::write_value(w, out, &S_MCP_TOOL_RESULT_JSON_OUT)?;
        Ok(())
    })
}

static S_JSON_RPC_RESULT_0: wire::Schema = wire::Schema::Dyn;
static S_JSON_RPC_RESULT_1: wire::Schema = wire::Schema::Dyn;
static S_JSON_RPC_RESULT_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Dyn);

/// `jsonRpcResult` over the binary wire.
#[napi(js_name = "jsonRpcResultWire")]
pub fn json_rpc_result_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: serde_json::Value = wire::read_value(r, &S_JSON_RPC_RESULT_0)?;
        let result: serde_json::Value = wire::read_value(r, &S_JSON_RPC_RESULT_1)?;
        let out = &port::json_rpc_result(id, result)?;
        wire::write_value(w, out, &S_JSON_RPC_RESULT_OUT)?;
        Ok(())
    })
}

static S_JSON_RPC_ERROR_0: wire::Schema = wire::Schema::Dyn;
static S_JSON_RPC_ERROR_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Dyn);

/// `jsonRpcError` over the binary wire.
#[napi(js_name = "jsonRpcErrorWire")]
pub fn json_rpc_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: serde_json::Value = wire::read_value(r, &S_JSON_RPC_ERROR_0)?;
        let code: f64 = r.f64()?;
        let message: String = r.string16()?;
        let out = &port::json_rpc_error(id, code, message)?;
        wire::write_value(w, out, &S_JSON_RPC_ERROR_OUT)?;
        Ok(())
    })
}

static S_PARSE_MCP_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_MCP_MESSAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseMcpMessage` over the binary wire.
#[napi(js_name = "parseMcpMessageWire")]
pub fn parse_mcp_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: serde_json::Value = wire::read_value(r, &S_PARSE_MCP_MESSAGE_0)?;
        let out = &port::parse_mcp_message(raw)?;
        wire::write_value(w, out, &S_PARSE_MCP_MESSAGE_OUT)?;
        Ok(())
    })
}

static S_NEGOTIATE_MCP_PROTOCOL_VERSION_0: wire::Schema = wire::Schema::Dyn;

/// `negotiateMcpProtocolVersion` over the binary wire.
#[napi(js_name = "negotiateMcpProtocolVersionWire")]
pub fn negotiate_mcp_protocol_version_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let requested: serde_json::Value =
            wire::read_value(r, &S_NEGOTIATE_MCP_PROTOCOL_VERSION_0)?;
        let out = &port::negotiate_mcp_protocol_version(requested)?;
        w.string16(out);
        Ok(())
    })
}

static S_BUILD_MCP_INITIALIZE_RESULT_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_MCP_INITIALIZE_RESULT_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Dyn);

/// `buildMcpInitializeResult` over the binary wire.
#[napi(js_name = "buildMcpInitializeResultWire")]
pub fn build_mcp_initialize_result_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_MCP_INITIALIZE_RESULT_0)?;
        let out = &port::build_mcp_initialize_result(input)?;
        wire::write_value(w, out, &S_BUILD_MCP_INITIALIZE_RESULT_OUT)?;
        Ok(())
    })
}

static S_SANITIZE_TOOL_INPUT_SCHEMA_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_TOOL_INPUT_SCHEMA_OUT: wire::Schema = wire::Schema::Dyn;

/// `sanitizeToolInputSchema` over the binary wire.
#[napi(js_name = "sanitizeToolInputSchemaWire")]
pub fn sanitize_tool_input_schema_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let schema: serde_json::Value = wire::read_value(r, &S_SANITIZE_TOOL_INPUT_SCHEMA_0)?;
        let out = &port::sanitize_tool_input_schema(schema)?;
        wire::write_value(w, out, &S_SANITIZE_TOOL_INPUT_SCHEMA_OUT)?;
        Ok(())
    })
}

static S_IS_JSON_RECORD_0: wire::Schema = wire::Schema::Dyn;

/// `isJsonRecord` over the binary wire.
#[napi(js_name = "isJsonRecordWire")]
pub fn is_json_record_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: serde_json::Value = wire::read_value(r, &S_IS_JSON_RECORD_0)?;
        let out = &port::is_json_record(node)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_COUNT_SCHEMA_KEY_OCCURRENCES_0: wire::Schema = wire::Schema::Dyn;

/// `countSchemaKeyOccurrences` over the binary wire.
#[napi(js_name = "countSchemaKeyOccurrencesWire")]
pub fn count_schema_key_occurrences_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: serde_json::Value = wire::read_value(r, &S_COUNT_SCHEMA_KEY_OCCURRENCES_0)?;
        let key: String = r.string16()?;
        let out = &port::count_schema_key_occurrences(node, key)?;
        w.f64(*out);
        Ok(())
    })
}

static S_AGENT_GATEWAY_CAPABILITIES_FOR_0: wire::Schema = wire::Schema::Dyn;

/// `agentGatewayCapabilitiesFor` over the binary wire.
#[napi(js_name = "agentGatewayCapabilitiesForWire")]
pub fn agent_gateway_capabilities_for_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_AGENT_GATEWAY_CAPABILITIES_FOR_0)?;
        let out = &port::agent_gateway_capabilities_for(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_AGENT_GATEWAY_PROVIDER_CAPABILITIES_FOR_0: wire::Schema = wire::Schema::Dyn;

/// `agentGatewayProviderCapabilitiesFor` over the binary wire.
#[napi(js_name = "agentGatewayProviderCapabilitiesForWire")]
pub fn agent_gateway_provider_capabilities_for_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_AGENT_GATEWAY_PROVIDER_CAPABILITIES_FOR_0)?;
        let out = &port::agent_gateway_provider_capabilities_for(provider)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_AGENT_GATEWAY_SESSION_LEASE_OPTIONS_FOR_0: wire::Schema = wire::Schema::Dyn;
static S_AGENT_GATEWAY_SESSION_LEASE_OPTIONS_FOR_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[(
        "additionalCapabilities",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16)),
    )]));

/// `agentGatewaySessionLeaseOptionsFor` over the binary wire.
#[napi(js_name = "agentGatewaySessionLeaseOptionsForWire")]
pub fn agent_gateway_session_lease_options_for_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_AGENT_GATEWAY_SESSION_LEASE_OPTIONS_FOR_0)?;
        let provider: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::agent_gateway_session_lease_options_for(input, provider)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_AGENT_GATEWAY_SESSION_LEASE_OPTIONS_FOR_OUT.inner())?;
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_CAPTURE_AGENT_GATEWAY_CAPABILITY_INPUT_0: wire::Schema = wire::Schema::Dyn;
static S_CAPTURE_AGENT_GATEWAY_CAPABILITY_INPUT_OUT: wire::Schema = wire::Schema::Obj(&[(
    "enableComputerControl",
    wire::Schema::OptJs(&wire::Schema::Bool),
)]);

/// `captureAgentGatewayCapabilityInput` over the binary wire.
#[napi(js_name = "captureAgentGatewayCapabilityInputWire")]
pub fn capture_agent_gateway_capability_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_CAPTURE_AGENT_GATEWAY_CAPABILITY_INPUT_0)?;
        let out = &port::capture_agent_gateway_capability_input(input)?;
        wire::write_value(w, out, &S_CAPTURE_AGENT_GATEWAY_CAPABILITY_INPUT_OUT)?;
        Ok(())
    })
}

static S_AGENT_GATEWAY_TARGET_OPTION_GUIDANCE_0: wire::Schema = wire::Schema::Dyn;
static S_AGENT_GATEWAY_TARGET_OPTION_GUIDANCE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("primaryOptionKey", wire::Schema::Str16),
    (
        "alternativeOptionKeys",
        wire::Schema::List(&wire::Schema::Str16),
    ),
    ("optionSelectionRule", wire::Schema::Str16),
    (
        "providerOptions",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("key", wire::Schema::Str16),
            ("valueType", wire::Schema::Str16),
            ("allowedValues", wire::Schema::Dyn),
            ("allowedValuesSource", wire::Schema::Str16),
            (
                "allowsCustomValue",
                wire::Schema::OptJs(&wire::Schema::Bool),
            ),
        ])),
    ),
    ("optionsByModel", wire::Schema::Dyn),
    (
        "exampleTarget",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("provider", wire::Schema::Dyn),
            ("instanceId", wire::Schema::Str16),
            ("model", wire::Schema::Str16),
            ("options", wire::Schema::Dyn),
        ])),
    ),
]);

/// `agentGatewayTargetOptionGuidance` over the binary wire.
#[napi(js_name = "agentGatewayTargetOptionGuidanceWire")]
pub fn agent_gateway_target_option_guidance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let catalog: serde_json::Value =
            wire::read_value(r, &S_AGENT_GATEWAY_TARGET_OPTION_GUIDANCE_0)?;
        let out = &port::agent_gateway_target_option_guidance(catalog)?;
        wire::write_value(w, out, &S_AGENT_GATEWAY_TARGET_OPTION_GUIDANCE_OUT)?;
        Ok(())
    })
}

static S_READ_DIAGNOSTIC_PAGE_LIMIT_0: wire::Schema = wire::Schema::Dyn;

/// `readDiagnosticPageLimit` over the binary wire.
#[napi(js_name = "readDiagnosticPageLimitWire")]
pub fn read_diagnostic_page_limit_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_DIAGNOSTIC_PAGE_LIMIT_0)?;
        let out = &port::read_diagnostic_page_limit(args)?;
        w.f64(*out);
        Ok(())
    })
}

static S_DIAGNOSTIC_EVENT_MESSAGE_ID_0: wire::Schema = wire::Schema::Dyn;

/// `diagnosticEventMessageId` over the binary wire.
#[napi(js_name = "diagnosticEventMessageIdWire")]
pub fn diagnostic_event_message_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_DIAGNOSTIC_EVENT_MESSAGE_ID_0)?;
        let out = &port::diagnostic_event_message_id(event)?;
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

static S_GROUP_DIAGNOSTIC_EVENTS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("event", wire::Schema::Dyn),
    ("coalescedEventCount", wire::Schema::Num),
    ("oldestSequence", wire::Schema::Num),
]));

/// `groupDiagnosticEvents` over the binary wire.
#[napi(js_name = "groupDiagnosticEventsWire")]
pub fn group_diagnostic_events_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let events_newest_first: Vec<serde_json::Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let out = &port::group_diagnostic_events(events_newest_first)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_GROUP_DIAGNOSTIC_EVENTS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_SHAPE_DIAGNOSTIC_EVENTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `shapeDiagnosticEvents` over the binary wire.
#[napi(js_name = "shapeDiagnosticEventsWire")]
pub fn shape_diagnostic_events_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let events_newest_first: Vec<serde_json::Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let payload_mode: String = r.string16()?;
        let out = &port::shape_diagnostic_events(events_newest_first, payload_mode)?;
        wire::write_value(w, out, &S_SHAPE_DIAGNOSTIC_EVENTS_OUT)?;
        Ok(())
    })
}

static S_DERIVE_AGENT_THREAD_STATUS_0: wire::Schema = wire::Schema::Dyn;

/// `deriveAgentThreadStatus` over the binary wire.
#[napi(js_name = "deriveAgentThreadStatusWire")]
pub fn derive_agent_thread_status_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value = wire::read_value(r, &S_DERIVE_AGENT_THREAD_STATUS_0)?;
        let out = &port::derive_agent_thread_status(thread)?;
        w.string16(out);
        Ok(())
    })
}

static S_SUMMARIZE_THREAD_SHELL_0: wire::Schema = wire::Schema::Dyn;
static S_SUMMARIZE_THREAD_SHELL_OUT: wire::Schema = wire::Schema::Obj(&[
    ("threadId", wire::Schema::Str16),
    ("projectId", wire::Schema::Str16),
    ("title", wire::Schema::Str16),
    ("provider", wire::Schema::Str16),
    ("model", wire::Schema::Str16),
    ("status", wire::Schema::Str16),
    ("parentThreadId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("creationSource", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("envMode", wire::Schema::Str16),
    ("branch", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("worktreePath", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("archived", wire::Schema::Bool),
    ("isSelf", wire::Schema::Bool),
    ("updatedAt", wire::Schema::Str16),
]);

/// `summarizeThreadShell` over the binary wire.
#[napi(js_name = "summarizeThreadShellWire")]
pub fn summarize_thread_shell_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value = wire::read_value(r, &S_SUMMARIZE_THREAD_SHELL_0)?;
        let caller_thread_id: String = r.string16()?;
        let out = &port::summarize_thread_shell(thread, caller_thread_id)?;
        wire::write_value(w, out, &S_SUMMARIZE_THREAD_SHELL_OUT)?;
        Ok(())
    })
}

static S_SUMMARIZE_WAIT_THREAD_TEXT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("summary", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("truncated", wire::Schema::Bool),
]);

/// `summarizeWaitThreadText` over the binary wire.
#[napi(js_name = "summarizeWaitThreadTextWire")]
pub fn summarize_wait_thread_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::summarize_wait_thread_text(text)?;
        wire::write_value(w, out, &S_SUMMARIZE_WAIT_THREAD_TEXT_OUT)?;
        Ok(())
    })
}

static S_PAGINATE_THREAD_MESSAGES_0: wire::Schema = wire::Schema::Dyn;
static S_PAGINATE_THREAD_MESSAGES_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "messages",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("index", wire::Schema::Num),
            ("messageId", wire::Schema::Str16),
            ("messageVersion", wire::Schema::Str16),
            ("role", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
            ("truncated", wire::Schema::Bool),
            ("dispatchOrigin", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("createdAt", wire::Schema::Str16),
        ])),
    ),
    ("totalMessages", wire::Schema::Num),
    ("effectiveMessageLimit", wire::Schema::Num),
    ("effectiveMaxMessageChars", wire::Schema::Num),
    ("nextCursor", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "messagePage",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("index", wire::Schema::Num),
            ("messageId", wire::Schema::Str16),
            ("messageVersion", wire::Schema::Str16),
            ("offsetChars", wire::Schema::Num),
            ("endOffsetChars", wire::Schema::Num),
            ("totalChars", wire::Schema::Num),
            ("nextOffsetChars", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
]);

/// `paginateThreadMessages` over the binary wire.
#[napi(js_name = "paginateThreadMessagesWire")]
pub fn paginate_thread_messages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PAGINATE_THREAD_MESSAGES_0)?;
        let out = &port::paginate_thread_messages(input)?;
        wire::write_value(w, out, &S_PAGINATE_THREAD_MESSAGES_OUT)?;
        Ok(())
    })
}

static S_SUMMARIZE_THREAD_DETAIL_0: wire::Schema = wire::Schema::Dyn;
static S_SUMMARIZE_THREAD_DETAIL_OUT: wire::Schema = wire::Schema::Obj(&[
    ("threadId", wire::Schema::Str16),
    ("projectId", wire::Schema::Str16),
    ("title", wire::Schema::Str16),
    ("goal", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("provider", wire::Schema::Str16),
    ("model", wire::Schema::Str16),
    ("status", wire::Schema::Str16),
    ("sessionStatus", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("latestTurnState", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("parentThreadId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("creationSource", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("envMode", wire::Schema::Str16),
    ("branch", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("worktreePath", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("archived", wire::Schema::Bool),
    (
        "pullRequests",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("url", wire::Schema::Str16),
            ("number", wire::Schema::Num),
            ("title", wire::Schema::Str16),
            ("state", wire::Schema::Str16),
            ("isDraft", wire::Schema::Bool),
            ("headBranch", wire::Schema::Str16),
            ("baseBranch", wire::Schema::Str16),
            ("source", wire::Schema::Str16),
        ])),
    ),
    ("lastError", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("createdAt", wire::Schema::Str16),
    ("updatedAt", wire::Schema::Str16),
    (
        "messages",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("index", wire::Schema::Num),
            ("messageId", wire::Schema::Str16),
            ("messageVersion", wire::Schema::Str16),
            ("role", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
            ("truncated", wire::Schema::Bool),
            ("dispatchOrigin", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("createdAt", wire::Schema::Str16),
        ])),
    ),
    ("totalMessages", wire::Schema::Num),
    ("effectiveMessageLimit", wire::Schema::Num),
    ("effectiveMaxMessageChars", wire::Schema::Num),
    ("nextCursor", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "messagePage",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("index", wire::Schema::Num),
            ("messageId", wire::Schema::Str16),
            ("messageVersion", wire::Schema::Str16),
            ("offsetChars", wire::Schema::Num),
            ("endOffsetChars", wire::Schema::Num),
            ("totalChars", wire::Schema::Num),
            ("nextOffsetChars", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
]);

/// `summarizeThreadDetail` over the binary wire.
#[napi(js_name = "summarizeThreadDetailWire")]
pub fn summarize_thread_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_SUMMARIZE_THREAD_DETAIL_0)?;
        let out = &port::summarize_thread_detail(input)?;
        wire::write_value(w, out, &S_SUMMARIZE_THREAD_DETAIL_OUT)?;
        Ok(())
    })
}

static S_READ_STRING_ARG_0: wire::Schema = wire::Schema::Dyn;

/// `readStringArg` over the binary wire.
#[napi(js_name = "readStringArgWire")]
pub fn read_string_arg_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_STRING_ARG_0)?;
        let name: String = r.string16()?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::read_string_arg(args, name, options)?;
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

static S_READ_VERBATIM_STRING_ARG_0: wire::Schema = wire::Schema::Dyn;

/// `readVerbatimStringArg` over the binary wire.
#[napi(js_name = "readVerbatimStringArgWire")]
pub fn read_verbatim_string_arg_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_VERBATIM_STRING_ARG_0)?;
        let name: String = r.string16()?;
        let out = &port::read_verbatim_string_arg(args, name)?;
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

static S_READ_NUMBER_ARG_0: wire::Schema = wire::Schema::Dyn;

/// `readNumberArg` over the binary wire.
#[napi(js_name = "readNumberArgWire")]
pub fn read_number_arg_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_NUMBER_ARG_0)?;
        let name: String = r.string16()?;
        let out = &port::read_number_arg(args, name)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_READ_BOOLEAN_ARG_0: wire::Schema = wire::Schema::Dyn;

/// `readBooleanArg` over the binary wire.
#[napi(js_name = "readBooleanArgWire")]
pub fn read_boolean_arg_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_BOOLEAN_ARG_0)?;
        let name: String = r.string16()?;
        let out = &port::read_boolean_arg(args, name)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.bool(*x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_READ_ISO_TIMESTAMP_ARG_0: wire::Schema = wire::Schema::Dyn;

/// `readIsoTimestampArg` over the binary wire.
#[napi(js_name = "readIsoTimestampArgWire")]
pub fn read_iso_timestamp_arg_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_ISO_TIMESTAMP_ARG_0)?;
        let name: String = r.string16()?;
        let out = &port::read_iso_timestamp_arg(args, name)?;
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

static S_READ_RECORD_ARG_0: wire::Schema = wire::Schema::Dyn;
static S_READ_RECORD_ARG_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Dyn));

/// `readRecordArg` over the binary wire.
#[napi(js_name = "readRecordArgWire")]
pub fn read_record_arg_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_RECORD_ARG_0)?;
        let name: String = r.string16()?;
        let out = &port::read_record_arg(args, name)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_READ_RECORD_ARG_OUT.inner())?;
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_READ_STRING_ARRAY_ARG_0: wire::Schema = wire::Schema::Dyn;

/// `readStringArrayArg` over the binary wire.
#[napi(js_name = "readStringArrayArgWire")]
pub fn read_string_array_arg_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_STRING_ARRAY_ARG_0)?;
        let name: String = r.string16()?;
        let out = &port::read_string_array_arg(args, name)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.tag(1);
                w.count(x0.len());
                for x1 in x0.iter() {
                    w.string16(x1);
                }
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_PARSE_PROVIDER_KIND_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseProviderKind` over the binary wire.
#[napi(js_name = "parseProviderKindWire")]
pub fn parse_provider_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::parse_provider_kind(raw)?;
        wire::write_value(w, out, &S_PARSE_PROVIDER_KIND_OUT)?;
        Ok(())
    })
}

static S_READ_MODEL_SELECTION_ARG_0: wire::Schema = wire::Schema::Dyn;

/// `readModelSelectionArg` over the binary wire.
#[napi(js_name = "readModelSelectionArgWire")]
pub fn read_model_selection_arg_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_READ_MODEL_SELECTION_ARG_0)?;
        let name: String = r.string16()?;
        let out = &port::read_model_selection_arg(args, name)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_BUILD_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildModelSelection` over the binary wire.
#[napi(js_name = "buildModelSelectionWire")]
pub fn build_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_BUILD_MODEL_SELECTION_0)?;
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let fallback_selection: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::build_model_selection(provider, model, fallback_selection)?;
        wire::write_value(w, out, &S_BUILD_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_DECODE_CREATE_THREADS_INPUT_0: wire::Schema = wire::Schema::Dyn;
static S_DECODE_CREATE_THREADS_INPUT_OUT: wire::Schema = wire::Schema::Dyn;

/// `decodeCreateThreadsInput` over the binary wire.
#[napi(js_name = "decodeCreateThreadsInputWire")]
pub fn decode_create_threads_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_DECODE_CREATE_THREADS_INPUT_0)?;
        let out = &port::decode_create_threads_input(value)?;
        wire::write_value(w, out, &S_DECODE_CREATE_THREADS_INPUT_OUT)?;
        Ok(())
    })
}

static S_DECODE_WAIT_FOR_THREADS_INPUT_0: wire::Schema = wire::Schema::Dyn;
static S_DECODE_WAIT_FOR_THREADS_INPUT_OUT: wire::Schema = wire::Schema::Dyn;

/// `decodeWaitForThreadsInput` over the binary wire.
#[napi(js_name = "decodeWaitForThreadsInputWire")]
pub fn decode_wait_for_threads_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_DECODE_WAIT_FOR_THREADS_INPUT_0)?;
        let out = &port::decode_wait_for_threads_input(value)?;
        wire::write_value(w, out, &S_DECODE_WAIT_FOR_THREADS_INPUT_OUT)?;
        Ok(())
    })
}

static S_FILTER_TOOLS_BY_CAPABILITY_0: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[(
    "requiredCapability",
    wire::Schema::Dyn,
)]));
static S_FILTER_TOOLS_BY_CAPABILITY_1: wire::Schema = wire::Schema::Dyn;
static S_FILTER_TOOLS_BY_CAPABILITY_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[(
        "requiredCapability",
        wire::Schema::Dyn,
    )]));

/// `filterToolsByCapability` over the binary wire.
#[napi(js_name = "filterToolsByCapabilityWire")]
pub fn filter_tools_by_capability_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tools: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_FILTER_TOOLS_BY_CAPABILITY_0.inner()))?;
        let capabilities: serde_json::Value = wire::read_value(r, &S_FILTER_TOOLS_BY_CAPABILITY_1)?;
        let out = &port::filter_tools_by_capability(tools, capabilities)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_FILTER_TOOLS_BY_CAPABILITY_OUT.inner())?;
        }
        Ok(())
    })
}

static S_GATEWAY_TOOL_ERROR_RESULT_0: wire::Schema = wire::Schema::Dyn;
static S_GATEWAY_TOOL_ERROR_RESULT_OUT: wire::Schema = wire::Schema::Dyn;

/// `gatewayToolErrorResult` over the binary wire.
#[napi(js_name = "gatewayToolErrorResultWire")]
pub fn gateway_tool_error_result_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let error: serde_json::Value = wire::read_value(r, &S_GATEWAY_TOOL_ERROR_RESULT_0)?;
        let out = &port::gateway_tool_error_result(error)?;
        wire::write_value(w, out, &S_GATEWAY_TOOL_ERROR_RESULT_OUT)?;
        Ok(())
    })
}

static S_PARSE_TURN_PLAN_STEPS_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_TURN_PLAN_STEPS_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseTurnPlanSteps` over the binary wire.
#[napi(js_name = "parseTurnPlanStepsWire")]
pub fn parse_turn_plan_steps_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PARSE_TURN_PLAN_STEPS_0)?;
        let out = &port::parse_turn_plan_steps(input)?;
        wire::write_value(w, out, &S_PARSE_TURN_PLAN_STEPS_OUT)?;
        Ok(())
    })
}
