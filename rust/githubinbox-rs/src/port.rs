use serde_json::{Map, Value};

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    obj.get(key).unwrap_or(&Value::Null)
}

fn member_err<'a>(obj: &'a Value, key: &str) -> Result<&'a Value, String> {
    match obj {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => Ok(m.get(key).unwrap_or(&Value::Null)),
        v if crate::js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        _ => Ok(&Value::Null),
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}' || c == '\u{00A0}')
}

fn trim_lower(v: &Value, expr: &str) -> Result<String, String> {
    match v.as_str() {
        Some(s) => Ok(js_trim(s).to_lowercase()),
        None => Err(format!("Cannot read properties of {expr}")),
    }
}

fn js_num_str(v: &Value) -> String {
    crate::js::number_to_string(crate::js::as_number(v).unwrap_or(f64::NAN))
}

fn js_falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|x| x == 0.0 || x.is_nan()),
        Value::String(s) => s.is_empty(),
        _ => false,
    }
}

fn is_set(v: &Value) -> Option<&Vec<Value>> {
    v.get("$set").and_then(Value::as_array)
}

/// Port of `githubInboxFailureBackoffMs`.
pub fn github_inbox_failure_backoff_ms(consecutive_failures: f64) -> Result<f64, String> {
    if consecutive_failures.is_nan() {
        return Ok(f64::NAN);
    }
    let exponent = (consecutive_failures.floor() - 1.0).max(0.0);
    Ok(600_000.0_f64.min(30_000.0 * 2f64.powf(exponent.min(16.0))))
}

/// Port of `githubInboxRemoteItemState`.
pub fn github_inbox_remote_item_state(item: Value) -> Result<Value, String> {
    let remote = member_err(&item, "item")?;
    let state = match remote {
        Value::Null => return Err("Cannot read properties of null (reading 'state')".into()),
        _ if crate::js::is_undefined(remote) => {
            return Err("Cannot read properties of undefined (reading 'state')".into());
        }
        _ => member(remote, "state"),
    };
    Ok(Value::String(if state.as_str() == Some("open") {
        "open".into()
    } else {
        "closed".into()
    }))
}

fn same_login(actor: &Value, normalized_viewer: &str) -> bool {
    !normalized_viewer.is_empty()
        && member_err(actor, "login")
            .ok()
            .and_then(|l| l.as_str())
            .is_some_and(|l| l.trim().to_lowercase() == normalized_viewer)
}

/// Port of `githubViewerInvolvement`.
pub fn github_viewer_involvement(input: Value) -> Result<Value, String> {
    let author = member_err(&input, "author")?;
    let viewer = member_err(&input, "viewer")?;
    let normalized_viewer = trim_lower(viewer, "undefined (reading 'trim')")?;
    let assignees = member(&input, "assignees");
    let assigned = match assignees {
        Value::Array(list) => list.iter().any(|a| same_login(a, &normalized_viewer)),
        _ => false,
    };
    let authored = same_login(author, &normalized_viewer);
    let involved =
        authored || assigned || member(&input, "matchedInvolvesSearch") == &Value::Bool(true);
    Ok(serde_json::json!({
        "authored": authored,
        "assigned": assigned,
        "involved": involved,
    }))
}

/// Port of `buildIssueListEntry`.
pub fn build_issue_list_entry(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot destructure property 'issue' of 'input' as it is null.".into());
    }
    let project = member(&input, "project");
    let issue = member(&input, "issue");
    let project_id = member_err(project, "id")?;
    let is_pinned = member(&input, "isPinned");
    let context = serde_json::json!({
        "projectId": project_id,
        "projectTitle": member(project, "title"),
        "isPinned": is_pinned,
    });
    let mut entry = Map::new();
    for (k, v) in [
        ("projectId", project_id.clone()),
        ("projectTitle", member(project, "title").clone()),
        ("projectContexts", Value::Array(vec![context])),
        ("repository", member(&input, "repository").clone()),
        ("number", member(issue, "number").clone()),
        ("title", member(issue, "title").clone()),
        ("url", member(issue, "url").clone()),
        ("author", member(issue, "author").clone()),
        ("state", member(issue, "state").clone()),
        ("stateReason", member(issue, "stateReason").clone()),
        ("labels", member(issue, "labels").clone()),
        ("assignees", member(issue, "assignees").clone()),
        ("commentCount", member(issue, "commentCount").clone()),
        ("createdAt", member(issue, "createdAt").clone()),
        ("updatedAt", member(issue, "updatedAt").clone()),
        ("closedAt", member(issue, "closedAt").clone()),
        ("isPinned", is_pinned.clone()),
        (
            "viewerInvolvement",
            member(&input, "viewerInvolvement").clone(),
        ),
    ] {
        entry.insert(k.to_string(), v);
    }
    Ok(Value::Object(entry))
}

