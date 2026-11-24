//! `pullRequests.*` native handlers: PR detail/diff/action/comment, project pins and
//! Auto-fix CI state. All GitHub access goes through the `gh` CLI exactly like the
//! upstream server layer; repository identity and pins live in state.sqlite.

use rusqlite::params;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::git_domain::run as git;
use crate::github_inbox::{gh_run, summarize_error, GhOutput};
use crate::todo::WsErr;

const DETAIL_FIELDS: &str = "number,title,body,url,author,state,isDraft,mergeable,mergeStateStatus,additions,deletions,changedFiles,headRefName,baseRefName,reviewDecision,reviewRequests,reviews,comments,statusCheckRollup,commits,labels,maintainerCanModify,createdAt,updatedAt,mergedAt,closedAt";
const SUMMARY_FIELDS: &str = "number,title,url,baseRefName,headRefName,state,mergedAt,isDraft,mergeable,additions,deletions,changedFiles,isCrossRepository,headRepository,headRepositoryOwner,updatedAt";
const MAX_DIFF_BYTES: usize = 8 * 1024 * 1024;
const MAX_REVIEW_COMMENTS: usize = 20;
const MAX_REVIEW_PAGES: usize = 5;
const MAX_PINS: i64 = 20;

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

/// Tagged `PullRequestsUnavailableError` — the relay's error_json translates the
/// `PR_UNAVAILABLE:` code prefix into the schema-tagged wire shape.
fn unavailable(reason: &str, message: impl Into<String>) -> WsErr {
    let code: &'static str = Box::leak(format!("PR_UNAVAILABLE:{reason}").into_boxed_str());
    WsErr(message.into(), Some(code))
}

pub fn handles(tag: &str) -> bool {
    tag.starts_with("pullRequests.")
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "pullRequests.detail" => detail(payload),
        "pullRequests.diff" => diff(payload),
        "pullRequests.action" => action(payload),
        "pullRequests.comment" => comment(payload),
        "pullRequests.setPinned" => set_pinned(payload),
        "pullRequests.getAutoFix" => get_auto_fix(payload),
        "pullRequests.setAutoFix" => set_auto_fix(payload),
        _ => Err(err(format!("unhandled native tag {tag}"))),
    }
}

// ---------------------------------------------------------------------------------------------
// payload + identity helpers
// ---------------------------------------------------------------------------------------------

fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

fn opt_str(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty())
}

fn str_at<'a>(v: &'a Value, k: &str) -> &'a str {
    opt_str(get(v, k)).unwrap_or("")
}

fn nodes<'a>(v: &'a Value, k: &str) -> Vec<&'a Value> {
    get(v, k)
        .as_array()
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

fn str_field(payload: &Value, key: &str) -> Result<String, WsErr> {
    opt_str(get(payload, key))
        .map(str::to_string)
        .ok_or_else(|| err(format!("{key} is required")))
}

fn int_field(payload: &Value, key: &str) -> Result<i64, WsErr> {
    get(payload, key)
        .as_i64()
        .filter(|n| *n > 0)
        .ok_or_else(|| err(format!("{key} must be a positive integer")))
}

/// `validateRepository`: exactly one `owner/name` segment pair.
fn valid_repo(repo: &str) -> bool {
    let Some((owner, name)) = repo.split_once('/') else {
        return false;
    };
    if owner.is_empty() || name.contains('/') {
        return false;
    }
    if !owner
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
        || !owner.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        || !owner
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return false;
    }
    name.len() <= 100
        && !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

pub(crate) fn validate_repository(repo: &str) -> Result<String, WsErr> {
    let normalized = repo.trim();
    if valid_repo(normalized) {
        Ok(normalized.to_string())
    } else {
        Err(err("Invalid GitHub repository identity."))
    }
}

fn selector(repo: &str) -> String {
    format!("github.com/{repo}")
}

/// `parseGitHubRepositoryNameWithOwnerFromRemoteUrl` — the four URL forms.
fn repo_from_remote_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    for prefix in [
        "git@github.com:",
        "ssh://git@github.com/",
        "https://github.com/",
        "git://github.com/",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest.split(['?', '#']).next().unwrap_or(rest);
            let rest = rest.trim_end_matches('/');
            let rest = rest.strip_suffix(".git").unwrap_or(rest);
            let mut parts = rest.split('/');
            if let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) {
                let candidate = format!("{owner}/{name}");
                if valid_repo(&candidate) {
                    return Some(candidate);
                }
            }
            return None;
        }
    }
    // Credential-stripping retry: https://user:pass@github.com/owner/repo
    if let Some(pos) = trimmed.find("github.com/") {
        let rest = &trimmed[pos + "github.com/".len()..];
        let rest = rest.trim_end_matches('/');
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        let mut parts = rest.split('/');
        if let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) {
            let candidate = format!("{owner}/{name}");
            if valid_repo(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// `normalizeGitHubPullRequestUrl` — canonical `https://github.com/o/r/pull/n`.
fn normalize_pr_url(url: &str) -> Option<String> {
    let rest = url.trim().strip_prefix("https://github.com/")?;
    let rest = rest.split(['?', '#']).next().unwrap_or(rest);
    let parts: Vec<&str> = rest.trim_end_matches('/').split('/').collect();
    if parts.len() == 4 && parts[2] == "pull" {
        if let Ok(n) = parts[3].parse::<i64>() {
            if n > 0 {
                let repo = format!("{}/{}", parts[0], parts[1]);
                if valid_repo(&repo) {
                    return Some(format!("https://github.com/{repo}/pull/{n}"));
                }
            }
        }
    }
    None
}

fn avatar_url_for(login: &str) -> String {
    format!("https://github.com/{login}.png")
}

fn actor(node: &Value) -> Value {
    match opt_str(get(node, "login")).or_else(|| opt_str(get(node, "slug"))) {
        Some(login) => json!({
            "login": login,
            "name": opt_str(get(node, "name")),
            "avatarUrl": opt_str(get(node, "avatarUrl")).map(str::to_string)
                .or_else(|| Some(avatar_url_for(login))),
            "url": opt_str(get(node, "url")),
        }),
        None => Value::Null,
    }
}

fn pr_state(node: &Value) -> &'static str {
    if opt_str(get(node, "mergedAt")).is_some() || get(node, "state").as_str() == Some("MERGED") {
        "merged"
    } else if get(node, "state").as_str() == Some("CLOSED") {
        "closed"
    } else {
        "open"
    }
}

fn mergeability(raw: Option<&str>) -> &'static str {
    match raw {
        Some("MERGEABLE") => "mergeable",
        Some("CONFLICTING") => "conflicting",
        _ => "unknown",
    }
}

