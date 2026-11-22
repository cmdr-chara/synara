//! projectAgent.* read methods against the state db copy.
//! Ported from apps/server/src/projectAgent/Layers/ProjectAgentService.ts +
//! persistence/Layers/ProjectAgentRepository.ts (Synara impl; contracts only
//! set the wire shapes). Write/runtime methods stay relayed.
use crate::orchestration_snapshot::open_state_copy;
use crate::todo::{state_dir, WsErr};
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

fn with_state_copy<T>(
    message: &'static str,
    f: impl FnOnce(&Connection) -> Result<T, WsErr>,
) -> Result<T, WsErr> {
    let Some(copy) = open_state_copy() else {
        return Err(err(message));
    };
    f(&copy.conn)
}

fn sval(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, Option<String>>(idx) {
        Ok(Some(s)) => Value::String(s),
        _ => Value::Null,
    }
}

fn ival(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, Option<i64>>(idx) {
        Ok(Some(n)) => json!(n),
        _ => Value::Null,
    }
}

fn bval(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, Option<i64>>(idx) {
        Ok(Some(1)) => Value::Bool(true),
        _ => Value::Bool(false),
    }
}

fn jval(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, Option<String>>(idx) {
        Ok(Some(s)) => serde_json::from_str(&s).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

// ---- list cursor (packages/shared encodeProjectAgentListCursor) ----

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn b64url_encode(data: &str) -> String {
    let b = data.as_bytes();
    let mut out = String::new();
    for chunk in b.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((*chunk.get(1).unwrap_or(&0) as u32) << 8)
            | (*chunk.get(2).unwrap_or(&0) as u32);
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(B64[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(B64[n as usize & 63] as char);
        }
    }
    out
}

fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    let mut table = [255u8; 256];
    for (i, c) in B64.iter().enumerate() {
        table[*c as usize] = i as u8;
    }
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            return None;
        }
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            let v = table[*c as usize];
            if v == 255 {
                return None;
            }
            n |= (v as u32) << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

struct Cursor {
    created_at: String,
    id: String,
    sequence: Option<i64>,
}

fn decode_cursor(raw: Option<&str>) -> Option<Cursor> {
    let raw = raw?;
    let decoded = b64url_decode(raw)?;
    let text = String::from_utf8(decoded).ok()?;
    let split = text.find('\t')?;
    if split == 0 {
        return None;
    }
    let remainder = &text[split + 1..];
    let (id, sequence) = match remainder.find('\t') {
        None => (remainder, None),
        Some(i) => {
            let seq_raw = &remainder[i + 1..];
            let sequence = if seq_raw.is_empty() {
                None
            } else {
                seq_raw.parse::<i64>().ok()
            };
            (&remainder[..i], sequence)
        }
    };
    Some(Cursor {
        created_at: text[..split].to_string(),
        id: id.to_string(),
        sequence,
    })
}

fn encode_cursor(created_at: &str, id: &str, sequence: Option<i64>) -> String {
    let seq = sequence.map(|n| n.to_string()).unwrap_or_default();
    b64url_encode(&format!("{created_at}\t{id}\t{seq}"))
}

// ---- row builders ----

fn task_row(conn: &Connection, row: &rusqlite::Row) -> Result<Value, WsErr> {
    let id: String = row.get(0).map_err(|e| err(e.to_string()))?;
    let mut deps = conn
        .prepare("SELECT depends_on_task_id FROM project_agent_task_dependencies WHERE task_id = ?1")
        .map_err(|e| err(e.to_string()))?;
    let depends: Vec<Value> = deps
        .query_map([&id], |r| r.get::<_, String>(0))
        .map_err(|e| err(e.to_string()))?
        .filter_map(|d| d.ok().map(Value::String))
        .collect();
    Ok(json!({
        "id": id,
        "projectId": sval(row, 1),
        "goalId": sval(row, 2),
        "title": sval(row, 3),
        "description": sval(row, 4),
        "acceptanceCriteria": sval(row, 5),
        "status": sval(row, 6),
        "assignedThreadId": sval(row, 7),
        "repairCount": ival(row, 8),
        "archivedAt": sval(row, 9),
        "revision": ival(row, 10),
        "createdAt": sval(row, 11),
        "updatedAt": sval(row, 12),
        "dependsOnTaskIds": depends,
    }))
}

// ---- shared queries ----

fn query_tasks(
    conn: &Connection,
    project_id: &str,
    goal_id: Option<&str>,
    include_archived: bool,
    limit: i64,
    cursor: Option<&Cursor>,
) -> Result<Vec<Value>, WsErr> {
    let mut sql = String::from(
        "SELECT task_id, project_id, goal_id, title, description, acceptance_criteria, status, \
         assigned_thread_id, repair_count, archived_at, revision, created_at, updated_at \
         FROM project_agent_tasks WHERE project_id = ?1",
    );
    let mut params: Vec<rusqlite::types::Value> = vec![project_id.to_string().into()];
    if let Some(g) = goal_id {
        sql.push_str(" AND goal_id = ?");
        params.push(g.to_string().into());
    }
    if !include_archived {
        sql.push_str(" AND archived_at IS NULL");
    }
    if let Some(c) = cursor {
        sql.push_str(" AND (created_at < ? OR (created_at = ? AND task_id < ?))");
        params.push(c.created_at.clone().into());
        params.push(c.created_at.clone().into());
        params.push(c.id.clone().into());
    }
    sql.push_str(" ORDER BY created_at DESC, task_id DESC LIMIT ?");
    params.push(limit.into());
    let mut stmt = conn.prepare(&sql).map_err(|e| err(e.to_string()))?;
    let mut rows = stmt
        .query(params_from_iter(params))
        .map_err(|e| err(e.to_string()))?;
    let mut tasks = Vec::new();
    while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
        tasks.push(task_row(conn, row)?);
    }
    Ok(tasks)
}

fn query_activity(
    conn: &Connection,
    project_id: &str,
    limit: i64,
    cursor: Option<&Cursor>,
) -> Result<Vec<Value>, WsErr> {
    let mut sql = String::from(
        "SELECT activity_id, project_id, sequence, kind, actor_kind, actor_thread_id, \
         goal_id, task_id, source_json, summary, created_at \
         FROM project_agent_activity WHERE project_id = ?1",
    );
    let mut params: Vec<rusqlite::types::Value> = vec![project_id.to_string().into()];
    if let Some(c) = cursor {
        if let Some(seq) = c.sequence {
            sql.push_str(" AND sequence < ?");
            params.push(seq.into());
        } else {
            sql.push_str(" AND (created_at < ? OR (created_at = ? AND activity_id < ?))");
            params.push(c.created_at.clone().into());
            params.push(c.created_at.clone().into());
            params.push(c.id.clone().into());
        }
    }
    sql.push_str(" ORDER BY sequence DESC LIMIT ?");
    params.push(limit.into());
    let mut stmt = conn.prepare(&sql).map_err(|e| err(e.to_string()))?;
    let mut rows = stmt
        .query(params_from_iter(params))
        .map_err(|e| err(e.to_string()))?;
    let mut activity = Vec::new();
    while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
        let seq: i64 = row.get(2).map_err(|e| err(e.to_string()))?;
        activity.push(json!({
            "id": sval(row, 0),
            "projectId": sval(row, 1),
            "sequence": seq,
            "kind": sval(row, 3),
            "actorKind": sval(row, 4),
            "actorThreadId": sval(row, 5),
            "goalId": sval(row, 6),
            "taskId": sval(row, 7),
            "source": jval(row, 8),
            "summary": sval(row, 9),
            "createdAt": sval(row, 10),
        }));
    }
    Ok(activity)
}

// ---- methods ----

fn list_tasks(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let goal_id = payload["goalId"].as_str().map(str::to_string);
    let include_archived = payload["includeArchived"].as_bool().unwrap_or(false);
    let limit = payload["limit"].as_i64().unwrap_or(50);
    let cursor = decode_cursor(payload["cursor"].as_str());
    with_state_copy("Failed to list project tasks.", move |conn| {
        let tasks = query_tasks(
            conn,
            &project_id,
            goal_id.as_deref(),
            include_archived,
            limit,
            cursor.as_ref(),
        )?;
        let next_cursor = if tasks.len() as i64 == limit && !tasks.is_empty() {
            let last = &tasks[tasks.len() - 1];
            Value::String(encode_cursor(
                last["createdAt"].as_str().unwrap_or_default(),
                last["id"].as_str().unwrap_or_default(),
                None,
            ))
        } else {
            Value::Null
        };
        Ok(json!({ "tasks": tasks, "nextCursor": next_cursor }))
    })
}

fn list_activity(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let limit = payload["limit"].as_i64().unwrap_or(50);
    let cursor = decode_cursor(payload["cursor"].as_str());
    with_state_copy("Failed to list project activity.", move |conn| {
        let activity = query_activity(conn, &project_id, limit, cursor.as_ref())?;
        let next_cursor = if activity.len() as i64 == limit && !activity.is_empty() {
            let last = &activity[activity.len() - 1];
            Value::String(encode_cursor(
                last["createdAt"].as_str().unwrap_or_default(),
                last["id"].as_str().unwrap_or_default(),
                last["sequence"].as_i64(),
            ))
        } else {
            Value::Null
        };
        Ok(json!({ "activity": activity, "nextCursor": next_cursor }))
    })
}

fn list_documents(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let prefix = payload["prefix"].as_str().map(str::to_string);
    with_state_copy("Failed to list project documents.", move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT project_id, logical_path, revision, content_hash, disk_hash, \
                 conflict_pending, updated_at FROM project_agent_document_heads \
                 WHERE project_id = ?1 ORDER BY logical_path ASC",
            )
            .map_err(|e| err(e.to_string()))?;
        let mut rows = stmt.query([&project_id]).map_err(|e| err(e.to_string()))?;
        let mut documents = Vec::new();
        while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
            let logical_path: String = row.get(1).map_err(|e| err(e.to_string()))?;
            if let Some(p) = &prefix {
                if !logical_path.starts_with(p.as_str()) {
                    continue;
                }
            }
            documents.push(json!({
                "projectId": sval(row, 0),
                "logicalPath": logical_path,
                "revision": ival(row, 2),
                "contentHash": sval(row, 3),
                "diskHash": sval(row, 4),
                "conflictPending": bval(row, 5),
                "updatedAt": sval(row, 6),
            }));
        }
        Ok(json!({ "documents": documents }))
    })
}

fn normalize_doc_path(raw: &str) -> Result<String, WsErr> {
    let trimmed = raw.trim().replace('\\', "/");
    if trimmed.is_empty() {
        return Err(err("Document path is empty."));
    }
    if trimmed.starts_with('/')
        || trimmed.starts_with("//")
        || (trimmed.len() > 1 && trimmed.as_bytes()[1] == b':' && trimmed.as_bytes()[0].is_ascii_alphabetic())
    {
        return Err(err("Document path must be relative."));
    }
    let segments: Vec<&str> = trimmed
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if segments.is_empty() {
        return Err(err("Document path is empty."));
    }
    for seg in &segments {
        if *seg == ".." || seg.contains('\0') {
            return Err(err(format!("Document path rejects segment \"{seg}\".")));
        }
    }
    Ok(segments.join("/"))
}

fn doc_context_path(project_id: &str, logical_path: &str) -> PathBuf {
    PathBuf::from(state_dir())
        .join("project-context")
        .join(project_id)
        .join(logical_path)
}

fn read_document(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let logical_path = normalize_doc_path(payload["logicalPath"].as_str().unwrap_or(""))?;
    let revision = payload["revision"].as_i64();
    let pid = project_id.clone();
    with_state_copy("Failed to read project document.", move |conn| {
        let head = conn
            .query_row(
                "SELECT revision, content_hash, disk_hash, conflict_pending, updated_at \
                 FROM project_agent_document_heads WHERE project_id = ?1 AND logical_path = ?2",
                rusqlite::params![pid, logical_path],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        let Some((h_rev, h_hash, h_disk, _h_conflict, h_updated)) = head else {
            return Err(err("Document was not found."));
        };
        // document revision (latest unless a specific revision was asked for)
        let mut sql = String::from(
            "SELECT revision_id, revision, content, content_hash, author_kind, author_thread_id, \
             sources_json, created_at FROM project_agent_documents \
             WHERE project_id = ?1 AND logical_path = ?2",
        );
        let mut params: Vec<rusqlite::types::Value> =
            vec![pid.clone().into(), logical_path.clone().into()];
        if let Some(r) = revision {
            sql.push_str(" AND revision = ?");
            params.push(r.into());
        }
        sql.push_str(" ORDER BY revision DESC LIMIT 1");
        let doc = conn
            .query_row(&sql, params_from_iter(params), |row| {
                Ok(json!({
                    "id": sval(row, 0),
                    "projectId": pid.clone(),
                    "logicalPath": logical_path.clone(),
                    "revision": row.get::<_, i64>(1)?,
                    "content": row.get::<_, String>(2)?,
                    "contentHash": row.get::<_, Option<String>>(3)?,
                    "authorKind": row.get::<_, Option<String>>(4)?,
                    "authorThreadId": row.get::<_, Option<String>>(5)?,
                    "sources": match row.get::<_, Option<String>>(6)? {
                        Some(s) => serde_json::from_str(&s).unwrap_or(Value::Null),
                        None => Value::Null,
                    },
                    "createdAt": row.get::<_, Option<String>>(7)?,
                }))
            })
            .optional()
            .map_err(|e| err(e.to_string()))?;
        let Some(document) = doc else {
            return Err(err("Document revision was not found."));
        };
        // disk mirror: hash it when it exists and is a regular file
        let path = doc_context_path(&pid, &logical_path);
        let disk_hash = std::fs::symlink_metadata(&path)
            .ok()
            .filter(|m| !m.file_type().is_symlink() && m.is_file())
            .and_then(|_| {
                let mut f = std::fs::File::open(&path).ok()?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf).ok()?;
                Some(format!("{:x}", Sha256::digest(&buf)))
            });
        let synced_marker = h_disk.clone().unwrap_or_else(|| h_hash.clone().unwrap_or_default());
        let content_hash = h_hash.clone().unwrap_or_default();
        let conflict_pending = match &disk_hash {
            Some(dh) => *dh != synced_marker && *dh != content_hash,
            None => false,
        };
        let mut hist_stmt = conn
            .prepare(
                "SELECT revision, content_hash, author_kind, created_at FROM project_agent_documents \
                 WHERE project_id = ?1 AND logical_path = ?2 ORDER BY revision DESC LIMIT 50",
            )
            .map_err(|e| err(e.to_string()))?;
        let mut hist_rows = hist_stmt
            .query(rusqlite::params![pid, logical_path])
            .map_err(|e| err(e.to_string()))?;
        let mut history = Vec::new();
        while let Some(row) = hist_rows.next().map_err(|e| err(e.to_string()))? {
            history.push(json!({
                "revision": ival(row, 0),
                "contentHash": sval(row, 1),
                "authorKind": sval(row, 2),
                "createdAt": sval(row, 3),
            }));
        }
        Ok(json!({
            "head": {
                "projectId": pid,
                "logicalPath": logical_path,
                "revision": h_rev,
                "contentHash": h_hash,
                "diskHash": disk_hash.map(Value::String).unwrap_or(Value::Null),
                "conflictPending": conflict_pending,
                "updatedAt": h_updated,
            },
            "document": document,
            "history": history,
        }))
    })
}

