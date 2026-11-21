//! JS bindings for `apps/web/src/keybindings.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `shortcutKeyFromEventCode`.
#[napi(js_name = "shortcutKeyFromEventCode")]
pub fn shortcut_key_from_event_code(
    code: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::shortcut_key_from_event_code(code)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isUnassignedKeybinding`.
#[napi(js_name = "isUnassignedKeybinding")]
pub fn is_unassigned_keybinding(binding: String) -> napi::Result<bool> {
    run(move || {
        let binding = crate::js::from_json(&binding)?;
        let out = port::is_unassigned_keybinding(binding)?;
        Ok(out)
    })
}

/// `isShortcutDispatchSuspended`.
#[napi(js_name = "isShortcutDispatchSuspended")]
pub fn is_shortcut_dispatch_suspended() -> napi::Result<bool> {
    run(move || {
        let out = port::is_shortcut_dispatch_suspended()?;
        Ok(out)
    })
}

/// `matchesShortcut`.
#[napi(js_name = "matchesShortcut")]
pub fn matches_shortcut(event: String, shortcut: String, platform: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let shortcut = crate::js::from_json(&shortcut)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::matches_shortcut(event, shortcut, platform)?;
        Ok(out)
    })
}

/// `evaluateWhenNode`.
#[napi(js_name = "evaluateWhenNode")]
pub fn evaluate_when_node(node: String, context: String) -> napi::Result<String> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let context = crate::js::from_json(&context)?;
        let out = port::evaluate_when_node(node, context)?;
        crate::js::to_json(&out)
    })
}

/// `shortcutConflictKey`.
#[napi(js_name = "shortcutConflictKey")]
pub fn shortcut_conflict_key(shortcut: String, platform: String) -> napi::Result<String> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::shortcut_conflict_key(shortcut, platform)?;
        Ok(out)
    })
}

/// `findEffectiveKeybindingForCommand`.
#[napi(js_name = "findEffectiveKeybindingForCommand")]
pub fn find_effective_keybinding_for_command(
    keybindings: String,
    command: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let keybindings = crate::js::from_json(&keybindings)?;
        let command = crate::js::from_json(&command)?;
        let options = crate::js::from_json(&options)?;
        let out = port::find_effective_keybinding_for_command(keybindings, command, options)?;
        crate::js::to_json(&out)
    })
}

/// `resolveKeybindingForCommand`.
#[napi(js_name = "resolveKeybindingForCommand")]
pub fn resolve_keybinding_for_command(
    keybindings: String,
    command: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let keybindings = crate::js::from_json(&keybindings)?;
        let command = crate::js::from_json(&command)?;
        let options = crate::js::from_json(&options)?;
        let out = port::resolve_keybinding_for_command(keybindings, command, options)?;
        crate::js::to_json(&out)
    })
}

/// `formatKeybindingWhenExpression`.
#[napi(js_name = "formatKeybindingWhenExpression")]
pub fn format_keybinding_when_expression(node: String) -> napi::Result<String> {
    run(move || {
        let node = crate::js::from_json(&node)?;
        let out = port::format_keybinding_when_expression(node)?;
        crate::js::to_json(&out)
    })
}

/// `resolveShortcutCommand`.
#[napi(js_name = "resolveShortcutCommand")]
pub fn resolve_shortcut_command(
    event: String,
    keybindings: String,
    options: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let keybindings = crate::js::from_json(&keybindings)?;
        let options = crate::js::from_json(&options)?;
        let out = port::resolve_shortcut_command(event, keybindings, options)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `formatShortcutLabel`.
#[napi(js_name = "formatShortcutLabel")]
pub fn format_shortcut_label(shortcut: String, platform: String) -> napi::Result<String> {
    run(move || {
        let shortcut = crate::js::from_json(&shortcut)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::format_shortcut_label(shortcut, platform)?;
        Ok(out)
    })
}

/// `splitShortcutLabel`.
#[napi(js_name = "splitShortcutLabel")]
pub fn split_shortcut_label(shortcut_label: String) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::split_shortcut_label(shortcut_label)?;
        Ok(out)
    })
}

/// `shortcutLabelForCommand`.
#[napi(js_name = "shortcutLabelForCommand")]
pub fn shortcut_label_for_command(
    keybindings: String,
    command: String,
    options: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let keybindings = crate::js::from_json(&keybindings)?;
        let command = crate::js::from_json(&command)?;
        let options = crate::js::from_json(&options)?;
        let out = port::shortcut_label_for_command(keybindings, command, options)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `threadJumpCommandForIndex`.
#[napi(js_name = "threadJumpCommandForIndex")]
pub fn thread_jump_command_for_index(index: f64) -> napi::Result<String> {
    run(move || {
        let out = port::thread_jump_command_for_index(index)?;
        crate::js::to_json(&out)
    })
}

/// `threadJumpIndexFromCommand`.
#[napi(js_name = "threadJumpIndexFromCommand")]
pub fn thread_jump_index_from_command(
    command: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let out = port::thread_jump_index_from_command(command)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `spaceJumpCommandForIndex`.
#[napi(js_name = "spaceJumpCommandForIndex")]
pub fn space_jump_command_for_index(index: f64) -> napi::Result<String> {
    run(move || {
        let out = port::space_jump_command_for_index(index)?;
        crate::js::to_json(&out)
    })
}

/// `spaceJumpIndexFromCommand`.
#[napi(js_name = "spaceJumpIndexFromCommand")]
pub fn space_jump_index_from_command(
    command: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let out = port::space_jump_index_from_command(command)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `shouldShowThreadJumpHints`.
#[napi(js_name = "shouldShowThreadJumpHints")]
pub fn should_show_thread_jump_hints(
    event: String,
    keybindings: String,
    options: String,
) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let keybindings = crate::js::from_json(&keybindings)?;
        let options = crate::js::from_json(&options)?;
        let out = port::should_show_thread_jump_hints(event, keybindings, options)?;
        Ok(out)
    })
}

/// `isOpenFavoriteEditorShortcut`.
#[napi(js_name = "isOpenFavoriteEditorShortcut")]
pub fn is_open_favorite_editor_shortcut(
    event: String,
    keybindings: String,
    options: String,
) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let keybindings = crate::js::from_json(&keybindings)?;
        let options = crate::js::from_json(&options)?;
        let out = port::is_open_favorite_editor_shortcut(event, keybindings, options)?;
        Ok(out)
    })
}

/// `isEditorFileSaveShortcut`.
#[napi(js_name = "isEditorFileSaveShortcut")]
pub fn is_editor_file_save_shortcut(
    event: String,
    keybindings: String,
    options: String,
) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let keybindings = crate::js::from_json(&keybindings)?;
        let options = crate::js::from_json(&options)?;
        let out = port::is_editor_file_save_shortcut(event, keybindings, options)?;
        Ok(out)
    })
}

/// `resolveEditorShortcutCommand`.
#[napi(js_name = "resolveEditorShortcutCommand")]
pub fn resolve_editor_shortcut_command(
    event: String,
    keybindings: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let keybindings = crate::js::from_json(&keybindings)?;
        let options = crate::js::from_json(&options)?;
        let out = port::resolve_editor_shortcut_command(event, keybindings, options)?;
        crate::js::to_json(&out)
    })
}

/// `isTerminalClearShortcut`.
#[napi(js_name = "isTerminalClearShortcut")]
pub fn is_terminal_clear_shortcut(event: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::is_terminal_clear_shortcut(event)?;
        Ok(out)
    })
}

/// `isKeyboardShortcutsHelpShortcut`.
#[napi(js_name = "isKeyboardShortcutsHelpShortcut")]
pub fn is_keyboard_shortcuts_help_shortcut(event: String, platform: String) -> napi::Result<bool> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::is_keyboard_shortcuts_help_shortcut(event, platform)?;
        Ok(out)
    })
}

/// `terminalNavigationShortcutData`.
#[napi(js_name = "terminalNavigationShortcutData")]
pub fn terminal_navigation_shortcut_data(
    event: String,
    platform: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::terminal_navigation_shortcut_data(event, platform)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
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

/// `shortcutKeyFromEventCode` over the binary wire.
#[napi(js_name = "shortcutKeyFromEventCodeWire")]
pub fn shortcut_key_from_event_code_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let code: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::shortcut_key_from_event_code(code)?;
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

static S_IS_UNASSIGNED_KEYBINDING_0: wire::Schema = wire::Schema::Dyn;

/// `isUnassignedKeybinding` over the binary wire.
#[napi(js_name = "isUnassignedKeybindingWire")]
pub fn is_unassigned_keybinding_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let binding: serde_json::Value = wire::read_value(r, &S_IS_UNASSIGNED_KEYBINDING_0)?;
        let out = &port::is_unassigned_keybinding(binding)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isShortcutDispatchSuspended` over the binary wire.
#[napi(js_name = "isShortcutDispatchSuspendedWire")]
pub fn is_shortcut_dispatch_suspended_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::is_shortcut_dispatch_suspended()?;
        w.bool(*out);
        Ok(())
    })
}

