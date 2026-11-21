//! JS bindings for `packages/shared/src/migrationRecovery.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `migrationRecoveryMarkerPath`.
#[napi(js_name = "migrationRecoveryMarkerPath")]
pub fn migration_recovery_marker_path(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::migration_recovery_marker_path(db_path)?;
        Ok(out)
    })
}

/// `migrationBackupDirectory`.
#[napi(js_name = "migrationBackupDirectory")]
pub fn migration_backup_directory(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::migration_backup_directory(db_path)?;
        Ok(out)
    })
}

/// `migrationBackupProvenancePath`.
#[napi(js_name = "migrationBackupProvenancePath")]
pub fn migration_backup_provenance_path(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::migration_backup_provenance_path(db_path)?;
        Ok(out)
    })
}

/// `parseMigrationRecoveryResumeState`.
#[napi(js_name = "parseMigrationRecoveryResumeState")]
pub fn parse_migration_recovery_resume_state(marker_text: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_migration_recovery_resume_state(marker_text)?;
        crate::js::to_json(&out)
    })
}

/// `createMigrationDivergenceConsentToken`.
#[napi(js_name = "createMigrationDivergenceConsentToken")]
pub fn create_migration_divergence_consent_token(challenge: String) -> napi::Result<String> {
    run(move || {
        let challenge = crate::js::from_json(&challenge)?;
        let out = port::create_migration_divergence_consent_token(challenge)?;
        Ok(out)
    })
}

/// `migrationRuntimeSourceDigest`.
#[napi(js_name = "migrationRuntimeSourceDigest")]
pub fn migration_runtime_source_digest(source: String) -> napi::Result<String> {
    run(move || {
        let out = port::migration_runtime_source_digest(source)?;
        Ok(out)
    })
}

/// `findMigrationRuntimeIdentityMismatch`.
#[napi(js_name = "findMigrationRuntimeIdentityMismatch")]
pub fn find_migration_runtime_identity_mismatch(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::find_migration_runtime_identity_mismatch(input)?;
        crate::js::to_json(&out)
    })
}

/// `serializeMigrationDivergenceConsentChallenge`.
#[napi(js_name = "serializeMigrationDivergenceConsentChallenge")]
pub fn serialize_migration_divergence_consent_challenge(challenge: String) -> napi::Result<String> {
    run(move || {
        let challenge = crate::js::from_json(&challenge)?;
        let out = port::serialize_migration_divergence_consent_challenge(challenge)?;
        Ok(out)
    })
}

/// `parseMigrationDivergenceConsentChallenge`.
#[napi(js_name = "parseMigrationDivergenceConsentChallenge")]
pub fn parse_migration_divergence_consent_challenge(output: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_migration_divergence_consent_challenge(output)?;
        crate::js::to_json(&out)
    })
}

/// `serializeMigrationSchemaTooNewStartupBlock`.
#[napi(js_name = "serializeMigrationSchemaTooNewStartupBlock")]
pub fn serialize_migration_schema_too_new_startup_block(block: String) -> napi::Result<String> {
    run(move || {
        let block = crate::js::from_json(&block)?;
        let out = port::serialize_migration_schema_too_new_startup_block(block)?;
        Ok(out)
    })
}

/// `parseMigrationSchemaTooNewStartupBlock`.
#[napi(js_name = "parseMigrationSchemaTooNewStartupBlock")]
pub fn parse_migration_schema_too_new_startup_block(output: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_migration_schema_too_new_startup_block(output)?;
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

/// `migrationRecoveryMarkerPath` over the binary wire.
#[napi(js_name = "migrationRecoveryMarkerPathWire")]
pub fn migration_recovery_marker_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::migration_recovery_marker_path(db_path)?;
        w.string16(out);
        Ok(())
    })
}

/// `migrationBackupDirectory` over the binary wire.
#[napi(js_name = "migrationBackupDirectoryWire")]
pub fn migration_backup_directory_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::migration_backup_directory(db_path)?;
        w.string16(out);
        Ok(())
    })
}

/// `migrationBackupProvenancePath` over the binary wire.
#[napi(js_name = "migrationBackupProvenancePathWire")]
pub fn migration_backup_provenance_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::migration_backup_provenance_path(db_path)?;
        w.string16(out);
        Ok(())
    })
}

static S_PARSE_MIGRATION_RECOVERY_RESUME_STATE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("attempts", wire::Schema::Num),
        ("exhausted", wire::Schema::Bool),
    ]));

/// `parseMigrationRecoveryResumeState` over the binary wire.
#[napi(js_name = "parseMigrationRecoveryResumeStateWire")]
pub fn parse_migration_recovery_resume_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let marker_text: String = r.string16()?;
        let out = &port::parse_migration_recovery_resume_state(marker_text)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PARSE_MIGRATION_RECOVERY_RESUME_STATE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_CREATE_MIGRATION_DIVERGENCE_CONSENT_TOKEN_0: wire::Schema = wire::Schema::Dyn;

