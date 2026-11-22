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

const MAX_ANCESTOR_WALK_DEPTH: usize = 32;

/// Clean-room `isWorkspaceRelativePathSafe`: non-empty, not absolute
/// (POSIX `/` or Windows `X:\`/`X:/`), no `.`/`..` segments.
fn is_workspace_relative_path_safe(value: &str) -> bool {
    let t = value.trim();
    if t.is_empty() || t.starts_with('/') {
        return false;
    }
    let b = t.as_bytes();
    if b.len() >= 3 && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') && b[0].is_ascii_alphabetic()
    {
        return false;
    }
    t.split(['/', '\\']).all(|seg| seg != ".." && seg != ".")
}

/// Clean-room `isContainedPath`: candidate equals root or sits under it.
fn is_contained_path(root: &Path, candidate: &Path) -> bool {
    candidate == root || candidate.starts_with(root)
}

fn realpath(p: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(p).ok()
}

/// `projects.resolveOutOfRootFileReference` — a chat reference that is missing
/// under the workspace root may be relative to an ancestor of the root
/// (agents emit paths like `Outbox/note.md` for a root inside a deeper dir).
/// Returns the canonical absolute path of the first ancestor match, or null.
/// Upstream semantics (apps/server/src/workspace/outOfRootFileReference.ts,
/// unflagged — readable source):
/// - unsafe reference (null bytes, absolute, `.`/`..`) → null
/// - missing root, or root outside the real home → null
/// - the in-root join existing (file or dir) → null — this method only
///   relocates references genuinely missing under the root
/// - ancestors of the root (nearest first, excluding the root itself) are
///   tried up to the home directory, depth-capped at 32
/// - the match must canonicalize to a regular file still inside home
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
    let null_result = || Ok(json!({ "fullPath": Value::Null }));
    if rel.contains('\0') || !is_workspace_relative_path_safe(rel) {
        return null_result();
    }
    let (Some(real_home), Some(real_root)) = (realpath(&home_dir()), realpath(&cwd)) else {
        return null_result();
    };
    if !is_contained_path(&real_home, &real_root) {
        return null_result();
    }
    let segments: Vec<&str> = rel.split(['/', '\\']).collect();
    let in_root = segments.iter().fold(real_root.clone(), |p, s| p.join(s));
    match in_root.symlink_metadata() {
        Ok(_) => return null_result(),
        Err(e) => {
            // Only a genuinely missing path permits relocation (ENOENT=2,
            // ENOTDIR=20); other failures belong to the original read.
            let missing = e.kind() == std::io::ErrorKind::NotFound || e.raw_os_error() == Some(20);
            if !missing {
                return null_result();
            }
        }
    }
    let mut ancestor = real_root.parent().map(Path::to_path_buf);
    let mut depth = 0;
    while let Some(dir) = ancestor {
        if depth >= MAX_ANCESTOR_WALK_DEPTH || !is_contained_path(&real_home, &dir) {
            break;
        }
        let candidate = segments.iter().fold(dir.clone(), |p, s| p.join(s));
        if let Some(real_candidate) = realpath(&candidate) {
            if is_contained_path(&real_home, &real_candidate) && real_candidate.is_file() {
                return Ok(json!({ "fullPath": real_candidate.to_string_lossy() }));
            }
        }
        let parent = dir.parent().map(Path::to_path_buf);
        if parent.as_deref() == Some(dir.as_path()) {
            break;
        }
        ancestor = parent;
        depth += 1;
    }
    null_result()
}
