//! `githubInbox.*` native handlers: repository inbox list, issue detail and issue comment,
//! backed by the `gh` CLI. Repository inventory is resolved from the projection DB plus each
//! project's git remotes; pins come from `project_pull_request_pins`.

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use crate::orchestration_snapshot::{now_iso, open_state_copy, query_objects};
use crate::todo::WsErr;

const PAGE_SIZE: usize = 50;
const GITHUB_HOST: &str = "github.com";

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("GITHUB_INBOX"))
}

pub fn handle(tag: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    match tag {
        "githubInbox.list" => list(payload).map(Some),
        "githubInbox.issueDetail" => issue_detail(payload).map(Some),
        "githubInbox.issueComment" => issue_comment(payload).map(Some),
        _ => Ok(None),
    }
}

fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

fn str_at<'a>(v: &'a Value, k: &str) -> &'a str {
    get(v, k).as_str().unwrap_or("")
}

fn opt_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

fn non_negative(v: &Value) -> i64 {
    v.as_i64().filter(|n| *n >= 0).unwrap_or(0)
}

fn nodes(v: &Value, k: &str) -> Vec<Value> {
    get(v, k)
        .get("nodes")
        .and_then(|n| n.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|n| !n.is_null())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// gh process boundary
// ---------------------------------------------------------------------------------------------

pub(crate) struct GhOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub(crate) fn gh_run(args: &[String], cwd: &str) -> Result<GhOutput, String> {
    let out = Command::new("gh")
        .args(args)
        .current_dir(if cwd.is_empty() { "." } else { cwd })
        .output()
        .map_err(|e| format!("gh CLI is not installed or failed to launch ({e})"))?;
    Ok(GhOutput {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

/// Port of `summarizeGitHubInboxError`: first meaningful line of the failure, `gh:` stripped.
pub(crate) fn summarize_error(stderr: &str) -> String {
    let text = stderr.trim().trim_start_matches("gh:").trim();
    for line in text.lines() {
        let l = line.trim();
        if !l.is_empty() {
            return if l.chars().count() > 300 {
                format!("{}...", l.chars().take(297).collect::<String>())
            } else {
                l.to_string()
            };
        }
    }
    "GitHub request failed.".to_string()
}

pub(crate) fn gh_json(args: &[String], cwd: &str) -> Result<Value, String> {
    let out = gh_run(args, cwd)?;
    if out.code != 0 {
        return Err(summarize_error(&out.stderr));
    }
    serde_json::from_str(&out.stdout).map_err(|e| format!("gh returned invalid JSON ({e})"))
}

pub(crate) fn graphql_host(
    host: &str,
    query: &str,
    vars: &[String],
    cwd: &str,
) -> Result<Value, String> {
    let mut args: Vec<String> = vec![
        "api".into(),
        "graphql".into(),
        "--hostname".into(),
        host.into(),
        "-f".into(),
        format!("query={query}"),
    ];
    args.extend(vars.iter().cloned());
    gh_json(&args, cwd)
}

fn graphql(query: &str, vars: &[String], cwd: &str) -> Result<Value, String> {
    let mut args: Vec<String> = vec![
        "api".into(),
        "graphql".into(),
        "--hostname".into(),
        GITHUB_HOST.into(),
        "-f".into(),
        format!("query={query}"),
    ];
    args.extend(vars.iter().cloned());
    gh_json(&args, cwd)
}

// ---------------------------------------------------------------------------------------------
// GraphQL documents (same selections as the upstream inbox read)
// ---------------------------------------------------------------------------------------------

fn inbox_fragments() -> String {
    "fragment InboxActorFields on Actor { __typename login avatarUrl url ... on User { name } }\n\
     fragment InboxPullRequestFields on PullRequest {\n  __typename number title url state isDraft additions deletions createdAt updatedAt closedAt mergedAt\n  \
     headRefName baseRefName reviewDecision mergeable\n  author { ...InboxActorFields }\n  \
     reviewRequests(first: 20) { nodes { requestedReviewer { __typename ... on User { login } ... on Team { slug } } } }\n  \
     labels(first: 20) { nodes { name color } }\n  assignees(first: 10) { nodes { login avatarUrl url name } }\n  \
     comments { totalCount }\n  stackEntry { position }\n  stack { number size baseRefName }\n}\n\
     fragment InboxIssueFields on Issue {\n  __typename number title url state stateReason createdAt updatedAt closedAt\n  \
     author { ...InboxActorFields }\n  labels(first: 20) { nodes { name color } }\n  \
     assignees(first: 10) { nodes { login avatarUrl url name } }\n  comments { totalCount }\n}"
        .to_string()
}

fn inbox_query(sort: &str) -> String {
    let order = if sort == "created" { "CREATED_AT" } else { "UPDATED_AT" };
    format!(
        "query($owner: String!, $name: String!, $prStates: [PullRequestState!], $issueStates: [IssueState!], $reviewQuery: String!, $includeReview: Boolean!) {{\n  \
         viewer {{ login }}\n  rateLimit {{ cost remaining resetAt }}\n  \
         repository(owner: $owner, name: $name) {{\n    \
         pullRequests(states: $prStates, first: {PAGE_SIZE}, orderBy: {{field: {order}, direction: DESC}}) {{ totalCount nodes {{ ...InboxPullRequestFields }} }}\n    \
         issues(states: $issueStates, first: {PAGE_SIZE}, orderBy: {{field: {order}, direction: DESC}}) {{ totalCount nodes {{ ...InboxIssueFields }} }}\n  }}\n  \
         reviewRequested: search(query: $reviewQuery, type: ISSUE, first: {PAGE_SIZE}) @include(if: $includeReview) {{ issueCount nodes {{ ... on PullRequest {{ number }} }} }}\n}}\n{}",
        inbox_fragments()
    )
}

fn involvement_query() -> String {
    format!(
        "query($mineQuery: String!) {{\n  rateLimit {{ cost remaining resetAt }}\n  \
         mine: search(query: $mineQuery, type: ISSUE, first: {PAGE_SIZE}) {{\n    \
         issueCount nodes {{ __typename ...InboxPullRequestFields ...InboxIssueFields }}\n  }}\n}}\n{}",
        inbox_fragments()
    )
}

fn inbox_vars(repo: &str, state: &str) -> Vec<String> {
    let (owner, name) = repo.split_once('/').unwrap_or(("", ""));
    let mut v = vec!["-f".into(), format!("owner={owner}"), "-f".into(), format!("name={name}")];
    if state == "open" {
        v.push("-f".into());
        v.push("prStates[]=OPEN".into());
        v.push("-f".into());
        v.push("issueStates[]=OPEN".into());
    } else {
        for s in ["CLOSED", "MERGED"] {
            v.push("-f".into());
            v.push(format!("prStates[]={s}"));
        }
        v.push("-f".into());
        v.push("issueStates[]=CLOSED".into());
    }
    v.push("-f".into());
    v.push(format!("reviewQuery=repo:{repo} is:pr is:open review-requested:@me"));
    v.push("-F".into());
    v.push(format!("includeReview={}", state == "open"));
    v
}

// ---------------------------------------------------------------------------------------------
// Project inventory + pins
// ---------------------------------------------------------------------------------------------

struct Project {
    id: String,
    title: String,
    workspace_root: String,
}

fn live_projects() -> Vec<Project> {
    let Some(copy) = open_state_copy() else {
        return Vec::new();
    };
    query_objects(
        &copy.conn,
        "SELECT project_id, title, workspace_root, kind, deleted_at FROM projection_projects \
         ORDER BY created_at, project_id",
        &[],
        &[],
        &[],
    )
    .into_iter()
    .filter(|p| get(p, "deletedAt").is_null() && str_at(p, "kind") == "project")
    .map(|p| Project {
        id: str_at(&p, "projectId").to_string(),
        title: str_at(&p, "title").to_string(),
        workspace_root: str_at(&p, "workspaceRoot").to_string(),
    })
    .filter(|p| !p.id.is_empty() && !p.workspace_root.is_empty())
    .collect()
}

fn pins() -> HashSet<(String, String, i64)> {
    let mut set = HashSet::new();
    let Some(copy) = open_state_copy() else {
        return set;
    };
    let Ok(mut stmt) = copy.conn.prepare(
        "SELECT project_id, repository_key, pull_request_number FROM project_pull_request_pins",
    ) else {
        return set;
    };
    if let Ok(rows) = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    }) {
        for row in rows.flatten() {
            set.insert(row);
        }
    }
    set
}

fn project_by_id(id: &str) -> Option<Project> {
    live_projects().into_iter().find(|p| p.id == id)
}

fn validate_project_repository(project: &Project, input: &str) -> Result<String, WsErr> {
    let repository = crate::pull_requests::validate_repository(input)?;
    let inventory = crate::pull_requests::resolve_repositories(&project.workspace_root)?;
    inventory
        .into_iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(&repository))
        .ok_or_else(|| err("GitHub repository does not belong to the selected project."))
}

// ---------------------------------------------------------------------------------------------
// Node normalization
// ---------------------------------------------------------------------------------------------

fn actor(node: &Value) -> Value {
    let login = match opt_str(get(node, "login")) {
        Some(l) => l,
        None => return Value::Null,
    };
    json!({
        "login": login,
        "name": opt_str(get(node, "name")),
        "avatarUrl": opt_str(get(node, "avatarUrl")),
        "url": opt_str(get(node, "url")),
    })
}

fn inbox_actors(node: &Value, k: &str) -> Vec<Value> {
    nodes(node, k)
        .into_iter()
        .map(|a| actor(&a))
        .filter(|a| !a.is_null())
        .collect()
}

fn labels(node: &Value) -> Vec<Value> {
    nodes(node, "labels")
        .into_iter()
        .filter_map(|l| {
            opt_str(get(&l, "name")).map(|name| json!({"name": name, "color": opt_str(get(&l, "color"))}))
        })
        .collect()
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

fn issue_state(node: &Value) -> &'static str {
    if get(node, "state").as_str() == Some("CLOSED") {
        "closed"
    } else {
        "open"
    }
}

fn issue_state_reason(node: &Value, state: &str) -> Value {
    if state == "open" {
        return Value::Null;
    }
    match get(node, "stateReason").as_str() {
        Some("COMPLETED") => json!("completed"),
        Some("NOT_PLANNED") => json!("not-planned"),
        Some("DUPLICATE") => json!("duplicate"),
        _ => Value::Null,
    }
}

fn mergeability(node: &Value) -> &'static str {
    match get(node, "mergeable").as_str() {
        Some("MERGEABLE") => "mergeable",
        Some("CONFLICTING") => "conflicting",
        _ => "unknown",
    }
}

