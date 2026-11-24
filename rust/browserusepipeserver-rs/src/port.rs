//! The Rust port of `apps/desktop/src/browserUsePipeServer.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

const PIPE_DIR: &str = "synara-browser-host";
const HOST_PIPE_ENV: &str = "SYNARA_BROWSER_HOST_PIPE_PATH";
const USE_PIPE_ENV: &str = "SYNARA_BROWSER_USE_PIPE_PATH";
const CAPABILITY_ENV: &str = "SYNARA_BROWSER_HOST_CAPABILITY";
const CAPABILITY_FD_ENV: &str = "SYNARA_BROWSER_HOST_CAPABILITY_FD";

// The corpus was recorded with `process.pid`, `process.getuid()` and
// `Crypto.randomUUID()` frozen; the env vars
// replay those frozen constants. Production leaves them unset → real values.
fn volatile_pid() -> f64 {
    std::env::var("SYNARA_PIPE_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| std::process::id() as f64)
}

fn volatile_uid() -> Option<i64> {
    if let Ok(s) = std::env::var("SYNARA_PIPE_UID") {
        return s.trim().parse().ok();
    }
    if cfg!(windows) {
        return None;
    }
    let out = std::process::Command::new("id").arg("-u").output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn volatile_uuid() -> String {
    if let Ok(s) = std::env::var("SYNARA_PIPE_UUID") {
        return s;
    }
    let mut bytes = [0u8; 16];
    let filled = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut bytes))
        .is_ok();
    if !filled {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let mut h = RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        bytes[..8].copy_from_slice(&h.finish().to_le_bytes());
        h.write_u64(std::process::id() as u64);
        bytes[8..].copy_from_slice(&h.finish().to_le_bytes());
    }
    bytes[6] = bytes[6] & 0x0f | 0x40;
    bytes[8] = bytes[8] & 0x3f | 0x80;
    let h = |b: &[u8]| b.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        h(&bytes[0..4]),
        h(&bytes[4..6]),
        h(&bytes[6..8]),
        h(&bytes[8..10]),
        h(&bytes[10..16])
    )
}

fn host_platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        "ios" => "darwin",
        "android" => "android",
        other => other,
    }
}

fn is_nil(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn js_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => crate::js::string_units(v).map(|u| String::from_utf16_lossy(&u)),
    }
}

/// `String(value)` — the implicit conversion template literals use.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                if is_nil(item) {
                    String::new()
                } else {
                    js_to_string(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if let Some(units) = crate::js::string_units(v) {
                return String::from_utf16_lossy(&units);
            }
            if let Some(n) = crate::js::as_number(v) {
                return crate::js::number_to_string(n);
            }
            if o.len() == 1 {
                if let Some(digits) = o.get("$bigint").and_then(Value::as_str) {
                    return digits.to_string();
                }
                if o.contains_key("$undefined") {
                    return "undefined".into();
                }
            }
            "[object Object]".into()
        }
    }
}

/// `str.trim()` under JS whitespace rules (Unicode White_Space plus U+FEFF).
fn js_trim(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
        .to_string()
}

fn lookup<'v>(v: &'v Value, key: &str) -> Option<&'v Value> {
    v.as_object().and_then(|o| o.get(key))
}

/// Port of `resolveDefaultBrowserHostPipePath` (`apps/desktop/src/browserUsePipeServer.ts:110`).
pub fn resolve_default_browser_host_pipe_path(
    platform: serde_json::Value,
    pid: serde_json::Value,
) -> Result<String, String> {
    let pid = if crate::js::is_undefined(&pid) {
        crate::js::number(volatile_pid())
    } else {
        pid
    };
    let suffix = format!("{}-{}", js_to_string(&pid), volatile_uuid());
    let platform = if crate::js::is_undefined(&platform) {
        Value::String(host_platform().into())
    } else {
        platform
    };
    if js_string(&platform).as_deref() == Some("win32") {
        return Ok(format!("\\\\.\\pipe\\{PIPE_DIR}-{suffix}"));
    }
    let dir = match volatile_uid() {
        Some(uid) => format!("{PIPE_DIR}-{uid}"),
        None => PIPE_DIR.to_string(),
    };
    Ok(format!("/tmp/{dir}/{suffix}.sock"))
}

