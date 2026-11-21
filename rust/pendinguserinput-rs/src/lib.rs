//! JS bindings for `apps/web/src/pendingUserInput.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

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

/// `resolvePendingUserInputAnswer`.
#[napi(js_name = "resolvePendingUserInputAnswer")]
pub fn resolve_pending_user_input_answer(question: String, draft: String) -> napi::Result<String> {
    run(move || {
        let question = crate::js::from_json(&question)?;
        let draft = crate::js::from_json(&draft)?;
        let out = port::resolve_pending_user_input_answer(question, draft)?;
        crate::js::to_json(&out)
    })
}

/// `setPendingUserInputCustomAnswer`.
#[napi(js_name = "setPendingUserInputCustomAnswer")]
pub fn set_pending_user_input_custom_answer(
    draft: String,
    custom_answer: String,
) -> napi::Result<String> {
    run(move || {
        let draft = crate::js::from_json(&draft)?;
        let out = port::set_pending_user_input_custom_answer(draft, custom_answer)?;
        crate::js::to_json(&out)
    })
}

/// `togglePendingUserInputOptionSelection`.
#[napi(js_name = "togglePendingUserInputOptionSelection")]
pub fn toggle_pending_user_input_option_selection(
    question: String,
    draft: String,
    option_label: String,
) -> napi::Result<String> {
    run(move || {
        let question = crate::js::from_json(&question)?;
        let draft = crate::js::from_json(&draft)?;
        let out = port::toggle_pending_user_input_option_selection(question, draft, option_label)?;
        crate::js::to_json(&out)
    })
}

/// `buildPendingUserInputAnswers`.
#[napi(js_name = "buildPendingUserInputAnswers")]
pub fn build_pending_user_input_answers(
    questions: String,
    draft_answers: String,
) -> napi::Result<String> {
    run(move || {
        let questions = crate::js::from_json(&questions)?;
        let draft_answers = crate::js::from_json(&draft_answers)?;
        let out = port::build_pending_user_input_answers(questions, draft_answers)?;
        crate::js::to_json(&out)
    })
}

/// `hasCompletePendingUserInputAnswers`.
#[napi(js_name = "hasCompletePendingUserInputAnswers")]
pub fn has_complete_pending_user_input_answers(answers: String) -> napi::Result<bool> {
    run(move || {
        let answers = crate::js::from_json(&answers)?;
        let out = port::has_complete_pending_user_input_answers(answers)?;
        Ok(out)
    })
}

/// `omitNullPendingUserInputAnswers`.
#[napi(js_name = "omitNullPendingUserInputAnswers")]
pub fn omit_null_pending_user_input_answers(answers: String) -> napi::Result<String> {
    run(move || {
        let answers = crate::js::from_json(&answers)?;
        let out = port::omit_null_pending_user_input_answers(answers)?;
        crate::js::to_json(&out)
    })
}

/// `derivePendingUserInputProgress`.
#[napi(js_name = "derivePendingUserInputProgress")]
pub fn derive_pending_user_input_progress(
    questions: String,
    draft_answers: String,
    question_index: f64,
) -> napi::Result<String> {
    run(move || {
        let questions = crate::js::from_json(&questions)?;
        let draft_answers = crate::js::from_json(&draft_answers)?;
        let out =
            port::derive_pending_user_input_progress(questions, draft_answers, question_index)?;
        crate::js::to_json(&out)
    })
}

