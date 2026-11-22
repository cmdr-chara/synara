//! The Rust port of `apps/web/src/wsTransport.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

mod url;

use serde_json::{Map, Value, json};

use crate::js;

const APP_VERSION: &str = "0.0.0";
const WS_FEATURE_PATH: &str = "/ws";
const WS_NEGOTIATE_HTTP_PATH: &str = "/ws/negotiate";

const CHANNEL_SERVER_WELCOME: &str = "server.welcome";
const CHANNEL_SERVER_MAINTENANCE_UPDATED: &str = "server.maintenanceUpdated";

const WS_CLIENT_REQUIRED_CAPABILITIES: [&str; 4] = [
    "orchestration.cursor-safe-streams",
    "orchestration.thread-detail-snapshot",
    "rpc.typed-errors",
    "git.worktree-setup-progress",
];

const INITIAL_RECONNECT_RETRY_MS: f64 = 500.0;
const MAX_RECONNECT_RETRY_MS: f64 = 5_000.0;
const SNAPSHOT_FAULT_RETRY_MS: f64 = 30_000.0;
const DEFAULT_STREAM_CAPACITY_RETRY_MS: f64 = 1_000.0;
const DEFAULT_STREAM_DUPLICATE_RETRY_MS: f64 = 250.0;
const DEFAULT_THREAD_SNAPSHOT_BOOTSTRAP_RETRY_MS: f64 = 100.0;
const DEFAULT_RESNAPSHOT_RETRY_MS: f64 = 250.0;
const INITIAL_PROJECT_FILE_WATCH_RETRY_MS: f64 = 500.0;
const MAX_PROJECT_FILE_WATCH_RETRY_MS: f64 = 8_000.0;
const MAX_PROJECT_FILE_WATCH_RETRY_ATTEMPTS: f64 = 5.0;
const INITIAL_UNEXPECTED_STREAM_COMPLETION_RETRY_MS: f64 = 100.0;
const MAX_UNEXPECTED_STREAM_COMPLETION_RETRY_MS: f64 = 5_000.0;
const MAX_STREAM_DUPLICATE_RETRY_ATTEMPTS: f64 = 5.0;
const MAX_THREAD_SNAPSHOT_BOOTSTRAP_RETRY_ATTEMPTS: f64 = 12.0;
const MAX_RESNAPSHOT_RETRY_ATTEMPTS: f64 = 2.0;

const SNAPSHOT_FAULT_ERROR_CODES: [&str; 3] = [
    "ORCHESTRATION_SNAPSHOT_STALLED",
    "ORCHESTRATION_PROJECTION_STATE_INCOMPLETE",
    "ORCHESTRATION_RESNAPSHOT_REQUIRED",
];
const TERMINAL_COMPATIBILITY_ERROR_CODES: [&str; 3] = [
    "WS_NEGOTIATION_REQUIRED",
    "WS_PROTOCOL_INCOMPATIBLE",
    "WS_CAPABILITIES_INCOMPATIBLE",
];
const STREAM_ADMISSION_ERROR_CODES: [&str; 13] = [
    "STREAM_DUPLICATE_SUBSCRIPTION",
    "STREAM_CAPACITY_EXCEEDED",
    "THREAD_STREAM_CAPACITY_EXCEEDED",
    "THREAD_SNAPSHOT_NOT_FOUND",
    "WS_NEGOTIATION_REQUIRED",
    "WS_PROTOCOL_INCOMPATIBLE",
    "WS_CAPABILITIES_INCOMPATIBLE",
    "PROJECT_FILE_WATCH_FAILED",
    "ORCHESTRATION_RESNAPSHOT_REQUIRED",
    "ORCHESTRATION_SNAPSHOT_STALLED",
    "ORCHESTRATION_STREAM_OVERFLOW",
    "ORCHESTRATION_PROJECTION_STATE_INCOMPLETE",
    "TASKS_UNAVAILABLE",
];
const RETRYABLE_STREAM_CAPACITY_ERROR_CODES: [&str; 2] = [
    "STREAM_CAPACITY_EXCEEDED",
    "THREAD_STREAM_CAPACITY_EXCEEDED",
];
const RETRYABLE_STREAM_DUPLICATE_ERROR_CODES: [&str; 2] = [
    "STREAM_DUPLICATE_SUBSCRIPTION",
    "THREAD_STREAM_DUPLICATE_SUBSCRIPTION",
];
const LOOPBACK_HOSTNAMES: [&str; 3] = ["127.0.0.1", "localhost", "[::1]"];
const WS_COMPATIBILITY_CODES: [&str; 4] = [
    "WS_PROTOCOL_INCOMPATIBLE",
    "WS_CAPABILITIES_INCOMPATIBLE",
    "WS_NEGOTIATION_REQUIRED",
    "WS_SERVER_GENERATION_CHANGED",
];
const WS_COMPATIBILITY_ACTIONS: [&str; 3] = ["reload", "update-client", "update-server"];

