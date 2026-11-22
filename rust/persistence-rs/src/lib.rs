//! JS bindings for `apps/server/src/persistence/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `migrationTrackerFingerprint`.
#[napi(js_name = "migrationTrackerFingerprint")]
pub fn migration_tracker_fingerprint(rows: String) -> napi::Result<String> {
    run(move || {
        let rows = crate::js::from_json(&rows)?;
        let out = port::migration_tracker_fingerprint(rows)?;
        Ok(out)
    })
}

/// `classifyAccountMigrationLineage`.
#[napi(js_name = "classifyAccountMigrationLineage")]
pub fn classify_account_migration_lineage(rows: String, canonical: String) -> napi::Result<String> {
    run(move || {
        let rows = crate::js::from_json(&rows)?;
        let canonical = crate::js::from_json(&canonical)?;
        let out = port::classify_account_migration_lineage(rows, canonical)?;
        crate::js::to_json(&out)
    })
}

/// `validateAccountMigrationSchema`.
#[napi(js_name = "validateAccountMigrationSchema")]
pub fn validate_account_migration_schema(
    columns: String,
    rows: String,
    requires_identity: String,
) -> napi::Result<()> {
    run(move || {
        let columns = crate::js::from_json(&columns)?;
        let rows = crate::js::from_json(&rows)?;
        let requires_identity = crate::js::from_json(&requires_identity)?;
        port::validate_account_migration_schema(columns, rows, requires_identity)?;
        Ok(())
    })
}

/// `acquireDatabaseLifecycleLock`.
#[napi(js_name = "acquireDatabaseLifecycleLock")]
pub fn acquire_database_lifecycle_lock(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::acquire_database_lifecycle_lock(db_path)?;
        crate::js::to_json(&out)
    })
}

/// `releaseDatabaseLifecycleLock`.
#[napi(js_name = "releaseDatabaseLifecycleLock")]
pub fn release_database_lifecycle_lock(lock: String) -> napi::Result<String> {
    run(move || {
        let lock = crate::js::from_json(&lock)?;
        let out = port::release_database_lifecycle_lock(lock)?;
        crate::js::to_json(&out)
    })
}

/// `toPersistenceDecodeError`.
#[napi(js_name = "toPersistenceDecodeError")]
pub fn to_persistence_decode_error(operation: String) -> napi::Result<String> {
    run(move || {
        let out = port::to_persistence_decode_error(operation)?;
        crate::js::to_json(&out)
    })
}

/// `toPersistenceDecodeCauseError`.
#[napi(js_name = "toPersistenceDecodeCauseError")]
pub fn to_persistence_decode_cause_error(operation: String) -> napi::Result<String> {
    run(move || {
        let out = port::to_persistence_decode_cause_error(operation)?;
        crate::js::to_json(&out)
    })
}

/// `isPersistenceError`.
#[napi(js_name = "isPersistenceError")]
pub fn is_persistence_error(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::is_persistence_error(u)?;
        crate::js::to_json(&out)
    })
}

/// `estimateMigrationBackupRequiredBytes`.
#[napi(js_name = "estimateMigrationBackupRequiredBytes")]
pub fn estimate_migration_backup_required_bytes(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::estimate_migration_backup_required_bytes(db_path)?;
        crate::js::to_json(&out)
    })
}

/// `pruneMigrationBackups`.
#[napi(js_name = "pruneMigrationBackups")]
pub fn prune_migration_backups(db_path: String, retention: String) -> napi::Result<String> {
    run(move || {
        let retention = crate::js::from_json(&retention)?;
        let out = port::prune_migration_backups(db_path, retention)?;
        crate::js::to_json(&out)
    })
}

/// `createMigrationBackup`.
#[napi(js_name = "createMigrationBackup")]
pub fn create_migration_backup(db_path: String, plan: String) -> napi::Result<String> {
    run(move || {
        let plan = crate::js::from_json(&plan)?;
        let out = port::create_migration_backup(db_path, plan)?;
        crate::js::to_json(&out)
    })
}

/// `inspectSqliteMigrationBackup`.
#[napi(js_name = "inspectSqliteMigrationBackup")]
pub fn inspect_sqlite_migration_backup(backup_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::inspect_sqlite_migration_backup(backup_path)?;
        crate::js::to_json(&out)
    })
}

/// `inspectCompletedMigrationBackupForSchemaTooNew`.
#[napi(js_name = "inspectCompletedMigrationBackupForSchemaTooNew")]
pub fn inspect_completed_migration_backup_for_schema_too_new(
    db_path: String,
    input: String,
) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::inspect_completed_migration_backup_for_schema_too_new(db_path, input)?;
        crate::js::to_json(&out)
    })
}

/// `reclaimOrphanedMigrationArtifacts`.
#[napi(js_name = "reclaimOrphanedMigrationArtifacts")]
pub fn reclaim_orphaned_migration_artifacts(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::reclaim_orphaned_migration_artifacts(db_path)?;
        crate::js::to_json(&out)
    })
}

/// `requireNoPendingMigrationRecovery`.
#[napi(js_name = "requireNoPendingMigrationRecovery")]
pub fn require_no_pending_migration_recovery(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::require_no_pending_migration_recovery(db_path)?;
        crate::js::to_json(&out)
    })
}

