//! Account usage synchronization, a compact native port of upstream
//! `accountUsageReporter.ts`. The server recomputes recent per-minute buckets
//! from local projections and pushes absolute values; the account service
//! upserts them, so overlaps and retries are idempotent. Only counters and
//! model/skill dimensions leave the machine — never prompts or content.

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;

const RECENT_WINDOW: &str = "-2 days";
const BACKFILL_WINDOW: &str = "-20 years";
const MAX_BUCKETS: usize = 500;

fn enabled() -> bool {
    let flavor = match std::env::var("SYNARA_DESKTOP_BUNDLE_ID")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("com.emanueledipietro.synara") => "production",
        Some("com.emanueledipietro.synara.remote") => "remote",
        _ => "other",
    };
    std::env::var("SYNARA_ACCOUNT_PROFILE_SYNC").ok().as_deref() == Some("1")
        || ((flavor == "production" || flavor == "remote")
            && std::env::var("SYNARA_ACCOUNT_PROFILE_SYNC").is_err())
}

fn minute(iso: &str) -> Option<String> {
    (iso.len() >= 16).then(|| format!("{}:00Z", &iso[..16]))
}

fn dimension(value: Option<String>, fallback: &str) -> String {
    let value = value.unwrap_or_default();
    let trimmed = value.trim();
    let selected = if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    };
    selected.chars().take(200).collect()
}

fn tokens_from_usage(raw: Option<&str>) -> u64 {
    let Some(raw) = raw else { return 0 };
    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(raw) else {
        return 0;
    };
    object
        .values()
        .map(|entry| {
            entry
                .get("inputTokens")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                + entry
                    .get("outputTokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
        })
        .sum()
}

fn model_buckets(conn: &Connection, window: &str) -> Result<Vec<Value>, String> {
    let mut grouped: HashMap<String, Value> = HashMap::new();
    let mut statement = conn
        .prepare(
            "SELECT t.requested_at, \
              json_extract(e.payload_json, '$.modelSelection.provider') AS provider, \
              json_extract(e.payload_json, '$.modelSelection.model') AS model, \
              COALESCE(json_extract(e.payload_json, '$.modelSelection.options.effort'), \
                json_extract(e.payload_json, '$.modelSelection.options.reasoningEffort'), \
                json_extract(e.payload_json, '$.modelSelection.options.thinkingLevel'), \
                json_extract(e.payload_json, '$.modelSelection.options.variant')) AS reasoning \
             FROM projection_turns t LEFT JOIN orchestration_events e ON e.event_id = ( \
               SELECT r.event_id FROM orchestration_events r WHERE r.event_type = 'thread.turn-start-requested' \
                 AND r.stream_id = t.thread_id AND json_extract(r.payload_json, '$.messageId') = t.pending_message_id \
               ORDER BY r.stream_version DESC LIMIT 1) \
             WHERE julianday(t.requested_at) >= julianday('now', ?1)",
        )
        .map_err(|e| format!("prepare account usage turns: {e}"))?;
    let rows = statement
        .query_map(params![window], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| format!("read account usage turns: {e}"))?;
    for row in rows {
        let (requested_at, provider, model, reasoning) = row.map_err(|e| e.to_string())?;
        let Some(minute) = minute(&requested_at) else {
            continue;
        };
        let provider = dimension(provider, "unknown");
        let model = dimension(model, "unknown");
        let reasoning = reasoning.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.chars().take(200).collect::<String>())
        });
        let key = format!(
            "{minute}\0{provider}\0{model}\0{}",
            reasoning.clone().unwrap_or_default()
        );
        let entry = grouped.entry(key).or_insert_with(|| {
            json!({
                "minute": minute,
                "provider": provider,
                "model": model,
                "reasoning": reasoning,
                "tokens": 0,
                "turns": 0,
                "prompts": 0,
            })
        });
        entry["turns"] = json!(entry["turns"].as_u64().unwrap_or(0) + 1);
        entry["prompts"] = json!(entry["prompts"].as_u64().unwrap_or(0) + 1);
    }
    drop(statement);

    let mut token_statement = conn
        .prepare(
            "SELECT t.completed_at, \
              json_extract(e.payload_json, '$.modelSelection.provider') AS provider, \
              json_extract(e.payload_json, '$.modelSelection.model') AS model, \
              COALESCE(json_extract(e.payload_json, '$.modelSelection.options.effort'), \
                json_extract(e.payload_json, '$.modelSelection.options.reasoningEffort'), \
                json_extract(e.payload_json, '$.modelSelection.options.thinkingLevel'), \
                json_extract(e.payload_json, '$.modelSelection.options.variant')) AS reasoning, \
              (SELECT json_extract(a.payload_json, '$.modelUsage') FROM projection_thread_activities a \
               WHERE a.kind = 'turn.completed' AND a.thread_id = t.thread_id AND a.turn_id = t.turn_id \
               ORDER BY a.created_at DESC, a.activity_id DESC LIMIT 1) AS usage \
             FROM projection_turns t LEFT JOIN orchestration_events e ON e.event_id = ( \
               SELECT r.event_id FROM orchestration_events r WHERE r.event_type = 'thread.turn-start-requested' \
                 AND r.stream_id = t.thread_id AND json_extract(r.payload_json, '$.messageId') = t.pending_message_id \
               ORDER BY r.stream_version DESC LIMIT 1) \
             WHERE t.completed_at IS NOT NULL AND julianday(t.completed_at) >= julianday('now', ?1)",
        )
        .map_err(|e| format!("prepare account usage tokens: {e}"))?;
    let rows = token_statement
        .query_map(params![window], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| format!("read account usage tokens: {e}"))?;
    for row in rows {
        let (completed_at, provider, model, reasoning, usage) = row.map_err(|e| e.to_string())?;
        let Some(minute) = minute(&completed_at) else {
            continue;
        };
        let provider = dimension(provider, "unknown");
        let model = dimension(model, "unknown");
        let reasoning = reasoning.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.chars().take(200).collect::<String>())
        });
        let key = format!(
            "{minute}\0{provider}\0{model}\0{}",
            reasoning.clone().unwrap_or_default()
        );
        let entry = grouped.entry(key).or_insert_with(|| {
            json!({
                "minute": minute,
                "provider": provider,
                "model": model,
                "reasoning": reasoning,
                "tokens": 0,
                "turns": 0,
                "prompts": 0,
            })
        });
        let tokens = tokens_from_usage(usage.as_deref());
        entry["tokens"] = json!(entry["tokens"].as_u64().unwrap_or(0) + tokens);
    }
    let mut buckets: Vec<Value> = grouped.into_values().collect();
    buckets.sort_by_key(|bucket| bucket["minute"].as_str().unwrap_or("").to_string());
    buckets.truncate(MAX_BUCKETS);
    Ok(buckets)
}

