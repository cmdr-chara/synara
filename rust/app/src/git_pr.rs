//! gh-backed `git.*` WS-RPC methods: pull-request resolution/snapshot,
//! detached worktrees and `preparePullRequestThread`. Mirrors
//! `packages/contracts/src/git.ts` wire shapes; semantics follow
//! `GitManager`/`GitCore`/`GitHubCli` in the upstream server.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::git_domain::{cwd_of, err, ok, require_repo, run, stdout, str_arg};
use crate::github_inbox::{gh_json, gh_run, graphql_host, summarize_error};
use crate::todo::WsErr;

const PR_SUMMARY_FIELDS: &str = "number,title,url,baseRefName,headRefName,state,mergedAt,isDraft,mergeable,additions,deletions,changedFiles,isCrossRepository,headRepository,headRepositoryOwner,updatedAt";
const PR_SNAPSHOT_FIELDS: &str = "number,title,url,baseRefName,headRefName,state,mergedAt,isDraft,mergeable,additions,deletions,changedFiles,isCrossRepository,headRepository,headRepositoryOwner,updatedAt,headRefOid,statusCheckRollup";

const REVIEW_THREADS_QUERY: &str = "query($owner: String!, $repo: String!, $number: Int!, $first: Int!, $after: String) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      reviewThreads(first: $first, after: $after) {
        nodes {
          isResolved
          comments(first: 1) {
            nodes {
              id
              body
              path
              url
              createdAt
              author { login }
            }
          }
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
    }
  }
}";

const REVIEW_THREAD_PAGE_SIZE: usize = 50;
const REVIEW_THREAD_PAGE_LIMIT: usize = 5;
const REVIEW_COMMENT_LIMIT: usize = 20;

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "git.resolvePullRequest"
            | "git.pullRequestSnapshot"
            | "git.createDetachedWorktree"
            | "git.preparePullRequestThread"
            | "git.runStackedAction"
            | "git.handoffThread"
    )
}

/// `Ok(None)` = `Schema.Void` success.
pub fn handle(tag: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    match tag {
        "git.resolvePullRequest" => resolve_pull_request(&cwd_of(payload)?, payload).map(Some),
        "git.pullRequestSnapshot" => pull_request_snapshot(&cwd_of(payload)?, payload).map(Some),
        "git.createDetachedWorktree" => {
            create_detached_worktree(&cwd_of(payload)?, payload).map(Some)
        }
        "git.preparePullRequestThread" => {
            prepare_pull_request_thread(&cwd_of(payload)?, payload).map(Some)
        }
        "git.runStackedAction" => run_stacked_action(&cwd_of(payload)?, payload).map(Some),
        "git.handoffThread" => handoff_thread(&cwd_of(payload)?, payload).map(Some),
        _ => Err(err(format!("unhandled git tag {tag}"))),
    }
}

// ---------------------------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------------------------

fn gh_err(e: String) -> WsErr {
    err(e)
}

fn normalize_pr_reference(reference: &str) -> String {
    let trimmed = reference.trim();
    if let Some(rest) = trimmed.strip_prefix('#') {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return rest.to_string();
        }
    }
    trimmed.to_string()
}

fn canonicalize(path: &str) -> String {
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}

fn opt_str(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

fn diff_count(v: &Value) -> Value {
    v.as_i64()
        .filter(|n| *n >= 0)
        .map(|n| json!(n))
        .unwrap_or(Value::Null)
}

/// `toResolvedPullRequest` + the head-remote fields the worktree flow needs.
fn normalize_pr_summary(raw: &Value) -> (Value, HeadRemoteInfo) {
    let merged_at = opt_str(&raw["mergedAt"]).unwrap_or_default();
    let state_raw = raw["state"].as_str().unwrap_or("");
    let state = if !merged_at.is_empty() || state_raw == "MERGED" {
        "merged"
    } else if state_raw == "CLOSED" {
        "closed"
    } else {
        "open"
    };
    let mergeability = match raw["mergeable"].as_str() {
        Some("MERGEABLE") => "mergeable",
        Some("CONFLICTING") => "conflicting",
        _ => "unknown",
    };
    let head_nwo = raw["headRepository"]["nameWithOwner"].as_str().map(String::from);
    let head_owner = opt_str(&raw["headRepositoryOwner"]["login"]).or_else(|| {
        head_nwo
            .as_deref()
            .and_then(|n| n.split('/').next())
            .map(String::from)
    });
    let pr = json!({
        "number": raw["number"],
        "title": raw["title"],
        "url": raw["url"],
        "baseBranch": raw["baseRefName"],
        "headBranch": raw["headRefName"],
        "state": state,
        "isDraft": raw["isDraft"].as_bool().unwrap_or(false),
        "mergeability": mergeability,
        "additions": diff_count(&raw["additions"]),
        "deletions": diff_count(&raw["deletions"]),
        "changedFiles": diff_count(&raw["changedFiles"]),
    });
    let info = HeadRemoteInfo {
        is_cross_repository: raw["isCrossRepository"].as_bool(),
        head_repo_nwo: head_nwo,
        head_owner_login: head_owner,
    };
    (pr, info)
}

struct HeadRemoteInfo {
    is_cross_repository: Option<bool>,
    head_repo_nwo: Option<String>,
    head_owner_login: Option<String>,
}

/// `parsePullRequestRepositoryFromUrl`: host + owner + repo from a PR web URL.
fn parse_pr_repo_from_url(url: &str) -> Option<(String, String, String)> {
    let rest = url.trim().strip_prefix("https://").or_else(|| url.trim().strip_prefix("http://"))?;
    let mut parts = rest.split('/');
    let host = parts.next()?.to_string();
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    let kind = parts.next()?;
    if kind != "pull" || host.is_empty() || owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((host, owner, repo))
}

/// `resolveHeadRepositoryNameWithOwner`: explicit headRepository.nameWithOwner,
/// else owner/repo rebuilt from the PR url for cross-repo heads.
fn head_repo_nwo(pr: &Value, info: &HeadRemoteInfo) -> Option<String> {
    if let Some(nwo) = &info.head_repo_nwo {
        let n = nwo.trim();
        if !n.is_empty() {
            return Some(n.to_string());
        }
    }
    if info.is_cross_repository != Some(true) {
        return None;
    }
    let owner = info.head_owner_login.as_deref()?.trim();
    if owner.is_empty() {
        return None;
    }
    let (_host, _o, repo) = parse_pr_repo_from_url(pr["url"].as_str()?)?;
    Some(format!("{owner}/{repo}"))
}

/// `resolvePullRequestWorktreeLocalBranchName`.
fn local_pr_branch(pr: &Value, info: &HeadRemoteInfo) -> String {
    let head = pr["headBranch"].as_str().unwrap_or("").to_string();
    if info.is_cross_repository != Some(true) {
        return head;
    }
    let sanitized = sanitize_branch_fragment(&head);
    let suffix = if sanitized.is_empty() { "head" } else { sanitized.as_str() };
    format!("synara/pr-{}/{suffix}", pr["number"].as_i64().unwrap_or(0))
}

/// Port of `sanitizeBranchFragment` (packages/shared/src/git.ts).
fn sanitize_branch_fragment(raw: &str) -> String {
    let normalized: String = raw
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| *c != '\'' && *c != '"' && *c != '`')
        .collect();
    let normalized = normalized.trim_matches(|c: char| "./\t _-".contains(c));
    let mut out = String::with_capacity(normalized.len());
    let mut last_dash = false;
    for c in normalized.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-') {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    // collapse runs of '-' and '/'
    let collapsed = out
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");
    let trimmed = collapsed.trim_matches(|c: char| "./_-".contains(c));
    let sliced: String = trimmed.chars().take(64).collect();
    let sliced = sliced.trim_end_matches(|c: char| "./_-".contains(c)).to_string();
    if sliced.is_empty() { "update".to_string() } else { sliced }
}

fn sanitize_remote_name(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' })
        .collect();
    let s = s.trim_matches('-');
    if s.is_empty() { "fork".to_string() } else { s.to_string() }
}

