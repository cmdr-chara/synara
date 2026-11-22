// git.handoffThread — native port of
// apps/server/src/git/Layers/GitManager.ts (handoffThread + recovery helpers)
// and apps/server/src/gitHandoffOperations.ts (durable operation ledger).
// Pure git worktree/stash choreography; the durable ledger fences retries and
// the thread meta update is dispatched through the native engine.

use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

use crate::todo::WsErr;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), None)
}

fn mgr_err(detail: impl Into<String>) -> WsErr {
    err(format!("Git manager failed in handoffThread: {}", detail.into()))
}

pub fn handles(tag: &str) -> bool {
    tag == "git.handoffThread" && std::env::var("SYNARA_NO_UPSTREAM").is_ok()
}

pub fn handle(payload: &Value) -> Result<Value, WsErr> {
    handoff_thread(payload)
}

// ── Git primitives ──

fn git_out(cwd: &str, args: &[&str]) -> Result<std::process::Output, WsErr> {
    crate::git_domain::run(cwd, args)
}

fn git_ok(cwd: &str, args: &[&str]) -> Result<(), WsErr> {
    let out = git_out(cwd, args)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(err(stderr.trim().to_string()));
    }
    Ok(())
}

fn read_stash_ref(cwd: &str) -> Option<String> {
    let out = git_out(cwd, &["rev-parse", "--verify", "--quiet", "refs/stash"]).ok()?;
    if !out.status.success() {
        return None;
    }
    let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

fn read_head_ref(cwd: &str) -> Option<String> {
    let out = git_out(cwd, &["rev-parse", "HEAD"]).ok()?;
    let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

fn checkout_detached(cwd: &str, git_ref: &str) -> Result<(), WsErr> {
    git_ok(cwd, &["checkout", "--detach", git_ref])
}

fn checkout_branch(cwd: &str, branch: &str) -> Result<(), WsErr> {
    git_ok(cwd, &["checkout", branch])
}

fn status_details(cwd: &str) -> Result<(Option<String>, bool), WsErr> {
    let branch = crate::git_domain::current_branch(cwd);
    let out = git_out(cwd, &["status", "--porcelain=v1", "-z", "--untracked-files=all"])?;
    let dirty = !String::from_utf8_lossy(&out.stdout).trim_matches('\0').is_empty();
    Ok((branch, dirty))
}

fn list_local_branch_names(cwd: &str) -> Vec<String> {
    git_out(cwd, &["branch", "--list", "--format=%(refname:short)"])
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn combine_messages(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    let parts: Vec<String> = [
        String::from_utf8_lossy(stdout).trim().to_string(),
        String::from_utf8_lossy(stderr).trim().to_string(),
    ]
    .into_iter()
    .filter(|p| !p.is_empty())
    .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n").trim().to_string())
    }
}

struct Stash {
    had_changes: bool,
    stash_ref: Option<String>,
}

fn stash_working_tree(cwd: &str, label: &str) -> Result<Stash, WsErr> {
    let (_, dirty) = status_details(cwd)?;
    if !dirty {
        return Ok(Stash {
            had_changes: false,
            stash_ref: None,
        });
    }
    let before = read_stash_ref(cwd);
    git_ok(cwd, &["stash", "push", "--include-untracked", "-m", label])?;
    let after = read_stash_ref(cwd);
    if after == before {
        return Err(mgr_err(
            "Git did not create a stash entry while preparing the thread handoff.",
        ));
    }
    Ok(Stash {
        had_changes: true,
        stash_ref: after,
    })
}

fn drop_stash_by_sha(cwd: &str, sha: &str) {
    let Ok(out) = git_out(cwd, &["stash", "list", "--format=%H"]) else {
        return;
    };
    if !out.status.success() {
        return;
    }
    let index = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .position(|l| l == sha);
    if let Some(i) = index {
        let _ = git_out(cwd, &["stash", "drop", &format!("stash@{{{i}}}")]);
    }
}

struct PopResult {
    conflicts: bool,
    message: Option<String>,
}

/// Pop-style semantics over a stash captured by SHA: `stash apply` accepts any
/// stash-shaped commit, then drop the matching list entry on success.
fn pop_stash(cwd: &str, stash_ref: Option<&str>) -> PopResult {
    let Some(stash_ref) = stash_ref else {
        return PopResult {
            conflicts: false,
            message: None,
        };
    };
    let result = git_out(cwd, &["stash", "apply", "--index", stash_ref]);
    match result {
        Ok(out) if out.status.success() => {
            drop_stash_by_sha(cwd, stash_ref);
            PopResult {
                conflicts: false,
                message: None,
            }
        }
        Ok(out) => PopResult {
            conflicts: true,
            message: Some(
                combine_messages(&out.stdout, &out.stderr)
                    .unwrap_or_else(|| {
                        "Git reported conflicts while applying the handed off changes."
                            .to_string()
                    }),
            ),
        },
        Err(e) => PopResult {
            conflicts: true,
            message: Some(e.0),
        },
    }
}

fn restore_stashes(restores: &[(&str, Option<&str>)]) {
    for (cwd, r) in restores {
        let _ = pop_stash(cwd, *r);
    }
}

fn resolve_foreground_fallback_branch(cwd: &str, excluded: &str) -> Option<String> {
    let doc = crate::git_domain::list_branches(cwd).ok()?;
    let branches = doc.get("branches")?.as_array()?;
    let local: Vec<&Value> = branches
        .iter()
        .filter(|b| {
            b.get("isRemote").and_then(Value::as_bool) != Some(true)
                && b.get("name").and_then(Value::as_str) != Some(excluded)
                && b.get("worktreePath").map(Value::is_null).unwrap_or(true)
        })
        .collect();
    if let Some(d) = local
        .iter()
        .find(|b| b.get("isDefault").and_then(Value::as_bool) == Some(true))
    {
        return d.get("name").and_then(Value::as_str).map(String::from);
    }
    local
        .first()
        .and_then(|b| b.get("name").and_then(Value::as_str))
        .map(String::from)
}

fn build_named_worktree_path(cwd: &str, name: &str) -> String {
    let repo = std::path::Path::new(cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");
    let safe = name.trim().replace('/', "-");
    format!("{}/{repo}/{safe}", crate::git_domain::worktrees_dir())
}

struct CreatedWorktree {
    path: String,
    branch: Option<String>,
    git_ref: Option<String>,
}

fn create_worktree(
    cwd: &str,
    branch: &str,
    new_branch: Option<&str>,
    path: &str,
) -> Result<CreatedWorktree, WsErr> {
    let mut args: Vec<String> = vec!["worktree".into(), "add".into()];
    if let Some(nb) = new_branch {
        args.push("-b".into());
        args.push(nb.to_string());
    }
    args.push(path.to_string());
    args.push(branch.to_string());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    git_ok(cwd, &refs)?;
    Ok(CreatedWorktree {
        path: path.to_string(),
        branch: Some(new_branch.unwrap_or(branch).to_string()),
        git_ref: None,
    })
}

fn create_detached_worktree(
    cwd: &str,
    git_ref: &str,
    path: &str,
    new_branch: Option<&str>,
) -> Result<CreatedWorktree, WsErr> {
    let out = git_out(
        cwd,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{git_ref}^{{commit}}"),
        ],
    )?;
    let resolved = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if resolved.is_empty() {
        return Err(err(format!("git could not resolve ref '{git_ref}'")));
    }
    if let Some(nb) = new_branch {
        git_ok(cwd, &["branch", nb, &resolved])?;
        let add = git_out(cwd, &["worktree", "add", path, nb]);
        match add {
            Ok(o) if o.status.success() => {
                return Ok(CreatedWorktree {
                    path: path.to_string(),
                    branch: Some(nb.to_string()),
                    git_ref: Some(resolved),
                });
            }
            Ok(o) => {
                let _ = git_out(cwd, &["branch", "-D", nb]);
                return Err(err(
                    String::from_utf8_lossy(&o.stderr).trim().to_string(),
                ));
            }
            Err(e) => {
                let _ = git_out(cwd, &["branch", "-D", nb]);
                return Err(e);
            }
        }
    }
    git_ok(cwd, &["worktree", "add", "--detach", path, &resolved])?;
    Ok(CreatedWorktree {
        path: path.to_string(),
        branch: None,
        git_ref: Some(resolved),
    })
}

fn remove_worktree(cwd: &str, path: &str, force: bool) -> Result<(), WsErr> {
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(path);
    git_ok(cwd, &args)
}

fn resolve_push_remote_name(cwd: &str, branch: &str) -> Option<String> {
    let keyed = |key: &str| -> Option<String> {
        let out = git_out(cwd, &["config", "--get", key]).ok()?;
        let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    };
    if let Some(v) = keyed(&format!("branch.{branch}.pushRemote")) {
        return Some(v);
    }
    if let Some(v) = keyed("remote.pushDefault") {
        return Some(v);
    }
    let remotes: Vec<String> = git_out(cwd, &["remote"])
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    if remotes.iter().any(|r| r == "origin") {
        return Some("origin".to_string());
    }
    remotes.into_iter().next()
}

fn publish_branch(cwd: &str, branch: &str) -> Result<(), WsErr> {
    let Some(remote) = resolve_push_remote_name(cwd, branch) else {
        return Err(err(
            "Cannot publish branch because no git remote is configured for this repository.",
        ));
    };
    git_ok(cwd, &["push", "-u", &remote, branch])
}

// ── Recovery helpers ──

struct Recovery {
    checkout_restored: bool,
    stash_restored: bool,
    notes: Vec<String>,
}

fn restore_local_handoff_source(
    cwd: &str,
    original_branch: Option<&str>,
    original_head_ref: Option<&str>,
    current_branch: Option<&str>,
    stash_ref: Option<&str>,
) -> Recovery {
    let mut notes: Vec<String> = Vec::new();
    let mut checkout_restored = original_branch == current_branch;
    if let (Some(orig), Some(cur)) = (original_branch, current_branch) {
        if orig != cur {
            match checkout_branch(cwd, orig) {
                Ok(()) => checkout_restored = true,
                Err(e) => {
                    notes.push(format!(
                        "Local could not be returned to '{orig}': {}",
                        e.0
                    ));
                }
            }
        }
    } else if original_branch.is_none() {
        if let Some(head) = original_head_ref {
            match checkout_detached(cwd, head) {
                Ok(()) => checkout_restored = true,
                Err(e) => {
                    notes.push(format!(
                        "Local could not be returned to its previous detached HEAD: {}",
                        e.0
                    ));
                }
            }
        }
    }
    let pop = pop_stash(cwd, stash_ref);
    let stash_restored = !pop.conflicts;
    if pop.conflicts {
        notes.push(format!(
            "{}\nThe local stash entry was kept for recovery.",
            pop.message.unwrap_or_else(|| {
                "Git reported conflicts while restoring the original Local changes.".into()
            })
        ));
    }
    Recovery {
        checkout_restored,
        stash_restored,
        notes,
    }
}

struct RemovedWorktreeRecovery {
    worktree_recreated: bool,
    worktree_changes_restored: bool,
    local_changes_restored: bool,
    notes: Vec<String>,
}

fn restore_removed_worktree(
    cwd: &str,
    worktree_path: Option<&str>,
    branch: Option<&str>,
    git_ref: Option<&str>,
    worktree_stash_ref: Option<&str>,
    local_stash_ref: Option<&str>,
) -> RemovedWorktreeRecovery {
    let mut notes: Vec<String> = Vec::new();
    let mut recreated = false;
    let mut wt_restored = worktree_stash_ref.is_none();
    let local_restored;
    if let Some(wt_path) = worktree_path {
        let created: Result<CreatedWorktree, WsErr> = if let Some(b) = branch {
            create_worktree(cwd, b, None, wt_path)
        } else if let Some(r) = git_ref {
            create_detached_worktree(cwd, r, wt_path, None)
        } else {
            Err(err("no ref"))
        };
        match created {
            Ok(wt) => {
                recreated = true;
                let pop = pop_stash(&wt.path, worktree_stash_ref);
                wt_restored = !pop.conflicts;
                if pop.conflicts {
                    notes.push(format!(
                        "{}\nThe worktree stash entry was kept for recovery.",
                        pop.message.unwrap_or_else(|| {
                            "Git reported conflicts while restoring the recovered worktree changes."
                                .into()
                        })
                    ));
                }
            }
            Err(_) => {
                if worktree_stash_ref.is_some() {
                    notes.push(
                        "The thread worktree could not be recreated automatically. Its uncommitted changes were kept in the Git stash for manual recovery."
                            .to_string(),
                    );
                }
            }
        }
    }
    let pop = pop_stash(cwd, local_stash_ref);
    local_restored = !pop.conflicts;
    if pop.conflicts {
        notes.push(format!(
            "{}\nThe local stash entry was kept for recovery.",
            pop.message.unwrap_or_else(|| {
                "Git reported conflicts while restoring your previous local changes.".into()
            })
        ));
    }
    RemovedWorktreeRecovery {
        worktree_recreated: recreated,
        worktree_changes_restored: wt_restored,
        local_changes_restored: local_restored,
        notes,
    }
}

fn rollback_failed_local_transfer(
    cwd: &str,
    original_branch: Option<&str>,
    original_head_ref: Option<&str>,
    current_branch: Option<&str>,
    worktree_path: Option<&str>,
    worktree_branch: Option<&str>,
    worktree_ref: Option<&str>,
    worktree_stash_ref: Option<&str>,
    local_stash_ref: Option<&str>,
) -> Value {
    let wt = restore_removed_worktree(
        cwd,
        worktree_path,
        worktree_branch,
        worktree_ref,
        worktree_stash_ref,
        None,
    );
    let local = restore_local_handoff_source(
        cwd,
        original_branch,
        original_head_ref,
        current_branch,
        local_stash_ref,
    );
    let mut notes = wt.notes.clone();
    notes.extend(local.notes);
    json!({
        "worktreeRecreated": wt.worktree_recreated,
        "worktreeChangesRestored": wt.worktree_changes_restored,
        "localCheckoutRestored": local.checkout_restored,
        "localChangesRestored": local.stash_restored,
        "recoveryNotes": notes,
    })
}

fn rollback_failed_worktree_transfer(
    cwd: &str,
    worktree_path: &str,
    original_branch: Option<&str>,
    original_head_ref: Option<&str>,
    current_branch: Option<&str>,
    stash_ref: Option<&str>,
) -> Value {
    let mut notes: Vec<String> = Vec::new();
    let removed = match remove_worktree(cwd, worktree_path, true) {
        Ok(()) => true,
        Err(e) => {
            notes.push(format!(
                "The newly created worktree could not be removed automatically: {}",
                e.0
            ));
            false
        }
    };
    let local = restore_local_handoff_source(
        cwd,
        original_branch,
        original_head_ref,
        current_branch,
        stash_ref,
    );
    notes.extend(local.notes.clone());
    json!({
        "worktreeRemoved": removed,
        "checkoutRestored": local.checkout_restored,
        "stashRestored": local.stash_restored,
        "recoveryNotes": notes,
    })
}

fn failed_local_handoff_detail(base: &str, r: &RemovedWorktreeRecovery) -> String {
    let mut parts = vec![
        if r.worktree_recreated {
            "The original worktree was recreated.".to_string()
        } else {
            "The original worktree could not be recreated automatically.".to_string()
        },
        if r.worktree_changes_restored {
            "Recovered worktree changes were reapplied.".to_string()
        } else {
            "Recovered worktree changes remain in the Git stash.".to_string()
        },
        if r.local_changes_restored {
            "Previous local changes were restored.".to_string()
        } else {
            "Previous local changes remain in the Git stash.".to_string()
        },
    ];
    parts.extend(r.notes.iter().cloned());
    format!("{base} {}", parts.join(" ")).trim().to_string()
}

fn failed_local_transfer_detail(base: &str, r: &Value) -> String {
    let b = |k: &str| r.get(k).and_then(Value::as_bool).unwrap_or(false);
    let notes: Vec<String> = r
        .get("recoveryNotes")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let mut parts = vec![
        if b("worktreeRecreated") {
            "The original worktree was recreated.".to_string()
        } else {
            "The original worktree could not be recreated automatically.".to_string()
        },
        if b("worktreeChangesRestored") {
            "The thread changes were restored to that worktree.".to_string()
        } else {
            "The thread changes remain in the Git stash.".to_string()
        },
        if b("localCheckoutRestored") {
            "Local checkout was restored.".to_string()
        } else {
            "Local checkout could not be fully restored automatically.".to_string()
        },
        if b("localChangesRestored") {
            "Previous local changes were restored.".to_string()
        } else {
            "Previous local changes remain in the Git stash.".to_string()
        },
    ];
    parts.extend(notes.iter().cloned());
    format!("{base} {}", parts.join(" ")).trim().to_string()
}

fn failed_worktree_handoff_detail(base: &str, r: &Recovery) -> String {
    let mut parts = vec![
        if r.checkout_restored {
            "Local checkout was restored.".to_string()
        } else {
            "Local checkout could not be fully restored automatically.".to_string()
        },
        if r.stash_restored {
            "Previous local changes were restored.".to_string()
        } else {
            "Previous local changes remain in the Git stash.".to_string()
        },
    ];
    parts.extend(r.notes.iter().cloned());
    format!("{base} {}", parts.join(" ")).trim().to_string()
}

fn failed_worktree_transfer_detail(base: &str, r: &Value) -> String {
    let b = |k: &str| r.get(k).and_then(Value::as_bool).unwrap_or(false);
    let notes: Vec<String> = r
        .get("recoveryNotes")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let mut parts = vec![
        if b("worktreeRemoved") {
            "The new worktree was removed.".to_string()
        } else {
            "The new worktree could not be removed automatically.".to_string()
        },
        if b("checkoutRestored") {
            "Local checkout was restored.".to_string()
        } else {
            "Local checkout could not be fully restored automatically.".to_string()
        },
        if b("stashRestored") {
            "Previous local changes were restored.".to_string()
        } else {
            "Previous local changes remain in the Git stash. Run `git stash list` in Local to recover them.".to_string()
        },
    ];
    parts.extend(notes.iter().cloned());
    format!("{base} {}", parts.join(" ")).trim().to_string()
}

// ── Handoff intent ──

enum Intent {
    CreateNew {
        worktree_name: String,
        base_branch: Option<String>,
    },
    ReuseAssociated {
        path: Option<String>,
        branch: Option<String>,
        git_ref: Option<String>,
        base_branch: Option<String>,
    },
}

fn resolve_intent(input: &Value, local_branch: Option<&str>) -> Option<Intent> {
    let g = |k: &str| input.get(k).and_then(Value::as_str).map(String::from);
    let name = g("preferredNewWorktreeName")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let base = g("preferredWorktreeBaseBranch")
        .or_else(|| g("currentBranch"))
        .or_else(|| local_branch.map(String::from));
    if !name.is_empty() {
        return Some(Intent::CreateNew {
            worktree_name: name,
            base_branch: base,
        });
    }
    let path = g("associatedWorktreePath");
    let branch = g("associatedWorktreeBranch");
    let git_ref = g("associatedWorktreeRef");
    if path.is_none() && branch.is_none() && git_ref.is_none() {
        return None;
    }
    Some(Intent::ReuseAssociated {
        path,
        branch,
        git_ref,
        base_branch: base,
    })
}

fn ns(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

// ── Ledger (gitHandoffOperations.ts) ──

fn ledger_conn() -> Result<Connection, WsErr> {
    crate::orchestration_engine::open_state_rw()
}

struct ExistingOp {
    thread_id: String,
    input_json: String,
    phase: String,
    result_json: Option<String>,
}

fn read_operation(conn: &Connection, command_id: &str) -> Option<ExistingOp> {
    conn.query_row(
        "SELECT thread_id, input_json, phase, result_json FROM git_handoff_operations \
         WHERE command_id = ?1",
        params![command_id],
        |r| {
            Ok(ExistingOp {
                thread_id: r.get(0)?,
                input_json: r.get(1)?,
                phase: r.get(2)?,
                result_json: r.get(3)?,
            })
        },
    )
    .ok()
}

// ── Main flow ──

fn handoff_thread(payload: &Value) -> Result<Value, WsErr> {
    let command_id = ns(payload, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = ns(payload, "threadId").ok_or_else(|| err("missing threadId"))?;
    let input_json = serde_json::to_string(payload).unwrap_or_default();

    // beginGitHandoff: fence by durable command identity.
    let conn = ledger_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let now = crate::todo::iso_now_ms();
    let inserted = conn
        .execute(
            "INSERT INTO git_handoff_operations \
             (command_id, thread_id, input_json, phase, result_json, created_at, updated_at) \
             VALUES (?1, ?2, ?3, 'pending', NULL, ?4, ?4) \
             ON CONFLICT (command_id) DO NOTHING",
            params![command_id, thread_id, input_json, now],
        )
        .map_err(|e| err(e.to_string()))?;
    if inserted == 0 {
        let existing = read_operation(&conn, &command_id);
        let Some(op) = existing else {
            return Err(err(format!(
                "Git handoff command identity {command_id} was reused with different input."
            )));
        };
        if op.thread_id != thread_id || op.input_json != input_json {
            return Err(err(format!(
                "Git handoff command identity {command_id} was reused with different input."
            )));
        }
        match op.phase.as_str() {
            "pending" => {
                return Err(err("This Git handoff is already running."));
            }
            "uncertain" => {
                return Err(err(
                    "This Git handoff was interrupted before its filesystem result became durable; inspect the repository before retrying.",
                ));
            }
            _ => {
                // git_applied | completed — replay the stored result.
                if let Some(rj) = op.result_json {
                    if let Ok(result) = serde_json::from_str::<Value>(&rj) {
                        return Ok(result);
                    }
                }
                return Err(err("Git handoff result could not be decoded."));
            }
        }
    }

    let git_result = run_git_handoff(payload);
    let result = match git_result {
        Ok(r) => r,
        Err(e) => {
            // discardPendingGitHandoff — the pending row only survives a
            // successful git application.
            let _ = conn.execute(
                "DELETE FROM git_handoff_operations \
                 WHERE command_id = ?1 AND phase = 'pending'",
                params![command_id],
            );
            return Err(e);
        }
    };
    let _ = conn.execute(
        "UPDATE git_handoff_operations SET phase = 'git_applied', result_json = ?2, \
         updated_at = ?3 WHERE command_id = ?1 AND phase = 'pending'",
        params![
            command_id,
            serde_json::to_string(&result).unwrap_or_default(),
            crate::todo::iso_now_ms()
        ],
    );

    // gitHandoffMetadataCommand → thread.meta.update.
    let mut meta = Map::new();
    meta.insert("type".into(), json!("thread.meta.update"));
    meta.insert("commandId".into(), json!(command_id));
    meta.insert("threadId".into(), json!(thread_id));
    for key in [
        "branch",
        "worktreePath",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
    ] {
        meta.insert(key.into(), result.get(key).cloned().unwrap_or(Value::Null));
    }
    meta.insert(
        "envMode".into(),
        result.get("targetMode").cloned().unwrap_or(Value::Null),
    );
    if result.get("targetMode").and_then(Value::as_str) == Some("worktree") {
        meta.insert("createBranchFlowCompleted".into(), json!(false));
    }
    let _ = crate::orchestration_engine::handle(
        "orchestration.dispatchCommand",
        &Value::Object(meta),
    );

    let _ = conn.execute(
        "UPDATE git_handoff_operations SET phase = 'completed', updated_at = ?2 \
         WHERE command_id = ?1 AND phase = 'git_applied'",
        params![command_id, crate::todo::iso_now_ms()],
    );
    Ok(result)
}

/// GitManager.handoffThread — local | worktree transfer choreography.
fn run_git_handoff(input: &Value) -> Result<Value, WsErr> {
    let cwd = ns(input, "cwd").ok_or_else(|| err("missing cwd"))?;
    let target_mode = ns(input, "targetMode").ok_or_else(|| err("missing targetMode"))?;
    let (local_branch, _) = status_details(&cwd)?;

    if target_mode == "local" {
        return handoff_to_local(input, &cwd, local_branch);
    }
    handoff_to_worktree(input, &cwd, local_branch)
}

fn handoff_to_local(
    input: &Value,
    cwd: &str,
    local_branch: Option<String>,
) -> Result<Value, WsErr> {
    let worktree_path = ns(input, "worktreePath").ok_or_else(|| {
        mgr_err(
            "Cannot hand off to Local because this thread does not have a materialized worktree.",
        )
    })?;
    let worktree_head = read_head_ref(&worktree_path);
    let target_local_branch = ns(input, "currentBranch")
        .or_else(|| ns(input, "associatedWorktreeBranch"))
        .or_else(|| ns(input, "preferredLocalBranch"));
    if target_local_branch.is_none() && worktree_head.is_none() {
        return Err(mgr_err(
            "Cannot hand off to Local because the worktree thread does not have a recoverable HEAD reference.",
        ));
    }
    let associated_path = ns(input, "associatedWorktreePath").or_else(|| Some(worktree_path.clone()));
    let associated_branch =
        ns(input, "associatedWorktreeBranch").or_else(|| ns(input, "currentBranch"));
    let associated_ref = ns(input, "associatedWorktreeRef")
        .or_else(|| worktree_head.clone())
        .or_else(|| associated_branch.clone());
    let original_local_branch = local_branch.clone();
    let original_local_head = read_head_ref(cwd);
    let mut branch_after_prep = original_local_branch.clone();

    let preserved_local = stash_working_tree(
        cwd,
        &format!("synara preserve local handoff {}", crate::orchestration_engine::uuid_v4()),
    )?;
    let source = stash_working_tree(
        &worktree_path,
        &format!("synara handoff to local {}", crate::orchestration_engine::uuid_v4()),
    )?;

    if let Err(e) = remove_worktree(cwd, &worktree_path, false) {
        restore_stashes(&[
            (&worktree_path, source.stash_ref.as_deref()),
            (cwd, preserved_local.stash_ref.as_deref()),
        ]);
        return Err(e);
    }

    if let Some(target) = &target_local_branch {
        if local_branch.as_deref() != Some(target.as_str()) {
            if let Err(e) = checkout_branch(cwd, target) {
                let rec = restore_removed_worktree(
                    cwd,
                    associated_path.as_deref(),
                    associated_branch.as_deref(),
                    associated_ref.as_deref(),
                    source.stash_ref.as_deref(),
                    preserved_local.stash_ref.as_deref(),
                );
                return Err(mgr_err(failed_local_handoff_detail(&e.0, &rec)));
            }
            branch_after_prep = Some(target.clone());
        }
    } else if let Some(head) = &worktree_head {
        if let Err(e) = checkout_detached(cwd, head) {
            let rec = restore_removed_worktree(
                cwd,
                associated_path.as_deref(),
                associated_branch.as_deref(),
                associated_ref.as_deref(),
                source.stash_ref.as_deref(),
                preserved_local.stash_ref.as_deref(),
            );
            return Err(mgr_err(failed_local_handoff_detail(&e.0, &rec)));
        }
        branch_after_prep = None;
    }

    let thread_transfer = pop_stash(cwd, source.stash_ref.as_deref());
    if thread_transfer.conflicts {
        let recovery = rollback_failed_local_transfer(
            cwd,
            original_local_branch.as_deref(),
            original_local_head.as_deref(),
            branch_after_prep.as_deref(),
            associated_path.as_deref(),
            associated_branch.as_deref(),
            associated_ref.as_deref(),
            source.stash_ref.as_deref(),
            preserved_local.stash_ref.as_deref(),
        );
        return Err(mgr_err(failed_local_transfer_detail(
            &format!(
                "{} The handoff was rolled back so the thread stays in its worktree.",
                thread_transfer.message.unwrap_or_else(|| {
                    "Git reported conflicts while applying the handed off changes.".into()
                })
            ),
            &recovery,
        )));
    }

    let local_transfer = pop_stash(cwd, preserved_local.stash_ref.as_deref());
    let moved = source.had_changes;
    let restored = preserved_local.had_changes;
    let label = match &target_local_branch {
        Some(b) => format!("main local checkout on '{b}'"),
        None => "local checkout in detached HEAD".to_string(),
    };
    let message = if local_transfer.conflicts {
        format!(
            "{}\nYour previous local stash entry was kept for recovery.",
            local_transfer.message.unwrap_or_else(|| {
                "Git reported conflicts while restoring your previous local changes.".into()
            })
        )
    } else if moved && restored {
        format!("Moved the thread back to the {label}, carried its uncommitted work over, and restored your previous local changes.")
    } else if moved {
        format!("Moved the thread back to the {label} and carried its uncommitted work over.")
    } else if restored {
        format!("Moved the thread back to the {label} and restored your previous local changes.")
    } else {
        format!("Moved the thread back to the {label}.")
    };

    Ok(json!({
        "targetMode": "local",
        "branch": target_local_branch,
        "worktreePath": Value::Null,
        "associatedWorktreePath": associated_path,
        "associatedWorktreeBranch": associated_branch,
        "associatedWorktreeRef": associated_ref,
        "changesTransferred": source.had_changes || preserved_local.had_changes,
        "conflictsDetected": local_transfer.conflicts,
        "message": message,
    }))
}

fn handoff_to_worktree(
    input: &Value,
    cwd: &str,
    local_branch: Option<String>,
) -> Result<Value, WsErr> {
    let intent = resolve_intent(input, local_branch.as_deref()).ok_or_else(|| {
        mgr_err("Cannot hand off to a worktree because no worktree target is available.")
    })?;
    let (target_name, assoc_path, assoc_branch, assoc_ref, base_branch) = match &intent {
        Intent::CreateNew {
            worktree_name,
            base_branch,
        } => (
            Some(worktree_name.clone()),
            None,
            None,
            None,
            base_branch.clone(),
        ),
        Intent::ReuseAssociated {
            path,
            branch,
            git_ref,
            base_branch,
        } => (
            None,
            path.clone(),
            branch.clone(),
            git_ref.clone(),
            base_branch.clone(),
        ),
    };
    if base_branch.is_none() && assoc_branch.is_none() && assoc_ref.is_none() {
        return Err(mgr_err(
            "Select a base branch before handing off this thread to a worktree.",
        ));
    }

    let source = stash_working_tree(
        cwd,
        &format!("synara handoff to worktree {}", crate::orchestration_engine::uuid_v4()),
    )?;
    let source_branch = local_branch.clone().or_else(|| ns(input, "currentBranch"));
    let source_head = read_head_ref(cwd);
    let mut foreground_branch = local_branch.clone();

    if let Some(sb) = &source_branch {
        if Some(sb.clone()) == assoc_branch {
            let fallback = resolve_foreground_fallback_branch(cwd, sb);
            match fallback {
                None => {
                    let Some(head) = &source_head else {
                        let _ = pop_stash(cwd, source.stash_ref.as_deref());
                        return Err(mgr_err(format!(
                            "Cannot hand off '{sb}' to a worktree because there is no recoverable local HEAD reference available."
                        )));
                    };
                    if let Err(e) = checkout_detached(cwd, head) {
                        let _ = pop_stash(cwd, source.stash_ref.as_deref());
                        return Err(e);
                    }
                    foreground_branch = None;
                }
                Some(fb) => {
                    if let Err(e) = checkout_branch(cwd, &fb) {
                        let _ = pop_stash(cwd, source.stash_ref.as_deref());
                        return Err(e);
                    }
                    foreground_branch = Some(fb);
                }
            }
        }
    }

    let created: Result<CreatedWorktree, WsErr> = (|| {
        if assoc_ref.is_some() && assoc_branch.is_none() {
            let path = assoc_path.clone().unwrap_or_else(|| {
                build_named_worktree_path(cwd, assoc_ref.as_deref().unwrap_or("worktree"))
            });
            return create_detached_worktree(
                cwd,
                assoc_ref.as_deref().unwrap(),
                &path,
                None,
            );
        }
        if let Some(name) = &target_name {
            let Some(base) = &base_branch else {
                return Err(mgr_err(
                    "Select a base branch before creating a new worktree.",
                ));
            };
            let path = assoc_path
                .clone()
                .unwrap_or_else(|| build_named_worktree_path(cwd, name));
            return create_worktree(cwd, base, Some(name), &path);
        }
        if let Some(ab) = &assoc_branch {
            let path = assoc_path
                .clone()
                .unwrap_or_else(|| build_named_worktree_path(cwd, ab));
            if list_local_branch_names(cwd).iter().any(|n| n == ab) {
                return create_worktree(cwd, ab, None, &path);
            }
            let Some(base) = &base_branch else {
                return create_detached_worktree(cwd, ab, &path, None);
            };
            return create_worktree(cwd, base, Some(ab), &path);
        }
        let Some(base) = &base_branch else {
            return create_detached_worktree(
                cwd,
                assoc_ref.as_deref().unwrap_or("HEAD"),
                assoc_path.as_deref().unwrap_or("/tmp"),
                None,
            );
        };
        let path = assoc_path.clone().unwrap_or_else(|| {
            build_named_worktree_path(cwd, target_name.as_deref().unwrap_or(base))
        });
        create_detached_worktree(cwd, base, &path, target_name.as_deref())
    })();

    let worktree = match created {
        Ok(w) => w,
        Err(e) => {
            let recovery = restore_local_handoff_source(
                cwd,
                source_branch.as_deref(),
                source_head.as_deref(),
                foreground_branch.as_deref(),
                source.stash_ref.as_deref(),
            );
            return Err(mgr_err(failed_worktree_handoff_detail(&e.0, &recovery)));
        }
    };

    let transfer = pop_stash(&worktree.path, source.stash_ref.as_deref());
    if transfer.conflicts {
        let recovery = rollback_failed_worktree_transfer(
            cwd,
            &worktree.path,
            source_branch.as_deref(),
            source_head.as_deref(),
            foreground_branch.as_deref(),
            source.stash_ref.as_deref(),
        );
        return Err(mgr_err(failed_worktree_transfer_detail(
            &format!(
                "{} The stash entry was kept for recovery.",
                transfer.message.unwrap_or_else(|| {
                    "Git reported conflicts while applying the handed off changes.".into()
                })
            ),
            &recovery,
        )));
    }

    let (mat_branch, _) = status_details(&worktree.path).unwrap_or((None, false));
    let mat_ref = read_head_ref(&worktree.path)
        .or_else(|| worktree.git_ref.clone())
        .or_else(|| worktree.branch.clone());
    if let Some(b) = &mat_branch {
        // Publishing is best-effort for local-only repositories.
        let _ = publish_branch(&worktree.path, b);
    }
    let summary = if let Some(fb) = &foreground_branch {
        if Some(fb.clone()) != source_branch {
            format!("The thread moved into its worktree and Local returned to '{fb}'.")
        } else {
            "The thread moved into its worktree.".to_string()
        }
    } else if source_branch.is_some() && source_branch == assoc_branch {
        "The thread moved into its worktree and Local returned to a detached HEAD.".to_string()
    } else {
        "The thread moved into its worktree.".to_string()
    };
    let message = if source.had_changes {
        format!("{summary} Uncommitted local changes were carried over.")
    } else {
        summary
    };

    Ok(json!({
        "targetMode": "worktree",
        "branch": mat_branch,
        "worktreePath": worktree.path,
        "associatedWorktreePath": worktree.path,
        "associatedWorktreeBranch": mat_branch,
        "associatedWorktreeRef": mat_ref,
        "changesTransferred": source.had_changes,
        "conflictsDetected": false,
        "message": message,
    }))
}