fn list_thread_index(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    with_state_copy("Failed to list project threads.", move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT project_id, thread_id, excluded, archived, summary_status, \
                 last_updated_at, last_summarized_at FROM project_agent_thread_index \
                 WHERE project_id = ?1 ORDER BY last_updated_at DESC",
            )
            .map_err(|e| err(e.to_string()))?;
        let mut rows = stmt.query([&project_id]).map_err(|e| err(e.to_string()))?;
        let mut threads = Vec::new();
        while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
            threads.push(json!({
                "projectId": sval(row, 0),
                "threadId": sval(row, 1),
                "excluded": bval(row, 2),
                "archived": bval(row, 3),
                "summaryStatus": sval(row, 4),
                "lastUpdatedAt": sval(row, 5),
                "lastSummarizedAt": sval(row, 6),
            }));
        }
        Ok(json!({ "threads": threads }))
    })
}

fn list_evidence(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let task_id = payload["taskId"].as_str().unwrap_or_default().to_string();
    with_state_copy("Failed to list task evidence.", move |conn| {
        let task_project: Option<String> = conn
            .query_row(
                "SELECT project_id FROM project_agent_tasks WHERE task_id = ?1",
                [&task_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        let Some(tp) = task_project else {
            return Err(err("Task was not found."));
        };
        if tp != project_id {
            return Err(err("Task does not belong to this project."));
        }
        let mut stmt = conn
            .prepare(
                "SELECT evidence_id, project_id, task_id, attempt_id, kind, classification, \
                 author_kind, author_thread_id, source_thread_id, source_message_id, \
                 source_turn_id, summary, created_at FROM project_agent_evidence \
                 WHERE task_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| err(e.to_string()))?;
        let mut rows = stmt.query([&task_id]).map_err(|e| err(e.to_string()))?;
        let mut evidence = Vec::new();
        while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
            evidence.push(json!({
                "id": sval(row, 0),
                "projectId": sval(row, 1),
                "taskId": sval(row, 2),
                "attemptId": sval(row, 3),
                "kind": sval(row, 4),
                "classification": sval(row, 5),
                "authorKind": sval(row, 6),
                "authorThreadId": sval(row, 7),
                "sourceThreadId": sval(row, 8),
                "sourceMessageId": sval(row, 9),
                "sourceTurnId": sval(row, 10),
                "summary": sval(row, 11),
                "createdAt": sval(row, 12),
            }));
        }
        Ok(json!({ "evidence": evidence }))
    })
}

// ---- coordinator status + workspace roots ----

/// packages/shared normalizeWorkspaceRootForComparison (non-Windows path).
fn normalize_root(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let slashed = trimmed.replace('\\', "/");
    let (prefix, rest) = if slashed.starts_with("//") {
        ("//", &slashed[2..])
    } else if slashed.starts_with('/') {
        ("/", &slashed[1..])
    } else {
        ("", slashed.as_str())
    };
    let mut body = String::new();
    let mut prev_slash = false;
    for c in rest.chars() {
        if c == '/' {
            if !prev_slash {
                body.push(c);
            }
            prev_slash = true;
        } else {
            body.push(c);
            prev_slash = false;
        }
    }
    while body.ends_with('/') {
        body.pop();
    }
    let mut normalized = if !prefix.is_empty() {
        format!("{prefix}{body}")
    } else {
        body
    };
    if normalized.is_empty() {
        normalized = prefix.to_string();
    }
    // macOS /private aliasing (platform is always darwin here).
    for p in ["/private/var/", "/private/tmp/"] {
        if let Some(stripped) = normalized.strip_prefix(p) {
            return format!("/{}/{}", &p[8..], stripped);
        }
    }
    normalized
}

fn root_within(candidate: &str, ancestor: &str) -> bool {
    let c = normalize_root(candidate);
    let a = normalize_root(ancestor);
    if c.is_empty() || a.is_empty() {
        return false;
    }
    if c == a {
        return true;
    }
    let prefix = format!("{a}/");
    c.starts_with(&prefix)
}

fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/devin".to_string())
}

/// Live thread state the coordinator-status resolver needs.
struct ThreadState {
    archived: bool,
    pending_ok: bool,
    working: bool,
    session_starting: bool,
}

fn thread_state(conn: &Connection, thread_id: &str) -> Option<ThreadState> {
    // projection_threads
    let (archived, approvals, inputs) = conn
        .query_row(
            "SELECT archived_at, pending_approval_count, pending_user_input_count \
             FROM projection_threads WHERE thread_id = ?1",
            [thread_id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?.is_some(),
                    r.get::<_, i64>(1).unwrap_or(0),
                    r.get::<_, i64>(2).unwrap_or(0),
                ))
            },
        )
        .optional()
        .ok()??;
    // session: no orchestration_status column in this schema → status only
    let session_status: Option<String> = conn
        .query_row(
            "SELECT status FROM projection_thread_sessions WHERE thread_id = ?1",
            [thread_id],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let can_answer = match &session_status {
        None => true,
        Some(s) => !matches!(s.as_str(), "closed" | "error" | "stopped"),
    };
    let pending_ok = can_answer && (approvals > 0 || inputs > 0);
    // latest turn: prefer latest_turn_id, else latest started
    let latest_turn_id: Option<String> = conn
        .query_row(
            "SELECT latest_turn_id FROM projection_threads WHERE thread_id = ?1",
            [thread_id],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let turn: Option<(String, Option<String>, Option<String>)> = if let Some(lt) = latest_turn_id {
        conn.query_row(
            "SELECT state, started_at, completed_at FROM projection_turns WHERE turn_id = ?1",
            [lt],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .ok()
        .flatten()
    } else {
        conn.query_row(
            "SELECT state, started_at, completed_at FROM projection_turns \
             WHERE thread_id = ?1 ORDER BY started_at DESC LIMIT 1",
            [thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .ok()
        .flatten()
    };
    // isThreadActivelyWorking: session running AND no completed latest turn
    let working = if session_status.as_deref() == Some("running") {
        match &turn {
            None => true,
            Some((state, started, completed)) => {
                // hasLiveLatestTurn = started && !settled
                let settled = match (started, completed) {
                    (None, _) | (_, None) => false,
                    (Some(_), Some(_)) => {
                        if state == "interrupted" || state == "error" {
                            true
                        } else {
                            // session exists and is running → not settled
                            false
                        }
                    }
                };
                started.is_some() && !settled
            }
        }
    } else {
        false
    };
    let session_starting = matches!(session_status.as_deref(), Some("connecting") | Some("starting"));
    Some(ThreadState {
        archived,
        pending_ok,
        working,
        session_starting,
    })
}

/// resolveGroupCoordinatorStatus — configured is always true here.
fn coordinator_status(goal_status: Option<&str>, thread: Option<ThreadState>) -> &'static str {
    match goal_status {
        Some("paused") => return "paused",
        Some("stopped") | Some("cancelled") => return "stopped",
        Some("active") => return "running",
        _ => {}
    }
    match thread {
        None => "idle",
        Some(t) => {
            if t.archived {
                "idle"
            } else if t.pending_ok || t.working || t.session_starting {
                "running"
            } else {
                "idle"
            }
        }
    }
}

fn linked_ids(conn: &Connection, project_id: &str) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT linked_project_id FROM project_agent_linked_projects WHERE project_id = ?1")
    {
        if let Ok(rows) = stmt.query_map([project_id], |r| r.get::<_, String>(0)) {
            out = rows.filter_map(|r| r.ok().map(Value::String)).collect();
        }
    }
    out
}

fn seed_instructions_hash() -> String {
    format!("{:x}", Sha256::digest(b"# Instructions\n\n"))
}

fn is_group_coordinator_host(kind: Option<&str>, workspace_root: Option<&str>) -> bool {
    let (Some(k), Some(w)) = (kind, workspace_root) else {
        return false;
    };
    let home = home_dir();
    let chat_root = format!("{home}/Documents/Synara");
    match k {
        "studio" => root_within(w, &format!("{chat_root}/Studio")),
        "group" => root_within(w, &format!("{chat_root}/Groups")),
        _ => false,
    }
}

fn list_summaries(payload: &Value) -> Result<Value, WsErr> {
    let _ = payload;
    with_state_copy("Failed to list project agents.", move |conn| {
        // summary rows: config + active/paused goal + instructions hash
        let mut stmt = conn
            .prepare(
                "SELECT c.project_id, c.coordinator_name, c.coordinator_thread_id, \
                 c.coordinator_icon, c.coordinator_color, c.revision, c.paused_at, \
                 c.archived_at, g.status, c.goal, dh.content_hash \
                 FROM project_agent_configs c \
                 LEFT JOIN project_agent_goals g ON g.project_id = c.project_id \
                   AND g.status IN ('active','paused') \
                 LEFT JOIN project_agent_document_heads dh ON dh.project_id = c.project_id \
                   AND dh.logical_path = 'instructions.md'",
            )
            .map_err(|e| err(e.to_string()))?;
        let mut rows = stmt.query([]).map_err(|e| err(e.to_string()))?;
        let mut summaries = Vec::new();
        let seed_hash = seed_instructions_hash();
        while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
            let project_id: String = row.get(0).map_err(|e| err(e.to_string()))?;
            let coord_thread: Option<String> = row.get(2).map_err(|e| err(e.to_string()))?;
            let goal_status: Option<String> = row.get(8).map_err(|e| err(e.to_string()))?;
            let goal: Option<String> = row.get(9).map_err(|e| err(e.to_string()))?;
            let instr_hash: Option<String> = row.get(10).map_err(|e| err(e.to_string()))?;
            // project shell: must be a group/studio container under its root
            let shell = conn
                .query_row(
                    "SELECT kind, workspace_root FROM projection_projects WHERE project_id = ?1",
                    [&project_id],
                    |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
                )
                .optional()
                .map_err(|e| err(e.to_string()))?;
            let Some((kind, wroot)) = shell else { continue };
            if !is_group_coordinator_host(kind.as_deref(), wroot.as_deref()) {
                continue;
            }
            // members: thread index (visible) + task-assigned live + coordinator
            let mut member_ids: Vec<String> = Vec::new();
            let mut member_stmt = conn
                .prepare(
                    "SELECT thread_id FROM project_agent_thread_index \
                     WHERE project_id = ?1 AND excluded = 0 AND archived = 0 \
                     UNION \
                     SELECT assigned_thread_id FROM project_agent_tasks \
                     WHERE project_id = ?1 AND assigned_thread_id IS NOT NULL \
                       AND archived_at IS NULL \
                       AND status IN ('planned','ready','running','blocked','review')",
                )
                .map_err(|e| err(e.to_string()))?;
            let mrows = member_stmt
                .query_map([&project_id], |r| r.get::<_, String>(0))
                .map_err(|e| err(e.to_string()))?;
            for m in mrows.flatten() {
                member_ids.push(m);
            }
            if let Some(ct) = &coord_thread {
                member_ids.retain(|id| id != ct);
                member_ids.insert(0, ct.clone());
            }
            // needsYou workers
            let mut needs_you: Vec<String> = Vec::new();
            if let Ok(mut ws) = conn.prepare(
                "SELECT thread_id FROM project_agent_managed_workers \
                 WHERE project_id = ?1 AND needs_you = 1",
            ) {
                if let Ok(wrows) = ws.query_map([&project_id], |r| r.get::<_, String>(0)) {
                    needs_you = wrows.flatten().collect();
                }
            }
            let ts = coord_thread.as_deref().and_then(|t| thread_state(conn, t));
            summaries.push(json!({
                "projectId": project_id,
                "configured": true,
                "coordinatorName": sval(row, 1),
                "coordinatorThreadId": coord_thread.map(Value::String).unwrap_or(Value::Null),
                "coordinatorIcon": sval(row, 3),
                "coordinatorColor": sval(row, 4),
                "coordinatorStatus": coordinator_status(goal_status.as_deref(), ts),
                "revision": ival(row, 5),
                "pausedAt": sval(row, 6),
                "archivedAt": sval(row, 7),
                "memberThreadIds": member_ids,
                "needsYouThreadIds": needs_you,
                "linkedProjectIds": linked_ids(conn, &project_id),
                "hasGoal": goal_status.is_some()
                    || goal.as_ref().map(|g| !g.trim().is_empty()).unwrap_or(false),
                "instructionsConfigured": instr_hash
                    .as_ref()
                    .map(|h| *h != seed_hash)
                    .unwrap_or(false),
            }));
        }
        Ok(json!({ "summaries": summaries }))
    })
}

// ---- getOverview ----

fn config_wire(conn: &Connection, project_id: &str) -> Result<Option<Value>, WsErr> {
    let row = conn
        .query_row(
            "SELECT coordinator_thread_id, coordinator_name, coordinator_model_selection_json, \
             coordinator_provider_options_json, worker_routing_json, limits_json, capture_enabled, \
             enabled, automation_id, revision, created_at, updated_at, disabled_at, goal, icon, \
             coordinator_icon, coordinator_color, auto_memory_enabled, library_path, \
             library_remote_url, library_push_on_change, paused_at, archived_at, \
             paused_automation_ids \
             FROM project_agent_configs WHERE project_id = ?1",
            [project_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, i64>(9)?,
                    r.get::<_, String>(10)?,
                    r.get::<_, String>(11)?,
                    r.get::<_, Option<String>>(12)?,
                    r.get::<_, Option<String>>(13)?,
                    r.get::<_, Option<String>>(14)?,
                    r.get::<_, Option<String>>(15)?,
                    r.get::<_, Option<String>>(16)?,
                    r.get::<_, i64>(17)?,
                    r.get::<_, Option<String>>(18)?,
                    r.get::<_, Option<String>>(19)?,
                    r.get::<_, i64>(20)?,
                    r.get::<_, Option<String>>(21)?,
                    r.get::<_, Option<String>>(22)?,
                    r.get::<_, Option<String>>(23)?,
                ))
            },
        )
        .optional()
        .map_err(|e| err(e.to_string()))?;
    let Some((
        coord_thread,
        coord_name,
        model_sel,
        provider_opts,
        worker_routing,
        limits,
        capture_enabled,
        enabled,
        automation_id,
        revision,
        created_at,
        updated_at,
        disabled_at,
        goal,
        icon,
        coord_icon,
        coord_color,
        auto_memory,
        library_path,
        library_remote,
        library_push,
        paused_at,
        archived_at,
        paused_automation,
    )) = row
    else {
        return Ok(None);
    };
    let j = |v: Option<String>| -> Value {
        v.and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null)
    };
    let mut cfg = json!({
        "projectId": project_id,
        "coordinatorThreadId": coord_thread,
        "coordinatorName": coord_name,
        "coordinatorModelSelection": j(model_sel),
        "limits": j(limits),
        "captureEnabled": capture_enabled == 1,
        "enabled": enabled == 1,
        "automationId": automation_id,
        "revision": revision,
        "createdAt": created_at,
        "updatedAt": updated_at,
        "disabledAt": disabled_at,
        "autoMemoryEnabled": auto_memory == 1,
        "libraryPushOnChange": library_push == 1,
        "pausedAt": paused_at,
        "archivedAt": archived_at,
        "pausedAutomationIds": j(paused_automation),
        "linkedProjectIds": linked_ids(conn, project_id),
    });
    let o = cfg.as_object_mut().unwrap();
    if let Some(v) = provider_opts.filter(|s| !s.is_empty()) {
        o.insert("coordinatorProviderOptions".into(), j(Some(v)));
    }
    if let Some(v) = worker_routing.filter(|s| !s.is_empty()) {
        o.insert("workerRouting".into(), j(Some(v)));
    }
    if let Some(v) = goal.filter(|s| !s.is_empty()) {
        o.insert("goal".into(), Value::String(v));
    }
    if let Some(v) = icon.filter(|s| !s.trim().is_empty()) {
        o.insert("icon".into(), Value::String(v.trim().to_string()));
    }
    if let Some(v) = coord_icon.filter(|s| !s.trim().is_empty()) {
        o.insert("coordinatorIcon".into(), Value::String(v.trim().to_string()));
    }
    if let Some(v) = coord_color.filter(|s| !s.trim().is_empty()) {
        o.insert("coordinatorColor".into(), Value::String(v.trim().to_string()));
    }
    if let Some(v) = library_path.filter(|s| !s.is_empty()) {
        o.insert("libraryPath".into(), Value::String(v));
    }
    if let Some(v) = library_remote.filter(|s| !s.is_empty()) {
        o.insert("libraryRemoteUrl".into(), Value::String(v));
    }
    Ok(Some(cfg))
}