fn review_request_logins(node: &Value) -> Vec<String> {
    nodes(node, "reviewRequests")
        .into_iter()
        .filter(|r| get(r, "requestedReviewer").get("__typename").and_then(Value::as_str) != Some("Team"))
        .filter_map(|r| get(&r, "requestedReviewer").get("login").and_then(|l| opt_str(l)))
        .collect()
}

fn stack_of(node: &Value) -> Value {
    let stack = get(node, "stack");
    let entry = get(node, "stackEntry");
    if stack.is_null() || entry.is_null() {
        return Value::Null;
    }
    let (size, position) = (non_negative(get(stack, "size")), non_negative(get(entry, "position")));
    if position == 0 || position > size {
        return Value::Null;
    }
    json!({
        "number": non_negative(get(stack, "number")),
        "size": size,
        "position": position,
        "baseBranch": str_at(stack, "baseRefName"),
    })
}

fn inbox_pr(node: &Value) -> Value {
    json!({
        "number": non_negative(get(node, "number")),
        "title": str_at(node, "title"),
        "url": str_at(node, "url"),
        "author": actor(get(node, "author")),
        "headBranch": str_at(node, "headRefName"),
        "baseBranch": str_at(node, "baseRefName"),
        "state": pr_state(node),
        "isDraft": get(node, "isDraft").as_bool().unwrap_or(false),
        "additions": non_negative(get(node, "additions")),
        "deletions": non_negative(get(node, "deletions")),
        "createdAt": str_at(node, "createdAt"),
        "updatedAt": str_at(node, "updatedAt"),
        "reviewDecision": opt_str(get(node, "reviewDecision")),
        "labels": labels(node),
        "mergeability": mergeability(node),
        "stack": stack_of(node),
        "commentCount": non_negative(get(get(node, "comments"), "totalCount")),
        "assignees": inbox_actors(node, "assignees"),
    })
}

