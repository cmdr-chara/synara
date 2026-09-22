//! Read-only provider history discovery. A preview is an immutable, bounded text
//! snapshot, never a provider session, permission grant or executable prompt.
use crate::{StorageError, WorkspaceError, WorkspaceResult, WorkspaceService, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use synara_core::{Project, ProjectId, Role, Task, WorkspaceLocation};
use synara_runtime::WorkspaceFs;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_FILES: usize = 256;
const MAX_ENTRIES: usize = 10000;
const MAX_RECORDS: usize = 20000;
const MAX_MESSAGES: usize = 4096;
const MAX_TEXT: usize = 4 * 1024 * 1024;
const MAX_LINE: usize = 1024 * 1024;
const MAX_MESSAGE: usize = 256 * 1024;
pub(crate) const IMPORT_LEDGER: &str = "history-imports-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryProvider {
    Codex,
    Claude,
}
impl HistoryProvider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude",
        }
    }
}
#[derive(Clone, Debug)]
pub struct HistoryFile {
    pub relative_path: PathBuf,
    pub bytes: u64,
}
#[derive(Clone)]
pub struct HistorySource {
    pub(crate) provider: HistoryProvider,
    pub(crate) fs: Arc<WorkspaceFs>,
}
impl std::fmt::Debug for HistorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistorySource")
            .field("provider", &self.provider)
            .field("root", &self.fs.root())
            .finish()
    }
}
#[derive(Clone, Debug)]
pub struct HistoryDiscovery {
    pub source: HistorySource,
    pub files: Vec<HistoryFile>,
    pub skipped: usize,
    pub limited: bool,
}
#[derive(Clone, Debug)]
pub struct ImportedMessage {
    pub role: Role,
    pub text: String,
    pub timestamp_ms: Option<i64>,
}
#[derive(Clone, Debug)]
pub struct HistoryLeaf {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug)]
pub struct HistoryPreview {
    pub(crate) source: HistorySource,
    pub(crate) file: HistoryFile,
    pub(crate) digest: String,
    pub(crate) session: Uuid,
    pub(crate) title: String,
    pub(crate) cwd: Option<String>,
    pub(crate) messages: Vec<ImportedMessage>,
    pub(crate) warnings: Vec<String>,
    pub(crate) leaves: Vec<HistoryLeaf>,
    pub(crate) selected_leaf: Option<String>,
}
impl HistoryPreview {
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn provider(&self) -> HistoryProvider {
        self.source.provider
    }
    pub fn session_id(&self) -> String {
        self.session.to_string()
    }
    pub fn source_path(&self) -> PathBuf {
        self.source.fs.root().join(&self.file.relative_path)
    }
    pub fn source_cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }
    pub fn messages(&self) -> &[ImportedMessage] {
        &self.messages
    }
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    pub fn leaves(&self) -> &[HistoryLeaf] {
        &self.leaves
    }
    pub fn selected_leaf(&self) -> Option<&str> {
        self.selected_leaf.as_deref()
    }
    pub fn file(&self) -> &HistoryFile {
        &self.file
    }
    pub fn source(&self) -> HistorySource {
        self.source.clone()
    }
    pub(crate) fn identity(&self) -> String {
        hex::encode(Sha256::digest(
            format!("{}:{}", self.provider().label(), self.session).as_bytes(),
        ))
    }
    pub(crate) fn verify_source(&self, cancel: &CancellationToken) -> WorkspaceResult<()> {
        check_cancel(cancel)?;
        let data = self.source.fs.read_blob(&self.file.relative_path)?;
        if hex::encode(Sha256::digest(&data)) != self.digest {
            return Err(invalid(
                "Source history changed after preview. Review it again. Nothing was imported.",
            ));
        }
        check_cancel(cancel)
    }
}
#[derive(Clone, Debug)]
pub struct HistoryImportReview {
    pub(crate) preview: HistoryPreview,
    pub(crate) project: Project,
    pub(crate) location: WorkspaceLocation,
    pub(crate) working_directory: PathBuf,
    pub(crate) agent_id: String,
    pub(crate) agent_name: String,
    pub(crate) prior: Option<HistoryImportReceipt>,
}
impl HistoryImportReview {
    pub fn preview(&self) -> &HistoryPreview {
        &self.preview
    }
    pub fn destination(&self) -> &Project {
        &self.project
    }
    pub fn directory(&self) -> &Path {
        &self.working_directory
    }
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }
    pub fn prior(&self) -> Option<&HistoryImportReceipt> {
        self.prior.as_ref()
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryImportReceipt {
    pub identity: String,
    pub sha256: String,
    pub provider: HistoryProvider,
    pub session_id: Uuid,
    pub leaf: Option<String>,
    pub task: synara_core::TaskId,
    pub project: ProjectId,
    pub imported_at_ms: i64,
    pub message_count: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HistoryImportLedger {
    pub version: u32,
    pub receipts: Vec<HistoryImportReceipt>,
}
impl Default for HistoryImportLedger {
    fn default() -> Self {
        Self {
            version: 1,
            receipts: vec![],
        }
    }
}
impl HistoryImportLedger {
    pub(crate) fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1 || self.receipts.len() > 4096 {
            return Err(StorageError::Limit.into());
        }
        let mut ids = HashSet::new();
        for receipt in &self.receipts {
            if !ids.insert(&receipt.identity)
                || receipt.identity
                    != hex::encode(Sha256::digest(
                        format!("{}:{}", receipt.provider.label(), receipt.session_id).as_bytes(),
                    ))
                || receipt.sha256.len() != 64
                || !receipt.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                || receipt.message_count == 0
                || receipt.message_count > MAX_MESSAGES
                || receipt
                    .leaf
                    .as_ref()
                    .is_some_and(|leaf| Uuid::parse_str(leaf).is_err())
            {
                return Err(StorageError::Identity.into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub enum HistoryImportOutcome {
    Imported(Task),
    AlreadyImported {
        task: Option<Task>,
        source_changed: bool,
    },
}
fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
fn check_cancel(cancel: &CancellationToken) -> WorkspaceResult<()> {
    if cancel.is_cancelled() {
        Err(invalid(
            "History operation cancelled. Source files are unchanged.",
        ))
    } else {
        Ok(())
    }
}
fn safe_label(text: &str, maximum: usize) -> String {
    let mut label = String::new();
    let mut length = 0;
    for word in text.split_whitespace() {
        if length == maximum {
            break;
        }
        if !label.is_empty() {
            label.push(' ');
            length += 1;
        }
        for character in word.chars().take(maximum.saturating_sub(length)) {
            label.push(character);
            length += 1;
        }
    }
    label
}
fn session_id(value: &Value) -> WorkspaceResult<Uuid> {
    value
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| invalid("History has no supported session identity."))
}
fn timestamp(value: &Value) -> Option<i64> {
    let text = value.as_str()?;
    if text.len() > 64 {
        return None;
    }
    let millis = chrono::DateTime::parse_from_rfc3339(text)
        .ok()?
        .timestamp_millis();
    (millis >= 0 && millis <= now_ms().saturating_add(86_400_000)).then_some(millis)
}
fn bounded_text(text: &str) -> WorkspaceResult<String> {
    if text.len() > MAX_MESSAGE || text.contains('\0') {
        return Err(invalid("A history message is invalid or exceeds 256 KiB."));
    }
    Ok(text.to_owned())
}
fn text_blocks(
    content: &Value,
    format: HistoryProvider,
    excluded: &mut usize,
) -> WorkspaceResult<String> {
    if let Some(text) = content.as_str() {
        return bounded_text(text);
    }
    let blocks = content
        .as_array()
        .ok_or_else(|| invalid("Unsupported message content. No history was imported."))?;
    let mut text = String::new();
    for block in blocks {
        let kind = block.get("type").and_then(Value::as_str).unwrap_or("");
        let allowed = match format {
            HistoryProvider::Codex => matches!(kind, "input_text" | "output_text"),
            HistoryProvider::Claude => kind == "text",
        };
        if allowed {
            let part = block
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("Text block is malformed."))?;
            if !text.is_empty() {
                text.push('\n');
            }
            if text.len().saturating_add(part.len()) > MAX_MESSAGE {
                return Err(invalid("A history message exceeds 256 KiB."));
            }
            text.push_str(part);
        } else {
            *excluded += 1;
        }
    }
    bounded_text(&text)
}
fn append(messages: &mut Vec<ImportedMessage>, message: ImportedMessage) -> WorkspaceResult<()> {
    if message.text.trim().is_empty() {
        return Ok(());
    }
    if messages.len() == MAX_MESSAGES {
        return Err(invalid(
            "History exceeds 4096 visible messages. Choose a smaller source history.",
        ));
    }
    messages.push(message);
    Ok(())
}
impl HistorySource {
    pub fn root(&self) -> &Path {
        self.fs.root()
    }
    pub fn provider(&self) -> HistoryProvider {
        self.provider
    }
    pub async fn discover(
        root: PathBuf,
        provider: HistoryProvider,
        cancel: CancellationToken,
    ) -> WorkspaceResult<HistoryDiscovery> {
        tokio::task::spawn_blocking(move || {
            check_cancel(&cancel)?;
            if !root.is_absolute() {
                return Err(invalid("Choose an explicit absolute history folder."));
            }
            let fs = Arc::new(WorkspaceFs::open(&root)?);
            let source = Self { provider, fs };
            let mut result = HistoryDiscovery {
                source: source.clone(),
                files: vec![],
                skipped: 0,
                limited: false,
            };
            let mut directories = VecDeque::from([(PathBuf::new(), 0usize)]);
            let mut visited = 0;
            let mut examined = 0usize;
            let deadline = Instant::now() + Duration::from_secs(10);
            while let Some((directory, depth)) = directories.pop_front() {
                check_cancel(&cancel)?;
                if visited >= 256 || Instant::now() >= deadline {
                    result.limited = true;
                    break;
                }
                visited += 1;
                let entries = match source.fs.entries(&directory) {
                    Ok(entries) => entries,
                    Err(_) => {
                        result.skipped += 1;
                        continue;
                    }
                };
                for entry in entries {
                    check_cancel(&cancel)?;
                    if examined == MAX_ENTRIES {
                        result.limited = true;
                        break;
                    }
                    examined += 1;
                    if entry.symlink {
                        result.skipped += 1;
                        continue;
                    }
                    if entry.directory {
                        if matches!(
                            entry.name.as_str(),
                            ".git" | "node_modules" | "subagents" | "agents"
                        ) {
                            continue;
                        }
                        if depth >= 8 || directories.len() >= 256 {
                            result.limited = true;
                            continue;
                        }
                        directories.push_back((entry.relative_path, depth + 1));
                        continue;
                    }
                    if entry
                        .relative_path
                        .extension()
                        .is_none_or(|ext| ext != "jsonl")
                        || (provider == HistoryProvider::Claude && entry.name.starts_with("agent-"))
                    {
                        continue;
                    }
                    let bytes = match source.fs.file_length(&entry.relative_path) {
                        Ok(bytes) if bytes > 0 && bytes <= 8 * 1024 * 1024 => bytes,
                        _ => {
                            result.skipped += 1;
                            continue;
                        }
                    };
                    if result.files.len() == MAX_FILES {
                        result.limited = true;
                        break;
                    }
                    result.files.push(HistoryFile {
                        relative_path: entry.relative_path,
                        bytes,
                    });
                }
                if result.files.len() == MAX_FILES || examined == MAX_ENTRIES {
                    result.limited = true;
                    break;
                }
            }
            result
                .files
                .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
            Ok(result)
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
    }
    pub async fn preview(
        &self,
        file: HistoryFile,
        leaf: Option<String>,
        cancel: CancellationToken,
    ) -> WorkspaceResult<HistoryPreview> {
        let source = self.clone();
        tokio::task::spawn_blocking(move || {
            check_cancel(&cancel)?;
            if file.relative_path.extension().and_then(|s| s.to_str()) != Some("jsonl")
                || source.provider == HistoryProvider::Claude && file.relative_path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("agent-"))
                || source.provider == HistoryProvider::Codex && leaf.is_some() {
                return Err(invalid("Choose a supported main-session JSONL file and branch."));
            }
            let bytes = source.fs.read_blob(&file.relative_path)?;
            let file = HistoryFile { relative_path: file.relative_path, bytes: bytes.len() as u64 };
            let text = std::str::from_utf8(&bytes).map_err(|_| invalid("History is not UTF-8 JSONL."))?;
            let records = text.lines().count();
            if records > MAX_RECORDS { return Err(invalid("History exceeds 20000 records.")); }
            let mut rows = Vec::new();
            for (index, line) in text.lines().enumerate() {
                check_cancel(&cancel)?;
                if line.trim().is_empty() { continue; }
                if line.len() > MAX_LINE { return Err(invalid("A history record exceeds 1 MiB.")); }
                rows.push(serde_json::from_str::<Value>(line).map_err(|_| invalid(&format!("Invalid or incomplete JSONL at line {}. Wait for the source writer to finish and review again.", index + 1)))?);
            }
            let parsed = match source.provider { HistoryProvider::Codex => parse_codex(&rows)?, HistoryProvider::Claude => parse_claude(&rows, leaf.as_deref())? };
            if parsed.messages.is_empty() { return Err(invalid("This source has no supported visible user/assistant text.")); }
            if parsed.messages.iter().try_fold(0usize, |n,m| n.checked_add(m.text.len())).is_none_or(|n| n > MAX_TEXT) {
                return Err(invalid("Visible history text exceeds 4 MiB."));
            }
            check_cancel(&cancel)?;
            let title = safe_label(&parsed.messages.iter().find(|m| m.role == Role::User).unwrap_or(&parsed.messages[0]).text, 80);
            Ok(HistoryPreview { source, file, digest: hex::encode(Sha256::digest(&bytes)), session: parsed.session,
                title, cwd: parsed.cwd, messages: parsed.messages, warnings: parsed.warnings, leaves: parsed.leaves, selected_leaf: parsed.selected_leaf })
        }).await.map_err(|_| WorkspaceError::Worker)?
    }
}
struct Parsed {
    session: Uuid,
    cwd: Option<String>,
    messages: Vec<ImportedMessage>,
    warnings: Vec<String>,
    leaves: Vec<HistoryLeaf>,
    selected_leaf: Option<String>,
}
fn parse_codex(rows: &[Value]) -> WorkspaceResult<Parsed> {
    let mut session = None;
    let mut cwd = None;
    let mut events = vec![];
    let mut responses = vec![];
    let mut excluded = 0;
    let mut compacted = false;
    for row in rows {
        let payload = &row["payload"];
        match row["type"].as_str() {
            Some("session_meta") => {
                let id = session_id(&payload["id"])?;
                if session.is_some_and(|previous| previous != id) {
                    return Err(invalid("Mixed session identities in one source file."));
                }
                session = Some(id);
                cwd = payload["cwd"].as_str().map(|text| safe_label(text, 4096));
            }
            Some("event_msg") => {
                let role = match payload["type"].as_str() {
                    Some("user_message") => Role::User,
                    Some("agent_message") => Role::Assistant,
                    _ => {
                        excluded += 1;
                        continue;
                    }
                };
                let text = payload["message"]
                    .as_str()
                    .ok_or_else(|| invalid("Codex visible-message event is malformed."))?;
                append(
                    &mut events,
                    ImportedMessage {
                        role,
                        text: bounded_text(text)?,
                        timestamp_ms: timestamp(&row["timestamp"]),
                    },
                )?;
            }
            Some("response_item") if payload["type"] == "message" => {
                let role = match payload["role"].as_str() {
                    Some("user") => Role::User,
                    Some("assistant") => Role::Assistant,
                    _ => {
                        excluded += 1;
                        continue;
                    }
                };
                if payload["channel"] == "analysis" {
                    excluded += 1;
                    continue;
                }
                let text = text_blocks(&payload["content"], HistoryProvider::Codex, &mut excluded)?;
                append(
                    &mut responses,
                    ImportedMessage {
                        role,
                        text,
                        timestamp_ms: timestamp(&row["timestamp"]),
                    },
                )?;
            }
            Some("compacted") => {
                compacted = true;
                excluded += 1;
            }
            _ => excluded += 1,
        }
    }
    let mut warnings = vec![format!(
        "{excluded} non-chat records/blocks excluded. No tools, reasoning, permissions, configuration or provider session are imported."
    )];
    let messages = if events.is_empty() {
        warnings.push("This older rollout has no visible-message events. The preview uses response-item user/assistant text, which may include provider-injected user context. Review it before importing.".into());
        responses
    } else {
        warnings.push("Using Codex visible-message events. Corresponding response items are not imported twice.".into());
        events
    };
    if compacted {
        warnings.push("Source contains compaction. Its replacement/hidden context is excluded. Only the visible text retained in this file is imported.".into());
    }
    Ok(Parsed {
        session: session.ok_or_else(|| invalid("No Codex session_meta record found."))?,
        cwd,
        messages,
        warnings,
        leaves: vec![],
        selected_leaf: None,
    })
}
#[derive(Clone, PartialEq)]
struct ClaudeNode {
    id: Uuid,
    parent: Option<Uuid>,
    message: Option<(Role, String)>,
    timestamp: Option<i64>,
}
fn parse_claude(rows: &[Value], selected: Option<&str>) -> WorkspaceResult<Parsed> {
    let mut session = None;
    let mut cwd = None;
    let mut nodes = vec![];
    let mut by_id = HashMap::new();
    let mut excluded = 0usize;
    for row in rows {
        if row["isSidechain"] == true {
            excluded += 1;
            continue;
        }
        let kind = row["type"].as_str().unwrap_or("");
        if !matches!(kind, "user" | "assistant" | "system") {
            excluded += 1;
            continue;
        }
        if let Some(id) = row.get("sessionId").filter(|id| !id.is_null()) {
            let id = session_id(id)?;
            if session.is_some_and(|previous| previous != id) {
                return Err(invalid("Mixed Claude sessions in one source file."));
            }
            session = Some(id);
        }
        if let Some(path) = row["cwd"].as_str() {
            cwd = Some(safe_label(path, 4096));
        }
        let Some(id) = row.get("uuid") else {
            excluded += 1;
            continue;
        };
        let id = session_id(id)?;
        let parent = match row.get("parentUuid") {
            Some(Value::Null) => None,
            Some(value) => Some(session_id(value)?),
            None => {
                return Err(invalid(
                    "Claude record has no branch parent metadata. This format cannot be imported safely yet.",
                ));
            }
        };
        let message = if matches!(kind, "user" | "assistant") && row["isMeta"] != true {
            let role = if kind == "user" {
                Role::User
            } else {
                Role::Assistant
            };
            if row["message"]["role"]
                .as_str()
                .is_some_and(|inner| inner != kind)
            {
                return Err(invalid("Claude message roles disagree."));
            }
            let text = text_blocks(
                &row["message"]["content"],
                HistoryProvider::Claude,
                &mut excluded,
            )?;
            (!text.trim().is_empty()).then_some((role, text))
        } else {
            excluded += 1;
            None
        };
        let node = ClaudeNode {
            id,
            parent,
            message,
            timestamp: timestamp(&row["timestamp"]),
        };
        if let Some(previous) = by_id.get(&id).copied() {
            if nodes[previous] != node {
                return Err(invalid(
                    "Claude history reuses a record identity with different content.",
                ));
            }
            continue;
        }
        by_id.insert(id, nodes.len());
        nodes.push(node);
    }
    let parents: HashSet<Uuid> = nodes.iter().filter_map(|node| node.parent).collect();
    let leaves: Vec<_> = nodes
        .iter()
        .filter(|node| !parents.contains(&node.id))
        .collect();
    if leaves.is_empty() || leaves.len() > 256 {
        return Err(invalid(
            "Claude branch graph is empty, cyclic or exceeds 256 leaves.",
        ));
    }
    let selected = selected
        .map(|s| Uuid::parse_str(s).map_err(|_| invalid("Invalid Claude branch selection.")))
        .transpose()?;
    let leaf = match selected {
        Some(id) => leaves
            .iter()
            .find(|node| node.id == id)
            .copied()
            .ok_or_else(|| invalid("The reviewed Claude branch no longer exists."))?,
        None => leaves
            .last()
            .copied()
            .ok_or_else(|| invalid("No Claude branch available."))?,
    };
    let selected_leaf = Some(leaf.id.to_string());
    let labels = leaves
        .iter()
        .map(|node| HistoryLeaf {
            id: node.id.to_string(),
            label: node
                .message
                .as_ref()
                .map(|(_, text)| safe_label(text, 80))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Non-text branch ending".into()),
        })
        .collect();
    let mut chain = vec![];
    let mut seen = HashSet::new();
    let mut cursor = Some(leaf.id);
    while let Some(id) = cursor {
        if !seen.insert(id) {
            return Err(invalid("Claude history contains a parent cycle."));
        }
        let node = by_id.get(&id).map(|index|&nodes[*index]).ok_or_else(||invalid("Selected Claude branch has a missing ancestor. Import requires a complete local branch chain."))?;
        chain.push(node);
        cursor = node.parent;
    }
    let mut messages = vec![];
    for node in chain.into_iter().rev() {
        if let Some((role, text)) = &node.message {
            append(
                &mut messages,
                ImportedMessage {
                    role: *role,
                    text: text.clone(),
                    timestamp_ms: node.timestamp,
                },
            )?;
        }
    }
    let warnings = vec![
        format!(
            "{} branch ending(s) found. Only the selected branch's ancestor chain is imported. The default is the last ending in file order, not a merge of branches.",
            leaves.len()
        ),
        format!(
            "{excluded} non-chat records/blocks excluded. Subagent sidechains, tools, reasoning, images, permissions and provider sessions are not imported."
        ),
    ];
    Ok(Parsed {
        session: session.ok_or_else(|| invalid("No Claude session identity found."))?,
        cwd,
        messages,
        warnings,
        leaves: labels,
        selected_leaf,
    })
}
impl WorkspaceService {
    pub async fn review_history_import(
        &self,
        preview: HistoryPreview,
        project: ProjectId,
        agent_id: String,
    ) -> WorkspaceResult<HistoryImportReview> {
        self.access(move |store| store.review_history_import(preview, project, agent_id))
            .await
    }
    pub async fn import_history(
        &self,
        review: HistoryImportReview,
        cancel: CancellationToken,
    ) -> WorkspaceResult<HistoryImportOutcome> {
        // Source I/O is completed before acquiring the shared storage owner.
        let checked = review.clone();
        let check_token = cancel.clone();
        tokio::task::spawn_blocking(move || checked.preview.verify_source(&check_token))
            .await
            .map_err(|_| WorkspaceError::Worker)??;
        self.access(move |store| store.import_history(review, &cancel))
            .await
    }
}
#[cfg(test)]
mod tests;
