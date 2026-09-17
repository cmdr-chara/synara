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
    #[error("stored data exceeds the configured limit")]
    Limit,
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
        let connection = Connection::open_with_flags(
            path,
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
        if version > 2 {
            return Err(StorageError::NewerSchema);
        }
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        if version == 0 {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE workspaces(id TEXT PRIMARY KEY, data TEXT NOT NULL);\nCREATE TABLE projects(id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id), data TEXT NOT NULL);\nCREATE TABLE tasks(id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id), thread_id TEXT NOT NULL UNIQUE, updated_ms INTEGER NOT NULL, data TEXT NOT NULL);\nCREATE TABLE sessions(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), data TEXT NOT NULL);\nCREATE TABLE events(thread_id TEXT NOT NULL REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>0), id TEXT NOT NULL UNIQUE, timestamp_ms INTEGER NOT NULL, data TEXT NOT NULL, PRIMARY KEY(thread_id,sequence));\nCREATE INDEX task_recency ON tasks(updated_ms DESC);\nCREATE TABLE preferences(key TEXT PRIMARY KEY, data TEXT NOT NULL);\nPRAGMA user_version=1;")?;
            tx.commit()?;
        }
        if version < 2 {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE event_heads(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>=0), bytes INTEGER NOT NULL CHECK(bytes>=0));
INSERT INTO event_heads SELECT thread_id,MAX(sequence),SUM(length(CAST(data AS BLOB))) FROM events GROUP BY thread_id;
PRAGMA user_version=2;")?;
            tx.commit()?;
        }
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
            "SELECT data FROM tasks WHERE project_id=?1 ORDER BY updated_ms DESC",
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
        let data: Option<String> = self
            .connection
            .query_row("SELECT data FROM preferences WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()?;
        data.map(|data| decode(&data)).transpose()
    }
    pub fn set_preference<T: serde::Serialize>(&self, key: &str, value: &T) -> StorageResult<()> {
        if !matches!(
            key,
            "appearance" | "selection" | "window" | "agent_profiles"
        ) {
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
    fn schema_one_migrates_existing_event_heads() {
        let mut store = Store::memory().unwrap();
        let task = seed(&store);
        store.append(&event(&task, 1)).unwrap();
        store
            .connection
            .execute_batch("DROP TABLE event_heads; PRAGMA user_version=1")
            .unwrap();
        let mut store = Store::initialize(store.connection).unwrap();
        assert_eq!(store.last_sequence(task.thread_id).unwrap(), 1);
        store.append(&event(&task, 2)).unwrap();
        assert_eq!(store.replay(task.thread_id).unwrap().last_sequence, 2);
    }
}
