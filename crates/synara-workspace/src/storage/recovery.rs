//! User-directed, no-clobber SQLite recovery. These operations never launch a process.
use super::*;
use rusqlite::{
    backup::{Backup, StepResult},
    limits::Limit,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf, time::Instant};
use tokio_util::sync::CancellationToken;

/// Resource policy for an explicit backup/restore operation, not an automatic startup action.
#[derive(Clone, Debug)]
pub struct RecoveryOptions {
    pub cancellation: CancellationToken,
    pub timeout: Duration,
    pub max_bytes: u64,
}
impl Default for RecoveryOptions {
    fn default() -> Self {
        Self {
            cancellation: CancellationToken::new(),
            timeout: Duration::from_secs(60),
            max_bytes: 1024 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RecoveryReceipt {
    pub path: PathBuf,
    pub bytes: u64,
    /// Integrity fingerprint only. It is not a signature or proof of a trusted origin.
    pub sha256: String,
    pub schema_version: u32,
}
struct Budget<'a> {
    options: &'a RecoveryOptions,
    deadline: Instant,
}
impl<'a> Budget<'a> {
    fn new(options: &'a RecoveryOptions) -> StorageResult<Self> {
        if options.max_bytes < 4096
            || options.max_bytes > 1024 * 1024 * 1024
            || options.timeout > Duration::from_secs(300)
        {
            return Err(StorageError::Limit);
        }
        let budget = Self {
            options,
            deadline: Instant::now()
                .checked_add(options.timeout)
                .ok_or(StorageError::Limit)?,
        };
        budget.check()?;
        Ok(budget)
    }
    fn check(&self) -> StorageResult<()> {
        if self.options.cancellation.is_cancelled() {
            return Err(StorageError::RecoveryCancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(StorageError::RecoveryTimeout);
        }
        Ok(())
    }
    fn guard(&self, connection: &Connection) -> StorageResult<()> {
        connection.busy_timeout(Duration::from_millis(20))?;
        connection.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON;")?;
        connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 8 * 1024 * 1024 + 4096)?;
        connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 1024 * 1024)?;
        connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)?;
        connection.set_limit(Limit::SQLITE_LIMIT_COLUMN, 64)?;
        connection.set_limit(Limit::SQLITE_LIMIT_EXPR_DEPTH, 128)?;
        let cancel = self.options.cancellation.clone();
        let deadline = self.deadline;
        connection.progress_handler(
            10_000,
            Some(move || cancel.is_cancelled() || Instant::now() >= deadline),
        )?;
        Ok(())
    }
    fn result<T>(&self, result: StorageResult<T>) -> StorageResult<T> {
        // Turn an interrupted SQLite query into the actual user-facing cancellation/deadline.
        if result.is_err() {
            self.check()?;
        }
        result
    }
}

impl Store {
    /// Make a self-contained snapshot, including committed WAL data. The destination must
    /// not exist. The source connection and catalog remain live and are never replaced.
    pub fn backup_to(
        &self,
        destination: &Path,
        options: &RecoveryOptions,
    ) -> StorageResult<RecoveryReceipt> {
        let budget = Budget::new(options)?;
        budget.result(publish_snapshot(
            &self.connection,
            destination,
            &budget,
            false,
        ))
    }

