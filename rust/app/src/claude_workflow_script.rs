//! Static, best-effort inspection of Claude Code workflow scripts and results,
//! ported from upstream `claudeWorkflowScript.ts`. Workflow scripts must open
//! with `export const meta = {...}` as a pure literal, so meta can be read
//! without evaluating the script; everything returns None instead of throwing
//! when the input does not match that shape.

use serde_json::{Map, Value};

fn is_quote(c: char) -> bool {
    c == '"' || c == '\'' || c == '`'
}

struct Scanner<'a> {
    chars: Vec<char>,
    _src: &'a str,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().collect(),
            _src: src,
        }
    }
    fn at(&self, i: usize) -> Option<char> {
        self.chars.get(i).copied()
    }
    fn len(&self) -> usize {
        self.chars.len()
    }
    /// Byte/char-index slice as a String for regex-free scanning helpers.
    fn slice(&self, from: usize, to: usize) -> String {
        self.chars[from.min(self.len())..to.min(self.len())].iter().collect()
    }
}

/// Consumes whitespace plus `//` line comments and `/* ... */` block comments.
/// An unterminated block comment jumps to end-of-source, which the caller
/// treats as a mismatch.
fn skip_trivia(scan: &Scanner, from: usize) -> usize {
    let mut at = from;
    while at < scan.len() {
        let c = scan.at(at).unwrap();
        if c.is_whitespace() {
            at += 1;
            continue;
        }
        if c == '/' && scan.at(at + 1) == Some('/') {
            // scan to newline
            let mut i = at + 2;
            while i < scan.len() && scan.at(i) != Some('\n') {
                i += 1;
            }
            at = i;
            continue;
        }
        if c == '/' && scan.at(at + 1) == Some('*') {
            // find closing */
            let mut i = at + 2;
            let mut close = None;
            while i + 1 < scan.len() {
                if scan.at(i) == Some('*') && scan.at(i + 1) == Some('/') {
                    close = Some(i);
                    break;
                }
                i += 1;
            }
            match close {
                None => return scan.len(),
                Some(ci) => {
                    at = ci + 2;
                    continue;
                }
            }
        }
        break;
    }
    at
}

/// Parses a string literal starting at the quote char. Escape handling mirrors
/// upstream: a backslash consumes itself plus the next char verbatim (no
/// escape-sequence interpretation). Template literals containing `${` are
/// rejected because they are computed, not pure.
fn parse_string(scan: &Scanner, from: usize) -> Option<(String, usize)> {
    let quote = scan.at(from)?;
    let mut at = from + 1;
    let mut text = String::new();
    while at < scan.len() {
        let c = scan.at(at).unwrap();
        if c == '\\' && at + 1 < scan.len() {
            text.push(scan.at(at + 1).unwrap());
            at += 2;
            continue;
        }
        if quote == '`' && c == '$' && scan.at(at + 1) == Some('{') {
            return None;
        }
        if c == quote {
            return Some((text, at + 1));
        }
        text.push(c);
        at += 1;
    }
    None
}