fn set_has(v: &Value, needle: &Value) -> Result<bool, String> {
    match is_set(v) {
        Some(items) => Ok(items.iter().any(|x| {
            if let (Some(a), Some(b)) = (crate::js::as_number(x), crate::js::as_number(needle)) {
                a == b
            } else {
                x == needle
            }
        })),
        None => Err("context.involvedNumbers.has is not a function".into()),
    }
}

fn merge_project_contexts(entries: &[Value]) -> Vec<Value> {
    let mut by_id: Vec<(String, Value)> = Vec::new();
    for entry in entries {
        if let Some(contexts) = member(entry, "projectContexts").as_array() {
            for context in contexts {
                let pid = js_num_str(member(context, "projectId"));
                if let Some((_, existing)) = by_id.iter_mut().find(|(id, _)| *id == pid) {
                    if member(context, "isPinned") == &Value::Bool(true) {
                        existing["isPinned"] = Value::Bool(true);
                    }
                } else {
                    by_id.push((pid, context.clone()));
                }
            }
        }
    }
    by_id.sort_by(|(a, _), (b, _)| a.cmp(b));
    by_id.into_iter().map(|(_, c)| c).collect()
}

/// Port of `buildGitHubInboxItem`.
pub fn build_git_hub_inbox_item(context: Value, remote: Value) -> Result<Value, String> {
    let item = member_err(&remote, "item")?;
    let number = member(item, "number");
    let matched_involves = set_has(member(&context, "involvedNumbers"), number)?;
    let kind = member(&remote, "kind").as_str().unwrap_or("");
    let viewer = member(&context, "viewer");
    let involvement_input = serde_json::json!({
        "author": member(item, "author"),
        "assignees": member(item, "assignees"),
        "viewer": viewer,
        "matchedInvolvesSearch": matched_involves,
    });
    let viewer_involvement = github_viewer_involvement(involvement_input)?;
    let projects = member(&context, "projects");
    let projects = match projects {
        Value::Array(p) => p,
        _ => return Err("context.projects.map is not a function".into()),
    };
    let pinned_keys = member(&context, "pinnedKeys");
    let mut entries = Vec::new();
    for project in projects {
        let project_id = member_err(project, "id")?;
        let identity = format!(
            "{}\u{0}{}\u{0}{}",
            js_num_str(project_id),
            trim_lower(member(&context, "repository"), "context.repository")?,
            js_num_str(number)
        );
        let is_pinned = match is_set(pinned_keys) {
            Some(keys) => keys.iter().any(|k| k.as_str() == Some(identity.as_str())),
            None => false,
        };
        let mut input_obj = Map::new();
        input_obj.insert("project".to_string(), project.clone());
        input_obj.insert(
            "repository".to_string(),
            member(&context, "repository").clone(),
        );
        input_obj.insert("isPinned".to_string(), Value::Bool(is_pinned));
        input_obj.insert("viewerInvolvement".to_string(), viewer_involvement.clone());
        if kind == "pullRequest" {
            input_obj.insert("pullRequest".to_string(), item.clone());
            input_obj.insert(
                "viewerReviewRequested".to_string(),
                member(item, "reviewRequestLogins").clone(),
            );
            entries.push(build_pull_request_list_entry(Value::Object(input_obj))?);
        } else {
            input_obj.insert("issue".to_string(), item.clone());
            entries.push(build_issue_list_entry(Value::Object(input_obj))?);
        }
    }
    let Some(first) = entries.first() else {
        return Err("Cannot read properties of undefined (reading 'projectId')".into());
    };
    let contexts = merge_project_contexts(&entries);
    let any_pinned = contexts
        .iter()
        .any(|c| member(c, "isPinned") == &Value::Bool(true));
    let mut out = first.clone();
    if let Some(obj) = out.as_object_mut() {
        obj.insert("projectContexts".to_string(), Value::Array(contexts));
        obj.insert("isPinned".to_string(), Value::Bool(any_pinned));
    }
    let mut wrapped = Map::new();
    wrapped.insert(
        "kind".to_string(),
        Value::String(if kind == "pullRequest" {
            "pullRequest".into()
        } else {
            "issue".into()
        }),
    );
    if let Value::Object(m) = out {
        wrapped.extend(m);
    }
    Ok(Value::Object(wrapped))
}

