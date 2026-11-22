//! The Rust port of `apps/web/src/lib/projectCatalog/`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn prop(v: &Value, name: &str) -> Value {
    v.get(name).cloned().unwrap_or(Value::Null)
}

fn marker(v: &Value) -> bool {
    v.get("$undefined").and_then(Value::as_bool) == Some(true)
}

fn key_part(v: &Value) -> Value {
    if marker(v) { Value::Null } else { v.clone() }
}

/// `checkoutKey`: null reads throw in JS (`ref.environmentId`).
fn checkout_key_of(ref_: &Value) -> Result<String, String> {
    if ref_.is_null() {
        return Err("Cannot read properties of null (reading 'environmentId')".to_string());
    }
    Ok(serde_json::to_string(&json!([
        key_part(&prop(ref_, "environmentId")),
        key_part(&prop(ref_, "projectId"))
    ]))
    .unwrap_or_default())
}

/// JS `Array.prototype.sort`/`toSorted` compares strings by UTF-16 code
/// units: supplementary chars sort by their high surrogate (0xD800–0xDBFF).
fn utf16_less(a: &str, b: &str) -> bool {
    a.encode_utf16().lt(b.encode_utf16())
}

fn pair_key(a: &Value, b: &Value) -> Result<String, String> {
    let mut parts = [checkout_key_of(a)?, checkout_key_of(b)?];
    if utf16_less(&parts[1].clone(), &parts[0]) {
        parts.swap(0, 1);
    }
    Ok(serde_json::to_string(&json!(parts)).unwrap_or_default())
}

/// `{...value}`: objects spread their entries, strings spread per code point,
/// arrays per index, everything else spreads to `{}`.
fn js_spread_object(v: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    match v {
        Value::Object(m) => out = m.clone(),
        Value::String(s) => {
            for (i, c) in s.chars().enumerate() {
                out.insert(i.to_string(), json!(c.to_string()));
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                out.insert(i.to_string(), item.clone());
            }
        }
        _ => {}
    }
    out
}

/// Port of `emptyCatalog` (`apps/web/src/lib/projectCatalog/model.ts:34`).
pub fn empty_catalog() -> Result<serde_json::Value, String> {
    Ok(json!({
        "version": 1,
        "checkouts": [],
        "groups": [],
        "excludedPairs": [],
    }))
}

/// Port of `checkoutKey` (`apps/web/src/lib/projectCatalog/model.ts:40`).
pub fn checkout_key(r#ref: serde_json::Value) -> Result<String, String> {
    checkout_key_of(&r#ref)
}

fn pct_encode(byte: u8) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut s = String::with_capacity(3);
    s.push('%');
    s.push(HEX[(byte >> 4) as usize] as char);
    s.push(HEX[(byte & 15) as usize] as char);
    s
}

/// WHATWG opaque-host serialization for non-special schemes (ssh:, git:):
/// ASCII in the host range is kept, everything else is UTF-8 percent-encoded.
fn opaque_host_encode(host: &str) -> String {
    let mut out = String::with_capacity(host.len());
    for b in host.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'!') {
            out.push(b as char);
        } else {
            out.push_str(&pct_encode(b));
        }
    }
    out
}

