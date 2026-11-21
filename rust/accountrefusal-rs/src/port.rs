//! The Rust port of `apps/server/src/accountRefusal.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn account_status(error: &Value) -> Option<f64> {
    let mut value = error;
    for _ in 0..6 {
        if value.is_null() || crate::js::is_undefined(value) {
            return None;
        }
        if let Value::Object(o) = value
            && let Some(s) = o.get("status").and_then(Value::as_f64)
        {
            return Some(s);
        }
        value = match value {
            Value::Object(o) => o.get("cause").unwrap_or(&Value::Null),
            _ => &Value::Null,
        };
    }
    None
}

fn account_field<'a>(error: &'a Value, key: &str) -> Option<&'a Value> {
    let mut value = error;
    for _ in 0..6 {
        if value.is_null() || crate::js::is_undefined(value) {
            return None;
        }
        if let Value::Object(o) = value
            && o.get("status").and_then(Value::as_f64).is_some()
        {
            return o.get(key);
        }
        value = match value {
            Value::Object(o) => o.get("cause").unwrap_or(&Value::Null),
            _ => &Value::Null,
        };
    }
    None
}

/// Port of `isAccountRefusal` (`apps/server/src/accountRefusal.ts:18`).
pub fn is_account_refusal(error: serde_json::Value) -> Result<bool, String> {
    Ok(account_status(&error) == Some(403.0))
}

/// Port of `isRemoteAccessRefusal` (`apps/server/src/accountRefusal.ts:27`).
pub fn is_remote_access_refusal(error: serde_json::Value) -> Result<bool, String> {
    if account_status(&error) != Some(403.0) {
        return Ok(false);
    }
    let code = account_field(&error, "code").and_then(Value::as_str);
    let message = account_field(&error, "message").and_then(Value::as_str);
    Ok(code == Some("subscription_required")
        || (code == Some("not_host_owner")
            && message == Some("Remote test access is not enabled for this account")))
}
