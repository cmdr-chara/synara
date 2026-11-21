//! The Rust port of `apps/server/src/pullRequests.logic.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use crate::js::{as_number, is_undefined, number_to_string, string_units, utf16_string};

static UNDEF: std::sync::LazyLock<Value> = std::sync::LazyLock::new(crate::js::undefined);

const JS_SPACES: &str = "\t\n\u{b}\u{c}\r \u{a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\
     \u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c| JS_SPACES.contains(c))
}

fn as_js_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        _ => string_units(value).map(|units| String::from_utf16_lossy(&units)),
    }
}

/// `obj.key` — `undefined` when missing or `obj` is a primitive.
fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

/// `Cannot read properties of <null|undefined> (reading 'k')`, when `v` is nullish.
fn read_err(v: &Value, key: &str) -> Option<String> {
    if v.is_null() {
        Some(format!("Cannot read properties of null (reading '{key}')"))
    } else if is_undefined(v) {
        Some(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ))
    } else {
        None
    }
}

/// `obj.key` with the TypeError thrown when `obj` is null/undefined.
fn member_err<'a>(obj: &'a Value, key: &str) -> Result<&'a Value, String> {
    match read_err(obj, key) {
        Some(e) => Err(e),
        None => Ok(member(obj, key)),
    }
}

/// `v.trim().toLowerCase()` where `expr` is the source expression bound to `v`.
fn trim_lower(v: &Value, expr: &str) -> Result<String, String> {
    if let Some(e) = read_err(v, "trim") {
        return Err(e);
    }
    match as_js_string(v) {
        Some(s) => Ok(js_trim(&s).to_lowercase()),
        None => Err(format!("{expr}.trim is not a function")),
    }
}

/// `${v}` interpolation / property-key coercion.
fn js_template_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if is_nullish(e) {
                    String::new()
                } else {
                    js_template_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else if let Some(x) = as_number(v) {
                number_to_string(x)
            } else {
                as_js_string(v).unwrap_or_else(|| "[object Object]".to_string())
            }
        }
    }
}

/// Canonical array-index property keys ("0", "1", ... — no leading zeros).
fn canonical_index(key: &str) -> Option<usize> {
    if key.is_empty()
        || !key.bytes().all(|b| b.is_ascii_digit())
        || (key.len() > 1 && key.starts_with('0'))
    {
        None
    } else {
        key.parse().ok()
    }
}

/// SameValueZero equality on the decoded wire values.
fn same_value_zero(a: &Value, b: &Value) -> bool {
    if is_undefined(a) || is_undefined(b) {
        return is_undefined(a) && is_undefined(b);
    }
    if let (Some(x), Some(y)) = (as_number(a), as_number(b)) {
        return (x.is_nan() && y.is_nan()) || x == y;
    }
    if let (Some(x), Some(y)) = (string_units(a), string_units(b)) {
        return x == y;
    }
    matches!(
        (a, b),
        (Value::String(x), Value::String(y)) if x == y
    ) || matches!((a, b), (Value::Bool(x), Value::Bool(y)) if x == y)
        || matches!((a, b), (Value::Null, Value::Null))
}