    /// Restore into a NEW path only. Never overwrite an open/corrupt/original database.
    /// Older supported schemas are upgraded in staging. The input remains read-only.
    /// Returning this receipt does not open a workspace, restore a session, or execute prompts.
    pub fn restore_to(
        backup: &Path,
        destination: &Path,
        options: &RecoveryOptions,
    ) -> StorageResult<RecoveryReceipt> {
        let budget = Budget::new(options)?;
        let backup = database_path(backup)?;
        let metadata = std::fs::symlink_metadata(&backup)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > options.max_bytes
        {
            return Err(StorageError::InvalidBackup);
        }
        let source = Connection::open_with_flags(
            &backup,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        budget.guard(&source)?;
        // Validate the copied snapshot, not a pre-copy schema that another process could change.
        budget.result(publish_snapshot(&source, destination, &budget, true))
    }
}

fn destination_path(destination: &Path) -> StorageResult<PathBuf> {
    if !destination.is_absolute() || destination.file_name().is_none() {
        return Err(StorageError::RecoveryDestination);
    }
    match std::fs::symlink_metadata(destination) {
        Ok(_) => return Err(StorageError::RecoveryDestination),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = destination
        .parent()
        .ok_or(StorageError::RecoveryDestination)?
        .canonicalize()?;
    // Canonicalize the chosen parent once. Ancestor symlinks are resolved deliberately rather
    // than interpreting a later textual replacement as a new recovery destination.
    Ok(parent.join(
        destination
            .file_name()
            .ok_or(StorageError::RecoveryDestination)?,
    ))
}

fn publish_snapshot(
    source: &Connection,
    destination: &Path,
    budget: &Budget<'_>,
    migrate: bool,
) -> StorageResult<RecoveryReceipt> {
    budget.check()?;
    let destination = destination_path(destination)?;
    let parent = destination
        .parent()
        .ok_or(StorageError::RecoveryDestination)?;
    // Private directory also owns any SQLite sidecars on all failure paths.
    let staging = tempfile::Builder::new()
        .prefix(".synara-recovery-")
        .tempdir_in(parent)?;
    let staged_path = staging.path().join("snapshot.sqlite3");
    let mut target = Connection::open(&staged_path)?;
    budget.guard(&target)?;
    let page_size =
        u64::try_from(source.query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0))?)
            .map_err(|_| StorageError::InvalidBackup)?;
    if !(512..=65_536).contains(&page_size) {
        return Err(StorageError::InvalidBackup);
    }
    let pages = u64::try_from(source.query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))?)
        .map_err(|_| StorageError::InvalidBackup)?;
    if pages
        .checked_mul(page_size)
        .is_none_or(|bytes| bytes > budget.options.max_bytes)
    {
        return Err(StorageError::Limit);
    }
    {
        let copy = Backup::new(source, &mut target)?;
        loop {
            budget.check()?;
            let state = copy.step(64)?;
            let progress = copy.progress();
            if u64::try_from(progress.pagecount)
                .ok()
                .and_then(|pages| pages.checked_mul(page_size))
                .is_none_or(|bytes| bytes > budget.options.max_bytes)
            {
                return Err(StorageError::Limit);
            }
            match state {
                StepResult::Done => break,
                StepResult::Busy | StepResult::Locked => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                StepResult::More => {}
                _ => return Err(StorageError::InvalidBackup),
            }
        }
    }
    let version = validate_schema(&target, budget)?;
    if version < 3 {
        if !migrate {
            return Err(StorageError::InvalidBackup);
        }
        target = Store::initialize(target)?.connection;
        budget.guard(&target)?;
    }
    validate_data(&target, budget)?;
    // Journal mode DELETE ensures the published file needs no WAL/SHM sidecars.
    target.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")?;
    target
        .close()
        .map_err(|(_, error)| StorageError::Database(error))?;
    budget.check()?;
    let mut input = File::open(&staged_path)?;
    let bytes = input.metadata()?.len();
    if bytes > budget.options.max_bytes {
        return Err(StorageError::Limit);
    }
    // Publish via an atomic no-clobber persist. The staging directory is private, and
    // intermediate data never appears under the requested destination filename.
    let mut published = tempfile::NamedTempFile::new_in(parent)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut copied = 0_u64;
    loop {
        budget.check()?;
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        copied = copied.checked_add(read as u64).ok_or(StorageError::Limit)?;
        if copied > budget.options.max_bytes {
            return Err(StorageError::Limit);
        }
        digest.update(&buffer[..read]);
        std::io::Write::write_all(&mut published, &buffer[..read])?;
    }
    if copied != bytes {
        return Err(StorageError::InvalidBackup);
    }
    published.as_file().sync_all()?;
    budget.check()?;
    published.persist_noclobber(&destination).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            StorageError::RecoveryDestination
        } else {
            StorageError::Io(error.error)
        }
    })?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(RecoveryReceipt {
        path: destination,
        bytes,
        sha256: hex::encode(digest.finalize()),
        schema_version: 3,
    })
}

fn schema_map(
    connection: &Connection,
) -> StorageResult<BTreeMap<String, (String, Option<String>)>> {
    let mut statement =
        connection.prepare("SELECT name,type,sql FROM sqlite_schema ORDER BY name")?;
    let mut rows = statement.query([])?;
    let mut result = BTreeMap::new();
    while let Some(row) = rows.next()? {
        if result.len() >= 64 {
            return Err(StorageError::InvalidBackup);
        }
        let sql: Option<String> = row.get(2)?;
        // Whitespace formatting is immaterial, but do not accept different SQL semantics.
        let sql = sql.map(|sql| sql.split_whitespace().collect::<Vec<_>>().join(" "));
        result.insert(row.get(0)?, (row.get(1)?, sql));
    }
    Ok(result)
}

