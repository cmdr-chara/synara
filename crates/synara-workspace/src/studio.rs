//! Read-only Studio outputs and previews. Filesystem reads retain WorkspaceFs's
//! handle-relative containment. Merely browsing never runs tools or writes files.
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
use synara_core::{TaskId, TaskScope, ToolOutput, ToolStatus, WorkspaceLocation};
use synara_runtime::{RuntimeError, WorkspaceFs};

mod export;
pub use export::*;
mod pdf;
pub use pdf::*;

static PREVIEWS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
const MAX_FILES: usize = 500;
const MAX_ENTRIES: usize = 10_000;
const MAX_DEPTH: usize = 8;
const MAX_PREVIEW_TEXT: usize = 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 16_000_000;
const MAX_STUDIO_TEXT_VERSIONS: usize = 12;
const MAX_STUDIO_TEXT_VERSION_BYTES: usize = 128 * 1024;
const MAX_STUDIO_TEXT_VERSION_TOTAL_BYTES: usize = 1024 * 1024;

/// Reporting turn reconstructed from durable events. File contents remain current.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioOutputTurn {
    pub number: usize,
    pub started_at_ms: i64,
}
#[derive(Clone, Debug)]
struct ReportedOutput {
    task: TaskId,
    turn: Option<StudioOutputTurn>,
    timestamp_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioFile {
    pub path: PathBuf,
    pub bytes: u64,
    /// A completed tool reported this path as a changed file. This is not a
    /// claim about files with no durable attribution.
    pub reported_output: bool,
    pub source_task: Option<TaskId>,
    pub source_turn: Option<StudioOutputTurn>,
    pub reported_at_ms: Option<i64>,
}
#[derive(Clone, Debug, Default)]
pub struct StudioFiles {
    pub entries: Vec<StudioFile>,
    pub limited: bool,
    pub unreadable: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewImageFormat {
    Png,
    Jpeg,
}
#[derive(Clone, Debug)]
pub enum StudioPreview {
    Text {
        text: String,
        markdown: bool,
    },
    DocumentText {
        text: String,
    },
    Image {
        bytes: Vec<u8>,
        format: PreviewImageFormat,
        width: u32,
        height: u32,
    },
    Unsupported {
        bytes: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudioTextVersion {
    pub task: TaskId,
    pub path: PathBuf,
    pub text: String,
    pub captured_at_ms: i64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub source_task: Option<TaskId>,
    #[serde(default)]
    pub source_turn: Option<StudioOutputTurn>,
    #[serde(default)]
    pub reported_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredStudioTextVersions {
    version: u32,
    entries: Vec<StudioTextVersion>,
}
impl Default for StoredStudioTextVersions {
    fn default() -> Self {
        Self {
            version: 1,
            entries: Vec::new(),
        }
    }
}
impl StoredStudioTextVersions {
    fn validate(&self, task: TaskId) -> WorkspaceResult<()> {
        let total = self
            .entries
            .iter()
            .try_fold(0usize, |total, entry| total.checked_add(entry.text.len()))
            .ok_or_else(|| {
                WorkspaceError::Invalid("Studio version history is too large.".into())
            })?;
        if self.version != 1
            || self.entries.len() > MAX_STUDIO_TEXT_VERSIONS
            || total > MAX_STUDIO_TEXT_VERSION_TOTAL_BYTES
            || self.entries.iter().any(|entry| {
                entry.task != task
                    || !visible(&entry.path)
                    || entry.text.len() > MAX_STUDIO_TEXT_VERSION_BYTES
                    || entry.captured_at_ms <= 0
                    || entry.source_task.is_some() != entry.source_turn.is_some()
                    || entry.reported_at_ms.is_some_and(|timestamp| timestamp < 0)
            })
        {
            return Err(WorkspaceError::Invalid(
                "Stored Studio version history is invalid or exceeds its bounds.".into(),
            ));
        }
        Ok(())
    }
}
fn studio_versions_key(task: TaskId) -> String {
    format!("task-studio-versions:{task}")
}

fn visible(path: &Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    if text.is_empty() || text.contains('\\') || text.chars().any(char::is_control) {
        return false;
    }
    let mut segments = Vec::new();
    for component in path.components() {
        let Component::Normal(name) = component else {
            return false;
        };
        let Some(name) = name.to_str() else {
            return false;
        };
        let lower = name.to_lowercase();
        if name.starts_with('.') || lower == "node_modules" {
            return false;
        }
        segments.push(lower);
    }
    !segments.is_empty()
        && !matches!(
            segments[0].as_str(),
            "tmp" | "logs" | "inbox" | "context" | "skills"
        )
        && !(segments.len() == 1 && matches!(segments[0].as_str(), "agents.md" | "claude.md"))
}
fn scan(root: &Path, reported: HashMap<PathBuf, ReportedOutput>) -> WorkspaceResult<StudioFiles> {
    let fs = WorkspaceFs::open(root)?;
    let mut pending = vec![(PathBuf::new(), 0)];
    let mut result = StudioFiles::default();
    let mut visited = 0;
    let deadline = Instant::now() + Duration::from_secs(3);
    'scan: while let Some((directory, depth)) = pending.pop() {
        let entries = match fs.entries(&directory) {
            Ok(entries) => entries,
            Err(error) if directory.as_os_str().is_empty() => return Err(error.into()),
            Err(_) => {
                result.unreadable += 1;
                continue;
            }
        };
        for entry in entries {
            visited += 1;
            if visited > MAX_ENTRIES
                || result.entries.len() >= MAX_FILES
                || Instant::now() >= deadline
            {
                result.limited = true;
                break 'scan;
            }
            if entry.symlink || !visible(&entry.relative_path) {
                continue;
            }
            if entry.directory {
                if depth < MAX_DEPTH {
                    pending.push((entry.relative_path, depth + 1));
                } else {
                    result.limited = true;
                }
            } else {
                match fs.file_length(&entry.relative_path) {
                    Ok(bytes) => result.entries.push(StudioFile {
                        reported_output: reported.contains_key(&entry.relative_path),
                        source_task: reported.get(&entry.relative_path).map(|output| output.task),
                        source_turn: reported
                            .get(&entry.relative_path)
                            .and_then(|output| output.turn.clone()),
                        reported_at_ms: reported
                            .get(&entry.relative_path)
                            .map(|output| output.timestamp_ms),
                        path: entry.relative_path,
                        bytes,
                    }),
                    Err(_) => result.unreadable += 1,
                }
            }
        }
    }
    result.entries.sort_by(|a, b| {
        b.reported_output
            .cmp(&a.reported_output)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(result)
}
pub(crate) fn image_size(bytes: &[u8]) -> Option<(PreviewImageFormat, u32, u32)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        && bytes.get(12..16) == Some(b"IHDR")
        && bytes.len() >= 24
    {
        return Some((
            PreviewImageFormat::Png,
            u32::from_be_bytes(bytes[16..20].try_into().ok()?),
            u32::from_be_bytes(bytes[20..24].try_into().ok()?),
        ));
    }
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut offset = 2usize;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            return None;
        }
        while bytes.get(offset) == Some(&0xff) {
            offset += 1;
        }
        let marker = *bytes.get(offset)?;
        offset += 1;
        if matches!(marker, 0xd9 | 0xda) {
            return None;
        }
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        let length = u16::from_be_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?) as usize;
        if length < 2 || offset.checked_add(length)? > bytes.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if length < 8 {
                return None;
            }
            let height = u16::from_be_bytes(bytes[offset + 3..offset + 5].try_into().ok()?) as u32;
            let width = u16::from_be_bytes(bytes[offset + 5..offset + 7].try_into().ok()?) as u32;
            return Some((PreviewImageFormat::Jpeg, width, height));
        }
        offset += length;
    }
    None
}
fn preview(root: &Path, path: &Path) -> WorkspaceResult<StudioPreview> {
    if !visible(path) {
        return Err(
            RuntimeError::Denied("This path is not a visible Studio output.".into()).into(),
        );
    }
    let fs = WorkspaceFs::open(root)?;
    let size = fs.file_length(path)?;
    if size > 8 * 1024 * 1024 {
        return Ok(StudioPreview::Unsupported { bytes: size });
    }
    let bytes = fs.read_blob(path)?;
    if bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        let (bytes, width, height) = crate::storage::still_webp_preview(&bytes)?;
        return Ok(StudioPreview::Image {
            bytes,
            format: PreviewImageFormat::Png,
            width,
            height,
        });
    }
    if let Some((format, width, height)) = image_size(&bytes) {
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS
        {
            return Err(WorkspaceError::Invalid(
                "Image dimensions exceed the safe preview limit.".into(),
            ));
        }
        return Ok(StudioPreview::Image {
            bytes,
            format,
            width,
            height,
        });
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if matches!(
        extension.as_str(),
        "docx" | "odt" | "odp" | "ods" | "pptx" | "xlsx"
    ) {
        if bytes.len() > crate::MAX_ATTACHMENT_BATCH_BYTES {
            return Ok(StudioPreview::Unsupported { bytes: size });
        }
        let text = match extension.as_str() {
            "docx" => crate::storage::docx_text(&bytes)?,
            "odt" => crate::storage::odt_text(&bytes)?,
            "odp" => crate::storage::odp_text(&bytes)?,
            "ods" => crate::storage::ods_text(&bytes)?,
            "pptx" => crate::storage::pptx_text(&bytes)?,
            "xlsx" => crate::storage::xlsx_text(&bytes)?,
            _ => unreachable!("document extension matched above"),
        };
        return Ok(StudioPreview::DocumentText { text });
    }
    if matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "pdf" | "mp4"
    ) {
        return Ok(StudioPreview::Unsupported { bytes: size });
    }
    if bytes.len() <= MAX_PREVIEW_TEXT && !bytes.contains(&0) {
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        if let Ok(text) = std::str::from_utf8(bytes) {
            return Ok(StudioPreview::Text {
                text: text.to_owned(),
                markdown: matches!(extension.as_str(), "md" | "markdown"),
            });
        }
    }
    Ok(StudioPreview::Unsupported { bytes: size })
}
impl WorkspaceService {
    async fn local_studio_root(&self, id: TaskId) -> WorkspaceResult<PathBuf> {
        let task = self.task(id).await?;
        if task.scope != TaskScope::Studio {
            return Err(WorkspaceError::Invalid("Select a Hub thread first.".into()));
        }
        let workspace = self.workspace_for_task(&task).await?;
        if !matches!(workspace.location, WorkspaceLocation::Local { .. }) {
            return Err(WorkspaceError::Invalid(
                "Library previews currently require a local workspace.".into(),
            ));
        }
        Ok(task.working_directory)
    }
    pub async fn studio_files(&self, id: TaskId) -> WorkspaceResult<StudioFiles> {
        let root = self.local_studio_root(id).await?;
        let task = self.task(id).await?;
        // Inspect only Hub threads in the exact same working directory. A shared
        // project ID is never permission to relabel paths from another worktree.
        let mut peers: Vec<_> = self
            .catalog()
            .await?
            .tasks
            .into_iter()
            .filter(|peer| {
                peer.project_id == task.project_id
                    && peer.scope == TaskScope::Studio
                    && peer.working_directory == root
            })
            .collect();
        peers.sort_by_key(|peer| (std::cmp::Reverse(peer.updated_at_ms), peer.id));
        let attribution_limited = peers.len() > 64;
        let mut paths = Vec::new();
        let mut remaining = 4096usize;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut limited = attribution_limited;
        for peer in peers.into_iter().take(64) {
            if remaining == 0 {
                limited = true;
                break;
            }
            let thread = match tokio::time::timeout_at(deadline, self.thread(peer.thread_id)).await
            {
                Ok(Ok(thread)) => thread,
                Ok(Err(_)) | Err(_) => {
                    limited = true;
                    break;
                }
            };
            for tool in thread
                .tools
                .values()
                .filter(|tool| tool.status == ToolStatus::Completed)
            {
                let origin = thread.tool_output_origins.get(&tool.id);
                let turn = origin
                    .and_then(|origin| origin.turn_index)
                    .and_then(|index| {
                        thread.turns.get(index).map(|turn| StudioOutputTurn {
                            number: index + 1,
                            started_at_ms: turn.started_at_ms,
                        })
                    });
                for path in tool
                    .output
                    .iter()
                    .filter_map(|output| match output {
                        ToolOutput::Diff { path, .. } => Some(PathBuf::from(path)),
                        _ => None,
                    })
                    .take(remaining)
                {
                    paths.push((
                        path,
                        ReportedOutput {
                            task: peer.id,
                            turn: turn.clone(),
                            timestamp_ms: origin.map_or(i64::MIN, |origin| origin.timestamp_ms),
                        },
                    ));
                    remaining -= 1;
                }
                if remaining == 0 {
                    limited = true;
                    break;
                }
            }
        }
        tokio::task::spawn_blocking(move || {
            let fs = WorkspaceFs::open(&root)?;
            let mut reported: HashMap<PathBuf, ReportedOutput> = HashMap::new();
            for (path, output) in paths {
                if let Ok(path) = fs.relative(&path)
                    && visible(&path)
                {
                    // A recently opened chat does not outrank a newer actual file report.
                    let replace = reported.get(&path).is_none_or(|old| {
                        (output.timestamp_ms, output.task) > (old.timestamp_ms, old.task)
                    });
                    if replace {
                        reported.insert(path, output);
                    }
                }
            }
            let mut listing = scan(&root, reported)?;
            listing.limited |= limited;
            Ok(listing)
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
    }
    pub async fn studio_preview(
        &self,
        id: TaskId,
        path: PathBuf,
    ) -> WorkspaceResult<StudioPreview> {
        let root = self.local_studio_root(id).await?;
        let permit = tokio::time::timeout(Duration::from_secs(3), PREVIEWS.acquire())
            .await
            .map_err(|_| {
                WorkspaceError::Invalid("Another preview is still decoding. Try again.".into())
            })?
            .map_err(|_| WorkspaceError::Worker)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            preview(&root, &path)
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
    }

    async fn capture_studio_text_version_record(
        &self,
        id: TaskId,
        path: PathBuf,
        text: String,
        source_task: Option<TaskId>,
        source_turn: Option<StudioOutputTurn>,
        reported_at_ms: Option<i64>,
    ) -> WorkspaceResult<(Vec<StudioTextVersion>, bool)> {
        self.local_studio_root(id).await?;
        if !visible(&path)
            || text.len() > MAX_STUDIO_TEXT_VERSION_BYTES
            || source_task.is_some() != source_turn.is_some()
            || reported_at_ms.is_some_and(|timestamp| timestamp < 0)
        {
            return Err(WorkspaceError::Invalid(
                "This Studio text preview is outside the durable version-history bounds.".into(),
            ));
        }
        self.access(move |store| {
            store.task(id)?.ok_or(WorkspaceError::NotFound)?;
            let key = studio_versions_key(id);
            let mut stored = store
                .preference::<StoredStudioTextVersions>(&key)?
                .unwrap_or_default();
            stored.validate(id)?;
            let changed = stored
                .entries
                .iter()
                .rev()
                .find(|entry| entry.path == path)
                .is_none_or(|entry| entry.text != text);
            if changed {
                while stored.entries.len().saturating_add(1) > MAX_STUDIO_TEXT_VERSIONS
                    || stored
                        .entries
                        .iter()
                        .map(|entry| entry.text.len())
                        .sum::<usize>()
                        .saturating_add(text.len())
                        > MAX_STUDIO_TEXT_VERSION_TOTAL_BYTES
                {
                    let Some(index) = stored.entries.iter().position(|entry| !entry.pinned) else {
                        return Err(WorkspaceError::Invalid(
                            "Pinned Studio versions fill the retention budget. Unpin or clear a version before capturing another."
                                .into(),
                        ));
                    };
                    stored.entries.remove(index);
                }
                stored.entries.push(StudioTextVersion {
                    task: id,
                    path: path.clone(),
                    text,
                    captured_at_ms: chrono::Utc::now().timestamp_millis(),
                    pinned: false,
                    source_task,
                    source_turn,
                    reported_at_ms,
                });
                stored.validate(id)?;
                store.set_preference(&key, &stored)?;
            }
            Ok((
                stored
                    .entries
                    .into_iter()
                    .filter(|entry| entry.path == path)
                    .collect(),
                changed,
            ))
        })
        .await
    }

    /// Persist only a bounded text preview already selected and read through the
    /// Studio filesystem boundary. Source files are never written by this ledger.
    pub async fn capture_studio_text_version(
        &self,
        id: TaskId,
        path: PathBuf,
        text: String,
    ) -> WorkspaceResult<Vec<StudioTextVersion>> {
        self.capture_studio_text_version_record(id, path, text, None, None, None)
            .await
            .map(|(versions, _)| versions)
    }

    /// Snapshot current bounded UTF-8 files that completed tools reported for this
    /// Hub. Attribution comes from durable tool/turn events, not file timestamps.
    pub async fn capture_reported_studio_text_versions(
        &self,
        id: TaskId,
    ) -> WorkspaceResult<usize> {
        let root = self.local_studio_root(id).await?;
        let mut candidates: Vec<_> = self
            .studio_files(id)
            .await?
            .entries
            .into_iter()
            .filter(|entry| {
                entry.reported_output
                    && entry.source_task.is_some()
                    && entry.source_turn.is_some()
                    && entry.bytes > 0
                    && entry.bytes <= MAX_STUDIO_TEXT_VERSION_BYTES as u64
            })
            .collect();
        candidates.sort_by_key(|entry| {
            (
                std::cmp::Reverse(entry.reported_at_ms.unwrap_or(i64::MIN)),
                entry.path.clone(),
            )
        });
        candidates.truncate(16);

        let records = tokio::task::spawn_blocking(move || {
            let fs = WorkspaceFs::open(&root)?;
            let mut records = Vec::new();
            for entry in candidates {
                let bytes = match fs.read_blob(&entry.path) {
                    Ok(bytes) if bytes.len() <= MAX_STUDIO_TEXT_VERSION_BYTES => bytes,
                    Ok(_) | Err(_) => continue,
                };
                if bytes.contains(&0) {
                    continue;
                }
                let Ok(text) = String::from_utf8(bytes) else {
                    continue;
                };
                records.push((
                    entry.path,
                    text,
                    entry.source_task,
                    entry.source_turn,
                    entry.reported_at_ms,
                ));
            }
            Ok::<_, WorkspaceError>(records)
        })
        .await
        .map_err(|_| WorkspaceError::Worker)??;

        let mut captured = 0usize;
        for (path, text, source_task, source_turn, reported_at_ms) in records {
            let (_, changed) = self
                .capture_studio_text_version_record(
                    id,
                    path,
                    text,
                    source_task,
                    source_turn,
                    reported_at_ms,
                )
                .await?;
            captured = captured.saturating_add(usize::from(changed));
        }
        Ok(captured)
    }

    /// Pin or unpin one exact durable snapshot. Pinning affects only automatic
    /// bounded retention; explicit clear remains an explicit destructive action.
    pub async fn set_studio_text_version_pinned(
        &self,
        snapshot: StudioTextVersion,
        pinned: bool,
    ) -> WorkspaceResult<Vec<StudioTextVersion>> {
        self.local_studio_root(snapshot.task).await?;
        if !visible(&snapshot.path)
            || snapshot.text.len() > MAX_STUDIO_TEXT_VERSION_BYTES
            || snapshot.captured_at_ms <= 0
        {
            return Err(WorkspaceError::Invalid(
                "This Studio text version is outside the pinning bounds.".into(),
            ));
        }
        self.access(move |store| {
            store.task(snapshot.task)?.ok_or(WorkspaceError::NotFound)?;
            let key = studio_versions_key(snapshot.task);
            let mut stored = store
                .preference::<StoredStudioTextVersions>(&key)?
                .unwrap_or_default();
            stored.validate(snapshot.task)?;
            let Some(entry) = stored.entries.iter_mut().find(|entry| **entry == snapshot) else {
                return Err(WorkspaceError::Invalid(
                    "The selected Studio version changed or was removed. Refresh and choose it again."
                        .into(),
                ));
            };
            entry.pinned = pinned;
            stored.validate(snapshot.task)?;
            store.set_preference(&key, &stored)?;
            Ok(stored
                .entries
                .into_iter()
                .filter(|entry| entry.path == snapshot.path)
                .collect())
        })
        .await
    }

    /// Export one exact durable text snapshot without reading or changing the
    /// current workspace file. A removed/evicted/stale snapshot fails closed.
    pub async fn export_studio_text_version(
        &self,
        snapshot: StudioTextVersion,
        destination: PathBuf,
    ) -> WorkspaceResult<()> {
        self.local_studio_root(snapshot.task).await?;
        if !visible(&snapshot.path)
            || snapshot.text.len() > MAX_STUDIO_TEXT_VERSION_BYTES
            || snapshot.captured_at_ms <= 0
        {
            return Err(WorkspaceError::Invalid(
                "This Studio text version is outside the export bounds.".into(),
            ));
        }
        let bytes = self
            .access(move |store| {
                store.task(snapshot.task)?.ok_or(WorkspaceError::NotFound)?;
                let stored = store
                    .preference::<StoredStudioTextVersions>(&studio_versions_key(snapshot.task))?
                    .unwrap_or_default();
                stored.validate(snapshot.task)?;
                if !stored.entries.contains(&snapshot) {
                    return Err(WorkspaceError::Invalid(
                        "The selected Studio version changed or was removed. Refresh and choose it again."
                            .into(),
                    ));
                }
                Ok(snapshot.text.into_bytes())
            })
            .await?;
        tokio::task::spawn_blocking(move || crate::storage::write_new_export(&destination, &bytes))
            .await
            .map_err(|_| WorkspaceError::Worker)??;
        Ok(())
    }

    /// Explicitly clear durable text-preview history for one visible Studio
    /// path. The workspace file itself is never read, written or removed here.
    pub async fn clear_studio_text_versions(
        &self,
        id: TaskId,
        path: PathBuf,
        confirmed: bool,
    ) -> WorkspaceResult<usize> {
        if !confirmed {
            return Err(WorkspaceError::Invalid(
                "Clearing Studio preview history requires explicit confirmation.".into(),
            ));
        }
        self.local_studio_root(id).await?;
        if !visible(&path) {
            return Err(WorkspaceError::Invalid(
                "This path is not a visible Studio output.".into(),
            ));
        }
        self.access(move |store| {
            store.task(id)?.ok_or(WorkspaceError::NotFound)?;
            let key = studio_versions_key(id);
            let mut stored = store
                .preference::<StoredStudioTextVersions>(&key)?
                .unwrap_or_default();
            stored.validate(id)?;
            let before = stored.entries.len();
            stored.entries.retain(|entry| entry.path != path);
            let removed = before.saturating_sub(stored.entries.len());
            if removed > 0 {
                stored.validate(id)?;
                store.set_preference(&key, &stored)?;
            }
            Ok(removed)
        })
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_paths_exclude_infrastructure_and_hostile_segments() {
        for path in [
            "",
            "../outside",
            "/tmp/a",
            ".git/config",
            "logs/a",
            "SKILLS/a",
            "inbox/a",
            "AGENTS.md",
            "a/node_modules/x",
            "a\\b",
            "x\ny",
        ] {
            assert!(!visible(Path::new(path)), "{path}");
        }
        for path in ["report.md", "images/日本語.png", "a/b.txt"] {
            assert!(visible(Path::new(path)));
        }
    }
    #[tokio::test]
    async fn studio_listing_preview_and_attribution_restore_without_autostart() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("report.md"), "# Caffè\n\nExact text  \n").unwrap();
        std::fs::create_dir(dir.path().join("inbox")).unwrap();
        std::fs::write(dir.path().join("inbox/private.txt"), "private").unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_scoped_task_with_draft(
                project.id,
                "Studio".into(),
                agent,
                TaskScope::Studio,
                "draft".into(),
            )
            .await
            .unwrap();
        let list = service.studio_files(task.id).await.unwrap();
        assert_eq!(list.entries.len(), 1);
        assert!(!list.entries[0].reported_output);
        assert!(matches!(
            service
                .studio_preview(task.id, "report.md".into())
                .await
                .unwrap(),
            StudioPreview::Text { markdown: true, .. }
        ));
        assert!(
            service
                .studio_preview(task.id, "../escape".into())
                .await
                .is_err()
        );
        assert_eq!(service.task_draft(task.id).await.unwrap(), "draft");
        assert!(service.session(task.thread_id).await.unwrap().is_none());
        assert!(
            service
                .thread(task.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
    }
    #[tokio::test]
    async fn studio_text_versions_survive_restart_dedupe_and_stay_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("studio");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("report.md"), "one").unwrap();
        let database = dir.path().join("workspace.sqlite3");
        let service = WorkspaceService::open(database.clone()).await.unwrap();
        let project = service.add_local_workspace(root.clone()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_scoped_task_with_draft(
                project.id,
                "Studio".into(),
                agent,
                TaskScope::Studio,
                "draft".into(),
            )
            .await
            .unwrap();

        assert_eq!(
            service
                .capture_studio_text_version(task.id, "report.md".into(), "one".into())
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            service
                .capture_studio_text_version(task.id, "report.md".into(), "one".into())
                .await
                .unwrap()
                .len(),
            1
        );
        let versions = service
            .capture_studio_text_version(task.id, "report.md".into(), "two".into())
            .await
            .unwrap();
        assert_eq!(versions.len(), 2);
        drop(service);

        let service = WorkspaceService::open(database).await.unwrap();
        let versions = service
            .capture_studio_text_version(task.id, "report.md".into(), "two".into())
            .await
            .unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].text, "one");
        assert_eq!(versions[1].text, "two");

        let old_snapshot = versions[0].clone();
        let versions = service
            .set_studio_text_version_pinned(old_snapshot.clone(), true)
            .await
            .unwrap();
        assert!(versions[0].pinned);
        let old_snapshot = versions[0].clone();

        let exported = dir.path().join("report-version.md");
        service
            .export_studio_text_version(old_snapshot.clone(), exported.clone())
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&exported).unwrap(), "one");
        std::fs::write(&exported, "user file").unwrap();
        assert!(
            service
                .export_studio_text_version(old_snapshot.clone(), exported.clone())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&exported).unwrap(), "user file");

        let payload = "x".repeat(96 * 1024);
        let mut versions = Vec::new();
        for index in 0..20 {
            versions = service
                .capture_studio_text_version(
                    task.id,
                    "report.md".into(),
                    format!("{index:02}{payload}"),
                )
                .await
                .unwrap();
        }
        assert!(versions.len() <= MAX_STUDIO_TEXT_VERSIONS);
        assert!(
            versions.iter().map(|entry| entry.text.len()).sum::<usize>()
                <= MAX_STUDIO_TEXT_VERSION_TOTAL_BYTES
        );
        assert!(versions.iter().any(|entry| {
            entry.captured_at_ms == old_snapshot.captured_at_ms
                && entry.text == old_snapshot.text
                && entry.pinned
        }));

        let other = service
            .capture_studio_text_version(task.id, "other.md".into(), "other".into())
            .await
            .unwrap();
        assert_eq!(other.len(), 1);
        assert!(
            service
                .clear_studio_text_versions(task.id, "report.md".into(), false)
                .await
                .is_err()
        );
        assert!(
            service
                .clear_studio_text_versions(task.id, "../escape.md".into(), true)
                .await
                .is_err()
        );
        assert!(
            service
                .clear_studio_text_versions(task.id, "report.md".into(), true)
                .await
                .unwrap()
                > 0
        );
        assert_eq!(
            std::fs::read_to_string(root.join("report.md")).unwrap(),
            "one"
        );
        assert_eq!(
            service
                .capture_studio_text_version(task.id, "other.md".into(), "other".into())
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            service
                .capture_studio_text_version(task.id, "report.md".into(), "fresh".into())
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            service
                .export_studio_text_version(
                    old_snapshot.clone(),
                    dir.path().join("stale-report-version.md"),
                )
                .await
                .is_err()
        );
        assert!(
            service
                .set_studio_text_version_pinned(old_snapshot, false)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn reported_text_outputs_capture_turn_attributed_versions_without_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("studio-reported");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("report.md"), "version one").unwrap();

        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(root.clone()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_scoped_task_with_draft(
                project.id,
                "Studio output".into(),
                agent,
                TaskScope::Studio,
                "draft".into(),
            )
            .await
            .unwrap();

        service
            .record(
                task.thread_id,
                synara_core::ThreadEvent::PromptStarted {
                    turn: "turn-1".into(),
                },
            )
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                synara_core::ThreadEvent::ToolChanged {
                    patch: synara_core::ToolPatch {
                        id: "tool-1".into(),
                        title: Some("Write report".into()),
                        status: Some(ToolStatus::Completed),
                        kind: Some("edit".into()),
                        output: Some(vec![ToolOutput::Diff {
                            path: "report.md".into(),
                            before: None,
                            after: Some("version one".into()),
                        }]),
                    },
                },
            )
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                synara_core::ThreadEvent::PromptFinished {
                    reason: "stop".into(),
                },
            )
            .await
            .unwrap();

        assert_eq!(
            service
                .capture_reported_studio_text_versions(task.id)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            service
                .capture_reported_studio_text_versions(task.id)
                .await
                .unwrap(),
            0
        );

        std::fs::write(root.join("report.md"), "version two").unwrap();
        service
            .record(
                task.thread_id,
                synara_core::ThreadEvent::PromptStarted {
                    turn: "turn-2".into(),
                },
            )
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                synara_core::ThreadEvent::ToolChanged {
                    patch: synara_core::ToolPatch {
                        id: "tool-2".into(),
                        title: Some("Rewrite report".into()),
                        status: Some(ToolStatus::Completed),
                        kind: Some("edit".into()),
                        output: Some(vec![ToolOutput::Diff {
                            path: "report.md".into(),
                            before: Some("version one".into()),
                            after: Some("version two".into()),
                        }]),
                    },
                },
            )
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                synara_core::ThreadEvent::PromptFinished {
                    reason: "stop".into(),
                },
            )
            .await
            .unwrap();

        assert_eq!(
            service
                .capture_reported_studio_text_versions(task.id)
                .await
                .unwrap(),
            1
        );
        let versions = service
            .capture_studio_text_version(task.id, "report.md".into(), "version two".into())
            .await
            .unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].text, "version one");
        assert_eq!(versions[1].text, "version two");
        assert_eq!(versions[0].source_task, Some(task.id));
        assert_eq!(versions[1].source_task, Some(task.id));
        assert_eq!(
            versions[0].source_turn.as_ref().map(|turn| turn.number),
            Some(1)
        );
        assert_eq!(
            versions[1].source_turn.as_ref().map(|turn| turn.number),
            Some(2)
        );
        assert!(
            versions
                .iter()
                .all(|version| version.reported_at_ms.is_some())
        );
    }

    #[test]
    fn preview_limits_and_symlinks_keep_filesystem_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        png[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        std::fs::write(dir.path().join("huge.png"), png).unwrap();
        assert!(preview(dir.path(), Path::new("huge.png")).is_err());
        std::fs::write(dir.path().join("binary.dat"), [0, 1, 2]).unwrap();
        assert!(matches!(
            preview(dir.path(), Path::new("binary.dat")).unwrap(),
            StudioPreview::Unsupported { .. }
        ));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc/passwd", dir.path().join("external.txt")).unwrap();
            assert!(preview(dir.path(), Path::new("external.txt")).is_err());
        }
    }

    #[test]
    fn still_webp_preview_decodes_pixels_without_rewriting_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let pixels = [
            230, 40, 10, 255, 0, 150, 30, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let mut webp = Vec::new();
        image::codecs::webp::WebPEncoder::new_lossless(&mut webp)
            .encode(&pixels, 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();
        std::fs::write(dir.path().join("still.webp"), &webp).unwrap();
        let StudioPreview::Image {
            bytes,
            format,
            width,
            height,
        } = preview(dir.path(), Path::new("still.webp")).unwrap()
        else {
            panic!("still WebP must have a native image preview");
        };
        assert_eq!(format, PreviewImageFormat::Png);
        assert_eq!((width, height), (2, 2));
        let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .unwrap()
            .into_rgba8();
        assert_eq!(decoded.as_raw().as_slice(), pixels.as_slice());
        assert_eq!(std::fs::read(dir.path().join("still.webp")).unwrap(), webp);
        std::fs::write(dir.path().join("broken.webp"), b"RIFF0000WEBPinvalid").unwrap();
        assert!(preview(dir.path(), Path::new("broken.webp")).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.path().join("still.webp"), dir.path().join("link.webp"))
                .unwrap();
            assert!(preview(dir.path(), Path::new("link.webp")).is_err());
        }
    }
    #[tokio::test]
    async fn reporting_turn_uses_latest_actual_output_not_recent_chat_activity_and_survives_reopen()
    {
        use synara_core::{ThreadEvent, ToolPatch};
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("studio");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("result.txt"), "current content").unwrap();
        let db = dir.path().join("workspace.sqlite3");
        let service = WorkspaceService::open(db.clone()).await.unwrap();
        let project = service.add_local_workspace(root.clone()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let first = service
            .create_scoped_task(project.id, "First".into(), agent.clone(), TaskScope::Studio)
            .await
            .unwrap();
        let second = service
            .create_scoped_task(project.id, "Second".into(), agent, TaskScope::Studio)
            .await
            .unwrap();
        let report = || ThreadEvent::ToolChanged {
            patch: ToolPatch {
                id: "write".into(),
                status: Some(ToolStatus::Completed),
                output: Some(vec![ToolOutput::Diff {
                    path: "result.txt".into(),
                    before: None,
                    after: Some("reported content".into()),
                }]),
                ..ToolPatch::default()
            },
        };
        for task in [&first, &second] {
            service
                .record(
                    task.thread_id,
                    ThreadEvent::PromptStarted { turn: "one".into() },
                )
                .await
                .unwrap();
            service.record(task.thread_id, report()).await.unwrap();
            service
                .record(
                    task.thread_id,
                    ThreadEvent::PromptFinished {
                        reason: "end_turn".into(),
                    },
                )
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        // The first chat becomes most recently active, but did not replace its file report.
        service
            .record(
                first.thread_id,
                ThreadEvent::PromptStarted { turn: "two".into() },
            )
            .await
            .unwrap();
        service
            .record(
                first.thread_id,
                ThreadEvent::ToolChanged {
                    patch: ToolPatch {
                        id: "write".into(),
                        status: Some(ToolStatus::Completed),
                        title: Some("Status refresh".into()),
                        ..ToolPatch::default()
                    },
                },
            )
            .await
            .unwrap();
        service
            .record(
                first.thread_id,
                ThreadEvent::PromptFinished {
                    reason: "end_turn".into(),
                },
            )
            .await
            .unwrap();
        let listing = service.studio_files(first.id).await.unwrap();
        assert_eq!(listing.entries.len(), 1);
        let file = &listing.entries[0];
        assert!(file.reported_output);
        assert_eq!(file.source_task, Some(second.id));
        assert_eq!(file.source_turn.as_ref().unwrap().number, 1);
        assert!(file.reported_at_ms.is_some());
        drop(service);
        let reopened = WorkspaceService::open(db).await.unwrap();
        assert_eq!(
            reopened.studio_files(second.id).await.unwrap().entries,
            listing.entries
        );
        let StudioPreview::Text { text, .. } = reopened
            .studio_preview(second.id, "result.txt".into())
            .await
            .unwrap()
        else {
            panic!("text");
        };
        assert_eq!(text, "current content");
        assert!(reopened.session(first.thread_id).await.unwrap().is_none());
        assert!(reopened.session(second.thread_id).await.unwrap().is_none());
    }
}
