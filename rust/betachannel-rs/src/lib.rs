//! JS bindings for `apps/desktop/src/betaChannel.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `spawnDetached`.
#[napi(js_name = "spawnDetached")]
pub async fn spawn_detached(command: String, env: String) -> napi::Result<()> {
    run_async(async move {
        let env = crate::js::from_json(&env)?;
        port::spawn_detached(command, env).await?;
        Ok(())
    })
    .await
}

/// `betaLaunchEnvironment`.
#[napi(js_name = "betaLaunchEnvironment")]
pub fn beta_launch_environment(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::beta_launch_environment(input)?;
        crate::js::to_json(&out)
    })
}

/// `stableLaunchEnvironment`.
#[napi(js_name = "stableLaunchEnvironment")]
pub fn stable_launch_environment(env: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::stable_launch_environment(env)?;
        crate::js::to_json(&out)
    })
}

/// `detectStableExecutable`.
#[napi(js_name = "detectStableExecutable")]
pub fn detect_stable_executable(
    platform: String,
    home_dir: String,
    env: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let home_dir = crate::js::from_json(&home_dir)?;
        let env = crate::js::from_json(&env)?;
        let out = port::detect_stable_executable(platform, home_dir, env)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `betaMacInstallDirs`.
#[napi(js_name = "betaMacInstallDirs")]
pub fn beta_mac_install_dirs(input: String) -> napi::Result<Vec<String>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::beta_mac_install_dirs(input)?;
        Ok(out)
    })
}

/// `detectBetaInstall`.
#[napi(js_name = "detectBetaInstall")]
pub fn detect_beta_install(
    platform: String,
    home_dir: String,
    env: String,
) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let home_dir = crate::js::from_json(&home_dir)?;
        let env = crate::js::from_json(&env)?;
        let out = port::detect_beta_install(platform, home_dir, env)?;
        crate::js::to_json(&out)
    })
}

/// `isBetaServerRunning`.
#[napi(js_name = "isBetaServerRunning")]
pub fn is_beta_server_running(beta_home_dir: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_beta_server_running(beta_home_dir)?;
        Ok(out)
    })
}

/// `readBetaImportResult`.
#[napi(js_name = "readBetaImportResult")]
pub fn read_beta_import_result(beta_home_dir: String) -> napi::Result<String> {
    run(move || {
        let out = port::read_beta_import_result(beta_home_dir)?;
        crate::js::to_json(&out)
    })
}

/// `writeBetaImportRequest`.
#[napi(js_name = "writeBetaImportRequest")]
pub fn write_beta_import_request(input: String) -> napi::Result<()> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        port::write_beta_import_request(input)?;
        Ok(())
    })
}

/// `launchBetaInstall`.
#[napi(js_name = "launchBetaInstall")]
pub async fn launch_beta_install(
    detection: String,
    platform: String,
    env: String,
) -> napi::Result<()> {
    run_async(async move {
        let detection = crate::js::from_json(&detection)?;
        let platform = crate::js::from_json(&platform)?;
        let env = crate::js::from_json(&env)?;
        port::launch_beta_install(detection, platform, env).await?;
        Ok(())
    })
    .await
}

