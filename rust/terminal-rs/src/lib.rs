//! JS bindings for `apps/server/src/terminal/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `capHistoryLines`.
#[napi(js_name = "capHistoryLines")]
pub fn cap_history_lines(history: String, max_lines: f64) -> napi::Result<String> {
    run(move || {
        let out = port::cap_history_lines(history, max_lines)?;
        Ok(out)
    })
}

/// `capHistoryBytes`.
#[napi(js_name = "capHistoryBytes")]
pub fn cap_history_bytes(
    history: String,
    max_bytes: f64,
    scan_window: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let out = port::cap_history_bytes(history, max_bytes, scan_window)?;
        Ok(out)
    })
}

/// `capHistoryByLimits`.
#[napi(js_name = "capHistoryByLimits")]
pub fn cap_history_by_limits(history: String, limits: String) -> napi::Result<String> {
    run(move || {
        let limits = crate::js::from_json(&limits)?;
        let out = port::cap_history_by_limits(history, limits)?;
        Ok(out)
    })
}

/// `buildProviderProfileProcessEnv`.
#[napi(js_name = "buildProviderProfileProcessEnv")]
pub fn build_provider_profile_process_env(
    profile: String,
    base_env: String,
) -> napi::Result<String> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let base_env = crate::js::from_json(&base_env)?;
        let out = port::build_provider_profile_process_env(profile, base_env)?;
        crate::js::to_json(&out)
    })
}

/// `buildProviderProfileWrapperScript`.
#[napi(js_name = "buildProviderProfileWrapperScript")]
pub fn build_provider_profile_wrapper_script(profile: String) -> napi::Result<String> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let out = port::build_provider_profile_wrapper_script(profile)?;
        Ok(out)
    })
}

/// `prepareManagedTerminalWrappers`.
#[napi(js_name = "prepareManagedTerminalWrappers")]
pub fn prepare_managed_terminal_wrappers(options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::prepare_managed_terminal_wrappers(options)?;
        crate::js::to_json(&out)
    })
}

/// `applyManagedTerminalAgentWrapperEnv`.
#[napi(js_name = "applyManagedTerminalAgentWrapperEnv")]
pub fn apply_managed_terminal_agent_wrapper_env(
    env: String,
    wrapper_state: String,
) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let wrapper_state = crate::js::from_json(&wrapper_state)?;
        let out = port::apply_managed_terminal_agent_wrapper_env(env, wrapper_state)?;
        crate::js::to_json(&out)
    })
}

/// `prepareManagedTerminalAgentWrappers`.
#[napi(js_name = "prepareManagedTerminalAgentWrappers")]
pub fn prepare_managed_terminal_agent_wrappers(options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::prepare_managed_terminal_agent_wrappers(options)?;
        crate::js::to_json(&out)
    })
}

