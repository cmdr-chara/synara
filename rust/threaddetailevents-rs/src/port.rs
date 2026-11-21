//! The Rust port of `packages/shared/src/threadDetailEvents.ts` (clean-room port).

#![forbid(unsafe_code)]

use serde_json::Value;

const TYPES: &[&str] = &[
    "thread.message-sent",
    "thread.turn-start-requested",
    "thread.claude-cache-set",
    "thread.claude-cache-response-requested",
    "thread.session-stop-requested",
    "thread.turn-interrupt-requested",
    "thread.async-user-input-answered",
    "thread.proposed-plan-upserted",
    "thread.activity-appended",
    "thread.turn-diff-completed",
    "thread.reverted",
    "thread.conversation-rolled-back",
    "thread.session-set",
    "thread.meta-updated",
    "thread.pinned-message-added",
    "thread.pinned-message-removed",
    "thread.pinned-message-done-set",
    "thread.pinned-message-label-set",
    "thread.archived",
    "thread.unarchived",
    "thread.sidechat-activity-recorded",
    "thread.sidechat-expired",
];

pub fn is_thread_detail_event_for(event: Value, thread_id: Value) -> Result<bool, String> {
    if event.is_null() {
        return Err("Cannot read properties of null (reading 'aggregateKind')".into());
    }
    if crate::js::is_undefined(&event) {
        return Err("Cannot read properties of undefined (reading 'aggregateKind')".into());
    }
    Ok(
        event.get("aggregateKind").and_then(Value::as_str) == Some("thread")
            && event.get("aggregateId").is_some_and(|id| id == &thread_id)
            && event
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|t| TYPES.contains(&t)),
    )
}