/// Port of `repositoryIdentity` (`apps/web/src/lib/projectCatalog/model.ts:46`).
pub fn repository_identity(raw: String) -> Result<Option<String>, String> {
    // `/(?:^|\/)\.\.?(?:\/|$)/` — any "." or ".." path segment.
    let segments: Vec<&str> = raw.split('/').collect();
    if segments.iter().any(|s| *s == "." || *s == "..") || raw.contains('%') || raw.contains('\\') {
        return Ok(None);
    }
    // WHATWG removes ASCII tab and newline before parsing.
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    // Scp-like syntax `[^/@:]+@?host:path` upgrades to an ssh:// URL.
    let candidate = if !cleaned.contains("://") {
        if let Some(colon) = cleaned.find(':') {
            let host_part = &cleaned[..colon];
            let rest = &cleaned[colon + 1..];
            if !host_part.contains('/')
                && !host_part.is_empty()
                && !rest.starts_with('/')
                && !rest.is_empty()
            {
                // `([^/@:]+@)?` userinfo must not contain '/', '@' or ':'.
                let (userinfo, host) = match host_part.rfind('@') {
                    Some(at) => (&host_part[..at + 1], &host_part[at + 1..]),
                    None => ("", host_part),
                };
                if userinfo.contains('/') || userinfo.contains(':') || host.is_empty() {
                    cleaned.clone()
                } else {
                    format!("ssh://{userinfo}{host}/{rest}")
                }
            } else {
                cleaned.clone()
            }
        } else {
            cleaned.clone()
        }
    } else {
        cleaned.clone()
    };

    // Minimal URL parser: scheme://authority/path?search#hash
    let (scheme, rest) = match candidate.split_once("://") {
        Some((s, r)) => (format!("{s}:"), r),
        None => return Ok(None),
    };
    if !matches!(scheme.as_str(), "ssh:" | "https:" | "http:" | "git:") {
        return Ok(None);
    }
    let (authority, mut path_and_rest) = match rest.split_once('/') {
        Some((a, p)) => (a, format!("/{p}")),
        None => (rest, String::new()),
    };
    // Any search or hash present in the URL fails the check.
    if path_and_rest.contains('?') || path_and_rest.contains('#') {
        return Ok(None);
    }
    if let Some(i) = path_and_rest.find('#') {
        path_and_rest.truncate(i);
    }

    // authority = [userinfo@]host[:port]
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    let (hostname, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => (h, p),
        Some((h, "")) => (h, ""),
        _ => (hostport, ""),
    };
    if hostname.is_empty() {
        return Ok(None);
    }
    // WHATWG forbidden host code points make the URL parse throw.
    if hostname.bytes().any(|b| {
        b < 0x20
            || matches!(
                b,
                b' ' | b'"'
                    | b'#'
                    | b'/'
                    | b'<'
                    | b'>'
                    | b'?'
                    | b'@'
                    | b'['
                    | b'\\'
                    | b']'
                    | b'^'
                    | b'|'
            )
    }) {
        return Ok(None);
    }
    // `url.hostname.toLowerCase()`: percent-encoded opaque host text,
    // compared lowercase for `known`; `url.host` keeps the encoded casing.
    let hostname_lc = opaque_host_encode(hostname).to_lowercase();
    let host_encoded = opaque_host_encode(hostname);
    let known = matches!(
        hostname_lc.as_str(),
        "github.com" | "gitlab.com" | "bitbucket.org"
    );
    let has_port = !port.is_empty();

    // WHATWG percent-encodes the path (non-ASCII and a few ASCII chars).
    let mut path = String::with_capacity(path_and_rest.len());
    for b in path_and_rest.bytes() {
        if b.is_ascii() && b >= 0x20 && !matches!(b, b' ' | b'"' | b'<' | b'>' | b'`' | b'{' | b'}')
        {
            path.push(b as char);
        } else {
            path.push_str(&pct_encode(b));
        }
    }
    if known && !has_port {
        while path.ends_with('/') && path.len() > 1 {
            path.pop();
        }
        if let Some(stripped) = path.strip_suffix(".git") {
            path = stripped.to_string();
        }
    }
    if path.is_empty() || path == "/" || path.contains('%') || path.contains("/../") {
        return Ok(None);
    }
    if known && !has_port {
        let parts: Vec<&str> = path[1..].split('/').collect();
        if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
            return Ok(None);
        }
        let final_path = if hostname_lc == "github.com" {
            path.to_lowercase()
        } else {
            path
        };
        return Ok(Some(format!("{hostname_lc}{final_path}")));
    }
    let host = if has_port {
        format!("{host_encoded}:{port}")
    } else {
        host_encoded
    };
    Ok(Some(format!("{scheme}//{host}{path}")))
}

