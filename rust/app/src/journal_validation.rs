//! Startup validation for the durable orchestration journal.
//!
//! Upstream decodes every persisted event through the contracts schema before
//! projection replay. The native port historically trusted the JSON shape and
//! silently projected malformed legacy rows. This module is the first strict
//! boundary: it validates the event families whose malformed payloads corrupt
//! the thread/project read model, then refuses startup with the exact sequence
//! instead of serving a divergent projection as healthy.

use rusqlite::Connection;
use serde_json::Value;

const RUNTIME_MODES: &[&str] = &["approval-required", "auto", "full-access"];
const INTERACTION_MODES: &[&str] = &["default", "plan", "debug"];
const SESSION_STATUSES: &[&str] = &[
    "idle",
    "starting",
    "running",
    "ready",
    "interrupted",
    "stopped",
    "error",
];
const ENV_MODES: &[&str] = &["local", "worktree"];

pub fn validate_state_journal() -> Result<(), String> {
    let db = std::path::PathBuf::from(crate::todo::state_dir()).join("state.sqlite");
    let conn = Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("could not open state.sqlite for journal validation: {error}"))?;
    validate_journal(&conn)
}

pub(crate) fn validate_journal(conn: &Connection) -> Result<(), String> {
    let mut statement = conn
        .prepare(
            "SELECT sequence, event_type, payload_json, metadata_json, event_id, aggregate_kind, stream_id, occurred_at FROM orchestration_events ORDER BY sequence",
        )
        .map_err(|error| format!("could not read orchestration journal: {error}"))?;
    let mut rows = statement
        .query([])
        .map_err(|error| format!("could not scan orchestration journal: {error}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|error| format!("could not scan orchestration journal: {error}"))?
    {
        let sequence: i64 = row
            .get(0)
            .map_err(|error| format!("could not read journal sequence: {error}"))?;
        let event_type: String = row.get(1).map_err(|error| {
            format!("journal sequence {sequence}: unreadable event type: {error}")
        })?;
        let payload_json: String = row.get(2).map_err(|error| {
            format!("journal sequence {sequence} ({event_type}): unreadable payload_json: {error}")
        })?;
        let metadata_json: String = row.get(3).map_err(|error| {
            format!("journal sequence {sequence} ({event_type}): unreadable metadata_json: {error}")
        })?;
        let payload: Value = serde_json::from_str(&payload_json).map_err(|error| {
            format!("journal sequence {sequence} ({event_type}): payload_json is not valid JSON: {error}")
        })?;
        let metadata: Value = serde_json::from_str(&metadata_json).map_err(|error| {
            format!("journal sequence {sequence} ({event_type}): metadata_json is not valid JSON: {error}")
        })?;
        let event_id: String = row.get(4).map_err(|error| {
            format!("journal sequence {sequence} ({event_type}): unreadable event_id: {error}")
        })?;
        let aggregate_kind: String = row.get(5).map_err(|error| {
            format!(
                "journal sequence {sequence} ({event_type}): unreadable aggregate_kind: {error}"
            )
        })?;
        let stream_id: String = row.get(6).map_err(|error| {
            format!("journal sequence {sequence} ({event_type}): unreadable stream_id: {error}")
        })?;
        let occurred_at: String = row.get(7).map_err(|error| {
            format!("journal sequence {sequence} ({event_type}): unreadable occurred_at: {error}")
        })?;
        validate_event_metadata(sequence, &event_type, &metadata)?;
        validate_event_envelope(
            sequence,
            &event_type,
            &event_id,
            &aggregate_kind,
            &stream_id,
            &occurred_at,
        )?;
        let payload = crate::orchestration_engine::payload_after_persisted_upcast(
            &event_type,
            &payload,
            &metadata_json,
        );
        validate_event_payload(sequence, &event_type, &payload)?;
    }
    Ok(())
}

fn kind(value: Option<&Value>) -> &'static str {
    match value {
        None => "missing",
        Some(Value::Null) => "null",
        Some(Value::Bool(_)) => "boolean",
        Some(Value::Number(_)) => "number",
        Some(Value::String(_)) => "string",
        Some(Value::Array(_)) => "array",
        Some(Value::Object(_)) => "object",
    }
}

fn invalid(
    sequence: i64,
    event_type: &str,
    path: &str,
    expected: &str,
    value: Option<&Value>,
) -> String {
    format!(
        "journal sequence {sequence} ({event_type}): {path} expected {expected}, got {}",
        kind(value)
    )
}

fn require_object<'a>(
    sequence: i64,
    event_type: &str,
    path: &str,
    value: Option<&'a Value>,
) -> Result<&'a Value, String> {
    match value {
        Some(value) if value.is_object() => Ok(value),
        value => Err(invalid(sequence, event_type, path, "object", value)),
    }
}

