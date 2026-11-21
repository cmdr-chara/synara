//! JS bindings for `packages/shared/src/projectAgent.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `normalizeProjectDocumentPath`.
#[napi(js_name = "normalizeProjectDocumentPath")]
pub fn normalize_project_document_path(raw_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_project_document_path(raw_path)?;
        Ok(out)
    })
}

/// `isInboxDocumentPath`.
#[napi(js_name = "isInboxDocumentPath")]
pub fn is_inbox_document_path(logical_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_inbox_document_path(logical_path)?;
        Ok(out)
    })
}

/// `isUserOwnedDocumentPath`.
#[napi(js_name = "isUserOwnedDocumentPath")]
pub fn is_user_owned_document_path(logical_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_user_owned_document_path(logical_path)?;
        Ok(out)
    })
}

/// `isCoordinatorCuratedDocumentPath`.
#[napi(js_name = "isCoordinatorCuratedDocumentPath")]
pub fn is_coordinator_curated_document_path(logical_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_coordinator_curated_document_path(logical_path)?;
        Ok(out)
    })
}

/// `isGeneratedDocumentPath`.
#[napi(js_name = "isGeneratedDocumentPath")]
pub fn is_generated_document_path(logical_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_generated_document_path(logical_path)?;
        Ok(out)
    })
}

/// `isMemoryDocumentPath`.
#[napi(js_name = "isMemoryDocumentPath")]
pub fn is_memory_document_path(logical_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_memory_document_path(logical_path)?;
        Ok(out)
    })
}

/// `memoryThreadDocumentPath`.
#[napi(js_name = "memoryThreadDocumentPath")]
pub fn memory_thread_document_path(thread_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::memory_thread_document_path(thread_id)?;
        Ok(out)
    })
}

/// `isMemoryThreadDocumentPath`.
#[napi(js_name = "isMemoryThreadDocumentPath")]
pub fn is_memory_thread_document_path(logical_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_memory_thread_document_path(logical_path)?;
        Ok(out)
    })
}

/// `memoryThreadIdFromDocumentPath`.
#[napi(js_name = "memoryThreadIdFromDocumentPath")]
pub fn memory_thread_id_from_document_path(
    logical_path: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::memory_thread_id_from_document_path(logical_path)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isMemoryNoteDocumentPath`.
#[napi(js_name = "isMemoryNoteDocumentPath")]
pub fn is_memory_note_document_path(logical_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_memory_note_document_path(logical_path)?;
        Ok(out)
    })
}

/// `canWriteMemoryDocument`.
#[napi(js_name = "canWriteMemoryDocument")]
pub fn can_write_memory_document(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::can_write_memory_document(input)?;
        Ok(out)
    })
}

/// `isProjectContextPreviewPath`.
#[napi(js_name = "isProjectContextPreviewPath")]
pub fn is_project_context_preview_path(logical_path: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_project_context_preview_path(logical_path)?;
        Ok(out)
    })
}

/// `sanitizeProjectDigestSummary`.
#[napi(js_name = "sanitizeProjectDigestSummary")]
pub fn sanitize_project_digest_summary(
    summary: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::sanitize_project_digest_summary(summary)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `sanitizeProjectDigestFocusTitle`.
#[napi(js_name = "sanitizeProjectDigestFocusTitle")]
pub fn sanitize_project_digest_focus_title(title: String) -> napi::Result<String> {
    run(move || {
        let out = port::sanitize_project_digest_focus_title(title)?;
        Ok(out)
    })
}

/// `detectProjectTaskDependencyCycle`.
#[napi(js_name = "detectProjectTaskDependencyCycle")]
pub fn detect_project_task_dependency_cycle(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::detect_project_task_dependency_cycle(input)?;
        Ok(out)
    })
}

/// `truncateToContextBudget`.
#[napi(js_name = "truncateToContextBudget")]
pub fn truncate_to_context_budget(sections: String, budget: String) -> napi::Result<String> {
    run(move || {
        let sections = crate::js::from_json(&sections)?;
        let budget = crate::js::from_json(&budget)?;
        let out = port::truncate_to_context_budget(sections, budget)?;
        crate::js::to_json(&out)
    })
}

/// `encodeProjectAgentListCursor`.
#[napi(js_name = "encodeProjectAgentListCursor")]
pub fn encode_project_agent_list_cursor(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::encode_project_agent_list_cursor(input)?;
        Ok(out)
    })
}

/// `decodeProjectAgentListCursor`.
#[napi(js_name = "decodeProjectAgentListCursor")]
pub fn decode_project_agent_list_cursor(cursor: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::decode_project_agent_list_cursor(cursor)?;
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

/// `normalizeProjectDocumentPath` over the binary wire.
#[napi(js_name = "normalizeProjectDocumentPathWire")]
pub fn normalize_project_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_path: String = r.string16()?;
        let out = &port::normalize_project_document_path(raw_path)?;
        w.string16(out);
        Ok(())
    })
}

/// `isInboxDocumentPath` over the binary wire.
#[napi(js_name = "isInboxDocumentPathWire")]
pub fn is_inbox_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::is_inbox_document_path(logical_path)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isUserOwnedDocumentPath` over the binary wire.
#[napi(js_name = "isUserOwnedDocumentPathWire")]
pub fn is_user_owned_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::is_user_owned_document_path(logical_path)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isCoordinatorCuratedDocumentPath` over the binary wire.
#[napi(js_name = "isCoordinatorCuratedDocumentPathWire")]
pub fn is_coordinator_curated_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::is_coordinator_curated_document_path(logical_path)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isGeneratedDocumentPath` over the binary wire.
#[napi(js_name = "isGeneratedDocumentPathWire")]
pub fn is_generated_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::is_generated_document_path(logical_path)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isMemoryDocumentPath` over the binary wire.
#[napi(js_name = "isMemoryDocumentPathWire")]
pub fn is_memory_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::is_memory_document_path(logical_path)?;
        w.bool(*out);
        Ok(())
    })
}

/// `memoryThreadDocumentPath` over the binary wire.
#[napi(js_name = "memoryThreadDocumentPathWire")]
pub fn memory_thread_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: String = r.string16()?;
        let out = &port::memory_thread_document_path(thread_id)?;
        w.string16(out);
        Ok(())
    })
}

/// `isMemoryThreadDocumentPath` over the binary wire.
#[napi(js_name = "isMemoryThreadDocumentPathWire")]
pub fn is_memory_thread_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::is_memory_thread_document_path(logical_path)?;
        w.bool(*out);
        Ok(())
    })
}

/// `memoryThreadIdFromDocumentPath` over the binary wire.
#[napi(js_name = "memoryThreadIdFromDocumentPathWire")]
pub fn memory_thread_id_from_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::memory_thread_id_from_document_path(logical_path)?;
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

/// `isMemoryNoteDocumentPath` over the binary wire.
#[napi(js_name = "isMemoryNoteDocumentPathWire")]
pub fn is_memory_note_document_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::is_memory_note_document_path(logical_path)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CAN_WRITE_MEMORY_DOCUMENT_0: wire::Schema = wire::Schema::Dyn;

/// `canWriteMemoryDocument` over the binary wire.
#[napi(js_name = "canWriteMemoryDocumentWire")]
pub fn can_write_memory_document_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_CAN_WRITE_MEMORY_DOCUMENT_0)?;
        let out = &port::can_write_memory_document(input)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isProjectContextPreviewPath` over the binary wire.
#[napi(js_name = "isProjectContextPreviewPathWire")]
pub fn is_project_context_preview_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let logical_path: String = r.string16()?;
        let out = &port::is_project_context_preview_path(logical_path)?;
        w.bool(*out);
        Ok(())
    })
}

