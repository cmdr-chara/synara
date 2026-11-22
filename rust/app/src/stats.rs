//! `stats.*` — profile stats, token stats, recap and usage analytics, computed
//! natively from a byte copy of `state.sqlite` (the upstream process holds an
//! exclusive lock). Aggregation logic lives in the ported `*-rs` crates; this
//! module owns the SQL and the row shaping.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::todo::WsErr;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("STATS"))
}

// ── DB access ─────────────────────────────────────────────────────────────

/// Byte-copy `state.sqlite` (+ WAL, named `<copy>-wal`) into temp so reads work
/// while upstream holds the lock. The guard deletes the copies on drop.
struct StateCopy {
    conn: Connection,
    db: PathBuf,
    wal: PathBuf,
}

impl Drop for StateCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.db);
        let _ = std::fs::remove_file(&self.wal);
    }
}

fn open_state_copy() -> Option<StateCopy> {
    let db = PathBuf::from(crate::todo::state_dir()).join("state.sqlite");
    let tmp = std::env::temp_dir().join(format!("synara-stats-{}.sqlite", std::process::id()));
    let tmp_wal = PathBuf::from(format!("{}-wal", tmp.display()));
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp_wal);
    std::fs::copy(&db, &tmp).ok()?;
    let wal = db.with_file_name("state.sqlite-wal");
    if wal.exists() {
        let _ = std::fs::copy(&wal, &tmp_wal);
    }
    let conn = Connection::open(&tmp).ok()?;
    Some(StateCopy {
        conn,
        db: tmp,
        wal: tmp_wal,
    })
}

/// Run a query with no params, mapping each row to a JSON object keyed by
/// column name. Missing tables/columns yield an empty result, not an error —
/// stats are best-effort over whatever schema the DB has.
fn query_rows(conn: &Connection, sql: &str) -> Vec<Value> {
    query_rows_params(conn, sql, &[])
}

fn query_rows_params(conn: &Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Vec<Value> {
    let Ok(mut stmt) = conn.prepare(sql) else {
        return Vec::new();
    };
    let names: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let Ok(mut rows) = stmt.query(params) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        let mut obj = serde_json::Map::new();
        for (i, name) in names.iter().enumerate() {
            let v: Value = match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                Ok(rusqlite::types::ValueRef::Integer(n)) => json!(n),
                Ok(rusqlite::types::ValueRef::Real(f)) => json!(f),
                Ok(rusqlite::types::ValueRef::Text(t)) => {
                    Value::String(String::from_utf8_lossy(t).into_owned())
                }
                Ok(rusqlite::types::ValueRef::Blob(_)) | Err(_) => Value::Null,
            };
            obj.insert(name.clone(), v);
        }
        out.push(Value::Object(obj));
    }
    out
}

fn query_count(conn: &Connection, sql: &str) -> f64 {
    conn.prepare(sql)
        .and_then(|mut s| s.query_row([], |r| r.get::<_, f64>(0)))
        .unwrap_or(0.0)
}

// ── JS-side helpers ─────────────────────────────────────────────────────────

fn member<'a>(v: &'a Value, k: &str) -> &'a Value {
    static NULL: Value = Value::Null;
    v.get(k).unwrap_or(&NULL)
}

fn s(v: &Value) -> Option<&str> {
    v.as_str()
}

fn n(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// Epoch ms from an ISO-8601 string (`YYYY-MM-DD[THH:MM[:SS[.fff]]][Z|±HH:MM]`),
/// covering the `created_at` / payload formats the projections write.
fn date_parse_ms(v: &Value) -> f64 {
    let Some(t) = v.as_str() else {
        return n(v);
    };
    let b = t.as_bytes();
    if b.len() < 10 {
        return t.trim().parse::<f64>().unwrap_or(f64::NAN);
    }
    let digit = |i: usize| -> Option<i64> {
        let d = b.get(i)?;
        d.is_ascii_digit().then(|| (d - b'0') as i64)
    };
    let take = |i: usize, len: usize| -> Option<i64> {
        (0..len).try_fold(0i64, |acc, k| Some(acc * 10 + digit(i + k)?))
    };
    let (Some(y), Some(mo), Some(d)) = (take(0, 4), take(5, 2), take(8, 2)) else {
        return f64::NAN;
    };
    let mut h = 0;
    let mut mi = 0;
    let mut se = 0;
    let mut frac = 0.0;
    let mut tz_ms = 0i64;
    if b.len() > 10 && b[10] == b'T' {
        h = take(11, 2).unwrap_or(0);
        mi = take(14, 2).unwrap_or(0);
        if b.get(16) == Some(&b':') {
            se = take(17, 2).unwrap_or(0);
            if b.get(19) == Some(&b'.') {
                let mut i = 20;
                let mut div = 1000.0;
                while let Some(dg) = digit(i) {
                    frac += dg as f64 * div;
                    div /= 10.0;
                    i += 1;
                }
            }
        }
        // Trailing Z or ±HH:MM offset.
        if let Some(&sign) = b.get(b.len() - 6).filter(|c| **c == b'+' || **c == b'-') {
            let hh = take(b.len() - 5, 2).unwrap_or(0);
            let mm = take(b.len() - 2, 2).unwrap_or(0);
            tz_ms = (hh * 3_600_000 + mm * 60_000) * if sign == b'-' { -1 } else { 1 };
        }
    }
    // Days from civil (Howard Hinnant).
    let y_adj = y - if mo <= 2 { 1 } else { 0 };
    let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
    let yoe = y_adj - era * 400;
    let mo2 = if mo > 2 { mo - 3 } else { mo + 9 };
    let doy = (153 * mo2 + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    (days * 86_400_000 + h * 3_600_000 + mi * 60_000 + se * 1000) as f64 + frac - tz_ms as f64
}

/// `(YYYY-MM-DD, weekday 0=Sun..6=Sat)` for `ms` shifted by `offset_minutes`.
fn local_day(ms: f64, offset_minutes: f64) -> (String, i64) {
    let shifted = ms + offset_minutes * 60_000.0;
    let days = (shifted / 86_400_000.0).floor() as i64;
    // Civil from days (Hinnant).
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    let weekday = (days + 4).rem_euclid(7); // 1970-01-01 was a Thursday.
    (format!("{y:04}-{mo:02}-{d:02}"), weekday)
}

/// `YYYY-MM-DDTHH:MM` of the UTC quarter hour containing `ms`.
fn hour_bucket(ms: f64) -> String {
    let q = (ms / 900_000.0).floor() * 900_000.0;
    let (day, _) = local_day(q, 0.0);
    let mins = ((q / 60_000.0) as i64).rem_euclid(1440);
    format!("{day}T{:02}:{:02}", mins / 60, mins % 60)
}

/// ISO-8601 `YYYY-MM-DDTHH:MM:SS.sssZ` for an epoch-ms instant.
fn iso_of(ms: f64) -> String {
    let (day, _) = local_day(ms, 0.0);
    let sod = (ms as i64).rem_euclid(86_400_000);
    format!(
        "{day}T{:02}:{:02}:{:02}.{:03}Z",
        sod / 3_600_000,
        (sod / 60_000) % 60,
        (sod / 1000) % 60,
        sod % 1000
    )
}

// ── Shared row sources ──────────────────────────────────────────────────────

/// Native user prompts (live + deleted tombstones), `createdAt`-described.
fn prompt_rows(conn: &Connection, from: Option<&str>, to: Option<&str>) -> Vec<Value> {
    let range = |col: &str| match (from, to) {
        (Some(f), Some(t)) => format!(" AND {col} >= '{f}' AND {col} < '{t}'"),
        _ => String::new(),
    };
    let mut out = query_rows(
        conn,
        &format!(
            "SELECT m.created_at AS createdAt, m.thread_id AS threadId, t.project_id AS projectId, \
             p.title AS projectTitle \
             FROM projection_thread_messages m \
             JOIN projection_threads t ON t.thread_id = m.thread_id \
             LEFT JOIN projection_projects p ON p.project_id = t.project_id \
             WHERE m.role = 'user' AND m.source = 'native'{}",
            range("m.created_at")
        ),
    );
    out.extend(query_rows(
        conn,
        &format!(
            "SELECT d.created_at AS createdAt, d.thread_id AS threadId, d.project_id AS projectId, \
             p.title AS projectTitle \
             FROM profile_stats_deleted_prompts d \
             LEFT JOIN projection_projects p ON p.project_id = d.project_id \
             WHERE 1{}",
            range("d.created_at")
        ),
    ));
    out
}

/// `thread.turn-start-requested` events keyed by thread, for the ported
/// `aggregate_thread_turn_snapshot_rows`.
fn turn_start_events(conn: &Connection) -> HashMap<String, Vec<Value>> {
    let mut by_thread: HashMap<String, Vec<Value>> = HashMap::new();
    for row in query_rows(
        conn,
        "SELECT stream_id AS threadId, payload_json AS payloadJson FROM orchestration_events \
         WHERE event_type = 'thread.turn-start-requested'",
    ) {
        if let Some(t) = s(member(&row, "threadId")) {
            by_thread
                .entry(t.to_string())
                .or_default()
                .push(json!({"payloadJson": member(&row, "payloadJson")}));
        }
    }
    by_thread
}

/// `thread_id → model_selection_json` (parsed) for every projected thread.
fn thread_model_selections(conn: &Connection) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    for row in query_rows(
        conn,
        "SELECT thread_id AS threadId, model_selection_json AS msj FROM projection_threads",
    ) {
        let parsed = s(member(&row, "msj"))
            .and_then(|t| serde_json::from_str::<Value>(t).ok())
            .unwrap_or(Value::Null);
        if let Some(t) = s(member(&row, "threadId")) {
            map.insert(t.to_string(), parsed);
        }
    }
    map
}

/// `thread_id|turn_id → {provider, model, instanceId, dispatchOrigin}` from
/// `thread.turn-start-requested` payloads.
fn turn_selections(conn: &Connection) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    for row in query_rows(
        conn,
        "SELECT stream_id AS threadId, payload_json AS p FROM orchestration_events \
         WHERE event_type = 'thread.turn-start-requested'",
    ) {
        let Some(thread) = s(member(&row, "threadId")).map(str::to_string) else {
            continue;
        };
        let payload: Value = s(member(&row, "p"))
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or(Value::Null);
        let turn = s(member(&payload, "turnId")).unwrap_or("");
        let ms = member(&payload, "modelSelection").clone();
        map.insert(
            format!("{thread}\u{0}{turn}"),
            json!({
                "provider": member(&ms, "provider"),
                "model": member(&ms, "model"),
                "instanceId": member(&ms, "instanceId"),
                "dispatchOrigin": member(&payload, "dispatchOrigin"),
            }),
        );
    }
    map
}

