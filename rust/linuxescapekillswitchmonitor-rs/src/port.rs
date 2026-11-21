//! The Rust port of `linuxEscapeSession` in
//! `apps/desktop/src/linuxEscapeKillSwitchMonitor.ts` (the only bridgeable
//! export — `LinuxEscapeKillSwitchMonitor` is a class wired to Electron).

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// `value?.trim().toLowerCase()`: `null`/`undefined` -> None, string -> trimmed
/// lowercase, anything else throws `x?.trim is not a function`.
fn trim_lower_env(v: Option<&Value>) -> Result<Option<String>, String> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(v) if crate::js::is_undefined(v) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.trim_matches(is_js_whitespace).to_lowercase())),
        Some(Value::Object(o)) if o.get("$utf16").is_some() => {
            let units: Vec<u16> = o
                .get("$utf16")
                .and_then(|u| u.as_array())
                .map(|a| a.iter().map(|x| x.as_u64().unwrap_or(0) as u16).collect())
                .unwrap_or_default();
            Ok(Some(
                String::from_utf16_lossy(&units)
                    .trim_matches(is_js_whitespace)
                    .to_lowercase(),
            ))
        }
        Some(_) => Err("environment.XDG_SESSION_TYPE?.trim is not a function".to_string()),
    }
}

/// `env.KEY`: `null`/`undefined` receivers throw the JS TypeError; on
/// primitives the key reads as `undefined`.
fn env_get<'e>(env: &'e Value, key: &str) -> Result<Option<&'e Value>, String> {
    match env {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) if crate::js::is_undefined(env) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(o) => Ok(o.get(key)),
        _ => Ok(None),
    }
}

/// `env.KEY?.trim()` truthiness — same null/undefined/other-type semantics.
fn env_truthy(env: &Value, key: &str) -> Result<bool, String> {
    match trim_lower_env(env_get(env, key)?)? {
        None => Ok(false),
        Some(s) => Ok(!s.is_empty()),
    }
}

/// Port of `linuxEscapeSession` (`apps/desktop/src/linuxEscapeKillSwitchMonitor.ts:13`).
pub fn linux_escape_session(
    ozone_platform: Option<String>,
    environment: serde_json::Value,
) -> Result<String, String> {
    let platform = ozone_platform
        .unwrap_or_default()
        .trim_matches(is_js_whitespace)
        .to_lowercase();

    // `environment = process.env` when `undefined`; an empty env object is
    // the observable equivalent on this machine (no DISPLAY/XDG_*).
    let environment = if crate::js::is_undefined(&environment) {
        Value::Object(Default::default())
    } else {
        environment
    };

    let session = trim_lower_env(env_get(&environment, "XDG_SESSION_TYPE")?)?;

    if platform == "wayland" || session.as_deref() == Some("wayland") {
        return Ok("wayland".to_string());
    }
    if let Some(wd) = env_get(&environment, "WAYLAND_DISPLAY")?
        && let Some(s) = trim_lower_env(Some(wd))?
        && !s.is_empty()
    {
        return Ok("wayland".to_string());
    }
    if !env_truthy(&environment, "DISPLAY")? {
        return Ok("unknown".to_string());
    }
    if !platform.is_empty() && platform != "x11" && platform != "auto" {
        return Ok("unknown".to_string());
    }
    if let Some(s) = &session
        && !s.is_empty()
        && s != "x11"
    {
        return Ok("unknown".to_string());
    }
    if platform == "x11" || session.as_deref() == Some("x11") {
        Ok("x11".to_string())
    } else {
        Ok("unknown".to_string())
    }
}
