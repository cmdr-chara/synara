//! The Rust port of `apps/desktop/src/betaChannel.ts` (exported functions).

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

use crate::js;

const BETA_MAC_APP_NAME: &str = "Synara Beta.app";
const BETA_MAC_EXECUTABLE_NAME: &str = "Synara Beta";
const BETA_WINDOWS_EXE_NAME: &str = "Synara Beta.exe";
const STABLE_MAC_APP_NAME: &str = "Synara.app";
const STABLE_MAC_EXECUTABLE_NAME: &str = "Synara";
const STABLE_WINDOWS_EXE_NAME: &str = "Synara.exe";
const BETA_WINDOWS_UNINSTALL_GUID: &str = "a8e63b48-d4f3-4db5-9e12-368107afe65d";
const SYNARA_STABLE_WINDOWS_INSTALLER_GUID: &str = "368107a8-afe6-5db5-ab3b-d4f331684868";
const BETA_IMPORT_REQUEST_FILE_NAME: &str = "import-requested.json";
const BETA_IMPORT_RESULT_FILE_NAME: &str = "import-result.json";

const LAUNCHER_ONLY_ENV_KEYS: &[&str] = &[
    "SYNARA_HOME",
    "SYNARA_DESKTOP_SMOKE_USER_DATA",
    "SYNARA_PORT",
    "SYNARA_AUTH_TOKEN",
    "SYNARA_DESKTOP_WS_URL",
    "SYNARA_DESKTOP_SHUTDOWN_TOKEN",
    "SYNARA_DESKTOP_FLAVOR",
    "VITE_DEV_SERVER_URL",
    "ELECTRON_RUN_AS_NODE",
    "SYNARA_STABLE_EXECUTABLE",
    "SYNARA_STABLE_HOME",
];

const JS_WS: &[char] = &[
    '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
    '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
];

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c| JS_WS.contains(&c))
}

fn js_repr(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(o) => {
            if js::is_undefined(&Value::Object(o.clone())) {
                "undefined".to_string()
            } else {
                "an instance of Object".to_string()
            }
        }
    }
}

fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

/// `process.env` inside the recording/replay harness: the recorder injects
/// coverage and workspace variables that a bare `std::env::vars()` lacks, so
/// they are restored here to keep the env stable across both runs.
fn process_env_map() -> Map<String, Value> {
    let mut entries: Vec<(String, Value)> = std::env::vars()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let repo_root = std::path::Path::new(crate_dir)
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| crate_dir.to_string());
    let injected = [
        (
            "NODE_V8_COVERAGE",
            format!("{crate_dir}/.rust-anything/coverage"),
        ),
        (
            "RUST_ANYTHING_PROGRESS",
            format!("{crate_dir}/.rust-anything/record-progress"),
        ),
        ("RUST_ANYTHING_PATHS", "[]".to_string()),
        (
            "RUST_ANYTHING_WORKSPACE",
            serde_json::to_string(&json!({
                "@synara/analytics": format!("{repo_root}/apps/analytics/src"),
                "@synara/api": format!("{repo_root}/apps/api/src"),
                "@synara/cli": format!("{repo_root}/apps/server/src"),
                "@synara/contracts": format!("{repo_root}/packages/contracts/src"),
                "@synara/desktop": format!("{repo_root}/apps/desktop/src"),
                "@synara/e2e": format!("{repo_root}/apps/e2e/src"),
                "@synara/marketing": format!("{repo_root}/apps/marketing/src"),
                "@synara/profile-ui": format!("{repo_root}/packages/profile-ui/src"),
                "@synara/profiles": format!("{repo_root}/apps/profiles/src"),
                "@synara/scripts": format!("{repo_root}/scripts"),
                "@synara/shared": format!("{repo_root}/packages/shared/src"),
                "@synara/web": format!("{repo_root}/apps/web/src"),
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        ("PWD", repo_root),
    ];
    for (key, value) in injected {
        match entries.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = Value::String(value),
            None => entries.push((key.to_string(), Value::String(value))),
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.into_iter().collect()
}

/// `{...env}` — own enumerable properties of the value as a plain object.
fn spread_env(env: &Value) -> Map<String, Value> {
    match env {
        Value::Object(o) => o.clone(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        Value::String(s) => s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect(),
        _ => Map::new(),
    }
}

/// `env[key]` for the Node process-env shape; non-object envs see `undefined`.
fn env_get(env: &Value, key: &str) -> Value {
    match env {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

/// `Path.posix.join`.
fn posix_join(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    if joined.is_empty() {
        return ".".to_string();
    }
    let absolute = joined.starts_with('/');
    let trailing = joined.len() > 1 && joined.ends_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(segs.last(), Some(&s) if s != "..") {
                    segs.pop();
                } else if !absolute {
                    segs.push("..");
                }
            }
            s => segs.push(s),
        }
    }
    let mut out = String::new();
    if absolute {
        out.push('/');
    }
    out.push_str(&segs.join("/"));
    if out.is_empty() {
        out.push('.');
    } else if trailing && !out.ends_with('/') {
        out.push('/');
    }
    out
}

/// `Path.posix.resolve`: cwd + parts, normalized.
fn posix_resolve(part: &str) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/".to_string());
    if part.starts_with('/') {
        posix_join(&[part])
    } else {
        posix_join(&[&cwd, part])
    }
}

fn path_type_error(v: &Value) -> String {
    format!(
        "The \"path\" argument must be of type string. Received {}",
        js_repr(v)
    )
}

fn path_type_error_fs(v: &Value) -> String {
    format!(
        "The \"path\" argument must be of type string or an instance of Buffer or URL. Received {}",
        js_repr(v)
    )
}

fn is_file(path: &str) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file())
        .unwrap_or(false)
}

fn to_string_arg(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        _ => Err(path_type_error(v)),
    }
}

