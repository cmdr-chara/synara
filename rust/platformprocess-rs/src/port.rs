//! The Rust port of `packages/shared/src/platformProcess.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

// Sibling unit ports, included as modules (not linked crates: a napi crate dep
// would register a colliding `rustAnythingWireInit` export).
#[path = "../../executable-rs/src/port.rs"]
pub mod executable_port;
#[path = "../../platformenvironment-rs/src/port.rs"]
pub mod platformenvironment_port;
#[path = "../../windowsprocess-rs/src/port.rs"]
pub mod windowsprocess_port;

// ---- JS value helpers --------------------------------------------------------

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.get("$undefined").and_then(Value::as_bool) == Some(true))
}

fn undefined() -> Value {
    json!({ "$undefined": true })
}

fn nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    !nullish(v)
        && match v {
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64() != Some(0.0),
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
}

fn member(v: &Value, k: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{k}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        )),
        Value::Object(o) => Ok(o.get(k).cloned().unwrap_or_else(undefined)),
        _ => Ok(undefined()),
    }
}

/// `String(v)` coercion, the subset `RegExp.test` and template use can meet here.
fn js_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                n.as_f64().map_or_else(|| "NaN".into(), |f| f.to_string())
            }
        }
        _ if is_undefined(v) => "undefined".into(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if nullish(e) {
                    String::new()
                } else {
                    js_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

/// `env ?? process.env` — a snapshot map stands in for `process.env`.
fn env_snapshot(input_env: Option<&Value>) -> Value {
    match input_env {
        Some(v) if !nullish(v) => v.clone(),
        _ => {
            let mut m = Map::new();
            for (k, v) in std::env::vars() {
                m.insert(k, Value::String(v));
            }
            Value::Object(m)
        }
    }
}

/// Node's platform spelling for the current process.
fn current_platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

// ---- OS priority shims -------------------------------------------------------

/// Best-effort `os.getPriority`/`os.setPriority`; failures are swallowed like the
/// source's try/catch (it only logs a warning, output is still `undefined`).
#[cfg(unix)]
fn try_lower_priority(pid: i32, priority: i32) {
    use rustix::process::{Pid, getpriority_process, setpriority_process};
    let who = Pid::from_raw(pid);
    if let Ok(current) = getpriority_process(who)
        && current < priority
    {
        let _ = setpriority_process(who, priority);
    }
}

#[cfg(not(unix))]
fn try_lower_priority(_pid: i32, _priority: i32) {}

/// `os.getPriority()` for the current process, floored at 5 like
/// `inheritedAgentPriority`.
#[cfg(unix)]
fn inherited_agent_priority() -> i32 {
    rustix::process::getpriority_process(None)
        .unwrap_or(0)
        .max(5)
}

#[cfg(not(unix))]
fn inherited_agent_priority() -> i32 {
    5
}

// ---- port ---------------------------------------------------------------------

const WINDOWS_COMMAND_NOT_FOUND_EXIT_CODE: f64 = 9009.0;
const WINDOWS_COMMAND_NOT_FOUND_PATTERN: &str =
    "is not recognized as an internal or external command";

const PRIORITY_EXEC_SCRIPT: &str = "renice \"$1\" -p \"$$\" >/dev/null 2>/dev/null || printf \"%s\\n\" \"Synara: failed to lower agent process priority; continuing\" >&2; shift; exec \"$@\"";

fn priority_exec_args(command: &str, args: &[String], priority: i32) -> Vec<String> {
    let mut out = vec![
        "-c".into(),
        PRIORITY_EXEC_SCRIPT.into(),
        "synara-agent-priority".into(),
        priority.to_string(),
        command.into(),
    ];
    out.extend(args.iter().cloned());
    out
}

fn is_missing_directory(path: &str) -> bool {
    match std::fs::metadata(path) {
        Ok(m) => !m.is_dir(),
        Err(e) => matches!(e.raw_os_error(), Some(2) | Some(20)), // ENOENT | ENOTDIR
    }
}

/// Port of `lowerProcessPriority` (`packages/shared/src/platformProcess.ts:25`).
pub fn lower_process_priority(pid: Option<f64>, input: serde_json::Value) -> Result<(), String> {
    let valid = pid.is_some_and(|p| p.fract() == 0.0 && p.is_finite() && p > 0.0);
    if !valid || pid == Some(f64::from(std::process::id())) {
        return Ok(());
    }
    // `input = {}` applies on `undefined` only; a null argument throws TypeError.
    let input = if is_undefined(&input) {
        json!({})
    } else {
        input
    };
    let platform_v = member(&input, "platform")?;
    let platform = if nullish(&platform_v) {
        current_platform().to_string()
    } else {
        js_string(&platform_v)
    };
    let priority = if platform == "win32" { 10 } else { 5 };
    try_lower_priority(pid.unwrap_or(0.0) as i32, priority);
    Ok(())
}

/// Port of `isCommandNotFoundExit` (`packages/shared/src/platformProcess.ts:107`).
pub fn is_command_not_found_exit(input: serde_json::Value) -> Result<bool, String> {
    let platform_v = member(&input, "platform")?;
    let platform = if nullish(&platform_v) {
        current_platform().to_string()
    } else {
        js_string(&platform_v)
    };
    if platform != "win32" {
        return Ok(false);
    }
    let code = member(&input, "code")?;
    if code.as_f64() == Some(WINDOWS_COMMAND_NOT_FOUND_EXIT_CODE) {
        return Ok(true);
    }
    let stderr = member(&input, "stderr")?;
    Ok(js_string(&stderr)
        .to_lowercase()
        .contains(WINDOWS_COMMAND_NOT_FOUND_PATTERN))
}

/// `nativeExecutable`: on win32 an explicit `.ps1` path may bypass resolution;
/// otherwise delegate to the executable crate's `resolveExecutable` port.
fn native_executable(
    command: &str,
    platform: &str,
    env: &Value,
    cwd: Option<&Value>,
) -> Result<Option<String>, String> {
    if platform == "win32"
        && executable_port::has_path_separator(command.to_string())?
        && command.to_lowercase().ends_with(".ps1")
    {
        let script_path = if win32_is_abs(command) {
            command.to_string()
        } else {
            let base = match cwd {
                Some(Value::String(s)) => s.clone(),
                _ => std::env::current_dir()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| ".".into()),
            };
            win32_resolve(&base, command)
        };
        if std::fs::metadata(&script_path).is_ok_and(|m| m.is_file()) {
            return Ok(Some(command.to_string()));
        }
        return Ok(None);
    }
    let mut opts = Map::new();
    opts.insert("platform".into(), Value::String(platform.to_string()));
    opts.insert("env".into(), env.clone());
    if let Some(c) = cwd
        && !is_undefined(c)
    {
        opts.insert("cwd".into(), c.clone());
    }
    executable_port::resolve_executable(command.to_string(), Value::Object(opts))
}

fn win32_is_abs(p: &str) -> bool {
    p.starts_with(['/', '\\'])
        || (p.len() >= 3
            && p.as_bytes()[1] == b':'
            && matches!(p.as_bytes()[2], b'/' | b'\\')
            && p.as_bytes()[0].is_ascii_alphabetic())
}

fn win32_resolve(a: &str, b: &str) -> String {
    if b.is_empty() {
        a.to_string()
    } else if win32_is_abs(b) {
        b.to_string()
    } else {
        format!("{a}\\{b}")
    }
}

/// Port of `prepareProcess` (`packages/shared/src/platformProcess.ts:177`).
pub fn prepare_process(
    command: String,
    args: Vec<String>,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let input = if is_undefined(&input) {
        json!({})
    } else {
        input
    };
    let platform_v = member(&input, "platform")?;
    let platform = if nullish(&platform_v) {
        current_platform().to_string()
    } else {
        js_string(&platform_v)
    };
    let env_v = member(&input, "env")?;
    let env = env_snapshot(Some(&env_v));
    let cwd_v = member(&input, "cwd")?;
    let lower_priority = truthy(&member(&input, "lowerPriority")?);
    let require_executable = truthy(&member(&input, "requireExecutable")?);

    if platform == "win32" && truthy(&cwd_v) {
        let wsl_workspace = windowsprocess_port::parse_windows_wsl_unc_path(cwd_v.clone())?;
        if wsl_workspace.is_some() {
            let (guest_command, guest_args) = if lower_priority {
                (
                    "/bin/sh".to_string(),
                    priority_exec_args(&command, &args, 5),
                )
            } else {
                (command.clone(), args.clone())
            };
            let mut call_input = Map::new();
            call_input.insert("platform".into(), platform_v.clone());
            call_input.insert("cwd".into(), cwd_v.clone());
            call_input.insert("env".into(), env.clone());
            let prepared = windowsprocess_port::prepare_windows_safe_process(
                guest_command,
                guest_args,
                Value::Object(call_input),
            )?;
            let mut out = prepared.as_object().cloned().unwrap_or_default();
            out.insert("requestedCommand".into(), Value::String(command.clone()));
            out.insert("resolvedCommand".into(), Value::String(command));
            out.insert("executionBackend".into(), Value::String("wsl".into()));
            out.insert(
                "priorityBeforeExec".into(),
                Value::Bool(member(&input, "lowerPriority")? == Value::Bool(true)),
            );
            return Ok(Value::Object(out));
        }
    }

    if require_executable && !is_undefined(&cwd_v) {
        match &cwd_v {
            Value::String(s) => {
                if is_missing_directory(s) {
                    return Err(format!("Working directory not found: {s}"));
                }
            }
            other => {
                return Err(format!(
                    "The argument 'path' must be of type string or an instance of Buffer or URL. Received {}",
                    js_string(other)
                ));
            }
        }
    }

    let resolved = native_executable(&command, &platform, &env, Some(&cwd_v))?;
    if require_executable && resolved.is_none() {
        return Err(format!("Command not found: {command}"));
    }
    let resolved_command = resolved.clone().unwrap_or_else(|| command.clone());

    if platform != "win32" {
        let mut priority_before_exec = lower_priority && resolved.is_some();
        if priority_before_exec {
            let mut sh_opts = Map::new();
            sh_opts.insert("platform".into(), platform_v.clone());
            sh_opts.insert("env".into(), env.clone());
            if executable_port::resolve_executable("/bin/sh".to_string(), Value::Object(sh_opts))?
                .is_none()
            {
                priority_before_exec = false;
            }
        }
        let (out_command, out_args) = if priority_before_exec {
            (
                "/bin/sh".to_string(),
                priority_exec_args(&resolved_command, &args, inherited_agent_priority()),
            )
        } else {
            (resolved_command.clone(), args.clone())
        };
        return Ok(json!({
            "command": out_command,
            "args": out_args,
            "shell": false,
            "requestedCommand": command,
            "resolvedCommand": resolved_command,
            "executionBackend": "native",
            "priorityBeforeExec": priority_before_exec,
        }));
    }

    if resolved_command.to_lowercase().ends_with(".ps1") {
        let mut ps_args = vec![
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            resolved_command.clone(),
        ];
        ps_args.extend(args.iter().cloned());
        return Ok(json!({
            "command": platformenvironment_port::resolve_windows_power_shell_executable(env)?,
            "args": ps_args,
            "shell": false,
            "windowsHide": true,
            "requestedCommand": command,
            "resolvedCommand": resolved_command,
            "executionBackend": "native",
        }));
    }

    let mut call_input = Map::new();
    call_input.insert("platform".into(), platform_v.clone());
    call_input.insert("cwd".into(), cwd_v.clone());
    call_input.insert("env".into(), env.clone());
    let prepared = windowsprocess_port::prepare_windows_safe_process(
        resolved_command.clone(),
        args,
        Value::Object(call_input),
    )?;
    let mut out = prepared.as_object().cloned().unwrap_or_default();
    out.insert("requestedCommand".into(), Value::String(command));
    out.insert("resolvedCommand".into(), Value::String(resolved_command));
    out.insert("executionBackend".into(), Value::String("native".into()));
    Ok(Value::Object(out))
}
