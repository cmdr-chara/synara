//! JS bindings for `apps/web/src/rightDockStore.logic.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isSingletonPaneKind`.
#[napi(js_name = "isSingletonPaneKind")]
pub fn is_singleton_pane_kind(kind: String) -> napi::Result<bool> {
    run(move || {
        let kind = crate::js::from_json(&kind)?;
        let out = port::is_singleton_pane_kind(kind)?;
        Ok(out)
    })
}

/// `createDefaultRightDockState`.
#[napi(js_name = "createDefaultRightDockState")]
pub fn create_default_right_dock_state() -> napi::Result<String> {
    run(move || {
        let out = port::create_default_right_dock_state()?;
        crate::js::to_json(&out)
    })
}

/// `isRightDockPaneKind`.
#[napi(js_name = "isRightDockPaneKind")]
pub fn is_right_dock_pane_kind(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_right_dock_pane_kind(value)?;
        Ok(out)
    })
}

/// `sanitizeRightDockThreadState`.
#[napi(js_name = "sanitizeRightDockThreadState")]
pub fn sanitize_right_dock_thread_state(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::sanitize_right_dock_thread_state(value)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeRightDockStateByThreadId`.
#[napi(js_name = "sanitizeRightDockStateByThreadId")]
pub fn sanitize_right_dock_state_by_thread_id(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::sanitize_right_dock_state_by_thread_id(value)?;
        crate::js::to_json(&out)
    })
}

/// `openPaneInState`.
#[napi(js_name = "openPaneInState")]
pub fn open_pane_in_state(state: String, input: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let input = crate::js::from_json(&input)?;
        let out = port::open_pane_in_state(state, input)?;
        crate::js::to_json(&out)
    })
}

/// `closePaneInState`.
#[napi(js_name = "closePaneInState")]
pub fn close_pane_in_state(state: String, pane_id: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::close_pane_in_state(state, pane_id)?;
        crate::js::to_json(&out)
    })
}

/// `movePaneInState`.
#[napi(js_name = "movePaneInState")]
pub fn move_pane_in_state(
    state: String,
    pane_id: String,
    over_pane_id: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::move_pane_in_state(state, pane_id, over_pane_id)?;
        crate::js::to_json(&out)
    })
}

/// `setActivePaneInState`.
#[napi(js_name = "setActivePaneInState")]
pub fn set_active_pane_in_state(state: String, pane_id: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::set_active_pane_in_state(state, pane_id)?;
        crate::js::to_json(&out)
    })
}

/// `setDockOpenInState`.
#[napi(js_name = "setDockOpenInState")]
pub fn set_dock_open_in_state(state: String, open: bool) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::set_dock_open_in_state(state, open)?;
        crate::js::to_json(&out)
    })
}

/// `updatePaneInState`.
#[napi(js_name = "updatePaneInState")]
pub fn update_pane_in_state(state: String, pane_id: String, patch: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let patch = crate::js::from_json(&patch)?;
        let out = port::update_pane_in_state(state, pane_id, patch)?;
        crate::js::to_json(&out)
    })
}

/// `setSidechatPaneThreadInState`.
#[napi(js_name = "setSidechatPaneThreadInState")]
pub fn set_sidechat_pane_thread_in_state(state: String, input: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let input = crate::js::from_json(&input)?;
        let out = port::set_sidechat_pane_thread_in_state(state, input)?;
        crate::js::to_json(&out)
    })
}

/// `toggleSingletonPaneInState`.
#[napi(js_name = "toggleSingletonPaneInState")]
pub fn toggle_singleton_pane_in_state(state: String, input: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let input = crate::js::from_json(&input)?;
        let out = port::toggle_singleton_pane_in_state(state, input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveActivePane`.
#[napi(js_name = "resolveActivePane")]
pub fn resolve_active_pane(state: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::resolve_active_pane(state)?;
        crate::js::to_json(&out)
    })
}

/// `findMissingSidechatPaneIds`.
#[napi(js_name = "findMissingSidechatPaneIds")]
pub fn find_missing_sidechat_pane_ids(
    state: String,
    existing_thread_ids: String,
) -> napi::Result<Vec<String>> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let existing_thread_ids = crate::js::from_json(&existing_thread_ids)?;
        let out = port::find_missing_sidechat_pane_ids(state, existing_thread_ids)?;
        Ok(out)
    })
}

/// `resolveVisibleDockSidechatThreadIds`.
#[napi(js_name = "resolveVisibleDockSidechatThreadIds")]
pub fn resolve_visible_dock_sidechat_thread_ids(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_visible_dock_sidechat_thread_ids(input)?;
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

static S_IS_SINGLETON_PANE_KIND_0: wire::Schema = wire::Schema::Dyn;

/// `isSingletonPaneKind` over the binary wire.
#[napi(js_name = "isSingletonPaneKindWire")]
pub fn is_singleton_pane_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let kind: serde_json::Value = wire::read_value(r, &S_IS_SINGLETON_PANE_KIND_0)?;
        let out = &port::is_singleton_pane_kind(kind)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CREATE_DEFAULT_RIGHT_DOCK_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `createDefaultRightDockState` over the binary wire.
#[napi(js_name = "createDefaultRightDockStateWire")]
pub fn create_default_right_dock_state_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::create_default_right_dock_state()?;
        wire::write_value(w, out, &S_CREATE_DEFAULT_RIGHT_DOCK_STATE_OUT)?;
        Ok(())
    })
}

