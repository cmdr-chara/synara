//! JS bindings for `apps/web/src/keybindingEditor.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `whenConditionsCanHoldTogether`.
#[napi(js_name = "whenConditionsCanHoldTogether")]
pub fn when_conditions_can_hold_together(conditions: String, facts: String) -> napi::Result<bool> {
    run(move || {
        let conditions = crate::js::from_json(&conditions)?;
        let facts = crate::js::from_json(&facts)?;
        let out = port::when_conditions_can_hold_together(conditions, facts)?;
        Ok(out)
    })
}

/// `buildShortcutEditorRows`.
#[napi(js_name = "buildShortcutEditorRows")]
pub fn build_shortcut_editor_rows(source: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::build_shortcut_editor_rows(source)?;
        crate::js::to_json(&out)
    })
}

/// `filterShortcutEditorRows`.
#[napi(js_name = "filterShortcutEditorRows")]
pub fn filter_shortcut_editor_rows(rows: String, query: String) -> napi::Result<String> {
    run(move || {
        let rows = crate::js::from_json(&rows)?;
        let out = port::filter_shortcut_editor_rows(rows, query)?;
        crate::js::to_json(&out)
    })
}

/// `shortcutModifierHint`.
#[napi(js_name = "shortcutModifierHint")]
pub fn shortcut_modifier_hint(platform: String) -> napi::Result<String> {
    run(move || {
        let out = port::shortcut_modifier_hint(platform)?;
        Ok(out)
    })
}

/// `evaluateRecordedShortcut`.
#[napi(js_name = "evaluateRecordedShortcut")]
pub fn evaluate_recorded_shortcut(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::evaluate_recorded_shortcut(input)?;
        crate::js::to_json(&out)
    })
}

/// `shortcutSaveEdits`.
#[napi(js_name = "shortcutSaveEdits")]
pub fn shortcut_save_edits(recording: String, replacing: String) -> napi::Result<String> {
    run(move || {
        let recording = crate::js::from_json(&recording)?;
        let replacing = crate::js::from_json(&replacing)?;
        let out = port::shortcut_save_edits(recording, replacing)?;
        crate::js::to_json(&out)
    })
}

/// `shortcutRemoveEdits`.
#[napi(js_name = "shortcutRemoveEdits")]
pub fn shortcut_remove_edits(binding: String) -> napi::Result<String> {
    run(move || {
        let binding = crate::js::from_json(&binding)?;
        let out = port::shortcut_remove_edits(binding)?;
        crate::js::to_json(&out)
    })
}

/// `shortcutResetTakeovers`.
#[napi(js_name = "shortcutResetTakeovers")]
pub fn shortcut_reset_takeovers(source: String, row: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let row = crate::js::from_json(&row)?;
        let out = port::shortcut_reset_takeovers(source, row)?;
        crate::js::to_json(&out)
    })
}

/// `shortcutResetEdits`.
#[napi(js_name = "shortcutResetEdits")]
pub fn shortcut_reset_edits(source: String, row: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let row = crate::js::from_json(&row)?;
        let out = port::shortcut_reset_edits(source, row)?;
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

static S_WHEN_CONDITIONS_CAN_HOLD_TOGETHER_0: wire::Schema = wire::Schema::Dyn;
static S_WHEN_CONDITIONS_CAN_HOLD_TOGETHER_1: wire::Schema = wire::Schema::Dyn;

/// `whenConditionsCanHoldTogether` over the binary wire.
#[napi(js_name = "whenConditionsCanHoldTogetherWire")]
pub fn when_conditions_can_hold_together_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let conditions: serde_json::Value =
            wire::read_value(r, &S_WHEN_CONDITIONS_CAN_HOLD_TOGETHER_0)?;
        let facts: serde_json::Value = wire::read_value(r, &S_WHEN_CONDITIONS_CAN_HOLD_TOGETHER_1)?;
        let out = &port::when_conditions_can_hold_together(conditions, facts)?;
        w.bool(*out);
        Ok(())
    })
}

static S_BUILD_SHORTCUT_EDITOR_ROWS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_SHORTCUT_EDITOR_ROWS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("label", wire::Schema::Str16),
    ("description", wire::Schema::Str16),
    ("commands", wire::Schema::List(&wire::Schema::Dyn)),
    ("numbered", wire::Schema::Bool),
    (
        "bindings",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("label", wire::Schema::Str16),
            ("rules", wire::Schema::List(&wire::Schema::Dyn)),
        ])),
    ),
    ("isDefault", wire::Schema::Bool),
]));

/// `buildShortcutEditorRows` over the binary wire.
#[napi(js_name = "buildShortcutEditorRowsWire")]
pub fn build_shortcut_editor_rows_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value = wire::read_value(r, &S_BUILD_SHORTCUT_EDITOR_ROWS_0)?;
        let out = &port::build_shortcut_editor_rows(source)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_BUILD_SHORTCUT_EDITOR_ROWS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_FILTER_SHORTCUT_EDITOR_ROWS_0: wire::Schema = wire::Schema::Dyn;