/// `splitRecommendedOptionLabel`.
#[napi(js_name = "splitRecommendedOptionLabel")]
pub fn split_recommended_option_label(label: String) -> napi::Result<String> {
    run(move || {
        let out = port::split_recommended_option_label(label)?;
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

static S_RESOLVE_PENDING_USER_INPUT_ANSWER_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PENDING_USER_INPUT_ANSWER_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PENDING_USER_INPUT_ANSWER_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolvePendingUserInputAnswer` over the binary wire.
#[napi(js_name = "resolvePendingUserInputAnswerWire")]
pub fn resolve_pending_user_input_answer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let question: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_PENDING_USER_INPUT_ANSWER_0)?;
        let draft: serde_json::Value = wire::read_value(r, &S_RESOLVE_PENDING_USER_INPUT_ANSWER_1)?;
        let out = &port::resolve_pending_user_input_answer(question, draft)?;
        wire::write_value(w, out, &S_RESOLVE_PENDING_USER_INPUT_ANSWER_OUT)?;
        Ok(())
    })
}

static S_SET_PENDING_USER_INPUT_CUSTOM_ANSWER_0: wire::Schema = wire::Schema::Dyn;
static S_SET_PENDING_USER_INPUT_CUSTOM_ANSWER_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "selectedOptionLabels",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16)),
    ),
    ("customAnswer", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `setPendingUserInputCustomAnswer` over the binary wire.
#[napi(js_name = "setPendingUserInputCustomAnswerWire")]
pub fn set_pending_user_input_custom_answer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let draft: serde_json::Value =
            wire::read_value(r, &S_SET_PENDING_USER_INPUT_CUSTOM_ANSWER_0)?;
        let custom_answer: String = r.string16()?;
        let out = &port::set_pending_user_input_custom_answer(draft, custom_answer)?;
        wire::write_value(w, out, &S_SET_PENDING_USER_INPUT_CUSTOM_ANSWER_OUT)?;
        Ok(())
    })
}

static S_TOGGLE_PENDING_USER_INPUT_OPTION_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_TOGGLE_PENDING_USER_INPUT_OPTION_SELECTION_1: wire::Schema = wire::Schema::Dyn;
static S_TOGGLE_PENDING_USER_INPUT_OPTION_SELECTION_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "selectedOptionLabels",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16)),
    ),
    ("customAnswer", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `togglePendingUserInputOptionSelection` over the binary wire.
#[napi(js_name = "togglePendingUserInputOptionSelectionWire")]
pub fn toggle_pending_user_input_option_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let question: serde_json::Value =
            wire::read_value(r, &S_TOGGLE_PENDING_USER_INPUT_OPTION_SELECTION_0)?;
        let draft: serde_json::Value =
            wire::read_value(r, &S_TOGGLE_PENDING_USER_INPUT_OPTION_SELECTION_1)?;
        let option_label: String = r.string16()?;
        let out = &port::toggle_pending_user_input_option_selection(question, draft, option_label)?;
        wire::write_value(w, out, &S_TOGGLE_PENDING_USER_INPUT_OPTION_SELECTION_OUT)?;
        Ok(())
    })
}

static S_BUILD_PENDING_USER_INPUT_ANSWERS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PENDING_USER_INPUT_ANSWERS_1: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PENDING_USER_INPUT_ANSWERS_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Dyn));

/// `buildPendingUserInputAnswers` over the binary wire.
#[napi(js_name = "buildPendingUserInputAnswersWire")]
pub fn build_pending_user_input_answers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let questions: serde_json::Value =
            wire::read_value(r, &S_BUILD_PENDING_USER_INPUT_ANSWERS_0)?;
        let draft_answers: serde_json::Value =
            wire::read_value(r, &S_BUILD_PENDING_USER_INPUT_ANSWERS_1)?;
        let out = &port::build_pending_user_input_answers(questions, draft_answers)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_BUILD_PENDING_USER_INPUT_ANSWERS_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_HAS_COMPLETE_PENDING_USER_INPUT_ANSWERS_0: wire::Schema = wire::Schema::Dyn;

/// `hasCompletePendingUserInputAnswers` over the binary wire.
#[napi(js_name = "hasCompletePendingUserInputAnswersWire")]
pub fn has_complete_pending_user_input_answers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let answers: serde_json::Value =
            wire::read_value(r, &S_HAS_COMPLETE_PENDING_USER_INPUT_ANSWERS_0)?;
        let out = &port::has_complete_pending_user_input_answers(answers)?;
        w.bool(*out);
        Ok(())
    })
}

static S_OMIT_NULL_PENDING_USER_INPUT_ANSWERS_0: wire::Schema = wire::Schema::Dyn;
static S_OMIT_NULL_PENDING_USER_INPUT_ANSWERS_OUT: wire::Schema = wire::Schema::Dyn;

/// `omitNullPendingUserInputAnswers` over the binary wire.
#[napi(js_name = "omitNullPendingUserInputAnswersWire")]
pub fn omit_null_pending_user_input_answers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let answers: serde_json::Value =
            wire::read_value(r, &S_OMIT_NULL_PENDING_USER_INPUT_ANSWERS_0)?;
        let out = &port::omit_null_pending_user_input_answers(answers)?;
        wire::write_value(w, out, &S_OMIT_NULL_PENDING_USER_INPUT_ANSWERS_OUT)?;
        Ok(())
    })
}

static S_DERIVE_PENDING_USER_INPUT_PROGRESS_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_PENDING_USER_INPUT_PROGRESS_1: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_PENDING_USER_INPUT_PROGRESS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("questionIndex", wire::Schema::Num),
    ("activeQuestion", wire::Schema::Dyn),
    (
        "activeDraft",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "selectedOptionLabels",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16)),
            ),
            ("customAnswer", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "selectedOptionLabels",
        wire::Schema::List(&wire::Schema::Str16),
    ),
    ("customAnswer", wire::Schema::Str16),
    ("resolvedAnswer", wire::Schema::Dyn),
    ("usingCustomAnswer", wire::Schema::Bool),
    ("answeredQuestionCount", wire::Schema::Num),
    ("isLastQuestion", wire::Schema::Bool),
    ("isComplete", wire::Schema::Bool),
    ("canAdvance", wire::Schema::Bool),
]);

/// `derivePendingUserInputProgress` over the binary wire.
#[napi(js_name = "derivePendingUserInputProgressWire")]
pub fn derive_pending_user_input_progress_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let questions: serde_json::Value =
            wire::read_value(r, &S_DERIVE_PENDING_USER_INPUT_PROGRESS_0)?;
        let draft_answers: serde_json::Value =
            wire::read_value(r, &S_DERIVE_PENDING_USER_INPUT_PROGRESS_1)?;
        let question_index: f64 = r.f64()?;
        let out =
            &port::derive_pending_user_input_progress(questions, draft_answers, question_index)?;
        wire::write_value(w, out, &S_DERIVE_PENDING_USER_INPUT_PROGRESS_OUT)?;
        Ok(())
    })
}

static S_SPLIT_RECOMMENDED_OPTION_LABEL_OUT: wire::Schema = wire::Schema::Obj(&[
    ("label", wire::Schema::Str16),
    ("recommended", wire::Schema::Bool),
]);

/// `splitRecommendedOptionLabel` over the binary wire.
#[napi(js_name = "splitRecommendedOptionLabelWire")]
pub fn split_recommended_option_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let label: String = r.string16()?;
        let out = &port::split_recommended_option_label(label)?;
        wire::write_value(w, out, &S_SPLIT_RECOMMENDED_OPTION_LABEL_OUT)?;
        Ok(())
    })
}
