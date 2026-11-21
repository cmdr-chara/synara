//! The Rust port of `apps/desktop/src/bundleSwapDetection.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

fn member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(v.get(key).cloned().unwrap_or_else(js::undefined))
}

fn num_neq(a: &Value, b: &Value) -> bool {
    // `!==` on stat numbers.
    match (js::as_number(a), js::as_number(b)) {
        (Some(x), Some(y)) => x != y,
        _ => a != b,
    }
}

/// Port of `bundleSignatureFromStats` (`apps/desktop/src/bundleSwapDetection.ts:29`).
pub fn bundle_signature_from_stats(stats: serde_json::Value) -> Result<serde_json::Value, String> {
    let size = member(&stats, "size")?;
    let mtime = member(&stats, "mtimeMs")?;
    let ino = member(&stats, "ino")?;
    Ok(json!({ "size": size, "mtimeMs": mtime, "inode": ino }))
}

/// Port of `isWatchableBundlePath` (`apps/desktop/src/bundleSwapDetection.ts:35`).
pub fn is_watchable_bundle_path(app_path: String) -> Result<bool, String> {
    Ok(app_path.ends_with(".asar"))
}

fn swapped(baseline: &Value, current: &Value) -> Result<bool, String> {
    Ok(
        num_neq(&member(current, "size")?, &member(baseline, "size")?)
            || num_neq(&member(current, "mtimeMs")?, &member(baseline, "mtimeMs")?)
            || num_neq(&member(current, "inode")?, &member(baseline, "inode")?),
    )
}

/// Port of `isBundleSwapped` (`apps/desktop/src/bundleSwapDetection.ts:45`).
pub fn is_bundle_swapped(
    baseline: serde_json::Value,
    current: Option<serde_json::Value>,
) -> Result<bool, String> {
    let Some(cur) = current else {
        return Ok(false);
    };
    swapped(&baseline, &cur)
}

/// Port of `isBundleStable` (`apps/desktop/src/bundleSwapDetection.ts:63`).
pub fn is_bundle_stable(
    baseline: serde_json::Value,
    current: Option<serde_json::Value>,
) -> Result<bool, String> {
    let Some(cur) = current else {
        return Ok(false);
    };
    Ok(!swapped(&baseline, &cur)?)
}
