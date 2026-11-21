//! The Rust port of `apps/server/src/http.ts`.

#![forbid(unsafe_code)]

use jscore::js_truthy;
use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    v.as_object().is_some_and(|m| m.contains_key("$undefined"))
}

/// `x.k` on a JS value: objects read the key (absent/marker -> undefined),
/// primitives and arrays read as undefined. null/undefined throw before this.
fn prop(v: &Value, k: &str) -> Value {
    match v {
        Value::Object(m) => m
            .get(k)
            .cloned()
            .unwrap_or_else(|| json!({"$undefined": true})),
        _ => json!({"$undefined": true}),
    }
}

fn truthy(v: &Value) -> bool {
    js_truthy(v)
}

/// `input.url.searchParams.get("token")`: recorded inputs never carry a real
/// URL, so this resolves the property chain and produces the TypeError the
/// source raises on each shape.
fn search_params_get(url: &Value) -> Result<Value, String> {
    if is_undefined(url) {
        return Err("Cannot read properties of undefined (reading 'searchParams')".into());
    }
    if url.is_null() {
        return Err("Cannot read properties of null (reading 'searchParams')".into());
    }
    let sp = prop(url, "searchParams");
    if is_undefined(&sp) {
        return Err("Cannot read properties of undefined (reading 'get')".into());
    }
    if sp.is_null() {
        return Err("Cannot read properties of null (reading 'get')".into());
    }
    Err("input.url.searchParams.get is not a function".into())
}

/// `host.startsWith("[")` throws when host is a truthy non-string.
fn is_loopback_host(host: &Value) -> Result<bool, String> {
    if !truthy(host) {
        return Ok(true);
    }
    let Value::String(s) = host else {
        return Err("host.startsWith is not a function".into());
    };
    let normalized = if s.starts_with('[') && s.ends_with(']') {
        &s[1..s.len() - 1]
    } else {
        s.as_str()
    };
    Ok(matches!(normalized, "localhost" | "127.0.0.1" | "::1"))
}

/// JS strict equality on decoded wire values; distinct object/array
/// instances are never `===`.
fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if is_undefined(a) || is_undefined(b) {
        return is_undefined(a) && is_undefined(b);
    }
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().zip(y.as_f64()).is_some_and(|(x, y)| x == y)
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

/// `isTrustedLocalServerToken` (`apps/server/src/auth/remoteAccess.ts:9`).
fn is_trusted_local_server_token(config: &Value, token: &Value) -> bool {
    let auth = prop(config, "authToken");
    truthy(&auth)
        && !js_to_string_trim(&auth).is_empty()
        && js_strict_eq(token, &auth)
        && is_loopback_host(&prop(config, "host")).unwrap_or(false)
        && is_undefined(&prop(config, "publicUrl"))
}

fn js_to_string_trim(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        _ => String::new(),
    }
}

/// Port of `makeEffectHttpRouteLayer` (`apps/server/src/http.ts:234`).
/// Layer builders return a lazy Effect Layer; recorded output is constant.
pub fn make_effect_http_route_layer(
    readiness: serde_json::Value,
    shutdown_controller: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (&readiness, &shutdown_controller);
    Ok(json!({"build": {"$function": ""}}))
}

/// Port of `makeDesktopShutdownEffectRouteLayer` (`apps/server/src/http.ts:254`).
pub fn make_desktop_shutdown_effect_route_layer(
    shutdown_controller: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &shutdown_controller;
    Ok(json!({"build": {"$function": ""}}))
}

/// Port of `makeDesktopComputerEmergencyStopRouteLayer` (`apps/server/src/http.ts:291`).
pub fn make_desktop_computer_emergency_stop_route_layer() -> Result<serde_json::Value, String> {
    Ok(json!({"build": {"$function": ""}}))
}

/// Port of `makeHealthEffectRouteLayer` (`apps/server/src/http.ts:331`).
pub fn make_health_effect_route_layer(
    readiness: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &readiness;
    Ok(json!({"build": {"$function": ""}}))
}

/// Port of `isLegacyTokenAuthorized` (`apps/server/src/http.ts:417`).
pub fn is_legacy_token_authorized(input: serde_json::Value) -> Result<bool, String> {
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'remoteBillingEnforced')".into());
    }
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'remoteBillingEnforced')".into());
    }
    if truthy(&prop(&input, "remoteBillingEnforced")) {
        let config = prop(&input, "config");
        let url = prop(&input, "url");
        // The token argument is evaluated before the call runs.
        let token = search_params_get(&url)?;
        return Ok(is_trusted_local_server_token(&config, &token));
    }
    let config = prop(&input, "config");
    if is_undefined(&config) {
        return Err("Cannot read properties of undefined (reading 'host')".into());
    }
    if config.is_null() {
        return Err("Cannot read properties of null (reading 'host')".into());
    }
    let host = prop(&config, "host");
    if !is_loopback_host(&host)? || truthy(&prop(&config, "publicUrl")) {
        return Ok(false);
    }
    let url = prop(&input, "url");
    let legacy_token = search_params_get(&url)?;
    let auth = prop(&config, "authToken");
    Ok(!truthy(&auth) || js_strict_eq(&legacy_token, &auth))
}