fn gh_pr_view(cwd: &str, reference: &str, fields: &str) -> Result<Value, WsErr> {
    let args: Vec<String> = vec![
        "pr".into(),
        "view".into(),
        reference.into(),
        "--json".into(),
        fields.into(),
    ];
    gh_json(&args, cwd).map_err(gh_err)
}

// ---------------------------------------------------------------------------------------------
// git.resolvePullRequest
// ---------------------------------------------------------------------------------------------

fn resolve_pull_request(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    let reference = normalize_pr_reference(str_arg(payload, "reference")?);
    let raw = gh_pr_view(cwd, &reference, PR_SUMMARY_FIELDS)?;
    let (pr, _info) = normalize_pr_summary(&raw);
    Ok(json!({ "pullRequest": pr }))
}

// ---------------------------------------------------------------------------------------------
// git.pullRequestSnapshot
// ---------------------------------------------------------------------------------------------

fn normalize_check(item: &Value) -> Option<Value> {
    let name = opt_str(&item["name"]).or_else(|| opt_str(&item["context"]))?;
    let status = match item["state"].as_str() {
        Some("SUCCESS") => "success",
        Some("FAILURE") | Some("ERROR") => "failure",
        Some(_) => "pending",
        None => match item["status"].as_str() {
            Some(s) if s != "COMPLETED" => "pending",
            _ => match item["conclusion"].as_str() {
                Some("SUCCESS") => "success",
                Some("FAILURE") | Some("TIMED_OUT") | Some("ACTION_REQUIRED")
                | Some("STARTUP_FAILURE") => "failure",
                Some("SKIPPED") => "skipped",
                Some("CANCELLED") => "cancelled",
                Some("NEUTRAL") | Some("STALE") => "neutral",
                _ => "pending",
            },
        },
    };
    let url = opt_str(&item["detailsUrl"]).or_else(|| opt_str(&item["targetUrl"]));
    Some(json!({ "name": name, "status": status, "url": url }))
}

fn review_comments(
    cwd: &str,
    host: &str,
    owner: &str,
    repo: &str,
    number: i64,
) -> Result<(Vec<Value>, bool), String> {
    let mut comments: Vec<Value> = Vec::new();
    let mut after: Option<String> = None;
    let mut truncated = false;
    let mut pages = 0;
    loop {
        pages += 1;
        let mut vars: Vec<String> = vec![
            "-F".into(),
            format!("owner={owner}"),
            "-F".into(),
            format!("repo={repo}"),
            "-F".into(),
            format!("number={number}"),
            "-F".into(),
            format!("first={REVIEW_THREAD_PAGE_SIZE}"),
        ];
        if let Some(a) = &after {
            vars.push("-F".into());
            vars.push(format!("after={a}"));
        }
        let decoded = graphql_host(host, REVIEW_THREADS_QUERY, &vars, cwd)?;
        if let Some(errors) = decoded["errors"].as_array() {
            let messages: Vec<String> = errors
                .iter()
                .filter_map(|e| e["message"].as_str().map(str::trim).map(String::from))
                .filter(|m| !m.is_empty())
                .collect();
            if !messages.is_empty() {
                return Err(format!("GitHub GraphQL returned errors: {}", messages.join("; ")));
            }
        }
        let threads = decoded["data"]["repository"]["pullRequest"]["reviewThreads"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut page_comments: Vec<Value> = Vec::new();
        for thread in threads {
            if thread["isResolved"].as_bool() == Some(true) {
                continue;
            }
            let Some(root) = thread["comments"]["nodes"].as_array().and_then(|n| {
                n.iter().find(|c| !c.is_null())
            }) else {
                continue;
            };
            page_comments.push(json!({
                "id": root["id"],
                "author": opt_str(&root["author"]["login"]),
                "body": root["body"].as_str().unwrap_or(""),
                "path": opt_str(&root["path"]),
                "url": opt_str(&root["url"]),
                "createdAt": opt_str(&root["createdAt"]),
            }));
        }
        let remaining = REVIEW_COMMENT_LIMIT.saturating_sub(comments.len());
        if page_comments.len() > remaining {
            truncated = true;
        }
        comments.extend(page_comments.into_iter().take(remaining));

        let page_info = &decoded["data"]["repository"]["pullRequest"]["reviewThreads"]["pageInfo"];
        let has_next = page_info["hasNextPage"].as_bool() == Some(true);
        let end_cursor = opt_str(&page_info["endCursor"]);
        let can_next = has_next
            && end_cursor.is_some()
            && comments.len() < REVIEW_COMMENT_LIMIT
            && pages < REVIEW_THREAD_PAGE_LIMIT;
        if !can_next && has_next {
            truncated = true;
        }
        after = can_next.then(|| end_cursor.unwrap());
        if after.is_none() {
            break;
        }
    }
    Ok((comments, truncated))
}

fn pull_request_snapshot(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    let reference = normalize_pr_reference(str_arg(payload, "reference")?);
    let raw = gh_pr_view(cwd, &reference, PR_SNAPSHOT_FIELDS)?;
    let (pr, _info) = normalize_pr_summary(&raw);
    let checks: Vec<Value> = raw["statusCheckRollup"]
        .as_array()
        .map(|arr| arr.iter().filter_map(normalize_check).collect())
        .unwrap_or_default();

    let url = pr["url"].as_str().unwrap_or("");
    let (host, owner, repo) = parse_pr_repo_from_url(url).ok_or_else(|| {
        err(format!(
            "Could not determine the repository from the pull request URL: {url}"
        ))
    })?;
    let number = pr["number"].as_i64().unwrap_or(0);
    let (comments, comments_truncated, comments_error) =
        match review_comments(cwd, &host, &owner, &repo, number) {
            Ok((c, t)) => (c, t, Value::Null),
            Err(e) => (Vec::new(), false, json!(e)),
        };
    Ok(json!({
        "pullRequest": pr,
        "checks": checks,
        "comments": comments,
        "commentsTruncated": comments_truncated,
        "commentsError": comments_error,
    }))
}

// ---------------------------------------------------------------------------------------------
// git / gh plumbing for the worktree flows
// ---------------------------------------------------------------------------------------------

fn primary_remote_name(cwd: &str) -> Result<String, WsErr> {
    if ok(cwd, &["remote", "get-url", "origin"]) {
        return Ok("origin".to_string());
    }
    let remotes = stdout(cwd, &["remote"], "git remote")?;
    let first = remotes.lines().map(str::trim).find(|l| !l.is_empty());
    match first {
        Some(r) => Ok(r.to_string()),
        None => Err(err("No git remote is configured for this repository.")),
    }
}

fn config_value(cwd: &str, key: &str) -> Option<String> {
    stdout(cwd, &["config", "--get", key], "git")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn normalize_remote_url(url: &str) -> String {
    let s = url.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s);
    s.to_lowercase()
}

fn parse_remote_fetch_urls(cwd: &str) -> Vec<(String, String)> {
    let out = stdout(cwd, &["remote", "-v"], "git").unwrap_or_default();
    let mut remotes = Vec::new();
    for line in out.lines() {
        let t = line.trim();
        let Some((name, rest)) = t.split_once(|c: char| c.is_whitespace()) else {
            continue;
        };
        let Some((url, dir)) = rest.trim().rsplit_once(' ') else {
            continue;
        };
        if dir.trim() == "(fetch)" && !name.is_empty() {
            remotes.push((name.to_string(), url.to_string()));
        }
    }
    remotes
}

/// `ensureRemote`: reuse a remote whose fetch URL already matches, else add one.
fn ensure_remote(cwd: &str, preferred: &str, url: &str) -> Result<String, WsErr> {
    let preferred = sanitize_remote_name(preferred);
    let target = normalize_remote_url(url);
    let remotes = parse_remote_fetch_urls(cwd);
    for (name, remote_url) in &remotes {
        if normalize_remote_url(remote_url) == target {
            return Ok(name.clone());
        }
    }
    let mut candidate = preferred.clone();
    let mut suffix = 1;
    while remotes.iter().any(|(n, _)| n == &candidate) {
        candidate = format!("{preferred}-{suffix}");
        suffix += 1;
    }
    run(cwd, &["remote", "add", &candidate, url])
        .and_then(|o| {
            if o.status.success() {
                Ok(())
            } else {
                Err(err(format!(
                    "git remote add failed: {}",
                    String::from_utf8_lossy(&o.stderr).trim().lines().next().unwrap_or("")
                )))
            }
        })?;
    Ok(candidate)
}

fn repo_clone_urls(cwd: &str, nwo: &str) -> Result<(String, String), WsErr> {
    let raw = gh_json(
        &[
            "repo".into(),
            "view".into(),
            nwo.into(),
            "--json".into(),
            "nameWithOwner,url,sshUrl".into(),
        ],
        cwd,
    )
    .map_err(gh_err)?;
    Ok((
        raw["url"].as_str().unwrap_or("").to_string(),
        raw["sshUrl"].as_str().unwrap_or("").to_string(),
    ))
}

fn prefers_ssh(url: &str) -> bool {
    let t = url.trim();
    t.starts_with("git@") || t.starts_with("ssh://")
}

fn set_branch_upstream(cwd: &str, local: &str, remote: &str, remote_branch: &str) -> Result<(), WsErr> {
    stdout(
        cwd,
        &[
            "branch",
            "--set-upstream-to",
            &format!("{remote}/{remote_branch}"),
            local,
        ],
        "git branch --set-upstream-to",
    )?;
    Ok(())
}

fn fetch_pr_branch(cwd: &str, pr_number: i64, branch: &str) -> Result<(), WsErr> {
    let remote = primary_remote_name(cwd)?;
    let refspec = format!("+refs/pull/{pr_number}/head:refs/heads/{branch}");
    let out = run(cwd, &["fetch", "--quiet", "--no-tags", &remote, &refspec])?;
    if !out.status.success() {
        return Err(err(format!(
            "git fetch pull request branch failed: {}",
            String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("")
        )));
    }
    Ok(())
}

fn fetch_remote_branch(cwd: &str, remote: &str, remote_branch: &str, local: &str) -> Result<(), WsErr> {
    let refspec = format!("+refs/heads/{remote_branch}:refs/remotes/{remote}/{remote_branch}");
    let out = run(cwd, &["fetch", "--quiet", "--no-tags", remote, &refspec])?;
    if !out.status.success() {
        return Err(err(format!(
            "git fetch remote branch failed: {}",
            String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("")
        )));
    }
    let target = format!("{remote}/{remote_branch}");
    let exists = ok(cwd, &["show-ref", "--verify", "-q", &format!("refs/heads/{local}")]);
    let args: Vec<&str> = if exists {
        vec!["branch", "--force", local, &target]
    } else {
        vec!["branch", local, &target]
    };
    let out = run(cwd, &args)?;
    if !out.status.success() {
        return Err(err(format!(
            "git branch failed: {}",
            String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("")
        )));
    }
    Ok(())
}

fn parse_nwo_from_remote_url(url: &str) -> Option<String> {
    let t = url.trim();
    let body = if let Some(rest) = t.strip_prefix("git@github.com:") {
        rest
    } else if let Some(rest) = t.strip_prefix("ssh://git@github.com/") {
        rest
    } else if let Some(rest) = t.strip_prefix("https://github.com/") {
        rest
    } else if let Some(rest) = t.strip_prefix("git://github.com/") {
        rest
    } else {
        return None;
    };
    let body = body.trim_end_matches('/').strip_suffix(".git").unwrap_or(body.trim_end_matches('/'));
    let mut parts = body.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

fn remote_repo_nwo(cwd: &str, remote: Option<&str>) -> Option<String> {
    let remote = remote?;
    let url = config_value(cwd, &format!("remote.{remote}.url"))?;
    parse_nwo_from_remote_url(&url)
}

/// `configurePullRequestHeadUpstream`: best-effort, errors are logged upstream
/// (and silently swallowed — a failed upstream setup never fails the call).
fn configure_pr_head_upstream(cwd: &str, pr: &Value, info: &HeadRemoteInfo, local_branch: &str) {
    let head_branch = pr["headBranch"].as_str().unwrap_or("");
    let Some(nwo) = head_repo_nwo(pr, info) else { return };
    let inner = (|| -> Result<(), WsErr> {
        let (https, ssh) = repo_clone_urls(cwd, &nwo)?;
        let origin_url = config_value(cwd, "remote.origin.url").unwrap_or_default();
        let remote_url = if prefers_ssh(&origin_url) { ssh } else { https };
        let preferred = info
            .head_owner_login
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| nwo.split('/').next().map(String::from))
            .unwrap_or_else(|| "fork".to_string());
        let remote = ensure_remote(cwd, &preferred, &remote_url)?;
        set_branch_upstream(cwd, local_branch, &remote, head_branch)
    })();
    let _ = inner;
}

fn materialize_pr_head_branch(cwd: &str, pr: &Value, info: &HeadRemoteInfo, local: &str) -> Result<(), WsErr> {
    let head_branch = pr["headBranch"].as_str().unwrap_or("");
    let number = pr["number"].as_i64().unwrap_or(0);
    let Some(nwo) = head_repo_nwo(pr, info) else {
        return fetch_pr_branch(cwd, number, local);
    };
    let via_remote = (|| -> Result<(), WsErr> {
        let (https, ssh) = repo_clone_urls(cwd, &nwo)?;
        let origin_url = config_value(cwd, "remote.origin.url").unwrap_or_default();
        let remote_url = if prefers_ssh(&origin_url) { ssh } else { https };
        let preferred = info
            .head_owner_login
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| nwo.split('/').next().map(String::from))
            .unwrap_or_else(|| "fork".to_string());
        let remote = ensure_remote(cwd, &preferred, &remote_url)?;
        fetch_remote_branch(cwd, &remote, head_branch, local)?;
        set_branch_upstream(cwd, local, &remote, head_branch)
    })();
    if via_remote.is_err() {
        fetch_pr_branch(cwd, number, local)
    } else {
        via_remote
    }
}

