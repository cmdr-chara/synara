//! Schema-introspection helpers ported from upstream `migrations/schemaHelpers.ts`:
//! idempotent migrations guard on existing structure instead of relying on
//! driver-specific duplicate-column errors.

use rusqlite::Connection;

pub fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
        rusqlite::params![table, column],
        |r| r.get::<_, i64>(0),
    )
    .map(|v| v == 1)
    .unwrap_or(false)
}

/// Base tables only: a migration that rebuilds or retires a table must guard
/// on the presence of the pre-state, which a same-named view would not satisfy.
pub fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        rusqlite::params![table],
        |r| r.get::<_, i64>(0),
    )
    .map(|v| v == 1)
    .unwrap_or(false)
}

/// Primary-key columns in key order. Table rebuilds that only change the key
/// shape use this to recognize their own post-state and skip a destructive replay.
pub fn primary_key_columns(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = match conn
        .prepare("SELECT name FROM pragma_table_info(?1) WHERE pk > 0 ORDER BY pk ASC")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map(rusqlite::params![table], |r| r.get::<_, String>(0)) {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };
    rows.filter_map(|r| r.ok()).collect()
}

/// Execute a batch, swallowing duplicate-column/existing-object errors the way
/// upstream's `.pipe(Effect.catchTag("SqlError", () => Effect.void))` does for
/// idempotent ALTERs. Genuine syntax or constraint failures still propagate.
pub fn execute_lenient(conn: &Connection, sql: &str) -> Result<(), crate::todo::WsErr> {
    match conn.execute_batch(sql) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("duplicate column")
                || msg.contains("already exists")
                || msg.contains("no such column")
                || msg.contains("no such table")
            {
                Ok(())
            } else {
                Err(crate::acp_runner::err(format!("sql failed: {e}")))
            }
        }
    }
}

pub fn execute(conn: &Connection, sql: &str) -> Result<(), crate::todo::WsErr> {
    conn.execute_batch(sql)
        .map_err(|e| crate::acp_runner::err(format!("sql failed: {e}")))
}
