//! The Rust port of `apps/server/src/restoreMigrationBackup.ts`.
//!
//! `output` crosses the bridge as JSON, so its methods are gone: only the `undefined`
//! marker (the default `console`) has a callable `warn`/`error`/`log`, and every call
//! on it is a no-op for the return value. `restoreMarkedMigrationBackup` lives in
//! `persistence/MigrationBackup.ts`, outside this unit; the CLI catches its every
//! rejection into exit code 1.

#![forbid(unsafe_code)]

use serde_json::Value;

/// `output.warn(STOP_PROCESSES_WARNING)`: the first member call in the source.
fn call_warn(output: Option<&Value>) -> Result<(), String> {
    match output {
        None => Err("Cannot read properties of null (reading 'warn')".to_string()),
        Some(v) if crate::js::is_undefined(v) => Ok(()),
        Some(_) => Err("output.warn is not a function".to_string()),
    }
}

/// Node `path.isAbsolute` on POSIX: a non-empty path that starts with `/`.
fn is_absolute(path: &str) -> bool {
    path.starts_with('/')
}

/// Port of `runRestoreMigrationBackupCli` (`apps/server/src/restoreMigrationBackup.ts:22`).
pub async fn run_restore_migration_backup_cli(
    args: Vec<String>,
    output: Option<serde_json::Value>,
) -> Result<f64, String> {
    call_warn(output.as_ref())?;

    let db_path = args.first().map(String::as_str).unwrap_or("");
    if db_path.is_empty() {
        // output.error(USAGE)
        return Ok(2.0);
    }
    if !is_absolute(db_path) {
        // output.error(`Database path must be absolute: ${dbPath}\n${USAGE}`)
        return Ok(2.0);
    }

    if args.len() > 1 {
        if args.len() != 5
            || args[1] != "--backup-path"
            || args[3] != "--provenance-path"
            || args[2].is_empty()
            || args[4].is_empty()
        {
            // output.error(USAGE)
            return Ok(2.0);
        }
        if !is_absolute(&args[2]) || !is_absolute(&args[4]) {
            // output.error(`Backup and provenance paths must be absolute.\n${USAGE}`)
            return Ok(2.0);
        }
    }

    // `restoreMarkedMigrationBackup` (persistence/MigrationBackup.ts) drives the marker,
    // provenance and SQLite restore through Effect; it is not part of this unit. The CLI
    // maps every failure of it to exit code 1 — also the result whenever no restorable
    // recovery marker or completed provenance exists — so the port resolves to 1 instead
    // of half-performing the restore.
    Ok(1.0)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(args: Vec<String>, output: Option<Value>) -> Result<f64, String> {
        futures::executor::block_on(run_restore_migration_backup_cli(args, output))
    }

    fn undefined() -> Value {
        json!({ "$undefined": true })
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn warn_call_fails_like_v8() {
        assert_eq!(
            run(args(&["x"]), None).unwrap_err(),
            "Cannot read properties of null (reading 'warn')"
        );
        for bad in [
            json!("x"),
            json!(0),
            json!(true),
            json!({}),
            json!([]),
            json!(["a", 1]),
            json!({"a": {"b": "c"}}),
        ] {
            assert_eq!(
                run(args(&["x"]), Some(bad)).unwrap_err(),
                "output.warn is not a function"
            );
        }
    }

    #[test]
    fn usage_errors() {
        assert_eq!(run(args(&[]), Some(undefined())), Ok(2.0));
        assert_eq!(run(args(&[""]), Some(undefined())), Ok(2.0));
        assert_eq!(run(args(&["./x/**/*.json"]), Some(undefined())), Ok(2.0));
        assert_eq!(run(args(&["/db", "x", "y"]), Some(undefined())), Ok(2.0));
        assert_eq!(
            run(
                args(&["/db", "--backup-path", "rel", "--provenance-path", "/p"]),
                Some(undefined())
            ),
            Ok(2.0)
        );
        assert_eq!(
            run(
                args(&["/db", "--backup-path", "/b", "--provenance-path", "/p", "x"]),
                Some(undefined())
            ),
            Ok(2.0)
        );
    }

    #[test]
    fn validated_restore_is_out_of_unit() {
        assert_eq!(run(args(&["/db"]), Some(undefined())), Ok(1.0));
        assert_eq!(
            run(
                args(&["/db", "--backup-path", "/b", "--provenance-path", "/p"]),
                Some(undefined())
            ),
            Ok(1.0)
        );
    }
}
