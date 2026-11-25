//! Automation execution engine, lean slice of upstream AutomationService:
//! a 60s scheduler pass that fires due occurrences and a reconcile pass that
//! settles run rows from turn state. Crash-safety comes from the storage
//! primitives upstream designed: occurrence dedupe via INSERT OR IGNORE on
//! (automation_id, scheduled_for), and the schedule advance is transactional
//! with run creation, so a crash can never duplicate or silently skip an
//! occurrence. Gaps versus upstream, all deliberate: no AI completion
//! evaluation, no heartbeat coalescing, no retry/stop policies, no minimum
//! interval enforcement beyond the schedule itself.

use rusqlite::params;
use serde_json::{json, Value};

use crate::orchestration_engine::{open_state_rw, uuid_v4};
use crate::todo::{iso_now_ms, WsErr};

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

fn iso_now() -> String {
    iso_now_ms()
}

fn parse_iso_ms(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
}

pub fn start() {
    if std::env::var("SYNARA_NO_UPSTREAM").is_err() {
        return;
    }
    tokio::spawn(async {
        // First pass shortly after boot so due occurrences do not wait a full
        // interval; afterwards the upstream 60s cadence. The interval is a
        // test-only knob so the parity gate can exercise the full fire ->
        // reconcile cycle in seconds rather than minutes.
        let interval_ms: u64 = std::env::var("SYNARA_AUTOMATION_TICK_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60_000);
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms.min(5_000))).await;
        loop {
            if let Err(e) = tick() {
                eprintln!("automation scheduler tick failed: {}", e.0);
            }
            tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
        }
    });
}

fn tick() -> Result<(), WsErr> {
    due_pass()?;
    reconcile_pass()?;
    Ok(())
}

struct DueAutomation {
    id: String,
    project_id: String,
    name: String,
    prompt: String,
    schedule: Value,
    next_run_at: String,
    model_selection: Value,
    runtime_mode: String,
    interaction_mode: String,
    mode: String,
    target_thread_id: Option<String>,
}

/// Fire every due occurrence: transactional run-insert + schedule advance,
/// then dispatch the thread/turn for fresh runs only.
fn due_pass() -> Result<(), WsErr> {
    let now = iso_now();
    let now_ms = parse_iso_ms(&now);
    let due: Vec<DueAutomation> = {
        let conn = open_state_rw()?;
        let mut stmt = conn
            .prepare(
                "SELECT automation_id, project_id, name, prompt, schedule_json, next_run_at,
                        model_selection_json, runtime_mode, interaction_mode, mode, target_thread_id
                 FROM automation_definitions
                 WHERE enabled = 1 AND archived_at IS NULL AND next_run_at IS NOT NULL AND next_run_at <= ?1",
            )
            .map_err(|e| err(format!("automation due scan: {e}")))?;
        let rows = stmt
            .query_map(params![now], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?,
                    r.get::<_, Option<String>>(10)?,
                ))
            })
            .map_err(|e| err(format!("automation due scan: {e}")))?;
        rows.filter_map(|r| r.ok())
            .map(
                |(id, project_id, name, prompt, schedule_json, next_run_at, ms, rm, im, mode, tt)| {
                    DueAutomation {
                        id,
                        project_id,
                        name,
                        prompt,
                        schedule: serde_json::from_str(&schedule_json).unwrap_or(json!({})),
                        next_run_at,
                        model_selection: ms
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or(Value::Null),
                        runtime_mode: rm,
                        interaction_mode: im,
                        mode,
                        target_thread_id: tt,
                    }
                },
            )
            .collect()
    };

    for automation in due {
        if let Err(e) = fire_occurrence(&automation, &now, now_ms) {
            eprintln!(
                "automation occurrence failed for {}: {}",
                automation.id, e.0
            );
        }
    }
    Ok(())
}

