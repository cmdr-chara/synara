//! Shared login-shell environment hydration for the desktop shell and backend.
//! GUI launches often inherit a truncated service environment rather than the
//! user's terminal environment. The probe is cached because interactive shell
//! startup can cost about a second.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const HYDRATED_ENV_NAME: &str = "SYNARA_PATH_HYDRATED";
pub const HYDRATED_ENV_VALUE: &str = "1";
pub const LOGIN_SHELL_ENVIRONMENT_CACHE_FILE_NAME: &str = "login-shell-environment.json";

const CACHE_VERSION: u32 = 1;
const CACHE_MAX_AGE_MS: u64 = 7 * 24 * 60 * 60 * 1000;
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_NAMES: [&str; 7] = [
    "PATH",
    "SSH_AUTH_SOCK",
    "HOMEBREW_PREFIX",
    "HOMEBREW_CELLAR",
    "HOMEBREW_REPOSITORY",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellEnvironmentSyncResult {
    pub path_hydrated: bool,
}

pub fn is_shell_environment_hydrated() -> bool {
    std::env::var(HYDRATED_ENV_NAME).ok().as_deref() == Some(HYDRATED_ENV_VALUE)
        && std::env::var("PATH").ok().is_some_and(|path| !path.trim().is_empty())
}

pub fn apply_hydration_marker(hydrated: bool) {
    if hydrated {
        std::env::set_var(HYDRATED_ENV_NAME, HYDRATED_ENV_VALUE);
    } else {
        std::env::remove_var(HYDRATED_ENV_NAME);
    }
}

/// Desktop startup hydration. Mutates this process before the backend is
/// spawned, so the backend and every provider inherits the corrected values.
#[cfg(windows)]
pub fn sync_shell_environment(home_dir: &Path) -> ShellEnvironmentSyncResult {
    let _ = home_dir;
    sync_windows_environment()
}

/// Desktop startup hydration. Mutates this process before the backend is
/// spawned, so the backend and every provider inherits the corrected values.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn sync_shell_environment(home_dir: &Path) -> ShellEnvironmentSyncResult {
    let mut shell_environment = HashMap::new();
    for shell in login_shell_candidates() {
        match read_login_environment_cached(&shell, home_dir) {
            Ok(values) => {
                shell_environment = values;
                if shell_environment.contains_key("PATH") {
                    break;
                }
            }
            Err(error) => eprintln!(
                "shell environment: failed to read login shell environment from {shell}: {error}"
            ),
        }
    }

    #[cfg(target_os = "macos")]
    let launchctl_path = if shell_environment.contains_key("PATH") {
        None
    } else {
        read_launchctl_path()
    };
    #[cfg(not(target_os = "macos"))]
    let launchctl_path: Option<String> = None;
    let resolved_path = shell_environment
        .get("PATH")
        .cloned()
        .or(launchctl_path);
    let inherited_path = std::env::var("PATH").ok();
    if let Some(merged) = merge_path_entries(
        resolved_path.as_deref(),
        inherited_path.as_deref(),
        ':',
        false,
    ) {
        std::env::set_var("PATH", merged);
    }
    for name in PROBE_NAMES.iter().copied().filter(|name| *name != "PATH") {
        if std::env::var_os(name).is_none() {
            if let Some(value) = shell_environment.get(name) {
                std::env::set_var(name, value);
            }
        }
    }

    ShellEnvironmentSyncResult {
        path_hydrated: resolved_path.is_some(),
    }
}

/// Desktop startup hydration is a no-op outside the supported desktop targets.
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub fn sync_shell_environment(home_dir: &Path) -> ShellEnvironmentSyncResult {
    let _ = home_dir;
    ShellEnvironmentSyncResult { path_hydrated: false }
}