/// `prepareProviderAuthenticationSettings`.
#[napi(js_name = "prepareProviderAuthenticationSettings")]
pub fn prepare_provider_authentication_settings(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::prepare_provider_authentication_settings(input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveProviderAuthenticationLaunch`.
#[napi(js_name = "resolveProviderAuthenticationLaunch")]
pub async fn resolve_provider_authentication_launch(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_provider_authentication_launch(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `deriveManagedTerminalProfiles`.
#[napi(js_name = "deriveManagedTerminalProfiles")]
pub async fn derive_managed_terminal_profiles(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_managed_terminal_profiles(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `selectPtyAdapterRuntime`.
#[napi(js_name = "selectPtyAdapterRuntime")]
pub fn select_pty_adapter_runtime(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::select_pty_adapter_runtime(input)?;
        crate::js::to_json(&out)
    })
}

/// `inspectSubprocessActivity`.
#[napi(js_name = "inspectSubprocessActivity")]
pub fn inspect_subprocess_activity(
    parent_pid: f64,
    children_by_parent_pid: String,
) -> napi::Result<String> {
    run(move || {
        let children_by_parent_pid = crate::js::from_json(&children_by_parent_pid)?;
        let out = port::inspect_subprocess_activity(parent_pid, children_by_parent_pid)?;
        crate::js::to_json(&out)
    })
}

/// `captureProcessChildrenMap`.
#[napi(js_name = "captureProcessChildrenMap")]
pub async fn capture_process_children_map() -> napi::Result<String> {
    run_async(async move {
        let out = port::capture_process_children_map().await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `defaultSubprocessChecker`.
#[napi(js_name = "defaultSubprocessChecker")]
pub async fn default_subprocess_checker(terminal_pid: f64) -> napi::Result<String> {
    run_async(async move {
        let out = port::default_subprocess_checker(terminal_pid).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// Class `TerminalThreadTitleTracker`.
#[napi(js_name = "TerminalThreadTitleTracker")]
pub struct TerminalThreadTitleTracker {
    inner: port::TerminalThreadTitleTracker,
}

#[napi]
impl TerminalThreadTitleTracker {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        run(move || {
            let out = port::TerminalThreadTitleTracker::new()?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "reset")]
    pub fn reset(&mut self, thread_id: String, terminal_id: Option<String>) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            inner.reset(thread_id, terminal_id)?;
            Ok(())
        })
    }

    #[napi(js_name = "consumeWrite")]
    pub fn consume_write(&mut self, input: String) -> napi::Result<String> {
        let inner = &mut self.inner;
        run(move || {
            let input = crate::js::from_json(&input)?;
            let out = inner.consume_write(input)?;
            crate::js::to_json(&out)
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

/// `capHistoryLines` over the binary wire.
#[napi(js_name = "capHistoryLinesWire")]
pub fn cap_history_lines_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let history: String = r.string16()?;
        let max_lines: f64 = r.f64()?;
        let out = &port::cap_history_lines(history, max_lines)?;
        w.string16(out);
        Ok(())
    })
}

/// `capHistoryBytes` over the binary wire.
#[napi(js_name = "capHistoryBytesWire")]
pub fn cap_history_bytes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let history: String = r.string16()?;
        let max_bytes: f64 = r.f64()?;
        let scan_window: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::cap_history_bytes(history, max_bytes, scan_window)?;
        w.string16(out);
        Ok(())
    })
}

static S_CAP_HISTORY_BY_LIMITS_1: wire::Schema = wire::Schema::Dyn;

/// `capHistoryByLimits` over the binary wire.
#[napi(js_name = "capHistoryByLimitsWire")]
pub fn cap_history_by_limits_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let history: String = r.string16()?;
        let limits: serde_json::Value = wire::read_value(r, &S_CAP_HISTORY_BY_LIMITS_1)?;
        let out = &port::cap_history_by_limits(history, limits)?;
        w.string16(out);
        Ok(())
    })
}

static S_BUILD_PROVIDER_PROFILE_PROCESS_ENV_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PROVIDER_PROFILE_PROCESS_ENV_1: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PROVIDER_PROFILE_PROCESS_ENV_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildProviderProfileProcessEnv` over the binary wire.
#[napi(js_name = "buildProviderProfileProcessEnvWire")]
pub fn build_provider_profile_process_env_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: serde_json::Value =
            wire::read_value(r, &S_BUILD_PROVIDER_PROFILE_PROCESS_ENV_0)?;
        let base_env: serde_json::Value =
            wire::read_value(r, &S_BUILD_PROVIDER_PROFILE_PROCESS_ENV_1)?;
        let out = &port::build_provider_profile_process_env(profile, base_env)?;
        wire::write_value(w, out, &S_BUILD_PROVIDER_PROFILE_PROCESS_ENV_OUT)?;
        Ok(())
    })
}

static S_BUILD_PROVIDER_PROFILE_WRAPPER_SCRIPT_0: wire::Schema = wire::Schema::Dyn;

/// `buildProviderProfileWrapperScript` over the binary wire.
#[napi(js_name = "buildProviderProfileWrapperScriptWire")]
pub fn build_provider_profile_wrapper_script_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: serde_json::Value =
            wire::read_value(r, &S_BUILD_PROVIDER_PROFILE_WRAPPER_SCRIPT_0)?;
        let out = &port::build_provider_profile_wrapper_script(profile)?;
        w.string16(out);
        Ok(())
    })
}

static S_PREPARE_MANAGED_TERMINAL_WRAPPERS_0: wire::Schema = wire::Schema::Dyn;
static S_PREPARE_MANAGED_TERMINAL_WRAPPERS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("binDir", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("codexHomeDir", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("hookScriptPath", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "claudeSettingsPath",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("zshDir", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("targetPathByCliKind", wire::Schema::Dyn),
]);

/// `prepareManagedTerminalWrappers` over the binary wire.
#[napi(js_name = "prepareManagedTerminalWrappersWire")]
pub fn prepare_managed_terminal_wrappers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let options: serde_json::Value =
            wire::read_value(r, &S_PREPARE_MANAGED_TERMINAL_WRAPPERS_0)?;
        let out = &port::prepare_managed_terminal_wrappers(options)?;
        wire::write_value(w, out, &S_PREPARE_MANAGED_TERMINAL_WRAPPERS_OUT)?;
        Ok(())
    })
}

