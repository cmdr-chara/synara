//! Product-consent host execution snapshots, frozen once per completed UTC day.
//! Mirrors apps/server/src/hostUsageSnapshot.ts: the spool contains only bounded
//! allowlisted categories and counters — never account, installation, thread,
//! project, prompt, path, or provider payload identifiers.

use rusqlite::{Connection, params};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};

const HOST_USAGE_POLICY_FILE: &str = "host-usage-policy.json";
const HOST_USAGE_DIRECTORY: &str = "host-usage";
const HOST_USAGE_RECEIPTS_FILE: &str = "host-usage-receipts.json";
const HOST_USAGE_DAY_MS: i64 = 86_400_000;
const HOST_USAGE_SETTLE_MS: i64 = 10 * 60 * 1000;
const HOST_USAGE_RECEIPT_DAYS: i64 = 14;
const HOST_USAGE_MAX_ROWS: usize = 100;
const HOST_USAGE_MAX_TURNS: usize = 5_000;
const HOST_USAGE_MAX_COUNTER: u64 = 1_000_000_000_000;
const MAX_READ_BYTES: u64 = 128 * 1024;

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn iso(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_iso_ms(text: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|date| date.timestamp_millis())
}

fn read_json(path: &Path) -> Option<Map<String, Value>> {
    if std::fs::metadata(path).ok()?.len() > MAX_READ_BYTES {
        return None;
    }
    match serde_json::from_str::<Value>(&std::fs::read_to_string(path).ok()?).ok()? {
        Value::Object(object) => Some(object),
        _ => None,
    }
}

fn valid_policy_id(value: &str) -> bool {
    value.len() == 36
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch) || ch == '-')
}

struct Policy {
    policy_id: String,
    since: String,
    since_ms: i64,
}

fn read_policy(root: &Path, now: i64) -> Option<Policy> {
    let policy = read_json(&root.join(HOST_USAGE_POLICY_FILE))?;
    if policy.get("enabled") != Some(&Value::Bool(true)) {
        return None;
    }
    let policy_id = policy.get("policyId")?.as_str()?;
    if !valid_policy_id(policy_id) {
        return None;
    }
    let since = policy.get("since")?.as_str()?;
    let since_ms = parse_iso_ms(since)?;
    if since_ms > now {
        return None;
    }
    Some(Policy {
        policy_id: policy_id.to_string(),
        since: since.to_string(),
        since_ms,
    })
}

fn has_current_consent(root: &Path, policy: &Policy, now: i64) -> bool {
    let Some(current) = read_policy(root, now) else {
        return false;
    };
    if current.policy_id != policy.policy_id {
        return false;
    }
    let consent_path = root.join("consent.json");
    if consent_path.exists() {
        let Some(consent) = read_json(&consent_path) else {
            return false;
        };
        if consent.get("enabled") != Some(&Value::Bool(true)) {
            return false;
        }
    }
    true
}