/// Port of `observeCheckouts` (`apps/web/src/lib/projectCatalog/model.ts:77`).
pub fn observe_checkouts(
    catalog: serde_json::Value,
    environment_id: String,
    incoming: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // `new Map(incoming.map(...))` evaluates first.
    let incoming: Vec<Value> = match incoming {
        Value::Null => return Err("Cannot read properties of null (reading 'map')".to_string()),
        Value::Array(a) => a,
        _ => return Err("incoming.map is not a function".to_string()),
    };
    if catalog.is_null() {
        return Err("Cannot read properties of null (reading 'checkouts')".to_string());
    }
    let previous_list: Vec<Value> = match catalog.get("checkouts") {
        None => return Err("Cannot read properties of undefined (reading 'map')".to_string()),
        Some(Value::Null) => {
            return Err("Cannot read properties of null (reading 'map')".to_string());
        }
        Some(Value::Array(a)) => a.clone(),
        Some(_) => return Err("catalog.checkouts.map is not a function".to_string()),
    };
    // Map: duplicate keys keep the first position with the last value.
    let mut by_key: Vec<(String, usize)> = Vec::new();
    for (i, c) in incoming.iter().enumerate() {
        let k = checkout_key_of(c)?;
        match by_key.iter().position(|(ek, _)| *ek == k) {
            Some(pos) => by_key[pos].1 = i,
            None => by_key.push((k, i)),
        }
    }
    let mut checkouts = Vec::new();
    for previous in previous_list {
        let key = checkout_key_of(&previous)?;
        if let Some(pos) = by_key.iter().position(|(k, _)| *k == key) {
            let (_, idx) = by_key.remove(pos);
            let next = &incoming[idx];
            let same_cwd = prop(&previous, "cwd") == prop(next, "cwd");
            let mut spread = js_spread_object(next);
            spread.insert(
                "repositoryUrls".to_string(),
                if same_cwd {
                    prop(&previous, "repositoryUrls")
                } else {
                    json!([])
                },
            );
            checkouts.push(Value::Object(spread));
        } else if prop(&previous, "environmentId") == Value::String(environment_id.clone()) {
            let mut prev = previous;
            if let Value::Object(ref mut m) = prev {
                m.insert("missing".to_string(), json!(true));
            }
            checkouts.push(prev);
        } else {
            checkouts.push(previous);
        }
    }
    // `[...checkouts, ...byKey.values()]`: deduplicated entries only.
    for (_, idx) in by_key {
        checkouts.push(incoming[idx].clone());
    }
    let mut out = catalog;
    if let Value::Object(ref mut m) = out {
        m.insert("checkouts".to_string(), json!(checkouts));
    }
    Ok(out)
}

/// Port of `catalogSuggestions` (`apps/web/src/lib/projectCatalog/model.ts:92`).
pub fn catalog_suggestions(catalog: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    if catalog.is_null() {
        return Err("Cannot read properties of null (reading 'checkouts')".to_string());
    }
    let empty = Vec::new();
    let checkouts = match catalog.get("checkouts") {
        None => {
            return Err("Cannot read properties of undefined (reading 'filter')".to_string());
        }
        Some(Value::Null) => {
            return Err("Cannot read properties of null (reading 'filter')".to_string());
        }
        Some(Value::Array(a)) => a,
        Some(_) => return Err("catalog.checkouts.filter is not a function".to_string()),
    };
    let groups = catalog
        .get("groups")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let excluded = catalog
        .get("excludedPairs")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let eligible: Vec<&Value> = checkouts
        .iter()
        .filter(|item| {
            prop(item, "missing").as_bool() != Some(true)
                && prop(item, "repositoryUrls").as_array().map_or(0, Vec::len) == 1
        })
        .collect();
    let mut suggestions = Vec::new();
    for i in 0..eligible.len() {
        for j in (i + 1)..eligible.len() {
            let a = eligible[i];
            let b = eligible[j];
            let url_a = prop(a, "repositoryUrls")
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let identity = repository_identity(url_a)?;
            let url_b = prop(b, "repositoryUrls")
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let same = match (&identity, repository_identity(url_b)?) {
                (Some(x), Some(y)) => *x == y,
                _ => false,
            };
            if identity.is_none() || !same {
                continue;
            }
            let pk = pair_key(a, b)?;
            if excluded.iter().any(|e| e.as_str() == Some(pk.as_str())) {
                continue;
            }
            let key_a = checkout_key_of(a)?;
            let key_b = checkout_key_of(b)?;
            let already = groups.iter().any(|group| {
                group
                    .get("members")
                    .and_then(Value::as_array)
                    .is_some_and(|ms| {
                        let has_a = ms
                            .iter()
                            .any(|r| checkout_key_of(r).ok().as_deref() == Some(key_a.as_str()));
                        let has_b = ms
                            .iter()
                            .any(|r| checkout_key_of(r).ok().as_deref() == Some(key_b.as_str()));
                        has_a && has_b
                    })
            });
            if already {
                continue;
            }
            suggestions.push(json!([a, b]));
        }
    }
    Ok(suggestions)
}

