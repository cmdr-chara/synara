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

/// Validate tracker rows against the canonical list, repairing the private
/// account lineages upstream enumerates. Returns the validated rows.
fn validate_lineage(conn: &Connection, rows: &[(u32, String)]) -> Result<Vec<(u32, String)>, WsErr> {
    let canonical = canonical_migrations();
    let canonical_name = |id: u32| -> Option<&'static str> {
        canonical.iter().find(|(cid, _, _)| *cid == id).map(|(_, n, _)| *n)
    };
    // Strict prefix validation upstream performs before any repair: rows must
    // be contiguous ids starting at 1 and match canonical names, except for
    // the recognized private lineages.
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
        return Ok(rows.to_vec());
    }
    // Private account lineages: the tracker tail diverges from canonical
    // because private builds shipped account migrations at ids the release
    // lineage later reused. Upstream repairs by removing the divergent tail so
    // the canonical run resumes after the lineage prefix.
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
            conn.execute(
                "DELETE FROM effect_sql_migrations WHERE migration_id > ?1",
                params![prefix],
            )
            .map_err(|e| err(format!("lineage repair delete: {e}")))?;
            let remaining: Vec<(u32, String)> =
                rows.iter().take(prefix as usize).cloned().collect();
            return Ok(remaining);
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
pub fn run_pending(conn: &Connection) -> Result<Vec<u32>, WsErr> {
    ensure_tracker(conn)?;
    let rows = tracker_rows(conn)?;
    let validated = validate_lineage(conn, rows.as_slice())?;
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
pub fn run_state_migrations() -> Result<Vec<u32>, crate::todo::WsErr> {
    use rusqlite::Connection;
    use std::path::PathBuf;
    let dir = crate::todo::state_dir();
    std::fs::create_dir_all(&dir).ok();
    let db = PathBuf::from(dir).join("state.sqlite");
    let conn = Connection::open(&db)
        .map_err(|e| crate::acp_runner::err(format!("could not open state.sqlite: {e}")))?;
    run_pending(&conn)
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
    fn canonical_ids_are_contiguous() {
        let canonical = canonical_migrations();
        for (index, (id, _, _)) in canonical.iter().enumerate() {
            assert_eq!(*id as usize, index + 1, "migration ids must be contiguous");
        }
    }
}