fn inbox_issue(node: &Value) -> Value {
    let state = issue_state(node);
    json!({
        "number": non_negative(get(node, "number")),
        "title": str_at(node, "title"),
        "url": str_at(node, "url"),
        "author": actor(get(node, "author")),
        "state": state,
        "stateReason": issue_state_reason(node, state),
        "labels": labels(node),
        "assignees": inbox_actors(node, "assignees"),
        "commentCount": non_negative(get(get(node, "comments"), "totalCount")),
        "createdAt": str_at(node, "createdAt"),
        "updatedAt": str_at(node, "updatedAt"),
        "closedAt": opt_str(get(node, "closedAt")),
    })
}

fn involvement(author: &Value, assignees: &[Value], viewer: &str, matched: bool) -> Value {
    let v = viewer.trim().to_lowercase();
    let same = |a: &Value| !v.is_empty() && str_at(a, "login").to_lowercase() == v;
    let authored = !author.is_null() && same(author);
    let assigned = assignees.iter().any(same);
    json!({
        "authored": authored,
        "assigned": assigned,
        "involved": authored || assigned || matched,
    })
}

fn viewer_review_requested(
    author: &Value,
    logins: &[String],
    viewer: &str,
    matched_search: bool,
) -> bool {
    let v = viewer.trim().to_lowercase();
    let distinct = author.is_null() || str_at(author, "login").to_lowercase() != v;
    distinct && (matched_search || logins.iter().any(|l| l.trim().to_lowercase() == v))
}