/// `thread_id → project_id` and `project_id → {title, kind, workspaceRoot}`.
fn thread_projects(conn: &Connection) -> (HashMap<String, String>, HashMap<String, Value>) {
    let mut thread_project = HashMap::new();
    for row in query_rows(
        conn,
        "SELECT thread_id AS t, project_id AS p FROM projection_threads",
    ) {
        if let (Some(t), Some(p)) = (s(member(&row, "t")), s(member(&row, "p"))) {
            thread_project.insert(t.to_string(), p.to_string());
        }
    }
    let mut projects = HashMap::new();
    for row in query_rows(
        conn,
        "SELECT project_id AS id, title, workspace_root AS workspaceRoot, kind \
         FROM projection_projects",
    ) {
        if let Some(id) = s(member(&row, "id")) {
            projects.insert(id.to_string(), row);
        }
    }
    (thread_project, projects)
}

// ── Token deltas ────────────────────────────────────────────────────────────
//
// Mirror of `profilestatsarchive_rs::aggregate_thread_token_rows`, extended to
// keep the dispatch origin and the cache-read/split counters the recap and
// usage analytics need. Rows are per-activity payloads plus `createdAt` and the
// turn model-selection fields; the ported fn itself is used for profile token
// stats, where its user-only origin filter is exactly the contract.

struct DeltaRow {
    created_at: Value,
    provider: Option<String>,
    instance_id: Option<String>,
    model: Option<String>,
    origin: String,
    tokens: f64,
    cache_tokens: f64,
    split: Option<(f64, f64, f64, f64)>, // split, cacheRead, cacheWrite, output
}

fn str_field(row: &Value, key: &str) -> Option<String> {
    member(row, key).as_str().map(str::to_string)
}

fn finite_counter(row: &Value, key: &str) -> Option<f64> {
    member(row, key).as_f64().filter(|v| v.is_finite())
}

fn resolve_pm(row: &Value, fb: &Value) -> (Option<String>, Option<String>, Option<String>) {
    let stamped = str_field(row, "provider");
    let fb_provider = str_field(fb, "provider");
    let provider = stamped.clone().or_else(|| fb_provider.clone());
    let instance = str_field(row, "instanceId")
        .or_else(|| str_field(fb, "instanceId"))
        .or_else(|| provider.clone());
    let model = str_field(row, "model").or_else(|| {
        if stamped.is_none() || stamped.as_deref() == fb_provider.as_deref() {
            str_field(fb, "model")
        } else {
            None
        }
    });
    (provider, instance, model)
}

fn pm_key(sel: &(Option<String>, Option<String>, Option<String>)) -> String {
    format!(
        "{}\u{0}{}\u{0}{}",
        sel.0.as_deref().unwrap_or(""),
        sel.1.as_deref().unwrap_or(""),
        sel.2.as_deref().unwrap_or("")
    )
}

