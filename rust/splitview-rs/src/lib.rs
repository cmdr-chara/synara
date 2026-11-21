//! JS bindings for `apps/web/src/splitView.logic.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `findPaneById`.
#[napi(js_name = "findPaneById")]
pub fn find_pane_by_id(root: String, pane_id: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::find_pane_by_id(root, pane_id)?;
        crate::js::to_json(&out)
    })
}

/// `findLeafPaneById`.
#[napi(js_name = "findLeafPaneById")]
pub fn find_leaf_pane_by_id(root: String, pane_id: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::find_leaf_pane_by_id(root, pane_id)?;
        crate::js::to_json(&out)
    })
}

/// `findSplitNodeById`.
#[napi(js_name = "findSplitNodeById")]
pub fn find_split_node_by_id(root: String, pane_id: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::find_split_node_by_id(root, pane_id)?;
        crate::js::to_json(&out)
    })
}

/// `findParentSplitNode`.
#[napi(js_name = "findParentSplitNode")]
pub fn find_parent_split_node(root: String, pane_id: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::find_parent_split_node(root, pane_id)?;
        crate::js::to_json(&out)
    })
}

/// `findPaneDepth`.
#[napi(js_name = "findPaneDepth")]
pub fn find_pane_depth(
    root: String,
    pane_id: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::find_pane_depth(root, pane_id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `collectLeaves`.
#[napi(js_name = "collectLeaves")]
pub fn collect_leaves(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::collect_leaves(root)?;
        crate::js::to_json(&out)
    })
}

/// `layoutSplitPanes`.
#[napi(js_name = "layoutSplitPanes")]
pub fn layout_split_panes(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::layout_split_panes(root)?;
        crate::js::to_json(&out)
    })
}

/// `replacePaneInTree`.
#[napi(js_name = "replacePaneInTree")]
pub fn replace_pane_in_tree(
    root: String,
    pane_id: String,
    replacement: String,
) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let replacement = crate::js::from_json(&replacement)?;
        let out = port::replace_pane_in_tree(root, pane_id, replacement)?;
        crate::js::to_json(&out)
    })
}

/// `removeLeafByThreadId`.
#[napi(js_name = "removeLeafByThreadId")]
pub fn remove_leaf_by_thread_id(root: String, thread_id: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::remove_leaf_by_thread_id(root, thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `removeLeafByPaneId`.
#[napi(js_name = "removeLeafByPaneId")]
pub fn remove_leaf_by_pane_id(root: String, pane_id: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::remove_leaf_by_pane_id(root, pane_id)?;
        crate::js::to_json(&out)
    })
}

/// `canSubdivide`.
#[napi(js_name = "canSubdivide")]
pub fn can_subdivide(
    parent_direction: Option<String>,
    requested_direction: String,
) -> napi::Result<bool> {
    run(move || {
        let out = port::can_subdivide(parent_direction, requested_direction)?;
        Ok(out)
    })
}

/// `canSubdividePane`.
#[napi(js_name = "canSubdividePane")]
pub fn can_subdivide_pane(
    root: String,
    target_pane_id: String,
    requested_direction: String,
) -> napi::Result<bool> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::can_subdivide_pane(root, target_pane_id, requested_direction)?;
        Ok(out)
    })
}

/// `resolveDefaultFocusLeafId`.
#[napi(js_name = "resolveDefaultFocusLeafId")]
pub fn resolve_default_focus_leaf_id(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::resolve_default_focus_leaf_id(root)?;
        Ok(out)
    })
}

/// `isLegacySplitViewLike`.
#[napi(js_name = "isLegacySplitViewLike")]
pub fn is_legacy_split_view_like(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_legacy_split_view_like(value)?;
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

static S_FIND_PANE_BY_ID_0: wire::Schema = wire::Schema::Dyn;

/// `findPaneById` over the binary wire.
#[napi(js_name = "findPaneByIdWire")]
pub fn find_pane_by_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_FIND_PANE_BY_ID_0)?;
        let pane_id: String = r.string16()?;
        let out = &port::find_pane_by_id(root, pane_id)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_FIND_LEAF_PANE_BY_ID_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_LEAF_PANE_BY_ID_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("id", wire::Schema::Str16),
    ("threadId", wire::Schema::Dyn),
    (
        "panel",
        wire::Schema::Obj(&[
            ("panel", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("hasOpenedPanel", wire::Schema::Bool),
            ("lastOpenPanel", wire::Schema::Str16),
        ]),
    ),
]));

