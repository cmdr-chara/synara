//! JS bindings for `apps/server/src/device/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `fitDeviceMessage`.
#[napi(js_name = "fitDeviceMessage")]
pub fn fit_device_message(message: String) -> napi::Result<String> {
    run(move || {
        let out = port::fit_device_message(message)?;
        crate::js::to_json(&out)
    })
}

/// `parseSimctlDevices`.
#[napi(js_name = "parseSimctlDevices")]
pub fn parse_simctl_devices(json_text: String, catalogue: String) -> napi::Result<String> {
    run(move || {
        let catalogue = crate::js::from_json(&catalogue)?;
        let out = port::parse_simctl_devices(json_text, catalogue)?;
        crate::js::to_json(&out)
    })
}

/// `hasBootableIosRuntime`.
#[napi(js_name = "hasBootableIosRuntime")]
pub fn has_bootable_ios_runtime(devices: String) -> napi::Result<bool> {
    run(move || {
        let devices = crate::js::from_json(&devices)?;
        let out = port::has_bootable_ios_runtime(devices)?;
        Ok(out)
    })
}

/// `orderXcodeAppCandidates`.
#[napi(js_name = "orderXcodeAppCandidates")]
pub fn order_xcode_app_candidates(entries: Vec<String>) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::order_xcode_app_candidates(entries)?;
        Ok(out)
    })
}

/// `selectRecordingDirectory`.
#[napi(js_name = "selectRecordingDirectory")]
pub async fn select_recording_directory(
    candidates: Vec<String>,
    fallback: String,
) -> napi::Result<String> {
    run_async(async move {
        let out = port::select_recording_directory(candidates, fallback).await?;
        Ok(out)
    })
    .await
}

/// `normalizeUiNode`.
#[napi(js_name = "normalizeUiNode")]
pub fn normalize_ui_node(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::normalize_ui_node(raw)?;
        crate::js::to_json(&out)
    })
}

/// `readPngDimensions`.
#[napi(js_name = "readPngDimensions")]
pub fn read_png_dimensions(bytes: String) -> napi::Result<String> {
    run(move || {
        let bytes = crate::js::from_json(&bytes)?;
        let out = port::read_png_dimensions(bytes)?;
        crate::js::to_json(&out)
    })
}

/// `processIsAlive`.
#[napi(js_name = "processIsAlive")]
pub fn process_is_alive(pid: f64) -> napi::Result<bool> {
    run(move || {
        let out = port::process_is_alive(pid)?;
        Ok(out)
    })
}

/// `decodeResyncRequest`.
#[napi(js_name = "decodeResyncRequest")]
pub fn decode_resync_request(
    message: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let message = crate::js::from_json(&message)?;
        let out = port::decode_resync_request(message)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `unifiedLogArguments`.
#[napi(js_name = "unifiedLogArguments")]
pub fn unified_log_arguments(executable: String, query: String) -> napi::Result<Vec<String>> {
    run(move || {
        let query = crate::js::from_json(&query)?;
        let out = port::unified_log_arguments(executable, query)?;
        Ok(out)
    })
}

/// `tailLines`.
#[napi(js_name = "tailLines")]
pub fn tail_lines(lines: Vec<String>, limit: f64) -> napi::Result<String> {
    run(move || {
        let out = port::tail_lines(lines, limit)?;
        crate::js::to_json(&out)
    })
}

/// `parseCrashReport`.
#[napi(js_name = "parseCrashReport")]
pub fn parse_crash_report(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_crash_report(text)?;
        crate::js::to_json(&out)
    })
}

/// `readCrashReports`.
#[napi(js_name = "readCrashReports")]
pub async fn read_crash_reports(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::read_crash_reports(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `parseSimctlDeviceTypes`.
#[napi(js_name = "parseSimctlDeviceTypes")]
pub fn parse_simctl_device_types(json_text: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_simctl_device_types(json_text)?;
        crate::js::to_json(&out)
    })
}

/// `parseDeviceTypeProfile`.
#[napi(js_name = "parseDeviceTypeProfile")]
pub fn parse_device_type_profile(json_text: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_device_type_profile(json_text)?;
        crate::js::to_json(&out)
    })
}

