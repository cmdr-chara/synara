//! `projects.listDirectories` — recursive directory listing for the project
//! explorer and composer mention menu. Entries come back flat; the client
//! filters dotfiles and groups folders/files itself.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use crate::todo::WsErr;

const MAX_DEPTH: u32 = 32;
const REL_PATH_MAX: usize = 1024;

fn entry_kind(path: &Path) -> Option<&'static str> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.is_dir() {
        Some("directory")
    } else if meta.is_file() {
        Some("file")
    } else {
        None
    }
}

fn has_children(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut r| r.next().is_some())
        .unwrap_or(false)
}

fn collect(dir: &Path, depth: u32, include_files: bool, nested: bool, out: &mut Vec<Value>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<(String, PathBuf)> = read
        .flatten()
        .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
        .filter(|(name, _)| !name.is_empty())
        .collect();
    names.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, path) in names {
        let Some(kind) = entry_kind(&path) else {
            continue;
        };
        if kind == "file" && !include_files {
            continue;
        }
        let mut entry = json!({
            "path": path.to_string_lossy(),
            "name": name,
            "kind": kind,
        });
        if nested {
            entry["parentPath"] = json!(dir.to_string_lossy());
        }
        if kind == "directory" {
            entry["hasChildren"] = json!(has_children(&path));
            out.push(entry);
            if depth > 1 {
                collect(&path, depth - 1, include_files, true, out);
            }
        } else {
            out.push(entry);
        }
    }
}

pub fn list_directories(payload: &Value) -> Result<Value, WsErr> {
    let cwd = payload
        .get("cwd")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| WsErr("cwd is required".to_string(), Some("VALIDATION")))?;
    let rel = match payload.get("relativePath") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.trim().is_empty() && s.len() <= REL_PATH_MAX => {
            Some(s.as_str())
        }
        Some(_) => {
            return Err(WsErr(
                "relativePath must be a non-empty string of at most 1024 characters".to_string(),
                Some("VALIDATION"),
            ));
        }
    };
    let depth = match payload.get("depth") {
        None | Some(Value::Null) => 1u32,
        Some(Value::Number(n)) => n
            .as_u64()
            .filter(|d| *d >= 1 && *d <= MAX_DEPTH as u64)
            .map(|d| d as u32)
            .ok_or_else(|| {
                WsErr(
                    format!("depth must be an integer between 1 and {MAX_DEPTH}"),
                    Some("VALIDATION"),
                )
            })?,
        Some(_) => {
            return Err(WsErr(
                "depth must be a positive integer".to_string(),
                Some("VALIDATION"),
            ));
        }
    };
    let include_files = payload
        .get("includeFiles")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let root = match rel {
        Some(r) => Path::new(cwd).join(r),
        None => PathBuf::from(cwd),
    };
    // Stay inside `cwd`: reject a relativePath that escapes via `..`.
    let cwd_canon = std::fs::canonicalize(cwd).unwrap_or_else(|_| PathBuf::from(cwd));
    let root_canon = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
    if !root_canon.starts_with(&cwd_canon) {
        return Err(WsErr(
            "relativePath must stay inside cwd".to_string(),
            Some("VALIDATION"),
        ));
    }
    if !root.is_dir() {
        return Err(WsErr(
            format!("No such directory: {}", root.display()),
            Some("FS_BROWSE_FAILED"),
        ));
    }
    let mut entries: Vec<Value> = Vec::new();
    collect(&root, depth, include_files, false, &mut entries);
    Ok(json!({ "entries": entries }))
}