fn build_pull_request_list_entry(input: Value) -> Result<Value, String> {
    let pull_request = member(&input, "pullRequest");
    let project = member_err(&input, "project")?;
    let project_id = member_err(project, "id")?.clone();
    let is_pinned = member(&input, "isPinned");
    let context = serde_json::json!({
        "projectId": project_id,
        "projectTitle": member(project, "title"),
        "isPinned": is_pinned,
    });
    let mut entry = Map::new();
    for (k, v) in [
        ("projectId", project_id.clone()),
        ("projectTitle", member(project, "title").clone()),
        ("projectContexts", Value::Array(vec![context])),
        ("repository", member(&input, "repository").clone()),
        ("number", member(pull_request, "number").clone()),
        ("title", member(pull_request, "title").clone()),
        ("url", member(pull_request, "url").clone()),
        ("author", member(pull_request, "author").clone()),
        ("headBranch", member(pull_request, "headBranch").clone()),
        ("baseBranch", member(pull_request, "baseBranch").clone()),
        ("state", member(pull_request, "state").clone()),
        ("isDraft", member(pull_request, "isDraft").clone()),
        ("additions", member(pull_request, "additions").clone()),
        ("deletions", member(pull_request, "deletions").clone()),
        ("createdAt", member(pull_request, "createdAt").clone()),
        ("updatedAt", member(pull_request, "updatedAt").clone()),
        (
            "reviewDecision",
            member(pull_request, "reviewDecision").clone(),
        ),
        (
            "viewerReviewRequested",
            member(&input, "viewerReviewRequested").clone(),
        ),
        ("isPinned", is_pinned.clone()),
        ("mergeability", member(pull_request, "mergeability").clone()),
        ("stack", member(pull_request, "stack").clone()),
        ("labels", member(pull_request, "labels").clone()),
        ("commentCount", member(pull_request, "commentCount").clone()),
        ("assignees", member(pull_request, "assignees").clone()),
        (
            "viewerInvolvement",
            member(&input, "viewerInvolvement").clone(),
        ),
    ] {
        entry.insert(k.to_string(), v);
    }
    Ok(Value::Object(entry))
}

/// Port of `githubItemCacheKey`.
pub fn github_item_cache_key(repository: String, number: f64) -> Result<String, String> {
    Ok(format!(
        "{}\u{0}{}",
        js_trim(&repository).to_lowercase(),
        crate::js::number_to_string(number)
    ))
}

/// Port of `githubInboxItemIdentityKey`.
pub fn github_inbox_item_identity_key(item: Value) -> Result<String, String> {
    let repository = member_err(&item, "repository")?;
    let repo = trim_lower(repository, "undefined (reading 'trim')")?;
    Ok(format!("{repo}#{}", js_num_str(member(&item, "number"))))
}

/// Port of `summarizeGitHubInboxError`.
pub fn summarize_git_hub_inbox_error(error: Value) -> Result<String, String> {
    let reason = member_err(&error, "reason")?;
    if reason.as_str() == Some("rate-limited") {
        return Ok("GitHub rate limit reached.".into());
    }
    if reason.as_str() == Some("not-authenticated") || reason.as_str() == Some("not-installed") {
        let detail = member(&error, "detail");
        return Ok(detail
            .as_str()
            .unwrap_or("GitHub request failed.")
            .to_string());
    }
    let detail = member_err(&error, "detail")?;
    let detail = match detail.as_str() {
        Some(d) => d,
        None => return Err("Cannot read properties of undefined (reading 'replace')".into()),
    };
    let tail = stderr_tail(detail).unwrap_or(detail);
    let text = js_trim(tail.trim_start_matches("gh:").trim_start_matches("GH:"));
    let first_line = js_trim(text.split(['\n', '\r']).next().unwrap_or(""));
    if first_line.is_empty() {
        return Ok("GitHub request failed.".into());
    }
    Ok(if first_line.encode_utf16().count() > 300 {
        let truncated: String = first_line
            .encode_utf16()
            .take(297)
            .map(|u| char::from_u32(u as u32).unwrap_or('\u{FFFD}'))
            .collect();
        format!("{truncated}...")
    } else {
        first_line.to_string()
    })
}