fn require_string_at(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
    path: &str,
) -> Result<(), String> {
    let value = object.get(key);
    if value
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
    {
        Ok(())
    } else {
        Err(invalid(
            sequence,
            event_type,
            path,
            "non-empty string",
            value,
        ))
    }
}

fn require_string(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
) -> Result<(), String> {
    require_string_at(sequence, event_type, object, key, key)
}

fn require_nullable_string_at(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
    path: &str,
) -> Result<(), String> {
    let value = object.get(key);
    if value.is_some_and(|v| v.is_null() || v.as_str().is_some_and(|s| !s.trim().is_empty())) {
        Ok(())
    } else {
        Err(invalid(
            sequence,
            event_type,
            path,
            "non-empty string or null",
            value,
        ))
    }
}

fn require_nullable_string(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
) -> Result<(), String> {
    require_nullable_string_at(sequence, event_type, object, key, key)
}

fn optional_string(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
) -> Result<(), String> {
    if object.get(key).is_none() {
        return Ok(());
    }
    require_string(sequence, event_type, object, key)
}

fn optional_nullable_string(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
) -> Result<(), String> {
    if object.get(key).is_none() {
        return Ok(());
    }
    require_nullable_string(sequence, event_type, object, key)
}

fn optional_array(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
) -> Result<(), String> {
    if object.get(key).is_none() {
        return Ok(());
    }
    require_array(sequence, event_type, object, key)
}

fn optional_bool(sequence: i64, event_type: &str, object: &Value, key: &str) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    if value.is_boolean() {
        Ok(())
    } else {
        Err(invalid(sequence, event_type, key, "boolean", Some(value)))
    }
}

fn optional_enum_at(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
    path: &str,
    allowed: &[&str],
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    if value.as_str().is_some_and(|v| allowed.contains(&v)) {
        Ok(())
    } else {
        Err(invalid(
            sequence,
            event_type,
            path,
            &format!("one of {}", allowed.join(", ")),
            Some(value),
        ))
    }
}

fn optional_enum(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
    allowed: &[&str],
) -> Result<(), String> {
    optional_enum_at(sequence, event_type, object, key, key, allowed)
}

fn require_enum_at(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
    path: &str,
    allowed: &[&str],
) -> Result<(), String> {
    let value = object.get(key);
    if value
        .and_then(Value::as_str)
        .is_some_and(|v| allowed.contains(&v))
    {
        Ok(())
    } else {
        Err(invalid(
            sequence,
            event_type,
            path,
            &format!("one of {}", allowed.join(", ")),
            value,
        ))
    }
}

fn require_array(sequence: i64, event_type: &str, object: &Value, key: &str) -> Result<(), String> {
    let value = object.get(key);
    if value.is_some_and(Value::is_array) {
        Ok(())
    } else {
        Err(invalid(sequence, event_type, key, "array", value))
    }
}

fn require_model_selection(
    sequence: i64,
    event_type: &str,
    object: &Value,
    key: &str,
    nullable: bool,
) -> Result<(), String> {
    let value = object.get(key);
    // This validator runs after the v0 -> v1 payload upcast. Missing keys are
    // therefore already a schema failure: legacy rows that could be repaired
    // have gained either a normalized object or null by this point.
    if nullable && value.is_some_and(Value::is_null) {
        return Ok(());
    }
    require_object(sequence, event_type, key, value).map(|_| ())
}