fn worktrees_dir() -> PathBuf {
    // upstream: join(baseDir, "worktrees"); state_dir() = <base>/userdata
    Path::new(&crate::todo::state_dir())
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("worktrees")
}

/// Local branches: (name, worktree_path). Mirrors listBranches' worktree map.
fn local_branches_with_worktrees(cwd: &str) -> Vec<(String, Option<String>)> {
    let mut worktrees: Vec<(String, String)> = Vec::new();
    if let Ok(wt_out) = stdout(cwd, &["worktree", "list", "--porcelain"], "git") {
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
    }
    let refs = stdout(
        cwd,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
        "git",
    )
    .unwrap_or_default();
    refs.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|name| {
            let wt = worktrees
                .iter()
                .find(|(b, _)| b == name)
                .map(|(_, p)| p.clone());
            (name.to_string(), wt)
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// git.preparePullRequestThread
// ---------------------------------------------------------------------------------------------

fn find_local_head_branch(
    cwd: &str,
    pr: &Value,
    info: &HeadRemoteInfo,
    local_pr_branch: &str,
    root_worktree: &str,
    mode: &str,
) -> Result<Option<(String, Option<String>)>, WsErr> {
    let branches = local_branches_with_worktrees(cwd);
    if let Some(found) = branches.iter().find(|(n, _)| n == local_pr_branch) {
        return Ok(Some(found.clone()));
    }
    let head = pr["headBranch"].as_str().unwrap_or("");
    if local_pr_branch == head {
        return Ok(None);
    }
    let candidate = branches.iter().find(|(n, wt)| {
        n == head
            && wt.is_some()
            && canonicalize(wt.as_deref().unwrap()) != root_worktree
    });
    let Some(candidate) = candidate else { return Ok(None) };
    // Guard: a shared branch name does not identify a fork — if both repos are
    // known and differ, local mode errors and worktree mode ignores it.
    let remote_name = config_value(cwd, &format!("branch.{}.remote", candidate.0));
    let actual = remote_repo_nwo(cwd, remote_name.as_deref()).map(|s| s.to_lowercase());
    let expected = head_repo_nwo(pr, info).map(|s| s.to_lowercase());
    if expected.is_some() && actual.is_some() && expected != actual {
        if mode == "local" {
            return Err(err(
                "This branch is checked out in a worktree for a different GitHub repository. Use Worktree to prepare this pull request separately.",
            ));
        }
        return Ok(None);
    }
    Ok(Some(candidate.clone()))
}

fn prepare_pull_request_thread(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    let reference = normalize_pr_reference(str_arg(payload, "reference")?);
    let mode = str_arg(payload, "mode")?;
    let root_worktree = canonicalize(cwd);
    let raw = gh_pr_view(cwd, &reference, PR_SUMMARY_FIELDS)?;
    let (pr, info) = normalize_pr_summary(&raw);
    let head_branch = pr["headBranch"].as_str().unwrap_or("").to_string();
    let local_pr_branch = local_pr_branch(&pr, &info);

    let before = find_local_head_branch(cwd, &pr, &info, &local_pr_branch, &root_worktree, mode)?;
    let before_path = before
        .as_ref()
        .and_then(|(_, wt)| wt.as_deref().map(canonicalize));
    if let (Some((name, Some(wt))), _) = (&before, before_path.as_deref()) {
        if before_path.as_deref() != Some(root_worktree.as_str()) {
            let details_branch = stdout(wt, &["branch", "--show-current"], "git")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| head_branch.clone());
            configure_pr_head_upstream(wt, &pr, &info, &details_branch);
            return Ok(json!({
                "pullRequest": pr,
                "branch": name,
                "worktreePath": wt,
            }));
        }
    }

    if mode == "local" {
        let out = gh_run(
            &["pr".into(), "checkout".into(), reference.clone(), "--force".into()],
            cwd,
        )
        .map_err(gh_err)?;
        if out.code != 0 {
            return Err(err(summarize_error(&out.stderr)));
        }
        let branch = crate::git_domain::current_branch(cwd).unwrap_or_else(|| head_branch.clone());
        configure_pr_head_upstream(cwd, &pr, &info, &branch);
        return Ok(json!({
            "pullRequest": pr,
            "branch": branch,
            "worktreePath": Value::Null,
        }));
    }

    if before_path.as_deref() == Some(root_worktree.as_str()) {
        return Err(err(
            "This PR branch is already checked out in the main repo. Use Local, or switch the main repo off that branch before creating a worktree thread.",
        ));
    }

    materialize_pr_head_branch(cwd, &pr, &info, &local_pr_branch)?;

    let after = find_local_head_branch(cwd, &pr, &info, &local_pr_branch, &root_worktree, mode)?;
    let after_path = after
        .as_ref()
        .and_then(|(_, wt)| wt.as_deref().map(canonicalize));
    if let Some((name, Some(wt))) = &after {
        if after_path.as_deref() != Some(root_worktree.as_str()) {
            let details_branch = stdout(wt, &["branch", "--show-current"], "git")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| head_branch.clone());
            configure_pr_head_upstream(wt, &pr, &info, &details_branch);
            return Ok(json!({
                "pullRequest": pr,
                "branch": name,
                "worktreePath": wt,
            }));
        }
    }
    if after_path.as_deref() == Some(root_worktree.as_str()) {
        return Err(err(
            "This PR branch is already checked out in the main repo. Use Local, or switch the main repo off that branch before creating a worktree thread.",
        ));
    }

    // gitCore.createWorktree: worktreesDir/repoName/sanitized-branch
    let sanitized = local_pr_branch.replace('/', "-");
    let repo_name = Path::new(cwd)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string());
    let worktree_path = worktrees_dir().join(&repo_name).join(&sanitized);
    let wt = worktree_path.to_string_lossy().to_string();
    let out = run(cwd, &["worktree", "add", &wt, &local_pr_branch])?;
    if !out.status.success() {
        return Err(err(format!(
            "git worktree add failed: {}",
            String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("")
        )));
    }
    configure_pr_head_upstream(&wt, &pr, &info, &local_pr_branch);
    Ok(json!({
        "pullRequest": pr,
        "branch": local_pr_branch,
        "worktreePath": wt,
    }))
}

