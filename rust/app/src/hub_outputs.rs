//! Port of upstream `HubOutputReactor` + `studioOutputs.ts` scanning: the Hub
//! root is typically not a git repository, and file-change tool activities
//! miss files created by shell subprocesses. Scan the workspace before a
//! turn, rescan when it settles, and persist the diff as a
//! `studio.outputs.captured` activity that the Hub outputs listing reads.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

const MAX_SCAN_DEPTH: usize = 8;
const MAX_SCAN_FILES: usize = 4_000;
const MAX_SCAN_ENTRIES: usize = 20_000;
const MAX_TRACKED_TURN_BASELINES: usize = 128;

const EXCLUDED_TOP_LEVEL_DIRS: &[&str] = &["tmp", "logs", "inbox", "context", "skills"];
const EXCLUDED_ROOT_FILES: &[&str] = &["agents.md", "claude.md"];
const EXCLUDED_NESTED_DIRS: &[&str] = &["node_modules"];

fn is_safe_visible_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.starts_with('.')
        && !EXCLUDED_NESTED_DIRS.contains(&segment.to_lowercase().as_str())
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct FileStat {
    mtime_ms: i64,
    size: u64,
}

type Scan = HashMap<String, FileStat>;

/// Bounded walk of the workspace root collecting candidate output files.
/// Managed input/infra subtrees, hidden entries, and symlinked directories
/// are skipped; depth, entry-count, and file-count caps keep a pathological
/// tree from stalling the reactor.
pub(crate) fn scan_workspace_files(root: &Path) -> Scan {
    let mut files = HashMap::new();
    let mut pending: Vec<Vec<String>> = vec![Vec::new()];
    let mut scanned_entries = 0usize;

    while let Some(segments) = pending.pop() {
        if files.len() >= MAX_SCAN_FILES || scanned_entries >= MAX_SCAN_ENTRIES {
            break;
        }
        let dir_path: PathBuf = segments.iter().fold(root.to_path_buf(), |p, s| p.join(s));
        let Ok(read_dir) = std::fs::read_dir(&dir_path) else {
            continue;
        };
        for entry in read_dir.flatten() {
            if files.len() >= MAX_SCAN_FILES || scanned_entries >= MAX_SCAN_ENTRIES {
                break;
            }
            scanned_entries += 1;
            let name = entry.file_name().to_string_lossy().to_string();
            if !is_safe_visible_segment(&name) {
                continue;
            }
            if segments.is_empty()
                && (EXCLUDED_TOP_LEVEL_DIRS.contains(&name.to_lowercase().as_str())
                    || EXCLUDED_ROOT_FILES.contains(&name.to_lowercase().as_str()))
            {
                continue;
            }
            let mut child_segments = segments.clone();
            child_segments.push(name);
            // symlink_metadata does not follow links: a symlinked directory is
            // never descended, matching upstream's readLink guard.
            let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                // A symlinked file is still a listable output if it resolves;
                // a symlinked directory is skipped entirely.
                let Ok(target_meta) = std::fs::metadata(entry.path()) else {
                    continue;
                };
                if target_meta.is_file() {
                    let mtime_ms = target_meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    if files.len() < MAX_SCAN_FILES {
                        files.insert(
                            child_segments.join("/"),
                            FileStat {
                                mtime_ms,
                                size: target_meta.len(),
                            },
                        );
                    }
                }
                continue;
            }
            if meta.is_file() {
                if files.len() >= MAX_SCAN_FILES {
                    break;
                }
                let mtime_ms = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                files.insert(
                    child_segments.join("/"),
                    FileStat {
                        mtime_ms,
                        size: meta.len(),
                    },
                );
            } else if meta.is_dir() && child_segments.len() < MAX_SCAN_DEPTH {
                pending.push(child_segments);
            }
        }
    }
    files
}

/// Files present in `after` that are new or changed since `before`. Deletions
/// are ignored: a removed file is no longer a listable output.
pub(crate) fn diff_scans(before: &Scan, after: &Scan) -> Vec<String> {
    let mut changed = Vec::new();
    for (path, stat) in after {
        match before.get(path) {
            Some(prev) if prev == stat => {}
            _ => changed.push(path.clone()),
        }
    }
    changed.sort();
    changed
}

struct Baseline {
    root: PathBuf,
    files: Scan,
}

