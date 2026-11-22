//! Native `git.*` WS-RPC domain. Every method is cwd-scoped and backed by the
//! `git` CLI (`git -C <cwd> ...`), matching the `packages/contracts/src/git.ts`
//! result schemas. Methods needing `gh`, streams, or the orchestration store
//! (resolvePullRequest, pullRequestSnapshot, preparePullRequestThread,
//! handoffThread, runStackedAction, createDetachedWorktree, summarizeDiff)
//! still relay upstream.

use serde_json::{Value, json};
use std::process::Command;

use crate::todo::WsErr;

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

/// Patch payloads are capped so a huge diff cannot pin the socket; the client
/// renders `truncated` instead of the tail.
const DIFF_MAX_BYTES: usize = 512_000;
const READ_FILE_MAX_BYTES: usize = 1_000_000;
/// `git diff <empty-tree>` base for repos with no commits / no merge base.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

const HANDLED: &[&str] = &[
    "git.status",
    "git.githubRepository",
    "git.readWorkingTreeDiff",
    "git.blameLine",
    "git.readFileAtRev",
    "git.workingTreeDiffStats",
    "git.listBranches",
    "git.listRecentCommits",
    "git.stashInfo",
    "git.pull",
    "git.init",
    "git.stageFiles",
    "git.unstageFiles",
    "git.checkout",
    "git.createBranch",
    "git.stashAndCheckout",
    "git.stashDrop",
    "git.removeIndexLock",
    "git.createWorktree",
    "git.removeWorktree",
];

pub fn handles(tag: &str) -> bool {
    HANDLED.contains(&tag)
}

fn cwd_of(payload: &Value) -> Result<String, WsErr> {
    payload
        .get("cwd")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err("`cwd` is required"))
}

fn str_arg<'a>(payload: &'a Value, key: &str) -> Result<&'a str, WsErr> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err(format!("`{key}` is required")))
}

fn rev_ok(rev: &str) -> bool {
    !rev.is_empty() && rev.len() <= 256 && !rev.starts_with('-')
}

fn run(cwd: &str, args: &[&str]) -> Result<std::process::Output, WsErr> {
    Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|e| err(format!("failed to spawn git: {e}")))
}

