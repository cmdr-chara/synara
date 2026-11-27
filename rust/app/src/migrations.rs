//! Migration runner ported from upstream `apps/server/src/persistence/Migrations.ts`
//! plus effect's `unstable/sql/Migrator` semantics for the sqlite dialect:
//!
//! - tracker table `effect_sql_migrations(migration_id integer PRIMARY KEY
//!   NOT NULL, created_at datetime NOT NULL DEFAULT current_timestamp, name
//!   VARCHAR(255) NOT NULL)`
//! - one transaction around the whole run: the tracker rows for every pending
//!   migration are inserted first (a second concurrent runner fails the insert
//!   and reports "migrations already running"), then each migration executes
//!   in id order; any failure rolls the transaction back, tracker included.
//! - upstream lineage validation (`AccountMigrationLineage.ts`): private-build
//!   divergent tracker rows are recognized and repaired before the canonical
//!   run resumes after the lineage prefix.

use rusqlite::{params, Connection};

use crate::acp_runner::err;
use crate::todo::WsErr;

mod automation_identity;
mod model_selection_compat;
mod batch1;
mod helpers;

pub use helpers::{column_exists, primary_key_columns, table_exists};

/// (id, name) in canonical order — the same list `Migrations.ts` assembles
/// and `scripts/check-migration-lineage.ts` parses. Keep ids literal.
pub fn canonical_migrations() -> Vec<(u32, &'static str, MigrationFn)> {
    batch1::migrations()
}

pub type MigrationFn = fn(&Connection) -> Result<(), WsErr>;

fn ensure_tracker(conn: &Connection) -> Result<(), WsErr> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS effect_sql_migrations (
            migration_id integer PRIMARY KEY NOT NULL,
            created_at datetime NOT NULL DEFAULT current_timestamp,
            name VARCHAR(255) NOT NULL
        )",
    )
    .map_err(|e| err(format!("create migrations tracker: {e}")))
}

fn tracker_rows(conn: &Connection) -> Result<Vec<(u32, String)>, WsErr> {
    let mut stmt = conn
        .prepare("SELECT migration_id, name FROM effect_sql_migrations ORDER BY migration_id ASC")
        .map_err(|e| err(format!("read migrations tracker: {e}")))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| err(format!("read migrations tracker rows: {e}")))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Inspect tracker rows against the canonical list without mutating the
/// database. Private-build account lineage tails are recognized and represented
/// as a repair plan (`delete_above`), which `run_pending` executes inside the
/// migration transaction only after a pre-migration backup exists.
fn inspect_lineage(rows: &[(u32, String)]) -> Result<(Vec<(u32, String)>, Option<u32>), WsErr> {
    let canonical = canonical_migrations();
    let canonical_name = |id: u32| -> Option<&'static str> {
        canonical.iter().find(|(cid, _, _)| *cid == id).map(|(_, n, _)| *n)
    };
    if rows.len() > canonical.len() {
        let canonical_prefix_valid = rows.iter().take(canonical.len()).enumerate().all(|(index, (id, name))| {
            *id == index as u32 + 1 && canonical_name(*id) == Some(name.as_str())
        });
        if canonical_prefix_valid {
            let schema = rows.last().map(|(id, _)| *id).unwrap_or(0);
            return Err(err(format!(
                "database schema v{schema} is newer than this build supports (v{}). Update Synara before opening this data home.",
                canonical.len()
            )));
        }
    }
    let mut mismatches = Vec::new();
    for (index, (id, name)) in rows.iter().enumerate() {
        let expected_id = index as u32 + 1;
        if *id != expected_id {
            mismatches.push(format!("tracker id {id} at position {expected_id}"));
            continue;
        }
        if canonical_name(*id) != Some(name.as_str()) {
            mismatches.push(format!("tracker {id} named {name}"));
        }
    }
    if mismatches.is_empty() {
        return Ok((rows.to_vec(), None));
    }
    for lineage in ACCOUNT_LINEAGES {
        let prefix = lineage.0;
        let tail: &[&str] = lineage.1;
        let tail_len = rows.len() as u32 - prefix;
        if rows.len() as u32 <= prefix || tail_len < 1 || tail_len > tail.len() as u32 {
            continue;
        }
        let matches = rows.iter().enumerate().all(|(index, (id, name))| {
            let pos = index as u32 + 1;
            if *id != pos {
                return false;
            }
            if pos <= prefix {
                canonical_name(pos) == Some(name.as_str())
            } else {
                tail.get((pos - prefix - 1) as usize) == Some(&name.as_str())
            }
        });
        if matches {
            let remaining: Vec<(u32, String)> =
                rows.iter().take(prefix as usize).cloned().collect();
            return Ok((remaining, Some(prefix)));
        }
    }
    Err(err(format!(
        "migration lineage mismatch: {}",
        mismatches.join(", ")
    )))
}

