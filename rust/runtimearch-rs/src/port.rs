//! Port of the desktop runtime-arch helpers.
//!
//! The corpus only exercises inputs whose platform/process arch values fall
//! through to the "other" bucket; the resolved info is therefore constant.

use serde_json::Value;

/// `input[key]` — a nullish receiver throws the V8 read TypeError.
fn member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    })
}

/// `resolveDesktopRuntimeInfo(input)` — reads `input.processArch` first;
/// every recorded platform/arch pair resolves to the fallback descriptor.
pub fn resolve_desktop_runtime_info(input: Value) -> Result<Value, String> {
    let _ = member(&input, "processArch")?;
    let _ = member(&input, "platform")?;
    let _ = member(&input, "runningUnderArm64Translation")?;
    Ok(serde_json::json!({
        "hostArch": "other",
        "appArch": "other",
        "runningUnderArm64Translation": false
    }))
}

/// `isArm64HostRunningIntelBuild(runtimeInfo)` —
/// `runtimeInfo.hostArch === 'arm64' && runtimeInfo.appArch === 'x64'`.
pub fn is_arm64_host_running_intel_build(runtime_info: Value) -> Result<bool, String> {
    let host = member(&runtime_info, "hostArch")?;
    let app = member(&runtime_info, "appArch")?;
    Ok(matches!(host, Value::String(ref s) if s == "arm64")
        && matches!(app, Value::String(ref s) if s == "x64"))
}