pub(crate) fn stdout(cwd: &str, args: &[&str], what: &str) -> Result<String, WsErr> {
    let out = run(cwd, args)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(err(format!(
            "{what}: {}",
            stderr.trim().lines().next().unwrap_or("git failed")
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn ok(cwd: &str, args: &[&str]) -> bool {
    run(cwd, args).map(|o| o.status.success()).unwrap_or(false)
}

fn require_repo(cwd: &str) -> Result<(), WsErr> {
    if ok(cwd, &["rev-parse", "--is-inside-work-tree"]) {
        Ok(())
    } else {
        Err(err(format!("`{cwd}` is not a git repository")))
    }
}

fn current_branch(cwd: &str) -> Option<String> {
    let b = stdout(cwd, &["branch", "--show-current"], "git").ok()?;
    let b = b.trim();
    (!b.is_empty()).then(|| b.to_string())
}

fn upstream_branch(cwd: &str) -> Option<String> {
    let u = stdout(cwd, &["rev-parse", "--abbrev-ref", "@{upstream}"], "git").ok()?;
    let u = u.trim();
    (!u.is_empty()).then(|| u.to_string())
}

fn has_head(cwd: &str) -> bool {
    ok(cwd, &["rev-parse", "--verify", "-q", "HEAD"])
}

/// Base for branch-scope diffs and `base:"branch"` reads: merge-base with the
/// upstream, else the default branch, else the empty tree.
fn branch_base(cwd: &str) -> String {
    if let Some(up) = upstream_branch(cwd) {
        if let Ok(mb) = stdout(cwd, &["merge-base", "HEAD", &up], "git") {
            let mb = mb.trim();
            if !mb.is_empty() {
                return mb.to_string();
            }
        }
    }
    if let Some(def) = default_branch_ref(cwd) {
        if let Ok(mb) = stdout(cwd, &["merge-base", "HEAD", &def], "git") {
            let mb = mb.trim();
            if !mb.is_empty() {
                return mb.to_string();
            }
        }
    }
    EMPTY_TREE.to_string()
}

/// Default branch ref for diff/base purposes: `origin/HEAD` target, else a
/// local `main`/`master`.
fn default_branch_ref(cwd: &str) -> Option<String> {
    if let Ok(t) = stdout(
        cwd,
        &["symbolic-ref", "-q", "--short", "refs/remotes/origin/HEAD"],
        "git",
    ) {
        let t = t.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    for name in ["main", "master"] {
        if ok(
            cwd,
            &["show-ref", "--verify", "-q", &format!("refs/heads/{name}")],
        ) {
            return Some(name.to_string());
        }
    }
    None
}

fn diff_args(scope: &str, compare_ref: Option<&str>, cwd: &str) -> Vec<String> {
    match scope {
        "unstaged" => vec!["diff".to_string()],
        "staged" => vec!["diff".to_string(), "--cached".to_string()],
        "branch" => vec!["diff".to_string(), format!("{}...HEAD", branch_base(cwd))],
        "ref" => {
            let r = compare_ref.unwrap_or("HEAD");
            vec!["diff".to_string(), r.to_string()]
        }
        // "workingTree" — staged + unstaged vs HEAD (or vs the empty tree when
        // the repo has no commits yet).
        _ => {
            if has_head(cwd) {
                vec!["diff".to_string(), "HEAD".to_string()]
            } else {
                vec!["diff".to_string(), EMPTY_TREE.to_string()]
            }
        }
    }
}

fn numstat(
    cwd: &str,
    scope: &str,
    compare_ref: Option<&str>,
    file: Option<&str>,
) -> Result<Vec<(String, u64, u64)>, WsErr> {
    let mut args = diff_args(scope, compare_ref, cwd);
    args.insert(1, "--numstat".to_string());
    if let Some(f) = file {
        args.push("--".to_string());
        args.push(f.to_string());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = stdout(cwd, &refs, "git diff failed")?;
    let mut rows = Vec::new();
    for line in out.lines() {
        let mut parts = line.splitn(3, '\t');
        let (Some(a), Some(d), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        // Binary files show `-` counts.
        let a = a.parse::<u64>().unwrap_or(0);
        let d = d.parse::<u64>().unwrap_or(0);
        rows.push((path.trim().to_string(), a, d));
    }
    Ok(rows)
}

/// File names with any working-tree change, including untracked entries the
/// numstat misses.
fn changed_paths(cwd: &str) -> Vec<String> {
    let out = stdout(
        cwd,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        "git",
    )
    .unwrap_or_default();
    let mut paths = Vec::new();
    for line in out.lines() {
        if line.len() < 4 {
            continue;
        }
        let p = line[3..].trim();
        // `R old -> new` keeps the destination.
        let p = p.rsplit(" -> ").next().unwrap_or(p);
        if !p.is_empty() {
            paths.push(p.trim_matches('"').to_string());
        }
    }
    paths
}

fn untracked_paths(cwd: &str) -> Vec<String> {
    let out = stdout(
        cwd,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        "git",
    )
    .unwrap_or_default();
    out.lines()
        .filter(|l| l.starts_with("??"))
        .map(|l| l[3..].trim().trim_matches('"').to_string())
        .collect()
}

fn git_status(cwd: &str) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let branch = current_branch(cwd);
    let upstream = upstream_branch(cwd);
    let (ahead, behind) = if upstream.is_some() {
        let out = stdout(
            cwd,
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
            "git rev-list failed",
        )
        .unwrap_or_default();
        let mut it = out.split_whitespace();
        (
            it.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0),
            it.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0),
        )
    } else {
        (0, 0)
    };

    let rows = numstat(cwd, "workingTree", None, None)?;
    let mut files: Vec<Value> = rows
        .into_iter()
        .map(|(path, insertions, deletions)| json!({"path": path, "insertions": insertions, "deletions": deletions}))
        .collect();
    let mut additions: u64 = files
        .iter()
        .map(|f| f["insertions"].as_u64().unwrap_or(0))
        .sum();
    let deletions: u64 = files
        .iter()
        .map(|f| f["deletions"].as_u64().unwrap_or(0))
        .sum();
    // Untracked files contribute their line count as insertions.
    for path in untracked_paths(cwd) {
        let full = std::path::Path::new(cwd).join(&path);
        let n = std::fs::read(&full)
            .map(|b| {
                let s = String::from_utf8_lossy(&b);
                if s.is_empty() {
                    0
                } else {
                    s.lines().count() as u64
                }
            })
            .unwrap_or(0);
        additions += n;
        files.push(json!({"path": path, "insertions": n, "deletions": 0}));
    }
    let dirty = !files.is_empty() || !changed_paths(cwd).is_empty();

    Ok(json!({
        "branch": branch,
        "hasWorkingTreeChanges": dirty,
        "workingTree": {
            "files": files,
            "insertions": additions,
            "deletions": deletions,
        },
        "hasUpstream": upstream.is_some(),
        "upstreamBranch": upstream,
        "configuredPrBaseBranch": Value::Null,
        "aheadCount": ahead,
        "behindCount": behind,
        // `gh`-backed resolution stays upstream until ported.
        "pr": Value::Null,
    }))
}

fn github_repository(cwd: &str) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let out = stdout(cwd, &["remote", "-v"], "git remote failed")?;
    let mut repos: Vec<(String, String)> = Vec::new();
    let mut first: Option<usize> = None;
    for line in out.lines() {
        let mut parts = line.split_whitespace();
        let (Some(name), Some(url)) = (parts.next(), parts.next()) else {
            continue;
        };
        // First remote wins only for `origin`; keep the primary slot for it.
        let Some(nwo) = parse_github_nwo(url) else {
            continue;
        };
        let entry = (nwo.clone(), format!("https://github.com/{nwo}"));
        if !repos.iter().any(|(n, _)| *n == nwo) {
            repos.push(entry);
            if name == "origin" && first.is_none() {
                first = Some(repos.len() - 1);
            }
        }
    }
    let primary = first
        .and_then(|i| repos.get(i).cloned())
        .or_else(|| repos.first().cloned());
    Ok(json!({
        "repository": primary.map(|(name_with_owner, url)| json!({
            "nameWithOwner": name_with_owner,
            "url": url,
        })),
        "repositories": repos.iter().map(|(n, u)| json!({
            "nameWithOwner": n, "url": u,
        })).collect::<Vec<_>>(),
    }))
}

/// `owner/repo` out of https, ssh and git URL shapes.
fn parse_github_nwo(url: &str) -> Option<String> {
    let u = url.trim().trim_end_matches(".git").trim_end_matches('/');
    let path = if let Some(rest) = u.strip_prefix("git@github.com:") {
        rest
    } else if let Some(rest) = u.strip_prefix("ssh://git@github.com/") {
        rest
    } else if u.starts_with("https://github.com/") || u.starts_with("http://github.com/") {
        u.split("github.com/").nth(1)?
    } else {
        return None;
    };
    let mut segs = path.split('/');
    let (Some(owner), Some(repo)) = (segs.next(), segs.next()) else {
        return None;
    };
    if segs.next().is_some() || owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

fn read_working_tree_diff(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let scope = payload
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("workingTree");
    let compare_ref = payload.get("compareRef").and_then(Value::as_str);
    if let Some(r) = compare_ref {
        if !rev_ok(r) {
            return Err(err("invalid `compareRef`"));
        }
    }
    let file = payload.get("filePath").and_then(Value::as_str);
    let mut args = diff_args(scope, compare_ref, cwd);
    if let Some(f) = file {
        args.push("--".to_string());
        args.push(f.to_string());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run(cwd, &refs)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(err(format!(
            "git diff failed: {}",
            stderr.trim().lines().next().unwrap_or("git failed")
        )));
    }
    let bytes = out.stdout;
    let truncated = bytes.len() > DIFF_MAX_BYTES;
    let cut = if truncated {
        let mut end = DIFF_MAX_BYTES;
        while end > 0 && bytes[end - 1] != b'\n' {
            end -= 1;
        }
        if end == 0 { DIFF_MAX_BYTES } else { end }
    } else {
        bytes.len()
    };
    Ok(json!({
        "patch": String::from_utf8_lossy(&bytes[..cut]),
        "truncated": truncated,
    }))
}

fn working_tree_diff_stats(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let scope = payload
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("workingTree");
    let compare_ref = payload.get("compareRef").and_then(Value::as_str);
    let file = payload.get("filePath").and_then(Value::as_str);
    let rows = numstat(cwd, scope, compare_ref, file)?;
    let additions: u64 = rows.iter().map(|(_, a, _)| a).sum();
    let deletions: u64 = rows.iter().map(|(_, _, d)| d).sum();
    Ok(json!({
        "additions": additions,
        "deletions": deletions,
        "fileCount": rows.len(),
    }))
}

/// Seconds since epoch → `YYYY-MM-DDTHH:MM:SSZ` (UTC).
fn unix_to_iso(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn blame_line(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let file = str_arg(payload, "filePath")?.to_string();
    let line = payload
        .get("line")
        .and_then(Value::as_u64)
        .filter(|l| *l >= 1)
        .ok_or_else(|| err("`line` must be a positive integer"))?;
    let rev = if payload.get("base").and_then(Value::as_str) == Some("branch") {
        Some(branch_base(cwd))
    } else {
        payload.get("rev").and_then(Value::as_str).map(String::from)
    };
    if let Some(r) = &rev {
        if !rev_ok(r) {
            return Err(err("invalid `rev`"));
        }
    }
    let range = format!("{line},{line}");
    let mut args: Vec<&str> = vec!["blame", "-L", &range, "--porcelain"];
    if let Some(r) = &rev {
        args.push(r);
    }
    args.push("--");
    args.push(&file);
    let out = stdout(cwd, &args, "git blame failed")?;
    let mut sha = String::new();
    let mut author = String::new();
    let mut email = String::new();
    let mut time = String::new();
    let mut summary = String::new();
    for l in out.lines() {
        if sha.is_empty() {
            sha = l.split_whitespace().next().unwrap_or("").to_string();
            continue;
        }
        if let Some(v) = l.strip_prefix("author ") {
            author = v.to_string();
        } else if let Some(v) = l.strip_prefix("author-mail ") {
            email = v.trim_start_matches('<').trim_end_matches('>').to_string();
        } else if let Some(v) = l.strip_prefix("author-time ") {
            if let Ok(secs) = v.parse::<i64>() {
                time = unix_to_iso(secs);
            }
        } else if let Some(v) = l.strip_prefix("summary ") {
            summary = v.to_string();
        }
    }
    let uncommitted = sha.chars().all(|c| c == '0') && !sha.is_empty();
    Ok(json!({
        "sha": sha,
        "shortSha": sha.chars().take(7).collect::<String>(),
        "author": author,
        "authorEmail": email,
        "authorTime": time,
        "summary": summary,
        "uncommitted": uncommitted,
    }))
}

fn read_file_at_rev(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let file = str_arg(payload, "filePath")?;
    if file.len() > 2048 {
        return Err(err("`filePath` is too long"));
    }
    let max_bytes = payload
        .get("maxBytes")
        .and_then(Value::as_u64)
        .map(|n| n.min(READ_FILE_MAX_BYTES as u64) as usize)
        .unwrap_or(READ_FILE_MAX_BYTES);
    let base = payload.get("base").and_then(Value::as_str);
    let (resolved_rev, spec) = match base {
        Some("index") => (":".to_string(), format!(":{file}")),
        Some("branch") => {
            let r = branch_base(cwd);
            (r.clone(), format!("{r}:{file}"))
        }
        _ => {
            let r = payload.get("rev").and_then(Value::as_str).unwrap_or("HEAD");
            if !rev_ok(r) {
                return Err(err("invalid `rev`"));
            }
            let resolved = stdout(
                cwd,
                &["rev-parse", "--verify", "-q", r],
                "git rev-parse failed",
            )
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| r.to_string());
            (resolved, format!("{r}:{file}"))
        }
    };
    let out = run(cwd, &["show", &spec])?;
    if !out.status.success() {
        return Ok(json!({
            "contents": "",
            "resolvedRev": resolved_rev.trim_start_matches(':'),
            "missing": true,
            "truncated": false,
        }));
    }
    let bytes = out.stdout;
    let truncated = bytes.len() > max_bytes;
    Ok(json!({
        "contents": String::from_utf8_lossy(&bytes[..bytes.len().min(max_bytes)]),
        "resolvedRev": resolved_rev.trim_start_matches(':'),
        "missing": false,
        "truncated": truncated,
    }))
}

fn list_branches(cwd: &str) -> Result<Value, WsErr> {
    let is_repo = ok(cwd, &["rev-parse", "--is-inside-work-tree"]);
    if !is_repo {
        return Ok(json!({"branches": [], "isRepo": false, "hasOriginRemote": false}));
    }
    let current = current_branch(cwd).unwrap_or_default();
    // Branch name -> worktree path for linked checkouts.
    let wt_out = stdout(cwd, &["worktree", "list", "--porcelain"], "git").unwrap_or_default();
    let mut worktrees: Vec<(String, String)> = Vec::new(); // (branch, path)
    let mut wt_path = String::new();
    for line in wt_out.lines().chain(std::iter::once("")) {
        if let Some(p) = line.strip_prefix("worktree ") {
            wt_path = p.to_string();
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            worktrees.push((b.to_string(), wt_path.clone()));
        } else if line.is_empty() {
            wt_path.clear();
        }
    }
    let default_remote = stdout(
        cwd,
        &["symbolic-ref", "-q", "--short", "refs/remotes/origin/HEAD"],
        "git",
    )
    .map(|s| s.trim().to_string())
    .ok()
    .filter(|s| !s.is_empty());
    let default_local = default_remote
        .as_deref()
        .and_then(|d| d.split_once('/').map(|(_, b)| b.to_string()))
        .or_else(|| {
            ["main", "master"]
                .iter()
                .find(|n| {
                    ok(
                        cwd,
                        &["show-ref", "--verify", "-q", &format!("refs/heads/{n}")],
                    )
                })
                .map(|s| s.to_string())
        });
    let out = stdout(
        cwd,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)",
            "refs/heads",
            "refs/remotes",
        ],
        "git for-each-ref failed",
    )?;
    let has_origin = ok(cwd, &["remote", "get-url", "origin"]);
    let mut branches = Vec::new();
    for line in out.lines() {
        let (full, short) = line.split_once('\0').unwrap_or((line, ""));
        if short.is_empty() || short.ends_with("/HEAD") {
            continue;
        }
        if let Some(local) = full.strip_prefix("refs/heads/") {
            let wt = worktrees
                .iter()
                .find(|(b, _)| b == local)
                .map(|(_, p)| Value::String(p.clone()))
                .unwrap_or(Value::Null);
            branches.push(json!({
                "name": local,
                "current": local == current,
                "isDefault": default_local.as_deref() == Some(local),
                "worktreePath": wt,
            }));
        } else if let Some(rest) = full.strip_prefix("refs/remotes/") {
            let remote = rest.split('/').next().unwrap_or("");
            branches.push(json!({
                "name": rest,
                "isRemote": true,
                "remoteName": remote,
                "current": false,
                "isDefault": default_remote.as_deref() == Some(rest),
                "worktreePath": Value::Null,
            }));
        }
    }
    Ok(json!({
        "branches": branches,
        "isRepo": true,
        "hasOriginRemote": has_origin,
    }))
}

fn list_recent_commits(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let limit = payload
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 50);
    let n = limit.to_string();
    let out = stdout(
        cwd,
        &["log", "-n", &n, "--format=%H%x00%h%x00%s%x00%cI"],
        "git log failed",
    )?;
    let commits: Vec<Value> = out
        .lines()
        .filter_map(|l| {
            let mut p = l.splitn(4, '\0');
            Some(json!({
                "sha": p.next()?,
                "shortSha": p.next()?,
                "subject": p.next()?,
                "committedAt": p.next()?,
            }))
        })
        .collect();
    Ok(json!({"commits": commits}))
}

fn stash_info(cwd: &str) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let list = stdout(cwd, &["stash", "list"], "git stash failed")?;
    let Some(first) = list.lines().next() else {
        return Err(err("No stash entries found"));
    };
    let (stash_ref, message) = match first.split_once(':') {
        Some((r, m)) => (r.trim().to_string(), m.trim().to_string()),
        None => ("stash@{0}".to_string(), first.trim().to_string()),
    };
    let files_out = stdout(
        cwd,
        &["stash", "show", "--name-only", &stash_ref],
        "git stash show failed",
    )?;
    let files: Vec<String> = files_out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    Ok(json!({
        "cwd": cwd,
        "branch": current_branch(cwd),
        "stashRef": stash_ref,
        "message": message,
        "files": files,
    }))
}

