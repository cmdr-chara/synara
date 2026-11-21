//! JS bindings for `packages/shared/src/terminalThreads.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `dockTerminalScopeId`.
#[napi(js_name = "dockTerminalScopeId")]
pub fn dock_terminal_scope_id(host_thread_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::dock_terminal_scope_id(host_thread_id)?;
        Ok(out)
    })
}

/// `terminalScopeIdsForThread`.
#[napi(js_name = "terminalScopeIdsForThread")]
pub fn terminal_scope_ids_for_thread(host_thread_id: String) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::terminal_scope_ids_for_thread(host_thread_id)?;
        Ok(out)
    })
}

/// `isGenericTerminalThreadTitle`.
#[napi(js_name = "isGenericTerminalThreadTitle")]
pub fn is_generic_terminal_thread_title(title: Option<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::is_generic_terminal_thread_title(title)?;
        Ok(out)
    })
}

/// `defaultTerminalTitleForCliKind`.
#[napi(js_name = "defaultTerminalTitleForCliKind")]
pub fn default_terminal_title_for_cli_kind(cli_kind: String) -> napi::Result<String> {
    run(move || {
        let out = port::default_terminal_title_for_cli_kind(cli_kind)?;
        Ok(out)
    })
}

/// `managedTerminalCommandNameForCliKind`.
#[napi(js_name = "managedTerminalCommandNameForCliKind")]
pub fn managed_terminal_command_name_for_cli_kind(cli_kind: String) -> napi::Result<String> {
    run(move || {
        let cli_kind = crate::js::from_json(&cli_kind)?;
        let out = port::managed_terminal_command_name_for_cli_kind(cli_kind)?;
        crate::js::to_json(&out)
    })
}

/// `terminalCliKindFromValue`.
#[napi(js_name = "terminalCliKindFromValue")]
pub fn terminal_cli_kind_from_value(
    value: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::terminal_cli_kind_from_value(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `deriveTerminalProcessIdentity`.
#[napi(js_name = "deriveTerminalProcessIdentity")]
pub fn derive_terminal_process_identity(command: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::derive_terminal_process_identity(command)?;
        crate::js::to_json(&out)
    })
}

/// `deriveTerminalCommandIdentity`.
#[napi(js_name = "deriveTerminalCommandIdentity")]
pub fn derive_terminal_command_identity(command: String) -> napi::Result<String> {
    run(move || {
        let out = port::derive_terminal_command_identity(command)?;
        crate::js::to_json(&out)
    })
}

/// `deriveTerminalTitleFromCommand`.
#[napi(js_name = "deriveTerminalTitleFromCommand")]
pub fn derive_terminal_title_from_command(
    command: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::derive_terminal_title_from_command(command)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `consumeTerminalIdentityInput`.
#[napi(js_name = "consumeTerminalIdentityInput")]
pub fn consume_terminal_identity_input(buffer: String, data: String) -> napi::Result<String> {
    run(move || {
        let out = port::consume_terminal_identity_input(buffer, data)?;
        crate::js::to_json(&out)
    })
}

/// `consumeTerminalTitleInput`.
#[napi(js_name = "consumeTerminalTitleInput")]
pub fn consume_terminal_title_input(buffer: String, data: String) -> napi::Result<String> {
    run(move || {
        let out = port::consume_terminal_title_input(buffer, data)?;
        crate::js::to_json(&out)
    })
}

/// `resolveTerminalVisualIdentity`.
#[napi(js_name = "resolveTerminalVisualIdentity")]
pub fn resolve_terminal_visual_identity(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_terminal_visual_identity(input)?;
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

/// `dockTerminalScopeId` over the binary wire.
#[napi(js_name = "dockTerminalScopeIdWire")]
pub fn dock_terminal_scope_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let host_thread_id: String = r.string16()?;
        let out = &port::dock_terminal_scope_id(host_thread_id)?;
        w.string16(out);
        Ok(())
    })
}

/// `terminalScopeIdsForThread` over the binary wire.
#[napi(js_name = "terminalScopeIdsForThreadWire")]
pub fn terminal_scope_ids_for_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let host_thread_id: String = r.string16()?;
        let out = &port::terminal_scope_ids_for_thread(host_thread_id)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

/// `isGenericTerminalThreadTitle` over the binary wire.
#[napi(js_name = "isGenericTerminalThreadTitleWire")]
pub fn is_generic_terminal_thread_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let title: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_generic_terminal_thread_title(title)?;
        w.bool(*out);
        Ok(())
    })
}