fn validate_thread_created(sequence: i64, event_type: &str, payload: &Value) -> Result<(), String> {
    require_string(sequence, event_type, payload, "threadId")?;
    require_string(sequence, event_type, payload, "projectId")?;
    require_string(sequence, event_type, payload, "title")?;
    require_model_selection(sequence, event_type, payload, "modelSelection", false)?;
    require_nullable_string(sequence, event_type, payload, "branch")?;
    require_nullable_string(sequence, event_type, payload, "worktreePath")?;
    optional_enum(sequence, event_type, payload, "runtimeMode", RUNTIME_MODES)?;
    optional_enum(
        sequence,
        event_type,
        payload,
        "interactionMode",
        INTERACTION_MODES,
    )?;
    optional_enum(sequence, event_type, payload, "envMode", ENV_MODES)
}

fn validate_turn_start(sequence: i64, event_type: &str, payload: &Value) -> Result<(), String> {
    require_string(sequence, event_type, payload, "threadId")?;
    require_string(sequence, event_type, payload, "messageId")?;
    require_string(sequence, event_type, payload, "createdAt")?;
    if let Some(value) = payload.get("modelSelection") {
        require_object(sequence, event_type, "modelSelection", Some(value))?;
    }
    optional_enum(sequence, event_type, payload, "runtimeMode", RUNTIME_MODES)?;
    optional_enum(
        sequence,
        event_type,
        payload,
        "interactionMode",
        INTERACTION_MODES,
    )?;
    optional_enum(
        sequence,
        event_type,
        payload,
        "dispatchMode",
        &["queue", "steer"],
    )
}

fn validate_session_set(sequence: i64, event_type: &str, payload: &Value) -> Result<(), String> {
    require_string(sequence, event_type, payload, "threadId")?;
    let session = require_object(sequence, event_type, "session", payload.get("session"))?;
    require_string_at(
        sequence,
        event_type,
        session,
        "threadId",
        "session.threadId",
    )?;
    require_enum_at(
        sequence,
        event_type,
        session,
        "status",
        "session.status",
        SESSION_STATUSES,
    )?;
    require_nullable_string_at(
        sequence,
        event_type,
        session,
        "providerName",
        "session.providerName",
    )?;
    require_nullable_string_at(
        sequence,
        event_type,
        session,
        "activeTurnId",
        "session.activeTurnId",
    )?;
    require_nullable_string_at(
        sequence,
        event_type,
        session,
        "lastError",
        "session.lastError",
    )?;
    require_string_at(
        sequence,
        event_type,
        session,
        "updatedAt",
        "session.updatedAt",
    )?;
    optional_enum_at(
        sequence,
        event_type,
        session,
        "runtimeMode",
        "session.runtimeMode",
        RUNTIME_MODES,
    )
}

fn validate_project_created(
    sequence: i64,
    event_type: &str,
    payload: &Value,
) -> Result<(), String> {
    require_string(sequence, event_type, payload, "projectId")?;
    require_string(sequence, event_type, payload, "title")?;
    require_string(sequence, event_type, payload, "workspaceRoot")?;
    require_model_selection(sequence, event_type, payload, "defaultModelSelection", true)?;
    require_array(sequence, event_type, payload, "scripts")?;
    require_string(sequence, event_type, payload, "createdAt")?;
    require_string(sequence, event_type, payload, "updatedAt")
}

fn validate_thread_meta_updated(
    sequence: i64,
    event_type: &str,
    payload: &Value,
) -> Result<(), String> {
    require_string(sequence, event_type, payload, "threadId")?;
    optional_string(sequence, event_type, payload, "title")?;
    if let Some(value) = payload.get("modelSelection") {
        require_object(sequence, event_type, "modelSelection", Some(value))?;
    }
    optional_enum(sequence, event_type, payload, "envMode", ENV_MODES)?;
    for key in [
        "branch",
        "worktreePath",
        "workingDirectory",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
        "settledAt",
        "snoozedUntil",
        "snoozeReminderAt",
        "parentThreadId",
        "subagentAgentId",
        "subagentNickname",
        "subagentRole",
    ] {
        optional_nullable_string(sequence, event_type, payload, key)?;
    }
    optional_bool(sequence, event_type, payload, "createBranchFlowCompleted")?;
    optional_bool(sequence, event_type, payload, "isPinned")?;
    require_string(sequence, event_type, payload, "updatedAt")
}

