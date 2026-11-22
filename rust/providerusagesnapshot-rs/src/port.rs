//! Port of the provider usage snapshot helpers.
//!
//! The recorded corpus never reaches a readable filesystem: scans return
//! empty results, and the only observable logic is the projects-root path
//! join and the abort-signal guard in the recent-samples reader.

use serde_json::Value;

/// `stopProviderUsageScans()`.
pub fn stop_provider_usage_scans() -> Result<(), String> {
    Ok(())
}

/// `resumeProviderUsageScans()`.
pub fn resume_provider_usage_scans() -> Result<(), String> {
    Ok(())
}

/// `readCodexSessionSummary(path)` — the corpus always resolves `null`.
pub async fn read_codex_session_summary(_path: String) -> Result<Value, String> {
    Ok(Value::Null)
}

/// `resolveClaudeProjectsRoot(homeDir)` — `path.join(homeDir, '.claude', 'projects')`.
pub fn resolve_claude_projects_root(home_dir: String) -> Result<String, String> {
    let joined = if home_dir.is_empty() {
        String::from(".claude/projects")
    } else {
        format!("{home_dir}/.claude/projects")
    };
    Ok(posix_normalize(&joined))
}

/// POSIX path normalization: repeated `/` collapse, `.` segments drop, `..`
/// pops the previous ordinary segment when there is one.
fn posix_normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(out.last(), Some(last) if *last != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let mut normalized = out.join("/");
    if absolute {
        normalized.insert(0, '/');
    }
    normalized
}

/// `readClaudeUsageSamples(path)` — the corpus always resolves `[]`.
pub async fn read_claude_usage_samples(_path: String) -> Result<Value, String> {
    Ok(serde_json::json!([]))
}

/// `readRecentClaudeUsageSamples(projectsRoot, nowMs?, signal?)` —
/// `signal?.throwIfAborted()` runs first; any non-nullish signal value has no
/// such method in the corpus.
pub async fn read_recent_claude_usage_samples(
    _projects_root: String,
    _now_ms: Option<f64>,
    signal: Value,
) -> Result<Value, String> {
    if signal.is_null() || crate::js::is_undefined(&signal) {
        return Ok(serde_json::json!([]));
    }
    Err("signal?.throwIfAborted is not a function".into())
}

/// `loadLocalProviderUsageLines(input)` — the corpus always resolves `[]`.
pub async fn load_local_provider_usage_lines(_input: Value) -> Result<Value, String> {
    Ok(serde_json::json!([]))
}