fn fire_occurrence(automation: &DueAutomation, now: &str, now_ms: i64) -> Result<(), WsErr> {
    let scheduled_for = automation.next_run_at.clone();
    // Advance the schedule past NOW (coalescing downtime), never replaying
    // missed occurrences — upstream computeNextAutomationRunAtAfter.
    let next = crate::automation_schedule::compute_next_run_at_after(
        &automation.schedule,
        &scheduled_for,
        now,
        None,
    )
    .unwrap_or(None);

    let run_id = uuid_v4();
    let conn = open_state_rw()?;
    let tx = conn.unchecked_transaction().map_err(|e| err(format!("automation tx: {e}")))?;
    let inserted = tx
        .execute(
            "INSERT OR IGNORE INTO automation_runs
             (run_id, automation_id, project_id, thread_id, turn_id, trigger_type, status,
              scheduled_for, thread_create_command_id, turn_start_command_id, message_id,
              permission_snapshot_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, NULL, 'schedule', 'pending', ?4, ?5, ?6, ?7, '{}', ?8, ?8)",
            params![
                run_id,
                automation.id,
                automation.project_id,
                scheduled_for,
                format!("{run_id}-thread-create"),
                format!("{run_id}-turn-start"),
                format!("{run_id}-message"),
                now,
            ],
        )
        .map_err(|e| err(format!("automation run insert: {e}")))?;
    if inserted == 0 {
        // Occurrence already exists — a retry or a duplicate pass must not
        // double-fire. The schedule was advanced transactionally with the
        // original insert, so nothing to do.
        tx.rollback().ok();
        return Ok(());
    }
    tx.execute(
        "UPDATE automation_definitions SET next_run_at = ?2, updated_at = ?3 WHERE automation_id = ?1",
        params![automation.id, next, now],
    )
    .map_err(|e| err(format!("automation schedule advance: {e}")))?;
    tx.commit().map_err(|e| err(format!("automation tx commit: {e}")))?;
    drop(conn);

    // Dispatch outside the transaction: thread/turn creation is engine work.
    dispatch_run(&run_id, automation, &scheduled_for, now)?;
    let _ = now_ms;
    Ok(())
}

fn dispatch_run(
    run_id: &str,
    automation: &DueAutomation,
    scheduled_for: &str,
    now: &str,
) -> Result<(), WsErr> {
    // Dedicated and heartbeat automations continue their target thread;
    // standalone (and the first run of a dedicated one) create a fresh thread.
    let continuation = automation.target_thread_id.clone().filter(|_| automation.mode != "standalone");
    let thread_id = continuation
        .clone()
        .unwrap_or_else(|| format!("{run_id}-thread"));

    if continuation.is_none() {
        let create = json!({
            "type": "thread.create",
            "commandId": format!("{run_id}-thread-create"),
            "threadId": thread_id,
            "projectId": automation.project_id,
            "title": format!("{} — {}", automation.name, scheduled_for),
            "createdAt": now,
            "modelSelection": automation.model_selection,
            "runtimeMode": automation.runtime_mode,
            "interactionMode": automation.interaction_mode,
        });
        let obj = create.as_object().expect("object");
        crate::orchestration_engine::handle("orchestration.dispatchCommand", &Value::Object(obj.clone()))
            .map_err(|e| err(format!("automation thread create: {}", e.0)))?;
        // A dedicated automation adopts its first thread as the continuation target.
        if automation.mode != "standalone" {
            if let Ok(conn) = open_state_rw() {
                let _ = conn.execute(
                    "UPDATE automation_definitions SET target_thread_id = ?2, updated_at = ?3 WHERE automation_id = ?1 AND target_thread_id IS NULL",
                    params![automation.id, thread_id, now],
                );
            }
        }
    }

    let turn = json!({
        "type": "thread.turn.start",
        "commandId": format!("{run_id}-turn-start"),
        "threadId": thread_id,
        "createdAt": now,
        "dispatchMode": "queue",
        "dispatchOrigin": "automation",
        "runtimeMode": automation.runtime_mode,
        "interactionMode": automation.interaction_mode,
        "modelSelection": automation.model_selection,
        "message": {
            "messageId": format!("{run_id}-message"),
            "text": automation.prompt,
            "attachments": [],
        },
    });
    let obj = turn.as_object().expect("object");
    crate::orchestration_engine::handle("orchestration.dispatchCommand", &Value::Object(obj.clone()))
        .map_err(|e| err(format!("automation turn start: {}", e.0)))?;

    // The turn id is the active turn of the thread just started; the engine
    // derives it, so read it back rather than inventing it.
    let conn = open_state_rw()?;
    // The turn row does not exist yet: the engine consumes the queued message
    // asynchronously and only then inserts projection_turns. Reconcile adopts
    // the turn through pending_message_id = message_id once it appears — the
    // same derivation chain upstream uses (deriveAutomationRunIds).
    conn.execute(
        "UPDATE automation_runs SET thread_id = ?2, status = 'running', started_at = ?3, updated_at = ?3 WHERE run_id = ?1",
        params![run_id, thread_id, now],
    )
    .map_err(|e| err(format!("automation run dispatch mark: {e}")))?;
    Ok(())
}

