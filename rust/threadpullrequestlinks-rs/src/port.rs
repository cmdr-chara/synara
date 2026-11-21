//! The Rust port of `packages/shared/src/threadPullRequestLinks.ts`.
//!
//! Clean-room port: behavior is pinned by `cases.jsonl`, not by reading the
//! TypeScript implementation.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value};

const LINK_LIMIT: usize = 50;

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'
            | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ if js::is_undefined(v) => false,
        _ => true,
    }
}

/// `value.key` where `value` is anything but null/undefined (members are `undefined` when absent).
fn member(v: &Value, k: &str) -> Value {
    if let Value::Object(o) = v {
        o.get(k).cloned().unwrap_or_else(js::undefined)
    } else {
        js::undefined()
    }
}

/// `receiver.key` including the TypeError thrown on null/undefined receivers.
fn read_member(receiver: &Value, k: &str) -> Result<Value, String> {
    if receiver.is_null() {
        return Err(format!("Cannot read properties of null (reading '{k}')"));
    }
    if js::is_undefined(receiver) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        ));
    }
    Ok(member(receiver, k))
}

/// `links ?? []` as a slice; non-array truthy values fail with the call-site error text.
fn links_items<'a>(links: &'a Value, err: &str) -> Result<&'a [Value], String> {
    if nullish(links) {
        return Ok(&[]);
    }
    match links {
        Value::Array(a) => Ok(a),
        _ => Err(err.into()),
    }
}

fn is_valid_gh_repo_name_with_owner(repository: &str) -> bool {
    let normalized = js_trim(repository);
    let Some(sep) = normalized.find('/') else {
        return false;
    };
    if sep == 0 || Some(sep) != normalized.rfind('/') {
        return false;
    }
    let owner = &normalized[..sep];
    let name = &normalized[sep + 1..];
    let name_units = name.encode_utf16().count();
    if name_units == 0 || name_units > 100 || name == "." || name == ".." {
        return false;
    }
    let mut oc = owner.chars();
    let owner_ok = match (oc.next(), oc.next_back()) {
        (Some(first), last) => {
            first.is_ascii_alphanumeric()
                && owner.len() <= 39
                && owner[1..]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-')
                && last.is_none_or(|c| c.is_ascii_alphanumeric())
        }
        _ => false,
    };
    owner_ok
        && !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// `normalizeGitHubPullRequestUrl` on a string input.
fn normalize_gh_pr_url(input: &str) -> Option<String> {
    let trimmed = js_trim(input);
    // ^https?://github\.com/([^/\s]+)/([^/\s]+)/pull/\d+(?:[/?#].*)?$/i
    let lower = trimmed.to_lowercase();
    let rest = lower
        .strip_prefix("https://github.com/")
        .or_else(|| lower.strip_prefix("http://github.com/"))?;
    // The regex is case-insensitive but captures from the original; the prefix is
    // ASCII so the byte offset maps back to `trimmed`.
    let orig_rest = &trimmed[trimmed.len() - rest.len()..];
    // owner/repo segments: [^/\s]+ where \s is the JS whitespace class.
    fn segment(s: &str) -> Option<(&str, &str)> {
        let end = s.find(|c: char| c == '/' || is_js_space(c))?;
        if end == 0 || !s[end..].starts_with('/') {
            return None;
        }
        Some((&s[..end], &s[end + 1..]))
    }
    let (owner_raw, after_owner) = segment(orig_rest)?;
    let (repo_raw, tail) = segment(after_owner)?;
    let owner = js_trim(owner_raw);
    let repository = js_trim(repo_raw);
    // /pull/\d+(?:[/?#].*)?$
    let tail_lower = tail.to_lowercase();
    let after_pull = tail_lower.strip_prefix("/pull/")?;
    let digits: String = after_pull
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let after_digits = &after_pull[digits.len()..];
    if digits.is_empty()
        || !(after_digits.is_empty()
            || after_digits.starts_with('/')
            || after_digits.starts_with('?')
            || after_digits.starts_with('#'))
    {
        return None;
    }
    let name_with_owner = format!("{owner}/{repository}");
    if !is_valid_gh_repo_name_with_owner(&name_with_owner) {
        return None;
    }
    // /pull/(\d+) — first occurrence in the original string
    let number = pr_number(trimmed)?;
    if number > 0.0 && number.fract() == 0.0 && number.abs() <= 9_007_199_254_740_991.0 {
        Some(format!(
            "https://github.com/{}/pull/{}",
            name_with_owner.to_lowercase(),
            number as i64
        ))
    } else {
        None
    }
}

/// First `/pull/(\d+)` (case-insensitive) capture as a JS number.
fn pr_number(s: &str) -> Option<f64> {
    let lower = s.to_lowercase();
    let mut at = 0;
    while let Some(pos) = lower[at..].find("/pull/") {
        let start = at + pos + 6;
        let digits: String = s[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return digits.parse::<f64>().ok();
        }
        at = start;
    }
    None
}

/// `pullRequestLinkKey` on an already-string url (never throws).
fn key_str(url: &str) -> String {
    if let Some(n) = normalize_gh_pr_url(url) {
        return n;
    }
    let t = js_trim(url);
    let t = &t[..t.find(['?', '#']).unwrap_or(t.len())];
    t.trim_end_matches('/').to_lowercase()
}

/// `pullRequestLinkKey` on any JS value; reproduces the TypeErrors of the two `.trim()` sites.
fn key_of(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(key_str(s)),
        Value::Null => Err("Cannot read properties of null (reading 'trim')".into()),
        _ if js::is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'trim')".into())
        }
        _ => Err("input.trim is not a function".into()),
    }
}

