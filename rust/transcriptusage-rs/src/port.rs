//! The Rust port of `apps/server/src/transcriptUsage.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// `path.posix.normalize`.
fn normalize(p: &str) -> String {
    if p.is_empty() {
        return ".".to_string();
    }
    let abs = p.starts_with('/');
    let trailing = p.ends_with('/') && p.len() > 1;
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|last| *last != "..") {
                    out.pop();
                } else if !abs {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let mut r = String::new();
    if abs {
        r.push('/');
    }
    r.push_str(&out.join("/"));
    if r.is_empty() {
        r = if abs { "/".into() } else { ".".into() };
    }
    if trailing && r != "/" {
        r.push('/');
    }
    r
}

/// `path.posix.join`.
fn join(parts: &[&str]) -> String {
    let joined: Vec<&str> = parts.iter().copied().filter(|s| !s.is_empty()).collect();
    if joined.is_empty() {
        return ".".to_string();
    }
    normalize(&joined.join("/"))
}

/// `value?.trim() || fallback` — JS `String.prototype.trim` on env values.
fn env_trim_nonempty(name: &str) -> Option<String> {
    let v = std::env::var(name).ok()?;
    let trimmed = v.trim_matches(|c: char| c.is_whitespace() && c != '\u{85}' || c == '\u{FEFF}');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `resolveClaudeProjectsRoot` (`providerUsageSnapshot.ts:558`).
fn resolve_claude_projects_root(home_dir: &str) -> String {
    let config_dir = env_trim_nonempty("CLAUDE_CONFIG_DIR");
    join(&[
        &config_dir.unwrap_or_else(|| join(&[home_dir, ".claude"])),
        "projects",
    ])
}

/// Port of `defaultTranscriptRoots` (`apps/server/src/transcriptUsage.ts:43`).
pub fn default_transcript_roots(home_dir: String) -> Result<Vec<Value>, String> {
    let codex_home =
        env_trim_nonempty("CODEX_HOME").unwrap_or_else(|| join(&[&home_dir, ".codex"]));
    Ok(vec![
        json!({"provider": "claudeAgent", "path": resolve_claude_projects_root(&home_dir)}),
        json!({"provider": "codex", "path": join(&[&codex_home, "sessions"])}),
        json!({"provider": "codex", "path": join(&[&codex_home, "archived_sessions"])}),
    ])
}
