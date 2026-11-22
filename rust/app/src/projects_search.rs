//! `projects.prewarmSearchIndex`, `projects.searchEntries` and
//! `projects.searchLocalEntries` — workspace file search.
//!
//! The index is a cached entry list per root: `git ls-files` (tracked +
//! untracked, gitignore-aware) when the root is a worktree, otherwise a
//! recursive walk that skips VCS dirs, `node_modules` and dot-directories.
//! Queries are ranked fuzzy matches: case-insensitive subsequence on the
//! entry path, best score first.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::todo::WsErr;

const QUERY_MAX: usize = 256;
const SEARCH_ENTRIES_MAX_LIMIT: usize = 200;
const SEARCH_LOCAL_MAX_LIMIT: usize = 100;
const LOCAL_DEFAULT_LIMIT: usize = 50;
const INDEX_TTL: Duration = Duration::from_secs(30);
const WALK_MAX_ENTRIES: usize = 100_000;

struct IndexEntry {
    rel: String,
    kind: &'static str,
}

struct Index {
    entries: Vec<IndexEntry>,
    built_at: Instant,
}

static INDEXES: LazyLock<Mutex<HashMap<PathBuf, Index>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn git_entry_list(root: &Path) -> Option<Vec<IndexEntry>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut entries: Vec<IndexEntry> = Vec::new();
    let mut dirs: Vec<String> = Vec::new();
    for file in stdout.split('\0').filter(|s| !s.is_empty()) {
        entries.push(IndexEntry {
            rel: file.to_string(),
            kind: "file",
        });
        let mut prefix = Path::new(file).parent();
        while let Some(parent) = prefix {
            let rel = parent.to_string_lossy();
            if rel.is_empty() {
                break;
            }
            dirs.push(rel.to_string());
            prefix = parent.parent();
        }
    }
    dirs.sort();
    dirs.dedup();
    for rel in dirs {
        entries.push(IndexEntry {
            rel,
            kind: "directory",
        });
    }
    Some(entries)
}

fn walk_entry_list(root: &Path) -> Vec<IndexEntry> {
    let mut entries: Vec<IndexEntry> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if entries.len() >= WALK_MAX_ENTRIES {
            break;
        }
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for child in read.flatten() {
            let path = child.path();
            let name = child.file_name();
            let name = name.to_string_lossy();
            let Ok(meta) = child.metadata() else {
                continue;
            };
            if meta.is_dir() {
                if name.starts_with('.') || name == "node_modules" {
                    continue;
                }
                stack.push(path.clone());
            }
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            entries.push(IndexEntry {
                rel: rel.to_string_lossy().to_string(),
                kind: if meta.is_dir() { "directory" } else { "file" },
            });
        }
    }
    entries
}

fn build_index(root: &Path) -> Index {
    let entries = git_entry_list(root).unwrap_or_else(|| walk_entry_list(root));
    Index {
        entries,
        built_at: Instant::now(),
    }
}

fn index_fresh(index: &Index) -> bool {
    index.built_at.elapsed() < INDEX_TTL
}

fn index_for(root: &Path) -> usize {
    let mut map = INDEXES.lock().unwrap();
    let index = map
        .entry(root.to_path_buf())
        .and_modify(|i| {
            if !index_fresh(i) {
                *i = build_index(root);
            }
        })
        .or_insert_with(|| build_index(root));
    index.entries.len()
}

/// Rank a query against a candidate path: `Some(score)` when every query
/// character appears in order (case-insensitive subsequence); lower is better.
/// Contiguous and earlier matches outrank scattered and late ones.
fn fuzzy_score(query: &[char], candidate: &[char]) -> Option<u64> {
    let mut score: u64 = 0;
    let mut qi = 0;
    for (ci, &c) in candidate.iter().enumerate() {
        if qi < query.len() && c == query[qi] {
            score += (ci as u64) * 4;
            qi += 1;
        }
    }
    if qi < query.len() {
        return None;
    }
    Some(score + candidate.len() as u64)
}

fn norm_chars(s: &str) -> Vec<char> {
    s.to_lowercase().chars().collect()
}

fn validated_query(payload: &Value, min_len: usize) -> Result<String, WsErr> {
    let query = payload
        .get("query")
        .and_then(|v| v.as_str())
        .filter(|s| s.trim().len() >= min_len && s.len() <= QUERY_MAX)
        .ok_or_else(|| {
            WsErr(
                format!("query must be a string between {min_len} and {QUERY_MAX} characters"),
                Some("VALIDATION"),
            )
        })?;
    Ok(query.trim().to_string())
}

fn required_cwd(payload: &Value) -> Result<PathBuf, WsErr> {
    let cwd = payload
        .get("cwd")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| WsErr("cwd is required".to_string(), Some("VALIDATION")))?;
    Ok(PathBuf::from(cwd))
}