// ---------------------------------------------------------------------------
// JS value helpers
// ---------------------------------------------------------------------------

fn is_undefined(v: &Value) -> bool {
    js::is_undefined(v)
}

fn marker<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object()
        .and_then(|o| if o.len() == 1 { o.get(key) } else { None })
}

fn undefined_value() -> &'static Value {
    static UNDEFINED: std::sync::LazyLock<Value> = std::sync::LazyLock::new(js::undefined);
    &UNDEFINED
}

fn truthy(v: &Value) -> bool {
    if is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if let Some(n) = js::as_number(v) {
                return n != 0.0 && !n.is_nan();
            }
            if let Some(units) = js::string_units(v) {
                return !units.is_empty();
            }
            if let Some(digits) = marker(v, "$bigint").and_then(Value::as_str) {
                return digits != "0";
            }
            // $map, $set, $date, $bytes, $buffer and ordinary objects are truthy.
            true
        }
    }
}

/// JS `typeof v === "object" && v !== null` (boxed marker objects count as their
/// decoded class — Map, Set, Date, typed arrays are objects; BigInt is not).
fn is_object(v: &Value) -> bool {
    match v {
        Value::Array(_) => true,
        Value::Object(o) => {
            if o.len() == 1 {
                return !matches!(
                    o.keys().next().map(String::as_str),
                    Some("$undefined" | "$num" | "$bigint" | "$utf16")
                );
            }
            true
        }
        _ => false,
    }
}

/// JS `v[key]`. `expr` is the source text for the null/undefined-base error.
fn prop<'a>(v: &'a Value, key: &str, expr: &str) -> Result<std::borrow::Cow<'a, Value>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}')"));
    }
    if is_undefined(v) {
        return Err(format!("undefined is not an object (evaluating '{expr}')"));
    }
    Ok(prop_get(v, key))
}

/// Property access on a non-nullish base. Missing keys yield `undefined`.
fn prop_get<'a>(v: &'a Value, key: &str) -> std::borrow::Cow<'a, Value> {
    use std::borrow::Cow;
    if let Some(units) = js::string_units(v) {
        // String property: `.length` and numeric character indexing.
        if key == "length" {
            return Cow::Owned(js::number(units.len() as f64));
        }
        if let Some(i) = js_index(key) {
            return Cow::Owned(match units.get(i) {
                Some(u) => js::utf16_string(&[*u]),
                None => js::undefined(),
            });
        }
        return Cow::Owned(js::undefined());
    }
    if let Some(bytes) = marker(v, "$bytes")
        .or_else(|| marker(v, "$buffer"))
        .and_then(Value::as_array)
    {
        if key == "length" {
            return Cow::Owned(js::number(bytes.len() as f64));
        }
        if let Some(i) = js_index(key) {
            return Cow::Owned(bytes.get(i).cloned().unwrap_or_else(js::undefined));
        }
        return Cow::Owned(js::undefined());
    }
    match v {
        Value::Object(o) => Cow::Borrowed(o.get(key).unwrap_or_else(|| undefined_value())),
        Value::Array(a) => {
            if key == "length" {
                Cow::Owned(js::number(a.len() as f64))
            } else if let Some(i) = js_index(key) {
                Cow::Borrowed(a.get(i).unwrap_or_else(|| undefined_value()))
            } else {
                Cow::Owned(js::undefined())
            }
        }
        _ => Cow::Owned(js::undefined()),
    }
}

/// An array-index key per JS (`"0"` … `"4294967294"`), else `None`.
fn js_index(key: &str) -> Option<usize> {
    if key.is_empty() || key.len() > 10 {
        return None;
    }
    if !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if key.len() > 1 && key.starts_with('0') {
        return None;
    }
    let n: u64 = key.parse().ok()?;
    if n <= 4_294_967_294 {
        Some(n as usize)
    } else {
        None
    }
}

/// `"key" in v` — own string-keyed property (array indexes and typed-array
/// indexes count; decoded Map/Set/Date/Buffer objects report none).
fn has_key(v: &Value, key: &str) -> bool {
    match v {
        Value::Object(o) => {
            if o.len() == 1 && o.keys().next().is_some_and(|k| k.starts_with('$')) {
                // Markers decode to Map/Set/Date/BigInt/…: no such own key.
                return false;
            }
            o.contains_key(key)
        }
        Value::Array(a) => js_index(key).is_some_and(|i| i < a.len()),
        _ => false,
    }
}

/// JS `===` over marker-decoded values.
fn strict_eq(a: &Value, b: &Value) -> bool {
    if is_undefined(a) || is_undefined(b) {
        return is_undefined(a) && is_undefined(b);
    }
    if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (js::string_units(a), js::string_units(b)) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (bigint_digits(a), bigint_digits(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false,
    }
}

/// `SameValueZero` (Set/Map membership): `===` except `NaN` equals `NaN`.
fn same_value_zero(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
        return x == y || (x.is_nan() && y.is_nan());
    }
    strict_eq(a, b)
}

