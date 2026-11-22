//! JS bindings for `apps/server/src/externalMcp/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `makeWindowsRuntimeAclPowerShellInvocation`.
#[napi(js_name = "makeWindowsRuntimeAclPowerShellInvocation")]
pub fn make_windows_runtime_acl_power_shell_invocation(
    target_path: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::make_windows_runtime_acl_power_shell_invocation(target_path)?;
        crate::js::to_json(&out)
    })
}

/// `isOwnerPrivateWindowsRuntimeAcl`.
#[napi(js_name = "isOwnerPrivateWindowsRuntimeAcl")]
pub fn is_owner_private_windows_runtime_acl(snapshot: String) -> napi::Result<bool> {
    run(move || {
        let snapshot = crate::js::from_json(&snapshot)?;
        let out = port::is_owner_private_windows_runtime_acl(snapshot)?;
        Ok(out)
    })
}

/// `resolveExternalMcpBaseDir`.
#[napi(js_name = "resolveExternalMcpBaseDir")]
pub fn resolve_external_mcp_base_dir(home_dir: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_external_mcp_base_dir(home_dir)?;
        Ok(out)
    })
}

/// `externalMcpClientStorePath`.
#[napi(js_name = "externalMcpClientStorePath")]
pub fn external_mcp_client_store_path(
    base_dir: String,
    integration_id: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::external_mcp_client_store_path(base_dir, integration_id)?;
        Ok(out)
    })
}

/// `discoverServerRuntime`.
#[napi(js_name = "discoverServerRuntime")]
pub fn discover_server_runtime(base_dir: String) -> napi::Result<String> {
    run(move || {
        let out = port::discover_server_runtime(base_dir)?;
        crate::js::to_json(&out)
    })
}

/// `discoverExternalMcpRuntime`.
#[napi(js_name = "discoverExternalMcpRuntime")]
pub fn discover_external_mcp_runtime(base_dir: String) -> napi::Result<String> {
    run(move || {
        let out = port::discover_external_mcp_runtime(base_dir)?;
        crate::js::to_json(&out)
    })
}

/// `writeExternalMcpClientCredential`.
#[napi(js_name = "writeExternalMcpClientCredential")]
pub fn write_external_mcp_client_credential(
    base_dir: String,
    paired: String,
) -> napi::Result<String> {
    run(move || {
        let paired = crate::js::from_json(&paired)?;
        let out = port::write_external_mcp_client_credential(base_dir, paired)?;
        Ok(out)
    })
}

/// `readExternalMcpClientCredential`.
#[napi(js_name = "readExternalMcpClientCredential")]
pub fn read_external_mcp_client_credential(
    base_dir: String,
    integration_id: String,
) -> napi::Result<String> {
    run(move || {
        let integration_id = crate::js::from_json(&integration_id)?;
        let out = port::read_external_mcp_client_credential(base_dir, integration_id)?;
        crate::js::to_json(&out)
    })
}

/// `readExternalMcpResponseText`.
#[napi(js_name = "readExternalMcpResponseText")]
pub async fn read_external_mcp_response_text(
    response: String,
    _timeout_ms: String,
) -> napi::Result<String> {
    run_async(async move {
        let response = crate::js::from_json(&response)?;
        let _timeout_ms = crate::js::from_json(&_timeout_ms)?;
        let out = port::read_external_mcp_response_text(response, _timeout_ms).await?;
        Ok(out)
    })
    .await
}

/// `requestTimeoutForBody`.
#[napi(js_name = "requestTimeoutForBody")]
pub fn request_timeout_for_body(body: String) -> napi::Result<f64> {
    run(move || {
        let out = port::request_timeout_for_body(body)?;
        Ok(out)
    })
}

/// `makeExternalMcpExecutionAdmission`.
#[napi(js_name = "makeExternalMcpExecutionAdmission")]
pub fn make_external_mcp_execution_admission(_slot_count: f64) -> napi::Result<String> {
    run(move || {
        let out = port::make_external_mcp_execution_admission(_slot_count)?;
        crate::js::to_json(&out)
    })
}

/// `readExternalMcpBody`.
#[napi(js_name = "readExternalMcpBody")]
pub fn read_external_mcp_body(request: String, _timeout_ms: String) -> napi::Result<String> {
    run(move || {
        let request = crate::js::from_json(&request)?;
        let _timeout_ms = crate::js::from_json(&_timeout_ms)?;
        let out = port::read_external_mcp_body(request, _timeout_ms)?;
        crate::js::to_json(&out)
    })
}

