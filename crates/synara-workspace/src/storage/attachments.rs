//! User-selected attachment snapshots. No source path, URL loader or provider
//! authority is persisted. Pending data survives restart until explicitly sent/removed.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};
use synara_agent::{Prompt, PromptPart};
mod intake;
mod media;
#[cfg(test)]
mod tests;

pub const MAX_ATTACHMENT_BATCH_BYTES: usize = 2 * 1024 * 1024;
const MAX_STORED_BYTES: usize = 3 * 1024 * 1024;
const MAX_ATTACHMENTS: usize = 8;
const FOLDER_SNAPSHOT_PREFIX: &str = "Folder snapshot (names and types only) - ";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    Png,
    Jpeg,
    Text,
}
impl AttachmentKind {
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Text => "text/plain",
        }
    }
    pub fn is_image(self) -> bool {
        self != Self::Text
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentInfo {
    pub id: String,
    pub name: String,
    pub kind: AttachmentKind,
    pub bytes: usize,
    pub dimensions: Option<(u32, u32)>,
    #[serde(default)]
    pub source: synara_core::ImageSource,
}
impl AttachmentInfo {
    fn uri(&self) -> String {
        format!("synara-attachment://{}", self.id)
    }
    fn label(&self) -> String {
        if self.is_folder_snapshot() {
            format!("Attached folder snapshot: {}", self.name)
        } else {
            format!("Attached file: {}", self.name)
        }
    }
    pub fn is_folder_snapshot(&self) -> bool {
        self.kind == AttachmentKind::Text && self.name.starts_with(FOLDER_SNAPSHOT_PREFIX)
    }
}
#[derive(Clone, Debug, Default)]
pub struct AttachmentDraft {
    pub revision: u64,
    pub pending: Vec<AttachmentInfo>,
    pub recent: Vec<AttachmentInfo>,
}
impl AttachmentDraft {
    /// Matches the existing ACP user-text projection without embedding image bytes.
    pub fn transcript_text(&self, text: &str) -> String {
        let mut parts = vec![text.to_owned()];
        for info in &self.pending {
            parts.push(info.label());
            parts.push(if info.kind.is_image() {
                "[Image]".into()
            } else {
                format!("[Context: {}]", info.uri())
            });
        }
        parts.join("\n")
    }
    pub fn unsupported(&self, capabilities: &AgentCapabilities) -> Option<&'static str> {
        if self.pending.iter().any(|a| a.kind.is_image()) && !capabilities.image_prompts {
            Some(
                "This agent did not advertise image prompts. Remove the images or choose a compatible agent.",
            )
        } else if self.pending.iter().any(|a| a.kind == AttachmentKind::Text)
            && !capabilities.embedded_context
        {
            Some(
                "This agent did not advertise embedded file context. Remove the files or choose a compatible agent.",
            )
        } else {
            None
        }
    }
}
#[derive(Clone)]
pub enum AttachmentInput {
    File(PathBuf),
    /// Explicitly selected directory. Only a bounded, one-level listing is read;
    /// its source path and file contents are never persisted or sent.
    Folder(PathBuf),
    Bytes {
        name: String,
        bytes: Vec<u8>,
    },
    Capture {
        name: String,
        bytes: Vec<u8>,
        source: synara_core::ImageSource,
    },
}
pub struct AttachmentPreview {
    pub info: AttachmentInfo,
    pub bytes: Vec<u8>,
}
#[derive(Clone)]
pub enum AttachmentEdit {
    Remove(String),
    Reuse(String),
    ClearPending,
    ForgetRecent,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredAttachment {
    info: AttachmentInfo,
    hex: String,
}
impl StoredAttachment {
    fn bytes(&self) -> WorkspaceResult<Vec<u8>> {
        hex::decode(&self.hex)
            .map_err(|_| invalid("Stored attachment data is malformed. Nothing was replaced."))
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    version: u32,
    revision: u64,
    pending: Vec<StoredAttachment>,
    recent: Vec<StoredAttachment>,
}
impl Default for Stored {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            pending: vec![],
            recent: vec![],
        }
    }
}
impl Stored {
    fn summary(&self) -> AttachmentDraft {
        AttachmentDraft {
            revision: self.revision,
            pending: self.pending.iter().map(|a| a.info.clone()).collect(),
            recent: self.recent.iter().map(|a| a.info.clone()).collect(),
        }
    }
    fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1
            || self.pending.len() > MAX_ATTACHMENTS
            || self.recent.len() > MAX_ATTACHMENTS
        {
            return Err(invalid(
                "Unsupported or oversized stored attachment draft. Data was preserved.",
            ));
        }
        let mut ids = HashSet::new();
        let mut bytes = 0usize;
        for item in self.pending.iter().chain(&self.recent) {
            let info = &item.info;
            if !valid_id(&info.id)
                || !ids.insert(&info.id)
                || !valid_name(&info.name)
                || info.bytes == 0
                || info.bytes > MAX_ATTACHMENT_BATCH_BYTES
                || match (info.kind, info.dimensions) {
                    (AttachmentKind::Text, None) => false,
                    (AttachmentKind::Png | AttachmentKind::Jpeg, Some((w, h))) => {
                        w == 0
                            || h == 0
                            || w > 8192
                            || h > 8192
                            || u64::from(w) * u64::from(h) > 16_000_000
                    }
                    _ => true,
                }
                || item.hex.len() != info.bytes.saturating_mul(2)
                || !item.hex.bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err(invalid(
                    "Stored attachment metadata is invalid. Data was preserved.",
                ));
            }
            bytes = bytes.saturating_add(info.bytes);
        }
        if bytes > MAX_STORED_BYTES
            || self.pending.iter().map(|a| a.info.bytes).sum::<usize>() > MAX_ATTACHMENT_BATCH_BYTES
        {
            return Err(invalid(
                "Attachments exceed the storage limit. Data was preserved.",
            ));
        }
        Ok(())
    }
    fn trim_recent(&mut self) {
        while self.recent.len() > MAX_ATTACHMENTS
            || self
                .pending
                .iter()
                .chain(&self.recent)
                .map(|a| a.info.bytes)
                .sum::<usize>()
                > MAX_STORED_BYTES
        {
            if self.recent.is_empty() {
                break;
            }
            self.recent.remove(0);
        }
    }
}
fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
fn valid_id(id: &str) -> bool {
    id.len() == 36 && uuid::Uuid::parse_str(id).is_ok()
}
fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= 240
        && !name.chars().any(char::is_control)
        && !name.contains(['/', '\\'])
}
fn key(task: TaskId) -> String {
    format!("task-attachments:{task}")
}
fn read(connection: &Connection, task: TaskId) -> WorkspaceResult<Stored> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)",
            [task.to_string()],
            |r| r.get(0),
        )
        .map_err(StorageError::from)?;
    if !exists {
        return Err(WorkspaceError::NotFound);
    }
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [key(task)],
            |r| r.get(0),
        )
        .optional()
        .map_err(StorageError::from)?;
    let state: Stored = raw.as_deref().map(decode).transpose()?.unwrap_or_default();
    state.validate()?;
    Ok(state)
}
fn write(connection: &Connection, task: TaskId, state: &mut Stored) -> WorkspaceResult<()> {
    state.trim_recent();
    state.validate()?;
    state.revision = state.revision.checked_add(1).ok_or(StorageError::Limit)?;
    connection.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![key(task),encode(state)?]).map_err(StorageError::from)?;
    Ok(())
}
fn expected(state: &Stored, revision: u64) -> WorkspaceResult<()> {
    if state.revision == revision {
        Ok(())
    } else {
        Err(invalid(
            "Attachments changed elsewhere. Reload and review them before trying again.",
        ))
    }
}
impl WorkspaceService {
    pub async fn attachment_draft(&self, task: TaskId) -> WorkspaceResult<AttachmentDraft> {
        self.access(move |store| Ok(read(&store.connection, task)?.summary()))
            .await
    }
    /// Read and validate the complete selection before committing any of it.
    pub async fn add_attachments(
        &self,
        task: TaskId,
        revision: u64,
        inputs: Vec<AttachmentInput>,
    ) -> WorkspaceResult<AttachmentDraft> {
        let incoming = intake::prepare(inputs).await?;
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(StorageError::from)?;
            let mut state = read(&tx, task)?;
            expected(&state, revision)?;
            // Every explicit add owns a fresh ID, even for identical bytes. A
            // receipt from another window must not consume a new user's selection.
            state.pending.extend(incoming);
            write(&tx, task, &mut state)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(state.summary())
        })
        .await
    }
    pub async fn edit_attachments(
        &self,
        task: TaskId,
        revision: u64,
        edit: AttachmentEdit,
    ) -> WorkspaceResult<AttachmentDraft> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(StorageError::from)?;
            let mut state = read(&tx, task)?;
            expected(&state, revision)?;
            match edit {
                AttachmentEdit::Remove(id) => state.pending.retain(|a| a.info.id != id),
                AttachmentEdit::ClearPending => state.pending.clear(),
                AttachmentEdit::ForgetRecent => state.recent.clear(),
                AttachmentEdit::Reuse(id) => {
                    let index = state
                        .recent
                        .iter()
                        .position(|a| a.info.id == id)
                        .ok_or(WorkspaceError::NotFound)?;
                    let mut reused = state.recent.remove(index);
                    reused.info.id = uuid::Uuid::new_v4().to_string();
                    state.pending.push(reused);
                }
            }
            write(&tx, task, &mut state)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(state.summary())
        })
        .await
    }
    /// Local transcript acknowledgement is not proof of provider delivery. Keep
    /// the snapshots in Recent for an explicit retry, never automatically replay.
    pub async fn acknowledge_attachments(
        &self,
        task: TaskId,
        ids: Vec<String>,
    ) -> WorkspaceResult<AttachmentDraft> {
        if ids.len() > MAX_ATTACHMENTS || ids.iter().any(|id| !valid_id(id)) {
            return Err(StorageError::Identity.into());
        }
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(StorageError::from)?;
            let mut state = read(&tx, task)?;
            let mut keep = Vec::new();
            for item in std::mem::take(&mut state.pending) {
                if ids.contains(&item.info.id) {
                    state.recent.push(item);
                } else {
                    keep.push(item);
                }
            }
            state.pending = keep;
            write(&tx, task, &mut state)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(state.summary())
        })
        .await
    }
    pub async fn attachment_preview(
        &self,
        task: TaskId,
        id: String,
    ) -> WorkspaceResult<AttachmentPreview> {
        let item = self
            .access(move |store| {
                let state = read(&store.connection, task)?;
                state
                    .pending
                    .into_iter()
                    .chain(state.recent)
                    .find(|a| a.info.id == id)
                    .ok_or(WorkspaceError::NotFound)
            })
            .await?;
        intake::run(move || {
            let bytes = item.bytes()?;
            let info = intake::inspect(item.info.name.clone(), &bytes)?;
            if info.kind != item.info.kind || info.dimensions != item.info.dimensions {
                return Err(StorageError::Identity.into());
            }
            Ok(AttachmentPreview {
                info: item.info,
                bytes,
            })
        })
        .await
    }
    pub(crate) async fn attached_prompt(
        &self,
        task: TaskId,
        text: String,
        revision: u64,
    ) -> WorkspaceResult<Prompt> {
        let state = self
            .access(move |store| {
                let state = read(&store.connection, task)?;
                expected(&state, revision)?;
                Ok(state)
            })
            .await?;
        // Existing ACP encoding checks negotiated capability and the 4 MiB limit
        // before recording a prompt or invoking an agent method.
        intake::run(move || {
            let mut parts = vec![PromptPart::Text(text)];
            for item in state.pending {
                let bytes = item.bytes()?;
                let checked = intake::inspect(item.info.name.clone(), &bytes)?;
                if checked.kind != item.info.kind {
                    return Err(StorageError::Identity.into());
                }
                parts.push(PromptPart::Text(item.info.label()));
                parts.push(match item.info.kind {
                    AttachmentKind::Text => PromptPart::Context {
                        uri: item.info.uri(),
                        text: String::from_utf8(bytes).map_err(|_| StorageError::Identity)?,
                        mime_type: "text/plain".into(),
                    },
                    kind => PromptPart::MediaImage(synara_core::TranscriptImage {
                        source: item.info.source,
                        base64: intake::base64(&bytes),
                        mime_type: kind.mime_type().into(),
                    }),
                });
            }
            Ok(Prompt { parts })
        })
        .await
    }
}