fn validate_schema(connection: &Connection, budget: &Budget<'_>) -> StorageResult<i64> {
    budget.check()?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > 3 {
        return Err(StorageError::NewerSchema);
    }
    if !(1..=3).contains(&version) {
        return Err(StorageError::InvalidBackup);
    }
    let reference = Store::memory()?;
    let mut expected = schema_map(&reference.connection)?;
    if version < 3 {
        expected.retain(|name, _| {
            name != "thread_activity" && name != "sqlite_autoindex_thread_activity_1"
        });
    }
    if version < 2 {
        expected
            .retain(|name, _| name != "event_heads" && name != "sqlite_autoindex_event_heads_1");
    }
    if schema_map(connection)? != expected {
        return Err(StorageError::InvalidBackup);
    }
    let check: String = connection.query_row("PRAGMA quick_check(1)", [], |r| r.get(0))?;
    if check != "ok" {
        return Err(StorageError::InvalidBackup);
    }
    let mut foreign_keys = connection.prepare("PRAGMA foreign_key_check")?;
    if foreign_keys.query([])?.next()?.is_some() {
        return Err(StorageError::Identity);
    }
    Ok(version)
}

fn validate_data(connection: &Connection, budget: &Budget<'_>) -> StorageResult<()> {
    validate_schema(connection, budget)?;
    for (table, sql) in [
        ("workspaces", "SELECT id,'','',0,data FROM workspaces"),
        ("projects", "SELECT id,workspace_id,'',0,data FROM projects"),
        (
            "tasks",
            "SELECT id,project_id,thread_id,updated_ms,data FROM tasks",
        ),
        ("sessions", "SELECT thread_id,'','',0,data FROM sessions"),
        ("preferences", "SELECT key,'','',0,data FROM preferences"),
    ] {
        let mut query = connection.prepare(sql)?;
        let mut rows = query.query([])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            budget.check()?;
            count += 1;
            if count > 10_000 {
                return Err(StorageError::Limit);
            }
            let id: String = row.get(0)?;
            let parent: String = row.get(1)?;
            let thread: String = row.get(2)?;
            let updated: i64 = row.get(3)?;
            let data: String = row.get(4)?;
            let valid = match table {
                "workspaces" => decode::<Workspace>(&data)?.id.to_string() == id,
                "projects" => {
                    let item = decode::<Project>(&data)?;
                    item.id.to_string() == id && item.workspace_id.to_string() == parent
                }
                "tasks" => {
                    let item = decode::<Task>(&data)?;
                    item.id.to_string() == id
                        && item.project_id.to_string() == parent
                        && item.thread_id.to_string() == thread
                        && item.updated_at_ms == updated
                }
                "sessions" => {
                    let _ = decode::<SessionReference>(&data)?;
                    true
                }
                "preferences" => {
                    let _ = decode::<serde_json::Value>(&data)?;
                    valid_preference_key(&id)
                }
                _ => false,
            };
            if !valid {
                return Err(StorageError::Identity);
            }
        }
    }
    let mut tasks = connection.prepare("SELECT thread_id,data FROM tasks ORDER BY thread_id")?;
    let mut rows = tasks.query([])?;
    while let Some(row) = rows.next()? {
        budget.check()?;
        let thread: String = row.get(0)?;
        let task: Task = decode(&row.get::<_, String>(1)?)?;
        let mut sequence = 0_i64;
        let mut bytes = 0_i64;
        let mut events = connection
            .prepare("SELECT sequence,id,data FROM events WHERE thread_id=?1 ORDER BY sequence")?;
        let mut events = events.query([&thread])?;
        // Stream validation rather than hydrating every transcript into RAM.
        while let Some(event) = events.next()? {
            budget.check()?;
            sequence += 1;
            let id: String = event.get(1)?;
            let data: String = event.get(2)?;
            bytes += data.len() as i64;
            if event.get::<_, i64>(0)? != sequence || uuid::Uuid::parse_str(&id).is_err() {
                return Err(StorageError::Sequence);
            }
            if sequence > 200_000 || bytes > 128 * 1024 * 1024 {
                return Err(StorageError::Limit);
            }
            let _ = decode::<ThreadEvent>(&data)?;
        }
        let head: Option<(i64, i64)> = connection
            .query_row(
                "SELECT sequence,bytes FROM event_heads WHERE thread_id=?1",
                [&thread],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if head.unwrap_or((0, 0)) != (sequence, bytes) {
            return Err(StorageError::Sequence);
        }
        let activity: Option<(i64, String)> = connection
            .query_row(
                "SELECT sequence,data FROM thread_activity WHERE thread_id=?1",
                [&thread],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match activity {
            Some((last, data)) => {
                let activity = decode::<ThreadActivity>(&data)?;
                if last != sequence
                    || task.title != activity.title
                    || (task.state != TaskState::Archived && task.state != activity.state)
                {
                    return Err(StorageError::Sequence);
                }
            }
            None if sequence == 0 => {}
            None => return Err(StorageError::Sequence),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
