//! JS bindings for `apps/web/src/shortcutsSheet.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `listShortcutEditorDefinitions`.
#[napi(js_name = "listShortcutEditorDefinitions")]
pub fn list_shortcut_editor_definitions() -> napi::Result<String> {
    run(move || {
        let out = port::list_shortcut_editor_definitions()?;
        crate::js::to_json(&out)
    })
}

/// `shortcutSheetCommandLabel`.
#[napi(js_name = "shortcutSheetCommandLabel")]
pub fn shortcut_sheet_command_label(
    command: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let command = crate::js::from_json(&command)?;
        let out = port::shortcut_sheet_command_label(command)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `buildShortcutSheetSections`.
#[napi(js_name = "buildShortcutSheetSections")]
pub fn build_shortcut_sheet_sections(options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::build_shortcut_sheet_sections(options)?;
        crate::js::to_json(&out)
    })
}

/// `filterShortcutSheetSections`.
#[napi(js_name = "filterShortcutSheetSections")]
pub fn filter_shortcut_sheet_sections(sections: String, query: String) -> napi::Result<String> {
    run(move || {
        let sections = crate::js::from_json(&sections)?;
        let out = port::filter_shortcut_sheet_sections(sections, query)?;
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

static S_LIST_SHORTCUT_EDITOR_DEFINITIONS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Str16),
        ("label", wire::Schema::Str16),
        ("description", wire::Schema::Str16),
        ("commands", wire::Schema::List(&wire::Schema::Dyn)),
        (
            "members",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
                ("id", wire::Schema::Str16),
                ("label", wire::Schema::Str16),
                ("description", wire::Schema::Str16),
                ("commands", wire::Schema::List(&wire::Schema::Dyn)),
                (
                    "members",
                    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
                ),
            ]))),
        ),
    ]));

/// `listShortcutEditorDefinitions` over the binary wire.
#[napi(js_name = "listShortcutEditorDefinitionsWire")]
pub fn list_shortcut_editor_definitions_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::list_shortcut_editor_definitions()?;
        wire::write_value(w, out, &S_LIST_SHORTCUT_EDITOR_DEFINITIONS_OUT)?;
        Ok(())
    })
}

static S_SHORTCUT_SHEET_COMMAND_LABEL_0: wire::Schema = wire::Schema::Dyn;

/// `shortcutSheetCommandLabel` over the binary wire.
#[napi(js_name = "shortcutSheetCommandLabelWire")]
pub fn shortcut_sheet_command_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: Value = wire::read_value(r, &S_SHORTCUT_SHEET_COMMAND_LABEL_0)?;
        let out = &port::shortcut_sheet_command_label(command)?;
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

static S_BUILD_SHORTCUT_SHEET_SECTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_SHORTCUT_SHEET_SECTIONS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Str16),
        ("title", wire::Schema::Str16),
        ("description", wire::Schema::Str16),
        ("tone", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "entries",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("id", wire::Schema::Str16),
                ("command", wire::Schema::Dyn),
                ("binding", wire::Schema::Dyn),
                ("label", wire::Schema::Str16),
                ("description", wire::Schema::Str16),
                ("shortcutLabel", wire::Schema::Str16),
            ])),
        ),
    ]));

/// `buildShortcutSheetSections` over the binary wire.
#[napi(js_name = "buildShortcutSheetSectionsWire")]
pub fn build_shortcut_sheet_sections_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let options: Value = wire::read_value(r, &S_BUILD_SHORTCUT_SHEET_SECTIONS_0)?;
        let out = &port::build_shortcut_sheet_sections(options)?;
        wire::write_value(w, out, &S_BUILD_SHORTCUT_SHEET_SECTIONS_OUT)?;
        Ok(())
    })
}

static S_FILTER_SHORTCUT_SHEET_SECTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_FILTER_SHORTCUT_SHEET_SECTIONS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Str16),
        ("title", wire::Schema::Str16),
        ("description", wire::Schema::Str16),
        ("tone", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "entries",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("id", wire::Schema::Str16),
                ("command", wire::Schema::Dyn),
                ("binding", wire::Schema::Dyn),
                ("label", wire::Schema::Str16),
                ("description", wire::Schema::Str16),
                ("shortcutLabel", wire::Schema::Str16),
            ])),
        ),
    ]));

/// `filterShortcutSheetSections` over the binary wire.
#[napi(js_name = "filterShortcutSheetSectionsWire")]
pub fn filter_shortcut_sheet_sections_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sections: Value = wire::read_value(r, &S_FILTER_SHORTCUT_SHEET_SECTIONS_0)?;
        let query: String = r.string16()?;
        let out = &port::filter_shortcut_sheet_sections(sections, query)?;
        wire::write_value(w, out, &S_FILTER_SHORTCUT_SHEET_SECTIONS_OUT)?;
        Ok(())
    })
}
