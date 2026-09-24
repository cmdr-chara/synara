//! Committed file history without worktree conversion, checkout or filter execution.
use super::*;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

const MAX_BLAME_LINES: usize = 6000;
const MAX_BLAME_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct GitFileCommit {
    pub id: String,
    pub author: String,
    pub date: Option<String>,
    pub subject: String,
}
#[derive(Clone, Debug)]
pub struct GitFileHistory {
    root: PathBuf,
    path: String,
    pub head: String,
    pub commits: Vec<GitFileCommit>,
    pub limited: bool,
}
#[derive(Clone, Debug)]
pub struct GitFileRevision {
    pub commit: String,
    pub text: String,
}
#[derive(Clone, Debug)]
pub struct GitFileBlameLine {
    pub commit: String,
    pub author: String,
    pub date: Option<String>,
    pub subject: String,
}
#[derive(Clone, Debug)]
pub struct GitFileBlame {
    pub head: String,
    pub commit: String,
    pub lines: Vec<GitFileBlameLine>,
    pub limited: bool,
}
fn object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn label(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(512)
        .collect::<String>()
        .trim()
        .into()
}
fn parse_history(bytes: &[u8]) -> WorkspaceResult<Vec<GitFileCommit>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let invalid = || WorkspaceError::Invalid("Git returned malformed file history.".into());
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let fields: Vec<_> = text
        .strip_suffix('\0')
        .ok_or_else(invalid)?
        .split('\0')
        .collect();
    let (rows, remainder) = fields.as_chunks::<4>();
    if !remainder.is_empty() || rows.len() > 51 {
        return Err(invalid());
    }
    let mut commits = Vec::new();
    for row in rows {
        if !object_id(row[0]) {
            return Err(invalid());
        }
        commits.push(GitFileCommit {
            id: row[0].to_owned(),
            author: label(row[1]),
            subject: label(row[3]),
            date: row[2]
                .parse::<i64>()
                .ok()
                .and_then(|n| chrono::DateTime::from_timestamp(n, 0))
                .map(|date| date.format("%Y-%m-%d %H:%M UTC").to_string()),
        });
    }
    Ok(commits)
}
fn parse_blame(bytes: &[u8], line_count: usize) -> WorkspaceResult<Vec<GitFileBlameLine>> {
    let invalid = || WorkspaceError::Invalid("Git returned malformed line blame.".into());
    if line_count > MAX_BLAME_LINES {
        return Err(invalid());
    }
    if line_count == 0 {
        return if bytes.is_empty() {
            Ok(Vec::new())
        } else {
            Err(invalid())
        };
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let mut rows: Vec<Option<GitFileBlameLine>> = vec![None; line_count];
    let mut metadata_by_commit: HashMap<String, (String, Option<String>, String)> =
        HashMap::new();
    let mut output_lines = text.lines().peekable();
    while let Some(header) = output_lines.next() {
        let fields: Vec<_> = header.split_ascii_whitespace().collect();
        if !(3..=4).contains(&fields.len()) || !object_id(fields[0]) {
            return Err(invalid());
        }
        let original_line = fields[1].parse::<usize>().ok().filter(|line| *line > 0);
        let final_line = fields[2].parse::<usize>().ok().filter(|line| *line > 0);
        let count = fields
            .get(3)
            .map(|value| value.parse::<usize>().ok().filter(|count| *count > 0))
            .unwrap_or(Some(1));
        let (Some(_original_line), Some(final_line), Some(count)) =
            (original_line, final_line, count)
        else {
            return Err(invalid());
        };
        let end = final_line.checked_add(count - 1).ok_or_else(invalid)?;
        if final_line > line_count || end > line_count {
            return Err(invalid());
        }

        let mut author = None;
        let mut author_time = None;
        let mut subject = None;
        let mut found_filename = false;
        for metadata in output_lines.by_ref() {
            if let Some(value) = metadata.strip_prefix("author ") {
                author = Some(label(value));
            } else if let Some(value) = metadata.strip_prefix("author-time ") {
                author_time = value.parse::<i64>().ok();
            } else if let Some(value) = metadata.strip_prefix("summary ") {
                subject = Some(label(value));
            } else if metadata.starts_with("filename ") {
                found_filename = true;
                break;
            }
        }
        if !found_filename {
            return Err(invalid());
        }
        let commit = fields[0].to_owned();
        let (author, date, subject) = if author.is_some() || author_time.is_some() || subject.is_some() {
            let author = author.ok_or_else(invalid)?;
            let subject = subject.ok_or_else(invalid)?;
            let date = author_time
                .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
                .map(|date| date.format("%Y-%m-%d %H:%M UTC").to_string());
            let metadata = (author, date, subject);
            metadata_by_commit.insert(commit.clone(), metadata.clone());
            metadata
        } else {
            metadata_by_commit
                .get(&commit)
                .cloned()
                .ok_or_else(invalid)?
        };
        let row = GitFileBlameLine {
            commit,
            author,
            date,
            subject,
        };
        for slot in rows.iter_mut().take(end).skip(final_line - 1) {
            if slot.replace(row.clone()).is_some() {
                return Err(invalid());
            }
        }
    }
    rows.into_iter().collect::<Option<Vec<_>>>().ok_or_else(invalid)
}
fn history_args(args: Vec<String>) -> Vec<String> {
    // Missing promisor objects must be errors, never an implicit network fetch.
    let mut result = vec![
        "--no-replace-objects".into(),
        "--no-lazy-fetch".into(),
        "-c".into(),
        "protocol.allow=never".into(),
        "-c".into(),
        "credential.helper=".into(),
    ];
    result.extend(args);
    result
}
impl GitService {
    async fn check_history_head(&self, history: &GitFileHistory) -> WorkspaceResult<()> {
        let bytes = self
            .run(
                history_args(vec!["rev-parse".into(), "--verify".into(), "HEAD".into()]),
                256,
            )
            .await?;
        let head = String::from_utf8_lossy(&bytes).trim().to_owned();
        if !object_id(&head) || head != history.head {
            return Err(RuntimeError::Denied(
                "Repository HEAD changed. Reopen file history before requesting line blame."
                    .into(),
            )
            .into());
        }
        Ok(())
    }

    pub async fn file_history(
        &self,
        path: PathBuf,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<GitFileHistory> {
        if !self.host.is_local() {
            return Err(RuntimeError::Unsupported("File history currently requires a local repository. No local fallback was attempted.".into()).into());
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(RuntimeError::Closed.into()),
            result = async {
                let path = self.checked_path(path).await?;
                let bytes = self.run(history_args(vec!["rev-parse".into(), "--verify".into(), "HEAD".into()]), 256).await?;
                let head = String::from_utf8_lossy(&bytes).trim().to_owned();
                if !object_id(&head) { return Err(WorkspaceError::Invalid("This repository has no usable HEAD commit.".into())); }
                let bytes = self.run(history_args(vec!["log".into(), "--no-patch".into(), "--no-notes".into(), "--no-show-signature".into(), "--no-decorate".into(), "--no-renames".into(), "--encoding=UTF-8".into(), "-z".into(), "--format=%H%x00%an%x00%at%x00%s".into(), "--max-count=51".into(), head.clone(), "--".into(), path.clone()]), 256 * 1024).await?;
                let mut commits = parse_history(&bytes)?;
                let limited = commits.len() > 50;
                commits.truncate(50);
                Ok(GitFileHistory { root: self.root.clone(), path, head, commits, limited })
            } => result,
        }
    }
    pub async fn file_revision(
        &self,
        history: &GitFileHistory,
        commit: &str,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<GitFileRevision> {
        if !self.host.is_local()
            || history.root != self.root
            || !object_id(commit)
            || !history.commits.iter().any(|row| row.id == commit)
        {
            return Err(RuntimeError::Denied(
                "Choose a commit from this file's reviewed history.".into(),
            )
            .into());
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(RuntimeError::Closed.into()),
            result = async {
                // Inspect tree mode and immutable blob ID. Never follow a historical symlink.
                let tree = self.run(history_args(vec!["ls-tree".into(), "-z".into(), commit.into(), "--".into(), history.path.clone()]), 8192).await?;
                let invalid = || WorkspaceError::Invalid("This revision has no regular file at the selected path.".into());
                let record = std::str::from_utf8(&tree).map_err(|_| invalid())?.strip_suffix('\0').filter(|s| !s.contains('\0')).ok_or_else(invalid)?;
                let (header, _) = record.split_once('\t').ok_or_else(invalid)?;
                let fields: Vec<_> = header.split_whitespace().collect();
                if fields.len() != 3 || !matches!(fields[0], "100644" | "100755") || fields[1] != "blob" || !object_id(fields[2]) { return Err(invalid()); }
                let id = fields[2].to_owned();
                let size = self.run(history_args(vec!["cat-file".into(), "-s".into(), id.clone()]), 128).await?;
                let size: usize = String::from_utf8_lossy(&size).trim().parse().map_err(|_| invalid())?;
                if size > MAX_EDITOR_BYTES { return Err(RuntimeError::Limit.into()); }
                let bytes = self.run(history_args(vec!["cat-file".into(), "blob".into(), id]), MAX_EDITOR_BYTES).await?;
                if bytes.contains(&0) { return Err(RuntimeError::Unsupported("Binary revisions are not shown as text.".into()).into()); }
                let text = String::from_utf8(bytes).map_err(|_| RuntimeError::Unsupported("This revision is not UTF-8 text.".into()))?;
                Ok(GitFileRevision { commit: commit.into(), text })
            } => result,
        }
    }

    pub async fn file_blame(
        &self,
        history: &GitFileHistory,
        commit: &str,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<GitFileBlame> {
        if !self.host.is_local()
            || history.root != self.root
            || !object_id(commit)
            || !history.commits.iter().any(|row| row.id == commit)
        {
            return Err(RuntimeError::Denied(
                "Choose a commit from this file's reviewed history.".into(),
            )
            .into());
        }
        if history.path.chars().any(char::is_control) {
            return Err(RuntimeError::Unsupported(
                "Line blame requires a path without control characters.".into(),
            )
            .into());
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(RuntimeError::Closed.into()),
            result = async {
                self.check_history_head(history).await?;

                // Validate the immutable tree entry exactly as revision preview does.
                let tree = self.run(history_args(vec!["ls-tree".into(), "-z".into(), commit.into(), "--".into(), history.path.clone()]), 8192).await?;
                let invalid = || WorkspaceError::Invalid("This revision has no regular file at the selected path.".into());
                let record = std::str::from_utf8(&tree).map_err(|_| invalid())?.strip_suffix('\0').filter(|s| !s.contains('\0')).ok_or_else(invalid)?;
                let (header, _) = record.split_once('\t').ok_or_else(invalid)?;
                let fields: Vec<_> = header.split_whitespace().collect();
                if fields.len() != 3 || !matches!(fields[0], "100644" | "100755") || fields[1] != "blob" || !object_id(fields[2]) { return Err(invalid()); }
                let id = fields[2].to_owned();
                let size = self.run(history_args(vec!["cat-file".into(), "-s".into(), id.clone()]), 128).await?;
                let size: usize = String::from_utf8_lossy(&size).trim().parse().map_err(|_| invalid())?;
                if size > MAX_EDITOR_BYTES { return Err(RuntimeError::Limit.into()); }
                let blob = self.run(history_args(vec!["cat-file".into(), "blob".into(), id]), MAX_EDITOR_BYTES).await?;
                if blob.contains(&0) { return Err(RuntimeError::Unsupported("Binary revisions are not shown as text.".into()).into()); }
                let text = std::str::from_utf8(&blob).map_err(|_| RuntimeError::Unsupported("This revision is not UTF-8 text.".into()))?;
                let total_lines = text.lines().count();
                let shown_lines = total_lines.min(MAX_BLAME_LINES);
                let limited = total_lines > MAX_BLAME_LINES;
                if shown_lines == 0 {
                    self.check_history_head(history).await?;
                    return Ok(GitFileBlame { head: history.head.clone(), commit: commit.into(), lines: Vec::new(), limited });
                }

                // `--incremental` omits source text, keeping output bounded even when
                // the selected snapshot contains long lines. Textconv and external
                // diff drivers stay disabled, and configured ignore-revs files are
                // ignored so repository config cannot silently rewrite attribution.
                let bytes = self.run(history_args(vec![
                    "blame".into(),
                    "--incremental".into(),
                    "-l".into(),
                    "--root".into(),
                    "--no-textconv".into(),
                    "--no-ext-diff".into(),
                    "--no-ignore-revs-file".into(),
                    "--encoding=UTF-8".into(),
                    "-L".into(),
                    format!("1,{shown_lines}"),
                    commit.into(),
                    "--".into(),
                    history.path.clone(),
                ]), MAX_BLAME_OUTPUT_BYTES).await?;
                let lines = parse_blame(&bytes, shown_lines)?;
                self.check_history_head(history).await?;
                Ok(GitFileBlame { head: history.head.clone(), commit: commit.into(), lines, limited })
            } => result,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_parser_is_bounded_and_keeps_unknown_dates_unknown() {
        let input = format!("{}\0Dana\0not-a-time\0Subject\0", "a".repeat(40));
        let rows = parse_history(input.as_bytes()).unwrap();
        assert_eq!(rows[0].author, "Dana");
        assert!(parse_history(b"bad\0author\0invalid-time\0subject\0").is_err());
        assert!(parse_history(input.trim_end_matches('\0').as_bytes()).is_err());
        assert!(parse_history(input.repeat(52).as_bytes()).is_err());
        let input = format!("{}\0Dana\0not-a-time\0Subject\0", "a".repeat(64));
        assert!(parse_history(input.as_bytes()).unwrap()[0].date.is_none());
        assert_eq!(label(&"界".repeat(600)).chars().count(), 512);
    }
    #[test]
    fn incremental_blame_parser_maps_ranges_and_rejects_incomplete_records() {
        let first = "a".repeat(40);
        let second = "b".repeat(40);
        let input = format!(
            "{first} 1 2 1\nauthor Dana\nauthor-time 0\nsummary second lines\nfilename file.txt\n{first} 2 3 1\nfilename file.txt\n{second} 1 1 1\nauthor Riley\nauthor-time 1\nsummary first line\nfilename file.txt\n"
        );
        let rows = parse_blame(input.as_bytes(), 3).unwrap();
        assert_eq!(rows[0].commit, second);
        assert_eq!(rows[0].author, "Riley");
        assert_eq!(rows[1].commit, first);
        assert_eq!(rows[2].subject, "second lines");
        assert_eq!(rows[2].author, "Dana");
        assert!(parse_blame(input.as_bytes(), 4).is_err());
        assert!(parse_blame(b"bad header\n", 1).is_err());
        assert!(parse_blame(b"", 1).is_err());
        assert!(parse_blame(b"", 0).unwrap().is_empty());
    }
    #[tokio::test]
    async fn committed_history_preserves_edits_and_never_runs_clean_or_textconv_filters() {
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .current_dir(dir.path())
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            output.stdout
        };
        git(&["init", "-q"]);
        git(&["config", "user.name", "Fixture Author"]);
        git(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::create_dir(dir.path().join("nested")).unwrap();
        let file = "nested/literal [x] file.txt";
        std::fs::write(dir.path().join(file), "original\n").unwrap();
        git(&["add", "--", file]);
        git(&[
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgSign=false",
            "commit",
            "-qm",
            "Original",
        ]);
        std::fs::write(dir.path().join(file), "second\n").unwrap();
        git(&["add", "--", file]);
        git(&[
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgSign=false",
            "commit",
            "-qm",
            "Second",
        ]);
        // A normal worktree diff/blame would execute this clean filter. History must not.
        std::fs::write(
            dir.path().join(".gitattributes"),
            "* filter=probe diff=probe\n",
        )
        .unwrap();
        let command = format!("touch '{}'; cat", dir.path().join("executed").display());
        git(&["config", "filter.probe.clean", &command]);
        git(&["config", "diff.probe.textconv", &command]);
        std::fs::write(dir.path().join(file), "unsaved-on-disk\n").unwrap();
        let service = GitService::new(dir.path().join("nested"));
        let cancel = CancellationToken::new();
        let history = service
            .file_history("literal [x] file.txt".into(), &cancel)
            .await
            .unwrap();
        assert_eq!(history.commits.len(), 2);
        assert_eq!(history.commits[0].subject, "Second");
        let revision = service
            .file_revision(&history, &history.commits[1].id, &cancel)
            .await
            .unwrap();
        assert_eq!(revision.text, "original\n");
        let blame = service
            .file_blame(&history, &history.commits[0].id, &cancel)
            .await
            .unwrap();
        assert_eq!(blame.head, history.head);
        assert_eq!(blame.commit, history.commits[0].id);
        assert_eq!(blame.lines.len(), 1);
        assert_eq!(blame.lines[0].commit, history.commits[0].id);
        assert_eq!(blame.lines[0].author, "Fixture Author");
        assert_eq!(blame.lines[0].subject, "Second");
        assert!(GitService::new(dir.path().to_path_buf())
            .file_blame(&history, &history.commits[0].id, &cancel)
            .await
            .is_err());
        assert!(!dir.path().join("executed").exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join(file)).unwrap(),
            "unsaved-on-disk\n"
        );
        assert!(
            service
                .file_revision(&history, &"f".repeat(40), &cancel)
                .await
                .is_err()
        );
        assert!(
            service
                .file_history("../outside.txt".into(), &cancel)
                .await
                .is_err()
        );
        cancel.cancel();
        assert!(
            service
                .file_history("literal [x] file.txt".into(), &cancel)
                .await
                .is_err()
        );
        let cancel = CancellationToken::new();
        std::fs::remove_file(dir.path().join(".gitattributes")).unwrap();
        std::fs::write(dir.path().join(file), [b'a', 0, b'b']).unwrap();
        git(&["add", "--", file]);
        git(&[
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgSign=false",
            "commit",
            "-qm",
            "Binary",
        ]);
        let binary_history = service
            .file_history("literal [x] file.txt".into(), &cancel)
            .await
            .unwrap();
        assert!(matches!(
            service
                .file_revision(&binary_history, &binary_history.commits[0].id, &cancel)
                .await,
            Err(WorkspaceError::Runtime(RuntimeError::Unsupported(_)))
        ));
        std::fs::write(dir.path().join(file), vec![b'x'; MAX_EDITOR_BYTES + 1]).unwrap();
        git(&["add", "--", file]);
        git(&[
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgSign=false",
            "commit",
            "-qm",
            "Oversized",
        ]);
        let oversized = service
            .file_history("literal [x] file.txt".into(), &cancel)
            .await
            .unwrap();
        assert!(matches!(
            service
                .file_revision(&oversized, &oversized.commits[0].id, &cancel)
                .await,
            Err(WorkspaceError::Runtime(RuntimeError::Limit))
        ));
        assert!(service
            .file_blame(&history, &history.commits[1].id, &cancel)
            .await
            .is_err());
        // HEAD moving does not change an already selected immutable revision.
        assert_eq!(
            service
                .file_revision(&history, &history.commits[1].id, &cancel)
                .await
                .unwrap()
                .text,
            "original\n"
        );
    }
}
