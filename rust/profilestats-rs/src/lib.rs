//! JS bindings for `apps/server/src/profileStats.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `sqliteModifierFromUtcOffsetMinutes`.
#[napi(js_name = "sqliteModifierFromUtcOffsetMinutes")]
pub fn sqlite_modifier_from_utc_offset_minutes(offset_minutes: f64) -> napi::Result<String> {
    run(move || {
        let out = port::sqlite_modifier_from_utc_offset_minutes(offset_minutes)?;
        Ok(out)
    })
}

/// `num`.
#[napi(js_name = "num")]
pub fn num(value: String) -> napi::Result<f64> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::num(value)?;
        Ok(out)
    })
}

/// `nonEmptyString`.
#[napi(js_name = "nonEmptyString")]
pub fn non_empty_string(value: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::non_empty_string(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `aggregateProfileSkillUsageRows`.
#[napi(js_name = "aggregateProfileSkillUsageRows")]
pub fn aggregate_profile_skill_usage_rows(
    rows: String,
    archived_rows: String,
) -> napi::Result<String> {
    run(move || {
        let rows = crate::js::from_json(&rows)?;
        let archived_rows = crate::js::from_json(&archived_rows)?;
        let out = port::aggregate_profile_skill_usage_rows(rows, archived_rows)?;
        crate::js::to_json(&out)
    })
}

/// `heatmapIntensity`.
#[napi(js_name = "heatmapIntensity")]
pub fn heatmap_intensity(count: f64, sorted_active_counts: Vec<f64>) -> napi::Result<f64> {
    run(move || {
        let out = port::heatmap_intensity(count, sorted_active_counts)?;
        Ok(out)
    })
}

/// `normalizeProviderKind`.
#[napi(js_name = "normalizeProviderKind")]
pub fn normalize_provider_kind(value: String, model: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let model = crate::js::from_json(&model)?;
        let out = port::normalize_provider_kind(value, model)?;
        crate::js::to_json(&out)
    })
}

/// `turnModelSelectionCte`.
#[napi(js_name = "turnModelSelectionCte")]
pub fn turn_model_selection_cte(sql: String, scope: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let scope = crate::js::from_json(&scope)?;
        let out = port::turn_model_selection_cte(sql, scope)?;
        crate::js::to_json(&out)
    })
}

/// `userPromptEventsQuery`.
#[napi(js_name = "userPromptEventsQuery")]
pub fn user_prompt_events_query(sql: String, range: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let range = crate::js::from_json(&range)?;
        let out = port::user_prompt_events_query(sql, range)?;
        crate::js::to_json(&out)
    })
}

/// `tokenDeltaCtes`.
#[napi(js_name = "tokenDeltaCtes")]
pub fn token_delta_ctes(sql: String, scope: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let scope = crate::js::from_json(&scope)?;
        let out = port::token_delta_ctes(sql, scope)?;
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

/// `sqliteModifierFromUtcOffsetMinutes` over the binary wire.
#[napi(js_name = "sqliteModifierFromUtcOffsetMinutesWire")]
pub fn sqlite_modifier_from_utc_offset_minutes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let offset_minutes: f64 = r.f64()?;
        let out = &port::sqlite_modifier_from_utc_offset_minutes(offset_minutes)?;
        w.string16(out);
        Ok(())
    })
}

static S_NUM_0: wire::Schema = wire::Schema::Dyn;

/// `num` over the binary wire.
#[napi(js_name = "numWire")]
pub fn num_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_NUM_0)?;
        let out = &port::num(value)?;
        w.f64(*out);
        Ok(())
    })
}

static S_NON_EMPTY_STRING_0: wire::Schema = wire::Schema::Dyn;

/// `nonEmptyString` over the binary wire.
#[napi(js_name = "nonEmptyStringWire")]
pub fn non_empty_string_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_NON_EMPTY_STRING_0)?;
        let out = &port::non_empty_string(value)?;
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

static S_AGGREGATE_PROFILE_SKILL_USAGE_ROWS_0: wire::Schema = wire::Schema::Dyn;
static S_AGGREGATE_PROFILE_SKILL_USAGE_ROWS_1: wire::Schema = wire::Schema::Dyn;

/// `aggregateProfileSkillUsageRows` over the binary wire.
#[napi(js_name = "aggregateProfileSkillUsageRowsWire")]
pub fn aggregate_profile_skill_usage_rows_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let rows: serde_json::Value = wire::read_value(r, &S_AGGREGATE_PROFILE_SKILL_USAGE_ROWS_0)?;
        let archived_rows: serde_json::Value =
            wire::read_value(r, &S_AGGREGATE_PROFILE_SKILL_USAGE_ROWS_1)?;
        let out = &port::aggregate_profile_skill_usage_rows(rows, archived_rows)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `heatmapIntensity` over the binary wire.
#[napi(js_name = "heatmapIntensityWire")]
pub fn heatmap_intensity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let count: f64 = r.f64()?;
        let sorted_active_counts: Vec<f64> = r.list(|r| r.f64())?;
        let out = &port::heatmap_intensity(count, sorted_active_counts)?;
        w.f64(*out);
        Ok(())
    })
}

static S_NORMALIZE_PROVIDER_KIND_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PROVIDER_KIND_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeProviderKind` over the binary wire.
#[napi(js_name = "normalizeProviderKindWire")]
pub fn normalize_provider_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_NORMALIZE_PROVIDER_KIND_0)?;
        let model: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::normalize_provider_kind(value, model)?;
        wire::write_value(w, out, &S_NORMALIZE_PROVIDER_KIND_OUT)?;
        Ok(())
    })
}

static S_TURN_MODEL_SELECTION_CTE_0: wire::Schema = wire::Schema::Dyn;
static S_TURN_MODEL_SELECTION_CTE_OUT: wire::Schema = wire::Schema::Dyn;

/// `turnModelSelectionCte` over the binary wire.
#[napi(js_name = "turnModelSelectionCteWire")]
pub fn turn_model_selection_cte_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_TURN_MODEL_SELECTION_CTE_0)?;
        let scope: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::turn_model_selection_cte(sql, scope)?;
        wire::write_value(w, out, &S_TURN_MODEL_SELECTION_CTE_OUT)?;
        Ok(())
    })
}

static S_USER_PROMPT_EVENTS_QUERY_0: wire::Schema = wire::Schema::Dyn;
static S_USER_PROMPT_EVENTS_QUERY_OUT: wire::Schema = wire::Schema::Dyn;

/// `userPromptEventsQuery` over the binary wire.
#[napi(js_name = "userPromptEventsQueryWire")]
pub fn user_prompt_events_query_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_USER_PROMPT_EVENTS_QUERY_0)?;
        let range: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::user_prompt_events_query(sql, range)?;
        wire::write_value(w, out, &S_USER_PROMPT_EVENTS_QUERY_OUT)?;
        Ok(())
    })
}

static S_TOKEN_DELTA_CTES_0: wire::Schema = wire::Schema::Dyn;
static S_TOKEN_DELTA_CTES_OUT: wire::Schema = wire::Schema::Dyn;

/// `tokenDeltaCtes` over the binary wire.
#[napi(js_name = "tokenDeltaCtesWire")]
pub fn token_delta_ctes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_TOKEN_DELTA_CTES_0)?;
        let scope: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::token_delta_ctes(sql, scope)?;
        wire::write_value(w, out, &S_TOKEN_DELTA_CTES_OUT)?;
        Ok(())
    })
}
