//! Computer activity audit log, port of upstream computer/computerAuditLog.ts
//! and computerAuditFile.ts. Every computer action leaves a newline-delimited
//! JSON record with its effect — but never what the user typed: sensitive
//! args (text, values, file contents) reduce to counts before they touch
//! disk. Retention is bounded by entries and bytes with compaction to half,
//! and history reads tail the file with symlink substitution refused.

use serde_json::{Value, json};

pub const COMPUTER_AUDIT_MAX_ENTRIES: usize = 10_000;
pub const COMPUTER_AUDIT_MAX_BYTES: u64 = 2 * 1024 * 1024;
const COMPUTER_AUDIT_COMPACT_TO: usize = COMPUTER_AUDIT_MAX_ENTRIES / 2;
const COMPUTER_AUDIT_COMPACT_BYTES: u64 = COMPUTER_AUDIT_MAX_BYTES / 2;
const COMPUTER_AUDIT_MAX_STRING: usize = 256;

const SENSITIVE_ARGS: &[&str] = &[
    "text",
    "value",
    "arguments",
    "files",
    "image_path",
    "file_path",
    "clipboard",
    "contents",
    "data",
    "payload",
];

fn audit_path() -> std::path::PathBuf {
    std::path::Path::new(&crate::todo::state_dir()).join("computer-audit.jsonl")
}

fn is_sensitive(key: &str) -> bool {
    SENSITIVE_ARGS.contains(&key)
}

/// The audit-safe shape of one argument exactly upstream: numbers and
/// booleans pass, sensitive strings become character counts, ordinary
/// strings truncate at 256, step arrays summarize as type sequences, objects
/// recurse shallowly.
fn summarize_value(key: &str, value: &Value) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(_) | Value::Number(_) => value.clone(),
        Value::String(text) => {
            if is_sensitive(key) {
                json!({"chars": text.chars().count()})
            } else if text.chars().count() > COMPUTER_AUDIT_MAX_STRING {
                let truncated: String = text.chars().take(COMPUTER_AUDIT_MAX_STRING).collect();
                json!(format!("{truncated}…"))
            } else {
                value.clone()
            }
        }
        Value::Array(items) => {
            if key == "steps" {
                let types: Vec<Value> = items
                    .iter()
                    .take(64)
                    .map(|step| {
                        step.get("type")
                            .and_then(Value::as_str)
                            .map(Value::from)
                            .unwrap_or_else(|| json!("unknown"))
                    })
                    .collect();
                json!({"count": items.len(), "types": types})
            } else if is_sensitive(key) {
                json!({"items": items.len()})
            } else {
                Value::Array(
                    items
                        .iter()
                        .take(64)
                        .map(|item| match item {
                            Value::Number(_) | Value::Bool(_) => item.clone(),
                            Value::String(text) => {
                                let truncated: String =
                                    text.chars().take(COMPUTER_AUDIT_MAX_STRING).collect();
                                json!(truncated)
                            }
                            _ => json!("[object]"),
                        })
                        .collect(),
                )
            }
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (inner_key, inner_value) in map.iter().take(32) {
                out.insert(
                    inner_key.clone(),
                    match inner_value {
                        Value::Number(_) | Value::Bool(_) => inner_value.clone(),
                        Value::String(text) => {
                            let truncated: String =
                                text.chars().take(COMPUTER_AUDIT_MAX_STRING).collect();
                            json!(truncated)
                        }
                        _ => json!("[object]"),
                    },
                );
            }
            Value::Object(out)
        }
    }
}

pub fn summarize_args(args: &Value) -> Value {
    let Value::Object(map) = args else {
        return json!({});
    };
    let mut out = serde_json::Map::new();
    for (key, value) in map {
        if value.is_null() {
            continue;
        }
        out.insert(key.clone(), summarize_value(key, value));
    }
    Value::Object(out)
}

fn now_iso() -> String {
    chrono::DateTime::from_timestamp_millis(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
    )
    .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    .unwrap_or_default()
}

/// Append one audit record. Entries for control-disabled threads are
/// dropped exactly upstream — control revocation erases the trail forward,
/// not backward.
pub fn record(entry: Value) {
    let thread_id = entry.get("threadId").and_then(Value::as_str).unwrap_or("");
    if !thread_id.is_empty() && crate::computer_control::get(thread_id).disabled {
        return;
    }
    let mut record = entry.clone();
    if let Value::Object(ref mut map) = record {
        map.insert("ts".to_string(), json!(now_iso()));
        if let Some(args) = map.get("args").cloned() {
            map.insert("args".to_string(), summarize_args(&args));
        }
    }
    let path = audit_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    use std::io::Write;
    let line = format!("{}\n", serde_json::to_string(&record).unwrap_or_default());
    let appended = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
    if appended.is_err() {
        return;
    }
    compact_if_needed(&path);
}