fn contexts(projects: &[&Project], repo: &str, number: i64, pins: &HashSet<(String, String, i64)>) -> Vec<Value> {
    projects
        .iter()
        .map(|p| {
            json!({
                "projectId": p.id,
                "projectTitle": p.title,
                "isPinned": pins.contains(&(p.id.clone(), repo.to_lowercase(), number)),
            })
        })
        .collect()
}

fn pr_entry(
    node: &Value,
    repo: &str,
    projects: &[&Project],
    viewer: &str,
    review_requested: &HashSet<i64>,
    involved: &HashSet<i64>,
    pins: &HashSet<(String, String, i64)>,
) -> Value {
    let pr = inbox_pr(node);
    let number = non_negative(get(&pr, "number"));
    let ctxs = contexts(projects, repo, number, pins);
    let first = projects[0];
    let assignees: Vec<Value> = get(&pr, "assignees").as_array().cloned().unwrap_or_default();
    json!({
        "kind": "pullRequest",
        "projectId": first.id,
        "projectTitle": first.title,
        "projectContexts": ctxs,
        "repository": repo,
        "number": number,
        "title": get(&pr, "title"),
        "url": get(&pr, "url"),
        "author": get(&pr, "author"),
        "headBranch": get(&pr, "headBranch"),
        "baseBranch": get(&pr, "baseBranch"),
        "state": get(&pr, "state"),
        "isDraft": get(&pr, "isDraft"),
        "additions": get(&pr, "additions"),
        "deletions": get(&pr, "deletions"),
        "createdAt": get(&pr, "createdAt"),
        "updatedAt": get(&pr, "updatedAt"),
        "reviewDecision": get(&pr, "reviewDecision"),
        "viewerReviewRequested": viewer_review_requested(
            get(&pr, "author"),
            &review_request_logins(node),
            viewer,
            review_requested.contains(&number),
        ),
        "isPinned": ctxs.iter().any(|c| get(c, "isPinned").as_bool().unwrap_or(false)),
        "mergeability": get(&pr, "mergeability"),
        "stack": get(&pr, "stack"),
        "labels": get(&pr, "labels"),
        "commentCount": get(&pr, "commentCount"),
        "assignees": get(&pr, "assignees"),
        "viewerInvolvement": involvement(get(&pr, "author"), &assignees, viewer, involved.contains(&number)),
    })
}

