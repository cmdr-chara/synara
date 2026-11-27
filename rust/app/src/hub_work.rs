//! Hub work write path: submit/claim/reconcile for the durable hub work queue.
//! Ported from apps/server/src/projectAgent/hubWorkService.ts semantics:
//! task keys freeze at first decision so retries replay the same workers,
//! claims are atomic FIFO per project, reconcile keeps queue reasons honest.

use serde_json::{Value, json};

fn err(msg: impl Into<String>) -> crate::todo::WsErr {
    crate::todo::WsErr(msg.into(), Some("HUB_WORK"))
}

fn state_conn() -> Result<rusqlite::Connection, crate::todo::WsErr> {
    crate::orchestration_engine::open_state_rw()
}

fn stable_digest(value: &Value, len: usize) -> String {
    // Upstream stableGatewayDigest: sha256 over the canonical JSON, truncated.
    let canonical = canonical_json(value);
    let hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        canonical.hash(&mut h);
        format!("{:016x}{:016x}", h.finish(), h.finish().rotate_left(32))
    };
    hash.chars().take(len).collect()
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .iter()
                .map(|k| format!("{}:{}", serde_json::to_string(k).unwrap_or_default(), canonical_json(&map[*k])))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(arr) => {
            let inner: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", inner.join(","))
        }
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn normalize_task_key(key: &str) -> Option<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return None;
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | ':' | '-'))
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn positional_task_key(position: usize) -> String {
    format!("task-{position}")
}

fn work_item_id(project_id: &str, scope_key: &str, task_key: &str) -> String {
    let digest = stable_digest(&json!([project_id, scope_key, task_key]), 32);
    format!("hubwork-{digest}")
}

fn task_fingerprint(source_messages: &Value, spec: &Value, title: Option<&str>) -> String {
    stable_digest(
        &json!({
            "sourceMessages": source_messages,
            "spec": spec,
            "title": title,
        }),
        64,
    )
}

fn request_fingerprint(source_messages: &Value, task_keys: &[String]) -> String {
    stable_digest(
        &json!({
            "sourceMessages": source_messages,
            "taskKeys": task_keys,
        }),
        64,
    )
}

/// The frozen task key a record answers to: custom key when present, else the
/// positional key derived from its index.
fn logical_task_key(record: &Value) -> String {
    if let Some(key) = record.get("taskKey").and_then(Value::as_str) {
        return key.to_string();
    }
    positional_task_key(record.get("taskIndex").and_then(Value::as_i64).unwrap_or(0) as usize)
}

pub(crate) struct SubmitOutcome {
    pub items: Vec<Value>,
    pub replayed: bool,
}