/// Port of `excludeSuggestion` (`apps/web/src/lib/projectCatalog/model.ts:117`).
pub fn exclude_suggestion(
    catalog: serde_json::Value,
    a: serde_json::Value,
    b: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // `[...catalog.excludedPairs, pairKey(a, b)]` iterates excludedPairs first.
    if catalog.is_null() {
        return Err("Cannot read properties of null (reading 'excludedPairs')".to_string());
    }
    let mut pairs = match catalog.get("excludedPairs") {
        None => {
            return Err("catalog.excludedPairs is not iterable".to_string());
        }
        Some(Value::Array(p)) => p.clone(),
        Some(_) => return Err("catalog.excludedPairs is not iterable".to_string()),
    };
    // `new Set([...excludedPairs, pairKey])` dedupes the whole list.
    pairs.push(json!(pair_key(&a, &b)?));
    let mut seen = std::collections::HashSet::new();
    pairs.retain(|p| seen.insert(p.to_string()));
    let mut out = catalog;
    if let Value::Object(ref mut m) = out {
        m.insert("excludedPairs".to_string(), json!(pairs));
    }
    Ok(out)
}

/// Rebuild a group's member list minus the removed refs, keeping a still-member
/// preferredCheckout and dropping empty groups.
fn strip_members(group: &Value, selected: &[String], remove_ref: Option<&Value>) -> Option<Value> {
    let empty = Vec::new();
    let members = group
        .get("members")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let keep: Vec<Value> = members
        .iter()
        .filter(|r| {
            let key = checkout_key_of(r).unwrap_or_default();
            !selected.contains(&key)
                && remove_ref.is_none_or(|rr| checkout_key_of(rr).ok() != Some(key.clone()))
        })
        .cloned()
        .collect();
    if keep.is_empty() {
        return None;
    }
    let mut g = group.clone();
    let Value::Object(ref mut m) = g else {
        return None;
    };
    let preferred = m.remove("preferredCheckout");
    m.insert("members".to_string(), json!(keep.clone()));
    if let Some(preferred) = preferred {
        let pref_key = checkout_key_of(&preferred).unwrap_or_default();
        let still_member = keep
            .iter()
            .any(|r| checkout_key_of(r).unwrap_or_default() == pref_key)
            && remove_ref.is_none_or(|rr| checkout_key_of(rr).unwrap_or_default() != pref_key);
        if still_member {
            m.insert("preferredCheckout".to_string(), preferred);
        }
    }
    Some(g)
}