/// `findLeafPaneById` over the binary wire.
#[napi(js_name = "findLeafPaneByIdWire")]
pub fn find_leaf_pane_by_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_FIND_LEAF_PANE_BY_ID_0)?;
        let pane_id: String = r.string16()?;
        let out = &port::find_leaf_pane_by_id(root, pane_id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_FIND_LEAF_PANE_BY_ID_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_FIND_SPLIT_NODE_BY_ID_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_SPLIT_NODE_BY_ID_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("id", wire::Schema::Str16),
    ("direction", wire::Schema::Str16),
    ("first", wire::Schema::Dyn),
    ("second", wire::Schema::Dyn),
    ("ratio", wire::Schema::Num),
]));

/// `findSplitNodeById` over the binary wire.
#[napi(js_name = "findSplitNodeByIdWire")]
pub fn find_split_node_by_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_FIND_SPLIT_NODE_BY_ID_0)?;
        let pane_id: String = r.string16()?;
        let out = &port::find_split_node_by_id(root, pane_id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_FIND_SPLIT_NODE_BY_ID_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_FIND_PARENT_SPLIT_NODE_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_PARENT_SPLIT_NODE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("id", wire::Schema::Str16),
    ("direction", wire::Schema::Str16),
    ("first", wire::Schema::Dyn),
    ("second", wire::Schema::Dyn),
    ("ratio", wire::Schema::Num),
]));

/// `findParentSplitNode` over the binary wire.
#[napi(js_name = "findParentSplitNodeWire")]
pub fn find_parent_split_node_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_FIND_PARENT_SPLIT_NODE_0)?;
        let pane_id: String = r.string16()?;
        let out = &port::find_parent_split_node(root, pane_id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_FIND_PARENT_SPLIT_NODE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_FIND_PANE_DEPTH_0: wire::Schema = wire::Schema::Dyn;

/// `findPaneDepth` over the binary wire.
#[napi(js_name = "findPaneDepthWire")]
pub fn find_pane_depth_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_FIND_PANE_DEPTH_0)?;
        let pane_id: String = r.string16()?;
        let out = &port::find_pane_depth(root, pane_id)?;
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

static S_COLLECT_LEAVES_0: wire::Schema = wire::Schema::Dyn;
static S_COLLECT_LEAVES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("kind", wire::Schema::Str16),
    ("id", wire::Schema::Str16),
    ("threadId", wire::Schema::Dyn),
    (
        "panel",
        wire::Schema::Obj(&[
            ("panel", wire::Schema::Dyn),
            ("diffTurnId", wire::Schema::Dyn),
            ("diffFilePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("hasOpenedPanel", wire::Schema::Bool),
            ("lastOpenPanel", wire::Schema::Str16),
        ]),
    ),
]));

/// `collectLeaves` over the binary wire.
#[napi(js_name = "collectLeavesWire")]
pub fn collect_leaves_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_COLLECT_LEAVES_0)?;
        let out = &port::collect_leaves(root)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_COLLECT_LEAVES_OUT.inner())?;
        }
        Ok(())
    })
}

static S_LAYOUT_SPLIT_PANES_0: wire::Schema = wire::Schema::Dyn;
static S_LAYOUT_SPLIT_PANES_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "leaves",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "leaf",
                wire::Schema::Obj(&[
                    ("kind", wire::Schema::Str16),
                    ("id", wire::Schema::Dyn),
                    ("threadId", wire::Schema::Dyn),
                    ("panel", wire::Schema::Dyn),
                ]),
            ),
            (
                "rect",
                wire::Schema::Obj(&[
                    ("left", wire::Schema::Num),
                    ("top", wire::Schema::Num),
                    ("width", wire::Schema::Num),
                    ("height", wire::Schema::Num),
                ]),
            ),
        ])),
    ),
    (
        "splits",
        wire::Schema::List(&wire::Schema::Obj(&[
            (
                "node",
                wire::Schema::Obj(&[
                    ("kind", wire::Schema::Str16),
                    ("id", wire::Schema::Dyn),
                    ("direction", wire::Schema::Dyn),
                    ("first", wire::Schema::Dyn),
                    ("second", wire::Schema::Dyn),
                    ("ratio", wire::Schema::Num),
                ]),
            ),
            (
                "rect",
                wire::Schema::Obj(&[
                    ("left", wire::Schema::Num),
                    ("top", wire::Schema::Num),
                    ("width", wire::Schema::Num),
                    ("height", wire::Schema::Num),
                ]),
            ),
        ])),
    ),
]);