static S_FILTER_SHORTCUT_EDITOR_ROWS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("label", wire::Schema::Str16),
    ("description", wire::Schema::Str16),
    ("commands", wire::Schema::List(&wire::Schema::Dyn)),
    ("numbered", wire::Schema::Bool),
    (
        "bindings",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("label", wire::Schema::Str16),
            ("rules", wire::Schema::List(&wire::Schema::Dyn)),
        ])),
    ),
    ("isDefault", wire::Schema::Bool),
]));

/// `filterShortcutEditorRows` over the binary wire.
#[napi(js_name = "filterShortcutEditorRowsWire")]
pub fn filter_shortcut_editor_rows_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let rows: serde_json::Value = wire::read_value(r, &S_FILTER_SHORTCUT_EDITOR_ROWS_0)?;
        let query: String = r.string16()?;
        let out = &port::filter_shortcut_editor_rows(rows, query)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_FILTER_SHORTCUT_EDITOR_ROWS_OUT.inner())?;
        }
        Ok(())
    })
}

/// `shortcutModifierHint` over the binary wire.
#[napi(js_name = "shortcutModifierHintWire")]
pub fn shortcut_modifier_hint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: String = r.string16()?;
        let out = &port::shortcut_modifier_hint(platform)?;
        w.string16(out);
        Ok(())
    })
}

static S_EVALUATE_RECORDED_SHORTCUT_0: wire::Schema = wire::Schema::Dyn;
static S_EVALUATE_RECORDED_SHORTCUT_OUT: wire::Schema = wire::Schema::Dyn;

/// `evaluateRecordedShortcut` over the binary wire.
#[napi(js_name = "evaluateRecordedShortcutWire")]
pub fn evaluate_recorded_shortcut_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_EVALUATE_RECORDED_SHORTCUT_0)?;
        let out = &port::evaluate_recorded_shortcut(input)?;
        wire::write_value(w, out, &S_EVALUATE_RECORDED_SHORTCUT_OUT)?;
        Ok(())
    })
}

static S_SHORTCUT_SAVE_EDITS_0: wire::Schema = wire::Schema::Dyn;
static S_SHORTCUT_SAVE_EDITS_1: wire::Schema = wire::Schema::Dyn;

/// `shortcutSaveEdits` over the binary wire.
#[napi(js_name = "shortcutSaveEditsWire")]
pub fn shortcut_save_edits_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let recording: serde_json::Value = wire::read_value(r, &S_SHORTCUT_SAVE_EDITS_0)?;
        let replacing: serde_json::Value = wire::read_value(r, &S_SHORTCUT_SAVE_EDITS_1)?;
        let out = &port::shortcut_save_edits(recording, replacing)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SHORTCUT_REMOVE_EDITS_0: wire::Schema = wire::Schema::Dyn;

/// `shortcutRemoveEdits` over the binary wire.
#[napi(js_name = "shortcutRemoveEditsWire")]
pub fn shortcut_remove_edits_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let binding: serde_json::Value = wire::read_value(r, &S_SHORTCUT_REMOVE_EDITS_0)?;
        let out = &port::shortcut_remove_edits(binding)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SHORTCUT_RESET_TAKEOVERS_0: wire::Schema = wire::Schema::Dyn;
static S_SHORTCUT_RESET_TAKEOVERS_1: wire::Schema = wire::Schema::Dyn;
static S_SHORTCUT_RESET_TAKEOVERS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("rule", wire::Schema::Dyn),
    ("label", wire::Schema::Str16),
]));

/// `shortcutResetTakeovers` over the binary wire.
#[napi(js_name = "shortcutResetTakeoversWire")]
pub fn shortcut_reset_takeovers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value = wire::read_value(r, &S_SHORTCUT_RESET_TAKEOVERS_0)?;
        let row: serde_json::Value = wire::read_value(r, &S_SHORTCUT_RESET_TAKEOVERS_1)?;
        let out = &port::shortcut_reset_takeovers(source, row)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_SHORTCUT_RESET_TAKEOVERS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_SHORTCUT_RESET_EDITS_0: wire::Schema = wire::Schema::Dyn;
static S_SHORTCUT_RESET_EDITS_1: wire::Schema = wire::Schema::Dyn;

/// `shortcutResetEdits` over the binary wire.
#[napi(js_name = "shortcutResetEditsWire")]
pub fn shortcut_reset_edits_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value = wire::read_value(r, &S_SHORTCUT_RESET_EDITS_0)?;
        let row: serde_json::Value = wire::read_value(r, &S_SHORTCUT_RESET_EDITS_1)?;
        let out = &port::shortcut_reset_edits(source, row)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
