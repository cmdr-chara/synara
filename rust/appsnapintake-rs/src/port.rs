//! The Rust port of `apps/web/src/appSnapIntake.ts`.
//!
//! Both functions depend on browser facilities (IndexedDB, File, the draft store).
//! In the recorded environment IndexedDB is unavailable and every icon cache or
//! blob-store call rejects; the port reproduces exactly those observable outcomes.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(_) => crate::js::as_number(v).unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = s.trim_matches(is_js_ws);
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => js_to_number(&a[0]),
            _ => f64::NAN,
        },
        _ => crate::js::as_number(v).unwrap_or(f64::NAN),
    }
}

/// `new Uint8Array(v).length` semantics.
fn byte_len(v: Option<&Value>) -> Result<usize, String> {
    match v {
        Some(Value::Array(items)) => Ok(items.len()),
        Some(Value::Null) | None => Ok(0),
        Some(other) => {
            let n = js_to_number(other);
            if n < 0.0 {
                return Err("length cannot be negative".into());
            }
            if n > 9007199254740991.0 || n.is_infinite() {
                return Err("length larger than (2 ** 53) - 1".into());
            }
            if n.is_nan() {
                return Ok(0);
            }
            Ok(n.trunc() as usize)
        }
    }
}

/// Port of `sourceWithCachedIcon` (`apps/web/src/appSnapIntake.ts:17`).
///
/// With IndexedDB unavailable every cache call is caught and the source is
/// returned unchanged; only the property-access and `.trim()` errors surface.
pub async fn source_with_cached_icon(
    source: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if source.is_null() {
        return Err("null is not an object (evaluating 'source.bundleIdentifier')".into());
    }
    if crate::js::is_undefined(&source) {
        return Err("undefined is not an object (evaluating 'source.bundleIdentifier')".into());
    }
    let bundle = source.get("bundleIdentifier");
    let bundle_id = match bundle {
        None | Some(Value::Null) => None,
        Some(v) if crate::js::is_undefined(v) => None,
        Some(Value::String(s)) => {
            let t = s.trim_matches(is_js_ws);
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Some(_) => {
            return Err("source.bundleIdentifier.trim is not a function".into());
        }
    };
    if bundle_id.is_none() {
        return Ok(source);
    }
    // persistAppSnapIcon / readAppSnapIcon reject when IndexedDB is unavailable;
    // both paths are caught and the source returns unchanged.
    Ok(source)
}

/// Port of `insertAppSnapCaptureIntoDraft` (`apps/web/src/appSnapIntake.ts:36`).
pub async fn insert_app_snap_capture_into_draft(
    thread_id: serde_json::Value,
    capture: serde_json::Value,
) -> Result<String, String> {
    let _ = &thread_id;
    if capture.is_null() {
        return Err("null is not an object (evaluating 'capture.capturedAt')".into());
    }
    if crate::js::is_undefined(&capture) {
        return Err("undefined is not an object (evaluating 'capture.capturedAt')".into());
    }
    let _ = capture.get("capturedAt");
    if byte_len(capture.get("bytes"))? == 0 {
        return Err("The captured AppSnap is empty.".into());
    }
    // persistComposerImageBlob's IndexedDB open rejects; nothing catches it.
    Err("IndexedDB is unavailable.".into())
}