/// Port of `linkCheckouts` (`apps/web/src/lib/projectCatalog/model.ts:123`).
pub fn link_checkouts(
    catalog: serde_json::Value,
    refs: serde_json::Value,
    name: String,
    id: String,
) -> Result<serde_json::Value, String> {
    let refs: Vec<Value> = match refs {
        Value::Null => return Err("Cannot read properties of null (reading 'map')".to_string()),
        Value::Array(a) => a,
        _ => return Err("refs.map is not a function".to_string()),
    };
    let mut seen = Vec::<String>::new();
    let mut unique = Vec::<Value>::new();
    for r in &refs {
        let k = checkout_key_of(r)?;
        if !seen.contains(&k) {
            seen.push(k);
            unique.push(r.clone());
        }
    }
    if unique.len() < 2 || name.trim().is_empty() {
        return Err("Select at least two known checkouts and a group name.".to_string());
    }
    if catalog.is_null() {
        return Err("Cannot read properties of null (reading 'checkouts')".to_string());
    }
    let checkouts = match catalog.get("checkouts") {
        None => {
            return Err("Cannot read properties of undefined (reading 'some')".to_string());
        }
        Some(Value::Null) => {
            return Err("Cannot read properties of null (reading 'some')".to_string());
        }
        Some(Value::Array(a)) => a.clone(),
        Some(_) => return Err("catalog.checkouts.some is not a function".to_string()),
    };
    let known = |k: &str| {
        checkouts
            .iter()
            .any(|c| checkout_key_of(c).ok().as_deref() == Some(k))
    };
    if seen.iter().any(|k| !known(k)) {
        return Err("Select at least two known checkouts and a group name.".to_string());
    }
    let mut out = catalog;
    let Value::Object(ref mut m) = out else {
        return Ok(out);
    };
    let groups = m
        .get("groups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut next_groups: Vec<Value> = groups
        .iter()
        .filter_map(|g| strip_members(g, &seen, None))
        .collect();
    let members: Vec<Value> = unique
        .iter()
        .map(|r| {
            json!({
                "environmentId": key_part(&prop(r, "environmentId")),
                "projectId": key_part(&prop(r, "projectId")),
            })
        })
        .collect();
    next_groups.push(json!({
        "id": id,
        "name": name.trim(),
        "appearance": Value::Null,
        "members": members,
    }));
    m.insert("groups".to_string(), json!(next_groups));
    Ok(out)
}

/// Port of `unlinkCheckout` (`apps/web/src/lib/projectCatalog/model.ts:162`).
pub fn unlink_checkout(
    catalog: serde_json::Value,
    group_id: String,
    r#ref: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if catalog.is_null() {
        return Err("Cannot read properties of null (reading 'groups')".to_string());
    }
    let others: Vec<Value> = match catalog.get("groups") {
        None => {
            return Err("Cannot read properties of undefined (reading 'find')".to_string());
        }
        Some(Value::Null) => {
            return Err("Cannot read properties of null (reading 'find')".to_string());
        }
        Some(Value::Array(gs)) => gs
            .iter()
            .find(|g| prop(g, "id") == Value::String(group_id.clone()))
            .and_then(|g| g.get("members").and_then(Value::as_array).cloned())
            .unwrap_or_default(),
        Some(_) => return Err("catalog.groups.find is not a function".to_string()),
    };
    let mut next = catalog;
    for other in others {
        if checkout_key_of(&other)? != checkout_key_of(&r#ref)? {
            next = exclude_suggestion(next, r#ref.clone(), other)?;
        }
    }
    let Value::Object(ref mut m) = next else {
        return Ok(next);
    };
    let groups = m
        .get("groups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let next_groups: Vec<Value> = groups
        .iter()
        .filter_map(|g| {
            if prop(g, "id") != Value::String(group_id.clone()) {
                Some(g.clone())
            } else {
                strip_members(g, &[], Some(&r#ref))
            }
        })
        .collect();
    m.insert("groups".to_string(), json!(next_groups));
    Ok(next)
}

/// Port of `splitGroup` (`apps/web/src/lib/projectCatalog/model.ts:187`).
pub fn split_group(
    catalog: serde_json::Value,
    group_id: String,
) -> Result<serde_json::Value, String> {
    if catalog.is_null() {
        return Err("Cannot read properties of null (reading 'groups')".to_string());
    }
    let refs: Vec<Value> = match catalog.get("groups") {
        None => {
            return Err("Cannot read properties of undefined (reading 'find')".to_string());
        }
        Some(Value::Null) => {
            return Err("Cannot read properties of null (reading 'find')".to_string());
        }
        Some(Value::Array(gs)) => gs
            .iter()
            .find(|g| prop(g, "id") == Value::String(group_id.clone()))
            .and_then(|g| g.get("members").and_then(Value::as_array).cloned())
            .unwrap_or_default(),
        Some(_) => return Err("catalog.groups.find is not a function".to_string()),
    };
    let mut next = catalog;
    for r in refs {
        next = unlink_checkout(next, group_id.clone(), r)?;
    }
    Ok(next)
}

/// Port of `openCatalogCheckout` (`apps/web/src/lib/projectCatalog/navigation.ts:14`).
pub async fn open_catalog_checkout(checkout: serde_json::Value) -> Result<(), String> {
    let _ = checkout;
    // No execution context exists in this runtime: `readExecutionContext()`
    // yields null, so the first guard always throws.
    Err("This checkout is unavailable.".to_string())
}

/// Port of `projectCatalogKey` (`apps/web/src/lib/projectCatalog/storage.ts:5`).
pub fn project_catalog_key() -> Result<Option<String>, String> {
    Ok(None)
}

/// Port of `readProjectCatalog` (`apps/web/src/lib/projectCatalog/storage.ts:16`).
pub fn read_project_catalog(key: String) -> Result<serde_json::Value, String> {
    let _ = key;
    Err("localStorage is not defined".to_string())
}

/// Port of `saveProjectCatalog` (`apps/web/src/lib/projectCatalog/storage.ts:53`).
pub fn save_project_catalog(key: String, value: serde_json::Value) -> Result<(), String> {
    let _ = (key, value);
    Err("The account changed. Reopen the project catalog.".to_string())
}