// ---------------------------------------------------------------------------------------------
// git.createDetachedWorktree
// ---------------------------------------------------------------------------------------------

static WORKTREE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generated_detached_worktree_path() -> Result<String, WsErr> {
    for _ in 0..8 {
        let n = WORKTREE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs().wrapping_mul(2654435761) ^ n)
            .unwrap_or(n);
        let short = format!("{:04x}", seed & 0xffff);
        let parent = worktrees_dir().join(&short);
        let candidate = parent.join("synara");
        if candidate.exists() {
            continue;
        }
        std::fs::create_dir_all(&parent)
            .map_err(|e| err(format!("failed to prepare detached worktree path: {e}")))?;
        return Ok(candidate.to_string_lossy().to_string());
    }
    let fallback = worktrees_dir().join(format!("{:016x}", std::process::id()));
    std::fs::create_dir_all(&fallback)
        .map_err(|e| err(format!("failed to prepare detached worktree path: {e}")))?;
    Ok(fallback.join("synara").to_string_lossy().to_string())
}

/// `listWorktreeTransferPaths`: untracked + .worktreeinclude-ignored paths.
fn worktree_transfer_paths(source: &str) -> Result<Vec<String>, WsErr> {
    let mut paths: Vec<String> = Vec::new();
    let parse = |out: String| -> Vec<String> {
        out.split('\0')
            .filter(|e| !e.is_empty())
            .filter(|e| {
                let p = e;
                !Path::new(p).is_absolute()
                    && *p != ".."
                    && !p.starts_with("../")
                    && !p.split(['/', '\\']).any(|seg| seg == "..")
            })
            .map(String::from)
            .collect()
    };
    let untracked = run(
        source,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?;
    paths.extend(parse(String::from_utf8_lossy(&untracked.stdout).to_string()));
    let include_file = Path::new(source).join(".worktreeinclude");
    if include_file.is_file() {
        let included = run(
            source,
            &[
                "ls-files",
                "--others",
                "--ignored",
                "--exclude-from=.worktreeinclude",
                "-z",
            ],
        )?;
        paths.extend(parse(String::from_utf8_lossy(&included.stdout).to_string()));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn copy_checkout_changes(source: &str, worktree: &str) -> Result<(), WsErr> {
    let patch = run(
        source,
        &[
            "diff",
            "--binary",
            "--full-index",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "HEAD",
            "--",
        ],
    )?;
    if !patch.status.success() {
        return Err(err(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&patch.stderr).trim().lines().next().unwrap_or("")
        )));
    }
    let patch_bytes = &patch.stdout;
    let transfer_paths = worktree_transfer_paths(source)?;

    if !patch_bytes.is_empty() {
        let tmp = std::env::temp_dir().join(format!("synara-wtp-{}", std::process::id()));
        let guard = scopeguard_tmp(&tmp);
        std::fs::create_dir_all(&tmp)
            .map_err(|e| err(format!("could not create a temporary patch directory: {e}")))?;
        let patch_path = tmp.join("changes.patch");
        std::fs::write(&patch_path, patch_bytes)
            .map_err(|e| err(format!("could not write the temporary worktree patch: {e}")))?;
        let out = Command::new("git")
            .arg("-C")
            .arg(worktree)
            .args(["apply", "--whitespace=nowarn"])
            .arg(&patch_path)
            .output()
            .map_err(|e| err(format!("failed to spawn git: {e}")))?;
        drop(guard);
        if !out.status.success() {
            return Err(err(format!(
                "git apply failed: {}",
                String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("")
            )));
        }
    }

    let wt_root = Path::new(worktree);
    for rel in &transfer_paths {
        if rel.ends_with('/') {
            continue; // opaque nested repo / linked worktree
        }
        let src = Path::new(source).join(rel);
        // skip anything that lives inside the target worktree
        let rel_to_target = pathdiff(src.canonicalize().unwrap_or(src.clone()), wt_root);
        if !(rel_to_target == Path::new("..") || rel_to_target.starts_with("..") || rel_to_target.is_absolute()) {
            continue;
        }
        let dst = wt_root.join(rel);
        if dst.exists() || dst.symlink_metadata().is_ok() {
            return Err(err(format!(
                "could not copy {rel} into the detached worktree."
            )));
        }
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| err(format!("could not copy {rel} into the detached worktree: {e}")))?;
        }
        copy_preserve(&src, &dst)
            .map_err(|e| err(format!("could not copy {rel} into the detached worktree: {e}")))?;
    }
    Ok(())
}