/// Private account lineages from upstream AccountMigrationLineage.ts: prefix
/// id after which the canonical run resumes, then the private tail names.
const ACCOUNT_LINEAGES: &[(u32, &[&str])] = &[
    (134, &["AccountUsageSync", "AccountUsageSyncIdentity", "RemoteDeviceTrust", "RemoteConnectionPreferences", "RemoteAccessControls"]),
    (131, &["AccountUsageSync", "AccountUsageSyncIdentity", "RemoteDeviceTrust", "RemoteConnectionPreferences", "RemoteAccessControls"]),
    (130, &["AccountUsageSync", "AccountUsageSyncIdentity", "RemoteDeviceTrust", "RemoteConnectionPreferences", "RemoteAccessControls"]),
    (89, &["AccountUsageSync"]),
    (90, &["AccountUsageSync"]),
    (90, &["AccountUsageSync", "AccountUsageSyncIdentity"]),
    (98, &["AccountUsageSync", "AccountUsageSyncIdentity"]),
    (99, &["AccountUsageSync", "AccountUsageSyncIdentity"]),
    (108, &["AccountUsageSync", "AccountUsageSyncIdentity", "RemoteDeviceTrust", "RemoteConnectionPreferences"]),
    (126, &["AccountUsageSync", "AccountUsageSyncIdentity", "RemoteDeviceTrust", "RemoteConnectionPreferences"]),
];

/// Run every pending migration inside one transaction, mirroring the effect
/// Migrator run: insert tracker rows for all pending migrations first (the
/// lock), then execute in id order. Any failure rolls everything back.
/// Read-only lookahead: which migration ids would run against this database.
/// Backs the backup decision so fully migrated databases skip the copy.
pub fn pending_migrations(conn: &Connection) -> Result<Vec<u32>, WsErr> {
    ensure_tracker(conn)?;
    let rows = tracker_rows(conn)?;
    let (validated, _) = inspect_lineage(rows.as_slice())?;
    let latest = validated.iter().map(|(id, _)| *id).max().unwrap_or(0);
    Ok(canonical_migrations()
        .iter()
        .filter(|(id, _, _)| *id > latest)
        .map(|(id, _, _)| *id)
        .collect())
}

pub fn run_pending(conn: &Connection) -> Result<Vec<u32>, WsErr> {
    ensure_tracker(conn)?;
    let rows = tracker_rows(conn)?;
    let (validated, repair_delete_above) = inspect_lineage(rows.as_slice())?;
    let latest = validated.iter().map(|(id, _)| *id).max().unwrap_or(0);
    let all = canonical_migrations();
    let pending: Vec<&(u32, &'static str, MigrationFn)> =
        all.iter().filter(|(id, _, _)| *id > latest).collect();
    if pending.is_empty() {
        return Ok(Vec::new());
    }
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| err(format!("migration transaction: {e}")))?;
    if let Some(prefix) = repair_delete_above {
        tx.execute(
            "DELETE FROM effect_sql_migrations WHERE migration_id > ?1",
            params![prefix],
        )
        .map_err(|e| err(format!("lineage repair delete: {e}")))?;
    }
    for (id, name, _) in &pending {
        tx.execute(
            "INSERT INTO effect_sql_migrations (migration_id, name) VALUES (?1, ?2)",
            params![id, name],
        )
        .map_err(|e| err(format!("migrations already running: {e}")))?;
    }
    let mut ran = Vec::new();
    for (id, name, f) in &pending {
        f(&tx).map_err(|e| err(format!("migration {id}_{name} failed: {}", e.0)))?;
        ran.push(*id);
    }
    tx.commit().map_err(|e| err(format!("migration commit: {e}")))?;
    Ok(ran)
}

/// Open `state.sqlite` in the state directory and run every pending migration.
/// Returns the ids applied in this run; an empty vector means the database was
/// already current. Exclusivity comes from the single-transaction runner plus
/// the tracker-first lock, matching the upstream effect Migrator semantics.
pub const MIGRATION_BACKUP_RETENTION: usize = 5;
const MIGRATION_BACKUP_FREE_SPACE_FACTOR: u64 = 2;

#[derive(Clone, Debug)]
struct MigrationBackupPlan {
    source_version: String,
    target_version: u32,
}

#[derive(Clone, Debug)]
struct MigrationBackupResult {
    plan: MigrationBackupPlan,
    backup_path: std::path::PathBuf,
    created_at: String,
}