fn write_atomic(path: &Path, body: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("create spool directory: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
    }
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("snapshot"),
        crate::orchestration_engine::uuid_v4()
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| format!("open snapshot temporary: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600));
        }
        file.write_all(body.as_bytes())
            .map_err(|e| format!("write snapshot: {e}"))?;
        file.sync_all().map_err(|e| format!("sync snapshot: {e}"))?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|e| format!("publish snapshot: {e}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn cleanup_spool(root: &Path, policy: &Policy, now: i64) {
    let spool = root.join(HOST_USAGE_DIRECTORY);
    let Ok(entries) = std::fs::read_dir(&spool) else {
        return;
    };
    let expiry_cutoff = now - HOST_USAGE_RECEIPT_DAYS * HOST_USAGE_DAY_MS;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".json") else {
            continue;
        };
        if stem.len() != 47 || !stem.is_ascii() {
            continue;
        }
        let (id, date) = stem.split_at(36);
        if !id
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch) || ch == '-')
            || !date.starts_with('-')
        {
            continue;
        }
        let date = &date[1..];
        let mut remove = id != policy.policy_id;
        if !remove {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
                let Some(day_start) = date.and_hms_milli_opt(0, 0, 0, 0) else {
                    continue;
                };
                remove = day_start.and_utc().timestamp_millis() + HOST_USAGE_DAY_MS < expiry_cutoff;
            }
        }
        if remove {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn delivered_days(root: &Path, policy: &Policy) -> HashSet<String> {
    let Some(receipts) = read_json(&root.join(HOST_USAGE_RECEIPTS_FILE)) else {
        return HashSet::new();
    };
    if receipts.get("policyId").and_then(Value::as_str) != Some(policy.policy_id.as_str()) {
        return HashSet::new();
    }
    receipts
        .get("days")
        .and_then(Value::as_array)
        .map(|days| {
            days.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn valid_count(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .filter(|count| *count <= HOST_USAGE_MAX_COUNTER)
}

#[derive(Default)]
struct TokenMeasurements {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    truncated: bool,
}

fn token_measurements(raw: Option<&str>) -> TokenMeasurements {
    let mut result = TokenMeasurements::default();
    let Some(raw) = raw else {
        return result;
    };
    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(raw) else {
        return result;
    };
    if object.len() > 64 {
        result.truncated = true;
        return result;
    }
    if object.is_empty() {
        return result;
    }
    let entries: Vec<&Value> = object.values().collect();
    for (field, key) in [
        ("input", "inputTokens"),
        ("output", "outputTokens"),
        ("cached", "cacheReadInputTokens"),
    ] {
        let mut sum = 0_u64;
        let mut measured = true;
        for entry in &entries {
            let Some(value) = entry.get(key).and_then(valid_count) else {
                measured = false;
                break;
            };
            let Some(next) = sum.checked_add(value) else {
                measured = false;
                break;
            };
            if next > HOST_USAGE_MAX_COUNTER {
                measured = false;
                break;
            }
            sum = next;
        }
        if measured {
            match field {
                "input" => result.input_tokens = Some(sum),
                "output" => result.output_tokens = Some(sum),
                _ => result.cached_input_tokens = Some(sum),
            }
        }
    }
    result
}

#[derive(Default, Clone)]
struct UsageRow {
    provider: String,
    model: String,
    effort: String,
    interaction_mode: String,
    runtime_mode: String,
    origin: String,
    outcome: String,
    turns: u64,
    duration_ms: u64,
    duration_samples: u64,
    input_tokens: u64,
    input_samples: u64,
    output_tokens: u64,
    output_samples: u64,
    cached_input_tokens: u64,
    cached_input_samples: u64,
}

struct UsageGroup {
    row: UsageRow,
    overflow: HashSet<&'static str>,
}

impl UsageGroup {
    fn add(&mut self, field: &'static str, value: Option<u64>, capped_measurements: &mut u64) {
        let Some(value) = value else {
            return;
        };
        if self.overflow.contains(field) {
            return;
        }
        let (sum, samples) = match field {
            "durationMs" => (&mut self.row.duration_ms, &mut self.row.duration_samples),
            "inputTokens" => (&mut self.row.input_tokens, &mut self.row.input_samples),
            "outputTokens" => (&mut self.row.output_tokens, &mut self.row.output_samples),
            _ => (
                &mut self.row.cached_input_tokens,
                &mut self.row.cached_input_samples,
            ),
        };
        match sum.checked_add(value) {
            Some(total) if total <= HOST_USAGE_MAX_COUNTER => {
                *sum = total;
                *samples += 1;
            }
            _ => {
                *sum = 0;
                *samples = 0;
                self.overflow.insert(field);
                *capped_measurements += 1;
            }
        }
    }
}

struct TurnRow {
    state: String,
    provider: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    interaction_mode: Option<String>,
    runtime_mode: Option<String>,
    origin: Option<String>,
    duration_ms: Option<f64>,
    usage: Option<String>,
}

fn aggregate_turns(turns: Vec<TurnRow>) -> (Vec<UsageRow>, u64, u64, u64) {
    let mut groups: HashMap<String, UsageGroup> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut token_truncated_turns = 0_u64;
    let mut capped_measurements = 0_u64;
    for turn in turns {
        let outcome = match turn.state.as_str() {
            "completed" => "succeeded",
            "error" => "failed",
            "interrupted" => "cancelled",
            _ => continue,
        };
        let categories = hostusage_rs::port::usage_turn_categories(&json!({
            "provider": turn.provider,
            "model": turn.model,
            "effort": turn.effort,
            "interactionMode": turn.interaction_mode,
            "runtimeMode": turn.runtime_mode,
            "origin": turn.origin,
        }));
        let text = |key: &str| {
            categories
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or("other")
                .to_string()
        };
        let dimensions = [
            text("provider"),
            text("model"),
            text("effort"),
            text("interactionMode"),
            text("runtimeMode"),
            text("origin"),
            outcome.to_string(),
        ];
        let key = dimensions.join("|");
        if !groups.contains_key(&key) {
            if groups.len() >= HOST_USAGE_MAX_ROWS {
                continue;
            }
            order.push(key.clone());
            groups.insert(
                key.clone(),
                UsageGroup {
                    row: UsageRow {
                        provider: dimensions[0].clone(),
                        model: dimensions[1].clone(),
                        effort: dimensions[2].clone(),
                        interaction_mode: dimensions[3].clone(),
                        runtime_mode: dimensions[4].clone(),
                        origin: dimensions[5].clone(),
                        outcome: dimensions[6].clone(),
                        ..UsageRow::default()
                    },
                    overflow: HashSet::new(),
                },
            );
        }
        let Some(group) = groups.get_mut(&key) else {
            continue;
        };
        group.row.turns += 1;
        let duration = turn.duration_ms.and_then(|value| {
            if value.is_finite() && value >= 0.0 && value <= HOST_USAGE_MAX_COUNTER as f64 {
                Some(value.round() as u64)
            } else {
                None
            }
        });
        group.add("durationMs", duration, &mut capped_measurements);
        let tokens = token_measurements(turn.usage.as_deref());
        if tokens.truncated {
            token_truncated_turns += 1;
        }
        group.add("inputTokens", tokens.input_tokens, &mut capped_measurements);
        group.add(
            "outputTokens",
            tokens.output_tokens,
            &mut capped_measurements,
        );
        group.add(
            "cachedInputTokens",
            tokens.cached_input_tokens,
            &mut capped_measurements,
        );
    }
    let rows: Vec<UsageRow> = order
        .into_iter()
        .filter_map(|key| groups.remove(&key).map(|group| group.row))
        .collect();
    let included_turns = rows.iter().map(|row| row.turns).sum();
    (
        rows,
        included_turns,
        token_truncated_turns,
        capped_measurements,
    )
}

fn query_window(
    conn: &mut Connection,
    since_iso: &str,
    window_start: &str,
    window_end: &str,
) -> Result<(u64, u64, Vec<TurnRow>), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("open host usage transaction: {e}"))?;
    let (eligible, missing): (Option<i64>, Option<i64>) = tx
        .query_row(
            "SELECT \
              SUM(CASE WHEN julianday(completed_at) >= julianday(?1) AND julianday(completed_at) < julianday(?2) THEN 1 ELSE 0 END) AS eligible, \
              SUM(CASE WHEN julianday(completed_at) IS NULL AND julianday(requested_at) >= julianday(?1) AND julianday(requested_at) < julianday(?2) THEN 1 ELSE 0 END) AS missing \
             FROM projection_turns \
             WHERE state IN ('completed', 'error', 'interrupted') AND julianday(requested_at) >= julianday(?3)",
            params![window_start, window_end, since_iso],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("count host usage turns: {e}"))?;
    let mut statement = tx
        .prepare(
            "SELECT t.state, \
              json_extract(e.payload_json, '$.modelSelection.provider') AS provider, \
              json_extract(e.payload_json, '$.modelSelection.model') AS model, \
              COALESCE(json_extract(e.payload_json, '$.modelSelection.options.effort'), \
                json_extract(e.payload_json, '$.modelSelection.options.reasoningEffort'), \
                json_extract(e.payload_json, '$.modelSelection.options.thinkingLevel'), \
                json_extract(e.payload_json, '$.modelSelection.options.variant')) AS effort, \
              json_extract(e.payload_json, '$.interactionMode') AS interactionMode, \
              json_extract(e.payload_json, '$.runtimeMode') AS runtimeMode, \
              json_extract(e.payload_json, '$.dispatchOrigin') AS origin, \
              CASE WHEN t.started_at IS NOT NULL THEN (julianday(t.completed_at) - julianday(t.started_at)) * 86400000 END AS durationMs, \
              (SELECT json_extract(a.payload_json, '$.modelUsage') FROM projection_thread_activities a \
               WHERE a.kind = 'turn.completed' AND a.thread_id = t.thread_id AND a.turn_id = t.turn_id \
               ORDER BY a.created_at DESC, a.activity_id DESC LIMIT 1) AS usage \
             FROM projection_turns t LEFT JOIN orchestration_events e ON e.event_id = ( \
               SELECT r.event_id FROM orchestration_events r WHERE r.event_type = 'thread.turn-start-requested' \
                 AND r.stream_id = t.thread_id AND json_extract(r.payload_json, '$.messageId') = t.pending_message_id \
               ORDER BY r.stream_version DESC LIMIT 1) \
             WHERE t.state IN ('completed', 'error', 'interrupted') AND julianday(t.requested_at) >= julianday(?3) \
               AND julianday(t.completed_at) >= julianday(?1) AND julianday(t.completed_at) < julianday(?2) \
             ORDER BY julianday(t.completed_at), t.thread_id, t.turn_id LIMIT ?4",
        )
        .map_err(|e| format!("prepare host usage turns: {e}"))?;
    let rows = statement
        .query_map(
            params![
                window_start,
                window_end,
                since_iso,
                HOST_USAGE_MAX_TURNS as i64
            ],
            |row| {
                Ok(TurnRow {
                    state: row.get(0)?,
                    provider: row.get(1)?,
                    model: row.get(2)?,
                    effort: row.get(3)?,
                    interaction_mode: row.get(4)?,
                    runtime_mode: row.get(5)?,
                    origin: row.get(6)?,
                    duration_ms: row.get(7)?,
                    usage: row.get(8)?,
                })
            },
        )
        .map_err(|e| format!("read host usage turns: {e}"))?;
    let mut turns = Vec::new();
    for row in rows {
        turns.push(row.map_err(|e| format!("decode host usage turn: {e}"))?);
    }
    drop(statement);
    tx.commit()
        .map_err(|e| format!("close host usage transaction: {e}"))?;
    Ok((
        eligible.unwrap_or(0).max(0) as u64,
        missing.unwrap_or(0).max(0) as u64,
        turns,
    ))
}

fn usage_row_json(row: &UsageRow) -> Value {
    json!({
        "provider": row.provider,
        "model": row.model,
        "effort": row.effort,
        "interactionMode": row.interaction_mode,
        "runtimeMode": row.runtime_mode,
        "origin": row.origin,
        "outcome": row.outcome,
        "turns": row.turns,
        "durationMs": row.duration_ms,
        "durationSamples": row.duration_samples,
        "inputTokens": row.input_tokens,
        "inputSamples": row.input_samples,
        "outputTokens": row.output_tokens,
        "outputSamples": row.output_samples,
        "cachedInputTokens": row.cached_input_tokens,
        "cachedInputSamples": row.cached_input_samples,
    })
}

/// Generates unsent, settled UTC days for the current local consent generation.
/// A missing/disabled policy does no projection work, including on Stable.
pub fn write_host_usage_snapshots(
    home_dir: &Path,
    db_path: &Path,
    now: i64,
) -> Result<usize, String> {
    let root = home_dir.join("product-analytics");
    let Some(policy) = read_policy(&root, now) else {
        return Ok(0);
    };
    if !has_current_consent(&root, &policy, now) {
        return Ok(0);
    }
    cleanup_spool(&root, &policy, now);
    let delivered = delivered_days(&root, &policy);
    let spool = root.join(HOST_USAGE_DIRECTORY);
    let first = (policy.since_ms.div_euclid(HOST_USAGE_DAY_MS) * HOST_USAGE_DAY_MS).max(
        (now - HOST_USAGE_RECEIPT_DAYS * HOST_USAGE_DAY_MS).div_euclid(HOST_USAGE_DAY_MS)
            * HOST_USAGE_DAY_MS,
    );
    let mut written = 0_usize;
    let mut start = first;
    while start + HOST_USAGE_DAY_MS + HOST_USAGE_SETTLE_MS <= now {
        let window_start = iso(start);
        let window_end = iso(start + HOST_USAGE_DAY_MS);
        let day = &window_start[..10];
        let file = spool.join(format!("{}-{day}.json", policy.policy_id));
        if delivered.contains(day) || file.exists() {
            start += HOST_USAGE_DAY_MS;
            continue;
        }
        let covered_from = iso(start.max(policy.since_ms));
        let mut conn = Connection::open(db_path)
            .map_err(|e| format!("open state database for host usage: {e}"))?;
        let (eligible_turns, missing_completion_times, turns) =
            query_window(&mut conn, &policy.since, &window_start, &window_end)?;
        let (rows, included_turns, token_truncated_turns, capped_measurements) =
            aggregate_turns(turns);
        let snapshot = json!({
            "v": 1,
            "policyId": policy.policy_id,
            "windowStart": window_start,
            "windowEnd": window_end,
            "coveredFrom": covered_from,
            "generatedAt": iso(now),
            "eligibleTurns": eligible_turns,
            "missingCompletionTimes": missing_completion_times,
            "includedTurns": included_turns,
            "tokenTruncatedTurns": token_truncated_turns,
            "cappedMeasurements": capped_measurements,
            "rows": rows.iter().map(usage_row_json).collect::<Vec<_>>(),
        });
        if !has_current_consent(&root, &policy, now) {
            return Ok(written);
        }
        write_atomic(
            &file,
            &format!(
                "{}\n",
                serde_json::to_string(&snapshot).map_err(|e| e.to_string())?
            ),
        )?;
        if !has_current_consent(&root, &policy, now) {
            let _ = std::fs::remove_file(&file);
            return Ok(written);
        }
        written += 1;
        start += HOST_USAGE_DAY_MS;
    }
    Ok(written)
}

/// Starts the upstream one-minute maintenance loop. Without an enabled local
/// policy this returns before opening SQLite, so Stable and opted-out hosts do
/// no product-analytics projection work.
pub fn start_host_usage_snapshot_job() {
    std::thread::spawn(|| {
        loop {
            let state_dir = PathBuf::from(crate::todo::state_dir());
            let home = state_dir
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| state_dir.clone());
            let db = state_dir.join("state.sqlite");
            if let Err(error) = write_host_usage_snapshots(&home, &db, now_ms()) {
                eprintln!("host usage snapshot write failed: {error}");
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(text: &str) -> i64 {
        parse_iso_ms(text).unwrap()
    }

    fn setup_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE projection_turns (
              row_id INTEGER PRIMARY KEY AUTOINCREMENT,
              thread_id TEXT NOT NULL,
              turn_id TEXT,
              pending_message_id TEXT,
              assistant_message_id TEXT,
              state TEXT NOT NULL,
              requested_at TEXT NOT NULL,
              started_at TEXT,
              completed_at TEXT,
              checkpoint_turn_count INTEGER,
              checkpoint_ref TEXT,
              checkpoint_status TEXT,
              checkpoint_files_json TEXT NOT NULL DEFAULT '[]'
            );
            CREATE TABLE orchestration_events (
              event_id TEXT PRIMARY KEY,
              stream_id TEXT NOT NULL,
              stream_version INTEGER NOT NULL,
              event_type TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              created_at TEXT NOT NULL
            );
            CREATE TABLE projection_thread_activities (
              activity_id TEXT PRIMARY KEY,
              thread_id TEXT NOT NULL,
              turn_id TEXT,
              tone TEXT NOT NULL,
              kind TEXT NOT NULL,
              summary TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              created_at TEXT NOT NULL
            );",
        )
        .unwrap();
    }

    fn write_policy(root: &Path, enabled: bool) -> String {
        let analytics = root.join("product-analytics");
        std::fs::create_dir_all(&analytics).unwrap();
        let policy_id = "11111111-1111-4111-8111-111111111111";
        std::fs::write(
            analytics.join(HOST_USAGE_POLICY_FILE),
            json!({
                "enabled": enabled,
                "policyId": policy_id,
                "since": "2026-01-01T00:00:00.000Z",
            })
            .to_string(),
        )
        .unwrap();
        policy_id.to_string()
    }

    #[test]
    fn missing_policy_does_not_require_database() {
        let root = std::env::temp_dir().join(format!(
            "host-usage-disabled-{}",
            crate::orchestration_engine::uuid_v4()
        ));
        let written = write_host_usage_snapshots(
            &root,
            &root.join("missing.sqlite"),
            ms("2026-01-02T00:10:00.000Z"),
        )
        .unwrap();
        assert_eq!(written, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn completed_day_is_bounded_categorized_and_frozen() {
        let root = std::env::temp_dir().join(format!(
            "host-usage-test-{}",
            crate::orchestration_engine::uuid_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = root.join("state.sqlite");
        setup_db(&db);
        let policy_id = write_policy(&root, true);
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "INSERT INTO projection_turns (thread_id, turn_id, pending_message_id, state, requested_at, started_at, completed_at, checkpoint_files_json)
             VALUES ('thread-1','turn-1','message-1','completed','2026-01-01T11:59:00.000Z','2026-01-01T12:00:00.000Z','2026-01-01T12:01:00.000Z','[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orchestration_events (event_id, stream_id, stream_version, event_type, payload_json, created_at)
             VALUES ('event-1','thread-1',1,'thread.turn-start-requested',?1,'2026-01-01T11:59:00.000Z')",
            params![json!({
                "messageId":"message-1",
                "modelSelection":{"provider":"codex","model":"gpt-5.5","options":{"effort":"high"}},
                "interactionMode":"default",
                "runtimeMode":"full-access",
                "dispatchOrigin":"user"
            }).to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projection_thread_activities (activity_id, thread_id, turn_id, tone, kind, summary, payload_json, created_at)
             VALUES ('activity-1','thread-1','turn-1','agent','turn.completed','done',?1,'2026-01-01T12:01:00.000Z')",
            params![json!({"modelUsage":{"codex":{"inputTokens":10,"outputTokens":5,"cacheReadInputTokens":2}}}).to_string()],
        )
        .unwrap();
        drop(conn);

        let written =
            write_host_usage_snapshots(&root, &db, ms("2026-01-02T00:10:00.000Z")).unwrap();
        assert_eq!(written, 1);
        let file = root
            .join("product-analytics")
            .join(HOST_USAGE_DIRECTORY)
            .join(format!("{policy_id}-2026-01-01.json"));
        let snapshot: Value =
            serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        assert_eq!(snapshot["eligibleTurns"], 1);
        assert_eq!(snapshot["includedTurns"], 1);
        assert_eq!(snapshot["missingCompletionTimes"], 0);
        let row = &snapshot["rows"][0];
        assert_eq!(row["provider"], "codex");
        assert_eq!(row["model"], "gpt-5.5");
        assert_eq!(row["effort"], "high");
        assert_eq!(row["runtimeMode"], "full-access");
        assert_eq!(row["outcome"], "succeeded");
        assert_eq!(row["durationMs"], 60_000);
        assert_eq!(row["inputTokens"], 10);
        assert_eq!(row["outputTokens"], 5);
        assert_eq!(row["cachedInputTokens"], 2);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn opt_out_stops_generation() {
        let root = std::env::temp_dir().join(format!(
            "host-usage-optout-{}",
            crate::orchestration_engine::uuid_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = root.join("state.sqlite");
        setup_db(&db);
        write_policy(&root, true);
        std::fs::write(
            root.join("product-analytics").join("consent.json"),
            json!({"enabled": false}).to_string(),
        )
        .unwrap();
        let written =
            write_host_usage_snapshots(&root, &db, ms("2026-01-02T00:10:00.000Z")).unwrap();
        assert_eq!(written, 0);
        assert!(
            !root
                .join("product-analytics")
                .join(HOST_USAGE_DIRECTORY)
                .exists()
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