fn issue_entry(
    node: &Value,
    repo: &str,
    projects: &[&Project],
    viewer: &str,
    involved: &HashSet<i64>,
    pins: &HashSet<(String, String, i64)>,
) -> Value {
    let issue = inbox_issue(node);
    let number = non_negative(get(&issue, "number"));
    let ctxs = contexts(projects, repo, number, pins);
    let first = projects[0];
    let assignees: Vec<Value> = get(&issue, "assignees").as_array().cloned().unwrap_or_default();
    let mut e = Map::new();
    e.insert("kind".into(), json!("issue"));
    e.insert("projectId".into(), json!(first.id));
    e.insert("projectTitle".into(), json!(first.title));
    e.insert("projectContexts".into(), Value::Array(ctxs.clone()));
    e.insert("repository".into(), json!(repo));
    for k in ["number", "title", "url", "author", "state", "stateReason", "labels", "assignees", "commentCount", "createdAt", "updatedAt", "closedAt"] {
        e.insert(k.to_string(), get(&issue, k).clone());
    }
    e.insert(
        "isPinned".into(),
        json!(ctxs.iter().any(|c| get(c, "isPinned").as_bool().unwrap_or(false))),
    );
    e.insert(
        "viewerInvolvement".into(),
        involvement(get(&issue, "author"), &assignees, viewer, involved.contains(&number)),
    );
    Value::Object(e)
}

fn list_error(
    project: &Project,
    repository: Value,
    message: String,
    reason: &str,
    retry_at: Value,
) -> Value {
    json!({
        "projectId": project.id,
        "projectTitle": project.title,
        "repository": repository,
        "message": message,
        "reason": reason,
        "retryAt": retry_at,
        "showingCachedData": false,
    })
}

fn rate_limited(message: &str) -> bool {
    message.to_lowercase().contains("rate limit")
}

// ---------------------------------------------------------------------------------------------
// githubInbox.list
// ---------------------------------------------------------------------------------------------

