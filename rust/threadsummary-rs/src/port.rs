//! The Rust port of `packages/shared/src/threadSummary.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Port of `pendingRequestInstanceKey` (`packages/shared/src/threadSummary.ts:30`).
pub fn pending_request_instance_key(
    request_id: String,
    lifecycle_generation: Option<String>,
) -> Result<String, String> {
    Ok(format!(
        "{request_id}\u{0}{}",
        lifecycle_generation.as_deref().unwrap_or("legacy")
    ))
}

fn max_iso(left: &Value, right: &Value) -> Value {
    if left.is_null() {
        return right.clone();
    }
    // `left > right`: string pairs compare lexicographically, else numerically
    // (a NaN or non-comparable operand makes the comparison false).
    let gt = match (left, right) {
        (Value::String(l), Value::String(r)) => l > r,
        _ => match (js::as_number(left), js::as_number(right)) {
            (Some(l), Some(r)) => l > r,
            _ => false,
        },
    };
    if gt { left.clone() } else { right.clone() }
}

/// `left.localeCompare(right)`: byte-order comparison matches JS for ASCII.
fn locale_compare(a: Option<&Value>, b: Option<&Value>) -> Result<i64, String> {
    let nullish = match a {
        None => Some("undefined"),
        Some(Value::Null) => Some("null"),
        Some(v) if js::is_undefined(v) => Some("undefined"),
        _ => None,
    };
    if let Some(kind) = nullish {
        return Err(format!(
            "Cannot read properties of {kind} (reading 'localeCompare')"
        ));
    }
    match (a, b) {
        (Some(Value::String(l)), Some(Value::String(r))) => Ok(l.cmp(r) as i64),
        _ => Err("localeCompare is not a function".to_string()),
    }
}

fn compare_activities(left: &Value, right: &Value) -> Result<i64, String> {
    let seq = |a: &Value| {
        a.get("sequence")
            .and_then(js::as_number)
            .unwrap_or(MAX_SAFE_INTEGER)
    };
    let diff = seq(left) - seq(right);
    // `diff || ...`: NaN is falsy and falls through to localeCompare.
    if diff != 0.0 && !diff.is_nan() {
        return Ok(diff as i64);
    }
    let c = locale_compare(left.get("createdAt"), right.get("createdAt"))?;
    if c != 0 {
        return Ok(c);
    }
    locale_compare(left.get("id"), right.get("id"))
}

fn ordered_activities(activities: &[Value]) -> Result<Vec<Value>, String> {
    let mut stable = true;
    for i in 1..activities.len() {
        if compare_activities(&activities[i - 1], &activities[i])? > 0 {
            stable = false;
            break;
        }
    }
    if stable {
        return Ok(activities.to_vec());
    }
    let mut ordered = activities.to_vec();
    ordered.sort_by(|a, b| compare_activities(a, b).unwrap_or(0).cmp(&0));
    Ok(ordered)
}

/// Port of `approvalRequestKindFromRequestType` (`packages/shared/src/threadSummary.ts:92`).
pub fn approval_request_kind_from_request_type(
    request_type: serde_json::Value,
) -> Result<Option<String>, String> {
    let kind = match request_type.as_str() {
        Some("command_execution_approval") | Some("exec_command_approval") => "command",
        Some("file_read_approval") => "file-read",
        Some("file_change_approval") | Some("apply_patch_approval") => "file-change",
        Some("permissions_approval") => "permissions",
        // `tool_approval` is canonical; `dynamic_tool_call` is the legacy value.
        Some("tool_approval") | Some("dynamic_tool_call") => "tool",
        _ => return Ok(None),
    };
    Ok(Some(kind.to_string()))
}

/// Port of `isStalePendingRequestFailureDetail` (`packages/shared/src/threadSummary.ts:119`).
pub fn is_stale_pending_request_failure_detail(detail: Option<String>) -> Result<bool, String> {
    let Some(detail) = detail else {
        return Ok(false);
    };
    if detail.is_empty() {
        return Ok(false);
    }
    let normalized = detail.to_lowercase();
    Ok([
        "stale pending approval request",
        "stale pending user-input request",
        "unknown pending approval request",
        "unknown pending permission request",
        "unknown pending user-input request",
        "stale pending user input request",
        "unknown pending user input request",
    ]
    .iter()
    .any(|needle| normalized.contains(needle)))
}

