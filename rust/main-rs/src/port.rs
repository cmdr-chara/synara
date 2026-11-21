//! The Rust port of `apps/server/src/main.ts` (`makeServerStartupLogData`).

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `ToString` on a non-nullish value (objects land on their default
/// `toString()` — `[object Object]` — and arrays join elements with ",").
fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(items) => items.iter().map(js_to_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `{...config}` — own enumerable props: objects copy verbatim in order,
/// strings and arrays spread per UTF-16 index, other primitives add nothing.
fn spread_own(config: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    match config {
        Value::Object(o) => {
            for (k, v) in o {
                out.insert(k.clone(), v.clone());
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                out.insert(i.to_string(), v.clone());
            }
        }
        Value::String(s) => {
            for (i, unit) in s.encode_utf16().enumerate() {
                out.insert(i.to_string(), crate::js::utf16_string(&[unit]));
            }
        }
        _ => {}
    }
    out
}

/// Port of `makeServerStartupLogData` (`apps/server/src/main.ts:467`): spreads
/// the config, drops the secret-ish keys, then appends `devUrl` (stringified)
/// and `authEnabled` — so both land at the end of the object regardless of the
/// input's own key order.
pub fn make_server_startup_log_data(
    config: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut safe = spread_own(&config);
    for key in [
        "authToken",
        "desktopShutdownToken",
        "migrationDivergenceConsent",
        "devUrl",
    ] {
        safe.shift_remove(key);
    }
    // `config.devUrl?.toString()` — null/undefined throw here after the spread.
    let dev_url = match &config {
        Value::Null => {
            return Err("null is not an object (evaluating 'config.devUrl')".to_string());
        }
        v if crate::js::is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'config.devUrl')".to_string());
        }
        Value::Object(o) => o.get("devUrl"),
        _ => None,
    };
    let dev_url_out = match dev_url {
        Some(v) if !is_nullish(v) => Value::String(js_to_string(v)),
        _ => crate::js::undefined(),
    };
    safe.insert("devUrl".to_string(), dev_url_out);
    let auth_token = match &config {
        Value::Object(o) => o.get("authToken"),
        _ => None,
    };
    safe.insert(
        "authEnabled".to_string(),
        Value::Bool(auth_token.is_some_and(truthy)),
    );
    Ok(Value::Object(safe))
}