/// `inspectPendingMigrationRecovery`.
#[napi(js_name = "inspectPendingMigrationRecovery")]
pub fn inspect_pending_migration_recovery(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::inspect_pending_migration_recovery(db_path)?;
        crate::js::to_json(&out)
    })
}

/// `restoreMarkedMigrationBackup`.
#[napi(js_name = "restoreMarkedMigrationBackup")]
pub fn restore_marked_migration_backup(db_path: String, options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::restore_marked_migration_backup(db_path, options)?;
        crate::js::to_json(&out)
    })
}

/// `createMigrationDivergenceConsentChallenge`.
#[napi(js_name = "createMigrationDivergenceConsentChallenge")]
pub fn create_migration_divergence_consent_challenge(
    db_path: String,
    plan: String,
) -> napi::Result<String> {
    run(move || {
        let plan = crate::js::from_json(&plan)?;
        let out = port::create_migration_divergence_consent_challenge(db_path, plan)?;
        crate::js::to_json(&out)
    })
}

/// `createMigrationSchemaTooNewStartupBlockError`.
#[napi(js_name = "createMigrationSchemaTooNewStartupBlockError")]
pub fn create_migration_schema_too_new_startup_block_error(
    db_path: String,
    cause: String,
) -> napi::Result<String> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::create_migration_schema_too_new_startup_block_error(db_path, cause)?;
        crate::js::to_json(&out)
    })
}

/// `makeMigrationLoader`.
#[napi(js_name = "makeMigrationLoader")]
pub fn make_migration_loader(through_id: String) -> napi::Result<String> {
    run(move || {
        let through_id = crate::js::from_json(&through_id)?;
        let out = port::make_migration_loader(through_id)?;
        crate::js::to_json(&out)
    })
}

/// `isReleasedMainMigrationGap`.
#[napi(js_name = "isReleasedMainMigrationGap")]
pub fn is_released_main_migration_gap(recorded_names_by_id: String) -> napi::Result<String> {
    run(move || {
        let recorded_names_by_id = crate::js::from_json(&recorded_names_by_id)?;
        let out = port::is_released_main_migration_gap(recorded_names_by_id)?;
        crate::js::to_json(&out)
    })
}

/// `planAccountMigrationRepair`.
#[napi(js_name = "planAccountMigrationRepair")]
pub fn plan_account_migration_repair(rows: String) -> napi::Result<String> {
    run(move || {
        let rows = crate::js::from_json(&rows)?;
        let out = port::plan_account_migration_repair(rows)?;
        crate::js::to_json(&out)
    })
}

/// `planLegacyMigration32Rename`.
#[napi(js_name = "planLegacyMigration32Rename")]
pub fn plan_legacy_migration32_rename(recorded_names_by_id: String) -> napi::Result<String> {
    run(move || {
        let recorded_names_by_id = crate::js::from_json(&recorded_names_by_id)?;
        let out = port::plan_legacy_migration32_rename(recorded_names_by_id)?;
        crate::js::to_json(&out)
    })
}

/// `findFirstMigrationLineageDivergence`.
#[napi(js_name = "findFirstMigrationLineageDivergence")]
pub fn find_first_migration_lineage_divergence(
    recorded_names_by_id: String,
    high_water_mark: String,
) -> napi::Result<String> {
    run(move || {
        let recorded_names_by_id = crate::js::from_json(&recorded_names_by_id)?;
        let high_water_mark = crate::js::from_json(&high_water_mark)?;
        let out =
            port::find_first_migration_lineage_divergence(recorded_names_by_id, high_water_mark)?;
        crate::js::to_json(&out)
    })
}

/// `planMigrationLineageAliasRepairs`.
#[napi(js_name = "planMigrationLineageAliasRepairs")]
pub fn plan_migration_lineage_alias_repairs(recorded_names_by_id: String) -> napi::Result<String> {
    run(move || {
        let recorded_names_by_id = crate::js::from_json(&recorded_names_by_id)?;
        let out = port::plan_migration_lineage_alias_repairs(recorded_names_by_id)?;
        crate::js::to_json(&out)
    })
}

/// `runMigrations`.
#[napi(js_name = "runMigrations")]
pub fn run_migrations(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::run_migrations(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `reclaimFreedSqlitePages`.
#[napi(js_name = "reclaimFreedSqlitePages")]
pub fn reclaim_freed_sqlite_pages(sql: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let out = port::reclaim_freed_sqlite_pages(sql)?;
        crate::js::to_json(&out)
    })
}

/// `reclaimLegacySqliteDatabase`.
#[napi(js_name = "reclaimLegacySqliteDatabase")]
pub fn reclaim_legacy_sqlite_database(db_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::reclaim_legacy_sqlite_database(db_path)?;
        crate::js::to_json(&out)
    })
}

/// `notifyAfterCommit`.
#[napi(js_name = "notifyAfterCommit")]
pub fn notify_after_commit(notification: String) -> napi::Result<String> {
    run(move || {
        let notification = crate::js::from_json(&notification)?;
        let out = port::notify_after_commit(notification)?;
        crate::js::to_json(&out)
    })
}

/// `selectMessageTextChunks`.
#[napi(js_name = "selectMessageTextChunks")]
pub fn select_message_text_chunks(
    sql: String,
    table: String,
    segment: String,
) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let segment = crate::js::from_json(&segment)?;
        let out = port::select_message_text_chunks(sql, table, segment)?;
        crate::js::to_json(&out)
    })
}

