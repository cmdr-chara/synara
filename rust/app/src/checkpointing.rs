//! Hidden git-ref checkpoint capture, ported from upstream `checkpointing/`.
//!
//! Upstream captures the workspace into hidden git refs
//! (`refs/synara/checkpoints/<thread>/<kind>/<token>`) at turn boundaries so
//! turn diffs and checkpoint revert never touch the user's own refs. The Rust
//! port precomputes diff blobs into `checkpoint_diff_blobs` at capture time
//! instead of recomputing them per RPC.
//!
//! Capture pipeline (mirrors `CheckpointStore.captureCheckpoint`): seed a
//! throwaway index from the working index (or `read-tree HEAD`), `add -A`,
//! `write-tree`, `commit-tree` with the current HEAD as parent, `update-ref`.
//! Nothing touches the user's index, worktree, or visible refs.

use base64::Engine;
use rusqlite::{params, Connection};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::orchestration_engine::open_state_rw;
use crate::todo::WsErr;

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

const REFS_PREFIX: &str = "refs/synara/checkpoints";
const DIFF_MAX_OUTPUT_BYTES: usize = 10_000_000;

fn thread_token(thread_id: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(thread_id.as_bytes())
}

fn ref_for_turn(thread_id: &str, turn_count: i64) -> String {
    format!("{REFS_PREFIX}/{}/turn/{turn_count}", thread_token(thread_id))
}

fn git(cwd: &Path, args: &[&str], env_index: Option<&Path>) -> Result<std::process::Output, WsErr> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(cwd).args(args);
    if let Some(index) = env_index {
        cmd.env("GIT_INDEX_FILE", index);
        cmd.env("GIT_AUTHOR_NAME", "Synara");
        cmd.env("GIT_AUTHOR_EMAIL", "synara@users.noreply.github.com");
        cmd.env("GIT_COMMITTER_NAME", "Synara");
        cmd.env("GIT_COMMITTER_EMAIL", "synara@users.noreply.github.com");
    }
    cmd.output().map_err(|e| err(format!("git spawn: {e}")))
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(cwd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub(crate) fn is_git_repo(cwd: &Path) -> bool {
    git_stdout(cwd, &["rev-parse", "--is-inside-work-tree"])
        .map(|v| v == "true")
        .unwrap_or(false)
}

fn head_commit(cwd: &Path) -> Option<String> {
    git_stdout(cwd, &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])
        .filter(|v| !v.is_empty())
}

fn ref_commit(cwd: &Path, reference: &str) -> Option<String> {
    git_stdout(cwd, &["rev-parse", "--verify", "--quiet", &format!("{reference}^{{commit}}")])
        .filter(|v| !v.is_empty())
}

