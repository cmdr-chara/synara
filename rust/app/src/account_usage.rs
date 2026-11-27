//! Account usage synchronization, a compact native port of upstream
//! `accountUsageReporter.ts`. The server recomputes recent per-minute buckets
//! from local projections and pushes absolute values; the account service
//! upserts them, so overlaps and retries are idempotent. Only counters and
//! model/skill dimensions leave the machine — never prompts or content.

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

const WINDOW: &str = "-2 days";
const MAX_BUCKETS: usize = 500;

fn enabled() -> bool {
    let flavor = match std::env::var("SYNARA_DESKTOP_BUNDLE_ID").ok().as_deref().map(str::trim) {
        Some("com.emanueledipietro.synara") => "production",
        Some("com.emanueledipietro.synara.remote") => "remote",
        _ => "other",
    };
    std::env::var("SYNARA_ACCOUNT_PROFILE_SYNC").ok().as_deref() == Some("1")
        || ((flavor == "production" || flavor == "remote") && std::env::var("SYNARA_ACCOUNT_PROFILE_SYNC").is_err())
}

fn minute(iso: &str) -> Option<String> {
    (iso.len() >= 16).then(|| format!("{}:00Z", &iso[..16]))
}

fn dimension(value: Option<String>, fallback: &str) -> String {
    let value = value.unwrap_or_default();
    let trimmed = value.trim();
    let selected = if trimmed.is_empty() { fallback } else { trimmed };
    selected.chars().take(200).collect()
}

fn tokens_from_usage(raw: Option<&str>) -> u64 {
    let Some(raw) = raw else { return 0 };
    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(raw) else { return 0 };
    object
        .values()
        .map(|entry| {
            entry.get("inputTokens").and_then(Value::as_u64).unwrap_or(0)
                + entry.get("outputTokens").and_then(Value::as_u64).unwrap_or(0)
        })
        .sum()
}

fn model_buckets(conn: &Connection) -> Result<Vec<Value>, String> {
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
        .query_map(params![WINDOW], |row| {
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
        let Some(minute) = minute(&requested_at) else { continue };
        let provider = dimension(provider, "unknown");
        let model = dimension(model, "unknown");
        let reasoning = reasoning.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.chars().take(200).collect::<String>())
        });
        let key = format!("{minute}\0{provider}\0{model}\0{}", reasoning.clone().unwrap_or_default());
        let entry = grouped.entry(key).or_insert_with(|| json!({
            "minute": minute,
            "provider": provider,
            "model": model,
            "reasoning": reasoning,
            "tokens": 0,
            "turns": 0,
            "prompts": 0,
        }));
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
        .query_map(params![WINDOW], |row| {
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
        let Some(minute) = minute(&completed_at) else { continue };
        let provider = dimension(provider, "unknown");
        let model = dimension(model, "unknown");
        let reasoning = reasoning.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.chars().take(200).collect::<String>())
        });
        let key = format!("{minute}\0{provider}\0{model}\0{}", reasoning.clone().unwrap_or_default());
        let entry = grouped.entry(key).or_insert_with(|| json!({
            "minute": minute,
            "provider": provider,
            "model": model,
            "reasoning": reasoning,
            "tokens": 0,
            "turns": 0,
            "prompts": 0,
        }));
        let tokens = tokens_from_usage(usage.as_deref());
        entry["tokens"] = json!(entry["tokens"].as_u64().unwrap_or(0) + tokens);
    }
    let mut buckets: Vec<Value> = grouped.into_values().collect();
    buckets.sort_by_key(|bucket| bucket["minute"].as_str().unwrap_or("").to_string());
    buckets.truncate(MAX_BUCKETS);
    Ok(buckets)
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
    let conn = Connection::open(&db_path).map_err(|e| format!("open state database for account usage: {e}"))?;
    let models = model_buckets(&conn)?;
    if models.is_empty() {
        return Ok(());
    }
    let environment_id = crate::server_env::environment_id(&state_dir)
        .ok_or_else(|| "environment id unavailable".to_string())?;
    let body = json!({
        "environmentId": environment_id,
        "models": models,
        "skills": [],
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
    Ok(())
}

pub fn start_account_usage_reporter_job() {
    if !enabled() {
        return;
    }
    std::thread::spawn(|| loop {
        if let Err(error) = flush() {
            eprintln!("account usage sync failed: {error}");
        }
        std::thread::sleep(std::time::Duration::from_secs(60));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minute_is_fixed_utc_shape() {
        assert_eq!(minute("2026-11-27T07:41:23.456Z").as_deref(), Some("2026-11-27T07:41:00Z"));
        assert_eq!(minute("short"), None);
    }

    #[test]
    fn usage_tokens_sum_model_entries() {
        assert_eq!(
            tokens_from_usage(Some(r#"{"claude":{"inputTokens":10,"outputTokens":5},"x":{"inputTokens":2,"outputTokens":1}}"#)),
            18
        );
    }
}