/// `readExternalMcpManagementBody`.
#[napi(js_name = "readExternalMcpManagementBody")]
pub fn read_external_mcp_management_body(
    request: String,
    _timeout_ms: String,
) -> napi::Result<String> {
    run(move || {
        let request = crate::js::from_json(&request)?;
        let _timeout_ms = crate::js::from_json(&_timeout_ms)?;
        let out = port::read_external_mcp_management_body(request, _timeout_ms)?;
        crate::js::to_json(&out)
    })
}

/// `externalMcpLauncher`.
#[napi(js_name = "externalMcpLauncher")]
pub fn external_mcp_launcher(args: Vec<String>) -> napi::Result<String> {
    run(move || {
        let out = port::external_mcp_launcher(args)?;
        crate::js::to_json(&out)
    })
}

/// `externalMcpShellCommand`.
#[napi(js_name = "externalMcpShellCommand")]
pub fn external_mcp_shell_command(config: String, platform: String) -> napi::Result<String> {
    run(move || {
        let config = crate::js::from_json(&config)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::external_mcp_shell_command(config, platform)?;
        Ok(out)
    })
}

/// `buildExternalMcpOverviewProjects`.
#[napi(js_name = "buildExternalMcpOverviewProjects")]
pub fn build_external_mcp_overview_projects(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_external_mcp_overview_projects(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildExternalMcpOverviewNextSteps`.
#[napi(js_name = "buildExternalMcpOverviewNextSteps")]
pub fn build_external_mcp_overview_next_steps(capabilities: String) -> napi::Result<Vec<String>> {
    run(move || {
        let capabilities = crate::js::from_json(&capabilities)?;
        let out = port::build_external_mcp_overview_next_steps(capabilities)?;
        Ok(out)
    })
}

/// `resolveExternalMcpRuntimePolicy`.
#[napi(js_name = "resolveExternalMcpRuntimePolicy")]
pub fn resolve_external_mcp_runtime_policy(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_external_mcp_runtime_policy(input)?;
        crate::js::to_json(&out)
    })
}

/// `computeExternalMcpRuntimeProof`.
#[napi(js_name = "computeExternalMcpRuntimeProof")]
pub fn compute_external_mcp_runtime_proof(secret: String, nonce: String) -> napi::Result<String> {
    run(move || {
        let out = port::compute_external_mcp_runtime_proof(secret, nonce)?;
        Ok(out)
    })
}

/// `runtimeProofsMatch`.
#[napi(js_name = "runtimeProofsMatch")]
pub fn runtime_proofs_match(expected: String, actual: String) -> napi::Result<bool> {
    run(move || {
        let out = port::runtime_proofs_match(expected, actual)?;
        Ok(out)
    })
}

/// `quoteExternalMcpShellArgument`.
#[napi(js_name = "quoteExternalMcpShellArgument")]
pub fn quote_external_mcp_shell_argument(value: String) -> napi::Result<String> {
    run(move || {
        let out = port::quote_external_mcp_shell_argument(value)?;
        Ok(out)
    })
}

/// `requestedExternalMcpRunId`.
#[napi(js_name = "requestedExternalMcpRunId")]
pub fn requested_external_mcp_run_id(
    input: String,
    latest_turn_id: String,
) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let latest_turn_id = crate::js::from_json(&latest_turn_id)?;
        let out = port::requested_external_mcp_run_id(input, latest_turn_id)?;
        crate::js::to_json(&out)
    })
}

/// `latestExternalMcpWaitState`.
#[napi(js_name = "latestExternalMcpWaitState")]
pub fn latest_external_mcp_wait_state(thread: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::latest_external_mcp_wait_state(thread)?;
        crate::js::to_json(&out)
    })
}

/// `terminalExternalMcpSessionStateForRun`.
#[napi(js_name = "terminalExternalMcpSessionStateForRun")]
pub fn terminal_external_mcp_session_state_for_run(
    thread: String,
    run_id: String,
) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let run_id = crate::js::from_json(&run_id)?;
        let out = port::terminal_external_mcp_session_state_for_run(thread, run_id)?;
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

static S_MAKE_WINDOWS_RUNTIME_ACL_POWER_SHELL_INVOCATION_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeWindowsRuntimeAclPowerShellInvocation` over the binary wire.
#[napi(js_name = "makeWindowsRuntimeAclPowerShellInvocationWire")]
pub fn make_windows_runtime_acl_power_shell_invocation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let target_path: String = r.string16()?;
        let out = &port::make_windows_runtime_acl_power_shell_invocation(target_path)?;
        wire::write_value(
            w,
            out,
            &S_MAKE_WINDOWS_RUNTIME_ACL_POWER_SHELL_INVOCATION_OUT,
        )?;
        Ok(())
    })
}