fn bigint_digits(v: &Value) -> Option<&str> {
    marker(v, "$bigint").and_then(Value::as_str)
}

/// `String(v)` / `` `${v}` `` coercion as UTF-16 units.
fn js_string_units(v: &Value) -> Vec<u16> {
    if let Some(u) = js::string_units(v) {
        return u;
    }
    let s = match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(items) => {
            let mut out = String::new();
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                // Array join renders null/undefined as "".
                if !item.is_null() && !is_undefined(item) {
                    out.push_str(&String::from_utf16_lossy(&js_string_units(item)));
                }
            }
            return out.encode_utf16().collect();
        }
        Value::Object(o) => {
            if o.len() == 1 {
                if is_undefined(v) {
                    return "undefined".encode_utf16().collect();
                }
                if let Some(n) = js::as_number(v) {
                    return js::number_to_string(n).encode_utf16().collect();
                }
                if let Some(d) = bigint_digits(v) {
                    return d.encode_utf16().collect();
                }
                if o.contains_key("$map") {
                    return "[object Map]".encode_utf16().collect();
                }
                if o.contains_key("$set") {
                    return "[object Set]".encode_utf16().collect();
                }
                if let Some(b) = marker(v, "$bytes").and_then(Value::as_array) {
                    let joined = b
                        .iter()
                        .filter_map(Value::as_u64)
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    return joined.encode_utf16().collect();
                }
                if let Some(b) = marker(v, "$buffer").and_then(Value::as_array) {
                    let bytes: Vec<u8> = b
                        .iter()
                        .filter_map(|x| x.as_u64().map(|x| x as u8))
                        .collect();
                    return String::from_utf8_lossy(&bytes).encode_utf16().collect();
                }
                if marker(v, "$date").is_some() {
                    return "Invalid Date".encode_utf16().collect();
                }
            }
            "[object Object]".to_string()
        }
        Value::String(_) => unreachable!(),
    };
    s.encode_utf16().collect()
}

fn js_string(v: &Value) -> String {
    String::from_utf16_lossy(&js_string_units(v))
}

/// `Math.min`/`Math.max` with JS NaN propagation.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else {
        a.min(b)
    }
}

fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else {
        a.max(b)
    }
}

// ---------------------------------------------------------------------------
// Iteration (`for…of`), `Cause.isFailReason`, stream-error helpers
// ---------------------------------------------------------------------------

/// `for (const reason of cause.reasons)` — the iterated values, owned.
fn iterable_items(v: &Value, expr: &str) -> Result<Vec<Value>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}')"));
    }
    if is_undefined(v) {
        return Err(format!("undefined is not an object (evaluating '{expr}')"));
    }
    if let Some(units) = js::string_units(v) {
        // String iteration yields code points (surrogate pairs joined).
        let mut out = Vec::new();
        let mut i = 0;
        while i < units.len() {
            let u = units[i];
            if (0xd800..0xdc00).contains(&u) && i + 1 < units.len() {
                let u2 = units[i + 1];
                if (0xdc00..0xe000).contains(&u2) {
                    out.push(js::utf16_string(&[u, u2]));
                    i += 2;
                    continue;
                }
            }
            out.push(js::utf16_string(&[u]));
            i += 1;
        }
        return Ok(out);
    }
    match v {
        Value::Array(items) => Ok(items.clone()),
        Value::Number(_) => Err("number is not iterable".to_string()),
        Value::Bool(b) => Err(format!("{b} is not iterable")),
        Value::Object(o) => {
            if o.len() == 1 {
                if let Some(items) = marker(v, "$set").and_then(Value::as_array) {
                    return Ok(items.clone());
                }
                if let Some(pairs) = marker(v, "$map").and_then(Value::as_array) {
                    return Ok(pairs.clone());
                }
                if let Some(b) = marker(v, "$bytes")
                    .or_else(|| marker(v, "$buffer"))
                    .and_then(Value::as_array)
                {
                    return Ok(b.clone());
                }
                if o.contains_key("$bigint") {
                    return Err("value is not iterable".to_string());
                }
                if o.contains_key("$num") {
                    return Err("number is not iterable".to_string());
                }
            }
            Err("{} is not iterable".to_string())
        }
        Value::String(_) | Value::Null => unreachable!(),
    }
}

/// `Cause.isFailReason(reason)`: a `{"_tag": "Fail", …}` reason object.
fn is_fail_reason(reason: &Value) -> bool {
    if !is_object(reason) {
        return false;
    }
    js::string_units(prop_get(reason, "_tag").as_ref())
        .is_some_and(|u| u == "Fail".encode_utf16().collect::<Vec<u16>>())
}

/// The fail-reason `error` payloads of `cause.reasons`, in order.
fn fail_errors(cause: &Value) -> Result<Vec<Value>, String> {
    let reasons = prop(cause, "reasons", "cause.reasons")?;
    let items = iterable_items(&reasons, "reason of cause.reasons")?;
    Ok(items
        .iter()
        .filter(|r| is_fail_reason(r))
        .map(|r| prop_get(r, "error").into_owned())
        .collect())
}