/// `selectSegmentEndedAt`.
#[napi(js_name = "selectSegmentEndedAt")]
pub fn select_segment_ended_at(sql: String, table: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let out = port::select_segment_ended_at(sql, table)?;
        crate::js::to_json(&out)
    })
}

/// `joinMessageTextChunks`.
#[napi(js_name = "joinMessageTextChunks")]
pub fn join_message_text_chunks(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::join_message_text_chunks(row)?;
        crate::js::to_json(&out)
    })
}

/// `encodeMessageTextFallback`.
#[napi(js_name = "encodeMessageTextFallback")]
pub fn encode_message_text_fallback(
    text: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::encode_message_text_fallback(text)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `makeMessageTextChunks`.
#[napi(js_name = "makeMessageTextChunks")]
pub fn make_message_text_chunks(sql: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let out = port::make_message_text_chunks(sql)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeLegacyModelSelection`.
#[napi(js_name = "normalizeLegacyModelSelection")]
pub fn normalize_legacy_model_selection(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::normalize_legacy_model_selection(input)?;
        crate::js::to_json(&out)
    })
}

/// `normalizePersistedModelSelection`.
#[napi(js_name = "normalizePersistedModelSelection")]
pub fn normalize_persisted_model_selection(
    input: String,
    settings: String,
) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let settings = crate::js::from_json(&settings)?;
        let out = port::normalize_persisted_model_selection(input, settings)?;
        crate::js::to_json(&out)
    })
}

/// `orchestrationMessageFromStoredMessage`.
#[napi(js_name = "orchestrationMessageFromStoredMessage")]
pub fn orchestration_message_from_stored_message(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::orchestration_message_from_stored_message(row)?;
        crate::js::to_json(&out)
    })
}

/// `projectionThreadMessageFromRow`.
#[napi(js_name = "projectionThreadMessageFromRow")]
pub fn projection_thread_message_from_row(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::projection_thread_message_from_row(row)?;
        crate::js::to_json(&out)
    })
}

/// `orchestrationMessageFromProjectionRow`.
#[napi(js_name = "orchestrationMessageFromProjectionRow")]
pub fn orchestration_message_from_projection_row(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::orchestration_message_from_projection_row(row)?;
        crate::js::to_json(&out)
    })
}

/// `resolveStoredPullRequestLinks`.
#[napi(js_name = "resolveStoredPullRequestLinks")]
pub fn resolve_stored_pull_request_links(row: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let out = port::resolve_stored_pull_request_links(row)?;
        crate::js::to_json(&out)
    })
}

/// `resolveSqliteMemoryBudget`.
#[napi(js_name = "resolveSqliteMemoryBudget")]
pub fn resolve_sqlite_memory_budget(total_memory_bytes: String) -> napi::Result<String> {
    run(move || {
        let total_memory_bytes = crate::js::from_json(&total_memory_bytes)?;
        let out = port::resolve_sqlite_memory_budget(total_memory_bytes)?;
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

static S_MIGRATION_TRACKER_FINGERPRINT_0: wire::Schema = wire::Schema::Dyn;

/// `migrationTrackerFingerprint` over the binary wire.
#[napi(js_name = "migrationTrackerFingerprintWire")]
pub fn migration_tracker_fingerprint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let rows: serde_json::Value = wire::read_value(r, &S_MIGRATION_TRACKER_FINGERPRINT_0)?;
        let out = &port::migration_tracker_fingerprint(rows)?;
        w.string16(out);
        Ok(())
    })
}

static S_CLASSIFY_ACCOUNT_MIGRATION_LINEAGE_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_ACCOUNT_MIGRATION_LINEAGE_1: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_ACCOUNT_MIGRATION_LINEAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifyAccountMigrationLineage` over the binary wire.
#[napi(js_name = "classifyAccountMigrationLineageWire")]
pub fn classify_account_migration_lineage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let rows: serde_json::Value = wire::read_value(r, &S_CLASSIFY_ACCOUNT_MIGRATION_LINEAGE_0)?;
        let canonical: serde_json::Value =
            wire::read_value(r, &S_CLASSIFY_ACCOUNT_MIGRATION_LINEAGE_1)?;
        let out = &port::classify_account_migration_lineage(rows, canonical)?;
        wire::write_value(w, out, &S_CLASSIFY_ACCOUNT_MIGRATION_LINEAGE_OUT)?;
        Ok(())
    })
}

static S_VALIDATE_ACCOUNT_MIGRATION_SCHEMA_0: wire::Schema = wire::Schema::Dyn;
static S_VALIDATE_ACCOUNT_MIGRATION_SCHEMA_1: wire::Schema = wire::Schema::Dyn;
static S_VALIDATE_ACCOUNT_MIGRATION_SCHEMA_2: wire::Schema = wire::Schema::Dyn;

