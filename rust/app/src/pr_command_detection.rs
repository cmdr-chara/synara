//! Port of upstream `pullRequestCommandDetection.ts`: recognize a pull request
//! an agent created with `gh pr create` from a finished command item, so the
//! PR is linked even when the agent forgets the link tool. Conservative by
//! design: a missed link only loses the safety net, a false link would attach
//! somebody else's PR to the thread.

use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;

static SHELL_WRAPPER_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:\S*/)?(?:ba|z|da)?sh\s+(?:-[a-z]*c[a-z]*)\s+").unwrap()
});
static CREATE_COMMAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[;&|(\n]|&&|\|\|)\s*(?:(?:[A-Za-z_][A-Za-z0-9_]*=\S*|env|time|nohup|command)\s+)*(?:\S*/)?gh\s+pr\s+create\b").unwrap()
});
static LISTS_PULL_REQUESTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bgh\s+(?:pr\s+(?:list|view|status|checks|diff)|search\s+prs|api)\b").unwrap()
});
static URL_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(https://github\.com/[^/\s]+/[^/\s]+/pull/\d+)[ \t]*$").unwrap()
});
static HEREDOC_START: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<<-?\s*['"]?(\w+)['"]?"#).unwrap()
});
static SINGLE_QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'[^']*'").unwrap());
static DOUBLE_QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""(?:[^"\\]|\\.)*""#).unwrap());

const COMMAND_KEYS: &[&str] = &["command", "cmd"];
const COMMAND_CONTAINER_KEYS: &[&str] = &["item", "input", "arguments", "args", "params", "rawInput", "call"];
const OUTPUT_KEYS: &[&str] = &[
    "aggregatedOutput", "aggregated_output", "formattedOutput", "formatted_output",
    "output", "rawOutput", "raw_output", "result", "stdout",
];
const MAX_OUTPUT_CHARS: usize = 200_000;

fn command_text(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        let t = s.trim();
        return if t.is_empty() { None } else { Some(t.to_string()) };
    }
    if let Some(arr) = value.as_array() {
        if arr.iter().all(|e| e.is_string()) {
            let joined: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
            let t = joined.join(" ");
            let t = t.trim();
            return if t.is_empty() { None } else { Some(t.to_string()) };
        }
    }
    None
}

fn parse_json_record(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    value.as_object()
}

fn find_command(data: Option<&serde_json::Map<String, Value>>) -> Option<String> {
    let data = data?;
    for key in COMMAND_KEYS {
        if let Some(v) = data.get(*key) {
            if let Some(cmd) = command_text(v) {
                return Some(cmd);
            }
        }
    }
    for key in COMMAND_CONTAINER_KEYS {
        let Some(v) = data.get(*key) else { continue };
        // Upstream also parses string-encoded JSON here; tool calls in the
        // native pipeline carry structured records, so the object form covers it.
        if let Some(nested) = parse_json_record(v) {
            if let Some(cmd) = find_command(Some(nested)) {
                return Some(cmd);
            }
        }
    }
    None
}

fn collect_strings(value: &Value, out: &mut String, depth: usize) {
    if depth > 5 || out.len() > MAX_OUTPUT_CHARS {
        return;
    }
    match value {
        Value::String(s) => {
            if out.len() + s.len() <= MAX_OUTPUT_CHARS {
                out.push_str(s);
                out.push('\n');
            }
        }
        Value::Array(arr) => {
            for entry in arr {
                collect_strings(entry, out, depth + 1);
            }
        }
        Value::Object(map) => {
            for entry in map.values() {
                collect_strings(entry, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn collect_output(value: &Value, out: &mut String, depth: usize) {
    let Some(record) = value.as_object() else { return };
    if depth > 3 {
        return;
    }
    for (key, entry) in record {
        if OUTPUT_KEYS.contains(&key.as_str()) {
            collect_strings(entry, out, 0);
        } else if key == "item" {
            collect_output(entry, out, depth + 1);
        }
    }
}

fn unwrap_shell(command: &str) -> String {
    if let Some(m) = SHELL_WRAPPER_PREFIX.find(command) {
        let rest = &command[m.end()..];
        let bytes = rest.as_bytes();
        if bytes.len() >= 2 && (bytes[0] == b'\'' || bytes[0] == b'"') {
            let quote = bytes[0];
            if bytes[bytes.len() - 1] == quote {
                return rest[1..rest.len() - 1].to_string();
            }
        }
    }
    command.to_string()
}

/// Quoted text and heredoc bodies are arguments (titles, PR bodies), never commands.
fn strip_heredocs(command: &str) -> String {
    let mut result = String::with_capacity(command.len());
    let mut rest = command;
    loop {
        let Some(m) = HEREDOC_START.find(rest) else {
            result.push_str(rest);
            break;
        };
        let caps = HEREDOC_START.captures(rest).unwrap();
        let delim = caps[1].to_string();
        result.push_str(&rest[..m.start()]);
        let after_start = &rest[m.end()..];
        // find a line that is exactly the delimiter (optional whitespace)
        let mut end_pos = None;
        let mut offset = 0;
        for line in after_start.split_inclusive('\n') {
            let trimmed = line.trim_end_matches('\n');
            if trimmed.trim() == delim {
                end_pos = Some(offset + line.len());
                break;
            }
            offset += line.len();
        }
        match end_pos {
            Some(pos) => {
                result.push('\n');
                rest = &after_start[pos..];
            }
            None => {
                // unterminated heredoc: drop the rest, matching upstream's
                // conservative treatment of malformed input
                break;
            }
        }
    }
    result
}

fn strip_arguments(command: &str) -> String {
    let no_heredoc = strip_heredocs(command);
    let no_single = SINGLE_QUOTED.replace_all(&no_heredoc, "''");
    DOUBLE_QUOTED.replace_all(&no_single, "\"\"").to_string()
}

/// URLs of pull requests a finished, successful command item created with
/// `gh pr create`. Empty when the command did not create a PR at a command
/// position, also listed PRs, mentions the URL itself, or printed more PR
/// URLs than it ran create commands.
pub(crate) fn detect_created_pull_request_urls(
    item_type: &str,
    status: Option<&str>,
    data: &Value,
) -> Vec<String> {
    if item_type != "command_execution" || status == Some("failed") {
        return Vec::new();
    }
    let raw_command = match find_command(data.as_object()) {
        Some(c) => c,
        None => return Vec::new(),
    };
    let command = strip_arguments(&unwrap_shell(&raw_command));
    let create_count = CREATE_COMMAND.find_iter(&command).count();
    if create_count == 0 || LISTS_PULL_REQUESTS.is_match(&command) {
        return Vec::new();
    }
    let mut output = String::new();
    collect_output(data, &mut output, 0);
    let mut urls: Vec<String> = Vec::new();
    for caps in URL_LINE.captures_iter(&output) {
        let url = caps[1].to_string();
        if !urls.contains(&url) {
            urls.push(url);
        }
    }
    if urls.is_empty() || urls.len() > create_count {
        return Vec::new();
    }
    if urls.iter().any(|url| raw_command.contains(url.as_str())) {
        return Vec::new();
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_simple_pr_create() {
        let data = json!({
            "command": "gh pr create --title x --body y",
            "output": "https://github.com/o/r/pull/42\n"
        });
        let urls = detect_created_pull_request_urls("command_execution", Some("completed"), &data);
        assert_eq!(urls, vec!["https://github.com/o/r/pull/42"]);
    }

    #[test]
    fn ignores_listing_commands() {
        let data = json!({
            "command": "gh pr create --title x && gh pr list",
            "output": "https://github.com/o/r/pull/42\n"
        });
        assert!(detect_created_pull_request_urls("command_execution", Some("completed"), &data).is_empty());
    }

    #[test]
    fn ignores_failed_commands() {
        let data = json!({
            "command": "gh pr create --title x",
            "output": "https://github.com/o/r/pull/42\n"
        });
        assert!(detect_created_pull_request_urls("command_execution", Some("failed"), &data).is_empty());
    }

    #[test]
    fn ignores_create_inside_quotes() {
        let data = json!({
            "command": "echo \"gh pr create\"",
            "output": "https://github.com/o/r/pull/42\n"
        });
        assert!(detect_created_pull_request_urls("command_execution", Some("completed"), &data).is_empty());
    }

    #[test]
    fn ignores_when_more_urls_than_creates() {
        let data = json!({
            "command": "gh pr create --title x",
            "output": "https://github.com/o/r/pull/42\nhttps://github.com/o/r/pull/43\n"
        });
        assert!(detect_created_pull_request_urls("command_execution", Some("completed"), &data).is_empty());
    }

    #[test]
    fn unwraps_shell_wrapper() {
        let data = json!({
            "command": "bash -c 'gh pr create --title x'",
            "output": "https://github.com/o/r/pull/42\n"
        });
        let urls = detect_created_pull_request_urls("command_execution", None, &data);
        assert_eq!(urls, vec!["https://github.com/o/r/pull/42"]);
    }
}

/// Resolve detected PR URLs with `gh` and dispatch `thread.pull-request.link`
/// with source "created" for each, mirroring upstream
/// `linkCommandCreatedPullRequests`. Runs off the runner hot path; failures
/// only lose the safety-net link.
pub(crate) fn link_command_created_pull_requests(thread_id: &str, urls: Vec<String>) {
    let Ok(conn) = crate::orchestration_engine::open_state_rw() else { return };
    let archived: bool = conn
        .query_row(
            "SELECT archived_at IS NOT NULL FROM projection_threads WHERE thread_id = ?1",
            [thread_id],
            |r| r.get(0),
        )
        .unwrap_or(true);
    if archived {
        return;
    }
    for url in urls {
        // Skip when an explicit link already exists for this URL.
        let existing: String = conn
            .query_row(
                "SELECT COALESCE(pull_requests_json, '[]') FROM projection_threads WHERE thread_id = ?1",
                [thread_id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "[]".into());
        if let Ok(links) = serde_json::from_str::<Value>(&existing) {
            if let Some(arr) = links.as_array() {
                let already_explicit = arr.iter().any(|l| {
                    l.pointer("/pullRequest/url").and_then(Value::as_str) == Some(url.as_str())
                        && matches!(
                            l.get("source").and_then(Value::as_str),
                            Some("user") | Some("agent")
                        )
                });
                if already_explicit {
                    continue;
                }
            }
        }
        // Resolve the PR through gh, same as upstream gitManager.resolvePullRequest.
        let out = std::process::Command::new("gh")
            .args([
                "pr",
                "view",
                &url,
                "--json",
                "number,title,url,state,headRefName,baseRefName,isDraft",
            ])
            .output();
        let Ok(out) = out else { continue };
        if !out.status.success() {
            continue;
        }
        let Ok(info) = serde_json::from_slice::<Value>(&out.stdout) else { continue };
        let state = match info.get("state").and_then(Value::as_str).unwrap_or("") {
            "OPEN" => "open",
            "MERGED" => "merged",
            "CLOSED" => "closed",
            _ => "unknown",
        };
        let pr = json!({
            "url": info.get("url").cloned().unwrap_or(json!(url)),
            "number": info.get("number").cloned().unwrap_or(Value::Null),
            "title": info.get("title").cloned().unwrap_or(Value::Null),
            "state": state,
            "draft": info.get("isDraft").cloned().unwrap_or(json!(false)),
            "headRefName": info.get("headRefName").cloned().unwrap_or(Value::Null),
            "baseRefName": info.get("baseRefName").cloned().unwrap_or(Value::Null),
        });
        let mut cmd = serde_json::Map::new();
        cmd.insert("type".into(), json!("thread.pull-request.link"));
        cmd.insert(
            "commandId".into(),
            json!(format!("server:command-pr:{}", crate::orchestration_engine::uuid_v4())),
        );
        cmd.insert("threadId".into(), json!(thread_id));
        cmd.insert("createdAt".into(), json!(crate::todo::iso_now_ms()));
        cmd.insert("pullRequest".into(), pr);
        cmd.insert("source".into(), json!("created"));
        let _ = crate::orchestration_engine::handle(
            "orchestration.dispatchCommand",
            &Value::Object(cmd),
        );
    }
}

