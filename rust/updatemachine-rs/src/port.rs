//! The Rust port of `apps/desktop/src/updateMachine.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

#[path = "../../updatestate-rs/src/port.rs"]
pub mod updatestate_port;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    })
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Number(_) | Value::Array(_) | Value::Object(_) => !is_undefined(v),
    }
}

/// Object spread `{...v}`: objects clone their map, strings spread per UTF-16
/// code unit, arrays by index, everything else yields nothing.
fn spread(v: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    match v {
        Value::Object(m) if !is_undefined(v) => out = m.clone(),
        Value::Array(a) => {
            for (i, e) in a.iter().enumerate() {
                out.insert(i.to_string(), e.clone());
            }
        }
        Value::String(_) => {
            if let Some(units) = crate::js::string_units(v) {
                for (i, u) in units.iter().enumerate() {
                    out.insert(i.to_string(), crate::js::utf16_string(&[*u]));
                }
            }
        }
        _ => {}
    }
    out
}

/// `field === "literal"` strict equality.
fn eq_str(v: &Value, s: &str) -> bool {
    v.as_str() == Some(s)
}

/// `parseUpdateVersion`'s `version.trim()` on a non-string argument.
fn trim_err(v: &Value) -> String {
    if is_undefined(v) {
        "Cannot read properties of undefined (reading 'trim')".to_string()
    } else {
        "version.trim is not a function".to_string()
    }
}