static S_MATCHES_SHORTCUT_0: wire::Schema = wire::Schema::Dyn;
static S_MATCHES_SHORTCUT_1: wire::Schema = wire::Schema::Dyn;

/// `matchesShortcut` over the binary wire.
#[napi(js_name = "matchesShortcutWire")]
pub fn matches_shortcut_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_MATCHES_SHORTCUT_0)?;
        let shortcut: serde_json::Value = wire::read_value(r, &S_MATCHES_SHORTCUT_1)?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::matches_shortcut(event, shortcut, platform)?;
        w.bool(*out);
        Ok(())
    })
}

static S_EVALUATE_WHEN_NODE_0: wire::Schema = wire::Schema::Dyn;
static S_EVALUATE_WHEN_NODE_1: wire::Schema = wire::Schema::Dyn;
static S_EVALUATE_WHEN_NODE_OUT: wire::Schema = wire::Schema::Bool;

/// `evaluateWhenNode` over the binary wire.
#[napi(js_name = "evaluateWhenNodeWire")]
pub fn evaluate_when_node_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: serde_json::Value = wire::read_value(r, &S_EVALUATE_WHEN_NODE_0)?;
        let context: serde_json::Value = wire::read_value(r, &S_EVALUATE_WHEN_NODE_1)?;
        let out = &port::evaluate_when_node(node, context)?;
        wire::write_value(w, out, &S_EVALUATE_WHEN_NODE_OUT)?;
        Ok(())
    })
}

