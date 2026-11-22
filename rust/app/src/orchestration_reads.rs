//! Read-only `orchestration.*` methods served from the same byte copy of
//! `state.sqlite` used by `orchestration_snapshot`: event replay, stored
//! checkpoint diffs, and provider-delivery blocker evidence.

use rusqlite::params;
use serde_json::{Value, json};

use crate::orchestration_snapshot::open_state_copy;
use crate::todo::WsErr;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("ORCHESTRATION"))
}

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "orchestration.replayEvents"
            | "orchestration.getTurnDiff"
            | "orchestration.getFullThreadDiff"
            | "orchestration.listProviderDeliveryBlockers"
            | "orchestration.unsubscribeShell"
            | "orchestration.unsubscribeThread"
    )
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "orchestration.replayEvents" => replay_events(payload),
        "orchestration.getTurnDiff" => turn_diff(payload, false),
        "orchestration.getFullThreadDiff" => turn_diff(payload, true),
        "orchestration.listProviderDeliveryBlockers" => delivery_blockers(payload),
        // Streams are ended by the client interrupting the subscribe request;
        // the unsubscribe calls only release bookkeeping upstream.
        "orchestration.unsubscribeShell" | "orchestration.unsubscribeThread" => Ok(Value::Null),
        _ => Err(err("unhandled orchestration read tag")),
    }
}

/// Thread-stream events after `after_seq`: (sequence, wire event) pairs,
/// for `subscribeThread` polling. New state copy per call.
pub(crate) fn thread_events_after(
    thread_id: &str,
    after_seq: i64,
) -> Result<Vec<(i64, Value)>, WsErr> {
    let Some(copy) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    events_after(&copy.conn, Some(thread_id), after_seq)
}

/// All-domain events after `after_seq`, for `subscribeDomainEvents` polling.
pub(crate) fn domain_events_after(after_seq: i64) -> Result<Vec<(i64, Value)>, WsErr> {
    let Some(copy) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    events_after(&copy.conn, None, after_seq)
}

fn events_after(
    conn: &rusqlite::Connection,
    thread_id: Option<&str>,
    after_seq: i64,
) -> Result<Vec<(i64, Value)>, WsErr> {
    let cols = "sequence, event_id, aggregate_kind, stream_id, event_type, \
                occurred_at, command_id, causation_event_id, correlation_id, \
                payload_json, metadata_json";
    let (sql, args): (String, Vec<String>) = if let Some(tid) = thread_id {
        (
            format!(
                "SELECT {cols} FROM orchestration_events \
                 WHERE sequence > ?1 AND aggregate_kind = 'thread' AND stream_id = ?2 \
                 ORDER BY sequence"
            ),
            vec![after_seq.to_string(), tid.to_string()],
        )
    } else {
        (
            format!(
                "SELECT {cols} FROM orchestration_events WHERE sequence > ?1 ORDER BY sequence"
            ),
            vec![after_seq.to_string()],
        )
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| err(e.to_string()))?;
    let mut rows = stmt
        .query(rusqlite::params_from_iter(args.iter()))
        .map_err(|e| err(e.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
        let seq = row.get::<_, i64>(0).unwrap_or(0);
        out.push((
            seq,
            event_row(
                seq,
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
                row.get::<_, String>(5).unwrap_or_default(),
                row.get::<_, Option<String>>(6).unwrap_or(None),
                row.get::<_, Option<String>>(7).unwrap_or(None),
                row.get::<_, Option<String>>(8).unwrap_or(None),
                row.get::<_, String>(9).unwrap_or_default(),
                row.get::<_, String>(10).unwrap_or_default(),
            ),
        ));
    }
    Ok(out)
}

fn str_opt<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(|x| x.as_str())
}

fn i64_arg(v: &Value, k: &str) -> Result<i64, WsErr> {
    v.get(k)
        .and_then(|x| x.as_i64())
        .ok_or_else(|| err(format!("{k} is required")))
}

/// One `orchestration_events` row as an OrchestrationEvent wire object.
fn event_row(
    sequence: i64,
    event_id: String,
    aggregate_kind: String,
    stream_id: String,
    event_type: String,
    occurred_at: String,
    command_id: Option<String>,
    causation_event_id: Option<String>,
    correlation_id: Option<String>,
    payload_json: String,
    metadata_json: String,
) -> Value {
    json!({
        "sequence": sequence,
        "eventId": event_id,
        "aggregateKind": aggregate_kind,
        "aggregateId": stream_id,
        "occurredAt": occurred_at,
        "commandId": command_id,
        "causationEventId": causation_event_id,
        "correlationId": correlation_id,
        "metadata": serde_json::from_str::<Value>(&metadata_json).unwrap_or(json!({})),
        "type": event_type,
        "payload": serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null),
    })
}

