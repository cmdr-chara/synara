//! Read-only Studio outputs and previews. Filesystem reads retain WorkspaceFs's
//! handle-relative containment. Merely browsing never runs tools or writes files.
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
use synara_core::{TaskId, TaskScope, ToolOutput, ToolStatus, WorkspaceLocation};
use synara_runtime::{RuntimeError, WorkspaceFs};

const MAX_FILES: usize = 500;
const MAX_ENTRIES: usize = 10_000;
const MAX_DEPTH: usize = 8;
const MAX_PREVIEW_TEXT: usize = 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 16_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioFile {
    pub path: PathBuf,
    pub bytes: u64,
    /// A completed tool reported this path as a changed file. This is not a
    /// claim about files with no durable attribution.
    pub reported_output: bool,
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
fn scan(root: &Path, reported: HashSet<PathBuf>) -> WorkspaceResult<StudioFiles> {
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
                        reported_output: reported.contains(&entry.relative_path),
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
fn image_size(bytes: &[u8]) -> Option<(PreviewImageFormat, u32, u32)> {
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
            return Err(WorkspaceError::Invalid(
                "Select a Studio conversation first.".into(),
            ));
        }
        let workspace = self.workspace_for_task(&task).await?;
        if !matches!(workspace.location, WorkspaceLocation::Local { .. }) {
            return Err(WorkspaceError::Invalid(
                "Studio previews currently require a local workspace.".into(),
            ));
        }
        Ok(task.working_directory)
    }
    pub async fn studio_files(&self, id: TaskId) -> WorkspaceResult<StudioFiles> {
        let root = self.local_studio_root(id).await?;
        let task = self.task(id).await?;
        let thread = self.thread(task.thread_id).await?;
        // Only structured completed diff outputs provide attribution. A prose
        // filename or arbitrary URI never becomes trusted filesystem authority.
        let paths = thread
            .tools
            .values()
            .filter(|t| t.status == ToolStatus::Completed)
            .flat_map(|t| t.output.iter())
            .filter_map(|output| match output {
                ToolOutput::Diff { path, .. } => Some(PathBuf::from(path)),
                _ => None,
            })
            .collect::<Vec<_>>();
        tokio::task::spawn_blocking(move || {
            let fs = WorkspaceFs::open(&root)?;
            let reported = paths
                .iter()
                .filter_map(|path| fs.relative(path).ok())
                .filter(|path| visible(path))
                .collect();
            scan(&root, reported)
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
        tokio::task::spawn_blocking(move || preview(&root, &path))
            .await
            .map_err(|_| WorkspaceError::Worker)?
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
}