/// JS `===` for the JSON-transported values used here (objects compare by identity → false).
fn strict_eq(a: &Value, b: &Value) -> bool {
    if js::is_undefined(a) && js::is_undefined(b) {
        return true;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => match (js::as_number(a), js::as_number(b)) {
            (Some(x), Some(y)) => x == y, // -0 === 0, NaN !== NaN
            _ => match (a, b) {
                (Value::String(_), Value::String(_)) => {
                    match (js::string_units(a), js::string_units(b)) {
                        (Some(x), Some(y)) => x == y,
                        _ => a == b,
                    }
                }
                _ => false,
            },
        },
    }
}

/// `link.dismissedAt == null` (`isPullRequestLinkVisible`).
fn link_visible(link: &Value) -> Result<bool, String> {
    if link.is_null() {
        return Err("Cannot read properties of null (reading 'dismissedAt')".into());
    }
    if js::is_undefined(link) {
        return Err("Cannot read properties of undefined (reading 'dismissedAt')".into());
    }
    Ok(nullish(&member(link, "dismissedAt")))
}

/// `link.pullRequest` (throws on null/undefined links).
fn link_pr(link: &Value) -> Result<Value, String> {
    read_member(link, "pullRequest")
}

/// `link.pullRequest.state === "open"`.
fn link_is_open(link: &Value) -> Result<bool, String> {
    let pr = link_pr(link)?;
    let state = read_member(&pr, "state")?;
    Ok(matches!(&state, Value::String(s) if s == "open"))
}

/// `pullRequestLinkKey(link.pullRequest.url)` for a link inside a list.
fn link_key(link: &Value) -> Result<String, String> {
    let pr = link_pr(link)?;
    let url = read_member(&pr, "url")?;
    key_of(&url)
}

fn explicit_source(v: &Value) -> bool {
    matches!(v, Value::String(s) if matches!(s.as_str(), "created" | "agent" | "user" | "checkout"))
}