fn goal_wire(row: &rusqlite::Row) -> Result<Value, rusqlite::Error> {
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "projectId": row.get::<_, String>(1)?,
        "objective": row.get::<_, String>(2)?,
        "authorizationSource": row.get::<_, String>(3)?,
        "scopeVersion": row.get::<_, i64>(4)?,
        "acceptanceCriteria": row.get::<_, Option<String>>(5)?,
        "limits": match row.get::<_, Option<String>>(6)? {
            Some(s) => serde_json::from_str(&s).unwrap_or(Value::Null),
            None => Value::Null,
        },
        "status": row.get::<_, String>(7)?,
        "continuationCount": row.get::<_, i64>(8)?,
        "workerCreationCount": row.get::<_, i64>(9)?,
        "authorizedAt": row.get::<_, String>(10)?,
        "revision": row.get::<_, i64>(11)?,
        "createdAt": row.get::<_, String>(12)?,
        "updatedAt": row.get::<_, String>(13)?,
    }))
}

const GOAL_COLS: &str = "goal_id, project_id, objective, authorization_source, scope_version, \
     acceptance_criteria, limits_json, status, continuation_count, worker_creation_count, \
     authorized_at, revision, created_at, updated_at";

fn strip_ansi(s: &str) -> String {
    // \x1b\[[0-9;]*[A-Za-z]
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            let mut consumed = String::new();
            let mut done = false;
            while let Some(&n) = chars.peek() {
                if n.is_ascii_digit() || n == ';' {
                    consumed.push(n);
                    chars.next();
                } else {
                    if n.is_ascii_alphabetic() {
                        chars.next();
                        done = true;
                    }
                    break;
                }
            }
            if !done {
                out.push_str("\u{1b}[");
                out.push_str(&consumed);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn sanitize_digest_summary(summary: Option<String>) -> Option<String> {
    let s = summary?;
    let mut next = s.trim().to_string();
    if next.is_empty() {
        return None;
    }
    // GOAL_GATE_DIGEST_SENTENCES (simple literals, case-insensitive)
    for pat in [
        "Start a goal to begin bounded coordination.",
        "Assigned work starts only after a goal is started.",
        "No worker completion is confirmed.",
    ] {
        let lower = next.to_lowercase();
        let pat_l = pat.to_lowercase();
        while let Some(i) = lower.find(&pat_l) {
            next = format!("{}{}", &next[..i], &next[i + pat.len()..]);
            break;
        }
        let _ = lower;
    }
    // collapse whitespace, " ." → "."
    let mut collapsed = String::new();
    let mut prev_space = false;
    for c in next.chars() {
        if c.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
            }
            prev_space = true;
        } else {
            collapsed.push(c);
            prev_space = false;
        }
    }
    next = collapsed.replace(" .", ".").trim().to_string();
    if next == "Coordinator is configured." || next == "Coordinator is configured" {
        return Some("Coordinator is ready.".to_string());
    }
    if next.is_empty() {
        Some("Coordinator is ready.".to_string())
    } else {
        Some(next)
    }
}

fn get_overview(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    with_state_copy("Failed to load project overview.", move |conn| {
        let cfg = config_wire(conn, &project_id)
            .map_err(|_| err("Failed to load project coordinator."))?;
        let links = linked_ids(conn, &project_id);
        let Some(config) = cfg else {
            return Ok(json!({
                "projectId": project_id,
                "configured": false,
                "config": null,
                "linkedProjectIds": links,
                "goal": null,
                "digest": null,
                "blockers": [],
                "recentOutcomes": [],
                "workers": [],
                "hubWorkItems": [],
                "coordinatorStatus": "unconfigured",
            }));
        };
        // active goal
        let goal = conn
            .query_row(
                &format!(
                    "SELECT {GOAL_COLS} FROM project_agent_goals WHERE project_id = ?1 \
                     AND status IN ('active','paused') ORDER BY updated_at DESC, goal_id DESC LIMIT 1"
                ),
                [&project_id],
                goal_wire,
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        // digest
        let digest = conn
            .query_row(
                "SELECT summary, focus_items_json, coverage_from_sequence, coverage_to_sequence, \
                 historical_coverage, summarized_thread_count, pending_thread_count, \
                 generation_state, generated_at, last_good_at, last_error \
                 FROM project_agent_digests WHERE project_id = ?1",
                [&project_id],
                |r| {
                    let mut le: Option<String> = r.get(10)?;
                    if let Some(e) = le.take() {
                        let cleaned = strip_ansi(&e).trim().to_string();
                        le = Some(if cleaned.chars().count() <= 2000 {
                            cleaned
                        } else {
                            let mut c: String = cleaned.chars().take(1999).collect();
                            c.push('\u{2026}');
                            c
                        });
                    }
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, i64>(5)?,
                        r.get::<_, i64>(6)?,
                        r.get::<_, String>(7)?,
                        r.get::<_, Option<String>>(8)?,
                        r.get::<_, Option<String>>(9)?,
                        le,
                    ))
                },
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        let digest_val = match digest {
            None => Value::Null,
            Some((
                summary,
                focus,
                cov_from,
                cov_to,
                hist_cov,
                summarized,
                pending,
                gen_state,
                generated,
                last_good,
                last_error,
            )) => json!({
                "projectId": project_id,
                "summary": sanitize_digest_summary(Some(summary)).unwrap_or_default(),
                "focusItems": serde_json::from_str::<Value>(&focus).unwrap_or(json!([])),
                "coverageFromSequence": cov_from,
                "coverageToSequence": cov_to,
                "historicalCoverage": hist_cov,
                "summarizedThreadCount": summarized,
                "pendingThreadCount": pending,
                "generationState": gen_state,
                "generatedAt": generated,
                "lastGoodAt": last_good,
                "lastError": last_error,
            }),
        };
        let tasks = query_tasks(conn, &project_id, None, false, 100, None)?;
        let blockers: Vec<Value> = tasks
            .iter()
            .filter(|t| t["status"].as_str() == Some("blocked"))
            .map(|t| {
                json!({
                    "taskId": t["id"],
                    "title": t["title"],
                    "reason": t["acceptanceCriteria"].as_str().unwrap_or("Blocked"),
                })
            })
            .collect();
        let activity = query_activity(conn, &project_id, 8, None)?;
        // managed workers (wire = row aliases + needsYou bool)
        let mut workers = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT project_id, thread_id, batch_id, request_id, title, task_id, settled_at, \
             settle_outcome, waiting_since, stuck_kind, stuck_since, task_prompt, \
             recovery_episode, recovery_step, nudge_at, recoveries_used, needs_you, \
             needs_you_at, active_turn_origin, active_turn_command_id, result_summary, \
             result_at, created_at, updated_at \
             FROM project_agent_managed_workers WHERE project_id = ?1 \
             ORDER BY created_at ASC, thread_id ASC",
        ) {
            if let Ok(mut rows) = stmt.query([&project_id]) {
                while let Ok(Some(row)) = rows.next() {
                    workers.push(json!({
                        "projectId": sval(row, 0),
                        "threadId": sval(row, 1),
                        "batchId": sval(row, 2),
                        "requestId": sval(row, 3),
                        "title": sval(row, 4),
                        "taskId": sval(row, 5),
                        "settledAt": sval(row, 6),
                        "settleOutcome": sval(row, 7),
                        "waitingSince": sval(row, 8),
                        "stuckKind": sval(row, 9),
                        "stuckSince": sval(row, 10),
                        "taskPrompt": sval(row, 11),
                        "recoveryEpisode": sval(row, 12),
                        "recoveryStep": ival(row, 13),
                        "nudgeAt": sval(row, 14),
                        "recoveriesUsed": ival(row, 15),
                        "needsYou": bval(row, 16),
                        "needsYouAt": sval(row, 17),
                        "activeTurnOrigin": sval(row, 18),
                        "activeTurnCommandId": sval(row, 19),
                        "resultSummary": sval(row, 20),
                        "resultAt": sval(row, 21),
                        "createdAt": sval(row, 22),
                        "updatedAt": sval(row, 23),
                    }));
                }
            }
        }
        let coord_thread = config["coordinatorThreadId"].as_str().map(str::to_string);
        let goal_status = goal.as_ref().and_then(|g| g["status"].as_str().map(str::to_string));
        let ts = coord_thread.as_deref().and_then(|t| thread_state(conn, t));
        Ok(json!({
            "projectId": project_id,
            "configured": true,
            "config": config,
            "linkedProjectIds": links,
            "goal": goal.unwrap_or(Value::Null),
            "digest": digest_val,
            "blockers": blockers,
            "recentOutcomes": activity,
            "workers": workers,
            "hubWorkItems": [],
            "coordinatorStatus": coordinator_status(goal_status.as_deref(), ts),
        }))
    })
}

// ---- write methods (state writes gated on SYNARA_NO_UPSTREAM via open_state_rw) ----

fn goal_row(row: &rusqlite::Row) -> Result<Value, rusqlite::Error> {
    goal_wire(row)
}

fn get_goal(conn: &Connection, goal_id: &str) -> Result<Option<Value>, WsErr> {
    conn.query_row(
        &format!("SELECT {GOAL_COLS} FROM project_agent_goals WHERE goal_id = ?1"),
        [goal_id],
        goal_row,
    )
    .optional()
    .map_err(|e| err(e.to_string()))
}

fn get_active_goal(conn: &Connection, project_id: &str) -> Result<Option<Value>, WsErr> {
    conn.query_row(
        &format!(
            "SELECT {GOAL_COLS} FROM project_agent_goals WHERE project_id = ?1 \
             AND status IN ('active','paused') ORDER BY updated_at DESC, goal_id DESC LIMIT 1"
        ),
        [project_id],
        goal_row,
    )
    .optional()
    .map_err(|e| err(e.to_string()))
}