/// `createMigrationDivergenceConsentToken` over the binary wire.
#[napi(js_name = "createMigrationDivergenceConsentTokenWire")]
pub fn create_migration_divergence_consent_token_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let challenge: serde_json::Value =
            wire::read_value(r, &S_CREATE_MIGRATION_DIVERGENCE_CONSENT_TOKEN_0)?;
        let out = &port::create_migration_divergence_consent_token(challenge)?;
        w.string16(out);
        Ok(())
    })
}

/// `migrationRuntimeSourceDigest` over the binary wire.
#[napi(js_name = "migrationRuntimeSourceDigestWire")]
pub fn migration_runtime_source_digest_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: String = r.string16()?;
        let out = &port::migration_runtime_source_digest(source)?;
        w.string16(out);
        Ok(())
    })
}

static S_FIND_MIGRATION_RUNTIME_IDENTITY_MISMATCH_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_MIGRATION_RUNTIME_IDENTITY_MISMATCH_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("kind", wire::Schema::Str16),
        ("expectedDigest", wire::Schema::Str16),
        ("actualDigest", wire::Schema::Str16),
    ]));

/// `findMigrationRuntimeIdentityMismatch` over the binary wire.
#[napi(js_name = "findMigrationRuntimeIdentityMismatchWire")]
pub fn find_migration_runtime_identity_mismatch_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_FIND_MIGRATION_RUNTIME_IDENTITY_MISMATCH_0)?;
        let out = &port::find_migration_runtime_identity_mismatch(input)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(
                    w,
                    x0,
                    S_FIND_MIGRATION_RUNTIME_IDENTITY_MISMATCH_OUT.inner(),
                )?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_SERIALIZE_MIGRATION_DIVERGENCE_CONSENT_CHALLENGE_0: wire::Schema = wire::Schema::Dyn;

/// `serializeMigrationDivergenceConsentChallenge` over the binary wire.
#[napi(js_name = "serializeMigrationDivergenceConsentChallengeWire")]
pub fn serialize_migration_divergence_consent_challenge_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let challenge: serde_json::Value =
            wire::read_value(r, &S_SERIALIZE_MIGRATION_DIVERGENCE_CONSENT_CHALLENGE_0)?;
        let out = &port::serialize_migration_divergence_consent_challenge(challenge)?;
        w.string16(out);
        Ok(())
    })
}

static S_PARSE_MIGRATION_DIVERGENCE_CONSENT_CHALLENGE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("version", wire::Schema::Num),
        ("databasePath", wire::Schema::Str16),
        ("backupDirectory", wire::Schema::Str16),
        ("sourceVersion", wire::Schema::Str16),
        ("targetVersion", wire::Schema::Num),
        ("firstDivergedId", wire::Schema::Num),
        ("expectedName", wire::Schema::Str16),
        ("recordedName", wire::Schema::Str16),
        ("highWaterMark", wire::Schema::Num),
        ("lineageFingerprint", wire::Schema::Str16),
        ("consentToken", wire::Schema::Str16),
    ]));

/// `parseMigrationDivergenceConsentChallenge` over the binary wire.
#[napi(js_name = "parseMigrationDivergenceConsentChallengeWire")]
pub fn parse_migration_divergence_consent_challenge_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let output: String = r.string16()?;
        let out = &port::parse_migration_divergence_consent_challenge(output)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(
                    w,
                    x0,
                    S_PARSE_MIGRATION_DIVERGENCE_CONSENT_CHALLENGE_OUT.inner(),
                )?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_SERIALIZE_MIGRATION_SCHEMA_TOO_NEW_STARTUP_BLOCK_0: wire::Schema = wire::Schema::Dyn;

/// `serializeMigrationSchemaTooNewStartupBlock` over the binary wire.
#[napi(js_name = "serializeMigrationSchemaTooNewStartupBlockWire")]
pub fn serialize_migration_schema_too_new_startup_block_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let block: serde_json::Value =
            wire::read_value(r, &S_SERIALIZE_MIGRATION_SCHEMA_TOO_NEW_STARTUP_BLOCK_0)?;
        let out = &port::serialize_migration_schema_too_new_startup_block(block)?;
        w.string16(out);
        Ok(())
    })
}

static S_PARSE_MIGRATION_SCHEMA_TOO_NEW_STARTUP_BLOCK_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("version", wire::Schema::Num),
        ("databasePath", wire::Schema::Str16),
        ("databaseMigrationId", wire::Schema::Num),
        ("latestSupportedMigrationId", wire::Schema::Num),
        ("recovery", wire::Schema::Dyn),
    ]));

/// `parseMigrationSchemaTooNewStartupBlock` over the binary wire.
#[napi(js_name = "parseMigrationSchemaTooNewStartupBlockWire")]
pub fn parse_migration_schema_too_new_startup_block_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let output: String = r.string16()?;
        let out = &port::parse_migration_schema_too_new_startup_block(output)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(
                    w,
                    x0,
                    S_PARSE_MIGRATION_SCHEMA_TOO_NEW_STARTUP_BLOCK_OUT.inner(),
                )?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}