/// `orchestration.replayEvents`: events after a cursor, optionally scoped to
/// one thread's stream.
fn replay_events(payload: &Value) -> Result<Value, WsErr> {
    let after = i64_arg(payload, "fromSequenceExclusive")?;
    let thread_id = str_opt(payload, "threadId");
    let Some(copy) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &copy.conn;
    Ok(Value::Array(
        events_after(conn, thread_id, after)?
            .into_iter()
            .map(|(_, e)| e)
            .collect(),
    ))
}

/// `orchestration.getTurnDiff` / `getFullThreadDiff`: diffs are stored
/// precomputed in `checkpoint_diff_blobs` keyed by (thread, from, to) —
/// a full-thread diff is the range 0..toTurnCount.
fn turn_diff(payload: &Value, full: bool) -> Result<Value, WsErr> {
    let thread_id = str_opt(payload, "threadId").unwrap_or_default().to_string();
    if thread_id.is_empty() {
        return Err(err("threadId is required"));
    }
    let to_turn = i64_arg(payload, "toTurnCount")?;
    let from_turn = if full {
        0
    } else {
        i64_arg(payload, "fromTurnCount")?
    };
    if from_turn > to_turn {
        return Err(err("fromTurnCount must be less than or equal to toTurnCount"));
    }
    let ignore_ws = payload
        .get("ignoreWhitespace")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if ignore_ws {
        return Err(err(
            "ignoreWhitespace requires a live checkpoint recompute, unavailable natively",
        ));
    }
    let Some(copy) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let diff: Option<String> = copy
        .conn
        .query_row(
            "SELECT diff FROM checkpoint_diff_blobs \
             WHERE thread_id = ?1 AND from_turn_count = ?2 AND to_turn_count = ?3",
            params![thread_id, from_turn, to_turn],
            |r| r.get(0),
        )
        .ok();
    let Some(diff) = diff else {
        return Err(err(format!(
            "no checkpoint diff recorded for turns {from_turn}..{to_turn}"
        )));
    };
    Ok(json!({
        "threadId": thread_id,
        "fromTurnCount": from_turn,
        "toTurnCount": to_turn,
        "diff": diff,
    }))
}

/// `orchestration.listProviderDeliveryBlockers`: deliveries stuck dead or
/// uncertain, joined with their event identity and latest reconciliation.
fn delivery_blockers(payload: &Value) -> Result<Value, WsErr> {
    let thread_id = str_opt(payload, "threadId");
    let limit = payload
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .clamp(1, 500);
    let Some(copy) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &copy.conn;
    let mut sql = String::from(
        "SELECT d.consumer_name, d.event_sequence, d.thread_id, d.state, \
                d.attempt_count, d.last_error, d.updated_at, \
                e.event_id, e.event_type, e.occurred_at, \
                r.outcome, r.reconciled_at, r.reconciled_by, r.note \
         FROM orchestration_event_deliveries d \
         JOIN orchestration_events e ON e.sequence = d.event_sequence \
         LEFT JOIN provider_delivery_reconciliations r \
           ON r.consumer_name = d.consumer_name AND r.event_sequence = d.event_sequence \
          AND r.reconciled_at = ( \
            SELECT MAX(reconciled_at) FROM provider_delivery_reconciliations \
            WHERE consumer_name = d.consumer_name AND event_sequence = d.event_sequence \
          ) \
         WHERE d.state IN ('dead','uncertain')",
    );
    let mut args: Vec<String> = Vec::new();
    if let Some(tid) = thread_id {
        args.push(tid.to_string());
        sql.push_str(&format!(" AND d.thread_id = ?{}", args.len()));
    }
    args.push(limit.to_string());
    sql.push_str(&format!(" ORDER BY d.event_sequence LIMIT ?{}", args.len()));
    let mut stmt = conn.prepare(&sql).map_err(|e| err(e.to_string()))?;
    let mut rows = stmt
        .query(rusqlite::params_from_iter(args.iter()))
        .map_err(|e| err(e.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
        out.push(json!({
            "consumerName": row.get::<_, String>(0).unwrap_or_default(),
            "eventSequence": row.get::<_, i64>(1).unwrap_or(0),
            "eventId": row.get::<_, String>(7).unwrap_or_default(),
            "eventType": row.get::<_, String>(8).unwrap_or_default(),
            "occurredAt": row.get::<_, String>(9).unwrap_or_default(),
            "threadId": row.get::<_, String>(2).unwrap_or_default(),
            "state": row.get::<_, String>(3).unwrap_or_default(),
            "attemptCount": row.get::<_, i64>(4).unwrap_or(0),
            "lastError": row.get::<_, Option<String>>(5).unwrap_or(None),
            "updatedAt": row.get::<_, String>(6).unwrap_or_default(),
            "lastReconciliationOutcome": row.get::<_, Option<String>>(10).unwrap_or(None),
            "lastReconciledAt": row.get::<_, Option<String>>(11).unwrap_or(None),
            "lastReconciledBy": row.get::<_, Option<String>>(12).unwrap_or(None),
            "lastReconciliationNote": row.get::<_, Option<String>>(13).unwrap_or(None),
        }));
    }
    Ok(Value::Array(out))
}