fn save_goal(
    conn: &Connection,
    g: &Value,
    expected_revision: Option<i64>,
) -> Result<(), WsErr> {
    let cols = "goal_id, project_id, objective, authorization_source, scope_version, \
                acceptance_criteria, limits_json, status, continuation_count, \
                worker_creation_count, authorized_at, revision, created_at, updated_at";
    let params = rusqlite::params![
        g["id"].as_str(),
        g["projectId"].as_str(),
        g["objective"].as_str(),
        g["authorizationSource"].as_str(),
        g["scopeVersion"].as_i64().unwrap_or(1),
        g["acceptanceCriteria"].as_str(),
        g["limits"].to_string(),
        g["status"].as_str(),
        g["continuationCount"].as_i64().unwrap_or(0),
        g["workerCreationCount"].as_i64().unwrap_or(0),
        g["authorizedAt"].as_str(),
        g["revision"].as_i64().unwrap_or(1),
        g["createdAt"].as_str(),
        g["updatedAt"].as_str(),
    ];
    if expected_revision.is_none() {
        conn.execute(
            &format!("INSERT INTO project_agent_goals ({cols}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)"),
            params,
        )
        .map_err(|e| err(e.to_string()))?;
        return Ok(());
    }
    let changed = conn
        .execute(
            &format!("UPDATE project_agent_goals SET ({cols}) = (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14) WHERE goal_id = ?1 AND revision = ?15"),
            rusqlite::params![
                g["id"].as_str(),
                g["projectId"].as_str(),
                g["objective"].as_str(),
                g["authorizationSource"].as_str(),
                g["scopeVersion"].as_i64().unwrap_or(1),
                g["acceptanceCriteria"].as_str(),
                g["limits"].to_string(),
                g["status"].as_str(),
                g["continuationCount"].as_i64().unwrap_or(0),
                g["workerCreationCount"].as_i64().unwrap_or(0),
                g["authorizedAt"].as_str(),
                g["revision"].as_i64().unwrap_or(1),
                g["createdAt"].as_str(),
                g["updatedAt"].as_str(),
                expected_revision.unwrap_or(0),
            ],
        )
        .map_err(|e| err(e.to_string()))?;
    if changed != 1 {
        return Err(err("Failed to update project goal."));
    }
    Ok(())
}

fn get_task(conn: &Connection, task_id: &str) -> Result<Option<Value>, WsErr> {
    conn.query_row(
        "SELECT task_id, project_id, goal_id, title, description, acceptance_criteria, \
         status, assigned_thread_id, repair_count, archived_at, revision, created_at, updated_at \
         FROM project_agent_tasks WHERE task_id = ?1",
        [task_id],
        |row| task_row(conn, row).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.0,
            )))
        }),
    )
    .optional()
    .map_err(|e| err(e.to_string()))
}

fn save_task(conn: &Connection, t: &Value, expected_revision: Option<i64>) -> Result<(), WsErr> {
    if expected_revision.is_none() {
        conn.execute(
            "INSERT INTO project_agent_tasks (task_id, project_id, goal_id, title, description, \
             acceptance_criteria, status, assigned_thread_id, repair_count, archived_at, revision, \
             created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            rusqlite::params![
                t["id"].as_str(),
                t["projectId"].as_str(),
                t["goalId"].as_str(),
                t["title"].as_str(),
                t["description"].as_str(),
                t["acceptanceCriteria"].as_str(),
                t["status"].as_str(),
                t["assignedThreadId"].as_str(),
                t["repairCount"].as_i64().unwrap_or(0),
                t["archivedAt"].as_str(),
                t["revision"].as_i64().unwrap_or(1),
                t["createdAt"].as_str(),
                t["updatedAt"].as_str(),
            ],
        )
        .map_err(|e| err(e.to_string()))?;
    } else {
        let changed = conn
            .execute(
                "UPDATE project_agent_tasks SET goal_id=?3, title=?4, description=?5, \
                 acceptance_criteria=?6, status=?7, assigned_thread_id=?8, repair_count=?9, \
                 archived_at=?10, revision=?11, updated_at=?13 \
                 WHERE task_id=?1 AND project_id=?2 AND revision=?14",
                rusqlite::params![
                    t["id"].as_str(),
                    t["projectId"].as_str(),
                    t["goalId"].as_str(),
                    t["title"].as_str(),
                    t["description"].as_str(),
                    t["acceptanceCriteria"].as_str(),
                    t["status"].as_str(),
                    t["assignedThreadId"].as_str(),
                    t["repairCount"].as_i64().unwrap_or(0),
                    t["archivedAt"].as_str(),
                    t["revision"].as_i64().unwrap_or(1),
                    t["createdAt"].as_str(),
                    t["updatedAt"].as_str(),
                    expected_revision.unwrap_or(0),
                ],
            )
            .map_err(|e| err(e.to_string()))?;
        if changed != 1 {
            return Err(err("Failed to update project task."));
        }
    }
    let task_id = t["id"].as_str().unwrap_or_default();
    let project_id = t["projectId"].as_str().unwrap_or_default();
    conn.execute(
        "DELETE FROM project_agent_task_dependencies WHERE task_id = ?1",
        [task_id],
    )
    .map_err(|e| err(e.to_string()))?;
    if let Some(deps) = t["dependsOnTaskIds"].as_array() {
        for d in deps {
            conn.execute(
                "INSERT INTO project_agent_task_dependencies (task_id, depends_on_task_id, project_id) \
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![task_id, d.as_str(), project_id],
            )
            .map_err(|e| err(e.to_string()))?;
        }
    }
    Ok(())
}