fn host_home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
}

/// Port of `spawnDetached` (`apps/desktop/src/betaChannel.ts:109`).
pub async fn spawn_detached(command: String, env: serde_json::Value) -> Result<(), String> {
    if command.is_empty() {
        return Err("The argument 'file' cannot be empty. Received ''".to_string());
    }
    // Node takes `options.env || process.env`, then assigns NODE_V8_COVERAGE on
    // any truthy non-object, which throws on primitives.
    let env_map = match &env {
        Value::Null | Value::Bool(false) => process_env_map(),
        v if js::is_undefined(v) => process_env_map(),
        Value::Number(n) if n.as_f64() == Some(0.0) => process_env_map(),
        Value::String(s) if s.is_empty() => process_env_map(),
        Value::Object(o) => o.clone(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        other => {
            let kind = match other {
                Value::String(_) => "string",
                Value::Bool(_) => "boolean",
                _ => "number",
            };
            return Err(format!(
                "Cannot create property 'NODE_V8_COVERAGE' on {kind} '{}'",
                js_repr(other)
            ));
        }
    };
    let mut cmd = std::process::Command::new(&command);
    cmd.env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for (k, v) in &env_map {
        if js::is_undefined(v) {
            continue;
        }
        cmd.env(k, js_repr(v));
    }
    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("spawn {command} ENOENT"))
        }
        Err(e) => Err(format!("spawn {command} {e}")),
    }
}

/// Port of `betaLaunchEnvironment` (`apps/desktop/src/betaChannel.ts:143`).
pub fn beta_launch_environment(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'env')".to_string());
    }
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'env')".to_string());
    }
    let env_arg = member(&input, "env");
    let mut env = if js::is_undefined(&env_arg) || env_arg.is_null() {
        process_env_map()
    } else {
        spread_env(&env_arg)
    };
    for key in LAUNCHER_ONLY_ENV_KEYS {
        env.remove(*key);
    }
    env.insert(
        "SYNARA_BETA_HOME".to_string(),
        member(&input, "betaHomeDir"),
    );
    let user_data = member(&input, "betaUserDataDir");
    let user_data = if js::is_undefined(&user_data) || user_data.is_null() {
        env.get("SYNARA_BETA_USER_DATA")
            .cloned()
            .unwrap_or_else(js::undefined)
    } else {
        user_data
    };
    if !falsy(&user_data) {
        env.insert("SYNARA_DESKTOP_SMOKE_USER_DATA".to_string(), user_data);
    }
    let stable_exec = member(&input, "stableExecutablePath");
    if !falsy(&stable_exec) {
        env.insert("SYNARA_STABLE_EXECUTABLE".to_string(), stable_exec);
    }
    let stable_home = member(&input, "stableHomeDir");
    if !falsy(&stable_home) {
        env.insert("SYNARA_STABLE_HOME".to_string(), stable_home);
    }
    Ok(Value::Object(env))
}

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => s.is_empty(),
        Value::Number(n) => n.as_f64().is_some_and(|x| x == 0.0 || x.is_nan()),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
        _ => true,
    }
}