/// `Date.parse` — strict ISO-8601 only; every recorded `linkedAt` is garbage → NaN → 0.
fn date_parse(s: &str) -> f64 {
    let t = js_trim(s);
    let (date, time) = match t.find(['T', ' ']) {
        Some(i) => (&t[..i], Some(&t[i + 1..])),
        None => (t, None),
    };
    let mut it = date.split('-');
    let (year_s, month_s, day_s) = (it.next(), it.next(), it.next());
    if it.next().is_some() {
        return f64::NAN;
    }
    let year: i64 = match year_s {
        Some(y) if y.len() == 4 && y.bytes().all(|b| b.is_ascii_digit()) => y.parse().unwrap(),
        _ => return f64::NAN,
    };
    let month: i64 = match month_s {
        Some(m) if m.len() == 2 => match m.parse::<i64>() {
            Ok(m) if (1..=12).contains(&m) => m,
            _ => return f64::NAN,
        },
        None => 1,
        Some(_) => return f64::NAN,
    };
    let day: i64 = match day_s {
        Some(d) if d.len() == 2 => match d.parse::<i64>() {
            Ok(d) if (1..=31).contains(&d) => d,
            _ => return f64::NAN,
        },
        None => 1,
        Some(_) => return f64::NAN,
    };
    let mut h = 0i64;
    let mut mi = 0i64;
    let mut sec_ms = 0.0f64;
    let mut offset_min = 0i64;
    if let Some(ts) = time {
        let (clock, offset) = if let Some(off) = ts.strip_suffix(['Z', 'z']) {
            (off, 0)
        } else if let Some(i) = ts.find('+') {
            match tz_offset_minutes(&ts[i + 1..]) {
                Some(m) => (&ts[..i], m),
                None => return f64::NAN,
            }
        } else if let Some(i) = ts.rfind('-') {
            match tz_offset_minutes(&ts[i + 1..]) {
                Some(m) => (&ts[..i], -m),
                None => return f64::NAN,
            }
        } else {
            (ts, 0)
        };
        offset_min = offset;
        let mut parts = clock.split(':');
        h = match parts.next() {
            Some(x) if x.len() == 2 => match x.parse() {
                Ok(x) if x <= 24 => x,
                _ => return f64::NAN,
            },
            _ => return f64::NAN,
        };
        mi = match parts.next() {
            Some(x) if x.len() == 2 => match x.parse() {
                Ok(x) if x <= 59 => x,
                _ => return f64::NAN,
            },
            _ => return f64::NAN,
        };
        if let Some(sx) = parts.next() {
            if sx.len() < 2 || !sx[..2].bytes().all(|b| b.is_ascii_digit()) {
                return f64::NAN;
            }
            sec_ms = match sx.parse::<f64>() {
                Ok(x) if x < 60.0 => x * 1000.0,
                _ => return f64::NAN,
            };
        }
        if parts.next().is_some() {
            return f64::NAN;
        }
    }
    let days = days_from_civil(year, month, day);
    (days as f64) * 86_400_000.0 + ((h * 60 + mi - offset_min) * 60_000) as f64 + sec_ms
}