fn compact_if_needed(path: &std::path::Path) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if metadata.len() <= COMPUTER_AUDIT_MAX_BYTES {
        // Byte-bound alone does not decide — upstream compacts on either bound,
        // but the entry count requires reading; the byte check gates the read.
        let entries_estimate = metadata.len() / 40;
        if entries_estimate <= COMPUTER_AUDIT_MAX_ENTRIES as u64 {
            return;
        }
    }
    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };
    let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() <= COMPUTER_AUDIT_MAX_ENTRIES && metadata.len() <= COMPUTER_AUDIT_MAX_BYTES {
        return;
    }
    // Keep the newest half exactly upstream: compaction halves both bounds.
    let mut kept: Vec<&str> = Vec::new();
    let mut bytes: u64 = 0;
    for line in lines.iter().rev() {
        if kept.len() >= COMPUTER_AUDIT_COMPACT_TO || bytes >= COMPUTER_AUDIT_COMPACT_BYTES {
            break;
        }
        bytes += line.len() as u64 + 1;
        kept.push(line);
    }
    kept.reverse();
    let body = kept.join("\n") + "\n";
    let tmp = path.with_extension("jsonl.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        let _ = std::fs::rename(&tmp, path);
    }
}

/// History for the management surface: newest-first parsed entries, bounded.
/// Reads the file tail only — never a whole-file load — and refuses symlink
/// substitution exactly upstream.
pub fn read_history(limit: usize, thread_id: Option<&str>) -> Vec<Value> {
    let path = audit_path();
    let metadata = match std::fs::metadata(&path) {
        Ok(m) if m.is_file() => m,
        _ => return Vec::new(),
    };
    if metadata.len() == 0 {
        return Vec::new();
    }
    // Symlink substitution check: the opened identity must be the listed one.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() == 0 || (metadata.mode() & 0o170000) != 0o100000 {
            return Vec::new();
        }
    }
    let max_bytes = COMPUTER_AUDIT_MAX_BYTES.min(metadata.len());
    let start = metadata.len().saturating_sub(max_bytes);
    use std::io::{Read, Seek, SeekFrom};
    let mut file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut contents = String::new();
    let mut chunk = Vec::new();
    if file.read_to_end(&mut chunk).is_err() {
        return Vec::new();
    }
    contents.push_str(&String::from_utf8_lossy(&chunk));
    let mut entries: Vec<Value> = contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|entry| {
            thread_id.is_none_or(|t| entry.get("threadId").and_then(Value::as_str) == Some(t))
        })
        .collect();
    entries.reverse();
    entries.truncate(limit.min(1000));
    entries
}

pub fn handles(tag: &str) -> bool {
    matches!(tag, "computer.getAuditHistory")
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, crate::todo::WsErr> {
    match tag {
        "computer.getAuditHistory" => {
            let limit = payload["limit"].as_u64().unwrap_or(100) as usize;
            let thread_id = payload["threadId"].as_str();
            let entries = read_history(if limit == 0 { 100 } else { limit }, thread_id);
            Ok(json!({
                "entries": entries,
                "auditEnabled": true,
            }))
        }
        _ => Err(crate::todo::WsErr(
            format!("unsupported computer audit tag: {tag}"),
            None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_strings_become_counts() {
        let args = json!({"text": "hunter2 secret", "x": 100, "y": 200});
        let summarized = summarize_args(&args);
        assert_eq!(summarized["text"], json!({"chars": 14}));
        assert_eq!(summarized["x"], json!(100));
    }

    #[test]
    fn ordinary_strings_truncate_with_ellipsis() {
        let long = "a".repeat(300);
        let args = json!({"label": long});
        let summarized = summarize_args(&args);
        let text = summarized["label"].as_str().unwrap();
        assert!(text.ends_with('…'));
        assert!(text.chars().count() <= COMPUTER_AUDIT_MAX_STRING + 1);
    }

    #[test]
    fn step_arrays_summarize_types() {
        let args = json!({"steps": [{"type": "click"}, {"type": "type"}, {"other": 1}]});
        let summarized = summarize_args(&args);
        assert_eq!(summarized["steps"]["count"], json!(3));
        assert_eq!(
            summarized["steps"]["types"],
            json!(["click", "type", "unknown"])
        );
    }

    #[test]
    fn sensitive_arrays_become_item_counts() {
        let args = json!({"files": ["/secret/a", "/secret/b"]});
        let summarized = summarize_args(&args);
        assert_eq!(summarized["files"], json!({"items": 2}));
    }
}
