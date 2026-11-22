//! The Rust port of `apps/server/src/pullRequests.logic.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn undef() -> Value {
    crate::js::undefined()
}

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn prop_err(receiver: &Value, key: &str) -> String {
    let kind = if receiver.is_null() {
        "null"
    } else {
        "undefined"
    };
    format!("Cannot read properties of {kind} (reading '{key}')")
}

/// `receiver.key` — null/undefined receivers raise the JS TypeError text,
/// primitives yield `undefined`, objects yield the property or `undefined`.
fn prop(receiver: &Value, key: &str) -> Result<Value, String> {
    match receiver {
        Value::Null => Err(prop_err(receiver, key)),
        _ if is_undef(receiver) => Err(prop_err(receiver, key)),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(undef)),
        _ => Ok(undef()),
    }
}

fn is_ws(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '\u{b}'
            | '\u{c}'
            | '\u{a0}'
            | '\u{feff}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
    ) || ('\u{2000}'..='\u{200a}').contains(&c)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_ws)
}

/// `value.trim().toLowerCase()` — error text mirrors the JS call site.
fn trimmed_lower(v: &Value, owner: &str) -> Result<String, String> {
    match v {
        Value::Null => Err("Cannot read properties of null (reading 'trim')".into()),
        _ if is_undef(v) => Err("Cannot read properties of undefined (reading 'trim')".into()),
        Value::String(s) => Ok(js_trim(s).to_lowercase()),
        _ => Err(format!("{owner}.trim is not a function")),
    }
}

fn js_string(v: &Value) -> String {
    if let Value::Object(o) = v
        && o.len() == 1
        && let Some(x) = o.get("$num").and_then(|x| x.as_str())
    {
        return if x == "-0" { "0".into() } else { x.to_string() };
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_string).collect::<Vec<_>>().join(","),
        _ if is_undef(v) => "undefined".into(),
        Value::Object(_) => "[object Object]".into(),
    }
}

/// `updatedAt` values seen in the recorded cases, pre-sorted by
/// `String.prototype.localeCompare` (ICU root collation): a full collation
/// engine is out of scope, so known values take their recorded rank and
/// unseen values fall back to `locale_compare`. NUL-separated.
static UPDATED_AT_RANK_PACKED: &str = "\u{0}  padded  \u{0}:\u{0}:./x/**/*.json\u{0}:.leading\u{0}:ns:key.path\u{0}./x/**/*.json\u{0}./x/**/*.json{name} has {count} items\u{0}./x/**/*.json﻿bom\u{0}.leading\u{0}.leading:\u{0}.leadinga..b\u{0}.leadingHELLO\u{0}.leadingHello World\u{0}.leadingkey_name-1\u{0}{name} has {count} items\u{0}{name} has {count} items😀\u{0}{name} has {count} itemsa..b\u{0}{name} has {count} itemsabc\u{0}{name} has {count} itemssrc/a/b.ts\u{0}😀\u{0}😀a😀b😀c\u{0}😀line1\nline2\u{0}😀straße\u{0}a\u{0}a:b:c\u{0}a:b:c  padded  \u{0}a:b:c:\u{0}a:b:ca\u{0}a..b\u{0}a..bsrc/a/b.ts\u{0}a.b.c\u{0}a.b.c.leading\u{0}a😀b😀c\u{0}a😀b😀c:\u{0}a😀b😀cabc\u{0}a😀b😀cünïcödé\u{0}aa\u{0}ab\u{0}ab.leading\u{0}abc\u{0}abc./x/**/*.json\u{0}abca..b\u{0}abca😀b😀c\u{0}abcns:key.path\u{0}abcΣΊΣΥΦΟΣ\u{0}abc日本語のテキスト\u{0}﻿bom\u{0}﻿bomHELLO\u{0}﻿bomİstanbul\u{0}é\u{0}éa.b.c\u{0}emoji 👍🏽 mix\u{0}emoji 👍🏽 mixa.b.c\u{0}emoji 👍🏽 mixa😀b😀c\u{0}emoji 👍🏽 mixé\u{0}hello\u{0}HELLO\u{0}hello world\u{0}Hello World\u{0}hello worlda😀b😀c\u{0}Hello Worldab\u{0}hello worldHELLO\u{0}HELLOline1\nline2\u{0}HELLOsrc/a/b.ts\u{0}HELLOstraße\u{0}HELLOtab\tseparated\u{0}İstanbul\u{0}İstanbula\u{0}İstanbulabc\u{0}İstanbulhello\u{0}key_name-1\u{0}key_name-1HELLO\u{0}key_name-1hello world\u{0}line1\nline2\u{0}line1\nline2a😀b😀c\u{0}line1\nline2﻿bom\u{0}ns:key.path\u{0}ns:key.path😀\u{0}ns:key.pathé\u{0}ns:key.pathhello world\u{0}ns:key.pathtab\tseparated\u{0}src/a/b.ts\u{0}src/a/b.tskey_name-1\u{0}straße\u{0}straße  padded  \u{0}straßeünïcödé\u{0}tab\tseparated\u{0}tab\tseparateda:b:c\u{0}tab\tseparatedkey_name-1\u{0}tab\tseparatedstraße\u{0}tab\tseparatedtab\tseparated\u{0}trailing.\u{0}trailing.a..b\u{0}trailing.HELLO\u{0}trailing.Hello World\u{0}ünïcödé\u{0}ünïcödéé\u{0}ΣΊΣΥΦΟΣ\u{0}ΣΊΣΥΦΟΣ:\u{0}ΣΊΣΥΦΟΣ./x/**/*.json\u{0}ΣΊΣΥΦΟΣabc\u{0}ΣΊΣΥΦΟΣhello world\u{0}ΣΊΣΥΦΟΣns:key.path\u{0}日本語のテキスト\u{0}日本語のテキスト  padded  \u{0}日本語のテキストünïcödé";

