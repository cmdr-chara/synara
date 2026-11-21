//! The Rust port of `apps/server/src/hostConnectivitySupervisor.ts`.

#![forbid(unsafe_code)]

use jscore::{js_truthy, json_stringify};
use serde_json::{Value, json};

fn undefined() -> Value {
    json!({"$undefined": true})
}

fn is_undefined(v: &Value) -> bool {
    v.as_object().is_some_and(|m| m.contains_key("$undefined"))
}

/// Decode wire markers to plain JSON for `JSON.stringify` output.
fn decode(v: &Value) -> Value {
    if let Value::Object(m) = v {
        if m.contains_key("$undefined") {
            // JSON.stringify turns undefined array elements into null.
            return Value::Null;
        }
        if let Some(n) = m.get("$num").and_then(Value::as_str) {
            let f = match n {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                "-0" => -0.0,
                _ => n.parse().unwrap_or(f64::NAN),
            };
            return Value::from(f); // non-finite -> null
        }
        if let Some(units) = m.get("$utf16").and_then(Value::as_array) {
            let units: Vec<u16> = units
                .iter()
                .filter_map(|u| u.as_u64().map(|u| u as u16))
                .collect();
            return Value::String(String::from_utf16_lossy(&units));
        }
    }
    v.clone()
}

/// JS strict equality `a === b` on decoded wire values; objects/arrays only
/// compare equal by reference, so two Value instances are never `===`.
fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (decode(a), decode(b)) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().zip(y.as_f64()).is_some_and(|(x, y)| x == y)
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

/// Port of `hostLinkKey` (`apps/server/src/hostConnectivitySupervisor.ts:26`).
pub fn host_link_key(file: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    let Some(Value::Object(file)) = file else {
        return Ok(undefined());
    };
    let host_id = file.get("hostId").cloned().unwrap_or_else(undefined);
    let owner = file
        .get("hostOwnerUserId")
        .cloned()
        .unwrap_or_else(undefined);
    let generation = file
        .get("hostKeyGeneration")
        .cloned()
        .unwrap_or_else(undefined);
    let user_id = file.get("userId").cloned().unwrap_or_else(undefined);
    if !js_truthy(&host_id) || !js_truthy(&owner) || is_undefined(&generation) {
        return Ok(undefined());
    }
    if !is_undefined(&user_id) && !js_strict_eq(&user_id, &owner) {
        return Ok(undefined());
    }
    let org = file
        .get("organizationId")
        .cloned()
        .unwrap_or_else(undefined);
    let org = if is_undefined(&org) || org.is_null() {
        Value::Null
    } else {
        decode(&org)
    };
    let row = json!([
        decode(file.get("accountUrl").unwrap_or(&Value::Null)),
        decode(&host_id),
        decode(&owner),
        decode(&generation),
        org,
    ]);
    Ok(Value::String(json_stringify(&row)))
}
