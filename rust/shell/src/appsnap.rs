//! AppSnap helper resolution and audio-level monitoring.
//!
//! The Swift helper owns all microphone/system-audio access. This module only
//! starts it while a subscriber exists, combines subscriptions exactly like the
//! desktop monitor, parses its NDJSON boundary and stops the tap when idle.

use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

pub const MAX_MICROPHONE_ID_LENGTH: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioLevelSource {
    System,
    Microphone,
    Both,
}

impl AudioLevelSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Microphone => "microphone",
            Self::Both => "both",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioLevelSubscription {
    pub source: AudioLevelSource,
    pub microphone_id: Option<String>,
}

pub fn combine_audio_level_sources(sources: impl IntoIterator<Item = AudioLevelSource>) -> Option<AudioLevelSource> {
    let mut system = false;
    let mut microphone = false;
    for source in sources {
        if source != AudioLevelSource::Microphone {
            system = true;
        }
        if source != AudioLevelSource::System {
            microphone = true;
        }
    }
    match (system, microphone) {
        (true, true) => Some(AudioLevelSource::Both),
        (true, false) => Some(AudioLevelSource::System),
        (false, true) => Some(AudioLevelSource::Microphone),
        (false, false) => None,
    }
}

pub fn combine_audio_level_subscriptions(
    subscriptions: impl IntoIterator<Item = AudioLevelSubscription>,
) -> Option<AudioLevelSubscription> {
    let all: Vec<_> = subscriptions.into_iter().collect();
    let source = combine_audio_level_sources(all.iter().map(|subscription| subscription.source))?;
    let microphone_id = if source == AudioLevelSource::System {
        None
    } else {
        all.iter()
            .filter(|subscription| subscription.source != AudioLevelSource::System)
            .last()
            .and_then(|subscription| subscription.microphone_id.clone())
    };
    Some(AudioLevelSubscription { source, microphone_id })
}

