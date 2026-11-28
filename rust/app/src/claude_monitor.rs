//! Claude Monitor events, ported from upstream `claudeMonitorEvents.ts`: the
//! SDK stream never forwards the `<task-notification>` user turn that wakes the
//! agent, so the transcript tail is read back for Monitor batches. Best-effort
//! by design — filesystem and parse failures yield no events, never an error.

use serde_json::Value;

const TRANSCRIPT_TAIL_BYTES: u64 = 512 * 1024;
const MAX_MONITOR_EVENT_CHARS: usize = 2_000;

#[derive(Default)]
pub struct MonitorCursor {
    session_id: Option<String>,
    path: Option<String>,
    offset: Option<u64>,
    seen_ids: std::collections::HashSet<String>,
}

fn tag_text<'a>(body: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    Some(&body[start..end])
}

pub struct MonitorEvent {
    pub id: String,
    pub created_at: String,
    pub task_id: String,
    pub name: String,
    pub output: String,
    pub outcome: String,
    pub message: String,
}

/// Parse one `<task-notification>` body, upstream parseClaudeMonitorEventNotification.
pub fn parse_monitor_notification(text: &str) -> Option<(String, String, String, String, String)> {
    let body = tag_text(text, "task-notification")?;
    let task_id = tag_text(body, "task-id")?.trim().to_string();
    let event = tag_text(body, "event")?;
    if task_id.is_empty() {
        return None;
    }
    let raw_name = tag_text(body, "summary").unwrap_or("").trim().to_string();
    let mut name = raw_name;
    // `Monitor event: "<name>"` per batch, `Monitor "<name>" stream ended` at the end.
    for prefix in ["Monitor event:", "Monitor"] {
        if let Some(rest) = name.clone().trim_start().strip_prefix(prefix) {
            name = rest.trim_start().to_string();
        }
    }
    if let Some(stripped) = name.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            name = stripped[..end].to_string();
        }
    }
    let status = tag_text(body, "status").map(str::trim).unwrap_or("");
    let outcome = match status {
        "completed" | "failed" | "stopped" => status.to_string(),
        "cancelled" => "stopped".to_string(),
        _ => "updated".to_string(),
    };
    let output: String = event.trim().chars().take(MAX_MONITOR_EVENT_CHARS).collect();
    let display_name: String = {
        let n = name.trim_end().to_string();
        let n = if let Some(idx) = n.to_lowercase().find(" stream ended") {
            n[..idx].to_string()
        } else {
            n
        };
        n.chars().take(MAX_MONITOR_EVENT_CHARS).collect()
    };
    let lines_joined = event
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let mut message = [name.clone(), lines_joined]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" — ");
    if message.is_empty() {
        return None;
    }
    if message.chars().count() > MAX_MONITOR_EVENT_CHARS {
        message = format!(
            "{}…",
            message
                .chars()
                .take(MAX_MONITOR_EVENT_CHARS - 1)
                .collect::<String>()
        );
    }
    Some((task_id, display_name, output, outcome, message))
}