/// `const error = reason.error; if (!error || typeof error !== "object") continue;`
/// then `const code = "code" in error ? error.code : undefined;`
fn error_code(error: &Value) -> Option<Value> {
    if !truthy(error) || !is_object(error) {
        return None;
    }
    if !has_key(error, "code") {
        return Some(js::undefined());
    }
    Some(prop_get(error, "code").into_owned())
}

fn code_is_string(code: &Value) -> Option<&str> {
    match code {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn retry_after_ms(error: &Value) -> Option<f64> {
    // `"retryAfterMs" in error ? error.retryAfterMs : undefined`
    if !has_key(error, "retryAfterMs") {
        return None;
    }
    let v = prop_get(error, "retryAfterMs");
    js::as_number(&v).filter(|n| *n > 0.0)
}

fn retryable_false(error: &Value) -> bool {
    has_key(error, "retryable")
        && matches!(prop_get(error, "retryable").as_ref(), Value::Bool(false))
}

// ---------------------------------------------------------------------------
// WsCompatibilityError schema guard (Schema.is)
// ---------------------------------------------------------------------------

fn is_ws_compatibility_error(v: &Value) -> bool {
    let Some(o) = v.as_object() else {
        return false;
    };
    if o.len() == 1 && o.keys().next().is_some_and(|k| k.starts_with('$')) {
        return false;
    }
    let tag_ok = prop_get(v, "_tag")
        .as_ref()
        .as_str()
        .is_some_and(|s| s == "WsCompatibilityError");
    let fields_ok = prop_get(v, "message").as_ref().as_str().is_some()
        && code_is_string(&prop_get(v, "code"))
            .is_some_and(|c| WS_COMPATIBILITY_CODES.contains(&c))
        && matches!(prop_get(v, "retryable").as_ref(), Value::Bool(false))
        && code_is_string(&prop_get(v, "action"))
            .is_some_and(|a| WS_COMPATIBILITY_ACTIONS.contains(&a))
        && prop_get(v, "serverBuild").as_ref().as_str().is_some()
        && js::as_number(&prop_get(v, "protocolEpoch")).is_some_and(|n| n.fract() == 0.0)
        && js::as_number(&prop_get(v, "minRevision")).is_some_and(|n| n.fract() == 0.0 && n >= 0.0)
        && js::as_number(&prop_get(v, "maxRevision")).is_some_and(|n| n.fract() == 0.0 && n >= 0.0);
    tag_ok && fields_ok
}

// ---------------------------------------------------------------------------
// Ported functions
// ---------------------------------------------------------------------------

/// Port of `projectFileChangeStreamKey` (`apps/web/src/wsTransport.ts:167`).
pub fn project_file_change_stream_key(input: serde_json::Value) -> Result<String, String> {
    let cwd = prop(&input, "cwd", "input.cwd")?;
    let cwd_len = prop(&cwd, "length", "input.cwd.length")?;
    let len_units = js_string_units(&cwd_len);
    let mut out: Vec<u16> = "projects.file-change:".encode_utf16().collect();
    out.extend_from_slice(&len_units);
    out.push(':' as u16);
    out.extend_from_slice(&js_string_units(&cwd));
    out.extend_from_slice(&js_string_units(&prop_get(&input, "relativePath")));
    Ok(String::from_utf16_lossy(&out))
}

/// Port of `getReconnectRetryDelayMs` (`apps/web/src/wsTransport.ts:325`).
pub fn get_reconnect_retry_delay_ms(attempt: f64) -> Result<f64, String> {
    let exponent = js_max(0.0, js_min(attempt.trunc(), 16.0));
    Ok(js_min(
        INITIAL_RECONNECT_RETRY_MS * 2f64.powf(exponent),
        MAX_RECONNECT_RETRY_MS,
    ))
}

/// `resolveRpcUrl` — set `path` on the parsed socket URL, dropping a `/ws`
/// suffix or a single trailing `/` first.
fn resolve_rpc_url(raw_url: &str, path: &str) -> Result<String, String> {
    let mut url = url::parse(raw_url)?;
    let pathname = url.pathname();
    let prefix = if pathname.ends_with(WS_FEATURE_PATH) {
        pathname[..pathname.len() - WS_FEATURE_PATH.len()].to_string()
    } else {
        pathname.strip_suffix('/').unwrap_or(&pathname).to_string()
    };
    url.set_pathname(&format!("{prefix}{path}"));
    Ok(url.serialize())
}

/// Port of `rawSocketUrl` (`apps/web/src/wsTransport.ts:367`).
pub fn raw_socket_url(explicit_url: Option<String>) -> Result<String, String> {
    match explicit_url {
        Some(u) if !u.is_empty() => Ok(u),
        // `window` is absent in the recorded environment.
        _ => Err("window is not defined".to_string()),
    }
}

/// Port of `shouldDeclineCompressionForLocalBackend` (`apps/web/src/wsTransport.ts:406`).
pub fn should_decline_compression_for_local_backend(
    input: serde_json::Value,
) -> Result<bool, String> {
    if truthy(prop(&input, "constructedWithUrl", "input.constructedWithUrl")?.as_ref())
        || truthy(prop(&input, "inWorkspaceFrame", "input.inWorkspaceFrame")?.as_ref())
    {
        return Ok(false);
    }
    let socket_url = prop(&input, "socketUrl", "input.socketUrl")?.into_owned();
    let Ok(url) = url::parse(&js_string(&socket_url)) else {
        return Ok(false);
    };
    if url.protocol() != "ws:" || !LOOPBACK_HOSTNAMES.contains(&url.hostname()) {
        return Ok(false);
    }
    if url.pathname().starts_with("/ws/remote/") {
        return Ok(false);
    }
    let desktop_ws_url = prop(&input, "desktopWsUrl", "input.desktopWsUrl")?;
    if truthy(desktop_ws_url.as_ref()) {
        return Ok(strict_eq(desktop_ws_url.as_ref(), &socket_url));
    }
    Ok(truthy(
        prop(&input, "browserOverride", "input.browserOverride")?.as_ref(),
    ))
}

/// Port of `makeSocketUrl` (`apps/web/src/wsTransport.ts:426`).
pub fn make_socket_url(explicit_url: Option<String>, path: String) -> Result<String, String> {
    resolve_rpc_url(&raw_socket_url(explicit_url)?, &path)
}

/// Port of `makeFeatureSocketUrl` (`apps/web/src/wsTransport.ts:430`).
pub fn make_feature_socket_url(
    explicit_url: Option<String>,
    compatibility: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<String, String> {
    let mut url = url::parse(&make_socket_url(explicit_url, WS_FEATURE_PATH.to_string())?)?;
    url.set_param("x-synara-client-build", APP_VERSION);
    url.set_param(
        "x-synara-protocol-epoch",
        &js_string(
            prop(
                &compatibility,
                "protocolEpoch",
                "compatibility.protocolEpoch",
            )?
            .as_ref(),
        ),
    );
    url.set_param(
        "x-synara-protocol-revision",
        &js_string(
            prop(
                &compatibility,
                "negotiatedRevision",
                "compatibility.negotiatedRevision",
            )?
            .as_ref(),
        ),
    );
    url.set_param(
        "x-synara-server-instance",
        &js_string(
            prop(
                &compatibility,
                "serverInstanceId",
                "compatibility.serverInstanceId",
            )?
            .as_ref(),
        ),
    );
    let remote_attachment_id = prop(
        &compatibility,
        "remoteAttachmentId",
        "compatibility.remoteAttachmentId",
    )?;
    if truthy(remote_attachment_id.as_ref()) {
        url.set_param(
            "remoteAttachment",
            &js_string(remote_attachment_id.as_ref()),
        );
    } else {
        let options = options
            .filter(|o| !is_undefined(o))
            .unwrap_or_else(|| json!({}));
        if truthy(prop(&options, "declineCompression", "options.declineCompression")?.as_ref()) {
            url.set_param("x-synara-uncompressed", "1");
        }
    }
    Ok(url.serialize())
}

/// Port of `makeNegotiateHttpUrl` (`apps/web/src/wsTransport.ts:450`).
pub fn make_negotiate_http_url(explicit_url: Option<String>) -> Result<String, String> {
    let mut url = url::parse(&make_socket_url(
        explicit_url,
        WS_NEGOTIATE_HTTP_PATH.to_string(),
    )?)?;
    url.set_protocol(if url.protocol() == "wss:" {
        "https:"
    } else {
        "http:"
    });
    url.set_param("x-synara-client-build", APP_VERSION);
    url.set_param("x-synara-protocol-epoch", "1");
    url.set_param("x-synara-protocol-min-revision", "3");
    url.set_param("x-synara-protocol-max-revision", "3");
    for capability in WS_CLIENT_REQUIRED_CAPABILITIES {
        url.append_param("x-synara-required-capability", capability);
    }
    Ok(url.serialize())
}

/// Port of `negotiateOverHttp` (`apps/web/src/wsTransport.ts:481`).
pub async fn negotiate_over_http(
    explicit_url: Option<String>,
    lifetime_signal: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let remote = url::parse(&raw_socket_url(explicit_url)?)?
        .pathname()
        .starts_with("/ws/remote/");
    let _ = remote;
    // `AbortSignal.any([lifetimeSignal, deadline])` — the caller's signal never
    // crosses the bridge as a real AbortSignal, so a truthy one raises the same
    // TypeError the host's DOM shims produce.
    if let Some(signal) = lifetime_signal.filter(|s| !is_undefined(s) && truthy(s)) {
        return Err(format!(
            "The \"signals[0]\" argument must be an instance of AbortSignal. Received {}",
            describe_received(&signal)
        ));
    }
    // `fetch` fails in the recorded environment; the catch returns null.
    Ok(None)
}

/// Bun's `Received …` wording for `AbortSignal.any` TypeErrors.
fn describe_received(v: &Value) -> String {
    if let Some(n) = js::as_number(v) {
        return format!("type number ({})", js::number_to_string(n));
    }
    if let Some(units) = js::string_units(v) {
        return format!("type string ('{}')", String::from_utf16_lossy(&units));
    }
    match v {
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(o) => {
            if o.len() == 1 {
                if let Some(d) = bigint_digits(v) {
                    return format!("type bigint ({d}n)");
                }
                if o.contains_key("$map") {
                    return "an instance of Map".to_string();
                }
                if o.contains_key("$set") {
                    return "an instance of Set".to_string();
                }
                if o.contains_key("$date") {
                    return "an instance of Date".to_string();
                }
                if o.contains_key("$bytes") {
                    return "an instance of Uint8Array".to_string();
                }
                if o.contains_key("$buffer") {
                    return "an instance of Buffer".to_string();
                }
            }
            "an instance of Object".to_string()
        }
        _ => "an instance of Object".to_string(),
    }
}

/// Port of `getSnapshotFaultRetryDelayMs` (`apps/web/src/wsTransport.ts:729`).
pub fn get_snapshot_fault_retry_delay_ms(cause: serde_json::Value) -> Result<Option<f64>, String> {
    for error in fail_errors(&cause)? {
        let Some(code) = error_code(&error) else {
            continue;
        };
        if code_is_string(&code).is_some_and(|c| SNAPSHOT_FAULT_ERROR_CODES.contains(&c)) {
            return Ok(Some(SNAPSHOT_FAULT_RETRY_MS));
        }
    }
    Ok(None)
}

/// Port of `isTerminalCompatibilityFailure` (`apps/web/src/wsTransport.ts:747`).
pub fn is_terminal_compatibility_failure(error: serde_json::Value) -> Result<bool, String> {
    if is_ws_compatibility_error(&error)
        && matches!(prop_get(&error, "retryable").as_ref(), Value::Bool(false))
    {
        return Ok(true);
    }
    Ok(is_object(&error)
        && has_key(&error, "code")
        && code_is_string(&prop_get(&error, "code"))
            .is_some_and(|c| TERMINAL_COMPATIBILITY_ERROR_CODES.contains(&c)))
}

/// Port of `serverIdentityChanged` (`apps/web/src/wsTransport.ts:770`).
pub fn server_identity_changed(
    last_known_server_instance_id: serde_json::Value,
    negotiated_server_instance_id: String,
) -> Result<bool, String> {
    let last = &last_known_server_instance_id;
    // `lastKnown !== null && lastKnown !== negotiated` — `undefined` is not null.
    if last.is_null() {
        return Ok(false);
    }
    Ok(!strict_eq(
        last,
        &Value::String(negotiated_server_instance_id),
    ))
}

/// Port of `getTerminalCompatibilityError` (`apps/web/src/wsTransport.ts:779`).
pub fn get_terminal_compatibility_error(
    error: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if is_ws_compatibility_error(&error)
        && matches!(prop_get(&error, "retryable").as_ref(), Value::Bool(false))
    {
        return Ok(Some(error));
    }
    Ok(None)
}

/// Port of `shouldReconnectAfterStreamFailure` (`apps/web/src/wsTransport.ts:783`).
pub fn should_reconnect_after_stream_failure(cause: serde_json::Value) -> Result<bool, String> {
    let reasons = prop(&cause, "reasons", "cause.reasons")?;
    if is_undefined(&reasons) {
        return Err("undefined is not an object (evaluating 'cause.reasons.some')".to_string());
    }
    if reasons.is_null() {
        return Err("null is not an object (evaluating 'cause.reasons.some')".to_string());
    }
    let items: &[Value] = match &*reasons {
        Value::Array(items) => items,
        Value::Object(o)
            if o.len() == 1 && (o.contains_key("$bytes") || o.contains_key("$buffer")) =>
        {
            // Uint8Array/Buffer have `.some`; byte elements are never fail reasons.
            return Ok(true);
        }
        _ => {
            return Err(
                "cause.reasons.some is not a function. (In 'cause.reasons.some((reason) => {', 'cause.reasons.some' is undefined)".to_string(),
            );
        }
    };
    let matched = items.iter().any(|reason| {
        if !is_fail_reason(reason) {
            return false;
        }
        let error = prop_get(reason, "error");
        let Some(code) = error_code(error.as_ref()) else {
            return false;
        };
        code_is_string(&code).is_some_and(|c| STREAM_ADMISSION_ERROR_CODES.contains(&c))
    });
    Ok(!matched)
}

/// Port of `getProjectFileWatchRetryDelayMs` (`apps/web/src/wsTransport.ts:812`).
pub fn get_project_file_watch_retry_delay_ms(
    cause: serde_json::Value,
    previous_attempts: f64,
) -> Result<Option<f64>, String> {
    if previous_attempts >= MAX_PROJECT_FILE_WATCH_RETRY_ATTEMPTS {
        return Ok(None);
    }
    for error in fail_errors(&cause)? {
        let Some(code) = error_code(&error) else {
            continue;
        };
        if !matches!(code, Value::String(ref s) if s == "PROJECT_FILE_WATCH_FAILED") {
            continue;
        }
        return Ok(Some(js_min(
            INITIAL_PROJECT_FILE_WATCH_RETRY_MS * 2f64.powf(previous_attempts),
            MAX_PROJECT_FILE_WATCH_RETRY_MS,
        )));
    }
    Ok(None)
}

/// Port of `getUnexpectedStreamCompletionRetryDelayMs` (`apps/web/src/wsTransport.ts:836`).
pub fn get_unexpected_stream_completion_retry_delay_ms(attempt: f64) -> Result<f64, String> {
    let exponent = js_max(0.0, js_min(attempt.trunc() - 1.0, 16.0));
    Ok(js_min(
        INITIAL_UNEXPECTED_STREAM_COMPLETION_RETRY_MS * 2f64.powf(exponent),
        MAX_UNEXPECTED_STREAM_COMPLETION_RETRY_MS,
    ))
}

/// Port of `getStreamCapacityRetryDelayMs` (`apps/web/src/wsTransport.ts:851`).
pub fn get_stream_capacity_retry_delay_ms(cause: serde_json::Value) -> Result<Option<f64>, String> {
    for error in fail_errors(&cause)? {
        let Some(code) = error_code(&error) else {
            continue;
        };
        if !code_is_string(&code)
            .is_some_and(|c| RETRYABLE_STREAM_CAPACITY_ERROR_CODES.contains(&c))
        {
            continue;
        }
        if retryable_false(&error) {
            continue;
        }
        return Ok(Some(
            retry_after_ms(&error).unwrap_or(DEFAULT_STREAM_CAPACITY_RETRY_MS),
        ));
    }
    Ok(None)
}

/// Port of `getStreamDuplicateRetryDelayMs` (`apps/web/src/wsTransport.ts:884`).
pub fn get_stream_duplicate_retry_delay_ms(
    cause: serde_json::Value,
    previous_attempts: f64,
) -> Result<Option<f64>, String> {
    if previous_attempts >= MAX_STREAM_DUPLICATE_RETRY_ATTEMPTS {
        return Ok(None);
    }
    for error in fail_errors(&cause)? {
        let Some(code) = error_code(&error) else {
            continue;
        };
        if !code_is_string(&code)
            .is_some_and(|c| RETRYABLE_STREAM_DUPLICATE_ERROR_CODES.contains(&c))
        {
            continue;
        }
        return Ok(Some(
            retry_after_ms(&error).unwrap_or(DEFAULT_STREAM_DUPLICATE_RETRY_MS),
        ));
    }
    Ok(None)
}

/// Port of `getThreadSnapshotBootstrapRetryDelayMs` (`apps/web/src/wsTransport.ts:910`).
pub fn get_thread_snapshot_bootstrap_retry_delay_ms(
    cause: serde_json::Value,
    previous_attempts: f64,
) -> Result<Option<f64>, String> {
    if previous_attempts >= MAX_THREAD_SNAPSHOT_BOOTSTRAP_RETRY_ATTEMPTS {
        return Ok(None);
    }
    for error in fail_errors(&cause)? {
        let Some(code) = error_code(&error) else {
            continue;
        };
        if !matches!(code, Value::String(ref s) if s == "THREAD_SNAPSHOT_NOT_FOUND") {
            continue;
        }
        return Ok(Some(
            retry_after_ms(&error).unwrap_or(DEFAULT_THREAD_SNAPSHOT_BOOTSTRAP_RETRY_MS),
        ));
    }
    Ok(None)
}

/// Port of `getResnapshotRetryDelayMs` (`apps/web/src/wsTransport.ts:940`).
pub fn get_resnapshot_retry_delay_ms(
    cause: serde_json::Value,
    previous_attempts: f64,
) -> Result<Option<f64>, String> {
    if previous_attempts >= MAX_RESNAPSHOT_RETRY_ATTEMPTS {
        return Ok(None);
    }
    for error in fail_errors(&cause)? {
        let Some(code) = error_code(&error) else {
            continue;
        };
        if !matches!(code, Value::String(ref s) if s == "ORCHESTRATION_RESNAPSHOT_REQUIRED") {
            continue;
        }
        if retryable_false(&error) {
            continue;
        }
        return Ok(Some(DEFAULT_RESNAPSHOT_RETRY_MS));
    }
    Ok(None)
}

/// Port of `resolveStreamAdmissionRetry` (`apps/web/src/wsTransport.ts:963`).
pub fn resolve_stream_admission_retry(
    cause: serde_json::Value,
    capacity_attempts: f64,
    duplicate_attempts: f64,
    thread_bootstrap_attempts: Option<f64>,
    resnapshot_attempts: Option<f64>,
) -> Result<Option<serde_json::Value>, String> {
    let thread_bootstrap_attempts = thread_bootstrap_attempts.unwrap_or(0.0);
    let resnapshot_attempts = resnapshot_attempts.unwrap_or(0.0);

    let retry = |kind: &str, attempt: f64, delay_ms: f64| -> Value {
        let mut o = Map::new();
        o.insert("kind".to_string(), json!(kind));
        o.insert("attempt".to_string(), js::number(attempt + 1.0));
        o.insert("delayMs".to_string(), js::number(delay_ms));
        Value::Object(o)
    };

    if let Some(delay_ms) = get_stream_capacity_retry_delay_ms(cause.clone())? {
        return Ok(Some(retry("capacity", capacity_attempts, delay_ms)));
    }
    if let Some(delay_ms) = get_stream_duplicate_retry_delay_ms(cause.clone(), duplicate_attempts)?
    {
        return Ok(Some(retry("duplicate", duplicate_attempts, delay_ms)));
    }
    if let Some(delay_ms) =
        get_thread_snapshot_bootstrap_retry_delay_ms(cause.clone(), thread_bootstrap_attempts)?
    {
        return Ok(Some(retry(
            "thread-bootstrap",
            thread_bootstrap_attempts,
            delay_ms,
        )));
    }
    let Some(delay_ms) = get_resnapshot_retry_delay_ms(cause.clone(), resnapshot_attempts)? else {
        return Ok(None);
    };
    Ok(Some(retry("resnapshot", resnapshot_attempts, delay_ms)))
}

/// Port of `threadStreamInputsEqual` (`apps/web/src/wsTransport.ts:1025`).
pub fn thread_stream_inputs_equal(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<bool, String> {
    if strict_eq(&left, &right) {
        return Ok(true);
    }
    if !truthy(&left) || !truthy(&right) || !is_object(&left) || !is_object(&right) {
        return Ok(false);
    }
    let left_entries = own_entries(&left);
    let right_keys = own_keys(&right);
    if left_entries.len() != right_keys.len() {
        return Ok(false);
    }
    Ok(left_entries
        .iter()
        .all(|(k, v)| strict_eq(prop_get(&right, k).as_ref(), v)))
}

/// `Object.entries(v)` for a value already known `typeof === "object"`.
fn own_entries(v: &Value) -> Vec<(String, Value)> {
    match v {
        Value::Object(o) => o.iter().map(|(k, x)| (k.clone(), x.clone())).collect(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// `Object.keys(v)` for a value already known `typeof === "object"`.
fn own_keys(v: &Value) -> Vec<String> {
    match v {
        Value::Object(o) => o.keys().cloned().collect(),
        Value::Array(a) => (0..a.len()).map(|i| i.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// Port of `isServerLifecyclePushChannel` (`apps/web/src/wsTransport.ts:1062`).
pub fn is_server_lifecycle_push_channel(channel: String) -> Result<bool, String> {
    Ok(channel == CHANNEL_SERVER_WELCOME || channel == CHANNEL_SERVER_MAINTENANCE_UPDATED)
}

/// Port of `shouldKeepServerLifecycleStream` (`apps/web/src/wsTransport.ts:1066`).
pub fn should_keep_server_lifecycle_stream(
    active_channels: serde_json::Value,
) -> Result<bool, String> {
    if active_channels.is_null() {
        return Err("null is not an object (evaluating 'activeChannels.has')".to_string());
    }
    if is_undefined(&active_channels) {
        return Err("undefined is not an object (evaluating 'activeChannels.has')".to_string());
    }
    // `.has` exists on Set and Map; anything else fails the call.
    let has = |key: &str| -> Option<bool> {
        if let Some(items) = marker(&active_channels, "$set").and_then(Value::as_array) {
            let key_val = Value::String(key.to_string());
            return Some(items.iter().any(|x| same_value_zero(x, &key_val)));
        }
        if let Some(pairs) = marker(&active_channels, "$map").and_then(Value::as_array) {
            let key_val = Value::String(key.to_string());
            return Some(pairs.iter().any(|p| {
                p.as_array()
                    .is_some_and(|kv| kv.first().is_some_and(|k| same_value_zero(k, &key_val)))
            }));
        }
        None
    };
    match (
        has(CHANNEL_SERVER_WELCOME),
        has(CHANNEL_SERVER_MAINTENANCE_UPDATED),
    ) {
        (Some(a), Some(b)) => Ok(a || b),
        _ => Err("activeChannels.has is not a function. (In 'activeChannels.has(WS_CHANNELS.serverWelcome)', 'activeChannels.has' is undefined)".to_string()),
    }
}