static S_APPLY_MANAGED_TERMINAL_AGENT_WRAPPER_ENV_0: wire::Schema = wire::Schema::Dyn;
static S_APPLY_MANAGED_TERMINAL_AGENT_WRAPPER_ENV_1: wire::Schema = wire::Schema::Dyn;
static S_APPLY_MANAGED_TERMINAL_AGENT_WRAPPER_ENV_OUT: wire::Schema = wire::Schema::Dyn;

/// `applyManagedTerminalAgentWrapperEnv` over the binary wire.
#[napi(js_name = "applyManagedTerminalAgentWrapperEnvWire")]
pub fn apply_managed_terminal_agent_wrapper_env_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value =
            wire::read_value(r, &S_APPLY_MANAGED_TERMINAL_AGENT_WRAPPER_ENV_0)?;
        let wrapper_state: serde_json::Value =
            wire::read_value(r, &S_APPLY_MANAGED_TERMINAL_AGENT_WRAPPER_ENV_1)?;
        let out = &port::apply_managed_terminal_agent_wrapper_env(env, wrapper_state)?;
        wire::write_value(w, out, &S_APPLY_MANAGED_TERMINAL_AGENT_WRAPPER_ENV_OUT)?;
        Ok(())
    })
}

static S_PREPARE_MANAGED_TERMINAL_AGENT_WRAPPERS_0: wire::Schema = wire::Schema::Dyn;
static S_PREPARE_MANAGED_TERMINAL_AGENT_WRAPPERS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("binDir", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("codexHomeDir", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("hookScriptPath", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "claudeSettingsPath",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("zshDir", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("targetPathByCliKind", wire::Schema::Dyn),
]);

/// `prepareManagedTerminalAgentWrappers` over the binary wire.
#[napi(js_name = "prepareManagedTerminalAgentWrappersWire")]
pub fn prepare_managed_terminal_agent_wrappers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let options: serde_json::Value =
            wire::read_value(r, &S_PREPARE_MANAGED_TERMINAL_AGENT_WRAPPERS_0)?;
        let out = &port::prepare_managed_terminal_agent_wrappers(options)?;
        wire::write_value(w, out, &S_PREPARE_MANAGED_TERMINAL_AGENT_WRAPPERS_OUT)?;
        Ok(())
    })
}

static S_PREPARE_PROVIDER_AUTHENTICATION_SETTINGS_0: wire::Schema = wire::Schema::Dyn;
static S_PREPARE_PROVIDER_AUTHENTICATION_SETTINGS_OUT: wire::Schema = wire::Schema::Dyn;

/// `prepareProviderAuthenticationSettings` over the binary wire.
#[napi(js_name = "prepareProviderAuthenticationSettingsWire")]
pub fn prepare_provider_authentication_settings_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_PREPARE_PROVIDER_AUTHENTICATION_SETTINGS_0)?;
        let out = &port::prepare_provider_authentication_settings(input)?;
        wire::write_value(w, out, &S_PREPARE_PROVIDER_AUTHENTICATION_SETTINGS_OUT)?;
        Ok(())
    })
}

static S_SELECT_PTY_ADAPTER_RUNTIME_0: wire::Schema = wire::Schema::Dyn;
static S_SELECT_PTY_ADAPTER_RUNTIME_OUT: wire::Schema = wire::Schema::Str16;

/// `selectPtyAdapterRuntime` over the binary wire.
#[napi(js_name = "selectPtyAdapterRuntimeWire")]
pub fn select_pty_adapter_runtime_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_SELECT_PTY_ADAPTER_RUNTIME_0)?;
        let out = &port::select_pty_adapter_runtime(input)?;
        wire::write_value(w, out, &S_SELECT_PTY_ADAPTER_RUNTIME_OUT)?;
        Ok(())
    })
}

static S_INSPECT_SUBPROCESS_ACTIVITY_1: wire::Schema = wire::Schema::Dyn;
static S_INSPECT_SUBPROCESS_ACTIVITY_OUT: wire::Schema = wire::Schema::Obj(&[
    ("cliKind", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("hasRunningSubprocess", wire::Schema::Bool),
    ("hasProviderDescendant", wire::Schema::Bool),
    ("hasNonProviderSubprocess", wire::Schema::Bool),
]);

/// `inspectSubprocessActivity` over the binary wire.
#[napi(js_name = "inspectSubprocessActivityWire")]
pub fn inspect_subprocess_activity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let parent_pid: f64 = r.f64()?;
        let children_by_parent_pid: serde_json::Value =
            wire::read_value(r, &S_INSPECT_SUBPROCESS_ACTIVITY_1)?;
        let out = &port::inspect_subprocess_activity(parent_pid, children_by_parent_pid)?;
        wire::write_value(w, out, &S_INSPECT_SUBPROCESS_ACTIVITY_OUT)?;
        Ok(())
    })
}
