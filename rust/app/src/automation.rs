//! Native `automation.*` WS-RPC handlers (list/getMemory/run triage/delete/
//! subscribe). Reads run against a byte copy of state.sqlite; writes require
//! `SYNARA_NO_UPSTREAM` (upstream holds the db lock otherwise).

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};

use crate::orchestration_engine::open_state_rw;
use crate::orchestration_snapshot::open_state_copy;
use crate::todo::WsErr;

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

fn no_upstream() -> bool {
    std::env::var("SYNARA_NO_UPSTREAM").map(|v| v == "1").unwrap_or(false)
}

fn iso_now() -> String {
    std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Opens the state db copy and runs `f` on the connection.
fn with_state_copy<T>(
    message: &'static str,
    f: impl FnOnce(&Connection) -> Result<T, WsErr>,
) -> Result<T, WsErr> {
    let Some(copy) = open_state_copy() else {
        return Err(err(message));
    };
    f(&copy.conn)
}

fn json_col(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, Option<String>>(idx) {
        Ok(Some(s)) => serde_json::from_str(&s).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn opt_str(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, Option<String>>(idx) {
        Ok(Some(s)) => Value::String(s),
        _ => Value::Null,
    }
}

fn opt_int(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, Option<i64>>(idx) {
        Ok(Some(n)) => json!(n),
        _ => Value::Null,
    }
}

fn sval(row: &rusqlite::Row, idx: usize) -> Value {
    match row.get::<_, String>(idx) {
        Ok(s) => Value::String(s),
        _ => Value::Null,
    }
}

fn definition_from_row(row: &rusqlite::Row) -> Map<String, Value> {
    let mut d = Map::new();
    d.insert("id".into(), sval(row, 0));
    d.insert("projectId".into(), sval(row, 1));
    d.insert("sourceThreadId".into(), opt_str(row, 2));
    d.insert("name".into(), sval(row, 3));
    d.insert("prompt".into(), sval(row, 4));
    d.insert("schedule".into(), json_col(row, 5));
    d.insert("enabled".into(), json!(row.get::<_, i64>(6).unwrap_or(0) == 1));
    d.insert("nextRunAt".into(), opt_str(row, 7));
    d.insert("modelSelection".into(), json_col(row, 8));
    // providerOptions (col 9) is stripped from the wire list shape.
    d.insert("runtimeMode".into(), sval(row, 10));
    d.insert("interactionMode".into(), sval(row, 11));
    d.insert("worktreeMode".into(), sval(row, 12));
    d.insert("mode".into(), sval(row, 13));
    d.insert("targetThreadId".into(), opt_str(row, 14));
    d.insert("proposalState".into(), opt_str(row, 15));
    d.insert("notificationPolicy".into(), sval(row, 16));
    d.insert("heartbeatCooldownSeconds".into(), json!(row.get::<_, i64>(17).unwrap_or(60)));
    d.insert("maxIterations".into(), opt_int(row, 18));
    d.insert(
        "stopAfterConsecutiveFailures".into(),
        opt_int(row, 19),
    );
    d.insert(
        "consecutiveFailureCount".into(),
        json!(row.get::<_, i64>(20).unwrap_or(0)),
    );
    d.insert("disabledReason".into(), opt_str(row, 21));
    d.insert("disabledAt".into(), opt_str(row, 22));
    d.insert("completionPolicy".into(), json_col(row, 23));
    d.insert(
        "completionPolicyVersion".into(),
        json!(row.get::<_, i64>(24).unwrap_or(0)),
    );
    d.insert("completionPolicyUpdatedAt".into(), sval(row, 25));
    d.insert("minimumIntervalSeconds".into(), json!(row.get::<_, i64>(26).unwrap_or(60)));
    d.insert("maxRuntimeSeconds".into(), opt_int(row, 27));
    d.insert("retryPolicy".into(), json_col(row, 28));
    d.insert("misfirePolicy".into(), sval(row, 29));
    d.insert(
        "acknowledgedRisks".into(),
        match json_col(row, 30) {
            Value::Array(a) => Value::Array(a),
            _ => json!([]),
        },
    );
    d.insert("iterationCount".into(), json!(row.get::<_, i64>(31).unwrap_or(0)));
    d.insert(
        "managedByProject".into(),
        json!(row.get::<_, i64>(32).unwrap_or(0) == 1),
    );
    d.insert("createdAt".into(), sval(row, 33));
    d.insert("updatedAt".into(), sval(row, 34));
    d.insert("archivedAt".into(), opt_str(row, 35));
    d
}

const DEF_COLS: &str = "automation_id, project_id, source_thread_id, name, prompt, \
    schedule_json, enabled, next_run_at, model_selection_json, provider_options_json, \
    runtime_mode, interaction_mode, worktree_mode, mode, target_thread_id, proposal_state, \
    COALESCE(notification_policy,'all'), COALESCE(heartbeat_cooldown_seconds,60), \
    max_iterations, stop_after_consecutive_failures, COALESCE(consecutive_failure_count,0), \
    disabled_reason, disabled_at, completion_policy_json, COALESCE(completion_policy_version,0), \
    COALESCE(completion_policy_updated_at, updated_at, created_at, '1970-01-01T00:00:00.000Z'), \
    COALESCE(minimum_interval_seconds,60), max_runtime_seconds, retry_policy_json, \
    misfire_policy, acknowledged_risks_json, COALESCE(iteration_count,0), \
    COALESCE(managed_by_project,0), created_at, updated_at, archived_at";

fn run_from_row(row: &rusqlite::Row) -> Map<String, Value> {
    let mut r = Map::new();
    r.insert("id".into(), sval(row, 0));
    r.insert("automationId".into(), sval(row, 1));
    r.insert("projectId".into(), sval(row, 2));
    r.insert("threadId".into(), opt_str(row, 3));
    r.insert("turnId".into(), opt_str(row, 4));
    r.insert(
        "trigger".into(),
        json!({ "type": sval(row, 5) }),
    );
    r.insert("status".into(), sval(row, 6));
    r.insert("scheduledFor".into(), sval(row, 7));
    r.insert("deferredUntil".into(), opt_str(row, 8));
    r.insert("claimedBy".into(), opt_str(row, 9));
    r.insert("claimedAt".into(), opt_str(row, 10));
    r.insert("leaseExpiresAt".into(), opt_str(row, 11));
    r.insert("startedAt".into(), opt_str(row, 12));
    r.insert("finishedAt".into(), opt_str(row, 13));
    r.insert("threadCreateCommandId".into(), opt_str(row, 14));
    r.insert("turnStartCommandId".into(), opt_str(row, 15));
    r.insert("messageId".into(), opt_str(row, 16));
    r.insert("error".into(), opt_str(row, 17));
    let result = json_col(row, 18);
    if result.is_null() {
        r.insert("result".into(), Value::Null);
    } else {
        r.insert("result".into(), result);
    }
    r.insert("permissionSnapshot".into(), json_col(row, 19));
    r.insert("createdAt".into(), sval(row, 20));
    r.insert("updatedAt".into(), sval(row, 21));
    r
}

const RUN_COLS: &str = "run_id, automation_id, project_id, thread_id, turn_id, trigger_type, \
    status, scheduled_for, deferred_until, claimed_by, claimed_at, lease_expires_at, \
    started_at, finished_at, thread_create_command_id, turn_start_command_id, message_id, \
    error, result_json, permission_snapshot_json, created_at, updated_at";

fn list_definitions(
    conn: &Connection,
    project_id: Option<&str>,
    include_archived: bool,
) -> Result<Vec<Value>, WsErr> {
    let mut out = Vec::new();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {DEF_COLS} FROM automation_definitions \
             WHERE (?1 IS NULL OR project_id = ?1) \
               AND (?2 = 1 OR archived_at IS NULL) \
             ORDER BY updated_at DESC, automation_id ASC"
        ))
        .map_err(|e| err(e.to_string()))?;
    let rows = stmt
        .query_map(params![project_id, include_archived as i64], |r| {
            Ok(definition_from_row(r))
        })
        .map_err(|e| err(e.to_string()))?;
    for r in rows.flatten() {
        out.push(Value::Object(r));
    }
    Ok(out)
}

fn list_runs(
    conn: &Connection,
    project_id: Option<&str>,
    include_archived: bool,
) -> Result<Vec<Value>, WsErr> {
    let mut out = Vec::new();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT run_id, runs.automation_id, runs.project_id, thread_id, turn_id, trigger_type, \
             status, scheduled_for, deferred_until, claimed_by, claimed_at, lease_expires_at, \
             started_at, finished_at, thread_create_command_id, turn_start_command_id, message_id, \
             error, result_json, permission_snapshot_json, runs.created_at, runs.updated_at \
             FROM automation_runs runs \
             INNER JOIN automation_definitions definitions \
               ON definitions.automation_id = runs.automation_id \
             WHERE (?1 IS NULL OR runs.project_id = ?1) \
               AND (?2 = 1 OR definitions.archived_at IS NULL) \
             ORDER BY runs.scheduled_for DESC, runs.run_id DESC LIMIT 500"
        ))
        .map_err(|e| err(e.to_string()))?;
    let rows = stmt
        .query_map(params![project_id, include_archived as i64], |r| {
            Ok(run_from_row(r))
        })
        .map_err(|e| err(e.to_string()))?;
    for r in rows.flatten() {
        out.push(Value::Object(r));
    }
    Ok(out)
}