static S_IS_OWNER_PRIVATE_WINDOWS_RUNTIME_ACL_0: wire::Schema = wire::Schema::Dyn;

/// `isOwnerPrivateWindowsRuntimeAcl` over the binary wire.
#[napi(js_name = "isOwnerPrivateWindowsRuntimeAclWire")]
pub fn is_owner_private_windows_runtime_acl_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let snapshot: serde_json::Value =
            wire::read_value(r, &S_IS_OWNER_PRIVATE_WINDOWS_RUNTIME_ACL_0)?;
        let out = &port::is_owner_private_windows_runtime_acl(snapshot)?;
        w.bool(*out);
        Ok(())
    })
}

/// `resolveExternalMcpBaseDir` over the binary wire.
#[napi(js_name = "resolveExternalMcpBaseDirWire")]
pub fn resolve_external_mcp_base_dir_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home_dir: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::resolve_external_mcp_base_dir(home_dir)?;
        w.string16(out);
        Ok(())
    })
}

/// `externalMcpClientStorePath` over the binary wire.
#[napi(js_name = "externalMcpClientStorePathWire")]
pub fn external_mcp_client_store_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_dir: String = r.string16()?;
        let integration_id: String = r.string16()?;
        let out = &port::external_mcp_client_store_path(base_dir, integration_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_DISCOVER_SERVER_RUNTIME_OUT: wire::Schema = wire::Schema::Dyn;

/// `discoverServerRuntime` over the binary wire.
#[napi(js_name = "discoverServerRuntimeWire")]
pub fn discover_server_runtime_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_dir: String = r.string16()?;
        let out = &port::discover_server_runtime(base_dir)?;
        wire::write_value(w, out, &S_DISCOVER_SERVER_RUNTIME_OUT)?;
        Ok(())
    })
}

static S_DISCOVER_EXTERNAL_MCP_RUNTIME_OUT: wire::Schema = wire::Schema::Dyn;

/// `discoverExternalMcpRuntime` over the binary wire.
#[napi(js_name = "discoverExternalMcpRuntimeWire")]
pub fn discover_external_mcp_runtime_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_dir: String = r.string16()?;
        let out = &port::discover_external_mcp_runtime(base_dir)?;
        wire::write_value(w, out, &S_DISCOVER_EXTERNAL_MCP_RUNTIME_OUT)?;
        Ok(())
    })
}

static S_WRITE_EXTERNAL_MCP_CLIENT_CREDENTIAL_1: wire::Schema = wire::Schema::Dyn;

/// `writeExternalMcpClientCredential` over the binary wire.
#[napi(js_name = "writeExternalMcpClientCredentialWire")]
pub fn write_external_mcp_client_credential_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_dir: String = r.string16()?;
        let paired: serde_json::Value =
            wire::read_value(r, &S_WRITE_EXTERNAL_MCP_CLIENT_CREDENTIAL_1)?;
        let out = &port::write_external_mcp_client_credential(base_dir, paired)?;
        w.string16(out);
        Ok(())
    })
}

static S_READ_EXTERNAL_MCP_CLIENT_CREDENTIAL_1: wire::Schema = wire::Schema::Dyn;
static S_READ_EXTERNAL_MCP_CLIENT_CREDENTIAL_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    ("integrationId", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("credential", wire::Schema::Str16),
    ("expiresAt", wire::Schema::Str16),
    ("pairedAt", wire::Schema::Str16),
]);

/// `readExternalMcpClientCredential` over the binary wire.
#[napi(js_name = "readExternalMcpClientCredentialWire")]
pub fn read_external_mcp_client_credential_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_dir: String = r.string16()?;
        let integration_id: serde_json::Value =
            wire::read_value(r, &S_READ_EXTERNAL_MCP_CLIENT_CREDENTIAL_1)?;
        let out = &port::read_external_mcp_client_credential(base_dir, integration_id)?;
        wire::write_value(w, out, &S_READ_EXTERNAL_MCP_CLIENT_CREDENTIAL_OUT)?;
        Ok(())
    })
}

/// `requestTimeoutForBody` over the binary wire.
#[napi(js_name = "requestTimeoutForBodyWire")]
pub fn request_timeout_for_body_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let body: String = r.string16()?;
        let out = &port::request_timeout_for_body(body)?;
        w.f64(*out);
        Ok(())
    })
}

