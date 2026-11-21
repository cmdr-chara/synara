//! The Rust port of `apps/desktop/src/runtimeArch.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if js::is_undefined(v) {
        return None;
    }
    v.as_object().and_then(|o| o.get(key))
}

// INFALLIBLE: NaN is falsy in JS, `x != 0.0` would be wrong.
#[allow(clippy::float_cmp, clippy::double_comparisons)]
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None => false,
        Some(v) if js::is_undefined(v) => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x > 0.0 || x < 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

fn canonical_desktop_arch(arch: Option<&Value>) -> &'static str {
    match arch.and_then(Value::as_str) {
        Some("arm64") => "arm64",
        Some("x64") => "x64",
        _ => "other",
    }
}

/// Port of `resolveDesktopRuntimeInfo` (`apps/desktop/src/runtimeArch.ts:15`).
pub fn resolve_desktop_runtime_info(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'processArch')".to_string());
    }
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'processArch')".to_string());
    }
    let app_arch = canonical_desktop_arch(prop(&input, "processArch"));
    if prop(&input, "platform").and_then(Value::as_str) != Some("darwin") {
        return Ok(json!({
            "hostArch": app_arch,
            "appArch": app_arch,
            "runningUnderArm64Translation": false,
        }));
    }
    let translated = truthy(prop(&input, "runningUnderArm64Translation"));
    let host_arch = if app_arch == "arm64" || translated {
        "arm64"
    } else {
        app_arch
    };
    Ok(json!({
        "hostArch": host_arch,
        "appArch": app_arch,
        "runningUnderArm64Translation": prop(&input, "runningUnderArm64Translation")
            .cloned()
            .unwrap_or_else(js::undefined),
    }))
}

/// Port of `isArm64HostRunningIntelBuild` (`apps/desktop/src/runtimeArch.ts:37`).
pub fn is_arm64_host_running_intel_build(runtime_info: Value) -> Result<bool, String> {
    if runtime_info.is_null() {
        return Err("Cannot read properties of null (reading 'hostArch')".to_string());
    }
    if js::is_undefined(&runtime_info) {
        return Err("Cannot read properties of undefined (reading 'hostArch')".to_string());
    }
    Ok(
        prop(&runtime_info, "hostArch").and_then(Value::as_str) == Some("arm64")
            && prop(&runtime_info, "appArch").and_then(Value::as_str) == Some("x64"),
    )
}
