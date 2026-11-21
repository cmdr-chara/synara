//! The Rust port of `packages/shared/src/git.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::{js, utf16};

const WORKTREE_BRANCH_PREFIX: &str = "synara";
const PRE_CUTOVER_WORKTREE_NAMESPACE_HASHES: [u64; 2] = [0x8559131d2c062cf0, 0xd53b4ab95395e345];

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

/// FNV-1a 64 over UTF-16 code units (`charCodeAt`), with `asUintN(64)` wraparound.
fn hash_worktree_namespace(value: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for unit in utf16::units(value) {
        hash ^= u64::from(unit);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn is_pre_cutover_worktree_namespace(value: &str) -> bool {
    PRE_CUTOVER_WORKTREE_NAMESPACE_HASHES.contains(&hash_worktree_namespace(value))
}

/// `.slice(0, 64)` on UTF-16 units.
fn utf16_first(s: &str, n: usize) -> String {
    let u = utf16::units(s);
    utf16::slice(&u, 0, n.min(u.len()))
}

/// Port of `sanitizeBranchFragment` (`packages/shared/src/git.ts:24`).
pub fn sanitize_branch_fragment(raw: String) -> Result<String, String> {
    // .trim().toLowerCase().replace(/['"`]/g, "")
    let lower: String = js_trim(&raw).to_lowercase().chars().collect();
    let no_quotes: String = lower
        .chars()
        .filter(|c| !matches!(c, '\'' | '"' | '`'))
        .collect();
    // /[./\s_-]+/ edge trim
    let edge = |c: char| matches!(c, '.' | '/' | '_' | '-') || is_js_space(c);
    let normalized = no_quotes.trim_matches(edge);

    // /[^a-z0-9/_-]+/g -> "-"
    let mut collapsed = String::with_capacity(normalized.len());
    let mut in_bad = false;
    for c in normalized.chars() {
        if matches!(c, 'a'..='z' | '0'..='9' | '/' | '_' | '-') {
            collapsed.push(c);
            in_bad = false;
        } else if !in_bad {
            collapsed.push('-');
            in_bad = true;
        }
    }
    // /\/+/g -> "/"
    let mut single_slash = String::with_capacity(collapsed.len());
    let mut prev_slash = false;
    for c in collapsed.chars() {
        if c == '/' {
            if !prev_slash {
                single_slash.push(c);
            }
            prev_slash = true;
        } else {
            single_slash.push(c);
            prev_slash = false;
        }
    }
    // /-+/g -> "-"
    let mut single_dash = String::with_capacity(single_slash.len());
    let mut prev_dash = false;
    for c in single_slash.chars() {
        if c == '-' {
            if !prev_dash {
                single_dash.push(c);
            }
            prev_dash = true;
        } else {
            single_dash.push(c);
            prev_dash = false;
        }
    }
    // /^[./_-]+|[./_-]+$/g, then slice(0,64), then trailing trim again
    let edge2 = |c: char| matches!(c, '.' | '/' | '_' | '-');
    let inner = single_dash.trim_matches(edge2);
    let sliced = utf16_first(inner, 64);
    let fragment = sliced.trim_end_matches(edge2);

    Ok(if fragment.is_empty() {
        "update".to_string()
    } else {
        fragment.to_string()
    })
}

/// Port of `sanitizeFeatureBranchName` (`packages/shared/src/git.ts:46`).
pub fn sanitize_feature_branch_name(raw: String) -> Result<String, String> {
    let sanitized = sanitize_branch_fragment(raw)?;
    if sanitized.contains('/') {
        return Ok(if sanitized.starts_with("feature/") {
            sanitized
        } else {
            format!("feature/{sanitized}")
        });
    }
    Ok(format!("feature/{sanitized}"))
}

fn conflict_component(name: &str, existing: &std::collections::HashSet<String>) -> Option<usize> {
    let components: Vec<&str> = name.split('/').collect();
    for index in 0..components.len() {
        if existing.contains(&components[..=index].join("/")) {
            return Some(index);
        }
    }
    let prefix = format!("{name}/");
    if existing.iter().any(|b| b.starts_with(&prefix)) {
        return Some(components.len() - 1);
    }
    None
}

fn resolve_unique_branch_name(existing_branch_names: &[String], resolved_base: &str) -> String {
    let existing: std::collections::HashSet<String> = existing_branch_names
        .iter()
        .map(|b| b.to_lowercase())
        .collect();

    let mut component = match conflict_component(resolved_base, &existing) {
        None => return resolved_base.to_string(),
        Some(c) => c,
    };
    let components: Vec<&str> = resolved_base.split('/').collect();
    let mut suffix = 2u32;
    loop {
        let mut candidate_components: Vec<String> =
            components.iter().map(|s| s.to_string()).collect();
        candidate_components[component] = format!("{}-{suffix}", components[component]);
        let candidate = candidate_components.join("/");
        match conflict_component(&candidate, &existing) {
            None => return candidate,
            Some(conflict) if conflict < component => {
                component = conflict;
                suffix = 2;
            }
            Some(_) => suffix += 1,
        }
    }
}

/// Port of `resolveAutoFeatureBranchName` (`packages/shared/src/git.ts:101`).
pub fn resolve_auto_feature_branch_name(
    existing_branch_names: Vec<String>,
    preferred_branch: Option<String>,
) -> Result<String, String> {
    let preferred = preferred_branch.as_deref().map(js_trim);
    let resolved_base = sanitize_feature_branch_name(
        match preferred {
            Some(p) if !p.is_empty() => p,
            _ => "feature/update",
        }
        .to_string(),
    )?;
    Ok(resolve_unique_branch_name(
        &existing_branch_names,
        &resolved_base,
    ))
}

/// Port of `buildSynaraBranchName` (`packages/shared/src/git.ts:112`).
pub fn build_synara_branch_name(preferred_branch: Option<String>) -> Result<String, String> {
    let preferred = preferred_branch
        .as_deref()
        .map(js_trim)
        .unwrap_or_default()
        .to_string();
    let separator_index = preferred.find('/');
    let existing_namespace = match separator_index {
        Some(i) if i > 0 => &preferred[..i],
        _ => "",
    }
    .to_lowercase();
    let normalized_existing = if existing_namespace == "codex"
        || existing_namespace == WORKTREE_BRANCH_PREFIX
        || is_pre_cutover_worktree_namespace(&existing_namespace)
    {
        preferred[separator_index.unwrap_or(0) + 1..].to_string()
    } else {
        preferred.clone()
    };
    let frag = if normalized_existing.is_empty() {
        "update".to_string()
    } else {
        normalized_existing
    };
    Ok(format!(
        "{WORKTREE_BRANCH_PREFIX}/{}",
        sanitize_branch_fragment(frag)?
    ))
}

/// Port of `resolveUniqueSynaraBranchName` (`packages/shared/src/git.ts:128`).
pub fn resolve_unique_synara_branch_name(
    existing_branch_names: Vec<String>,
    preferred_branch: Option<String>,
) -> Result<String, String> {
    let resolved_base = build_synara_branch_name(preferred_branch)?;
    Ok(resolve_unique_branch_name(
        &existing_branch_names,
        &resolved_base,
    ))
}

/// `/^([a-z0-9][a-z0-9-]*)\/[0-9a-f]{8}$/` on the trimmed, lowercased branch —
/// returns the namespace when it matches.
fn temp_branch_namespace(branch: &str) -> Option<String> {
    let s = js_trim(branch).to_lowercase();
    let (ns, rest) = s.split_once('/')?;
    if ns.is_empty()
        || !ns.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-'))
        || !ns.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }
    if rest.len() != 8 || !rest.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
        return None;
    }
    Some(ns.to_string())
}

