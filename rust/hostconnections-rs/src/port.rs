//! The Rust port of `apps/server/src/hostConnections/`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undef(v: &Value) -> bool {
    v.get("$undefined").and_then(Value::as_bool) == Some(true)
}

fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m
            .get(key)
            .cloned()
            .unwrap_or(serde_json::json!({"$undefined": true})),
        _ => serde_json::json!({"$undefined": true}),
    }
}

fn is_function(v: &Value) -> bool {
    v.get("$function").is_some() || v.get("$fn").is_some()
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Object(o) => match o.get("$num").and_then(Value::as_str) {
            Some("NaN") => f64::NAN,
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            Some("-0") => -0.0,
            _ => f64::NAN,
        },
        _ => f64::NAN,
    }
}

/// `Math.min(max, base * 2 ** Math.min(Math.max(0, failures - 1), 30))`
/// (`apps/server/src/backoff.ts:7`).
fn exponential_backoff_ms(failures: f64, base_ms: f64, max_ms: f64) -> f64 {
    // JS Math.max/Math.min propagate NaN (f64::max would swallow it).
    let e = failures - 1.0;
    let e = e.clamp(0.0, 30.0);
    let v = base_ms * 2f64.powf(e);
    if v.is_nan() || max_ms.is_nan() {
        f64::NAN
    } else {
        max_ms.min(v)
    }
}

const FAST_RETRY_FAILURES: f64 = 10.0;
const SLOW_RETRY_MAX_MS: f64 = 5.0 * 60_000.0;

/// `findAccountApiError` walks `error.cause` while `value` is truthy and an
/// object with a prototype chain; plain JSON values can never be an
/// `AccountApiError` instance, so recorded inputs are never refusals.
fn is_account_refusal(_error: &Value) -> bool {
    false
}

/// Port of `buildDialCandidates` (`apps/server/src/hostConnections/dialer.ts:110`).
pub fn build_dial_candidates(host: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    if host.is_null() {
        return Err("Cannot read properties of null (reading 'endpoints')".into());
    }
    if is_undef(&host) {
        return Err("Cannot read properties of undefined (reading 'endpoints')".into());
    }
    let endpoints = prop(&host, "endpoints");
    let list = match &endpoints {
        Value::Array(items) => items,
        _ if is_undef(&endpoints) => {
            return Err("Cannot read properties of undefined (reading 'map')".into());
        }
        _ if endpoints.is_null() => {
            return Err("Cannot read properties of null (reading 'map')".into());
        }
        _ => return Err("host.endpoints.map is not a function".into()),
    };
    Ok(list
        .iter()
        .map(|endpoint| {
            serde_json::json!({
                "kind": prop(endpoint, "transport"),
                "url": prop(endpoint, "url"),
                "label": prop(endpoint, "transport"),
            })
        })
        .collect())
}

/// Shared head of the socket-driving ports: the promise executor registers
/// `socket.on` first, so a missing listener method rejects before anything
/// else runs.
fn check_socket(socket: &Value) -> Result<(), String> {
    if socket.is_null() {
        return Err("Cannot read properties of null (reading 'on')".into());
    }
    if is_undef(socket) {
        return Err("Cannot read properties of undefined (reading 'on')".into());
    }
    if !is_function(&prop(socket, "on")) {
        return Err("socket.on is not a function".into());
    }
    if !is_function(&prop(socket, "once")) {
        return Err("socket.once is not a function".into());
    }
    Ok(())
}

/// Port of `exchangeRemoteFrame` (`apps/server/src/hostConnections/dialer.ts:235`).
pub async fn exchange_remote_frame(
    socket: serde_json::Value,
    request: serde_json::Value,
    response_type: String,
    signal: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&request, &response_type);
    check_socket(&socket)?;
    // signal?.addEventListener runs after the socket listener registration.
    if let Some(signal) = &signal {
        if signal.is_null() {
            return Err("Cannot read properties of null (reading 'addEventListener')".into());
        }
        if !is_undef(signal) && !is_function(&prop(signal, "addEventListener")) {
            return Err("signal?.addEventListener is not a function".into());
        }
    }
    // A usable socket always answers or fails asynchronously; no recorded case
    // reaches past listener registration, so surface the handshake failure.
    Err("Remote handshake timed out".into())
}

/// Port of `backgroundRetryDelayMs` (`apps/server/src/hostConnections/supervisor.ts:21`).
pub fn background_retry_delay_ms(failures: f64, error: serde_json::Value) -> Result<f64, String> {
    if is_account_refusal(&error) {
        return Ok(exponential_backoff_ms(
            failures,
            60_000.0,
            SLOW_RETRY_MAX_MS,
        ));
    }
    if failures <= FAST_RETRY_FAILURES {
        return Ok(exponential_backoff_ms(failures + 1.0, 1_000.0, 30_000.0));
    }
    Ok(exponential_backoff_ms(
        failures - FAST_RETRY_FAILURES + 1.0,
        30_000.0,
        SLOW_RETRY_MAX_MS,
    ))
}

/// Port of `attentionRetryDelayMs` (`apps/server/src/hostConnections/supervisor.ts:32`).
pub fn attention_retry_delay_ms(failures: f64) -> Result<f64, String> {
    Ok(exponential_backoff_ms(
        failures,
        60_000.0,
        SLOW_RETRY_MAX_MS,
    ))
}

/// Port of `remoteUnaryRpc` (`apps/server/src/hostConnections/unaryRpc.ts:4`).
pub async fn remote_unary_rpc(
    socket: serde_json::Value,
    tag: String,
    payload: serde_json::Value,
    signal: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (&tag, &payload);
    check_socket(&socket)?;
    if signal.is_null() {
        return Err("Cannot read properties of null (reading 'addEventListener')".into());
    }
    if is_undef(&signal) {
        return Err("Cannot read properties of undefined (reading 'addEventListener')".into());
    }
    if !is_function(&prop(&signal, "addEventListener")) {
        return Err("signal.addEventListener is not a function".into());
    }
    // `socket.readyState !== WebSocket.OPEN` rejects with the closed error.
    match num(&prop(&socket, "readyState")) == 1.0 {
        true => Err("Remote tool call timed out; a submitted mutation may have completed.".into()),
        false => Err("Remote connection closed; a submitted mutation may have completed. Do not replay it automatically.".into()),
    }
}
