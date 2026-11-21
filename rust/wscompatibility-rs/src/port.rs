//! The Rust port of `apps/server/src/wsCompatibility.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

const WS_PROTOCOL_EPOCH: f64 = 1.0;
const WS_PROTOCOL_MIN_REVISION: f64 = 3.0;
const WS_PROTOCOL_MAX_REVISION: f64 = 3.0;
const SERVER_BUILD: &str = "1.6.4";
// `randomUUID()` at module load in the original; one fixed id per process here.
const SERVER_INSTANCE_ID: &str = "00000000-0000-0000-0000-000000000000";

const WS_SERVER_CAPABILITIES: &[&str] = &[
    "remote-agent.gateway",
    "ws.negotiate",
    "ws.bootstrap",
    "git.worktree-setup-progress",
    "projects.github-provisioning",
    "projects.file-watch",
    "orchestration.turn-dispatch-settlement",
    "orchestration.thread-goal-visible-start",
    "orchestration.usage-limit-recovery",
    "git.action-recovery",
    "server.runtime-status",
    "transport.http-negotiate",
];

fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) => o.get(key),
        _ => None,
    }
}

fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(a) => {
            if a.is_empty() {
                0.0
            } else if a.len() == 1 {
                js_to_number(&a[0])
            } else {
                f64::NAN
            }
        }
        Value::Object(_) => f64::NAN,
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a.iter().map(js_to_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `{~effect/Effect/args: {reasons: [{_tag:"Fail", error:{$error:msg}}]}}` — the
/// wire serialization the recorder captured for `Effect.fail`.
fn fail_effect(message: String) -> Value {
    json!({
        "~effect/Effect/args": {
            "~effect/Cause": "~effect/Cause",
            "reasons": [{
                "~effect/Cause/Reason": "~effect/Cause/Reason",
                "annotations": {"$map": []},
                "_tag": "Fail",
                "error": {"$error": message},
            }],
        }
    })
}

fn succeed_effect(value: Value) -> Value {
    json!({
        "~effect/Effect/args": {
            "~effect/Cause": "~effect/Cause",
            "reasons": [{
                "~effect/Cause/Reason": "~effect/Cause/Reason",
                "annotations": {"$map": []},
                "_tag": "Success",
                "value": value,
            }],
        }
    })
}

/// Port of `negotiateWsCompatibility` (`apps/server/src/wsCompatibility.ts:38`).
pub fn negotiate_ws_compatibility(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("null is not an object (evaluating 'input.minRevision')".to_string());
    }
    if crate::js::is_undefined(&input) {
        return Err("undefined is not an object (evaluating 'input.minRevision')".to_string());
    }
    let undefined = crate::js::undefined();
    let min_revision = get(&input, "minRevision").unwrap_or(&undefined);
    let max_revision = get(&input, "maxRevision").unwrap_or(&undefined);
    let epoch = get(&input, "protocolEpoch").unwrap_or(&undefined);

    // `input.minRevision > input.maxRevision` — NaN on either side is false.
    if js_to_number(min_revision) > js_to_number(max_revision) {
        return Ok(fail_effect(
            "The client sent an invalid WebSocket protocol range.".to_string(),
        ));
    }
    // `input.protocolEpoch !== WS_PROTOCOL_EPOCH` (strict).
    let epoch_is_one = matches!(epoch, Value::Number(n) if n.as_f64() == Some(WS_PROTOCOL_EPOCH));
    if !epoch_is_one {
        return Ok(fail_effect(format!(
            "WebSocket protocol epoch {} is incompatible with server epoch {}.",
            if is_nullish(epoch) {
                "undefined".to_string()
            } else {
                js_to_string(epoch)
            },
            crate::js::number_to_string(WS_PROTOCOL_EPOCH),
        )));
    }
    let lowest = js_to_number(min_revision).max(WS_PROTOCOL_MIN_REVISION);
    let negotiated = js_to_number(max_revision).min(WS_PROTOCOL_MAX_REVISION);
    if negotiated < lowest {
        return Ok(fail_effect(format!(
            "WebSocket protocol revisions {}-{} do not overlap server revisions {}-{}.",
            js_to_string(min_revision),
            js_to_string(max_revision),
            crate::js::number_to_string(WS_PROTOCOL_MIN_REVISION),
            crate::js::number_to_string(WS_PROTOCOL_MAX_REVISION),
        )));
    }
    let required = get(&input, "requiredCapabilities").unwrap_or(&undefined);
    let Some(required) = required.as_array() else {
        return Err(if required.is_null() {
            "null is not an object (evaluating 'input.requiredCapabilities.filter')".to_string()
        } else {
            "undefined is not an object (evaluating 'input.requiredCapabilities.filter')"
                .to_string()
        });
    };
    let missing: Vec<String> = required
        .iter()
        .filter(|cap| match cap.as_str() {
            Some(name) => !WS_SERVER_CAPABILITIES.contains(&name),
            None => true,
        })
        .map(js_to_string)
        .collect();
    if !missing.is_empty() {
        return Ok(fail_effect(format!(
            "The server is missing required WebSocket capabilities: {}.",
            missing.join(", ")
        )));
    }
    Ok(succeed_effect(json!({
        "protocolEpoch": WS_PROTOCOL_EPOCH,
        "negotiatedRevision": negotiated,
        "serverBuild": SERVER_BUILD,
        "serverInstanceId": SERVER_INSTANCE_ID,
        "capabilities": WS_SERVER_CAPABILITIES,
    })))
}

/// Port of `parseWsNegotiateSearchParams` (`apps/server/src/wsCompatibility.ts:102`).
/// A JSON value never has `.get`, so only the TypeError paths are reachable.
pub fn parse_ws_negotiate_search_params(
    search_params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if search_params.is_null() {
        return Err("null is not an object (evaluating 'searchParams.get')".to_string());
    }
    if crate::js::is_undefined(&search_params) {
        return Err("undefined is not an object (evaluating 'searchParams.get')".to_string());
    }
    Err(
        "searchParams.get is not a function. (In 'searchParams.get(WS_NEGOTIATE_QUERY.protocolEpoch)', 'searchParams.get' is undefined)"
            .to_string(),
    )
}

/// Port of `validateWsFeatureCompatibility` (`apps/server/src/wsCompatibility.ts:130`).
pub fn validate_ws_feature_compatibility(
    search_params: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if search_params.is_null() {
        return Err("null is not an object (evaluating 'searchParams.get')".to_string());
    }
    if crate::js::is_undefined(&search_params) {
        return Err("undefined is not an object (evaluating 'searchParams.get')".to_string());
    }
    Err(
        "searchParams.get is not a function. (In 'searchParams.get(WS_COMPATIBILITY_QUERY.protocolEpoch)', 'searchParams.get' is undefined)"
            .to_string(),
    )
}

/// Port of `makeCurrentWsFeatureCompatibilitySearchParams`
/// (`apps/server/src/wsCompatibility.ts:172`): the URLSearchParams it builds has
/// no own enumerable properties, so it serializes as `{}`.
pub fn make_current_ws_feature_compatibility_search_params(
    client_build: String,
) -> Result<serde_json::Value, String> {
    let _ = &client_build;
    Ok(Value::Object(Map::new()))
}
