//! Live per-agent runtime state for Claude dynamic workflows, ported from
//! upstream `claudeWorkflowRuntime.ts`: polled incrementally from the run's
//! transcript directory. Everything is byte-offset incremental and
//! best-effort — parse failures and fs errors degrade to "no update".

use std::collections::HashMap;

use serde_json::Value;

pub const MAX_CLAUDE_WORKFLOW_FILE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_CHUNK_BYTES: u64 = 512 * 1024;
const RECENT_TOOL_NAMES: usize = 3;
pub const WORKFLOW_PROMPT_PREVIEW_CHARS: usize = 160;

pub struct AgentAccum {
    pub agent_id: String,
    pub state: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub tokens: Option<u64>,
    pub tool_calls: u64,
    pub recent_tool_names: Vec<String>,
    pub prompt_preview: Option<String>,
    pub started_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub transcript_offset: u64,
    pub transcript_skipped: bool,
    pub seen_tool_use_ids: std::collections::HashSet<String>,
}

impl AgentAccum {
    fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            state: "running".to_string(),
            model: None,
            effort: None,
            tokens: None,
            tool_calls: 0,
            recent_tool_names: Vec::new(),
            prompt_preview: None,
            started_at: None,
            last_activity_at: None,
            transcript_offset: 0,
            transcript_skipped: false,
            seen_tool_use_ids: std::collections::HashSet::new(),
        }
    }
}

#[derive(Default)]
pub struct WorkflowRuntimeState {
    pub journal_offset: u64,
    pub journal_skipped: bool,
    // Insertion order is journal start order; labels zip against it.
    pub agents: HashMap<String, AgentAccum>,
    pub insertion_order: Vec<String>,
}

fn parse_json_line(line: &str) -> Option<Value> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    match serde_json::from_str::<Value>(trimmed) { Ok(v) if v.is_object() => Some(v), _ => None }
}

/// Journal lines: {"type":"started"|"result","key":"v2:<hash>","agentId":"..."}.
pub fn apply_journal_lines(state: &mut WorkflowRuntimeState, lines: &[String]) -> bool {
    let mut changed = false;
    for line in lines {
        let Some(record) = parse_json_line(line) else { continue };
        let Some(agent_id) = record.get("agentId").and_then(Value::as_str) else {
            continue;
        };
        match record.get("type").and_then(Value::as_str) {
            Some("started") => {
                if !state.agents.contains_key(agent_id) {
                    state.agents.insert(agent_id.to_string(), AgentAccum::new(agent_id));
                    state.insertion_order.push(agent_id.to_string());
                    changed = true;
                }
            }
            Some("result") => {
                if !state.agents.contains_key(agent_id) {
                    state.agents.insert(agent_id.to_string(), AgentAccum::new(agent_id));
                    state.insertion_order.push(agent_id.to_string());
                }
                let agent = state.agents.get_mut(agent_id).expect("inserted");
                if agent.state != "completed" {
                    agent.state = "completed".to_string();
                    changed = true;
                }
            }
            _ => {}
        }
    }
    changed
}

/// Agent transcript lines (Claude session jsonl shape): the first plain-string
/// user line is the prompt; assistant lines carry model/usage/tool_use blocks.
pub fn apply_agent_transcript_lines(agent: &mut AgentAccum, lines: &[String]) -> bool {
    let mut changed = false;
    for line in lines {
        let Some(record) = parse_json_line(line) else { continue };
        if let Some(timestamp) = record.get("timestamp").and_then(Value::as_str) {
            if agent.started_at.is_none() {
                agent.started_at = Some(timestamp.to_string());
            }
            if agent.last_activity_at.as_deref() != Some(timestamp) {
                agent.last_activity_at = Some(timestamp.to_string());
                changed = true;
            }
        }
        let Some(message) = record.get("message").filter(|v| v.is_object()) else {
            continue;
        };
        if record.get("type").and_then(Value::as_str) == Some("user")
            && agent.prompt_preview.is_none()
        {
            if let Some(content) = message.get("content").and_then(Value::as_str) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    agent.prompt_preview = Some(
                        trimmed.chars().take(WORKFLOW_PROMPT_PREVIEW_CHARS).collect(),
                    );
                    changed = true;
                }
            }
        }
        if record.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        if let Some(model) = message.get("model").and_then(Value::as_str) {
            if !model.is_empty() && agent.model.as_deref() != Some(model) {
                agent.model = Some(model.to_string());
                changed = true;
            }
        }
        // Reasoning effort rides on the transcript line itself (sibling of
        // `message`), not inside the API message payload.
        if let Some(effort) = record.get("effort").and_then(Value::as_str) {
            if !effort.is_empty() && agent.effort.as_deref() != Some(effort) {
                agent.effort = Some(effort.to_string());
                changed = true;
            }
        }
        if let Some(usage) = message.get("usage").filter(|v| v.is_object()) {
            let num = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
            let total = num("input_tokens")
                + num("output_tokens")
                + num("cache_creation_input_tokens")
                + num("cache_read_input_tokens");
            if total > 0 && agent.tokens != Some(total) {
                agent.tokens = Some(total);
                changed = true;
            }
        }
        if let Some(content) = message.get("content").and_then(Value::as_array) {
            for block in content {
                if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                    continue;
                }
                let block_id = block.get("id").and_then(Value::as_str);
                if let Some(id) = block_id {
                    if agent.seen_tool_use_ids.contains(id) {
                        continue;
                    }
                    agent.seen_tool_use_ids.insert(id.to_string());
                }
                agent.tool_calls += 1;
                if let Some(name) = block.get("name").and_then(Value::as_str) {
                    let name = name.trim();
                    if !name.is_empty() {
                        agent.recent_tool_names.push(name.to_string());
                        if agent.recent_tool_names.len() > RECENT_TOOL_NAMES {
                            let overflow = agent.recent_tool_names.len() - RECENT_TOOL_NAMES;
                            agent.recent_tool_names.drain(0..overflow);
                        }
                    }
                }
                changed = true;
            }
        }
    }
    changed
}