/// Submit a batch of hub tasks: creates queued work items exactly once per
/// (project, scope, taskKey). The caller must be the hub coordinator thread.
pub(crate) fn submit(
    caller_thread: &str,
    caller_turn: Option<&str>,
    request_id: &str,
    source_messages: &[Value],
    tasks: &[Value],
) -> Result<SubmitOutcome, crate::todo::WsErr> {
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    // The caller must be the coordinator thread of an enabled hub config.
    let (project_id, config_json): (String, String) = conn
        .query_row(
            "SELECT project_id, limits_json FROM project_agent_configs \
             WHERE coordinator_thread_id = ?1 AND enabled = 1",
            [caller_thread],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| err("Only the hub coordinator can submit work."))?;
    if tasks.is_empty() {
        return Err(err("The hub work plan has no tasks."));
    }
    if source_messages.is_empty()
        || source_messages
            .iter()
            .any(|m| m.get("threadId").and_then(Value::as_str) != Some(caller_thread))
    {
        return Err(err(
            "Hub work requires original messages from this coordinator conversation.",
        ));
    }
    let mut ordered_sources = source_messages.to_vec();
    ordered_sources.sort_by_key(|m| {
        m.get("messageId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    });
    let scope_key = format!("source:{}", stable_digest(&json!(ordered_sources), 64));
    let source_by_id: std::collections::HashMap<String, Value> = source_messages
        .iter()
        .filter_map(|m| {
            m.get("messageId")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), m.clone()))
        })
        .collect();

    // Frozen records already persisted for this scope.
    let mut stmt = conn
        .prepare(
            "SELECT work_item_id, task_index, record_json FROM project_agent_work_items \
             WHERE project_id = ?1 AND scope_key = ?2 ORDER BY task_index",
        )
        .map_err(|e| err(e.to_string()))?;
    let frozen_rows: Vec<(String, i64, String)> = stmt
        .query_map(rusqlite::params![project_id, scope_key], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| err(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();
    let mut frozen_by_key: std::collections::HashMap<String, (String, Value)> =
        std::collections::HashMap::new();
    let mut max_index: i64 = -1;
    for (id, index, raw) in &frozen_rows {
        max_index = max_index.max(*index);
        let record = serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({}));
        frozen_by_key.insert(logical_task_key(&record), (id.clone(), record));
    }

    let mut decisions: Vec<(String, String, Value, Value)> = Vec::new(); // key, fingerprint, spec, task
    let mut claimed_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (position, task) in tasks.iter().enumerate() {
        let spec = task.get("spec").cloned().unwrap_or_else(|| task.clone());
        let title = task
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| spec.get("title").and_then(Value::as_str).map(str::to_string));
        // Task sources: the referenced context messages when pinned, else the
        // whole frozen source request.
        let task_sources: Vec<Value> = match spec.get("contextMessageIds").and_then(Value::as_array) {
            Some(ids) => {
                let mut out = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for id in ids.iter().filter_map(|v| v.as_str()) {
                    if !seen.insert(id.to_string()) {
                        return Err(err(
                            "Task context references must belong to this frozen source request.",
                        ));
                    }
                    match source_by_id.get(id) {
                        Some(m) => out.push(m.clone()),
                        None => {
                            return Err(err(
                                "Task context references must belong to this frozen source request.",
                            ))
                        }
                    }
                }
                out
            }
            None => source_messages.to_vec(),
        };
        if task_sources.is_empty() {
            return Err(err(
                "Task context references must belong to this frozen source request.",
            ));
        }
        let task_key = match spec.get("taskKey").and_then(Value::as_str) {
            Some(raw) => normalize_task_key(raw).ok_or_else(|| {
                err("taskKey must be 1-128 characters of lowercase letters, digits, '.', '_', ':' or '-'.")
            })?,
            None => positional_task_key(position),
        };
        if !claimed_keys.insert(task_key.clone()) {
            return Err(err(format!(
                "Task key '{task_key}' appears twice in one hub work plan."
            )));
        }
        let mut frozen_spec = spec.clone();
        frozen_spec
            .as_object_mut()
            .map(|o| o.insert("taskKey".into(), json!(task_key)));
        // The routing reason annotates the decision but is not part of its identity.
        let mut decision_spec = frozen_spec.clone();
        decision_spec.as_object_mut().map(|o| o.remove("routingReason"));
        let fingerprint = task_fingerprint(&json!(task_sources), &decision_spec, title.as_deref());
        if let Some((_id, existing)) = frozen_by_key.get(&task_key) {
            let existing_fp = existing
                .get("taskFingerprint")
                .and_then(Value::as_str)
                .unwrap_or("");
            if existing_fp != fingerprint {
                return Err(err(format!(
                    "Task key '{task_key}' already names a different task in this hub request. Keep the frozen plan, or use a new taskKey for new work."
                )));
            }
        }
        decisions.push((task_key, fingerprint, frozen_spec, task.clone()));
    }

    let fresh: Vec<&(String, String, Value, Value)> = decisions
        .iter()
        .filter(|(key, _, _, _)| !frozen_by_key.contains_key(key))
        .collect();
    let max_new = serde_json::from_str::<Value>(&config_json)
        .ok()
        .and_then(|limits| limits.get("maxNewWorkersPerTurn").and_then(Value::as_i64))
        .unwrap_or(4);
    if fresh.len() as i64 > max_new.min(8) {
        return Err(err("The hub work plan exceeds the per-turn task limit."));
    }

    let task_keys: Vec<String> = decisions.iter().map(|(k, _, _, _)| k.clone()).collect();
    let fingerprint = request_fingerprint(&json!(ordered_sources), &task_keys);
    let now = crate::todo::iso_now_ms();
    let mut task_index = max_index;
    let mut inserted: Vec<Value> = Vec::new();
    let tx = conn.unchecked_transaction().map_err(|e| err(e.to_string()))?;
    for (task_key, task_fp, spec, task) in &fresh {
        task_index += 1;
        let id = work_item_id(&project_id, &scope_key, task_key);
        let title = {
            let explicit = task.get("title").and_then(Value::as_str);
            let spec_title = spec.get("title").and_then(Value::as_str);
            let prompt_first = spec
                .get("prompt")
                .and_then(Value::as_str)
                .and_then(|p| p.lines().next());
            let raw = explicit.or(spec_title).or(prompt_first).unwrap_or("Task");
            let trimmed: String = raw.trim().chars().take(240).collect();
            if trimmed.is_empty() { "Task".to_string() } else { trimmed }
        };
        let first_source = source_messages
            .first()
            .and_then(|m| m.get("messageId").and_then(Value::as_str))
            .unwrap_or("");
        let record = json!({
            "workItemId": id,
            "projectId": project_id,
            "delegationTurnId": caller_turn,
            "targetProjectId": spec.get("projectId").and_then(Value::as_str).unwrap_or(&project_id),
            "requestId": request_id,
            "scopeKey": scope_key,
            "fingerprint": fingerprint,
            "taskIndex": task_index,
            "taskKey": task_key,
            "taskFingerprint": task_fp,
            "sourceThreadId": caller_thread,
            "sourceMessageId": first_source,
            "sourceMessages": source_messages,
            "creationSpec": spec,
            "title": title,
            "workerThreadId": Value::Null,
            "state": "queued",
            "queueReason": "Waiting for an available worker slot",
            "resultSummary": Value::Null,
            "progress": Value::Null,
            "slotHeld": false,
            "revision": 0,
            "createdAt": now,
            "updatedAt": now,
        });
        tx.execute(
            "INSERT OR IGNORE INTO project_agent_work_items \
             (work_item_id, project_id, scope_key, task_index, state, worker_thread_id, \
              slot_held, revision, record_json) VALUES (?1,?2,?3,?4,'queued',NULL,0,0,?5)",
            rusqlite::params![id, project_id, scope_key, task_index, record.to_string()],
        )
        .map_err(|e| err(e.to_string()))?;
        inserted.push(record);
    }
    tx.commit().map_err(|e| err(e.to_string()))?;
    let replayed = inserted.is_empty() && !decisions.is_empty();
    // Replay: return the frozen rows for the claimed keys.
    let items = if replayed {
        decisions
            .iter()
            .filter_map(|(key, _, _, _)| frozen_by_key.get(key).map(|(_, r)| r.clone()))
            .collect()
    } else {
        inserted
    };
    Ok(SubmitOutcome { items, replayed })
}