fn non_negative(v: &Value) -> i64 {
    v.as_i64().unwrap_or(0).max(0)
}

// ---------------------------------------------------------------------------------------------
// Project + repository inventory
// ---------------------------------------------------------------------------------------------

struct Project {
    id: String,
    title: String,
    workspace_root: String,
    #[allow(dead_code)]
    kind: String,
}

fn find_project(project_id: &str) -> Result<Project, WsErr> {
    let Some(copy) = crate::orchestration_snapshot::open_state_copy() else {
        return Err(err("Project not found."));
    };
    let mut stmt = copy
        .conn
        .prepare(
            "SELECT project_id, title, workspace_root, kind FROM projection_projects \
             WHERE project_id = ?1 AND deleted_at IS NULL",
        )
        .map_err(|e| err(format!("{e}")))?;
    let row = stmt
        .query_row(params![project_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3).unwrap_or_default(),
            ))
        })
        .ok();
    match row {
        Some((id, title, workspace_root, kind)) => Ok(Project {
            id,
            title,
            workspace_root,
            kind,
        }),
        None => Err(err("Project not found.")),
    }
}

/// `resolveGitHubRepositories(cwd)`: branch remote → pushDefault → origin → sorted names.
pub(crate) fn resolve_repositories(cwd: &str) -> Result<Vec<String>, WsErr> {
    let unavailable =
        || err("GitHub repository inventory is unavailable.".to_string());
    let branch_out = git(cwd, &["branch", "--show-current"]).map_err(|_| unavailable())?;
    if !branch_out.status.success() {
        return Err(unavailable());
    }
    let branch = String::from_utf8_lossy(&branch_out.stdout)
        .trim()
        .to_string();
    let pattern = "^(remote\\..*\\.url|remote\\.pushDefault|branch\\..*\\.remote)$";
    let cfg = git(cwd, &["config", "--null", "--get-regexp", pattern])
        .map_err(|_| unavailable())?;
    if !cfg.status.success() && cfg.status.code() != Some(1) {
        return Err(unavailable());
    }
    let text = String::from_utf8_lossy(&cfg.stdout);
    let mut branch_remote: Option<String> = None;
    let mut push_default: Option<String> = None;
    let mut urls: BTreeMap<String, String> = BTreeMap::new();
    let branch_remote_key = format!("branch.{branch}.remote").to_lowercase();
    for record in text.split('\0') {
        let Some((key, value)) = record.split_once('\n') else {
            continue;
        };
        let key = key.trim_end_matches('\r');
        let value = value.trim();
        if value.is_empty() || value == "." {
            continue;
        }
        let lower_key = key.to_lowercase();
        if lower_key == branch_remote_key {
            branch_remote = Some(value.to_string());
        } else if lower_key == "remote.pushdefault" {
            push_default = Some(value.to_string());
        } else if lower_key.starts_with("remote.") && lower_key.ends_with(".url") && key.len() > "remote..url".len() {
            let name = &key["remote.".len()..key.len() - ".url".len()];
            urls.entry(name.to_string()).or_insert_with(|| value.to_string());
        }
    }
    let mut candidates: Vec<String> = Vec::new();
    for c in [branch_remote, push_default, Some("origin".into())]
        .into_iter()
        .flatten()
    {
        candidates.push(c);
    }
    candidates.extend(urls.keys().cloned());
    let mut seen = HashSet::new();
    let mut repos = Vec::new();
    for name in candidates {
        if !seen.insert(name.clone()) || !urls.contains_key(&name) {
            continue;
        }
        let mut repo = urls.get(&name).and_then(|u| repo_from_remote_url(u));
        if repo.is_none() {
            // Alias expansion: `git remote get-url` resolves insteadOf/shortnames.
            if let Ok(out) = git(cwd, &["remote", "get-url", &name]) {
                if out.status.success() {
                    repo = repo_from_remote_url(&String::from_utf8_lossy(&out.stdout));
                }
            }
        }
        if let Some(r) = repo {
            repos.push(r);
        }
    }
    let mut deduped: Vec<String> = Vec::new();
    let mut lower = HashSet::new();
    for r in repos {
        if lower.insert(r.to_lowercase()) {
            deduped.push(r);
        }
    }
    Ok(deduped)
}

/// `validateProjectRepository`: identity + membership in the project's remote inventory.
fn validate_project_repository(project: &Project, repo: &str) -> Result<String, WsErr> {
    let _ = validate_repository(repo)?;
    let repos = resolve_repositories(&project.workspace_root)?;
    repos
        .into_iter()
        .find(|r| r.eq_ignore_ascii_case(repo))
        .ok_or_else(|| err("GitHub repository does not belong to the selected project."))
}

// ---------------------------------------------------------------------------------------------
// gh plumbing
// ---------------------------------------------------------------------------------------------