/// Snapshot assembly: labels zip against journal start order, upstream
/// claudeWorkflowRuntimeSnapshots.
pub fn runtime_snapshots(state: &WorkflowRuntimeState, labels: &[String]) -> Vec<Value> {
    let mut out = Vec::new();
    for (index, agent_id) in state.insertion_order.iter().enumerate() {
        let Some(agent) = state.agents.get(agent_id) else { continue };
        let mut snap = serde_json::json!({
            "agentId": agent.agent_id,
            "state": agent.state,
        });
        if let Some(label) = labels.get(index) {
            snap["label"] = serde_json::json!(label);
        }
        if let Some(model) = &agent.model {
            snap["model"] = serde_json::json!(model);
        }
        if let Some(effort) = &agent.effort {
            snap["effort"] = serde_json::json!(effort);
        }
        if let Some(tokens) = agent.tokens {
            snap["tokens"] = serde_json::json!(tokens);
        }
        if agent.tool_calls > 0 {
            snap["toolCalls"] = serde_json::json!(agent.tool_calls);
        }
        if !agent.recent_tool_names.is_empty() {
            snap["recentToolNames"] = serde_json::json!(agent.recent_tool_names);
        }
        if let Some(preview) = &agent.prompt_preview {
            snap["promptPreview"] = serde_json::json!(preview);
        }
        if let Some(started) = &agent.started_at {
            snap["startedAt"] = serde_json::json!(started);
        }
        if let Some(last) = &agent.last_activity_at {
            snap["lastActivityAt"] = serde_json::json!(last);
        }
        out.push(snap);
    }
    out
}

pub struct AppendedLines {
    pub lines: Vec<String>,
    pub next_offset: u64,
    pub skipped: bool,
}

/// Complete lines appended past `offset`; the trailing partial line stays
/// unconsumed until a later tick.
pub fn read_appended_lines(path: &str, offset: u64, max_file_bytes: u64) -> Option<AppendedLines> {
    let size = std::fs::metadata(path).ok()?.len();
    if size <= offset {
        return None;
    }
    if size > max_file_bytes {
        return Some(AppendedLines {
            lines: Vec::new(),
            next_offset: offset,
            skipped: true,
        });
    }
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let take = (size - offset).min(MAX_CHUNK_BYTES) as usize;
    let mut buf = vec![0u8; take];
    let mut read = 0usize;
    while read < take {
        match file.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => break,
        }
    }
    if read == 0 {
        return None;
    }
    buf.truncate(read);
    let last_newline = buf.iter().rposition(|b| *b == b'\n')?;
    let text = String::from_utf8_lossy(&buf[..last_newline]).to_string();
    Some(AppendedLines {
        lines: text.split('\n').map(str::to_string).collect(),
        next_offset: offset + last_newline as u64 + 1,
        skipped: false,
    })
}

/// Settled workflow output within the same safety bound as live transcripts.
pub fn read_workflow_output_text(path: &str) -> Option<String> {
    let size = std::fs::metadata(path).ok()?.len();
    if size == 0 || size > MAX_CLAUDE_WORKFLOW_FILE_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok().filter(|s| !s.is_empty())
}