fn session_key(row: &Value) -> String {
    member(row, "usageSessionId")
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn counter_key(row: &Value, fb: &Value) -> String {
    let counter = str_field(row, "counterProvider").or_else(|| resolve_pm(row, fb).0);
    format!("{}\u{0}{}", counter.unwrap_or_default(), session_key(row))
}

/// Token deltas for one thread's activity rows (already in stream order),
/// across all dispatch origins. `claudeAgent` context-window rows are skipped
// like the ported aggregate — Claude usage comes from `turn.completed`
// `modelUsage` instead.
fn token_deltas(rows: &[Value], fb: &Value) -> Vec<DeltaRow> {
    let non_claude: Vec<&Value> = rows
        .iter()
        .filter(|r| resolve_pm(r, fb).0.as_deref() != Some("claudeAgent"))
        .collect();

    let cumulative_pms: HashSet<String> = non_claude
        .iter()
        .filter(|r| finite_counter(r, "totalProcessedTokens").is_some())
        .map(|r| format!("{}\u{0}{}", pm_key(&resolve_pm(r, fb)), session_key(r)))
        .collect();

    let mut out: Vec<DeltaRow> = Vec::new();
    let push = |out: &mut Vec<DeltaRow>, row: &Value, delta: f64, cache: f64| {
        let sel = resolve_pm(row, fb);
        let split = match (
            finite_counter(row, "splitTokens"),
            finite_counter(row, "splitCacheTokens"),
            finite_counter(row, "cacheWriteTokens"),
            finite_counter(row, "outputTokens"),
        ) {
            (a, b, c, d) if a.is_some() || b.is_some() || c.is_some() || d.is_some() => Some((
                a.unwrap_or(0.0),
                b.unwrap_or(0.0),
                c.unwrap_or(0.0),
                d.unwrap_or(0.0),
            )),
            _ => None,
        };
        out.push(DeltaRow {
            created_at: member(row, "createdAt").clone(),
            provider: sel.0,
            instance_id: sel.1,
            model: sel.2,
            origin: str_field(row, "dispatchOrigin").unwrap_or_else(|| "user".to_string()),
            tokens: delta,
            cache_tokens: cache,
            split,
        });
    };

    // Cumulative `totalProcessedTokens` deltas per (counterProvider, session).
    let mut prev_total: HashMap<String, f64> = HashMap::new();
    let mut prev_cache: HashMap<String, f64> = HashMap::new();
    for row in &non_claude {
        let Some(total) = finite_counter(row, "totalProcessedTokens") else {
            continue;
        };
        let key = counter_key(row, fb);
        let delta = match prev_total.get(&key).copied() {
            None => total,
            Some(prev) if total < prev => total,
            Some(prev) => (total - prev).max(0.0),
        };
        prev_total.insert(key.clone(), total);
        let cache = finite_counter(row, "cachedInputTokens").map_or(0.0, |c| {
            let d = match prev_cache.get(&key).copied() {
                None => c,
                Some(prev) if c < prev => c,
                Some(prev) => (c - prev).max(0.0),
            };
            prev_cache.insert(key.clone(), c);
            d
        });
        if delta <= 0.0 || member(row, "createdAt").is_null() {
            continue;
        }
        push(&mut out, row, delta, cache);
    }

    // `usedTokens` deltas for provider/models that never report a cumulative
    // total.
    let mut prev_used: HashMap<String, (f64, String)> = HashMap::new();
    for row in &non_claude {
        let sel = resolve_pm(row, fb);
        let pmk = pm_key(&sel);
        if cumulative_pms.contains(&format!("{pmk}\u{0}{}", session_key(row))) {
            continue;
        }
        let Some(total) = finite_counter(row, "usedTokens") else {
            continue;
        };
        let key = counter_key(row, fb);
        let delta = match prev_used.get(&key) {
            None => total,
            Some((prev, prev_pmk)) if total < *prev && pmk != *prev_pmk => total,
            Some((prev, _)) => (total - prev).max(0.0),
        };
        prev_used.insert(key, (total, pmk));
        if delta <= 0.0 || member(row, "createdAt").is_null() {
            continue;
        }
        push(&mut out, row, delta, 0.0);
    }
    out
}

/// Context-window activity rows joined to their turn's model selection, in
/// stream order, grouped by thread.
fn token_activity_rows(
    conn: &Connection,
    turn_sel: &HashMap<String, Value>,
    selections: &HashMap<String, Value>,
) -> Vec<(String, Value, Vec<Value>)> {
    let raw = query_rows(
        conn,
        "SELECT thread_id AS threadId, turn_id AS turnId, created_at AS createdAt, payload_json \
         FROM projection_thread_activities \
         WHERE kind = 'context-window.updated' \
         ORDER BY thread_id, sequence, created_at, activity_id",
    );
    let mut by_thread: HashMap<String, Vec<Value>> = HashMap::new();
    for row in raw {
        let Some(thread) = s(member(&row, "threadId")).map(str::to_string) else {
            continue;
        };
        let mut obj = s(member(&row, "payload_json"))
            .and_then(|t| serde_json::from_str::<Value>(t).ok())
            .unwrap_or(json!({}));
        if !obj.is_object() {
            obj = json!({});
        }
        obj["createdAt"] = member(&row, "createdAt").clone();
        let key = format!("{}\u{0}{}", thread, s(member(&row, "turnId")).unwrap_or(""));
        if let Some(ms) = turn_sel.get(&key) {
            // The emitting provider stays stamped on the payload; model and
            // dispatch origin come from the turn's request.
            if !s(member(&ms, "provider")).unwrap_or("").is_empty()
                && member(&obj, "provider").is_null()
            {
                obj["provider"] = member(&ms, "provider").clone();
            }
            obj["model"] = member(&ms, "model").clone();
            if let Some(iid) = s(member(&ms, "instanceId")) {
                obj["instanceId"] = json!(iid);
            }
            obj["dispatchOrigin"] = member(&ms, "dispatchOrigin").clone();
        }
        by_thread.entry(thread).or_default().push(obj);
    }
    by_thread
        .into_iter()
        .map(|(t, rows)| {
            (
                t.clone(),
                selections.get(&t).cloned().unwrap_or(Value::Null),
                rows,
            )
        })
        .collect()
}

/// Claude `turn.completed` per-model usage rows (multi-model turns attribute
/// exactly), joined to the turn selection for origin/instance.
fn claude_model_usage_rows(conn: &Connection, turn_sel: &HashMap<String, Value>) -> Vec<DeltaRow> {
    let mut out = Vec::new();
    for row in query_rows(
        conn,
        "SELECT thread_id AS threadId, turn_id AS turnId, created_at AS createdAt, payload_json \
         FROM projection_thread_activities WHERE kind = 'turn.completed' \
         ORDER BY thread_id, sequence, created_at, activity_id",
    ) {
        let payload: Value = s(member(&row, "payload_json"))
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or(Value::Null);
        let Some(usage) = member(&payload, "modelUsage").as_object() else {
            continue;
        };
        let thread = s(member(&row, "threadId")).unwrap_or("");
        let key = format!("{}\u{0}{}", thread, s(member(&row, "turnId")).unwrap_or(""));
        let ms = turn_sel.get(&key).cloned().unwrap_or(Value::Null);
        let provider = s(member(&payload, "provider"))
            .map(str::to_string)
            .or_else(|| str_field(&ms, "provider"))
            .unwrap_or_else(|| "claudeAgent".to_string());
        for (model, u) in usage {
            let num = |k: &str| finite_counter(u, k).unwrap_or(0.0);
            let input = num("inputTokens");
            let output = num("outputTokens");
            let cache_read = num("cacheReadInputTokens");
            let cache_write = num("cacheCreationInputTokens");
            let total = num("totalTokens");
            let tokens = if total > 0.0 {
                total
            } else {
                input + output + cache_read + cache_write
            };
            if tokens <= 0.0 {
                continue;
            }
            out.push(DeltaRow {
                created_at: member(&row, "createdAt").clone(),
                provider: Some(provider.clone()),
                instance_id: str_field(&ms, "instanceId"),
                model: Some(model.clone()),
                origin: str_field(&ms, "dispatchOrigin").unwrap_or_else(|| "user".into()),
                tokens,
                cache_tokens: cache_read,
                split: Some((tokens, cache_read, cache_write, output)),
            });
        }
    }
    out
}

fn origin_str(origin: &str) -> &'static str {
    match origin {
        "automation" => "automation",
        "agent" => "agent",
        _ => "user",
    }
}