/// Reads one literal value (string/number/boolean/null/array/object) starting
/// at `index`. None on anything computed — identifiers, calls, template
/// interpolation — which is exactly the pure-literal contract for meta.
fn parse_literal(scan: &Scanner, index: usize) -> Option<(Value, usize)> {
    let at = skip_trivia(scan, index);
    let c = scan.at(at)?;

    if is_quote(c) {
        let (s, end) = parse_string(scan, at)?;
        return Some((Value::String(s), end));
    }

    if c == '[' {
        let mut items = Vec::new();
        let mut cursor = skip_trivia(scan, at + 1);
        while cursor < scan.len() && scan.at(cursor) != Some(']') {
            let (item, end) = parse_literal(scan, cursor)?;
            items.push(item);
            cursor = skip_trivia(scan, end);
            if scan.at(cursor) == Some(',') {
                cursor = skip_trivia(scan, cursor + 1);
            }
        }
        return if scan.at(cursor) == Some(']') {
            Some((Value::Array(items), cursor + 1))
        } else {
            None
        };
    }

    if c == '{' {
        let mut record = Map::new();
        let mut cursor = skip_trivia(scan, at + 1);
        while cursor < scan.len() && scan.at(cursor) != Some('}') {
            let key: String;
            if scan.at(cursor).is_some_and(is_quote) {
                let (k, end) = parse_string(scan, cursor)?;
                key = k;
                cursor = end;
            } else {
                // identifier: [A-Za-z_$][\w$]*
                let first = scan.at(cursor)?;
                if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
                    return None;
                }
                let mut end = cursor + 1;
                while end < scan.len() {
                    let ch = scan.at(end).unwrap();
                    if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
                        end += 1;
                    } else {
                        break;
                    }
                }
                key = scan.slice(cursor, end);
                cursor = end;
            }
            cursor = skip_trivia(scan, cursor);
            if scan.at(cursor) != Some(':') {
                return None;
            }
            let (entry, end) = parse_literal(scan, cursor + 1)?;
            record.insert(key, entry);
            cursor = skip_trivia(scan, end);
            if scan.at(cursor) == Some(',') {
                cursor = skip_trivia(scan, cursor + 1);
            }
        }
        return if scan.at(cursor) == Some('}') {
            Some((Value::Object(record), cursor + 1))
        } else {
            None
        };
    }

    // primitives: true|false|null|-?\d+(\.\d+)?
    let rest = scan.slice(at, scan.len());
    for (word, value) in [("true", Value::Bool(true)), ("false", Value::Bool(false))] {
        if rest.starts_with(word) {
            return Some((value, at + word.chars().count()));
        }
    }
    if rest.starts_with("null") {
        return Some((Value::Null, at + 4));
    }
    // number: -?\d+(\.\d+)?
    let chars: Vec<char> = rest.chars().collect();
    let mut i = 0;
    if chars.first() == Some(&'-') {
        i = 1;
    }
    let digit_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i > digit_start {
        if i < chars.len() && chars[i] == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())
        {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
        let raw: String = chars[..i].iter().collect();
        if let Ok(num) = raw.parse::<f64>() {
            return Some((serde_json::json!(num), at + i));
        }
    }
    None
}

fn read_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `export const meta = {...}` pure-literal extraction.
pub fn parse_workflow_script_meta(script: &str) -> Option<Value> {
    // regex-free scan for /export\s+const\s+meta\s*=/
    let scan = Scanner::new(script);
    let mut decl_end = None;
    let mut i = 0;
    while i + 6 < scan.len() {
        if scan.slice(i, i + 6) == "export" {
            let mut j = i + 6;
            // \s+
            let ws_start = j;
            while j < scan.len() && scan.at(j).is_some_and(|c| c.is_whitespace()) {
                j += 1;
            }
            if j > ws_start && scan.slice(j, j + 5) == "const" {
                j += 5;
                let ws2 = j;
                while j < scan.len() && scan.at(j).is_some_and(|c| c.is_whitespace()) {
                    j += 1;
                }
                if j > ws2 && scan.slice(j, j + 4) == "meta" {
                    j += 4;
                    while j < scan.len() && scan.at(j).is_some_and(|c| c.is_whitespace()) {
                        j += 1;
                    }
                    if scan.at(j) == Some('=') {
                        decl_end = Some(j + 1);
                        break;
                    }
                }
            }
        }
        i += 1;
    }
    let start = decl_end?;
    let (parsed, _end) = parse_literal(&scan, start)?;
    let record = parsed.as_object()?;

    let name = record.get("name").and_then(read_string);
    let description = record.get("description").and_then(read_string);
    // phases: array of {title, detail?}
    let phases = record.get("phases").and_then(Value::as_array).and_then(|arr| {
        let phases: Vec<Value> = arr
            .iter()
            .filter_map(|entry| {
                let obj = entry.as_object()?;
                let title = obj.get("title").and_then(read_string)?;
                let mut phase = serde_json::json!({ "title": title });
                if let Some(detail) = obj.get("detail").and_then(read_string) {
                    phase["detail"] = Value::String(detail);
                }
                Some(phase)
            })
            .collect();
        if phases.is_empty() {
            None
        } else {
            Some(phases)
        }
    });
    if name.is_none() && description.is_none() && phases.is_none() {
        return None;
    }
    let mut out = serde_json::json!({});
    if let Some(n) = name {
        out["name"] = Value::String(n);
    }
    if let Some(d) = description {
        out["description"] = Value::String(d);
    }
    if let Some(p) = phases {
        out["phases"] = Value::Array(p);
    }
    Some(out)
}

