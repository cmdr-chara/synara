//! JS bindings for `apps/server/src/persistence/Migrations/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveAutomationProviderIdentity`.
#[napi(js_name = "resolveAutomationProviderIdentity")]
pub fn resolve_automation_provider_identity(
    model_selection_value: String,
    provider_options_value: String,
    provider_hint_value: String,
) -> napi::Result<String> {
    run(move || {
        let model_selection_value = crate::js::from_json(&model_selection_value)?;
        let provider_options_value = crate::js::from_json(&provider_options_value)?;
        let provider_hint_value = crate::js::from_json(&provider_hint_value)?;
        let out = port::resolve_automation_provider_identity(
            model_selection_value,
            provider_options_value,
            provider_hint_value,
        )?;
        crate::js::to_json(&out)
    })
}

/// `makeUnresolvedAutomationModelSelection`.
#[napi(js_name = "makeUnresolvedAutomationModelSelection")]
pub fn make_unresolved_automation_model_selection(
    model_selection_value: String,
    provider_options_value: String,
    provider_hint_value: String,
) -> napi::Result<String> {
    run(move || {
        let model_selection_value = crate::js::from_json(&model_selection_value)?;
        let provider_options_value = crate::js::from_json(&provider_options_value)?;
        let provider_hint_value = crate::js::from_json(&provider_hint_value)?;
        let out = port::make_unresolved_automation_model_selection(
            model_selection_value,
            provider_options_value,
            provider_hint_value,
        )?;
        crate::js::to_json(&out)
    })
}

/// `makeUnresolvedAutomationPermissionSnapshot`.
#[napi(js_name = "makeUnresolvedAutomationPermissionSnapshot")]
pub fn make_unresolved_automation_permission_snapshot(
    snapshot_value: String,
    created_at_fallback: String,
) -> napi::Result<String> {
    run(move || {
        let snapshot_value = crate::js::from_json(&snapshot_value)?;
        let out = port::make_unresolved_automation_permission_snapshot(
            snapshot_value,
            created_at_fallback,
        )?;
        crate::js::to_json(&out)
    })
}

/// `parseJson`.
#[napi(js_name = "parseJson")]
pub fn parse_json(value: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_json(value)?;
        crate::js::to_json(&out)
    })
}

/// `columnExists`.
#[napi(js_name = "columnExists")]
pub fn column_exists(sql: String, table_name: String, column_name: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let out = port::column_exists(sql, table_name, column_name)?;
        crate::js::to_json(&out)
    })
}

/// `tableExists`.
#[napi(js_name = "tableExists")]
pub fn table_exists(sql: String, table_name: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let out = port::table_exists(sql, table_name)?;
        crate::js::to_json(&out)
    })
}

/// `primaryKeyColumns`.
#[napi(js_name = "primaryKeyColumns")]
pub fn primary_key_columns(sql: String, table_name: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let out = port::primary_key_columns(sql, table_name)?;
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

static S_RESOLVE_AUTOMATION_PROVIDER_IDENTITY_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_AUTOMATION_PROVIDER_IDENTITY_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_AUTOMATION_PROVIDER_IDENTITY_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveAutomationProviderIdentity` over the binary wire.
#[napi(js_name = "resolveAutomationProviderIdentityWire")]
pub fn resolve_automation_provider_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_selection_value: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_AUTOMATION_PROVIDER_IDENTITY_0)?;
        let provider_options_value: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_AUTOMATION_PROVIDER_IDENTITY_1)?;
        let provider_hint_value: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_automation_provider_identity(
            model_selection_value,
            provider_options_value,
            provider_hint_value,
        )?;
        wire::write_value(w, out, &S_RESOLVE_AUTOMATION_PROVIDER_IDENTITY_OUT)?;
        Ok(())
    })
}

static S_MAKE_UNRESOLVED_AUTOMATION_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_UNRESOLVED_AUTOMATION_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Obj(&[
    ("provider", wire::Schema::Dyn),
    ("modelSelection", wire::Schema::Map16(&wire::Schema::Dyn)),
]);

/// `makeUnresolvedAutomationModelSelection` over the binary wire.
#[napi(js_name = "makeUnresolvedAutomationModelSelectionWire")]
pub fn make_unresolved_automation_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_selection_value: serde_json::Value =
            wire::read_value(r, &S_MAKE_UNRESOLVED_AUTOMATION_MODEL_SELECTION_0)?;
        let provider_options_value: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let provider_hint_value: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::make_unresolved_automation_model_selection(
            model_selection_value,
            provider_options_value,
            provider_hint_value,
        )?;
        wire::write_value(w, out, &S_MAKE_UNRESOLVED_AUTOMATION_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_MAKE_UNRESOLVED_AUTOMATION_PERMISSION_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_UNRESOLVED_AUTOMATION_PERMISSION_SNAPSHOT_OUT: wire::Schema =
    wire::Schema::Map16(&wire::Schema::Dyn);

/// `makeUnresolvedAutomationPermissionSnapshot` over the binary wire.
#[napi(js_name = "makeUnresolvedAutomationPermissionSnapshotWire")]
pub fn make_unresolved_automation_permission_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let snapshot_value: serde_json::Value =
            wire::read_value(r, &S_MAKE_UNRESOLVED_AUTOMATION_PERMISSION_SNAPSHOT_0)?;
        let created_at_fallback: String = r.string16()?;
        let out = &port::make_unresolved_automation_permission_snapshot(
            snapshot_value,
            created_at_fallback,
        )?;
        wire::write_value(
            w,
            out,
            &S_MAKE_UNRESOLVED_AUTOMATION_PERMISSION_SNAPSHOT_OUT,
        )?;
        Ok(())
    })
}

static S_PARSE_JSON_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseJson` over the binary wire.
#[napi(js_name = "parseJsonWire")]
pub fn parse_json_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::parse_json(value)?;
        wire::write_value(w, out, &S_PARSE_JSON_OUT)?;
        Ok(())
    })
}

static S_COLUMN_EXISTS_0: wire::Schema = wire::Schema::Dyn;
static S_COLUMN_EXISTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `columnExists` over the binary wire.
#[napi(js_name = "columnExistsWire")]
pub fn column_exists_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_COLUMN_EXISTS_0)?;
        let table_name: String = r.string16()?;
        let column_name: String = r.string16()?;
        let out = &port::column_exists(sql, table_name, column_name)?;
        wire::write_value(w, out, &S_COLUMN_EXISTS_OUT)?;
        Ok(())
    })
}

static S_TABLE_EXISTS_0: wire::Schema = wire::Schema::Dyn;
static S_TABLE_EXISTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `tableExists` over the binary wire.
#[napi(js_name = "tableExistsWire")]
pub fn table_exists_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_TABLE_EXISTS_0)?;
        let table_name: String = r.string16()?;
        let out = &port::table_exists(sql, table_name)?;
        wire::write_value(w, out, &S_TABLE_EXISTS_OUT)?;
        Ok(())
    })
}

static S_PRIMARY_KEY_COLUMNS_0: wire::Schema = wire::Schema::Dyn;
static S_PRIMARY_KEY_COLUMNS_OUT: wire::Schema = wire::Schema::Dyn;

/// `primaryKeyColumns` over the binary wire.
#[napi(js_name = "primaryKeyColumnsWire")]
pub fn primary_key_columns_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_PRIMARY_KEY_COLUMNS_0)?;
        let table_name: String = r.string16()?;
        let out = &port::primary_key_columns(sql, table_name)?;
        wire::write_value(w, out, &S_PRIMARY_KEY_COLUMNS_OUT)?;
        Ok(())
    })
}
