//! The Rust port of `scripts/dev-runner.ts`.

use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    v.as_object().is_some_and(|o| o.contains_key("$undefined"))
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

/// A JS number's view of `v`: `Number` plus the `{"$num": ...}` marker.
fn as_js_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) if o.len() == 1 => match o.get("$num")?.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}

const EFFECT_ARGS: &str = "~effect/Effect/args";

fn undefined() -> Value {
    json!({"$undefined": true})
}

const MAX_HASH_OFFSET: u32 = 3000;

/// `Hash.string` from effect: DJB2 walked back-to-front over UTF-16 units,
/// finished by `optimize` (`(n & 0xbfffffff) | ((n >>> 1) & 0x40000000)`).
fn hash_string(s: &str) -> u32 {
    let mut h: i32 = 5381;
    let units: Vec<u16> = s.encode_utf16().collect();
    for unit in units.iter().rev() {
        h = ((h as i64).wrapping_mul(33) as i32) ^ (*unit as i32);
    }
    let optimized = (h & (0xbfffffff_u32 as i32)) | (((h as u32) >> 1) & 0x40000000) as i32;
    optimized as u32
}

/// `getBooleanFlagValue`: positive wins raw, negative is inverted, else
/// undefined; errors on nullish input before reading `positive`.
fn boolean_flag_value(input: Option<&Value>) -> Result<Option<Value>, String> {
    match input {
        None => Err("Cannot read properties of undefined (reading 'positive')".to_string()),
        Some(v) if v.is_null() => {
            Err("Cannot read properties of null (reading 'positive')".to_string())
        }
        Some(v) if is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'positive')".to_string())
        }
        Some(v) => {
            if let Some(p) = prop(v, "positive")
                && !is_undefined(p)
            {
                return Ok(Some(p.clone()));
            }
            if let Some(n) = prop(v, "negative")
                && !is_undefined(n)
            {
                return Ok(Some(Value::Bool(!jscore::js_truthy(n))));
            }
            Ok(None)
        }
    }
}

/// Port of `readDevRunnerBooleanEnvironment` (`scripts/dev-runner.ts:89`).
pub fn read_dev_runner_boolean_environment(
    environment: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if environment.is_null() || is_undefined(&environment) {
        return Err("Cannot convert undefined or null to object".to_string());
    }
    Ok(json!({
        EFFECT_ARGS: { EFFECT_ARGS: {"$function": ""} },
        "~effect/Effect/failureCont": {"$function": ""},
    }))
}

/// Port of `resolveOffset` (`scripts/dev-runner.ts:106`).
pub fn resolve_offset(config: serde_json::Value) -> Result<serde_json::Value, String> {
    if config.is_null() {
        return Err("Cannot read properties of null (reading 'portOffset')".to_string());
    }
    let port_offset = prop(&config, "portOffset");
    if let Some(v) = port_offset
        && !is_undefined(v)
    {
        if as_js_num(v).is_some_and(|n| n < 0.0) {
            return Err(format!(
                "Invalid SYNARA_PORT_OFFSET: {}",
                jscore::js_to_string(v)
            ));
        }
        return Ok(json!({
            "offset": v,
            "source": format!("SYNARA_PORT_OFFSET={}", jscore::js_to_string(v)),
        }));
    }

    let dev_instance = prop(&config, "devInstance");
    let seed = match dev_instance {
        None => None,
        Some(v) if v.is_null() || is_undefined(v) => None,
        Some(Value::String(s)) => Some(s.trim_matches(jscore::is_space).to_string()),
        Some(_) => {
            return Err("config.devInstance?.trim is not a function".to_string());
        }
    };
    let Some(seed) = seed.filter(|s| !s.is_empty()) else {
        return Ok(json!({"offset": 0, "source": "default ports"}));
    };

    if seed.bytes().all(|b| b.is_ascii_digit()) {
        let offset: f64 = seed.parse().unwrap_or(f64::NAN);
        return Ok(json!({
            "offset": offset,
            "source": format!("numeric SYNARA_DEV_INSTANCE={seed}"),
        }));
    }

    let offset = (hash_string(&seed) % MAX_HASH_OFFSET) + 1;
    Ok(json!({
        "offset": offset,
        "source": format!("hashed SYNARA_DEV_INSTANCE={seed}"),
    }))
}

/// Port of `createDevRunnerEnv` (`scripts/dev-runner.ts:172`).
pub fn create_dev_runner_env(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    if arg0.is_null() {
        return Err(
            "Cannot destructure property 'mode' of 'object null' as it is null.".to_string(),
        );
    }
    if is_undefined(&arg0) {
        return Err(
            "Cannot destructure property 'mode' of 'undefined' as it is undefined.".to_string(),
        );
    }
    Ok(json!({EFFECT_ARGS: {"$function": ""}}))
}

/// Port of `resolveDevRunnerBooleanOverrides` (`scripts/dev-runner.ts:420`).
pub fn resolve_dev_runner_boolean_overrides(
    input: serde_json::Value,
    environment: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'noBrowser')".to_string());
    }
    let mut out = serde_json::Map::new();
    for key in [
        "noBrowser",
        "autoBootstrapProjectFromCwd",
        "logWebSocketEvents",
    ] {
        let resolved = match boolean_flag_value(prop(&input, key))? {
            Some(v) if !v.is_null() => v,
            _ => prop(&environment, key).cloned().unwrap_or_else(undefined),
        };
        out.insert(key.to_string(), resolved);
    }
    Ok(Value::Object(out))
}
