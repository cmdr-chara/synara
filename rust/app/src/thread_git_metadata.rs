//! Port of upstream `threadGitMetadata.ts` + the capture side of
//! `ThreadGitMetadataReactor.ts`: at provider-turn boundaries, observe the
//! workspace branch, look up the branch-managed PR, derive the durable
//! metadata patch and apply it through the command path. Keeps the sidebar
//! branch/PR labels in sync with what the agent actually did.

use serde_json::{json, Map, Value};
use std::path::Path;
use std::process::Command;

const WORKTREE_BRANCH_PREFIX: &str = "synara";

fn is_temporary_worktree_branch(branch: &str) -> bool {
    let b = branch.trim().to_lowercase();
    // Upstream pattern: `<namespace>/<token>` where namespace is the worktree
    // branch prefix or a pre-cutover namespace.
    match b.split_once('/') {
        Some((ns, _)) => ns == WORKTREE_BRANCH_PREFIX || ns == "t3" || ns == "cua",
        None => false,
    }
}

/// Upstream `resolveThreadBranchRegressionGuard`: never regress a real branch
/// label to a temporary worktree branch.
fn guard_branch_regression(current: Option<&str>, next: Option<&str>) -> Option<String> {
    match (current, next) {
        (Some(c), Some(n))
            if !is_temporary_worktree_branch(c) && is_temporary_worktree_branch(n) =>
        {
            Some(c.to_string())
        }
        (_, n) => n.map(str::to_string),
    }
}

fn pr_equal(a: &Value, b: &Value) -> bool {
    if a.is_null() && b.is_null() {
        return true;
    }
    if a.is_null() || b.is_null() {
        return false;
    }
    let num = |v: &Value, k: &str| v.get(k).cloned().unwrap_or(Value::Null);
    num(a, "number") == num(b, "number")
        && num(a, "title") == num(b, "title")
        && num(a, "url") == num(b, "url")
        && num(a, "state") == num(b, "state")
}

pub(crate) struct MetadataPatch {
    pub branch: Option<Option<String>>,
    pub discovered_pr: Option<Value>, // Value::Null means "cleared"
    pub pr_changed: bool,
    pub associated_worktree_path: Option<String>,
    pub associated_worktree_branch: Option<String>,
}