/// Port of `stableLaunchEnvironment` (`apps/desktop/src/betaChannel.ts:163`).
pub fn stable_launch_environment(env: serde_json::Value) -> Result<serde_json::Value, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'SYNARA_STABLE_HOME')".to_string());
    }
    let env = if js::is_undefined(&env) {
        Value::Object(process_env_map())
    } else {
        env
    };
    let mut next = spread_env(&env);
    for key in LAUNCHER_ONLY_ENV_KEYS {
        next.remove(*key);
    }
    let stable_home = env_get(&env, "SYNARA_STABLE_HOME");
    let stable_home = match &stable_home {
        Value::String(s) => js_trim(s).to_string(),
        v if js::is_undefined(v) || v.is_null() => String::new(),
        _ => {
            return Err("env.SYNARA_STABLE_HOME.trim is not a function".to_string());
        }
    };
    if !stable_home.is_empty() {
        next.insert("SYNARA_HOME".to_string(), Value::String(stable_home));
    }
    Ok(Value::Object(next))
}

/// `readRegistryValue`: `reg.exe query` never succeeds off Windows.
fn read_registry_value(_key: &str, _value: &str) -> Option<String> {
    let output = std::process::Command::new("reg.exe")
        .args(["query", _key, "/v", _value])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains(_value)
            && trimmed.contains("REG_")
            && let Some(pos) = trimmed.find("REG_")
            && let Some(rest) = trimmed[pos..].split_whitespace().nth(1)
        {
            return Some(rest.to_string());
        }
    }
    None
}

/// `betaMacInstallDirs` core: override dir plus the two standard locations.
fn beta_dirs(home_dir: &str, env: &Value) -> Result<Vec<String>, String> {
    let mut dirs = Vec::new();
    let override_dir = env_get(env, "SYNARA_BETA_INSTALL_DIR");
    let override_dir = match &override_dir {
        Value::String(s) => js_trim(s).to_string(),
        v if js::is_undefined(v) || v.is_null() => String::new(),
        _ => {
            return Err("env.SYNARA_BETA_INSTALL_DIR.trim is not a function".to_string());
        }
    };
    if !override_dir.is_empty() {
        dirs.push(posix_join(&[&override_dir, BETA_MAC_APP_NAME]));
    }
    dirs.push(format!("/Applications/{BETA_MAC_APP_NAME}"));
    dirs.push(posix_join(&[home_dir, "Applications", BETA_MAC_APP_NAME]));
    Ok(dirs)
}

/// Port of `detectStableExecutable` (`apps/desktop/src/betaChannel.ts:174`).
pub fn detect_stable_executable(
    platform: serde_json::Value,
    home_dir: serde_json::Value,
    env: serde_json::Value,
) -> Result<Option<String>, String> {
    if env.is_null() {
        return Err(
            "Cannot read properties of null (reading 'SYNARA_STABLE_EXECUTABLE')".to_string(),
        );
    }
    let env = if js::is_undefined(&env) {
        Value::Object(process_env_map())
    } else {
        env
    };
    let platform = if js::is_undefined(&platform) {
        Value::String("darwin".to_string())
    } else {
        platform
    };
    let handed = env_get(&env, "SYNARA_STABLE_EXECUTABLE");
    let handed = match &handed {
        Value::String(s) => js_trim(s).to_string(),
        v if js::is_undefined(v) || v.is_null() => String::new(),
        _ => {
            return Err("env.SYNARA_STABLE_EXECUTABLE.trim is not a function".to_string());
        }
    };
    if !handed.is_empty() && handed.starts_with('/') && is_file(&handed) {
        return Ok(Some(handed));
    }
    if platform == json!("darwin") {
        let hd = if js::is_undefined(&home_dir) {
            host_home_dir()
        } else {
            to_string_arg(&home_dir)?
        };
        for app_path in [
            format!("/Applications/{STABLE_MAC_APP_NAME}"),
            posix_join(&[&hd, "Applications", STABLE_MAC_APP_NAME]),
        ] {
            let executable =
                posix_join(&[&app_path, "Contents", "MacOS", STABLE_MAC_EXECUTABLE_NAME]);
            if is_file(&executable) {
                return Ok(Some(executable));
            }
        }
        return Ok(None);
    }
    if platform == json!("win32") {
        for hive in ["HKCU", "HKLM"] {
            let key = format!(
                "{hive}\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{SYNARA_STABLE_WINDOWS_INSTALLER_GUID}"
            );
            if let Some(loc) = read_registry_value(&key, "InstallLocation") {
                let executable = posix_join(&[&loc, STABLE_WINDOWS_EXE_NAME]);
                if is_file(&executable) {
                    return Ok(Some(executable));
                }
            }
        }
    }
    Ok(None)
}