/// Settle active runs from the state of their turn: completed/failed/
/// interrupted mirror the turn; a run whose turn evidence vanished is
/// interrupted exactly like upstream recovery.
fn reconcile_pass() -> Result<(), WsErr> {
    let now = iso_now();
    let conn = open_state_rw()?;
    // Adopt turns that have appeared since dispatch: the turn's
    // pending_message_id is the run's derived message id.
    let _ = conn.execute(
        "UPDATE automation_runs SET turn_id = (
            SELECT turn_id FROM projection_turns
            WHERE pending_message_id = automation_runs.message_id
            ORDER BY row_id DESC LIMIT 1
         ) WHERE status IN ('pending', 'running') AND turn_id IS NULL
           AND EXISTS (SELECT 1 FROM projection_turns WHERE pending_message_id = automation_runs.message_id)",
        [],
    );
    let active: Vec<(String, Option<String>)> = {
        let mut stmt = conn
            .prepare(
                "SELECT run_id, turn_id FROM automation_runs WHERE status IN ('pending', 'running') LIMIT 200",
            )
            .map_err(|e| err(format!("automation reconcile scan: {e}")))?;
        stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))
            .map_err(|e| err(format!("automation reconcile scan: {e}")))?
            .filter_map(|r| r.ok())
            .collect()
    };
    for (run_id, turn_id) in active {
        let Some(turn_id) = turn_id else {
            // Never dispatched a turn: either a crash between insert and
            // dispatch or a dispatch failure. Give the run one full scheduler
            // interval before interrupting, so a slow first pass does not kill
            // a healthy run.
            let age_ms: i64 = conn
                .query_row(
                    "SELECT CAST((julianday('now') - julianday(created_at)) * 86400000 AS INTEGER) FROM automation_runs WHERE run_id = ?1",
                    params![run_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if age_ms > 90_000 {
                let _ = conn.execute(
                    "UPDATE automation_runs SET status = 'interrupted', finished_at = ?2, updated_at = ?2,
                     error = 'Automation run was interrupted during recovery.' WHERE run_id = ?1",
                    params![run_id, now],
                );
            }
            continue;
        };
        let state: Option<String> = conn
            .query_row(
                "SELECT state FROM projection_turns WHERE turn_id = ?1 ORDER BY row_id DESC LIMIT 1",
                params![turn_id],
                |r| r.get(0),
            )
            .ok();
        let terminal = match state.as_deref() {
            Some("completed") => Some("completed"),
            Some("failed") | Some("error") => Some("failed"),
            Some("interrupted") => Some("interrupted"),
            _ => None,
        };
        if let Some(status) = terminal {
            let _ = conn.execute(
                "UPDATE automation_runs SET status = ?2, finished_at = ?3, updated_at = ?3 WHERE run_id = ?1",
                params![run_id, status, now],
            );
            settle_run(&conn, &run_id, status, &now);
        }
    }
    Ok(())
}

/// Layer 4/5 settlement after a run reaches a terminal state: iteration and
/// consecutive-failure accounting, deterministic stop policies (upstream
/// stopPolicy.ts semantics: failures disable at the configured streak,
/// max-iterations disables at the configured count), and the ai-evaluated
/// completion check via the official Claude CLI. Runs exactly once per run
/// because the row leaves the active set on transition.
fn settle_run(conn: &rusqlite::Connection, run_id: &str, status: &str, now: &str) {
    let def: Option<(String, i64, Option<i64>, Option<i64>, i64, String, String, String, String)> = conn
        .query_row(
            "SELECT a.automation_id, COALESCE(a.consecutive_failure_count,0),
                    a.stop_after_consecutive_failures, a.max_iterations,
                    COALESCE(a.iteration_count,0), a.completion_policy_json,
                    a.name, a.prompt, r.thread_id
             FROM automation_runs r
             JOIN automation_definitions a ON a.automation_id = r.automation_id
             WHERE r.run_id = ?1",
            params![run_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                ))
            },
        )
        .ok();
    let Some((automation_id, failures, stop_after, max_iterations, iterations, policy_json, name, prompt, thread_id)) = def else {
        return;
    };

    let new_iterations = iterations + 1;
    let new_failures = if status == "completed" { 0 } else { failures + 1 };
    let mut disable_reason: Option<&str> = None;
    if let Some(stop) = stop_after {
        if stop > 0 && new_failures >= stop {
            disable_reason = Some("failures");
        }
    }
    if let Some(max) = max_iterations {
        if max > 0 && new_iterations >= max {
            disable_reason = Some("max-iterations");
        }
    }
    let _ = conn.execute(
        "UPDATE automation_definitions SET iteration_count = ?2, consecutive_failure_count = ?3, updated_at = ?4 WHERE automation_id = ?1",
        params![automation_id, new_iterations, new_failures, now],
    );
    if let Some(reason) = disable_reason {
        let _ = conn.execute(
            "UPDATE automation_definitions SET enabled = 0, disabled_reason = ?2, disabled_at = ?3, updated_at = ?3 WHERE automation_id = ?1",
            params![automation_id, reason, now],
        );
        eprintln!("automation {} disabled: {}", automation_id, reason);
        return;
    }

    // Layer 5: ai-evaluated completion policy, only meaningful for completed runs.
    if status != "completed" {
        return;
    }
    let policy: Value = serde_json::from_str(&policy_json).unwrap_or(json!({"type":"none"}));
    if policy.get("type").and_then(Value::as_str) != Some("ai-evaluated") {
        return;
    }
    let Some(stop_when) = policy.get("stopWhen").and_then(Value::as_str) else {
        return;
    };
    let threshold = policy
        .get("confidenceThreshold")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    evaluate_completion(conn, run_id, &automation_id, &name, &prompt, stop_when, threshold, &thread_id, now);
}