/// Capture the full worktree state (tracked + untracked, honoring ignore rules)
/// into `reference`. No-op when the ref already exists. Returns the commit id.
fn capture(cwd: &Path, reference: &str) -> Result<Option<String>, WsErr> {
    if let Some(existing) = ref_commit(cwd, reference) {
        return Ok(Some(existing));
    }
    if !is_git_repo(cwd) {
        return Ok(None);
    }
    let temp_dir = std::env::temp_dir().join(format!("synara-checkpoint-{}", uuid_v4()));
    std::fs::create_dir_all(&temp_dir).map_err(|e| err(format!("temp dir: {e}")))?;
    let temp_index = temp_dir.join("index");

    let result = (|| -> Result<Option<String>, WsErr> {
        // Seed the throwaway index from the working index to preserve Git's
        // stat cache; fall back to read-tree HEAD when there is no index yet.
        let working_index = git_stdout(cwd, &["rev-parse", "--git-path", "index"])
            .filter(|v| !v.is_empty())
            .map(|v| {
                let p = PathBuf::from(&v);
                if p.is_absolute() { p } else { cwd.join(p) }
            });
        let mut seeded = false;
        if let Some(index_path) = &working_index {
            if index_path.exists() && std::fs::copy(index_path, &temp_index).is_ok() {
                seeded = true;
                // Verify racily-clean entries without discarding the cache.
                let _ = git(cwd, &["update-index", "--really-refresh"], Some(&temp_index));
            }
        }
        if !seeded && head_commit(cwd).is_some() {
            git(cwd, &["read-tree", "HEAD"], Some(&temp_index))?;
        }

        let add = git(cwd, &["add", "-A", "--", "."], Some(&temp_index))?;
        if !add.status.success() {
            return Err(err(format!(
                "git add failed: {}",
                String::from_utf8_lossy(&add.stderr).trim()
            )));
        }
        let tree = git(cwd, &["write-tree"], Some(&temp_index))?;
        if !tree.status.success() {
            return Err(err("git write-tree failed"));
        }
        let tree_id = String::from_utf8_lossy(&tree.stdout).trim().to_string();

        let mut commit_args: Vec<String> = vec![tree_id.clone()];
        if let Some(head) = head_commit(cwd) {
            commit_args.push("-p".into());
            commit_args.push(head);
        }
        let message = format!("synara checkpoint\n\nSynara-Head: {}", head_commit(cwd).unwrap_or_default());
        let commit = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .arg("commit-tree")
            .args(&commit_args)
            .arg("-m")
            .arg(&message)
            .env("GIT_AUTHOR_NAME", "Synara")
            .env("GIT_AUTHOR_EMAIL", "synara@users.noreply.github.com")
            .env("GIT_COMMITTER_NAME", "Synara")
            .env("GIT_COMMITTER_EMAIL", "synara@users.noreply.github.com")
            .output()
            .map_err(|e| err(format!("commit-tree spawn: {e}")))?;
        if !commit.status.success() {
            return Err(err(format!(
                "commit-tree failed: {}",
                String::from_utf8_lossy(&commit.stderr).trim()
            )));
        }
        let commit_id = String::from_utf8_lossy(&commit.stdout).trim().to_string();

        let update = git(cwd, &["update-ref", reference, &commit_id], None)?;
        if !update.status.success() {
            return Err(err(format!(
                "update-ref failed: {}",
                String::from_utf8_lossy(&update.stderr).trim()
            )));
        }
        Ok(Some(commit_id))
    })();

    let _ = std::fs::remove_dir_all(&temp_dir);
    result
}

/// `git diff` between two refs, bounded like upstream
/// (`CHECKPOINT_DIFF_MAX_OUTPUT_BYTES`); empty-string diff when the refs
/// resolve to the same tree. None when either ref is missing.
fn diff_refs(cwd: &Path, from_ref: &str, to_ref: &str) -> Option<String> {
    if ref_commit(cwd, from_ref).is_none() || ref_commit(cwd, to_ref).is_none() {
        return None;
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["diff", "--no-color", from_ref, to_ref])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let bytes = if out.stdout.len() > DIFF_MAX_OUTPUT_BYTES {
        out.stdout[..DIFF_MAX_OUTPUT_BYTES].to_vec()
    } else {
        out.stdout
    };
    Some(String::from_utf8_lossy(&bytes).to_string())
}