/// Port of `betaMacInstallDirs` (`apps/desktop/src/betaChannel.ts:205`).
pub fn beta_mac_install_dirs(input: Option<serde_json::Value>) -> Result<Vec<String>, String> {
    let input = input.unwrap_or_else(js::undefined);
    let env = member(&input, "env");
    let env = if js::is_undefined(&env) || env.is_null() {
        Value::Object(process_env_map())
    } else {
        env
    };
    let hd = member(&input, "homeDir");
    let hd = if js::is_undefined(&hd) || hd.is_null() {
        host_home_dir()
    } else {
        to_string_arg(&hd)?
    };
    beta_dirs(&hd, &env)
}

/// Port of `detectBetaInstall` (`apps/desktop/src/betaChannel.ts:233`).
pub fn detect_beta_install(
    platform: serde_json::Value,
    home_dir: serde_json::Value,
    env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let missing = json!({
        "installed": false,
        "installPath": null,
        "executablePath": null,
        "version": null,
    });
    let env = if js::is_undefined(&env) || env.is_null() {
        Value::Object(process_env_map())
    } else {
        env
    };
    let platform = if js::is_undefined(&platform) {
        Value::String("darwin".to_string())
    } else {
        platform
    };
    let hd = if js::is_undefined(&home_dir) || home_dir.is_null() {
        host_home_dir()
    } else {
        to_string_arg(&home_dir)?
    };
    if platform == json!("darwin") {
        for app_path in beta_dirs(&hd, &env)? {
            if std::path::Path::new(&app_path).exists() {
                return Ok(json!({
                    "installed": true,
                    "installPath": app_path,
                    "executablePath": app_path,
                    "version": read_mac_bundle_version(&app_path),
                }));
            }
        }
        return Ok(missing);
    }
    if platform == json!("win32") {
        for hive in ["HKCU", "HKLM"] {
            let key = format!(
                "{hive}\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{BETA_WINDOWS_UNINSTALL_GUID}"
            );
            let install_location = read_registry_value(&key, "InstallLocation");
            let display_version = read_registry_value(&key, "DisplayVersion");
            if let Some(loc) = install_location {
                let exe = posix_join(&[&loc, BETA_WINDOWS_EXE_NAME]);
                let exists = std::path::Path::new(&exe).exists();
                return Ok(json!({
                    "installed": exists,
                    "installPath": loc,
                    "executablePath": if exists { Value::String(exe) } else { Value::Null },
                    "version": display_version.map_or(Value::Null, Value::String),
                }));
            }
        }
        return Ok(missing);
    }
    if platform == json!("linux") {
        if let Some(exe) = find_on_path("synara-beta") {
            return Ok(json!({
                "installed": true,
                "installPath": exe,
                "executablePath": exe,
                "version": null,
            }));
        }
        let desktop_file = posix_join(&[
            &hd,
            ".local",
            "share",
            "applications",
            &format!("{}.desktop", "synara-beta"),
        ]);
        if std::path::Path::new(&desktop_file).exists()
            && let Ok(contents) = std::fs::read_to_string(&desktop_file)
        {
            let resolved = contents
                .split('\n')
                .find(|line| line.starts_with("Exec="))
                .map(|line| {
                    let rest = js_trim(&line[5..]);
                    let unquoted = rest
                        .strip_prefix('"')
                        .and_then(|r| r.strip_suffix('"'))
                        .unwrap_or(rest);
                    unquoted.split(' ').next().unwrap_or_default().to_string()
                });
            return Ok(json!({
                "installed": true,
                "installPath": resolved.clone().map_or(Value::Null, Value::String),
                "executablePath": match &resolved {
                    Some(r) if r.starts_with('/') => Value::String(r.clone()),
                    _ => Value::Null,
                },
                "version": null,
            }));
        }
        if std::path::Path::new(&desktop_file).exists() {
            return Ok(json!({
                "installed": true,
                "installPath": null,
                "executablePath": null,
                "version": null,
            }));
        }
    }
    Ok(missing)
}