// ── stats.getProfileStats ───────────────────────────────────────────────────

fn get_profile_stats(payload: &Value) -> Result<Value, WsErr> {
    let offset = n(member(payload, "utcOffsetMinutes"));
    // The ported modifier pins the same local-day math `local_day` applies.
    let _modifier =
        profilestats_rs::port::sqlite_modifier_from_utc_offset_minutes(offset).map_err(err)?;
    let Some(state) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &state.conn;

    // Activity: prompts per local day (live + deleted).
    let prompts = prompt_rows(conn, None, None);
    let mut day_counts: HashMap<String, (f64, i64)> = HashMap::new();
    let mut hour_counts: HashMap<i64, f64> = HashMap::new();
    for row in &prompts {
        let ms = date_parse_ms(member(row, "createdAt"));
        if !ms.is_finite() {
            continue;
        }
        let (day, weekday) = local_day(ms, offset);
        day_counts.entry(day).or_insert((0.0, weekday)).0 += 1.0;
        let local_ms = ms + offset * 60_000.0;
        let hour = ((local_ms / 3_600_000.0) as i64).rem_euclid(24);
        *hour_counts.entry(hour).or_default() += 1.0;
    }
    let total_prompts = prompts.len() as f64;
    let total_threads = query_count(conn, "SELECT COUNT(*) FROM projection_threads")
        + query_count(conn, "SELECT COUNT(*) FROM profile_stats_deleted_threads");

    let mut days: Vec<(String, i64, f64)> = day_counts
        .iter()
        .map(|(d, (c, w))| (d.clone(), *w, *c))
        .collect();
    days.sort_by(|a, b| a.0.cmp(&b.0));
    let sorted_active: Vec<f64> = {
        let mut v: Vec<f64> = days.iter().map(|d| d.2).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        v
    };
    let heatmap: Vec<Value> = days
        .iter()
        .map(|(d, w, c)| {
            let intensity =
                profilestats_rs::port::heatmap_intensity(*c, sorted_active.clone()).unwrap_or(0.0);
            json!({"day": d, "count": c, "weekday": w, "intensity": intensity.max(0.0)})
        })
        .collect();

    // Streaks over consecutive local days.
    let day_set: HashSet<&String> = days.iter().map(|d| &d.0).collect();
    let day_ms = |ymd: &str| -> f64 { date_parse_ms(&json!(format!("{ymd}T00:00:00Z"))) };
    let mut longest = 0i64;
    {
        let mut run = 0i64;
        let mut prev: Option<String> = None;
        for (d, _, _) in &days {
            let consecutive = prev
                .as_deref()
                .is_some_and(|p| (day_ms(d) - day_ms(p)).abs() < 86_400_000.0 * 1.5);
            run = if consecutive { run + 1 } else { 1 };
            longest = longest.max(run);
            prev = Some(d.clone());
        }
    }
    let (today, _) = local_day(now_ms(), offset);
    let mut current = 0i64;
    {
        let mut cursor = today.clone();
        // A streak still counts when today has no prompts yet but yesterday does.
        if !day_set.contains(&cursor)
            && let Some((d, _, _)) = days.last()
            && (day_ms(&cursor) - day_ms(d)).abs() < 86_400_000.0 * 1.5
        {
            cursor = d.clone();
        }
        while day_set.contains(&cursor) {
            current += 1;
            cursor = match local_day(day_ms(&cursor) - 86_400_000.0, 0.0) {
                (d, _) => d,
            };
        }
    }

    // Provider/model usage from turn-start snapshots + deleted aggregates.
    let events = turn_start_events(conn);
    let selections = thread_model_selections(conn);
    let mut turn_rows: Vec<Value> = Vec::new();
    for (thread, sel) in &selections {
        let evs = events.get(thread).cloned().unwrap_or_default();
        if evs.is_empty() {
            continue;
        }
        turn_rows.extend(
            profilestatsarchive_rs::port::aggregate_thread_turn_snapshot_rows(
                Value::Array(evs),
                sel.clone(),
            )
            .unwrap_or_default(),
        );
    }
    for row in query_rows(
        conn,
        "SELECT provider, NULL AS instanceId, model, reasoning, turn_count AS turnCount \
         FROM profile_stats_deleted_turns",
    ) {
        turn_rows.push(row);
    }
    // Merge identical (provider, instanceId, model) rows.
    let mut merged: Vec<(String, String, String, String, f64)> = Vec::new();
    for row in &turn_rows {
        let provider = profilestats_rs::port::normalize_provider_kind(
            member(row, "provider").clone(),
            Some(member(row, "model").clone()),
        )
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".into());
        let instance = str_field(row, "instanceId").unwrap_or_else(|| "unknown".into());
        let model = str_field(row, "model").unwrap_or_else(|| "unknown".into());
        let reasoning = str_field(row, "reasoning").unwrap_or_default();
        let count = n(member(row, "turnCount")).max(0.0);
        if count <= 0.0 {
            continue;
        }
        match merged
            .iter_mut()
            .find(|r| r.0 == provider && r.1 == instance && r.2 == model && r.3 == reasoning)
        {
            Some(r) => r.4 += count,
            None => merged.push((provider, instance, model, reasoning, count)),
        }
    }
    let total_turns: f64 = merged.iter().map(|r| r.4).sum();
    let mut provider_totals: Vec<(String, f64)> = Vec::new();
    let mut reasoning_totals: Vec<(String, f64)> = Vec::new();
    for (provider, _, _, reasoning, count) in &merged {
        if provider != "unknown" {
            match provider_totals.iter_mut().find(|p| &p.0 == provider) {
                Some(p) => p.1 += count,
                None => provider_totals.push((provider.clone(), *count)),
            }
        }
        if !reasoning.is_empty() {
            match reasoning_totals.iter_mut().find(|r| &r.0 == reasoning) {
                Some(r) => r.1 += count,
                None => reasoning_totals.push((reasoning.clone(), *count)),
            }
        }
    }
    provider_totals.sort_by(|a, b| b.1.total_cmp(&a.1));
    reasoning_totals.sort_by(|a, b| b.1.total_cmp(&a.1));
    merged.sort_by(|a, b| b.4.total_cmp(&a.4));
    let provider_models: Vec<Value> = merged
        .iter()
        .map(|(p, i, m, _, c)| {
            json!({
                "provider": p,
                "instanceId": i,
                "model": m,
                "turnCount": c,
                "percent": if total_turns > 0.0 { c / total_turns * 100.0 } else { 0.0 },
            })
        })
        .collect();

    // Skills: message skill/mention references + deleted aggregates.
    let skill_rows = query_rows(
        conn,
        "SELECT skills_json AS skillsJson, text, mentions_json AS mentionsJson \
         FROM projection_thread_messages",
    );
    let archived_skills = query_rows(
        conn,
        "SELECT name, kind, run_count AS runCount FROM profile_stats_deleted_skills",
    );
    let skills = profilestats_rs::port::aggregate_profile_skill_usage_rows(
        Value::Array(skill_rows),
        Value::Array(archived_skills),
    )
    .unwrap_or_default();

    // Most worked real project.
    let (thread_project, projects) = thread_projects(conn);
    let mut per_project: HashMap<String, (f64, HashSet<String>, HashSet<String>, f64)> =
        HashMap::new();
    for row in &prompts {
        let pid = s(member(row, "projectId"))
            .map(str::to_string)
            .or_else(|| s(member(row, "threadId")).and_then(|t| thread_project.get(t).cloned()));
        let Some(pid) = pid else { continue };
        let Some(project) = projects.get(&pid) else {
            continue;
        };
        if s(member(project, "kind")) != Some("project") {
            continue;
        }
        let entry = per_project.entry(pid).or_default();
        entry.0 += 1.0;
        if let Some(t) = s(member(row, "threadId")) {
            entry.1.insert(t.to_string());
        }
        let ms = date_parse_ms(member(row, "createdAt"));
        if ms.is_finite() {
            entry.2.insert(local_day(ms, offset).0);
            entry.3 = entry.3.max(ms);
        }
    }
    let mut best_project: Option<Value> = None;
    for (pid, (prompt_count, _threads, day_set_p, last_ms)) in &per_project {
        let Some(project) = projects.get(pid) else {
            continue;
        };
        let thread_count = conn
            .prepare(
                "SELECT (SELECT COUNT(*) FROM projection_threads WHERE project_id = ?1) \
                 + (SELECT COUNT(*) FROM profile_stats_deleted_threads WHERE project_id = ?1)",
            )
            .and_then(|mut st| st.query_row([pid], |r| r.get::<_, f64>(0)))
            .unwrap_or(0.0);
        let row = json!({
            "projectId": pid,
            "title": member(project, "title"),
            "workspaceRoot": member(project, "workspaceRoot"),
            "promptCount": prompt_count,
            "threadCount": thread_count,
            "activeDays": day_set_p.len(),
            "lastWorkedAt": iso_of(*last_ms),
        });
        let beat = best_project.as_ref().is_none_or(|b| {
            n(member(&row, "promptCount")) > n(member(b, "promptCount"))
                || (n(member(&row, "promptCount")) == n(member(b, "promptCount"))
                    && date_parse_ms(member(&row, "lastWorkedAt"))
                        > date_parse_ms(member(b, "lastWorkedAt")))
        });
        if beat {
            best_project = Some(row);
        }
    }

    // Identity from $HOME.
    let home = std::env::var("HOME").unwrap_or_default();
    let base = home.rsplit('/').next().unwrap_or("").to_string();
    let initials: String = base
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(2)
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase();
    let initials = if initials.is_empty() {
        base.chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default()
    } else {
        initials
    };

    // Active hours: peak prompt local hour.
    let active_hours = hour_counts
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(h, c)| {
            json!({
                "startHour": h,
                "endHour": (h + 1) % 24,
                "turnCount": c,
                "label": Value::Null,
            })
        })
        .unwrap_or(json!({
            "startHour": Value::Null,
            "endHour": Value::Null,
            "turnCount": 0,
            "label": Value::Null,
        }));

    let prompts_today = day_counts.get(&today).map(|d| d.0).unwrap_or(0.0);
    Ok(json!({
        "generatedAt": crate::server_local_servers::iso_now(),
        "timezone": {"utcOffsetMinutes": offset.trunc() as i64, "today": today},
        "identity": {
            "homeDirBasename": base,
            "initials": initials,
            "defaultHandle": format!("@{base}"),
        },
        "activity": {
            "currentStreakDays": current,
            "longestStreakDays": longest,
            "totalPromptsSent": total_prompts,
            "totalThreads": total_threads,
            "promptsToday": prompts_today,
            "heatmapMetric": "prompts",
            "heatmap": heatmap,
        },
        "activeHours": active_hours,
        "insights": {
            "topProvider": provider_totals.first().map(|p| json!(p.0)).unwrap_or(Value::Null),
            "topProviderPercent": provider_totals.first().map(|p| json!(if total_turns > 0.0 { p.1 / total_turns * 100.0 } else { 0.0 })).unwrap_or(Value::Null),
            "topReasoning": reasoning_totals.first().map(|r| json!(r.0)).unwrap_or(Value::Null),
            "topReasoningPercent": reasoning_totals.first().map(|r| json!(if total_turns > 0.0 { r.1 / total_turns * 100.0 } else { 0.0 })).unwrap_or(Value::Null),
            "skillsExplored": skills.len(),
            "totalSkillsUsed": skills.iter().map(|k| n(member(k, "runCount"))).sum::<f64>(),
        },
        "providerModels": provider_models,
        "skills": skills,
        "mostUsedSkill": skills.first().cloned().unwrap_or(Value::Null),
        "mostWorkedProject": best_project.unwrap_or(Value::Null),
        "quota": {
            "status": "unavailable",
            "provider": Value::Null,
            "window": Value::Null,
            "usedPercent": Value::Null,
            "resetsAt": Value::Null,
            "planName": Value::Null,
        },
    }))
}