fn migration_backup_directory(db: &std::path::Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.backups", db.display()))
}

fn migration_recovery_marker_path(db: &std::path::Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.migration-recovery.json", db.display()))
}

fn migration_backup_provenance_path(db: &std::path::Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.migration-backup.json", db.display()))
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn compact_timestamp(iso: &str) -> String {
    iso.chars().filter(|ch| !matches!(ch, '-' | ':' | '.')).collect()
}

fn safe_version_label(value: &str) -> String {
    value.chars().map(|ch| if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' { ch } else { '-' }).collect()
}

fn latest_migration_id() -> u32 {
    canonical_migrations().iter().map(|(id, _, _)| *id).max().unwrap_or(0)
}

/// Read-only upstream backup-plan inspection. It runs before the runner creates
/// or repairs the tracker so an untracked user database is still recognized.
fn inspect_migration_backup_plan(conn: &Connection) -> Result<Option<MigrationBackupPlan>, WsErr> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|e| err(format!("inspect migration tables: {e}")))?;
    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| err(format!("inspect migration tables: {e}")))?
        .filter_map(|row| row.ok())
        .collect();
    if tables.is_empty() {
        return Ok(None);
    }
    let target_version = latest_migration_id();
    if !tables.iter().any(|name| name == "effect_sql_migrations") {
        return Ok(Some(MigrationBackupPlan { source_version: "untracked".into(), target_version }));
    }
    let recorded = (|| -> Result<Vec<(u32, String)>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT migration_id, name FROM effect_sql_migrations ORDER BY migration_id ASC")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?)))?;
        rows.collect()
    })();
    let recorded = match recorded {
        Ok(rows) => rows,
        Err(_) => return Ok(Some(MigrationBackupPlan { source_version: "malformed-tracker".into(), target_version })),
    };
    if recorded.is_empty() {
        let user_tables = tables.iter().filter(|name| name.as_str() != "effect_sql_migrations").count();
        return Ok((user_tables > 0).then_some(MigrationBackupPlan { source_version: "untracked".into(), target_version }));
    }
    let high_water_mark = recorded.iter().map(|(id, _)| *id).max().unwrap_or(0);
    Ok((high_water_mark < target_version).then_some(MigrationBackupPlan {
        source_version: format!("v{high_water_mark}"),
        target_version,
    }))
}

fn estimate_backup_required_bytes(conn: &Connection, db: &std::path::Path) -> Option<u64> {
    let logical = conn
        .query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get::<_, i64>(0),
        )
        .ok()
        .filter(|bytes| *bytes >= 0)
        .map(|bytes| bytes as u64);
    let main = std::fs::metadata(db).ok()?.len();
    let snapshot = if let Some(logical) = logical {
        main.max(logical)
    } else {
        let wal = std::fs::metadata(format!("{}-wal", db.display())).map(|meta| meta.len()).unwrap_or(0);
        main.saturating_add(wal)
    };
    Some(
        snapshot
            .saturating_mul(MIGRATION_BACKUP_FREE_SPACE_FACTOR)
            .saturating_add(snapshot / 4)
            .saturating_add(64 * 1024),
    )
}

#[cfg(unix)]
fn available_bytes(path: &std::path::Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt as _;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        (libc::statvfs(path.as_ptr(), &mut stat) == 0)
            .then(|| (stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64))
    }
}

#[cfg(not(unix))]
fn available_bytes(_path: &std::path::Path) -> Option<u64> { None }

fn assert_backup_space(required: Option<u64>, directory: &std::path::Path) -> Result<(), String> {
    let Some(required) = required else { return Ok(()) };
    let Some(available) = available_bytes(directory) else { return Ok(()) };
    if available < required {
        return Err(format!(
            "not enough free disk space to back up the database before upgrading it: need about {} bytes in {}, only {} bytes free",
            required,
            directory.display(),
            available
        ));
    }
    Ok(())
}

fn ensure_private_dir(path: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| format!("backup directory: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("backup directory permissions: {e}"))?;
    }
    Ok(())
}

fn ensure_private_file(path: &std::path::Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("private file permissions: {e}"))?;
    }
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("private file inspection: {e}"))?;
    if !meta.file_type().is_file() || meta.file_type().is_symlink() {
        return Err(format!("migration backup is not a regular file: {}", path.display()));
    }
    Ok(())
}

fn sync_directory(path: &std::path::Path) {
    #[cfg(unix)]
    if let Ok(dir) = std::fs::File::open(path) {
        let _ = dir.sync_all();
    }
}