/// Port of `isTemporaryWorktreeBranch` (`packages/shared/src/git.ts:136`).
pub fn is_temporary_worktree_branch(branch: String) -> Result<bool, String> {
    let Some(namespace) = temp_branch_namespace(&branch) else {
        return Ok(false);
    };
    Ok(namespace == WORKTREE_BRANCH_PREFIX || is_pre_cutover_worktree_namespace(&namespace))
}

/// Port of `resolveThreadBranchRegressionGuard` (`packages/shared/src/git.ts:152`).
pub fn resolve_thread_branch_regression_guard(
    input: serde_json::Value,
) -> Result<Option<String>, String> {
    let member = |k: &str| -> Result<Value, String> {
        match &input {
            Value::Null => Err(format!("Cannot read properties of null (reading '{k}')")),
            Value::Object(o) => Ok(o.get(k).cloned().unwrap_or_else(js::undefined)),
            _ => Ok(js::undefined()),
        }
    };
    let current = member("currentBranch")?;
    let next = member("nextBranch")?;

    // `input.currentBranch !== null` — JS `undefined !== null` is true, and
    // `isTemporaryWorktreeBranch(undefined)` throws on `.trim`.
    fn temp_check(v: &Value) -> Result<bool, String> {
        match v {
            Value::String(s) => is_temporary_worktree_branch(s.clone()),
            Value::Null => Ok(false), // guarded out by `!== null` in JS
            _ if js::is_undefined(v) => {
                Err("Cannot read properties of undefined (reading 'trim')".into())
            }
            _ => Err("branch.trim is not a function".into()),
        }
    }

    let guard =
        !current.is_null() && !next.is_null() && !temp_check(&current)? && { temp_check(&next)? };
    if guard {
        Ok(current.as_str().map(str::to_string))
    } else {
        match &next {
            Value::String(s) => Ok(Some(s.clone())),
            _ => Ok(None),
        }
    }
}