// ── stats.getProfileTokenStats ──────────────────────────────────────────────

fn get_profile_token_stats(payload: &Value) -> Result<Value, WsErr> {
    let offset = n(member(payload, "utcOffsetMinutes"));
    let Some(state) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &state.conn;
    let turn_sel = turn_selections(conn);
    let selections = thread_model_selections(conn);

    // Live deltas (ported fn keeps user-origin rows only) + Claude model usage.
    let mut deltas: Vec<Value> = Vec::new();
    for (thread, fb, rows) in token_activity_rows(conn, &turn_sel, &selections) {
        let _ = thread;
        if let Ok(rows) =
            profilestatsarchive_rs::port::aggregate_thread_token_rows(Value::Array(rows), fb)
        {
            deltas.extend(rows);
        }
    }
    for d in claude_model_usage_rows(conn, &turn_sel)
        .into_iter()
        .filter(|d| d.origin == "user")
    {
        deltas.push(json!({
            "createdAt": d.created_at,
            "provider": d.provider.map(Value::String).unwrap_or(Value::Null),
            "instanceId": d.instance_id.map(Value::String).unwrap_or(Value::Null),
            "model": d.model.map(Value::String).unwrap_or(Value::Null),
            "tokens": d.tokens,
        }));
    }
    // Deleted tombstones keep lifetime totals after purges.
    deltas.extend(query_rows(
        conn,
        "SELECT created_at AS createdAt, provider, NULL AS instanceId, NULL AS model, tokens \
         FROM profile_stats_deleted_tokens",
    ));
    // Claude usage from before per-model accounting existed (no timestamp:
    // lifetime only).
    let legacy = query_count(
        conn,
        "SELECT COALESCE(SUM(tokens), 0) FROM profile_stats_claude_legacy_usage",
    );

    let mut lifetime = legacy;
    let mut by_day: HashMap<String, (f64, i64)> = HashMap::new();
    let mut by_model: Vec<(String, String, String, f64)> = Vec::new();
    let mut provider_tokens: Vec<(String, f64)> = Vec::new();
    for row in &deltas {
        let tokens = n(member(row, "tokens")).max(0.0);
        if tokens <= 0.0 {
            continue;
        }
        lifetime += tokens;
        let ms = date_parse_ms(member(row, "createdAt"));
        if ms.is_finite() {
            let (day, weekday) = local_day(ms, offset);
            by_day.entry(day).or_insert((0.0, weekday)).0 += tokens;
        }
        let provider = profilestats_rs::port::normalize_provider_kind(
            member(row, "provider").clone(),
            Some(member(row, "model").clone()),
        )
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".into());
        let instance = str_field(row, "instanceId").unwrap_or_else(|| "unknown".into());
        let model = str_field(row, "model").unwrap_or_else(|| "unknown".into());
        match by_model
            .iter_mut()
            .find(|m| m.0 == provider && m.1 == instance && m.2 == model)
        {
            Some(m) => m.3 += tokens,
            None => by_model.push((provider.clone(), instance, model, tokens)),
        }
        if provider != "unknown" {
            match provider_tokens.iter_mut().find(|p| p.0 == provider) {
                Some(p) => p.1 += tokens,
                None => provider_tokens.push((provider, tokens)),
            }
        }
    }

    // Providers with turns but no token telemetry are "unavailable".
    let events = turn_start_events(conn);
    let mut turn_providers: HashSet<String> = HashSet::new();
    for (thread, sel) in &selections {
        let evs = events.get(thread).cloned().unwrap_or_default();
        for row in profilestatsarchive_rs::port::aggregate_thread_turn_snapshot_rows(
            Value::Array(evs),
            sel.clone(),
        )
        .unwrap_or_default()
        {
            let p = profilestats_rs::port::normalize_provider_kind(
                member(&row, "provider").clone(),
                Some(member(&row, "model").clone()),
            )
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".into());
            if p != "unknown" {
                turn_providers.insert(p);
            }
        }
    }
    for row in query_rows(
        conn,
        "SELECT DISTINCT provider FROM profile_stats_deleted_turns",
    ) {
        if let Some(p) =
            profilestats_rs::port::normalize_provider_kind(member(&row, "provider").clone(), None)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
            && p != "unknown"
        {
            turn_providers.insert(p);
        }
    }
    let token_providers: HashSet<String> = provider_tokens.iter().map(|p| p.0.clone()).collect();
    let mut unavailable: Vec<String> = turn_providers
        .difference(&token_providers)
        .cloned()
        .collect();
    unavailable.sort();

    provider_tokens.sort_by(|a, b| b.1.total_cmp(&a.1));
    by_model.sort_by(|a, b| b.3.total_cmp(&a.3));
    let mut days: Vec<(String, i64, f64)> = by_day
        .iter()
        .map(|(d, (c, w))| (d.clone(), *w, *c))
        .collect();
    days.sort_by(|a, b| a.0.cmp(&b.0));
    let sorted_active: Vec<f64> = {
        let mut v: Vec<f64> = days.iter().map(|d| d.2).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        v
    };
    let heatmap: Vec<Value> = days
        .iter()
        .map(|(d, w, c)| {
            json!({
                "day": d,
                "count": c,
                "weekday": w,
                "intensity": profilestats_rs::port::heatmap_intensity(*c, sorted_active.clone())
                    .unwrap_or(0.0)
                    .max(0.0),
            })
        })
        .collect();
    let peak = days.iter().max_by(|a, b| a.2.total_cmp(&b.2));

    let available = lifetime > 0.0 || !deltas.is_empty();
    Ok(json!({
        "available": available,
        "lifetimeTotalTokens": if lifetime > 0.0 { json!(lifetime) } else { Value::Null },
        "peakDayTokens": peak.map(|p| json!(p.2)).unwrap_or(Value::Null),
        "peakDay": peak.map(|p| json!(p.0)).unwrap_or(Value::Null),
        "providers": provider_tokens.iter().map(|p| json!(p.0)).collect::<Vec<_>>(),
        "unavailableProviders": unavailable,
        "topProvider": provider_tokens.first().map(|p| json!(p.0)).unwrap_or(Value::Null),
        "topProviderPercent": provider_tokens.first().map(|p| {
            json!(if lifetime > 0.0 { p.1 / lifetime * 100.0 } else { 0.0 })
        }).unwrap_or(Value::Null),
        "models": by_model.iter().map(|(p, i, m, t)| json!({
            "provider": p,
            "instanceId": i,
            "model": m,
            "tokens": t,
            "percent": if lifetime > 0.0 { t / lifetime * 100.0 } else { 0.0 },
        })).collect::<Vec<_>>(),
        "heatmapMetric": "tokens",
        "heatmap": heatmap,
    }))
}