/// `value?.trim()` where `value` is an env member; `None` mirrors `undefined`/`null`.
fn trimmed_env(env: &Value, key: &str) -> Result<Option<String>, String> {
    let value = match lookup(env, key) {
        Some(v) => v,
        None => return Ok(None),
    };
    if is_nil(value) {
        return Ok(None);
    }
    match js_string(value) {
        Some(s) => Ok(Some(js_trim(&s))),
        None => Err(format!("env[{key}]?.trim is not a function")),
    }
}

/// Port of `resolveConfiguredBrowserHostPipePath` (`apps/desktop/src/browserUsePipeServer.ts:126`).
pub fn resolve_configured_browser_host_pipe_path(
    env: serde_json::Value,
    platform: serde_json::Value,
) -> Result<String, String> {
    let process_env;
    let env = if crate::js::is_undefined(&env) {
        process_env = Value::Object(
            std::env::vars()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        );
        &process_env
    } else {
        &env
    };
    if env.is_null() {
        return Err(format!(
            "Cannot read properties of null (reading '{HOST_PIPE_ENV}')"
        ));
    }
    let configured = match trimmed_env(env, HOST_PIPE_ENV)? {
        Some(s) if !s.is_empty() => Some(s),
        _ => trimmed_env(env, USE_PIPE_ENV)?.filter(|s| !s.is_empty()),
    };
    match configured {
        Some(path) => Ok(path),
        None => resolve_default_browser_host_pipe_path(platform, crate::js::undefined()),
    }
}

/// `{ ...value }`: own enumerable keys of objects/arrays/strings; primitives give none.
fn spread(value: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    match value {
        Value::Object(o) => {
            if let Some(units) = crate::js::string_units(value) {
                spread_units(&units, &mut out);
            } else if o.len() == 1
                && (o.contains_key("$undefined")
                    || o.contains_key("$num")
                    || o.contains_key("$bigint")
                    || o.contains_key("$date")
                    || o.contains_key("$map")
                    || o.contains_key("$set"))
            {
                // Primitive markers spread nothing; Map/Set carry no own keys.
            } else if let Some(bytes) = o
                .get("$bytes")
                .or_else(|| o.get("$buffer"))
                .and_then(Value::as_array)
                .filter(|_| o.len() == 1)
            {
                for (i, b) in bytes.iter().enumerate() {
                    out.insert(i.to_string(), b.clone());
                }
            } else {
                for (k, v) in o {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                out.insert(i.to_string(), item.clone());
            }
        }
        Value::String(s) => {
            let units: Vec<u16> = s.encode_utf16().collect();
            spread_units(&units, &mut out);
        }
        _ => {}
    }
    out
}

fn spread_units(units: &[u16], out: &mut Map<String, Value>) {
    for (i, u) in units.iter().enumerate() {
        out.insert(i.to_string(), crate::js::utf16_string(&[*u]));
    }
}

/// Port of `resolveBrowserHostPipeBackendEnv` (`apps/desktop/src/browserUsePipeServer.ts:137`).
pub fn resolve_browser_host_pipe_backend_env(
    inherited_env: serde_json::Value,
    active_pipe_path: Option<String>,
    capability_fd: Option<f64>,
) -> Result<serde_json::Value, String> {
    let mut env = spread(&inherited_env);
    for key in [
        HOST_PIPE_ENV,
        USE_PIPE_ENV,
        CAPABILITY_ENV,
        CAPABILITY_FD_ENV,
    ] {
        env.shift_remove(key);
    }
    let pipe_path = active_pipe_path.as_deref().map(js_trim);
    if let Some(path) = pipe_path.filter(|p| !p.is_empty())
        && let Some(fd) =
            capability_fd.filter(|fd| fd.fract() == 0.0 && fd.is_finite() && *fd >= 3.0)
    {
        env.insert(HOST_PIPE_ENV.into(), Value::String(path.clone()));
        env.insert(USE_PIPE_ENV.into(), Value::String(path));
        env.insert(
            CAPABILITY_FD_ENV.into(),
            Value::String(crate::js::number_to_string(fd)),
        );
    }
    Ok(Value::Object(env))
}