fn tz_offset_minutes(rest: &str) -> Option<i64> {
    if rest.is_empty() {
        return Some(0);
    }
    let (h, m) = if let Some((h, m)) = rest.split_once(':') {
        (h, m)
    } else {
        (rest, "0")
    };
    let h: i64 = h.parse().ok()?;
    let m: i64 = m.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn linked_at_ms(link: &Value) -> f64 {
    let raw = member(link, "linkedAt");
    let s = match &raw {
        Value::String(s) => s.clone(),
        _ => js::number_to_string(js::as_number(&raw).unwrap_or(f64::NAN)),
    };
    let parsed = date_parse(&s);
    if parsed.is_finite() { parsed } else { 0.0 }
}

/// Index of the link whose `pullRequest.url` matches `key` (the `findIndex` body).
fn index_of_key(items: &[Value], key: &str) -> Result<Option<usize>, String> {
    for (i, l) in items.iter().enumerate() {
        if link_key(l)? == key {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

fn find_index_of(items: &[Value], url: &Value) -> Result<Option<usize>, String> {
    let key = key_of(url)?;
    index_of_key(items, &key)
}

/// `replaceLinkAt(links, index, next)`.
fn replace_at(items: &[Value], index: usize, next: Value) -> Vec<Value> {
    items
        .iter()
        .enumerate()
        .map(|(i, l)| if i == index { next.clone() } else { l.clone() })
        .collect()
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.into(), v);
    }
    Value::Object(m)
}

/// Index of `resolveCurrentPullRequestLink`'s pick (`link !== current` is identity-based).
fn resolve_current_index(items: &[Value]) -> Result<Option<usize>, String> {
    let mut current: Option<usize> = None;
    for (i, link) in items.iter().enumerate() {
        if !link_visible(link)? {
            continue;
        }
        let Some(ci) = current else {
            current = Some(i);
            continue;
        };
        let is_open = link_is_open(link)?;
        let cur_open = link_is_open(&items[ci])?;
        if is_open != cur_open {
            if is_open {
                current = Some(i);
            }
            continue;
        }
        if linked_at_ms(link) >= linked_at_ms(&items[ci]) {
            current = Some(i);
        }
    }
    Ok(current)
}

fn cap_links(links: Vec<Value>) -> Result<Vec<Value>, String> {
    if links.len() <= LINK_LIMIT {
        return Ok(links);
    }
    let current = resolve_current_index(&links)?;
    let mut removable: Vec<(usize, i64, f64)> = Vec::with_capacity(links.len());
    for (i, link) in links.iter().enumerate() {
        if Some(i) == current {
            continue;
        }
        let rank = if !link_visible(link)? {
            0
        } else if !link_is_open(link)? {
            1
        } else {
            2
        };
        removable.push((i, rank, linked_at_ms(link)));
    }
    js::sort(&mut removable, |a, b| {
        let rank = (a.1 - b.1) as f64;
        if rank != 0.0 { rank } else { a.2 - b.2 }
    });
    let drop = links.len() - LINK_LIMIT;
    let removed: std::collections::HashSet<usize> =
        removable.iter().take(drop).map(|r| r.0).collect();
    Ok(links
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !removed.contains(i))
        .map(|(_, l)| l)
        .collect())
}

/// `links ?? []` spread into a Vec plus a trailing item.
fn with_appended(links: &[Value], extra: Value) -> Vec<Value> {
    let mut v = links.to_vec();
    v.push(extra);
    v
}

/// Port of `pullRequestLinkKey` (`packages/shared/src/threadPullRequestLinks.ts:29`).
pub fn pull_request_link_key(url: String) -> Result<String, String> {
    Ok(key_str(&url))
}

/// Port of `isExplicitPullRequestLinkSource` (`packages/shared/src/threadPullRequestLinks.ts:40`).
pub fn is_explicit_pull_request_link_source(source: serde_json::Value) -> Result<bool, String> {
    Ok(explicit_source(&source))
}

/// Port of `isPullRequestLinkVisible` (`packages/shared/src/threadPullRequestLinks.ts:46`).
pub fn is_pull_request_link_visible(link: serde_json::Value) -> Result<bool, String> {
    link_visible(&link)
}

/// Port of `visiblePullRequestLinks` (`packages/shared/src/threadPullRequestLinks.ts:50`).
pub fn visible_pull_request_links(
    links: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let items = links_items(&links, "(links ?? []).filter is not a function")?;
    let mut out = Vec::with_capacity(items.len());
    for l in items {
        if link_visible(l)? {
            out.push(l.clone());
        }
    }
    Ok(out)
}

/// Port of `findPullRequestLink` (`packages/shared/src/threadPullRequestLinks.ts:56`).
pub fn find_pull_request_link(
    links: serde_json::Value,
    url: String,
) -> Result<Option<serde_json::Value>, String> {
    let key = key_str(&url);
    let items = links_items(&links, "(links ?? []).find is not a function")?;
    match index_of_key(items, &key)? {
        Some(i) => Ok(Some(items[i].clone())),
        None => Ok(None),
    }
}

/// Port of `isPullRequestDismissed` (`packages/shared/src/threadPullRequestLinks.ts:65`).
pub fn is_pull_request_dismissed(links: serde_json::Value, url: String) -> Result<bool, String> {
    match find_pull_request_link(links, url)? {
        Some(l) => Ok(!nullish(&member(&l, "dismissedAt"))),
        None => Ok(false),
    }
}

/// Port of `compareLinkedPullRequests` (`packages/shared/src/threadPullRequestLinks.ts:78`).
pub fn compare_linked_pull_requests(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<f64, String> {
    let left_open = !link_is_open(&left)? as i64;
    let right_open = !link_is_open(&right)? as i64;
    if left_open != right_open {
        return Ok((left_open - right_open) as f64);
    }
    Ok(linked_at_ms(&right) - linked_at_ms(&left))
}

/// Port of `resolveCurrentPullRequestLink` (`packages/shared/src/threadPullRequestLinks.ts:92`).
pub fn resolve_current_pull_request_link(
    links: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    // `for ... of` iterates strings by code point.
    let items: Vec<Value> = if nullish(&links) {
        Vec::new()
    } else if let Value::String(s) = &links {
        s.chars().map(|c| Value::String(c.to_string())).collect()
    } else if let Value::Array(a) = &links {
        a.clone()
    } else {
        return Err("(links ?? []) is not iterable".into());
    };
    match resolve_current_index(&items)? {
        Some(i) => Ok(Some(items[i].clone())),
        None => Ok(None),
    }
}

/// Port of `resolveCurrentPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:113`).
pub fn resolve_current_pull_request(
    links: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let Some(link) = resolve_current_pull_request_link(links)? else {
        return Ok(None);
    };
    let pr = member(&link, "pullRequest");
    Ok(if nullish(&pr) { None } else { Some(pr) })
}

/// Port of `resolveBranchManagedPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:120`).
pub fn resolve_branch_managed_pull_request(
    links: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let items = links_items(&links, "(links ?? []).filter is not a function")?;
    let mut kept = Vec::with_capacity(items.len());
    for l in items {
        if !explicit_source(&read_member(l, "source")?) {
            kept.push(l.clone());
        }
    }
    resolve_current_pull_request(Value::Array(kept))
}

/// Port of `legacyPullRequestLinks` (`packages/shared/src/threadPullRequestLinks.ts:129`).
pub fn legacy_pull_request_links(
    pull_request: serde_json::Value,
    linked_at: String,
) -> Result<Vec<serde_json::Value>, String> {
    if truthy(&pull_request) {
        Ok(vec![obj(vec![
            ("pullRequest", pull_request),
            ("source", Value::from("legacy")),
            ("linkedAt", Value::from(linked_at)),
        ])])
    } else {
        Ok(Vec::new())
    }
}

/// Port of `pullRequestLinksFromEventPayload` (`packages/shared/src/threadPullRequestLinks.ts:141`).
pub fn pull_request_links_from_event_payload(
    payload: serde_json::Value,
    occurred_at: String,
) -> Result<serde_json::Value, String> {
    if payload.is_null() {
        return Err("Cannot read properties of null (reading 'pullRequests')".into());
    }
    if js::is_undefined(&payload) {
        return Err("Cannot read properties of undefined (reading 'pullRequests')".into());
    }
    let pull_requests = member(&payload, "pullRequests");
    if !js::is_undefined(&pull_requests) {
        return match pull_requests {
            Value::Array(a) => Ok(Value::Array(a)),
            Value::String(s) => Ok(Value::Array(
                s.chars().map(|c| Value::String(c.to_string())).collect(),
            )),
            _ => Err("payload.pullRequests is not iterable".into()),
        };
    }
    let last_known = member(&payload, "lastKnownPr");
    if !js::is_undefined(&last_known) {
        return Ok(Value::Array(legacy_pull_request_links(
            last_known,
            occurred_at,
        )?));
    }
    Ok(js::undefined())
}

/// Port of `linkPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:190`).
pub fn link_pull_request(
    links: serde_json::Value,
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let pr = read_member(&input, "pullRequest")?;
    let url = read_member(&pr, "url")?;
    // `indexOfPullRequestLink` runs `pullRequestLinkKey(url)` before `links.findIndex`.
    let key = key_of(&url)?;
    let items = links_items(&links, "links.findIndex is not a function")?;
    let index = index_of_key(items, &key)?;
    let fresh = obj(vec![
        ("pullRequest", pr.clone()),
        ("source", member(&input, "source")),
        ("linkedAt", member(&input, "linkedAt")),
    ]);
    let Some(index) = index else {
        return cap_links(with_appended(items, fresh));
    };
    let existing = &items[index];
    if !link_visible(existing)? {
        return Ok(replace_at(items, index, fresh));
    }
    let source = if explicit_source(&member(existing, "source")) {
        member(existing, "source")
    } else {
        member(&input, "source")
    };
    let next = obj(vec![
        ("pullRequest", pr),
        ("source", source),
        ("linkedAt", member(existing, "linkedAt")),
    ]);
    Ok(replace_at(items, index, next))
}

/// Port of `unlinkPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:216`).
pub fn unlink_pull_request(
    links: serde_json::Value,
    url: String,
    dismissed_at: String,
) -> Result<Option<Vec<serde_json::Value>>, String> {
    let items = links_items(&links, "links.findIndex is not a function")?;
    let key = key_str(&url);
    let Some(index) = index_of_key(items, &key)? else {
        return Ok(None);
    };
    let existing = &items[index];
    if !link_visible(existing)? {
        return Ok(None);
    }
    let mut next = match existing {
        Value::Object(o) => o.clone(),
        _ => Map::new(),
    };
    next.insert("dismissedAt".into(), Value::from(dismissed_at));
    Ok(Some(replace_at(items, index, Value::Object(next))))
}

/// Port of `syncDiscoveredPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:233`).
pub fn sync_discovered_pull_request(
    links: serde_json::Value,
    pull_request: serde_json::Value,
    observed_at: String,
) -> Result<Vec<serde_json::Value>, String> {
    let key = if truthy(&pull_request) {
        Some(key_of(&member(&pull_request, "url"))?)
    } else {
        None
    };
    let items = links_items(&links, "(links ?? []).filter is not a function")?;
    let mut kept: Vec<Value> = Vec::with_capacity(items.len());
    for l in items {
        if !link_visible(l)? || explicit_source(&member(l, "source")) {
            kept.push(l.clone());
            continue;
        }
        let lk = link_key(l)?;
        if Some(lk) == key {
            kept.push(l.clone());
        }
    }
    if pull_request.is_null() {
        return Ok(kept);
    }
    let url = read_member(&pull_request, "url")?;
    let Some(index) = find_index_of(&kept, &url)? else {
        return cap_links(with_appended(
            &kept,
            obj(vec![
                ("pullRequest", pull_request),
                ("source", Value::from("discovered")),
                ("linkedAt", Value::from(observed_at)),
            ]),
        ));
    };
    let existing = &kept[index];
    if !link_visible(existing)? {
        return Ok(kept);
    }
    let mut next = match existing {
        Value::Object(o) => o.clone(),
        _ => Map::new(),
    };
    next.insert("pullRequest".into(), pull_request);
    Ok(replace_at(&kept, index, Value::Object(next)))
}

/// Port of `refreshPullRequestSnapshots` (`packages/shared/src/threadPullRequestLinks.ts:259`).
pub fn refresh_pull_request_snapshots(
    links: serde_json::Value,
    pull_requests: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let prs: &[Value] = match &pull_requests {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'map')".into());
        }
        _ if js::is_undefined(&pull_requests) => {
            return Err("Cannot read properties of undefined (reading 'map')".into());
        }
        Value::Array(a) => a,
        _ => return Err("pullRequests.map is not a function".into()),
    };
    let mut by_key: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    for pr in prs {
        let url = read_member(pr, "url")?;
        by_key.insert(key_of(&url)?, pr.clone());
    }
    let items = links_items(&links, "(links ?? []).map is not a function")?;
    let mut out = Vec::with_capacity(items.len());
    for link in items {
        if !link_visible(link)? {
            out.push(link.clone());
            continue;
        }
        let key = link_key(link)?;
        match by_key.get(&key) {
            Some(refreshed) => {
                let mut next = match link {
                    Value::Object(o) => o.clone(),
                    _ => Map::new(),
                };
                next.insert("pullRequest".into(), refreshed.clone());
                out.push(Value::Object(next));
            }
            None => out.push(link.clone()),
        }
    }
    Ok(out)
}

