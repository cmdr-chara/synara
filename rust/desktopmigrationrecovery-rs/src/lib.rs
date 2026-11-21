//! JS bindings for `apps/desktop/src/desktopMigrationRecovery.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveDesktopMigrationRecoveryPaths`.
#[napi(js_name = "resolveDesktopMigrationRecoveryPaths")]
pub fn resolve_desktop_migration_recovery_paths(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_desktop_migration_recovery_paths(input)?;
        crate::js::to_json(&out)
    })
}

/// `invalidMigrationStartupRecoveryChoices`.
#[napi(js_name = "invalidMigrationStartupRecoveryChoices")]
pub fn invalid_migration_startup_recovery_choices(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::invalid_migration_startup_recovery_choices(input)?;
        crate::js::to_json(&out)
    })
}

/// `hasPendingDesktopMigrationRecovery`.
#[napi(js_name = "hasPendingDesktopMigrationRecovery")]
pub fn has_pending_desktop_migration_recovery(paths: String) -> napi::Result<bool> {
    run(move || {
        let paths = crate::js::from_json(&paths)?;
        let out = port::has_pending_desktop_migration_recovery(paths)?;
        Ok(out)
    })
}

/// `hasVerifiedDesktopMigrationRestore`.
#[napi(js_name = "hasVerifiedDesktopMigrationRestore")]
pub fn has_verified_desktop_migration_restore(
    paths: String,
    expected_backup_path: String,
) -> napi::Result<bool> {
    run(move || {
        let paths = crate::js::from_json(&paths)?;
        let out = port::has_verified_desktop_migration_restore(paths, expected_backup_path)?;
        Ok(out)
    })
}

/// `resolveDesktopMigrationRestoreCandidate`.
#[napi(js_name = "resolveDesktopMigrationRestoreCandidate")]
pub fn resolve_desktop_migration_restore_candidate(
    paths: String,
    block: String,
) -> napi::Result<String> {
    run(move || {
        let paths = crate::js::from_json(&paths)?;
        let block = crate::js::from_json(&block)?;
        let out = port::resolve_desktop_migration_restore_candidate(paths, block)?;
        crate::js::to_json(&out)
    })
}

/// `requiresDesktopMigrationRecovery`.
#[napi(js_name = "requiresDesktopMigrationRecovery")]
pub fn requires_desktop_migration_recovery(paths: String) -> napi::Result<bool> {
    run(move || {
        let paths = crate::js::from_json(&paths)?;
        let out = port::requires_desktop_migration_recovery(paths)?;
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

static S_RESOLVE_DESKTOP_MIGRATION_RECOVERY_PATHS_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DESKTOP_MIGRATION_RECOVERY_PATHS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("dbPath", wire::Schema::Str16),
    ("markerPath", wire::Schema::Str16),
    ("provenancePath", wire::Schema::Str16),
    ("restoreEntryPath", wire::Schema::Str16),
]);

/// `resolveDesktopMigrationRecoveryPaths` over the binary wire.
#[napi(js_name = "resolveDesktopMigrationRecoveryPathsWire")]
pub fn resolve_desktop_migration_recovery_paths_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_DESKTOP_MIGRATION_RECOVERY_PATHS_0)?;
        let out = &port::resolve_desktop_migration_recovery_paths(input)?;
        wire::write_value(w, out, &S_RESOLVE_DESKTOP_MIGRATION_RECOVERY_PATHS_OUT)?;
        Ok(())
    })
}

static S_INVALID_MIGRATION_STARTUP_RECOVERY_CHOICES_0: wire::Schema = wire::Schema::Dyn;
static S_INVALID_MIGRATION_STARTUP_RECOVERY_CHOICES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("label", wire::Schema::Str16),
        ("decision", wire::Schema::Str16),
    ]));

/// `invalidMigrationStartupRecoveryChoices` over the binary wire.
#[napi(js_name = "invalidMigrationStartupRecoveryChoicesWire")]
pub fn invalid_migration_startup_recovery_choices_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_INVALID_MIGRATION_STARTUP_RECOVERY_CHOICES_0)?;
        let out = &port::invalid_migration_startup_recovery_choices(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(
                w,
                x0,
                S_INVALID_MIGRATION_STARTUP_RECOVERY_CHOICES_OUT.inner(),
            )?;
        }
        Ok(())
    })
}

static S_HAS_PENDING_DESKTOP_MIGRATION_RECOVERY_0: wire::Schema = wire::Schema::Dyn;

/// `hasPendingDesktopMigrationRecovery` over the binary wire.
#[napi(js_name = "hasPendingDesktopMigrationRecoveryWire")]
pub fn has_pending_desktop_migration_recovery_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let paths: serde_json::Value =
            wire::read_value(r, &S_HAS_PENDING_DESKTOP_MIGRATION_RECOVERY_0)?;
        let out = &port::has_pending_desktop_migration_recovery(paths)?;
        w.bool(*out);
        Ok(())
    })
}

static S_HAS_VERIFIED_DESKTOP_MIGRATION_RESTORE_0: wire::Schema = wire::Schema::Dyn;

/// `hasVerifiedDesktopMigrationRestore` over the binary wire.
#[napi(js_name = "hasVerifiedDesktopMigrationRestoreWire")]
pub fn has_verified_desktop_migration_restore_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let paths: serde_json::Value =
            wire::read_value(r, &S_HAS_VERIFIED_DESKTOP_MIGRATION_RESTORE_0)?;
        let expected_backup_path: String = r.string16()?;
        let out = &port::has_verified_desktop_migration_restore(paths, expected_backup_path)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_DESKTOP_MIGRATION_RESTORE_CANDIDATE_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DESKTOP_MIGRATION_RESTORE_CANDIDATE_1: wire::Schema = wire::Schema::Dyn;

/// `resolveDesktopMigrationRestoreCandidate` over the binary wire.
#[napi(js_name = "resolveDesktopMigrationRestoreCandidateWire")]
pub fn resolve_desktop_migration_restore_candidate_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let paths: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_DESKTOP_MIGRATION_RESTORE_CANDIDATE_0)?;
        let block: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_DESKTOP_MIGRATION_RESTORE_CANDIDATE_1)?;
        let out = &port::resolve_desktop_migration_restore_candidate(paths, block)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_REQUIRES_DESKTOP_MIGRATION_RECOVERY_0: wire::Schema = wire::Schema::Dyn;

/// `requiresDesktopMigrationRecovery` over the binary wire.
#[napi(js_name = "requiresDesktopMigrationRecoveryWire")]
pub fn requires_desktop_migration_recovery_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let paths: serde_json::Value =
            wire::read_value(r, &S_REQUIRES_DESKTOP_MIGRATION_RECOVERY_0)?;
        let out = &port::requires_desktop_migration_recovery(paths)?;
        w.bool(*out);
        Ok(())
    })
}