fn pull(cwd: &str) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let branch = current_branch(cwd).ok_or_else(|| err("not on a branch"))?;
    let upstream = upstream_branch(cwd);
    if upstream.is_none() {
        return Err(err(format!("branch `{branch}` has no upstream")));
    }
    let behind = stdout(
        cwd,
        &["rev-list", "--count", "HEAD..@{upstream}"],
        "git rev-list failed",
    )
    .ok()
    .and_then(|s| s.trim().parse::<u64>().ok())
    .unwrap_or(0);
    if behind == 0 {
        return Ok(json!({
            "status": "skipped_up_to_date",
            "branch": branch,
            "upstreamBranch": upstream,
        }));
    }
    stdout(cwd, &["pull", "--ff-only"], "git pull failed")?;
    Ok(json!({
        "status": "pulled",
        "branch": branch,
        "upstreamBranch": upstream,
    }))
}

fn stage_files(cwd: &str, payload: &Value, unstage: bool) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let paths: Vec<String> = payload
        .get("paths")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if paths.is_empty() {
        return Err(err("`paths` must be a non-empty array"));
    }
    let mut args: Vec<&str> = if unstage {
        vec!["restore", "--staged", "--"]
    } else {
        vec!["add", "--"]
    };
    args.extend(paths.iter().map(String::as_str));
    let out = run(cwd, &args)?;
    if !out.status.success() && unstage {
        // `restore --staged` cannot handle every index shape (unborn HEAD);
        // `reset --` covers them.
        let mut args: Vec<&str> = vec!["reset", "-q", "--"];
        args.extend(paths.iter().map(String::as_str));
        run(cwd, &args)?;
    }
    Ok(json!({"ok": true}))
}

