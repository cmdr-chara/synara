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
            "SELECT kind, workspace_root FROM projection_projects WHERE project_id = ?1",
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
        _ => Err(err("Unknown projectAgent method")),
    }
}