fn task_edges(conn: &Connection, project_id: &str) -> Result<Vec<(String, String)>, WsErr> {
    let mut stmt = conn
        .prepare("SELECT task_id, depends_on_task_id FROM project_agent_task_dependencies WHERE project_id = ?1")
        .map_err(|e| err(e.to_string()))?;
    let rows = stmt
        .query_map([project_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| err(e.to_string()))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn deps_cycle(task_id: &str, deps: &[String], edges: &[(String, String)]) -> bool {
    use std::collections::HashMap;
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (t, d) in edges {
        adj.entry(t.as_str()).or_default().push(d.as_str());
    }
    adj.insert(task_id, deps.iter().map(String::as_str).collect());
    fn visit<'a>(
        node: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        visiting: &mut std::collections::HashSet<&'a str>,
        visited: &mut std::collections::HashSet<&'a str>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        visiting.insert(node);
        if let Some(next) = adj.get(node) {
            for n in next {
                if visit(n, adj, visiting, visited) {
                    return true;
                }
            }
        }
        visiting.remove(node);
        visited.insert(node);
        false
    }
    visit(
        task_id,
        &adj,
        &mut std::collections::HashSet::new(),
        &mut std::collections::HashSet::new(),
    )
}

fn append_activity(
    conn: &Connection,
    project_id: &str,
    kind: &str,
    actor_kind: &str,
    actor_thread_id: Option<&str>,
    goal_id: Option<&str>,
    task_id: Option<&str>,
    summary: &str,
    created_at: &str,
) -> Result<(), WsErr> {
    conn.execute(
        "INSERT INTO project_agent_activity \
         (activity_id, project_id, sequence, kind, actor_kind, actor_thread_id, goal_id, task_id, \
          source_json, summary, created_at) \
         SELECT ?1, ?2, COALESCE((SELECT MAX(sequence) FROM project_agent_activity \
            WHERE project_id = ?2), 0) + 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10",
        rusqlite::params![
            crate::orchestration_engine::uuid_v4(),
            project_id,
            kind,
            actor_kind,
            actor_thread_id,
            goal_id,
            task_id,
            Value::Null.to_string(),
            summary,
            created_at,
        ],
    )
    .map_err(|e| err(e.to_string()))?;
    Ok(())
}

fn replay_receipt(conn: &Connection, request_id: &str, project_id: &str) -> Option<Value> {
    conn.query_row(
        "SELECT result_json FROM project_agent_receipts WHERE request_id = ?1 AND project_id = ?2",
        rusqlite::params![request_id, project_id],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
    .and_then(|s| serde_json::from_str(&s).ok())
}

fn store_receipt(
    conn: &Connection,
    request_id: &str,
    project_id: &str,
    operation: &str,
    result: &Value,
) {
    let _ = conn.execute(
        "INSERT OR REPLACE INTO project_agent_receipts (request_id, project_id, operation, result_json, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            request_id,
            project_id,
            operation,
            result.to_string(),
            crate::todo::iso_now_ms()
        ],
    );
}

fn require_hub_project(conn: &Connection, project_id: &str) -> Result<(), WsErr> {
    let shell = conn
        .query_row(
            "SELECT kind, workspace_root FROM projection_projects WHERE project_id = ?1 AND deleted_at IS NULL",
            [project_id],
            |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|e| err(e.to_string()))?;
    let Some((kind, wroot)) = shell else {
        return Err(err(format!("Project \"{project_id}\" was not found.")));
    };
    if !is_group_coordinator_host(kind.as_deref(), wroot.as_deref()) {
        return Err(err("The coordinator is only available on hubs."));
    }
    Ok(())
}

fn require_config_exists(conn: &Connection, project_id: &str) -> Result<(), WsErr> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(1) FROM project_agent_configs WHERE project_id = ?1",
            [project_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    if !exists {
        return Err(err("Project Coordinator is not configured."));
    }
    Ok(())
}

fn create_task(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let goal_id = payload["goalId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let deps: Vec<String> = payload["dependsOnTaskIds"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let title = payload["title"].as_str().unwrap_or_default().to_string();
    let description = payload["description"].as_str().map(str::to_string);
    let criteria = payload["acceptanceCriteria"].as_str().map(str::to_string);
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to create project task."))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    let goal = get_goal(&conn, &goal_id)?
        .ok_or_else(|| err("Goal was not found."))?;
    if goal["projectId"].as_str() != Some(project_id.as_str()) {
        return Err(err("Goal does not belong to this project."));
    }
    if goal["status"].as_str() != Some("active") {
        return Err(err("Tasks can only be created for an active goal."));
    }
    let task_id = crate::orchestration_engine::uuid_v4();
    let edges = task_edges(&conn, &project_id)
        .map_err(|_| err("Failed to load task dependencies."))?;
    if deps_cycle(&task_id, &deps, &edges) {
        return Err(err("Task dependencies cannot form a cycle."));
    }
    let mut all_done = true;
    for dep in &deps {
        match get_task(&conn, dep).map_err(|_| err("Failed to load task dependency."))? {
            Some(t) if t["projectId"].as_str() == Some(project_id.as_str()) => {
                if t["status"].as_str() != Some("done") {
                    all_done = false;
                }
            }
            _ => return Err(err("Task dependencies must belong to the same project.")),
        }
    }
    let now = crate::todo::iso_now_ms();
    let task = json!({
        "id": task_id,
        "projectId": project_id,
        "goalId": goal_id,
        "title": title,
        "description": description.map(Value::String).unwrap_or(Value::Null),
        "acceptanceCriteria": criteria.map(Value::String).unwrap_or(Value::Null),
        "status": if deps.is_empty() || all_done { "ready" } else { "planned" },
        "dependsOnTaskIds": deps,
        "assignedThreadId": null,
        "repairCount": 0,
        "archivedAt": null,
        "revision": 1,
        "createdAt": now,
        "updatedAt": now,
    });
    save_task(&conn, &task, None).map_err(|_| err("Failed to create project task."))?;
    append_activity(
        &conn,
        &project_id,
        "task-created",
        "user",
        None,
        Some(&goal_id),
        Some(task["id"].as_str().unwrap_or_default()),
        &format!("Created task: {}", task["title"].as_str().unwrap_or_default()),
        &now,
    )?;
    store_receipt(&conn, &request_id, &project_id, "createTask", &task);
    Ok(task)
}

fn update_task(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let task_id = payload["taskId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to update project task."))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    let current = get_task(&conn, &task_id)
        .map_err(|_| err("Failed to load project task."))?
        .ok_or_else(|| err("Task was not found."))?;
    if current["projectId"].as_str() != Some(project_id.as_str()) {
        return Err(err("Task does not belong to this project."));
    }
    let wants_accept =
        payload["accept"].as_bool() == Some(true) || payload["status"].as_str() == Some("done");
    if wants_accept {
        let st = current["status"].as_str().unwrap_or_default();
        if st != "review" && st != "ready" && st != "running" {
            return Err(err("Only reviewed work can be accepted against recorded evidence."));
        }
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM project_agent_evidence WHERE task_id = ?1",
                [&task_id],
                |r| r.get(0),
            )
            .map_err(|e| err(e.to_string()))?;
        if n == 0 {
            return Err(err("Acceptance requires recorded evidence."));
        }
    }
    let deps: Vec<String> = if payload.get("dependsOnTaskIds").is_some() {
        let d: Vec<String> = payload["dependsOnTaskIds"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        let edges = task_edges(&conn, &project_id)
            .map_err(|_| err("Failed to load task dependencies."))?;
        if deps_cycle(&task_id, &d, &edges) {
            return Err(err("Task dependencies cannot form a cycle."));
        }
        for dep in &d {
            match get_task(&conn, dep).map_err(|_| err("Failed to load a task dependency."))? {
                Some(t) if t["projectId"].as_str() == Some(project_id.as_str()) => {}
                _ => return Err(err("Task dependencies must belong to the same project.")),
            }
        }
        d
    } else {
        current["dependsOnTaskIds"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };
    let now = crate::todo::iso_now_ms();
    let mut updated = current.clone();
    let o = updated.as_object_mut().unwrap();
    if let Some(t) = payload["title"].as_str() {
        o.insert("title".into(), Value::String(t.to_string()));
    }
    if let Some(d) = payload.get("description") {
        o.insert("description".into(), d.clone());
    }
    if let Some(c) = payload.get("acceptanceCriteria") {
        o.insert("acceptanceCriteria".into(), c.clone());
    }
    o.insert(
        "status".into(),
        Value::String(
            if wants_accept {
                "done"
            } else {
                payload["status"].as_str().unwrap_or_else(|| {
                    current["status"].as_str().unwrap_or_default()
                })
            }
            .to_string(),
        ),
    );
    o.insert("dependsOnTaskIds".into(), json!(deps));
    if let Some(a) = payload.get("archived") {
        o.insert(
            "archivedAt".into(),
            if a.as_bool() == Some(true) {
                if current["archivedAt"].is_string() {
                    current["archivedAt"].clone()
                } else {
                    Value::String(now.clone())
                }
            } else {
                Value::Null
            },
        );
    }
    o.insert(
        "revision".into(),
        json!(current["revision"].as_i64().unwrap_or(0) + 1),
    );
    o.insert("updatedAt".into(), Value::String(now.clone()));
    let expected = payload["expectedRevision"].as_i64();
    save_task(&conn, &updated, expected)
        .map_err(|_| err("Failed to update project task."))?;
    if wants_accept {
        // unblockDependents: planned/blocked tasks whose deps are all done
        let tasks = query_tasks(&conn, &project_id, None, false, 100, None)
            .map_err(|_| err("Failed to load dependent tasks."))?;
        for task in &tasks {
            let st = task["status"].as_str().unwrap_or_default();
            if st != "planned" && st != "blocked" {
                continue;
            }
            let deps: Vec<String> = task["dependsOnTaskIds"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            if !deps.iter().any(|d| d == &task_id) {
                continue;
            }
            let ready = deps.iter().all(|id| {
                get_task(&conn, id)
                    .ok()
                    .flatten()
                    .map(|t| t["status"].as_str() == Some("done"))
                    .unwrap_or(false)
            });
            if !ready {
                continue;
            }
            let mut dep_task = task.clone();
            let d = dep_task.as_object_mut().unwrap();
            d.insert("status".into(), Value::String("ready".into()));
            d.insert(
                "revision".into(),
                json!(task["revision"].as_i64().unwrap_or(0) + 1),
            );
            d.insert("updatedAt".into(), Value::String(now.clone()));
            let _ = save_task(
                &conn,
                &dep_task,
                Some(task["revision"].as_i64().unwrap_or(0)),
            );
        }
    }
    let kind = if wants_accept { "task-accepted" } else { "task-updated" };
    let summary = if wants_accept {
        format!("Accepted task: {}", updated["title"].as_str().unwrap_or_default())
    } else {
        format!("Updated task: {}", updated["title"].as_str().unwrap_or_default())
    };
    append_activity(
        &conn,
        &project_id,
        kind,
        "user",
        None,
        updated["goalId"].as_str(),
        Some(&task_id),
        &summary,
        &now,
    )?;
    store_receipt(&conn, &request_id, &project_id, "updateTask", &updated);
    Ok(updated)
}

fn start_goal(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let objective = payload["objective"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to start project goal."))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    require_hub_project(&conn, &project_id)?;
    require_config_exists(&conn, &project_id)?;
    if get_active_goal(&conn, &project_id)
        .map_err(|_| err("Failed to load project goal."))?
        .is_some()
    {
        return Err(err("This project already has an active or paused goal."));
    }
    // limits: input.limits ?? config.limits ?? DEFAULT_PROJECT_AGENT_LIMITS
    let limits = if let Some(l) = payload.get("limits") {
        l.clone()
    } else {
        let cfg_limits = conn
            .query_row(
                "SELECT limits_json FROM project_agent_configs WHERE project_id = ?1",
                [&project_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()
            .ok()
            .flatten()
            .flatten()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .filter(|v| !v.is_null());
        cfg_limits.unwrap_or_else(|| {
            json!({
                "maxConcurrentWorkers": 3,
                "maxOutstandingTasks": 50,
                "maxTaskAttempts": 3,
                "workerStaleMs": 900000,
                "workerContextChars": 12000,
                "reviewInstructions": null,
            })
        })
    };
    let now = crate::todo::iso_now_ms();
    let goal = json!({
        "id": crate::orchestration_engine::uuid_v4(),
        "projectId": project_id,
        "objective": objective,
        "authorizationSource": "user",
        "scopeVersion": 1,
        "acceptanceCriteria": payload["acceptanceCriteria"].clone(),
        "limits": limits,
        "status": "active",
        "continuationCount": 0,
        "workerCreationCount": 0,
        "authorizedAt": now,
        "revision": 1,
        "createdAt": now,
        "updatedAt": now,
    });
    save_goal(&conn, &goal, None).map_err(|_| err("Failed to start project goal."))?;
    append_activity(
        &conn,
        &project_id,
        "goal-started",
        "user",
        None,
        goal["id"].as_str(),
        None,
        &format!("Started goal: {}", goal["objective"].as_str().unwrap_or_default()),
        &now,
    )?;
    store_receipt(&conn, &request_id, &project_id, "startGoal", &goal);
    Ok(goal)
}

fn update_goal(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let goal_id = payload["goalId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to update project goal."))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    require_hub_project(&conn, &project_id)?;
    let current = get_goal(&conn, &goal_id)
        .map_err(|_| err("Failed to load project goal."))?
        .ok_or_else(|| err("Goal was not found."))?;
    if current["projectId"].as_str() != Some(project_id.as_str()) {
        return Err(err("Goal does not belong to this project."));
    }
    let now = crate::todo::iso_now_ms();
    let mut updated = current.clone();
    let o = updated.as_object_mut().unwrap();
    if let Some(v) = payload["objective"].as_str() {
        o.insert("objective".into(), Value::String(v.to_string()));
    }
    if let Some(v) = payload.get("acceptanceCriteria") {
        o.insert("acceptanceCriteria".into(), v.clone());
    }
    if let Some(v) = payload["scopeVersion"].as_i64() {
        o.insert("scopeVersion".into(), json!(v));
    }
    o.insert(
        "revision".into(),
        json!(current["revision"].as_i64().unwrap_or(0) + 1),
    );
    o.insert("updatedAt".into(), Value::String(now));
    let expected = payload["expectedRevision"].as_i64();
    save_goal(&conn, &updated, expected)
        .map_err(|_| err("Failed to update project goal."))?;
    append_activity(
        &conn,
        &project_id,
        "goal-updated",
        "user",
        None,
        Some(&goal_id),
        None,
        "Updated authorized goal scope.",
        &updated["updatedAt"].as_str().unwrap_or_default().to_string(),
    )?;
    store_receipt(&conn, &request_id, &project_id, "updateGoal", &updated);
    Ok(updated)
}

fn update_goal_status(payload: &Value, status: &str, kind: &str, summary: &str) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let goal_id = payload["goalId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to update project goal."))?;
    require_hub_project(&conn, &project_id)?;
    let current = get_goal(&conn, &goal_id)
        .map_err(|_| err("Failed to load project goal."))?
        .ok_or_else(|| err("Goal was not found."))?;
    if current["projectId"].as_str() != Some(project_id.as_str()) {
        return Err(err("Goal does not belong to this project."));
    }
    if status == "active" && current["status"].as_str() == Some("stopped") {
        return Err(err("A stopped goal cannot be resumed."));
    }
    let now = crate::todo::iso_now_ms();
    let mut updated = current.clone();
    let o = updated.as_object_mut().unwrap();
    o.insert("status".into(), Value::String(status.to_string()));
    o.insert(
        "revision".into(),
        json!(current["revision"].as_i64().unwrap_or(0) + 1),
    );
    o.insert("updatedAt".into(), Value::String(now.clone()));
    let expected = payload["expectedRevision"].as_i64();
    save_goal(&conn, &updated, expected)
        .map_err(|_| err("Failed to update project goal."))?;
    if status == "stopped" && current["status"].as_str() != Some("stopped") {
        // interrupt coordinator + running worker turns (best effort)
        if let Some(ct) = conn
            .query_row(
                "SELECT coordinator_thread_id FROM project_agent_configs WHERE project_id = ?1",
                [&project_id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
        {
            let _ = crate::orchestration_engine::handle(
                "orchestration.dispatchCommand",
                &json!({"type":"thread.turn.interrupt","commandId":crate::orchestration_engine::uuid_v4(),"threadId":ct,"createdAt":now}),
            );
        }
        let tasks = query_tasks(&conn, &project_id, Some(&goal_id), false, 100, None)
            .unwrap_or_default();
        for task in &tasks {
            let st = task["status"].as_str().unwrap_or_default();
            if st == "done" || st == "cancelled" {
                continue;
            }
            if let Some(tid) = task["assignedThreadId"].as_str() {
                let _ = crate::orchestration_engine::handle(
                    "orchestration.dispatchCommand",
                    &json!({"type":"thread.turn.interrupt","commandId":crate::orchestration_engine::uuid_v4(),"threadId":tid,"createdAt":now}),
                );
            }
        }
    }
    append_activity(
        &conn,
        &project_id,
        kind,
        "user",
        None,
        Some(&goal_id),
        None,
        summary,
        &updated["updatedAt"].as_str().unwrap_or_default().to_string(),
    )?;
    Ok(updated)
}

fn exclude_thread(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let thread_id = payload["threadId"].as_str().unwrap_or_default().to_string();
    let excluded = payload["excluded"].as_bool().unwrap_or(false);
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to update thread coverage."))?;
    require_hub_project(&conn, &project_id)?;
    require_config_exists(&conn, &project_id)?;
    let current = conn
        .query_row(
            "SELECT archived, summary_status, last_summarized_at FROM project_agent_thread_index \
             WHERE project_id = ?1 AND thread_id = ?2",
            rusqlite::params![&project_id, &thread_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| err(e.to_string()))?;
    let now = crate::todo::iso_now_ms();
    let entry = json!({
        "projectId": project_id,
        "threadId": thread_id,
        "excluded": excluded,
        "archived": current.as_ref().map(|(a,_,_)| *a == 1).unwrap_or(false),
        "summaryStatus": if excluded {
            "skipped"
        } else {
            current.as_ref().map(|(_,s,_)| s.as_str()).unwrap_or("pending")
        },
        "lastUpdatedAt": now,
        "lastSummarizedAt": current.as_ref().and_then(|(_,_,l)| l.clone()).map(Value::String).unwrap_or(Value::Null),
    });
    conn.execute(
        "INSERT OR REPLACE INTO project_agent_thread_index \
         (project_id, thread_id, excluded, archived, summary_status, last_updated_at, last_summarized_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            project_id,
            thread_id,
            if excluded { 1 } else { 0 },
            if entry["archived"].as_bool() == Some(true) { 1 } else { 0 },
            entry["summaryStatus"].as_str(),
            entry["lastUpdatedAt"].as_str(),
            entry["lastSummarizedAt"].as_str(),
        ],
    )
    .map_err(|e| err(e.to_string()))?;
    Ok(entry)
}

// ---- config row ----

fn config_row(conn: &Connection, project_id: &str) -> Result<Option<Vec<Option<String>>>, WsErr> {
    conn.query_row(
        "SELECT coordinator_thread_id, coordinator_name, coordinator_model_selection_json, \
         coordinator_provider_options_json, worker_routing_json, limits_json, capture_enabled, \
         enabled, automation_id, revision, created_at, updated_at, disabled_at, goal, icon, \
         coordinator_icon, coordinator_color, auto_memory_enabled, library_path, \
         library_remote_url, library_push_on_change, paused_at, archived_at, \
         paused_automation_ids \
         FROM project_agent_configs WHERE project_id = ?1",
        [project_id],
        |r| {
            let mut v = Vec::with_capacity(25);
            for i in 0..24 {
                let raw = r.get::<_, rusqlite::types::Value>(i)?;
                v.push(match raw {
                    rusqlite::types::Value::Null => None,
                    rusqlite::types::Value::Integer(n) => Some(n.to_string()),
                    rusqlite::types::Value::Real(f) => Some(f.to_string()),
                    rusqlite::types::Value::Text(s) => Some(s),
                    rusqlite::types::Value::Blob(_) => None,
                });
            }
            Ok(v)
        },
    )
    .optional()
    .map_err(|e| err(e.to_string()))
}

fn cfg_get(c: &[Option<String>], i: usize) -> Option<String> {
    c.get(i).cloned().flatten()
}

fn save_config(
    conn: &Connection,
    project_id: &str,
    c: &[Option<String>],
    expected_revision: Option<i64>,
    error_msg: &'static str,
) -> Result<(), WsErr> {
    if expected_revision.is_none() {
        conn.execute(
            "INSERT INTO project_agent_configs (project_id, coordinator_thread_id, \
             coordinator_name, coordinator_model_selection_json, \
             coordinator_provider_options_json, worker_routing_json, limits_json, \
             capture_enabled, enabled, automation_id, revision, created_at, updated_at, \
             disabled_at, goal, icon, coordinator_icon, coordinator_color, \
             auto_memory_enabled, library_path, library_remote_url, library_push_on_change, \
             paused_at, archived_at, paused_automation_ids) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
            rusqlite::params![
                project_id,
                cfg_get(c, 0), cfg_get(c, 1), cfg_get(c, 2), cfg_get(c, 3), cfg_get(c, 4),
                cfg_get(c, 5), cfg_get(c, 6), cfg_get(c, 7), cfg_get(c, 8), cfg_get(c, 9),
                cfg_get(c, 10), cfg_get(c, 11), cfg_get(c, 12), cfg_get(c, 13), cfg_get(c, 14),
                cfg_get(c, 15), cfg_get(c, 16), cfg_get(c, 17), cfg_get(c, 18), cfg_get(c, 19),
                cfg_get(c, 20), cfg_get(c, 21), cfg_get(c, 22), cfg_get(c, 23),
            ],
        )
        .map_err(|e| err(e.to_string()))?;
        return Ok(());
    }
    let changed = conn
        .execute(
            "UPDATE project_agent_configs SET coordinator_thread_id=?2, coordinator_name=?3, \
             coordinator_model_selection_json=?4, coordinator_provider_options_json=?5, \
             worker_routing_json=?6, limits_json=?7, capture_enabled=?8, enabled=?9, \
             automation_id=?10, revision=?11, created_at=?12, updated_at=?13, disabled_at=?14, \
             goal=?15, icon=?16, coordinator_icon=?17, coordinator_color=?18, \
             auto_memory_enabled=?19, library_path=?20, library_remote_url=?21, \
             library_push_on_change=?22, paused_at=?23, archived_at=?24, \
             paused_automation_ids=?25 \
             WHERE project_id=?1 AND revision=?26",
            rusqlite::params![
                project_id,
                cfg_get(c, 0), cfg_get(c, 1), cfg_get(c, 2), cfg_get(c, 3), cfg_get(c, 4),
                cfg_get(c, 5), cfg_get(c, 6), cfg_get(c, 7), cfg_get(c, 8), cfg_get(c, 9),
                cfg_get(c, 10), cfg_get(c, 11), cfg_get(c, 12), cfg_get(c, 13), cfg_get(c, 14),
                cfg_get(c, 15), cfg_get(c, 16), cfg_get(c, 17), cfg_get(c, 18), cfg_get(c, 19),
                cfg_get(c, 20), cfg_get(c, 21), cfg_get(c, 22), cfg_get(c, 23),
                expected_revision.unwrap_or(0),
            ],
        )
        .map_err(|e| err(e.to_string()))?;
    if changed != 1 {
        return Err(err(error_msg));
    }
    Ok(())
}

fn overview_for(conn: &Connection, project_id: &str) -> Result<Value, WsErr> {
    // buildOverview inline against a read connection (same shape as get_overview).
    let cfg_exists = config_row(conn, project_id)?.is_some();
    if !cfg_exists {
        return Ok(json!({
            "projectId": project_id, "configured": false, "config": null,
            "linkedProjectIds": linked_ids(conn, project_id), "goal": null,
            "digest": null, "blockers": [], "recentOutcomes": [], "workers": [],
            "hubWorkItems": [], "coordinatorStatus": "unconfigured",
        }));
    }
    get_overview(&json!({"projectId": project_id}))
}

fn list_enabled_automation_ids(conn: &Connection, project_id: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT automation_id FROM automation_definitions \
         WHERE project_id = ?1 AND enabled = 1 AND archived_at IS NULL",
    ) {
        if let Ok(rows) = stmt.query_map([project_id], |r| r.get::<_, String>(0)) {
            out = rows.flatten().collect();
        }
    }
    out
}

fn set_automations_enabled(conn: &Connection, ids: &[String], enabled: bool) {
    for id in ids {
        let _ = conn.execute(
            "UPDATE automation_definitions SET enabled = ?2, updated_at = ?3 WHERE automation_id = ?1",
            rusqlite::params![id, if enabled { 1 } else { 0 }, crate::todo::iso_now_ms()],
        );
    }
}

fn group_thread_ids(conn: &Connection, project_id: &str) -> Vec<(String, bool)> {
    // (thread_id, archived) for threads living in the project
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT thread_id, archived_at FROM projection_threads WHERE project_id = ?1 AND deleted_at IS NULL",
    ) {
        if let Ok(rows) = stmt.query_map([project_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?.is_some()))
        }) {
            out = rows.flatten().collect();
        }
    }
    out
}

fn dispatch_thread(cmd_type: &str, thread_id: &str) {
    let _ = crate::orchestration_engine::handle(
        "orchestration.dispatchCommand",
        &json!({
            "type": cmd_type,
            "commandId": crate::orchestration_engine::uuid_v4(),
            "threadId": thread_id,
            "createdAt": crate::todo::iso_now_ms(),
        }),
    );
}

fn lifecycle_group(
    payload: &Value,
    action: &str,
    activity_summary: &'static str,
    fail_msg: &'static str,
) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw().map_err(|_| err(fail_msg))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    require_hub_project(&conn, &project_id)?;
    let cfg = config_row(&conn, &project_id)?
        .ok_or_else(|| err("Project Coordinator is not configured."))?;
    let now = crate::todo::iso_now_ms();
    match action {
        "pause" | "archive" => {
            let flag_at = cfg_get(&cfg, if action == "pause" { 21 } else { 22 });
            if flag_at.is_none() {
                let ids = list_enabled_automation_ids(&conn, &project_id);
                set_automations_enabled(&conn, &ids, false);
                let mut c = cfg.clone();
                let mut paused: Vec<Value> = cfg_get(&cfg, 23)
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();
                for id in &ids {
                    if !paused.iter().any(|v| v.as_str() == Some(id)) {
                        paused.push(Value::String(id.clone()));
                    }
                }
                let idx = if action == "pause" { 21 } else { 22 };
                c[idx] = Some(now.clone());
                c[23] = Some(Value::Array(paused).to_string());
                c[9] = Some((cfg_get(&cfg, 9).and_then(|r| r.parse::<i64>().ok()).unwrap_or(0) + 1).to_string());
                c[11] = Some(now.clone());
                save_config(&conn, &project_id, &c, cfg_get(&cfg, 9).and_then(|r| r.parse().ok()), fail_msg)?;
                let coord = cfg_get(&cfg, 0).unwrap_or_default();
                if action == "pause" {
                    // interrupt live turns on member threads
                    for (tid, _) in group_thread_ids(&conn, &project_id) {
                        if let Some(ts) = thread_state(&conn, &tid) {
                            if ts.working || ts.session_starting {
                                dispatch_thread("thread.turn.interrupt", &tid);
                            }
                        }
                    }
                    if !coord.is_empty() {
                        if let Some(ts) = thread_state(&conn, &coord) {
                            if ts.working {
                                dispatch_thread("thread.turn.interrupt", &coord);
                            }
                        }
                    }
                } else {
                    // archive: coordinator + group threads
                    if !coord.is_empty() {
                        dispatch_thread("thread.archive", &coord);
                    }
                    for (tid, arch) in group_thread_ids(&conn, &project_id) {
                        if !arch && tid != coord {
                            dispatch_thread("thread.archive", &tid);
                        }
                    }
                }
                append_activity(&conn, &project_id, "config-updated", "user", None, None, None,
                    activity_summary, &crate::todo::iso_now_ms())?;
            }
        }
        "resume" | "unarchive" => {
            let flag_idx = if action == "resume" { 21 } else { 22 };
            if cfg_get(&cfg, flag_idx).is_some() {
                let paused_at_set = cfg_get(&cfg, 21).is_some();
                let restore: Vec<String> = if action == "unarchive" && paused_at_set {
                    Vec::new()
                } else {
                    cfg_get(&cfg, 23)
                        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
                        .unwrap_or_default()
                };
                set_automations_enabled(&conn, &restore, true);
                let mut c = cfg.clone();
                c[flag_idx] = None;
                if !(action == "unarchive" && paused_at_set) {
                    c[23] = Some("[]".to_string());
                }
                c[9] = Some((cfg_get(&cfg, 9).and_then(|r| r.parse::<i64>().ok()).unwrap_or(0) + 1).to_string());
                c[11] = Some(now.clone());
                save_config(&conn, &project_id, &c, cfg_get(&cfg, 9).and_then(|r| r.parse().ok()), fail_msg)?;
                if action == "unarchive" {
                    let coord = cfg_get(&cfg, 0).unwrap_or_default();
                    if !coord.is_empty() {
                        dispatch_thread("thread.unarchive", &coord);
                    }
                    for (tid, arch) in group_thread_ids(&conn, &project_id) {
                        if arch && tid != coord {
                            dispatch_thread("thread.unarchive", &tid);
                        }
                    }
                }
                append_activity(&conn, &project_id, "config-updated", "user", None, None, None,
                    activity_summary, &crate::todo::iso_now_ms())?;
            }
        }
        _ => {}
    }
    let overview = overview_for(&conn, &project_id)?;
    store_receipt(&conn, &request_id, &project_id, action, &overview);
    Ok(overview)
}

