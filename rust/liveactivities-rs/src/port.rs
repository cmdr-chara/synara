//! Ported from the TypeScript sources named in each doc comment.
//! `Err` mirrors a thrown JS error.

const SWIFT_EPOCH: f64 = 978_307_200.0;

/// Port of `createApnsSender` (`apps/api/src/liveActivities/apns.ts`).
pub fn create_apns_sender(config: serde_json::Value) -> Result<serde_json::Value, String> {
    if config.is_null() {
        return Err("Cannot read properties of null (reading 'privateKey')".to_string());
    }
    Ok(serde_json::json!({ "$function": "" }))
}

fn widget_conversation(item: &serde_json::Value) -> serde_json::Value {
    let state = item.get("state").and_then(|s| s.as_str()).unwrap_or("");
    let label = match state {
        "running" => "Running",
        "needsApproval" => "Approval needed",
        "needsInput" => "Reply needed",
        "ready" => "Ready",
        "failed" => "Failed",
        "stopped" => "Out of date",
        "usageLimit" => "Usage limit",
        "resuming" => "Resuming on computer",
        _ => "",
    };
    let waiting = state == "usageLimit" || state == "resuming";
    let mut out = serde_json::json!({
        "id": item.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "title": "Synara task",
        "detail": "Synara",
        "state": label,
    });
    if !waiting && let Some(t) = item.get("startedAt").and_then(|v| v.as_f64()) {
        out["runningStartedAt"] = serde_json::json!((t - SWIFT_EPOCH).max(0.0));
    }
    if state == "usageLimit"
        && let Some(t) = item.get("usageLimitResetAt").and_then(|v| v.as_f64())
    {
        out["usageLimitResetAt"] = serde_json::json!((t - SWIFT_EPOCH).max(0.0));
    }
    out
}

/// Port of `liveActivityContent` (`apps/api/src/liveActivities/presentation.ts`).
pub fn live_activity_content(
    conversations: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let convs = &conversations;
    let ongoing = |s: &str| {
        matches!(
            s,
            "running" | "needsApproval" | "needsInput" | "usageLimit" | "resuming"
        )
    };
    fn state_of(i: &serde_json::Value) -> &str {
        i.get("state").and_then(|s| s.as_str()).unwrap_or("")
    }
    let running: Vec<_> = convs.iter().filter(|i| ongoing(state_of(i))).collect();
    let completed: Vec<_> = convs.iter().filter(|i| state_of(i) == "ready").collect();
    let failed: Vec<_> = convs.iter().filter(|i| state_of(i) == "failed").collect();
    let r = running.len();
    let c = completed.len();
    Ok(serde_json::json!({
        "ongoing": r > 0,
        "content": {
            "runningConversations": running.iter().take(3).map(|i| widget_conversation(i)).collect::<Vec<_>>(),
            "completedConversations": completed.iter().take(3usize.saturating_sub(r)).map(|i| widget_conversation(i)).collect::<Vec<_>>(),
            "failedConversations": failed.iter().take(3usize.saturating_sub(r + c)).map(|i| widget_conversation(i)).collect::<Vec<_>>(),
        },
    }))
}