pub fn helper_arguments(subscription: &AudioLevelSubscription) -> Vec<String> {
    let mut args = vec!["--audio-level".to_string()];
    if subscription.source != AudioLevelSource::Microphone {
        args.extend(["--source".into(), "system".into()]);
    }
    if subscription.source != AudioLevelSource::System {
        args.extend(["--source".into(), "microphone".into()]);
        if let Some(id) = &subscription.microphone_id {
            args.extend(["--input-device".into(), id.clone()]);
        }
    }
    args
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioLevelMessage {
    Ready,
    Level(f64),
    Error { code: String, message: String },
}

pub fn parse_audio_level_message(line: &str) -> Option<AudioLevelMessage> {
    let value: Value = serde_json::from_str(line).ok()?;
    match value.get("type").and_then(Value::as_str)? {
        "ready" => Some(AudioLevelMessage::Ready),
        "audio-level" => {
            let level = value.get("level").and_then(Value::as_f64)?;
            level.is_finite().then(|| AudioLevelMessage::Level(level.clamp(0.0, 1.0)))
        }
        "error" => {
            let code = value.get("code").and_then(Value::as_str)?.to_string();
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or(&code)
                .to_string();
            Some(AudioLevelMessage::Error { code, message })
        }
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioInputDevice {
    pub id: String,
    pub name: String,
    pub bluetooth: bool,
    pub default: bool,
}

pub fn parse_audio_inputs_message(output: &str) -> Vec<AudioInputDevice> {
    for line in output.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else { continue };
        if value.get("type").and_then(Value::as_str) != Some("audio-inputs") {
            continue;
        }
        let mut devices = Vec::new();
        for raw in value.get("devices").and_then(Value::as_array).into_iter().flatten() {
            let Some(id) = raw.get("id").and_then(Value::as_str) else { continue };
            if id.is_empty() || id.chars().map(char::len_utf16).sum::<usize>() > MAX_MICROPHONE_ID_LENGTH {
                continue;
            }
            let name = raw
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .unwrap_or(id)
                .to_string();
            devices.push(AudioInputDevice {
                id: id.into(),
                name,
                bluetooth: raw.get("bluetooth").and_then(Value::as_bool) == Some(true),
                default: raw.get("default").and_then(Value::as_bool) == Some(true),
            });
        }
        return devices;
    }
    Vec::new()
}

/// Packaged helper first, then the development build locations used by the
/// Electron pipeline. `SYNARA_APPSNAP_HELPER` is an explicit operator override.
pub fn resolve_appsnap_helper() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SYNARA_APPSNAP_HELPER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let exe = std::env::current_exe().ok()?;
    if let Some(mac_os) = exe.parent() {
        let bundled = mac_os.join("../Helpers/synara-appsnap-helper");
        if bundled.is_file() {
            return bundled.canonicalize().ok().or(Some(bundled));
        }
    }
    for ancestor in exe.ancestors() {
        for candidate in [
            ancestor.join("apps/desktop/native/appsnap/build/synara-appsnap-helper"),
            ancestor.join("apps/desktop/.electron-runtime/appsnap/synara-appsnap-helper"),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn list_audio_input_devices(helper: &std::path::Path) -> Vec<AudioInputDevice> {
    let Ok(mut child) = Command::new(helper)
        .arg("--list-audio-inputs")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Vec::new();
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => std::thread::sleep(std::time::Duration::from_millis(20)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Vec::new();
            }
            Err(_) => return Vec::new(),
        }
    }
    child
        .wait_with_output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| parse_audio_inputs_message(&stdout))
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioLevelStatus {
    Off,
    Active,
    Unavailable,
}

struct MonitorState {
    child: Option<Child>,
    child_subscription: Option<AudioLevelSubscription>,
    subscribers: std::collections::HashMap<u64, AudioLevelSubscription>,
    status: AudioLevelStatus,
    generation: u64,
}

pub struct AudioLevelMonitor {
    helper: PathBuf,
    state: Arc<Mutex<MonitorState>>,
    on_level: Arc<dyn Fn(f64) + Send + Sync>,
    on_error: Arc<dyn Fn(String) + Send + Sync>,
}

impl AudioLevelMonitor {
    pub fn new(
        helper: PathBuf,
        on_level: impl Fn(f64) + Send + Sync + 'static,
        on_error: impl Fn(String) + Send + Sync + 'static,
    ) -> Self {
        Self {
            helper,
            state: Arc::new(Mutex::new(MonitorState {
                child: None,
                child_subscription: None,
                subscribers: std::collections::HashMap::new(),
                status: AudioLevelStatus::Off,
                generation: 0,
            })),
            on_level: Arc::new(on_level),
            on_error: Arc::new(on_error),
        }
    }

    pub fn status(&self) -> AudioLevelStatus {
        self.state.lock().map(|state| state.status).unwrap_or(AudioLevelStatus::Off)
    }

    pub fn is_running(&self) -> bool {
        self.state.lock().ok().map(|mut state| {
            if let Some(child) = state.child.as_mut() {
                if matches!(child.try_wait(), Ok(None)) {
                    return true;
                }
                state.child = None;
                state.child_subscription = None;
                state.status = AudioLevelStatus::Unavailable;
                state.generation += 1;
            }
            false
        }).unwrap_or(false)
    }

    pub fn set_subscription(
        &self,
        subscriber: u64,
        subscription: Option<AudioLevelSubscription>,
    ) -> AudioLevelStatus {
        let wanted = {
            let Ok(mut state) = self.state.lock() else { return AudioLevelStatus::Off };
            if let Some(subscription) = subscription {
                state.subscribers.insert(subscriber, subscription);
            } else {
                state.subscribers.remove(&subscriber);
            }
            combine_audio_level_subscriptions(state.subscribers.values().cloned().collect::<Vec<_>>())
        };
        match wanted {
            None => {
                self.stop_helper();
                if let Ok(mut state) = self.state.lock() {
                    state.status = AudioLevelStatus::Off;
                }
                AudioLevelStatus::Off
            }
            Some(wanted) => {
                let restart = self
                    .state
                    .lock()
                    .map(|state| state.child_subscription.as_ref() != Some(&wanted))
                    .unwrap_or(true);
                if restart {
                    self.stop_helper();
                    self.start_helper(wanted);
                }
                self.status()
            }
        }
    }

    fn start_helper(&self, subscription: AudioLevelSubscription) {
        let spawn = Command::new(&self.helper)
            .args(helper_arguments(&subscription))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match spawn {
            Ok(child) => child,
            Err(error) => {
                (self.on_error)(format!("The audio level helper could not start: {error}"));
                if let Ok(mut state) = self.state.lock() {
                    state.status = AudioLevelStatus::Unavailable;
                }
                return;
            }
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let generation = if let Ok(mut state) = self.state.lock() {
            state.status = AudioLevelStatus::Active;
            state.child_subscription = Some(subscription);
            state.generation += 1;
            state.generation
        } else {
            0
        };
        if let Some(stdout) = stdout {
            let state_ref = Arc::clone(&self.state);
            let on_level = Arc::clone(&self.on_level);
            let on_error = Arc::clone(&self.on_error);
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    if state_ref.lock().map(|state| state.generation != generation).unwrap_or(true) {
                        break;
                    }
                    let Ok(line) = line else { break };
                    match parse_audio_level_message(&line) {
                        Some(AudioLevelMessage::Level(level)) => on_level(level),
                        Some(AudioLevelMessage::Error { code, message }) => {
                            if let Ok(mut state) = state_ref.lock() {
                                state.status = AudioLevelStatus::Unavailable;
                            }
                            on_error(format!("{code}: {message}"));
                        }
                        _ => {}
                    }
                }
                if let Ok(mut state) = state_ref.lock() {
                    if state.generation == generation && state.child.is_some() {
                        state.status = AudioLevelStatus::Unavailable;
                    }
                }
            });
        }
        if let Some(stderr) = stderr {
            let on_error = Arc::clone(&self.on_error);
            std::thread::spawn(move || {
                let mut text = String::new();
                use std::io::Read as _;
                let mut reader = BufReader::new(stderr).take(4_000);
                if reader.read_to_string(&mut text).is_ok() && !text.trim().is_empty() {
                    on_error(format!("audio level helper: {}", text.trim()));
                }
            });
        }
        if let Ok(mut state) = self.state.lock() {
            state.child = Some(child);
        }
    }

    fn stop_helper(&self) {
        let child = self.state.lock().ok().and_then(|mut state| {
            state.child_subscription = None;
            state.generation += 1;
            state.child.take()
        });
        if let Some(mut child) = child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for AudioLevelMonitor {
    fn drop(&mut self) {
        self.stop_helper();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_sources_like_upstream() {
        assert_eq!(combine_audio_level_sources([]), None);
        assert_eq!(combine_audio_level_sources([AudioLevelSource::System]), Some(AudioLevelSource::System));
        assert_eq!(combine_audio_level_sources([AudioLevelSource::System, AudioLevelSource::Microphone]), Some(AudioLevelSource::Both));
    }

    #[test]
    fn newest_microphone_choice_wins() {
        let combined = combine_audio_level_subscriptions([
            AudioLevelSubscription { source: AudioLevelSource::Microphone, microphone_id: Some("old".into()) },
            AudioLevelSubscription { source: AudioLevelSource::Both, microphone_id: Some("new".into()) },
        ]).unwrap();
        assert_eq!(combined.source, AudioLevelSource::Both);
        assert_eq!(combined.microphone_id.as_deref(), Some("new"));
    }

    #[test]
    fn helper_arguments_match_subscription() {
        let subscription = AudioLevelSubscription { source: AudioLevelSource::Both, microphone_id: Some("mic".into()) };
        assert_eq!(helper_arguments(&subscription), ["--audio-level", "--source", "system", "--source", "microphone", "--input-device", "mic"]);
    }

    #[test]
    fn parses_helper_messages() {
        assert_eq!(parse_audio_level_message(r#"{"type":"audio-level","level":3}"#), Some(AudioLevelMessage::Level(1.0)));
        assert_eq!(parse_audio_level_message(r#"{"type":"ready"}"#), Some(AudioLevelMessage::Ready));
        assert!(parse_audio_level_message("not json").is_none());
    }

    #[test]
    fn parses_input_devices_and_falls_back_to_id_name() {
        let devices = parse_audio_inputs_message(r#"{"type":"audio-inputs","devices":[{"id":"mic-1","bluetooth":true,"default":true}]}"#);
        assert_eq!(devices, vec![AudioInputDevice { id: "mic-1".into(), name: "mic-1".into(), bluetooth: true, default: true }]);
    }

    #[test]
    fn resolves_development_helper_when_built() {
        let expected = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/native/appsnap/build/synara-appsnap-helper")
            .canonicalize();
        if let Ok(expected) = expected {
            assert_eq!(resolve_appsnap_helper().and_then(|path| path.canonicalize().ok()), Some(expected));
        }
    }
}

// ---- AppSnap capture operations -------------------------------------------------------------

const APPSNAP_PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const APPSNAP_MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppSnapCapture {
    pub id: String,
    pub captured_at: String,
    pub name: String,
    pub size_bytes: usize,
    pub bytes: Vec<u8>,
    pub source_app_name: Option<String>,
    pub source_bundle_identifier: Option<String>,
    pub source_app_icon_data_url: Option<String>,
    pub source_window_title: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSnapWindowEntry {
    pub window_id: u64,
    pub app_name: Option<String>,
    pub bundle_identifier: Option<String>,
    pub window_title: Option<String>,
    pub app_icon_data_url: Option<String>,
}

fn valid_appsnap_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn path_inside(directory: &std::path::Path, path: &std::path::Path) -> bool {
    let Ok(directory) = directory.canonicalize() else { return false };
    let Ok(path) = path.canonicalize() else { return false };
    path.starts_with(directory)
}

fn read_validated_png(path: &std::path::Path, output_directory: &std::path::Path) -> Result<Vec<u8>, String> {
    if !path_inside(output_directory, path) {
        return Err("Invalid AppSnap capture path.".into());
    }
    let metadata = std::fs::metadata(path).map_err(|_| "Invalid AppSnap capture path.".to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() as usize > APPSNAP_MAX_IMAGE_BYTES {
        return Err("Invalid AppSnap capture image.".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "The AppSnap capture changed while it was read.".to_string())?;
    if bytes.len() as u64 != metadata.len() || bytes.len() < APPSNAP_PNG_SIGNATURE.len() || bytes[..8] != APPSNAP_PNG_SIGNATURE {
        return Err("Invalid AppSnap capture image.".into());
    }
    Ok(bytes)
}

fn optional_text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(|s| s.chars().take(240).collect())
}

fn capture_from_message(message: &Value, output_directory: &std::path::Path) -> Result<AppSnapCapture, String> {
    let id = message.get("id").and_then(Value::as_str).filter(|id| valid_appsnap_id(id)).ok_or("Invalid AppSnap capture id.")?;
    let path = message.get("path").and_then(Value::as_str).ok_or("Invalid AppSnap capture path.")?;
    let bytes = read_validated_png(std::path::Path::new(path), output_directory)?;
    Ok(AppSnapCapture {
        id: id.to_string(),
        captured_at: message.get("capturedAt").and_then(Value::as_str).unwrap_or_default().to_string(),
        name: message.get("name").and_then(Value::as_str).filter(|name| !name.is_empty()).unwrap_or("AppSnap capture.png").chars().take(240).collect(),
        size_bytes: bytes.len(),
        bytes,
        source_app_name: optional_text(message, "sourceAppName"),
        source_bundle_identifier: optional_text(message, "sourceBundleIdentifier"),
        source_app_icon_data_url: message.get("sourceAppIconDataUrl").and_then(Value::as_str).filter(|v| v.starts_with("data:image/png;base64,") && v.len() <= 256_000).map(str::to_string),
        source_window_title: optional_text(message, "sourceWindowTitle"),
    })
}

fn spawn_capture_helper(
    helper: &std::path::Path,
    output_directory: &std::path::Path,
    excluded_bundle_id: &str,
) -> Result<Child, String> {
    Command::new(helper)
        .arg("--watch")
        .arg("--external-trigger")
        .arg("--output-dir")
        .arg(output_directory)
        .arg("--excluded-bundle-id")
        .arg(excluded_bundle_id)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start the AppSnap helper: {error}"))
}

fn write_helper_line(child: &mut Child, line: &str) -> Result<(), String> {
    use std::io::Write as _;
    let Some(stdin) = child.stdin.as_mut() else { return Err("The AppSnap helper input is unavailable.".into()) };
    stdin.write_all(line.as_bytes()).and_then(|_| stdin.write_all(b"\n")).and_then(|_| stdin.flush())
        .map_err(|error| format!("Could not ask the AppSnap helper for a capture: {error}"))
}

fn read_capture_messages(
    child: &mut Child,
    output_directory: &std::path::Path,
    request: &str,
    expected_capture_id: Option<&str>,
    timeout: std::time::Duration,
) -> Result<Value, String> {
    use std::io::{BufRead as _, BufReader};
    let stdout = child.stdout.take().ok_or("The AppSnap helper output is unavailable.")?;
    let mut reader = BufReader::new(stdout);
    let deadline = std::time::Instant::now() + timeout;
    let mut triggered = false;
    loop {
        if std::time::Instant::now() >= deadline {
            return Err("AppSnap capture timed out.".into());
        }
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Err("The AppSnap helper exited before capturing.".into()),
            Ok(_) => {}
            Err(error) => return Err(format!("Could not read the AppSnap helper: {error}")),
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
        match message.get("type").and_then(Value::as_str) {
            Some("ready") if !triggered => {
                write_helper_line(child, request)?;
                triggered = true;
            }
            Some("error") => {
                let matches_request = expected_capture_id.is_none_or(|id| {
                    message.get("id").and_then(Value::as_str) == Some(id)
                        || message.get("requestId").and_then(Value::as_str) == Some(id)
                        || message.get("id").is_none()
                });
                if matches_request {
                    return Err(message.get("message").and_then(Value::as_str).unwrap_or("AppSnap capture failed.").to_string());
                }
            }
            Some("captured") => {
                if expected_capture_id.is_some_and(|id| message.get("id").and_then(Value::as_str) != Some(id)) {
                    continue;
                }
                let capture = capture_from_message(&message, output_directory)?;
                return serde_json::to_value(capture).map_err(|error| error.to_string());
            }
            _ => {}
        }
    }
}

fn finish_capture_helper(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn requested_capture(
    helper: &std::path::Path,
    capture_directory: &std::path::Path,
    excluded_bundle_id: &str,
    request: &str,
    expected_capture_id: Option<&str>,
    timeout: std::time::Duration,
) -> Result<AppSnapCapture, String> {
    std::fs::create_dir_all(capture_directory).map_err(|error| error.to_string())?;
    let output_directory = capture_directory.join(format!("requested-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&output_directory).map_err(|error| error.to_string())?;
    let result = (|| {
        let mut child = spawn_capture_helper(helper, &output_directory, excluded_bundle_id)?;
        let result = read_capture_messages(&mut child, &output_directory, request, expected_capture_id, timeout);
        finish_capture_helper(&mut child);
        result.and_then(|value| serde_json::from_value(value).map_err(|error| error.to_string()))
    })();
    let _ = std::fs::remove_dir_all(&output_directory);
    result
}

/// User-requested capture of the current foreground application. The helper
/// resolves the target before Synara can react, so focus restoration cannot
/// silently substitute a different app.
pub fn capture_current_app(
    helper: &std::path::Path,
    capture_directory: &std::path::Path,
    excluded_bundle_id: &str,
) -> Result<AppSnapCapture, String> {
    requested_capture(
        helper,
        capture_directory,
        excluded_bundle_id,
        "trigger",
        None,
        std::time::Duration::from_secs(10),
    )
}

/// Capture one exact macOS window selected by the owner.
pub fn capture_window(
    helper: &std::path::Path,
    capture_directory: &std::path::Path,
    excluded_bundle_id: &str,
    window_id: u64,
) -> Result<AppSnapCapture, String> {
    if window_id == 0 || window_id > 0xffff_ffff {
        return Err("captureWindow requires a valid macOS window id.".into());
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    requested_capture(
        helper,
        capture_directory,
        excluded_bundle_id,
        &format!("capture-window {request_id} {window_id}"),
        Some(&request_id),
        std::time::Duration::from_secs(20),
    )
}

pub fn parse_appsnap_windows(message: &Value, request_id: &str) -> Option<Vec<AppSnapWindowEntry>> {
    if message.get("type").and_then(Value::as_str) != Some("windows")
        || message.get("requestId").and_then(Value::as_str) != Some(request_id)
    {
        return None;
    }
    let mut windows = Vec::new();
    for raw in message.get("windows").and_then(Value::as_array)? {
        let window_id = raw.get("windowId").and_then(Value::as_u64).filter(|id| *id > 0 && *id <= 0xffff_ffff)?;
        windows.push(AppSnapWindowEntry {
            window_id,
            app_name: optional_text(raw, "appName"),
            bundle_identifier: optional_text(raw, "bundleIdentifier"),
            window_title: optional_text(raw, "windowTitle"),
            app_icon_data_url: raw.get("appIconDataUrl").and_then(Value::as_str).filter(|v| v.starts_with("data:image/png;base64,") && v.len() <= 256_000).map(str::to_string),
        });
    }
    Some(windows)
}

/// Lists capturable visible windows through the real helper. This operation is
/// synchronous like the upstream picker IPC and bounded to five seconds.
pub fn list_windows(
    helper: &std::path::Path,
    capture_directory: &std::path::Path,
    excluded_bundle_id: &str,
) -> Result<Vec<AppSnapWindowEntry>, String> {
    std::fs::create_dir_all(capture_directory).map_err(|error| error.to_string())?;
    let output_directory = capture_directory.join(format!("windows-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&output_directory).map_err(|error| error.to_string())?;
    let result = (|| {
        let mut child = spawn_capture_helper(helper, &output_directory, excluded_bundle_id)?;
        let stdout = child.stdout.take().ok_or("The AppSnap helper output is unavailable.")?;
        use std::io::{BufRead as _, BufReader};
        let mut reader = BufReader::new(stdout);
        let request_id = uuid::Uuid::new_v4().to_string();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut sent = false;
        let outcome = loop {
            if std::time::Instant::now() >= deadline {
                break Err("Listing AppSnap windows timed out.".into());
            }
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break Err("The AppSnap helper exited before listing windows.".into()),
                Ok(_) => {}
                Err(error) => break Err(format!("Could not read the AppSnap helper: {error}")),
            }
            let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
            match message.get("type").and_then(Value::as_str) {
                Some("ready") if !sent => {
                    write_helper_line(&mut child, &format!("list-windows {request_id}"))?;
                    sent = true;
                }
                Some("windows") => {
                    if let Some(windows) = parse_appsnap_windows(&message, &request_id) {
                        break Ok(windows);
                    }
                }
                Some("error") if message.get("requestId").and_then(Value::as_str) == Some(request_id.as_str()) => {
                    break Err(message.get("message").and_then(Value::as_str).unwrap_or("Could not list AppSnap windows.").to_string());
                }
                _ => {}
            }
        };
        finish_capture_helper(&mut child);
        outcome
    })();
    let _ = std::fs::remove_dir_all(&output_directory);
    result
}

#[cfg(test)]
mod appsnap_capture_tests {
    use super::*;

    #[test]
    fn capture_message_validates_png_and_metadata() {
        let root = std::env::temp_dir().join(format!("appsnap-capture-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("capture.png");
        let mut png = APPSNAP_PNG_SIGNATURE.to_vec();
        png.extend_from_slice(b"png-bytes");
        std::fs::write(&image, &png).unwrap();
        let message = serde_json::json!({
            "type": "captured",
            "id": "capture-1",
            "capturedAt": "2026-01-01T00:00:00Z",
            "path": image,
            "name": "capture.png",
            "sourceAppName": "Safari",
            "sourceWindowTitle": "Example",
        });
        let capture = capture_from_message(&message, &root).unwrap();
        assert_eq!(capture.size_bytes, png.len());
        assert_eq!(capture.source_app_name.as_deref(), Some("Safari"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn window_message_requires_matching_request() {
        let message = serde_json::json!({
            "type": "windows",
            "requestId": "req-1",
            "windows": [{"windowId": 42, "appName": "Finder"}]
        });
        assert!(parse_appsnap_windows(&message, "other").is_none());
        let windows = parse_appsnap_windows(&message, "req-1").unwrap();
        assert_eq!(windows[0].window_id, 42);
    }
}
