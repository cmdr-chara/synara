//! Beta-side consumer of the stable app's "Copy my data to Beta" handoff.
//! Runs before migrations/opening the beta database: stable writes
//! `<betaHome>/import-requested.json`, beta stages a consistent snapshot and
//! reports `<betaHome>/import-result.json` for the stable UI.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const REQUEST_FILE: &str = "import-requested.json";
const RESULT_FILE: &str = "import-result.json";
const REMOTE_SUSPENSION_FILE: &str = "remote-suspended-after-import.json";
const REQUEST_MAX_AGE_MS: i64 = 60 * 60 * 1000;
const LIVE_COPY_ATTEMPTS: usize = 5;

const EXCLUDED_ENTRIES: &[&str] = &[
    "environment-id",
    "remote-trust",
    "remote-connections.json",
    REMOTE_SUSPENSION_FILE,
    "logs",
    "diagnostics",
    "device-boot-ownership.json",
    "server-runtime.json",
    "quit-resume.json",
    REQUEST_FILE,
    RESULT_FILE,
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    version: u32,
    requested_at: String,
    source_home_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    version: u32,
    completed_at: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportOutcome {
    pub consumed: bool,
    pub ok: bool,
    pub error: Option<String>,
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn unique_path(parent: &Path, prefix: &str) -> PathBuf {
    parent.join(format!(
        "{prefix}{}",
        crate::orchestration_engine::uuid_v4()
    ))
}

fn write_json_atomic(path: &Path, value: &impl Serialize, private: bool) -> Result<(), String> {
    let parent = path.parent().ok_or("path has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let tmp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("json"),
        crate::orchestration_engine::uuid_v4()
    ));
    let body = format!(
        "{}\n",
        serde_json::to_string(value).map_err(|error| error.to_string())?
    );
    fs::write(&tmp, body).map_err(|error| error.to_string())?;
    if private {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
        }
    }
    fs::rename(&tmp, path).map_err(|error| error.to_string())
}

fn write_import_result(beta_home: &Path, ok: bool, error: Option<String>) {
    let result = ImportResult {
        version: 1,
        completed_at: now_iso(),
        ok,
        error,
    };
    let _ = write_json_atomic(&beta_home.join(RESULT_FILE), &result, true);
}

fn read_import_request(path: &Path) -> Option<ImportRequest> {
    let request: ImportRequest = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    (request.version == 1 && !request.source_home_dir.trim().is_empty()).then_some(request)
}

fn is_state_db_entry(name: &str) -> bool {
    if name == "state.sqlite" {
        return true;
    }
    name.strip_prefix("state.sqlite")
        .is_some_and(|rest| rest.starts_with('.') || rest.starts_with('-'))
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

fn secret_identity_excluded(relative: &Path) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();
    [
        "host-identity",
        "device-identity",
        "host-secrets-sync-key",
        "remote-tls",
        "remote-trust",
    ]
    .iter()
    .any(|prefix| {
        first == *prefix
            || first
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('.') || rest.starts_with('-'))
    })
}

fn copy_filtered_entry(
    source: &Path,
    target: &Path,
    root: &Path,
    exclude_secret_identity: bool,
) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(source).map_err(|error| format!("{}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Cannot import a linked state entry: {}",
            source.display()
        ));
    }
    if exclude_secret_identity {
        if let Ok(relative) = source.strip_prefix(root) {
            if !relative.as_os_str().is_empty() && secret_identity_excluded(relative) {
                return Ok(());
            }
        }
    }
    if metadata.is_dir() {
        fs::create_dir_all(target).map_err(|error| format!("{}: {error}", target.display()))?;
        for entry in
            fs::read_dir(source).map_err(|error| format!("{}: {error}", source.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            copy_filtered_entry(
                &entry.path(),
                &target.join(entry.file_name()),
                root,
                exclude_secret_identity,
            )?;
        }
    } else if metadata.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::copy(source, target)
            .map_err(|error| format!("{} → {}: {error}", source.display(), target.display()))?;
    }
    Ok(())
}