fn validate_project_meta_updated(
    sequence: i64,
    event_type: &str,
    payload: &Value,
) -> Result<(), String> {
    require_string(sequence, event_type, payload, "projectId")?;
    optional_enum(
        sequence,
        event_type,
        payload,
        "kind",
        &["project", "chat", "studio", "group"],
    )?;
    optional_string(sequence, event_type, payload, "title")?;
    optional_string(sequence, event_type, payload, "workspaceRoot")?;
    if let Some(value) = payload.get("defaultModelSelection") {
        if !value.is_null() {
            require_object(sequence, event_type, "defaultModelSelection", Some(value))?;
        }
    }
    optional_array(sequence, event_type, payload, "scripts")?;
    optional_array(sequence, event_type, payload, "additionalFolders")?;
    optional_bool(sequence, event_type, payload, "isPinned")?;
    optional_nullable_string(sequence, event_type, payload, "spaceId")?;
    require_string(sequence, event_type, payload, "updatedAt")
}

fn validate_event_envelope(
    sequence: i64,
    event_type: &str,
    event_id: &str,
    aggregate_kind: &str,
    stream_id: &str,
    occurred_at: &str,
) -> Result<(), String> {
    for (path, value) in [
        ("eventId", event_id),
        ("aggregateId", stream_id),
        ("occurredAt", occurred_at),
    ] {
        if value.trim().is_empty() {
            return Err(format!(
                "journal sequence {sequence} ({event_type}): {path} expected non-empty string, got string"
            ));
        }
    }
    if !matches!(aggregate_kind, "space" | "project" | "thread") {
        return Err(format!(
            "journal sequence {sequence} ({event_type}): aggregateKind expected one of space, project, thread, got string"
        ));
    }
    Ok(())
}

fn validate_event_metadata(
    sequence: i64,
    event_type: &str,
    metadata: &Value,
) -> Result<(), String> {
    let object = require_object(sequence, event_type, "metadata", Some(metadata))?;
    let Some(version_value) =
        object.get(crate::orchestration_engine::PERSISTED_EVENT_SCHEMA_VERSION_KEY)
    else {
        // Missing means the legacy v0 row format; the upcaster handles those rows.
        return Ok(());
    };
    let version = version_value.as_i64().or_else(|| {
        version_value
            .as_u64()
            .and_then(|v| i64::try_from(v).ok())
            .or_else(|| {
                version_value.as_f64().and_then(|v| {
                    (v.fract() == 0.0 && v.abs() <= 9_007_199_254_740_992.0).then(|| v as i64)
                })
            })
    });
    let Some(version) = version else {
        return Err(format!(
            "journal sequence {sequence} ({event_type}): invalid persisted event schema version; expected a non-negative safe integer"
        ));
    };
    if version < 0 {
        return Err(format!(
            "journal sequence {sequence} ({event_type}): invalid persisted event schema version {version}; expected a non-negative safe integer"
        ));
    }
    if version > crate::orchestration_engine::CURRENT_PERSISTED_EVENT_SCHEMA_VERSION {
        return Err(format!(
            "journal sequence {sequence} ({event_type}): unsupported persisted event schema version {version}; this build supports through {}",
            crate::orchestration_engine::CURRENT_PERSISTED_EVENT_SCHEMA_VERSION
        ));
    }
    Ok(())
}

