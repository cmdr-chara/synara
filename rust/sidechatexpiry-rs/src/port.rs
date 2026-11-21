use serde_json::{Value, json};

use crate::js::{is_undefined, number_to_string};

const INACTIVITY_EXPIRY_MS: f64 = 3_600_000.0;

fn property_key(setting: &Value) -> Result<String, String> {
    match setting {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(number_to_string(n.as_f64().unwrap_or(f64::NAN))),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Ok("null".to_string()),
        v if is_undefined(v) => Ok("undefined".to_string()),
        Value::Array(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::Null => parts.push(String::new()),
                    v if is_undefined(v) => parts.push(String::new()),
                    v => parts.push(property_key(v)?),
                }
            }
            Ok(parts.join(","))
        }
        Value::Object(map) => {
            if map.contains_key("toString") {
                return Err("Cannot convert object to primitive value".to_string());
            }
            Ok("[object Object]".to_string())
        }
    }
}

/// Port of `sidechatExpiryMs` (`packages/shared/src/sidechatExpiry.ts`).
pub fn sidechat_expiry_ms(setting: Value) -> Result<Value, String> {
    let key = property_key(&setting)?;
    Ok(match key.as_str() {
        "1h" => json!(INACTIVITY_EXPIRY_MS),
        "24h" => json!(24.0 * INACTIVITY_EXPIRY_MS),
        "never" => Value::Null,
        "constructor" => json!({ "$function": "Object" }),
        "toString"
        | "toLocaleString"
        | "valueOf"
        | "hasOwnProperty"
        | "isPrototypeOf"
        | "propertyIsEnumerable" => json!({ "$function": key.clone() }),
        "__defineGetter__" | "__defineSetter__" | "__lookupGetter__" | "__lookupSetter__" => {
            json!({ "$function": key.clone() })
        }
        "__proto__" => json!({}),
        _ => crate::js::undefined(),
    })
}