fn copy_state_entries(
    source_state: &Path,
    staged_state: &Path,
    existing_state: &Path,
) -> Result<Vec<String>, String> {
    fs::create_dir_all(staged_state).map_err(|error| error.to_string())?;
    let mut copied = Vec::new();
    for entry in fs::read_dir(source_state)
        .map_err(|error| format!("{}: {error}", source_state.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if EXCLUDED_ENTRIES.contains(&name.as_str())
            || is_state_db_entry(&name)
            || name.ends_with(".lifecycle-lock")
        {
            continue;
        }
        let source = entry.path();
        let staged = staged_state.join(&name);
        let metadata = fs::symlink_metadata(&source)
            .map_err(|error| format!("{}: {error}", source.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "Cannot import a linked state entry: {}",
                source.display()
            ));
        }
        if metadata.is_dir() {
            let existing = existing_state.join(&name);
            if existing.exists() {
                if is_symlink(&existing) {
                    return Err(format!(
                        "Cannot replace a linked Beta state entry: {}",
                        existing.display()
                    ));
                }
                if existing.is_dir() {
                    // Overlay semantics preserve Beta-only files in shared
                    // directories before Stable files overwrite matching paths.
                    copy_filtered_entry(&existing, &staged, &existing, false)?;
                }
            }
            copy_filtered_entry(&source, &staged, &source, name == "secrets")?;
        } else if metadata.is_file() {
            fs::copy(&source, &staged)
                .map_err(|error| format!("{} → {}: {error}", source.display(), staged.display()))?;
        } else {
            continue;
        }
        copied.push(name);
    }
    Ok(copied)
}

fn live_database_signature(db: &Path) -> Result<String, String> {
    use std::io::Read as _;
    let metadata = fs::metadata(db).map_err(|error| format!("{}: {error}", db.display()))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let wal = format!("{}-wal", db.display());
    let mut wal_header = String::from("none");
    if let Ok(mut file) = fs::File::open(wal) {
        let mut header = [0u8; 32];
        if let Ok(count) = file.read(&mut header) {
            wal_header = header[..count]
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
        }
    }
    Ok(format!("{}:{modified}:{wal_header}", metadata.len()))
}

fn copy_live_database(source_db: &Path, staging_dir: &Path) -> Result<PathBuf, String> {
    let staged_db = staging_dir.join("state.sqlite");
    for _ in 0..LIVE_COPY_ATTEMPTS {
        if staging_dir.exists() {
            fs::remove_dir_all(staging_dir).map_err(|error| error.to_string())?;
        }
        fs::create_dir_all(staging_dir).map_err(|error| error.to_string())?;
        let before = live_database_signature(source_db)?;
        fs::copy(source_db, &staged_db).map_err(|error| format!("database copy: {error}"))?;
        for suffix in ["-wal", "-journal"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", source_db.display()));
            if sidecar.is_file() {
                fs::copy(&sidecar, staging_dir.join(format!("state.sqlite{suffix}")))
                    .map_err(|error| format!("database sidecar copy: {error}"))?;
            }
        }
        if live_database_signature(source_db)? == before {
            return Ok(staged_db);
        }
    }
    Err("Synara kept rewriting its database during the copy. Try again in a moment.".into())
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn vacuum_into(source_db: &Path, target: &Path) -> Result<(), String> {
    let conn =
        Connection::open(source_db).map_err(|error| format!("open database snapshot: {error}"))?;
    conn.execute_batch(&format!(
        "VACUUM INTO {}",
        sql_string_literal(&target.to_string_lossy())
    ))
    .map_err(|error| format!("vacuum database snapshot: {error}"))
}

fn inspect_snapshot_migration(db: &Path, latest_migration_id: u32) -> Result<(), String> {
    let conn = Connection::open(db).map_err(|error| format!("open imported database: {error}"))?;
    let migration_id: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(migration_id), 0) FROM effect_sql_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "Unrecognized migration lineage in Stable data.".to_string())?;
    crate::migrations::pending_migrations(&conn)
        .map_err(|_| "Unrecognized migration lineage in Stable data.".to_string())?;
    if migration_id > latest_migration_id {
        return Err(
            "Synara is newer than this Synara Beta. Update Synara Beta, then copy your data again."
                .into(),
        );
    }
    if crate::migrations::table_exists(&conn, "account_usage_sync") {
        let identity =
            crate::migrations::column_exists(&conn, "account_usage_sync", "account_identity");
        let sql = if identity {
            "UPDATE account_usage_sync SET watermark_minute = NULL, last_failure_at = NULL, account_identity = NULL"
        } else {
            "UPDATE account_usage_sync SET watermark_minute = NULL, last_failure_at = NULL"
        };
        conn.execute_batch(sql)
            .map_err(|error| format!("reset account sync state: {error}"))?;
    }
    Ok(())
}