static S_MAKE_EXTERNAL_MCP_EXECUTION_ADMISSION_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeExternalMcpExecutionAdmission` over the binary wire.
#[napi(js_name = "makeExternalMcpExecutionAdmissionWire")]
pub fn make_external_mcp_execution_admission_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let _slot_count: f64 = r.f64()?;
        let out = &port::make_external_mcp_execution_admission(_slot_count)?;
        wire::write_value(w, out, &S_MAKE_EXTERNAL_MCP_EXECUTION_ADMISSION_OUT)?;
        Ok(())
    })
}

static S_READ_EXTERNAL_MCP_BODY_0: wire::Schema = wire::Schema::Dyn;
static S_READ_EXTERNAL_MCP_BODY_OUT: wire::Schema = wire::Schema::Dyn;

/// `readExternalMcpBody` over the binary wire.
#[napi(js_name = "readExternalMcpBodyWire")]
pub fn read_external_mcp_body_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let request: serde_json::Value = wire::read_value(r, &S_READ_EXTERNAL_MCP_BODY_0)?;
        let _timeout_ms: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::read_external_mcp_body(request, _timeout_ms)?;
        wire::write_value(w, out, &S_READ_EXTERNAL_MCP_BODY_OUT)?;
        Ok(())
    })
}

static S_READ_EXTERNAL_MCP_MANAGEMENT_BODY_0: wire::Schema = wire::Schema::Dyn;
static S_READ_EXTERNAL_MCP_MANAGEMENT_BODY_OUT: wire::Schema = wire::Schema::Dyn;

/// `readExternalMcpManagementBody` over the binary wire.
#[napi(js_name = "readExternalMcpManagementBodyWire")]
pub fn read_external_mcp_management_body_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let request: serde_json::Value =
            wire::read_value(r, &S_READ_EXTERNAL_MCP_MANAGEMENT_BODY_0)?;
        let _timeout_ms: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::read_external_mcp_management_body(request, _timeout_ms)?;
        wire::write_value(w, out, &S_READ_EXTERNAL_MCP_MANAGEMENT_BODY_OUT)?;
        Ok(())
    })
}

static S_EXTERNAL_MCP_LAUNCHER_OUT: wire::Schema = wire::Schema::Dyn;

/// `externalMcpLauncher` over the binary wire.
#[napi(js_name = "externalMcpLauncherWire")]
pub fn external_mcp_launcher_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::external_mcp_launcher(args)?;
        wire::write_value(w, out, &S_EXTERNAL_MCP_LAUNCHER_OUT)?;
        Ok(())
    })
}

static S_EXTERNAL_MCP_SHELL_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_EXTERNAL_MCP_SHELL_COMMAND_1: wire::Schema = wire::Schema::Dyn;

/// `externalMcpShellCommand` over the binary wire.
#[napi(js_name = "externalMcpShellCommandWire")]
pub fn external_mcp_shell_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: serde_json::Value = wire::read_value(r, &S_EXTERNAL_MCP_SHELL_COMMAND_0)?;
        let platform: serde_json::Value = wire::read_value(r, &S_EXTERNAL_MCP_SHELL_COMMAND_1)?;
        let out = &port::external_mcp_shell_command(config, platform)?;
        w.string16(out);
        Ok(())
    })
}

static S_BUILD_EXTERNAL_MCP_OVERVIEW_PROJECTS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_EXTERNAL_MCP_OVERVIEW_PROJECTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildExternalMcpOverviewProjects` over the binary wire.
#[napi(js_name = "buildExternalMcpOverviewProjectsWire")]
pub fn build_external_mcp_overview_projects_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_BUILD_EXTERNAL_MCP_OVERVIEW_PROJECTS_0)?;
        let out = &port::build_external_mcp_overview_projects(input)?;
        wire::write_value(w, out, &S_BUILD_EXTERNAL_MCP_OVERVIEW_PROJECTS_OUT)?;
        Ok(())
    })
}

static S_BUILD_EXTERNAL_MCP_OVERVIEW_NEXT_STEPS_0: wire::Schema = wire::Schema::Dyn;