static S_IS_RIGHT_DOCK_PANE_KIND_0: wire::Schema = wire::Schema::Dyn;

/// `isRightDockPaneKind` over the binary wire.
#[napi(js_name = "isRightDockPaneKindWire")]
pub fn is_right_dock_pane_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_IS_RIGHT_DOCK_PANE_KIND_0)?;
        let out = &port::is_right_dock_pane_kind(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SANITIZE_RIGHT_DOCK_THREAD_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_RIGHT_DOCK_THREAD_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `sanitizeRightDockThreadState` over the binary wire.
#[napi(js_name = "sanitizeRightDockThreadStateWire")]
pub fn sanitize_right_dock_thread_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_SANITIZE_RIGHT_DOCK_THREAD_STATE_0)?;
        let out = &port::sanitize_right_dock_thread_state(value)?;
        wire::write_value(w, out, &S_SANITIZE_RIGHT_DOCK_THREAD_STATE_OUT)?;
        Ok(())
    })
}

static S_SANITIZE_RIGHT_DOCK_STATE_BY_THREAD_ID_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_RIGHT_DOCK_STATE_BY_THREAD_ID_OUT: wire::Schema =
    wire::Schema::Map16(&wire::Schema::Obj(&[
        ("open", wire::Schema::Bool),
        (
            "panes",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("id", wire::Schema::Dyn),
                ("kind", wire::Schema::Dyn),
                ("threadId", wire::Schema::Dyn),
                ("diffTurnId", wire::Schema::Dyn),
                ("diffFilePath", wire::Schema::Dyn),
                ("filePath", wire::Schema::Dyn),
                ("pullRequestProjectId", wire::Schema::Dyn),
                ("pullRequestRepository", wire::Schema::Dyn),
                ("pullRequestNumber", wire::Schema::Dyn),
                ("pullRequestInitialTab", wire::Schema::Dyn),
            ])),
        ),
        ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `sanitizeRightDockStateByThreadId` over the binary wire.
#[napi(js_name = "sanitizeRightDockStateByThreadIdWire")]
pub fn sanitize_right_dock_state_by_thread_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value =
            wire::read_value(r, &S_SANITIZE_RIGHT_DOCK_STATE_BY_THREAD_ID_0)?;
        let out = &port::sanitize_right_dock_state_by_thread_id(value)?;
        wire::write_value(w, out, &S_SANITIZE_RIGHT_DOCK_STATE_BY_THREAD_ID_OUT)?;
        Ok(())
    })
}

static S_OPEN_PANE_IN_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_OPEN_PANE_IN_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_OPEN_PANE_IN_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `openPaneInState` over the binary wire.
#[napi(js_name = "openPaneInStateWire")]
pub fn open_pane_in_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_OPEN_PANE_IN_STATE_0)?;
        let input: serde_json::Value = wire::read_value(r, &S_OPEN_PANE_IN_STATE_1)?;
        let out = &port::open_pane_in_state(state, input)?;
        wire::write_value(w, out, &S_OPEN_PANE_IN_STATE_OUT)?;
        Ok(())
    })
}

static S_CLOSE_PANE_IN_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_CLOSE_PANE_IN_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `closePaneInState` over the binary wire.
#[napi(js_name = "closePaneInStateWire")]
pub fn close_pane_in_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_CLOSE_PANE_IN_STATE_0)?;
        let pane_id: String = r.string16()?;
        let out = &port::close_pane_in_state(state, pane_id)?;
        wire::write_value(w, out, &S_CLOSE_PANE_IN_STATE_OUT)?;
        Ok(())
    })
}

static S_MOVE_PANE_IN_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_MOVE_PANE_IN_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `movePaneInState` over the binary wire.
#[napi(js_name = "movePaneInStateWire")]
pub fn move_pane_in_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_MOVE_PANE_IN_STATE_0)?;
        let pane_id: String = r.string16()?;
        let over_pane_id: String = r.string16()?;
        let out = &port::move_pane_in_state(state, pane_id, over_pane_id)?;
        wire::write_value(w, out, &S_MOVE_PANE_IN_STATE_OUT)?;
        Ok(())
    })
}

static S_SET_ACTIVE_PANE_IN_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_SET_ACTIVE_PANE_IN_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `setActivePaneInState` over the binary wire.
#[napi(js_name = "setActivePaneInStateWire")]
pub fn set_active_pane_in_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_SET_ACTIVE_PANE_IN_STATE_0)?;
        let pane_id: String = r.string16()?;
        let out = &port::set_active_pane_in_state(state, pane_id)?;
        wire::write_value(w, out, &S_SET_ACTIVE_PANE_IN_STATE_OUT)?;
        Ok(())
    })
}

