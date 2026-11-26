//! Stable-side probe and handoff into a parallel Synara Beta install.
//! The beta server consumes the import marker; stable never writes beta state.

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const REQUEST_FILE: &str = "import-requested.json";
const RESULT_FILE: &str = "import-result.json";
const BETA_APP_NAME: &str = "Synara Beta.app";
const BETA_EXECUTABLE_NAME: &str = "synara-shell";
const BETA_WINDOWS_GUID: &str = "a8e63b48-d4f3-4db5-9e12-368107afe65d";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetaInstallDetection {
    pub installed: bool,
    pub install_path: Option<PathBuf>,
    pub executable_path: Option<PathBuf>,
    pub version: Option<String>,
}

impl BetaInstallDetection {
    fn missing() -> Self {
        Self { installed: false, install_path: None, executable_path: None, version: None }
    }
}

pub fn beta_home_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("SYNARA_BETA_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(home);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".synara-beta")
}

fn read_mac_bundle_version(app_path: &Path) -> Option<String> {
    let plist = app_path.join("Contents/Info.plist");
    let text = std::fs::read_to_string(plist).ok()?;
    let key = text.find("CFBundleShortVersionString")?;
    let after = text.get(key..)?;
    let start = after.find("<string>")? + "<string>".len();
    let end = after.get(start..)?.find("</string>")? + start;
    Some(after[start..end].trim().to_string()).filter(|version| !version.is_empty())
}

pub fn beta_mac_install_dirs(home_dir: &Path, env: &HashMap<String, String>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(override_dir) = env.get("SYNARA_BETA_INSTALL_DIR").filter(|value| !value.trim().is_empty()) {
        dirs.push(PathBuf::from(override_dir.trim()).join(BETA_APP_NAME));
    }
    dirs.push(PathBuf::from("/Applications").join(BETA_APP_NAME));
    dirs.push(home_dir.join("Applications").join(BETA_APP_NAME));
    dirs
}

fn find_on_path(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(binary);
        if candidate.is_file() { return Some(candidate); }
    }
    None
}

pub fn detect_beta_install(home_dir: &Path, env: &HashMap<String, String>) -> BetaInstallDetection {
    #[cfg(target_os = "macos")]
    {
        for app_path in beta_mac_install_dirs(home_dir, env) {
            if app_path.exists() {
                return BetaInstallDetection {
                    installed: true,
                    install_path: Some(app_path.clone()),
                    executable_path: Some(app_path.clone()),
                    version: read_mac_bundle_version(&app_path),
                };
            }
        }
        return BetaInstallDetection::missing();
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(executable) = find_on_path("synara-beta") {
            return BetaInstallDetection {
                installed: true,
                install_path: Some(executable.clone()),
                executable_path: Some(executable),
                version: None,
            };
        }
        let desktop_file = home_dir.join(".local/share/applications/synara-beta.desktop");
        if let Ok(text) = std::fs::read_to_string(desktop_file) {
            let executable = text.lines()
                .find_map(|line| line.strip_prefix("Exec="))
                .map(|line| line.trim().trim_matches('"').split_whitespace().next().unwrap_or_default())
                .filter(|value| !value.is_empty())
                .map(PathBuf::from);
            return BetaInstallDetection {
                installed: true,
                install_path: executable.clone(),
                executable_path: executable.filter(|path| path.is_absolute()),
                version: None,
            };
        }
        return BetaInstallDetection::missing();
    }
    #[cfg(target_os = "windows")]
    {
        for hive in ["HKCU", "HKLM"] {
            let key = format!(r"{hive}\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{BETA_WINDOWS_GUID}");
            if let Ok(output) = Command::new("reg.exe").args(["query", &key, "/v", "InstallLocation"]).output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let location = stdout.lines().find_map(|line| {
                        let line = line.trim();
                        line.strip_prefix("InstallLocation")
                            .and_then(|rest| rest.split_whitespace().nth(1).map(str::to_string))
                    });
                    if let Some(location) = location {
                        let install = PathBuf::from(location);
                        let exe = install.join("Synara Beta.exe");
                        return BetaInstallDetection {
                            installed: exe.is_file(),
                            install_path: Some(install),
                            executable_path: exe.is_file().then_some(exe),
                            version: None,
                        };
                    }
                }
            }
        }
        return BetaInstallDetection::missing();
    }
    #[allow(unreachable_code)]
    BetaInstallDetection::missing()
}

pub fn is_beta_server_running(beta_home: &Path) -> bool {
    let runtime = beta_home.join("userdata/server-runtime.json");
    let Ok(text) = std::fs::read_to_string(runtime) else { return false };
    let Some(pid) = serde_json::from_str::<serde_json::Value>(&text).ok()
        .and_then(|value| value.get("pid").and_then(|pid| pid.as_i64()))
        .filter(|pid| *pid > 0) else { return false };
    unsafe { libc::kill(pid as i32, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest<'a> {
    version: u32,
    requested_at: String,
    source_home_dir: &'a str,
}

pub fn write_beta_import_request(beta_home: &Path, source_home: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(beta_home).map_err(|error| error.to_string())?;
    let target = beta_home.join(REQUEST_FILE);
    let tmp = beta_home.join(format!(".{REQUEST_FILE}.{}.tmp", std::process::id()));
    let source = if source_home.is_absolute() {
        source_home.to_path_buf()
    } else {
        std::env::current_dir().map_err(|error| error.to_string())?.join(source_home)
    };
    let body = serde_json::to_string(&ImportRequest {
        version: 1,
        requested_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        source_home_dir: &source.to_string_lossy(),
    }).map_err(|error| error.to_string())?;
    std::fs::write(&tmp, format!("{body}\n")).map_err(|error| error.to_string())?;
    std::fs::rename(&tmp, &target).map_err(|error| error.to_string())?;
    Ok(target)
}

pub fn read_beta_import_result(beta_home: &Path) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(beta_home.join(RESULT_FILE)).ok()?).ok()?;
    (value.get("version").and_then(|v| v.as_u64()) == Some(1)
        && value.get("completedAt").and_then(|v| v.as_str()).is_some()
        && value.get("ok").and_then(|v| v.as_bool()).is_some())
    .then_some(value)
}

