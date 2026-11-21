//! JS bindings for `scripts/check-migration-lineage.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parseMigrationLineage`.
#[napi(js_name = "parseMigrationLineage")]
pub fn parse_migration_lineage(source: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::parse_migration_lineage(source)?;
        crate::js::to_json(&out)
    })
}

/// `parseMigrationLineageAllowances`.
#[napi(js_name = "parseMigrationLineageAllowances")]
pub fn parse_migration_lineage_allowances(source: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::parse_migration_lineage_allowances(source)?;
        crate::js::to_json(&out)
    })
}

/// `findLineageStructureViolations`.
#[napi(js_name = "findLineageStructureViolations")]
pub fn find_lineage_structure_violations(entries: String) -> napi::Result<String> {
    run(move || {
        let entries = crate::js::from_json(&entries)?;
        let out = port::find_lineage_structure_violations(entries)?;
        crate::js::to_json(&out)
    })
}

/// `findReleasedLineageViolations`.
#[napi(js_name = "findReleasedLineageViolations")]
pub fn find_released_lineage_violations(
    released: String,
    current: String,
    allowances: String,
) -> napi::Result<String> {
    run(move || {
        let released = crate::js::from_json(&released)?;
        let current = crate::js::from_json(&current)?;
        let allowances = crate::js::from_json(&allowances)?;
        let out = port::find_released_lineage_violations(released, current, allowances)?;
        crate::js::to_json(&out)
    })
}

/// `resolveMigrationLineageFamily`.
#[napi(js_name = "resolveMigrationLineageFamily")]
pub fn resolve_migration_lineage_family(
    server_version: String,
    desktop_version: String,
) -> napi::Result<String> {
    run(move || {
        let server_version = crate::js::from_json(&server_version)?;
        let desktop_version = crate::js::from_json(&desktop_version)?;
        let out = port::resolve_migration_lineage_family(server_version, desktop_version)?;
        crate::js::to_json(&out)
    })
}

/// `groupViolationsByPair`.
#[napi(js_name = "groupViolationsByPair")]
pub fn group_violations_by_pair(per_tag: String) -> napi::Result<String> {
    run(move || {
        let per_tag = crate::js::from_json(&per_tag)?;
        let out = port::group_violations_by_pair(per_tag)?;
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

static S_PARSE_MIGRATION_LINEAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_MIGRATION_LINEAGE_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Num),
    ("name", wire::Schema::Str16),
]));

/// `parseMigrationLineage` over the binary wire.
#[napi(js_name = "parseMigrationLineageWire")]
pub fn parse_migration_lineage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: Value = wire::read_value(r, &S_PARSE_MIGRATION_LINEAGE_0)?;
        let out = &port::parse_migration_lineage(source)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_MIGRATION_LINEAGE_OUT.inner())?;
        }
        Ok(())
    })
}

static S_PARSE_MIGRATION_LINEAGE_ALLOWANCES_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_MIGRATION_LINEAGE_ALLOWANCES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Num),
        ("name", wire::Schema::Str16),
    ]));

/// `parseMigrationLineageAllowances` over the binary wire.
#[napi(js_name = "parseMigrationLineageAllowancesWire")]
pub fn parse_migration_lineage_allowances_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: Value = wire::read_value(r, &S_PARSE_MIGRATION_LINEAGE_ALLOWANCES_0)?;
        let out = &port::parse_migration_lineage_allowances(source)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_MIGRATION_LINEAGE_ALLOWANCES_OUT.inner())?;
        }
        Ok(())
    })
}

static S_FIND_LINEAGE_STRUCTURE_VIOLATIONS_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_LINEAGE_STRUCTURE_VIOLATIONS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Str16);

/// `findLineageStructureViolations` over the binary wire.
#[napi(js_name = "findLineageStructureViolationsWire")]
pub fn find_lineage_structure_violations_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let entries: Value = wire::read_value(r, &S_FIND_LINEAGE_STRUCTURE_VIOLATIONS_0)?;
        let out = &port::find_lineage_structure_violations(entries)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_FIND_LINEAGE_STRUCTURE_VIOLATIONS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_FIND_RELEASED_LINEAGE_VIOLATIONS_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_RELEASED_LINEAGE_VIOLATIONS_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_RELEASED_LINEAGE_VIOLATIONS_2: wire::Schema = wire::Schema::Dyn;
static S_FIND_RELEASED_LINEAGE_VIOLATIONS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Num),
        ("releasedName", wire::Schema::Str16),
        ("currentName", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `findReleasedLineageViolations` over the binary wire.
#[napi(js_name = "findReleasedLineageViolationsWire")]
pub fn find_released_lineage_violations_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let released: Value = wire::read_value(r, &S_FIND_RELEASED_LINEAGE_VIOLATIONS_0)?;
        let current: Value = wire::read_value(r, &S_FIND_RELEASED_LINEAGE_VIOLATIONS_1)?;
        let allowances: Value = wire::read_value(r, &S_FIND_RELEASED_LINEAGE_VIOLATIONS_2)?;
        let out = &port::find_released_lineage_violations(released, current, allowances)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_FIND_RELEASED_LINEAGE_VIOLATIONS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_RESOLVE_MIGRATION_LINEAGE_FAMILY_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_MIGRATION_LINEAGE_FAMILY_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_MIGRATION_LINEAGE_FAMILY_OUT: wire::Schema = wire::Schema::Str16;

/// `resolveMigrationLineageFamily` over the binary wire.
#[napi(js_name = "resolveMigrationLineageFamilyWire")]
pub fn resolve_migration_lineage_family_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let server_version: Value = wire::read_value(r, &S_RESOLVE_MIGRATION_LINEAGE_FAMILY_0)?;
        let desktop_version: Value = wire::read_value(r, &S_RESOLVE_MIGRATION_LINEAGE_FAMILY_1)?;
        let out = &port::resolve_migration_lineage_family(server_version, desktop_version)?;
        wire::write_value(w, out, &S_RESOLVE_MIGRATION_LINEAGE_FAMILY_OUT)?;
        Ok(())
    })
}

static S_GROUP_VIOLATIONS_BY_PAIR_0: wire::Schema = wire::Schema::Dyn;
static S_GROUP_VIOLATIONS_BY_PAIR_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    (
        "violation",
        wire::Schema::Obj(&[
            ("id", wire::Schema::Num),
            ("releasedName", wire::Schema::Str16),
            ("currentName", wire::Schema::OptJs(&wire::Schema::Str16)),
        ]),
    ),
    ("tags", wire::Schema::List(&wire::Schema::Str16)),
]));

/// `groupViolationsByPair` over the binary wire.
#[napi(js_name = "groupViolationsByPairWire")]
pub fn group_violations_by_pair_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let per_tag: Value = wire::read_value(r, &S_GROUP_VIOLATIONS_BY_PAIR_0)?;
        let out = &port::group_violations_by_pair(per_tag)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_GROUP_VIOLATIONS_BY_PAIR_OUT.inner())?;
        }
        Ok(())
    })
}
