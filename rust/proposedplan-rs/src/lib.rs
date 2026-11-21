//! JS bindings for `apps/web/src/proposedPlan.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `proposedPlanTitle`.
#[napi(js_name = "proposedPlanTitle")]
pub fn proposed_plan_title(
    plan_markdown: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::proposed_plan_title(plan_markdown)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `stripDisplayedPlanMarkdown`.
#[napi(js_name = "stripDisplayedPlanMarkdown")]
pub fn strip_displayed_plan_markdown(plan_markdown: String) -> napi::Result<String> {
    run(move || {
        let out = port::strip_displayed_plan_markdown(plan_markdown)?;
        Ok(out)
    })
}

/// `buildCollapsedProposedPlanPreviewMarkdown`.
#[napi(js_name = "buildCollapsedProposedPlanPreviewMarkdown")]
pub fn build_collapsed_proposed_plan_preview_markdown(
    plan_markdown: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out =
            port::build_collapsed_proposed_plan_preview_markdown(plan_markdown, Some(options))?;
        Ok(out)
    })
}

/// `buildPlanImplementationPrompt`.
#[napi(js_name = "buildPlanImplementationPrompt")]
pub fn build_plan_implementation_prompt(plan_markdown: String) -> napi::Result<String> {
    run(move || {
        let out = port::build_plan_implementation_prompt(plan_markdown)?;
        Ok(out)
    })
}

/// `resolvePlanFollowUpSubmission`.
#[napi(js_name = "resolvePlanFollowUpSubmission")]
pub fn resolve_plan_follow_up_submission(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_plan_follow_up_submission(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildPlanImplementationThreadTitle`.
#[napi(js_name = "buildPlanImplementationThreadTitle")]
pub fn build_plan_implementation_thread_title(plan_markdown: String) -> napi::Result<String> {
    run(move || {
        let out = port::build_plan_implementation_thread_title(plan_markdown)?;
        Ok(out)
    })
}

/// `buildProposedPlanMarkdownFilename`.
#[napi(js_name = "buildProposedPlanMarkdownFilename")]
pub fn build_proposed_plan_markdown_filename(plan_markdown: String) -> napi::Result<String> {
    run(move || {
        let out = port::build_proposed_plan_markdown_filename(plan_markdown)?;
        Ok(out)
    })
}

/// `normalizePlanMarkdownForExport`.
#[napi(js_name = "normalizePlanMarkdownForExport")]
pub fn normalize_plan_markdown_for_export(plan_markdown: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_plan_markdown_for_export(plan_markdown)?;
        Ok(out)
    })
}

/// `stripProposedPlanBlocksFromText`.
#[napi(js_name = "stripProposedPlanBlocksFromText")]
pub fn strip_proposed_plan_blocks_from_text(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::strip_proposed_plan_blocks_from_text(text)?;
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

/// `proposedPlanTitle` over the binary wire.
#[napi(js_name = "proposedPlanTitleWire")]
pub fn proposed_plan_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let plan_markdown: String = r.string16()?;
        let out = &port::proposed_plan_title(plan_markdown)?;
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

/// `stripDisplayedPlanMarkdown` over the binary wire.
#[napi(js_name = "stripDisplayedPlanMarkdownWire")]
pub fn strip_displayed_plan_markdown_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let plan_markdown: String = r.string16()?;
        let out = &port::strip_displayed_plan_markdown(plan_markdown)?;
        w.string16(out);
        Ok(())
    })
}

/// `buildCollapsedProposedPlanPreviewMarkdown` over the binary wire.
#[napi(js_name = "buildCollapsedProposedPlanPreviewMarkdownWire")]
pub fn build_collapsed_proposed_plan_preview_markdown_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let plan_markdown: String = r.string16()?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::build_collapsed_proposed_plan_preview_markdown(plan_markdown, options)?;
        w.string16(out);
        Ok(())
    })
}

/// `buildPlanImplementationPrompt` over the binary wire.
#[napi(js_name = "buildPlanImplementationPromptWire")]
pub fn build_plan_implementation_prompt_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let plan_markdown: String = r.string16()?;
        let out = &port::build_plan_implementation_prompt(plan_markdown)?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_PLAN_FOLLOW_UP_SUBMISSION_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PLAN_FOLLOW_UP_SUBMISSION_OUT: wire::Schema = wire::Schema::Obj(&[
    ("text", wire::Schema::Str16),
    ("interactionMode", wire::Schema::Str16),
]);

/// `resolvePlanFollowUpSubmission` over the binary wire.
#[napi(js_name = "resolvePlanFollowUpSubmissionWire")]
pub fn resolve_plan_follow_up_submission_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_PLAN_FOLLOW_UP_SUBMISSION_0)?;
        let out = &port::resolve_plan_follow_up_submission(input)?;
        wire::write_value(w, out, &S_RESOLVE_PLAN_FOLLOW_UP_SUBMISSION_OUT)?;
        Ok(())
    })
}

/// `buildPlanImplementationThreadTitle` over the binary wire.
#[napi(js_name = "buildPlanImplementationThreadTitleWire")]
pub fn build_plan_implementation_thread_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let plan_markdown: String = r.string16()?;
        let out = &port::build_plan_implementation_thread_title(plan_markdown)?;
        w.string16(out);
        Ok(())
    })
}

/// `buildProposedPlanMarkdownFilename` over the binary wire.
#[napi(js_name = "buildProposedPlanMarkdownFilenameWire")]
pub fn build_proposed_plan_markdown_filename_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let plan_markdown: String = r.string16()?;
        let out = &port::build_proposed_plan_markdown_filename(plan_markdown)?;
        w.string16(out);
        Ok(())
    })
}

/// `normalizePlanMarkdownForExport` over the binary wire.
#[napi(js_name = "normalizePlanMarkdownForExportWire")]
pub fn normalize_plan_markdown_for_export_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let plan_markdown: String = r.string16()?;
        let out = &port::normalize_plan_markdown_for_export(plan_markdown)?;
        w.string16(out);
        Ok(())
    })
}

/// `stripProposedPlanBlocksFromText` over the binary wire.
#[napi(js_name = "stripProposedPlanBlocksFromTextWire")]
pub fn strip_proposed_plan_blocks_from_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::strip_proposed_plan_blocks_from_text(text)?;
        w.string16(out);
        Ok(())
    })
}
