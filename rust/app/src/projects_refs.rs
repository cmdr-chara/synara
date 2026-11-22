//! `projects.resolveWorkspaceFileReferences` — map chat file references onto
//! real workspace-relative paths (exact hit first, then unique suffix match
//! against the search index).
//!
//! `projects.resolveOutOfRootFileReference` — retry a workspace-relative
//! reference against ancestors of the project root, bounded to the user's
//! home directory, returning the absolute path it resolves to.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use crate::projects_search::index_entries;
use crate::todo::WsErr;

const PATH_MAX: usize = 2048;
const MAX_PATHS: usize = 128;

fn cwd_of(payload: &Value) -> Result<PathBuf, WsErr> {
    payload
        .get("cwd")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| WsErr("cwd is required".to_string(), Some("VALIDATION")))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Resolve one reference: exact `<cwd>/<rel>` exists → `rel`; otherwise the
/// unique index entry ending with `/rel` wins; ambiguous or absent → null.
fn resolve_reference(cwd: &Path, rel: &str) -> Option<String> {
    if cwd.join(rel).exists() {
        return Some(rel.to_string());
    }
    let suffix = format!("/{rel}");
    let mut hit: Option<String> = None;
    for entry in index_entries(cwd) {
        if entry.rel.ends_with(&suffix) {
            if hit.is_some() {
                return None;
            }
            hit = Some(entry.rel.clone());
        }
    }
    hit
}

pub fn resolve_workspace_file_references(payload: &Value) -> Result<Value, WsErr> {
    let cwd = cwd_of(payload)?;
    let paths = payload
        .get("relativePaths")
        .and_then(|v| v.as_array())
        .filter(|a| !a.is_empty() && a.len() <= MAX_PATHS)
        .ok_or_else(|| {
            WsErr(
                format!("relativePaths must be an array of 1..{MAX_PATHS} entries"),
                Some("VALIDATION"),
            )
        })?;
    let mut resolved: Vec<Value> = Vec::with_capacity(paths.len());
    for raw in paths {
        let rel = raw
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= PATH_MAX);
        match rel {
            Some(rel) => match resolve_reference(&cwd, rel) {
                Some(found) => resolved.push(json!(found)),
                None => resolved.push(Value::Null),
            },
            None => {
                return Err(WsErr(
                    format!(
                        "relativePaths entries must be non-empty strings of at most {PATH_MAX} characters"
                    ),
                    Some("VALIDATION"),
                ));
            }
        }
    }
    Ok(json!({ "relativePaths": resolved }))
}

pub fn resolve_out_of_root_file_reference(payload: &Value) -> Result<Value, WsErr> {
    let cwd = cwd_of(payload)?;
    let rel = payload
        .get("relativePath")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= PATH_MAX)
        .ok_or_else(|| {
            WsErr(
                "relativePath must be a non-empty string of at most 2048 characters".to_string(),
                Some("VALIDATION"),
            )
        })?;
    let home = home_dir();
    let cwd_canon = std::fs::canonicalize(&cwd).unwrap_or(cwd.clone());
    let home_canon = std::fs::canonicalize(&home).unwrap_or(home.clone());
    let mut dir = Some(cwd_canon.as_path());
    while let Some(d) = dir {
        let candidate = d.join(rel);
        if candidate.is_file() {
            return Ok(json!({ "fullPath": candidate.to_string_lossy() }));
        }
        if d == home_canon {
            break;
        }
        dir = d.parent();
    }
    Ok(json!({ "fullPath": Value::Null }))
}