/// One poll tick: fold new journal lines and per-agent transcript tails into
/// `state`. Returns true when anything observable changed.
pub fn collect_runtime(transcript_dir: &str, state: &mut WorkflowRuntimeState) -> bool {
    let mut changed = false;
    if !state.journal_skipped {
        let journal_path = format!("{transcript_dir}/journal.jsonl");
        if let Some(journal) = read_appended_lines(
            &journal_path,
            state.journal_offset,
            MAX_CLAUDE_WORKFLOW_FILE_BYTES,
        ) {
            if journal.skipped {
                state.journal_skipped = true;
            } else {
                state.journal_offset = journal.next_offset;
                changed = apply_journal_lines(state, &journal.lines) || changed;
            }
        }
    }
    let ids: Vec<String> = state.insertion_order.clone();
    for agent_id in ids {
        let Some(agent) = state.agents.get_mut(&agent_id) else { continue };
        if agent.transcript_skipped {
            continue;
        }
        let path = format!("{transcript_dir}/agent-{}.jsonl", agent.agent_id);
        if let Some(tail) = read_appended_lines(
            &path,
            agent.transcript_offset,
            MAX_CLAUDE_WORKFLOW_FILE_BYTES,
        ) {
            if tail.skipped {
                agent.transcript_skipped = true;
                continue;
            }
            agent.transcript_offset = tail.next_offset;
            changed = apply_agent_transcript_lines(agent, &tail.lines) || changed;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_start_and_result_lifecycle() {
        let mut state = WorkflowRuntimeState::default();
        let changed = apply_journal_lines(
            &mut state,
            &[
                r#"{"type":"started","key":"v2:a","agentId":"agent-1"}"#.to_string(),
                r#"{"type":"started","key":"v2:b","agentId":"agent-2"}"#.to_string(),
            ],
        );
        assert!(changed);
        assert_eq!(state.agents.len(), 2);
        assert_eq!(state.agents["agent-1"].state, "running");
        apply_journal_lines(&mut state, &[r#"{"type":"result","agentId":"agent-1"}"#.to_string()]);
        assert_eq!(state.agents["agent-1"].state, "completed");
        assert_eq!(state.agents["agent-2"].state, "running");
    }

    #[test]
    fn transcript_accumulates_usage_and_tools() {
        let mut agent = AgentAccum::new("agent-1");
        let lines = vec![
            serde_json::json!({"type":"user","timestamp":"2025-06-01T10:00:00Z","message":{"content":"investigate the flaky test"}}).to_string(),
            serde_json::json!({"type":"assistant","timestamp":"2025-06-01T10:00:05Z","effort":"high","message":{"model":"claude-opus","usage":{"input_tokens":100,"output_tokens":50},"content":[{"type":"tool_use","id":"tu-1","name":"Read"}]}}).to_string(),
        ];
        assert!(apply_agent_transcript_lines(&mut agent, &lines));
        assert_eq!(agent.model.as_deref(), Some("claude-opus"));
        assert_eq!(agent.effort.as_deref(), Some("high"));
        assert_eq!(agent.tokens, Some(150));
        assert_eq!(agent.tool_calls, 1);
        assert_eq!(agent.recent_tool_names, vec!["Read"]);
        assert_eq!(
            agent.prompt_preview.as_deref(),
            Some("investigate the flaky test")
        );
        // The same assistant line again: no double counting of the tool call.
        assert!(!apply_agent_transcript_lines(&mut agent, &lines[1..]));
        assert_eq!(agent.tool_calls, 1);
    }

    #[test]
    fn incremental_read_consumes_whole_lines_only() {
        let path = std::env::temp_dir().join(format!("wf-lines-{}", std::process::id()));
        std::fs::write(&path, "one\ntwo\npart").unwrap();
        let read = read_appended_lines(path.to_str().unwrap(), 0, MAX_CLAUDE_WORKFLOW_FILE_BYTES)
            .unwrap();
        assert_eq!(read.lines, vec!["one", "two"]);
        // Offset sits after the second newline; the partial line waits.
        assert_eq!(read.next_offset, 8);
        std::fs::write(&path, "one\ntwo\npartial\n").unwrap();
        let read = read_appended_lines(path.to_str().unwrap(), 8, MAX_CLAUDE_WORKFLOW_FILE_BYTES)
            .unwrap();
        assert_eq!(read.lines, vec!["partial"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn collect_runtime_ticks() {
        let dir = std::env::temp_dir().join(format!("wf-collect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("journal.jsonl"),
            "{\"type\":\"started\",\"agentId\":\"agent-7\"}\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("agent-agent-7.jsonl"),
            "{\"type\":\"user\",\"timestamp\":\"2025-06-01T10:00:00Z\",\"message\":{\"content\":\"scan the repo\"}}\n",
        )
        .unwrap();
        let mut state = WorkflowRuntimeState::default();
        assert!(collect_runtime(dir.to_str().unwrap(), &mut state));
        assert_eq!(state.agents.len(), 1);
        let snaps = runtime_snapshots(&state, &["phase one".to_string()]);
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0]["label"], "phase one");
        assert_eq!(snaps[0]["agentId"], "agent-7");
        assert!(!collect_runtime(dir.to_str().unwrap(), &mut state));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