static S_SET_DOCK_OPEN_IN_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_SET_DOCK_OPEN_IN_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `setDockOpenInState` over the binary wire.
#[napi(js_name = "setDockOpenInStateWire")]
pub fn set_dock_open_in_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_SET_DOCK_OPEN_IN_STATE_0)?;
        let open: bool = r.bool()?;
        let out = &port::set_dock_open_in_state(state, open)?;
        wire::write_value(w, out, &S_SET_DOCK_OPEN_IN_STATE_OUT)?;
        Ok(())
    })
}

static S_UPDATE_PANE_IN_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_UPDATE_PANE_IN_STATE_2: wire::Schema = wire::Schema::Dyn;
static S_UPDATE_PANE_IN_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `updatePaneInState` over the binary wire.
#[napi(js_name = "updatePaneInStateWire")]
pub fn update_pane_in_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_UPDATE_PANE_IN_STATE_0)?;
        let pane_id: String = r.string16()?;
        let patch: serde_json::Value = wire::read_value(r, &S_UPDATE_PANE_IN_STATE_2)?;
        let out = &port::update_pane_in_state(state, pane_id, patch)?;
        wire::write_value(w, out, &S_UPDATE_PANE_IN_STATE_OUT)?;
        Ok(())
    })
}

static S_SET_SIDECHAT_PANE_THREAD_IN_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_SET_SIDECHAT_PANE_THREAD_IN_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_SET_SIDECHAT_PANE_THREAD_IN_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `setSidechatPaneThreadInState` over the binary wire.
#[napi(js_name = "setSidechatPaneThreadInStateWire")]
pub fn set_sidechat_pane_thread_in_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_SET_SIDECHAT_PANE_THREAD_IN_STATE_0)?;
        let input: serde_json::Value = wire::read_value(r, &S_SET_SIDECHAT_PANE_THREAD_IN_STATE_1)?;
        let out = &port::set_sidechat_pane_thread_in_state(state, input)?;
        wire::write_value(w, out, &S_SET_SIDECHAT_PANE_THREAD_IN_STATE_OUT)?;
        Ok(())
    })
}

static S_TOGGLE_SINGLETON_PANE_IN_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_TOGGLE_SINGLETON_PANE_IN_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_TOGGLE_SINGLETON_PANE_IN_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("open", wire::Schema::Bool),
    (
        "panes",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("kind", wire::Schema::Dyn),
            ("threadId", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("pullRequestProjectId", wire::Schema::Dyn),
            (
                "pullRequestRepository",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
            ("pullRequestInitialTab", wire::Schema::Dyn),
        ])),
    ),
    ("activePaneId", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `toggleSingletonPaneInState` over the binary wire.
#[napi(js_name = "toggleSingletonPaneInStateWire")]
pub fn toggle_singleton_pane_in_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_TOGGLE_SINGLETON_PANE_IN_STATE_0)?;
        let input: serde_json::Value = wire::read_value(r, &S_TOGGLE_SINGLETON_PANE_IN_STATE_1)?;
        let out = &port::toggle_singleton_pane_in_state(state, input)?;
        wire::write_value(w, out, &S_TOGGLE_SINGLETON_PANE_IN_STATE_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_ACTIVE_PANE_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_ACTIVE_PANE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("kind", wire::Schema::Dyn),
    ("threadId", wire::Schema::Dyn),
    ("diffTurnId", wire::Schema::Dyn),
    ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("filePath", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("pullRequestProjectId", wire::Schema::Dyn),
    (
        "pullRequestRepository",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("pullRequestNumber", wire::Schema::OptJs(&wire::Schema::Num)),
    (
        "pullRequestInitialTab",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
]));

/// `resolveActivePane` over the binary wire.
#[napi(js_name = "resolveActivePaneWire")]
pub fn resolve_active_pane_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_RESOLVE_ACTIVE_PANE_0)?;
        let out = &port::resolve_active_pane(state)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_RESOLVE_ACTIVE_PANE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_FIND_MISSING_SIDECHAT_PANE_IDS_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_MISSING_SIDECHAT_PANE_IDS_1: wire::Schema = wire::Schema::Dyn;

/// `findMissingSidechatPaneIds` over the binary wire.
#[napi(js_name = "findMissingSidechatPaneIdsWire")]
pub fn find_missing_sidechat_pane_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_FIND_MISSING_SIDECHAT_PANE_IDS_0)?;
        let existing_thread_ids: serde_json::Value =
            wire::read_value(r, &S_FIND_MISSING_SIDECHAT_PANE_IDS_1)?;
        let out = &port::find_missing_sidechat_pane_ids(state, existing_thread_ids)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_RESOLVE_VISIBLE_DOCK_SIDECHAT_THREAD_IDS_0: wire::Schema = wire::Schema::Dyn;

/// `resolveVisibleDockSidechatThreadIds` over the binary wire.
#[napi(js_name = "resolveVisibleDockSidechatThreadIdsWire")]
pub fn resolve_visible_dock_sidechat_thread_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_VISIBLE_DOCK_SIDECHAT_THREAD_IDS_0)?;
        let out = &port::resolve_visible_dock_sidechat_thread_ids(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