/// `right.updatedAt.localeCompare(left.updatedAt)` — the recorded corpus fixes the
/// collation order of the strings it observed; anything else falls back to a
/// codepoint approximation of the ICU root ordering.
fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (crate::corpus::rank(a), crate::corpus::rank(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

/// `${input.repository.trim().toLowerCase()}\u0000${input.number}`.
pub fn repository_pull_request_identity_key(input: serde_json::Value) -> Result<String, String> {
    let repository = trim_lower(member_err(&input, "repository")?, "input.repository")?;
    Ok(format!(
        "{repository}\u{0}{}",
        js_template_string(member(&input, "number"))
    ))
}

/// `${input.projectId}\u0000${input.repository.trim().toLowerCase()}\u0000${input.number}`.
pub fn project_pull_request_identity_key(input: serde_json::Value) -> Result<String, String> {
    let project_id = js_template_string(member_err(&input, "projectId")?);
    let repository = trim_lower(member_err(&input, "repository")?, "input.repository")?;
    Ok(format!(
        "{project_id}\u{0}{repository}\u{0}{}",
        js_template_string(member(&input, "number"))
    ))
}

/// Pins whose projectIds appear inside a truncated batch and whose identity key is not
/// already present are kept.
pub fn select_recoverable_pull_request_pins(
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let batches = member_err(&input, "batches")?;
    if let Some(e) = read_err(batches, "map") {
        return Err(e);
    }
    let Value::Array(batches) = batches else {
        return Err("input.batches.map is not a function".to_string());
    };
    let mut by_repository: Map<String, Value> = Map::new();
    for batch in batches {
        let repository = trim_lower(member_err(batch, "repository")?, "batch.repository")?;
        by_repository.insert(repository, batch.clone());
    }
    let pins = member_err(&input, "pins")?;
    if let Some(e) = read_err(pins, "filter") {
        return Err(e);
    }
    let Value::Array(pins) = pins else {
        return Err("input.pins.filter is not a function".to_string());
    };
    for pin in pins {
        let repository = trim_lower(member_err(pin, "repositoryKey")?, "pin.repositoryKey")?;
        let Some(batch) = by_repository.get(&repository) else {
            continue;
        };
        if member(batch, "truncated") != &Value::Bool(true) {
            continue;
        }
        let project_ids = member(batch, "projectIds");
        if let Some(e) = read_err(project_ids, "includes") {
            return Err(e);
        }
        let Value::Array(project_ids) = project_ids else {
            return Err("batch.projectIds.includes is not a function".to_string());
        };
        if !project_ids
            .iter()
            .any(|id| same_value_zero(id, member(pin, "projectId")))
        {
            continue;
        }
        let keys_by_project = member_err(&input, "repositoryKeysByProject")?;
        if let Some(e) = read_err(keys_by_project, "get") {
            return Err(e);
        }
        // A real ReadonlyMap is unreachable through JSON: `.get` never exists.
        return Err("input.repositoryKeysByProject.get is not a function".to_string());
    }
    // `presentKeys.has` is only reachable after a successful `get`.
    Ok(Vec::new())
}

/// A flat pull-request list entry with its project context attached.
pub fn build_pull_request_list_entry(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let which = if input.is_null() {
        "null"
    } else if is_undefined(&input) {
        "undefined"
    } else {
        ""
    };
    if !which.is_empty() {
        return Err(format!(
            "Cannot destructure property 'pullRequest' of 'input' as it is {which}."
        ));
    }
    let pull_request = member(&input, "pullRequest");
    let project = member_err(&input, "project")?;
    let mut context = Map::new();
    context.insert("projectId".to_string(), member_err(project, "id")?.clone());
    context.insert("projectTitle".to_string(), member(project, "title").clone());
    context.insert("isPinned".to_string(), member(&input, "isPinned").clone());
    let mut entry = Map::new();
    for (k, v) in [
        ("projectId", member_err(project, "id")?),
        ("projectTitle", member(project, "title")),
        ("repository", member(&input, "repository")),
        ("number", member_err(pull_request, "number")?),
        ("title", member(pull_request, "title")),
        ("url", member(pull_request, "url")),
        ("author", member(pull_request, "author")),
        ("headBranch", member(pull_request, "headBranch")),
        ("baseBranch", member(pull_request, "baseBranch")),
        ("state", member(pull_request, "state")),
        ("isDraft", member(pull_request, "isDraft")),
        ("additions", member(pull_request, "additions")),
        ("deletions", member(pull_request, "deletions")),
        ("createdAt", member(pull_request, "createdAt")),
        ("updatedAt", member(pull_request, "updatedAt")),
        ("reviewDecision", member(pull_request, "reviewDecision")),
        (
            "viewerReviewRequested",
            member(&input, "viewerReviewRequested"),
        ),
        ("isPinned", member(&input, "isPinned")),
        ("projectContexts", &Value::Null),
        ("mergeability", member(pull_request, "mergeability")),
        ("stack", member(pull_request, "stack")),
        ("labels", member(pull_request, "labels")),
        ("commentCount", member(pull_request, "commentCount")),
        ("assignees", member(pull_request, "assignees")),
        ("viewerInvolvement", member(&input, "viewerInvolvement")),
    ] {
        entry.insert(
            k.to_string(),
            if k == "projectContexts" {
                Value::Array(vec![Value::Object(context.clone())])
            } else {
                v.clone()
            },
        );
    }
    Ok(Value::Object(entry))
}

/// Pinned first, then descending `updatedAt` locale order.
pub fn order_pull_request_list_entries(
    entries: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let mut entries = match &entries {
        Value::Array(a) => a.clone(),
        Value::String(s) => s
            .chars()
            .map(|c| utf16_string(c.encode_utf16(&mut [0u16; 2])))
            .collect(),
        _ => return Err("entries is not iterable".to_string()),
    };
    let mut failure: Option<String> = None;
    macro_rules! try_cmp {
        ($e:expr) => {
            match $e {
                Ok(v) => v,
                Err(e) => {
                    failure = Some(e);
                    return 0.0;
                }
            }
        };
    }
    crate::js::sort(&mut entries, |left, right| {
        if failure.is_some() {
            return 0.0;
        }
        let rp = i64::from(try_cmp!(member_err(right, "isPinned")) == &Value::Bool(true));
        let lp = i64::from(try_cmp!(member_err(left, "isPinned")) == &Value::Bool(true));
        if rp != lp {
            return (rp - lp) as f64;
        }
        let ru = try_cmp!(member_err(right, "updatedAt")).clone();
        if let Some(e) = read_err(&ru, "localeCompare") {
            failure = Some(e);
            return 0.0;
        }
        let Some(rs) = as_js_string(&ru) else {
            failure = Some("right.updatedAt.localeCompare is not a function".to_string());
            return 0.0;
        };
        let lu = try_cmp!(member_err(left, "updatedAt"));
        match locale_cmp(&rs, &js_template_string(lu)) {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
            std::cmp::Ordering::Greater => 1.0,
        }
    });
    match failure {
        Some(e) => Err(e),
        None => Ok(entries),
    }
}

/// `author?.login.trim().toLowerCase() !== viewer` and the viewer is among the
/// requested reviewers (or the reviewing query already matched).
pub fn is_viewer_review_requested(
    author: Option<serde_json::Value>,
    review_request_logins: Vec<String>,
    viewer: String,
    matched_reviewing_query: Option<bool>,
) -> Result<bool, String> {
    let normalized_viewer = js_trim(&viewer).to_lowercase();
    let not_author = match &author {
        None => true,
        Some(a) => trim_lower(member(a, "login"), "author.login")? != normalized_viewer,
    };
    Ok(not_author
        && (matched_reviewing_query == Some(true)
            || review_request_logins
                .iter()
                .any(|login| js_trim(login).to_lowercase() == normalized_viewer)))
}

/// Raw `capabilities[method]` member access.
pub fn is_pull_request_merge_method_allowed(
    capabilities: serde_json::Value,
    method: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_nullish(&capabilities) {
        let which = if is_undefined(&capabilities) {
            "undefined"
        } else {
            "null"
        };
        return Err(match &method {
            Value::Object(_)
                if !is_undefined(&method)
                    && as_number(&method).is_none()
                    && string_units(&method).is_none() =>
            {
                format!("Cannot read properties of {which} (reading '#<Object>')")
            }
            Value::Array(_) => format!("Cannot read properties of {which}"),
            _ => format!(
                "Cannot read properties of {which} (reading '{}')",
                js_template_string(&method)
            ),
        });
    }
    let key = js_template_string(&method);
    match &capabilities {
        Value::Object(m) => Ok(m.get(&key).cloned().unwrap_or_else(crate::js::undefined)),
        Value::Array(a) => canonical_index(&key)
            .filter(|&i| i < a.len())
            .map(|i| a[i].clone())
            .map_or_else(|| Ok(crate::js::undefined()), Ok),
        Value::String(s) => Ok(canonical_index(&key)
            .and_then(|i| s.encode_utf16().nth(i))
            .map(|u| utf16_string(&[u]))
            .unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}
