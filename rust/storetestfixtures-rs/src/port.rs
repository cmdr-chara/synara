//! The Rust port of `apps/web/src/storeTestFixtures.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

fn prop(value: &Value, key: &str) -> Value {
    value.get(key).cloned().unwrap_or_else(js::undefined)
}

fn prop_or_err(value: &Value, key: &str) -> Result<Value, String> {
    if value.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if js::is_undefined(value) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(prop(value, key))
}

fn truthy(value: &Value) -> bool {
    if let Some(n) = js::as_number(value) {
        return n != 0.0 && !n.is_nan();
    }
    if js::is_undefined(value) {
        return false;
    }
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

const MARKER_KEYS: [&str; 10] = [
    "$undefined",
    "$num",
    "$function",
    "$map",
    "$set",
    "$date",
    "$bytes",
    "$typed",
    "$buffer",
    "$bigint",
];

// Own-enumerable properties as a JS object spread sees them, in source order.
fn spread_pairs(value: &Value) -> Vec<(String, Value)> {
    if js::is_undefined(value) || value.is_null() {
        return Vec::new();
    }
    if let Some(units) = js::string_units(value) {
        return units
            .iter()
            .enumerate()
            .map(|(i, u)| (i.to_string(), js::utf16_string(&[*u])))
            .collect();
    }
    match value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| (i.to_string(), item.clone()))
            .collect(),
        Value::Object(o) => {
            if o.len() == 1 {
                let k = o.keys().next().map(String::as_str).unwrap_or("");
                if MARKER_KEYS.contains(&k) {
                    return Vec::new();
                }
            }
            o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        }
        _ => Vec::new(),
    }
}

fn set_key(map: &mut Map<String, Value>, key: String, value: Value) {
    map.insert(key, value);
}

// JS objects serialize integer-index keys first, ascending.
fn is_array_index(key: &str) -> Option<u32> {
    if key.is_empty() || key.len() > 10 {
        return None;
    }
    if key.len() > 1 && key.starts_with('0') {
        return None;
    }
    let n: u64 = key.parse().ok()?;
    if n < 4294967295 { Some(n as u32) } else { None }
}

fn reorder_js_keys(value: &mut Value) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(reorder_js_keys),
        Value::Object(o) => {
            for v in o.values_mut() {
                reorder_js_keys(v);
            }
            let entries: Vec<(String, Value)> =
                o.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            let mut indexed: Vec<(u32, String, Value)> = Vec::new();
            let mut rest: Vec<(String, Value)> = Vec::new();
            for (k, v) in entries {
                match is_array_index(&k) {
                    Some(n) => indexed.push((n, k, v)),
                    None => rest.push((k, v)),
                }
            }
            indexed.sort_by_key(|(n, _, _)| *n);
            *o = Map::new();
            for (_, k, v) in indexed {
                o.insert(k, v);
            }
            for (k, v) in rest {
                o.insert(k, v);
            }
        }
        _ => {}
    }
}