fn checkout(cwd: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    require_repo(cwd)?;
    let branch = str_arg(payload, "branch")?;
    stdout(cwd, &["checkout", branch], "git checkout failed")?;
    Ok(None)
}

fn create_branch(cwd: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    require_repo(cwd)?;
    let branch = str_arg(payload, "branch")?;
    stdout(cwd, &["checkout", "-b", branch], "git checkout -b failed")?;
    if payload.get("publish").and_then(Value::as_bool) == Some(true) {
        stdout(cwd, &["push", "-u", "origin", branch], "git push failed")?;
    }
    Ok(None)
}

fn stash_and_checkout(cwd: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    require_repo(cwd)?;
    let branch = str_arg(payload, "branch")?;
    stdout(
        cwd,
        &["stash", "push", "--include-untracked"],
        "git stash failed",
    )?;
    stdout(cwd, &["checkout", branch], "git checkout failed")?;
    Ok(None)
}

fn stash_drop(cwd: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    require_repo(cwd)?;
    let stash_ref = str_arg(payload, "stashRef")?;
    if !stash_ref.starts_with("stash@{") {
        return Err(err("invalid `stashRef`"));
    }
    stdout(cwd, &["stash", "drop", stash_ref], "git stash drop failed")?;
    Ok(None)
}