/// Backend startup PATH hydration. Unlike the desktop pass this does not stamp
/// the child-process marker: the marker means an ancestor already hydrated PATH.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn fix_path(home_dir: &Path) {
    if is_shell_environment_hydrated() {
        return;
    }
    let inherited_path = std::env::var("PATH").ok();
    let mut shell_path = None;
    for shell in login_shell_candidates() {
        match read_login_environment_cached(&shell, home_dir) {
            Ok(values) => {
                shell_path = values.get("PATH").cloned();
                if shell_path.is_some() {
                    break;
                }
            }
            Err(error) => eprintln!(
                "shell environment: failed to read PATH from login shell {shell}: {error}"
            ),
        }
    }
    #[cfg(target_os = "macos")]
    let resolved_path = shell_path.or_else(read_launchctl_path);
    #[cfg(not(target_os = "macos"))]
    let resolved_path = shell_path;
    if let Some(merged) = merge_path_entries(
        resolved_path.as_deref(),
        inherited_path.as_deref(),
        ':',
        false,
    ) {
        std::env::set_var("PATH", merged);
    }
}

/// Backend startup PATH hydration is a no-op outside macOS/Linux.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn fix_path(home_dir: &Path) {
    let _ = home_dir;
}

#[cfg(windows)]
fn sync_windows_environment() -> ShellEnvironmentSyncResult {
    match read_windows_persistent_environment() {
        Ok(persisted) => {
            let inherited_path = std::env::var("PATH").ok();
            let persisted_path = persisted
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("PATH"))
                .map(|(_, value)| value.clone());
            if let Some(merged) = merge_path_entries(
                persisted_path.as_deref(),
                inherited_path.as_deref(),
                ';',
                true,
            ) {
                std::env::set_var("PATH", merged);
            }
            for (name, value) in persisted {
                if name.eq_ignore_ascii_case("PATH") || std::env::var_os(&name).is_some() {
                    continue;
                }
                std::env::set_var(name, value);
            }
            ShellEnvironmentSyncResult {
                path_hydrated: persisted_path.is_some(),
            }
        }
        Err(error) => {
            eprintln!("shell environment: failed to read Windows environment: {error}");
            ShellEnvironmentSyncResult { path_hydrated: false }
        }
    }
}

fn merge_path_entries(
    preferred_path: Option<&str>,
    inherited_path: Option<&str>,
    delimiter: char,
    windows: bool,
) -> Option<String> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();
    for value in [preferred_path, inherited_path].into_iter().flatten() {
        for entry in value.split(delimiter) {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                continue;
            }
            let key = if windows {
                trimmed
                    .trim_end_matches(['/', '\\'])
                    .to_ascii_lowercase()
            } else {
                trimmed.to_string()
            };
            if seen.insert(key) {
                merged.push(trimmed.to_string());
            }
        }
    }
    (!merged.is_empty()).then(|| merged.join(&delimiter.to_string()))
}

fn trim_non_empty(value: Option<String>) -> Option<String> {
    let trimmed = value?.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn login_shell_candidates() -> Vec<String> {
    let fallback = if cfg!(target_os = "macos") {
        Some("/bin/zsh".to_string())
    } else if cfg!(target_os = "linux") {
        Some("/bin/bash".to_string())
    } else {
        None
    };
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for candidate in [
        trim_non_empty(std::env::var("SHELL").ok()),
        user_login_shell(),
        fallback,
    ]
    .into_iter()
    .flatten()
    {
        if seen.insert(candidate.clone()) {
            candidates.push(candidate);
        }
    }
    candidates
}

#[cfg(unix)]
fn user_login_shell() -> Option<String> {
    unsafe {
        let passwd = libc::getpwuid(libc::getuid());
        if passwd.is_null() || (*passwd).pw_shell.is_null() {
            return None;
        }
        trim_non_empty(Some(
            CStr::from_ptr((*passwd).pw_shell)
                .to_string_lossy()
                .into_owned(),
        ))
    }
}

#[cfg(not(unix))]
fn user_login_shell() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn read_launchctl_path() -> Option<String> {
    let output = Command::new("/bin/launchctl")
        .args(["getenv", "PATH"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output.status.success().then(|| {
        trim_non_empty(String::from_utf8(output.stdout).ok())
    })?
}

fn is_valid_environment_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn capture_start(name: &str) -> String {
    format!("__SYNARA_ENV_{name}_START__")
}

fn capture_end(name: &str) -> String {
    format!("__SYNARA_ENV_{name}_END__")
}

fn build_capture_command(names: &[&str], nushell: bool) -> Result<String, String> {
    let mut parts = Vec::new();
    for name in names {
        if !is_valid_environment_name(name) {
            return Err(format!("unsupported environment variable name: {name}"));
        }
        if nushell {
            parts.push(format!(
                "print '{}'; do {{ ^printenv {name} }} | complete | get stdout | print --no-newline; print '{}'",
                capture_start(name),
                capture_end(name)
            ));
        } else {
            parts.push(format!(
                "printf '%s\\n' '{}'; printenv {name} || true; printf '%s\\n' '{}'",
                capture_start(name),
                capture_end(name)
            ));
        }
    }
    Ok(parts.join("; "))
}

fn extract_environment_value(output: &str, name: &str) -> Option<String> {
    let start = output.find(&capture_start(name))? + capture_start(name).len();
    let end = output[start..].find(&capture_end(name))? + start;
    let mut value = &output[start..end];
    if let Some(rest) = value.strip_prefix('\n') {
        value = rest;
    }
    if let Some(rest) = value.strip_suffix('\n') {
        value = rest;
    }
    (!value.is_empty()).then(|| value.to_string())
}

fn run_with_timeout(mut child: std::process::Child) -> Result<String, String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "probe stdout unavailable".to_string())?;
    let reader = thread::spawn(move || {
        let mut output = String::new();
        let mut stdout = stdout;
        let _ = stdout.read_to_string(&mut output);
        output
    });
    let deadline = Instant::now() + PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                return Ok(reader.join().unwrap_or_default());
            }
            Ok(Some(status)) => {
                let _ = reader.join();
                return Err(format!("probe exited with {status}"));
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err("probe timed out".into());
            }
            Err(error) => return Err(format!("could not wait for probe: {error}")),
        }
    }
}

