//! Project-search index ownership shared by local workspaces and the SSH helper.
//! Git repositories use Git's own ignore engine; non-Git folders only skip the
//! generated/infrastructure directories used by upstream project search.
use crate::{FileEntry, RuntimeError, WorkspaceFs};
use std::{
    collections::{BTreeSet, HashSet},
    env,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_INDEX_ENTRIES: usize = 25_000;
const MAX_SOURCE_PATHS: usize = 100_000;
const MAX_GIT_INDEX_BYTES: usize = 16 * 1024 * 1024;
const MAX_GIT_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const MAX_GIT_IGNORE_STDIN_BYTES: usize = 256 * 1024;
const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) fn entries(fs: &WorkspaceFs) -> Result<Vec<FileEntry>, RuntimeError> {
    let inside_git = git_inside_worktree(fs.root());
    if inside_git && let Some(entries) = git_entries(fs)? {
        return Ok(entries);
    }
    walk_entries(fs, inside_git)
}

fn generated_directory(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".convex"
            | "node_modules"
            | ".next"
            | ".turbo"
            | "dist"
            | "build"
            | "out"
            | ".cache"
    )
}

fn top_level_generated(path: &Path) -> bool {
    path.components().next().is_some_and(|component| {
        matches!(
            component,
            Component::Normal(name)
                if name.to_str().is_some_and(generated_directory)
        )
    })
}

fn walk_entries(fs: &WorkspaceFs, apply_git_ignore: bool) -> Result<Vec<FileEntry>, RuntimeError> {
    let mut pending = vec![PathBuf::new()];
    let mut entries = Vec::new();
    while !pending.is_empty() && entries.len() < MAX_INDEX_ENTRIES {
        let current = std::mem::take(&mut pending);
        let mut next = Vec::new();
        for directory in current {
            let candidates = fs.entries(&directory)?;
            let ignored = if apply_git_ignore {
                git_ignored_paths(
                    fs.root(),
                    candidates.iter().map(|entry| &entry.relative_path),
                )
                .unwrap_or_default()
            } else {
                HashSet::new()
            };
            for entry in candidates {
                if entry.symlink
                    || ignored.contains(&entry.relative_path)
                    || (entry.directory && generated_directory(&entry.name))
                {
                    continue;
                }
                if entry.directory {
                    next.push(entry.relative_path.clone());
                }
                entries.push(entry);
                if entries.len() >= MAX_INDEX_ENTRIES {
                    break;
                }
            }
            if entries.len() >= MAX_INDEX_ENTRIES {
                break;
            }
        }
        pending = next;
    }
    Ok(entries)
}

fn git_inside_worktree(root: &Path) -> bool {
    run_git(
        root,
        &["rev-parse", "--is-inside-work-tree"],
        None,
        32,
        false,
    )
    .is_some_and(|output| {
        std::str::from_utf8(&output)
            .ok()
            .is_some_and(|text| text.trim() == "true")
    })
}