struct TmpGuard(PathBuf);
fn scopeguard_tmp(p: &Path) -> TmpGuard {
    TmpGuard(p.to_path_buf())
}
impl Drop for TmpGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn pathdiff(from: PathBuf, to: &Path) -> PathBuf {
    let to_abs = to.canonicalize().unwrap_or_else(|_| to.to_path_buf());
    let from_abs = from;
    let mut result = PathBuf::new();
    let mut fi = from_abs.components().peekable();
    let mut ti = to_abs.components().peekable();
    loop {
        match (fi.peek(), ti.peek()) {
            (Some(a), Some(b)) if a == b => {
                fi.next();
                ti.next();
            }
            (Some(_), Some(_)) => {
                result.push("..");
                ti.next();
            }
            (Some(_), None) => {
                result.push("..");
                fi.next();
            }
            (None, Some(t)) => {
                result.push(t.as_os_str());
                ti.next();
            }
            (None, None) => break,
        }
    }
    if result.as_os_str().is_empty() {
        PathBuf::from("")
    } else {
        result
    }
}

fn copy_preserve(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    let meta = std::fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(src)?;
        std::os::unix::fs::symlink(&target, dst)?;
        return Ok(());
    }
    if meta.is_dir() {
        std::fs::create_dir_all(dst)?;
        std::fs::set_permissions(dst, meta.permissions())?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_preserve(&entry.path(), &dst.join(entry.file_name()))?;
        }
        return Ok(());
    }
    std::fs::copy(src, dst)?;
    std::fs::set_permissions(dst, meta.permissions())?;
    Ok(())
}

fn create_detached_worktree(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let reference = str_arg(payload, "ref")?;
    let new_branch = payload
        .get("newBranch")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let explicit_path = payload
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let copy_from = payload
        .get("copyChangesFrom")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let resolved_ref = stdout(
        cwd,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{reference}^{{commit}}"),
        ],
        "git rev-parse",
    )?
    .trim()
    .to_string();

    if let Some(src) = &copy_from {
        let source_head = stdout(src, &["rev-parse", "--verify", "HEAD^{commit}"], "git")?
            .trim()
            .to_string();
        if source_head != resolved_ref {
            return Err(err(
                "Cannot copy checkout changes because the selected ref is not that checkout's HEAD.",
            ));
        }
    }

    let worktree_path = match explicit_path {
        Some(p) => p,
        None => generated_detached_worktree_path()?,
    };

    if let Some(nb) = &new_branch {
        let out = run(cwd, &["branch", nb, &resolved_ref])?;
        if !out.status.success() {
            return Err(err(format!(
                "git branch failed: {}",
                String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("")
            )));
        }
    }

    let use_parallel = config_value(cwd, "checkout.workers").is_none();
    let mut args: Vec<String> = Vec::new();
    if use_parallel {
        args.extend(["-c".to_string(), "checkout.workers=0".to_string()]);
    }
    args.push("worktree".into());
    args.push("add".into());
    if let Some(nb) = &new_branch {
        args.push(worktree_path.clone());
        args.push(nb.clone());
    } else {
        args.push("--detach".into());
        args.push(worktree_path.clone());
        args.push(resolved_ref.clone());
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(&args)
        .output()
        .map_err(|e| err(format!("failed to spawn git: {e}")))?;
    if !out.status.success() {
        if let Some(nb) = &new_branch {
            let _ = run(cwd, &["branch", "-D", nb]);
        }
        return Err(err(format!(
            "git worktree add failed: {}",
            String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("")
        )));
    }

    if let Some(src) = &copy_from {
        if let Err(e) = copy_checkout_changes(src, &worktree_path) {
            let _ = run(cwd, &["worktree", "remove", "--force", &worktree_path]);
            if let Some(nb) = &new_branch {
                let _ = run(cwd, &["branch", "-D", nb]);
            }
            return Err(e);
        }
    }

    Ok(json!({
        "worktree": {
            "path": worktree_path,
            "ref": resolved_ref,
            "branch": new_branch,
        }
    }))
}

// ---------------------------------------------------------------------------------------------
// git.runStackedAction — linear commit/push/create_pr pipeline.
// Progress-event streaming is not emitted natively (recovery/resume bookkeeping
// lives upstream); the returned per-step statuses match the contract.
// ---------------------------------------------------------------------------------------------