/// `readDeviceTypeCatalogue`.
#[napi(js_name = "readDeviceTypeCatalogue")]
pub async fn read_device_type_catalogue(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::read_device_type_catalogue(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `effectiveValue`.
#[napi(js_name = "effectiveValue")]
pub fn effective_value(node: String) -> napi::Result<String> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let out = port::effective_value(node)?;
        crate::js::to_json(&out)
    })
}

/// `deviceKeyboardTop`.
#[napi(js_name = "deviceKeyboardTop")]
pub fn device_keyboard_top(root: String) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::device_keyboard_top(root)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isOnScreen`.
#[napi(js_name = "isOnScreen")]
pub fn is_on_screen(node: String, root: String) -> napi::Result<bool> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let root = crate::js::from_json(&root)?;
        let out = port::is_on_screen(node, root)?;
        Ok(out)
    })
}

/// `formatUiSnapshot`.
#[napi(js_name = "formatUiSnapshot")]
pub fn format_ui_snapshot(root: String, options: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let options = crate::js::from_json(&options)?;
        let out = port::format_ui_snapshot(root, options)?;
        crate::js::to_json(&out)
    })
}

/// `uiFingerprint`.
#[napi(js_name = "uiFingerprint")]
pub fn ui_fingerprint(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::ui_fingerprint(root)?;
        Ok(out)
    })
}

/// `findFocusedNode`.
#[napi(js_name = "findFocusedNode")]
pub fn find_focused_node(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::find_focused_node(root)?;
        crate::js::to_json(&out)
    })
}

/// `isSecureField`.
#[napi(js_name = "isSecureField")]
pub fn is_secure_field(node: String) -> napi::Result<bool> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let out = port::is_secure_field(node)?;
        Ok(out)
    })
}

/// `parseHelperProbe`.
#[napi(js_name = "parseHelperProbe")]
pub fn parse_helper_probe(stdout: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_helper_probe(stdout)?;
        crate::js::to_json(&out)
    })
}

/// `describeToolchain`.
#[napi(js_name = "describeToolchain")]
pub fn describe_toolchain(toolchain: String) -> napi::Result<String> {
    run(move || {
        let toolchain = crate::js::from_json(&toolchain)?;
        let out = port::describe_toolchain(toolchain)?;
        Ok(out)
    })
}

/// `describeBrokenCapabilities`.
#[napi(js_name = "describeBrokenCapabilities")]
pub fn describe_broken_capabilities(broken: String, toolchain: String) -> napi::Result<String> {
    run(move || {
        let broken = crate::js::from_json(&broken)?;
        let toolchain = crate::js::from_json(&toolchain)?;
        let out = port::describe_broken_capabilities(broken, toolchain)?;
        Ok(out)
    })
}

/// `capabilityUnavailableMessage`.
#[napi(js_name = "capabilityUnavailableMessage")]
pub fn capability_unavailable_message(
    capability: String,
    toolchain: String,
) -> napi::Result<String> {
    run(move || {
        let capability = crate::js::from_json(&capability)?;
        let toolchain = crate::js::from_json(&toolchain)?;
        let out = port::capability_unavailable_message(capability, toolchain)?;
        Ok(out)
    })
}

/// `availabilityFromProbe`.
#[napi(js_name = "availabilityFromProbe")]
pub fn availability_from_probe(probe: String) -> napi::Result<String> {
    run(move || {
        let probe = crate::js::from_json(&probe)?;
        let out = port::availability_from_probe(probe)?;
        crate::js::to_json(&out)
    })
}

/// `xcodeAppRoot`.
#[napi(js_name = "xcodeAppRoot")]
pub fn xcode_app_root(developer_dir: String) -> napi::Result<String> {
    run(move || {
        let out = port::xcode_app_root(developer_dir)?;
        Ok(out)
    })
}

/// `sandboxDisabled`.
#[napi(js_name = "sandboxDisabled")]
pub fn sandbox_disabled(env: String) -> napi::Result<bool> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::sandbox_disabled(env)?;
        Ok(out)
    })
}

/// `sandboxedHelperCommand`.
#[napi(js_name = "sandboxedHelperCommand")]
pub async fn sandboxed_helper_command(argv: Vec<String>, context: String) -> napi::Result<String> {
    run_async(async move {
        let context = crate::js::from_json(&context)?;
        let out = port::sandboxed_helper_command(argv, context).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `describeSandboxSuspicion`.
#[napi(js_name = "describeSandboxSuspicion")]
pub fn describe_sandbox_suspicion(profile_path: String) -> napi::Result<String> {
    run(move || {
        let profile_path = crate::js::from_json(&profile_path)?;
        let out = port::describe_sandbox_suspicion(profile_path)?;
        Ok(out)
    })
}

/// `targetPhrase`.
#[napi(js_name = "targetPhrase")]
pub fn target_phrase(target: String) -> napi::Result<String> {
    run(move || {
        let target = crate::js::from_json(&target)?;
        let out = port::target_phrase(target)?;
        Ok(out)
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

/// `isTargetable`.
#[napi(js_name = "isTargetable")]
pub fn is_targetable(node: String) -> napi::Result<bool> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let out = port::is_targetable(node)?;
        Ok(out)
    })
}

/// `visibleLabels`.
#[napi(js_name = "visibleLabels")]
pub fn visible_labels(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::visible_labels(root)?;
        crate::js::to_json(&out)
    })
}

/// `findTarget`.
#[napi(js_name = "findTarget")]
pub fn find_target(root: String, target: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let target = crate::js::from_json(&target)?;
        let out = port::find_target(root, target)?;
        crate::js::to_json(&out)
    })
}

/// `planScrollStep`.
#[napi(js_name = "planScrollStep")]
pub fn plan_scroll_step(node: String, root: String) -> napi::Result<String> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let root = crate::js::from_json(&root)?;
        let out = port::plan_scroll_step(node, root)?;
        crate::js::to_json(&out)
    })
}

/// `readTapRequest`.
#[napi(js_name = "readTapRequest")]
pub fn read_tap_request(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::read_tap_request(input)?;
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

static S_FIT_DEVICE_MESSAGE_OUT: wire::Schema = wire::Schema::Str16;

/// `fitDeviceMessage` over the binary wire.
#[napi(js_name = "fitDeviceMessageWire")]
pub fn fit_device_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: String = r.string16()?;
        let out = &port::fit_device_message(message)?;
        wire::write_value(w, out, &S_FIT_DEVICE_MESSAGE_OUT)?;
        Ok(())
    })
}

static S_PARSE_SIMCTL_DEVICES_1: wire::Schema = wire::Schema::Dyn;

/// `parseSimctlDevices` over the binary wire.
#[napi(js_name = "parseSimctlDevicesWire")]
pub fn parse_simctl_devices_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let json_text: String = r.string16()?;
        let catalogue: serde_json::Value = wire::read_value(r, &S_PARSE_SIMCTL_DEVICES_1)?;
        let out = &port::parse_simctl_devices(json_text, catalogue)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_HAS_BOOTABLE_IOS_RUNTIME_0: wire::Schema = wire::Schema::Dyn;

/// `hasBootableIosRuntime` over the binary wire.
#[napi(js_name = "hasBootableIosRuntimeWire")]
pub fn has_bootable_ios_runtime_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let devices: serde_json::Value = wire::read_value(r, &S_HAS_BOOTABLE_IOS_RUNTIME_0)?;
        let out = &port::has_bootable_ios_runtime(devices)?;
        w.bool(*out);
        Ok(())
    })
}

/// `orderXcodeAppCandidates` over the binary wire.
#[napi(js_name = "orderXcodeAppCandidatesWire")]
pub fn order_xcode_app_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let entries: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::order_xcode_app_candidates(entries)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_NORMALIZE_UI_NODE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_UI_NODE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("role", wire::Schema::Str16),
    ("subrole", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("frame", wire::Schema::Dyn),
    ("activationPoint", wire::Schema::Dyn),
    ("identifier", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("enabled", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("focused", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("selected", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("placeholder", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("unframed", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "children",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("role", wire::Schema::Str16),
            ("subrole", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("frame", wire::Schema::Dyn),
            ("activationPoint", wire::Schema::Dyn),
            ("identifier", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("enabled", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("focused", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("selected", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("placeholder", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("unframed", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("children", wire::Schema::List(&wire::Schema::Dyn)),
        ])),
    ),
]);

/// `normalizeUiNode` over the binary wire.
#[napi(js_name = "normalizeUiNodeWire")]
pub fn normalize_ui_node_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: serde_json::Value = wire::read_value(r, &S_NORMALIZE_UI_NODE_0)?;
        let out = &port::normalize_ui_node(raw)?;
        wire::write_value(w, out, &S_NORMALIZE_UI_NODE_OUT)?;
        Ok(())
    })
}

static S_READ_PNG_DIMENSIONS_0: wire::Schema = wire::Schema::Dyn;
static S_READ_PNG_DIMENSIONS_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("width", wire::Schema::Num),
    ("height", wire::Schema::Num),
]));

/// `readPngDimensions` over the binary wire.
#[napi(js_name = "readPngDimensionsWire")]
pub fn read_png_dimensions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let bytes: serde_json::Value = wire::read_value(r, &S_READ_PNG_DIMENSIONS_0)?;
        let out = &port::read_png_dimensions(bytes)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_READ_PNG_DIMENSIONS_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `processIsAlive` over the binary wire.
#[napi(js_name = "processIsAliveWire")]
pub fn process_is_alive_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pid: f64 = r.f64()?;
        let out = &port::process_is_alive(pid)?;
        w.bool(*out);
        Ok(())
    })
}

static S_DECODE_RESYNC_REQUEST_0: wire::Schema = wire::Schema::Dyn;

/// `decodeResyncRequest` over the binary wire.
#[napi(js_name = "decodeResyncRequestWire")]
pub fn decode_resync_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: serde_json::Value = wire::read_value(r, &S_DECODE_RESYNC_REQUEST_0)?;
        let out = &port::decode_resync_request(message)?;
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

static S_UNIFIED_LOG_ARGUMENTS_1: wire::Schema = wire::Schema::Dyn;

/// `unifiedLogArguments` over the binary wire.
#[napi(js_name = "unifiedLogArgumentsWire")]
pub fn unified_log_arguments_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let executable: String = r.string16()?;
        let query: serde_json::Value = wire::read_value(r, &S_UNIFIED_LOG_ARGUMENTS_1)?;
        let out = &port::unified_log_arguments(executable, query)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_TAIL_LINES_OUT: wire::Schema = wire::Schema::Obj(&[
    ("lines", wire::Schema::List(&wire::Schema::Str16)),
    ("omitted", wire::Schema::Num),
]);

/// `tailLines` over the binary wire.
#[napi(js_name = "tailLinesWire")]
pub fn tail_lines_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let lines: Vec<String> = r.list(|r| r.string16())?;
        let limit: f64 = r.f64()?;
        let out = &port::tail_lines(lines, limit)?;
        wire::write_value(w, out, &S_TAIL_LINES_OUT)?;
        Ok(())
    })
}

static S_PARSE_CRASH_REPORT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("procPath", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("exception", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("reason", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `parseCrashReport` over the binary wire.
#[napi(js_name = "parseCrashReportWire")]
pub fn parse_crash_report_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::parse_crash_report(text)?;
        wire::write_value(w, out, &S_PARSE_CRASH_REPORT_OUT)?;
        Ok(())
    })
}

static S_PARSE_SIMCTL_DEVICE_TYPES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("identifier", wire::Schema::Str16),
    ("family", wire::Schema::Dyn),
    ("profilePath", wire::Schema::Str16),
]));

/// `parseSimctlDeviceTypes` over the binary wire.
#[napi(js_name = "parseSimctlDeviceTypesWire")]
pub fn parse_simctl_device_types_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let json_text: String = r.string16()?;
        let out = &port::parse_simctl_device_types(json_text)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_SIMCTL_DEVICE_TYPES_OUT.inner())?;
        }
        Ok(())
    })
}

/// `parseDeviceTypeProfile` over the binary wire.
#[napi(js_name = "parseDeviceTypeProfileWire")]
pub fn parse_device_type_profile_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let json_text: String = r.string16()?;
        let out = &port::parse_device_type_profile(json_text)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_EFFECTIVE_VALUE_0: wire::Schema = wire::Schema::Dyn;
static S_EFFECTIVE_VALUE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `effectiveValue` over the binary wire.
#[napi(js_name = "effectiveValueWire")]
pub fn effective_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: serde_json::Value = wire::read_value(r, &S_EFFECTIVE_VALUE_0)?;
        let out = &port::effective_value(node)?;
        wire::write_value(w, out, &S_EFFECTIVE_VALUE_OUT)?;
        Ok(())
    })
}

static S_DEVICE_KEYBOARD_TOP_0: wire::Schema = wire::Schema::Dyn;

/// `deviceKeyboardTop` over the binary wire.
#[napi(js_name = "deviceKeyboardTopWire")]
pub fn device_keyboard_top_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_DEVICE_KEYBOARD_TOP_0)?;
        let out = &port::device_keyboard_top(root)?;
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

static S_IS_ON_SCREEN_0: wire::Schema = wire::Schema::Dyn;
static S_IS_ON_SCREEN_1: wire::Schema = wire::Schema::Dyn;

/// `isOnScreen` over the binary wire.
#[napi(js_name = "isOnScreenWire")]
pub fn is_on_screen_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: serde_json::Value = wire::read_value(r, &S_IS_ON_SCREEN_0)?;
        let root: serde_json::Value = wire::read_value(r, &S_IS_ON_SCREEN_1)?;
        let out = &port::is_on_screen(node, root)?;
        w.bool(*out);
        Ok(())
    })
}

static S_FORMAT_UI_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_FORMAT_UI_SNAPSHOT_1: wire::Schema = wire::Schema::Dyn;
static S_FORMAT_UI_SNAPSHOT_OUT: wire::Schema = wire::Schema::Str16;

/// `formatUiSnapshot` over the binary wire.
#[napi(js_name = "formatUiSnapshotWire")]
pub fn format_ui_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_FORMAT_UI_SNAPSHOT_0)?;
        let options: serde_json::Value = wire::read_value(r, &S_FORMAT_UI_SNAPSHOT_1)?;
        let out = &port::format_ui_snapshot(root, options)?;
        wire::write_value(w, out, &S_FORMAT_UI_SNAPSHOT_OUT)?;
        Ok(())
    })
}

static S_UI_FINGERPRINT_0: wire::Schema = wire::Schema::Dyn;

/// `uiFingerprint` over the binary wire.
#[napi(js_name = "uiFingerprintWire")]
pub fn ui_fingerprint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_UI_FINGERPRINT_0)?;
        let out = &port::ui_fingerprint(root)?;
        w.string16(out);
        Ok(())
    })
}

static S_FIND_FOCUSED_NODE_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_FOCUSED_NODE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("role", wire::Schema::Str16),
    ("subrole", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("frame", wire::Schema::Dyn),
    ("activationPoint", wire::Schema::Dyn),
    ("identifier", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("enabled", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("focused", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("selected", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("placeholder", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("unframed", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "children",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("role", wire::Schema::Str16),
            ("subrole", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("frame", wire::Schema::Dyn),
            ("activationPoint", wire::Schema::Dyn),
            ("identifier", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("enabled", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("focused", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("selected", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("placeholder", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("unframed", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("children", wire::Schema::List(&wire::Schema::Dyn)),
        ])),
    ),
]));

/// `findFocusedNode` over the binary wire.
#[napi(js_name = "findFocusedNodeWire")]
pub fn find_focused_node_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_FIND_FOCUSED_NODE_0)?;
        let out = &port::find_focused_node(root)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_FIND_FOCUSED_NODE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_IS_SECURE_FIELD_0: wire::Schema = wire::Schema::Dyn;

/// `isSecureField` over the binary wire.
#[napi(js_name = "isSecureFieldWire")]
pub fn is_secure_field_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: serde_json::Value = wire::read_value(r, &S_IS_SECURE_FIELD_0)?;
        let out = &port::is_secure_field(node)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PARSE_HELPER_PROBE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("ok", wire::Schema::Bool),
    ("capabilities", wire::Schema::List(&wire::Schema::Dyn)),
    ("toolchain", wire::Schema::Dyn),
    ("error", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `parseHelperProbe` over the binary wire.
#[napi(js_name = "parseHelperProbeWire")]
pub fn parse_helper_probe_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let stdout: String = r.string16()?;
        let out = &port::parse_helper_probe(stdout)?;
        wire::write_value(w, out, &S_PARSE_HELPER_PROBE_OUT)?;
        Ok(())
    })
}

/// `describeToolchain` over the binary wire.
#[napi(js_name = "describeToolchainWire")]
pub fn describe_toolchain_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let toolchain: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::describe_toolchain(toolchain)?;
        w.string16(out);
        Ok(())
    })
}

/// `describeBrokenCapabilities` over the binary wire.
#[napi(js_name = "describeBrokenCapabilitiesWire")]
pub fn describe_broken_capabilities_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let broken: Vec<serde_json::Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let toolchain: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::describe_broken_capabilities(broken, toolchain)?;
        w.string16(out);
        Ok(())
    })
}

static S_CAPABILITY_UNAVAILABLE_MESSAGE_0: wire::Schema = wire::Schema::Dyn;

/// `capabilityUnavailableMessage` over the binary wire.
#[napi(js_name = "capabilityUnavailableMessageWire")]
pub fn capability_unavailable_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let capability: serde_json::Value =
            wire::read_value(r, &S_CAPABILITY_UNAVAILABLE_MESSAGE_0)?;
        let toolchain: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::capability_unavailable_message(capability, toolchain)?;
        w.string16(out);
        Ok(())
    })
}

static S_AVAILABILITY_FROM_PROBE_0: wire::Schema = wire::Schema::Dyn;
static S_AVAILABILITY_FROM_PROBE_OUT: wire::Schema = wire::Schema::Dyn;

/// `availabilityFromProbe` over the binary wire.
#[napi(js_name = "availabilityFromProbeWire")]
pub fn availability_from_probe_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let probe: serde_json::Value = wire::read_value(r, &S_AVAILABILITY_FROM_PROBE_0)?;
        let out = &port::availability_from_probe(probe)?;
        wire::write_value(w, out, &S_AVAILABILITY_FROM_PROBE_OUT)?;
        Ok(())
    })
}

/// `xcodeAppRoot` over the binary wire.
#[napi(js_name = "xcodeAppRootWire")]
pub fn xcode_app_root_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let developer_dir: String = r.string16()?;
        let out = &port::xcode_app_root(developer_dir)?;
        w.string16(out);
        Ok(())
    })
}

static S_SANDBOX_DISABLED_0: wire::Schema = wire::Schema::Dyn;

/// `sandboxDisabled` over the binary wire.
#[napi(js_name = "sandboxDisabledWire")]
pub fn sandbox_disabled_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_SANDBOX_DISABLED_0)?;
        let out = &port::sandbox_disabled(env)?;
        w.bool(*out);
        Ok(())
    })
}

static S_DESCRIBE_SANDBOX_SUSPICION_0: wire::Schema = wire::Schema::Dyn;

/// `describeSandboxSuspicion` over the binary wire.
#[napi(js_name = "describeSandboxSuspicionWire")]
pub fn describe_sandbox_suspicion_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile_path: serde_json::Value = wire::read_value(r, &S_DESCRIBE_SANDBOX_SUSPICION_0)?;
        let out = &port::describe_sandbox_suspicion(profile_path)?;
        w.string16(out);
        Ok(())
    })
}

static S_TARGET_PHRASE_0: wire::Schema = wire::Schema::Dyn;

/// `targetPhrase` over the binary wire.
#[napi(js_name = "targetPhraseWire")]
pub fn target_phrase_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let target: serde_json::Value = wire::read_value(r, &S_TARGET_PHRASE_0)?;
        let out = &port::target_phrase(target)?;
        w.string16(out);
        Ok(())
    })
}

static S_DESCRIBE_TARGET_0: wire::Schema = wire::Schema::Dyn;

/// `describeTarget` over the binary wire.
#[napi(js_name = "describeTargetWire")]
pub fn describe_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let target: serde_json::Value = wire::read_value(r, &S_DESCRIBE_TARGET_0)?;
        let out = &port::describe_target(target)?;
        w.string16(out);
        Ok(())
    })
}

static S_IS_TARGETABLE_0: wire::Schema = wire::Schema::Dyn;

/// `isTargetable` over the binary wire.
#[napi(js_name = "isTargetableWire")]
pub fn is_targetable_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: serde_json::Value = wire::read_value(r, &S_IS_TARGETABLE_0)?;
        let out = &port::is_targetable(node)?;
        w.bool(*out);
        Ok(())
    })
}

static S_VISIBLE_LABELS_0: wire::Schema = wire::Schema::Dyn;
static S_VISIBLE_LABELS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Str16);

/// `visibleLabels` over the binary wire.
#[napi(js_name = "visibleLabelsWire")]
pub fn visible_labels_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_VISIBLE_LABELS_0)?;
        let out = &port::visible_labels(root)?;
        wire::write_value(w, out, &S_VISIBLE_LABELS_OUT)?;
        Ok(())
    })
}

static S_FIND_TARGET_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_TARGET_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_TARGET_OUT: wire::Schema = wire::Schema::Obj(&[
    ("point", wire::Schema::Dyn),
    (
        "node",
        wire::Schema::Obj(&[
            ("role", wire::Schema::Str16),
            ("subrole", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("value", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("frame", wire::Schema::Dyn),
            ("activationPoint", wire::Schema::Dyn),
            ("identifier", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("enabled", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("focused", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("selected", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("placeholder", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("unframed", wire::Schema::OptJs(&wire::Schema::Bool)),
            (
                "children",
                wire::Schema::List(&wire::Schema::Obj(&[
                    ("role", wire::Schema::Dyn),
                    ("subrole", wire::Schema::Dyn),
                    ("label", wire::Schema::Dyn),
                    ("value", wire::Schema::Dyn),
                    ("frame", wire::Schema::Dyn),
                    ("activationPoint", wire::Schema::Dyn),
                    ("identifier", wire::Schema::Dyn),
                    ("enabled", wire::Schema::Dyn),
                    ("focused", wire::Schema::Dyn),
                    ("selected", wire::Schema::Dyn),
                    ("placeholder", wire::Schema::Dyn),
                    ("unframed", wire::Schema::Dyn),
                    ("children", wire::Schema::Dyn),
                ])),
            ),
        ]),
    ),
    ("onScreen", wire::Schema::Bool),
]);

/// `findTarget` over the binary wire.
#[napi(js_name = "findTargetWire")]
pub fn find_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_FIND_TARGET_0)?;
        let target: serde_json::Value = wire::read_value(r, &S_FIND_TARGET_1)?;
        let out = &port::find_target(root, target)?;
        wire::write_value(w, out, &S_FIND_TARGET_OUT)?;
        Ok(())
    })
}

static S_PLAN_SCROLL_STEP_0: wire::Schema = wire::Schema::Dyn;
static S_PLAN_SCROLL_STEP_1: wire::Schema = wire::Schema::Dyn;
static S_PLAN_SCROLL_STEP_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("fromX", wire::Schema::Num),
    ("fromY", wire::Schema::Num),
    ("toX", wire::Schema::Num),
    ("toY", wire::Schema::Num),
    ("durationMs", wire::Schema::Num),
]));

/// `planScrollStep` over the binary wire.
#[napi(js_name = "planScrollStepWire")]
pub fn plan_scroll_step_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: serde_json::Value = wire::read_value(r, &S_PLAN_SCROLL_STEP_0)?;
        let root: serde_json::Value = wire::read_value(r, &S_PLAN_SCROLL_STEP_1)?;
        let out = &port::plan_scroll_step(node, root)?;
        wire::write_value(w, out, &S_PLAN_SCROLL_STEP_OUT)?;
        Ok(())
    })
}

static S_READ_TAP_REQUEST_0: wire::Schema = wire::Schema::Dyn;
static S_READ_TAP_REQUEST_OUT: wire::Schema = wire::Schema::Dyn;

/// `readTapRequest` over the binary wire.
#[napi(js_name = "readTapRequestWire")]
pub fn read_tap_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_READ_TAP_REQUEST_0)?;
        let out = &port::read_tap_request(input)?;
        wire::write_value(w, out, &S_READ_TAP_REQUEST_OUT)?;
        Ok(())
    })
}