static BASELINES: LazyLock<Mutex<HashMap<String, Baseline>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Resolve the Hub scan root for a thread: group/studio container projects
/// whose workspace is NOT a git repository (git repos get precise checkpoint
/// attribution instead).
fn resolve_hub_scan_root(conn: &rusqlite::Connection, thread_id: &str) -> Option<PathBuf> {
    let (kind, cwd): (String, String) = conn
        .query_row(
            "SELECT p.kind, COALESCE(NULLIF(t.working_directory, ''), NULLIF(t.worktree_path, ''), p.workspace_root) \
             FROM projection_threads t JOIN projection_projects p ON p.project_id = t.project_id \
             WHERE t.thread_id = ?1",
            [thread_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok()?;
    if !(kind == "group" || kind == "studio") {
        return None;
    }
    if cwd.is_empty() {
        return None;
    }
    let root = PathBuf::from(cwd);
    if crate::checkpointing::is_git_repo(&root) {
        return None;
    }
    Some(root)
}

/// Snapshot the baseline before a turn starts. No-op for non-Hub threads.
pub(crate) fn capture_baseline(thread_id: &str) {
    let Ok(conn) = crate::orchestration_engine::open_state_rw() else {
        return;
    };
    let Some(root) = resolve_hub_scan_root(&conn, thread_id) else {
        return;
    };
    let files = scan_workspace_files(&root);
    if let Ok(mut map) = BASELINES.lock() {
        if map.len() >= MAX_TRACKED_TURN_BASELINES {
            // Evict the oldest entry (insertion order is not tracked; evicting
            // an arbitrary key is still bounded and lossless for active turns).
            if let Some(key) = map.keys().next().cloned() {
                map.remove(&key);
            }
        }
        map.insert(thread_id.to_string(), Baseline { root, files });
    }
}

/// Rescan at turn completion and persist the diff as a captured-outputs
/// activity. No-op when no baseline exists for the thread.
pub(crate) fn capture_outputs(thread_id: &str, turn_id: &str) {
    let baseline = BASELINES
        .lock()
        .ok()
        .and_then(|mut map| map.remove(thread_id));
    let Some(baseline) = baseline else { return };
    let after = scan_workspace_files(&baseline.root);
    let changed = diff_scans(&baseline.files, &after);
    if changed.is_empty() {
        return;
    }
    let payload = json!({
        "threadId": thread_id,
        "turnId": turn_id,
        "paths": changed,
    });
    let activity = json!({
        "id": crate::orchestration_engine::uuid_v4(),
        "tone": "info",
        "kind": "studio.outputs.captured",
        "summary": "Studio outputs captured",
        "payload": payload,
        "turnId": turn_id,
        "createdAt": crate::todo::iso_now_ms(),
    });
    let mut cmd = serde_json::Map::new();
    cmd.insert("type".into(), json!("thread.activity.append"));
    cmd.insert(
        "commandId".into(),
        json!(format!(
            "server:hub-outputs:{}",
            crate::orchestration_engine::uuid_v4()
        )),
    );
    cmd.insert("threadId".into(), json!(thread_id));
    cmd.insert("createdAt".into(), json!(crate::todo::iso_now_ms()));
    cmd.insert("activity".into(), activity);
    let _ =
        crate::orchestration_engine::handle("orchestration.dispatchCommand", &Value::Object(cmd));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "synara-hub-test-{}-{}",
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scan_skips_hidden_and_infra_entries() {
        let root = temp_root("scan");
        std::fs::write(root.join("output.png"), b"x").unwrap();
        std::fs::write(root.join(".DS_Store"), b"x").unwrap();
        std::fs::write(root.join("AGENTS.md"), b"x").unwrap();
        std::fs::create_dir_all(root.join("node_modules/dep")).unwrap();
        std::fs::write(root.join("node_modules/dep/index.js"), b"x").unwrap();
        std::fs::create_dir_all(root.join("inbox")).unwrap();
        std::fs::write(root.join("inbox/file.txt"), b"x").unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/readme.md"), b"x").unwrap();

        let scan = scan_workspace_files(&root);
        assert!(scan.contains_key("output.png"));
        assert!(scan.contains_key("docs/readme.md"));
        assert!(!scan.keys().any(|k| k.contains("DS_Store")));
        assert!(!scan.keys().any(|k| k.contains("node_modules")));
        assert!(!scan.keys().any(|k| k.starts_with("inbox")));
        assert!(!scan.keys().any(|k| k == "AGENTS.md"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn diff_reports_new_and_changed_only() {
        let mut before = Scan::new();
        before.insert(
            "a.txt".into(),
            FileStat {
                mtime_ms: 1,
                size: 10,
            },
        );
        before.insert(
            "gone.txt".into(),
            FileStat {
                mtime_ms: 1,
                size: 5,
            },
        );
        let mut after = Scan::new();
        after.insert(
            "a.txt".into(),
            FileStat {
                mtime_ms: 1,
                size: 10,
            },
        );
        after.insert(
            "b.txt".into(),
            FileStat {
                mtime_ms: 2,
                size: 20,
            },
        );
        after.insert(
            "c.txt".into(),
            FileStat {
                mtime_ms: 2,
                size: 5,
            },
        );
        let diff = diff_scans(&before, &after);
        assert_eq!(diff, vec!["b.txt", "c.txt"]);
    }
}