/// Port of `buildStalePendingRequestFailureDetail` (`packages/shared/src/threadSummary.ts:165`).
pub fn build_stale_pending_request_failure_detail(
    request_kind: String,
    request_id: String,
) -> Result<String, String> {
    Ok(format!(
        "Stale pending {request_kind} request: {request_id}. Provider callback state does not survive app restarts or recovered sessions. Restart the turn to continue."
    ))
}

fn lifecycle_generation_from_payload(
    payload: Option<&serde_json::Map<String, Value>>,
) -> Option<String> {
    payload
        .and_then(|p| p.get("lifecycleGeneration"))
        .and_then(Value::as_str)
        .filter(|g| !g.is_empty())
        .map(str::to_string)
}

fn instance_key(request_id: &Value, generation: Option<&Value>) -> String {
    format!(
        "{}\u{0}{}",
        jscore::js_to_string(request_id),
        generation
            .map(jscore::js_to_string)
            .as_deref()
            .unwrap_or("legacy")
    )
}

// `deleteOpenRequest`: with a generation only that instance key is removed;
// without one every open request carrying this id goes.
fn delete_open_request(
    open: &mut Vec<(String, Value)>,
    request_id: &Value,
    generation: Option<&Value>,
) {
    if generation.is_some() {
        let key = instance_key(request_id, generation);
        open.retain(|(k, _)| k != &key);
        return;
    }
    open.retain(|(_, v)| v != request_id);
}

fn replace_open_request(
    open: &mut Vec<(String, Value)>,
    request_id: &Value,
    generation: Option<&Value>,
) {
    delete_open_request(open, request_id, None);
    open.push((instance_key(request_id, generation), request_id.clone()));
}

fn has_structured_user_input_questions(payload: Option<&serde_json::Map<String, Value>>) -> bool {
    let Some(questions) = payload
        .and_then(|p| p.get("questions"))
        .and_then(Value::as_array)
    else {
        return false;
    };
    questions.iter().any(|entry| {
        let Some(question) = entry.as_object() else {
            return false;
        };
        let has_strings = ["id", "header", "question"]
            .iter()
            .all(|k| question.get(*k).is_some_and(Value::is_string));
        has_strings
            && question
                .get("options")
                .and_then(Value::as_array)
                .is_some_and(|options| {
                    options.iter().any(|option| {
                        option.as_object().is_some_and(|o| {
                            o.get("label").is_some_and(Value::is_string)
                                && o.get("description").is_some_and(Value::is_string)
                        })
                    })
                })
    })
}

fn request_ids_payload(approvals: Vec<(String, Value)>, inputs: Vec<(String, Value)>) -> Value {
    let mut out = serde_json::Map::new();
    out.insert(
        "approvalRequestIds".to_string(),
        Value::Array(approvals.into_iter().map(|(_, v)| v).collect()),
    );
    out.insert(
        "userInputRequestIds".to_string(),
        Value::Array(inputs.into_iter().map(|(_, v)| v).collect()),
    );
    Value::Object(out)
}