pub(crate) fn gh_stdin(args: &[String], stdin: &str, cwd: &str) -> Result<GhOutput, String> {
    let mut child = Command::new("gh")
        .args(args)
        .current_dir(if cwd.is_empty() { "." } else { cwd })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("gh CLI is not installed or failed to launch ({e})"))?;
    if let Some(mut pipe) = child.stdin.take() {
        let _ = pipe.write_all(stdin.as_bytes());
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("gh failed to finish ({e})"))?;
    Ok(GhOutput {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

fn is_rate_limit(stderr: &str) -> bool {
    let low = stderr.to_lowercase();
    low.contains("rate limit") || low.contains("http 429") || low.contains("abuse detection")
}

/// `toPullRequestsRpcError`-equivalent mapping for a failed `gh` call.
fn gh_failed(out: &GhOutput, fallback: &str) -> WsErr {
    let low = out.stderr.to_lowercase();
    if low.contains("gh auth login") {
        return unavailable(
            "gh-not-authenticated",
            "GitHub CLI is not authenticated. Run `gh auth login` and retry.",
        );
    }
    if is_rate_limit(&out.stderr) {
        return unavailable("rate-limited", summarize_error(&out.stderr));
    }
    // Upstream `rpcEffect(op, fallback)`: any non-tagged failure surfaces the
    // per-method fallback text, not the gh stderr.
    err(fallback)
}

fn gh_spawn(e: String) -> WsErr {
    unavailable(
        "gh-not-installed",
        format!("GitHub CLI (`gh`) is required but not available on PATH. ({e})"),
    )
}

/// Run `gh`; success → output, spawn error → gh-not-installed, exit → mapped error.
fn gh(args: &[String], cwd: &str, fallback: &str) -> Result<GhOutput, WsErr> {
    let out = gh_run(args, cwd).map_err(gh_spawn)?;
    if out.code != 0 {
        return Err(gh_failed(&out, fallback));
    }
    Ok(out)
}

fn gh_json_v(args: &[String], cwd: &str, fallback: &str) -> Result<Value, WsErr> {
    let out = gh(args, cwd, fallback)?;
    serde_json::from_str(&out.stdout)
        .map_err(|e| err(format!("{fallback}: gh returned invalid JSON ({e})")))
}

fn graphql(query: &str, vars: &[String], cwd: &str, fallback: &str) -> Result<Value, WsErr> {
    let mut args: Vec<String> = vec![
        "api".into(),
        "graphql".into(),
        "--hostname".into(),
        "github.com".into(),
        "-f".into(),
        format!("query={query}"),
    ];
    args.extend(vars.iter().cloned());
    gh_json_v(&args, cwd, fallback)
}

// ---------------------------------------------------------------------------------------------
// detail
// ---------------------------------------------------------------------------------------------

fn normalize_checks(rollup: &Value) -> Vec<Value> {
    let mut checks = Vec::new();
    for node in nodes(rollup, "nodes") {
        let typename = str_at(node, "__typename");
        if typename == "CheckRun" {
            let name = {
                let n = str_at(node, "name");
                match opt_str(get(get(node, "workflow"), "name")) {
                    Some(wf) => format!("{wf} / {n}"),
                    None => n.to_string(),
                }
            };
            let status = if get(node, "status").as_str() != Some("COMPLETED") {
                "pending"
            } else {
                match get(node, "conclusion").as_str() {
                    Some("SUCCESS") => "success",
                    Some("FAILURE") | Some("ERROR") => "failure",
                    Some("SKIPPED") => "skipped",
                    Some("NEUTRAL") => "neutral",
                    Some("CANCELLED") => "cancelled",
                    _ => "pending",
                }
            };
            checks.push(json!({
                "name": name,
                "status": status,
                "description": Value::Null,
                "url": opt_str(get(node, "detailsUrl")),
                "startedAt": opt_str(get(node, "startedAt")),
                "completedAt": opt_str(get(node, "completedAt")),
            }));
        } else if typename == "StatusContext" {
            let status = match get(node, "state").as_str() {
                Some("SUCCESS") => "success",
                Some("FAILURE") | Some("ERROR") => "failure",
                _ => "pending",
            };
            checks.push(json!({
                "name": str_at(node, "context"),
                "status": status,
                "description": opt_str(get(node, "description")),
                "url": opt_str(get(node, "targetUrl")),
                "startedAt": opt_str(get(node, "createdAt")),
                "completedAt": opt_str(get(node, "createdAt")),
            }));
        }
    }
    checks
}

fn normalize_commits(commits: &Value) -> Vec<Value> {
    nodes(commits, "nodes")
        .into_iter()
        .map(|c| {
            let commit = get(c, "commit");
            let authors = nodes(commit, "authors")
                .into_iter()
                .map(|a| {
                    let login = opt_str(get(a, "login"));
                    let user = get(a, "user");
                    let real_login = login.or_else(|| opt_str(get(user, "login")));
                    json!({
                        "login": real_login,
                        "name": opt_str(get(a, "name")).or_else(|| opt_str(get(user, "name"))),
                        "avatarUrl": real_login.map(avatar_url_for)
                            .or_else(|| opt_str(get(user, "avatarUrl")).map(str::to_string)),
                        "url": real_login
                            .filter(|l| !l.starts_with("app/"))
                            .map(|l| format!("https://github.com/{l}")),
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "oid": str_at(commit, "oid"),
                "messageHeadline": str_at(commit, "messageHeadline"),
                "messageBody": str_at(commit, "messageBody"),
                "committedDate": str_at(commit, "committedDate"),
                "authors": authors,
            })
        })
        .collect()
}

fn normalize_reviewers(detail: &Value) -> Vec<Value> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for n in nodes(get(detail, "reviewRequests"), "nodes") {
        let reviewer = get(n, "requestedReviewer");
        if let Some(login) = opt_str(get(reviewer, "login")).or_else(|| opt_str(get(reviewer, "slug")))
        {
            if seen.insert(login.to_lowercase()) {
                out.push(actor(reviewer));
            }
        }
    }
    for r in nodes(get(detail, "reviews"), "nodes") {
        let a = get(r, "author");
        if let Some(login) = opt_str(get(a, "login")) {
            if seen.insert(login.to_lowercase()) {
                out.push(actor(a));
            }
        }
    }
    out
}

fn issue_comments(detail: &Value) -> Vec<Value> {
    nodes(get(detail, "comments"), "nodes")
        .into_iter()
        .map(|c| {
            json!({
                "id": str_at(c, "id"),
                "kind": "issue-comment",
                "author": actor(get(c, "author")),
                "body": str_at(c, "body"),
                "createdAt": str_at(c, "createdAt"),
                "updatedAt": opt_str(get(c, "updatedAt")),
                "url": opt_str(get(c, "url")),
                "path": Value::Null,
                "reviewState": Value::Null,
            })
        })
        .collect()
}

fn review_comments(detail: &Value) -> Vec<Value> {
    nodes(get(detail, "reviews"), "nodes")
        .into_iter()
        .filter(|r| !str_at(r, "body").is_empty())
        .map(|r| {
            json!({
                "id": str_at(r, "id"),
                "kind": "review",
                "author": actor(get(r, "author")),
                "body": str_at(r, "body"),
                "createdAt": opt_str(get(r, "submittedAt")).unwrap_or(str_at(r, "createdAt")),
                "updatedAt": opt_str(get(r, "updatedAt")),
                "url": opt_str(get(r, "url")),
                "path": Value::Null,
                "reviewState": opt_str(get(r, "state")),
            })
        })
        .collect()
}

const REVIEW_THREADS_QUERY: &str = "query($owner:String!,$name:String!,$number:Int!,$cursor:String) { \
repository(owner:$owner,name:$name) { pullRequest(number:$number) { \
reviewThreads(first:50, after:$cursor) { pageInfo { hasNextPage endCursor } \
nodes { path line comments(first:1) { nodes { id body url createdAt updatedAt \
author { login avatarUrl url ... on User { name } } } } } } } } }";

fn review_thread_comments(
    owner: &str,
    name: &str,
    number: i64,
    cwd: &str,
) -> Result<(Vec<Value>, bool), WsErr> {
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    let mut incomplete = false;
    for _ in 0..MAX_REVIEW_PAGES {
        if out.len() >= MAX_REVIEW_COMMENTS {
            break;
        }
        let mut vars = vec![
            "-f".into(),
            format!("owner={owner}"),
            "-f".into(),
            format!("name={name}"),
            "-F".into(),
            format!("number={number}"),
        ];
        if let Some(c) = &cursor {
            vars.push("-f".into());
            vars.push(format!("cursor={c}"));
        }
        let data = graphql(REVIEW_THREADS_QUERY, &vars, cwd, "Failed to load pull request")?;
        let pr = get(get(get(&data, "data"), "repository"), "pullRequest");
        let threads = get(pr, "reviewThreads");
        for t in nodes(threads, "nodes") {
            for c in nodes(get(t, "comments"), "nodes").into_iter().take(1) {
                out.push(json!({
                    "id": str_at(c, "id"),
                    "kind": "review-comment",
                    "author": actor(get(c, "author")),
                    "body": str_at(c, "body"),
                    "createdAt": str_at(c, "createdAt"),
                    "updatedAt": opt_str(get(c, "updatedAt")),
                    "url": opt_str(get(c, "url")),
                    "path": opt_str(get(t, "path")),
                    "reviewState": Value::Null,
                }));
                if out.len() >= MAX_REVIEW_COMMENTS {
                    break;
                }
            }
        }
        let page = get(threads, "pageInfo");
        if get(page, "hasNextPage").as_bool() == Some(true) {
            cursor = opt_str(get(page, "endCursor")).map(str::to_string);
            if cursor.is_none() {
                break;
            }
        } else {
            break;
        }
    }
    if out.len() >= MAX_REVIEW_COMMENTS {
        incomplete = true;
    }
    Ok((out, incomplete))
}

const STACK_QUERY: &str = "query($owner:String!,$name:String!,$number:Int!,$cursor:String) { \
repository(owner:$owner,name:$name) { pullRequest(number:$number) { \
stackEntry { position stack { number size baseRefName \
entries(first:100, after:$cursor) { pageInfo { hasNextPage endCursor } \
nodes { position pullRequest { number title url state isDraft headRefName baseRefName \
mergeable mergeStateStatus } } } } } } } }";

/// Stack lookup is best-effort: `Ok(None)` means standalone, `Err` means metadata incomplete.
fn read_stack(
    owner: &str,
    name: &str,
    number: i64,
    cwd: &str,
) -> Result<Option<(Value, bool)>, WsErr> {
    let mut entries: Vec<Value> = Vec::new();
    let mut cursor: Option<String> = None;
    let mut meta: Option<Value> = None;
    let mut position = 0_i64;
    for _ in 0..10 {
        let mut vars = vec![
            "-f".into(),
            format!("owner={owner}"),
            "-f".into(),
            format!("name={name}"),
            "-F".into(),
            format!("number={number}"),
        ];
        if let Some(c) = &cursor {
            vars.push("-f".into());
            vars.push(format!("cursor={c}"));
        }
        let data = match graphql(STACK_QUERY, &vars, cwd, "") {
            Ok(d) => d,
            Err(e) => {
                // GraphQL schema lacks stack fields on older GHES/GH errors: treat as standalone.
                if e.0.is_empty() || e.0.contains("stack") {
                    return Ok(None);
                }
                return Err(e);
            }
        };
        let pr = get(get(get(&data, "data"), "repository"), "pullRequest");
        let entry = get(pr, "stackEntry");
        let stack = get(entry, "stack");
        if stack.is_null() {
            return Ok(None);
        }
        position = get(entry, "position").as_i64().unwrap_or(0);
        if meta.is_none() {
            meta = Some(stack.clone());
        }
        for n in nodes(get(stack, "entries"), "nodes") {
            entries.push(n.clone());
        }
        // Entries hold `position` + `pullRequest`; flatten below.
        let page = get(get(stack, "entries"), "pageInfo");
        if get(page, "hasNextPage").as_bool() == Some(true) {
            cursor = opt_str(get(page, "endCursor")).map(str::to_string);
            if cursor.is_none() {
                break;
            }
        } else {
            break;
        }
    }
    let Some(stack) = meta else {
        return Ok(None);
    };
    let size = get(&stack, "size").as_i64().unwrap_or(entries.len() as i64);
    let incomplete = size != entries.len() as i64;
    let normalized: Vec<Value> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let n = get(e, "pullRequest");
            json!({
                "position": get(e, "position").as_i64().unwrap_or(i as i64 + 1),
                "number": get(n, "number").as_i64().unwrap_or(0),
                "title": str_at(n, "title"),
                "url": str_at(n, "url"),
                "headBranch": str_at(n, "headRefName"),
                "baseBranch": str_at(n, "baseRefName"),
                "state": pr_state(n),
                "isDraft": get(n, "isDraft").as_bool().unwrap_or(false),
                "mergeability": mergeability(get(n, "mergeable").as_str()),
                "mergeStateStatus": opt_str(get(n, "mergeStateStatus")),
            })
        })
        .collect();
    Ok(Some((
        json!({
            "number": get(&stack, "number").as_i64().unwrap_or(0),
            "size": size,
            "position": position,
            "baseBranch": str_at(&stack, "baseRefName"),
            "entries": normalized,
        }),
        incomplete,
    )))
}

fn read_merge_caps(repo: &str, cwd: &str) -> Result<Value, WsErr> {
    let v = gh_json_v(
        &[
            "repo".into(),
            "view".into(),
            selector(repo),
            "--json".into(),
            "mergeCommitAllowed,squashMergeAllowed,rebaseMergeAllowed,deleteBranchOnMerge".into(),
        ],
        cwd,
        "Failed to load pull request merge capabilities",
    )?;
    Ok(json!({
        "merge": get(&v, "mergeCommitAllowed").as_bool().unwrap_or(false),
        "squash": get(&v, "squashMergeAllowed").as_bool().unwrap_or(false),
        "rebase": get(&v, "rebaseMergeAllowed").as_bool().unwrap_or(false),
        "deleteBranchOnMerge": get(&v, "deleteBranchOnMerge").as_bool().unwrap_or(false),
    }))
}

fn detail(payload: &Value) -> Result<Value, WsErr> {
    let project = find_project(&str_field(payload, "projectId")?)?;
    let repo = validate_project_repository(&project, &str_field(payload, "repository")?)?;
    let number = int_field(payload, "number")?;
    let cwd = project.workspace_root.clone();
    let pr_view = |fields: &str| {
        gh_json_v(
            &[
                "pr".into(),
                "view".into(),
                number.to_string(),
                "--repo".into(),
                selector(&repo),
                "--json".into(),
                fields.into(),
            ],
            &cwd,
            "Failed to load pull request",
        )
    };
    // `reviewRequests` pulls Team nodes which need `read:org`; tokens without it
    // make the whole `gh pr view` fail. Retry with the org-scoped field dropped.
    let d = pr_view(DETAIL_FIELDS)
        .or_else(|_| pr_view(&DETAIL_FIELDS.replace(",reviewRequests", "")))?;
    let (owner, name) = repo.split_once('/').unwrap();
    let caps = read_merge_caps(&repo, &cwd)?;
    let (mut comments, incomplete) =
        review_thread_comments(owner, name, number, &cwd).unwrap_or_default();
    comments.extend(issue_comments(&d));
    comments.extend(review_comments(&d));
    comments.sort_by(|a, b| str_at(a, "createdAt").cmp(str_at(b, "createdAt")));
    let truncated = get(get(&d, "comments"), "totalCount")
        .as_i64()
        .is_some_and(|t| t > 100);
    let (stack, stack_incomplete) = match read_stack(owner, name, number, &cwd) {
        Ok(Some((s, inc))) => (s, inc),
        Ok(None) => (Value::Null, false),
        Err(_) => (Value::Null, true),
    };
    let mergeable_raw = get(&d, "mergeable").as_str().map(str::to_string);
    Ok(json!({
        "projectId": project.id,
        "projectTitle": project.title,
        "workspaceRoot": project.workspace_root,
        "repository": repo,
        "number": get(&d, "number").as_i64().unwrap_or(number),
        "title": str_at(&d, "title"),
        "body": str_at(&d, "body"),
        "url": str_at(&d, "url"),
        "author": actor(get(&d, "author")),
        "state": pr_state(&d),
        "isDraft": get(&d, "isDraft").as_bool().unwrap_or(false),
        "mergeable": mergeable_raw.as_deref(),
        "mergeability": mergeability(mergeable_raw.as_deref()),
        "mergeStateStatus": opt_str(get(&d, "mergeStateStatus")),
        "reviewDecision": opt_str(get(&d, "reviewDecision")),
        "additions": non_negative(get(&d, "additions")),
        "deletions": non_negative(get(&d, "deletions")),
        "changedFiles": non_negative(get(&d, "changedFiles")),
        "headBranch": str_at(&d, "headRefName"),
        "baseBranch": str_at(&d, "baseRefName"),
        "createdAt": str_at(&d, "createdAt"),
        "updatedAt": str_at(&d, "updatedAt"),
        "mergedAt": opt_str(get(&d, "mergedAt")),
        "closedAt": opt_str(get(&d, "closedAt")),
        "maintainerCanModify": get(&d, "maintainerCanModify").as_bool().unwrap_or(false),
        "reviewers": normalize_reviewers(&d),
        "labels": nodes(get(&d, "labels"), "nodes").into_iter().map(|l| json!({
            "name": str_at(l, "name"), "color": opt_str(get(l, "color")),
        })).collect::<Vec<_>>(),
        "checks": normalize_checks(get(&d, "statusCheckRollup")),
        "comments": comments,
        "commentsTruncated": truncated,
        "commentsIncomplete": incomplete,
        "commits": normalize_commits(get(&d, "commits")),
        "mergeCapabilities": caps,
        "stack": stack,
        "stackMetadataIncomplete": stack_incomplete,
    }))
}

// ---------------------------------------------------------------------------------------------
// diff
// ---------------------------------------------------------------------------------------------

fn is_shallow(cwd: &str) -> bool {
    git(cwd, &["rev-parse", "--is-shallow-repository"])
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false)
}