/// Port of upstream `deriveThreadGitMetadataPatch`. `current_pr` is the
/// branch-managed (discovered) link only; explicit links never enter here.
pub(crate) fn derive_patch(
    current_branch: Option<&str>,
    current_pr: &Value,
    observed_branch: Option<&str>,
    lookup_resolved: bool,
    lookup_pr: &Value,
    dedicated_worktree: Option<(&str, Option<&str>, Option<&str>)>, // (cwd, current_path, current_branch)
) -> Option<MetadataPatch> {
    let guarded = guard_branch_regression(current_branch, observed_branch);
    if guarded.as_deref() != observed_branch {
        return None;
    }
    let branch_changed = current_branch != observed_branch;
    let mut next_pr = current_pr.clone();

    if dedicated_worktree.is_none() {
        // Shared checkout: a branch without a PR is not evidence the thread
        // lost its PR; only replace when a PR is actually found.
        if lookup_resolved && !lookup_pr.is_null() {
            next_pr = lookup_pr.clone();
        }
    } else if observed_branch.is_none() {
        next_pr = Value::Null;
    } else if lookup_resolved {
        next_pr = lookup_pr.clone();
    } else if branch_changed {
        next_pr = Value::Null;
    }

    let pr_changed = !pr_equal(current_pr, &next_pr);

    let mut associated_path: Option<String> = None;
    let mut associated_branch: Option<String> = None;
    if let Some((cwd, current_path, current_wt_branch)) = dedicated_worktree {
        if current_path != Some(cwd) {
            associated_path = Some(cwd.to_string());
        }
        if observed_branch.is_some() && current_wt_branch != observed_branch {
            associated_branch = observed_branch.map(str::to_string);
        }
    }

    if !branch_changed && !pr_changed && associated_path.is_none() && associated_branch.is_none() {
        return None;
    }

    Some(MetadataPatch {
        branch: if branch_changed {
            Some(observed_branch.map(str::to_string))
        } else {
            None
        },
        discovered_pr: if pr_changed { Some(next_pr) } else { None },
        pr_changed,
        associated_worktree_path: associated_path,
        associated_worktree_branch: associated_branch,
    })
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Look up the open PR for a branch, mirroring upstream
/// `gitManager.pullRequestForBranch`. Returns Err for "unavailable" (gh
/// missing/failing), Ok(Value::Null) for "resolved, no PR".
fn lookup_pr_for_branch(cwd: &Path, branch: &str) -> Result<Value, ()> {
    let out = Command::new("gh")
        .args([
            "pr",
            "list",
            "--head",
            branch,
            "--json",
            "number,title,url,state,headRefName,baseRefName,isDraft",
            "--limit",
            "1",
        ])
        .current_dir(cwd)
        .output()
        .map_err(|_| ())?;
    if !out.status.success() {
        return Err(());
    }
    let arr: Value = serde_json::from_slice(&out.stdout).map_err(|_| ())?;
    let Some(first) = arr.as_array().and_then(|a| a.first()) else {
        return Ok(Value::Null);
    };
    let state = match first.get("state").and_then(Value::as_str).unwrap_or("") {
        "OPEN" => "open",
        "MERGED" => "merged",
        "CLOSED" => "closed",
        _ => "unknown",
    };
    Ok(json!({
        "url": first.get("url").cloned().unwrap_or(Value::Null),
        "number": first.get("number").cloned().unwrap_or(Value::Null),
        "title": first.get("title").cloned().unwrap_or(Value::Null),
        "state": state,
        "draft": first.get("isDraft").cloned().unwrap_or(json!(false)),
        "headBranch": first.get("headRefName").cloned().unwrap_or(Value::Null),
        "baseBranch": first.get("baseRefName").cloned().unwrap_or(Value::Null),
    }))
}

fn now_iso() -> String {
    crate::todo::iso_now_ms()
}

/// Capture and reconcile durable git metadata for a thread. Invoked at turn
/// completion; runs off the runner hot path in a spawned thread.
pub(crate) fn spawn_capture(thread_id: String) {
    std::thread::spawn(move || {
        let _ = capture_inner(&thread_id);
    });
}

fn capture_inner(thread_id: &str) -> Result<(), ()> {
    let conn = crate::orchestration_engine::open_state_rw().map_err(|_| ())?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|_| ())?;

    let row: Option<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String, String)> = conn
        .query_row(
            "SELECT branch, worktree_path, associated_worktree_path, associated_worktree_branch, \
             working_directory, archived_at, COALESCE(pull_requests_json, '[]'), \
             COALESCE((SELECT workspace_root FROM projection_projects WHERE project_id = projection_threads.project_id), '') \
             FROM projection_threads WHERE thread_id = ?1",
            [thread_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )
        .ok();
    let Some((branch, worktree_path, assoc_path, assoc_branch, working_dir, archived, links_json, project_root)) = row else {
        return Err(());
    };
    if archived.is_some() {
        return Ok(());
    }
    let cwd_s = working_dir
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| worktree_path.clone().filter(|s| !s.is_empty()))
        .or_else(|| assoc_path.clone().filter(|s| !s.is_empty()))
        .or(if project_root.is_empty() { None } else { Some(project_root) });
    let Some(cwd_s) = cwd_s else { return Ok(()) };
    let cwd = Path::new(&cwd_s);
    if git_stdout(cwd, &["rev-parse", "--is-inside-work-tree"]).as_deref() != Some("true") {
        return Ok(());
    }

    let observed_raw = git_stdout(cwd, &["rev-parse", "--abbrev-ref", "HEAD"]);
    // Detached HEAD reports literally "HEAD" — upstream treats that as null.
    let observed = observed_raw.filter(|b| b != "HEAD");

    let has_dedicated = worktree_path.is_some();
    let (lookup_resolved, lookup_pr) = match &observed {
        None => (true, Value::Null),
        Some(b) => match lookup_pr_for_branch(cwd, b) {
            Ok(pr) => (true, pr),
            Err(()) => (false, Value::Null),
        },
    };

    // The branch-managed (discovered) PR currently linked.
    let links: Value = serde_json::from_str(&links_json).unwrap_or(json!([]));
    let current_pr = links
        .as_array()
        .and_then(|arr| {
            arr.iter().find(|l| {
                l.get("source").and_then(Value::as_str) == Some("discovered")
                    && l.get("state").and_then(Value::as_str) != Some("closed")
            })
        })
        .map(|l| l.pointer("/pullRequest").cloned().unwrap_or(Value::Null))
        .unwrap_or(Value::Null);

    let dedicated = if has_dedicated {
        Some((
            cwd_s.as_str(),
            assoc_path.as_deref(),
            assoc_branch.as_deref(),
        ))
    } else {
        None
    };

    let Some(patch) = derive_patch(
        branch.as_deref(),
        &current_pr,
        observed.as_deref(),
        lookup_resolved,
        &lookup_pr,
        dedicated,
    ) else {
        return Ok(());
    };

    // thread.meta.update for branch + associated worktree columns.
    let mut meta: Map<String, Value> = Map::new();
    let mut has_meta = false;
    if let Some(b) = &patch.branch {
        meta.insert("branch".into(), match b {
            Some(s) => json!(s),
            None => Value::Null,
        });
        has_meta = true;
    }
    if let Some(p) = &patch.associated_worktree_path {
        meta.insert("associatedWorktreePath".into(), json!(p));
        has_meta = true;
    }
    if let Some(b) = &patch.associated_worktree_branch {
        meta.insert("associatedWorktreeBranch".into(), json!(b));
        meta.insert("associatedWorktreeRef".into(), json!(b));
        has_meta = true;
    }
    if has_meta {
        let mut cmd = Map::new();
        cmd.insert("type".into(), json!("thread.meta.update"));
        cmd.insert(
            "commandId".into(),
            json!(format!("server:git-meta:{}", crate::orchestration_engine::uuid_v4())),
        );
        cmd.insert("threadId".into(), json!(thread_id));
        cmd.insert("createdAt".into(), json!(now_iso()));
        for (k, v) in meta {
            cmd.insert(k, v);
        }
        let _ = crate::orchestration_engine::handle(
            "orchestration.dispatchCommand",
            &Value::Object(cmd),
        );
    }

    // thread.pull-request.sync for the branch-managed link.
    if patch.pr_changed {
        let mut cmd = Map::new();
        cmd.insert("type".into(), json!("thread.pull-request.sync"));
        cmd.insert(
            "commandId".into(),
            json!(format!("server:git-meta:{}", crate::orchestration_engine::uuid_v4())),
        );
        cmd.insert("threadId".into(), json!(thread_id));
        cmd.insert("createdAt".into(), json!(now_iso()));
        if let Some(pr) = patch.discovered_pr {
            cmd.insert("discovered".into(), pr);
        }
        let _ = crate::orchestration_engine::handle(
            "orchestration.dispatchCommand",
            &Value::Object(cmd),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_checkout_never_clears_pr_on_empty_lookup() {
        let current = json!({"url": "https://github.com/o/r/pull/1", "number": 1});
        let patch = derive_patch(
            Some("main"),
            &current,
            Some("main"),
            true,
            &Value::Null, // no PR found on branch
            None,         // shared checkout
        );
        assert!(patch.is_none(), "no branch or PR change expected");
    }

    #[test]
    fn dedicated_worktree_clears_pr_when_branch_has_none() {
        let current = json!({"url": "https://github.com/o/r/pull/1", "number": 1});
        let patch = derive_patch(
            Some("feat"),
            &current,
            Some("feat"),
            true,
            &Value::Null,
            Some(("/wt", Some("/wt"), Some("feat"))),
        )
        .expect("pr change expected");
        assert!(patch.pr_changed);
        assert!(patch.discovered_pr.unwrap().is_null());
    }

    #[test]
    fn unavailable_lookup_preserves_pr_on_unchanged_branch() {
        let current = json!({"url": "https://github.com/o/r/pull/1", "number": 1});
        let patch = derive_patch(
            Some("feat"),
            &current,
            Some("feat"),
            false,
            &Value::Null,
            Some(("/wt", Some("/wt"), Some("feat"))),
        );
        assert!(patch.is_none());
    }

    #[test]
    fn unavailable_lookup_clears_pr_when_branch_changed() {
        let current = json!({"url": "https://github.com/o/r/pull/1", "number": 1});
        let patch = derive_patch(
            Some("feat"),
            &current,
            Some("other"),
            false,
            &Value::Null,
            Some(("/wt", Some("/wt"), Some("other"))),
        )
        .expect("branch change should surface");
        assert!(patch.pr_changed);
        assert_eq!(patch.branch, Some(Some("other".into())));
    }

    #[test]
    fn regression_guard_blocks_temporary_branch() {
        let patch = derive_patch(
            Some("main"),
            &Value::Null,
            Some("synara/abcd1234"),
            true,
            &Value::Null,
            None,
        );
        assert!(patch.is_none(), "temporary worktree branch must not regress label");
    }

    #[test]
    fn real_branch_change_is_applied() {
        let patch = derive_patch(
            Some("main"),
            &Value::Null,
            Some("feature-x"),
            true,
            &Value::Null,
            None,
        )
        .expect("branch change expected");
        assert_eq!(patch.branch, Some(Some("feature-x".into())));
    }

    #[test]
    fn associated_worktree_fields_patch_when_stale() {
        let patch = derive_patch(
            Some("feat"),
            &Value::Null,
            Some("feat"),
            true,
            &Value::Null,
            Some(("/wt/new", Some("/wt/old"), Some("old-branch"))),
        )
        .expect("worktree patch expected");
        assert_eq!(patch.associated_worktree_path.as_deref(), Some("/wt/new"));
        assert_eq!(patch.associated_worktree_branch.as_deref(), Some("feat"));
    }
}