// ── stats.getRecap ──────────────────────────────────────────────────────────

fn get_recap(payload: &Value) -> Result<Value, WsErr> {
    let ranges = recapstats_rs::port::resolve_recap_ranges(payload.clone()).map_err(err)?;
    if let Some(e) = s(member(&ranges, "error")) {
        return Err(err(e));
    }
    let window = member(&ranges, "window").clone();
    let from = s(member(&window, "from")).unwrap_or("").to_string();
    let to = s(member(&window, "to")).unwrap_or("").to_string();
    let Some(state) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &state.conn;

    let prompt_rows = prompt_rows(conn, Some(&from), Some(&to));

    // Turn rows: requested in the window, model/provider via turn selection.
    let turn_sel = turn_selections(conn);
    let selections = thread_model_selections(conn);
    let mut turn_rows: Vec<Value> = Vec::new();
    for row in query_rows(
        conn,
        &format!(
            "SELECT thread_id AS threadId, turn_id AS turnId, state, requested_at AS requestedAt, \
             started_at AS startedAt, completed_at AS completedAt \
             FROM projection_turns WHERE requested_at >= '{from}' AND requested_at < '{to}'"
        ),
    ) {
        let key = format!(
            "{}\u{0}{}",
            s(member(&row, "threadId")).unwrap_or(""),
            s(member(&row, "turnId")).unwrap_or("")
        );
        let ms = turn_sel.get(&key).cloned().unwrap_or(Value::Null);
        let fb = selections
            .get(s(member(&row, "threadId")).unwrap_or(""))
            .cloned()
            .unwrap_or(Value::Null);
        turn_rows.push(json!({
            "requestedAt": member(&row, "requestedAt"),
            "startedAt": member(&row, "startedAt"),
            "completedAt": member(&row, "completedAt"),
            "state": member(&row, "state"),
            "provider": if member(&ms, "provider").is_null() { member(&fb, "provider").clone() } else { member(&ms, "provider").clone() },
            "model": if member(&ms, "model").is_null() { member(&fb, "model").clone() } else { member(&ms, "model").clone() },
        }));
    }

    // Token rows: all-origins deltas + Claude usage + deleted analytics rows,
    // each emitted once per covering bucket (0 = window, 1.. = slots).
    let range_list: Vec<Value> = {
        let mut v = vec![window.clone()];
        if let Value::Array(slots) = member(&ranges, "slots") {
            v.extend(slots.iter().cloned());
        }
        v
    };
    let range_ms: Vec<(f64, f64)> = range_list
        .iter()
        .map(|r| (n(member(r, "fromMs")), n(member(r, "toMs"))))
        .collect();
    let (thread_project, projects) = thread_projects(conn);
    let project_of = |thread: &str| -> (Value, Value) {
        let pid = thread_project.get(thread).cloned().map(Value::String);
        match &pid {
            Some(Value::String(id)) => {
                let title = projects
                    .get(id)
                    .map(|p| member(p, "title").clone())
                    .unwrap_or(Value::Null);
                (Value::String(id.clone()), title)
            }
            _ => (Value::Null, Value::Null),
        }
    };
    let mut token_rows: Vec<Value> = Vec::new();
    let emit_token = |token_rows: &mut Vec<Value>, d: &DeltaRow, pid: &Value, title: &Value| {
        let ms = date_parse_ms(&d.created_at);
        if !ms.is_finite() {
            return;
        }
        for (bucket, (rf, rt)) in range_ms.iter().enumerate() {
            if ms >= *rf && ms < *rt {
                token_rows.push(json!({
                    "bucket": bucket,
                    "tokens": d.tokens,
                    "cacheTokens": d.cache_tokens,
                    "origin": origin_str(&d.origin),
                    "provider": d.provider.clone().map(Value::String).unwrap_or(Value::Null),
                    "model": d.model.clone().map(Value::String).unwrap_or(Value::Null),
                    "projectId": pid,
                    "title": title,
                }));
            }
        }
    };
    for (thread, fb, rows) in token_activity_rows(conn, &turn_sel, &selections) {
        let (pid, title) = project_of(&thread);
        for d in token_deltas(&rows, &fb) {
            emit_token(&mut token_rows, &d, &pid, &title);
        }
    }
    for row in query_rows(
        conn,
        "SELECT thread_id AS threadId, turn_id AS turnId, created_at AS createdAt, payload_json \
         FROM projection_thread_activities WHERE kind = 'turn.completed' \
         ORDER BY thread_id, sequence, created_at, activity_id",
    ) {
        let Some(thread) = s(member(&row, "threadId")).map(str::to_string) else {
            continue;
        };
        let (pid, title) = project_of(&thread);
        let payload: Value = s(member(&row, "payload_json"))
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or(Value::Null);
        let Some(usage) = member(&payload, "modelUsage").as_object() else {
            continue;
        };
        let key = format!("{}\u{0}{}", thread, s(member(&row, "turnId")).unwrap_or(""));
        let ms_sel = turn_sel.get(&key).cloned().unwrap_or(Value::Null);
        let provider = s(member(&payload, "provider"))
            .map(str::to_string)
            .or_else(|| str_field(&ms_sel, "provider"))
            .unwrap_or_else(|| "claudeAgent".to_string());
        for (model, u) in usage {
            let num = |k: &str| finite_counter(u, k).unwrap_or(0.0);
            let input = num("inputTokens");
            let output = num("outputTokens");
            let cache_read = num("cacheReadInputTokens");
            let cache_write = num("cacheCreationInputTokens");
            let total = num("totalTokens");
            let tokens = if total > 0.0 {
                total
            } else {
                input + output + cache_read + cache_write
            };
            if tokens <= 0.0 {
                continue;
            }
            let d = DeltaRow {
                created_at: member(&row, "createdAt").clone(),
                provider: Some(provider.clone()),
                instance_id: str_field(&ms_sel, "instanceId"),
                model: Some(model.clone()),
                origin: str_field(&ms_sel, "dispatchOrigin").unwrap_or_else(|| "user".into()),
                tokens,
                cache_tokens: cache_read,
                split: Some((tokens, cache_read, cache_write, output)),
            };
            emit_token(&mut token_rows, &d, &pid, &title);
        }
    }
    // Deleted analytics tokens are pre-aggregated deltas with their own origin.
    for row in query_rows(
        conn,
        &format!(
            "SELECT created_at AS createdAt, tokens, dispatch_origin AS dispatchOrigin, provider, \
             model, project_id AS projectId FROM usage_analytics_deleted_tokens \
             WHERE created_at >= '{from}' AND created_at < '{to}'"
        ),
    ) {
        let d = DeltaRow {
            created_at: member(&row, "createdAt").clone(),
            provider: str_field(&row, "provider"),
            instance_id: None,
            model: str_field(&row, "model"),
            origin: str_field(&row, "dispatchOrigin").unwrap_or_else(|| "user".into()),
            tokens: n(member(&row, "tokens")).max(0.0),
            cache_tokens: 0.0,
            split: None,
        };
        let pid = member(&row, "projectId").clone();
        let title = s(&pid)
            .and_then(|id| projects.get(id))
            .map(|p| member(p, "title").clone())
            .unwrap_or(Value::Null);
        emit_token(&mut token_rows, &d, &pid, &title);
    }

    recapstats_rs::port::build_recap(json!({
        "ranges": ranges,
        "promptRows": prompt_rows,
        "tokenRows": token_rows,
        "turnRows": turn_rows,
        "nowMs": now_ms(),
    }))
    .map_err(err)
    .map(|mut out| {
        out["generatedAt"] = json!(crate::server_local_servers::iso_now());
        out
    })
}