fn missing_object(stderr: &str) -> bool {
    let low = stderr.to_lowercase();
    ["bad object", "unknown revision", "not a valid object name", "no merge base", "bad revision"]
        .iter()
        .any(|p| low.contains(p))
}

fn no_merge_base(stderr: &str) -> bool {
    stderr.to_lowercase().contains("no merge base")
}

/// `resolvePullRequestFetchSource`: first configured fetch remote that parses to the
/// repository, else the canonical https URL. Matches `(fetch)` lines only.
fn remote_for_repo(cwd: &str, repo: &str) -> String {
    if let Ok(out) = git(cwd, &["remote", "-v"]) {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut parts = line.split_whitespace();
            let (Some(name), Some(url)) = (parts.next(), parts.next()) else {
                continue;
            };
            if name.starts_with('-') || !line.contains("(fetch)") {
                continue;
            }
            if repo_from_remote_url(url).is_some_and(|r| r.eq_ignore_ascii_case(repo)) {
                return name.to_string();
            }
        }
    }
    format!("https://github.com/{repo}.git")
}

fn local_diff(cwd: &str, base_sha: &str, head_sha: &str) -> Result<String, WsErr> {
    let out = git(
        cwd,
        &[
            "diff",
            "--no-color",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            &format!("{base_sha}...{head_sha}"),
        ],
    )
    .map_err(|e| err(e.0))?;
    if !out.status.success() {
        return Err(err(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn local_pr_diff(repo: &str, number: i64, cwd: &str) -> Result<String, WsErr> {
    let refs = gh(
        &[
            "api".into(),
            "--hostname".into(),
            "github.com".into(),
            format!("repos/{repo}/pulls/{number}"),
            "--jq".into(),
            "[.base.ref,.base.sha,.head.sha]|join(\" \")".into(),
        ],
        cwd,
        "Failed to load pull request diff",
    )?;
    let text = refs.stdout.trim().to_string();
    let text = text.as_str();
    let mut it = text.split_whitespace();
    let (Some(base_ref), Some(base_sha), Some(head_sha)) = (it.next(), it.next(), it.next()) else {
        return Err(err("Failed to load pull request diff"));
    };
    let remote = remote_for_repo(cwd, repo);
    match local_diff(cwd, base_sha, head_sha) {
        Ok(d) => return Ok(d),
        Err(e) if !missing_object(&e.0) => return Err(e),
        _ => {}
    }
    // Missing objects: fetch the PR head + base ref, then retry.
    let shallow = is_shallow(cwd);
    let mut fetch: Vec<String> = vec!["fetch".into(), "--quiet".into()];
    if shallow {
        fetch.push("--deepen=64".into());
    }
    fetch.push("--".into());
    fetch.push(remote.clone());
    fetch.push(format!("refs/pull/{number}/head"));
    fetch.push(base_ref.to_string());
    let args: Vec<&str> = fetch.iter().map(String::as_str).collect();
    let out = git(cwd, &args).map_err(|e| err(e.0))?;
    if !out.status.success() {
        return Err(err(format!(
            "git fetch failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    match local_diff(cwd, base_sha, head_sha) {
        Ok(d) => Ok(d),
        Err(e) if shallow && no_merge_base(&e.0) => {
            for deepen in ["256", "1024"] {
                let args = [
                    "fetch",
                    "--quiet",
                    &format!("--deepen={deepen}"),
                    "--",
                    &remote,
                    &format!("refs/pull/{number}/head"),
                    base_ref,
                ];
                let _ = git(cwd, &args);
                if let Ok(d) = local_diff(cwd, base_sha, head_sha) {
                    return Ok(d);
                }
            }
            Err(e)
        }
        Err(e) => Err(e),
    }
}

fn diff(payload: &Value) -> Result<Value, WsErr> {
    let project = find_project(&str_field(payload, "projectId")?)?;
    let repo = validate_project_repository(&project, &str_field(payload, "repository")?)?;
    let number = int_field(payload, "number")?;
    let cwd = project.workspace_root.clone();
    let args = [
        "pr".into(),
        "diff".into(),
        number.to_string(),
        "--repo".into(),
        selector(&repo),
        "--color".into(),
        "never".into(),
    ];
    let out = gh_run(&args, &cwd).map_err(gh_spawn)?;
    let patch = if out.code == 0 {
        out.stdout
    } else {
        let low = out.stderr.to_lowercase();
        if low.contains("exceeded the maximum number of files") || low.contains("too_large") {
            local_pr_diff(&repo, number, &cwd)?
        } else {
            return Err(gh_failed(&out, "Failed to load pull request diff"));
        }
    };
    let (patch, truncated) = if patch.len() > MAX_DIFF_BYTES {
        (patch[..MAX_DIFF_BYTES].to_string(), true)
    } else {
        (patch, false)
    };
    Ok(json!({ "patch": patch, "truncated": truncated }))
}

// ---------------------------------------------------------------------------------------------
// action
// ---------------------------------------------------------------------------------------------

fn merge_async(repo: &str, number: i64, method: &str, cwd: &str) -> Result<String, WsErr> {
    let endpoint = format!("repos/{repo}/pulls/{number}/merge-async");
    let args = [
        "api".into(),
        "-X".into(),
        "PUT".into(),
        endpoint.clone(),
        "-f".into(),
        format!("merge_method={method}"),
        "-f".into(),
        "merge_action=default".into(),
    ];
    let out = gh_run(&args, cwd).map_err(gh_spawn)?;
    if out.code != 0 {
        let low = out.stderr.to_lowercase();
        if out.code == 4 || low.contains("404") || low.contains("not found") {
            // Endpoint unavailable (older GHES): synchronous merge.
            let sync = gh(
                &[
                    "pr".into(),
                    "merge".into(),
                    number.to_string(),
                    "--repo".into(),
                    selector(repo),
                    format!("--{method}"),
                ],
                cwd,
                "Pull request action failed",
            )?;
            let _ = sync;
            return Ok("merged".into());
        }
        return Err(gh_failed(&out, "Pull request action failed"));
    }
    let v: Value = serde_json::from_str(&out.stdout).unwrap_or(Value::Null);
    let uuid = opt_str(get(&v, "uuid")).unwrap_or("").to_string();
    if uuid.is_empty() {
        return Ok("merged".into());
    }
    let poll_ep = format!("{endpoint}/{uuid}");
    for _ in 0..300 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let status = gh_json_v(
            &["api".into(), poll_ep.clone()],
            cwd,
            "Pull request action failed",
        )?;
        match str_at(&status, "status") {
            "merged" => return Ok("merged".into()),
            "enqueued" => return Ok("enqueued".into()),
            "failed" => {
                return Err(err(
                    opt_str(get(&status, "message"))
                        .unwrap_or("GitHub could not merge the pull request.")
                        .to_string(),
                ))
            }
            _ => continue,
        }
    }
    Err(err(
        "GitHub's asynchronous merge did not finish within five minutes.",
    ))
}

fn refresh_reopened_link(thread_id: &str, repo: &str, number: i64) {
    let canonical = format!("https://github.com/{repo}/pull/{number}");
    let Some(copy) = crate::orchestration_snapshot::open_state_copy() else {
        return;
    };
    let links_json: Option<String> = copy
        .conn
        .query_row(
            "SELECT pull_requests_json FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok();
    let Some(links_json) = links_json else { return };
    let Ok(links) = serde_json::from_str::<Value>(&links_json) else {
        return;
    };
    let Some(link) = nodes(&links, "").into_iter().find(|l| {
        get(get(l, "pullRequest"), "url")
            .as_str()
            .map(|u| u.trim_end_matches('/').eq_ignore_ascii_case(&canonical))
            .unwrap_or(false)
    }) else {
        return;
    };
    let link = link.clone();
    if !get(&link, "dismissedAt").is_null() {
        return;
    }
    let pr = get(&link, "pullRequest");
    if get(pr, "state").as_str() != Some("closed") {
        return;
    }
    let mut snapshot = pr.clone();
    if let Some(o) = snapshot.as_object_mut() {
        o.insert("state".into(), json!("open"));
    }
    let command = json!({
        "command": {
            "type": "thread.pull-request.sync",
            "commandId": format!("server:pr-reopen:{}", crate::orchestration_engine::uuid_v4()),
            "threadId": thread_id,
            "snapshots": [snapshot],
        }
    });
    let _ = crate::orchestration_engine::handle("orchestration.dispatchCommand", &command);
}

fn action(payload: &Value) -> Result<Value, WsErr> {
    let project = find_project(&str_field(payload, "projectId")?)?;
    let repo = validate_project_repository(&project, &str_field(payload, "repository")?)?;
    let number = int_field(payload, "number")?;
    let action_kind = str_field(payload, "action")?;
    let cwd = project.workspace_root.clone();
    let mut merge_outcome: Option<String> = None;
    match action_kind.as_str() {
        "merge" => {
            let method = opt_str(get(payload, "mergeMethod")).unwrap_or("merge").to_string();
            let caps = read_merge_caps(&repo, &cwd)?;
            let allowed = match method.as_str() {
                "merge" | "squash" | "rebase" => get(&caps, &method).as_bool().unwrap_or(false),
                _ => false,
            };
            if !allowed {
                return Err(err(format!(
                    "The repository does not allow the {method} merge method."
                )));
            }
            merge_outcome = Some(merge_async(&repo, number, &method, &cwd)?);
        }
        "ready" | "draft" | "close" | "reopen" => {
            let args = match action_kind.as_str() {
                "ready" => vec!["pr", "ready"],
                "draft" => vec!["pr", "ready", "--undo"],
                "close" => vec!["pr", "close"],
                _ => vec!["pr", "reopen"],
            };
            let mut argv: Vec<String> = args.into_iter().map(str::to_string).collect();
            argv.push(number.to_string());
            argv.extend(["--repo".into(), selector(&repo)]);
            gh(&argv, &cwd, "Pull request action failed")?;
            if action_kind == "reopen" {
                if let Some(thread_id) = opt_str(get(payload, "threadId")) {
                    // Persist the reopened state on the thread's existing link.
                    let links_thread = {
                        let Some(copy) = crate::orchestration_snapshot::open_state_copy()
                        else {
                            return Err(err("thread lookup failed"));
                        };
                        copy.conn
                            .query_row(
                                "SELECT project_id FROM projection_threads WHERE thread_id = ?1",
                                params![thread_id],
                                |r| r.get::<_, String>(0),
                            )
                            .ok()
                    };
                    if links_thread.as_deref() == Some(project.id.as_str()) {
                        refresh_reopened_link(thread_id, &repo, number);
                    }
                }
            }
        }
        other => return Err(err(format!("unsupported pull request action {other}"))),
    }
    Ok(json!({
        "projectId": project.id,
        "repository": repo,
        "number": number,
        "workspaceRoot": project.workspace_root,
        "mergeOutcome": merge_outcome,
    }))
}

fn comment(payload: &Value) -> Result<Value, WsErr> {
    let project = find_project(&str_field(payload, "projectId")?)?;
    let repo = validate_project_repository(&project, &str_field(payload, "repository")?)?;
    let number = int_field(payload, "number")?;
    let body = str_field(payload, "body")?;
    if body.chars().count() > 65536 {
        return Err(err("body exceeds GitHub's 65536 character limit"));
    }
    let args = [
        "pr".into(),
        "comment".into(),
        number.to_string(),
        "--repo".into(),
        selector(&repo),
        "--body-file".into(),
        "-".into(),
    ];
    let out = gh_stdin(&args, &body, &project.workspace_root).map_err(gh_spawn)?;
    if out.code != 0 {
        return Err(gh_failed(&out, "Could not post the comment"));
    }
    Ok(json!({
        "projectId": project.id,
        "repository": repo,
        "number": number,
    }))
}

// ---------------------------------------------------------------------------------------------
// pins
// ---------------------------------------------------------------------------------------------

fn set_pinned(payload: &Value) -> Result<Value, WsErr> {
    let project = find_project(&str_field(payload, "projectId")?)?;
    let number = int_field(payload, "number")?;
    let is_pinned = get(payload, "isPinned").as_bool().unwrap_or(false);
    let input_repo = str_field(payload, "repository")?;
    let repo = if is_pinned {
        validate_project_repository(&project, &input_repo)?
    } else {
        validate_repository(&input_repo)?
    };
    if std::env::var("SYNARA_NO_UPSTREAM").is_err() {
        return Err(err("pullRequests.setPinned requires SYNARA_NO_UPSTREAM"));
    }
    let conn = crate::orchestration_engine::open_state_rw()?;
    let key = repo.to_lowercase();
    if is_pinned {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_pull_request_pins WHERE project_id = ?1",
                params![project.id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_pull_request_pins \
                 WHERE project_id = ?1 AND repository_key = ?2 AND pull_request_number = ?3",
                params![project.id, key, number],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if exists == 0 && count >= MAX_PINS {
            return Err(err("A project can pin at most 20 pull requests."));
        }
        conn.execute(
            "INSERT INTO project_pull_request_pins \
             (project_id, repository_key, pull_request_number) VALUES (?1, ?2, ?3) \
             ON CONFLICT DO NOTHING",
            params![project.id, key, number],
        )
        .map_err(|e| err(format!("{e}")))?;
    } else {
        conn.execute(
            "DELETE FROM project_pull_request_pins \
             WHERE project_id = ?1 AND repository_key = ?2 AND pull_request_number = ?3",
            params![project.id, key, number],
        )
        .map_err(|e| err(format!("{e}")))?;
    }
    Ok(json!({
        "projectId": project.id,
        "repository": repo,
        "number": number,
        "isPinned": is_pinned,
    }))
}

// ---------------------------------------------------------------------------------------------
// Auto-fix CI
// ---------------------------------------------------------------------------------------------

fn auto_fix_state(row: (String, String, Option<String>, String, Option<String>, i64, Option<String>, String)) -> Value {
    let mut o = Map::new();
    o.insert("threadId".into(), json!(row.0));
    o.insert("pullRequestUrl".into(), json!(row.1));
    if let Some(r) = row.2 {
        o.insert("requestedPullRequestUrl".into(), json!(r));
    }
    o.insert("status".into(), json!(row.3));
    o.insert("pauseReason".into(), row.4.map(Value::String).unwrap_or(Value::Null));
    o.insert("attempts".into(), json!(row.5));
    o.insert(
        "lastHandledHeadSha".into(),
        row.6.map(Value::String).unwrap_or(Value::Null),
    );
    o.insert("updatedAt".into(), json!(row.7));
    Value::Object(o)
}

fn get_auto_fix(payload: &Value) -> Result<Value, WsErr> {
    let thread_id = str_field(payload, "threadId")?;
    let Some(copy) = crate::orchestration_snapshot::open_state_copy() else {
        return Ok(json!({ "states": [] }));
    };
    let Ok(mut stmt) = copy.conn.prepare(
        "SELECT thread_id, pull_request_url, requested_pull_request_url, status, \
         pause_reason, attempts, last_handled_head_sha, updated_at \
         FROM pull_request_auto_fix WHERE thread_id = ?1 ORDER BY updated_at",
    ) else {
        return Ok(json!({ "states": [] }));
    };
    let states: Vec<Value> = stmt
        .query_map(params![thread_id], |r| {
            Ok(auto_fix_state((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, String>(7)?,
            )))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default();
    Ok(json!({ "states": states }))
}

/// `resolveThreadWorkspaceCwd`: worktree mode → worktreePath; else workingDirectory ?? projectCwd.
fn thread_workspace_cwd(
    project_root: &str,
    project_kind: &str,
    env_mode: &str,
    worktree_path: Option<&str>,
    working_dir: Option<&str>,
) -> Option<String> {
    if env_mode == "worktree" {
        let wt = worktree_path?;
        if let Some(dir) = working_dir {
            if dir.starts_with(wt) {
                return Some(dir.to_string());
            }
        }
        return Some(wt.to_string());
    }
    if let Some(dir) = working_dir {
        return Some(dir.to_string());
    }
    // `chat` projects have no durable cwd until a worktree exists.
    if project_kind == "chat" && worktree_path.is_none() {
        return None;
    }
    if project_root.is_empty() {
        None
    } else {
        Some(project_root.to_string())
    }
}

fn set_auto_fix(payload: &Value) -> Result<Value, WsErr> {
    let thread_id = str_field(payload, "threadId")?;
    let enabled = get(payload, "enabled").as_bool().unwrap_or(false);
    let canonical = opt_str(get(payload, "pullRequestUrl"))
        .and_then(normalize_pr_url)
        .ok_or_else(|| err("A valid GitHub pull request URL is required."))?;
    if std::env::var("SYNARA_NO_UPSTREAM").is_err() {
        return Err(err("pullRequests.setAutoFix requires SYNARA_NO_UPSTREAM"));
    }
    let conn = crate::orchestration_engine::open_state_rw()?;
    if !enabled {
        conn.execute(
            "DELETE FROM pull_request_auto_fix \
             WHERE pull_request_url = ?1 OR requested_pull_request_url = ?1",
            params![canonical],
        )
        .map_err(|e| err(format!("{e}")))?;
        return Ok(json!({ "state": Value::Null }));
    }
    let thread = conn
        .query_row(
            "SELECT t.project_id, t.env_mode, t.worktree_path, t.working_directory, \
             p.workspace_root, p.kind \
             FROM projection_threads t \
             LEFT JOIN projection_projects p ON p.project_id = t.project_id \
             WHERE t.thread_id = ?1 AND t.archived_at IS NULL AND t.deleted_at IS NULL",
            params![thread_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1).unwrap_or_default(),
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4).unwrap_or_default(),
                    r.get::<_, String>(5).unwrap_or_default(),
                ))
            },
        )
        .ok()
        .ok_or_else(|| err("This thread is no longer available."))?;
    let cwd = thread_workspace_cwd(
        &thread.4,
        &thread.5,
        &thread.1,
        thread.2.as_deref(),
        thread.3.as_deref(),
    )
    .ok_or_else(|| err("This thread has no workspace to fix the pull request in."))?;
    let summary = gh_json_v(
        &[
            "pr".into(),
            "view".into(),
            canonical.clone(),
            "--json".into(),
            format!("{SUMMARY_FIELDS},headRefOid,statusCheckRollup"),
        ],
        &cwd,
        "Failed to update Auto-fix CI",
    )?;
    if pr_state(&summary) != "open" {
        return Err(err("Only open pull requests can be auto-fixed."));
    }
    let canonical_url = opt_str(get(&summary, "url"))
        .and_then(normalize_pr_url)
        .ok_or_else(|| err("GitHub returned an invalid pull request URL."))?;
    let conflict: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pull_request_auto_fix \
             WHERE pull_request_url = ?1 AND thread_id != ?2",
            params![canonical_url, thread_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if conflict > 0 {
        return Err(err(
            "Auto-fix CI is already enabled for this pull request in another chat. \
             Turn it off there first.",
        ));
    }
    conn.execute(
        "INSERT INTO pull_request_auto_fix \
         (thread_id, pull_request_url, requested_pull_request_url, status, pause_reason, \
          attempts, last_handled_head_sha, updated_at) \
         VALUES (?1, ?2, ?3, 'watching', NULL, 0, NULL, ?4) \
         ON CONFLICT(thread_id, pull_request_url) DO UPDATE SET \
         status = 'watching', pause_reason = NULL, attempts = 0, \
         requested_pull_request_url = ?3, updated_at = ?4",
        params![
            thread_id,
            canonical_url,
            canonical,
            chrono_now(),
        ],
    )
        .map_err(|e| err(format!("{e}")))?;
    Ok(json!({
        "state": {
            "threadId": thread_id,
            "pullRequestUrl": canonical_url,
            "requestedPullRequestUrl": canonical,
            "status": "watching",
            "pauseReason": Value::Null,
            "attempts": 0,
            "lastHandledHeadSha": Value::Null,
            "updatedAt": chrono_now(),
        }
    }))
}

fn chrono_now() -> String {
    let out = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok();
    out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}
