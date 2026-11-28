//! Resume-cursor fallback for imported external provider sessions.
//! `orchestration.importThread` binds a provider-native cursor in
//! `provider_session_runtime`; runners prefer their live projected provider
//! thread id and use this only when no live session has been recorded yet.

use serde_json::Value;

fn cursor_string(cursor: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        cursor
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

pub(crate) fn session_id_from_cursor(provider: &str, cursor_json: &str) -> Option<String> {
    let cursor = serde_json::from_str::<Value>(cursor_json).ok()?;
    match provider {
        "opencode" => cursor_string(&cursor, &["openCodeSessionId"]),
        "omp" | "droid" => cursor_string(&cursor, &["sessionId"]),
        "codex" | "claude" | "claudeAgent" => cursor_string(&cursor, &["resume", "threadId"]),
        _ => cursor_string(
            &cursor,
            &["resume", "threadId", "sessionId", "openCodeSessionId"],
        ),
    }
}

pub(crate) fn imported_session_id(
    conn: &rusqlite::Connection,
    thread_id: &str,
    provider: &str,
) -> Option<String> {
    let cursor_json: Option<String> = conn
        .query_row(
            "SELECT resume_cursor_json FROM provider_session_runtime WHERE thread_id = ?1",
            rusqlite::params![thread_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();
    cursor_json.and_then(|json| session_id_from_cursor(provider, &json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_provider_cursor_shapes() {
        assert_eq!(
            session_id_from_cursor("opencode", r#"{"openCodeSessionId":"oc-1"}"#),
            Some("oc-1".to_string())
        );
        assert_eq!(
            session_id_from_cursor("omp", r#"{"schemaVersion":1,"sessionId":"omp-1"}"#),
            Some("omp-1".to_string())
        );
        assert_eq!(
            session_id_from_cursor("claudeAgent", r#"{"resume":"claude-1"}"#),
            Some("claude-1".to_string())
        );
        assert_eq!(
            session_id_from_cursor("opencode", r#"{"resume":"wrong"}"#),
            None
        );
    }
}