/// Returns the text of one call's argument list, from the opening paren to its
/// balanced close, skipping over string literals and comments. Comment regions
/// are elided to a single space so downstream extraction never sees
/// commented-out content; string contents are preserved exactly.
fn read_balanced_call(scan: &Scanner, open_paren: usize) -> Option<String> {
    let mut depth = 0usize;
    let mut at = open_paren;
    let mut segment_start = open_paren;
    let mut parts: Vec<String> = Vec::new();

    while at < scan.len() {
        let c = scan.at(at).unwrap();
        if is_quote(c) {
            let quote = c;
            at += 1;
            while at < scan.len() && scan.at(at) != Some(quote) {
                at += if scan.at(at) == Some('\\') { 2 } else { 1 };
            }
            at += 1;
            continue;
        }
        if c == '/' && scan.at(at + 1) == Some('/') {
            parts.push(scan.slice(segment_start, at));
            parts.push(" ".to_string());
            let mut i = at + 2;
            while i < scan.len() && scan.at(i) != Some('\n') {
                i += 1;
            }
            at = i;
            segment_start = at;
            continue;
        }
        if c == '/' && scan.at(at + 1) == Some('*') {
            let mut i = at + 2;
            let mut close = None;
            while i + 1 < scan.len() {
                if scan.at(i) == Some('*') && scan.at(i + 1) == Some('/') {
                    close = Some(i);
                    break;
                }
                i += 1;
            }
            let ci = close?;
            parts.push(scan.slice(segment_start, at));
            parts.push(" ".to_string());
            at = ci + 2;
            segment_start = at;
            continue;
        }
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth = depth.checked_sub(1).unwrap_or(usize::MAX);
            if depth == 0 {
                parts.push(scan.slice(segment_start, at));
                let joined: String = parts.concat();
                return Some(joined.chars().skip(1).collect());
            }
        }
        at += 1;
    }
    None
}

