mod chat_preferences;
mod review;
pub use review::{MAX_COMMIT_DRAFT_BYTES, ReviewPreferences, ReviewScope};
mod organization;
mod task_context;
pub use organization::{NativeSpace, OrganizationEdit, SpaceSymbol, WorkspaceOrganization};
pub use task_context::{
    ChecklistItem, MAX_CHECKLIST_ITEMS, MAX_CHECKLIST_TEXT, MAX_NOTE_BYTES, TaskContext,
};
mod conversation_tools;
pub use conversation_tools::{MessageAnchor, MessageSearch};
mod task_creation;
pub use chat_preferences::ModelFavorite;
mod recovery;
pub use recovery::*;

use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use std::{path::Path, time::Duration};
use synara_core::*;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("invalid persisted data: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("filesystem: {0}")]
    Io(#[from] std::io::Error),
    #[error("this database was created by a newer Synara version")]
    NewerSchema,
    #[error("stored event sequence or identity is inconsistent")]
    Sequence,
    #[error("persisted object ownership cannot be changed")]
    Identity,
    #[error("persisted object still owns child records")]
    NotEmpty,
    #[error("stored data exceeds the configured limit")]
    Limit,
    #[error("recovery was cancelled")]
    RecoveryCancelled,
    #[error("recovery exceeded its deadline")]
    RecoveryTimeout,
    #[error("backup is not a supported, consistent Synara database")]
    InvalidBackup,
    #[error("recovery destination already exists or is not an explicit new file")]
    RecoveryDestination,
    #[error("conversation replay: {0}")]
    Replay(#[from] ReplayError),
}
pub type StorageResult<T> = Result<T, StorageError>;
/// SQLite access is synchronous and belongs on the workspace worker, never the UI thread.
pub struct Store {
    connection: Connection,
}
impl Store {
    pub fn open(path: &Path) -> StorageResult<Self> {
        // Resolve OS aliases in the chosen directory (e.g. macOS /var -> /private/var),
        // but never canonicalize/follow the database leaf itself.
        let path = database_path(path)?;
        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        let store = Self::initialize(connection)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(store)
    }
    pub fn memory() -> StorageResult<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }
    fn initialize(mut connection: Connection) -> StorageResult<Self> {
        connection.busy_timeout(Duration::from_secs(3))?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 3 {
            return Err(StorageError::NewerSchema);
        }
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        if version == 3 {
            return Ok(Self { connection });
        }
        // Re-read the schema after acquiring the writer lock: another opener may
        // have migrated it while this connection was waiting. Keep the complete
        // upgrade atomic, including event-head and activity backfills.
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 3 {
            return Err(StorageError::NewerSchema);
        }
        if version == 0 {
            tx.execute_batch("CREATE TABLE workspaces(id TEXT PRIMARY KEY, data TEXT NOT NULL);\nCREATE TABLE projects(id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id), data TEXT NOT NULL);\nCREATE TABLE tasks(id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id), thread_id TEXT NOT NULL UNIQUE, updated_ms INTEGER NOT NULL, data TEXT NOT NULL);\nCREATE TABLE sessions(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), data TEXT NOT NULL);\nCREATE TABLE events(thread_id TEXT NOT NULL REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>0), id TEXT NOT NULL UNIQUE, timestamp_ms INTEGER NOT NULL, data TEXT NOT NULL, PRIMARY KEY(thread_id,sequence));\nCREATE INDEX task_recency ON tasks(updated_ms DESC);\nCREATE TABLE preferences(key TEXT PRIMARY KEY, data TEXT NOT NULL);\nPRAGMA user_version=1;")?;
        }
        if version < 2 {
            tx.execute_batch("CREATE TABLE event_heads(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>=0), bytes INTEGER NOT NULL CHECK(bytes>=0));
INSERT INTO event_heads SELECT thread_id,MAX(sequence),SUM(length(CAST(data AS BLOB))) FROM events GROUP BY thread_id;
PRAGMA user_version=2;")?;
        }
        if version < 3 {
            tx.execute_batch("CREATE TABLE thread_activity(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>=0), data TEXT NOT NULL);")?;
            // Backfill on the blocking storage worker. The migration and catalog repair
            // are one transaction, so a failed replay preserves the previous schema.
            let tasks: Vec<Task> = {
                let mut query = tx.prepare("SELECT data FROM tasks")?;
                let mut rows = query.query([])?;
                let mut tasks = vec![];
                while let Some(row) = rows.next()? {
                    if tasks.len() >= 10_000 {
                        return Err(StorageError::Limit);
                    }
                    tasks.push(decode(&row.get::<_, String>(0)?)?);
                }
                tasks
            };
            for task in tasks {
                let mut activity = ThreadActivity::new(task.title.clone());
                let mut sequence = 0_i64;
                let mut timestamp = task.updated_at_ms;
                {
                    let mut query = tx.prepare("SELECT sequence,timestamp_ms,data FROM events WHERE thread_id=?1 ORDER BY sequence")?;
                    let mut rows = query.query([task.thread_id.to_string()])?;
                    while let Some(row) = rows.next()? {
                        let next: i64 = row.get(0)?;
                        if next != sequence + 1 || next > 200_000 {
                            return Err(StorageError::Sequence);
                        }
                        sequence = next;
                        timestamp = timestamp.max(row.get(1)?);
                        activity.apply(&decode::<ThreadEvent>(&row.get::<_, String>(2)?)?);
                    }
                }
                update_activity(&tx, task, sequence, timestamp, &activity)?;
            }
            tx.execute_batch("PRAGMA user_version=3")?;
        }
        tx.commit()?;
        Ok(Self { connection })
    }
    pub fn create_workspace_project(
        &mut self,
        workspace: &Workspace,
        project: &Project,
    ) -> StorageResult<()> {
        if workspace.id != project.workspace_id {
            return Err(StorageError::Identity);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO workspaces(id,data) VALUES(?1,?2)",
            params![workspace.id.to_string(), encode(workspace)?],
        )?;
        tx.execute(
            "INSERT INTO projects(id,workspace_id,data) VALUES(?1,?2,?3)",
            params![
                project.id.to_string(),
                project.workspace_id.to_string(),
                encode(project)?
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn create_workspace_project_with_preference<T: serde::Serialize>(
        &mut self,
        workspace: &Workspace,
        project: &Project,
        key: &str,
        value: &T,
    ) -> StorageResult<()> {
        if workspace.id != project.workspace_id || !valid_preference_key(key) {
            return Err(StorageError::Identity);
        }
        let workspace_data = encode(workspace)?;
        let project_data = encode(project)?;
        let preference_data = encode(value)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO workspaces(id,data) VALUES(?1,?2)",
            params![workspace.id.to_string(), workspace_data],
        )?;
        tx.execute(
            "INSERT INTO projects(id,workspace_id,data) VALUES(?1,?2,?3)",
            params![
                project.id.to_string(),
                project.workspace_id.to_string(),
                project_data
            ],
        )?;
        tx.execute(
            "INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",
            params![key, preference_data],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn save_workspace(&self, workspace: &Workspace) -> StorageResult<()> {
        self.connection.execute("INSERT INTO workspaces(id,data) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![workspace.id.to_string(),encode(workspace)?])?;
        Ok(())
    }
    pub fn save_project(&self, project: &Project) -> StorageResult<()> {
        let changed = self.connection.execute("INSERT INTO projects(id,workspace_id,data) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data WHERE workspace_id=excluded.workspace_id",params![project.id.to_string(),project.workspace_id.to_string(),encode(project)?])?;
        if changed != 1 {
            return Err(StorageError::Identity);
        }
        Ok(())
    }
    pub fn save_task(&self, task: &Task) -> StorageResult<()> {
        let changed = self.connection.execute("INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET updated_ms=excluded.updated_ms,data=excluded.data WHERE project_id=excluded.project_id AND thread_id=excluded.thread_id",params![task.id.to_string(),task.project_id.to_string(),task.thread_id.to_string(),task.updated_at_ms,encode(task)?])?;
        if changed != 1 {
            return Err(StorageError::Identity);
        }
        Ok(())
    }
    pub fn workspaces(&self) -> StorageResult<Vec<Workspace>> {
        self.query_json("SELECT data FROM workspaces ORDER BY id", [])
    }
    pub fn projects(&self, workspace: WorkspaceId) -> StorageResult<Vec<Project>> {
        self.query_json(
            "SELECT data FROM projects WHERE workspace_id=?1 ORDER BY id",
            [workspace.to_string()],
        )
    }
    pub fn tasks(&self, project: ProjectId) -> StorageResult<Vec<Task>> {
        self.query_json(
            "SELECT data FROM tasks WHERE project_id=?1 ORDER BY updated_ms DESC, id ASC",
            [project.to_string()],
        )
    }
    pub fn task(&self, id: TaskId) -> StorageResult<Option<Task>> {
        let data: Option<String> = self
            .connection
            .query_row(
                "SELECT data FROM tasks WHERE id=?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        data.map(|data| decode(&data)).transpose()
    }
    pub fn delete_task(&mut self, id: TaskId) -> StorageResult<bool> {
        let Some(task) = self.task(id)? else {
            return Ok(false);
        };
        if task.state != TaskState::Archived {
            return Err(StorageError::Identity);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "DELETE FROM sessions WHERE thread_id=?1",
            [task.thread_id.to_string()],
        )?;
        tx.execute(
            "DELETE FROM events WHERE thread_id=?1",
            [task.thread_id.to_string()],
        )?;
        tx.execute(
            "DELETE FROM event_heads WHERE thread_id=?1",
            [task.thread_id.to_string()],
        )?;
        tx.execute(
            "DELETE FROM thread_activity WHERE thread_id=?1",
            [task.thread_id.to_string()],
        )?;
        tx.execute(
            "DELETE FROM preferences WHERE key IN (?1,?2,?3)",
            params![
                format!("task-draft:{id}"),
                format!("message-pins:{id}"),
                format!("task-context:{id}")
            ],
        )?;
        let changed = tx.execute("DELETE FROM tasks WHERE id=?1", [id.to_string()])?;
        tx.commit()?;
        Ok(changed == 1)
    }
    pub fn delete_project(&mut self, id: ProjectId) -> StorageResult<bool> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id=?1",
            [id.to_string()],
            |row| row.get(0),
        )?;
        if count != 0 {
            return Err(StorageError::NotEmpty);
        }
        Ok(self
            .connection
            .execute("DELETE FROM projects WHERE id=?1", [id.to_string()])?
            == 1)
    }
    pub fn delete_workspace(&mut self, id: WorkspaceId) -> StorageResult<bool> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM projects WHERE workspace_id=?1",
            [id.to_string()],
            |row| row.get(0),
        )?;
        if count != 0 {
            return Err(StorageError::NotEmpty);
        }
        Ok(self
            .connection
            .execute("DELETE FROM workspaces WHERE id=?1", [id.to_string()])?
            == 1)
    }
    pub fn save_session(&self, thread: ThreadId, session: &SessionReference) -> StorageResult<()> {
        self.connection.execute("INSERT INTO sessions(thread_id,data) VALUES(?1,?2) ON CONFLICT(thread_id) DO UPDATE SET data=excluded.data",params![thread.to_string(),encode(session)?])?;
        Ok(())
    }
    pub fn session(&self, thread: ThreadId) -> StorageResult<Option<SessionReference>> {
        let data: Option<String> = self
            .connection
            .query_row(
                "SELECT data FROM sessions WHERE thread_id=?1",
                [thread.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        data.map(|data| decode(&data)).transpose()
    }
    pub fn forget_session(&self, thread: ThreadId) -> StorageResult<()> {
        self.connection.execute(
            "DELETE FROM sessions WHERE thread_id=?1",
            [thread.to_string()],
        )?;
        Ok(())
    }
    pub fn append(&mut self, envelope: &EventEnvelope) -> StorageResult<bool> {
        let encoded = encode(&envelope.event)?;
        let sequence = i64::try_from(envelope.sequence).map_err(|_| StorageError::Sequence)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(String, i64, i64, String)> = transaction
            .query_row(
                "SELECT thread_id,sequence,timestamp_ms,data FROM events WHERE id=?1",
                [envelope.id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        if let Some((thread, stored_sequence, timestamp, data)) = existing {
            if thread == envelope.thread_id.to_string()
                && stored_sequence == sequence
                && timestamp == envelope.timestamp_ms
                && data == encoded
            {
                return Ok(false);
            }
            return Err(StorageError::Sequence);
        }
        let (last, bytes): (i64, i64) = transaction
            .query_row(
                "SELECT sequence,bytes FROM event_heads WHERE thread_id=?1",
                [envelope.thread_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .unwrap_or((0, 0));
        if last.checked_add(1) != Some(sequence) || sequence <= 0 {
            return Err(StorageError::Sequence);
        }
        let total_bytes = bytes
            .checked_add(encoded.len() as i64)
            .ok_or(StorageError::Limit)?;
        if last >= 200_000 || total_bytes > 128 * 1024 * 1024 {
            return Err(StorageError::Limit);
        }
        transaction.execute(
            "INSERT INTO events(thread_id,sequence,id,timestamp_ms,data) VALUES(?1,?2,?3,?4,?5)",
            params![
                envelope.thread_id.to_string(),
                sequence,
                envelope.id.to_string(),
                envelope.timestamp_ms,
                encoded
            ],
        )?;
        transaction.execute(
            "INSERT INTO event_heads(thread_id,sequence,bytes) VALUES(?1,?2,?3) ON CONFLICT(thread_id) DO UPDATE SET sequence=excluded.sequence,bytes=excluded.bytes",
            params![envelope.thread_id.to_string(), sequence, total_bytes],
        )?;
        let task_data: String = transaction.query_row(
            "SELECT data FROM tasks WHERE thread_id=?1",
            [envelope.thread_id.to_string()],
            |r| r.get(0),
        )?;
        let task: Task = decode(&task_data)?;
        let stored: Option<(i64, String)> = transaction
            .query_row(
                "SELECT sequence,data FROM thread_activity WHERE thread_id=?1",
                [envelope.thread_id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let mut activity = match stored {
            Some((previous, data)) if previous == last => decode::<ThreadActivity>(&data)?,
            None if last == 0 => ThreadActivity::new(task.title.clone()),
            _ => return Err(StorageError::Sequence),
        };
        activity.apply(&envelope.event);
        update_activity(
            &transaction,
            task,
            sequence,
            envelope.timestamp_ms,
            &activity,
        )?;
        transaction.commit()?;
        Ok(true)
    }
    pub fn events(
        &self,
        thread: ThreadId,
        after: u64,
        limit: usize,
    ) -> StorageResult<Vec<EventEnvelope>> {
        let mut query=self.connection.prepare("SELECT sequence,id,timestamp_ms,data FROM events WHERE thread_id=?1 AND sequence>?2 ORDER BY sequence LIMIT ?3")?;
        let after = i64::try_from(after).map_err(|_| StorageError::Sequence)?;
        let rows = query.query_map(
            params![thread.to_string(), after, limit.min(4096) as i64],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        let mut result = Vec::new();
        for row in rows {
            let (sequence, id, timestamp_ms, data) = row?;
            let id = uuid::Uuid::parse_str(&id).map_err(|_| StorageError::Sequence)?;
            result.push(EventEnvelope {
                id: EventId(id),
                thread_id: thread,
                sequence: u64::try_from(sequence).map_err(|_| StorageError::Sequence)?,
                timestamp_ms,
                event: decode(&data)?,
            });
        }
        Ok(result)
    }
    pub fn last_sequence(&self, thread: ThreadId) -> StorageResult<u64> {
        let value: i64 = self
            .connection
            .query_row(
                "SELECT sequence FROM event_heads WHERE thread_id=?1",
                [thread.to_string()],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);
        u64::try_from(value).map_err(|_| StorageError::Sequence)
    }
    pub fn replay(&self, id: ThreadId) -> StorageResult<Thread> {
        let mut thread = Thread::new(id);
        loop {
            let events = self.events(id, thread.last_sequence, 256)?;
            if events.is_empty() {
                return Ok(thread);
            }
            for envelope in events {
                thread.apply(&envelope)?;
            }
        }
    }
    /// Preferences must contain non-secret UI settings. Credentials are never stored here.
    pub fn preference<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> StorageResult<Option<T>> {
        self.preference_raw(key)?
            .map(|data| decode(&data))
            .transpose()
    }
    pub(crate) fn preference_raw(&self, key: &str) -> StorageResult<Option<String>> {
        Ok(self
            .connection
            .query_row("SELECT data FROM preferences WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }
    pub fn set_preference<T: serde::Serialize>(&self, key: &str, value: &T) -> StorageResult<()> {
        if !valid_preference_key(key) {
            return Err(StorageError::Limit);
        }
        self.connection.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",params![key,encode(value)?])?;
        Ok(())
    }
    fn query_json<T: serde::de::DeserializeOwned, P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> StorageResult<Vec<T>> {
        let mut query = self.connection.prepare(sql)?;
        let rows = query.query_map(params, |row| row.get::<_, String>(0))?;
        let mut results = Vec::new();
        for row in rows {
            if results.len() >= 10_000 {
                return Err(StorageError::Limit);
            }
            results.push(decode(&row?)?);
        }
        Ok(results)
    }
}
fn update_activity(
    tx: &rusqlite::Transaction<'_>,
    mut task: Task,
    sequence: i64,
    timestamp: i64,
    activity: &ThreadActivity,
) -> StorageResult<()> {
    if task.state != TaskState::Archived {
        task.state = activity.state;
    }
    task.title.clone_from(&activity.title);
    task.updated_at_ms = task.updated_at_ms.max(timestamp);
    tx.execute("INSERT INTO thread_activity(thread_id,sequence,data) VALUES(?1,?2,?3) ON CONFLICT(thread_id) DO UPDATE SET sequence=excluded.sequence,data=excluded.data", params![task.thread_id.to_string(), sequence, encode(activity)?])?;
    tx.execute(
        "UPDATE tasks SET data=?1,updated_ms=?2 WHERE id=?3",
        params![encode(&task)?, task.updated_at_ms, task.id.to_string()],
    )?;
    Ok(())
}
fn database_path(path: &Path) -> StorageResult<std::path::PathBuf> {
    let name = path.file_name().ok_or(StorageError::Identity)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent.canonicalize()?.join(name))
}
fn valid_preference_key(key: &str) -> bool {
    if matches!(
        key,
        "model-favorites" | "environment-layout" | "workspace-organization"
    ) {
        return true;
    }
    if let Some(id) = key
        .strip_prefix("task-draft:")
        .or_else(|| key.strip_prefix("message-pins:"))
        .or_else(|| key.strip_prefix("task-context:"))
    {
        return id.len() == 36
            && serde_json::from_value::<TaskId>(serde_json::Value::String(id.into())).is_ok();
    }
    matches!(
        key,
        "appearance" | "selection" | "window" | "agent_profiles" | "ssh_profiles" | "settings"
    )
}

fn encode<T: serde::Serialize>(value: &T) -> StorageResult<String> {
    let text = serde_json::to_string(value)?;
    if text.len() > 8 * 1024 * 1024 {
        return Err(StorageError::Limit);
    }
    Ok(text)
}
fn decode<T: serde::de::DeserializeOwned>(text: &str) -> StorageResult<T> {
    if text.len() > 8 * 1024 * 1024 {
        return Err(StorageError::Limit);
    }
    Ok(serde_json::from_str(text)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seed(store: &Store) -> Task {
        let workspace = Workspace {
            id: WorkspaceId::new(),
            name: "Workspace".into(),
            location: WorkspaceLocation::Local {
                root: "/tmp".into(),
            },
        };
        store.save_workspace(&workspace).unwrap();
        let project = Project {
            id: ProjectId::new(),
            workspace_id: workspace.id,
            name: "Project".into(),
            relative_directory: "".into(),
        };
        store.save_project(&project).unwrap();
        let task = Task {
            id: TaskId::new(),
            project_id: project.id,
            title: "Task".into(),
            state: TaskState::Ready,
            thread_id: ThreadId::new(),
            agent_id: "custom".into(),
            working_directory: "/tmp".into(),
            updated_at_ms: 0,
            scope: TaskScope::Project,
        };
        store.save_task(&task).unwrap();
        task
    }
    fn event(task: &Task, sequence: u64) -> EventEnvelope {
        EventEnvelope {
            id: EventId::new(),
            thread_id: task.thread_id,
            sequence,
            timestamp_ms: 42,
            event: ThreadEvent::TextDelta {
                message_id: None,
                role: Role::Assistant,
                text: "Hello 😀\n".into(),
            },
        }
    }
    #[test]
    fn deletion_requires_archival_and_preserves_parent_ownership_rules() {
        let mut store = Store::memory().unwrap();
        let workspace = Workspace {
            id: WorkspaceId::new(),
            name: "Workspace".into(),
            location: WorkspaceLocation::Local {
                root: std::env::temp_dir(),
            },
        };
        let project = Project {
            id: ProjectId::new(),
            workspace_id: workspace.id,
            name: "Project".into(),
            relative_directory: std::path::PathBuf::new(),
        };
        store
            .create_workspace_project(&workspace, &project)
            .unwrap();
        let mut task = Task {
            id: TaskId::new(),
            project_id: project.id,
            title: "Task".into(),
            state: TaskState::Ready,
            thread_id: ThreadId::new(),
            agent_id: "agent".into(),
            working_directory: std::env::temp_dir(),
            updated_at_ms: 1,
            scope: TaskScope::Project,
        };
        store.save_task(&task).unwrap();
        assert!(matches!(
            store.delete_task(task.id),
            Err(StorageError::Identity)
        ));
        assert!(matches!(
            store.delete_project(project.id),
            Err(StorageError::NotEmpty)
        ));
        task.state = TaskState::Archived;
        store.save_task(&task).unwrap();
        assert!(store.delete_task(task.id).unwrap());
        assert!(store.delete_project(project.id).unwrap());
        assert!(store.delete_workspace(workspace.id).unwrap());
        assert!(!store.delete_task(task.id).unwrap());
    }

    #[test]
    fn workspace_and_conversation_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let task = seed(&store);
        store.append(&event(&task, 1)).unwrap();
        let session = SessionReference {
            agent_id: "custom".into(),
            remote_id: "session-42".into(),
            working_directory: "/tmp".into(),
            title: None,
        };
        store.save_session(task.thread_id, &session).unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(store.task(task.id).unwrap().unwrap().title, "Task");
        assert_eq!(
            store.replay(task.thread_id).unwrap().messages[0].text,
            "Hello 😀\n"
        );
        assert_eq!(store.session(task.thread_id).unwrap(), Some(session));
        assert_eq!(store.last_sequence(task.thread_id).unwrap(), 1);
    }
    #[test]
    fn duplicates_are_idempotent_but_changed_envelopes_are_rejected() {
        let mut store = Store::memory().unwrap();
        let task = seed(&store);
        let mut e = event(&task, 1);
        assert!(store.append(&e).unwrap());
        assert!(!store.append(&e).unwrap());
        e.timestamp_ms += 1;
        assert!(matches!(store.append(&e), Err(StorageError::Sequence)));
        assert!(matches!(
            store.append(&event(&task, 3)),
            Err(StorageError::Sequence)
        ));
        assert!(matches!(
            store.append(&event(&task, u64::MAX)),
            Err(StorageError::Sequence)
        ));
        assert_eq!(store.last_sequence(task.thread_id).unwrap(), 1);
        assert!(store.append(&event(&task, 2)).unwrap());
    }
    #[test]
    fn object_ownership_cannot_be_silently_reassigned() {
        let store = Store::memory().unwrap();
        let mut task = seed(&store);
        task.thread_id = ThreadId::new();
        assert!(matches!(
            store.save_task(&task),
            Err(StorageError::Identity)
        ));
        assert_ne!(
            store.task(task.id).unwrap().unwrap().thread_id,
            task.thread_id
        );
    }
    #[test]
    fn newer_schema_is_rejected_without_changing_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("PRAGMA user_version=999").unwrap();
        drop(connection);
        let before = std::fs::read(&path).unwrap();
        assert!(matches!(Store::open(&path), Err(StorageError::NewerSchema)));
        assert_eq!(before, std::fs::read(&path).unwrap());
    }
    #[test]
    fn pagination_keeps_exact_event_order() {
        let mut store = Store::memory().unwrap();
        let task = seed(&store);
        for n in 1..=270 {
            store.append(&event(&task, n)).unwrap();
        }
        assert_eq!(store.events(task.thread_id, 256, 10).unwrap().len(), 10);
        assert_eq!(store.replay(task.thread_id).unwrap().last_sequence, 270);
        assert!(store.events(task.thread_id, 0, 0).unwrap().is_empty());
    }
    #[test]
    fn activity_and_task_state_survive_restart_and_overlapping_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let task = seed(&store);
        let emit = |store: &mut Store, event| {
            let mut e =
                super::tests::event(&task, store.last_sequence(task.thread_id).unwrap() + 1);
            e.event = event;
            store.append(&e).unwrap();
        };
        emit(
            &mut store,
            ThreadEvent::PromptStarted {
                turn: "turn".into(),
            },
        );
        for id in ["first", "second"] {
            emit(
                &mut store,
                ThreadEvent::PermissionRequested {
                    request: PermissionRequest {
                        id: id.into(),
                        title: "Run?".into(),
                        tool_id: None,
                        choices: vec![],
                    },
                },
            );
        }
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(
            store.task(task.id).unwrap().unwrap().state,
            TaskState::Waiting
        );
        emit(
            &mut store,
            ThreadEvent::PermissionResolved {
                id: "first".into(),
                selected: None,
            },
        );
        assert_eq!(
            store.task(task.id).unwrap().unwrap().state,
            TaskState::Waiting
        );
        emit(
            &mut store,
            ThreadEvent::PermissionResolved {
                id: "second".into(),
                selected: None,
            },
        );
        assert_eq!(
            store.task(task.id).unwrap().unwrap().state,
            TaskState::Running
        );
        emit(
            &mut store,
            ThreadEvent::PromptFinished {
                reason: "end_turn".into(),
            },
        );
        assert_eq!(
            store.task(task.id).unwrap().unwrap().state,
            TaskState::Completed
        );
    }
    #[test]
    fn schema_two_backfills_metadata_and_preserves_transcript_and_session() {
        let mut store = Store::memory().unwrap();
        let task = seed(&store);
        let mut e = event(&task, 1);
        e.event = ThreadEvent::TitleChanged {
            title: "Agent title".into(),
        };
        store.append(&e).unwrap();
        let mut e = event(&task, 2);
        e.event = ThreadEvent::PromptStarted { turn: "t".into() };
        store.append(&e).unwrap();
        // Reconstruct a v2 catalog containing the previously stale metadata.
        store.save_task(&task).unwrap();
        store
            .connection
            .execute_batch("DROP TABLE thread_activity; PRAGMA user_version=2")
            .unwrap();
        let mut store = Store::initialize(store.connection).unwrap();
        let repaired = store.task(task.id).unwrap().unwrap();
        assert_eq!(repaired.title, "Agent title");
        assert_eq!(repaired.state, TaskState::Running);
        assert_eq!(store.last_sequence(task.thread_id).unwrap(), 2);
        store.append(&event(&task, 3)).unwrap();
        assert_eq!(
            store.replay(task.thread_id).unwrap().state,
            TaskState::Running
        );
    }
    #[test]
    fn failed_schema_two_backfill_rolls_back_without_partial_catalog_repair() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad-v2.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let task = seed(&store);
        store.append(&event(&task, 1)).unwrap();
        store
            .connection
            .execute_batch(
                "DROP TABLE thread_activity; PRAGMA user_version=2; UPDATE events SET sequence=4",
            )
            .unwrap();
        drop(store);
        assert!(matches!(Store::open(&path), Err(StorageError::Sequence)));
        let db = Connection::open(path).unwrap();
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let tables: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='thread_activity'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
        let sequence: i64 = db
            .query_row("SELECT sequence FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sequence, 4);
    }
    #[test]
    fn stale_activity_rejects_append_atomically() {
        let mut store = Store::memory().unwrap();
        let task = seed(&store);
        store.append(&event(&task, 1)).unwrap();
        store
            .connection
            .execute("UPDATE thread_activity SET sequence=0", [])
            .unwrap();
        assert!(matches!(
            store.append(&event(&task, 2)),
            Err(StorageError::Sequence)
        ));
        assert_eq!(store.last_sequence(task.thread_id).unwrap(), 1);
        assert_eq!(store.events(task.thread_id, 0, 10).unwrap().len(), 1);
    }
    #[test]
    fn failed_history_restores_title_consistently_in_catalog_and_transcript() {
        let mut store = Store::memory().unwrap();
        let task = seed(&store);
        for (n, content) in [
            ThreadEvent::TitleChanged {
                title: "Original".into(),
            },
            ThreadEvent::HistoryStarted,
            ThreadEvent::TitleChanged {
                title: "Partial replay".into(),
            },
            ThreadEvent::Error {
                message: "Replay failed".into(),
                recoverable: false,
            },
        ]
        .into_iter()
        .enumerate()
        {
            let mut e = event(&task, n as u64 + 1);
            e.event = content;
            store.append(&e).unwrap();
        }
        let task = store.task(task.id).unwrap().unwrap();
        let thread = store.replay(task.thread_id).unwrap();
        assert_eq!(task.title, "Original");
        assert_eq!(task.title, thread.title);
        assert_eq!(task.state, thread.state);
        assert_eq!(task.state, TaskState::Failed);
    }
    #[test]
    fn schema_one_migrates_existing_event_heads() {
        let mut store = Store::memory().unwrap();
        let task = seed(&store);
        store.append(&event(&task, 1)).unwrap();
        store
            .connection
            .execute_batch(
                "DROP TABLE thread_activity; DROP TABLE event_heads; PRAGMA user_version=1",
            )
            .unwrap();
        let mut store = Store::initialize(store.connection).unwrap();
        assert_eq!(store.last_sequence(task.thread_id).unwrap(), 1);
        store.append(&event(&task, 2)).unwrap();
        assert_eq!(store.replay(task.thread_id).unwrap().last_sequence, 2);
    }

    #[test]
    fn task_recency_ties_have_stable_order_across_updates_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("task-order.sqlite3");
        let store = Store::open(&path).unwrap();
        let older = seed(&store);
        let mut first = Task {
            id: TaskId::new(),
            thread_id: ThreadId::new(),
            updated_at_ms: 100,
            ..older.clone()
        };
        let mut second = Task {
            id: TaskId::new(),
            thread_id: ThreadId::new(),
            ..first.clone()
        };
        if first.id.to_string() > second.id.to_string() {
            std::mem::swap(&mut first, &mut second);
        }
        // Deliberately insert equal-recency tasks in reverse identity order.
        store.save_task(&second).unwrap();
        store.save_task(&first).unwrap();
        let expected = vec![first.id, second.id, older.id];
        let task_ids = |store: &Store| {
            store
                .tasks(older.project_id)
                .unwrap()
                .into_iter()
                .map(|task| task.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(task_ids(&store), expected);
        second.title = "Renamed without changing recency".into();
        store.save_task(&second).unwrap();
        assert_eq!(task_ids(&store), expected);
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(task_ids(&store), expected);
        assert_eq!(store.task(second.id).unwrap().unwrap().title, second.title);
    }
    #[test]
    fn failed_schema_one_backfill_rolls_back_the_entire_upgrade() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad-v1.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let task = seed(&store);
        store.append(&event(&task, 1)).unwrap();
        let before = encode(&store.task(task.id).unwrap().unwrap()).unwrap();
        store
            .connection
            .execute_batch(
                "DROP TABLE thread_activity; DROP TABLE event_heads;
                 PRAGMA user_version=1; UPDATE events SET sequence=4",
            )
            .unwrap();
        drop(store);
        assert!(matches!(Store::open(&path), Err(StorageError::Sequence)));
        let db = Connection::open(&path).unwrap();
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
        let new_tables: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE name IN ('event_heads', 'thread_activity')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(new_tables, 0);
        let sequence: i64 = db
            .query_row("SELECT sequence FROM events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(sequence, 4);
        let after: String = db
            .query_row("SELECT data FROM tasks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(after, before);
    }
    #[test]
    fn failed_fresh_schema_creation_preserves_existing_tables() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.sqlite3");
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE event_heads(marker TEXT NOT NULL);
             INSERT INTO event_heads VALUES('preserve me')",
        )
        .unwrap();
        drop(db);
        assert!(Store::open(&path).is_err());
        let db = Connection::open(&path).unwrap();
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 0);
        let tables: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 1);
        let marker: String = db
            .query_row("SELECT marker FROM event_heads", [], |row| row.get(0))
            .unwrap();
        assert_eq!(marker, "preserve me");
    }
    #[test]
    fn concurrent_openers_preserve_one_migrated_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("concurrent-v1.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let task = seed(&store);
        store.append(&event(&task, 1)).unwrap();
        store
            .connection
            .execute_batch(
                "DROP TABLE thread_activity; DROP TABLE event_heads; PRAGMA user_version=1",
            )
            .unwrap();
        drop(store);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                let thread = task.thread_id;
                std::thread::spawn(move || {
                    barrier.wait();
                    let store = Store::open(&path)?;
                    assert_eq!(store.last_sequence(thread)?, 1);
                    assert_eq!(store.replay(thread)?.last_sequence, 1);
                    StorageResult::Ok(())
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
        let mut store = Store::open(&path).unwrap();
        store.append(&event(&task, 2)).unwrap();
        assert_eq!(store.replay(task.thread_id).unwrap().last_sequence, 2);
    }
}