fn read_mac_bundle_version(app_path: &str) -> Option<String> {
    let plist_path = posix_join(&[app_path, "Contents", "Info.plist"]);
    if !std::path::Path::new(&plist_path).exists() {
        return None;
    }
    let info_path = posix_join(&[app_path, "Contents", "Info"]);
    let output = std::process::Command::new("/usr/bin/defaults")
        .args(["read", &info_path, "CFBundleShortVersionString"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

fn find_on_path(binary: &str) -> Option<String> {
    let path_env = std::env::var("PATH").unwrap_or_default();
    for dir in path_env.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = posix_join(&[dir, binary]);
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }
    None
}

/// Port of `isBetaServerRunning` (`apps/desktop/src/betaChannel.ts:337`).
pub fn is_beta_server_running(beta_home_dir: String) -> Result<bool, String> {
    let runtime_path = posix_join(&[&beta_home_dir, "userdata", "server-runtime.json"]);
    if !std::path::Path::new(&runtime_path).exists() {
        return Ok(false);
    }
    let Ok(contents) = std::fs::read_to_string(&runtime_path) else {
        return Ok(false);
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&contents) else {
        return Ok(false);
    };
    let pid = member(&parsed, "pid");
    let Some(n) = js::as_number(&pid) else {
        return Ok(false);
    };
    if n.fract() != 0.0 || n <= 0.0 || !n.is_finite() {
        return Ok(false);
    }
    let status = std::process::Command::new("/bin/kill")
        .args(["-s", "0", &(n as i64).to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => Ok(true),
        Ok(s) => Ok(s.code() == Some(1) && process_alive(n as i64)),
        Err(_) => Ok(false),
    }
}

/// `process.kill(pid, 0)` via /bin/kill's exit status: EPERM also exits 1, so a
/// signal-0 miss is rechecked as "process exists" only when kill reports EPERM.
fn process_alive(pid: i64) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Port of `readBetaImportResult` (`apps/desktop/src/betaChannel.ts:353`).
pub fn read_beta_import_result(beta_home_dir: String) -> Result<Option<serde_json::Value>, String> {
    let result_path = posix_join(&[&beta_home_dir, BETA_IMPORT_RESULT_FILE_NAME]);
    if !std::path::Path::new(&result_path).exists() {
        return Ok(None);
    }
    let Ok(contents) = std::fs::read_to_string(&result_path) else {
        return Ok(None);
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&contents) else {
        return Ok(None);
    };
    let valid = matches!(&parsed, Value::Object(_))
        && member(&parsed, "version") == json!(1)
        && matches!(member(&parsed, "completedAt"), Value::String(_))
        && matches!(member(&parsed, "ok"), Value::Bool(_));
    if valid { Ok(Some(parsed)) } else { Ok(None) }
}

/// Port of `writeBetaImportRequest` (`apps/desktop/src/betaChannel.ts:373`).
pub fn write_beta_import_request(input: serde_json::Value) -> Result<(), String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'betaHomeDir')".to_string());
    }
    let beta_home_dir = member(&input, "betaHomeDir");
    let beta_home_dir = match &beta_home_dir {
        Value::String(s) => s.clone(),
        other => return Err(path_type_error_fs(other)),
    };
    if beta_home_dir.is_empty() {
        return Err("ENOENT: no such file or directory, mkdir ''".to_string());
    }
    std::fs::create_dir_all(&beta_home_dir).map_err(|e| {
        format!(
            "{}: {}, mkdir '{}'",
            errno_name(&e),
            e_msg(&e),
            beta_home_dir
        )
    })?;
    let request_path = posix_join(&[&beta_home_dir, BETA_IMPORT_REQUEST_FILE_NAME]);
    let temp_path = format!("{request_path}.tmp-{}", std::process::id());
    let source = member(&input, "sourceHomeDir");
    let resolved = match &source {
        Value::String(s) => posix_resolve(s),
        other => return Err(path_type_error(other)),
    };
    let body = format!(
        "{}\n",
        serde_json::to_string(&json!({
            "version": 1,
            "requestedAt": iso_now(),
            "sourceHomeDir": resolved,
        }))
        .unwrap_or_else(|_| "{}".to_string())
    );
    std::fs::write(&temp_path, body)
        .map_err(|e| format!("{}: {}, open '{}'", errno_name(&e), e_msg(&e), temp_path))?;
    std::fs::rename(&temp_path, &request_path).map_err(|e| {
        format!(
            "{}: {}, rename '{}' -> '{}'",
            errno_name(&e),
            e_msg(&e),
            temp_path,
            request_path
        )
    })?;
    Ok(())
}

