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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Self::initialize(connection)
    }
    pub fn memory() -> StorageResult<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }
    fn initialize(mut connection: Connection) -> StorageResult<Self> {
        connection.busy_timeout(Duration::from_secs(3))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 1 {
            return Err(StorageError::NewerSchema);
        }
        if version == 0 {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE workspaces(id TEXT PRIMARY KEY, data TEXT NOT NULL);\nCREATE TABLE projects(id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id), data TEXT NOT NULL);\nCREATE TABLE tasks(id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id), thread_id TEXT NOT NULL UNIQUE, updated_ms INTEGER NOT NULL, data TEXT NOT NULL);\nCREATE TABLE sessions(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), data TEXT NOT NULL);\nCREATE TABLE events(thread_id TEXT NOT NULL REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>0), id TEXT NOT NULL UNIQUE, timestamp_ms INTEGER NOT NULL, data TEXT NOT NULL, PRIMARY KEY(thread_id,sequence));\nCREATE INDEX task_recency ON tasks(updated_ms DESC);\nCREATE TABLE preferences(key TEXT PRIMARY KEY, data TEXT NOT NULL);\nPRAGMA user_version=1;")?;
            tx.commit()?;
        }
        Ok(Self { connection })
    }
    pub fn save_workspace(&self, workspace: &Workspace) -> StorageResult<()> {
        self.connection.execute("INSERT INTO workspaces(id,data) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![workspace.id.to_string(),encode(workspace)?])?;
        Ok(())
    }
    pub fn save_project(&self, project: &Project) -> StorageResult<()> {
        self.connection.execute("INSERT INTO projects(id,workspace_id,data) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data WHERE workspace_id=excluded.workspace_id",params![project.id.to_string(),project.workspace_id.to_string(),encode(project)?])?;
        Ok(())
    }
    pub fn save_task(&self, task: &Task) -> StorageResult<()> {
        self.connection.execute("INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET updated_ms=excluded.updated_ms,data=excluded.data WHERE project_id=excluded.project_id AND thread_id=excluded.thread_id",params![task.id.to_string(),task.project_id.to_string(),task.thread_id.to_string(),task.updated_at_ms,encode(task)?])?;
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
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(String, u64, String)> = transaction
            .query_row(
                "SELECT thread_id,sequence,data FROM events WHERE id=?1",
                [envelope.id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if let Some((thread, sequence, data)) = existing {
            if thread == envelope.thread_id.to_string()
                && sequence == envelope.sequence
                && data == encoded
            {
                return Ok(false);
            }
            return Err(StorageError::Sequence);
        }
        let (last,bytes):(u64,u64)=transaction.query_row("SELECT COALESCE(MAX(sequence),0),COALESCE(SUM(length(CAST(data AS BLOB))),0) FROM events WHERE thread_id=?1",[envelope.thread_id.to_string()],|row|Ok((row.get(0)?,row.get(1)?)))?;
        if last + 1 != envelope.sequence {
            return Err(StorageError::Sequence);
        }
        if last >= 200_000 || bytes + encoded.len() as u64 > 128 * 1024 * 1024 {
            return Err(StorageError::Limit);
        }
        transaction.execute(
            "INSERT INTO events(thread_id,sequence,id,timestamp_ms,data) VALUES(?1,?2,?3,?4,?5)",
            params![
                envelope.thread_id.to_string(),
                envelope.sequence,
                envelope.id.to_string(),
                envelope.timestamp_ms,
                encoded
            ],
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
        let rows = query.query_map(params![thread.to_string(), after, limit.min(4096)], |row| {
            Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (sequence, id, timestamp_ms, data) = row?;
            let id = uuid::Uuid::parse_str(&id).map_err(|_| StorageError::Sequence)?;
            result.push(EventEnvelope {
                id: EventId(id),
                thread_id: thread,
                sequence,
                timestamp_ms,
                event: decode(&data)?,
            });
        }
        Ok(result)
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