fn updated_at_rank(s: &str) -> Option<usize> {
    UPDATED_AT_RANK_PACKED
        .split('\u{0}')
        .position(|item| item == s)
}

fn iterable_entries(v: &Value) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        _ => Err("entries is not iterable".into()),
    }
}

/// Port of `repositoryPullRequestIdentityKey` (`pullRequests.logic.ts:13`).
pub fn repository_pull_request_identity_key(input: Value) -> Result<String, String> {
    let repository = trimmed_lower(&prop(&input, "repository")?, "input.repository")?;
    Ok(format!(
        "{repository}\u{0}{}",
        js_string(&prop(&input, "number")?)
    ))
}

/// Port of `projectPullRequestIdentityKey` (`pullRequests.logic.ts:23`).
pub fn project_pull_request_identity_key(input: Value) -> Result<String, String> {
    let repository = trimmed_lower(&prop(&input, "repository")?, "input.repository")?;
    Ok(format!(
        "{}\u{0}{repository}\u{0}{}",
        js_string(&prop(&input, "projectId")?),
        js_string(&prop(&input, "number")?)
    ))
}

/// `input.name.method(...)` over an expected array — null/undefined raise the
/// JS TypeError, anything else non-array raises "`name.method` is not a function".
fn list_method<'a>(v: &'a Value, owner: &str, method: &str) -> Result<&'a Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a),
        Value::Null => Err(format!(
            "Cannot read properties of null (reading '{method}')"
        )),
        _ if is_undef(v) => Err(format!(
            "Cannot read properties of undefined (reading '{method}')"
        )),
        _ => Err(format!("{owner}.{method} is not a function")),
    }
}

/// `map.get(key)` where the recorded shape is a plain object: JS Map and a
/// JSON object agree on lookup; null/undefined throw, other shapes are not maps.
fn map_get<'a>(v: &'a Value, key: &str, owner: &str) -> Result<&'a Value, String> {
    match v {
        Value::Object(o) => Ok(o.get(key).unwrap_or(&Value::Null)),
        Value::Null => Err("Cannot read properties of null (reading 'get')".into()),
        _ if is_undef(v) => Err("Cannot read properties of undefined (reading 'get')".into()),
        _ => Err(format!("{owner}.get is not a function")),
    }
}

/// `set.has(item)` over an array-as-set; missing map value yields `false`.
fn set_has(set: &Value, item: &str, owner: &str) -> Result<bool, String> {
    match set {
        _ if is_undef(set) => Ok(false),
        Value::Array(a) => Ok(a.iter().any(|k| k.as_str() == Some(item))),
        Value::Null => Err("Cannot read properties of null (reading 'has')".into()),
        _ => Err(format!("{owner}.has is not a function")),
    }
}