/// `layoutSplitPanes` over the binary wire.
#[napi(js_name = "layoutSplitPanesWire")]
pub fn layout_split_panes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_LAYOUT_SPLIT_PANES_0)?;
        let out = &port::layout_split_panes(root)?;
        wire::write_value(w, out, &S_LAYOUT_SPLIT_PANES_OUT)?;
        Ok(())
    })
}

static S_REPLACE_PANE_IN_TREE_0: wire::Schema = wire::Schema::Dyn;
static S_REPLACE_PANE_IN_TREE_2: wire::Schema = wire::Schema::Dyn;
static S_REPLACE_PANE_IN_TREE_OUT: wire::Schema = wire::Schema::Dyn;

/// `replacePaneInTree` over the binary wire.
#[napi(js_name = "replacePaneInTreeWire")]
pub fn replace_pane_in_tree_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_REPLACE_PANE_IN_TREE_0)?;
        let pane_id: String = r.string16()?;
        let replacement: serde_json::Value = wire::read_value(r, &S_REPLACE_PANE_IN_TREE_2)?;
        let out = &port::replace_pane_in_tree(root, pane_id, replacement)?;
        wire::write_value(w, out, &S_REPLACE_PANE_IN_TREE_OUT)?;
        Ok(())
    })
}

static S_REMOVE_LEAF_BY_THREAD_ID_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_LEAF_BY_THREAD_ID_1: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_LEAF_BY_THREAD_ID_OUT: wire::Schema = wire::Schema::Obj(&[
    ("nextRoot", wire::Schema::Dyn),
    ("removedLeafIds", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `removeLeafByThreadId` over the binary wire.
#[napi(js_name = "removeLeafByThreadIdWire")]
pub fn remove_leaf_by_thread_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_REMOVE_LEAF_BY_THREAD_ID_0)?;
        let thread_id: serde_json::Value = wire::read_value(r, &S_REMOVE_LEAF_BY_THREAD_ID_1)?;
        let out = &port::remove_leaf_by_thread_id(root, thread_id)?;
        wire::write_value(w, out, &S_REMOVE_LEAF_BY_THREAD_ID_OUT)?;
        Ok(())
    })
}

static S_REMOVE_LEAF_BY_PANE_ID_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_LEAF_BY_PANE_ID_OUT: wire::Schema = wire::Schema::Obj(&[
    ("nextRoot", wire::Schema::Dyn),
    ("removedLeafIds", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `removeLeafByPaneId` over the binary wire.
#[napi(js_name = "removeLeafByPaneIdWire")]
pub fn remove_leaf_by_pane_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_REMOVE_LEAF_BY_PANE_ID_0)?;
        let pane_id: String = r.string16()?;
        let out = &port::remove_leaf_by_pane_id(root, pane_id)?;
        wire::write_value(w, out, &S_REMOVE_LEAF_BY_PANE_ID_OUT)?;
        Ok(())
    })
}

/// `canSubdivide` over the binary wire.
#[napi(js_name = "canSubdivideWire")]
pub fn can_subdivide_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let parent_direction: Option<String> = r.opt_js(|r| r.string16())?;
        let requested_direction: String = r.string16()?;
        let out = &port::can_subdivide(parent_direction, requested_direction)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CAN_SUBDIVIDE_PANE_0: wire::Schema = wire::Schema::Dyn;

/// `canSubdividePane` over the binary wire.
#[napi(js_name = "canSubdividePaneWire")]
pub fn can_subdivide_pane_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_CAN_SUBDIVIDE_PANE_0)?;
        let target_pane_id: String = r.string16()?;
        let requested_direction: String = r.string16()?;
        let out = &port::can_subdivide_pane(root, target_pane_id, requested_direction)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_DEFAULT_FOCUS_LEAF_ID_0: wire::Schema = wire::Schema::Dyn;

/// `resolveDefaultFocusLeafId` over the binary wire.
#[napi(js_name = "resolveDefaultFocusLeafIdWire")]
pub fn resolve_default_focus_leaf_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_RESOLVE_DEFAULT_FOCUS_LEAF_ID_0)?;
        let out = &port::resolve_default_focus_leaf_id(root)?;
        w.string16(out);
        Ok(())
    })
}

static S_IS_LEGACY_SPLIT_VIEW_LIKE_0: wire::Schema = wire::Schema::Dyn;

/// `isLegacySplitViewLike` over the binary wire.
#[napi(js_name = "isLegacySplitViewLikeWire")]
pub fn is_legacy_split_view_like_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_IS_LEGACY_SPLIT_VIEW_LIKE_0)?;
        let out = &port::is_legacy_split_view_like(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