/// regex-free `\bkey\s*:\s*(['"`])((?:\\.|(?!quote)[^\\])*)quote` extraction.
fn read_option_string_literal(call_text: &str, key: &str) -> Option<String> {
    let chars: Vec<char> = call_text.chars().collect();
    let key_chars: Vec<char> = key.chars().collect();
    let mut i = 0;
    while i + key_chars.len() < chars.len() {
        // word boundary before key
        let boundary = i == 0
            || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_' || chars[i - 1] == '$');
        if boundary && chars[i..i + key_chars.len()] == key_chars[..] {
            let mut j = i + key_chars.len();
            // key must not continue as an identifier char
            let end_ok = j >= chars.len()
                || !(chars[j].is_ascii_alphanumeric() || chars[j] == '_' || chars[j] == '$');
            if end_ok {
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && chars[j] == ':' {
                    j += 1;
                    while j < chars.len() && chars[j].is_whitespace() {
                        j += 1;
                    }
                    if j < chars.len() && is_quote(chars[j]) {
                        let quote = chars[j];
                        j += 1;
                        let mut value = String::new();
                        while j < chars.len() {
                            if chars[j] == '\\' && j + 1 < chars.len() {
                                // upstream replaceAll(/\\(.)/g, "$1"): unwraps
                                // the escape to the literal char.
                                value.push(chars[j + 1]);
                                j += 2;
                                continue;
                            }
                            if chars[j] == quote {
                                let trimmed = value.trim().to_string();
                                if !trimmed.is_empty() && !trimmed.contains("${") {
                                    return Some(trimmed);
                                }
                                return None;
                            }
                            value.push(chars[j]);
                            j += 1;
                        }
                        return None;
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Scans agent(...) call options for string-literal {label, phase, model,
/// effort} opts. Computed values are skipped; the map is a planning fallback
/// for agent rows before (or without) live per-agent data.
pub fn extract_workflow_agent_plans(script: &str) -> Option<Value> {
    let scan = Scanner::new(script);
    let mut plans = Map::new();
    let mut i = 0;
    while i + 5 < scan.len() {
        // \bagent\s*\(
        if scan.slice(i, i + 5) == "agent" {
            let boundary = i == 0 || {
                let prev = scan.at(i - 1).unwrap();
                !(prev.is_ascii_alphanumeric() || prev == '_' || prev == '$')
            };
            if boundary {
                let mut j = i + 5;
                while j < scan.len() && scan.at(j).is_some_and(|c| c.is_whitespace()) {
                    j += 1;
                }
                if scan.at(j) == Some('(') {
                    if let Some(call_text) = read_balanced_call(&scan, j) {
                        if let Some(label) = read_option_string_literal(&call_text, "label") {
                            let phase = read_option_string_literal(&call_text, "phase");
                            let model = read_option_string_literal(&call_text, "model");
                            let effort = read_option_string_literal(&call_text, "effort");
                            if phase.is_some() || model.is_some() || effort.is_some() {
                                let mut plan = serde_json::json!({});
                                if let Some(p) = phase {
                                    plan["phase"] = Value::String(p);
                                }
                                if let Some(m) = model {
                                    plan["model"] = Value::String(m);
                                }
                                if let Some(e) = effort {
                                    plan["effort"] = Value::String(e);
                                }
                                plans.insert(label, plan);
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
    if plans.is_empty() {
        None
    } else {
        Some(Value::Object(plans))
    }
}

/// Legacy label -> phase map derived from the plans; still emitted so older
/// persisted-event consumers keep working.
pub fn extract_workflow_agent_phases(script: &str) -> Option<Value> {
    let plans = extract_workflow_agent_plans(script)?;
    let obj = plans.as_object()?;
    let mut out = Map::new();
    for (label, plan) in obj {
        if let Some(phase) = plan.get("phase").and_then(Value::as_str) {
            out.insert(label.clone(), Value::String(phase.to_string()));
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(Value::Object(out))
    }
}

fn is_run_id_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
}

/// `\bwf_[a-z0-9-]{6,}\b` scan.
fn find_run_id(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 3 < chars.len() {
        let boundary = i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_');
        if boundary && chars[i] == 'w' && chars[i + 1] == 'f' && chars[i + 2] == '_' {
            let mut j = i + 3;
            while j < chars.len() && is_run_id_char(chars[j]) {
                j += 1;
            }
            let len = j - (i + 3);
            let end_boundary = j >= chars.len() || !(chars[j].is_ascii_alphanumeric() || chars[j] == '_');
            if len >= 6 && end_boundary {
                return Some(chars[i..j].iter().collect());
            }
        }
        i += 1;
    }
    None
}

/// Launch identifiers from the Workflow tool result. taskType was added after
/// the original structured result shape, so it may be absent on older results.
pub fn parse_workflow_launch(value: &Value) -> Option<Value> {
    if let Some(s) = value.as_str() {
        if let Ok(parsed) = serde_json::from_str::<Value>(s) {
            return parse_workflow_launch(&parsed);
        }
        return None;
    }
    let record = value.as_object()?;
    if let Some(task_type) = record.get("taskType") {
        if task_type.as_str() != Some("local_workflow") {
            return None;
        }
    }
    let task_id = record.get("taskId").and_then(read_string);
    let run_id = record.get("runId").and_then(read_string);
    let script_path = record.get("scriptPath").and_then(read_string);
    let transcript_dir = record.get("transcriptDir").and_then(read_string);
    if run_id.is_none() && script_path.is_none() {
        return None;
    }
    let mut out = serde_json::json!({});
    if let Some(t) = task_id {
        out["taskId"] = Value::String(t);
    }
    if let Some(r) = run_id {
        if find_run_id(&r).as_deref() == Some(r.as_str()) {
            out["runId"] = Value::String(r);
        }
    }
    if let Some(p) = script_path {
        out["scriptPath"] = Value::String(p);
    }
    if let Some(d) = transcript_dir {
        out["transcriptDir"] = Value::String(d);
    }
    Some(out)
}

/// Text fallback: run id pattern + a script-ish path from lines mentioning
/// "script" or "persisted".
pub fn parse_workflow_launch_from_text(text: &str) -> Option<Value> {
    let run_id = find_run_id(text);
    let script_path = text
        .lines()
        .filter(|line| {
            let lower = line.to_lowercase();
            lower.contains("script") || lower.contains("persisted")
        })
        .filter_map(|line| {
            // /(\/[^\s"'`]+\.[a-z]{2,4})\b/
            let chars: Vec<char> = line.chars().collect();
            let mut best = None;
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == '/' {
                    let mut j = i;
                    while j < chars.len()
                        && !chars[j].is_whitespace()
                        && chars[j] != '"'
                        && chars[j] != '\''
                        && chars[j] != '`'
                    {
                        j += 1;
                    }
                    let candidate: String = chars[i..j].iter().collect();
                    // must end with .xx or .xxx/.xxxx lowercase extension
                    if let Some(dot) = candidate.rfind('.') {
                        let ext = &candidate[dot + 1..];
                        if (2..=4).contains(&ext.len())
                            && ext.chars().all(|c| c.is_ascii_lowercase())
                            && dot > 0
                        {
                            best = Some(candidate);
                            break;
                        }
                    }
                }
                i += 1;
            }
            best
        })
        .next();
    if run_id.is_none() && script_path.is_none() {
        return None;
    }
    let mut out = serde_json::json!({});
    if let Some(r) = run_id {
        out["runId"] = Value::String(r);
    }
    if let Some(p) = script_path {
        out["scriptPath"] = Value::String(p);
    }
    Some(out)
}

fn read_int(value: &Value) -> Option<u64> {
    value.as_u64()
}

/// Final per-agent snapshots from the workflow's output_file JSON
/// (`workflowProgress` array with workflow_agent entries).
pub fn parse_workflow_progress_agents(content: &str) -> Option<Value> {
    let parsed = serde_json::from_str::<Value>(content).ok()?;
    let progress = parsed.get("workflowProgress")?.as_array()?;
    let agents: Vec<Value> = progress
        .iter()
        .filter_map(|entry| {
            let record = entry.as_object()?;
            if record.get("type").and_then(Value::as_str) != Some("workflow_agent") {
                return None;
            }
            let label = record.get("label").and_then(read_string)?;
            let mut snap = serde_json::json!({ "label": label });
            if let Some(v) = record.get("phaseIndex").and_then(read_int) {
                snap["phaseIndex"] = serde_json::json!(v);
            }
            for (key, field) in [
                ("phaseTitle", "phaseTitle"),
                ("agentId", "agentId"),
                ("model", "model"),
                ("effort", "effort"),
                ("state", "state"),
                ("lastToolName", "lastToolName"),
            ] {
                if let Some(s) = record.get(key).and_then(read_string) {
                    snap[field] = Value::String(s);
                }
            }
            for key in ["tokens", "toolCalls", "durationMs"] {
                if let Some(v) = record.get(key).and_then(read_int) {
                    snap[key] = serde_json::json!(v);
                }
            }
            if let Some(p) = record.get("promptPreview").and_then(read_string) {
                let truncated: String = p
                    .chars()
                    .take(crate::claude_workflow_runtime::WORKFLOW_PROMPT_PREVIEW_CHARS)
                    .collect();
                snap["promptPreview"] = Value::String(truncated);
            }
            Some(snap)
        })
        .collect();
    if agents.is_empty() {
        None
    } else {
        Some(Value::Array(agents))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_pure_literal_extraction() {
        let script = r#"
export const meta = {
  name: "Repo sweep",
  description: " scans everything ", // trailing comment
  phases: [
    { title: "Scan", detail: "read the tree" },
    { title: "Fix" },
  ],
};
export async function run() {}
"#;
        let meta = parse_workflow_script_meta(script).expect("meta");
        assert_eq!(meta["name"], "Repo sweep");
        assert_eq!(meta["description"], "scans everything");
        assert_eq!(meta["phases"].as_array().unwrap().len(), 2);
        assert_eq!(meta["phases"][0]["detail"], "read the tree");
    }

    #[test]
    fn meta_rejects_computed_values() {
        let script = "export const meta = { name: compute() };";
        assert!(parse_workflow_script_meta(script).is_none());
        let template = "export const meta = { name: `x${y}` };";
        assert!(parse_workflow_script_meta(template).is_none());
    }

    #[test]
    fn meta_without_any_known_field_is_none() {
        assert!(parse_workflow_script_meta("export const meta = { other: 1 };").is_none());
        assert!(parse_workflow_script_meta("const meta = { name: 'x' };").is_none());
    }

    #[test]
    fn agent_plans_skip_comments_and_unwrap_escapes() {
        let script = r#"
agent("scan the repo", { label: "scanner", phase: "Scan", model: "opus" });
agent(" Scan the repo", {
  // label: "fake", phase: "Wrong",
  label: 'real',
  phase: 'Scan2',
});
agent("second", { label: "fixer", effort: "high" });
agent("third", { label: "plain" });
"#;
        let plans = extract_workflow_agent_plans(script).expect("plans");
        let obj = plans.as_object().unwrap();
        assert_eq!(obj.len(), 3, "commented-out options elided, opt-less agents excluded");
        assert_eq!(obj["real"]["phase"], "Scan2", "commented-out fake option must not win");
        assert!(obj["real"].get("label").is_none());
        assert_eq!(obj["scanner"]["phase"], "Scan");
        assert_eq!(obj["scanner"]["model"], "opus");
        assert_eq!(obj["fixer"]["effort"], "high");
        let phases = extract_workflow_agent_phases(script).expect("phases");
        assert_eq!(phases["scanner"], "Scan");
        assert!(phases.get("fixer").is_none());
    }

    #[test]
    fn launch_parsing_structured_and_text() {
        let structured = serde_json::json!({
            "taskType": "local_workflow",
            "taskId": "t-1",
            "runId": "wf_abc123",
            "scriptPath": "/tmp/wf.js",
            "transcriptDir": "/tmp/wf",
        });
        let launch = parse_workflow_launch(&structured).expect("launch");
        assert_eq!(launch["runId"], "wf_abc123");
        assert_eq!(launch["taskId"], "t-1");

        let wrong_type = serde_json::json!({"taskType": "other", "runId": "wf_abc123"});
        assert!(parse_workflow_launch(&wrong_type).is_none());

        let text = "Workflow started as wf_z9y8x7.\nScript persisted to /tmp/run-1/wf.js\n";
        let from_text = parse_workflow_launch_from_text(text).expect("text launch");
        assert_eq!(from_text["runId"], "wf_z9y8x7");
        assert_eq!(from_text["scriptPath"], "/tmp/run-1/wf.js");
    }

    #[test]
    fn progress_agents_from_output_json() {
        let content = r#"{"workflowProgress":[
            {"type":"workflow_agent","label":"scanner","phaseIndex":0,"agentId":"a1","model":"opus","tokens":1500,"toolCalls":7,"promptPreview":"scan it"},
            {"type":"other","label":"skip"}
        ]}"#;
        let agents = parse_workflow_progress_agents(content).expect("agents");
        let arr = agents.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["label"], "scanner");
        assert_eq!(arr[0]["tokens"], 1500);
        assert_eq!(arr[0]["promptPreview"], "scan it");
        assert!(parse_workflow_progress_agents("{}").is_none());
    }
}