fn reference_names(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw else { return Vec::new() };
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| entry.get("name").and_then(Value::as_str))
        .filter_map(|name| {
            let normalized = name.trim_start_matches(['$', '/', '@']).trim();
            (!normalized.is_empty()).then(|| normalized.chars().take(200).collect::<String>())
        })
        .collect()
}

fn skill_buckets(conn: &Connection, window: &str) -> Result<Vec<Value>, String> {
    let mut grouped: HashMap<String, Value> = HashMap::new();
    let mut statement = conn
        .prepare(
            "SELECT created_at, skills_json, mentions_json FROM projection_thread_messages \
             WHERE role = 'user' AND julianday(created_at) >= julianday('now', ?1)",
        )
        .map_err(|e| format!("prepare account usage skills: {e}"))?;
    let rows = statement
        .query_map(params![window], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| format!("read account usage skills: {e}"))?;
    for row in rows {
        let (created_at, skills_json, mentions_json) = row.map_err(|e| e.to_string())?;
        let Some(minute) = minute(&created_at) else {
            continue;
        };
        for (kind, names) in [
            ("skill", reference_names(skills_json.as_deref())),
            ("agent", reference_names(mentions_json.as_deref())),
        ] {
            for name in names {
                let key = format!("{minute}\0{kind}\0{}", name.to_lowercase());
                let entry = grouped.entry(key).or_insert_with(|| {
                    json!({
                        "minute": minute,
                        "name": name,
                        "kind": kind,
                        "runs": 0,
                    })
                });
                entry["runs"] = json!(entry["runs"].as_u64().unwrap_or(0) + 1);
            }
        }
    }
    let mut buckets: Vec<Value> = grouped.into_values().collect();
    buckets.sort_by_key(|bucket| bucket["minute"].as_str().unwrap_or("").to_string());
    Ok(buckets)
}

