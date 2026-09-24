//! Durable, user-owned conversation utilities. None of these operations submits
//! prompts, changes approval policy, or launches an agent or workspace process.
mod archive;
mod related;
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
pub use related::{
    HandoffReview, HandoffTarget, RelatedThreadKind, RevisionSource, SideThreadIndex, ThreadOrigin,
    ThreadRecap,
};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

const MAX_PINS: usize = 256;
const MAX_PIN_BYTES: usize = 512 * 1024;
const MAX_QUERY_BYTES: usize = 1024;
const MAX_SEARCH_HITS: usize = 500;
const MAX_EXPORT_BYTES: usize = 8 * 1024 * 1024;
const MAX_REPLAY_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageAnchor {
    pub id: String,
    pub role: Role,
}
impl MessageAnchor {
    pub fn matches(&self, message: &Message) -> bool {
        self.id == message.id && self.role == message.role
    }
    fn valid(&self) -> bool {
        !self.id.is_empty() && self.id.len() <= 1024 && !self.id.chars().any(char::is_control)
    }
}
impl From<&Message> for MessageAnchor {
    fn from(message: &Message) -> Self {
        Self {
            id: message.id.clone(),
            role: message.role,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct MessageSearch {
    pub hits: Vec<MessageAnchor>,
    pub sequence: u64,
    pub limited: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MessagePins {
    version: u32,
    entries: Vec<MessageAnchor>,
}
impl Default for MessagePins {
    fn default() -> Self {
        Self {
            version: 1,
            entries: Vec::new(),
        }
    }
}
impl MessagePins {
    fn validate(&self) -> StorageResult<()> {
        if self.version != 1
            || self.entries.len() > MAX_PINS
            || self
                .entries
                .iter()
                .enumerate()
                .any(|(i, entry)| !entry.valid() || self.entries[..i].contains(entry))
        {
            return Err(StorageError::Identity);
        }
        Ok(())
    }
}
fn pins_key(task: TaskId) -> String {
    format!("message-pins:{task}")
}
fn read_pins(connection: &Connection, task: TaskId) -> StorageResult<MessagePins> {
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [pins_key(task)],
            |row| row.get(0),
        )
        .optional()?;
    let stored = match raw {
        Some(raw) if raw.len() > MAX_PIN_BYTES => return Err(StorageError::Limit),
        Some(raw) => decode::<MessagePins>(&raw)?,
        None => MessagePins::default(),
    };
    stored.validate()?;
    Ok(stored)
}
/// Called under a read/write transaction so task identity, head and events are
/// from one SQLite snapshot. Actual row bytes are bounded even for corrupt data.
fn read_conversation(connection: &Connection, id: TaskId) -> WorkspaceResult<(Task, Thread)> {
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM tasks WHERE id=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(StorageError::from)?;
    let task: Task = decode(&raw.ok_or(WorkspaceError::NotFound)?)?;
    if task.id != id {
        return Err(StorageError::Identity.into());
    }
    let mut thread = Thread::new(task.thread_id);
    let mut query = connection
        .prepare(
            "SELECT sequence,id,timestamp_ms,data FROM events WHERE thread_id=?1 ORDER BY sequence",
        )
        .map_err(StorageError::from)?;
    let mut rows = query
        .query([task.thread_id.to_string()])
        .map_err(StorageError::from)?;
    let mut total_bytes = 0usize;
    while let Some(row) = rows.next().map_err(StorageError::from)? {
        let data: String = row.get(3).map_err(StorageError::from)?;
        total_bytes = total_bytes
            .checked_add(data.len())
            .ok_or(StorageError::Limit)?;
        if total_bytes > MAX_REPLAY_BYTES || thread.last_sequence >= 200_000 {
            return Err(StorageError::Limit.into());
        }
        let sequence: i64 = row.get(0).map_err(StorageError::from)?;
        let event_id: String = row.get(1).map_err(StorageError::from)?;
        let envelope = EventEnvelope {
            id: EventId(uuid::Uuid::parse_str(&event_id).map_err(|_| StorageError::Sequence)?),
            thread_id: task.thread_id,
            sequence: u64::try_from(sequence).map_err(|_| StorageError::Sequence)?,
            timestamp_ms: row.get(2).map_err(StorageError::from)?,
            event: decode(&data)?,
        };
        thread.apply(&envelope).map_err(StorageError::from)?;
    }
    let head: Option<i64> = connection
        .query_row(
            "SELECT sequence FROM event_heads WHERE thread_id=?1",
            [task.thread_id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(StorageError::from)?;
    if u64::try_from(head.unwrap_or(0)).map_err(|_| StorageError::Sequence)? != thread.last_sequence
    {
        return Err(StorageError::Sequence.into());
    }
    Ok((task, thread))
}
fn find_in_thread(thread: &Thread, query: &str) -> MessageSearch {
    let query = query.trim().to_lowercase();
    let mut result = MessageSearch {
        sequence: thread.last_sequence,
        ..Default::default()
    };
    if query.is_empty() {
        return result;
    }
    // Search reconstructed messages, not individual deltas. Never concatenate
    // different roles/messages, and do not interpret SQL wildcards or regexes.
    for item in &thread.timeline {
        if let TranscriptItem::Message { index } = item
            && let Some(message) = thread.messages.get(*index)
            && message.text.to_lowercase().contains(&query)
        {
            if result.hits.len() == MAX_SEARCH_HITS {
                result.limited = true;
                break;
            }
            result.hits.push(MessageAnchor::from(message));
        }
    }
    result
}
fn text_export(task: &Task, thread: &Thread) -> StorageResult<String> {
    let mut text = String::from("# Synara text conversation\n\n");
    let title: String = task.title.split_whitespace().collect::<Vec<_>>().join(" ");
    text.push_str(&title);
    text.push_str("\n\nText-only export. Tool payloads, attachments, authentication and session configuration are not included.\n");
    for item in &thread.timeline {
        if let TranscriptItem::Message { index } = item
            && let Some(message) = thread.messages.get(*index)
        {
            let role = match message.role {
                Role::User => "User",
                Role::Assistant => "Assistant",
                Role::Reasoning => "Reasoning",
            };
            let heading = format!("\n## {role}\n\n");
            if text
                .len()
                .saturating_add(heading.len())
                .saturating_add(message.text.len())
                .saturating_add(1)
                > MAX_EXPORT_BYTES
            {
                return Err(StorageError::Limit);
            }
            text.push_str(&heading);
            text.push_str(&message.text);
            text.push('\n');
        }
    }
    if text.len() > MAX_EXPORT_BYTES {
        return Err(StorageError::Limit);
    }
    Ok(text)
}
/// Complete a private file before making the requested filename visible. The
/// hard-link creation fails rather than replacing any existing file or symlink.
pub(crate) fn write_new_export(destination: &Path, bytes: &[u8]) -> StorageResult<()> {
    if !destination.is_absolute() || destination.file_name().is_none() {
        return Err(StorageError::RecoveryDestination);
    }
    let parent = destination
        .parent()
        .ok_or(StorageError::RecoveryDestination)?
        .canonicalize()?;
    let destination = parent.join(
        destination
            .file_name()
            .ok_or(StorageError::RecoveryDestination)?,
    );
    let staging = parent.join(format!(".synara-export-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&staging)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::hard_link(&staging, &destination)?;
        Ok::<_, std::io::Error>(())
    })();
    drop(file);
    let _ = fs::remove_file(&staging);
    result.map_err(Into::into)
}
impl WorkspaceService {
    pub async fn message_pins(&self, task: TaskId) -> WorkspaceResult<Vec<MessageAnchor>> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Deferred)
                .map_err(StorageError::from)?;
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)",
                    [task.to_string()],
                    |row| row.get(0),
                )
                .map_err(StorageError::from)?;
            if !exists {
                return Err(WorkspaceError::NotFound);
            }
            let pins = read_pins(&tx, task)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(pins.entries)
        })
        .await
    }
    pub async fn set_message_pin(
        &self,
        task: TaskId,
        anchor: MessageAnchor,
        enabled: bool,
    ) -> WorkspaceResult<Vec<MessageAnchor>> {
        if !anchor.valid() {
            return Err(StorageError::Identity.into());
        }
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(StorageError::from)?;
            let (_, thread) = read_conversation(&tx, task)?;
            let mut pins = read_pins(&tx, task)?;
            if enabled {
                if !thread.messages.iter().any(|message| anchor.matches(message)) {
                    return Err(WorkspaceError::NotFound);
                }
                if !pins.entries.contains(&anchor) { pins.entries.push(anchor); }
            } else {
                // A stale pin can always be explicitly removed, without
                // manufacturing a message or rewriting hydrated history.
                pins.entries.retain(|candidate| candidate != &anchor);
            }
            pins.validate()?;
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![pins_key(task), encode(&pins)?]).map_err(StorageError::from)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(pins.entries)
        }).await
    }
    pub async fn find_messages(
        &self,
        task: TaskId,
        query: String,
    ) -> WorkspaceResult<MessageSearch> {
        if query.len() > MAX_QUERY_BYTES {
            return Err(StorageError::Limit.into());
        }
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Deferred)
                .map_err(StorageError::from)?;
            let (_, thread) = read_conversation(&tx, task)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(find_in_thread(&thread, &query))
        })
        .await
    }
    pub async fn text_conversation(&self, task: TaskId) -> WorkspaceResult<String> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Deferred)
                .map_err(StorageError::from)?;
            let (task, thread) = read_conversation(&tx, task)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(text_export(&task, &thread)?)
        })
        .await
    }
    pub async fn export_text_conversation(
        &self,
        task: TaskId,
        destination: PathBuf,
    ) -> WorkspaceResult<()> {
        let text = self.text_conversation(task).await?;
        tokio::task::spawn_blocking(move || write_new_export(&destination, text.as_bytes()))
            .await
            .map_err(|_| WorkspaceError::Worker)??;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn seed(service: &WorkspaceService, root: &Path) -> Task {
        let project = service.add_local_workspace(root.to_owned()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_task(project.id, "Conversation utilities".into(), agent)
            .await
            .unwrap();
        let id = task.thread_id;
        service
            .access(move |store| {
                let events = [
                    ThreadEvent::PromptStarted {
                        turn: "turn".into(),
                    },
                    ThreadEvent::TextDelta {
                        message_id: Some("shared".into()),
                        role: Role::User,
                        text: "Keep draft private.".into(),
                    },
                    ThreadEvent::TextDelta {
                        message_id: Some("shared".into()),
                        role: Role::Assistant,
                        text: "Caffè 日".into(),
                    },
                    ThreadEvent::TextDelta {
                        message_id: Some("shared".into()),
                        role: Role::Assistant,
                        text: "本語\n  exact whitespace\n".into(),
                    },
                    ThreadEvent::TextDelta {
                        message_id: Some("thinking".into()),
                        role: Role::Reasoning,
                        text: "Separate reasoning text".into(),
                    },
                    ThreadEvent::PromptFinished {
                        reason: "end_turn".into(),
                    },
                ];
                for (i, event) in events.into_iter().enumerate() {
                    store.append(&EventEnvelope {
                        id: EventId::new(),
                        thread_id: id,
                        sequence: i as u64 + 1,
                        timestamp_ms: i as i64,
                        event,
                    })?;
                }
                Ok(())
            })
            .await
            .unwrap();
        task
    }
    fn anchor(role: Role) -> MessageAnchor {
        MessageAnchor {
            id: "shared".into(),
            role,
        }
    }
    #[tokio::test]
    async fn search_joins_message_chunks_not_roles_and_treats_wildcards_literally() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let task = seed(&service, dir.path()).await;
        let found = service
            .find_messages(task.id, "CAFfÈ 日本語".into())
            .await
            .unwrap();
        assert_eq!(found.hits, [anchor(Role::Assistant)]);
        assert_eq!(found.sequence, 6);
        assert!(!found.limited);
        for query in ["private.Caffè", "%", "_", "does not exist"] {
            assert!(
                service
                    .find_messages(task.id, query.into())
                    .await
                    .unwrap()
                    .hits
                    .is_empty()
            );
        }
        assert!(
            service
                .find_messages(task.id, "x".repeat(MAX_QUERY_BYTES + 1))
                .await
                .is_err()
        );
        assert!(
            service
                .find_messages(TaskId::new(), "hello".into())
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn pins_are_role_scoped_idempotent_and_do_not_change_transcript_or_draft() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let task = seed(&service, dir.path()).await;
        service
            .save_task_draft(task.id, "Unsent text".into())
            .await
            .unwrap();
        let before = service.text_conversation(task.id).await.unwrap();
        service
            .set_message_pin(task.id, anchor(Role::User), true)
            .await
            .unwrap();
        service
            .set_message_pin(task.id, anchor(Role::Assistant), true)
            .await
            .unwrap();
        assert_eq!(
            service
                .set_message_pin(task.id, anchor(Role::User), true)
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            service
                .set_message_pin(task.id, anchor(Role::User), false)
                .await
                .unwrap(),
            [anchor(Role::Assistant)]
        );
        assert!(
            service
                .set_message_pin(
                    task.id,
                    MessageAnchor {
                        id: "unknown".into(),
                        role: Role::User
                    },
                    true
                )
                .await
                .is_err()
        );
        assert_eq!(service.text_conversation(task.id).await.unwrap(), before);
        assert_eq!(service.task_draft(task.id).await.unwrap(), "Unsent text");
        assert!(service.session(task.thread_id).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn pins_survive_restart_concurrent_writes_and_settings_saves() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.sqlite3");
        let a = WorkspaceService::open(path.clone()).await.unwrap();
        let task = seed(&a, dir.path()).await;
        let b = WorkspaceService::open(path.clone()).await.unwrap();
        let settings = a.settings().await.unwrap().settings;
        let (x, y) = tokio::join!(
            a.set_message_pin(task.id, anchor(Role::User), true),
            b.set_message_pin(task.id, anchor(Role::Assistant), true)
        );
        x.unwrap();
        y.unwrap();
        a.save_settings(settings).await.unwrap();
        drop(a);
        drop(b);
        let reopened = WorkspaceService::open(path).await.unwrap();
        let pins = reopened.message_pins(task.id).await.unwrap();
        assert_eq!(pins.len(), 2);
        assert!(pins.contains(&anchor(Role::User)) && pins.contains(&anchor(Role::Assistant)));
    }
    #[tokio::test]
    async fn malformed_pin_data_is_not_replaced_and_delete_removes_pin_preference() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let task = seed(&service, dir.path()).await;
        service
            .access(move |store| {
                store.set_preference(
                    &pins_key(task.id),
                    &serde_json::json!({"version":99,"entries":[]}),
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(service.message_pins(task.id).await.is_err());
        assert!(
            service
                .set_message_pin(task.id, anchor(Role::User), true)
                .await
                .is_err()
        );
        service
            .access(move |store| {
                assert_eq!(
                    store
                        .preference::<serde_json::Value>(&pins_key(task.id))?
                        .unwrap()["version"],
                    99
                );
                Ok(())
            })
            .await
            .unwrap();
        service.archive_task(task.id).await.unwrap();
        service.delete_task(task.id).await.unwrap();
        service
            .access(move |store| {
                assert!(store.preference_raw(&pins_key(task.id))?.is_none());
                Ok(())
            })
            .await
            .unwrap();
        assert!(
            service
                .set_message_pin(task.id, anchor(Role::User), true)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn export_preserves_unicode_and_whitespace_but_excludes_unsent_text() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let task = seed(&service, dir.path()).await;
        service
            .save_task_draft(task.id, "DRAFT-CANARY-NOT-FOR-EXPORT".into())
            .await
            .unwrap();
        let text = service.text_conversation(task.id).await.unwrap();
        assert!(text.contains("Caffè 日本語\n  exact whitespace\n"));
        assert!(!text.contains("DRAFT-CANARY"));
        let path = dir.path().join("conversation.md");
        service
            .export_text_conversation(task.id, path.clone())
            .await
            .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
        fs::write(&path, "Existing user file").unwrap();
        assert!(
            service
                .export_text_conversation(task.id, path.clone())
                .await
                .is_err()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "Existing user file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            let link = dir.path().join("symlink.md");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(
                service
                    .export_text_conversation(task.id, link)
                    .await
                    .is_err()
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), "Existing user file");
        }
        assert!(!fs::read_dir(dir.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".synara-export-")
        }));
    }
    #[test]
    fn pin_input_bounds_and_keys_are_strict() {
        for id in ["", "newline\n", &"x".repeat(1025)] {
            assert!(
                !MessageAnchor {
                    id: id.into(),
                    role: Role::User
                }
                .valid()
            );
        }
        let pins = MessagePins {
            version: 1,
            entries: vec![anchor(Role::User); MAX_PINS + 1],
        };
        assert!(pins.validate().is_err());
        for key in [
            "message-pins:",
            "message-pins:../settings",
            "message-pins:invalid",
        ] {
            assert!(!valid_preference_key(key));
        }
        assert!(valid_preference_key(&pins_key(TaskId::new())));
    }
    #[tokio::test]
    async fn related_side_thread_and_revision_creation_are_additive_and_unsent() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let task = seed(&service, dir.path()).await;
        service
            .save_task_draft(task.id, "MAIN-DRAFT-CANARY".into())
            .await
            .unwrap();

        let side = service
            .create_side_thread(
                task.id,
                task.agent_id.clone(),
                Some(anchor(Role::Assistant)),
            )
            .await
            .unwrap();
        assert_eq!(side.working_directory, task.working_directory);
        assert_eq!(side.project_id, task.project_id);
        assert!(service.session(side.thread_id).await.unwrap().is_none());
        assert!(
            service
                .thread(side.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
        assert!(
            service
                .task_draft(side.id)
                .await
                .unwrap()
                .contains("Caffè 日本語")
        );

        let source = service
            .revision_source(task.id, anchor(Role::User))
            .await
            .unwrap();
        let revision = service
            .branch_user_revision(source, "Edited question".into())
            .await
            .unwrap();
        let revised_draft = service.task_draft(revision.id).await.unwrap();
        assert!(revised_draft.ends_with("Edited question"));
        assert!(!revised_draft.contains("Caffè 日本語"));
        assert!(service.session(revision.thread_id).await.unwrap().is_none());
        assert!(
            service
                .thread(revision.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );

        assert_eq!(
            service.task_draft(task.id).await.unwrap(),
            "MAIN-DRAFT-CANARY"
        );
        assert_eq!(service.catalog().await.unwrap().tasks.len(), 3);
    }
    #[tokio::test]
    async fn zip_export_keeps_exact_snapshot_private_and_refuses_active_or_existing_targets() {
        use std::io::Read;
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let task = seed(&service, dir.path()).await;
        service
            .save_task_draft(task.id, "UNSENT-DO-NOT-EXPORT".into())
            .await
            .unwrap();
        let thread_id = task.thread_id;
        service
            .access(move |store| {
                store.append(&EventEnvelope {
                    id: EventId::new(),
                    thread_id,
                    sequence: 7,
                    timestamp_ms: 6,
                    event: ThreadEvent::ToolChanged {
                        patch: ToolPatch {
                            id: "tool".into(),
                            title: Some("tool title".into()),
                            status: Some(ToolStatus::Completed),
                            kind: Some("text".into()),
                            output: Some(vec![ToolOutput::Text {
                                text: "TOOL-PAYLOAD-CANARY".into(),
                            }]),
                        },
                    },
                })?;
                Ok(())
            })
            .await
            .unwrap();
        let original = service.text_conversation(task.id).await.unwrap();
        let path = dir.path().join("conversation.zip");
        service
            .export_zip_conversation(task.id, path.clone())
            .await
            .unwrap();
        let bytes = fs::read(&path).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone())).unwrap();
        assert_eq!(zip.len(), 2);
        assert_eq!(
            zip.file_names().collect::<Vec<_>>(),
            ["thread.json", "transcript.md"]
        );
        let mut json = String::new();
        zip.by_name("thread.json")
            .unwrap()
            .read_to_string(&mut json)
            .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(payload["format"], "synara-thread-export-v1");
        assert_eq!(payload["snapshotSequence"], 7);
        assert_eq!(payload["taskScope"], serde_json::to_value(task.scope).unwrap());
        assert_eq!(payload["turns"].as_array().unwrap().len(), 1);
        assert_eq!(payload["turns"][0]["startedAtMs"], 0);
        assert_eq!(payload["turns"][0]["finishedAtMs"], 5);
        assert_eq!(payload["turns"][0]["failed"], false);
        assert_eq!(payload["messages"].as_array().unwrap().len(), 3);
        assert_eq!(
            payload["messages"][1]["text"],
            "Caffè 日本語\n  exact whitespace\n"
        );
        assert_eq!(payload["messages"][1]["role"], "assistant");
        assert_eq!(payload["messages"][0]["updatedAtMs"], 1);
        assert_eq!(payload["messages"][1]["updatedAtMs"], 3);
        assert_eq!(payload["messages"][2]["updatedAtMs"], 4);
        assert_eq!(payload["messages"][0]["createdAtMs"], 1);
        assert_eq!(payload["messages"][1]["createdAtMs"], 2);
        assert_eq!(payload["messages"][2]["createdAtMs"], 4);
        assert!(!json.contains("UNSENT-DO-NOT-EXPORT"));
        assert!(!json.contains("TOOL-PAYLOAD-CANARY"));
        assert!(payload.get("workingDirectory").is_none());
        assert!(payload.get("configuration").is_none());
        let mut markdown = String::new();
        zip.by_name("transcript.md")
            .unwrap()
            .read_to_string(&mut markdown)
            .unwrap();
        assert_eq!(markdown, original);
        assert!(
            service
                .export_zip_conversation(task.id, path.clone())
                .await
                .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(service.text_conversation(task.id).await.unwrap(), original);
        let thread_id = task.thread_id;
        service
            .access(move |store| {
                store.append(&EventEnvelope {
                    id: EventId::new(),
                    thread_id,
                    sequence: 8,
                    timestamp_ms: 10,
                    event: ThreadEvent::PromptStarted {
                        turn: "active".into(),
                    },
                })?;
                Ok(())
            })
            .await
            .unwrap();
        let running = dir.path().join("running.zip");
        assert!(
            service
                .export_zip_conversation(task.id, running.clone())
                .await
                .is_err()
        );
        assert!(!running.exists());
        assert!(fs::read_dir(dir.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".synara-export-")
        }));
    }

    #[tokio::test]
    async fn zip_export_derives_message_update_times_from_durable_events_after_restart() {
        use std::io::Read;
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("state.sqlite3");
        let service = WorkspaceService::open(database.clone()).await.unwrap();
        let task = seed(&service, dir.path()).await;
        let barrier_event = EventId::new();
        let explicit_event = EventId::new();
        let new_tail_event = EventId::new();
        let tail_event = EventId::new();
        let fallback_event = EventId::new();
        let thread_id = task.thread_id;
        service
            .access(move |store| {
                for (sequence, id, timestamp_ms, event) in [
                    (
                        7,
                        barrier_event,
                        7,
                        ThreadEvent::Notice {
                            message: "timeline barrier".into(),
                        },
                    ),
                    (
                        8,
                        explicit_event,
                        8,
                        ThreadEvent::TextDelta {
                            message_id: Some("shared".into()),
                            role: Role::Assistant,
                            text: " later".into(),
                        },
                    ),
                    (
                        9,
                        new_tail_event,
                        9,
                        ThreadEvent::TextDelta {
                            message_id: Some("later-reasoning".into()),
                            role: Role::Reasoning,
                            text: "new tail".into(),
                        },
                    ),
                    (
                        10,
                        tail_event,
                        10,
                        ThreadEvent::TextDelta {
                            message_id: None,
                            role: Role::Reasoning,
                            text: " continued".into(),
                        },
                    ),
                    (
                        11,
                        fallback_event,
                        11,
                        ThreadEvent::TextDelta {
                            message_id: None,
                            role: Role::User,
                            text: "anonymous message".into(),
                        },
                    ),
                ] {
                    store.append(&EventEnvelope {
                        id,
                        thread_id,
                        sequence,
                        timestamp_ms,
                        event,
                    })?;
                }
                Ok(())
            })
            .await
            .unwrap();
        drop(service);

        let reopened = WorkspaceService::open(database).await.unwrap();
        let path = dir.path().join("updated-times.zip");
        reopened
            .export_zip_conversation(task.id, path.clone())
            .await
            .unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(fs::read(path).unwrap())).unwrap();
        let mut json = String::new();
        zip.by_name("thread.json")
            .unwrap()
            .read_to_string(&mut json)
            .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(payload["snapshotSequence"], 11);
        assert_eq!(payload["messages"][1]["updatedAtMs"], 8);
        assert_eq!(payload["messages"][2]["updatedAtMs"], 4);
        assert_eq!(payload["messages"][3]["updatedAtMs"], 10);
        assert_eq!(payload["messages"][3]["text"], "new tail continued");
        assert_eq!(
            payload["messages"][4]["id"],
            format!("event-{fallback_event}")
        );
        assert_eq!(payload["messages"][4]["updatedAtMs"], 11);
    }

    #[tokio::test]
    async fn zip_export_resets_message_update_projection_for_completed_history_replay() {
        use std::io::Read;
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let task = seed(&service, dir.path()).await;
        let thread_id = task.thread_id;
        service
            .access(move |store| {
                for (sequence, timestamp_ms, event) in [
                    (7, 7, ThreadEvent::HistoryStarted),
                    (
                        8,
                        8,
                        ThreadEvent::TextDelta {
                            message_id: Some("restored".into()),
                            role: Role::User,
                            text: "restored history".into(),
                        },
                    ),
                    (9, 9, ThreadEvent::HistoryCompleted),
                ] {
                    store.append(&EventEnvelope {
                        id: EventId::new(),
                        thread_id,
                        sequence,
                        timestamp_ms,
                        event,
                    })?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let path = dir.path().join("restored.zip");
        service
            .export_zip_conversation(task.id, path.clone())
            .await
            .unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(fs::read(path).unwrap())).unwrap();
        let mut json = String::new();
        zip.by_name("thread.json")
            .unwrap()
            .read_to_string(&mut json)
            .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(payload["snapshotSequence"], 9);
        assert_eq!(payload["messages"].as_array().unwrap().len(), 1);
        assert_eq!(payload["messages"][0]["id"], "restored");
        assert_eq!(payload["messages"][0]["createdAtMs"], 8);
        assert_eq!(payload["messages"][0]["updatedAtMs"], 8);
    }
}
