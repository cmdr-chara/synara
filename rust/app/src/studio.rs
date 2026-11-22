//! Native `studio.listThreadOutputs`: lists files a Studio/group chat produced
//! under its workspace root (checkpoint diffs + file-change/captured output
//! activities), mirroring apps/server/src/studioOutputs.ts.

use rusqlite::params;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::orchestration_snapshot::open_state_copy;
use crate::todo::WsErr;

const MAX_THREAD_OUTPUT_FILES: usize = 500;
const EXCLUDED_TOP_LEVEL: &[&str] = &["tmp", "logs", "inbox", "context", "skills"];
const EXCLUDED_ROOT_FILES: &[&str] = &["agents.md", "claude.md"];
const EXCLUDED_NESTED: &[&str] = &["node_modules"];
const STUDIO_SUBDIRS: &[&str] = &[
    "Inbox", "Context", "Logs", "Skills", "tmp", "Outbox/Content", "Outbox/Daily", "Outbox/Images",
    "Outbox/Notion", "Outbox/TikTok", "Outbox/YouTube",
];

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

const STUDIO_INSTRUCTIONS: &str = "# Studio Workspace\n\nThis folder is the shared workspace for Synara Studio chats. Keep it organized so the\napp can attribute and surface what you produce.\n\n## Where files go\n\n- `Outbox/<Category>/` — every final deliverable: documents, PDFs, images, exports,\n  anything the user asked you to produce. Use an existing category (Content, Daily,\n  Images, Notion, TikTok, YouTube) or create a new one that fits.\n- `tmp/` — scratch space: helper scripts, intermediate artifacts, downloads you only\n  need while working. Anything here may be cleaned up at any time.\n- `Inbox/` — files the user drops in for you to process. Read from here; do not write.\n- `Context/` — reference material and background docs. Read-only unless asked.\n- `Logs/` and `Skills/` — managed by Synara; leave them alone.\n\n## Rules\n\n- Never leave a deliverable in `tmp/` or loose in this root: move the finished file\n  into `Outbox/<Category>/` as the last step of your work.\n- Prefer descriptive, dated file names, e.g. `2026-07-08_customer_report.pdf`.\n";

/// Best-effort self-heal of `~/Documents/Synara/Studio`.
fn scaffold_studio_root() {
    let Ok(home) = std::env::var("HOME") else {
        return;
    };
    let root = Path::new(&home).join("Documents").join("Synara").join("Studio");
    let _ = std::fs::create_dir_all(&root);
    for sub in STUDIO_SUBDIRS {
        let _ = std::fs::create_dir_all(root.join(sub));
    }
    for name in ["AGENTS.md", "CLAUDE.md"] {
        let p = root.join(name);
        if !p.exists() {
            let _ = std::fs::write(&p, STUDIO_INSTRUCTIONS);
        }
    }
}

fn safe_segment(seg: &str) -> bool {
    !seg.is_empty() && seg != "." && seg != ".." && !seg.starts_with('.')
}

fn is_output_relative_path(rel: &str) -> bool {
    let segs: Vec<&str> = rel.split('/').collect();
    if segs.is_empty() || segs[0].is_empty() {
        return false;
    }
    if segs.len() == 1 && EXCLUDED_ROOT_FILES.contains(&segs[0].to_lowercase().as_str()) {
        return false;
    }
    if segs.len() > 1 && EXCLUDED_TOP_LEVEL.contains(&segs[0].to_lowercase().as_str()) {
        return false;
    }
    segs.iter()
        .all(|s| safe_segment(s) && !EXCLUDED_NESTED.contains(&s.to_lowercase().as_str()))
}

/// Pulls `path`/`filePath`/`file_path` strings out of an activity payload's
/// `data`, max depth 6, skipping failed/declined payloads.
fn collect_activity_paths(payload: &Value, out: &mut Vec<String>, depth: usize) {
    if depth > 6 {
        return;
    }
    match payload {
        Value::Array(items) => {
            for item in items {
                collect_activity_paths(item, out, depth + 1);
            }
        }
        Value::Object(map) => {
            for (key, child) in map {
                if matches!(key.as_str(), "path" | "filePath" | "file_path") {
                    if let Some(s) = child.as_str() {
                        let s = s.trim();
                        if !s.is_empty() {
                            out.push(s.to_string());
                        }
                        continue;
                    }
                }
                collect_activity_paths(child, out, depth + 1);
            }
        }
        _ => {}
    }
}

/// worktree mode → worktreePath; else workingDirectory ?? workspaceRoot.
fn workspace_cwd(
    workspace_root: &str,
    env_mode: &str,
    worktree_path: Option<&str>,
    working_dir: Option<&str>,
) -> Option<String> {
    if env_mode == "worktree" {
        let wt = worktree_path?;
        if let Some(dir) = working_dir {
            if dir.starts_with(wt) {
                return Some(dir.to_string());
            }
        }
        return Some(wt.to_string());
    }
    if let Some(dir) = working_dir {
        return Some(dir.to_string());
    }
    if workspace_root.is_empty() {
        None
    } else {
        Some(workspace_root.to_string())
    }
}

pub fn handles(tag: &str) -> bool {
    tag == "studio.listThreadOutputs"
}

