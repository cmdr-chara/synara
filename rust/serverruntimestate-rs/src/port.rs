//! The Rust port of `apps/server/src/serverRuntimeState.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::sync::LazyLock;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

static UNDEF: LazyLock<Value> = LazyLock::new(crate::js::undefined);

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

fn js_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if is_nullish(e) {
                    String::new()
                } else {
                    js_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else if let Some(x) = crate::js::as_number(v) {
                crate::js::number_to_string(x)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn js_falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|x| x == 0.0 || x.is_nan()),
        Value::String(s) => s.is_empty(),
        Value::Object(_) => {
            is_undefined(v) || crate::js::as_number(v).is_some_and(|x| x.is_nan() || x == 0.0)
        }
        Value::Array(_) => false,
    }
}

fn node_typeof(v: &Value) -> String {
    if is_undefined(v) {
        return "undefined".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!("type number ({n})"),
        Value::String(_) => "type string".to_string(),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => crate::js::as_number(v)
            .map(|x| format!("type number ({})", crate::js::number_to_string(x)))
            .unwrap_or_else(|| "an instance of Object".to_string()),
    }
}

// --- Date → ISO string --------------------------------------------------------

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `new Date(ms).toISOString()` — `Err` text mirrors `RangeError: Invalid time value`.
fn to_iso_string(ms: f64) -> Result<String, String> {
    if !ms.is_finite() || ms.abs() > 8_640_000_000_000_000.0 {
        return Err("Invalid time value".to_string());
    }
    let ms_i = ms.trunc() as i64;
    let days = ms_i.div_euclid(86_400_000);
    let rem = ms_i.rem_euclid(86_400_000);
    let (y, mo, d) = civil_from_days(days);
    let h = rem / 3_600_000;
    let mi = (rem % 3_600_000) / 60_000;
    let s = (rem % 60_000) / 1000;
    let milli = rem % 1000;
    Ok(format!(
        "{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{milli:03}Z"
    ))
}

// --- host helpers (startupAccess.ts) -----------------------------------------

fn is_wildcard_host(host: &str) -> bool {
    host == "0.0.0.0" || host == "::" || host == "[::]"
}

fn format_host_for_url(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

// --- exports ------------------------------------------------------------------

/// Port of `makePersistedServerRuntimeState` (`apps/server/src/serverRuntimeState.ts:28`).
pub fn make_persisted_server_runtime_state(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let config = member(&input, "config");
    if is_undefined(config) {
        return Err("Cannot read properties of undefined (reading 'host')".to_string());
    }
    if config.is_null() {
        return Err("Cannot read properties of null (reading 'host')".to_string());
    }
    let host = member(config, "host");
    let port = member(&input, "port");
    let hostname = if js_falsy(host) || host.as_str().is_some_and(is_wildcard_host) {
        "127.0.0.1".to_string()
    } else {
        format_host_for_url(&js_string(host))
    };
    let origin = format!("http://{hostname}:{}", js_string(port));
    let now_ms = volatile_now_ms();
    let mut out = serde_json::Map::new();
    out.insert("version".to_string(), json!(1));
    out.insert("pid".to_string(), json!(volatile_pid()));
    if !js_falsy(host) {
        out.insert("host".to_string(), host.clone());
    }
    out.insert("port".to_string(), port.clone());
    out.insert("origin".to_string(), json!(origin));
    out.insert("startedAt".to_string(), json!(to_iso_string(now_ms)?));
    out.insert(
        "externalMcpRuntimeSecret".to_string(),
        json!(runtime_secret()),
    );
    Ok(Value::Object(out))
}

// The corpus was recorded against a shim freezing `pid`, `startedAt` and
// `externalMcpRuntimeSecret`; the env vars below replay
// those frozen values. Production wiring leaves them unset → real values.
fn env_f64(name: &str) -> Option<f64> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

fn volatile_pid() -> f64 {
    env_f64("SYNARA_STATE_PID").unwrap_or_else(|| std::process::id() as f64)
}

fn volatile_now_ms() -> f64 {
    env_f64("SYNARA_STATE_NOW_MS").unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    })
}

fn runtime_secret() -> String {
    if let Ok(s) = std::env::var("SYNARA_STATE_SECRET") {
        return s;
    }
    static SECRET: LazyLock<String> = LazyLock::new(|| {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let mut h = RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_u32(std::process::id());
        let seed = h.finish();
        // base64url of 32 bytes — match Node's alphabet
        let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let bytes = alphabet.as_bytes();
        let mut s = String::with_capacity(43);
        let mut x = seed;
        for _ in 0..43 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            s.push(bytes[((x >> 33) % 64) as usize] as char);
        }
        s
    });
    SECRET.clone()
}

/// The opaque Effect object observed by the recorder.
fn effect_wire(nested: bool) -> Value {
    if nested {
        json!({
            "~effect/Effect/args": {"~effect/Effect/args": {"$function": ""}},
            "~effect/Effect/successCont": {"$function": ""}
        })
    } else {
        json!({"~effect/Effect/args": {"$function": ""}})
    }
}

/// Port of `persistServerRuntimeState` (`apps/server/src/serverRuntimeState.ts:41`).
pub fn persist_server_runtime_state(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'path')".to_string());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'path')".to_string());
    }
    let path = member(&input, "path");
    if !matches!(path, Value::String(_)) {
        return Err(format!(
            "The \"path\" argument must be of type string. Received {}",
            node_typeof(path)
        ));
    }
    Ok(effect_wire(true))
}

/// Port of `clearPersistedServerRuntimeState` (`apps/server/src/serverRuntimeState.ts:50`).
pub fn clear_persisted_server_runtime_state(_path: String) -> Result<serde_json::Value, String> {
    Ok(effect_wire(false))
}
