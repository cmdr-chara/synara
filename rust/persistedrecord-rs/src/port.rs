//! The Rust port of `isPlainObject` (`apps/web/src/persistedRecord.ts:6`).

/// `typeof value === "object" && value !== null && !Array.isArray(value)`.
///
/// Wire-codec markers that decode to primitives keep the primitive answer;
/// every other object marker (`$bytes`, `$buffer`, `$map`, `$set`, `$date`,
/// `$typed`, `$object`) decodes to a non-null non-array JS object.
pub fn is_plain_object(value: serde_json::Value) -> Result<bool, String> {
    Ok(match value {
        serde_json::Value::Object(o) => {
            !o.contains_key("$undefined")
                && !o.contains_key("$num")
                && !o.contains_key("$bigint")
                && !o.contains_key("$utf16")
        }
        _ => false,
    })
}