fn stderr_tail(detail: &str) -> Option<&str> {
    for marker in [") ", "timed out. "] {
        if let Some(pos) = detail.rfind(marker) {
            let start = pos + marker.len();
            if start < detail.len() {
                let tail = &detail[start..];
                if !tail.is_empty()
                    && (marker == "timed out. " || detail[..pos].ends_with("(code="))
                {
                    return Some(tail);
                }
            }
        }
    }
    None
}

/// Port of `mergeRepositoryInbox`.
pub fn merge_repository_inbox(lists: Value, involvement: Option<Value>) -> Result<Value, String> {
    let involvement = involvement.unwrap_or(Value::Null);
    if js_falsy(&involvement) {
        let mut out = spread(&lists);
        out.insert("involvedNumbers".to_string(), Value::Array(vec![]));
        return Ok(Value::Object(out));
    }
    let mut present: Vec<Option<f64>> = Vec::new();
    let pull_reqs = iterable_member(&lists, "pullRequests")?;
    let issues_src = iterable_member(&lists, "issues")?;
    let mut pull_requests = pull_reqs.to_vec();
    let mut issues = issues_src.to_vec();
    for item in pull_requests.iter().chain(issues.iter()) {
        present.push(crate::js::as_number(member(item, "number")));
    }
    let items = member(&involvement, "items");
    let items = match items {
        Value::Array(i) => i.clone(),
        _ => return Err("involvement.items is not iterable".into()),
    };
    for remote in items {
        let item = member_err(&remote, "item")?;
        let n = crate::js::as_number(member(item, "number"));
        if present.iter().any(|p| opt_eq(*p, n)) {
            continue;
        }
        present.push(n);
        if member(&remote, "kind").as_str() == Some("pullRequest") {
            pull_requests.push(item.clone());
        } else {
            issues.push(item.clone());
        }
    }
    let mut out = spread(&lists);
    out.insert("pullRequests".to_string(), Value::Array(pull_requests));
    out.insert("issues".to_string(), Value::Array(issues));
    out.insert(
        "involvedNumbers".to_string(),
        member(&involvement, "involvedNumbers").clone(),
    );
    out.insert(
        "rateLimit".to_string(),
        lower_remaining(
            member(&lists, "rateLimit"),
            member(&involvement, "rateLimit"),
        ),
    );
    Ok(Value::Object(out))
}

/// JS `Set` SameValueZero membership on Option<f64>: `NaN` dedups with `NaN`, `None`
/// (missing/undefined number) dedups with `None`.
fn opt_eq(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x == y || (x.is_nan() && y.is_nan()),
        (None, None) => true,
        _ => false,
    }
}

fn iterable_member<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => match m.get(key) {
            Some(Value::Array(a)) => Ok(a),
            Some(_) | None => Err(format!("lists.{key} is not iterable")),
        },
        _ => Err(format!("lists.{key} is not iterable")),
    }
}

fn lower_remaining(left: &Value, right: &Value) -> Value {
    match (js_falsy(left), js_falsy(right)) {
        (true, _) | (_, true) => {
            if js_falsy(left) {
                if js_falsy(right) {
                    Value::Null
                } else {
                    right.clone()
                }
            } else {
                left.clone()
            }
        }
        _ => {
            let l = crate::js::as_number(member(left, "remaining")).unwrap_or(f64::NAN);
            let r = crate::js::as_number(member(right, "remaining")).unwrap_or(f64::NAN);
            if r < l { right.clone() } else { left.clone() }
        }
    }
}

/// Object-spread of an arbitrary JS value: arrays/indexable values spread index keys, strings
/// spread characters, objects their own keys, primitives nothing.
fn spread(v: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                out.insert(k.clone(), val.clone());
            }
        }
        Value::Array(a) => {
            for (i, val) in a.iter().enumerate() {
                out.insert(i.to_string(), val.clone());
            }
        }
        Value::String(s) => {
            for (i, u) in s.encode_utf16().enumerate() {
                out.insert(
                    i.to_string(),
                    Value::String(char::from_u32(u as u32).unwrap_or('\u{FFFD}').to_string()),
                );
            }
        }
        _ => {}
    }
    out
}