fn read_environment_from_login_shell(shell: &str, names: &[&str]) -> Result<HashMap<String, String>, String> {
    if names.is_empty() {
        return Ok(HashMap::new());
    }
    let nushell = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("nu") || name.eq_ignore_ascii_case("nu.exe"));
    let command = build_capture_command(names, nushell)?;
    let args: Vec<&str> = if nushell {
        vec!["--login", "--interactive", "--commands", &command]
    } else {
        vec!["-ilc", &command]
    };
    let mut command_builder = Command::new(shell);
    command_builder
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command_builder.creation_flags(CREATE_NO_WINDOW);
    }
    let child = command_builder
        .spawn()
        .map_err(|error| format!("could not start {shell}: {error}"))?;
    let output = run_with_timeout(child)?;
    Ok(names
        .iter()
        .filter_map(|name| {
            extract_environment_value(&output, name).map(|value| (name.to_string(), value))
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StartupFileFingerprint {
    path: String,
    mtime_ms: u64,
    size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CacheKey {
    version: u32,
    platform: String,
    shell: String,
    uid: Option<u32>,
    home_directory: String,
    startup_files: Vec<StartupFileFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    key: CacheKey,
    captured_at_ms: u64,
    names: Vec<String>,
    environment: HashMap<String, String>,
}

fn cache_path(home_dir: &Path) -> PathBuf {
    home_dir
        .join("cache")
        .join(LOGIN_SHELL_ENVIRONMENT_CACHE_FILE_NAME)
}

fn unix_millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn now_millis() -> u64 {
    unix_millis(SystemTime::now())
}

fn fingerprint_file(path: &Path) -> Option<StartupFileFingerprint> {
    let metadata = fs::metadata(path).ok()?;
    metadata.is_file().then(|| StartupFileFingerprint {
        path: path.to_string_lossy().into_owned(),
        mtime_ms: metadata.modified().map(unix_millis).unwrap_or(0),
        size: metadata.len(),
    })
}

fn shell_startup_files(shell: &str, home_directory: &Path) -> Vec<PathBuf> {
    let shell_name = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if shell_name == "zsh" {
        return [
            "/etc/zshenv",
            "/etc/zprofile",
            "/etc/zshrc",
            "/etc/zlogin",
            "/etc/zsh/zshenv",
            "/etc/zsh/zprofile",
            "/etc/zsh/zshrc",
            "/etc/zsh/zlogin",
        ]
        .into_iter()
        .map(PathBuf::from)
        .chain([".zshenv", ".zprofile", ".zshrc", ".zlogin"].into_iter().map(|name| home_directory.join(name)))
        .collect();
    }
    if shell_name == "bash" || shell_name == "sh" {
        return ["/etc/profile", "/etc/bashrc", "/etc/bash.bashrc"]
            .into_iter()
            .map(PathBuf::from)
            .chain(
                [".bash_profile", ".bash_login", ".bashrc", ".profile"]
                    .into_iter()
                    .map(|name| home_directory.join(name)),
            )
            .collect();
    }
    if shell_name == "fish" {
        return vec![
            PathBuf::from("/etc/fish/config.fish"),
            home_directory.join(".config/fish/config.fish"),
        ];
    }
    if shell_name.eq_ignore_ascii_case("nu") || shell_name.eq_ignore_ascii_case("nu.exe") {
        let config = if cfg!(target_os = "macos") {
            home_directory.join("Library/Application Support/nushell")
        } else if cfg!(windows) {
            home_directory.join("AppData/Roaming/nushell")
        } else {
            home_directory.join(".config/nushell")
        };
        return ["env.nu", "config.nu", "login.nu"]
            .into_iter()
            .map(|name| config.join(name))
            .collect();
    }
    vec![PathBuf::from("/etc/profile"), home_directory.join(".profile")]
}

fn compute_cache_key(shell: &str, home_directory: &Path) -> CacheKey {
    let startup_files = std::iter::once(PathBuf::from(shell))
        .chain(shell_startup_files(shell, home_directory))
        .filter_map(|path| fingerprint_file(&path))
        .collect();
    #[cfg(unix)]
    let uid = Some(unsafe { libc::getuid() });
    #[cfg(not(unix))]
    let uid = None;
    CacheKey {
        version: CACHE_VERSION,
        platform: std::env::consts::OS.into(),
        shell: shell.into(),
        uid,
        home_directory: home_directory.to_string_lossy().into_owned(),
        startup_files,
    }
}

fn user_home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn read_cache_entry(path: &Path) -> Option<CacheEntry> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn write_cache_entry(path: &Path, entry: &CacheEntry) {
    let Some(parent) = path.parent() else { return };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let temporary = path.with_extension(format!("json.{}.partial", std::process::id()));
    let write = (|| -> std::io::Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        use std::io::Write;
        file.write_all(serde_json::to_string(entry).map_err(std::io::Error::other)?.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if write.is_err() {
        let _ = fs::remove_file(temporary);
    }
}

fn reusable_cached_environment(entry: &CacheEntry) -> Option<HashMap<String, String>> {
    let mut environment = entry.environment.clone();
    let current_socket = trim_non_empty(std::env::var("SSH_AUTH_SOCK").ok());
    if let Some(current_socket) = current_socket {
        environment.insert("SSH_AUTH_SOCK".into(), current_socket);
        return Some(environment);
    }
    if let Some(cached_socket) = environment.get("SSH_AUTH_SOCK") {
        if !Path::new(cached_socket).exists() {
            return None;
        }
    }
    Some(environment)
}

fn read_login_environment_cached(
    shell: &str,
    synara_home: &Path,
) -> Result<HashMap<String, String>, String> {
    let key = compute_cache_key(shell, &user_home_directory());
    let path = cache_path(synara_home);
    if let Some(entry) = read_cache_entry(&path) {
        if entry.key == key
            && now_millis().saturating_sub(entry.captured_at_ms) < CACHE_MAX_AGE_MS
            && PROBE_NAMES.iter().all(|name| entry.names.iter().any(|cached| cached == name))
        {
            if let Some(environment) = reusable_cached_environment(&entry) {
                return Ok(environment);
            }
        }
    }

    let environment = read_environment_from_login_shell(shell, &PROBE_NAMES)?;
    if environment.contains_key("PATH") {
        write_cache_entry(
            &path,
            &CacheEntry {
                key,
                captured_at_ms: now_millis(),
                names: PROBE_NAMES.iter().map(|name| name.to_string()).collect(),
                environment: environment.clone(),
            },
        );
    }
    Ok(environment)
}

#[cfg(windows)]
fn read_windows_persistent_environment() -> Result<HashMap<String, String>, String> {
    let script = "$ErrorActionPreference='Stop';function dump($s){$m=[ordered]@{};$v=[Environment]::GetEnvironmentVariables($s);foreach($k in $v.Keys){$m[[string]$k]=[Environment]::ExpandEnvironmentVariables([string]$v[$k])};$m};$o=[ordered]@{machine=(dump 'Machine');user=(dump 'User')};[Console]::Out.Write(($o|ConvertTo-Json -Compress -Depth 3))";
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let powershell = format!("{system_root}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    let mut command = Command::new(powershell);
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped());
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
    let child = command.spawn().map_err(|error| error.to_string())?;
    let output = run_with_timeout(child)?;
    let parsed: serde_json::Value = serde_json::from_str(output.trim()).map_err(|error| error.to_string())?;
    let mut merged = HashMap::new();
    let mut key_by_lower = HashMap::<String, String>::new();
    for scope in ["machine", "user"] {
        if let Some(values) = parsed.get(scope).and_then(serde_json::Value::as_object) {
            for (name, value) in values {
                let Some(value) = value.as_str().map(str::trim).filter(|value| !value.is_empty()) else {
                    continue;
                };
                if name.eq_ignore_ascii_case("PATH") {
                    continue;
                }
                let lower = name.to_ascii_lowercase();
                if let Some(existing) = key_by_lower.get(&lower) {
                    merged.remove(existing);
                }
                key_by_lower.insert(lower, name.clone());
                merged.insert(name.clone(), value.to_string());
            }
        }
    }
    let machine_path = parsed
        .get("machine")
        .and_then(serde_json::Value::as_object)
        .and_then(|values| values.iter().find(|(name, _)| name.eq_ignore_ascii_case("PATH")))
        .and_then(|(_, value)| value.as_str());
    let user_path = parsed
        .get("user")
        .and_then(serde_json::Value::as_object)
        .and_then(|values| values.iter().find(|(name, _)| name.eq_ignore_ascii_case("PATH")))
        .and_then(|(_, value)| value.as_str());
    let combined = [machine_path, user_path]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(";");
    if !combined.is_empty() {
        merged.insert("PATH".into(), combined);
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_path_entries_dedupes_and_prefers_login_shell() {
        assert_eq!(
            merge_path_entries(
                Some("/opt/homebrew/bin:/usr/bin"),
                Some("/Users/test/.local/bin:/usr/bin"),
                ':',
                false,
            ),
            Some("/opt/homebrew/bin:/usr/bin:/Users/test/.local/bin".into())
        );
    }

    #[test]
    fn windows_path_merge_normalizes_case_and_trailing_separators() {
        assert_eq!(
            merge_path_entries(
                Some("C:\\Tools\\;C:\\Windows"),
                Some("c:\\tools;C:\\Users\\test"),
                ';',
                true,
            ),
            Some("C:\\Tools\\;C:\\Windows;C:\\Users\\test".into())
        );
    }

    #[test]
    fn capture_output_extracts_multiline_values() {
        let output = "noise\n__SYNARA_ENV_PATH_START__\n/a:/b\n__SYNARA_ENV_PATH_END__\nmore";
        assert_eq!(extract_environment_value(output, "PATH"), Some("/a:/b".into()));
        assert_eq!(extract_environment_value(output, "MISSING"), None);
    }

    #[test]
    fn capture_command_rejects_invalid_names() {
        assert!(build_capture_command(&["BAD;rm"], false).is_err());
        assert!(build_capture_command(&["PATH"], false).is_ok());
    }

    #[test]
    fn cache_key_tracks_shell_and_platform() {
        let home = Path::new("/tmp/synara-test-home");
        let key = compute_cache_key("/bin/zsh", home);
        assert_eq!(key.version, CACHE_VERSION);
        assert_eq!(key.shell, "/bin/zsh");
        assert_eq!(key.platform, std::env::consts::OS);
    }


    #[cfg(target_os = "macos")]
    #[test]
    fn posix_probe_extracts_environment_values() {
        let environment = read_environment_from_login_shell("/bin/sh", &["PATH"])
            .expect("/bin/sh probe");
        assert!(environment
            .get("PATH")
            .is_some_and(|path| path.split(':').any(|entry| entry == "/usr/bin")));
    }
}