/// Claim the next queued work item for admission: FIFO by sequence, atomic
/// transition into `starting` with the worker slot reserved.
pub(crate) fn claim_next(project_id: &str) -> Result<Option<Value>, crate::todo::WsErr> {
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let row: Option<(String, String, i64)> = conn
        .query_row(
            "SELECT work_item_id, record_json, revision FROM project_agent_work_items \
             WHERE project_id = ?1 AND state = 'queued' ORDER BY sequence LIMIT 1",
            [project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();
    let Some((id, raw, revision)) = row else { return Ok(None) };
    let mut record = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
    record
        .as_object_mut()
        .map(|o| {
            o.insert("state".into(), json!("starting"));
            o.insert("slotHeld".into(), json!(true));
            o.insert("revision".into(), json!(revision + 1));
            o.insert("updatedAt".into(), json!(crate::todo::iso_now_ms()));
        });
    let updated = conn
        .execute(
            "UPDATE project_agent_work_items SET state = 'starting', slot_held = 1, \
             revision = revision + 1, record_json = ?1 \
             WHERE work_item_id = ?2 AND revision = ?3 AND state = 'queued'",
            rusqlite::params![record.to_string(), id, revision],
        )
        .map_err(|e| err(e.to_string()))?;
    if updated == 0 {
        // Lost the race with another claim — retry reports nothing this round.
        return Ok(None);
    }
    Ok(Some(record))
}

/// Keep queue reasons honest for queued work: paused hubs and unlinked target
/// repositories explain why items wait.
pub(crate) fn reconcile_queue_reasons(project_id: &str) -> Result<(), crate::todo::WsErr> {
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let enabled: bool = conn
        .query_row(
            "SELECT enabled FROM project_agent_configs WHERE project_id = ?1",
            [project_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(false);
    let mut stmt = conn
        .prepare(
            "SELECT work_item_id, record_json, revision FROM project_agent_work_items \
             WHERE project_id = ?1 AND state = 'queued'",
        )
        .map_err(|e| err(e.to_string()))?;
    let rows: Vec<(String, String, i64)> = stmt
        .query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| err(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();
    for (id, raw, revision) in rows {
        let mut record = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
        let target = record
            .get("targetProjectId")
            .and_then(Value::as_str)
            .unwrap_or(project_id)
            .to_string();
        let linked = target == project_id
            || conn
                .query_row(
                    "SELECT 1 FROM project_agent_linked_projects \
                     WHERE project_id = ?1 AND linked_project_id = ?2",
                    rusqlite::params![project_id, target],
                    |_| Ok(()),
                )
                .is_ok();
        let reason = if !enabled {
            "Resume this hub before starting work"
        } else if !linked {
            "Link this repository before starting"
        } else {
            "Waiting for an available worker slot"
        };
        let current = record.get("queueReason").and_then(Value::as_str).unwrap_or("");
        if current != reason {
            record.as_object_mut().map(|o| {
                o.insert("queueReason".into(), json!(reason));
                o.insert("revision".into(), json!(revision + 1));
            });
            let _ = conn.execute(
                "UPDATE project_agent_work_items SET record_json = ?1, revision = revision + 1 \
                 WHERE work_item_id = ?2 AND revision = ?3",
                rusqlite::params![record.to_string(), id, revision],
            );
        }
    }
    Ok(())
}

/// Admit one queued item into a fresh worker thread: claims the item, creates
/// the worker thread in the target project with the frozen creation spec,
/// starts the first turn with the spec prompt, and records the worker mapping.
/// Admission is exactly-once per item: the worker thread id derives from the
/// work item id, so a crash before the attach write replays into the same
/// thread instead of forking a second worker.
pub(crate) fn admit_next(project_id: &str) -> Result<bool, crate::todo::WsErr> {
    let Some(record) = claim_next(project_id)? else {
        return Ok(false);
    };
    let work_item_id = record
        .get("workItemId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let spec = record.get("creationSpec").cloned().unwrap_or_else(|| json!({}));
    let target_project = record
        .get("targetProjectId")
        .and_then(Value::as_str)
        .unwrap_or(project_id)
        .to_string();
    let prompt = spec
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if prompt.trim().is_empty() {
        release_failed(&work_item_id);
        return Err(err("Hub work item has no prompt."));
    }
    let title = record
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Hub task")
        .to_string();
    let thread_id = format!("{}-worker", work_item_id);
    let now = crate::todo::iso_now_ms();
    let create = json!({
        "type": "thread.create",
        "commandId": format!("{work_item_id}-thread-create"),
        "threadId": thread_id,
        "projectId": target_project,
        "title": title,
        "createdAt": now,
    });
    if let Err(e) = crate::orchestration_engine::handle("orchestration.dispatchCommand", &create) {
        if !e.0.contains("UNIQUE") && !e.0.contains("already exists") {
            release_failed(&work_item_id);
            return Err(err(format!("hub worker thread create: {}", e.0)));
        }
    }
    let turn = json!({
        "type": "thread.turn.start",
        "commandId": format!("{work_item_id}-turn-start"),
        "threadId": thread_id,
        "createdAt": crate::todo::iso_now_ms(),
        "dispatchMode": "queue",
        "dispatchOrigin": "hubWork",
        "message": {
            "messageId": format!("{work_item_id}-message"),
            "text": prompt,
            "attachments": [],
        },
    });
    if let Err(e) = crate::orchestration_engine::handle("orchestration.dispatchCommand", &turn) {
        release_failed(&work_item_id);
        return Err(err(format!("hub worker turn start: {}", e.0)));
    }
    // Attach the worker and mark the item running.
    if let Ok(conn) = state_conn() {
        if let Ok(raw) = conn.query_row(
            "SELECT record_json, revision FROM project_agent_work_items WHERE work_item_id = ?1",
            [&work_item_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        ) {
            let (raw, revision) = raw;
            let mut current = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
            current.as_object_mut().map(|o| {
                o.insert("workerThreadId".into(), json!(thread_id));
                o.insert("state".into(), json!("running"));
                o.insert("admittedAt".into(), json!(crate::todo::iso_now_ms()));
                o.insert("revision".into(), json!(revision + 1));
            });
            let _ = conn.execute(
                "UPDATE project_agent_work_items SET state = 'running', worker_thread_id = ?1, \
                 revision = revision + 1, record_json = ?2 WHERE work_item_id = ?3",
                rusqlite::params![thread_id, current.to_string(), work_item_id],
            );
        }
    }
    Ok(true)
}

fn release_failed(work_item_id: &str) {
    if let Ok(conn) = state_conn() {
        if let Ok((raw, revision)) = conn.query_row(
            "SELECT record_json, revision FROM project_agent_work_items WHERE work_item_id = ?1",
            [work_item_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        ) {
            let mut record = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
            record.as_object_mut().map(|o| {
                o.insert("state".into(), json!("queued"));
                o.insert("slotHeld".into(), json!(false));
                o.insert("revision".into(), json!(revision + 1));
            });
            let _ = conn.execute(
                "UPDATE project_agent_work_items SET state = 'queued', slot_held = 0, \
                 revision = revision + 1, record_json = ?1 WHERE work_item_id = ?2",
                rusqlite::params![record.to_string(), work_item_id],
            );
        }
    }
}

/// Background admission loop: every few seconds each enabled hub project
/// reconciles its queue reasons and admits waiting work while slots allow.
pub(crate) fn spawn_admission_loop() {
    std::thread::spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let Ok(conn) = state_conn() else { continue };
        let projects: Vec<String> = {
            let mut stmt = match conn.prepare(
                "SELECT project_id FROM project_agent_configs WHERE enabled = 1",
            ) {
                Ok(s) => s,
                Err(_) => continue,
            };
            stmt.query_map([], |r| r.get::<_, String>(0))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default()
        };
        drop(conn);
        for project_id in projects {
            let _ = reconcile_queue_reasons(&project_id);
            // Admit while items claim successfully; each admission spawns a
            // worker thread so the loop naturally paces itself.
            for _ in 0..4 {
                match admit_next(&project_id) {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(_) => break,
                }
            }
        }
    });
}