fn snapshot_stable_database(
    source_db: &Path,
    target_db: &Path,
    latest_migration_id: u32,
) -> Result<(), String> {
    let staging = PathBuf::from(format!(
        "{}.import-{}",
        target_db.display(),
        std::process::id()
    ));
    let live_dir = PathBuf::from(format!("{}.src", staging.display()));
    let _ = fs::remove_file(&staging);
    let _ = fs::remove_dir_all(&live_dir);
    if let Some(parent) = target_db.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let result = (|| {
        if vacuum_into(source_db, &staging).is_err() {
            let _ = fs::remove_file(&staging);
            let live_copy = copy_live_database(source_db, &live_dir)?;
            vacuum_into(&live_copy, &staging)?;
        }
        inspect_snapshot_migration(&staging, latest_migration_id)?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let _ = fs::remove_file(format!("{}{suffix}", target_db.display()));
        }
        fs::rename(&staging, target_db)
            .map_err(|error| format!("commit database snapshot: {error}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(&staging);
    let _ = fs::remove_dir_all(&live_dir);
    result
}

fn commit_staged_import(
    staged_state: &Path,
    target_state: &Path,
    copied_entries: &[String],
) -> Result<(), String> {
    fs::create_dir_all(target_state).map_err(|error| error.to_string())?;
    let beta_home = target_state.parent().unwrap_or_else(|| Path::new("."));
    let backup = unique_path(beta_home, ".beta-import-backup-");
    fs::create_dir_all(&backup).map_err(|error| error.to_string())?;
    let mut entries: Vec<String> = copied_entries.to_vec();
    for suffix in ["-wal", "-shm", "-journal"] {
        entries.push(format!("state.sqlite{suffix}"));
    }
    entries.push("state.sqlite".into());
    entries.dedup();
    let mut committed: Vec<(String, bool)> = Vec::new();
    let result = (|| -> Result<(), String> {
        for name in &entries {
            let target = target_state.join(name);
            let previous = backup.join(name);
            let staged = staged_state.join(name);
            let had_previous = fs::symlink_metadata(&target).is_ok();
            if had_previous {
                fs::rename(&target, &previous)
                    .map_err(|error| format!("backup {name}: {error}"))?;
            }
            committed.push((name.clone(), had_previous));
            if fs::symlink_metadata(&staged).is_ok() {
                fs::rename(&staged, &target).map_err(|error| format!("commit {name}: {error}"))?;
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut rollback_errors = Vec::new();
        for (name, had_previous) in committed.iter().rev() {
            let target = target_state.join(name);
            if target.is_dir() {
                let _ = fs::remove_dir_all(&target);
            } else {
                let _ = fs::remove_file(&target);
            }
            if *had_previous {
                if let Err(rollback) = fs::rename(backup.join(name), &target) {
                    rollback_errors.push(format!("{name}: {rollback}"));
                }
            }
        }
        if rollback_errors.is_empty() {
            let _ = fs::remove_dir_all(&backup);
        } else {
            return Err(format!(
                "Beta import failed and rollback was incomplete. Previous data remains at {}: {}",
                backup.display(),
                rollback_errors.join("; ")
            ));
        }
        return Err(error);
    }
    let _ = fs::remove_dir_all(&backup);
    Ok(())
}

fn real_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn latest_known_migration() -> u32 {
    crate::migrations::canonical_migrations()
        .iter()
        .map(|(id, _, _)| *id)
        .max()
        .unwrap_or(0)
}

pub(crate) fn run_beta_import(
    beta_home: &Path,
    state_dir: &Path,
    latest_migration_id: u32,
    allowed_source_homes: &[PathBuf],
) -> ImportOutcome {
    let marker = beta_home.join(REQUEST_FILE);
    if !marker.exists() {
        return ImportOutcome {
            consumed: false,
            ok: true,
            error: None,
        };
    }
    let finish = |ok: bool, error: Option<String>| {
        write_import_result(beta_home, ok, error.clone());
        if marker.is_dir() {
            let _ = fs::remove_dir_all(&marker);
        } else {
            let _ = fs::remove_file(&marker);
        }
        ImportOutcome {
            consumed: true,
            ok,
            error,
        }
    };
    let Some(request) = read_import_request(&marker) else {
        return finish(false, Some("import marker was malformed".into()));
    };
    let requested_at = chrono::DateTime::parse_from_rfc3339(&request.requested_at)
        .map(|time| time.timestamp_millis())
        .unwrap_or(0);
    if now_ms() - requested_at > REQUEST_MAX_AGE_MS {
        if marker.is_dir() {
            let _ = fs::remove_dir_all(&marker);
        } else {
            let _ = fs::remove_file(&marker);
        }
        return ImportOutcome {
            consumed: true,
            ok: true,
            error: None,
        };
    }
    let source_home = absolute_path(Path::new(&request.source_home_dir));
    if real_path(&source_home) == real_path(beta_home) {
        return finish(
            false,
            Some("import source points at the beta home itself".into()),
        );
    }
    if !allowed_source_homes
        .iter()
        .map(|path| absolute_path(path))
        .any(|path| path == source_home)
    {
        return finish(
            false,
            Some("import source is not the Synara data folder".into()),
        );
    }
    let source_state = source_home.join("userdata");
    let source_db = source_state.join("state.sqlite");
    if !source_db.exists() {
        return finish(
            false,
            Some(format!(
                "stable database not found at {}",
                source_db.display()
            )),
        );
    }
    if is_symlink(&source_state) || is_symlink(&source_db) || is_symlink(state_dir) {
        return finish(false, Some("Cannot import a linked state entry".into()));
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        if is_symlink(&PathBuf::from(format!("{}{suffix}", source_db.display()))) {
            return finish(
                false,
                Some("Cannot import a linked database sidecar".into()),
            );
        }
    }

    let result = (|| -> Result<(), String> {
        let staged_root = unique_path(beta_home, ".beta-import-");
        fs::create_dir_all(&staged_root).map_err(|error| error.to_string())?;
        let result = (|| {
            let staged_state = staged_root.join("userdata");
            snapshot_stable_database(
                &source_db,
                &staged_state.join("state.sqlite"),
                latest_migration_id,
            )?;
            let mut copied = copy_state_entries(&source_state, &staged_state, state_dir)?;
            write_json_atomic(
                &staged_state.join(REMOTE_SUSPENSION_FILE),
                &json!({"version": 1, "reason": "stable-import"}),
                true,
            )?;
            copied.push(REMOTE_SUSPENSION_FILE.into());
            commit_staged_import(&staged_state, state_dir, &copied)
        })();
        let _ = fs::remove_dir_all(&staged_root);
        result
    })();
    match result {
        Ok(()) => finish(true, None),
        Err(error) => finish(false, Some(error)),
    }
}

fn beta_home_from_state_dir(state_dir: &Path) -> PathBuf {
    state_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| state_dir.to_path_buf())
}

/// Consume the beta import marker before the backend opens or migrates its
/// state database. Non-beta builds never consume it: a stray marker in any
/// other home must stay inert exactly like upstream.
pub fn run_beta_import_if_requested() -> ImportOutcome {
    if std::env::var("SYNARA_DESKTOP_FLAVOR").ok().as_deref() != Some("beta") {
        return ImportOutcome {
            consumed: false,
            ok: true,
            error: None,
        };
    }
    let state_dir = PathBuf::from(crate::todo::state_dir());
    let beta_home = beta_home_from_state_dir(&state_dir);
    let stable_home = std::env::var_os("SYNARA_STABLE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".synara")))
        .unwrap_or_else(|| PathBuf::from(".synara"));
    run_beta_import(
        &beta_home,
        &state_dir,
        latest_known_migration(),
        &[stable_home],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "synara-beta-import-test-{}",
            crate::orchestration_engine::uuid_v4()
        ));
        let stable = root.join(".synara");
        let beta = root.join(".synara-beta");
        fs::create_dir_all(stable.join("userdata")).unwrap();
        fs::create_dir_all(beta.join("userdata")).unwrap();
        (root, stable, beta)
    }

    #[test]
    fn state_db_entry_matching_covers_only_database_sidecars() {
        assert!(is_state_db_entry("state.sqlite"));
        assert!(is_state_db_entry("state.sqlite-wal"));
        assert!(is_state_db_entry("state.sqlite.import-1"));
        assert!(is_state_db_entry("state.sqlite-backup"));
        assert!(!is_state_db_entry("settings.json"));
    }

    #[test]
    fn secret_identity_prefixes_are_excluded_without_blocking_provider_secrets() {
        assert!(secret_identity_excluded(Path::new("host-identity")));
        assert!(secret_identity_excluded(Path::new("device-identity.json")));
        assert!(secret_identity_excluded(Path::new("remote-tls/key.pem")));
        assert!(!secret_identity_excluded(Path::new("anthropic/api-key")));
        assert!(!secret_identity_excluded(Path::new("other-host-identity")));
    }

    #[test]
    fn successful_import_snapshots_database_and_preserves_beta_only_directory_entries() {
        let (root, stable, beta) = fixture();
        let stable_state = stable.join("userdata");
        let beta_state = beta.join("userdata");
        fs::write(stable_state.join("settings.json"), "{\"theme\":\"dark\"}").unwrap();
        fs::create_dir_all(stable_state.join("secrets/provider")).unwrap();
        fs::write(stable_state.join("secrets/provider/key"), "stable-provider").unwrap();
        fs::write(stable_state.join("secrets/host-identity"), "stable-host").unwrap();
        fs::create_dir_all(beta_state.join("secrets/beta-only")).unwrap();
        fs::write(beta_state.join("secrets/beta-only/key"), "beta-secret").unwrap();
        fs::write(beta_state.join("settings.json"), "{\"beta\":true}").unwrap();

        let source_db = stable_state.join("state.sqlite");
        let conn = Connection::open(&source_db).unwrap();
        conn.execute_batch(
            "CREATE TABLE effect_sql_migrations (migration_id integer PRIMARY KEY NOT NULL, created_at datetime NOT NULL DEFAULT current_timestamp, name VARCHAR(255) NOT NULL);
             CREATE TABLE account_usage_sync (watermark_minute TEXT, last_failure_at TEXT, account_identity TEXT);"
        ).unwrap();
        for (id, name, _) in crate::migrations::canonical_migrations() {
            conn.execute(
                "INSERT INTO effect_sql_migrations (migration_id, name) VALUES (?1, ?2)",
                rusqlite::params![id, name],
            )
            .unwrap();
        }
        drop(conn);
        write_json_atomic(
            &beta.join(REQUEST_FILE),
            &json!({"version":1,"requestedAt":now_iso(),"sourceHomeDir":stable.to_string_lossy()}),
            true,
        )
        .unwrap();

        let outcome = run_beta_import(
            &beta,
            &beta_state,
            latest_known_migration(),
            &[stable.clone()],
        );
        assert!(outcome.consumed);
        assert!(outcome.ok, "{:?}", outcome.error);
        assert!(!beta.join(REQUEST_FILE).exists());
        assert_eq!(
            fs::read_to_string(beta_state.join("settings.json")).unwrap(),
            "{\"theme\":\"dark\"}"
        );
        assert_eq!(
            fs::read_to_string(beta_state.join("secrets/provider/key")).unwrap(),
            "stable-provider"
        );
        assert_eq!(
            fs::read_to_string(beta_state.join("secrets/beta-only/key")).unwrap(),
            "beta-secret"
        );
        assert!(!beta_state.join("secrets/host-identity").exists());
        assert!(beta_state.join(REMOTE_SUSPENSION_FILE).exists());
        let imported = Connection::open(beta_state.join("state.sqlite")).unwrap();
        let migration_count: u32 = imported
            .query_row("SELECT COUNT(*) FROM effect_sql_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            migration_count as usize,
            crate::migrations::canonical_migrations().len()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn import_copies_database_and_settings_then_removes_marker() {
        let (root, stable, beta) = fixture();
        let stable_state = stable.join("userdata");
        let beta_state = beta.join("userdata");
        fs::write(stable_state.join("settings.json"), "{\"theme\":\"dark\"}").unwrap();
        fs::write(stable_state.join("state.sqlite"), b"not-a-real-db").unwrap();
        write_json_atomic(
            &beta.join(REQUEST_FILE),
            &json!({"version":1,"requestedAt":now_iso(),"sourceHomeDir":stable.to_string_lossy()}),
            true,
        )
        .unwrap();

        // The malformed fixture database must fail before commit, consume the
        // marker, report the failure, and preserve existing beta state.
        fs::write(beta_state.join("settings.json"), "{\"beta\":true}").unwrap();
        let outcome = run_beta_import(
            &beta,
            &beta_state,
            latest_known_migration(),
            &[stable.clone()],
        );
        assert!(outcome.consumed);
        assert!(!outcome.ok);
        assert!(!beta.join(REQUEST_FILE).exists());
        assert!(beta.join(RESULT_FILE).exists());
        assert_eq!(
            fs::read_to_string(beta_state.join("settings.json")).unwrap(),
            "{\"beta\":true}"
        );
        let _ = fs::remove_dir_all(root);
    }
}