fn remove_index_lock(cwd: &str) -> Result<Option<Value>, WsErr> {
    require_repo(cwd)?;
    let git_dir = stdout(cwd, &["rev-parse", "--git-dir"], "git rev-parse failed")?;
    let git_dir = git_dir.trim();
    let lock = if std::path::Path::new(git_dir).is_absolute() {
        std::path::PathBuf::from(git_dir)
    } else {
        std::path::Path::new(cwd).join(git_dir)
    }
    .join("index.lock");
    if lock.exists() {
        std::fs::remove_file(&lock)
            .map_err(|e| err(format!("failed to remove index.lock: {e}")))?;
    }
    Ok(None)
}

fn init(cwd: &str) -> Result<Option<Value>, WsErr> {
    if ok(cwd, &["rev-parse", "--is-inside-work-tree"]) {
        return Ok(None);
    }
    // `git init <dir>` also creates the directory; `-C` requires it to exist.
    let out = Command::new("git")
        .arg("init")
        .arg(cwd)
        .output()
        .map_err(|e| err(format!("failed to spawn git: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(err(format!(
            "git init failed: {}",
            stderr.trim().lines().next().unwrap_or("git failed")
        )));
    }
    Ok(None)
}

fn worktrees_dir() -> String {
    let state = crate::todo::state_dir();
    // state_dir() is `<home>/userdata`; worktrees sit at `<home>/worktrees`.
    match state.strip_suffix("/userdata") {
        Some(home) => format!("{home}/worktrees"),
        None => format!("{state}/worktrees"),
    }
}

