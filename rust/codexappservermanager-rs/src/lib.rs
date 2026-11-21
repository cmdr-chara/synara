//! JS bindings for `apps/server/src/codexAppServerManager.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `readCodexAccountSnapshot`.
#[napi(js_name = "readCodexAccountSnapshot")]
pub fn read_codex_account_snapshot(response: String) -> napi::Result<String> {
    run(move || {
        let response = crate::js::from_json(&response)?;
        let out = port::read_codex_account_snapshot(response)?;
        crate::js::to_json(&out)
    })
}

/// `buildCodexThreadOpenRequest`.
#[napi(js_name = "buildCodexThreadOpenRequest")]
pub fn build_codex_thread_open_request(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_codex_thread_open_request(input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveCodexThreadOpenMinimumVersion`.
#[napi(js_name = "resolveCodexThreadOpenMinimumVersion")]
pub fn resolve_codex_thread_open_minimum_version(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_codex_thread_open_minimum_version(input)?;
        crate::js::to_json(&out)
    })
}

/// `shouldWarnCodexFreshStartWithoutResume`.
#[napi(js_name = "shouldWarnCodexFreshStartWithoutResume")]
pub fn should_warn_codex_fresh_start_without_resume(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::should_warn_codex_fresh_start_without_resume(input)?;
        Ok(out)
    })
}

/// `resolveCodexModelForAccount`.
#[napi(js_name = "resolveCodexModelForAccount")]
pub fn resolve_codex_model_for_account(
    model: Option<String>,
    account: String,
) -> napi::Result<String> {
    run(move || {
        let account = crate::js::from_json(&account)?;
        let out = port::resolve_codex_model_for_account(model, account)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeCodexModelSlug`.
#[napi(js_name = "normalizeCodexModelSlug")]
pub fn normalize_codex_model_slug(
    model: Option<String>,
    preferred_id: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_codex_model_slug(model, preferred_id)?;
        crate::js::to_json(&out)
    })
}

/// `buildCodexCollaborationMode`.
#[napi(js_name = "buildCodexCollaborationMode")]
pub fn build_codex_collaboration_mode(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_codex_collaboration_mode(input)?;
        crate::js::to_json(&out)
    })
}

/// `parseCodexUserInputQuestions`.
#[napi(js_name = "parseCodexUserInputQuestions")]
pub fn parse_codex_user_input_questions(payload: String) -> napi::Result<String> {
    run(move || {
        let payload = crate::js::from_json(&payload)?;
        let out = port::parse_codex_user_input_questions(payload)?;
        crate::js::to_json(&out)
    })
}

/// `classifyCodexStderrLine`.
#[napi(js_name = "classifyCodexStderrLine")]
pub fn classify_codex_stderr_line(
    raw_line: String,
    mirrored_user_config_path: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::classify_codex_stderr_line(raw_line, mirrored_user_config_path)?;
        crate::js::to_json(&out)
    })
}

/// `readResumeCursorThreadId`.
#[napi(js_name = "readResumeCursorThreadId")]
pub fn read_resume_cursor_thread_id(resume_cursor: String) -> napi::Result<String> {
    run(move || {
        let resume_cursor = crate::js::from_json(&resume_cursor)?;
        let out = port::read_resume_cursor_thread_id(resume_cursor)?;
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

static S_READ_CODEX_ACCOUNT_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_READ_CODEX_ACCOUNT_SNAPSHOT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("type", wire::Schema::Str16),
    ("planType", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("sparkEnabled", wire::Schema::Bool),
]);

/// `readCodexAccountSnapshot` over the binary wire.
#[napi(js_name = "readCodexAccountSnapshotWire")]
pub fn read_codex_account_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let response: Value = wire::read_value(r, &S_READ_CODEX_ACCOUNT_SNAPSHOT_0)?;
        let out = &port::read_codex_account_snapshot(response)?;
        wire::write_value(w, out, &S_READ_CODEX_ACCOUNT_SNAPSHOT_OUT)?;
        Ok(())
    })
}

static S_BUILD_CODEX_THREAD_OPEN_REQUEST_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_CODEX_THREAD_OPEN_REQUEST_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildCodexThreadOpenRequest` over the binary wire.
#[napi(js_name = "buildCodexThreadOpenRequestWire")]
pub fn build_codex_thread_open_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_BUILD_CODEX_THREAD_OPEN_REQUEST_0)?;
        let out = &port::build_codex_thread_open_request(input)?;
        wire::write_value(w, out, &S_BUILD_CODEX_THREAD_OPEN_REQUEST_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_CODEX_THREAD_OPEN_MINIMUM_VERSION_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_CODEX_THREAD_OPEN_MINIMUM_VERSION_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveCodexThreadOpenMinimumVersion` over the binary wire.
