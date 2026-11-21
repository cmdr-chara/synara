//! The Rust port of `apps/desktop/src/mediaPermissions.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `shouldAllowMediaPermissionRequest` (`apps/desktop/src/mediaPermissions.ts:15`).
pub fn should_allow_media_permission_request(details: serde_json::Value) -> Result<bool, String> {
    // `typeof details !== "object" || details === null` → true; arrays are objects.
    if !matches!(details, Value::Object(_) | Value::Array(_)) || details.is_null() {
        return Ok(true);
    }
    let Value::Object(record) = &details else {
        return Ok(true);
    };
    if let Some(Value::Array(types)) = record
        .get("mediaTypes")
        .filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))
    {
        return Ok(types
            .iter()
            .all(|t| matches!(t, Value::String(s) if s == "audio")));
    }
    if let Some(Value::String(media_type)) = record.get("mediaType") {
        return Ok(media_type == "audio");
    }
    Ok(true)
}