fn write_private_json(path: &std::path::Path, value: &serde_json::Value) -> Result<(), String> {
    use std::io::Write as _;
    let parent = path.parent().ok_or("path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(
        ".{}.{}.partial",
        path.file_name().and_then(|name| name.to_str()).unwrap_or("json"),
        crate::orchestration_engine::uuid_v4()
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(&temporary).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600));
        }
        file.write_all(format!("{}\n", serde_json::to_string_pretty(value).map_err(|e| e.to_string())?).as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        sync_directory(parent);
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn prune_migration_backups(db: &std::path::Path) {
    let directory = migration_backup_directory(db);
    let basename = db.file_name().and_then(|name| name.to_str()).unwrap_or("state.sqlite");
    let prefix = format!("{basename}.pre-migration-");
    let Ok(entries) = std::fs::read_dir(&directory) else { return };
    let mut backups: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with(&prefix) && (name.ends_with(".sqlite") || name.ends_with(".sqlite.gz"))
        })
        .collect();
    backups.sort_by_key(|entry| entry.file_name());
    backups.reverse();
    for entry in backups.into_iter().skip(MIGRATION_BACKUP_RETENTION) {
        let path = entry.path();
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }
}

fn remove_backup_partials(db: &std::path::Path) {
    let directory = migration_backup_directory(db);
    let basename = db.file_name().and_then(|name| name.to_str()).unwrap_or("state.sqlite");
    let prefix = format!(".{basename}.pre-migration-");
    let Ok(entries) = std::fs::read_dir(&directory) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(&prefix) || !name.ends_with(".partial") { continue; }
        let path = entry.path();
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Creates the upstream-shaped compressed pre-migration snapshot. The database
/// is copied through `VACUUM INTO` into an exclusive private scratch directory,
/// gzip-compressed into `<db>.backups`, fsynced, atomically renamed and rotated
/// at the upstream retention of five.
pub fn create_migration_backup(conn: &Connection, db: &std::path::Path, plan: &MigrationBackupPlan) -> Result<MigrationBackupResult, String> {
    use std::io::{Read as _, Write as _};
    let directory = migration_backup_directory(db);
    ensure_private_dir(&directory)?;
    remove_backup_partials(db);
    assert_backup_space(estimate_backup_required_bytes(conn, db), &directory)?;
    let created_at = now_iso();
    let basename = db.file_name().and_then(|name| name.to_str()).unwrap_or("state.sqlite");
    let unique_suffix = format!("{}-{}", compact_timestamp(&created_at), crate::orchestration_engine::uuid_v4());
    let final_name = format!(
        "{basename}.pre-migration-{}-to-v{}-{unique_suffix}.sqlite",
        safe_version_label(&plan.source_version),
        plan.target_version
    );
    let backup_path = directory.join(format!("{final_name}.gz"));
    let snapshot_directory = directory.join(format!(".{final_name}.partial"));
    let snapshot_path = snapshot_directory.join("snapshot.sqlite");
    let compressed_temporary = directory.join(format!(".{final_name}.gz.partial"));
    let result = (|| -> Result<(), String> {
        std::fs::create_dir(&snapshot_directory).map_err(|e| format!("backup scratch directory: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(&snapshot_directory, std::fs::Permissions::from_mode(0o700));
        }
        conn.execute("VACUUM INTO ?1", params![snapshot_path.to_string_lossy().as_ref()])
            .map_err(|e| format!("backup snapshot: {e}"))?;
        ensure_private_file(&snapshot_path)?;
        let mut input = std::fs::File::open(&snapshot_path).map_err(|e| format!("backup snapshot open: {e}"))?;
        let output = std::fs::OpenOptions::new().create_new(true).write(true).open(&compressed_temporary)
            .map_err(|e| format!("backup compressed open: {e}"))?;
        let mut encoder = flate2::write::GzEncoder::new(output, flate2::Compression::default());
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let size = input.read(&mut buffer).map_err(|e| format!("backup snapshot read: {e}"))?;
            if size == 0 { break; }
            encoder.write_all(&buffer[..size]).map_err(|e| format!("backup compression: {e}"))?;
        }
        let output = encoder.finish().map_err(|e| format!("backup compression finish: {e}"))?;
        output.sync_all().map_err(|e| format!("backup sync: {e}"))?;
        ensure_private_file(&compressed_temporary)?;
        std::fs::rename(&compressed_temporary, &backup_path).map_err(|e| format!("backup rename: {e}"))?;
        sync_directory(&directory);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&snapshot_directory);
    let _ = std::fs::remove_file(&compressed_temporary);
    result?;
    prune_migration_backups(db);
    Ok(MigrationBackupResult { plan: plan.clone(), backup_path, created_at })
}

fn recovery_payload(db: &std::path::Path, backup: &MigrationBackupResult) -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "databasePath": db.to_string_lossy(),
        "backupPath": backup.backup_path.to_string_lossy(),
        "sourceVersion": backup.plan.source_version,
        "targetVersion": backup.plan.target_version,
        "lineageDivergence": serde_json::Value::Null,
        "phase": "migration-in-progress",
        "createdAt": backup.created_at,
        "resumeAttempts": 0,
        "restore": {"executable":"synara-restore-migration-backup","arguments":[db.to_string_lossy()]},
        "recovery": "Stop every Synara process, then run the explicit migration-backup restore command for this database."
    })
}

