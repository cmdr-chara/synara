//! JS bindings for `benchmarks/desktop-perf/lib/launch.mjs`: generated from the signatures in `port.rs` by rust-anything.
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

/// `assertIsolatedHome`.
#[napi(js_name = "assertIsolatedHome")]
pub fn assert_isolated_home(home: String) -> napi::Result<String> {
    run(move || {
        let home = crate::js::from_json(&home)?;
        let out = port::assert_isolated_home(home)?;
        crate::js::to_json(&out)
    })
}

/// `resolveElectronBinary`.
#[napi(js_name = "resolveElectronBinary")]
pub fn resolve_electron_binary(build_dir: String) -> napi::Result<String> {
    run(move || {
        let build_dir = crate::js::from_json(&build_dir)?;
        let out = port::resolve_electron_binary(build_dir)?;
        crate::js::to_json(&out)
    })
}

/// `freePort`.
#[napi(js_name = "freePort")]
pub async fn free_port() -> napi::Result<String> {
    run_async(async move {
        let out = port::free_port().await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `isolatedEnvironment`.
#[napi(js_name = "isolatedEnvironment")]
pub fn isolated_environment(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::isolated_environment(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `processTree`.
#[napi(js_name = "processTree")]
pub fn process_tree(root_pid: String) -> napi::Result<String> {
    run(move || {
        let root_pid = crate::js::from_json(&root_pid)?;
        let out = port::process_tree(root_pid)?;
        crate::js::to_json(&out)
    })
}

/// `classifyProcess`.
#[napi(js_name = "classifyProcess")]
pub fn classify_process(row: String, arg1: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::classify_process(row, arg1)?;
        crate::js::to_json(&out)
    })
}

/// `launchDesktop`.
#[napi(js_name = "launchDesktop")]
pub fn launch_desktop(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::launch_desktop(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `attachMainPage`.
#[napi(js_name = "attachMainPage")]
pub async fn attach_main_page(app: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let app = crate::js::from_json(&app)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::attach_main_page(app, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `quitDesktop`.
#[napi(js_name = "quitDesktop")]
pub async fn quit_desktop(app: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let app = crate::js::from_json(&app)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::quit_desktop(app, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `isFrontmost`.
#[napi(js_name = "isFrontmost")]
pub fn is_frontmost(app: String) -> napi::Result<String> {
    run(move || {
        let app = crate::js::from_json(&app)?;
        let out = port::is_frontmost(app)?;
        crate::js::to_json(&out)
    })
}

/// `secondsSinceUserInput`.
#[napi(js_name = "secondsSinceUserInput")]
pub fn seconds_since_user_input() -> napi::Result<String> {
    run(move || {
        let out = port::seconds_since_user_input()?;
        crate::js::to_json(&out)
    })
}

/// `waitForQuietMac`.
#[napi(js_name = "waitForQuietMac")]
pub async fn wait_for_quiet_mac(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::wait_for_quiet_mac(arg0).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `screenLocked`.
#[napi(js_name = "screenLocked")]
pub fn screen_locked() -> napi::Result<String> {
    run(move || {
        let out = port::screen_locked()?;
        crate::js::to_json(&out)
    })
}

/// `frontmostProblem`.
#[napi(js_name = "frontmostProblem")]
pub fn frontmost_problem(app: String, when: String) -> napi::Result<String> {
    run(move || {
        let app = crate::js::from_json(&app)?;
        let when = crate::js::from_json(&when)?;
        let out = port::frontmost_problem(app, when)?;
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

static S_ASSERT_ISOLATED_HOME_0: wire::Schema = wire::Schema::Dyn;
static S_ASSERT_ISOLATED_HOME_OUT: wire::Schema = wire::Schema::Dyn;

/// `assertIsolatedHome` over the binary wire.
#[napi(js_name = "assertIsolatedHomeWire")]
pub fn assert_isolated_home_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home: serde_json::Value = wire::read_value(r, &S_ASSERT_ISOLATED_HOME_0)?;
        let out = &port::assert_isolated_home(home)?;
        wire::write_value(w, out, &S_ASSERT_ISOLATED_HOME_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_ELECTRON_BINARY_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_ELECTRON_BINARY_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveElectronBinary` over the binary wire.
#[napi(js_name = "resolveElectronBinaryWire")]
pub fn resolve_electron_binary_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let build_dir: serde_json::Value = wire::read_value(r, &S_RESOLVE_ELECTRON_BINARY_0)?;
        let out = &port::resolve_electron_binary(build_dir)?;
        wire::write_value(w, out, &S_RESOLVE_ELECTRON_BINARY_OUT)?;
        Ok(())
    })
}

static S_ISOLATED_ENVIRONMENT_0: wire::Schema = wire::Schema::Dyn;
static S_ISOLATED_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `isolatedEnvironment` over the binary wire.
#[napi(js_name = "isolatedEnvironmentWire")]
pub fn isolated_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: serde_json::Value = wire::read_value(r, &S_ISOLATED_ENVIRONMENT_0)?;
        let out = &port::isolated_environment(arg0)?;
        wire::write_value(w, out, &S_ISOLATED_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

static S_PROCESS_TREE_0: wire::Schema = wire::Schema::Dyn;
static S_PROCESS_TREE_OUT: wire::Schema = wire::Schema::Dyn;

/// `processTree` over the binary wire.
#[napi(js_name = "processTreeWire")]
pub fn process_tree_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root_pid: serde_json::Value = wire::read_value(r, &S_PROCESS_TREE_0)?;
        let out = &port::process_tree(root_pid)?;
        wire::write_value(w, out, &S_PROCESS_TREE_OUT)?;
        Ok(())
    })
}

static S_CLASSIFY_PROCESS_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_PROCESS_1: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_PROCESS_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifyProcess` over the binary wire.
#[napi(js_name = "classifyProcessWire")]
pub fn classify_process_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_CLASSIFY_PROCESS_0)?;
        let arg1: serde_json::Value = wire::read_value(r, &S_CLASSIFY_PROCESS_1)?;
        let out = &port::classify_process(row, arg1)?;
        wire::write_value(w, out, &S_CLASSIFY_PROCESS_OUT)?;
        Ok(())
    })
}

static S_LAUNCH_DESKTOP_0: wire::Schema = wire::Schema::Dyn;
static S_LAUNCH_DESKTOP_OUT: wire::Schema = wire::Schema::Dyn;

/// `launchDesktop` over the binary wire.
#[napi(js_name = "launchDesktopWire")]
pub fn launch_desktop_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: serde_json::Value = wire::read_value(r, &S_LAUNCH_DESKTOP_0)?;
        let out = &port::launch_desktop(arg0)?;
        wire::write_value(w, out, &S_LAUNCH_DESKTOP_OUT)?;
        Ok(())
    })
}

static S_IS_FRONTMOST_0: wire::Schema = wire::Schema::Dyn;
static S_IS_FRONTMOST_OUT: wire::Schema = wire::Schema::Dyn;

/// `isFrontmost` over the binary wire.
#[napi(js_name = "isFrontmostWire")]
pub fn is_frontmost_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app: serde_json::Value = wire::read_value(r, &S_IS_FRONTMOST_0)?;
        let out = &port::is_frontmost(app)?;
        wire::write_value(w, out, &S_IS_FRONTMOST_OUT)?;
        Ok(())
    })
}

static S_SECONDS_SINCE_USER_INPUT_OUT: wire::Schema = wire::Schema::Dyn;

/// `secondsSinceUserInput` over the binary wire.
#[napi(js_name = "secondsSinceUserInputWire")]
pub fn seconds_since_user_input_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::seconds_since_user_input()?;
        wire::write_value(w, out, &S_SECONDS_SINCE_USER_INPUT_OUT)?;
        Ok(())
    })
}

static S_SCREEN_LOCKED_OUT: wire::Schema = wire::Schema::Dyn;

/// `screenLocked` over the binary wire.
#[napi(js_name = "screenLockedWire")]
pub fn screen_locked_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::screen_locked()?;
        wire::write_value(w, out, &S_SCREEN_LOCKED_OUT)?;
        Ok(())
    })
}

static S_FRONTMOST_PROBLEM_0: wire::Schema = wire::Schema::Dyn;
static S_FRONTMOST_PROBLEM_1: wire::Schema = wire::Schema::Dyn;
static S_FRONTMOST_PROBLEM_OUT: wire::Schema = wire::Schema::Dyn;

/// `frontmostProblem` over the binary wire.
#[napi(js_name = "frontmostProblemWire")]
pub fn frontmost_problem_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app: serde_json::Value = wire::read_value(r, &S_FRONTMOST_PROBLEM_0)?;
        let when: serde_json::Value = wire::read_value(r, &S_FRONTMOST_PROBLEM_1)?;
        let out = &port::frontmost_problem(app, when)?;
        wire::write_value(w, out, &S_FRONTMOST_PROBLEM_OUT)?;
        Ok(())
    })
}