fn run_stacked_action(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    let action = str_arg(payload, "action")?.to_string();
    let wants_commit = matches!(action.as_str(), "commit" | "commit_push" | "commit_push_pr");
    let wants_push = matches!(action.as_str(), "push" | "commit_push" | "commit_push_pr");
    let wants_pr = matches!(action.as_str(), "create_pr" | "commit_push_pr");
    if !(wants_commit || wants_push || wants_pr) {
        return Err(err(format!("unknown stacked action {action}")));
    }
    require_repo(cwd)?;

    // Dirty-tree safety guard for push/pr-only actions.
    let dirty = !stdout(cwd, &["status", "--porcelain"], "git status")?
        .trim()
        .is_empty();
    let allow_dirty = payload
        .get("allowDirtyWorkingTree")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if dirty && !allow_dirty && !wants_commit {
        return Err(err(
            "working tree has uncommitted changes; commit or stash them first",
        ));
    }

    // -- branch step -------------------------------------------------------------
    let mut branch_status = "skipped_not_requested";
    let mut branch_name = Value::Null;
    if payload
        .get("featureBranch")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let base = payload
            .get("commitMessage")
            .and_then(|v| v.as_str())
            .or_else(|| payload.get("threadId").and_then(|v| v.as_str()))
            .unwrap_or("work");
        let slug: String = base
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .take(6)
            .collect::<Vec<_>>()
            .join("-");
        let name = format!("synara/{slug}");
        run(cwd, &["checkout", "-b", &name])
            .and_then(|o| {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(err(String::from_utf8_lossy(&o.stderr).trim().to_string()))
                }
            })?;
        branch_status = "created";
        branch_name = json!(name);
    }

    // -- commit step -------------------------------------------------------------
    let mut commit_status = if wants_commit {
        "skipped_no_changes"
    } else {
        "skipped_not_requested"
    };
    let mut commit_sha = Value::Null;
    let mut commit_subject = Value::Null;
    if wants_commit {
        let file_paths: Vec<String> = payload
            .get("filePaths")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        if file_paths.is_empty() {
            run(cwd, &["add", "-A"])
                .map_err(|e| err(format!("git add failed: {}", e.0)))?;
        } else {
            let mut args = vec!["add", "--"];
            for f in &file_paths {
                args.push(f.as_str());
            }
            run(cwd, &args).map_err(|e| err(format!("git add failed: {}", e.0)))?;
        }
        let staged = !stdout(cwd, &["diff", "--cached", "--name-only"], "git diff")?
            .trim()
            .is_empty();
        if staged {
            let msg = payload
                .get("commitMessage")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    err("commitMessage is required (text generation needs a provider runtime)")
                })?;
            run(cwd, &["commit", "-m", &msg]).and_then(|o| {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(err(String::from_utf8_lossy(&o.stderr).trim().to_string()))
                }
            })?;
            commit_status = "created";
            commit_sha = json!(stdout(cwd, &["rev-parse", "HEAD"], "rev-parse")?
                .trim()
                .to_string());
            commit_subject = json!(stdout(cwd, &["log", "-1", "--pretty=%s"], "log")?
                .trim()
                .to_string());
        }
    }

    // -- push step ---------------------------------------------------------------
    let mut push_status = if wants_push {
        "pushed"
    } else {
        "skipped_not_requested"
    };
    let mut push_branch = Value::Null;
    let mut push_upstream = Value::Null;
    let mut set_upstream = false;
    if wants_push {
        let branch = crate::git_domain::current_branch(cwd)
            .ok_or_else(|| err("not on a branch"))?;
        push_branch = json!(branch);
        let upstream = crate::git_domain::upstream_branch(cwd);
        let unpushed = upstream.as_ref().map(|u| {
            stdout(cwd, &["rev-list", &format!("{u}..HEAD"), "--count"], "rev-list")
                .ok()
                .and_then(|s| s.trim().parse::<i64>().ok())
                .unwrap_or(0)
        });
        if matches!(unpushed, Some(0)) {
            push_status = "skipped_up_to_date";
            push_upstream = upstream.map(|u| json!(u)).unwrap_or(Value::Null);
        } else {
            let args: Vec<String> = if upstream.is_some() {
                vec!["push".into()]
            } else {
                set_upstream = true;
                vec!["push".into(), "-u".into(), "origin".into(), "HEAD".into()]
            };
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run(cwd, &arg_refs).and_then(|o| {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(err(String::from_utf8_lossy(&o.stderr).trim().to_string()))
                }
            })?;
            if set_upstream {
                push_upstream = json!(format!("origin/{branch}"));
            } else {
                push_upstream = upstream.map(|u| json!(u)).unwrap_or(Value::Null);
            }
        }
    }

    // -- pr step -----------------------------------------------------------------
    let mut pr_status = "skipped_not_requested";
    let mut pr_url = Value::Null;
    let mut pr_number = Value::Null;
    let mut pr_base = Value::Null;
    let mut pr_head = Value::Null;
    let mut pr_title = Value::Null;
    if wants_pr {
        let head = crate::git_domain::current_branch(cwd)
            .ok_or_else(|| err("not on a branch"))?;
        pr_head = json!(head.clone());
        let base = stdout(
            cwd,
            &["symbolic-ref", "refs/remotes/origin/HEAD", "--short"],
            "origin HEAD",
        )
        .ok()
        .map(|s| s.trim().trim_start_matches("origin/").to_string())
        .unwrap_or_else(|| "main".to_string());
        pr_base = json!(base.clone());
        // Existing open PR for this head?
        let existing = gh_json(
            &[
                "pr".into(),
                "list".into(),
                "--head".into(),
                head.clone(),
                "--state".into(),
                "open".into(),
                "--json".into(),
                "url,number".into(),
            ],
            cwd,
        )
        .ok();
        let open = existing
            .and_then(|v| v.as_array().cloned())
            .and_then(|a| a.into_iter().next());
        if let Some(pr) = open {
            pr_status = "opened_existing";
            pr_url = pr.get("url").cloned().unwrap_or(Value::Null);
            pr_number = pr.get("number").cloned().unwrap_or(Value::Null);
        } else {
            let title = payload
                .get("prTitle")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| {
                    commit_subject
                        .as_str()
                        .map(str::to_string)
                        .or_else(|| stdout(cwd, &["log", "-1", "--pretty=%s"], "log").ok().map(|s| s.trim().to_string()))
                })
                .ok_or_else(|| err("could not determine a PR title"))?;
            let body = payload
                .get("prBody")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_default();
            let mut args = vec![
                "pr".to_string(),
                "create".to_string(),
                "--title".to_string(),
                title.clone(),
                "--body".to_string(),
                body,
                "--base".to_string(),
                base,
            ];
            if payload
                .get("prDraft")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                args.push("--draft".to_string());
            }
            let out = gh_run(&args, cwd).map_err(gh_err)?;
            if out.code != 0 {
                return Err(err(out.stderr.trim().to_string()));
            }
            let url = out.stdout.trim().to_string();
            pr_url = json!(url);
            pr_status = "created";
            pr_number = url
                .rsplit('/')
                .next()
                .and_then(|n| n.parse::<i64>().ok())
                .map(|n| json!(n))
                .unwrap_or(Value::Null);
            pr_title = json!(title);
        }
        if pr_title.is_null() {
            pr_title = stdout(cwd, &["log", "-1", "--pretty=%s"], "log")
                .ok()
                .map(|s| json!(s.trim().to_string()))
                .unwrap_or(Value::Null);
        }
    }

    Ok(json!({
        "action": action,
        "branch": {"status": branch_status, "name": branch_name},
        "commit": {"status": commit_status, "commitSha": commit_sha, "subject": commit_subject},
        "push": {
            "status": push_status,
            "branch": push_branch,
            "upstreamBranch": push_upstream,
            "setUpstream": set_upstream,
        },
        "pr": {
            "status": pr_status,
            "url": pr_url,
            "number": pr_number,
            "baseBranch": pr_base,
            "headBranch": pr_head,
            "title": pr_title,
        },
    }))
}

// ---------------------------------------------------------------------------------------------
// git.handoffThread — transactional thread re-homing between Local and a worktree,
// ported from upstream GitManager.handoffThread. The durable operation journal
// (beginGitHandoff/completeGitHandoff crash recovery) is a documented gap: the
// git state machine and its rollback paths are faithful.
// ---------------------------------------------------------------------------------------------