fn search_ranked(root: &Path, query: &str) -> Vec<(u64, String, &'static str)> {
    index_for(root);
    let map = INDEXES.lock().unwrap();
    let Some(index) = map.get(root) else {
        return Vec::new();
    };
    let query_chars = norm_chars(query);
    let mut scored: Vec<(u64, String, &'static str)> = index
        .entries
        .iter()
        .filter_map(|e| {
            fuzzy_score(&query_chars, &norm_chars(&e.rel))
                .map(|score| (score, e.rel.clone(), e.kind))
        })
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    scored
}

pub fn prewarm_search_index(payload: &Value) -> Result<Value, WsErr> {
    let root = required_cwd(payload)?;
    if !root.is_dir() {
        return Err(WsErr(
            format!("No such directory: {}", root.display()),
            Some("FS_BROWSE_FAILED"),
        ));
    }
    let fresh = INDEXES
        .lock()
        .unwrap()
        .get(&root)
        .map(index_fresh)
        .unwrap_or(false);
    if fresh {
        return Ok(json!({ "started": false }));
    }
    let root_clone = root.clone();
    std::thread::spawn(move || {
        let built = build_index(&root_clone);
        INDEXES
            .lock()
            .unwrap()
            .insert(root_clone.to_path_buf(), built);
    });
    Ok(json!({ "started": true }))
}

pub fn search_entries(payload: &Value) -> Result<Value, WsErr> {
    let root = required_cwd(payload)?;
    let query = validated_query(payload, 1)?;
    let limit = payload
        .get("limit")
        .and_then(|v| v.as_u64())
        .filter(|l| *l >= 1 && *l <= SEARCH_ENTRIES_MAX_LIMIT as u64)
        .ok_or_else(|| {
            WsErr(
                format!("limit must be an integer between 1 and {SEARCH_ENTRIES_MAX_LIMIT}"),
                Some("VALIDATION"),
            )
        })? as usize;
    let kind_filter = match payload.get("kind") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s == "file" || s == "directory" => Some(s.clone()),
        Some(_) => {
            return Err(WsErr(
                "kind must be \"file\" or \"directory\"".to_string(),
                Some("VALIDATION"),
            ));
        }
    };
    if !root.is_dir() {
        return Err(WsErr(
            format!("No such directory: {}", root.display()),
            Some("FS_BROWSE_FAILED"),
        ));
    }
    let scored = search_ranked(&root, &query);
    let filtered: Vec<Value> = scored
        .iter()
        .filter(|(_, _, kind)| kind_filter.as_deref().map(|k| k == *kind).unwrap_or(true))
        .take(limit + 1)
        .map(|(_, rel, kind)| {
            let parent = Path::new(rel).parent().map(|p| p.to_string_lossy());
            let mut entry = json!({ "path": rel, "kind": kind });
            if let Some(parent) = parent {
                if !parent.is_empty() {
                    entry["parentPath"] = json!(parent);
                }
            }
            entry
        })
        .collect();
    let truncated = filtered.len() > limit;
    let entries: Vec<Value> = filtered.into_iter().take(limit).collect();
    Ok(json!({ "entries": entries, "truncated": truncated }))
}

pub fn search_local_entries(payload: &Value) -> Result<Value, WsErr> {
    let root = payload
        .get("rootPath")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| WsErr("rootPath is required".to_string(), Some("VALIDATION")))?;
    let query = validated_query(payload, 1)?;
    let limit = payload
        .get("limit")
        .and_then(|v| v.as_u64())
        .filter(|l| *l >= 1 && *l <= SEARCH_LOCAL_MAX_LIMIT as u64)
        .map(|l| l as usize)
        .unwrap_or(LOCAL_DEFAULT_LIMIT);
    let include_files = payload
        .get("includeFiles")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !root.is_dir() {
        return Err(WsErr(
            format!("No such directory: {}", root.display()),
            Some("FS_BROWSE_FAILED"),
        ));
    }
    let scored = search_ranked(&root, &query);
    let filtered: Vec<Value> = scored
        .iter()
        .filter(|(_, _, kind)| include_files || *kind == "directory")
        .take(limit + 1)
        .map(|(_, rel, kind)| {
            let abs = root.join(rel);
            let name = Path::new(rel)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| rel.clone());
            let mut entry = json!({
                "path": abs.to_string_lossy(),
                "name": name,
                "kind": kind,
            });
            if let Some(parent) = abs.parent() {
                entry["parentPath"] = json!(parent.to_string_lossy());
            }
            entry
        })
        .collect();
    let truncated = filtered.len() > limit;
    let entries: Vec<Value> = filtered.into_iter().take(limit).collect();
    Ok(json!({ "entries": entries, "truncated": truncated }))
}