/// `validateAccountMigrationSchema` over the binary wire.
#[napi(js_name = "validateAccountMigrationSchemaWire")]
pub fn validate_account_migration_schema_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let columns: serde_json::Value =
            wire::read_value(r, &S_VALIDATE_ACCOUNT_MIGRATION_SCHEMA_0)?;
        let rows: serde_json::Value = wire::read_value(r, &S_VALIDATE_ACCOUNT_MIGRATION_SCHEMA_1)?;
        let requires_identity: serde_json::Value =
            wire::read_value(r, &S_VALIDATE_ACCOUNT_MIGRATION_SCHEMA_2)?;
        let () = port::validate_account_migration_schema(columns, rows, requires_identity)?;
        Ok(())
    })
}

static S_ACQUIRE_DATABASE_LIFECYCLE_LOCK_OUT: wire::Schema = wire::Schema::Dyn;

/// `acquireDatabaseLifecycleLock` over the binary wire.
#[napi(js_name = "acquireDatabaseLifecycleLockWire")]
pub fn acquire_database_lifecycle_lock_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::acquire_database_lifecycle_lock(db_path)?;
        wire::write_value(w, out, &S_ACQUIRE_DATABASE_LIFECYCLE_LOCK_OUT)?;
        Ok(())
    })
}

static S_RELEASE_DATABASE_LIFECYCLE_LOCK_0: wire::Schema = wire::Schema::Dyn;
static S_RELEASE_DATABASE_LIFECYCLE_LOCK_OUT: wire::Schema = wire::Schema::Dyn;

/// `releaseDatabaseLifecycleLock` over the binary wire.
#[napi(js_name = "releaseDatabaseLifecycleLockWire")]
pub fn release_database_lifecycle_lock_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let lock: serde_json::Value = wire::read_value(r, &S_RELEASE_DATABASE_LIFECYCLE_LOCK_0)?;
        let out = &port::release_database_lifecycle_lock(lock)?;
        wire::write_value(w, out, &S_RELEASE_DATABASE_LIFECYCLE_LOCK_OUT)?;
        Ok(())
    })
}

static S_TO_PERSISTENCE_DECODE_ERROR_OUT: wire::Schema = wire::Schema::Dyn;

/// `toPersistenceDecodeError` over the binary wire.
#[napi(js_name = "toPersistenceDecodeErrorWire")]
pub fn to_persistence_decode_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let operation: String = r.string16()?;
        let out = &port::to_persistence_decode_error(operation)?;
        wire::write_value(w, out, &S_TO_PERSISTENCE_DECODE_ERROR_OUT)?;
        Ok(())
    })
}

static S_TO_PERSISTENCE_DECODE_CAUSE_ERROR_OUT: wire::Schema = wire::Schema::Dyn;

/// `toPersistenceDecodeCauseError` over the binary wire.
#[napi(js_name = "toPersistenceDecodeCauseErrorWire")]
pub fn to_persistence_decode_cause_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let operation: String = r.string16()?;
        let out = &port::to_persistence_decode_cause_error(operation)?;
        wire::write_value(w, out, &S_TO_PERSISTENCE_DECODE_CAUSE_ERROR_OUT)?;
        Ok(())
    })
}

static S_IS_PERSISTENCE_ERROR_0: wire::Schema = wire::Schema::Dyn;
static S_IS_PERSISTENCE_ERROR_OUT: wire::Schema = wire::Schema::Dyn;

/// `isPersistenceError` over the binary wire.
#[napi(js_name = "isPersistenceErrorWire")]
pub fn is_persistence_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: serde_json::Value = wire::read_value(r, &S_IS_PERSISTENCE_ERROR_0)?;
        let out = &port::is_persistence_error(u)?;
        wire::write_value(w, out, &S_IS_PERSISTENCE_ERROR_OUT)?;
        Ok(())
    })
}

static S_ESTIMATE_MIGRATION_BACKUP_REQUIRED_BYTES_OUT: wire::Schema = wire::Schema::Dyn;

/// `estimateMigrationBackupRequiredBytes` over the binary wire.
#[napi(js_name = "estimateMigrationBackupRequiredBytesWire")]
pub fn estimate_migration_backup_required_bytes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::estimate_migration_backup_required_bytes(db_path)?;
        wire::write_value(w, out, &S_ESTIMATE_MIGRATION_BACKUP_REQUIRED_BYTES_OUT)?;
        Ok(())
    })
}

static S_PRUNE_MIGRATION_BACKUPS_1: wire::Schema = wire::Schema::Dyn;
static S_PRUNE_MIGRATION_BACKUPS_OUT: wire::Schema = wire::Schema::Dyn;

/// `pruneMigrationBackups` over the binary wire.
#[napi(js_name = "pruneMigrationBackupsWire")]
pub fn prune_migration_backups_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let retention: serde_json::Value = wire::read_value(r, &S_PRUNE_MIGRATION_BACKUPS_1)?;
        let out = &port::prune_migration_backups(db_path, retention)?;
        wire::write_value(w, out, &S_PRUNE_MIGRATION_BACKUPS_OUT)?;
        Ok(())
    })
}

static S_CREATE_MIGRATION_BACKUP_1: wire::Schema = wire::Schema::Dyn;
static S_CREATE_MIGRATION_BACKUP_OUT: wire::Schema = wire::Schema::Dyn;

/// `createMigrationBackup` over the binary wire.
#[napi(js_name = "createMigrationBackupWire")]
pub fn create_migration_backup_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let plan: serde_json::Value = wire::read_value(r, &S_CREATE_MIGRATION_BACKUP_1)?;
        let out = &port::create_migration_backup(db_path, plan)?;
        wire::write_value(w, out, &S_CREATE_MIGRATION_BACKUP_OUT)?;
        Ok(())
    })
}

