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
