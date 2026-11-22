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