fn write_recovery_marker(db: &std::path::Path, payload: &serde_json::Value) -> Result<(), String> {
    write_private_json(&migration_recovery_marker_path(db), payload)
}

fn write_completed_provenance(db: &std::path::Path, payload: &serde_json::Value) -> Result<(), String> {
    let mut completed = payload.clone();
    completed["phase"] = serde_json::json!("migration-completed");
    completed["completedAt"] = serde_json::json!(now_iso());
    write_private_json(&migration_backup_provenance_path(db), &completed)
}


fn read_restore_record(db: &std::path::Path, path: &std::path::Path, label: &str) -> Result<Option<serde_json::Value>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{label} could not be read: {error}")),
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("invalid {label}: {e}"))?;
    if value.get("databasePath").and_then(serde_json::Value::as_str) != Some(db.to_string_lossy().as_ref()) {
        return Err(format!("invalid {label}: database path mismatch"));
    }
    if value.get("resumeAttempts").and_then(serde_json::Value::as_u64).is_none() {
        return Err(format!("invalid {label}: unreadable resume counter"));
    }
    Ok(Some(value))
}

fn validate_restore_backup_path(db: &std::path::Path, value: &serde_json::Value) -> Result<std::path::PathBuf, String> {
    let raw = value.get("backupPath").and_then(serde_json::Value::as_str).ok_or("migration record does not reference a backup")?;
    let backup = std::path::PathBuf::from(raw);
    let directory = migration_backup_directory(db);
    if backup.parent() != Some(directory.as_path()) {
        return Err(format!("migration backup escapes its backup directory: {}", backup.display()));
    }
    let basename = db.file_name().and_then(|name| name.to_str()).unwrap_or("state.sqlite");
    let name = backup.file_name().and_then(|name| name.to_str()).unwrap_or("");
    if !name.starts_with(&format!("{basename}.pre-migration-")) || !(name.ends_with(".sqlite.gz") || name.ends_with(".sqlite")) {
        return Err(format!("migration backup has an invalid generated name: {name}"));
    }
    let meta = std::fs::symlink_metadata(&backup).map_err(|e| format!("migration backup is unavailable: {e}"))?;
    if !meta.file_type().is_file() || meta.file_type().is_symlink() {
        return Err(format!("migration backup is not a regular file: {}", backup.display()));
    }
    let canonical_directory = directory.canonicalize().map_err(|e| format!("backup directory canonicalization: {e}"))?;
    let canonical_backup = backup.canonicalize().map_err(|e| format!("backup canonicalization: {e}"))?;
    if canonical_backup.parent() != Some(canonical_directory.as_path()) {
        return Err(format!("migration backup escapes its canonical directory: {}", backup.display()));
    }
    Ok(backup)
}

fn restore_file_fingerprint(path: &std::path::Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.file_type().is_file() { return None; }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        return Some(format!("{}:{}:{}:{}", meta.dev(), meta.ino(), meta.size(), meta.mtime_nsec()));
    }
    #[cfg(not(unix))]
    Some(format!("{}", meta.len()))
}

