//! Post-turn tidy formatting for files changed by provider processes.
//!
//! Providers edit the workspace directly, so the backend cannot intercept every
//! write. When enabled, terminal turns scan the small set of files reported by
//! git (or a bounded fallback scan) and format only files whose mtime advanced
//! during the turn. Unsupported/binary/large files are left untouched.

use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SCAN_FILES: usize = 10_000;
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".synara",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
];

fn unix_ms(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn modified_since(path: &Path, since_ms: i64) -> bool {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map(|time| unix_ms(time) >= since_ms.saturating_sub(1_000))
        .unwrap_or(false)
}

fn git_output(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn names_from_z(data: &[u8]) -> Vec<String> {
    data.split(|byte| *byte == 0)
        .filter_map(|part| std::str::from_utf8(part).ok())
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn git_candidates(root: &Path) -> Option<Vec<String>> {
    // `diff HEAD` covers tracked unstaged and staged edits; `ls-files --others`
    // adds untracked files while respecting ignore rules.
    let tracked = git_output(root, &["diff", "--name-only", "-z", "HEAD", "--"])?;
    let untracked = git_output(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    )?;
    let mut names = HashSet::new();
    names.extend(names_from_z(&tracked));
    names.extend(names_from_z(&untracked));
    Some(names.into_iter().collect())
}

fn scan_candidates(root: &Path) -> Vec<String> {
    let mut result = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut visited = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        if visited >= MAX_SCAN_FILES || depth > 8 {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if visited >= MAX_SCAN_FILES {
                break;
            }
            visited += 1;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if !SKIP_DIRS.contains(&name.as_str()) {
                    stack.push((path, depth + 1));
                }
            } else if file_type.is_file()
                && let Ok(relative) = path.strip_prefix(root)
            {
                result.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    result
}

fn format_candidate(root: &Path, relative: &str) -> bool {
    let path = root.join(relative);
    if !path.is_file() || tidy::Language::file(&path.to_string_lossy()).is_none() {
        return false;
    }
    crate::projects_fileops::format_file(&json!({
        "cwd": root,
        "relativePath": relative,
        "write": true,
        "preserveLineEnding": true,
    }))
    .ok()
    .and_then(|result| result.get("written").and_then(serde_json::Value::as_bool))
    .unwrap_or(false)
}

pub(crate) fn format_turn_workspace(root: PathBuf, since_ms: i64) -> usize {
    let candidates = if root.join(".git").exists() {
        git_candidates(&root).unwrap_or_else(|| scan_candidates(&root))
    } else {
        scan_candidates(&root)
    };
    let mut formatted = 0usize;
    for relative in candidates {
        let path = root.join(&relative);
        if modified_since(&path, since_ms) && format_candidate(&root, &relative) {
            formatted += 1;
        }
    }
    formatted
}

pub(crate) fn spawn_after_turn(root: PathBuf, since_ms: i64) {
    if !crate::server_settings::format_agent_files_enabled() || since_ms <= 0 {
        return;
    }
    std::thread::spawn(move || {
        let _ = format_turn_workspace(root, since_ms);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_files_modified_after_turn_start() {
        let root = std::env::temp_dir().join(format!("turn-format-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("x.js"), "const x={a:1}\n").unwrap();
        let now = unix_ms(SystemTime::now()).saturating_sub(1_000);
        assert_eq!(format_turn_workspace(root.clone(), now), 1);
        assert_eq!(
            std::fs::read_to_string(root.join("x.js")).unwrap(),
            "const x = { a: 1 };\n"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn leaves_older_files_untouched() {
        let root = std::env::temp_dir().join(format!("turn-format-old-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("x.js"), "const x={a:1}\n").unwrap();
        let future = unix_ms(SystemTime::now()) + 60_000;
        assert_eq!(format_turn_workspace(root.clone(), future), 0);
        assert_eq!(
            std::fs::read_to_string(root.join("x.js")).unwrap(),
            "const x={a:1}\n"
        );
        std::fs::remove_dir_all(root).ok();
    }
}