/// Port of `threadPullRequestsEqual` (`packages/shared/src/threadPullRequestLinks.ts:271`).
pub fn thread_pull_requests_equal(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<bool, String> {
    if strict_eq(&left, &right) {
        return Ok(true);
    }
    if nullish(&left) || nullish(&right) {
        return Ok(false);
    }
    let field = |l: &Value, r: &Value, k: &str| strict_eq(&member(l, k), &member(r, k));
    let with_default = |l: &Value, r: &Value, k: &str, default: Value| {
        let lv = member(l, k);
        let rv = member(r, k);
        strict_eq(
            if nullish(&lv) { &default } else { &lv },
            if nullish(&rv) { &default } else { &rv },
        )
    };
    Ok(field(&left, &right, "number")
        && field(&left, &right, "title")
        && field(&left, &right, "url")
        && field(&left, &right, "baseBranch")
        && field(&left, &right, "headBranch")
        && field(&left, &right, "state")
        && with_default(&left, &right, "isDraft", Value::Bool(false))
        && with_default(&left, &right, "mergeability", Value::from("unknown"))
        && with_default(&left, &right, "additions", Value::Null)
        && with_default(&left, &right, "deletions", Value::Null)
        && with_default(&left, &right, "changedFiles", Value::Null))
}

/// Port of `pullRequestLinksEqual` (`packages/shared/src/threadPullRequestLinks.ts:292`).
pub fn pull_request_links_equal(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<bool, String> {
    // `.length` on `left ?? []`: arrays → length, strings → UTF-16 units, else undefined.
    let js_len = |v: &Value| -> Option<f64> {
        match v {
            _ if nullish(v) => Some(0.0),
            Value::Array(a) => Some(a.len() as f64),
            Value::String(s) => Some(s.encode_utf16().count() as f64),
            _ => None,
        }
    };
    if js_len(&left) != js_len(&right) {
        return Ok(false);
    }
    let la: Vec<Value> = match &left {
        _ if nullish(&left) => Vec::new(),
        Value::Array(a) => a.clone(),
        _ => return Err("leftLinks.every is not a function".into()),
    };
    for (i, link) in la.iter().enumerate() {
        let other = match &right {
            Value::Array(a) => a.get(i).cloned().unwrap_or_else(js::undefined),
            Value::String(s) => s
                .chars()
                .nth(i)
                .map(|c| Value::String(c.to_string()))
                .unwrap_or_else(js::undefined),
            Value::Object(o) => o
                .get(i.to_string().as_str())
                .cloned()
                .unwrap_or_else(js::undefined),
            _ => js::undefined(),
        };
        if !strict_eq(
            &read_member(link, "source")?,
            &read_member(&other, "source")?,
        ) {
            return Ok(false);
        }
        if !strict_eq(
            &read_member(link, "linkedAt")?,
            &read_member(&other, "linkedAt")?,
        ) {
            return Ok(false);
        }
        let ld = read_member(link, "dismissedAt")?;
        let rd = read_member(&other, "dismissedAt")?;
        let ld = if nullish(&ld) { Value::Null } else { ld };
        let rd = if nullish(&rd) { Value::Null } else { rd };
        if !strict_eq(&ld, &rd) {
            return Ok(false);
        }
        if !thread_pull_requests_equal(
            read_member(link, "pullRequest")?,
            read_member(&other, "pullRequest")?,
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}
