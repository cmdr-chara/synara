//! Durable Git handoff operation journal, ported from upstream
//! `gitHandoffOperations.ts`. One durable handoff keyed by command id:
//!
//!   pending ─record result─> git_applied ─complete─> completed
//!      │                          ^ startup recovery replays the metadata, then completes
//!      ├─startup recovery─> uncertain   (process died before the result was durable)
//!      └─discard─> (row deleted; git failed, so the command may run again)
//!
//! `completed` and `uncertain` are terminal. Only `git_applied` and
//! `completed` carry a result.

use rusqlite::params;
use serde_json::{json, Value};

use crate::orchestration_engine::open_state_rw;
use crate::todo::{iso_now_ms, WsErr};

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

pub fn ensure_table(conn: &rusqlite::Connection) {
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS git_handoff_operations (
            command_id TEXT PRIMARY KEY,
            thread_id TEXT NOT NULL,
            input_json TEXT NOT NULL,
            phase TEXT NOT NULL,
            result_json TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    );
}

pub enum Operation {
    New,
    Pending,
    Uncertain,
    GitApplied(Value),
    Completed(Value),
}

pub fn begin(input: &Value) -> Result<Operation, WsErr> {
    let conn = open_state_rw()?;
    ensure_table(&conn);
    let command_id = input
        .get("commandId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("git handoff payload is missing commandId"))?;
    let thread_id = input
        .get("threadId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("git handoff payload is missing threadId"))?;
    let input_json = serde_json::to_string(input).unwrap_or_default();
    let now = iso_now_ms();
    let inserted = conn
        .execute(
            "INSERT OR IGNORE INTO git_handoff_operations
             (command_id, thread_id, input_json, phase, result_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'pending', NULL, ?4, ?4)",
            params![command_id, thread_id, input_json, now],
        )
        .map_err(|e| err(format!("Failed to begin Git handoff operation: {e}")))?;
    if inserted > 0 {
        return Ok(Operation::New);
    }
    let row = conn
        .query_row(
            "SELECT thread_id, input_json, phase, result_json FROM git_handoff_operations WHERE command_id = ?1",
            params![command_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(|e| err(format!("Failed to read Git handoff operation: {e}")))?;
    if row.0 != thread_id || row.1 != input_json {
        return Err(err(format!(
            "Git handoff command identity {command_id} was reused with different input."
        )));
    }
    let result = row.3.and_then(|j| serde_json::from_str::<Value>(&j).ok());
    Ok(match row.2.as_str() {
        "pending" => Operation::Pending,
        "uncertain" => Operation::Uncertain,
        "git_applied" => Operation::GitApplied(result.unwrap_or(Value::Null)),
        "completed" => Operation::Completed(result.unwrap_or(Value::Null)),
        other => return Err(err(format!("Unknown Git handoff phase {other}."))),
    })
}

pub fn record_result(command_id: &str, result: &Value) -> Result<(), WsErr> {
    let conn = open_state_rw()?;
    ensure_table(&conn);
    let now = iso_now_ms();
    conn.execute(
        "UPDATE git_handoff_operations SET phase = 'git_applied', result_json = ?2, updated_at = ?3
         WHERE command_id = ?1 AND phase = 'pending'",
        params![command_id, serde_json::to_string(result).unwrap_or_default(), now],
    )
    .map_err(|e| err(format!("Failed to persist applied Git handoff result: {e}")))?;
    Ok(())
}

pub fn complete(command_id: &str) -> Result<(), WsErr> {
    let conn = open_state_rw()?;
    ensure_table(&conn);
    let now = iso_now_ms();
    conn.execute(
        "UPDATE git_handoff_operations SET phase = 'completed', updated_at = ?2
         WHERE command_id = ?1 AND phase = 'git_applied'",
        params![command_id, now],
    )
    .map_err(|e| err(format!("Failed to complete Git handoff operation: {e}")))?;
    Ok(())
}

pub fn discard_pending(command_id: &str) {
    if let Ok(conn) = open_state_rw() {
        ensure_table(&conn);
        let _ = conn.execute(
            "DELETE FROM git_handoff_operations WHERE command_id = ?1 AND phase = 'pending'",
            params![command_id],
        );
    }
}

/// `resolveWorktreeHandoffWorkspaceMetadata` + `gitHandoffMetadataCommand`:
/// the thread.meta.update payload mirroring the handoff result.
pub fn metadata_command(command_id: &str, thread_id: &str, result: &Value) -> Value {
    let str_field = |key: &str| result.get(key).cloned().unwrap_or(Value::Null);
    let target_mode = result
        .get("targetMode")
        .and_then(Value::as_str)
        .unwrap_or("local");
    let mut command = json!({
        "type": "thread.meta.update",
        "commandId": command_id,
        "threadId": thread_id,
        "envMode": target_mode,
        "branch": str_field("branch"),
        "worktreePath": str_field("worktreePath"),
        "associatedWorktreePath": str_field("associatedWorktreePath"),
        "associatedWorktreeBranch": str_field("associatedWorktreeBranch"),
        "associatedWorktreeRef": str_field("associatedWorktreeRef"),
    });
    if target_mode == "worktree" {
        command["createBranchFlowCompleted"] = json!(false);
    }
    command
}

/// Startup recovery, upstream recoverGitHandoffOperations: fence interrupted
/// pending rows as uncertain, then replay the metadata command for every
/// git_applied row and complete it. Returns (fenced, recovered) counts.
pub fn recover() -> (usize, usize) {
    let Ok(conn) = open_state_rw() else { return (0, 0) };
    ensure_table(&conn);
    let now = iso_now_ms();
    let fenced = conn
        .execute(
            "UPDATE git_handoff_operations SET phase = 'uncertain', updated_at = ?1 WHERE phase = 'pending'",
            params![now],
        )
        .unwrap_or(0);
    let rows: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT command_id, thread_id, result_json FROM git_handoff_operations
             WHERE phase = 'git_applied' ORDER BY updated_at ASC, command_id ASC",
        )
        .and_then(|mut stmt| {
            let mapped = stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
            })?;
            Ok(mapped.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();
    let mut recovered = 0;
    for (command_id, thread_id, result_json) in rows {
        let Ok(result) = serde_json::from_str::<Value>(&result_json) else { continue };
        let command = metadata_command(&command_id, &thread_id, &result);
        if let Some(obj) = command.as_object() {
            if crate::orchestration_engine::thread_meta_update(obj).is_ok()
                && complete(&command_id).is_ok()
            {
                recovered += 1;
            }
        }
    }
    (fenced, recovered)
}


pub fn handles(tag: &str) -> bool {
    tag == "git.handoffThread"
}

/// WS handler with the journal lifecycle, upstream wsRpc gitHandoffThread.
pub fn handle(payload: &Value) -> Result<Value, WsErr> {
    let cwd = payload
        .get("cwd")
        .and_then(Value::as_str)
        .ok_or_else(|| err("git handoff payload is missing cwd"))?;
    let command_id = payload
        .get("commandId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let thread_id = payload
        .get("threadId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if command_id.is_empty() || thread_id.is_empty() {
        // Older clients without journal identifiers keep the direct path.
        return crate::git_pr::handoff_thread(cwd, payload);
    }
    match begin(payload)? {
        Operation::Pending => Err(err("This Git handoff is already running.")),
        Operation::Uncertain => Err(err(
            "This Git handoff was interrupted before its filesystem result became durable; inspect the repository before retrying.",
        )),
        Operation::Completed(result) => Ok(result),
        Operation::GitApplied(result) => {
            // Git work already landed before the crash; replay only the
            // metadata command, then complete the journal entry.
            if let Some(obj) = metadata_command(&command_id, &thread_id, &result).as_object() {
                let _ = crate::orchestration_engine::thread_meta_update(obj);
            }
            let _ = complete(&command_id);
            Ok(result)
        }
        Operation::New => match crate::git_pr::handoff_thread(cwd, payload) {
            Ok(result) => {
                let _ = record_result(&command_id, &result);
                if let Some(obj) = metadata_command(&command_id, &thread_id, &result).as_object() {
                    let _ = crate::orchestration_engine::thread_meta_update(obj);
                }
                let _ = complete(&command_id);
                Ok(result)
            }
            Err(e) => {
                discard_pending(&command_id);
                Err(e)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_command_shape() {
        let result = json!({
            "targetMode": "worktree",
            "branch": "feature-x",
            "worktreePath": "/tmp/wt",
            "associatedWorktreePath": null,
            "associatedWorktreeBranch": null,
            "associatedWorktreeRef": null,
        });
        let cmd = metadata_command("c-1", "t-1", &result);
        assert_eq!(cmd["type"], "thread.meta.update");
        assert_eq!(cmd["envMode"], "worktree");
        assert_eq!(cmd["branch"], "feature-x");
        assert_eq!(cmd["createBranchFlowCompleted"], false);
        let local = metadata_command("c-2", "t-2", &json!({"targetMode": "local"}));
        assert!(local.get("createBranchFlowCompleted").is_none());
    }
}