static S_INSPECT_SQLITE_MIGRATION_BACKUP_OUT: wire::Schema = wire::Schema::Dyn;

/// `inspectSqliteMigrationBackup` over the binary wire.
#[napi(js_name = "inspectSqliteMigrationBackupWire")]
pub fn inspect_sqlite_migration_backup_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let backup_path: String = r.string16()?;
        let out = &port::inspect_sqlite_migration_backup(backup_path)?;
        wire::write_value(w, out, &S_INSPECT_SQLITE_MIGRATION_BACKUP_OUT)?;
        Ok(())
    })
}

static S_INSPECT_COMPLETED_MIGRATION_BACKUP_FOR_SCHEMA_TOO_NEW_1: wire::Schema = wire::Schema::Dyn;
static S_INSPECT_COMPLETED_MIGRATION_BACKUP_FOR_SCHEMA_TOO_NEW_OUT: wire::Schema =
    wire::Schema::Dyn;

/// `inspectCompletedMigrationBackupForSchemaTooNew` over the binary wire.
#[napi(js_name = "inspectCompletedMigrationBackupForSchemaTooNewWire")]
pub fn inspect_completed_migration_backup_for_schema_too_new_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let input: serde_json::Value = wire::read_value(
            r,
            &S_INSPECT_COMPLETED_MIGRATION_BACKUP_FOR_SCHEMA_TOO_NEW_1,
        )?;
        let out = &port::inspect_completed_migration_backup_for_schema_too_new(db_path, input)?;
        wire::write_value(
            w,
            out,
            &S_INSPECT_COMPLETED_MIGRATION_BACKUP_FOR_SCHEMA_TOO_NEW_OUT,
        )?;
        Ok(())
    })
}

static S_RECLAIM_ORPHANED_MIGRATION_ARTIFACTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `reclaimOrphanedMigrationArtifacts` over the binary wire.
#[napi(js_name = "reclaimOrphanedMigrationArtifactsWire")]
pub fn reclaim_orphaned_migration_artifacts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::reclaim_orphaned_migration_artifacts(db_path)?;
        wire::write_value(w, out, &S_RECLAIM_ORPHANED_MIGRATION_ARTIFACTS_OUT)?;
        Ok(())
    })
}

static S_REQUIRE_NO_PENDING_MIGRATION_RECOVERY_OUT: wire::Schema = wire::Schema::Dyn;

/// `requireNoPendingMigrationRecovery` over the binary wire.
#[napi(js_name = "requireNoPendingMigrationRecoveryWire")]
pub fn require_no_pending_migration_recovery_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::require_no_pending_migration_recovery(db_path)?;
        wire::write_value(w, out, &S_REQUIRE_NO_PENDING_MIGRATION_RECOVERY_OUT)?;
        Ok(())
    })
}

static S_INSPECT_PENDING_MIGRATION_RECOVERY_OUT: wire::Schema = wire::Schema::Dyn;

/// `inspectPendingMigrationRecovery` over the binary wire.
#[napi(js_name = "inspectPendingMigrationRecoveryWire")]
pub fn inspect_pending_migration_recovery_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::inspect_pending_migration_recovery(db_path)?;
        wire::write_value(w, out, &S_INSPECT_PENDING_MIGRATION_RECOVERY_OUT)?;
        Ok(())
    })
}

static S_RESTORE_MARKED_MIGRATION_BACKUP_1: wire::Schema = wire::Schema::Dyn;
static S_RESTORE_MARKED_MIGRATION_BACKUP_OUT: wire::Schema = wire::Schema::Dyn;

/// `restoreMarkedMigrationBackup` over the binary wire.
#[napi(js_name = "restoreMarkedMigrationBackupWire")]
pub fn restore_marked_migration_backup_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let options: serde_json::Value = wire::read_value(r, &S_RESTORE_MARKED_MIGRATION_BACKUP_1)?;
        let out = &port::restore_marked_migration_backup(db_path, options)?;
        wire::write_value(w, out, &S_RESTORE_MARKED_MIGRATION_BACKUP_OUT)?;
        Ok(())
    })
}

static S_CREATE_MIGRATION_DIVERGENCE_CONSENT_CHALLENGE_1: wire::Schema = wire::Schema::Dyn;
static S_CREATE_MIGRATION_DIVERGENCE_CONSENT_CHALLENGE_OUT: wire::Schema =
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

/// `createMigrationDivergenceConsentChallenge` over the binary wire.
#[napi(js_name = "createMigrationDivergenceConsentChallengeWire")]
pub fn create_migration_divergence_consent_challenge_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let plan: serde_json::Value =
            wire::read_value(r, &S_CREATE_MIGRATION_DIVERGENCE_CONSENT_CHALLENGE_1)?;
        let out = &port::create_migration_divergence_consent_challenge(db_path, plan)?;
        wire::write_value(w, out, &S_CREATE_MIGRATION_DIVERGENCE_CONSENT_CHALLENGE_OUT)?;
        Ok(())
    })
}

