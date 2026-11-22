//! `filesystem.browse` — directory listing for path pickers and the search
//! palette. `{partialPath, cwd?}` in, `{parentPath, entries[]}` out; callers
//! filter hidden files and match a leaf prefix themselves, so the server
//! returns every entry in the directory.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use crate::todo::WsErr;

const PATH_MAX: usize = 512;

fn path_arg<'a>(payload: &'a Value, key: &str) -> Result<Option<&'a str>, WsErr> {
    match payload.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() || s.len() > PATH_MAX {
                return Err(WsErr(
                    format!("{key} must be a non-empty path of at most {PATH_MAX} characters"),
                    Some("VALIDATION"),
                ));
            }
            Ok(Some(s))
        }
        Some(_) => Err(WsErr(format!("{key} must be a string"), Some("VALIDATION"))),
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn expand(path: &str, cwd: Option<&str>) -> PathBuf {
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        home_dir().join(rest)
    } else if path == "~" {
        home_dir()
    } else {
        PathBuf::from(path)
    };
    if expanded.is_absolute() {
        expanded
    } else {
        match cwd {
            Some(dir) => Path::new(dir).join(expanded),
            None => home_dir().join(expanded),
        }
    }
}

fn list(dir: &Path) -> Result<Value, WsErr> {
    let read = std::fs::read_dir(dir).map_err(|e| {
        WsErr(
            format!("Cannot browse {}: {e}", dir.display()),
            Some("FS_BROWSE_FAILED"),
        )
    })?;
    let mut entries: Vec<Value> = Vec::new();
    for item in read.flatten() {
        let name = item.file_name().to_string_lossy().to_string();
        if name.is_empty() {
            continue;
        }
        entries.push(json!({
            "name": name,
            "fullPath": item.path().to_string_lossy(),
        }));
    }
    entries.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    Ok(json!({
        "parentPath": dir.to_string_lossy(),
        "entries": entries,
    }))
}

pub fn browse(payload: &Value) -> Result<Value, WsErr> {
    let partial = path_arg(payload, "partialPath")?
        .ok_or_else(|| WsErr("partialPath is required".to_string(), Some("VALIDATION")))?;
    let cwd = path_arg(payload, "cwd")?;
    let resolved = expand(partial, cwd);
    if resolved.is_dir() {
        return list(&resolved);
    }
    // A trailing partial segment lands on the parent so callers still get a
    // usable listing while a path is being typed.
    if let Some(parent) = resolved.parent() {
        if parent.is_dir() {
            return list(parent);
        }
    }
    Err(WsErr(
        format!("No such directory: {partial}"),
        Some("FS_BROWSE_FAILED"),
    ))
}