fn git_entries(fs: &WorkspaceFs) -> Result<Option<Vec<FileEntry>>, RuntimeError> {
    let Some(listed) = run_git(
        fs.root(),
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
        None,
        MAX_GIT_INDEX_BYTES,
        false,
    ) else {
        return Ok(None);
    };

    let mut paths = Vec::new();
    for raw in listed
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
    {
        let Ok(text) = std::str::from_utf8(raw) else {
            continue;
        };
        let path = PathBuf::from(text);
        if crate::validate_relative(&path).is_err() || top_level_generated(&path) {
            continue;
        }
        paths.push(path);
        if paths.len() > MAX_SOURCE_PATHS {
            return Err(RuntimeError::Limit);
        }
    }

    let ignored = git_ignored_paths(fs.root(), paths.iter()).unwrap_or_default();
    let mut directories = BTreeSet::new();
    let mut files = Vec::new();
    for path in paths {
        if ignored.contains(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        files.push(FileEntry {
            name: name.to_owned(),
            relative_path: path.clone(),
            directory: false,
            symlink: false,
        });
        let mut parent = path.parent();
        while let Some(directory) = parent {
            if directory.as_os_str().is_empty() {
                break;
            }
            directories.insert(directory.to_owned());
            parent = directory.parent();
        }
    }

    let mut entries = directories
        .into_iter()
        .filter_map(|relative_path| {
            let name = relative_path.file_name()?.to_str()?.to_owned();
            Some(FileEntry {
                name,
                relative_path,
                directory: true,
                symlink: false,
            })
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    entries.extend(files);
    entries.truncate(MAX_INDEX_ENTRIES);
    Ok(Some(entries))
}

fn git_ignored_paths<'a>(
    root: &Path,
    paths: impl IntoIterator<Item = &'a PathBuf>,
) -> Option<HashSet<PathBuf>> {
    let mut ignored = HashSet::new();
    let mut chunk = Vec::new();
    for path in paths {
        let text = path.to_str()?.replace('\\', "/");
        let required = text.len().checked_add(1)?;
        if required > MAX_GIT_IGNORE_STDIN_BYTES {
            return None;
        }
        if !chunk.is_empty() && chunk.len().checked_add(required)? > MAX_GIT_IGNORE_STDIN_BYTES {
            append_ignored_chunk(root, &chunk, &mut ignored)?;
            chunk.clear();
        }
        chunk.extend_from_slice(text.as_bytes());
        chunk.push(0);
    }
    if !chunk.is_empty() {
        append_ignored_chunk(root, &chunk, &mut ignored)?;
    }
    Some(ignored)
}

fn append_ignored_chunk(root: &Path, input: &[u8], ignored: &mut HashSet<PathBuf>) -> Option<()> {
    let output = run_git(
        root,
        &["check-ignore", "--no-index", "-z", "--stdin"],
        Some(input),
        MAX_GIT_IGNORE_STDIN_BYTES,
        true,
    )?;
    for raw in output
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
    {
        let text = std::str::from_utf8(raw).ok()?;
        let path = PathBuf::from(text);
        if crate::validate_relative(&path).is_ok() {
            ignored.insert(path);
        }
    }
    Some(())
}

fn resolve_git(root: &Path) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path).filter(|directory| directory.is_absolute()) {
        #[cfg(windows)]
        let candidates = ["git.exe"];
        #[cfg(not(windows))]
        let candidates = ["git"];
        for name in candidates {
            let candidate = directory.join(name);
            if !candidate.is_file() {
                continue;
            }
            let Ok(canonical) = candidate.canonicalize() else {
                continue;
            };
            if canonical.starts_with(root) {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let Ok(metadata) = canonical.metadata() else {
                    continue;
                };
                if metadata.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            return Some(canonical);
        }
    }
    None
}

fn run_git(
    root: &Path,
    args: &[&str],
    input: Option<&[u8]>,
    max_stdout: usize,
    allow_exit_one: bool,
) -> Option<Vec<u8>> {
    let git = resolve_git(root)?;
    let mut command = Command::new(git);
    command
        .args([
            "--no-pager",
            "--literal-pathspecs",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "credential.interactive=false",
        ])
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().ok()?;

    let stdin_writer = if let Some(input) = input {
        let mut stdin = child.stdin.take()?;
        let input = input.to_vec();
        Some(thread::spawn(move || stdin.write_all(&input)))
    } else {
        drop(child.stdin.take());
        None
    };
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout, max_stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, MAX_GIT_DIAGNOSTIC_BYTES));

    let deadline = Instant::now() + GIT_COMMAND_TIMEOUT;
    let status = loop {
        match child.try_wait().ok()? {
            Some(status) => break status,
            None if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if let Some(writer) = stdin_writer
        && writer.join().ok()?.is_err()
    {
        return None;
    }
    let (stdout, stdout_exceeded) = stdout_reader.join().ok()?.ok()?;
    let (_, stderr_exceeded) = stderr_reader.join().ok()?.ok()?;
    if stdout_exceeded || stderr_exceeded {
        return None;
    }
    if !status.success() && !(allow_exit_one && status.code() == Some(1)) {
        return None;
    }
    Some(stdout)
}

fn read_bounded(mut reader: impl Read, limit: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let mut retained = Vec::with_capacity(limit.min(64 * 1024));
    let mut exceeded = false;
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = limit.saturating_sub(retained.len());
        let keep = remaining.min(count);
        retained.extend_from_slice(&buffer[..keep]);
        exceeded |= keep < count;
    }
    Ok((retained, exceeded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_directory_policy_matches_upstream_set() {
        for name in [
            ".git",
            ".convex",
            "node_modules",
            ".next",
            ".turbo",
            "dist",
            "build",
            "out",
            ".cache",
        ] {
            assert!(generated_directory(name), "{name}");
        }
        for name in ["target", "coverage", ".venv", "__pycache__", "vendor"] {
            assert!(!generated_directory(name), "{name}");
        }
        assert!(top_level_generated(Path::new("node_modules/pkg/index.js")));
        assert!(!top_level_generated(Path::new(
            "src/node_modules/pkg/index.js"
        )));
    }
}