#[napi(js_name = "resolveCodexThreadOpenMinimumVersionWire")]
pub fn resolve_codex_thread_open_minimum_version_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_RESOLVE_CODEX_THREAD_OPEN_MINIMUM_VERSION_0)?;
        let out = &port::resolve_codex_thread_open_minimum_version(input)?;
        wire::write_value(w, out, &S_RESOLVE_CODEX_THREAD_OPEN_MINIMUM_VERSION_OUT)?;
        Ok(())
    })
}

static S_SHOULD_WARN_CODEX_FRESH_START_WITHOUT_RESUME_0: wire::Schema = wire::Schema::Dyn;

/// `shouldWarnCodexFreshStartWithoutResume` over the binary wire.
#[napi(js_name = "shouldWarnCodexFreshStartWithoutResumeWire")]
pub fn should_warn_codex_fresh_start_without_resume_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_SHOULD_WARN_CODEX_FRESH_START_WITHOUT_RESUME_0)?;
        let out = &port::should_warn_codex_fresh_start_without_resume(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_CODEX_MODEL_FOR_ACCOUNT_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_CODEX_MODEL_FOR_ACCOUNT_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveCodexModelForAccount` over the binary wire.
#[napi(js_name = "resolveCodexModelForAccountWire")]
pub fn resolve_codex_model_for_account_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let account: Value = wire::read_value(r, &S_RESOLVE_CODEX_MODEL_FOR_ACCOUNT_1)?;
        let out = &port::resolve_codex_model_for_account(model, account)?;
        wire::write_value(w, out, &S_RESOLVE_CODEX_MODEL_FOR_ACCOUNT_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_CODEX_MODEL_SLUG_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `normalizeCodexModelSlug` over the binary wire.
#[napi(js_name = "normalizeCodexModelSlugWire")]
pub fn normalize_codex_model_slug_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let preferred_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::normalize_codex_model_slug(model, preferred_id)?;
        wire::write_value(w, out, &S_NORMALIZE_CODEX_MODEL_SLUG_OUT)?;
        Ok(())
    })
}

static S_BUILD_CODEX_COLLABORATION_MODE_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_CODEX_COLLABORATION_MODE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("mode", wire::Schema::Str16),
        (
            "settings",
            wire::Schema::Obj(&[
                ("model", wire::Schema::Str16),
                ("reasoning_effort", wire::Schema::Str16),
                ("developer_instructions", wire::Schema::Str16),
            ]),
        ),
    ]));

/// `buildCodexCollaborationMode` over the binary wire.
#[napi(js_name = "buildCodexCollaborationModeWire")]
pub fn build_codex_collaboration_mode_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_BUILD_CODEX_COLLABORATION_MODE_0)?;
        let out = &port::build_codex_collaboration_mode(input)?;
        wire::write_value(w, out, &S_BUILD_CODEX_COLLABORATION_MODE_OUT)?;
        Ok(())
    })
}

static S_PARSE_CODEX_USER_INPUT_QUESTIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseCodexUserInputQuestions` over the binary wire.
#[napi(js_name = "parseCodexUserInputQuestionsWire")]
pub fn parse_codex_user_input_questions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::parse_codex_user_input_questions(payload)?;
        wire::write_value(w, out, &S_PARSE_CODEX_USER_INPUT_QUESTIONS_OUT)?;
        Ok(())
    })
}

static S_CLASSIFY_CODEX_STDERR_LINE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("message", wire::Schema::Str16),
    ("configWarning", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "mcpSubscriptionError",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
]));

/// `classifyCodexStderrLine` over the binary wire.
#[napi(js_name = "classifyCodexStderrLineWire")]
pub fn classify_codex_stderr_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_line: String = r.string16()?;
        let mirrored_user_config_path: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::classify_codex_stderr_line(raw_line, mirrored_user_config_path)?;
        wire::write_value(w, out, &S_CLASSIFY_CODEX_STDERR_LINE_OUT)?;
        Ok(())
    })
}

static S_READ_RESUME_CURSOR_THREAD_ID_0: wire::Schema = wire::Schema::Dyn;
static S_READ_RESUME_CURSOR_THREAD_ID_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `readResumeCursorThreadId` over the binary wire.
#[napi(js_name = "readResumeCursorThreadIdWire")]
pub fn read_resume_cursor_thread_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let resume_cursor: Value = wire::read_value(r, &S_READ_RESUME_CURSOR_THREAD_ID_0)?;
        let out = &port::read_resume_cursor_thread_id(resume_cursor)?;
        wire::write_value(w, out, &S_READ_RESUME_CURSOR_THREAD_ID_OUT)?;
        Ok(())
    })
}