fn chunks_by_minute(buckets: Vec<Value>) -> Vec<Vec<Value>> {
    let mut chunks: Vec<Vec<Value>> = Vec::new();
    let mut current: Vec<Value> = Vec::new();
    let mut index = 0;
    while index < buckets.len() {
        let minute = buckets[index]["minute"].as_str().unwrap_or("").to_string();
        let mut end = index;
        while end < buckets.len() && buckets[end]["minute"].as_str() == Some(minute.as_str()) {
            end += 1;
        }
        let group_len = end - index;
        if !current.is_empty() && current.len() + group_len > MAX_BUCKETS {
            chunks.push(std::mem::take(&mut current));
        }
        current.extend(buckets[index..end].iter().cloned());
        index = end;
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn sync_state_path() -> PathBuf {
    std::path::Path::new(&crate::todo::state_dir()).join("account-usage-sync.json")
}

fn backfill_completed() -> bool {
    std::fs::read_to_string(sync_state_path())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.get("backfillCompleted").and_then(Value::as_bool))
        == Some(true)
}

fn mark_backfill_completed() {
    let path = sync_state_path();
    let body = json!({"version": 1, "backfillCompleted": true}).to_string();
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(tmp, path);
    }
}

fn account_file_exists() -> bool {
    let state = PathBuf::from(crate::todo::state_dir());
    let root = if state.file_name().and_then(|name| name.to_str()) == Some("userdata") {
        state.parent().map(PathBuf::from).unwrap_or(state)
    } else {
        state
    };
    root.join("account-credentials.json").is_file()
}

fn flush() -> Result<(), String> {
    if !enabled() || !account_file_exists() {
        return Ok(());
    }
    let state_dir = crate::todo::state_dir();
    let db_path = std::path::Path::new(&state_dir).join("state.sqlite");
    if !db_path.is_file() {
        return Ok(());
    }
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("open state database for account usage: {e}"))?;
    let window = if backfill_completed() {
        RECENT_WINDOW
    } else {
        BACKFILL_WINDOW
    };
    let models = model_buckets(&conn, window)?;
    let skills = skill_buckets(&conn, window)?;
    if models.is_empty() && skills.is_empty() {
        return Ok(());
    }
    let environment_id = crate::server_env::environment_id(&state_dir)
        .ok_or_else(|| "environment id unavailable".to_string())?;
    let model_chunks = chunks_by_minute(models);
    let skill_chunks = chunks_by_minute(skills);
    let pushes = model_chunks.len().max(skill_chunks.len());
    for index in 0..pushes {
        let body = json!({
            "environmentId": environment_id,
            "models": model_chunks.get(index).cloned().unwrap_or_default(),
            "skills": skill_chunks.get(index).cloned().unwrap_or_default(),
        });
        crate::account::authenticated_request("Failed to sync account usage", |client, access| {
            client.request(
                reqwest::Method::POST,
                "/api/v1/usage",
                Some(access),
                Some(body.clone()),
                None,
                std::time::Duration::from_secs(15),
            )
        })
        .map_err(|e| e.0)?;
        if index + 1 < pushes {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    if window == BACKFILL_WINDOW {
        mark_backfill_completed();
    }
    Ok(())
}

pub fn start_account_usage_reporter_job() {
    if !enabled() {
        return;
    }
    std::thread::spawn(|| {
        loop {
            if let Err(error) = flush() {
                eprintln!("account usage sync failed: {error}");
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minute_is_fixed_utc_shape() {
        assert_eq!(
            minute("2026-11-27T07:41:23.456Z").as_deref(),
            Some("2026-11-27T07:41:00Z")
        );
        assert_eq!(minute("short"), None);
    }

    #[test]
    fn usage_tokens_sum_model_entries() {
        assert_eq!(
            tokens_from_usage(Some(
                r#"{"claude":{"inputTokens":10,"outputTokens":5},"x":{"inputTokens":2,"outputTokens":1}}"#
            )),
            18
        );
    }

    #[test]
    fn structured_reference_names_are_normalized() {
        assert_eq!(
            reference_names(Some(r#"[{"name":"$rust-anything"},{"name":"/review"}]"#)),
            vec!["rust-anything", "review"]
        );
    }
}