static S_CREATE_MIGRATION_SCHEMA_TOO_NEW_STARTUP_BLOCK_ERROR_1: wire::Schema = wire::Schema::Dyn;
static S_CREATE_MIGRATION_SCHEMA_TOO_NEW_STARTUP_BLOCK_ERROR_OUT: wire::Schema = wire::Schema::Dyn;

/// `createMigrationSchemaTooNewStartupBlockError` over the binary wire.
#[napi(js_name = "createMigrationSchemaTooNewStartupBlockErrorWire")]
pub fn create_migration_schema_too_new_startup_block_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let cause: serde_json::Value =
            wire::read_value(r, &S_CREATE_MIGRATION_SCHEMA_TOO_NEW_STARTUP_BLOCK_ERROR_1)?;
        let out = &port::create_migration_schema_too_new_startup_block_error(db_path, cause)?;
        wire::write_value(
            w,
            out,
            &S_CREATE_MIGRATION_SCHEMA_TOO_NEW_STARTUP_BLOCK_ERROR_OUT,
        )?;
        Ok(())
    })
}

static S_MAKE_MIGRATION_LOADER_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_MIGRATION_LOADER_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeMigrationLoader` over the binary wire.
#[napi(js_name = "makeMigrationLoaderWire")]
pub fn make_migration_loader_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let through_id: serde_json::Value = wire::read_value(r, &S_MAKE_MIGRATION_LOADER_0)?;
        let out = &port::make_migration_loader(through_id)?;
        wire::write_value(w, out, &S_MAKE_MIGRATION_LOADER_OUT)?;
        Ok(())
    })
}

static S_IS_RELEASED_MAIN_MIGRATION_GAP_0: wire::Schema = wire::Schema::Dyn;
static S_IS_RELEASED_MAIN_MIGRATION_GAP_OUT: wire::Schema = wire::Schema::Bool;

/// `isReleasedMainMigrationGap` over the binary wire.
#[napi(js_name = "isReleasedMainMigrationGapWire")]
pub fn is_released_main_migration_gap_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let recorded_names_by_id: serde_json::Value =
            wire::read_value(r, &S_IS_RELEASED_MAIN_MIGRATION_GAP_0)?;
        let out = &port::is_released_main_migration_gap(recorded_names_by_id)?;
        wire::write_value(w, out, &S_IS_RELEASED_MAIN_MIGRATION_GAP_OUT)?;
        Ok(())
    })
}

static S_PLAN_ACCOUNT_MIGRATION_REPAIR_0: wire::Schema = wire::Schema::Dyn;
static S_PLAN_ACCOUNT_MIGRATION_REPAIR_OUT: wire::Schema = wire::Schema::Dyn;

/// `planAccountMigrationRepair` over the binary wire.
#[napi(js_name = "planAccountMigrationRepairWire")]
pub fn plan_account_migration_repair_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let rows: serde_json::Value = wire::read_value(r, &S_PLAN_ACCOUNT_MIGRATION_REPAIR_0)?;
        let out = &port::plan_account_migration_repair(rows)?;
        wire::write_value(w, out, &S_PLAN_ACCOUNT_MIGRATION_REPAIR_OUT)?;
        Ok(())
    })
}

static S_PLAN_LEGACY_MIGRATION32_RENAME_0: wire::Schema = wire::Schema::Dyn;
static S_PLAN_LEGACY_MIGRATION32_RENAME_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `planLegacyMigration32Rename` over the binary wire.
#[napi(js_name = "planLegacyMigration32RenameWire")]
pub fn plan_legacy_migration32_rename_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let recorded_names_by_id: serde_json::Value =
            wire::read_value(r, &S_PLAN_LEGACY_MIGRATION32_RENAME_0)?;
        let out = &port::plan_legacy_migration32_rename(recorded_names_by_id)?;
        wire::write_value(w, out, &S_PLAN_LEGACY_MIGRATION32_RENAME_OUT)?;
        Ok(())
    })
}

static S_FIND_FIRST_MIGRATION_LINEAGE_DIVERGENCE_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_FIRST_MIGRATION_LINEAGE_DIVERGENCE_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_FIRST_MIGRATION_LINEAGE_DIVERGENCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `findFirstMigrationLineageDivergence` over the binary wire.
#[napi(js_name = "findFirstMigrationLineageDivergenceWire")]
pub fn find_first_migration_lineage_divergence_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let recorded_names_by_id: serde_json::Value =
            wire::read_value(r, &S_FIND_FIRST_MIGRATION_LINEAGE_DIVERGENCE_0)?;
        let high_water_mark: serde_json::Value =
            wire::read_value(r, &S_FIND_FIRST_MIGRATION_LINEAGE_DIVERGENCE_1)?;
        let out =
            &port::find_first_migration_lineage_divergence(recorded_names_by_id, high_water_mark)?;
        wire::write_value(w, out, &S_FIND_FIRST_MIGRATION_LINEAGE_DIVERGENCE_OUT)?;
        Ok(())
    })
}

static S_PLAN_MIGRATION_LINEAGE_ALIAS_REPAIRS_0: wire::Schema = wire::Schema::Dyn;
static S_PLAN_MIGRATION_LINEAGE_ALIAS_REPAIRS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Dyn);

/// `planMigrationLineageAliasRepairs` over the binary wire.
#[napi(js_name = "planMigrationLineageAliasRepairsWire")]
pub fn plan_migration_lineage_alias_repairs_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let recorded_names_by_id: serde_json::Value =
            wire::read_value(r, &S_PLAN_MIGRATION_LINEAGE_ALIAS_REPAIRS_0)?;
        let out = &port::plan_migration_lineage_alias_repairs(recorded_names_by_id)?;
        wire::write_value(w, out, &S_PLAN_MIGRATION_LINEAGE_ALIAS_REPAIRS_OUT)?;
        Ok(())
    })
}

static S_RUN_MIGRATIONS_0: wire::Schema = wire::Schema::Dyn;
static S_RUN_MIGRATIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `runMigrations` over the binary wire.
#[napi(js_name = "runMigrationsWire")]
pub fn run_migrations_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: serde_json::Value = wire::read_value(r, &S_RUN_MIGRATIONS_0)?;
        let out = &port::run_migrations(arg0)?;
        wire::write_value(w, out, &S_RUN_MIGRATIONS_OUT)?;
        Ok(())
    })
}

static S_RECLAIM_FREED_SQLITE_PAGES_0: wire::Schema = wire::Schema::Dyn;
static S_RECLAIM_FREED_SQLITE_PAGES_OUT: wire::Schema = wire::Schema::Dyn;

/// `reclaimFreedSqlitePages` over the binary wire.
#[napi(js_name = "reclaimFreedSqlitePagesWire")]
pub fn reclaim_freed_sqlite_pages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_RECLAIM_FREED_SQLITE_PAGES_0)?;
        let out = &port::reclaim_freed_sqlite_pages(sql)?;
        wire::write_value(w, out, &S_RECLAIM_FREED_SQLITE_PAGES_OUT)?;
        Ok(())
    })
}

static S_RECLAIM_LEGACY_SQLITE_DATABASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `reclaimLegacySqliteDatabase` over the binary wire.
#[napi(js_name = "reclaimLegacySqliteDatabaseWire")]
pub fn reclaim_legacy_sqlite_database_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let db_path: String = r.string16()?;
        let out = &port::reclaim_legacy_sqlite_database(db_path)?;
        wire::write_value(w, out, &S_RECLAIM_LEGACY_SQLITE_DATABASE_OUT)?;
        Ok(())
    })
}

static S_NOTIFY_AFTER_COMMIT_0: wire::Schema = wire::Schema::Dyn;
static S_NOTIFY_AFTER_COMMIT_OUT: wire::Schema = wire::Schema::Dyn;

/// `notifyAfterCommit` over the binary wire.
#[napi(js_name = "notifyAfterCommitWire")]
pub fn notify_after_commit_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let notification: serde_json::Value = wire::read_value(r, &S_NOTIFY_AFTER_COMMIT_0)?;
        let out = &port::notify_after_commit(notification)?;
        wire::write_value(w, out, &S_NOTIFY_AFTER_COMMIT_OUT)?;
        Ok(())
    })
}

static S_SELECT_MESSAGE_TEXT_CHUNKS_0: wire::Schema = wire::Schema::Dyn;
static S_SELECT_MESSAGE_TEXT_CHUNKS_2: wire::Schema = wire::Schema::Dyn;
static S_SELECT_MESSAGE_TEXT_CHUNKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `selectMessageTextChunks` over the binary wire.
#[napi(js_name = "selectMessageTextChunksWire")]
pub fn select_message_text_chunks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_SELECT_MESSAGE_TEXT_CHUNKS_0)?;
        let table: String = r.string16()?;
        let segment: serde_json::Value = wire::read_value(r, &S_SELECT_MESSAGE_TEXT_CHUNKS_2)?;
        let out = &port::select_message_text_chunks(sql, table, segment)?;
        wire::write_value(w, out, &S_SELECT_MESSAGE_TEXT_CHUNKS_OUT)?;
        Ok(())
    })
}

static S_SELECT_SEGMENT_ENDED_AT_0: wire::Schema = wire::Schema::Dyn;
static S_SELECT_SEGMENT_ENDED_AT_OUT: wire::Schema = wire::Schema::Dyn;

/// `selectSegmentEndedAt` over the binary wire.
#[napi(js_name = "selectSegmentEndedAtWire")]
pub fn select_segment_ended_at_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_SELECT_SEGMENT_ENDED_AT_0)?;
        let table: String = r.string16()?;
        let out = &port::select_segment_ended_at(sql, table)?;
        wire::write_value(w, out, &S_SELECT_SEGMENT_ENDED_AT_OUT)?;
        Ok(())
    })
}

static S_JOIN_MESSAGE_TEXT_CHUNKS_0: wire::Schema = wire::Schema::Dyn;
static S_JOIN_MESSAGE_TEXT_CHUNKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `joinMessageTextChunks` over the binary wire.
#[napi(js_name = "joinMessageTextChunksWire")]
pub fn join_message_text_chunks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_JOIN_MESSAGE_TEXT_CHUNKS_0)?;
        let out = &port::join_message_text_chunks(row)?;
        wire::write_value(w, out, &S_JOIN_MESSAGE_TEXT_CHUNKS_OUT)?;
        Ok(())
    })
}

/// `encodeMessageTextFallback` over the binary wire.
#[napi(js_name = "encodeMessageTextFallbackWire")]
pub fn encode_message_text_fallback_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::encode_message_text_fallback(text)?;
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

static S_MAKE_MESSAGE_TEXT_CHUNKS_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_MESSAGE_TEXT_CHUNKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeMessageTextChunks` over the binary wire.
#[napi(js_name = "makeMessageTextChunksWire")]
pub fn make_message_text_chunks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_MAKE_MESSAGE_TEXT_CHUNKS_0)?;
        let out = &port::make_message_text_chunks(sql)?;
        wire::write_value(w, out, &S_MAKE_MESSAGE_TEXT_CHUNKS_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_LEGACY_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_LEGACY_MODEL_SELECTION_OUT: wire::Schema =
    wire::Schema::Map16(&wire::Schema::Dyn);

/// `normalizeLegacyModelSelection` over the binary wire.
#[napi(js_name = "normalizeLegacyModelSelectionWire")]
pub fn normalize_legacy_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_NORMALIZE_LEGACY_MODEL_SELECTION_0)?;
        let out = &port::normalize_legacy_model_selection(input)?;
        wire::write_value(w, out, &S_NORMALIZE_LEGACY_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_PERSISTED_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PERSISTED_MODEL_SELECTION_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PERSISTED_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizePersistedModelSelection` over the binary wire.
#[napi(js_name = "normalizePersistedModelSelectionWire")]
pub fn normalize_persisted_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_PERSISTED_MODEL_SELECTION_0)?;
        let settings: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_PERSISTED_MODEL_SELECTION_1)?;
        let out = &port::normalize_persisted_model_selection(input, settings)?;
        wire::write_value(w, out, &S_NORMALIZE_PERSISTED_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_ORCHESTRATION_MESSAGE_FROM_STORED_MESSAGE_0: wire::Schema = wire::Schema::Dyn;
static S_ORCHESTRATION_MESSAGE_FROM_STORED_MESSAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `orchestrationMessageFromStoredMessage` over the binary wire.
#[napi(js_name = "orchestrationMessageFromStoredMessageWire")]
pub fn orchestration_message_from_stored_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value =
            wire::read_value(r, &S_ORCHESTRATION_MESSAGE_FROM_STORED_MESSAGE_0)?;
        let out = &port::orchestration_message_from_stored_message(row)?;
        wire::write_value(w, out, &S_ORCHESTRATION_MESSAGE_FROM_STORED_MESSAGE_OUT)?;
        Ok(())
    })
}

static S_PROJECTION_THREAD_MESSAGE_FROM_ROW_0: wire::Schema = wire::Schema::Dyn;
static S_PROJECTION_THREAD_MESSAGE_FROM_ROW_OUT: wire::Schema = wire::Schema::Dyn;

/// `projectionThreadMessageFromRow` over the binary wire.
#[napi(js_name = "projectionThreadMessageFromRowWire")]
pub fn projection_thread_message_from_row_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_PROJECTION_THREAD_MESSAGE_FROM_ROW_0)?;
        let out = &port::projection_thread_message_from_row(row)?;
        wire::write_value(w, out, &S_PROJECTION_THREAD_MESSAGE_FROM_ROW_OUT)?;
        Ok(())
    })
}

static S_ORCHESTRATION_MESSAGE_FROM_PROJECTION_ROW_0: wire::Schema = wire::Schema::Dyn;
static S_ORCHESTRATION_MESSAGE_FROM_PROJECTION_ROW_OUT: wire::Schema = wire::Schema::Dyn;

/// `orchestrationMessageFromProjectionRow` over the binary wire.
#[napi(js_name = "orchestrationMessageFromProjectionRowWire")]
pub fn orchestration_message_from_projection_row_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value =
            wire::read_value(r, &S_ORCHESTRATION_MESSAGE_FROM_PROJECTION_ROW_0)?;
        let out = &port::orchestration_message_from_projection_row(row)?;
        wire::write_value(w, out, &S_ORCHESTRATION_MESSAGE_FROM_PROJECTION_ROW_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_STORED_PULL_REQUEST_LINKS_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_STORED_PULL_REQUEST_LINKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveStoredPullRequestLinks` over the binary wire.
#[napi(js_name = "resolveStoredPullRequestLinksWire")]
pub fn resolve_stored_pull_request_links_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: serde_json::Value = wire::read_value(r, &S_RESOLVE_STORED_PULL_REQUEST_LINKS_0)?;
        let out = &port::resolve_stored_pull_request_links(row)?;
        wire::write_value(w, out, &S_RESOLVE_STORED_PULL_REQUEST_LINKS_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_SQLITE_MEMORY_BUDGET_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_SQLITE_MEMORY_BUDGET_OUT: wire::Schema = wire::Schema::Obj(&[
    ("cacheSizePragma", wire::Schema::Num),
    ("mmapSizeBytes", wire::Schema::Num),
]);

/// `resolveSqliteMemoryBudget` over the binary wire.
#[napi(js_name = "resolveSqliteMemoryBudgetWire")]
pub fn resolve_sqlite_memory_budget_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let total_memory_bytes: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_SQLITE_MEMORY_BUDGET_0)?;
        let out = &port::resolve_sqlite_memory_budget(total_memory_bytes)?;
        wire::write_value(w, out, &S_RESOLVE_SQLITE_MEMORY_BUDGET_OUT)?;
        Ok(())
    })
}