static S_SHORTCUT_CONFLICT_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `shortcutConflictKey` over the binary wire.
#[napi(js_name = "shortcutConflictKeyWire")]
pub fn shortcut_conflict_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: serde_json::Value = wire::read_value(r, &S_SHORTCUT_CONFLICT_KEY_0)?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::shortcut_conflict_key(shortcut, platform)?;
        w.string16(out);
        Ok(())
    })
}

static S_FIND_EFFECTIVE_KEYBINDING_FOR_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_EFFECTIVE_KEYBINDING_FOR_COMMAND_1: wire::Schema = wire::Schema::Dyn;

/// `findEffectiveKeybindingForCommand` over the binary wire.
#[napi(js_name = "findEffectiveKeybindingForCommandWire")]
pub fn find_effective_keybinding_for_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let keybindings: serde_json::Value =
            wire::read_value(r, &S_FIND_EFFECTIVE_KEYBINDING_FOR_COMMAND_0)?;
        let command: serde_json::Value =
            wire::read_value(r, &S_FIND_EFFECTIVE_KEYBINDING_FOR_COMMAND_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::find_effective_keybinding_for_command(keybindings, command, options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_KEYBINDING_FOR_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_KEYBINDING_FOR_COMMAND_1: wire::Schema = wire::Schema::Dyn;

/// `resolveKeybindingForCommand` over the binary wire.
#[napi(js_name = "resolveKeybindingForCommandWire")]
pub fn resolve_keybinding_for_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let keybindings: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_KEYBINDING_FOR_COMMAND_0)?;
        let command: serde_json::Value = wire::read_value(r, &S_RESOLVE_KEYBINDING_FOR_COMMAND_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_keybinding_for_command(keybindings, command, options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_FORMAT_KEYBINDING_WHEN_EXPRESSION_OUT: wire::Schema = wire::Schema::Str16;

/// `formatKeybindingWhenExpression` over the binary wire.
#[napi(js_name = "formatKeybindingWhenExpressionWire")]
pub fn format_keybinding_when_expression_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let node: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::format_keybinding_when_expression(node)?;
        wire::write_value(w, out, &S_FORMAT_KEYBINDING_WHEN_EXPRESSION_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_SHORTCUT_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_SHORTCUT_COMMAND_1: wire::Schema = wire::Schema::Dyn;

/// `resolveShortcutCommand` over the binary wire.
#[napi(js_name = "resolveShortcutCommandWire")]
pub fn resolve_shortcut_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_RESOLVE_SHORTCUT_COMMAND_0)?;
        let keybindings: serde_json::Value = wire::read_value(r, &S_RESOLVE_SHORTCUT_COMMAND_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_shortcut_command(event, keybindings, options)?;
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

static S_FORMAT_SHORTCUT_LABEL_0: wire::Schema = wire::Schema::Dyn;

/// `formatShortcutLabel` over the binary wire.
#[napi(js_name = "formatShortcutLabelWire")]
pub fn format_shortcut_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut: serde_json::Value = wire::read_value(r, &S_FORMAT_SHORTCUT_LABEL_0)?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::format_shortcut_label(shortcut, platform)?;
        w.string16(out);
        Ok(())
    })
}

/// `splitShortcutLabel` over the binary wire.
#[napi(js_name = "splitShortcutLabelWire")]
pub fn split_shortcut_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shortcut_label: String = r.string16()?;
        let out = &port::split_shortcut_label(shortcut_label)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_SHORTCUT_LABEL_FOR_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_SHORTCUT_LABEL_FOR_COMMAND_1: wire::Schema = wire::Schema::Dyn;

/// `shortcutLabelForCommand` over the binary wire.
#[napi(js_name = "shortcutLabelForCommandWire")]
pub fn shortcut_label_for_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let keybindings: serde_json::Value = wire::read_value(r, &S_SHORTCUT_LABEL_FOR_COMMAND_0)?;
        let command: serde_json::Value = wire::read_value(r, &S_SHORTCUT_LABEL_FOR_COMMAND_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::shortcut_label_for_command(keybindings, command, options)?;
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

/// `threadJumpCommandForIndex` over the binary wire.
#[napi(js_name = "threadJumpCommandForIndexWire")]
pub fn thread_jump_command_for_index_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let index: f64 = r.f64()?;
        let out = &port::thread_jump_command_for_index(index)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `threadJumpIndexFromCommand` over the binary wire.
#[napi(js_name = "threadJumpIndexFromCommandWire")]
pub fn thread_jump_index_from_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let out = &port::thread_jump_index_from_command(command)?;
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

/// `spaceJumpCommandForIndex` over the binary wire.
#[napi(js_name = "spaceJumpCommandForIndexWire")]
pub fn space_jump_command_for_index_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let index: f64 = r.f64()?;
        let out = &port::space_jump_command_for_index(index)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `spaceJumpIndexFromCommand` over the binary wire.
#[napi(js_name = "spaceJumpIndexFromCommandWire")]
pub fn space_jump_index_from_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let out = &port::space_jump_index_from_command(command)?;
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

static S_SHOULD_SHOW_THREAD_JUMP_HINTS_0: wire::Schema = wire::Schema::Dyn;
static S_SHOULD_SHOW_THREAD_JUMP_HINTS_1: wire::Schema = wire::Schema::Dyn;

/// `shouldShowThreadJumpHints` over the binary wire.
#[napi(js_name = "shouldShowThreadJumpHintsWire")]
pub fn should_show_thread_jump_hints_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_SHOULD_SHOW_THREAD_JUMP_HINTS_0)?;
        let keybindings: serde_json::Value =
            wire::read_value(r, &S_SHOULD_SHOW_THREAD_JUMP_HINTS_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::should_show_thread_jump_hints(event, keybindings, options)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_OPEN_FAVORITE_EDITOR_SHORTCUT_0: wire::Schema = wire::Schema::Dyn;
static S_IS_OPEN_FAVORITE_EDITOR_SHORTCUT_1: wire::Schema = wire::Schema::Dyn;

/// `isOpenFavoriteEditorShortcut` over the binary wire.
#[napi(js_name = "isOpenFavoriteEditorShortcutWire")]
pub fn is_open_favorite_editor_shortcut_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_IS_OPEN_FAVORITE_EDITOR_SHORTCUT_0)?;
        let keybindings: serde_json::Value =
            wire::read_value(r, &S_IS_OPEN_FAVORITE_EDITOR_SHORTCUT_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_open_favorite_editor_shortcut(event, keybindings, options)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_EDITOR_FILE_SAVE_SHORTCUT_0: wire::Schema = wire::Schema::Dyn;
static S_IS_EDITOR_FILE_SAVE_SHORTCUT_1: wire::Schema = wire::Schema::Dyn;

/// `isEditorFileSaveShortcut` over the binary wire.
#[napi(js_name = "isEditorFileSaveShortcutWire")]
pub fn is_editor_file_save_shortcut_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_IS_EDITOR_FILE_SAVE_SHORTCUT_0)?;
        let keybindings: serde_json::Value =
            wire::read_value(r, &S_IS_EDITOR_FILE_SAVE_SHORTCUT_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_editor_file_save_shortcut(event, keybindings, options)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_EDITOR_SHORTCUT_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_EDITOR_SHORTCUT_COMMAND_1: wire::Schema = wire::Schema::Dyn;

/// `resolveEditorShortcutCommand` over the binary wire.
#[napi(js_name = "resolveEditorShortcutCommandWire")]
pub fn resolve_editor_shortcut_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_RESOLVE_EDITOR_SHORTCUT_COMMAND_0)?;
        let keybindings: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_EDITOR_SHORTCUT_COMMAND_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_editor_shortcut_command(event, keybindings, options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_TERMINAL_CLEAR_SHORTCUT_0: wire::Schema = wire::Schema::Dyn;

/// `isTerminalClearShortcut` over the binary wire.
#[napi(js_name = "isTerminalClearShortcutWire")]
pub fn is_terminal_clear_shortcut_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_IS_TERMINAL_CLEAR_SHORTCUT_0)?;
        let out = &port::is_terminal_clear_shortcut(event)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_KEYBOARD_SHORTCUTS_HELP_SHORTCUT_0: wire::Schema = wire::Schema::Dyn;

/// `isKeyboardShortcutsHelpShortcut` over the binary wire.
#[napi(js_name = "isKeyboardShortcutsHelpShortcutWire")]
pub fn is_keyboard_shortcuts_help_shortcut_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value =
            wire::read_value(r, &S_IS_KEYBOARD_SHORTCUTS_HELP_SHORTCUT_0)?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_keyboard_shortcuts_help_shortcut(event, platform)?;
        w.bool(*out);
        Ok(())
    })
}

static S_TERMINAL_NAVIGATION_SHORTCUT_DATA_0: wire::Schema = wire::Schema::Dyn;

/// `terminalNavigationShortcutData` over the binary wire.
#[napi(js_name = "terminalNavigationShortcutDataWire")]
pub fn terminal_navigation_shortcut_data_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: serde_json::Value = wire::read_value(r, &S_TERMINAL_NAVIGATION_SHORTCUT_DATA_0)?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::terminal_navigation_shortcut_data(event, platform)?;
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