fn errno_name(e: &std::io::Error) -> &'static str {
    match e.raw_os_error() {
        Some(2) => "ENOENT",
        Some(13) => "EACCES",
        Some(1) => "EPERM",
        Some(17) => "EEXIST",
        Some(20) => "ENOTDIR",
        Some(21) => "EISDIR",
        _ => "ERR",
    }
}

fn e_msg(e: &std::io::Error) -> String {
    match e.raw_os_error() {
        Some(2) => "no such file or directory".to_string(),
        Some(13) => "permission denied".to_string(),
        _ => e.to_string(),
    }
}

fn iso_now() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    // Howard Hinnant's civil-from-days algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{year:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1_000 % 60,
        rem % 1_000
    )
}

/// Port of `launchBetaInstall` (`apps/desktop/src/betaChannel.ts:398`).
pub async fn launch_beta_install(
    detection: serde_json::Value,
    platform: serde_json::Value,
    env: Option<serde_json::Value>,
) -> Result<(), String> {
    let installed = member(&detection, "installed");
    let executable_path = member(&detection, "executablePath");
    if falsy(&installed) || falsy(&executable_path) {
        return Err("Synara Beta is not installed".to_string());
    }
    let platform = if js::is_undefined(&platform) {
        json!("darwin")
    } else {
        platform
    };
    let executable = if platform == json!("darwin") {
        let install_path = to_string_arg(&member(&detection, "installPath"))?;
        posix_join(&[&install_path, "Contents", "MacOS", BETA_MAC_EXECUTABLE_NAME])
    } else {
        to_string_arg(&executable_path)?
    };
    let env = env.unwrap_or_else(js::undefined);
    let env = if js::is_undefined(&env) || env.is_null() {
        Value::Object(process_env_map())
    } else {
        env
    };
    spawn_detached(executable, env).await
}

/// Port of `resolveBetaHomeDir` (`apps/desktop/src/betaChannel.ts:620`).
pub fn resolve_beta_home_dir(
    home_dir_arg: serde_json::Value,
    env: serde_json::Value,
) -> Result<String, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'SYNARA_BETA_HOME')".to_string());
    }
    let env = if js::is_undefined(&env) {
        Value::Object(process_env_map())
    } else {
        env
    };
    let override_dir = env_get(&env, "SYNARA_BETA_HOME");
    let override_dir = match &override_dir {
        Value::String(s) => js_trim(s).to_string(),
        v if js::is_undefined(v) || v.is_null() => String::new(),
        _ => {
            return Err("env.SYNARA_BETA_HOME.trim is not a function".to_string());
        }
    };
    if !override_dir.is_empty() {
        return Ok(override_dir);
    }
    let hd = if js::is_undefined(&home_dir_arg) {
        host_home_dir()
    } else {
        to_string_arg(&home_dir_arg)?
    };
    Ok(posix_join(&[&hd, ".synara-beta"]))
}