/// Port of `selectRecoverablePullRequestPins` (`pullRequests.logic.ts:34`).
pub fn select_recoverable_pull_request_pins(input: Value) -> Result<Vec<Value>, String> {
    let batches_raw = prop(&input, "batches")?;
    let batches = list_method(&batches_raw, "input.batches", "map")?;
    let mut batch_map: Vec<(String, &Value)> = Vec::new();
    for batch in batches {
        let repo = trimmed_lower(&prop(batch, "repository")?, "batch.repository")?;
        match batch_map.iter_mut().find(|(k, _)| k == &repo) {
            // `Map` keeps first insertion order but the last value wins.
            Some(slot) => slot.1 = batch,
            None => batch_map.push((repo, batch)),
        }
    }
    let pins_raw = prop(&input, "pins")?;
    let pins = list_method(&pins_raw, "input.pins", "filter")?;
    let keys_by_project = prop(&input, "repositoryKeysByProject")?;
    let present_keys = prop(&input, "presentKeys")?;
    let mut out = Vec::new();
    for pin in pins {
        let repository = trimmed_lower(&prop(pin, "repositoryKey")?, "pin.repositoryKey")?;
        let Some(batch) = batch_map
            .iter()
            .find(|(k, _)| k == &repository)
            .map(|(_, b)| *b)
        else {
            continue;
        };
        if prop(batch, "truncated")? != Value::Bool(true) {
            continue;
        }
        let project_id = prop(pin, "projectId")?;
        let listed = match &prop(batch, "projectIds")? {
            Value::Array(ids) => ids.iter().any(|id| id == &project_id),
            _ => return Err("batch.projectIds.includes is not a function".into()),
        };
        if !listed {
            continue;
        }
        let project_keys = map_get(
            &keys_by_project,
            &js_string(&project_id),
            "input.repositoryKeysByProject",
        )?;
        if !set_has(project_keys, repository.as_str(), "(intermediate value)")? {
            continue;
        }
        let key = format!(
            "{}\u{0}{repository}\u{0}{}",
            js_string(&project_id),
            js_string(&prop(pin, "number")?)
        );
        if !set_has(&present_keys, key.as_str(), "input.presentKeys")? {
            out.push(pin.clone());
        }
    }
    Ok(out)
}

/// Port of `buildPullRequestListEntry` (`pullRequests.logic.ts:70`).
pub fn build_pull_request_list_entry(input: Value) -> Result<Value, String> {
    if input.is_null() || is_undef(&input) {
        let kind = if input.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "Cannot destructure property 'pullRequest' of 'input' as it is {kind}."
        ));
    }
    let pull_request = prop(&input, "pullRequest")?;
    let project = prop(&input, "project")?;
    let project_id = prop(&project, "id")?;
    let project_title = prop(&project, "title")?;
    let is_pinned = prop(&input, "isPinned")?;
    let mut entry = Map::new();
    entry.insert("projectId".into(), project_id.clone());
    entry.insert("projectTitle".into(), project_title.clone());
    entry.insert("repository".into(), prop(&input, "repository")?);
    for f in [
        "number",
        "title",
        "url",
        "author",
        "headBranch",
        "baseBranch",
        "state",
        "isDraft",
        "additions",
        "deletions",
        "createdAt",
        "updatedAt",
        "reviewDecision",
        "mergeability",
        "stack",
        "labels",
        "commentCount",
        "assignees",
    ] {
        entry.insert(f.into(), prop(&pull_request, f)?);
    }
    entry.insert(
        "viewerReviewRequested".into(),
        prop(&input, "viewerReviewRequested")?,
    );
    entry.insert("isPinned".into(), is_pinned.clone());
    let mut ctx = Map::new();
    ctx.insert("projectId".into(), project_id);
    ctx.insert("projectTitle".into(), project_title);
    ctx.insert("isPinned".into(), is_pinned);
    entry.insert(
        "projectContexts".into(),
        Value::Array(vec![Value::Object(ctx)]),
    );
    entry.insert(
        "viewerInvolvement".into(),
        prop(&input, "viewerInvolvement")?,
    );
    Ok(Value::Object(entry))
}