fn handoff_opt_str(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn read_head_ref(cwd: &str) -> Option<String> {
    run(cwd, &["rev-parse", "HEAD"])
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_stash_ref(cwd: &str) -> Option<String> {
    run(cwd, &["rev-parse", "--verify", "--quiet", "refs/stash"])
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn has_worktree_changes(cwd: &str) -> bool {
    run(cwd, &["status", "--porcelain"])
        .ok()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}

struct StashEntry {
    had_changes: bool,
    stash_ref: Option<String>,
}

fn stash_working_tree(cwd: &str, label: &str) -> Result<StashEntry, WsErr> {
    if !has_worktree_changes(cwd) {
        return Ok(StashEntry { had_changes: false, stash_ref: None });
    }
    let before = read_stash_ref(cwd);
    let out = run(cwd, &["stash", "push", "--include-untracked", "-m", label])?;
    if !out.status.success() {
        return Err(err(format!(
            "git stash push failed: {}",
            String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("git failed")
        )));
    }
    let after = read_stash_ref(cwd);
    if after == before {
        return Err(err("Git did not create a stash entry while preparing the thread handoff."));
    }
    Ok(StashEntry { had_changes: true, stash_ref: after })
}

struct PopResult {
    conflicts: bool,
    message: Option<String>,
}

fn drop_stash_by_sha(cwd: &str, sha: &str) {
    let Ok(out) = run(cwd, &["stash", "list", "--format=%H"]) else { return };
    if !out.status.success() {
        return;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let index = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .position(|l| l == sha);
    if let Some(i) = index {
        let _ = run(cwd, &["stash", "drop", &format!("stash@{{{i}}}")]);
    }
}

fn pop_stash(cwd: &str, stash_ref: Option<&str>) -> PopResult {
    let Some(sha) = stash_ref else {
        return PopResult { conflicts: false, message: None };
    };
    // Apply by SHA (accepted for any stash-shaped commit) then drop the matching
    // list entry on success, upstream pop-style semantics.
    let result = run(cwd, &["stash", "apply", "--index", sha]);
    match result {
        Ok(out) if out.status.success() => {
            drop_stash_by_sha(cwd, sha);
            PopResult { conflicts: false, message: None }
        }
        Ok(out) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            PopResult {
                conflicts: true,
                message: Some(
                    combined
                        .trim()
                        .lines()
                        .next()
                        .unwrap_or("Git reported conflicts while applying the handed off changes.")
                        .to_string(),
                ),
            }
        }
        Err(e) => PopResult {
            conflicts: true,
            message: Some(format!(
                "Git reported conflicts while applying the handed off changes. ({})",
                e.0
            )),
        },
    }
}

fn checkout_detached(cwd: &str, refspec: &str) -> Result<(), WsErr> {
    let out = run(cwd, &["checkout", "--detach", refspec])?;
    if !out.status.success() {
        return Err(err(format!(
            "git checkout failed: {}",
            String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("git failed")
        )));
    }
    Ok(())
}

fn checkout_branch(cwd: &str, branch: &str) -> Result<(), WsErr> {
    let out = run(cwd, &["checkout", branch])?;
    if !out.status.success() {
        return Err(err(format!(
            "git checkout failed: {}",
            String::from_utf8_lossy(&out.stderr).trim().lines().next().unwrap_or("git failed")
        )));
    }
    Ok(())
}

fn list_local_branches(cwd: &str) -> Vec<String> {
    run(cwd, &["branch", "--format=%(refname:short)"])
        .ok()
        .filter(|o| o.status.success())
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

struct Recovery {
    checkout_restored: bool,
    stash_restored: bool,
    worktree_recreated: bool,
    worktree_removed: bool,
    worktree_changes_restored: bool,
    local_changes_restored: bool,
    notes: Vec<String>,
}

impl Recovery {
    fn new() -> Self {
        Self {
            checkout_restored: false,
            stash_restored: false,
            worktree_recreated: false,
            worktree_removed: false,
            worktree_changes_restored: false,
            local_changes_restored: false,
            notes: Vec::new(),
        }
    }
}

/// Restore the local checkout to its original branch (or detached HEAD) and
/// reapply the preserved stash, upstream restoreLocalHandoffSource.
fn restore_local_source(
    cwd: &str,
    original_branch: Option<&str>,
    original_head: Option<&str>,
    stash_ref: Option<&str>,
) -> Recovery {
    let mut rec = Recovery::new();
    if let Some(branch) = original_branch {
        rec.checkout_restored = checkout_branch(cwd, branch).is_ok();
    } else if let Some(head) = original_head {
        rec.checkout_restored = checkout_detached(cwd, head).is_ok();
    }
    if !rec.checkout_restored {
        rec.notes.push("Restore the checkout manually before retrying.".to_string());
    }
    if stash_ref.is_some() {
        rec.stash_restored = !pop_stash(cwd, stash_ref).conflicts;
        rec.local_changes_restored = rec.stash_restored;
    }
    rec
}

/// Recreate a removed worktree at its branch or ref and reapply its stashed
/// changes, shared by the local-handoff recovery paths.
fn recreate_worktree(
    cwd: &str,
    worktree_path: Option<&str>,
    branch: Option<&str>,
    refspec: Option<&str>,
    worktree_stash: Option<&str>,
) -> (bool, bool) {
    let Some(path) = worktree_path else {
        return (false, false);
    };
    let target = branch.map(String::from).or_else(|| refspec.map(String::from));
    let Some(target) = target else {
        return (false, false);
    };
    let recreated = if branch.is_some() {
        run(cwd, &["worktree", "add", path, &target])
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        run(cwd, &["worktree", "add", "--detach", path, &target])
            .map(|o| o.status.success())
            .unwrap_or(false)
    };
    if !recreated {
        return (false, false);
    }
    let changes = if worktree_stash.is_some() {
        !pop_stash(path, worktree_stash).conflicts
    } else {
        false
    };
    (true, changes)
}

fn handoff_thread(cwd: &str, payload: &Value) -> Result<Value, WsErr> {
    require_repo(cwd)?;
    let target_mode = str_arg(payload, "targetMode")?;
    let current_branch = handoff_opt_str(payload, "currentBranch");
    let worktree_path = handoff_opt_str(payload, "worktreePath");
    let assoc_path = handoff_opt_str(payload, "associatedWorktreePath");
    let assoc_branch = handoff_opt_str(payload, "associatedWorktreeBranch");
    let assoc_ref = handoff_opt_str(payload, "associatedWorktreeRef");
    let preferred_local = handoff_opt_str(payload, "preferredLocalBranch");
    let preferred_base = handoff_opt_str(payload, "preferredWorktreeBaseBranch");
    let preferred_new_name = handoff_opt_str(payload, "preferredNewWorktreeName");

    let local_branch_now = stdout(cwd, &["branch", "--show-current"], "git")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty());

    if target_mode == "local" {
        let Some(wt_path) = worktree_path.clone() else {
            return Err(err(
                "Cannot hand off to Local because this thread does not have a materialized worktree.",
            ));
        };
        let wt_head = read_head_ref(&wt_path);
        let target_local_branch = current_branch
            .clone()
            .or_else(|| assoc_branch.clone())
            .or_else(|| preferred_local.clone());
        if target_local_branch.is_none() && wt_head.is_none() {
            return Err(err(
                "Cannot hand off to Local because the worktree thread does not have a recoverable HEAD reference.",
            ));
        }
        let associated_path = assoc_path.clone().or_else(|| Some(wt_path.clone()));
        let associated_branch = assoc_branch.clone().or_else(|| current_branch.clone());
        let associated_ref = assoc_ref.clone().or_else(|| wt_head.clone()).or_else(|| associated_branch.clone());
        let original_local_branch = local_branch_now.clone();
        let original_local_head = read_head_ref(cwd);

        let preserved_local = stash_working_tree(cwd, &format!("synara preserve local handoff {}", uuid_short()))?;
        let source_stash = stash_working_tree(&wt_path, &format!("synara handoff to local {}", uuid_short()))?;

        // Remove the worktree; on failure restore both stashes before failing.
        let removed = run(cwd, &["worktree", "remove", "--force", &wt_path])
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !removed {
            let _ = pop_stash(&wt_path, source_stash.stash_ref.as_deref());
            let _ = pop_stash(cwd, preserved_local.stash_ref.as_deref());
            return Err(err("git worktree remove failed during the handoff; stashes were restored."));
        }

        // Move the local checkout onto the target branch (or detach at the worktree HEAD).
        let mut checkout_failed: Option<WsErr> = None;
        if let Some(tb) = &target_local_branch {
            if local_branch_now.as_deref() != Some(tb.as_str()) {
                if let Err(e) = checkout_branch(cwd, tb) {
                    checkout_failed = Some(e);
                }
            }
        } else if let Some(head) = &wt_head {
            if let Err(e) = checkout_detached(cwd, head) {
                checkout_failed = Some(e);
            }
        }
        if let Some(e) = checkout_failed {
            // Restore the removed worktree and both stashes, then fail.
            let (recreated, wt_changes) = recreate_worktree(
                cwd,
                associated_path.as_deref(),
                associated_branch.as_deref(),
                associated_ref.as_deref(),
                source_stash.stash_ref.as_deref(),
            );
            let local_restored = !pop_stash(cwd, preserved_local.stash_ref.as_deref()).conflicts;
            let detail = format!(
                "{} {} {} {}",
                e.0,
                if recreated { "The original worktree was recreated." } else { "The original worktree could not be recreated automatically." },
                if wt_changes { "Recovered worktree changes were reapplied." } else { "Recovered worktree changes remain in the Git stash." },
                if local_restored { "Previous local changes were restored." } else { "Previous local changes remain in the Git stash." },
            );
            return Err(err(detail));
        }

        // Apply the thread's changes onto Local.
        let transfer = pop_stash(cwd, source_stash.stash_ref.as_deref());
        if transfer.conflicts {
            // Roll back: restore original checkout, recreate the worktree with its changes.
            let rec = restore_local_source(
                cwd,
                original_local_branch.as_deref(),
                original_local_head.as_deref(),
                preserved_local.stash_ref.as_deref(),
            );
            let (recreated, wt_changes) = recreate_worktree(
                cwd,
                associated_path.as_deref(),
                associated_branch.as_deref(),
                associated_ref.as_deref(),
                source_stash.stash_ref.as_deref(),
            );
            let detail = format!(
                "{} The handoff was rolled back so the thread stays in its worktree. {} {} {} {}",
                transfer.message.as_deref().unwrap_or("Git reported conflicts while applying the handed off changes."),
                if recreated { "The original worktree was recreated." } else { "The original worktree could not be recreated automatically." },
                if wt_changes { "The thread changes were restored to that worktree." } else { "The thread changes remain in the Git stash." },
                if rec.checkout_restored { "Local checkout was restored." } else { "Local checkout could not be fully restored automatically." },
                if rec.local_changes_restored { "Previous local changes were restored." } else { "Previous local changes remain in the Git stash." },
            );
            return Err(err(detail));
        }

        let local_transfer = pop_stash(cwd, preserved_local.stash_ref.as_deref());
        let moved = source_stash.had_changes;
        let restored = preserved_local.had_changes;
        let label = target_local_branch
            .as_ref()
            .map(|b| format!("main local checkout on '{b}'"))
            .unwrap_or_else(|| "local checkout in detached HEAD".to_string());
        let base = match (moved, restored) {
            (true, true) => format!("Moved the thread back to the {label}, carried its uncommitted work over, and restored your previous local changes."),
            (true, false) => format!("Moved the thread back to the {label} and carried its uncommitted work over."),
            (false, true) => format!("Moved the thread back to the {label} and restored your previous local changes."),
            (false, false) => format!("Moved the thread back to the {label}."),
        };
        let message = if local_transfer.conflicts {
            format!(
                "{}\nYour previous local stash entry was kept for recovery.",
                local_transfer.message.as_deref().unwrap_or("Git reported conflicts while restoring your previous local changes.")
            )
        } else {
            base
        };
        return Ok(json!({
            "targetMode": "local",
            "branch": target_local_branch,
            "worktreePath": null,
            "associatedWorktreePath": associated_path,
            "associatedWorktreeBranch": associated_branch,
            "associatedWorktreeRef": associated_ref,
            "changesTransferred": moved || restored,
            "conflictsDetected": local_transfer.conflicts,
            "message": message,
        }));
    }

    // target_mode == "worktree"
    let intent_name = preferred_new_name.clone();
    let reuse = assoc_path.is_some() || assoc_branch.is_some() || assoc_ref.is_some();
    if intent_name.is_none() && !reuse {
        return Err(err("Cannot hand off to a worktree because no worktree target is available."));
    }
    let base_branch = preferred_base.clone().or_else(|| current_branch.clone()).or_else(|| local_branch_now.clone());
    if base_branch.is_none() && assoc_branch.is_none() && assoc_ref.is_none() {
        return Err(err("Select a base branch before handing off this thread to a worktree."));
    }

    let source_stash = stash_working_tree(cwd, &format!("synara handoff to worktree {}", uuid_short()))?;
    let source_branch = local_branch_now.clone().or_else(|| current_branch.clone());
    let source_head = read_head_ref(cwd);
    let mut foreground_after = local_branch_now.clone();

    // If Local is on the branch the worktree will take, move Local aside first.
    if source_branch.is_some() && source_branch == assoc_branch {
        let excluded = assoc_branch.clone().unwrap_or_default();
        let fallback = list_local_branches(cwd).into_iter().find(|b| *b != excluded);
        match fallback {
            None => {
                let Some(head) = source_head.clone() else {
                    let _ = pop_stash(cwd, source_stash.stash_ref.as_deref());
                    return Err(err(format!(
                        "Cannot hand off '{excluded}' to a worktree because there is no recoverable local HEAD reference available."
                    )));
                };
                if let Err(e) = checkout_detached(cwd, &head) {
                    let _ = pop_stash(cwd, source_stash.stash_ref.as_deref());
                    return Err(e);
                }
                foreground_after = None;
            }
            Some(fb) => {
                if let Err(e) = checkout_branch(cwd, &fb) {
                    let _ = pop_stash(cwd, source_stash.stash_ref.as_deref());
                    return Err(e);
                }
                foreground_after = Some(fb);
            }
        }
    }

    // Materialize the worktree.
    let created: Result<(String, Option<String>), WsErr> = (|| {
        if let (Some(aref), None) = (assoc_ref.clone(), assoc_branch.clone()) {
            let v = create_detached_worktree(
                cwd,
                &json!({"ref": aref, "path": assoc_path}),
            )?;
            let path = v.pointer("/worktree/path").and_then(Value::as_str).unwrap_or_default().to_string();
            return Ok((path, None));
        }
        if let Some(name) = &intent_name {
            let Some(base) = &base_branch else {
                return Err(err("Select a base branch before creating a new worktree."));
            };
            let v = crate::git_domain::handle(
                "git.createWorktree",
                &json!({"cwd": cwd, "branch": base, "newBranch": name}),
            )?;
            let path = v
                .as_ref()
                .and_then(|v| v.pointer("/worktree/path"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if path.is_empty() {
                return Err(err("createWorktree did not return a path"));
            }
            return Ok((path, Some(name.clone())));
        }
        if let Some(ab) = &assoc_branch {
            if list_local_branches(cwd).iter().any(|b| b == ab) {
                let v = crate::git_domain::handle(
                    "git.createWorktree",
                    &json!({"cwd": cwd, "branch": ab, "path": assoc_path}),
                )?;
                let path = v
                    .as_ref()
                    .and_then(|v| v.pointer("/worktree/path"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                return Ok((path, Some(ab.clone())));
            }
            if base_branch.is_none() {
                let v = create_detached_worktree(cwd, &json!({"ref": ab, "path": assoc_path}))?;
                let path = v.pointer("/worktree/path").and_then(Value::as_str).unwrap_or_default().to_string();
                return Ok((path, None));
            }
            let v = crate::git_domain::handle(
                "git.createWorktree",
                &json!({"cwd": cwd, "branch": base_branch, "newBranch": ab, "path": assoc_path}),
            )?;
            let path = v
                .as_ref()
                .and_then(|v| v.pointer("/worktree/path"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            return Ok((path, Some(ab.clone())));
        }
        let target = base_branch.clone().or_else(|| assoc_ref.clone()).ok_or_else(|| err("no worktree target"))?;
        let v = create_detached_worktree(cwd, &json!({"ref": target, "path": assoc_path}))?;
        let path = v.pointer("/worktree/path").and_then(Value::as_str).unwrap_or_default().to_string();
        Ok((path, None))
    })();

    let (wt_created_path, wt_created_branch) = match created {
        Ok(v) => v,
        Err(e) => {
            let rec = restore_local_source(
                cwd,
                source_branch.as_deref(),
                source_head.as_deref(),
                source_stash.stash_ref.as_deref(),
            );
            let detail = format!(
                "{} {} {}",
                e.0,
                if rec.checkout_restored { "Local checkout was restored." } else { "Local checkout could not be fully restored automatically." },
                if rec.stash_restored { "Previous local changes were restored." } else { "Previous local changes remain in the Git stash." },
            );
            return Err(err(detail));
        }
    };

    // Transfer the stashed changes into the worktree.
    let transfer = pop_stash(&wt_created_path, source_stash.stash_ref.as_deref());
    if transfer.conflicts {
        let removed = run(cwd, &["worktree", "remove", "--force", &wt_created_path])
            .map(|o| o.status.success())
            .unwrap_or(false);
        // The pop partially applied in the worktree; the stash entry itself
        // survives because apply-by-sha only drops on success.
        let rec = restore_local_source(
            cwd,
            source_branch.as_deref(),
            source_head.as_deref(),
            source_stash.stash_ref.as_deref(),
        );
        let detail = format!(
            "{} The stash entry was kept for recovery. {} {} {}",
            transfer.message.as_deref().unwrap_or("Git reported conflicts while applying the handed off changes."),
            if removed { "The new worktree was removed." } else { "The new worktree could not be removed automatically." },
            if rec.checkout_restored { "Local checkout was restored." } else { "Local checkout could not be fully restored automatically." },
            if rec.stash_restored { "Previous local changes were restored." } else { "Previous local changes remain in the Git stash. Run `git stash list` in Local to recover them." },
        );
        return Err(err(detail));
    }

    let materialized_branch = stdout(&wt_created_path, &["branch", "--show-current"], "git")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .or(wt_created_branch);
    let materialized_ref = read_head_ref(&wt_created_path).or_else(|| materialized_branch.clone());
    if let Some(branch) = &materialized_branch {
        // Publishing is best-effort: local-only repositories still hand off fine.
        let _ = run(&wt_created_path, &["push", "-u", "origin", branch]);
    }

    let moved = source_stash.had_changes;
    let summary = if foreground_after.is_some() && foreground_after != source_branch {
        format!(
            "The thread moved into its worktree and Local returned to '{}'.",
            foreground_after.as_deref().unwrap_or_default()
        )
    } else if foreground_after.is_none() && source_branch == assoc_branch {
        "The thread moved into its worktree and Local returned to a detached HEAD.".to_string()
    } else {
        "The thread moved into its worktree.".to_string()
    };
    let message = if moved {
        format!("{summary} Uncommitted local changes were carried over.")
    } else {
        summary
    };

    Ok(json!({
        "targetMode": "worktree",
        "branch": materialized_branch,
        "worktreePath": wt_created_path,
        "associatedWorktreePath": wt_created_path,
        "associatedWorktreeBranch": materialized_branch,
        "associatedWorktreeRef": materialized_ref,
        "changesTransferred": moved,
        "conflictsDetected": false,
        "message": message,
    }))
}

fn uuid_short() -> String {
    crate::orchestration_engine::uuid_v4()
}
