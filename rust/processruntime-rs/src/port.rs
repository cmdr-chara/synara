//! The Rust port of `packages/shared/src/processRuntime.ts`.
//!
//! Spawns host processes with the same observable contract as the Node wrappers:
//! validation order, Bun/Node error strings, the serialized ChildProcess shape and
//! the spawnSync/execSync result objects the recorder captured.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

const OPTIONS_NOT_OBJECT: &str =
    "options is not an Object. (evaluating '\"lowerPriority\" in options')";

fn undefined() -> Value {
    json!({ "$undefined": true })
}

fn is_undefined(value: &Value) -> bool {
    matches!(value, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

/// Bun's posix_spawn failure string: a PATH lookup miss for bare names, a direct
/// ENOENT for commands already containing a slash.
fn spawn_error(command: &str) -> String {
    if command.contains('/') || command.contains('\\') {
        format!("ENOENT: no such file or directory, posix_spawn '{command}'")
    } else {
        format!("Executable not found in $PATH: \"{command}\"")
    }
}

fn buffer(bytes: &[u8]) -> Value {
    json!({ "$buffer": bytes })
}

// The corpus was recorded with the volatile child `pid` frozen to 424242 (see
// rust/DECISIONS.md); SYNARA_SPAWN_PID replays that value. Unset → the real pid.
fn volatile_spawn_pid(real: u32) -> f64 {
    std::env::var("SYNARA_SPAWN_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(real as f64)
}

fn is_js_object(value: &Value) -> bool {
    // `in` works on objects, arrays and boxed marker types (Buffer, Map, Date...).
    if is_undefined(value) {
        return true; // undefined hits the `options = {}` default first
    }
    match value {
        Value::Object(_) | Value::Array(_) => {
            if let Value::Object(o) = value
                && o.len() == 1
                && let Some(k) = o.keys().next()
            {
                // Primitive markers behave like their primitive JS value.
                return !matches!(k.as_str(), "$num" | "$bigint" | "$utf16");
            }
            true
        }
        _ => false,
    }
}

fn require_spawn_options(options: &Value) -> Result<(), String> {
    if is_js_object(options) {
        Ok(())
    } else {
        Err(OPTIONS_NOT_OBJECT.to_string())
    }
}

fn require_nonempty_file(file: &str) -> Result<(), String> {
    if file.is_empty() {
        return Err("The argument 'file' cannot be empty. Received ''".to_string());
    }
    Ok(())
}

/// Where Node would resolve `command`: PATH lookup for bare names, the literal
/// path (cwd-relative) when it contains a slash.
fn resolve_executable(command: &str, env_path: Option<&str>) -> Option<String> {
    if command.contains('/') || command.contains('\\') {
        let p = std::path::Path::new(command);
        if p.is_absolute() {
            return p.exists().then(|| command.to_string());
        }
        let abs = std::env::current_dir().ok()?.join(command);
        return abs.exists().then(|| abs.to_string_lossy().into_owned());
    }
    let path = env_path
        .map(str::to_string)
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_default();
    for dir in path.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = format!("{}/{}", dir, command);
        let meta = std::fs::metadata(&candidate);
        if let Ok(m) = meta
            && m.is_file()
        {
            return Some(candidate);
        }
    }
    None
}

fn stream_in() -> Value {
    json!({
        "fd": null,
        "path": undefined(),
        "flags": "w",
        "mode": 438,
        "_writev": undefined(),
        "flush": false,
        "start": undefined(),
        "pos": undefined(),
        "bytesWritten": 0,
        "_write": { "$function": "underscoreWriteFast" },
        "write": { "$function": "writeFast" },
        "_final": { "$function": "finalFast" },
        "_events": {
            "close": undefined(),
            "error": undefined(),
            "prefinish": undefined(),
            "finish": undefined(),
            "drain": undefined(),
        },
        "_writableState": {
            "highWaterMark": 65536,
            "length": 0,
            "corked": 0,
            "onwrite": { "$function": "bound onwrite" },
            "writelen": 0,
            "bufferedIndex": 0,
            "pendingcb": 0,
        },
        "_maxListeners": undefined(),
        "_eventsCount": 1,
        "readable": false,
    })
}

fn stream_out() -> Value {
    json!({
        "_events": {
            "close": { "$function": "onceWrapper" },
            "error": undefined(),
            "data": undefined(),
            "end": undefined(),
            "readable": undefined(),
        },
        "_readableState": {
            "highWaterMark": 65536,
            "buffer": [],
            "bufferIndex": 0,
            "length": 0,
            "pipes": [],
            "awaitDrainWriters": null,
        },
        "_maxListeners": undefined(),
        "_read": { "$function": "read" },
        "_destroy": { "$function": "destroy" },
        "ref": { "$function": "ref" },
        "unref": { "$function": "unref" },
        "_eventsCount": 1,
    })
}

/// The serialized `ChildProcess` object the recorder captured: trackProcessSpawn's
/// spawn listener, spawnfile/spawnargs, and the pipe streams when the spawn went through.
fn child_object(spawnfile: &str, args: &[String], pid: Option<u32>, spawned: bool) -> Value {
    let mut spawnargs = Vec::with_capacity(args.len() + 1);
    spawnargs.push(Value::from(spawnfile));
    spawnargs.extend(args.iter().map(|a| Value::from(a.clone())));
    let mut child = Map::new();
    child.insert(
        "_events".to_string(),
        json!({ "spawn": { "$function": "onceWrapper" } }),
    );
    child.insert("_eventsCount".to_string(), json!(2));
    child.insert("_maxListeners".to_string(), undefined());
    child.insert("signalCode".to_string(), Value::Null);
    child.insert("exitCode".to_string(), Value::Null);
    child.insert("spawnfile".to_string(), Value::from(spawnfile));
    child.insert("spawnargs".to_string(), Value::Array(spawnargs));
    child.insert(
        "pid".to_string(),
        pid.map_or_else(undefined, |p| json!(volatile_spawn_pid(p))),
    );
    child.insert("channel".to_string(), undefined());
    child.insert("killed".to_string(), Value::Bool(false));
    if spawned {
        let stdin = stream_in();
        let stdout = stream_out();
        let stderr = stream_out();
        child.insert("stdin".to_string(), stdin.clone());
        child.insert("stdout".to_string(), stdout.clone());
        child.insert("stderr".to_string(), stderr.clone());
        child.insert("stdio".to_string(), json!([stdin, stdout, stderr]));
    }
    Value::Object(child)
}

/// Port of `spawnProcess` (`packages/shared/src/processRuntime.ts:119`).
pub fn spawn_process(command: String, args: Vec<String>, options: Value) -> Result<Value, String> {
    require_spawn_options(&options)?;
    require_nonempty_file(&command)?;
    match resolve_executable(&command, None) {
        Some(resolved) => {
            let spawned = std::process::Command::new(&resolved)
                .args(&args)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();
            match spawned {
                Ok(child) => Ok(child_object(&resolved, &args, Some(child.id()), true)),
                Err(_) => Ok(child_object(&command, &args, None, false)),
            }
        }
        None => Ok(child_object(&command, &args, None, false)),
    }
}

fn js_inspect(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("'{s}'"),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
    }
}

/// Port of `spawnPlannedProcess` (`packages/shared/src/processRuntime.ts:137`).
pub fn spawn_planned_process(plan: Value, options: Value) -> Result<Value, String> {
    if plan.is_null() {
        return Err("null is not an object (evaluating 'plan.command')".to_string());
    }
    // `const { platform: _p, ... } = options` throws only for null; the
    // `options = {}` default already absorbed `undefined`.
    if options.is_null() {
        return Err(
            "Cannot destructure property 'platform' from null or undefined value".to_string(),
        );
    }
    let command = match plan.get("command") {
        None | Some(Value::Null) => {
            return Err(
                "The \"file\" argument must be of type string. Received undefined".to_string(),
            );
        }
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "The \"file\" argument must be of type string. Received {}",
                js_inspect(other)
            ));
        }
    };
    let args: Vec<String> = match plan.get("args") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|a| a.as_str().map(str::to_string))
            .collect(),
        Some(_) => Vec::new(),
    };
    require_nonempty_file(&command)?;
    match resolve_executable(&command, None) {
        Some(resolved) => match std::process::Command::new(&resolved)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => Ok(child_object(&resolved, &args, Some(child.id()), true)),
            Err(_) => Ok(child_object(&command, &args, None, false)),
        },
        None => Ok(child_object(&command, &args, None, false)),
    }
}