/// `sanitizeProjectDigestSummary` over the binary wire.
#[napi(js_name = "sanitizeProjectDigestSummaryWire")]
pub fn sanitize_project_digest_summary_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let summary: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::sanitize_project_digest_summary(summary)?;
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

/// `sanitizeProjectDigestFocusTitle` over the binary wire.
#[napi(js_name = "sanitizeProjectDigestFocusTitleWire")]
pub fn sanitize_project_digest_focus_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let title: String = r.string16()?;
        let out = &port::sanitize_project_digest_focus_title(title)?;
        w.string16(out);
        Ok(())
    })
}

static S_DETECT_PROJECT_TASK_DEPENDENCY_CYCLE_0: wire::Schema = wire::Schema::Dyn;

/// `detectProjectTaskDependencyCycle` over the binary wire.
#[napi(js_name = "detectProjectTaskDependencyCycleWire")]
pub fn detect_project_task_dependency_cycle_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_DETECT_PROJECT_TASK_DEPENDENCY_CYCLE_0)?;
        let out = &port::detect_project_task_dependency_cycle(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_TRUNCATE_TO_CONTEXT_BUDGET_0: wire::Schema = wire::Schema::Dyn;
static S_TRUNCATE_TO_CONTEXT_BUDGET_1: wire::Schema = wire::Schema::Dyn;
static S_TRUNCATE_TO_CONTEXT_BUDGET_OUT: wire::Schema = wire::Schema::Obj(&[
    ("packet", wire::Schema::Str16),
    ("characterCount", wire::Schema::Num),
    ("truncated", wire::Schema::Bool),
]);

/// `truncateToContextBudget` over the binary wire.
#[napi(js_name = "truncateToContextBudgetWire")]
pub fn truncate_to_context_budget_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sections: serde_json::Value = wire::read_value(r, &S_TRUNCATE_TO_CONTEXT_BUDGET_0)?;
        let budget: serde_json::Value = wire::read_value(r, &S_TRUNCATE_TO_CONTEXT_BUDGET_1)?;
        let out = &port::truncate_to_context_budget(sections, budget)?;
        wire::write_value(w, out, &S_TRUNCATE_TO_CONTEXT_BUDGET_OUT)?;
        Ok(())
    })
}

static S_ENCODE_PROJECT_AGENT_LIST_CURSOR_0: wire::Schema = wire::Schema::Dyn;

/// `encodeProjectAgentListCursor` over the binary wire.
#[napi(js_name = "encodeProjectAgentListCursorWire")]
pub fn encode_project_agent_list_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_ENCODE_PROJECT_AGENT_LIST_CURSOR_0)?;
        let out = &port::encode_project_agent_list_cursor(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_DECODE_PROJECT_AGENT_LIST_CURSOR_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("createdAt", wire::Schema::Str16),
        ("id", wire::Schema::Str16),
        ("sequence", wire::Schema::OptJs(&wire::Schema::Num)),
    ]));

/// `decodeProjectAgentListCursor` over the binary wire.
#[napi(js_name = "decodeProjectAgentListCursorWire")]
pub fn decode_project_agent_list_cursor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cursor: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::decode_project_agent_list_cursor(cursor)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_DECODE_PROJECT_AGENT_LIST_CURSOR_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}