/// Extract text content from a transcript message entry (string or blocks).
fn notification_text(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }
    let blocks = content.as_array()?;
    let text = blocks
        .iter()
        .filter_map(|block| {
            if block.get("type").and_then(Value::as_str) == Some("text") {
                block
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() { None } else { Some(text) }
}

/// Locate the newest `<sessionId>.jsonl` under `<configDir>/projects/*` —
/// upstream sessionFiles never recurses into session directories.
fn find_transcript(config_dir: &str, session_id: &str) -> Option<String> {
    let projects = std::path::Path::new(config_dir).join("projects");
    let target = format!("{session_id}.jsonl");
    let mut newest: Option<(String, std::time::SystemTime)> = None;
    for project in std::fs::read_dir(&projects).ok()?.flatten() {
        if !project.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let candidate = project.path().join(&target);
        if !candidate.is_file() {
            continue;
        }
        let mtime = candidate
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let dominated = newest
            .as_ref()
            .map(|(_, best)| mtime <= *best)
            .unwrap_or(false);
        if !dominated {
            newest = Some((candidate.to_string_lossy().to_string(), mtime));
        }
    }
    newest.map(|(path, _)| path)
}

fn monitor_event_from_line(line: &str, not_before_ms: i64) -> Option<MonitorEvent> {
    let entry: Value = serde_json::from_str(line).ok()?;
    if entry.get("type").and_then(Value::as_str) != Some("user") {
        return None;
    }
    if entry.get("isSidechain").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    if entry.pointer("/origin/kind").and_then(Value::as_str) != Some("task-notification") {
        return None;
    }
    let uuid = entry.get("uuid").and_then(Value::as_str)?.to_string();
    let timestamp = entry.get("timestamp").and_then(Value::as_str)?;
    let created_ms = chrono_ms(timestamp)?;
    if created_ms < not_before_ms {
        return None;
    }
    let text = notification_text(entry.pointer("/message/content")?)?;
    let (task_id, name, output, outcome, message) = parse_monitor_notification(&text)?;
    Some(MonitorEvent {
        id: uuid,
        created_at: timestamp.to_string(),
        task_id,
        name,
        output,
        outcome,
        message,
    })
}

/// ISO-8601 timestamp to epoch milliseconds without pulling in a date crate:
/// the transcripts always write `YYYY-MM-DDTHH:MM:SS.mmmZ`.
fn chrono_ms(timestamp: &str) -> Option<i64> {
    let secs = timestamp.len() >= 20;
    if !secs {
        return None;
    }
    let year: i64 = timestamp.get(0..4)?.parse().ok()?;
    let month: i64 = timestamp.get(5..7)?.parse().ok()?;
    let day: i64 = timestamp.get(8..10)?.parse().ok()?;
    let hour: i64 = timestamp.get(11..13)?.parse().ok()?;
    let minute: i64 = timestamp.get(14..16)?.parse().ok()?;
    let second: i64 = timestamp.get(17..19)?.parse().ok()?;
    let millis: i64 = timestamp
        .get(20..23)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    // Days since epoch, civil calendar arithmetic.
    let days = days_from_civil(year, month, day);
    Some(days * 86_400_000 + hour * 3_600_000 + minute * 60_000 + second * 1_000 + millis)
}

/// Epoch milliseconds to `YYYY-MM-DDTHH:MM:SS.mmmZ`, inverse of chrono_ms.
pub fn iso_from_ms(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (hour, minute, second, millis) = (
        rem / 3_600_000,
        (rem % 3_600_000) / 60_000,
        (rem % 60_000) / 1_000,
        rem % 1_000,
    );
    // Civil-from-days, Howard Hinnant's algorithm inverted.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Read Monitor events persisted since the previous read, oldest first.
/// Never fails: every error path degrades to an empty batch.
pub fn read_monitor_events(
    cursor: &mut MonitorCursor,
    session_id: &str,
    config_dir: &str,
    not_before: &str,
) -> Vec<MonitorEvent> {
    if cursor.session_id.as_deref() != Some(session_id) {
        cursor.session_id = Some(session_id.to_string());
        cursor.path = None;
        cursor.offset = None;
    }
    if cursor.path.is_none() {
        cursor.path = find_transcript(config_dir, session_id);
    }
    let Some(path) = cursor.path.clone() else {
        return Vec::new();
    };
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut offset = cursor.offset.unwrap_or(0);
    if cursor.offset.is_none() || offset > size || size - offset > TRANSCRIPT_TAIL_BYTES {
        offset = size.saturating_sub(TRANSCRIPT_TAIL_BYTES);
    }
    if size <= offset {
        return Vec::new();
    }
    let mut file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    use std::io::{Read, Seek, SeekFrom};
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return Vec::new();
    }
    let mut buf = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }
    // Consume only complete lines; the tail fragment waits for the next read.
    let last_newline = buf.iter().rposition(|b| *b == b'\n');
    let Some(last) = last_newline else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&buf[..last]).to_string();
    cursor.offset = Some(offset + last as u64 + 1);
    let not_before_ms = chrono_ms(not_before).unwrap_or(0);
    let mut events = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(event) = monitor_event_from_line(line, not_before_ms) {
            if cursor.seen_ids.insert(event.id.clone()) {
                events.push(event);
            }
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_batch_notification() {
        let text = "<task-notification><task-id>task-1</task-id><status>updated</status><summary>Monitor event: \"deploy logs\"</summary><event>line one\nline two</event></task-notification>";
        let (task_id, name, output, outcome, message) = parse_monitor_notification(text).unwrap();
        assert_eq!(task_id, "task-1");
        assert_eq!(name, "deploy logs");
        assert_eq!(outcome, "updated");
        assert!(output.contains("line one"));
        assert!(message.contains("line one · line two"));
    }

    #[test]
    fn ignores_non_monitor_notifications() {
        assert!(parse_monitor_notification("plain text").is_none());
        assert!(
            parse_monitor_notification(
                "<task-notification><task-id>t</task-id></task-notification>"
            )
            .is_none()
        );
    }

    #[test]
    fn civil_date_math() {
        assert_eq!(chrono_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(chrono_ms("2025-01-01T00:00:00.000Z"), Some(1735689600000));
    }

    #[test]
    fn reads_incremental_events() {
        let dir = std::env::temp_dir().join(format!("claude-monitor-test-{}", std::process::id()));
        let projects = dir.join("projects").join("proj-slug");
        std::fs::create_dir_all(&projects).unwrap();
        let file = projects.join("sess-1.jsonl");
        let line = serde_json::json!({
            "type": "user", "uuid": "ev-1", "timestamp": "2025-06-01T10:00:00.000Z",
            "origin": {"kind": "task-notification"},
            "message": {"content": "<task-notification><task-id>task-9</task-id><summary>Monitor event: \"tail\"</summary><event>booted</event></task-notification>"},
        });
        std::fs::write(&file, format!("{line}\n")).unwrap();
        let mut cursor = MonitorCursor::default();
        let events = read_monitor_events(
            &mut cursor,
            "sess-1",
            dir.to_str().unwrap(),
            "2025-06-01T09:00:00.000Z",
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_id, "task-9");
        // A second read yields nothing new.
        let events = read_monitor_events(
            &mut cursor,
            "sess-1",
            dir.to_str().unwrap(),
            "2025-06-01T09:00:00.000Z",
        );
        assert!(events.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
