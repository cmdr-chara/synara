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

// ---- methods ----

fn list_tasks(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload["projectId"].as_str().unwrap_or_default().to_string();
    let goal_id = payload["goalId"].as_str().map(str::to_string);
    let include_archived = payload["includeArchived"].as_bool().unwrap_or(false);
    let limit = payload["limit"].as_i64().unwrap_or(50);
    let cursor = decode_cursor(payload["cursor"].as_str());
    with_state_copy("Failed to list project tasks.", move |conn| {
        let mut sql = String::from(
            "SELECT task_id, project_id, goal_id, title, description, acceptance_criteria, status, \
             assigned_thread_id, repair_count, archived_at, revision, created_at, updated_at \
             FROM project_agent_tasks WHERE project_id = ?1",
        );
        let mut params: Vec<rusqlite::types::Value> = vec![project_id.clone().into()];
        if let Some(g) = &goal_id {
            sql.push_str(" AND goal_id = ?");
            params.push(g.clone().into());
        }
        if !include_archived {
            sql.push_str(" AND archived_at IS NULL");
        }
        if let Some(c) = &cursor {
            sql.push_str(
                " AND (created_at < ? OR (created_at = ? AND task_id < ?))",
            );
            params.push(c.created_at.clone().into());
            params.push(c.created_at.clone().into());
            params.push(c.id.clone().into());
        }
        sql.push_str(" ORDER BY created_at DESC, task_id DESC LIMIT ?");
        params.push(limit.into());
        let mut stmt = conn.prepare(&sql).map_err(|e| err(e.to_string()))?;
        let mut tasks = Vec::new();
        let mut rows_iter = stmt
            .query(params_from_iter(params))
            .map_err(|e| err(e.to_string()))?;
        let mut last: Option<(String, String)> = None;
        while let Some(row) = rows_iter.next().map_err(|e| err(e.to_string()))? {
            let id: String = row.get(0).map_err(|e| err(e.to_string()))?;
            let created: String = row.get(11).map_err(|e| err(e.to_string()))?;
            last = Some((created, id));
            tasks.push(task_row(conn, row)?);
        }
        drop(rows_iter);
        drop(stmt);
        let next_cursor = if tasks.len() as i64 == limit && tasks.len() > 0 {
            let (created, id) = last.unwrap();
            Value::String(encode_cursor(&created, &id, None))
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
        let mut sql = String::from(
            "SELECT activity_id, project_id, sequence, kind, actor_kind, actor_thread_id, \
             goal_id, task_id, source_json, summary, created_at \
             FROM project_agent_activity WHERE project_id = ?1",
        );
        let mut params: Vec<rusqlite::types::Value> = vec![project_id.into()];
        if let Some(c) = &cursor {
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
        let mut last: Option<(String, String, i64)> = None;
        while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
            let seq: i64 = row.get(2).map_err(|e| err(e.to_string()))?;
            let id: String = row.get(0).map_err(|e| err(e.to_string()))?;
            let created: String = row.get(10).map_err(|e| err(e.to_string()))?;
            last = Some((created, id, seq));
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
        let next_cursor = if activity.len() as i64 == limit && activity.len() > 0 {
            let (created, id, seq) = last.unwrap();
            Value::String(encode_cursor(&created, &id, Some(seq)))
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

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "projectAgent.listTasks"
            | "projectAgent.listActivity"
            | "projectAgent.listDocuments"
            | "projectAgent.readDocument"
            | "projectAgent.listThreadIndex"
            | "projectAgent.listEvidence"
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
        _ => Err(err("Unknown projectAgent method")),
    }
}
