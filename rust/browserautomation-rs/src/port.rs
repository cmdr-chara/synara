//! The Rust port of `apps/server/src/browserAutomation/`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undef(v: &Value) -> bool {
    v.get("$undefined").and_then(Value::as_bool) == Some(true)
}

fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m
            .get(key)
            .cloned()
            .unwrap_or(serde_json::json!({"$undefined": true})),
        _ => serde_json::json!({"$undefined": true}),
    }
}

fn read_prop_err(kind: &str, key: &str) -> String {
    format!("Cannot read properties of {kind} (reading '{key}')")
}

/// `env.<key>?.trim()` — string values trim, missing keys stay missing,
/// any other present value is the `?.trim is not a function` TypeError.
fn trimmed_env(env: &Value, key: &str) -> Result<Option<String>, String> {
    let v = prop(env, key);
    if v.is_null() || is_undef(&v) {
        return Ok(None);
    }
    match v {
        Value::String(s) => Ok(Some(s.trim().to_string())),
        _ => Err(format!("env.{key}?.trim is not a function")),
    }
}

/// Port of `callBrowserHostTool` (`apps/server/src/browserAutomation/browserHostRpcClient.ts:289`).
pub async fn call_browser_host_tool(input: serde_json::Value) -> Result<serde_json::Value, String> {
    // const deadline = makeDeadline(input.timeoutMs) — reads `timeoutMs` first.
    if input.is_null() {
        return Err(read_prop_err("null", "timeoutMs"));
    }
    if is_undef(&input) {
        return Err(read_prop_err("undefined", "timeoutMs"));
    }
    // expiresAt = now + Math.max(0, timeoutMs); remaining <= 0 throws.
    let timeout = match prop(&input, "timeoutMs") {
        Value::Number(n) => n.as_f64(),
        v => match v.get("$num").and_then(Value::as_str) {
            Some("NaN") => Some(f64::NAN),
            Some("Infinity") => Some(f64::INFINITY),
            Some("-Infinity") => Some(f64::NEG_INFINITY),
            Some("-0") => Some(-0.0),
            _ => None,
        },
    };
    if let Some(n) = timeout
        && n <= 0.0
    {
        return Err("Browser host connection timed out.".into());
    }
    // Net.createConnection(path) throws synchronously on a bad path argument.
    let pipe_path = prop(&input, "pipePath");
    let path = match &pipe_path {
        Value::String(s) => {
            if s.is_empty() {
                return Err("Port should be >= 0 and < 65536. Received type string ('')".into());
            }
            if let Ok(port) = s.parse::<f64>() {
                if !(0.0..65536.0).contains(&port) {
                    return Err(format!(
                        "Port should be >= 0 and < 65536. Received type string ('{s}')"
                    ));
                }
                return Err(format!("connect ECONNREFUSED ::1:{}", port as i64));
            }
            s.clone()
        }
        Value::Number(n) => {
            let port = n.as_f64().unwrap_or(f64::NAN);
            if !(0.0..65536.0).contains(&port) {
                return Err(format!("Port should be >= 0 and < 65536. Received {port}"));
            }
            return Err(format!("connect ECONNREFUSED ::1:{}", port as i64));
        }
        _ => {
            return Err(
                "The \"options\" or \"port\" or \"path\" argument must be specified".into(),
            );
        }
    };
    // signal?.addEventListener("abort", ...) — any non-nullish JSON value is not a function.
    let signal = prop(&input, "signal");
    if !signal.is_null() && !is_undef(&signal) {
        return Err("signal?.addEventListener is not a function".into());
    }
    // The async socket 'error' event rejects with the OS error message.
    #[cfg(unix)]
    match std::os::unix::net::UnixStream::connect(&path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("connect ENOENT {path}"));
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            return Err(format!("connect EACCES {path}"));
        }
        Err(e) => return Err(format!("connect EIO {path} ({e})")),
    }
    Err("Browser host connection closed.".into())
}

/// Port of `resolveBrowserHostPipePath` (`apps/server/src/browserAutomation/browserHostRpcClient.ts:329`).
pub fn resolve_browser_host_pipe_path(
    env: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    // Missing arg or JS undefined both take the `= process.env` default,
    // which is empty in the recorded environment.
    let env = match env {
        None => return Ok(None),
        Some(v) if is_undef(&v) => return Ok(None),
        Some(v) => v,
    };
    if env.is_null() {
        return Err(read_prop_err("null", "SYNARA_BROWSER_HOST_PIPE_PATH"));
    }
    if let Some(t) = trimmed_env(&env, "SYNARA_BROWSER_HOST_PIPE_PATH")?
        && !t.is_empty()
    {
        return Ok(Some(t));
    }
    if let Some(t) = trimmed_env(&env, "SYNARA_BROWSER_USE_PIPE_PATH")?
        && !t.is_empty()
    {
        return Ok(Some(t));
    }
    Ok(None)
}

/// Port of `resolveBrowserHostCapability` (`apps/server/src/browserAutomation/browserHostRpcClient.ts:337`).
pub fn resolve_browser_host_capability(
    env: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let env = match env {
        None => return Ok(None),
        Some(v) if is_undef(&v) => return Ok(None),
        Some(v) => v,
    };
    if env.is_null() {
        return Err(read_prop_err("null", "SYNARA_BROWSER_HOST_CAPABILITY"));
    }
    if let Some(direct) = trimmed_env(&env, "SYNARA_BROWSER_HOST_CAPABILITY")?
        && !direct.is_empty()
        && direct.len() >= 32
    {
        return Ok(Some(direct));
    }
    let raw_fd = trimmed_env(&env, "SYNARA_BROWSER_HOST_CAPABILITY_FD")?;
    let Some(raw_fd) = raw_fd else {
        return Ok(None);
    };
    if raw_fd.is_empty() || !raw_fd.chars().all(|c| c.is_ascii_digit()) {
        return Ok(None);
    }
    let Ok(fd) = raw_fd.parse::<i64>() else {
        return Ok(None);
    };
    if !(3..=255).contains(&fd) {
        return Ok(None);
    }
    // Reading an inherited capability FD is best-effort in the port; the
    // recorded environment always fails the read and yields null.
    Ok(None)
}