fn list_memories(conn: &Connection) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT automation_id, content, updated_at FROM automation_memory ORDER BY updated_at DESC",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok(json!({
                "automationId": r.get::<_, String>(0).unwrap_or_default(),
                "content": r.get::<_, String>(1).unwrap_or_default(),
                "updatedAt": r.get::<_, String>(2).unwrap_or_default(),
            }))
        }) {
            for v in rows.flatten() {
                out.push(v);
            }
        }
    }
    out
}

fn load_run(conn: &Connection, run_id: &str) -> Result<Value, WsErr> {
    conn.query_row(
        &format!("SELECT {RUN_COLS} FROM automation_runs WHERE run_id = ?1"),
        params![run_id],
        |r| Ok(Value::Object(run_from_row(r))),
    )
    .map_err(|_| err("Failed to update automation run."))
}

fn write_run_result(conn: &Connection, run_id: &str, result: Value, now: &str) -> Result<(), WsErr> {
    conn.execute(
        "UPDATE automation_runs SET result_json = ?2, updated_at = ?3 WHERE run_id = ?1",
        params![run_id, result.to_string(), now],
    )
    .map_err(|e| err(e.to_string()))?;
    Ok(())
}

fn result_defaults(run: &Value) -> Map<String, Value> {
    match run.get("result") {
        Some(Value::Object(m)) => m.clone(),
        _ => Map::from_iter([
            ("outcome".into(), json!("unknown")),
            ("summary".into(), Value::Null),
            ("unread".into(), json!(true)),
            ("archivedAt".into(), Value::Null),
        ]),
    }
}