/// `resolveBetaHomeDir`.
#[napi(js_name = "resolveBetaHomeDir")]
pub fn resolve_beta_home_dir(home_dir: String, env: String) -> napi::Result<String> {
    run(move || {
        let home_dir = crate::js::from_json(&home_dir)?;
        let env = crate::js::from_json(&env)?;
        let out = port::resolve_beta_home_dir(home_dir, env)?;
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

static S_BETA_LAUNCH_ENVIRONMENT_0: wire::Schema = wire::Schema::Dyn;
static S_BETA_LAUNCH_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `betaLaunchEnvironment` over the binary wire.
#[napi(js_name = "betaLaunchEnvironmentWire")]
pub fn beta_launch_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BETA_LAUNCH_ENVIRONMENT_0)?;
        let out = &port::beta_launch_environment(input)?;
        wire::write_value(w, out, &S_BETA_LAUNCH_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

static S_STABLE_LAUNCH_ENVIRONMENT_0: wire::Schema = wire::Schema::Dyn;
static S_STABLE_LAUNCH_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `stableLaunchEnvironment` over the binary wire.
#[napi(js_name = "stableLaunchEnvironmentWire")]
pub fn stable_launch_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_STABLE_LAUNCH_ENVIRONMENT_0)?;
        let out = &port::stable_launch_environment(env)?;
        wire::write_value(w, out, &S_STABLE_LAUNCH_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

static S_DETECT_STABLE_EXECUTABLE_0: wire::Schema = wire::Schema::Dyn;
static S_DETECT_STABLE_EXECUTABLE_1: wire::Schema = wire::Schema::Dyn;
static S_DETECT_STABLE_EXECUTABLE_2: wire::Schema = wire::Schema::Dyn;

/// `detectStableExecutable` over the binary wire.
#[napi(js_name = "detectStableExecutableWire")]
pub fn detect_stable_executable_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: serde_json::Value = wire::read_value(r, &S_DETECT_STABLE_EXECUTABLE_0)?;
        let home_dir: serde_json::Value = wire::read_value(r, &S_DETECT_STABLE_EXECUTABLE_1)?;
        let env: serde_json::Value = wire::read_value(r, &S_DETECT_STABLE_EXECUTABLE_2)?;
        let out = &port::detect_stable_executable(platform, home_dir, env)?;
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

/// `betaMacInstallDirs` over the binary wire.
#[napi(js_name = "betaMacInstallDirsWire")]
pub fn beta_mac_install_dirs_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::beta_mac_install_dirs(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_DETECT_BETA_INSTALL_0: wire::Schema = wire::Schema::Dyn;
static S_DETECT_BETA_INSTALL_1: wire::Schema = wire::Schema::Dyn;
static S_DETECT_BETA_INSTALL_2: wire::Schema = wire::Schema::Dyn;
static S_DETECT_BETA_INSTALL_OUT: wire::Schema = wire::Schema::Obj(&[
    ("installed", wire::Schema::Bool),
    ("installPath", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("executablePath", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("version", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `detectBetaInstall` over the binary wire.
#[napi(js_name = "detectBetaInstallWire")]
pub fn detect_beta_install_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: serde_json::Value = wire::read_value(r, &S_DETECT_BETA_INSTALL_0)?;
        let home_dir: serde_json::Value = wire::read_value(r, &S_DETECT_BETA_INSTALL_1)?;
        let env: serde_json::Value = wire::read_value(r, &S_DETECT_BETA_INSTALL_2)?;
        let out = &port::detect_beta_install(platform, home_dir, env)?;
        wire::write_value(w, out, &S_DETECT_BETA_INSTALL_OUT)?;
        Ok(())
    })
}

/// `isBetaServerRunning` over the binary wire.
#[napi(js_name = "isBetaServerRunningWire")]
pub fn is_beta_server_running_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let beta_home_dir: String = r.string16()?;
        let out = &port::is_beta_server_running(beta_home_dir)?;
        w.bool(*out);
        Ok(())
    })
}

/// `readBetaImportResult` over the binary wire.
#[napi(js_name = "readBetaImportResultWire")]
pub fn read_beta_import_result_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let beta_home_dir: String = r.string16()?;
        let out = &port::read_beta_import_result(beta_home_dir)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_WRITE_BETA_IMPORT_REQUEST_0: wire::Schema = wire::Schema::Dyn;

/// `writeBetaImportRequest` over the binary wire.
#[napi(js_name = "writeBetaImportRequestWire")]
pub fn write_beta_import_request_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let input: serde_json::Value = wire::read_value(r, &S_WRITE_BETA_IMPORT_REQUEST_0)?;
        let () = port::write_beta_import_request(input)?;
        Ok(())
    })
}

static S_RESOLVE_BETA_HOME_DIR_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_BETA_HOME_DIR_1: wire::Schema = wire::Schema::Dyn;

/// `resolveBetaHomeDir` over the binary wire.
#[napi(js_name = "resolveBetaHomeDirWire")]
pub fn resolve_beta_home_dir_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home_dir: serde_json::Value = wire::read_value(r, &S_RESOLVE_BETA_HOME_DIR_0)?;
        let env: serde_json::Value = wire::read_value(r, &S_RESOLVE_BETA_HOME_DIR_1)?;
        let out = &port::resolve_beta_home_dir(home_dir, env)?;
        w.string16(out);
        Ok(())
    })
}