// ── stats.getUsageAnalytics ─────────────────────────────────────────────────

fn get_usage_analytics(payload: &Value) -> Result<Value, WsErr> {
    let window = usageanalyticsstats_rs::port::resolve_usage_analytics_window(payload.clone())
        .map_err(err)?;
    if let Some(e) = s(member(&window, "error")) {
        return Err(err(e));
    }
    let from = s(member(&window, "from")).unwrap_or("").to_string();
    let to = s(member(&window, "to")).unwrap_or("").to_string();
    let scope = s(member(payload, "scope")).unwrap_or("synara").to_string();
    let Some(state) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &state.conn;
    let turn_sel = turn_selections(conn);
    let selections = thread_model_selections(conn);
    let (thread_project, projects) = thread_projects(conn);
    let project_of = |thread: &str| -> (Value, Value) {
        let pid = thread_project.get(thread).cloned().map(Value::String);
        match &pid {
            Some(Value::String(id)) => {
                let title = projects
                    .get(id)
                    .map(|p| member(p, "title").clone())
                    .unwrap_or(Value::Null);
                (Value::String(id.clone()), title)
            }
            _ => (Value::Null, Value::Null),
        }
    };

    // Token rows: quarter-hour buckets over all-origins deltas + Claude usage.
    let mut token_rows: Vec<Value> = Vec::new();
    for (thread, fb, rows) in token_activity_rows(conn, &turn_sel, &selections) {
        let (pid, title) = project_of(&thread);
        for d in token_deltas(&rows, &fb) {
            let ms = date_parse_ms(&d.created_at);
            if !ms.is_finite() {
                continue;
            }
            token_rows.push(json!({
                "hour": hour_bucket(ms),
                "provider": d.provider.clone().map(Value::String).unwrap_or(Value::Null),
                "model": d.model.clone().map(Value::String).unwrap_or(Value::Null),
                "projectId": pid,
                "title": title,
                "origin": origin_str(&d.origin),
                "tokens": d.tokens,
                "cacheTokens": d.cache_tokens,
                "splitTokens": d.split.map(|s| s.0).unwrap_or(0.0),
                "splitCacheTokens": d.split.map(|s| s.1).unwrap_or(0.0),
                "cacheWriteTokens": d.split.map(|s| s.2).unwrap_or(0.0),
                "outputTokens": d.split.map(|s| s.3).unwrap_or(0.0),
            }));
        }
    }
    let mut cost_rows: Vec<Value> = Vec::new();
    for row in query_rows(
        conn,
        "SELECT thread_id AS threadId, turn_id AS turnId, created_at AS createdAt, payload_json \
         FROM projection_thread_activities WHERE kind = 'turn.completed' \
         ORDER BY thread_id, sequence, created_at, activity_id",
    ) {
        let Some(thread) = s(member(&row, "threadId")).map(str::to_string) else {
            continue;
        };
        let Some(payload) =
            s(member(&row, "payload_json")).and_then(|t| serde_json::from_str::<Value>(t).ok())
        else {
            continue;
        };
        let created_at = member(&row, "createdAt").clone();
        let (pid, title) = project_of(&thread);
        let key = format!("{}\u{0}{}", thread, s(member(&row, "turnId")).unwrap_or(""));
        let ms = turn_sel.get(&key).cloned().unwrap_or(Value::Null);
        // Reported cost rows (any provider emitting totalCostUsd).
        if let Some(cost) = member(&payload, "totalCostUsd").as_f64()
            && cost > 0.0
        {
            let hour_ms = date_parse_ms(&created_at);
            if hour_ms.is_finite() {
                let provider = s(member(&payload, "provider"))
                    .map(str::to_string)
                    .or_else(|| str_field(&ms, "provider"))
                    .unwrap_or_else(|| "unknown".into());
                cost_rows.push(json!({
                    "hour": hour_bucket(hour_ms),
                    "provider": provider,
                    "model": str_field(&ms, "model").map(Value::String).unwrap_or(Value::Null),
                    "projectId": pid,
                    "title": title,
                    "costUsd": cost,
                    "turns": 1,
                }));
            }
        }
        // Claude per-model usage as token rows with full input/output splits.
        if let Some(usage) = member(&payload, "modelUsage").as_object() {
            for (model, u) in usage {
                let num = |k: &str| finite_counter(u, k).unwrap_or(0.0);
                let input = num("inputTokens");
                let output = num("outputTokens");
                let cache_read = num("cacheReadInputTokens");
                let cache_write = num("cacheCreationInputTokens");
                let total = num("totalTokens");
                let tokens = if total > 0.0 {
                    total
                } else {
                    input + output + cache_read + cache_write
                };
                if tokens <= 0.0 {
                    continue;
                }
                token_rows.push(json!({
                    "hour": hour_bucket(date_parse_ms(&created_at)),
                    "provider": s(member(&payload, "provider"))
                        .map(str::to_string)
                        .or_else(|| str_field(&ms, "provider"))
                        .unwrap_or_else(|| "claudeAgent".into()),
                    "model": model,
                    "projectId": pid,
                    "title": title,
                    "origin": origin_str(&str_field(&ms, "dispatchOrigin").unwrap_or_default()),
                    "tokens": tokens,
                    "cacheTokens": cache_read,
                    "splitTokens": tokens,
                    "splitCacheTokens": cache_read,
                    "cacheWriteTokens": cache_write,
                    "outputTokens": output,
                }));
            }
        }
    }
    // Deleted analytics tokens.
    for row in query_rows(
        conn,
        &format!(
            "SELECT created_at AS createdAt, tokens, dispatch_origin AS dispatchOrigin, provider, \
             model, project_id AS projectId FROM usage_analytics_deleted_tokens \
             WHERE created_at >= '{from}' AND created_at < '{to}'"
        ),
    ) {
        let ms = date_parse_ms(member(&row, "createdAt"));
        if !ms.is_finite() {
            continue;
        }
        let pid = member(&row, "projectId").clone();
        let title = s(&pid)
            .and_then(|id| projects.get(id))
            .map(|p| member(p, "title").clone())
            .unwrap_or(Value::Null);
        token_rows.push(json!({
            "hour": hour_bucket(ms),
            "provider": member(&row, "provider"),
            "model": member(&row, "model"),
            "projectId": pid,
            "title": title,
            "origin": origin_str(&str_field(&row, "dispatchOrigin").unwrap_or_default()),
            "tokens": n(member(&row, "tokens")).max(0.0),
            "cacheTokens": 0,
            "splitTokens": 0,
            "splitCacheTokens": 0,
            "cacheWriteTokens": 0,
            "outputTokens": 0,
        }));
    }

    // Turn rows: requested in the window, grouped by quarter hour.
    let mut turn_groups: HashMap<String, Value> = HashMap::new();
    for row in query_rows(
        conn,
        &format!(
            "SELECT thread_id AS threadId, turn_id AS turnId, state, requested_at AS requestedAt, \
             completed_at AS completedAt FROM projection_turns \
             WHERE requested_at >= '{from}' AND requested_at < '{to}'"
        ),
    ) {
        let ms_req = date_parse_ms(member(&row, "requestedAt"));
        if !ms_req.is_finite() {
            continue;
        }
        let thread = s(member(&row, "threadId")).unwrap_or("");
        let key = format!("{}\u{0}{}", thread, s(member(&row, "turnId")).unwrap_or(""));
        let ms = turn_sel.get(&key).cloned().unwrap_or(Value::Null);
        let fb = selections.get(thread).cloned().unwrap_or(Value::Null);
        let provider = s(member(&ms, "provider"))
            .or_else(|| s(member(&fb, "provider")))
            .unwrap_or("unknown");
        let model = s(member(&ms, "model"))
            .or_else(|| s(member(&fb, "model")))
            .unwrap_or("unknown");
        let (pid, title) = project_of(thread);
        let gkey = format!(
            "{}\u{0}{provider}\u{0}{model}\u{0}{pid}",
            hour_bucket(ms_req)
        );
        let entry = turn_groups.entry(gkey).or_insert_with(|| {
            json!({
                "hour": hour_bucket(ms_req),
                "provider": provider,
                "model": model,
                "projectId": pid,
                "title": title,
                "turns": 0,
                "completedTurns": 0,
                "failedTurns": 0,
            })
        });
        entry["turns"] = json!(n(member(entry, "turns")) + 1.0);
        if !member(&row, "completedAt").is_null() {
            entry["completedTurns"] = json!(n(member(entry, "completedTurns")) + 1.0);
        }
        if s(member(&row, "state")) == Some("failed") {
            entry["failedTurns"] = json!(n(member(entry, "failedTurns")) + 1.0);
        }
    }
    let turn_rows: Vec<Value> = turn_groups.into_values().collect();

    usageanalyticsstats_rs::port::build_usage_analytics(json!({
        "window": {"from": from, "to": to},
        "tokenRows": token_rows,
        "turnRows": turn_rows,
        "costRows": cost_rows,
        "sessionFiles": Value::Null,
        "prices": {"status": "unavailable", "fetchedAt": Value::Null},
        "scope": scope,
        "nowMs": now_ms(),
    }))
    .map_err(err)
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "stats.getProfileStats" => get_profile_stats(payload),
        "stats.getProfileTokenStats" => get_profile_token_stats(payload),
        "stats.getRecap" => get_recap(payload),
        "stats.getUsageAnalytics" => get_usage_analytics(payload),
        _ => Err(err("unknown stats tag")),
    }
}
