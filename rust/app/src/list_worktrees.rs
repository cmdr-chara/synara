//! `server.listWorktrees` — enumerates managed worktrees under
//! `<home>/worktrees/<repo>/<worktree>` (same layout `git_domain` writes into)
//! and resolves each worktree's main-repo root via `git rev-parse`.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use crate::git_domain::stdout;
use crate::todo::WsErr;

fn worktrees_root() -> PathBuf {
    Path::new(&crate::todo::state_dir())
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .join("worktrees")
}

fn workspace_root(worktree: &Path) -> Option<String> {
    // `rev-parse --git-common-dir` resolves `<main-repo>/.git` even for linked
    // worktrees whose own .git is a file under .git/worktrees/.
    let common = stdout(
        &worktree.to_string_lossy(),
        &["rev-parse", "--git-common-dir"],
        "git",
    )
    .ok()?;
    let common = common.trim().to_string();
    if common.is_empty() {
        return None;
    }
    let common_path = Path::new(common.as_str());
    let common_abs = if common_path.is_absolute() {
        common_path.to_path_buf()
    } else {
        worktree.join(common_path)
    };
    common_abs.parent().map(|p| p.to_string_lossy().to_string())
}

pub fn list_worktrees() -> Result<Value, WsErr> {
    let root = worktrees_root();
    let mut worktrees: Vec<Value> = Vec::new();
    let repos = match std::fs::read_dir(&root) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({ "worktrees": worktrees }));
        }
        Err(e) => {
            return Err(WsErr(
                format!("Cannot list worktrees in {}: {e}", root.display()),
                Some("FS_BROWSE_FAILED"),
            ));
        }
    };
    for repo in repos.flatten() {
        let repo_path = repo.path();
        if !repo_path.is_dir() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&repo_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(root) = workspace_root(&path) else {
                continue;
            };
            let normalized = std::fs::canonicalize(&path).unwrap_or(path);
            worktrees.push(json!({
                "path": normalized.to_string_lossy(),
                "workspaceRoot": root,
            }));
        }
    }
    worktrees.sort_by(|a, b| {
        a["path"]
            .as_str()
            .unwrap_or("")
            .cmp(b["path"].as_str().unwrap_or(""))
    });
    Ok(json!({ "worktrees": worktrees }))
}