/// Port of `derivePendingThreadRequestIds` (`packages/shared/src/threadSummary.ts:231`).
pub fn derive_pending_thread_request_ids(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'pendingInteractions')".to_string());
    }
    let pending = input.get("pendingInteractions");
    if let Some(pending) = pending.filter(|p| !js::is_undefined(p)) {
        let mut approvals: Vec<(String, Value)> = Vec::new();
        let mut inputs: Vec<(String, Value)> = Vec::new();
        // `input.pendingInteractions ?? []`: null yields an empty projection;
        // strings iterate code points, other non-iterables raise.
        let interactions: Vec<Value> = match pending {
            Value::Array(a) => a.clone(),
            Value::Null => Vec::new(),
            Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
            _ => return Err("input.pendingInteractions is not iterable".to_string()),
        };
        for interaction in &interactions {
            let is_approval =
                interaction.get("interactionKind").and_then(Value::as_str) == Some("approval");
            let status = interaction.get("status").and_then(Value::as_str);
            if status != Some("pending") && status != Some("retryable") {
                continue;
            }
            let request_id = interaction
                .get("requestId")
                .cloned()
                .unwrap_or_else(js::undefined);
            let generation = interaction
                .get("lifecycleGeneration")
                .filter(|g| !js::is_undefined(g) && !g.is_null());
            let key = instance_key(&request_id, generation);
            let open = if is_approval {
                &mut approvals
            } else {
                &mut inputs
            };
            open.retain(|(k, _)| k != &key);
            open.push((key, request_id));
        }
        return Ok(request_ids_payload(approvals, inputs));
    }

    let activities = input.get("activities");
    let Some(activities) = activities.and_then(Value::as_array) else {
        return Err("Cannot read properties of undefined (reading 'length')".to_string());
    };
    let ordered = ordered_activities(activities)?;
    let mut open_approvals: Vec<(String, Value)> = Vec::new();
    let mut open_inputs: Vec<(String, Value)> = Vec::new();
    for activity in ordered {
        let payload = activity.get("payload").and_then(Value::as_object);
        let request_id = payload
            .and_then(|p| p.get("requestId"))
            .filter(|v| v.is_string())
            .cloned();
        let detail = payload
            .and_then(|p| p.get("detail"))
            .and_then(Value::as_str);
        let generation = lifecycle_generation_from_payload(payload).map(Value::String);
        let kind = activity.get("kind").and_then(Value::as_str).unwrap_or("");
        let Some(request_id) = request_id else {
            continue;
        };
        match kind {
            "approval.requested" => {
                let request_kind = payload
                    .and_then(|p| p.get("requestKind"))
                    .and_then(Value::as_str)
                    .filter(|k| {
                        ["command", "file-read", "file-change", "permissions", "tool"].contains(k)
                    })
                    .map(str::to_string)
                    .or_else(|| {
                        approval_request_kind_from_request_type(
                            payload
                                .and_then(|p| p.get("requestType"))
                                .cloned()
                                .unwrap_or_else(js::undefined),
                        )
                        .ok()
                        .flatten()
                    });
                if request_kind.is_some() {
                    replace_open_request(&mut open_approvals, &request_id, generation.as_ref());
                }
            }
            "approval.resolved" => {
                delete_open_request(&mut open_approvals, &request_id, generation.as_ref());
            }
            "provider.approval.respond.failed"
                if is_stale_pending_request_failure_detail(detail.map(str::to_string))? =>
            {
                delete_open_request(&mut open_approvals, &request_id, generation.as_ref());
            }
            "user-input.requested" => {
                if has_structured_user_input_questions(payload) {
                    replace_open_request(&mut open_inputs, &request_id, generation.as_ref());
                }
            }
            "user-input.resolved" => {
                delete_open_request(&mut open_inputs, &request_id, generation.as_ref());
            }
            "provider.user-input.respond.failed"
                if is_stale_pending_request_failure_detail(detail.map(str::to_string))? =>
            {
                delete_open_request(&mut open_inputs, &request_id, generation.as_ref());
            }
            _ => {}
        }
    }
    Ok(request_ids_payload(open_approvals, open_inputs))
}

/// Port of `resolveHumanMessageAt` (`packages/shared/src/threadSummary.ts:338`).
pub fn resolve_human_message_at(message: serde_json::Value) -> Result<serde_json::Value, String> {
    if message.is_null() {
        return Err("Cannot read properties of null (reading 'role')".to_string());
    }
    if js::is_undefined(&message) {
        return Err("Cannot read properties of undefined (reading 'role')".to_string());
    }
    if message.get("role").and_then(Value::as_str) != Some("user") {
        return Ok(Value::Null);
    }
    // `dispatchOrigin != null && dispatchOrigin !== "user"` (loose `!=`).
    match message.get("dispatchOrigin") {
        Some(origin)
            if !origin.is_null()
                && !js::is_undefined(origin)
                && origin.as_str() != Some("user") =>
        {
            return Ok(Value::Null);
        }
        _ => {}
    }
    let created = message
        .get("createdAt")
        .cloned()
        .unwrap_or_else(js::undefined);
    let right = message
        .get("updatedAt")
        .filter(|u| !u.is_null() && !js::is_undefined(u))
        .cloned()
        .unwrap_or_else(|| created.clone());
    Ok(max_iso(&created, &right))
}