/// Names of files differing between two refs, newline-free `-z` parsing.
fn changed_files(cwd: &Path, from_ref: &str, to_ref: &str) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["diff", "--name-only", "-z", from_ref, to_ref])
        .output();
    match out {
        Ok(o) if o.status.success() => o
            .stdout
            .split(|b| *b == 0)
            .filter_map(|p| std::str::from_utf8(p).ok())
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    let mut v = nanos ^ (pid << 64) ^ (nanos.rotate_left(37) << 13);
    let mut bytes = v.to_le_bytes();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    v = u128::from_le_bytes(bytes);
    let hex = format!("{v:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Latest checkpoint turn count recorded for the thread, 0 when none.
fn latest_turn_count(conn: &Connection, thread_id: &str) -> i64 {
    conn.query_row(
        "SELECT COALESCE(MAX(checkpoint_turn_count), 0) FROM projection_turns \
         WHERE thread_id = ?1 AND checkpoint_turn_count IS NOT NULL",
        params![thread_id],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
}

/// Capture the post-turn checkpoint for `thread_id` in `root` and record it:
/// the hidden ref on `projection_turns`, the changed-file list, and the
/// precomputed diff blobs for `checkpoint_diff_blobs` (turn range and full
/// thread range). Runs synchronously; callers spawn it off the runner thread.
pub(crate) fn capture_turn_checkpoint(thread_id: &str, turn_id: &str, root: PathBuf) {
    let _ = capture_turn_checkpoint_inner(thread_id, turn_id, &root);
}

fn capture_turn_checkpoint_inner(thread_id: &str, turn_id: &str, root: &Path) -> Result<(), WsErr> {
    if !is_git_repo(root) {
        return Ok(());
    }
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    // Turns with no recorded row (or an existing checkpoint) are not recaptured.
    let already: Option<String> = conn
        .query_row(
            "SELECT checkpoint_ref FROM projection_turns \
             WHERE thread_id = ?1 AND turn_id = ?2",
            params![thread_id, turn_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten();
    if already.as_deref().is_some_and(|v| !v.is_empty()) {
        return Ok(());
    }

    let prev_count = latest_turn_count(&conn, thread_id);
    let turn_count = prev_count + 1;
    let reference = ref_for_turn(thread_id, turn_count);
    let commit = capture(root, &reference)?;
    let Some(_commit) = commit else {
        return Ok(());
    };

    // The base state of the thread: the first turn's parent commit. Upstream
    // uses dedicated turn-start refs; the Rust port anchors the full-thread
    // diff at the first checkpoint's parent, which is the same worktree state
    // for threads that began inside Synara.
    let files_json;
    if prev_count == 0 {
        // First checkpoint of the thread: capture the parent state as turn/0
        // when HEAD exists, so the full-thread range has a real anchor.
        if head_commit(root).is_some() {
            let base_ref = ref_for_turn(thread_id, 0);
            // turn/0 mirrors the pre-turn tree: HEAD plus nothing else. When
            // the working tree already differed from HEAD before the first
            // turn, that drift is attributed to the first turn — the same
            // simplification noted in the projection comments.
            let _ = capture_tree_at_head(root, &base_ref);
        }
    }

    let prev_ref = ref_for_turn(thread_id, turn_count - 1);
    let diff = diff_refs(root, &prev_ref, &reference).unwrap_or_default();
    let files = changed_files(root, &prev_ref, &reference);
    files_json = json!(files).to_string();
    let now = crate::todo::iso_now_ms();

    conn.execute(
        "UPDATE projection_turns SET checkpoint_turn_count = ?3, checkpoint_ref = ?4, \
         checkpoint_status = 'ready', checkpoint_files_json = ?5 \
         WHERE thread_id = ?1 AND turn_id = ?2",
        params![thread_id, turn_id, turn_count, reference, files_json],
    )
    .map_err(|e| err(format!("checkpoint projection: {e}")))?;

    let mut insert_blob = |from: i64, to: i64, diff_text: &str| -> Result<(), WsErr> {
        conn.execute(
            "INSERT OR REPLACE INTO checkpoint_diff_blobs \
             (thread_id, from_turn_count, to_turn_count, diff, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![thread_id, from, to, diff_text, now],
        )
        .map_err(|e| err(format!("diff blob: {e}")))?;
        Ok(())
    };
    insert_blob(turn_count - 1, turn_count, &diff)?;
    if turn_count > 1 {
        // Refresh the full-thread range cheaply: previous full diff plus this
        // turn's diff concatenated is not valid unified-diff arithmetic, so
        // recompute against the base ref when it exists.
        let base_ref = ref_for_turn(thread_id, 0);
        if let Some(full) = diff_refs(root, &base_ref, &reference) {
            insert_blob(0, turn_count, &full)?;
        }
    } else {
        insert_blob(0, turn_count, &diff)?;
    }
    Ok(())
}

/// Capture a ref whose tree is exactly HEAD's tree (the pre-turn base state).
fn capture_tree_at_head(cwd: &Path, reference: &str) -> Result<Option<String>, WsErr> {
    if ref_commit(cwd, reference).is_some() {
        return Ok(ref_commit(cwd, reference));
    }
    let Some(head) = head_commit(cwd) else {
        return Ok(None);
    };
    let tree = git_stdout(cwd, &["rev-parse", "--verify", "--quiet", "HEAD^{tree}"])
        .filter(|v| !v.is_empty());
    let Some(tree_id) = tree else {
        return Ok(None);
    };
    let message = format!("synara checkpoint base\n\nSynara-Head: {head}");
    let commit = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["commit-tree", &tree_id, "-p", &head, "-m", &message])
        .env("GIT_AUTHOR_NAME", "Synara")
        .env("GIT_AUTHOR_EMAIL", "synara@users.noreply.github.com")
        .env("GIT_COMMITTER_NAME", "Synara")
        .env("GIT_COMMITTER_EMAIL", "synara@users.noreply.github.com")
        .output()
        .map_err(|e| err(format!("commit-tree spawn: {e}")))?;
    if !commit.status.success() {
        return Ok(None);
    }
    let commit_id = String::from_utf8_lossy(&commit.stdout).trim().to_string();
    let update = git(cwd, &["update-ref", reference, &commit_id], None)?;
    if !update.status.success() {
        return Ok(None);
    }
    Ok(Some(commit_id))
}

/// Spawn the capture off the caller thread; capture cost scales with
/// worktree size and must not block provider turn teardown.
pub(crate) fn spawn_capture(thread_id: String, turn_id: String, root: PathBuf) {
    std::thread::spawn(move || {
        capture_turn_checkpoint(&thread_id, &turn_id, root);
    });
}


/// Restore the worktree to the checkpoint recorded for `turn_count`, then
/// report completion through the internal command channel so the projection
/// lifecycle matches upstream (`checkpoint.revert.succeeded` /
/// `.failed` activities). Runs off the caller thread.
pub(crate) fn spawn_revert(thread_id: String, turn_count: i64, root: PathBuf) {
    std::thread::spawn(move || {
        let outcome = revert_inner(&thread_id, turn_count, &root);
        let now = crate::todo::iso_now_ms();
        let mut command = serde_json::Map::new();
        command.insert("type".into(), json!("thread.revert.complete"));
        command.insert("commandId".into(), json!(format!("revert-complete:{thread_id}:{turn_count}")));
        command.insert("threadId".into(), json!(thread_id));
        command.insert("createdAt".into(), json!(now));
        command.insert("turnCount".into(), json!(turn_count));
        match outcome {
            Ok(()) => {
                let _ = crate::orchestration_engine::handle(
                    "orchestration.dispatchCommand",
                    &serde_json::Value::Object(command),
                );
            }
            Err(e) => {
                let mut fail = serde_json::Map::new();
                fail.insert("type".into(), json!("thread.activity.append"));
                fail.insert(
                    "commandId".into(),
                    json!(format!("revert-failed:{thread_id}:{turn_count}")),
                );
                fail.insert("threadId".into(), json!(thread_id.clone()));
                fail.insert("createdAt".into(), json!(crate::todo::iso_now_ms()));
                fail.insert(
                    "activity".into(),
                    json!({
                        "id": uuid_v4(),
                        "tone": "error",
                        "kind": "checkpoint.revert.failed",
                        "summary": "Checkpoint revert failed",
                        "payload": {"turnCount": turn_count, "error": e.0},
                        "turnId": null,
                        "createdAt": crate::todo::iso_now_ms(),
                    }),
                );
                let _ = crate::orchestration_engine::handle(
                    "orchestration.dispatchCommand",
                    &serde_json::Value::Object(fail),
                );
            }
        }
    });
}

fn revert_inner(thread_id: &str, turn_count: i64, root: &Path) -> Result<(), WsErr> {
    let reference = ref_for_turn(thread_id, turn_count);
    if ref_commit(root, &reference).is_none() {
        return Err(err(format!("checkpoint ref {reference} not found")));
    }
    // Rescue snapshot of the pre-revert worktree, mirroring upstream
    // revert-rescue refs: if the restore itself fails midway, the user's
    // pre-revert state is recoverable from the hidden ref.
    let rescue = format!(
        "{REFS_PREFIX}/{}/revert-rescue/{}",
        thread_token(thread_id),
        uuid_v4()
    );
    let _ = capture(root, &rescue);

    // read-tree --reset -u syncs index and worktree to the checkpoint tree.
    // Because captures record `git add -A`, every non-ignored file present at
    // capture time is tracked afterward; `git clean -fd` then removes exactly
    // the files created after the checkpoint, which is the revert semantic.
    let reset = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["read-tree", "--reset", "-u", &reference])
        .output()
        .map_err(|e| err(format!("read-tree spawn: {e}")))?;
    if !reset.status.success() {
        return Err(err(format!(
            "read-tree --reset failed: {}",
            String::from_utf8_lossy(&reset.stderr).trim()
        )));
    }
    let clean = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["clean", "-fd"])
        .output()
        .map_err(|e| err(format!("clean spawn: {e}")))?;
    if !clean.status.success() {
        return Err(err(format!(
            "git clean failed: {}",
            String::from_utf8_lossy(&clean.stderr).trim()
        )));
    }
    // The rescue ref stays in place — it is the user's only copy of the
    // pre-revert state. Upstream deletes it after the conversation rollback
    // commits; the Rust port keeps it since refs cost nothing and are hidden.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn temp_repo() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("synara-ckpt-test-{}-{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(&dir).args(args).output().unwrap();
            assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@test"]);
        run(&["config", "user.name", "Test"]);
        std::fs::write(dir.join("a.txt"), "one").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "init"]);
        dir
    }

    #[test]
    fn capture_records_hidden_ref_with_worktree_state() {
        let dir = temp_repo();
        std::fs::write(dir.join("a.txt"), "modified").unwrap();
        std::fs::write(dir.join("new.txt"), "untracked").unwrap();
        let reference = "refs/synara/checkpoints/test/turn/1";
        let commit = capture(&dir, reference).unwrap().expect("captured");
        assert_eq!(ref_commit(&dir, reference).as_deref(), Some(commit.as_str()));
        // The checkpoint tree carries the edit and the untracked file.
        let tree_listing = git_stdout(&dir, &["ls-tree", "-r", "--name-only", reference]).unwrap();
        assert!(tree_listing.contains("a.txt"));
        assert!(tree_listing.contains("new.txt"));
        let content = git_stdout(&dir, &["show", &format!("{reference}:a.txt")]).unwrap();
        assert_eq!(content, "modified");
        // Capture is idempotent.
        let again = capture(&dir, reference).unwrap().unwrap();
        assert_eq!(again, commit);
        // The user's index and HEAD are untouched.
        assert!(head_commit(&dir).as_deref() != Some(commit.as_str()));
        let status = git_stdout(&dir, &["status", "--porcelain"]).unwrap();
        assert!(status.contains("new.txt") || status.contains("a.txt"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_refs_reports_changes_between_captures() {
        let dir = temp_repo();
        capture(&dir, "refs/synara/checkpoints/t/turn/0").unwrap();
        std::fs::write(dir.join("a.txt"), "two").unwrap();
        capture(&dir, "refs/synara/checkpoints/t/turn/1").unwrap();
        let diff = diff_refs(&dir, "refs/synara/checkpoints/t/turn/0", "refs/synara/checkpoints/t/turn/1").unwrap();
        assert!(diff.contains("a.txt"), "diff missing a.txt: {diff}");
        assert!(diff.contains("+two"));
        let files = changed_files(&dir, "refs/synara/checkpoints/t/turn/0", "refs/synara/checkpoints/t/turn/1");
        assert_eq!(files, vec!["a.txt".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capture_tree_at_head_mirrors_head() {
        let dir = temp_repo();
        std::fs::write(dir.join("a.txt"), "dirty").unwrap();
        let reference = "refs/synara/checkpoints/t/turn/0";
        capture_tree_at_head(&dir, reference).unwrap();
        let content = git_stdout(&dir, &["show", &format!("{reference}:a.txt")]).unwrap();
        assert_eq!(content, "one", "base ref must mirror HEAD, not the dirty worktree");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
