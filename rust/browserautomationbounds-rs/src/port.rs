//! The Rust port of `packages/contracts/src/browserAutomationBounds.ts`.

#![forbid(unsafe_code)]

const BOUNDS_ERROR: &str = "UTF-8 byte bounds must be non-negative safe integers with min <= max";

fn is_safe_bound(n: f64) -> bool {
    n.is_finite() && n.fract() == 0.0 && (0.0..=9_007_199_254_740_991.0).contains(&n)
}

/// JS `String(v)` coercion for the measured value. Infallible on transportable
/// values: the only `String()` thrower is `Symbol`, which never crosses the wire.
fn js_to_string(v: &serde_json::Value) -> String {
    if crate::js::is_undefined(v) {
        return String::from("undefined");
    }
    match v {
        serde_json::Value::Null => String::from("null"),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|i| {
                if matches!(i, serde_json::Value::Null) || crate::js::is_undefined(i) {
                    String::new()
                } else {
                    js_to_string(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        serde_json::Value::Object(_) => {
            if let Some(units) = crate::js::string_units(v) {
                // `String(v)` then utf8 bytes: unpaired surrogates become U+FFFD.
                return String::from_utf16_lossy(&units);
            }
            String::from("[object Object]")
        }
    }
}

/// Port of `utf8ByteLength` (`packages/contracts/src/browserAutomationBounds.ts`).
pub fn utf8_byte_length(value: serde_json::Value) -> Result<f64, String> {
    if crate::js::is_undefined(&value) {
        return Ok(0.0);
    }
    Ok(js_to_string(&value).len() as f64)
}

/// Port of `BoundedUtf8String` (`packages/contracts/src/browserAutomationBounds.ts`).
pub fn bounded_utf8_string(
    max_bytes: f64,
    min_bytes: Option<f64>,
) -> Result<serde_json::Value, String> {
    if !is_safe_bound(max_bytes) || min_bytes.is_some_and(|m| !is_safe_bound(m) || m > max_bytes) {
        return Err(String::from(BOUNDS_ERROR));
    }
    Ok(serde_json::json!({
        "ast": {
            "~effect/Schema": "~effect/Schema",
            "annotations": crate::js::undefined(),
            "checks": [{
                "_tag": "Filter",
                "run": { "$function": "" },
                "annotations": crate::js::undefined(),
                "aborted": false,
            }],
            "encoding": crate::js::undefined(),
            "context": crate::js::undefined(),
            "_tag": "String",
        },
        "rebuild": { "$function": "" },
        "makeUnsafe": { "$function": "" },
        "makeOption": { "$function": "" },
    }))
}