fn validate_event_payload(sequence: i64, event_type: &str, payload: &Value) -> Result<(), String> {
    require_object(sequence, event_type, "payload", Some(payload))?;
    match event_type {
        "thread.created" => validate_thread_created(sequence, event_type, payload),
        "thread.turn-start-requested" | "thread.turn-queued" => {
            validate_turn_start(sequence, event_type, payload)
        }
        "thread.session-set" => validate_session_set(sequence, event_type, payload),
        "project.created" => validate_project_created(sequence, event_type, payload),
        "thread.meta-updated" => validate_thread_meta_updated(sequence, event_type, payload),
        "project.meta-updated" => validate_project_meta_updated(sequence, event_type, payload),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_event(conn: &Connection, sequence: i64, event_type: &str, payload: &str) {
        conn.execute(
            "INSERT INTO orchestration_events (sequence, event_type, payload_json, metadata_json, event_id, aggregate_kind, stream_id, occurred_at) VALUES (?1, ?2, ?3, '{}', 'event-test', 'thread', 'stream-test', '2026-01-01T00:00:00.000Z')",
            rusqlite::params![sequence, event_type, payload],
        )
        .unwrap();
    }

    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE orchestration_events (sequence INTEGER PRIMARY KEY, event_type TEXT, payload_json TEXT, metadata_json TEXT, event_id TEXT, aggregate_kind TEXT, stream_id TEXT, occurred_at TEXT);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn accepts_contract_valid_core_events() {
        let conn = fixture();
        insert_event(
            &conn,
            1,
            "project.created",
            r#"{"projectId":"p1","title":"Project","workspaceRoot":"/tmp/project","defaultModelSelection":null,"scripts":[],"createdAt":"2026-01-01T00:00:00.000Z","updatedAt":"2026-01-01T00:00:00.000Z"}"#,
        );
        insert_event(
            &conn,
            2,
            "thread.created",
            r#"{"threadId":"t1","projectId":"p1","title":"Thread","modelSelection":{"provider":"codex","model":"gpt"},"runtimeMode":"full-access","interactionMode":"default","branch":null,"worktreePath":null}"#,
        );
        insert_event(
            &conn,
            3,
            "thread.turn-start-requested",
            r#"{"threadId":"t1","messageId":"m1","runtimeMode":"full-access","interactionMode":"default","dispatchMode":"queue","createdAt":"2026-01-01T00:00:00.000Z"}"#,
        );
        insert_event(
            &conn,
            4,
            "thread.session-set",
            r#"{"threadId":"t1","session":{"threadId":"t1","status":"ready","providerName":null,"runtimeMode":"full-access","activeTurnId":null,"lastError":null,"updatedAt":"2026-01-01T00:00:00.000Z"}}"#,
        );
        assert!(validate_journal(&conn).is_ok());
    }

    #[test]
    fn rejects_explicit_null_model_selection_with_sequence() {
        let conn = fixture();
        insert_event(
            &conn,
            7,
            "thread.created",
            r#"{"threadId":"t1","projectId":"p1","title":"Thread","modelSelection":null,"branch":null,"worktreePath":null}"#,
        );
        let error = validate_journal(&conn).unwrap_err();
        assert!(error.contains("sequence 7"));
        assert!(error.contains("modelSelection"));
        assert!(error.contains("got null"));
    }

    #[test]
    fn rejects_null_runtime_mode_even_when_defaultable_if_absent() {
        let conn = fixture();
        insert_event(
            &conn,
            3,
            "thread.turn-start-requested",
            r#"{"threadId":"t1","messageId":"m1","runtimeMode":null,"createdAt":"2026-01-01T00:00:00.000Z"}"#,
        );
        let error = validate_journal(&conn).unwrap_err();
        assert!(error.contains("runtimeMode"));
    }

    #[test]
    fn rejects_incomplete_session_payload() {
        let conn = fixture();
        insert_event(
            &conn,
            9,
            "thread.session-set",
            r#"{"threadId":"t1","session":{"status":"ready"}}"#,
        );
        let error = validate_journal(&conn).unwrap_err();
        assert!(error.contains("session.threadId"));
    }

    #[test]
    fn rejects_missing_session_status() {
        let conn = fixture();
        insert_event(
            &conn,
            10,
            "thread.session-set",
            r#"{"threadId":"t1","session":{"threadId":"t1","providerName":null,"activeTurnId":null,"lastError":null,"updatedAt":"2026-01-01T00:00:00.000Z"}}"#,
        );
        let error = validate_journal(&conn).unwrap_err();
        assert!(error.contains("session.status"));
    }

    #[test]
    fn upcasts_legacy_missing_thread_model_selection_before_validation() {
        let conn = fixture();
        insert_event(
            &conn,
            13,
            "thread.created",
            r#"{"threadId":"t1","projectId":"p1","title":"Thread","branch":null,"worktreePath":null}"#,
        );
        assert!(validate_journal(&conn).is_ok());
    }

    #[test]
    fn rejects_missing_model_selection_on_v1_rows() {
        let conn = fixture();
        conn.execute(
            "INSERT INTO orchestration_events (sequence, event_type, payload_json, metadata_json, event_id, aggregate_kind, stream_id, occurred_at) VALUES (14, 'thread.created', '{\"threadId\":\"t1\",\"projectId\":\"p1\",\"title\":\"Thread\",\"branch\":null,\"worktreePath\":null}', '{\"persistedEventSchemaVersion\":1}', 'event-test', 'thread', 'stream-test', '2026-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        let error = validate_journal(&conn).unwrap_err();
        assert!(error.contains("modelSelection"));
    }

    #[test]
    fn rejects_unsupported_persisted_event_schema_version() {
        let conn = fixture();
        conn.execute(
            "INSERT INTO orchestration_events (sequence, event_type, payload_json, metadata_json, event_id, aggregate_kind, stream_id, occurred_at) VALUES (11, 'thread.created', '{}', '{\"persistedEventSchemaVersion\":2}', 'event-test', 'thread', 'stream-test', '2026-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        let error = validate_journal(&conn).unwrap_err();
        assert!(error.contains("sequence 11"));
        assert!(error.contains("unsupported persisted event schema version 2"));
    }

    #[test]
    fn accepts_current_persisted_event_schema_version() {
        let conn = fixture();
        conn.execute(
            "INSERT INTO orchestration_events (sequence, event_type, payload_json, metadata_json, event_id, aggregate_kind, stream_id, occurred_at) VALUES (12, 'thread.created', '{}', '{\"persistedEventSchemaVersion\":1}', 'event-test', 'thread', 'stream-test', '2026-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        // The unknown-shape payload still fails payload validation after the
        // version gate, proving the current version itself was accepted.
        let error = validate_journal(&conn).unwrap_err();
        assert!(error.contains("threadId"));
        assert!(!error.contains("schema version"));
    }

    #[test]
    fn accepts_valid_meta_updated_events() {
        let conn = fixture();
        insert_event(
            &conn,
            20,
            "thread.meta-updated",
            r#"{"threadId":"t1","title":"Thread","modelSelection":{"provider":"codex","model":"gpt"},"branch":null,"isPinned":true,"updatedAt":"2026-01-01T00:00:00.000Z"}"#,
        );
        insert_event(
            &conn,
            21,
            "project.meta-updated",
            r#"{"projectId":"p1","kind":"project","defaultModelSelection":null,"scripts":[],"additionalFolders":[],"isPinned":false,"spaceId":null,"updatedAt":"2026-01-01T00:00:00.000Z"}"#,
        );
        assert!(validate_journal(&conn).is_ok());
    }

    #[test]
    fn rejects_invalid_meta_updated_shapes() {
        let conn = fixture();
        insert_event(
            &conn,
            22,
            "thread.meta-updated",
            r#"{"threadId":"t1","modelSelection":null,"updatedAt":"2026-01-01T00:00:00.000Z"}"#,
        );
        assert!(
            validate_journal(&conn)
                .unwrap_err()
                .contains("modelSelection")
        );

        let conn = fixture();
        insert_event(
            &conn,
            23,
            "project.meta-updated",
            r#"{"projectId":"p1","kind":"bad","updatedAt":"2026-01-01T00:00:00.000Z"}"#,
        );
        assert!(validate_journal(&conn).unwrap_err().contains("kind"));
    }

    #[test]
    fn rejects_invalid_event_envelope() {
        let conn = fixture();
        conn.execute(
            "INSERT INTO orchestration_events (sequence, event_type, payload_json, metadata_json, event_id, aggregate_kind, stream_id, occurred_at) VALUES (30, 'thread.created', '{}', '{}', '', 'thread', 'stream-test', '2026-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        assert!(validate_journal(&conn).unwrap_err().contains("eventId"));

        let conn = fixture();
        conn.execute(
            "INSERT INTO orchestration_events (sequence, event_type, payload_json, metadata_json, event_id, aggregate_kind, stream_id, occurred_at) VALUES (31, 'thread.created', '{}', '{}', 'event-test', 'bad', 'stream-test', '2026-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        assert!(
            validate_journal(&conn)
                .unwrap_err()
                .contains("aggregateKind")
        );
    }

    #[test]
    fn rejects_non_object_or_malformed_payload_json() {
        let conn = fixture();
        insert_event(&conn, 1, "thread.created", "not json");
        assert!(
            validate_journal(&conn)
                .unwrap_err()
                .contains("not valid JSON")
        );

        let conn = fixture();
        insert_event(&conn, 1, "thread.created", "[]");
        assert!(
            validate_journal(&conn)
                .unwrap_err()
                .contains("payload expected object")
        );
    }
}