fn list_result(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload
        .get("projectId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let include_archived = payload
        .get("includeArchived")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    with_state_copy("Failed to list automations.", |conn| {
        Ok(json!({
            "definitions": list_definitions(conn, project_id.as_deref(), include_archived)?,
            "runs": list_runs(conn, project_id.as_deref(), include_archived)?,
            "memories": list_memories(conn),
        }))
    })
}

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "automation.list"
            | "automation.getMemory"
            | "automation.markRunRead"
            | "automation.archiveRun"
            | "automation.cancelRun"
            | "automation.delete"
    )
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "automation.list" => list_result(payload),
        "automation.getMemory" => {
            let id = payload
                .get("automationId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| err("Failed to load automation memory."))?;
            with_state_copy("Failed to load automation memory.", |conn| {
                conn.query_row(
                    "SELECT automation_id, content, updated_at FROM automation_memory \
                     WHERE automation_id = ?1",
                    params![id],
                    |r| {
                        Ok(json!({
                            "automationId": r.get::<_, String>(0).unwrap_or_default(),
                            "content": r.get::<_, String>(1).unwrap_or_default(),
                            "updatedAt": r.get::<_, String>(2).unwrap_or_default(),
                        }))
                    },
                )
                .optional()
                .map_err(|_| err("Failed to load automation memory."))
                .map(|m| m.unwrap_or(Value::Null))
            })
        }
        "automation.markRunRead" | "automation.archiveRun" => {
            if !no_upstream() {
                return Err(err("automation writes require SYNARA_NO_UPSTREAM."));
            }
            let conn = open_state_rw().map_err(|_| err("Failed to update automation run."))?;
            let run_id = payload
                .get("runId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| err("Failed to update automation run."))?;
            let run = load_run(&conn, run_id)?;
            let now = iso_now();
            let mut result = result_defaults(&run);
            if tag == "automation.markRunRead" {
                let unread = payload
                    .get("unread")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                result.insert("unread".into(), json!(unread));
            } else {
                let archived = payload
                    .get("archived")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if archived {
                    result.insert("unread".into(), json!(false));
                }
                result.insert(
                    "archivedAt".into(),
                    if archived { json!(now) } else { Value::Null },
                );
            }
            write_run_result(&conn, run_id, Value::Object(result), &now)?;
            Ok(json!({ "run": load_run(&conn, run_id)? }))
        }
        "automation.cancelRun" => {
            if !no_upstream() {
                return Err(err("automation writes require SYNARA_NO_UPSTREAM."));
            }
            let conn = open_state_rw().map_err(|_| err("Failed to cancel automation run."))?;
            let run_id = payload
                .get("runId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| err("Failed to cancel automation run."))?;
            let now = iso_now();
            conn.execute(
                "UPDATE automation_runs \
                 SET status='cancelled', finished_at=?2, updated_at=?2, \
                     deferred_until=NULL, lease_expires_at=NULL, claimed_by=NULL \
                 WHERE run_id=?1 AND status IN ('pending','claimed','running','waiting-for-approval')",
                params![run_id, now],
            )
            .map_err(|_| err("Failed to cancel automation run."))?;
            let run = load_run(&conn, run_id)?;
            if run.get("status").and_then(|v| v.as_str()) == Some("cancelled") {
                write_run_result(
                    &conn,
                    run_id,
                    json!({
                        "outcome": "needs-attention",
                        "summary": "Automation run was cancelled.",
                        "severity": "warning",
                        "unread": true,
                        "archivedAt": null,
                    }),
                    &now,
                )?;
                return Ok(json!({ "run": load_run(&conn, run_id)? }));
            }
            Ok(json!({ "run": run }))
        }
        "automation.delete" => {
            if !no_upstream() {
                return Err(err("automation writes require SYNARA_NO_UPSTREAM."));
            }
            let conn = open_state_rw().map_err(|_| err("Failed to delete automation."))?;
            let id = payload
                .get("id")
                .or_else(|| payload.get("automationId"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| err("Failed to delete automation."))?;
            let now = iso_now();
            // Cancel any active runs first (best effort).
            let mut stmt = conn
                .prepare(
                    "SELECT run_id FROM automation_runs \
                     WHERE automation_id = ?1 AND status IN ('pending','claimed','running','waiting-for-approval')",
                )
                .map_err(|_| err("Failed to delete automation."))?;
            let active: Vec<String> = stmt
                .query_map(params![id], |r| r.get::<_, String>(0))
                .map_err(|_| err("Failed to delete automation."))?
                .flatten()
                .collect();
            for run_id in active {
                let _ = conn.execute(
                    "UPDATE automation_runs \
                     SET status='cancelled', finished_at=?2, updated_at=?2, \
                         deferred_until=NULL, lease_expires_at=NULL, claimed_by=NULL \
                     WHERE run_id=?1",
                    params![run_id, now],
                );
            }
            conn.execute(
                "UPDATE automation_definitions \
                 SET archived_at=?2, updated_at=?2, enabled=0 \
                 WHERE automation_id=?1",
                params![id, now],
            )
            .map_err(|_| err("Failed to delete automation."))?;
            Ok(json!({}))
        }
        _ => Err(err(format!("unsupported method {tag}"))),
    }
}

/// Snapshot payload for `automation.subscribe` — same shape as `automation.list`
/// plus the stream event wrapper.
pub fn snapshot_event(project_id: Option<&str>, include_archived: bool) -> Result<Value, WsErr> {
    with_state_copy("Failed to list automations.", |conn| {
        Ok(json!({
            "type": "snapshot",
            "definitions": list_definitions(conn, project_id, include_archived)?,
            "runs": list_runs(conn, project_id, include_archived)?,
            "memories": list_memories(conn),
        }))
    })
}