pub fn handle(_tag: &str, payload: &Value) -> Result<Value, WsErr> {
    let thread_id = payload
        .get("threadId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("Failed to list studio thread outputs"))?;
    scaffold_studio_root();
    let Some(copy) = open_state_copy() else {
        return Err(err("Failed to list studio thread outputs"));
    };
    let conn = &copy.conn;
    let ctx: Option<(String, String, String, String, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT t.thread_id, t.project_id, p.kind, p.workspace_root, \
                    t.env_mode, t.worktree_path, t.working_directory \
             FROM projection_threads t \
             INNER JOIN projection_projects p ON p.project_id = t.project_id \
             WHERE t.thread_id = ?1 AND t.deleted_at IS NULL LIMIT 1",
            params![thread_id],
            |r| {
                Ok((
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .ok();
    let Some((_project_id, kind, workspace_root, env_mode, worktree_path, working_dir)) = ctx
    else {
        return Ok(json!({ "entries": [] }));
    };
    if kind != "group" && kind != "studio" {
        return Ok(json!({ "entries": [] }));
    }
    let Some(cwd) = workspace_cwd(
        &workspace_root,
        &env_mode,
        worktree_path.as_deref(),
        working_dir.as_deref(),
    ) else {
        return Ok(json!({ "entries": [] }));
    };

    // Checkpoints: newest first (rows ASC by turn count, iterated in reverse).
    let mut checkpoint_files: Vec<String> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT checkpoint_files_json FROM projection_turns \
         WHERE thread_id = ?1 AND checkpoint_turn_count IS NOT NULL \
           AND (completed_at IS NOT NULL \
             OR (0 AND checkpoint_status = 'missing' AND checkpoint_ref LIKE 'provider-diff:%')) \
         ORDER BY checkpoint_turn_count ASC",
    ) {
        if let Ok(rows) = stmt.query_map(params![thread_id], |r| r.get::<_, String>(0)) {
            for rj in rows.flatten() {
                if let Ok(Value::Array(files)) = serde_json::from_str::<Value>(&rj) {
                    let mut v = Vec::new();
                    for f in files {
                        if let Some(p) = f.get("path").and_then(|p| p.as_str()) {
                            v.push(p.to_string());
                        }
                    }
                    checkpoint_files.push(v.join("\u{0}"));
                }
            }
        }
    }

    // Activity payloads, newest first.
    let mut activity_payloads: Vec<Value> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT payload_json FROM projection_thread_activities \
         WHERE thread_id = ?1 AND ( \
           (kind = 'tool.completed' AND json_extract(payload_json, '$.itemType') = 'file_change') \
           OR kind = 'studio.outputs.captured') \
         ORDER BY created_at DESC, activity_id DESC LIMIT 2000",
    ) {
        if let Ok(rows) = stmt.query_map(params![thread_id], |r| r.get::<_, String>(0)) {
            for rj in rows.flatten() {
                if let Ok(v) = serde_json::from_str::<Value>(&rj) {
                    activity_payloads.push(v);
                }
            }
        }
    }

    let root = PathBuf::from(&cwd);
    let mut rel_paths: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Checkpoint paths, newest checkpoint first (checkpoint_files rows are ASC).
    for files_blob in checkpoint_files.iter().rev() {
        for file in files_blob.split('\u{0}') {
            if rel_paths.len() >= MAX_THREAD_OUTPUT_FILES {
                break;
            }
            if is_output_relative_path(file) && !seen.contains(file) {
                seen.insert(file.to_string());
                rel_paths.push(file.to_string());
            }
        }
    }

    // Activity path candidates (failed/declined payloads skipped).
    for payload in &activity_payloads {
        if rel_paths.len() >= MAX_THREAD_OUTPUT_FILES {
            break;
        }
        if let Some(status) = payload.get("status").and_then(|v| v.as_str()) {
            let s = status.to_lowercase();
            if s == "failed" || s == "declined" {
                continue;
            }
        }
        let mut candidates: Vec<String> = Vec::new();
        if let Some(data) = payload.get("data") {
            collect_activity_paths(data, &mut candidates, 0);
        }
        for candidate in candidates {
            if rel_paths.len() >= MAX_THREAD_OUTPUT_FILES || candidate.contains('\0') {
                continue;
            }
            let abs = if Path::new(&candidate).is_absolute() {
                PathBuf::from(&candidate)
            } else {
                root.join(&candidate)
            };
            let abs = abs.canonicalize().unwrap_or(abs);
            let Ok(rel) = abs.strip_prefix(&root.canonicalize().unwrap_or(root.clone())) else {
                continue;
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if rel_str.is_empty()
                || rel_str == ".."
                || rel_str.starts_with("../")
                || rel_str.starts_with('/')
            {
                continue;
            }
            if is_output_relative_path(&rel_str) && !seen.contains(&rel_str) {
                seen.insert(rel_str.clone());
                rel_paths.push(rel_str);
            }
        }
    }

    // Stat + sort newest-first.
    let mut entries: Vec<(String, String, String, u64)> = Vec::new();
    for rel in rel_paths {
        let full = root.join(&rel);
        let Ok(meta) = std::fs::metadata(&full) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let name = rel.split('/').last().unwrap_or(&rel).to_string();
        entries.push((name, rel, full.to_string_lossy().to_string(), mtime_ms));
    }
    entries.sort_by(|a, b| b.3.cmp(&a.3));
    let wire: Vec<Value> = entries
        .into_iter()
        .map(|(name, rel, full, mtime_ms)| {
            let secs = mtime_ms / 1000;
            let iso = iso_from_unix(secs, (mtime_ms % 1000) as u32);
            json!({
                "name": name,
                "relativePath": rel,
                "fullPath": full,
                "modifiedAt": iso,
            })
        })
        .collect();
    Ok(json!({ "entries": wire }))
}

fn iso_from_unix(secs: u64, millis: u32) -> String {
    // UTC conversion via `date` keeps chrono out of the dep graph.
    let out = std::process::Command::new("date")
        .args(["-u", "-r", &secs.to_string(), "+%Y-%m-%dT%H:%M:%S"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "1970-01-01T00:00:00".into());
    format!("{out}.{millis:03}Z")
}