/// `buildExternalMcpOverviewNextSteps` over the binary wire.
#[napi(js_name = "buildExternalMcpOverviewNextStepsWire")]
pub fn build_external_mcp_overview_next_steps_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let capabilities: serde_json::Value =
            wire::read_value(r, &S_BUILD_EXTERNAL_MCP_OVERVIEW_NEXT_STEPS_0)?;
        let out = &port::build_external_mcp_overview_next_steps(capabilities)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_RESOLVE_EXTERNAL_MCP_RUNTIME_POLICY_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_EXTERNAL_MCP_RUNTIME_POLICY_OUT: wire::Schema = wire::Schema::Obj(&[
    ("environment", wire::Schema::Str16),
    ("runtimeMode", wire::Schema::Dyn),
]);

/// `resolveExternalMcpRuntimePolicy` over the binary wire.
#[napi(js_name = "resolveExternalMcpRuntimePolicyWire")]
pub fn resolve_external_mcp_runtime_policy_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_EXTERNAL_MCP_RUNTIME_POLICY_0)?;
        let out = &port::resolve_external_mcp_runtime_policy(input)?;
        wire::write_value(w, out, &S_RESOLVE_EXTERNAL_MCP_RUNTIME_POLICY_OUT)?;
        Ok(())
    })
}

/// `computeExternalMcpRuntimeProof` over the binary wire.
#[napi(js_name = "computeExternalMcpRuntimeProofWire")]
pub fn compute_external_mcp_runtime_proof_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let secret: String = r.string16()?;
        let nonce: String = r.string16()?;
        let out = &port::compute_external_mcp_runtime_proof(secret, nonce)?;
        w.string16(out);
        Ok(())
    })
}

/// `runtimeProofsMatch` over the binary wire.
#[napi(js_name = "runtimeProofsMatchWire")]
pub fn runtime_proofs_match_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let expected: String = r.string16()?;
        let actual: String = r.string16()?;
        let out = &port::runtime_proofs_match(expected, actual)?;
        w.bool(*out);
        Ok(())
    })
}

/// `quoteExternalMcpShellArgument` over the binary wire.
#[napi(js_name = "quoteExternalMcpShellArgumentWire")]
pub fn quote_external_mcp_shell_argument_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::quote_external_mcp_shell_argument(value)?;
        w.string16(out);
        Ok(())
    })
}

static S_REQUESTED_EXTERNAL_MCP_RUN_ID_0: wire::Schema = wire::Schema::Dyn;
static S_REQUESTED_EXTERNAL_MCP_RUN_ID_1: wire::Schema = wire::Schema::Dyn;
static S_REQUESTED_EXTERNAL_MCP_RUN_ID_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `requestedExternalMcpRunId` over the binary wire.
#[napi(js_name = "requestedExternalMcpRunIdWire")]
pub fn requested_external_mcp_run_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_REQUESTED_EXTERNAL_MCP_RUN_ID_0)?;
        let latest_turn_id: serde_json::Value =
            wire::read_value(r, &S_REQUESTED_EXTERNAL_MCP_RUN_ID_1)?;
        let out = &port::requested_external_mcp_run_id(input, latest_turn_id)?;
        wire::write_value(w, out, &S_REQUESTED_EXTERNAL_MCP_RUN_ID_OUT)?;
        Ok(())
    })
}

static S_LATEST_EXTERNAL_MCP_WAIT_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_LATEST_EXTERNAL_MCP_WAIT_STATE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("runId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("state", wire::Schema::Str16),
    ]));

/// `latestExternalMcpWaitState` over the binary wire.
#[napi(js_name = "latestExternalMcpWaitStateWire")]
pub fn latest_external_mcp_wait_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value = wire::read_value(r, &S_LATEST_EXTERNAL_MCP_WAIT_STATE_0)?;
        let out = &port::latest_external_mcp_wait_state(thread)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_LATEST_EXTERNAL_MCP_WAIT_STATE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_TERMINAL_EXTERNAL_MCP_SESSION_STATE_FOR_RUN_0: wire::Schema = wire::Schema::Dyn;
static S_TERMINAL_EXTERNAL_MCP_SESSION_STATE_FOR_RUN_1: wire::Schema = wire::Schema::Dyn;

/// `terminalExternalMcpSessionStateForRun` over the binary wire.
#[napi(js_name = "terminalExternalMcpSessionStateForRunWire")]
pub fn terminal_external_mcp_session_state_for_run_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value =
            wire::read_value(r, &S_TERMINAL_EXTERNAL_MCP_SESSION_STATE_FOR_RUN_0)?;
        let run_id: serde_json::Value =
            wire::read_value(r, &S_TERMINAL_EXTERNAL_MCP_SESSION_STATE_FOR_RUN_1)?;
        let out = &port::terminal_external_mcp_session_state_for_run(thread, run_id)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