fn copy_restore_backup(backup: &std::path::Path, temporary: &std::path::Path) -> Result<(), String> {
    use std::io::{Read as _, Write as _};
    let input = std::fs::File::open(backup).map_err(|e| format!("backup open: {e}"))?;
    if let Ok(meta) = std::fs::symlink_metadata(temporary) {
        if !meta.file_type().is_file() || meta.file_type().is_symlink() {
            return Err(format!("restore temporary path is not a regular file: {}", temporary.display()));
        }
        std::fs::remove_file(temporary).map_err(|e| format!("restore temporary cleanup: {e}"))?;
    }
    let mut output = std::fs::OpenOptions::new().create_new(true).write(true).open(temporary)
        .map_err(|e| format!("restore temporary open: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(temporary, std::fs::Permissions::from_mode(0o600));
    }
    if backup.extension().and_then(|ext| ext.to_str()) == Some("gz") {
        let mut decoder = flate2::read::GzDecoder::new(input);
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let size = decoder.read(&mut buffer).map_err(|e| format!("backup decompression: {e}"))?;
            if size == 0 { break; }
            output.write_all(&buffer[..size]).map_err(|e| format!("restore write: {e}"))?;
        }
    } else {
        let mut input = input;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let size = input.read(&mut buffer).map_err(|e| format!("backup read: {e}"))?;
            if size == 0 { break; }
            output.write_all(&buffer[..size]).map_err(|e| format!("restore write: {e}"))?;
        }
    }
    output.sync_all().map_err(|e| format!("restore sync: {e}"))?;
    drop(output);
    let head = std::fs::read(temporary).map_err(|e| format!("restored database validation: {e}"))?;
    if !head.starts_with(b"SQLite format 3") {
        return Err("restored migration backup is not a SQLite database".into());
    }
    Ok(())
}

fn move_existing_database_bundle_aside(db: &std::path::Path, restore_id: &str) -> Result<(), String> {
    for suffix in ["", "-wal", "-shm"] {
        let source = std::path::PathBuf::from(format!("{}{}", db.display(), suffix));
        if !source.exists() { continue; }
        let target = std::path::PathBuf::from(format!("{}.pre-restore-{}{}", db.display(), restore_id, suffix));
        if target.exists() && !source.exists() { continue; }
        std::fs::rename(&source, &target).map_err(|e| format!("move live database aside: {e}"))?;
    }
    Ok(())
}

/// Explicit one-shot restore for the active recovery marker or, when no active
/// marker exists, the latest completed migration provenance. Startup never calls
/// this: the operator invokes it after stopping every Synara process.
pub fn restore_marked_migration_backup(db: &std::path::Path) -> Result<(), String> {
    let marker_path = migration_recovery_marker_path(db);
    let provenance_path = migration_backup_provenance_path(db);
    let mut marker = read_restore_record(db, &marker_path, "migration recovery marker")?;
    if marker.is_none() {
        marker = read_restore_record(db, &provenance_path, "migration backup provenance")?;
        if marker.as_ref().and_then(|value| value.get("phase")).and_then(serde_json::Value::as_str) != Some("migration-completed") {
            return Err(format!("no migration recovery marker or completed backup provenance exists for {}", db.display()));
        }
    }
    let mut marker = marker.ok_or("missing migration recovery record")?;
    let backup = validate_restore_backup_path(db, &marker)?;
    let restore_id = marker.get("restoreId").and_then(serde_json::Value::as_str).map(str::to_string)
        .unwrap_or_else(crate::orchestration_engine::uuid_v4);
    let temporary = db.parent().unwrap_or(std::path::Path::new(".")).join(format!(".restore-{restore_id}.partial"));

    // If a previous restore died after the atomic install rename, the stored
    // fingerprint proves the live file is already the restored snapshot. Do not
    // move it aside again; only finish the durable bookkeeping.
    let expected_fingerprint = marker.get("restoredDatabaseFingerprint").and_then(serde_json::Value::as_str);
    let already_installed = expected_fingerprint.is_some_and(|expected| {
        restore_file_fingerprint(db).as_deref() == Some(expected)
            && !std::path::PathBuf::from(format!("{}-wal", db.display())).exists()
            && !std::path::PathBuf::from(format!("{}-shm", db.display())).exists()
    });
    if !already_installed {
        copy_restore_backup(&backup, &temporary)?;
        let fingerprint = restore_file_fingerprint(&temporary).ok_or("restored database fingerprint unavailable")?;
        marker["phase"] = serde_json::json!("migration-restore-in-progress");
        marker["restoreId"] = serde_json::json!(restore_id);
        marker["restoredDatabaseFingerprint"] = serde_json::json!(fingerprint);
        write_private_json(&marker_path, &marker)?;
        move_existing_database_bundle_aside(db, &restore_id)?;
        std::fs::rename(&temporary, db).map_err(|e| format!("install restored database: {e}"))?;
    }
    let mut completed = marker.clone();
    completed["phase"] = serde_json::json!("migration-restored");
    completed["restoredAt"] = serde_json::json!(now_iso());
    write_private_json(&provenance_path, &completed)?;
    let _ = std::fs::remove_file(&temporary);
    std::fs::remove_file(&marker_path).map_err(|e| format!("remove migration recovery marker: {e}"))?;
    sync_directory(db.parent().unwrap_or(std::path::Path::new(".")));
    Ok(())
}