/// Port of `spawnProcessSync` (`packages/shared/src/processRuntime.ts:174`).
pub fn spawn_process_sync(
    command: String,
    args: Vec<String>,
    options: Value,
) -> Result<Value, String> {
    require_spawn_options(&options)?;
    require_nonempty_file(&command)?;
    let resolved = match resolve_executable(&command, None) {
        Some(r) => r,
        None => {
            return Ok(json!({
                "signal": null,
                "status": null,
                "output": null,
                "pid": 0,
                "error": { "$error": spawn_error(&command) },
                "stdout": undefined(),
                "stderr": undefined(),
            }));
        }
    };
    let spawned = std::process::Command::new(&resolved)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    match spawned.and_then(|c| {
        let pid = c.id();
        c.wait_with_output().map(|o| (pid, o))
    }) {
        Ok((pid, out)) => {
            let status = out.status.code();
            let signal = if status.is_none() {
                signal_name(&out.status)
            } else {
                None
            };
            let stdout = buffer(&out.stdout);
            let stderr = buffer(&out.stderr);
            let mut obj = Map::new();
            obj.insert(
                "signal".to_string(),
                signal.map_or(Value::Null, Value::from),
            );
            obj.insert(
                "status".to_string(),
                status.map_or(Value::Null, |s| json!(s)),
            );
            obj.insert(
                "output".to_string(),
                json!([Value::Null, stdout.clone(), stderr.clone()]),
            );
            obj.insert("pid".to_string(), json!(volatile_spawn_pid(pid)));
            obj.insert("stdout".to_string(), stdout);
            obj.insert("stderr".to_string(), stderr);
            Ok(Value::Object(obj))
        }
        Err(_) => Ok(json!({
            "signal": null,
            "status": null,
            "output": null,
            "pid": 0,
            "error": { "$error": spawn_error(&command) },
            "stdout": undefined(),
            "stderr": undefined(),
        })),
    }
}