fn list(payload: &Value) -> Result<Value, WsErr> {
    let state = str_at(payload, "state");
    let state = if state == "closed" { "closed" } else { "open" };
    let sort = str_at(payload, "sort");
    let sort = if sort == "created" { "created" } else { "updated" };

    let projects = live_projects();
    let pin_set = pins();
    let mut repo_projects: BTreeMap<String, Vec<&Project>> = BTreeMap::new();
    let mut errors: Vec<Value> = Vec::new();

    for p in &projects {
        match crate::pull_requests::resolve_repositories(&p.workspace_root) {
            Ok(repos) => {
                // Upstream's inbox defaults to the preferred remote only; the full inventory
                // remains authoritative for access checks and pin ownership.
                if let Some(repo) = repos.first() {
                    repo_projects.entry(repo.clone()).or_default().push(p);
                } else {
                    errors.push(list_error(
                        p,
                        Value::Null,
                        "No GitHub repository could be resolved for this project.".into(),
                        "unavailable",
                        Value::Null,
                    ));
                }
            }
            Err(e) => errors.push(list_error(
                p,
                Value::Null,
                e.0,
                "unavailable",
                Value::Null,
            )),
        }
    }

    let mut items: Vec<Value> = Vec::new();
    let mut batches: Vec<Value> = Vec::new();
    let mut viewer: Option<String> = None;
    let mut rate_limit: Option<Value> = None;
    let mut review_requested_count: i64 = 0;
    let mut review_incomplete = false;

    for (repo, owners) in &repo_projects {
        let cwd = &owners[0].workspace_root;
        let resp = graphql(&inbox_query(sort), &inbox_vars(repo, state), cwd);
        let inv_resp = graphql(
            &involvement_query(),
            &["-f".into(), format!("mineQuery=repo:{repo} is:{state} involves:@me sort:{sort}-desc")],
            cwd,
        );
        let resp = match (resp, inv_resp) {
            (Ok(mut r), Ok(i)) => {
                if let Some(d) = r.get_mut("data").and_then(Value::as_object_mut) {
                    d.insert("mine".into(), i.pointer("/data/mine").cloned().unwrap_or(Value::Null));
                    d.insert(
                        "mineRateLimit".into(),
                        i.pointer("/data/rateLimit").cloned().unwrap_or(Value::Null),
                    );
                }
                r
            }
            (Err(e), _) | (_, Err(e)) => {
                let (reason, retry_at) = if rate_limited(&e) {
                    ("rate-limited", Value::Null)
                } else {
                    ("unavailable", Value::Null)
                };
                for p in owners {
                    errors.push(list_error(p, json!(repo), e.clone(), reason, retry_at.clone()));
                }
                if state == "open" {
                    review_incomplete = true;
                }
                continue;
            }
        };

        let data = get(&resp, "data");
        if viewer.is_none() {
            viewer = opt_str(get(data, "viewer").get("login").unwrap_or(&Value::Null));
        }
        for key in ["rateLimit", "mineRateLimit"] {
            if let Some(rl) = data.get(key).and_then(|r| {
                let remaining = non_negative(get(r, "remaining"));
                opt_str(get(r, "resetAt")).map(|reset| json!({"remaining": remaining, "resetAt": reset}))
            }) {
                rate_limit = Some(rl);
            }
        }
        let repository = get(data, "repository");
        if repository.is_null() {
            for p in owners {
                errors.push(list_error(
                    p,
                    json!(repo),
                    "GitHub repository lookup failed.".into(),
                    "unavailable",
                    Value::Null,
                ));
            }
            if state == "open" {
                review_incomplete = true;
            }
            continue;
        }

        let pr_total = non_negative(get(repository, "pullRequests").get("totalCount").unwrap_or(&Value::Null));
        let issue_total = non_negative(get(repository, "issues").get("totalCount").unwrap_or(&Value::Null));
        batches.push(json!({
            "repository": repo,
            "projectIds": owners.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
            "truncatedPullRequests": pr_total > PAGE_SIZE as i64,
            "truncatedIssues": issue_total > PAGE_SIZE as i64,
            "totalPullRequests": pr_total,
            "totalIssues": issue_total,
            "fetchedAt": now_iso(),
        }));

        let mut involved: HashSet<i64> = HashSet::new();
        for n in nodes(data, "mine") {
            let num = non_negative(get(&n, "number"));
            if num > 0 {
                involved.insert(num);
            }
        }
        let mut review_requested: HashSet<i64> = HashSet::new();
        if state == "open" {
            let rr = get(data, "reviewRequested");
            review_requested_count += non_negative(get(rr, "issueCount"));
            for n in nodes(data, "reviewRequested") {
                let num = non_negative(get(&n, "number"));
                if num > 0 {
                    review_requested.insert(num);
                }
            }
        }
        let v = viewer.clone().unwrap_or_default();
        for node in nodes(repository, "pullRequests") {
            if get(&node, "__typename").as_str() != Some("PullRequest") {
                continue;
            }
            items.push(pr_entry(&node, repo, owners, &v, &review_requested, &involved, &pin_set));
        }
        for node in nodes(repository, "issues") {
            if get(&node, "__typename").as_str() != Some("Issue") {
                continue;
            }
            items.push(issue_entry(&node, repo, owners, &v, &involved, &pin_set));
        }
    }

    let sort_key = if sort == "created" { "createdAt" } else { "updatedAt" };
    items.sort_by(|a, b| str_at(b, sort_key).cmp(str_at(a, sort_key)));

    Ok(json!({
        "viewer": viewer,
        "items": items,
        "errors": errors,
        "repositoryBatches": batches,
        "rateLimit": rate_limit.unwrap_or(Value::Null),
        "reviewRequestedCount": if state == "open" { review_requested_count } else { 0 },
        "reviewRequestedCountIncomplete": review_incomplete,
    }))
}

// ---------------------------------------------------------------------------------------------
// githubInbox.issueDetail / issueComment
// ---------------------------------------------------------------------------------------------

fn detail_error(msg: impl Into<String>) -> WsErr {
    err(msg)
}

fn issue_detail_query() -> &'static str {
    "query($owner: String!, $name: String!, $number: Int!) {\n  repository(owner: $owner, name: $name) {\n    issue(number: $number) {\n      number title url body state stateReason createdAt updatedAt closedAt\n      author { login avatarUrl url ... on User { name } }\n      assignees(first: 100) { nodes { login avatarUrl url name } }\n      labels(first: 100) { nodes { name color } }\n      comments(first: 100) {\n        totalCount pageInfo { hasNextPage }\n        nodes { id body url createdAt author { login avatarUrl url ... on User { name } } }\n      }\n    }\n  }\n}"
}

