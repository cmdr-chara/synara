//! The Rust port of `apps/desktop/src/backendShutdown.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `v.key`: TypeError on null/undefined, `undefined` (None) on primitives and
/// arrays, the member on real objects.
fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => {
            if is_undefined(v) {
                Err(format!(
                    "Cannot read properties of undefined (reading '{key}')"
                ))
            } else {
                Ok(o.get(key))
            }
        }
        _ => Ok(None),
    }
}

/// A JS string's view of `v`: `String` plus the `{"$utf16": ...}` marker.
fn as_js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("$utf16").and_then(Value::as_array).map(|units| {
            let u: Vec<u16> = units
                .iter()
                .filter_map(|x| x.as_u64().map(|n| n as u16))
                .collect();
            String::from_utf16_lossy(&u)
        }),
        _ => None,
    }
}

/// A JS number's view of `v`: `Number` plus the `{"$num": ...}` marker.
fn as_js_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) if o.len() == 1 => match o.get("$num")?.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}

/// JS truthiness after decoding wire markers: false, 0/-0, NaN, "", null and
/// undefined are falsy; everything else is truthy.
fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undefined(v) {
                return false;
            }
            if let Some(x) = as_js_num(v) {
                return x != 0.0 && !x.is_nan();
            }
            if let Some(s) = as_js_str(v) {
                return !s.is_empty();
            }
            true
        }
    }
}

/// Port of `requireWindowsBackendExit` (`apps/desktop/src/backendShutdown.ts:51`).
pub fn require_windows_backend_exit(result: serde_json::Value) -> Result<(), String> {
    if let Some(t) = prop(&result, "type")?
        && as_js_str(t).as_deref() == Some("timed-out")
    {
        return Err(
            "WindowsBackendShutdownTimeoutError: Timed out waiting for the Windows desktop backend to exit."
                .to_string(),
        );
    }
    Ok(())
}

/// Port of `shouldDeferDesktopWindowClose` (`apps/desktop/src/backendShutdown.ts:79`).
pub fn should_defer_desktop_window_close(input: serde_json::Value) -> Result<bool, String> {
    let platform = prop(&input, "platform")?;
    if !matches!(platform, Some(p) if as_js_str(p).as_deref() == Some("win32")) {
        return Ok(false);
    }
    let shutdown_complete = prop(&input, "shutdownComplete")?.is_some_and(is_truthy);
    let updater_handoff = prop(&input, "updaterHandoffActive")?.is_some_and(is_truthy);
    Ok(!shutdown_complete && !updater_handoff)
}

/// Port of `retainLiveBackendAfterShutdownFailure` (`apps/desktop/src/backendShutdown.ts:180`).
pub fn retain_live_backend_after_shutdown_failure(
    current: Option<serde_json::Value>,
    attempted: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    // `current === null` short-circuits before `attempted` is inspected; the
    // wire already maps null to None.
    match current {
        Some(c) => Ok(Some(c)),
        None => {
            let exit_code = prop(&attempted, "exitCode")?;
            let signal_code = prop(&attempted, "signalCode")?;
            let has_exited = !matches!(exit_code, Some(Value::Null))
                || !matches!(signal_code, Some(Value::Null));
            if has_exited {
                Ok(None) // current
            } else {
                Ok(Some(attempted))
            }
        }
    }
}