fn iter_messages(input: &Value) -> Result<Vec<Value>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'messages')".to_string());
    }
    match input.get("messages") {
        Some(Value::Array(a)) => Ok(a.clone()),
        Some(Value::String(s)) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        _ => Err("input.messages is not iterable".to_string()),
    }
}

fn resolve_latest_proposed_plan(input: &Value) -> Result<Option<Value>, String> {
    let plans = match input.get("proposedPlans") {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err("input.proposedPlans is not iterable".to_string()),
    };
    let turn_id = input.get("latestTurn").and_then(|t| t.get("turnId"));
    let by_order = |a: &Value, b: &Value| -> Result<i64, String> {
        let c = locale_compare(a.get("updatedAt"), b.get("updatedAt"))?;
        if c != 0 {
            return Ok(c);
        }
        locale_compare(a.get("id"), b.get("id"))
    };
    if let Some(turn_id) = turn_id.filter(|t| jscore::js_truthy(t)) {
        let mut matching: Vec<Value> = plans
            .iter()
            .filter(|p| p.get("turnId") == Some(turn_id))
            .cloned()
            .collect();
        matching.sort_by(|a, b| by_order(a, b).unwrap_or(0).cmp(&0));
        if let Some(plan) = matching.last() {
            return Ok(Some(plan.clone()));
        }
    }
    let mut sorted = plans;
    sorted.sort_by(|a, b| by_order(a, b).unwrap_or(0).cmp(&0));
    Ok(sorted.last().cloned())
}

fn derive_state(input: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut latest_user_at = Value::Null;
    let mut latest_human_at = Value::Null;
    for message in iter_messages(input)? {
        if message.is_null() {
            return Err("Cannot read properties of null (reading 'role')".to_string());
        }
        if message.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        latest_user_at = max_iso(
            &latest_user_at,
            &message
                .get("createdAt")
                .cloned()
                .unwrap_or_else(js::undefined),
        );
        let human_at = resolve_human_message_at(message)?;
        if !human_at.is_null() {
            latest_human_at = max_iso(&latest_human_at, &human_at);
        }
    }
    let pending = derive_pending_thread_request_ids(input.clone())?;
    let approvals = pending
        .get("approvalRequestIds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let inputs = pending
        .get("userInputRequestIds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let plan = resolve_latest_proposed_plan(input)?;
    let actionable = plan
        .as_ref()
        .is_some_and(|p| p.get("implementedAt").is_some_and(Value::is_null));
    let mut out = serde_json::Map::new();
    out.insert("latestUserMessageAt".to_string(), latest_user_at);
    out.insert("latestHumanMessageAt".to_string(), latest_human_at);
    out.insert(
        "pendingApprovalCount".to_string(),
        js::number(approvals.len() as f64),
    );
    out.insert(
        "pendingUserInputCount".to_string(),
        js::number(inputs.len() as f64),
    );
    out.insert(
        "hasPendingApprovals".to_string(),
        Value::Bool(!approvals.is_empty()),
    );
    out.insert(
        "hasPendingUserInput".to_string(),
        Value::Bool(!inputs.is_empty()),
    );
    out.insert(
        "hasActionableProposedPlan".to_string(),
        Value::Bool(actionable),
    );
    Ok(Value::Object(out))
}

/// Port of `deriveThreadSummaryState` (`packages/shared/src/threadSummary.ts:347`).
pub fn derive_thread_summary_state(input: serde_json::Value) -> Result<serde_json::Value, String> {
    derive_state(&input)
}

/// Port of `deriveThreadSummaryMetadata` (`packages/shared/src/threadSummary.ts:398`).
pub fn derive_thread_summary_metadata(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = derive_state(&input)?;
    let mut out = serde_json::Map::new();
    for key in [
        "latestUserMessageAt",
        "latestHumanMessageAt",
        "hasPendingApprovals",
        "hasPendingUserInput",
        "hasActionableProposedPlan",
    ] {
        out.insert(
            key.to_string(),
            state.get(key).cloned().unwrap_or(Value::Null),
        );
    }
    Ok(Value::Object(out))
}