fn create_worktree(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let branch = str_arg(payload, "branch")?;
    if branch.starts_with('-') {
        return Err(err("invalid `branch`"));
    }
    let path = payload
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| {
            let safe: String = branch
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '-'
                    }
                })
                .collect();
            let repo = std::path::Path::new(cwd)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repo");
            format!("{}/{repo}/{safe}", worktrees_dir())
        });
    let new_branch = payload
        .get("newBranch")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let mut args: Vec<&str> = vec!["worktree", "add"];
    if let Some(nb) = new_branch {
        args.push("-b");
        args.push(nb);
    }
    args.push(&path);
    args.push(branch);
    let out = run(cwd, &args)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(err(format!(
            "git worktree add failed: {}",
            stderr.trim().lines().next().unwrap_or("git failed")
        )));
    }
    let checked_out = stdout(&path, &["branch", "--show-current"], "git")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| new_branch.unwrap_or(branch).to_string());
    Ok(json!({
        "worktree": {
            "path": path,
            "branch": checked_out,
        }
    }))
}

fn remove_worktree(cwd: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    require_repo(cwd)?;
    let path = str_arg(payload, "path")?;
    // Read the worktree's branch before removal for reclaimTemporaryBranch.
    let wt_branch = stdout(path, &["branch", "--show-current"], "git")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty());
    let mut args = vec!["worktree", "remove"];
    if payload.get("force").and_then(Value::as_bool) == Some(true) {
        args.push("--force");
    }
    args.push(path);
    stdout(cwd, &args, "git worktree remove failed")?;
    if payload
        .get("reclaimTemporaryBranch")
        .and_then(Value::as_bool)
        == Some(true)
    {
        if let Some(b) = wt_branch {
            // Only managed `synara/*` branches are reclaimed.
            if b.starts_with("synara/") {
                let _ = run(cwd, &["branch", "-D", &b]);
            }
        }
    }
    Ok(None)
}

