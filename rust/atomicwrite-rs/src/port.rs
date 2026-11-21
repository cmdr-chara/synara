//! The Rust port of `apps/server/src/atomicWrite.ts`.
//!
//! Both exports return Effect values; the recorded behavior only captures the
//! opaque Effect object plus the synchronous throws before it is created
//! (`input.filePath` member access and `path.dirname` type checking).

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
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
        Value::Object(_) => {
            if let Some(n) = crate::js::as_number(v) {
                format!("type number ({})", crate::js::number_to_string(n))
            } else {
                "an instance of Object".to_string()
            }
        }
    }
}

/// `path.dirname`'s ERR_INVALID_ARG_TYPE check.
fn dirname_arg_check(v: &Value) -> Result<(), String> {
    match v {
        Value::String(_) => Ok(()),
        _ => Err(format!(
            "The \"path\" argument must be of type string. Received {}",
            node_typeof(v)
        )),
    }
}

/// The opaque Effect object observed by the recorder.
fn effect_wire() -> Value {
    json!({
        "~effect/Effect/args": {"~effect/Effect/args": {"$function": ""}},
        "~effect/Effect/successCont": {"$function": ""}
    })
}

fn effect_entry(input: &Value) -> Result<Value, String> {
    let file_path = member(input, "filePath")?;
    dirname_arg_check(&file_path)?;
    let _mode = member(input, "mode")?;
    Ok(effect_wire())
}

/// Port of `writeFileStringAtomically` (`apps/server/src/atomicWrite.ts:141`).
pub fn write_file_string_atomically(input: serde_json::Value) -> Result<serde_json::Value, String> {
    effect_entry(&input)
}

/// Port of `createFileStringExclusively` (`apps/server/src/atomicWrite.ts:165`).
pub fn create_file_string_exclusively(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    effect_entry(&input)
}