fn js_to_string(value: &Value) -> String {
    if let Some(n) = js::as_number(value) {
        return js::number_to_string(n);
    }
    if js::is_undefined(value) {
        return "undefined".to_string();
    }
    if let Some(units) = js::string_units(value) {
        return String::from_utf16_lossy(&units);
    }
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                if item.is_null() || js::is_undefined(item) {
                    String::new()
                } else {
                    js_to_string(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn to_property_key(value: &Value) -> String {
    js_to_string(value)
}

static UUID_COUNTER: AtomicU64 = AtomicU64::new(0x9e3779b97f4a7c15);

fn pseudo_uuid() -> String {
    let a = UUID_COUNTER.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed)
        ^ std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0);
    let b = a.wrapping_mul(0xbf58476d1ce4e5b9) ^ 0x94d049bb133111eb;
    let hi = a.rotate_left(13) ^ b;
    let lo = b.rotate_left(31) ^ a;
    let mut bytes = Vec::with_capacity(16);
    bytes.extend_from_slice(&hi.to_be_bytes());
    bytes.extend_from_slice(&lo.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

fn object(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Object(map)
}

fn codex_selection(model: &str) -> Value {
    object(vec![("provider", json!("codex")), ("model", json!(model))])
}

fn make_project_default() -> Value {
    object(vec![
        ("id", json!("project-1")),
        ("kind", json!("project")),
        ("name", json!("Project")),
        ("remoteName", json!("Project")),
        ("folderName", json!("project")),
        ("localName", Value::Null),
        ("cwd", json!("/tmp/project")),
        ("defaultModelSelection", codex_selection("gpt-5-codex")),
        ("expanded", json!(true)),
        ("spaceId", Value::Null),
        ("scripts", json!([])),
    ])
}

fn apply_overrides(base: &mut Map<String, Value>, overrides: &Value) {
    for (k, v) in spread_pairs(overrides) {
        set_key(base, k, v);
    }
}

fn with_overrides(mut base: Value, overrides: &Value) -> Value {
    if let Value::Object(map) = &mut base {
        apply_overrides(map, overrides);
    }
    reorder_js_keys(&mut base);
    base
}

// A `param = {}` default replaces undefined (but not null) before any property reads.
fn default_object(value: Value) -> Value {
    if js::is_undefined(&value) {
        json!({})
    } else {
        value
    }
}

fn overrides_arg(overrides: &Option<Value>) -> Value {
    match overrides {
        Some(v) => v.clone(),
        None => js::undefined(),
    }
}

// EventId.makeUnsafe / TurnId.makeUnsafe only accept strings.
fn make_unsafe_string(value: Value) -> Result<Value, String> {
    match &value {
        Value::String(_) => Ok(value),
        _ => Err(format!("Expected string, got {}", js_to_string(&value))),
    }
}

/// Port of `makeThread` (`apps/web/src/storeTestFixtures.ts:22`).
pub fn make_thread(overrides: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    let overrides = default_object(overrides_arg(&overrides));
    let defaults = object(vec![
        ("id", json!("thread-1")),
        ("codexThreadId", Value::Null),
        ("projectId", json!("project-1")),
        ("title", json!("Thread")),
        ("modelSelection", codex_selection("gpt-5-codex")),
        ("runtimeMode", json!("full-access")),
        ("interactionMode", json!("default")),
        ("session", Value::Null),
        ("messages", json!([])),
        ("turnDiffSummaries", json!([])),
        ("activities", json!([])),
        ("proposedPlans", json!([])),
        ("error", Value::Null),
        ("createdAt", json!("2026-02-13T00:00:00.000Z")),
        ("latestTurn", Value::Null),
        ("latestUserMessageAt", Value::Null),
        ("latestHumanMessageAt", Value::Null),
        ("hasPendingApprovals", json!(false)),
        ("hasPendingUserInput", json!(false)),
        ("hasActionableProposedPlan", json!(false)),
        ("envMode", json!("local")),
        ("branch", Value::Null),
        ("worktreePath", Value::Null),
        ("forkSourceThreadId", Value::Null),
        ("sidechatSourceThreadId", Value::Null),
        ("handoff", Value::Null),
    ]);
    let _ = js::is_undefined(&overrides); // spread of undefined adds nothing
    Ok(with_overrides(defaults, &overrides))
}

/// Port of `makeDomainEvent` (`apps/web/src/storeTestFixtures.ts:57`).
pub fn make_domain_event(
    r#type: serde_json::Value,
    payload: serde_json::Value,
    overrides: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let overrides = default_object(overrides);
    let has = |key: &str| -> Result<bool, String> {
        match &payload {
            Value::Object(o) => Ok(o.contains_key(key)),
            Value::Array(_) => Ok(false),
            _ => Err(format!(
                "Cannot use 'in' operator to search for '{key}' in {}",
                js_to_string(&payload)
            )),
        }
    };
    let (aggregate_id, aggregate_kind) = if has("threadId")? {
        (prop(&payload, "threadId"), "thread")
    } else if has("spaceId")? {
        (prop(&payload, "spaceId"), "space")
    } else if has("projectId")? {
        (prop(&payload, "projectId"), "project")
    } else {
        (json!("project-1"), "project")
    };
    let seq = prop_or_err(&overrides, "sequence")?;
    let sequence = if seq.is_null() || js::is_undefined(&seq) {
        json!(1)
    } else {
        seq
    };
    let event_id = prop_or_err(&overrides, "eventId")?;
    let event_id = if event_id.is_null() || js::is_undefined(&event_id) {
        json!(format!("event-{}", pseudo_uuid()))
    } else {
        event_id
    };
    let kind = prop_or_err(&overrides, "aggregateKind")?;
    let aggregate_kind = if kind.is_null() || js::is_undefined(&kind) {
        json!(aggregate_kind)
    } else {
        kind
    };
    let occurred = prop_or_err(&overrides, "occurredAt")?;
    let occurred_at = if occurred.is_null() || js::is_undefined(&occurred) {
        json!("2026-02-27T00:00:00.000Z")
    } else {
        occurred
    };
    let command = prop_or_err(&overrides, "commandId")?;
    let command_id = if command.is_null() || js::is_undefined(&command) {
        Value::Null
    } else {
        command
    };
    let causation = prop_or_err(&overrides, "causationEventId")?;
    let causation_id = if causation.is_null() || js::is_undefined(&causation) {
        Value::Null
    } else {
        causation
    };
    let correlation = prop_or_err(&overrides, "correlationId")?;
    let correlation_id = if correlation.is_null() || js::is_undefined(&correlation) {
        Value::Null
    } else {
        correlation
    };
    let metadata = prop_or_err(&overrides, "metadata")?;
    let metadata = if metadata.is_null() || js::is_undefined(&metadata) {
        json!({})
    } else {
        metadata
    };
    let base = object(vec![
        ("type", r#type),
        ("payload", payload),
        ("sequence", sequence),
        ("eventId", event_id),
        ("aggregateKind", aggregate_kind),
        ("aggregateId", aggregate_id),
        ("occurredAt", occurred_at),
        ("commandId", command_id),
        ("causationEventId", causation_id),
        ("correlationId", correlation_id),
        ("metadata", metadata),
    ]);
    Ok(with_overrides(base, &overrides))
}

/// Port of `makeActivity` (`apps/web/src/storeTestFixtures.ts:88`).
pub fn make_activity(overrides: serde_json::Value) -> Result<serde_json::Value, String> {
    let id = prop_or_err(&overrides, "id")?;
    let id = if id.is_null() || js::is_undefined(&id) {
        json!(pseudo_uuid())
    } else {
        make_unsafe_string(id)?
    };
    let created = prop_or_err(&overrides, "createdAt")?;
    let created_at = if created.is_null() || js::is_undefined(&created) {
        json!("2026-02-23T00:00:00.000Z")
    } else {
        created
    };
    let kind = prop_or_err(&overrides, "kind")?;
    let kind = if kind.is_null() || js::is_undefined(&kind) {
        json!("tool.started")
    } else {
        kind
    };
    let summary = prop_or_err(&overrides, "summary")?;
    let summary = if summary.is_null() || js::is_undefined(&summary) {
        json!("Tool call")
    } else {
        summary
    };
    let tone = prop_or_err(&overrides, "tone")?;
    let tone = if tone.is_null() || js::is_undefined(&tone) {
        json!("tool")
    } else {
        tone
    };
    let payload = prop_or_err(&overrides, "payload")?;
    let payload = if payload.is_null() || js::is_undefined(&payload) {
        json!({})
    } else {
        payload
    };
    let turn = prop_or_err(&overrides, "turnId")?;
    let turn_id = if truthy(&turn) {
        make_unsafe_string(turn)?
    } else {
        Value::Null
    };
    let mut map = Map::new();
    map.insert("id".to_string(), id);
    map.insert("createdAt".to_string(), created_at);
    map.insert("kind".to_string(), kind);
    map.insert("summary".to_string(), summary);
    map.insert("tone".to_string(), tone);
    map.insert("payload".to_string(), payload);
    map.insert("turnId".to_string(), turn_id);
    let sequence = prop_or_err(&overrides, "sequence")?;
    if !js::is_undefined(&sequence) {
        map.insert("sequence".to_string(), sequence);
    }
    let sequence_source = prop_or_err(&overrides, "sequenceSource")?;
    if !js::is_undefined(&sequence_source) {
        map.insert("sequenceSource".to_string(), sequence_source);
    }
    let mut out = Value::Object(map);
    reorder_js_keys(&mut out);
    Ok(out)
}

/// Port of `makeState` (`apps/web/src/storeTestFixtures.ts:112`).
pub fn make_state(thread: serde_json::Value) -> Result<serde_json::Value, String> {
    if thread.is_null() {
        return Err("Cannot destructure property 'session' of 'null' as it is null".to_string());
    }
    if js::is_undefined(&thread) {
        return Err(
            "Cannot destructure property 'session' of 'undefined' as it is undefined".to_string(),
        );
    }
    let session = prop(&thread, "session");
    let latest_turn = prop(&thread, "latestTurn");
    let pending_plan = prop(&thread, "pendingSourceProposedPlan");
    let messages = prop(&thread, "messages");
    let activities = prop(&thread, "activities");
    let proposed_plans = prop(&thread, "proposedPlans");
    let turn_diffs = prop(&thread, "turnDiffSummaries");
    const EXCLUDED: [&str; 7] = [
        "session",
        "latestTurn",
        "pendingSourceProposedPlan",
        "messages",
        "activities",
        "proposedPlans",
        "turnDiffSummaries",
    ];
    let mut shell = Map::new();
    for (k, v) in spread_pairs(&thread) {
        if !EXCLUDED.contains(&k.as_str()) {
            shell.insert(k, v);
        }
    }
    let thread_id = prop_or_err(&thread, "id")?;
    let key = to_property_key(&thread_id);

    let map_ids = |items: &Value, field: &str| -> Result<Vec<Value>, String> {
        match items {
            Value::Array(list) => {
                let mut out = Vec::new();
                for item in list {
                    out.push(prop_or_err(item, field)?);
                }
                Ok(out)
            }
            Value::Null => Err("Cannot read properties of null (reading 'map')".to_string()),
            _ if js::is_undefined(items) => {
                Err("Cannot read properties of undefined (reading 'map')".to_string())
            }
            _ => Err("messages.map is not a function".to_string()),
        }
    };
    let by_id = |items: &Value, field: &str| -> Result<Value, String> {
        match items {
            Value::Array(list) => {
                let mut map = Map::new();
                for item in list {
                    let id = prop_or_err(item, field)?;
                    map.insert(to_property_key(&id), item.clone());
                }
                Ok(Value::Object(map))
            }
            Value::Null => Err("Cannot read properties of null (reading 'map')".to_string()),
            _ if js::is_undefined(items) => {
                Err("Cannot read properties of undefined (reading 'map')".to_string())
            }
            _ => Err("messages.map is not a function".to_string()),
        }
    };

    let message_ids = map_ids(&messages, "id")?;
    let message_by = by_id(&messages, "id")?;
    let activity_ids = map_ids(&activities, "id")?;
    let activity_by = by_id(&activities, "id")?;
    let plan_ids = map_ids(&proposed_plans, "id")?;
    let plan_by = by_id(&proposed_plans, "id")?;
    let diff_ids = map_ids(&turn_diffs, "turnId")?;
    let diff_by = by_id(&turn_diffs, "turnId")?;

    let mut out = object(vec![
        ("spaces", json!([])),
        ("projects", json!([make_project_default()])),
        ("sidebarThreadSummaryById", json!({})),
        ("threadsHydrated", json!(true)),
        ("threadIds", json!([thread_id])),
        (
            "threadShellById",
            json!({ key.clone(): Value::Object(shell) }),
        ),
        ("threadSessionById", json!({ key.clone(): session })),
        (
            "threadTurnStateById",
            json!({ key.clone(): object(vec![
                ("latestTurn", latest_turn),
                ("pendingSourceProposedPlan", pending_plan),
            ]) }),
        ),
        ("messageIdsByThreadId", json!({ key.clone(): message_ids })),
        ("messageByThreadId", json!({ key.clone(): message_by })),
        (
            "activityIdsByThreadId",
            json!({ key.clone(): activity_ids }),
        ),
        ("activityByThreadId", json!({ key.clone(): activity_by })),
        (
            "proposedPlanIdsByThreadId",
            json!({ key.clone(): plan_ids }),
        ),
        ("proposedPlanByThreadId", json!({ key.clone(): plan_by })),
        ("turnDiffIdsByThreadId", json!({ key.clone(): diff_ids })),
        ("turnDiffSummaryByThreadId", json!({ key: diff_by })),
    ]);
    reorder_js_keys(&mut out);
    Ok(out)
}

/// Port of `makeProject` (`apps/web/src/storeTestFixtures.ts:153`).
pub fn make_project(overrides: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    let overrides = default_object(overrides_arg(&overrides));
    Ok(with_overrides(make_project_default(), &overrides))
}

/// Port of `makeReadModelThread` (`apps/web/src/storeTestFixtures.ts:175`).
pub fn make_read_model_thread(overrides: serde_json::Value) -> Result<serde_json::Value, String> {
    let defaults = object(vec![
        ("id", json!("thread-1")),
        ("projectId", json!("project-1")),
        ("title", json!("Thread")),
        ("modelSelection", codex_selection("gpt-5.3-codex")),
        ("runtimeMode", json!("full-access")),
        ("interactionMode", json!("default")),
        ("envMode", json!("local")),
        ("branch", Value::Null),
        ("worktreePath", Value::Null),
        ("forkSourceThreadId", Value::Null),
        ("sidechatSourceThreadId", Value::Null),
        ("latestTurn", Value::Null),
        ("createdAt", json!("2026-02-27T00:00:00.000Z")),
        ("updatedAt", json!("2026-02-27T00:00:00.000Z")),
        ("deletedAt", Value::Null),
        ("handoff", Value::Null),
        ("messages", json!([])),
        ("activities", json!([])),
        ("proposedPlans", json!([])),
        ("checkpoints", json!([])),
        ("session", Value::Null),
    ]);
    Ok(with_overrides(defaults, &overrides))
}

/// Port of `makeReadModel` (`apps/web/src/storeTestFixtures.ts:205`).
pub fn make_read_model(thread: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(object(vec![
        ("snapshotSequence", json!(1)),
        ("updatedAt", json!("2026-02-27T00:00:00.000Z")),
        ("spaces", json!([])),
        (
            "projects",
            json!([object(vec![
                ("id", json!("project-1")),
                ("kind", json!("project")),
                ("title", json!("Project")),
                ("workspaceRoot", json!("/tmp/project")),
                ("defaultModelSelection", codex_selection("gpt-5.3-codex")),
                ("createdAt", json!("2026-02-27T00:00:00.000Z")),
                ("updatedAt", json!("2026-02-27T00:00:00.000Z")),
                ("deletedAt", Value::Null),
                ("scripts", json!([])),
                ("spaceId", Value::Null),
            ])]),
        ),
        ("threads", json!([thread])),
    ]))
}

/// Port of `makeShellSnapshot` (`apps/web/src/storeTestFixtures.ts:233`).
pub fn make_shell_snapshot(thread: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(object(vec![
        ("snapshotSequence", json!(2)),
        ("updatedAt", json!("2026-02-27T00:01:00.000Z")),
        ("spaces", json!([])),
        (
            "projects",
            json!([object(vec![
                ("id", json!("project-1")),
                ("title", json!("Project")),
                ("workspaceRoot", json!("/tmp/project")),
                ("defaultModelSelection", codex_selection("gpt-5.3-codex")),
                ("createdAt", json!("2026-02-27T00:00:00.000Z")),
                ("updatedAt", json!("2026-02-27T00:00:00.000Z")),
                ("scripts", json!([])),
                ("spaceId", Value::Null),
            ])]),
        ),
        ("threads", json!([thread])),
    ]))
}

/// Port of `makeReadModelProject` (`apps/web/src/storeTestFixtures.ts:257`).
pub fn make_read_model_project(overrides: serde_json::Value) -> Result<serde_json::Value, String> {
    let defaults = object(vec![
        ("id", json!("project-1")),
        ("kind", json!("project")),
        ("title", json!("Project")),
        ("workspaceRoot", json!("/tmp/project")),
        ("defaultModelSelection", codex_selection("gpt-5.3-codex")),
        ("createdAt", json!("2026-02-27T00:00:00.000Z")),
        ("updatedAt", json!("2026-02-27T00:00:00.000Z")),
        ("deletedAt", Value::Null),
        ("scripts", json!([])),
        ("spaceId", Value::Null),
    ]);
    Ok(with_overrides(defaults, &overrides))
}

/// Port of `makeUsageLimitRecovery` (`apps/web/src/storeTestFixtures.ts:300`).
pub fn make_usage_limit_recovery(
    overrides: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let overrides = default_object(overrides_arg(&overrides));
    let defaults = object(vec![
        ("id", json!("usage-limit-episode-1")),
        ("turnId", json!("limited-turn")),
        ("resetAt", json!("2026-11-01T12:00:00.000Z")),
        ("autoResume", json!(true)),
        ("provider", json!("codex")),
        ("providerInstanceId", json!("codex")),
        ("model", json!("gpt-5.3-codex")),
        ("bindingKey", json!("test-binding")),
        ("policyEpoch", json!(1)),
    ]);
    Ok(with_overrides(defaults, &overrides))
}