fn link_project(payload: &Value, unlink: bool) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let linked_id = payload["linkedProjectId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err(if unlink { "Failed to unlink repository." } else { "Failed to link repository." }))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    require_hub_project(&conn, &project_id)?;
    if !unlink {
        if linked_id == project_id {
            return Err(err("A hub cannot link to itself."));
        }
        let shell = conn
            .query_row(
                "SELECT kind, title, workspace_root FROM projection_projects WHERE project_id = ?1",
                [&linked_id],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)),
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        let Some((kind, title, wroot)) = shell else {
            return Err(err(format!("Project \"{linked_id}\" was not found.")));
        };
        let home = home_dir();
        let chat_root = format!("{home}/Documents/Synara");
        let legacy_home = title == "Home"
            && (root_within(wroot.as_deref().unwrap_or(""), &chat_root)
                || root_within(wroot.as_deref().unwrap_or(""), &home));
        let _ = legacy_home; // equal-or-prefix: ordinary check is kind==project && !legacyHome(root==chatRoot||root==home)
        let normalized = normalize_root(wroot.as_deref().unwrap_or(""));
        let is_legacy = title == "Home"
            && (normalized == normalize_root(&chat_root) || normalized == normalize_root(&home));
        if kind.as_deref().unwrap_or("project") != "project" || is_legacy {
            return Err(err("Only ordinary repositories can be linked to a hub."));
        }
    }
    let ids = linked_ids(&conn, &project_id);
    let has = ids.iter().any(|v| v.as_str() == Some(linked_id.as_str()));
    if !unlink && !has {
        conn.execute(
            "INSERT INTO project_agent_linked_projects (project_id, linked_project_id, created_at) \
             VALUES (?1, ?2, ?3) ON CONFLICT (project_id, linked_project_id) DO NOTHING",
            rusqlite::params![&project_id, &linked_id, crate::todo::iso_now_ms()],
        )
        .map_err(|e| err(e.to_string()))?;
        let title: String = conn
            .query_row(
                "SELECT title FROM projection_projects WHERE project_id = ?1",
                [&linked_id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| linked_id.clone());
        append_activity(&conn, &project_id, "config-updated", "user", None, None, None,
            &format!("Linked repository {title}"), &crate::todo::iso_now_ms())?;
    }
    if unlink && has {
        conn.execute(
            "DELETE FROM project_agent_linked_projects WHERE project_id = ?1 AND linked_project_id = ?2",
            rusqlite::params![&project_id, &linked_id],
        )
        .map_err(|e| err(e.to_string()))?;
        let title: String = conn
            .query_row(
                "SELECT title FROM projection_projects WHERE project_id = ?1",
                [&linked_id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| linked_id.clone());
        append_activity(&conn, &project_id, "config-updated", "user", None, None, None,
            &format!("Unlinked repository {title}"), &crate::todo::iso_now_ms())?;
    }
    if unlink && config_row(&conn, &project_id)?.is_none() {
        return Err(err("Project Coordinator is not configured."));
    }
    let overview = overview_for(&conn, &project_id)?;
    let op = if unlink { "unlinkProject" } else { "linkProject" };
    store_receipt(&conn, &request_id, &project_id, op, &overview);
    Ok(overview)
}

fn write_document(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to write project document."))?;
    require_hub_project(&conn, &project_id)?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    let logical_path = normalize_doc_path(payload["logicalPath"].as_str().unwrap_or(""))?;
    let content = payload["content"].as_str().unwrap_or_default().to_string();
    let head = conn
        .query_row(
            "SELECT revision, disk_hash FROM project_agent_document_heads \
             WHERE project_id = ?1 AND logical_path = ?2",
            rusqlite::params![&project_id, &logical_path],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|e| err(e.to_string()))?;
    let (current_revision, head_disk) = head
        .map(|(r, d)| (r, d))
        .unwrap_or((0, None));
    if let Some(exp) = payload["expectedRevision"].as_i64() {
        if exp != current_revision {
            return Err(err("Document changed. Reload and retry with the latest revision."));
        }
    }
    let disk = std::fs::read_to_string(doc_context_path(&project_id, &logical_path)).ok();
    let disk_hash = disk
        .map(|d| format!("{:x}", Sha256::digest(d.as_bytes())))
        .unwrap_or_else(|| head_disk.clone().unwrap_or_default());
    let now = crate::todo::iso_now_ms();
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    let revision_no = current_revision + 1;
    let rev = json!({
        "id": crate::orchestration_engine::uuid_v4(),
        "projectId": project_id,
        "logicalPath": logical_path,
        "revision": revision_no,
        "content": content,
        "contentHash": content_hash,
        "authorKind": "user",
        "authorThreadId": null,
        "sources": payload["sources"].as_array().cloned().unwrap_or_default(),
        "createdAt": now,
    });
    conn.execute(
        "INSERT INTO project_agent_documents \
         (revision_id, project_id, logical_path, revision, content, content_hash, \
          author_kind, author_thread_id, sources_json, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            rev["id"].as_str(), project_id, logical_path, revision_no, content, content_hash,
            "user", Option::<String>::None, rev["sources"].to_string(), now,
        ],
    )
    .map_err(|e| err(e.to_string()))?;
    // head upsert keeps the old disk marker until the mirror write confirms
    conn.execute(
        "INSERT INTO project_agent_document_heads \
         (project_id, logical_path, revision, content_hash, disk_hash, conflict_pending, updated_at) \
         VALUES (?1,?2,?3,?4,?5,0,?6) \
         ON CONFLICT (project_id, logical_path) DO UPDATE SET \
           revision=?3, content_hash=?4, disk_hash=?5, conflict_pending=0, updated_at=?6",
        rusqlite::params![
            project_id, logical_path, revision_no, content_hash,
            head_disk.clone().unwrap_or_else(|| content_hash.clone()),
            now,
        ],
    )
    .map_err(|e| err(e.to_string()))?;
    // materialize mirror then mark synced
    let mirror = doc_context_path(&project_id, &logical_path);
    if let Some(parent) = mirror.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&mirror, &content)
        .map_err(|_| err("Failed to materialize project document."))?;
    conn.execute(
        "UPDATE project_agent_document_heads SET disk_hash = ?3, conflict_pending = 0 \
         WHERE project_id = ?1 AND logical_path = ?2",
        rusqlite::params![project_id, logical_path, content_hash],
    )
    .map_err(|e| err(e.to_string()))?;
    let _ = disk_hash;
    append_activity(&conn, &project_id, "document-written", "user", None, None, None,
        &format!("Wrote {logical_path}"), &now)?;
    store_receipt(&conn, &request_id, &project_id, "writeDocument", &rev);
    Ok(rev)
}