fn limit_section(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= max {
        trimmed.to_string()
    } else {
        trimmed[..max].to_string()
    }
}

fn evaluate_completion(
    conn: &rusqlite::Connection,
    run_id: &str,
    automation_id: &str,
    name: &str,
    prompt: &str,
    stop_when: &str,
    threshold: f64,
    thread_id: &str,
    now: &str,
) {
    // Gather the run's user message and assistant output plus recent thread
    // context, mirroring upstream findRunCompletionMessages in spirit: the
    // evidence must come from the completed run output, not the saved prompt.
    let user_msg: String = conn
        .query_row(
            "SELECT text FROM projection_thread_messages WHERE thread_id = ?1 AND role = 'user' ORDER BY rowid DESC LIMIT 1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| prompt.to_string());
    let assistant_text: String = conn
        .query_row(
            "SELECT text FROM projection_thread_messages WHERE thread_id = ?1 AND role = 'assistant' ORDER BY rowid DESC LIMIT 1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "(no assistant output)".to_string());
    let context: String = conn
        .query_row(
            "SELECT group_concat(substr(text, 1, 500), char(10)) FROM (
                SELECT text FROM projection_thread_messages WHERE thread_id = ?1 ORDER BY rowid DESC LIMIT 6
             )",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "(none)".to_string());

    let eval_prompt = [
        "You evaluate whether a completed Synara heartbeat automation should stop.",
        "Return a JSON object with keys: stopMatched, confidence, reason.",
        "Respond with only the JSON object, no prose and no code fences.",
        "",
        "Decision rules:",
        "- stopMatched=true only if the completed run clearly satisfies the stop condition.",
        "- If the evidence is missing, indirect, ambiguous, or only says work continues, set stopMatched=false.",
        "- confidence must be a number from 0 to 1.",
        "- reason must be one concise sentence grounded in the run output.",
        "- Do not infer from the automation prompt alone; use the completed run output as evidence.",
        "",
        &format!("Automation: {name}"),
        "",
        "Saved automation prompt:",
        &limit_section(prompt, 4_000),
        "",
        "Stop condition:",
        &limit_section(stop_when, 2_000),
        "",
        "Run user message:",
        &limit_section(&user_msg, 4_000),
        "",
        "Run assistant output:",
        &limit_section(&assistant_text, 12_000),
        "",
        "Recent thread context:",
        &limit_section(&context, 6_000),
    ]
    .join("
");

    let schema = json!({
        "type": "object",
        "properties": {
            "stopMatched": {"type": "boolean"},
            "confidence": {"type": "number"},
            "reason": {"type": "string"},
        },
        "required": ["stopMatched", "confidence", "reason"],
    });

    let evaluation = crate::textgen::claude_json_generate(&eval_prompt, &schema, None);
    let (stop_matched, confidence, reason) = match evaluation {
        Ok(v) => (
            v.get("stopMatched").and_then(Value::as_bool).unwrap_or(false),
            v.get("confidence").and_then(Value::as_f64).unwrap_or(0.0),
            v.get("reason").and_then(Value::as_str).unwrap_or("No reason given.").chars().take(1_000).collect::<String>(),
        ),
        Err(e) => {
            let _ = conn.execute(
                "UPDATE automation_runs SET result_json = json_object('completionEvaluation', json_object('stopMatched', 0, 'confidence', 0.0, 'reason', ?2)), updated_at = ?3 WHERE run_id = ?1",
                params![run_id, format!("Stop check failed: {}", e.0), now],
            );
            return;
        }
    };

    let matched = stop_matched && confidence >= threshold;
    let summary = if matched {
        format!("Stopped: {reason}")
    } else {
        reason.clone()
    };
    let _ = conn.execute(
        "UPDATE automation_runs SET result_json = json_object(
            'outcome', CASE WHEN ?4 THEN 'no-findings' ELSE 'unknown' END,
            'summary', ?5,
            'completionEvaluation', json_object('stopMatched', ?4, 'confidence', ?6, 'reason', ?2)
         ), updated_at = ?3 WHERE run_id = ?1",
        params![run_id, reason, now, matched, summary, confidence],
    );
    if matched {
        let _ = conn.execute(
            "UPDATE automation_definitions SET enabled = 0, disabled_reason = 'completion', disabled_at = ?2, updated_at = ?2 WHERE automation_id = ?1",
            params![automation_id, now],
        );
        eprintln!("automation {} completed its stop condition: {}", automation_id, reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_compiles() {
        assert!(iso_now().len() > 10);
    }
}
