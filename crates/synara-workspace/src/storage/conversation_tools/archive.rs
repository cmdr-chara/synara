//! Portable conversation snapshots, with explicit projections rather than
//! serializing runtime/session configuration or pending task state.
use super::*;
use std::{
    collections::{BTreeMap, HashSet},
    io::{self, Cursor},
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const MAX_JSON_BYTES: usize = 24 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: usize = 36 * 1024 * 1024;
const EXCLUDED: &[&str] = &[
    "unsent drafts",
    "authentication and session configuration",
    "tool payloads",
    "workspace files",
    "embedded image bytes",
    "unrecorded attachment references",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Archive<'a> {
    format: &'static str,
    task_id: TaskId,
    thread_id: ThreadId,
    title: &'a str,
    agent_id: &'a str,
    task_scope: TaskScope,
    updated_at_ms: i64,
    snapshot_sequence: u64,
    state: TaskState,
    current_model: Option<&'a str>,
    current_mode: Option<&'a str>,
    excluded: &'static [&'static str],
    turns: Vec<ArchiveTurn>,
    messages: Vec<ArchiveMessage<'a>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveTurn {
    started_at_ms: i64,
    finished_at_ms: Option<i64>,
    failed: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveMessage<'a> {
    id: &'a str,
    role: Role,
    text: &'a str,
    created_at_ms: Option<i64>,
    updated_at_ms: Option<i64>,
    images: Vec<ImageMetadata<'a>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImageMetadata<'a> {
    id: EventId,
    source: ImageSource,
    mime_type: &'a str,
    embedded: bool,
}

/// Limit JSON serialization before it reaches the compressor. Escaping can
/// expand text, so the existing Markdown limit is not a sufficient JSON budget.
struct BoundedWriter<W> {
    inner: W,
    remaining: usize,
}
impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::Error::other(
                "Structured conversation exceeds the 24 MiB export limit.",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
fn zip_error(error: zip::result::ZipError) -> WorkspaceError {
    WorkspaceError::Invalid(format!(
        "Could not create the conversation archive: {error}"
    ))
}

type MessageKey = (String, u8);

#[derive(Clone, Default)]
struct MessageTimeProjection {
    created_at_ms: BTreeMap<MessageKey, i64>,
    updated_at_ms: BTreeMap<MessageKey, i64>,
    tools: HashSet<String>,
    permissions: HashSet<String>,
    inputs: HashSet<String>,
    timeline_tail: Option<MessageKey>,
}

impl MessageTimeProjection {
    fn apply(&mut self, event: ThreadEvent, event_id: &str, timestamp_ms: i64) {
        match event {
            ThreadEvent::TextDelta {
                message_id, role, ..
            }
            | ThreadEvent::ImageMessage {
                message_id, role, ..
            } => {
                let role = role_key(role);
                let key = match message_id {
                    Some(id) => {
                        let key = (id, role);
                        // The reducer only moves the timeline tail when it creates
                        // a message. Existing IDs can receive later deltas after
                        // another timeline item was added.
                        if !self.updated_at_ms.contains_key(&key) {
                            self.timeline_tail = Some(key.clone());
                        }
                        key
                    }
                    None => match self.timeline_tail.as_ref() {
                        Some((id, tail_role)) if *tail_role == role => (id.clone(), role),
                        _ => {
                            let key = (format!("event-{event_id}"), role);
                            self.timeline_tail = Some(key.clone());
                            key
                        }
                    },
                };
                self.created_at_ms
                    .entry(key.clone())
                    .or_insert(timestamp_ms);
                self.updated_at_ms.insert(key, timestamp_ms);
            }
            ThreadEvent::ToolChanged { patch } => {
                if self.tools.insert(patch.id) {
                    self.timeline_tail = None;
                }
            }
            ThreadEvent::PermissionRequested { request } => {
                if self.permissions.insert(request.id) {
                    self.timeline_tail = None;
                }
            }
            ThreadEvent::PermissionResolved { id, .. } => {
                self.permissions.remove(&id);
            }
            ThreadEvent::UserInputRequested { request } => {
                if self.inputs.insert(request.id) {
                    self.timeline_tail = None;
                }
            }
            ThreadEvent::UserInputResolved { id } => {
                self.inputs.remove(&id);
            }
            ThreadEvent::Notice { .. }
            | ThreadEvent::SessionStatus { .. }
            | ThreadEvent::ContextCompaction { .. }
            | ThreadEvent::Error { .. } => self.timeline_tail = None,
            _ => {}
        }
    }
}

fn role_key(role: Role) -> u8 {
    match role {
        Role::User => 0,
        Role::Assistant => 1,
        Role::Reasoning => 2,
    }
}

/// Derive message creation and update times from the same durable event
/// snapshot as the transcript. IDs and roles are replayed with the reducer's
/// message/timeline rules so timestamps are never guessed from neighboring
/// messages or conflated when a provider reuses an ID across roles.
fn read_message_times(
    connection: &Connection,
    thread_id: ThreadId,
) -> WorkspaceResult<MessageTimeProjection> {
    message_times_with_limits(connection, thread_id, 200_000, MAX_REPLAY_BYTES)
}

fn message_times_with_limits(
    connection: &Connection,
    thread_id: ThreadId,
    max_events: usize,
    max_bytes: usize,
) -> WorkspaceResult<MessageTimeProjection> {
    let mut projection = MessageTimeProjection::default();
    let mut history_backup = None;
    let mut total_bytes = 0usize;
    let mut event_count = 0usize;
    let mut query = connection
        .prepare("SELECT id,timestamp_ms,data FROM events WHERE thread_id=?1 ORDER BY sequence")
        .map_err(StorageError::from)?;
    let mut rows = query
        .query([thread_id.to_string()])
        .map_err(StorageError::from)?;
    while let Some(row) = rows.next().map_err(StorageError::from)? {
        event_count += 1;
        if event_count > max_events {
            return Err(StorageError::Limit.into());
        }
        let event_id: String = row.get(0).map_err(StorageError::from)?;
        let timestamp_ms: i64 = row.get(1).map_err(StorageError::from)?;
        let data: String = row.get(2).map_err(StorageError::from)?;
        total_bytes = total_bytes
            .checked_add(data.len())
            .ok_or(StorageError::Limit)?;
        if total_bytes > max_bytes {
            return Err(StorageError::Limit.into());
        }
        let event: ThreadEvent = decode(&data)?;
        match event {
            ThreadEvent::HistoryStarted => {
                if history_backup.is_none() {
                    history_backup = Some(projection.clone());
                }
                projection = MessageTimeProjection::default();
            }
            ThreadEvent::HistoryCompleted => history_backup = None,
            ThreadEvent::Error {
                recoverable: false, ..
            } => {
                if let Some(previous) = history_backup.take() {
                    projection = previous;
                }
                // A fatal replay error appends a notice after restoring history.
                projection.timeline_tail = None;
            }
            event => projection.apply(event, &event_id, timestamp_ms),
        }
    }
    Ok(projection)
}

fn archive_bytes(
    task: &Task,
    thread: &Thread,
    message_times: &MessageTimeProjection,
) -> WorkspaceResult<Vec<u8>> {
    if matches!(task.state, TaskState::Running | TaskState::Waiting)
        || matches!(thread.state, TaskState::Running | TaskState::Waiting)
        || thread.history_in_progress()
        || thread
            .turns
            .last()
            .is_some_and(|turn| turn.finished_at_ms.is_none())
    {
        return Err(WorkspaceError::Invalid("Conversation is running, waiting or restoring history. Finish or stop it before exporting a ZIP.".into()));
    }
    let markdown = text_export(task, thread)?;
    let messages = thread
        .timeline
        .iter()
        .filter_map(|item| {
            let TranscriptItem::Message { index } = item else {
                return None;
            };
            thread.messages.get(*index).map(|message| ArchiveMessage {
                id: &message.id,
                role: message.role,
                text: &message.text,
                created_at_ms: message_times
                    .created_at_ms
                    .get(&(message.id.clone(), role_key(message.role)))
                    .copied(),
                updated_at_ms: message_times
                    .updated_at_ms
                    .get(&(message.id.clone(), role_key(message.role)))
                    .copied(),
                images: thread
                    .images
                    .iter()
                    .filter(|image| image.message_id == message.id && image.role == message.role)
                    .map(|image| ImageMetadata {
                        id: image.id,
                        source: image.image.source,
                        mime_type: &image.image.mime_type,
                        embedded: false,
                    })
                    .collect(),
            })
        })
        .collect();
    let snapshot = Archive {
        format: "synara-thread-export-v1",
        task_id: task.id,
        thread_id: task.thread_id,
        title: &task.title,
        agent_id: &task.agent_id,
        task_scope: task.scope,
        updated_at_ms: task.updated_at_ms,
        snapshot_sequence: thread.last_sequence,
        state: thread.state,
        current_model: thread.configuration.current_model.as_deref(),
        current_mode: thread.configuration.current_mode.as_deref(),
        excluded: EXCLUDED,
        turns: thread
            .turns
            .iter()
            .map(|turn| ArchiveTurn {
                started_at_ms: turn.started_at_ms,
                finished_at_ms: turn.finished_at_ms,
                failed: turn.failed,
            })
            .collect(),
        messages,
    };
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o600);
    zip.start_file("thread.json", options).map_err(zip_error)?;
    serde_json::to_writer_pretty(
        BoundedWriter {
            inner: &mut zip,
            remaining: MAX_JSON_BYTES,
        },
        &snapshot,
    )
    .map_err(StorageError::from)?;
    zip.start_file("transcript.md", options)
        .map_err(zip_error)?;
    zip.write_all(markdown.as_bytes())
        .map_err(StorageError::from)?;
    let bytes = zip.finish().map_err(zip_error)?.into_inner();
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(StorageError::Limit.into());
    }
    Ok(bytes)
}
impl WorkspaceService {
    /// Explicit local export of a completed durable snapshot. Both ZIP entries
    /// share one SQLite read snapshot and the existing no-overwrite publisher.
    pub async fn export_zip_conversation(
        &self,
        task: TaskId,
        destination: PathBuf,
    ) -> WorkspaceResult<()> {
        let snapshot = self
            .access(move |store| {
                let tx = store
                    .connection
                    .transaction_with_behavior(TransactionBehavior::Deferred)
                    .map_err(StorageError::from)?;
                let snapshot = read_conversation(&tx, task)?;
                let message_times = read_message_times(&tx, snapshot.1.id)?;
                tx.commit().map_err(StorageError::from)?;
                Ok((snapshot, message_times))
            })
            .await?;
        tokio::task::spawn_blocking(move || {
            let bytes = archive_bytes(&snapshot.0.0, &snapshot.0.1, &snapshot.1)?;
            write_new_export(&destination, &bytes)?;
            Ok(())
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_timestamp_scan_enforces_event_and_byte_bounds() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE events(thread_id TEXT,sequence INTEGER,id TEXT,timestamp_ms INTEGER,data TEXT);",
            )
            .unwrap();
        let thread_id = ThreadId::new();
        let event = serde_json::to_string(&ThreadEvent::Notice {
            message: "bounded".into(),
        })
        .unwrap();
        connection
            .execute(
                "INSERT INTO events(thread_id,sequence,id,timestamp_ms,data) VALUES(?1,1,'event',1,?2)",
                rusqlite::params![thread_id.to_string(), event],
            )
            .unwrap();

        assert!(message_times_with_limits(&connection, thread_id, 0, usize::MAX).is_err());
        assert!(message_times_with_limits(&connection, thread_id, 1, 0).is_err());
    }
}