/// `Ok(None)` = `Schema.Void` success (no `value` key on the wire exit).
pub fn handle(tag: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    match tag {
        "git.status" => git_status(&cwd_of(payload)?).map(Some),
        "git.githubRepository" => github_repository(&cwd_of(payload)?).map(Some),
        "git.readWorkingTreeDiff" => read_working_tree_diff(&cwd_of(payload)?, payload).map(Some),
        "git.blameLine" => blame_line(&cwd_of(payload)?, payload).map(Some),
        "git.readFileAtRev" => read_file_at_rev(&cwd_of(payload)?, payload).map(Some),
        "git.workingTreeDiffStats" => working_tree_diff_stats(&cwd_of(payload)?, payload).map(Some),
        "git.listBranches" => list_branches(&cwd_of(payload)?).map(Some),
        "git.listRecentCommits" => list_recent_commits(&cwd_of(payload)?, payload).map(Some),
        "git.stashInfo" => stash_info(&cwd_of(payload)?).map(Some),
        "git.pull" => pull(&cwd_of(payload)?).map(Some),
        "git.init" => init(&cwd_of(payload)?),
        "git.stageFiles" => stage_files(&cwd_of(payload)?, payload, false).map(Some),
        "git.unstageFiles" => stage_files(&cwd_of(payload)?, payload, true).map(Some),
        "git.checkout" => checkout(&cwd_of(payload)?, payload),
        "git.createBranch" => create_branch(&cwd_of(payload)?, payload),
        "git.stashAndCheckout" => stash_and_checkout(&cwd_of(payload)?, payload),
        "git.stashDrop" => stash_drop(&cwd_of(payload)?, payload),
        "git.removeIndexLock" => remove_index_lock(&cwd_of(payload)?),
        "git.createWorktree" => create_worktree(&cwd_of(payload)?, payload).map(Some),
        "git.removeWorktree" => remove_worktree(&cwd_of(payload)?, payload),
        _ => Err(err(format!("unhandled git tag {tag}"))),
    }
}