fn issue_detail(payload: &Value) -> Result<Value, WsErr> {
    let project_id = str_at(payload, "projectId");
    let number = get(payload, "number").as_i64().unwrap_or(0);
    if number <= 0 {
        return Err(detail_error("number must be a positive integer"));
    }
    let Some(project) = project_by_id(project_id) else {
        return Err(detail_error(format!("Project '{project_id}' was not found.")));
    };
    let repo = validate_project_repository(&project, str_at(payload, "repository"))?;
    let (owner, name) = repo.split_once('/').unwrap_or(("", ""));
    let response = graphql(
        issue_detail_query(),
        &[
            "-F".into(),
            format!("owner={owner}"),
            "-F".into(),
            format!("name={name}"),
            "-F".into(),
            format!("number={number}"),
        ],
        &project.workspace_root,
    )
    .map_err(detail_error)?;
    let issue = response
        .pointer("/data/repository/issue")
        .cloned()
        .unwrap_or(Value::Null);
    if issue.is_null() {
        return Err(detail_error("GitHub issue lookup failed."));
    }

    let state = issue_state(&issue);
    let comments_connection = get(&issue, "comments");
    let mut comments = Vec::new();
    for c in nodes(&issue, "comments") {
        comments.push(json!({
            "id": str_at(&c, "id"),
            "kind": "issue-comment",
            "author": actor(get(&c, "author")),
            "body": get(&c, "body"),
            "createdAt": get(&c, "createdAt"),
            "updatedAt": get(&c, "updatedAt"),
            "url": get(&c, "url"),
            "path": Value::Null,
            "reviewState": Value::Null,
        }));
    }
    comments.sort_by(|a, b| str_at(a, "createdAt").cmp(str_at(b, "createdAt")));
    let comment_count = non_negative(get(comments_connection, "totalCount"));
    let has_next_page = get(get(comments_connection, "pageInfo"), "hasNextPage")
        .as_bool()
        .unwrap_or(false);
    Ok(json!({
        "projectId": project.id,
        "projectTitle": project.title,
        "workspaceRoot": project.workspace_root,
        "repository": repo,
        "number": number,
        "title": get(&issue, "title"),
        "url": get(&issue, "url"),
        "author": actor(get(&issue, "author")),
        "state": state,
        "stateReason": issue_state_reason(&issue, state),
        "labels": labels(&issue),
        "assignees": inbox_actors(&issue, "assignees"),
        "commentCount": comment_count,
        "createdAt": get(&issue, "createdAt"),
        "updatedAt": get(&issue, "updatedAt"),
        "closedAt": get(&issue, "closedAt"),
        "body": get(&issue, "body").as_str().unwrap_or(""),
        "comments": comments,
        "commentsTruncated": has_next_page || (comments.len() as i64) < comment_count,
    }))
}

fn issue_comment(payload: &Value) -> Result<Value, WsErr> {
    let project_id = str_at(payload, "projectId");
    let number = get(payload, "number").as_i64().unwrap_or(0);
    if number <= 0 {
        return Err(err("number must be a positive integer"));
    }
    let body = str_at(payload, "body").trim();
    if body.is_empty() || body.chars().count() > 65_536 {
        return Err(err("body must be non-empty and at most 65536 characters"));
    }
    let Some(project) = project_by_id(project_id) else {
        return Err(err(format!("Project '{project_id}' was not found.")));
    };
    let repo = validate_project_repository(&project, str_at(payload, "repository"))?;
    let out = crate::pull_requests::gh_stdin(
        &[
            "issue".into(),
            "comment".into(),
            number.to_string(),
            "--repo".into(),
            format!("github.com/{repo}"),
            "--body-file".into(),
            "-".into(),
        ],
        body,
        &project.workspace_root,
    )
    .map_err(err)?;
    if out.code != 0 {
        return Err(err(summarize_error(&out.stderr)));
    }
    Ok(json!({
        "projectId": project.id,
        "repository": repo,
        "number": number,
        "workspaceRoot": project.workspace_root,
    }))
}

#[allow(dead_code)]
fn _unused() -> PathBuf {
    PathBuf::new()
}