pub fn run_state_migrations() -> Result<Vec<u32>, crate::todo::WsErr> {
    use std::path::PathBuf;
    let dir = crate::todo::state_dir();
    std::fs::create_dir_all(&dir).ok();
    let db = PathBuf::from(dir).join("state.sqlite");
    let marker = migration_recovery_marker_path(&db);
    let conn = Connection::open(&db)
        .map_err(|e| crate::acp_runner::err(format!("could not open state.sqlite: {e}")))?;

    // Upstream permits a bounded in-place resume of the interrupted migration.
    // It deliberately does not snapshot the already-interrupted database again;
    // the first attempt's backup remains the only safe rollback point. Attempts
    // are charged before running so a crash cannot restart-loop forever.
    if marker.exists() {
        let text = std::fs::read_to_string(&marker).map_err(|e| {
            crate::acp_runner::err(format!("migration recovery marker could not be read: {e}"))
        })?;
        let mut payload: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            crate::acp_runner::err(format!("migration recovery marker could not be validated: {e}"))
        })?;
        if payload.get("databasePath").and_then(serde_json::Value::as_str) != Some(db.to_string_lossy().as_ref())
            || payload.get("backupPath").and_then(serde_json::Value::as_str).is_none()
        {
            return Err(crate::acp_runner::err("migration recovery marker is invalid"));
        }
        let attempts = payload.get("resumeAttempts").and_then(serde_json::Value::as_u64).unwrap_or(0);
        let restoring = payload.get("phase").and_then(serde_json::Value::as_str) == Some("migration-restore-in-progress");
        if restoring || attempts >= 2 {
            return Err(crate::acp_runner::err(format!(
                "migration recovery is required for {}. Stop every Synara process, restore the backup referenced by {}, then start Synara again",
                db.display(),
                marker.display()
            )));
        }
        payload["targetVersion"] = serde_json::json!(latest_migration_id());
        payload["resumeAttempts"] = serde_json::json!(attempts + 1);
        payload["lastResumeAt"] = serde_json::json!(now_iso());
        write_recovery_marker(&db, &payload)
            .map_err(|e| crate::acp_runner::err(format!("migration recovery marker update failed: {e}")))?;
        eprintln!("migrations: resuming interrupted migration (attempt {})", attempts + 1);
        let result = run_pending(&conn);
        if result.is_ok() {
            let completed: u32 = conn
                .query_row("SELECT COALESCE(MAX(migration_id), 0) FROM effect_sql_migrations", [], |row| row.get(0))
                .map_err(|e| crate::acp_runner::err(format!("resumed migration validation failed: {e}")))?;
            if completed != latest_migration_id() {
                return Err(crate::acp_runner::err("resumed migration did not reach the expected schema"));
            }
            write_completed_provenance(&db, &payload)
                .map_err(|e| crate::acp_runner::err(format!("migration backup provenance failed: {e}")))?;
            std::fs::remove_file(&marker).map_err(|e| crate::acp_runner::err(format!("migration recovery marker removal failed: {e}")))?;
            sync_directory(db.parent().unwrap_or(std::path::Path::new(".")));
        }
        return result;
    }

    let plan = inspect_migration_backup_plan(&conn)?;
    let Some(plan) = plan else {
        return run_pending(&conn);
    };
    let backup = create_migration_backup(&conn, &db, &plan)
        .map_err(|e| crate::acp_runner::err(format!("pre-migration backup failed: {e}")))?;
    eprintln!("migrations: backup at {}", backup.backup_path.display());
    let payload = recovery_payload(&db, &backup);
    write_recovery_marker(&db, &payload)
        .map_err(|e| crate::acp_runner::err(format!("migration recovery marker failed: {e}")))?;
    let result = run_pending(&conn);
    if result.is_ok() {
        write_completed_provenance(&db, &payload)
            .map_err(|e| crate::acp_runner::err(format!("migration backup provenance failed: {e}")))?;
        std::fs::remove_file(&marker).map_err(|e| crate::acp_runner::err(format!("migration recovery marker removal failed: {e}")))?;
        sync_directory(db.parent().unwrap_or(std::path::Path::new(".")));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_table_matches_effect_sqlite_dialect() {
        let conn = Connection::open_in_memory().unwrap();
        if let Err(e) = ensure_tracker(&conn) { panic!("{}", e.0); }
        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name = 'effect_sql_migrations'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(sql.contains("migration_id integer PRIMARY KEY NOT NULL"));
        assert!(sql.contains("created_at datetime NOT NULL DEFAULT current_timestamp"));
        assert!(sql.contains("name VARCHAR(255) NOT NULL"));
    }

    #[test]
    fn fresh_run_applies_everything_in_order() {
        let conn = Connection::open_in_memory().unwrap();
        let ran = run_pending(&conn).map_err(|e| e.0).expect("run_pending");
        let canonical = canonical_migrations();
        assert_eq!(ran.len(), canonical.len());
        // A second run is a no-op.
        assert!(run_pending(&conn).map_err(|e| e.0).expect("rerun").is_empty());
    }

    #[test]
    fn compressed_backup_uses_upstream_directory_and_shape() {
        let root = std::env::temp_dir().join(format!("synara-migration-backup-test-{}", crate::orchestration_engine::uuid_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = root.join("state.sqlite");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE effect_sql_migrations (migration_id integer PRIMARY KEY NOT NULL, created_at datetime NOT NULL DEFAULT current_timestamp, name VARCHAR(255) NOT NULL); INSERT INTO effect_sql_migrations (migration_id, name) VALUES (1, 'OrchestrationEvents'); CREATE TABLE user_data (id INTEGER PRIMARY KEY);").unwrap();
        let plan = MigrationBackupPlan { source_version: "v1".into(), target_version: latest_migration_id() };
        let backup = create_migration_backup(&conn, &db, &plan).unwrap();
        assert_eq!(backup.backup_path.parent(), Some(migration_backup_directory(&db).as_path()));
        let name = backup.backup_path.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("state.sqlite.pre-migration-v1-to-v"));
        assert!(name.ends_with(".sqlite.gz"));
        let bytes = std::fs::read(&backup.backup_path).unwrap();
        assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
        let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
        let mut sqlite = Vec::new();
        use std::io::Read as _;
        decoder.read_to_end(&mut sqlite).unwrap();
        assert!(sqlite.starts_with(b"SQLite format 3"));
        drop(conn);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn explicit_restore_recovers_compressed_backup_and_records_provenance() {
        let root = std::env::temp_dir().join(format!("synara-migration-restore-test-{}", crate::orchestration_engine::uuid_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = root.join("state.sqlite");
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch("CREATE TABLE effect_sql_migrations (migration_id integer PRIMARY KEY NOT NULL, created_at datetime NOT NULL DEFAULT current_timestamp, name VARCHAR(255) NOT NULL); INSERT INTO effect_sql_migrations (migration_id, name) VALUES (1, 'OrchestrationEvents'); CREATE TABLE user_data (value TEXT); INSERT INTO user_data VALUES ('before');").unwrap();
            let plan = MigrationBackupPlan { source_version: "v1".into(), target_version: latest_migration_id() };
            let backup = create_migration_backup(&conn, &db, &plan).unwrap();
            let payload = recovery_payload(&db, &backup);
            write_recovery_marker(&db, &payload).unwrap();
        }
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute("UPDATE user_data SET value = 'after'", []).unwrap();
        }
        restore_marked_migration_backup(&db).unwrap();
        let conn = Connection::open(&db).unwrap();
        let value: String = conn.query_row("SELECT value FROM user_data", [], |row| row.get(0)).unwrap();
        assert_eq!(value, "before");
        assert!(!migration_recovery_marker_path(&db).exists());
        let provenance: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(migration_backup_provenance_path(&db)).unwrap()).unwrap();
        assert_eq!(provenance["phase"], "migration-restored");
        drop(conn);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn backup_plan_recognizes_untracked_user_database_without_mutating_tracker() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE user_data (id INTEGER PRIMARY KEY)").unwrap();
        let plan = inspect_migration_backup_plan(&conn).map_err(|e| e.0).unwrap().unwrap();
        assert_eq!(plan.source_version, "untracked");
        assert!(!table_exists(&conn, "effect_sql_migrations"));
    }

    #[test]
    fn schema_newer_than_build_fails_with_update_guidance() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_tracker(&conn).map_err(|e| e.0).unwrap();
        for (id, name, _) in canonical_migrations() {
            conn.execute("INSERT INTO effect_sql_migrations (migration_id, name) VALUES (?1, ?2)", params![id, name]).unwrap();
        }
        let next = canonical_migrations().len() as u32 + 1;
        conn.execute("INSERT INTO effect_sql_migrations (migration_id, name) VALUES (?1, 'FutureMigration')", params![next]).unwrap();
        let error = pending_migrations(&conn).unwrap_err().0;
        assert!(error.contains("newer than this build supports"));
        assert!(error.contains("Update Synara"));
    }

    #[test]
    fn canonical_ids_are_contiguous() {
        let canonical = canonical_migrations();
        for (index, (id, _, _)) in canonical.iter().enumerate() {
            assert_eq!(*id as usize, index + 1, "migration ids must be contiguous");
        }
    }
}