/// `defaultTerminalTitleForCliKind` over the binary wire.
#[napi(js_name = "defaultTerminalTitleForCliKindWire")]
pub fn default_terminal_title_for_cli_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cli_kind: String = r.string16()?;
        let out = &port::default_terminal_title_for_cli_kind(cli_kind)?;
        w.string16(out);
        Ok(())
    })
}

static S_MANAGED_TERMINAL_COMMAND_NAME_FOR_CLI_KIND_0: wire::Schema = wire::Schema::Dyn;
static S_MANAGED_TERMINAL_COMMAND_NAME_FOR_CLI_KIND_OUT: wire::Schema = wire::Schema::Str16;

/// `managedTerminalCommandNameForCliKind` over the binary wire.
#[napi(js_name = "managedTerminalCommandNameForCliKindWire")]
pub fn managed_terminal_command_name_for_cli_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cli_kind: serde_json::Value =
            wire::read_value(r, &S_MANAGED_TERMINAL_COMMAND_NAME_FOR_CLI_KIND_0)?;
        let out = &port::managed_terminal_command_name_for_cli_kind(cli_kind)?;
        wire::write_value(w, out, &S_MANAGED_TERMINAL_COMMAND_NAME_FOR_CLI_KIND_OUT)?;
        Ok(())
    })
}

/// `terminalCliKindFromValue` over the binary wire.
#[napi(js_name = "terminalCliKindFromValueWire")]
pub fn terminal_cli_kind_from_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::terminal_cli_kind_from_value(value)?;
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

static S_DERIVE_TERMINAL_PROCESS_IDENTITY_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("cliKind", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("iconKey", wire::Schema::Str16),
        ("title", wire::Schema::Str16),
    ]));

/// `deriveTerminalProcessIdentity` over the binary wire.
#[napi(js_name = "deriveTerminalProcessIdentityWire")]
pub fn derive_terminal_process_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::derive_terminal_process_identity(command)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_DERIVE_TERMINAL_PROCESS_IDENTITY_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_DERIVE_TERMINAL_COMMAND_IDENTITY_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("cliKind", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("iconKey", wire::Schema::Str16),
        ("title", wire::Schema::Str16),
    ]));

/// `deriveTerminalCommandIdentity` over the binary wire.
#[napi(js_name = "deriveTerminalCommandIdentityWire")]
pub fn derive_terminal_command_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let out = &port::derive_terminal_command_identity(command)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_DERIVE_TERMINAL_COMMAND_IDENTITY_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `deriveTerminalTitleFromCommand` over the binary wire.
#[napi(js_name = "deriveTerminalTitleFromCommandWire")]
pub fn derive_terminal_title_from_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let out = &port::derive_terminal_title_from_command(command)?;
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

static S_CONSUME_TERMINAL_IDENTITY_INPUT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("buffer", wire::Schema::Str16),
    (
        "identity",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("cliKind", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("iconKey", wire::Schema::Str16),
            ("title", wire::Schema::Str16),
        ])),
    ),
]);

/// `consumeTerminalIdentityInput` over the binary wire.
#[napi(js_name = "consumeTerminalIdentityInputWire")]
pub fn consume_terminal_identity_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let buffer: String = r.string16()?;
        let data: String = r.string16()?;
        let out = &port::consume_terminal_identity_input(buffer, data)?;
        wire::write_value(w, out, &S_CONSUME_TERMINAL_IDENTITY_INPUT_OUT)?;
        Ok(())
    })
}

static S_CONSUME_TERMINAL_TITLE_INPUT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("buffer", wire::Schema::Str16),
    ("title", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `consumeTerminalTitleInput` over the binary wire.
#[napi(js_name = "consumeTerminalTitleInputWire")]
pub fn consume_terminal_title_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let buffer: String = r.string16()?;
        let data: String = r.string16()?;
        let out = &port::consume_terminal_title_input(buffer, data)?;
        wire::write_value(w, out, &S_CONSUME_TERMINAL_TITLE_INPUT_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_TERMINAL_VISUAL_IDENTITY_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_TERMINAL_VISUAL_IDENTITY_OUT: wire::Schema =
    wire::Schema::Obj(&[("state", wire::Schema::Dyn)]);

/// `resolveTerminalVisualIdentity` over the binary wire.
#[napi(js_name = "resolveTerminalVisualIdentityWire")]
pub fn resolve_terminal_visual_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_TERMINAL_VISUAL_IDENTITY_0)?;
        let out = &port::resolve_terminal_visual_identity(input)?;
        wire::write_value(w, out, &S_RESOLVE_TERMINAL_VISUAL_IDENTITY_OUT)?;
        Ok(())
    })
}