#[cfg(unix)]
fn signal_name(status: &std::process::ExitStatus) -> Option<String> {
    use std::os::unix::process::ExitStatusExt;
    let sig = status.signal()?;
    let name = match sig {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        6 => "SIGABRT",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        15 => "SIGTERM",
        _ => return Some(format!("SIG{sig}")),
    };
    Some(name.to_string())
}

#[cfg(not(unix))]
fn signal_name(_status: &std::process::ExitStatus) -> Option<String> {
    None
}

/// Port of `execShellCommandSync` (`packages/shared/src/processRuntime.ts:192`).
pub fn exec_shell_command_sync(command: String, options: Value) -> Result<Value, String> {
    if options.is_null() {
        return Err(
            "Cannot destructure property 'platform' from null or undefined value".to_string(),
        );
    }
    require_nonempty_file(&command)?;
    let platform = options
        .get("platform")
        .and_then(|p| p.as_str())
        .unwrap_or(if cfg!(windows) { "win32" } else { "darwin" });
    let output = if platform == "win32" {
        let comspec = options
            .get("env")
            .and_then(|e| e.get("ComSpec"))
            .and_then(|c| c.as_str())
            .unwrap_or("cmd.exe");
        std::process::Command::new(comspec)
            .args(["/d", "/s", "/c", &command])
            .output()
    } else {
        std::process::Command::new("/bin/sh")
            .args(["-c", &command])
            .output()
    };
    match output {
        Ok(out) if out.status.success() => Ok(buffer(&out.stdout)),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!("Command failed: {command}\n{stderr}"))
        }
        Err(e) => Err(format!("Command failed: {command}\n{e}")),
    }
}