/// `isUpdateVersionNewer(current, candidate)` where `current` is a Value field.
fn is_newer(current: &Value, candidate: &str) -> Result<bool, String> {
    let Some(cur) = current.as_str() else {
        return Err(trim_err(current));
    };
    updatestate_port::is_update_version_newer(cur.to_string(), candidate.to_string())
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

/// Port of `createInitialDesktopUpdateState` (`apps/desktop/src/updateMachine.ts:12`).
pub fn create_initial_desktop_update_state(
    current_version: String,
    runtime_info: serde_json::Value,
    flavor: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let flavor = if is_undefined(&flavor) {
        s("production")
    } else {
        flavor
    };
    let mut out = Map::new();
    out.insert("enabled".into(), Value::Bool(false));
    out.insert("status".into(), s("disabled"));
    out.insert("currentVersion".into(), Value::String(current_version));
    out.insert("hostArch".into(), member(&runtime_info, "hostArch")?);
    out.insert("appArch".into(), member(&runtime_info, "appArch")?);
    out.insert(
        "runningUnderArm64Translation".into(),
        member(&runtime_info, "runningUnderArm64Translation")?,
    );
    out.insert("availableVersion".into(), Value::Null);
    out.insert("downloadedVersion".into(), Value::Null);
    out.insert("downloadPercent".into(), Value::Null);
    out.insert("checkedAt".into(), Value::Null);
    out.insert("message".into(), Value::Null);
    out.insert("errorContext".into(), Value::Null);
    out.insert("canRetry".into(), Value::Bool(false));
    out.insert("installFailureCount".into(), Value::from(0));
    out.insert("flavor".into(), flavor);
    out.insert("releaseUrl".into(), Value::Null);
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnCheckStart` (`apps/desktop/src/updateMachine.ts:37`).
pub fn reduce_desktop_update_state_on_check_start(
    state: serde_json::Value,
    checked_at: String,
) -> Result<serde_json::Value, String> {
    let mut out = spread(&state);
    out.insert("status".into(), s("checking"));
    out.insert("checkedAt".into(), Value::String(checked_at));
    out.insert("message".into(), Value::Null);
    out.insert("downloadPercent".into(), Value::Null);
    out.insert("errorContext".into(), Value::Null);
    out.insert("canRetry".into(), Value::Bool(false));
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnCheckFailure` (`apps/desktop/src/updateMachine.ts:52`).
pub fn reduce_desktop_update_state_on_check_failure(
    state: serde_json::Value,
    message: String,
    checked_at: String,
) -> Result<serde_json::Value, String> {
    let downloaded_is_null = member(&state, "downloadedVersion")?.is_null();
    let mut out = spread(&state);
    out.insert(
        "status".into(),
        s(if downloaded_is_null {
            "error"
        } else {
            "downloaded"
        }),
    );
    out.insert("message".into(), Value::String(message));
    out.insert("checkedAt".into(), Value::String(checked_at));
    out.insert(
        "downloadPercent".into(),
        if downloaded_is_null {
            Value::Null
        } else {
            crate::js::number(100.0)
        },
    );
    out.insert("errorContext".into(), s("check"));
    out.insert("canRetry".into(), Value::Bool(true));
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnUpdateAvailable` (`apps/desktop/src/updateMachine.ts:68`).
pub fn reduce_desktop_update_state_on_update_available(
    state: serde_json::Value,
    version: String,
    checked_at: String,
) -> Result<serde_json::Value, String> {
    let downloaded = member(&state, "downloadedVersion")?;
    if !downloaded.is_null() && !is_newer(&downloaded, &version)? {
        let mut out = spread(&state);
        out.insert("status".into(), s("downloaded"));
        out.insert("checkedAt".into(), Value::String(checked_at));
        out.insert("downloadPercent".into(), crate::js::number(100.0));
        out.insert("canRetry".into(), Value::Bool(true));
        return Ok(Value::Object(out));
    }
    let available = member(&state, "availableVersion")?;
    let install_failure_count = if eq_str(&available, &version) {
        member(&state, "installFailureCount")?
    } else {
        Value::from(0)
    };
    let mut out = spread(&state);
    out.insert("status".into(), s("available"));
    out.insert("availableVersion".into(), Value::String(version));
    out.insert("downloadedVersion".into(), Value::Null);
    out.insert("downloadPercent".into(), Value::Null);
    out.insert("checkedAt".into(), Value::String(checked_at));
    out.insert("message".into(), Value::Null);
    out.insert("errorContext".into(), Value::Null);
    out.insert("canRetry".into(), Value::Bool(false));
    out.insert("installFailureCount".into(), install_failure_count);
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnNoUpdate` (`apps/desktop/src/updateMachine.ts:98`).
pub fn reduce_desktop_update_state_on_no_update(
    state: serde_json::Value,
    checked_at: String,
) -> Result<serde_json::Value, String> {
    let downloaded = member(&state, "downloadedVersion")?;
    let kept = !downloaded.is_null()
        && is_newer(&member(&state, "currentVersion")?, &{
            match &downloaded {
                Value::String(v) => v.clone(),
                _ => return Err(trim_err(&downloaded)),
            }
        })?
        && updatestate_port::is_update_version_allowed_for_flavor(
            match &downloaded {
                Value::String(v) => v.clone(),
                _ => return Err(trim_err(&downloaded)),
            },
            member(&state, "flavor")?
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| {
                    // non-string flavor: comparisons against literals all fail
                    "\u{0}".to_string()
                }),
        )?;
    if kept {
        let mut out = spread(&state);
        out.insert("status".into(), s("downloaded"));
        out.insert("downloadPercent".into(), crate::js::number(100.0));
        out.insert("checkedAt".into(), Value::String(checked_at));
        out.insert("message".into(), Value::Null);
        out.insert("errorContext".into(), Value::Null);
        out.insert("canRetry".into(), Value::Bool(true));
        return Ok(Value::Object(out));
    }
    let mut out = spread(&state);
    out.insert("status".into(), s("up-to-date"));
    out.insert("availableVersion".into(), Value::Null);
    out.insert("downloadedVersion".into(), Value::Null);
    out.insert("downloadPercent".into(), Value::Null);
    out.insert("checkedAt".into(), Value::String(checked_at));
    out.insert("message".into(), Value::Null);
    out.insert("errorContext".into(), Value::Null);
    out.insert("canRetry".into(), Value::Bool(false));
    out.insert("installFailureCount".into(), Value::from(0));
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnDownloadStart` (`apps/desktop/src/updateMachine.ts:132`).
pub fn reduce_desktop_update_state_on_download_start(
    state: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = spread(&state);
    out.insert("status".into(), s("downloading"));
    out.insert("downloadPercent".into(), Value::Null);
    out.insert("message".into(), Value::Null);
    out.insert("errorContext".into(), Value::Null);
    out.insert("canRetry".into(), Value::Bool(false));
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnDownloadFailure` (`apps/desktop/src/updateMachine.ts:145`).
pub fn reduce_desktop_update_state_on_download_failure(
    state: serde_json::Value,
    message: String,
) -> Result<serde_json::Value, String> {
    let available = member(&state, "availableVersion")?;
    let mut out = spread(&state);
    out.insert(
        "status".into(),
        s(if truthy(&available) {
            "available"
        } else {
            "error"
        }),
    );
    out.insert("downloadedVersion".into(), Value::Null);
    out.insert("message".into(), Value::String(message));
    out.insert("downloadPercent".into(), Value::Null);
    out.insert("errorContext".into(), s("download"));
    out.insert("canRetry".into(), Value::Bool(!available.is_null()));
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnDownloadProgress` (`apps/desktop/src/updateMachine.ts:160`).
pub fn reduce_desktop_update_state_on_download_progress(
    state: serde_json::Value,
    percent: f64,
) -> Result<serde_json::Value, String> {
    let mut out = spread(&state);
    out.insert("status".into(), s("downloading"));
    out.insert("downloadPercent".into(), crate::js::number(percent));
    out.insert("message".into(), Value::Null);
    out.insert("errorContext".into(), Value::Null);
    out.insert("canRetry".into(), Value::Bool(false));
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnDownloadComplete` (`apps/desktop/src/updateMachine.ts:174`).
pub fn reduce_desktop_update_state_on_download_complete(
    state: serde_json::Value,
    version: String,
) -> Result<serde_json::Value, String> {
    if !eq_str(&member(&state, "status")?, "downloading")
        || !eq_str(&member(&state, "availableVersion")?, &version)
    {
        return Ok(state);
    }
    let install_failure_count = member(&state, "installFailureCount")?;
    let mut out = spread(&state);
    out.insert("status".into(), s("downloaded"));
    out.insert("availableVersion".into(), Value::String(version.clone()));
    out.insert("downloadedVersion".into(), Value::String(version));
    out.insert("downloadPercent".into(), crate::js::number(100.0));
    out.insert("message".into(), Value::Null);
    out.insert("errorContext".into(), Value::Null);
    out.insert("canRetry".into(), Value::Bool(true));
    out.insert("installFailureCount".into(), install_failure_count);
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnInstallStart` (`apps/desktop/src/updateMachine.ts:193`).
pub fn reduce_desktop_update_state_on_install_start(
    state: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = spread(&state);
    out.insert("message".into(), Value::Null);
    out.insert("errorContext".into(), Value::Null);
    out.insert("canRetry".into(), Value::Bool(false));
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnInstallFailure` (`apps/desktop/src/updateMachine.ts:199`).
pub fn reduce_desktop_update_state_on_install_failure(
    state: serde_json::Value,
    message: String,
) -> Result<serde_json::Value, String> {
    let mut out = spread(&state);
    out.insert("status".into(), s("downloaded"));
    out.insert("message".into(), Value::String(message));
    out.insert("errorContext".into(), s("install"));
    out.insert("canRetry".into(), Value::Bool(true));
    Ok(Value::Object(out))
}

/// Port of `reduceDesktopUpdateStateOnInstallRestartFailure` (`apps/desktop/src/updateMachine.ts:212`).
pub fn reduce_desktop_update_state_on_install_restart_failure(
    state: serde_json::Value,
    to_version: String,
    consecutive_failures: f64,
    message: String,
) -> Result<serde_json::Value, String> {
    let mut out = spread(&state);
    out.insert("status".into(), s("error"));
    out.insert("availableVersion".into(), Value::String(to_version));
    out.insert("downloadedVersion".into(), Value::Null);
    out.insert("downloadPercent".into(), Value::Null);
    out.insert("message".into(), Value::String(message));
    out.insert("errorContext".into(), s("install"));
    out.insert("canRetry".into(), Value::Bool(true));
    out.insert(
        "installFailureCount".into(),
        crate::js::number(consecutive_failures),
    );
    Ok(Value::Object(out))
}
