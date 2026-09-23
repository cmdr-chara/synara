//! Portable conversation snapshots, with explicit projections rather than
//! serializing runtime/session configuration or pending task state.
use super::*;
use std::{
    collections::BTreeMap,
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
    updated_at_ms: i64,
    snapshot_sequence: u64,
    state: TaskState,
    current_model: Option<&'a str>,
    current_mode: Option<&'a str>,
    excluded: &'static [&'static str],
    messages: Vec<ArchiveMessage<'a>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveMessage<'a> {
    id: &'a str,
    role: Role,
    text: &'a str,
    created_at_ms: Option<i64>,
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
fn archive_bytes(task: &Task, thread: &Thread) -> WorkspaceResult<Vec<u8>> {
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
    // The legacy timestamp index is keyed by ID, not role. Preserve unknown
    // timestamps when providers reuse an ID across roles instead of guessing.
    let mut id_counts = BTreeMap::new();
    for message in &thread.messages {
        *id_counts.entry(message.id.as_str()).or_insert(0usize) += 1;
    }
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
                created_at_ms: (id_counts.get(message.id.as_str()) == Some(&1))
                    .then(|| thread.message_timestamps.get(&message.id).copied())
                    .flatten(),
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
        updated_at_ms: task.updated_at_ms,
        snapshot_sequence: thread.last_sequence,
        state: thread.state,
        current_model: thread.configuration.current_model.as_deref(),
        current_mode: thread.configuration.current_mode.as_deref(),
        excluded: EXCLUDED,
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
                tx.commit().map_err(StorageError::from)?;
                Ok(snapshot)
            })
            .await?;
        tokio::task::spawn_blocking(move || {
            let bytes = archive_bytes(&snapshot.0, &snapshot.1)?;
            write_new_export(&destination, &bytes)?;
            Ok(())
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
    }
}
