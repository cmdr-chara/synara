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
                        line.strip_prefix("InstallLocation").and_then(|rest| {
                            let rest = rest.trim();
                            rest.strip_prefix("REG_EXPAND_SZ")
                                .or_else(|| rest.strip_prefix("REG_SZ"))
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .map(str::to_string)
                        })
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
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
    }
    #[cfg(windows)]
    {
        Command::new("tasklist.exe")
            .args(["/FO", "CSV", "/NH", "/FI", &format!("PID eq {pid}")])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
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

// ---------------------------------------------------------------------------
// macOS beta installer: download the newest beta zip from the update feed,
// verify checksum and bundle identity, then commit it into /Applications with
// same-volume staging and recovery of an interrupted swap.
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
const BETA_BUNDLE_ID: &str = "com.emanueledipietro.synara.beta";
#[cfg(target_os = "macos")]
const BETA_FEED_ATOM: &str = "https://github.com/Emanuele-web04/synara/releases.atom";

#[cfg(target_os = "macos")]
fn validate_feed_url(url: &str) -> Result<(), String> {
    if url.starts_with("https://") { return Ok(()); }
    if let Some(rest) = url.strip_prefix("http://") {
        let authority = rest.split('/').next().unwrap_or_default();
        let host_port = authority.rsplit('@').next().unwrap_or(authority);
        let host = host_port.trim_start_matches('[').split([':', ']']).next().unwrap_or_default();
        if matches!(host, "127.0.0.1" | "localhost" | "::1") { return Ok(()); }
        return Err(format!("Refusing plain-HTTP beta feed for non-loopback host {host}."));
    }
    Err("Unsupported beta feed protocol.".into())
}

#[cfg(target_os = "macos")]
fn resolve_redirect(current: &str, location: &str) -> String {
    if location.starts_with("http://") || location.starts_with("https://") {
        return location.to_string();
    }
    let base = current.rsplit_once('/').map(|(base, _)| base).unwrap_or(current);
    if location.starts_with('/') {
        let scheme_host = current.split('/').take(3).collect::<Vec<_>>().join("/");
        format!("{scheme_host}{location}")
    } else {
        format!("{base}/{location}")
    }
}

#[cfg(target_os = "macos")]
fn feed_response(url: &str) -> Result<ureq::Response, String> {
    let agent = ureq::AgentBuilder::new().redirects(0).build();
    let mut current = url.to_string();
    for _ in 0..=5 {
        validate_feed_url(&current)?;
        let response = agent.get(&current)
            .set("user-agent", "synara-desktop")
            .timeout(std::time::Duration::from_secs(45))
            .call()
            .map_err(|error| format!("Could not download beta feed: {error}"))?;
        let status = response.status();
        if (300..400).contains(&status) {
            let location = response.header("location").ok_or("Beta feed redirect missing its location.")?;
            current = resolve_redirect(&current, location);
            continue;
        }
        return Ok(response);
    }
    Err("Beta feed redirected too many times.".into())
}

#[cfg(target_os = "macos")]
fn fetch_text(url: &str) -> Result<String, String> {
    feed_response(url)?
        .into_string()
        .map_err(|error| format!("Could not read beta feed: {error}"))
}

#[cfg(target_os = "macos")]
fn download_file(url: &str, target: &Path) -> Result<(), String> {
    let response = feed_response(url)?;
    let mut reader = response.into_reader();
    let mut output = std::fs::File::create(target).map_err(|error| error.to_string())?;
    std::io::copy(&mut reader, &mut output).map_err(|error| format!("Could not save Synara Beta: {error}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn verify_beta_app_bundle(app_path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(app_path)
        .map_err(|_| format!("The download did not contain {}.", BETA_APP_NAME))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("The beta download isn't signed by Synara. It wasn't installed.".into());
    }
    let plist = std::fs::read_to_string(app_path.join("Contents/Info.plist"))
        .map_err(|_| format!("The download did not contain {}.", BETA_APP_NAME))?;
    let bundle_id = plist
        .split("<key>CFBundleIdentifier</key>")
        .nth(1)
        .and_then(|rest| rest.split("<string>").nth(1))
        .and_then(|rest| rest.split("</string>").next())
        .map(str::trim);
    if bundle_id != Some(BETA_BUNDLE_ID) {
        return Err(format!("Downloaded app is not Synara Beta (bundle id {}).", bundle_id.unwrap_or("missing")));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn verify_beta_signature(app_path: &Path, expected_team_id: Option<&str>) -> Result<(), String> {
    let Some(expected_team_id) = expected_team_id else { return Ok(()) };
    let unsigned = "The beta download isn't signed by Synara. It wasn't installed.";
    let verify = Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(app_path)
        .output()
        .map_err(|error| format!("Couldn't check the beta download's signature: {error}"))?;
    if !verify.status.success() { return Err(unsigned.into()); }
    let info = Command::new("codesign")
        .args(["-dv", "--verbose=4"])
        .arg(app_path)
        .output()
        .map_err(|error| format!("Couldn't check the beta download's signature: {error}"))?;
    let stderr = String::from_utf8_lossy(&info.stderr);
    let team = stderr.lines().find_map(|line| line.strip_prefix("TeamIdentifier="));
    if team != Some(expected_team_id) { return Err(unsigned.into()); }
    Ok(())
}

#[cfg(target_os = "macos")]
fn verify_beta_install_bundle(app_path: &Path, expected_team_id: Option<&str>) -> Result<(), String> {
    verify_beta_app_bundle(app_path)?;
    verify_beta_signature(app_path, expected_team_id)
}

#[cfg(target_os = "macos")]
fn beta_feed_manifest_url(feed_override: Option<&str>) -> Result<String, String> {
    if let Some(base) = feed_override.filter(|value| !value.trim().is_empty()) {
        return Ok(format!("{}/beta-mac.yml", base.trim_end_matches('/')));
    }
    let atom = fetch_text(BETA_FEED_ATOM)?;
    let tag = crate::updater::github_release_tags_from_atom(&atom)
        .into_iter()
        .find(|tag| crate::updater::version_allowed_for_flavor(tag.trim_start_matches('v'), crate::updater::UpdateFlavor::Beta))
        .ok_or("No Synara Beta release was found.")?;
    Ok(format!("https://github.com/Emanuele-web04/synara/releases/download/{tag}/beta-mac.yml"))
}

/// Download and install the newest beta build. This is macOS-only like the
/// upstream installer; Windows/Linux beta handoff uses already installed apps.
#[cfg(target_os = "macos")]
pub fn install_beta_from_feed(
    install_dir: &Path,
    feed_override: Option<&str>,
    expected_team_id: Option<&str>,
) -> Result<PathBuf, String> {
    let target = install_dir.join(BETA_APP_NAME);
    let previous = install_dir.join(format!("{BETA_APP_NAME}.previous"));
    if std::fs::symlink_metadata(&previous).is_ok() {
        if verify_beta_install_bundle(&target, expected_team_id).is_err() {
            verify_beta_install_bundle(&previous, expected_team_id)?;
            let _ = std::fs::remove_dir_all(&target);
            std::fs::rename(&previous, &target).map_err(|error| format!("recover previous beta: {error}"))?;
        } else {
            let _ = std::fs::remove_dir_all(&previous);
        }
    }

    let work = std::env::temp_dir().join(format!("synara-beta-install-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&work).map_err(|error| error.to_string())?;
    let result = (|| {
        let manifest_url = beta_feed_manifest_url(feed_override)?;
        let manifest_text = fetch_text(&manifest_url)?;
        let manifest = crate::updater::parse_update_manifest(&manifest_text)?;
        let arch = if std::env::consts::ARCH == "aarch64" { "arm64" } else { "x64" };
        let file = crate::updater::select_update_download_file(&manifest.files, arch, ".zip")?;
        let base = manifest_url.rsplit_once('/').map(|(base, _)| base).unwrap_or("");
        let zip_url = if file.url.starts_with("http://") || file.url.starts_with("https://") {
            file.url.clone()
        } else {
            format!("{base}/{}", file.url.trim_start_matches('/'))
        };
        let zip_path = work.join(file.url.rsplit('/').next().unwrap_or("synara-beta.zip"));
        download_file(&zip_url, &zip_path)?;
        let artifact = crate::updater::fingerprint_artifact(&zip_path)?;
        let digest = artifact.sha512.clone();
        if !digest.eq_ignore_ascii_case(&file.sha512) {
            let raw = decode_hex_sha512(&digest).unwrap_or_default();
            if base64::Engine::encode(&base64::engine::general_purpose::STANDARD, raw) != file.sha512 {
                return Err("The beta download failed its checksum. Try again.".into());
            }
        }
        let extracted = work.join("extracted");
        std::fs::create_dir_all(&extracted).map_err(|error| error.to_string())?;
        let ditto = Command::new("ditto").args(["-x", "-k"]).arg(&zip_path).arg(&extracted).output()
            .map_err(|error| format!("Could not extract Synara Beta: {error}"))?;
        if !ditto.status.success() {
            return Err(format!("Could not extract Synara Beta: {}", String::from_utf8_lossy(&ditto.stderr).trim()));
        }
        let app = extracted.join(BETA_APP_NAME);
        verify_beta_install_bundle(&app, expected_team_id)?;
        std::fs::create_dir_all(install_dir).map_err(|error| error.to_string())?;
        let staging = install_dir.join(format!(".synara-beta-install-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
        let staged_app = staging.join(BETA_APP_NAME);
        let install_result = (|| {
            // `mv` handles a cross-volume install destination; a direct rename
            // only works when /tmp and /Applications share one filesystem.
            let moved = Command::new("mv").arg(&app).arg(&staged_app).output()
                .map_err(|error| format!("stage beta app: {error}"))?;
            if !moved.status.success() {
                return Err(format!("stage beta app: {}", String::from_utf8_lossy(&moved.stderr).trim()));
            }
            let had_previous = std::fs::symlink_metadata(&target).is_ok();
            if had_previous {
                std::fs::rename(&target, &previous).map_err(|error| format!("move previous beta aside: {error}"))?;
            }
            if let Err(error) = std::fs::rename(&staged_app, &target) {
                if had_previous { let _ = std::fs::rename(&previous, &target); }
                return Err(format!("install beta app: {error}"));
            }
            let _ = std::fs::remove_dir_all(&previous);
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(&staging);
        install_result?;
        Ok(target.clone())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

#[cfg(target_os = "macos")]
fn decode_hex_sha512(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 { return None; }
    let mut out = Vec::new();
    for pair in value.as_bytes().chunks_exact(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Stable-side channel orchestration, mirroring upstream DesktopBetaChannel.
// The native UI/RPC layer can call these verbs without owning any policy.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetaActionResult {
    pub ok: bool,
    pub error: Option<&'static str>,
    pub message: Option<String>,
}

fn beta_action(ok: bool, error: Option<&'static str>, message: Option<String>) -> BetaActionResult {
    BetaActionResult { ok, error, message }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetaChannelState {
    pub supported: bool,
    pub flavor: &'static str,
    pub stable_installed: bool,
    pub installed: bool,
    pub version: Option<String>,
    pub can_install: bool,
    pub running: bool,
    pub last_import_at: Option<String>,
    pub last_import_error: Option<String>,
}

pub struct DesktopBetaChannel {
    beta_home: PathBuf,
    stable_home: PathBuf,
    user_home: PathBuf,
    env: HashMap<String, String>,
}

impl DesktopBetaChannel {
    pub fn current() -> Self {
        Self {
            beta_home: beta_home_dir(),
            stable_home: crate::backend::configured_home_dir(),
            user_home: std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")),
            env: std::env::vars().collect(),
        }
    }

    fn detect(&self) -> BetaInstallDetection {
        detect_beta_install(&self.user_home, &self.env)
    }

    fn can_install(&self) -> bool {
        cfg!(target_os = "macos")
    }

    fn production_only(&self) -> Option<BetaActionResult> {
        (crate::desktop_identity::runtime_flavor() != "production").then(|| {
            beta_action(false, Some("not-supported"), Some("Beta handoff is only available from stable Synara.".into()))
        })
    }

    pub fn state(&self) -> BetaChannelState {
        let flavor = crate::desktop_identity::runtime_flavor();
        let supported = flavor != "remote";
        let detection = supported.then(|| self.detect());
        let result = supported.then(|| read_beta_import_result(&self.beta_home)).flatten();
        let stable = flavor == "beta" && detect_stable_executable(&self.user_home, &self.env).is_some();
        BetaChannelState {
            supported,
            flavor,
            stable_installed: stable,
            installed: detection.as_ref().is_some_and(|detection| detection.installed),
            version: detection.and_then(|detection| detection.version),
            can_install: supported && self.can_install(),
            running: supported && is_beta_server_running(&self.beta_home),
            last_import_at: result.as_ref().filter(|value| value.get("ok").and_then(|ok| ok.as_bool()) == Some(true))
                .and_then(|value| value.get("completedAt").and_then(|completed| completed.as_str()).map(str::to_string)),
            last_import_error: result.as_ref().filter(|value| value.get("ok").and_then(|ok| ok.as_bool()) == Some(false))
                .map(|value| value.get("error").and_then(|error| error.as_str()).unwrap_or("import failed").to_string()),
        }
    }

    #[cfg(target_os = "macos")]
    fn current_team_id() -> Option<String> {
        let executable = std::env::current_exe().ok()?;
        let output = Command::new("codesign").args(["-dv", "--verbose=4"]).arg(executable).output().ok()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        stderr.lines()
            .find_map(|line| line.strip_prefix("TeamIdentifier="))
            .filter(|team| *team != "not set")
            .map(str::to_string)
    }

    fn ensure_installed(&self) -> Result<(), BetaActionResult> {
        if self.detect().installed { return Ok(()); }
        if !self.can_install() {
            return Err(beta_action(false, Some("not-installed"), Some("Synara Beta is not installed yet.".into())));
        }
        #[cfg(target_os = "macos")]
        {
            let install_dir = self.env.get("SYNARA_BETA_INSTALL_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/Applications"));
            let feed = self.env.get("SYNARA_BETA_FEED_URL").map(String::as_str);
            install_beta_from_feed(&install_dir, feed, Self::current_team_id().as_deref())
                .map_err(|message| beta_action(false, Some("install-failed"), Some(message)))?;
            if self.detect().installed { return Ok(()); }
            return Err(beta_action(false, Some("install-failed"), Some("The beta install finished but the app was not found.".into())));
        }
        #[allow(unreachable_code)]
        Err(beta_action(false, Some("not-installed"), Some("Synara Beta is not installed yet.".into())))
    }

    pub fn launch(&self) -> BetaActionResult {
        if let Some(result) = self.production_only() { return result; }
        let detection = self.detect();
        if !detection.installed || detection.executable_path.is_none() {
            return beta_action(false, Some("not-installed"), Some("Synara Beta is not installed yet.".into()));
        }
        let stable_executable = std::env::current_exe().ok();
        match launch_beta_install(&detection, &self.beta_home, &self.stable_home, stable_executable.as_deref()) {
            Ok(()) => beta_action(true, None, None),
            Err(message) => beta_action(false, Some("launch-failed"), Some(message)),
        }
    }

    pub fn install(&self) -> BetaActionResult {
        if let Some(result) = self.production_only() { return result; }
        match self.ensure_installed() {
            Ok(()) => self.launch(),
            Err(result) => result,
        }
    }

    pub fn import_and_launch(&self) -> BetaActionResult {
        if let Some(result) = self.production_only() { return result; }
        if let Err(result) = self.ensure_installed() { return result; }
        let detection = self.detect();
        if !detection.installed || detection.executable_path.is_none() {
            return beta_action(false, Some("not-installed"), Some("Synara Beta is not installed yet.".into()));
        }
        if is_beta_server_running(&self.beta_home) {
            return beta_action(false, Some("beta-running"), Some("Quit Synara Beta first so it can pick up the import on its next launch.".into()));
        }
        if let Err(message) = write_beta_import_request(&self.beta_home, &self.stable_home) {
            return beta_action(false, Some("internal"), Some(message));
        }
        let stable_executable = std::env::current_exe().ok();
        match launch_beta_install(&detection, &self.beta_home, &self.stable_home, stable_executable.as_deref()) {
            Ok(()) => beta_action(true, None, None),
            Err(message) => {
                let _ = std::fs::remove_file(self.beta_home.join(REQUEST_FILE));
                beta_action(false, Some("internal"), Some(message))
            }
        }
    }

    pub fn leave(&self) -> BetaActionResult {
        if crate::desktop_identity::runtime_flavor() != "beta" {
            return beta_action(false, Some("not-supported"), Some("Switching back is only available from Synara Beta.".into()));
        }
        let Some(executable) = detect_stable_executable(&self.user_home, &self.env) else {
            return beta_action(false, Some("not-installed"), Some("Synara isn't installed on this computer.".into()));
        };
        let stable_home = self.env.get("SYNARA_STABLE_HOME").map(PathBuf::from);
        match launch_stable(&executable, stable_home.as_deref()) {
            Ok(()) => beta_action(true, None, None),
            Err(message) => beta_action(false, Some("launch-failed"), Some(message)),
        }
    }
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
        uuid::Uuid::new_v4().to_string()
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


    #[cfg(target_os = "macos")]
    #[test]
    fn beta_feed_urls_require_https_except_loopback_http() {
        assert!(validate_feed_url("https://github.com/example").is_ok());
        assert!(validate_feed_url("http://127.0.0.1:9999/feed").is_ok());
        assert!(validate_feed_url("http://localhost/feed").is_ok());
        assert!(validate_feed_url("http://example.com/feed").is_err());
        assert!(validate_feed_url("ftp://example.com/feed").is_err());
    }


    #[cfg(target_os = "macos")]
    #[test]
    fn macos_feed_installer_downloads_verifies_and_commits_beta_app() {
        use base64::Engine as _;
        use sha2::Digest as _;
        use std::io::{Read as _, Write as _};

        let root = temp_root();
        let zip_input = root.join("zip-input");
        let app = zip_input.join(BETA_APP_NAME).join("Contents");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("Info.plist"),
            r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.emanueledipietro.synara.beta</string><key>CFBundleExecutable</key><string>synara-shell</string></dict></plist>"#,
        ).unwrap();
        let zip_path = root.join("beta.zip");
        let zipped = Command::new("ditto")
            .current_dir(&zip_input)
            .args(["-c", "-k", "--sequesterRsrc", "--keepParent", BETA_APP_NAME])
            .arg(&zip_path)
            .output()
            .unwrap();
        assert!(zipped.status.success(), "{}", String::from_utf8_lossy(&zipped.stderr));
        let zip_bytes = std::fs::read(&zip_path).unwrap();
        let sha512 = base64::engine::general_purpose::STANDARD.encode(sha2::Sha512::digest(&zip_bytes));
        let manifest = format!(
            "version: 1.6.7-beta.1\nfiles:\n  - url: beta.zip\n    sha512: {sha512}\n    size: {}\n",
            zip_bytes.len()
        );

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let manifest_bytes = manifest.into_bytes();
        let zip_response = zip_bytes.clone();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else { break };
                let mut buffer = [0u8; 4096];
                let Ok(count) = stream.read(&mut buffer) else { continue };
                let request = String::from_utf8_lossy(&buffer[..count]);
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                let (body, content_type) = if path == "/beta-mac.yml" {
                    (manifest_bytes.clone(), "text/yaml")
                } else if path == "/beta.zip" {
                    (zip_response.clone(), "application/zip")
                } else {
                    (Vec::new(), "text/plain")
                };
                let status = if body.is_empty() && path != "/beta.zip" && path != "/beta-mac.yml" { "404 Not Found" } else { "200 OK" };
                let head = format!("HTTP/1.1 {status}\r\ncontent-length: {}\r\ncontent-type: {content_type}\r\nconnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });

        let install_dir = root.join("install");
        let feed = format!("http://127.0.0.1:{port}");
        let installed = install_beta_from_feed(&install_dir, Some(&feed), None).unwrap();
        assert_eq!(installed, install_dir.join(BETA_APP_NAME));
        assert!(installed.join("Contents/Info.plist").is_file());
        assert!(!install_dir.join(format!("{BETA_APP_NAME}.previous")).exists());
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
