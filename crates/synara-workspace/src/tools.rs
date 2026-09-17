use crate::{WorkspaceError, WorkspaceResult};
use std::{path::PathBuf, sync::Arc, time::Duration};
use synara_runtime::{
    ExecutionHost, FileEntry, FileSnapshot, FileVersion, LaunchSpec, LocalHost, RuntimeError,
    WorkspaceFs,
};
use tokio::io::AsyncReadExt;

pub const MAX_EDITOR_BYTES: usize = 1024 * 1024;
pub async fn list_files(root: PathBuf, directory: PathBuf) -> WorkspaceResult<Vec<FileEntry>> {
    tokio::task::spawn_blocking(move || WorkspaceFs::open(&root)?.entries(&directory))
        .await
        .map_err(|_| WorkspaceError::Worker)?
        .map_err(Into::into)
}
#[derive(Clone, Debug)]
pub struct Document {
    pub path: PathBuf,
    pub snapshot: FileSnapshot,
}
pub async fn open_document(root: PathBuf, path: PathBuf) -> WorkspaceResult<Document> {
    tokio::task::spawn_blocking(move || {
        let fs = WorkspaceFs::open(&root)?;
        let path = fs.relative(&path)?;
        let snapshot = fs.read(&path)?;
        if snapshot.text.len() > MAX_EDITOR_BYTES {
            return Err(RuntimeError::Unsupported(
                "native editing is limited to 1 MiB per document".into(),
            ));
        }
        Ok(Document { path, snapshot })
    })
    .await
    .map_err(|_| WorkspaceError::Worker)?
    .map_err(Into::into)
}
pub async fn save_document(
    root: PathBuf,
    document: Document,
    text: String,
) -> WorkspaceResult<FileVersion> {
    if text.len() > MAX_EDITOR_BYTES {
        return Err(RuntimeError::Limit.into());
    }
    tokio::task::spawn_blocking(move || {
        WorkspaceFs::open(&root)?.write(
            &document.path,
            &text,
            Some(&document.snapshot.version),
            document.snapshot.utf8_bom,
        )
    })
    .await
    .map_err(|_| WorkspaceError::Worker)?
    .map_err(Into::into)
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitEntry {
    pub path: PathBuf,
    pub original_path: Option<PathBuf>,
    pub index_status: char,
    pub worktree_status: char,
}
impl GitEntry {
    pub fn staged(&self) -> bool {
        !matches!(self.index_status, ' ' | '?')
    }
    pub fn unstaged(&self) -> bool {
        self.worktree_status != ' '
    }
}
#[derive(Clone, Debug, Default)]
pub struct GitStatus {
    pub branch: String,
    pub entries: Vec<GitEntry>,
}
#[derive(Clone)]
pub struct GitService {
    root: PathBuf,
    host: Arc<dyn ExecutionHost>,
}
impl GitService {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            host: Arc::new(LocalHost),
        }
    }
    pub fn with_host(root: PathBuf, host: Arc<dyn ExecutionHost>) -> Self {
        Self { root, host }
    }
    async fn run(&self, args: Vec<String>, max_bytes: usize) -> WorkspaceResult<Vec<u8>> {
        let mut launch = LaunchSpec::new("git");
        launch.args = vec![
            "--no-pager".into(),
            "--literal-pathspecs".into(),
            "-c".into(),
            "core.fsmonitor=false".into(),
            "-c".into(),
            "core.hooksPath=/dev/null".into(),
            "-c".into(),
            "credential.interactive=false".into(),
        ];
        launch.args.extend(args);
        if self.host.is_local() {
            launch.env.insert("GIT_TERMINAL_PROMPT".into(), "0".into());
            launch.env.insert("GIT_OPTIONAL_LOCKS".into(), "0".into());
        }
        let process = self.host.spawn(&launch, &self.root).await?;
        let handle = process.handle.clone();
        let operation = async move {
            let read = async |stream: synara_runtime::ProcessReader,
                              limit: usize|
                   -> WorkspaceResult<Vec<u8>> {
                let mut bytes = vec![];
                stream
                    .take(limit as u64 + 1)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(RuntimeError::Io)?;
                if bytes.len() > limit {
                    return Err(RuntimeError::Limit.into());
                }
                Ok(bytes)
            };
            let (stdout, stderr, status) = tokio::try_join!(
                read(process.stdout, max_bytes),
                read(process.stderr, 64 * 1024),
                async { process.handle.wait().await.map_err(WorkspaceError::from) }
            )?;
            if !status.success() {
                return Err(WorkspaceError::Invalid(format!(
                    "Git failed: {}",
                    String::from_utf8_lossy(&stderr).trim()
                )));
            }
            Ok(stdout)
        };
        match tokio::time::timeout(Duration::from_secs(30), operation).await {
            Ok(Ok(bytes)) => Ok(bytes),
            Ok(Err(error)) => {
                let _ = handle.shutdown().await;
                Err(error)
            }
            Err(_) => {
                let _ = handle.shutdown().await;
                Err(RuntimeError::Timeout.into())
            }
        }
    }
    pub async fn status(&self) -> WorkspaceResult<GitStatus> {
        let bytes = self
            .run(
                vec![
                    "status".into(),
                    "--porcelain=v1".into(),
                    "-z".into(),
                    "--untracked-files=normal".into(),
                ],
                4 * 1024 * 1024,
            )
            .await?;
        let branch = self
            .run(vec!["branch".into(), "--show-current".into()], 8192)
            .await?;
        let branch = String::from_utf8_lossy(&branch).trim().to_owned();
        Ok(GitStatus {
            branch: if branch.is_empty() {
                "Detached HEAD".into()
            } else {
                branch
            },
            entries: parse_status(&bytes)?,
        })
    }
    pub async fn diff(&self, staged: bool, path: Option<PathBuf>) -> WorkspaceResult<String> {
        let mut args = vec![
            "diff".into(),
            "--no-color".into(),
            "--no-ext-diff".into(),
            "--no-textconv".into(),
        ];
        if staged {
            args.push("--cached".into());
        }
        args.push("--".into());
        if let Some(path) = path {
            args.push(self.checked_path(path).await?);
        }
        let bytes = self.run(args, 4 * 1024 * 1024).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
    async fn checked_path(&self, path: PathBuf) -> WorkspaceResult<String> {
        if self.host.is_local() {
            let root = self.root.clone();
            return tokio::task::spawn_blocking(move || {
                let relative = WorkspaceFs::open(&root)?.relative(&path)?;
                relative.into_os_string().into_string().map_err(|_| {
                    RuntimeError::Unsupported("this Git action requires a UTF-8 filename".into())
                })
            })
            .await
            .map_err(|_| WorkspaceError::Worker)?
            .map_err(Into::into);
        }
        if path.is_absolute()
            || path.components().any(|component| {
                !matches!(
                    component,
                    std::path::Component::Normal(_) | std::path::Component::CurDir
                )
            })
        {
            return Err(RuntimeError::Denied(
                "remote Git path must stay inside the selected workspace".into(),
            )
            .into());
        }
        path.into_os_string().into_string().map_err(|_| {
            RuntimeError::Unsupported("remote Git actions require UTF-8 filenames".into()).into()
        })
    }
    pub async fn stage(&self, path: PathBuf) -> WorkspaceResult<()> {
        let path = self.checked_path(path).await?;
        self.run(vec!["add".into(), "--".into(), path], 64 * 1024)
            .await?;
        Ok(())
    }
    pub async fn unstage(&self, path: PathBuf) -> WorkspaceResult<()> {
        let path = self.checked_path(path).await?;
        self.run(
            vec!["restore".into(), "--staged".into(), "--".into(), path],
            64 * 1024,
        )
        .await?;
        Ok(())
    }
    pub async fn commit(&self, message: String) -> WorkspaceResult<()> {
        if message.trim().is_empty() || message.len() > 64 * 1024 || message.contains('\0') {
            return Err(WorkspaceError::Invalid("enter a commit message".into()));
        }
        self.run(
            vec![
                "-c".into(),
                "commit.gpgSign=false".into(),
                "commit".into(),
                "-m".into(),
                message,
            ],
            1024 * 1024,
        )
        .await?;
        Ok(())
    }
}
fn parse_status(bytes: &[u8]) -> WorkspaceResult<Vec<GitEntry>> {
    let mut fields = bytes.split(|b| *b == 0).peekable();
    let mut entries = vec![];
    while let Some(field) = fields.next() {
        if field.is_empty() {
            continue;
        }
        if field.len() < 4 || field[2] != b' ' {
            return Err(WorkspaceError::Invalid(
                "malformed Git status response".into(),
            ));
        }
        let path = path_from_bytes(&field[3..]);
        let original_path = if matches!(field[0], b'R' | b'C') || matches!(field[1], b'R' | b'C') {
            Some(path_from_bytes(
                fields
                    .next()
                    .filter(|f| !f.is_empty())
                    .ok_or_else(|| WorkspaceError::Invalid("incomplete rename record".into()))?,
            ))
        } else {
            None
        };
        if entries.len() >= 20_000 {
            return Err(RuntimeError::Limit.into());
        }
        entries.push(GitEntry {
            path,
            original_path,
            index_status: field[0] as char,
            worktree_status: field[1] as char,
        });
    }
    Ok(entries)
}
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(bytes.to_vec()).into()
    }
    #[cfg(not(unix))]
    {
        String::from_utf8_lossy(bytes).as_ref().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn status_parser_handles_renames_spaces_and_newlines() {
        let entries =
            parse_status(b" M hello world.txt\0R  renamed\nfile.txt\0old.txt\0?? untracked.txt\0")
                .unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].path, Path::new("renamed\nfile.txt"));
        assert_eq!(
            entries[1].original_path.as_deref(),
            Some(Path::new("old.txt"))
        );
        assert!(entries[1].staged());
        assert!(!entries[1].unstaged());
        assert!(parse_status(b"R  new\0").is_err());
    }
    #[tokio::test]
    async fn editor_preserves_bom_and_line_endings_and_rejects_conflicting_save() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("file.txt");
        std::fs::write(&path, b"\xef\xbb\xbfline one\r\nline two\r\n").unwrap();
        let document = open_document(root.path().into(), "file.txt".into())
            .await
            .unwrap();
        save_document(
            root.path().into(),
            document.clone(),
            document.snapshot.text.replace("one", "1"),
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"\xef\xbb\xbfline 1\r\nline two\r\n"
        );
        assert!(matches!(
            save_document(root.path().into(), document, "lost update".into()).await,
            Err(WorkspaceError::Runtime(RuntimeError::Conflict))
        ));
    }
    #[tokio::test]
    async fn git_stage_diff_and_unstage_use_literal_filenames_without_executing_hooks() {
        let root = tempfile::tempdir().unwrap();
        let run = |args: &[&str]| {
            let result = std::process::Command::new("git")
                .args(args)
                .current_dir(root.path())
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@example.invalid"]);
        run(&["config", "user.name", "Test"]);
        std::fs::write(root.path().join("normal.txt"), "before\n").unwrap();
        run(&["add", "normal.txt"]);
        run(&["commit", "-qm", "initial"]);
        std::fs::write(root.path().join("normal.txt"), "after\n").unwrap();
        std::fs::write(root.path().join("[literal].txt"), "literal\n").unwrap();
        let git = GitService::new(root.path().into());
        assert!(
            git.status()
                .await
                .unwrap()
                .entries
                .iter()
                .any(|e| e.path == Path::new("normal.txt"))
        );
        assert!(git.diff(false, None).await.unwrap().contains("+after"));
        git.stage("[literal].txt".into()).await.unwrap();
        assert!(git.diff(true, None).await.unwrap().contains("+literal"));
        git.unstage("[literal].txt".into()).await.unwrap();
        assert!(git.diff(true, None).await.unwrap().is_empty());
        assert!(git.stage("../escape".into()).await.is_err());
    }
}