fn configure(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to save coordinator configuration."))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    require_hub_project(&conn, &project_id)?;
    let title: String = conn
        .query_row(
            "SELECT title FROM projection_projects WHERE project_id = ?1",
            [&project_id],
            |r| r.get(0),
        )
        .unwrap_or_default();
    let existing = config_row(&conn, &project_id)
        .map_err(|_| err("Failed to load project coordinator."))?;
    // provider enabled + installed
    let provider = payload["coordinatorModelSelection"]["provider"]
        .as_str()
        .unwrap_or_default();
    let display = match provider {
        "codex" => "Codex", "claudeAgent" => "Claude", "cursor" => "Cursor",
        "devin" => "Devin", "antigravity" => "Antigravity", "grok" => "Grok",
        "droid" => "Droid", "opencode" => "OpenCode", "pi" => "Pi",
        "omp" => "Oh My Pi", other => other,
    };
    let settings = crate::server_settings::current();
    let pcfg = &settings["providers"][provider];
    if pcfg.is_object() && pcfg["enabled"].as_bool() == Some(false) {
        return Err(err(format!("{display} is disabled in Settings > Providers.")));
    }
    let binary = pcfg["binaryPath"].as_str().unwrap_or(provider).to_string();
    let on_path = if binary.contains('/') {
        std::path::Path::new(&binary).exists()
    } else {
        std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths).any(|p| p.join(&binary).exists())
            })
            .unwrap_or(false)
    };
    if !on_path {
        return Err(err(format!("{display} is not installed or not on PATH.")));
    }
    let now = crate::todo::iso_now_ms();
    let coordinator_name = payload["coordinatorName"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| title.clone());
    let mut coord_thread: Option<String> = None;
    let mut automation_id: Option<String> = None;
    let mut revision = 1i64;
    let mut created_at = now.clone();
    if let Some(e) = &existing {
        if let Some(exp) = payload["expectedRevision"].as_i64() {
            if exp != cfg_get(e, 9).and_then(|r| r.parse::<i64>().ok()).unwrap_or(0) {
                return Err(err("Coordinator settings changed. Reload and retry."));
            }
        }
        coord_thread = cfg_get(e, 0);
        automation_id = cfg_get(e, 8);
        revision = cfg_get(e, 9).and_then(|r| r.parse::<i64>().ok()).unwrap_or(0) + 1;
        created_at = cfg_get(e, 10).unwrap_or_else(|| now.clone());
    } else {
        // adopt a pre-existing coordinator thread (title match)
        coord_thread = conn
            .query_row(
                "SELECT thread_id FROM projection_threads WHERE project_id = ?1 AND deleted_at IS NULL \
                 AND (title = ?2 OR title = ?3) LIMIT 1",
                rusqlite::params![&project_id, &coordinator_name, format!("{title} Coordinator")],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        if coord_thread.is_none() {
            let tid = crate::orchestration_engine::uuid_v4();
            crate::orchestration_engine::handle(
                "orchestration.dispatchCommand",
                &json!({
                    "type": "thread.create", "commandId": crate::orchestration_engine::uuid_v4(),
                    "threadId": tid, "projectId": project_id, "title": coordinator_name,
                    "modelSelection": payload["coordinatorModelSelection"],
                    "runtimeMode": "approval-required", "interactionMode": "default",
                    "envMode": "local", "branch": null, "worktreePath": null,
                    "createdAt": now,
                }),
            )
            .map_err(|_| err("Failed to create coordinator thread."))?;
            coord_thread = Some(tid);
        }
    }
    if automation_id.is_none() {
        if let Some(ct) = &coord_thread {
            automation_id = conn
                .query_row(
                    "SELECT automation_id FROM automation_definitions \
                     WHERE project_id = ?1 AND mode = 'heartbeat' \
                     AND schedule_json LIKE '%\"project-event\"%' AND source_thread_id = ?2 LIMIT 1",
                    rusqlite::params![&project_id, ct],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| err(e.to_string()))?;
        }
    }
    // libraryPath must be absolute without '..' segments
    if let Some(lp) = payload.get("libraryPath") {
        if !lp.is_null() {
            let v = lp.as_str().unwrap_or_default();
            let bad = !v.starts_with('/') && !(v.len() > 1 && v.as_bytes()[1] == b':');
            if bad || v.split(['/', '\\']).any(|s| s == "..") {
                return Err(err("libraryPath must be an absolute path without '..' segments."));
            }
        }
    }
    // merge per-field keep/replace/null-clears semantics
    let pick = |key: &str, old: Option<String>| -> Option<String> {
        match payload.get(key) {
            None => old,
            Some(v) if v.is_null() => None,
            Some(v) => Some(v.to_string().trim_matches('"').to_string()),
        }
    };
    let pick_str = |key: &str, old: Option<String>| -> Option<String> {
        match payload.get(key) {
            None => old,
            Some(v) if v.is_null() => None,
            Some(v) => Some(v.as_str().unwrap_or_default().to_string()),
        }
    };
    let old_limits = existing.as_ref().and_then(|e| cfg_get(e, 5));
    let limits = match payload.get("limits") {
        Some(v) => v.to_string(),
        None => old_limits.unwrap_or_else(|| {
            json!({"maxConcurrentWorkers":3,"maxNewWorkersPerTurn":8,"maxWorkerCreationsPerGoal":40,
                   "maxAutomaticContinuationsPerGoal":20,"maxRepairRoundsPerTask":2}).to_string()
        }),
    };
    let mut c: Vec<Option<String>> = vec![None; 24];
    c[0] = coord_thread.clone();
    c[1] = Some(coordinator_name.clone());
    c[2] = Some(payload["coordinatorModelSelection"].to_string());
    c[3] = pick("coordinatorProviderOptions", existing.as_ref().and_then(|e| cfg_get(e, 3)));
    c[4] = pick("workerRouting", existing.as_ref().and_then(|e| cfg_get(e, 4)));
    c[5] = Some(limits);
    c[6] = Some(
        payload["captureEnabled"]
            .as_bool()
            .or_else(|| existing.as_ref().and_then(|e| cfg_get(e, 6).map(|v| v == "1")))
            .map(|b| if b { "1" } else { "0" }.to_string())
            .unwrap_or_else(|| "1".into()),
    );
    c[7] = Some("1".into());
    c[8] = automation_id.clone();
    c[9] = Some(revision.to_string());
    c[10] = Some(created_at);
    c[11] = Some(now.clone());
    c[12] = None;
    c[13] = pick_str("goal", existing.as_ref().and_then(|e| cfg_get(e, 13)));
    c[14] = pick_str("icon", existing.as_ref().and_then(|e| cfg_get(e, 14)));
    c[15] = pick_str("coordinatorIcon", existing.as_ref().and_then(|e| cfg_get(e, 15)));
    c[16] = pick_str("coordinatorColor", existing.as_ref().and_then(|e| cfg_get(e, 16)));
    c[17] = Some(
        payload["autoMemoryEnabled"]
            .as_bool()
            .or_else(|| existing.as_ref().and_then(|e| cfg_get(e, 17).map(|v| v == "1")))
            .map(|b| if b { "1" } else { "0" }.to_string())
            .unwrap_or_else(|| "0".into()),
    );
    c[18] = pick_str("libraryPath", existing.as_ref().and_then(|e| cfg_get(e, 18)));
    c[19] = pick_str("libraryRemoteUrl", existing.as_ref().and_then(|e| cfg_get(e, 19)));
    c[20] = Some(
        payload["libraryPushOnChange"]
            .as_bool()
            .or_else(|| existing.as_ref().and_then(|e| cfg_get(e, 20).map(|v| v == "1")))
            .map(|b| if b { "1" } else { "0" }.to_string())
            .unwrap_or_else(|| "0".into()),
    );
    c[21] = existing.as_ref().and_then(|e| cfg_get(e, 21));
    c[22] = existing.as_ref().and_then(|e| cfg_get(e, 22));
    c[23] = existing.as_ref().and_then(|e| cfg_get(e, 23)).or(Some("[]".into()));
    save_config(&conn, &project_id, &c,
        existing.as_ref().and_then(|e| cfg_get(e, 9).and_then(|r| r.parse().ok())),
        "Failed to save coordinator configuration.")?;
    // seed docs + digest when there are no document heads
    let head_count: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM project_agent_document_heads WHERE project_id = ?1",
            [&project_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if head_count == 0 {
        for (path, content) in [
            ("overview.md", "# Overview\n\nCoordinator is not configured.\n"),
            ("instructions.md", "# Instructions\n\n"),
            ("notes.md", "# Notes\n\n"),
            ("decisions.md", "# Decisions\n\n"),
            ("archived.md", "# Archived\n\n"),
            ("artifacts/index.md", "# Artifacts\n\n"),
        ] {
            let content = if path == "instructions.md" {
                payload["importedInstructions"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(content)
            } else {
                content
            };
            let hash = format!("{:x}", Sha256::digest(content.as_bytes()));
            conn.execute(
                "INSERT INTO project_agent_documents \
                 (revision_id, project_id, logical_path, revision, content, content_hash, \
                  author_kind, author_thread_id, sources_json, created_at) \
                 VALUES (?1,?2,?3,1,?4,?5,'system',NULL,'[]',?6)",
                rusqlite::params![
                    crate::orchestration_engine::uuid_v4(), project_id, path, content, hash, now,
                ],
            )
            .map_err(|e| err(e.to_string()))?;
            conn.execute(
                "INSERT INTO project_agent_document_heads \
                 (project_id, logical_path, revision, content_hash, disk_hash, conflict_pending, updated_at) \
                 VALUES (?1,?2,1,?3,?3,0,?4)",
                rusqlite::params![project_id, path, hash, now],
            )
            .map_err(|e| err(e.to_string()))?;
            let mirror = doc_context_path(&project_id, path);
            if let Some(p) = mirror.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            let _ = std::fs::write(&mirror, content);
        }
        if payload["importedInstructions"].as_str().map(|s| !s.trim().is_empty()) == Some(true) {
            append_activity(&conn, &project_id, "document-written", "user", None, None, None,
                "Imported existing project instructions without overwriting newer server content.", &now)?;
        }
        // index project threads into thread_index
        let threads = group_thread_ids(&conn, &project_id);
        let excluded: std::collections::HashSet<String> = {
            let mut s = std::collections::HashSet::new();
            if let Ok(mut q) = conn.prepare(
                "SELECT thread_id FROM project_agent_thread_index WHERE project_id = ?1 AND excluded = 1",
            ) {
                if let Ok(rows) = q.query_map([&project_id], |r| r.get::<_, String>(0)) {
                    s = rows.flatten().collect();
                }
            }
            s
        };
        let mut pending = 0i64;
        let mut summarized = 0i64;
        for (i, (tid, archived)) in threads.iter().enumerate() {
            let ex = excluded.contains(tid);
            let covered = !ex && i < 20;
            if covered {
                summarized += 1;
            } else if !ex {
                pending += 1;
            }
            let status = if ex { "skipped" } else if covered { "covered" } else { "pending" };
            let _ = conn.execute(
                "INSERT OR REPLACE INTO project_agent_thread_index \
                 (project_id, thread_id, excluded, archived, summary_status, last_updated_at, last_summarized_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                rusqlite::params![
                    project_id, tid, if ex { 1 } else { 0 }, if *archived { 1 } else { 0 },
                    status, now,
                    if covered { Some(now.clone()) } else { None::<String> },
                ],
            );
        }
        conn.execute(
            "INSERT OR REPLACE INTO project_agent_digests \
             (project_id, summary, focus_items_json, pinned_focus_json, coverage_from_sequence, \
              coverage_to_sequence, historical_coverage, summarized_thread_count, \
              pending_thread_count, generation_state, generated_at, last_good_at, last_error) \
             VALUES (?1,'Coordinator is ready.','[]','[]',0,0,?2,?3,?4,'idle',?5,?5,NULL)",
            rusqlite::params![
                project_id,
                if pending > 0 { "partial" } else { "none" },
                summarized,
                pending,
                now,
            ],
        )
        .map_err(|e| err(e.to_string()))?;
    }
    // coordinator greeting (thread.messages.import, welcome id sha1-v5)
    if let Some(ct) = &coord_thread {
        let msg_id = welcome_message_id(ct);
        let has_msg: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM projection_messages WHERE message_id = ?1",
                [msg_id.as_str()],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if has_msg == 0 {
            let name = payload["userDisplayName"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| {
                    home_dir()
                        .trim_end_matches(['/', '\\'])
                        .rsplit(['/', '\\'])
                        .next()
                        .unwrap_or("there")
                        .to_string()
                });
            let text = format!(
                "Hi {name}, welcome to your new hub. I coordinate the work here: ask for whatever you need, and I'll either answer you directly or start threads to work on things in parallel.\n\nThreads do the work on their own. The Hub panel on the right shows which ones are working, which are done, and which need you. I'll keep an eye on what's running and post updates when something finishes or needs you. You can also ask me to change my instructions, add repositories, or send you scheduled updates."
            );
            let _ = crate::orchestration_engine::handle(
                "orchestration.dispatchCommand",
                &json!({
                    "type": "thread.messages.import", "commandId": crate::orchestration_engine::uuid_v4(),
                    "threadId": ct,
                    "messages": [{"messageId": msg_id, "role": "assistant", "text": text,
                                  "createdAt": now, "updatedAt": now}],
                    "createdAt": now,
                }),
            );
        }
    }
    // create heartbeat automation if missing
    if automation_id.is_none() {
        if let Some(ct) = &coord_thread {
            let aid = crate::orchestration_engine::uuid_v4();
            conn.execute(
                "INSERT INTO automation_definitions \
                 (automation_id, project_id, source_thread_id, name, prompt, schedule_json, \
                  enabled, next_run_at, model_selection_json, runtime_mode, interaction_mode, \
                  worktree_mode, mode, target_thread_id, stop_on_error, minimum_interval_seconds, \
                  max_runtime_seconds, retry_policy_json, misfire_policy, acknowledged_risks_json, \
                  iteration_count, created_at, updated_at, managed_by_project) \
                 VALUES (?1,?2,?3,?4,?5,?6,1,NULL,?7,'approval-required','default','auto',\
                  'heartbeat',?8,1,60,3600,'{\"type\":\"none\"}','coalesce','[]',0,?9,?9,1)",
                rusqlite::params![
                    aid, project_id, ct,
                    format!("{coordinator_name} events"),
                    "Watch this hub's threads. When a thread finishes, dies, errors, hits a quota limit, or is interrupted, reply in this chat with a short status. Review the outcome and update decisions.md.  Do not expand scope. Do not ask the user to start a goal. If everything is on track and nothing needs the user's attention, reply with exactly \"SILENT\" — no other text.",
                    json!({"type":"project-event","projectId":project_id}).to_string(),
                    payload["coordinatorModelSelection"].to_string(),
                    ct,
                    now,
                ],
            )
            .map_err(|e| err(e.to_string()))?;
            let mut c2 = c.clone();
            c2[8] = Some(aid.clone());
            c2[9] = Some((revision + 1).to_string());
            c2[11] = Some(crate::todo::iso_now_ms());
            let _ = save_config(&conn, &project_id, &c2, Some(revision),
                "Failed to link project automation.");
        }
    }
    // model changed → rebind live thread
    let old_model = existing.as_ref().and_then(|e| cfg_get(e, 2));
    if old_model.is_some() && old_model.as_deref() != Some(payload["coordinatorModelSelection"].to_string().as_str()) {
        if let Some(ct) = &coord_thread {
            let _ = crate::orchestration_engine::handle(
                "orchestration.dispatchCommand",
                &json!({
                    "type": "thread.meta.update",
                    "commandId": crate::orchestration_engine::uuid_v4(),
                    "threadId": ct,
                    "modelSelection": payload["coordinatorModelSelection"],
                    "createdAt": now,
                }),
            );
        }
    }
    append_activity(&conn, &project_id, "config-updated", "user", None, None, None,
        if existing.is_some() {
            "Updated Project Coordinator settings."
        } else {
            "Configured Project Coordinator."
        }, &now)?;
    let overview = overview_for(&conn, &project_id)?;
    store_receipt(&conn, &request_id, &project_id, "configure", &overview);
    Ok(overview)
}

fn export_documents(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let copy = open_state_copy().ok_or_else(|| err("Failed to load hub."))?;
    let conn = &copy.conn;
    require_hub_project(conn, &project_id)?;
    let paths = payload["logicalPaths"].as_array().cloned().unwrap_or_default();
    if paths.len() > 50 {
        return Err(err("Export at most 50 documents at a time."));
    }
    let dest_root = payload["destinationDirectory"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let dest_abs = std::fs::canonicalize(&dest_root).unwrap_or_else(|_| PathBuf::from(&dest_root));
    let mut exported: Vec<Value> = Vec::new();
    for raw in paths {
        let logical = normalize_doc_path(raw.as_str().unwrap_or(""))?;
        let content: Option<String> = conn
            .query_row(
                "SELECT content FROM project_agent_documents \
                 WHERE project_id = ?1 AND logical_path = ?2 ORDER BY revision DESC LIMIT 1",
                rusqlite::params![&project_id, &logical],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        let Some(content) = content else { continue };
        let dest = dest_abs.join(&logical);
        if !dest.starts_with(&dest_abs) {
            return Err(err("Export destination escaped the chosen directory."));
        }
        if let Some(p) = dest.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        std::fs::write(&dest, &content)
            .map_err(|_| err("Failed to export project document."))?;
        exported.push(Value::String(dest.to_string_lossy().to_string()));
    }
    Ok(json!({"exportedPaths": exported}))
}

fn restart_coordinator(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to restart the coordinator."))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    require_hub_project(&conn, &project_id)?;
    let cfg = config_row(&conn, &project_id)?
        .ok_or_else(|| err("Project Coordinator is not configured."))?;
    if cfg_get(&cfg, 21).is_some() || cfg_get(&cfg, 22).is_some() {
        return Err(err("Resume or unarchive the hub before restarting its coordinator."));
    }
    if let Some(ct) = cfg_get(&cfg, 0) {
        dispatch_thread("thread.session.stop", &ct);
    }
    append_activity(&conn, &project_id, "config-updated", "user", None, None, None,
        "Restarted coordinator.", &crate::todo::iso_now_ms())?;
    let overview = overview_for(&conn, &project_id)?;
    store_receipt(&conn, &request_id, &project_id, "restartCoordinator", &overview);
    Ok(overview)
}

fn delete_group(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let request_id = payload["requestId"].as_str().unwrap_or_default().to_string();
    let conn = crate::orchestration_engine::open_state_rw()
        .map_err(|_| err("Failed to delete the hub."))?;
    if let Some(r) = replay_receipt(&conn, &request_id, &project_id) {
        return Ok(r);
    }
    require_hub_project(&conn, &project_id)?;
    let title: String = conn
        .query_row(
            "SELECT title FROM projection_projects WHERE project_id = ?1",
            [&project_id],
            |r| r.get(0),
        )
        .unwrap_or_default();
    let confirm = payload["confirmName"].as_str().unwrap_or_default().trim();
    if confirm != title.trim() {
        return Err(err("Type the hub's name exactly to confirm deletion."));
    }
    if payload["requireEmpty"].as_bool() == Some(true) {
        let members: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM projection_threads WHERE project_id = ?1 AND deleted_at IS NULL",
                [&project_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let tasks: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM project_agent_tasks WHERE project_id = ?1",
                [&project_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if members > 0 || tasks > 0 {
            return Err(err("The hub still has work in it. Delete its threads and tasks first, or confirm deletion without the empty check."));
        }
        let non_seed: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM project_agent_document_heads WHERE project_id = ?1 \
                 AND logical_path NOT IN ('overview.md','instructions.md','notes.md','decisions.md','archived.md','artifacts/index.md')",
                [&project_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if non_seed > 0 {
            return Err(err("The hub still has work in it. Delete its threads and tasks first, or confirm deletion without the empty check."));
        }
    }
    // delete group threads (members only), then wipe project-agent tables
    let coord = config_row(&conn, &project_id)
        .ok()
        .flatten()
        .and_then(|c| cfg_get(&c, 0));
    if let Some(ct) = &coord {
        dispatch_thread("thread.delete", ct);
    }
    for (tid, _) in group_thread_ids(&conn, &project_id) {
        if Some(&tid) != coord.as_ref() {
            dispatch_thread("thread.delete", &tid);
        }
    }
    for t in [
        "project_agent_configs", "project_agent_tasks", "project_agent_task_dependencies",
        "project_agent_goals", "project_agent_activity", "project_agent_receipts",
        "project_agent_linked_projects", "project_agent_digests",
        "project_agent_thread_index", "project_agent_documents",
        "project_agent_document_heads", "project_agent_evidence",
        "project_agent_memory_index", "managed_workers",
    ] {
        let _ = conn.execute(
            &format!("DELETE FROM {t} WHERE project_id = ?1"),
            [&project_id],
        );
    }
    let _ = conn.execute(
        "DELETE FROM automation_definitions WHERE project_id = ?1",
        [&project_id],
    );
    // move the default library to trash (never a custom libraryPath)
    let mirror_root = PathBuf::from(state_dir()).join("project-context").join(&project_id);
    let home = home_dir();
    let trash = PathBuf::from(&home).join(".Trash");
    if mirror_root.exists() && trash.exists() {
        let target = trash.join(format!(
            "synara-project-context-{}-{}",
            project_id,
            crate::todo::iso_now_ms().replace([':', '.'], "-")
        ));
        let _ = std::fs::rename(&mirror_root, target);
    }
    // project.delete via orchestration
    let _ = crate::orchestration_engine::handle(
        "orchestration.dispatchCommand",
        &json!({
            "type": "project.delete",
            "commandId": crate::orchestration_engine::uuid_v4(),
            "projectId": project_id,
            "createdAt": crate::todo::iso_now_ms(),
        }),
    );
    let result = json!({"deletedProjectId": project_id});
    store_receipt(&conn, &request_id, &project_id, "deleteGroup", &result);
    Ok(result)
}

fn library_root(conn: &Connection, project_id: &str) -> Result<PathBuf, WsErr> {
    let cfg = config_row(conn, project_id).unwrap_or(None);
    if let Some(lp) = cfg.as_ref().and_then(|c| cfg_get(c, 18)) {
        let bad = !lp.starts_with('/') && !(lp.len() > 1 && lp.as_bytes()[1] == b':');
        if bad || lp.split(['/', '\\']).any(|s| s == "..") {
            return Err(err("libraryPath must be an absolute path without '..' segments."));
        }
        return Ok(PathBuf::from(lp));
    }
    Ok(PathBuf::from(state_dir())
        .join("project-context")
        .join(project_id)
        .join("library"))
}

fn ensure_library_repo(root: &PathBuf, managed: bool) -> Result<(), WsErr> {
    let _ = std::fs::create_dir_all(root);
    if managed && !root.join(".git").exists() {
        let _ = std::fs::create_dir_all(root.join("Artifacts"));
        let _ = std::fs::write(root.join(".gitignore"), ".DS_Store\nThumbs.db\n");
        let _ = std::fs::write(
            root.join(".gitattributes"),
            "*.png binary\n*.jpg binary\n*.jpeg binary\n*.gif binary\n*.webp binary\n*.bmp binary\n*.ico binary\n*.pdf binary\n*.zip binary\n*.tar binary\n*.gz binary\n*.7z binary\n*.mp4 binary\n*.mp3 binary\n*.wav binary\n",
        );
        for args in [
            vec!["init", "-q"],
            vec!["add", "-A"],
            vec![
                "-c", "user.name=Synara Library", "-c", "user.email=library@synara.local",
                "commit", "-q", "-m", "Initialize hub library", "--allow-empty",
            ],
        ] {
            let _ = std::process::Command::new("git")
                .args(&args)
                .current_dir(root)
                .output();
        }
    }
    Ok(())
}

fn library_list(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let copy = open_state_copy().ok_or_else(|| err("Failed to list the hub library."))?;
    let conn = &copy.conn;
    require_hub_project(conn, &project_id)?;
    let root = library_root(conn, &project_id)?;
    ensure_library_repo(&root, true)
        .map_err(|_| err("Failed to prepare the hub library."))?;
    let rel = payload["relativePath"].as_str().unwrap_or("");
    let normalized = if rel.is_empty() {
        String::new()
    } else {
        normalize_doc_path(rel)
            .map_err(|_| err(format!("Library path \"{rel}\" is not a valid relative path.")))?
    };
    let dir = if normalized.is_empty() {
        root.clone()
    } else {
        root.join(&normalized)
    };
    if !dir.starts_with(&root) || !dir.is_dir() {
        return Err(err("Failed to list the library directory."));
    }
    let mut entries: Vec<Value> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.')
                || matches!(name.as_str(), ".DS_Store" | "Thumbs.db" | ".git")
            {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            if meta.file_type().is_symlink() {
                continue;
            }
            let kind = if meta.is_dir() {
                "directory"
            } else if meta.is_file() {
                "file"
            } else {
                continue;
            };
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| crate::todo::iso_from_ms(d.as_millis() as i64))
                .unwrap_or_default();
            entries.push(json!({
                "name": name,
                "relativePath": if normalized.is_empty() {
                    name.clone()
                } else {
                    format!("{normalized}/{name}")
                },
                "kind": kind,
                "sizeBytes": if kind == "file" { meta.len() } else { 0 },
                "modifiedAt": modified,
            }));
        }
    }
    entries.sort_by(|a, b| {
        let ka = a["kind"].as_str().unwrap_or("");
        let kb = b["kind"].as_str().unwrap_or("");
        ka.cmp(kb).then(a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")))
    });
    Ok(json!({"root": root.to_string_lossy(), "entries": entries}))
}

fn welcome_message_id(thread_id: &str) -> String {
    // sha1 v5-style: hash `${threadId}:welcome`, set variant+version bits
    let mut h = sha1::Sha1::new();
    sha1::Digest::update(&mut h, format!("{thread_id}:welcome").as_bytes());
    let mut bytes: [u8; 20] = sha1::Digest::finalize(h).into();
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes[..16].iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32]
    )
}

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "projectAgent.listTasks"
            | "projectAgent.listActivity"
            | "projectAgent.listDocuments"
            | "projectAgent.readDocument"
            | "projectAgent.listThreadIndex"
            | "projectAgent.listEvidence"
            | "projectAgent.listSummaries"
            | "projectAgent.getOverview"
            | "projectAgent.createTask"
            | "projectAgent.updateTask"
            | "projectAgent.startGoal"
            | "projectAgent.updateGoal"
            | "projectAgent.pauseGoal"
            | "projectAgent.resumeGoal"
            | "projectAgent.stopGoal"
            | "projectAgent.excludeThread"
            | "projectAgent.configure"
            | "projectAgent.linkProject"
            | "projectAgent.unlinkProject"
            | "projectAgent.writeDocument"
            | "projectAgent.pause"
            | "projectAgent.resume"
            | "projectAgent.archive"
            | "projectAgent.unarchive"
            | "projectAgent.exportDocuments"
            | "projectAgent.restartCoordinator"
            | "projectAgent.delete"
            | "projectAgent.library.list"
    )
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "projectAgent.listTasks" => list_tasks(payload),
        "projectAgent.listActivity" => list_activity(payload),
        "projectAgent.listDocuments" => list_documents(payload),
        "projectAgent.readDocument" => read_document(payload),
        "projectAgent.listThreadIndex" => list_thread_index(payload),
        "projectAgent.listEvidence" => list_evidence(payload),
        "projectAgent.listSummaries" => list_summaries(payload),
        "projectAgent.getOverview" => get_overview(payload),
        "projectAgent.createTask" => create_task(payload),
        "projectAgent.updateTask" => update_task(payload),
        "projectAgent.startGoal" => start_goal(payload),
        "projectAgent.updateGoal" => update_goal(payload),
        "projectAgent.pauseGoal" => update_goal_status(
            payload,
            "paused",
            "goal-paused",
            "Paused the project goal. Current tasks may settle.",
        ),
        "projectAgent.resumeGoal" => {
            update_goal_status(payload, "active", "goal-resumed", "Resumed the project goal.")
        }
        "projectAgent.stopGoal" => update_goal_status(
            payload,
            "stopped",
            "goal-stopped",
            "Stopped the project goal. Pending continuations were cancelled.",
        ),
        "projectAgent.excludeThread" => exclude_thread(payload),
        "projectAgent.configure" => configure(payload),
        "projectAgent.linkProject" => link_project(payload, false),
        "projectAgent.unlinkProject" => link_project(payload, true),
        "projectAgent.writeDocument" => write_document(payload),
        "projectAgent.pause" => {
            lifecycle_group(payload, "pause", "Paused hub.", "Failed to pause the hub.")
        }
        "projectAgent.resume" => {
            lifecycle_group(payload, "resume", "Resumed hub.", "Failed to resume the hub.")
        }
        "projectAgent.archive" => {
            lifecycle_group(payload, "archive", "Archived hub.", "Failed to archive the hub.")
        }
        "projectAgent.unarchive" => lifecycle_group(
            payload,
            "unarchive",
            "Unarchived hub.",
            "Failed to unarchive the hub.",
        ),
        "projectAgent.exportDocuments" => export_documents(payload),
        "projectAgent.restartCoordinator" => restart_coordinator(payload),
        "projectAgent.delete" => delete_group(payload),
        "projectAgent.library.list" => library_list(payload),
        _ => Err(err("Unknown projectAgent method")),
    }
}