fn scrub_launcher_overrides(command: &mut Command) {
    for key in [
        "SYNARA_HOME",
        "SYNARA_PORT",
        "SYNARA_AUTH_TOKEN",
        "SYNARA_DESKTOP_WS_URL",
        "SYNARA_DESKTOP_SHUTDOWN_TOKEN",
        "SYNARA_DESKTOP_FLAVOR",
        "VITE_DEV_SERVER_URL",
        "ELECTRON_RUN_AS_NODE",
        "SYNARA_STABLE_EXECUTABLE",
        "SYNARA_STABLE_HOME",
    ] {
        command.env_remove(key);
    }
}

pub fn launch_beta_install(
    detection: &BetaInstallDetection,
    beta_home: &Path,
    stable_home: &Path,
    stable_executable: Option<&Path>,
) -> Result<(), String> {
    if !detection.installed || detection.executable_path.is_none() {
        return Err("Synara Beta is not installed".into());
    }
    let executable = if cfg!(target_os = "macos") {
        detection.install_path.as_ref().unwrap().join("Contents/MacOS").join(BETA_EXECUTABLE_NAME)
    } else {
        detection.executable_path.clone().unwrap()
    };
    let mut command = Command::new(executable);
    scrub_launcher_overrides(&mut command);
    command
        .env("SYNARA_BETA_HOME", beta_home)
        .env("SYNARA_STABLE_HOME", stable_home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(stable_executable) = stable_executable {
        command.env("SYNARA_STABLE_EXECUTABLE", stable_executable);
    }
    command.spawn().map_err(|error| format!("Could not launch Synara Beta: {error}"))?;
    Ok(())
}

/// Finds the stable executable beta should reopen when the user leaves beta.
/// The exact handed-over executable wins; standard macOS install locations are
/// the fallback, matching upstream's stable handoff semantics.
pub fn detect_stable_executable(home_dir: &Path, env: &HashMap<String, String>) -> Option<PathBuf> {
    if let Some(handed_over) = env.get("SYNARA_STABLE_EXECUTABLE").filter(|value| !value.trim().is_empty()) {
        let path = PathBuf::from(handed_over.trim());
        if path.is_absolute() && path.is_file() {
            return Some(path);
        }
    }
    if cfg!(target_os = "macos") {
        for app_path in [PathBuf::from("/Applications/Synara.app"), home_dir.join("Applications/Synara.app")] {
            let executable = app_path.join("Contents/MacOS/synara-shell");
            if executable.is_file() {
                return Some(executable);
            }
        }
    }
    None
}

/// Opens stable from beta without copying beta data back. Launcher-only and
/// beta-specific overrides are stripped; the stable data home is restored only
/// when stable handed one over at beta launch.
pub fn launch_stable(executable: &Path, stable_home: Option<&Path>) -> Result<(), String> {
    let mut command = Command::new(executable);
    scrub_launcher_overrides(&mut command);
    command.env_remove("SYNARA_BETA_HOME");
    if let Some(stable_home) = stable_home {
        command.env("SYNARA_HOME", stable_home);
    }
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    command.spawn().map_err(|error| format!("Could not launch Synara: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!("synara-beta-channel-test-{}", uuid_like()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn uuid_like() -> String {
        format!("{}-{}", std::process::id(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
    }

    #[test]
    fn writes_import_request_atomically_with_absolute_source_home() {
        let root = temp_root();
        let beta = root.join("beta-home");
        write_beta_import_request(&beta, Path::new("relative-home")).unwrap();
        let request: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(beta.join(REQUEST_FILE)).unwrap()).unwrap();
        assert_eq!(request["version"], 1);
        assert!(request["requestedAt"].as_str().unwrap().contains('T'));
        assert!(request["sourceHomeDir"].as_str().unwrap().starts_with('/'));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn import_result_requires_the_shared_shape() {
        let root = temp_root();
        std::fs::write(root.join(RESULT_FILE), "{\"version\":1,\"completedAt\":\"now\",\"ok\":true}").unwrap();
        assert!(read_beta_import_result(&root).is_some());
        std::fs::write(root.join(RESULT_FILE), "{\"version\":2,\"ok\":true}").unwrap();
        assert!(read_beta_import_result(&root).is_none());
        let _ = std::fs::remove_dir_all(root);
    }


    #[test]
    fn handed_over_stable_executable_wins_when_absolute_and_present() {
        let root = temp_root();
        let executable = root.join("synara-shell");
        std::fs::write(&executable, "x").unwrap();
        let env = HashMap::from([("SYNARA_STABLE_EXECUTABLE".into(), executable.to_string_lossy().into_owned())]);
        assert_eq!(detect_stable_executable(&root, &env), Some(executable));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn mac_probe_honors_install_dir_override_first() {
        let root = temp_root();
        let app = root.join("custom").join(BETA_APP_NAME);
        std::fs::create_dir_all(&app).unwrap();
        let env = HashMap::from([("SYNARA_BETA_INSTALL_DIR".into(), root.join("custom").to_string_lossy().into_owned())]);
        let dirs = beta_mac_install_dirs(&root, &env);
        assert_eq!(dirs.first(), Some(&app));
        let _ = std::fs::remove_dir_all(root);
    }
}