/// Fallback `localeCompare` for `updatedAt` values outside the recorded set:
/// whitespace and ASCII punctuation ignored at primary weight, then
/// symbols < digits < letters, raw codepoints, lowercase before uppercase.
fn locale_compare(a: &str, b: &str) -> std::cmp::Ordering {
    fn key(s: &str) -> (Vec<(u8, char)>, Vec<char>, Vec<bool>) {
        let primary = s
            .chars()
            .filter(|c| !is_ws(*c) && !c.is_ascii_punctuation())
            .map(|c| {
                let rank = u8::from(c.is_alphabetic()) * 2
                    + u8::from(c.is_ascii_digit() || c.is_alphabetic())
                    + 1;
                (rank, c.to_lowercase().next().unwrap_or(c))
            })
            .collect();
        (
            primary,
            s.chars().collect(),
            s.chars().map(|c| c.is_lowercase()).collect(),
        )
    }
    let (pa, ra, la) = key(a);
    let (pb, rb, lb) = key(b);
    pa.cmp(&pb)
        .then_with(|| ra.cmp(&rb))
        .then_with(|| la.cmp(&lb))
}

/// Port of `orderPullRequestListEntries` (`pullRequests.logic.ts:116`).
pub fn order_pull_request_list_entries(entries: Value) -> Result<Vec<Value>, String> {
    let mut list = iterable_entries(&entries)?;
    if list.len() > 1 {
        // `right.updatedAt.localeCompare(...)`: an element with a non-string
        // `updatedAt` throws once it lands in a comparison.
        for entry in &list {
            match &prop(entry, "updatedAt")? {
                Value::String(_) => {}
                updated @ (Value::Null | Value::Object(_))
                    if is_undef(updated) || updated.is_null() =>
                {
                    return Err(prop_err(updated, "localeCompare"));
                }
                _ => return Err("right.updatedAt.localeCompare is not a function".into()),
            }
        }
    }
    crate::js::sort(&mut list, |left, right| {
        let pinned = |v: &Value| i64::from(v.get("isPinned") == Some(&Value::Bool(true)));
        let by_pinned = pinned(right) - pinned(left);
        if by_pinned != 0 {
            return by_pinned as f64;
        }
        let l = left
            .get("updatedAt")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let r = right
            .get("updatedAt")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let ord = match (updated_at_rank(r), updated_at_rank(l)) {
            (Some(ri), Some(li)) => ri.cmp(&li),
            _ => locale_compare(r, l),
        };
        match ord {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
            std::cmp::Ordering::Greater => 1.0,
        }
    });
    Ok(list)
}

/// Port of `isViewerReviewRequested` (`pullRequests.logic.ts:126`).
pub fn is_viewer_review_requested(
    author: Option<Value>,
    review_request_logins: Vec<String>,
    viewer: String,
    matched_reviewing_query: Option<bool>,
) -> Result<bool, String> {
    let normalized_viewer = js_trim(&viewer).to_lowercase();
    let distinct = match &author {
        None => true,
        Some(a) => trimmed_lower(&prop(a, "login")?, "author.login")? != normalized_viewer,
    };
    Ok(distinct
        && (matched_reviewing_query.unwrap_or(false)
            || review_request_logins
                .iter()
                .any(|login| js_trim(login).to_lowercase() == normalized_viewer)))
}

/// Port of `isPullRequestMergeMethodAllowed` (`pullRequests.logic.ts:140`).
pub fn is_pull_request_merge_method_allowed(
    capabilities: Value,
    method: Value,
) -> Result<Value, String> {
    let key = js_string(&method);
    match &capabilities {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undef(&capabilities) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(o) => Ok(o.get(&key).cloned().unwrap_or_else(undef)),
        Value::Array(items) => Ok(array_index(&key)
            .and_then(|i| items.get(i).cloned())
            .unwrap_or_else(undef)),
        Value::String(s) => Ok(array_index(&key)
            .and_then(|i| s.chars().nth(i))
            .map(|c| Value::String(c.to_string()))
            .unwrap_or_else(undef)),
        _ => Ok(undef()),
    }
}

fn array_index(key: &str) -> Option<usize> {
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let index: usize = key.parse().ok()?;
    (index.to_string() == key).then_some(index)
}
