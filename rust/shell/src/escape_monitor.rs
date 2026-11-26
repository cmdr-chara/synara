//! Physical Escape and human-input monitor driven by the AppSnap helper.
//!
//! The helper owns the macOS listen-only event tap. This module owns process
//! lifecycle: explicit readiness, arm/disarm replay, bounded stderr reporting,
//! generation-gated stale readers and exponential respawn after unexpected exit.

use serde_json::Value;
use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const RESPAWN_MAX_DELAY: Duration = Duration::from_secs(30);
const LISTENER_START_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_HELPER_STDERR_CHARS: u64 = 4_000;

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalComputerInput {
    pub kind: String,
    pub pid: Option<i64>,
    pub window_id: Option<u64>,
    pub event_type: Option<i64>,
    pub source_pid: Option<i64>,
    pub source_state_id: Option<i64>,
    pub source_tagged: Option<bool>,
    pub modifier_key: Option<bool>,
    pub captured_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppSnapHelperMessage {
    Ready,
    Escape {
        captured_at: Option<String>,
    },
    EscapeMonitorState {
        armed: bool,
        captured_at: Option<String>,
    },
    PhysicalInput(PhysicalComputerInput),
    Error {
        code: String,
        message: String,
        captured_at: Option<String>,
    },
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn optional_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

pub fn parse_appsnap_helper_message(line: &str) -> Option<AppSnapHelperMessage> {
    let value: Value = serde_json::from_str(line).ok()?;
    match value.get("type").and_then(Value::as_str)? {
        "ready" => Some(AppSnapHelperMessage::Ready),
        "escape" => Some(AppSnapHelperMessage::Escape {
            captured_at: optional_string(&value, "capturedAt"),
        }),
        "escape-monitor-state" => Some(AppSnapHelperMessage::EscapeMonitorState {
            armed: value.get("armed").and_then(Value::as_bool)?,
            captured_at: optional_string(&value, "capturedAt"),
        }),
        "physical-input" => {
            let kind = value.get("kind").and_then(Value::as_str)?;
            if kind != "keyboard" && kind != "pointer" {
                return None;
            }
            let event_type = optional_i64(&value, "eventType")
                .filter(|event_type| [1, 3, 10, 12, 22, 25].contains(event_type));
            Some(AppSnapHelperMessage::PhysicalInput(PhysicalComputerInput {
                kind: kind.into(),
                pid: optional_i64(&value, "pid").filter(|pid| (1..=0x7fffffff).contains(pid)),
                window_id: optional_i64(&value, "windowId")
                    .and_then(|id| u64::try_from(id).ok())
                    .filter(|id| (1..=0xffffffff).contains(id)),
                event_type,
                source_pid: optional_i64(&value, "sourcePid")
                    .filter(|pid| (0..=0x7fffffff).contains(pid)),
                source_state_id: optional_i64(&value, "sourceStateId")
                    .filter(|id| [-1, 0, 1].contains(id)),
                source_tagged: value.get("sourceTagged").and_then(Value::as_bool),
                modifier_key: if event_type == Some(12) {
                    value.get("modifierKey").and_then(Value::as_bool)
                } else {
                    None
                },
                captured_at: optional_string(&value, "capturedAt"),
            }))
        }
        "error" => {
            let code = value
                .get("code")
                .and_then(Value::as_str)
                .filter(|code| !code.is_empty())?;
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .filter(|message| !message.is_empty())?;
            Some(AppSnapHelperMessage::Error {
                code: code.into(),
                message: message.into(),
                captured_at: optional_string(&value, "capturedAt"),
            })
        }
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputerInputMonitorState {
    pub ready: bool,
    pub error: Option<String>,
}

struct MonitorState {
    child: Option<Arc<Mutex<Child>>>,
    generation: u64,
    armed: bool,
    disposed: bool,
    ready: bool,
    error: Option<String>,
    consecutive_failures: u32,
    restart_pending: bool,
}

pub struct EscapeKillSwitchMonitor {
    inner: Arc<MonitorInner>,
}

struct MonitorInner {
    helper: PathBuf,
    state: Mutex<MonitorState>,
    on_escape: Arc<dyn Fn() + Send + Sync>,
    on_physical_input: Arc<dyn Fn(PhysicalComputerInput) + Send + Sync>,
    on_error: Arc<dyn Fn(String) + Send + Sync>,
}

impl EscapeKillSwitchMonitor {
    pub fn new(
        helper: PathBuf,
        on_escape: impl Fn() + Send + Sync + 'static,
        on_error: impl Fn(String) + Send + Sync + 'static,
    ) -> Self {
        Self::with_physical_input(helper, on_escape, |_| {}, on_error)
    }

    pub fn with_physical_input(
        helper: PathBuf,
        on_escape: impl Fn() + Send + Sync + 'static,
        on_physical_input: impl Fn(PhysicalComputerInput) + Send + Sync + 'static,
        on_error: impl Fn(String) + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(MonitorInner {
                helper,
                state: Mutex::new(MonitorState {
                    child: None,
                    generation: 0,
                    armed: false,
                    disposed: false,
                    ready: false,
                    error: Some("input_monitor_idle".into()),
                    consecutive_failures: 0,
                    restart_pending: false,
                }),
                on_escape: Arc::new(on_escape),
                on_physical_input: Arc::new(on_physical_input),
                on_error: Arc::new(on_error),
            }),
        }
    }

    pub fn state(&self) -> ComputerInputMonitorState {
        self.inner
            .state
            .lock()
            .map(|state| ComputerInputMonitorState {
                ready: state.ready,
                error: state.error.clone(),
            })
            .unwrap_or(ComputerInputMonitorState {
                ready: false,
                error: Some("input_monitor_unavailable".into()),
            })
    }

    pub fn is_running(&self) -> bool {
        self.inner
            .state
            .lock()
            .map(|state| state.child.is_some())
            .unwrap_or(false)
    }

    pub fn start(&self) {
        let should_spawn = self
            .inner
            .state
            .lock()
            .map(|state| !state.disposed && state.child.is_none() && !state.restart_pending)
            .unwrap_or(false);
        if should_spawn {
            self.inner.spawn_helper();
        }
    }

    /// Arms the listener and waits briefly for explicit readiness or a known
    /// permission/tap failure. Input admission remains fenced while starting.
    pub fn activate(&self) {
        self.set_armed(true);
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            let done = self
                .inner
                .state
                .lock()
                .map(|state| {
                    state.ready || state.error.as_deref() != Some("input_monitor_starting")
                })
                .unwrap_or(true);
            if done {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn set_armed(&self, armed: bool) {
        {
            let Ok(mut state) = self.inner.state.lock() else {
                return;
            };
            if state.disposed {
                return;
            }
            let changed = state.armed != armed;
            state.armed = armed;
            if changed {
                state.ready = false;
                state.error = Some(
                    if armed {
                        "input_monitor_starting"
                    } else {
                        "input_monitor_idle"
                    }
                    .into(),
                );
            }
        }
        if armed {
            if !self.is_running() {
                self.start();
            }
            self.write_command("arm");
        } else {
            self.write_command("disarm");
        }
    }

    fn write_command(&self, command: &str) {
        let child = self
            .inner
            .state
            .lock()
            .ok()
            .and_then(|state| state.child.clone());
        if let Some(child) = child {
            if let Ok(mut child) = child.lock() {
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(command.as_bytes());
                    let _ = stdin.write_all(b"\n");
                    let _ = stdin.flush();
                }
            }
        }
    }

    pub fn dispose(&self) {
        let child = self.inner.state.lock().ok().and_then(|mut state| {
            state.disposed = true;
            state.ready = false;
            state.error = Some("input_monitor_stopped".into());
            state.generation += 1;
            state.child.take()
        });
        if let Some(child) = child {
            if let Ok(mut child) = child.lock() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for EscapeKillSwitchMonitor {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl MonitorInner {
    fn set_starting(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.ready = false;
            state.error = Some("input_monitor_starting".into());
        }
    }

    fn spawn_helper(self: &Arc<Self>) {
        self.set_starting();
        let generation = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            state.generation += 1;
            state.restart_pending = false;
            state.generation
        };
        let spawned = Command::new(&self.helper)
            .arg("--escape-monitor")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(_) => {
                if let Ok(mut state) = self.state.lock() {
                    state.ready = false;
                    state.error = Some("input_monitor_unavailable".into());
                }
                (self.on_error)("The Escape monitor helper could not start.".into());
                self.schedule_restart();
                return;
            }
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child = Arc::new(Mutex::new(child));
        if let Ok(mut state) = self.state.lock() {
            state.child = Some(Arc::clone(&child));
        }

        if let Some(stdout) = stdout {
            let this = Arc::clone(self);
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    if this
                        .state
                        .lock()
                        .map(|state| state.generation != generation || state.disposed)
                        .unwrap_or(true)
                    {
                        break;
                    }
                    let Ok(line) = line else { break };
                    if let Some(message) = parse_appsnap_helper_message(&line) {
                        this.handle_message(message);
                    }
                }
            });
        }
        if let Some(stderr) = stderr {
            let this = Arc::clone(self);
            std::thread::spawn(move || {
                let mut text = String::new();
                if BufReader::new(stderr)
                    .take(MAX_HELPER_STDERR_CHARS)
                    .read_to_string(&mut text)
                    .is_ok()
                {
                    let diagnostic = text.trim().to_string();
                    if !diagnostic.is_empty()
                        && this
                            .state
                            .lock()
                            .map(|state| state.generation == generation && !state.disposed)
                            .unwrap_or(false)
                    {
                        (this.on_error)(format!("Escape monitor helper: {diagnostic}"));
                    }
                }
            });
        }

        let waiter = Arc::clone(self);
        let waiter_child = Arc::clone(&child);
        std::thread::spawn(move || {
            loop {
                let exited = waiter_child
                    .lock()
                    .ok()
                    .and_then(|mut child| child.try_wait().ok().map(|status| status.is_some()));
                if exited != Some(false) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let should_restart = waiter
                .state
                .lock()
                .map(|mut state| {
                    if state.generation != generation || state.disposed {
                        return false;
                    }
                    state.child = None;
                    state.ready = false;
                    state.error = Some("input_monitor_unavailable".into());
                    state.armed
                })
                .unwrap_or(false);
            if should_restart {
                waiter.schedule_restart();
            }
        });

        let readiness = Arc::clone(self);
        let readiness_child = Arc::clone(&child);
        std::thread::spawn(move || {
            std::thread::sleep(LISTENER_START_TIMEOUT);
            let stale = readiness
                .state
                .lock()
                .map(|state| {
                    state.generation != generation
                        || state.disposed
                        || !state.armed
                        || state.ready
                        || state.error.as_deref() != Some("input_monitor_starting")
                })
                .unwrap_or(true);
            if stale {
                return;
            }
            if let Ok(mut state) = readiness.state.lock() {
                state.ready = false;
                state.error = Some("input_monitor_unavailable".into());
                state.generation += 1;
                state.child = None;
            }
            if let Ok(mut child) = readiness_child.lock() {
                let _ = child.kill();
                let _ = child.wait();
            }
            (readiness.on_error)(
                "The Escape monitor helper did not acknowledge readiness; restarting.".into(),
            );
            readiness.schedule_restart();
        });

        if self.state.lock().map(|state| state.armed).unwrap_or(false) {
            if let Ok(mut child) = child.lock() {
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(b"arm\n");
                    let _ = stdin.flush();
                }
            }
        }
    }

    fn schedule_restart(self: &Arc<Self>) {
        let delay = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.disposed || !state.armed || state.restart_pending {
                return;
            }
            state.consecutive_failures = state.consecutive_failures.saturating_add(1);
            state.restart_pending = true;
            let exponent = state.consecutive_failures.saturating_sub(1).min(5);
            respawn_delay(exponent)
        };
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            let should_spawn = this
                .state
                .lock()
                .map(|mut state| {
                    if state.disposed || !state.armed {
                        state.restart_pending = false;
                        false
                    } else {
                        state.restart_pending = false;
                        true
                    }
                })
                .unwrap_or(false);
            if should_spawn {
                this.spawn_helper();
            }
        });
    }

    fn handle_message(&self, message: AppSnapHelperMessage) {
        match message {
            AppSnapHelperMessage::Ready => {
                let active = self
                    .state
                    .lock()
                    .map(|mut state| {
                        if state.armed {
                            state.ready = true;
                            state.error = None;
                            state.consecutive_failures = 0;
                        }
                        state.armed && state.ready
                    })
                    .unwrap_or(false);
                let _ = active;
            }
            AppSnapHelperMessage::Escape { .. } => {
                let active = self
                    .state
                    .lock()
                    .map(|state| state.armed && state.ready)
                    .unwrap_or(false);
                if active {
                    (self.on_escape)();
                }
            }
            AppSnapHelperMessage::PhysicalInput(input) => {
                let active = self
                    .state
                    .lock()
                    .map(|state| state.armed && state.ready)
                    .unwrap_or(false);
                if active {
                    (self.on_physical_input)(input);
                }
            }
            AppSnapHelperMessage::Error { code, message, .. } => {
                let mapped = if matches!(
                    code.as_str(),
                    "input-monitoring-required" | "event_tap_disabled" | "event_tap_unavailable"
                ) {
                    code
                } else {
                    "input_monitor_unavailable".into()
                };
                if let Ok(mut state) = self.state.lock() {
                    state.ready = false;
                    state.error = Some(mapped);
                }
                (self.on_error)(message);
            }
            AppSnapHelperMessage::EscapeMonitorState { .. } => {}
        }
    }
}

fn respawn_delay(exponent: u32) -> Duration {
    let multiplier = 1_u64 << exponent.min(5);
    let millis = 1_000_u64
        .saturating_mul(multiplier)
        .min(RESPAWN_MAX_DELAY.as_millis() as u64);
    Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_escape_and_monitor_state() {
        assert_eq!(
            parse_appsnap_helper_message(r#"{"type":"escape","capturedAt":"now"}"#),
            Some(AppSnapHelperMessage::Escape {
                captured_at: Some("now".into())
            })
        );
        assert_eq!(
            parse_appsnap_helper_message(r#"{"type":"escape-monitor-state","armed":true}"#),
            Some(AppSnapHelperMessage::EscapeMonitorState {
                armed: true,
                captured_at: None
            })
        );
    }

    #[test]
    fn parses_physical_input_with_strict_ranges() {
        let Some(AppSnapHelperMessage::PhysicalInput(input)) = parse_appsnap_helper_message(
            r#"{"type":"physical-input","kind":"keyboard","pid":700,"eventType":12,"sourcePid":1,"sourceStateId":0,"sourceTagged":false,"modifierKey":true}"#,
        ) else {
            panic!("expected physical input")
        };
        assert_eq!(input.pid, Some(700));
        assert_eq!(input.event_type, Some(12));
        assert_eq!(input.modifier_key, Some(true));
        assert!(parse_appsnap_helper_message(r#"{"type":"physical-input","kind":"keyboard","pid":0}"#)
            .is_some_and(|message| matches!(message, AppSnapHelperMessage::PhysicalInput(ref input) if input.pid.is_none())));
    }

    #[test]
    fn restart_delay_is_exponential_and_capped() {
        assert_eq!(respawn_delay(0), Duration::from_secs(1));
        assert_eq!(respawn_delay(1), Duration::from_secs(2));
        assert_eq!(respawn_delay(5), Duration::from_secs(30));
        assert_eq!(respawn_delay(99), Duration::from_secs(30));
    }

    #[test]
    fn armed_helper_escape_reaches_callback() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "synara-escape-monitor-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let helper = root.join("helper.sh");
        std::fs::write(
            &helper,
            "#!/bin/sh\necho '{\"type\":\"ready\"}'\nwhile IFS= read -r line; do\n  if [ \"$line\" = arm ]; then echo '{\"type\":\"escape\"}'; fi\ndone\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&helper, permissions).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let monitor = EscapeKillSwitchMonitor::new(helper, move || tx.send(()).unwrap(), |_| {});
        monitor.activate();
        assert!(
            rx.recv_timeout(Duration::from_secs(2)).is_ok(),
            "arming the fake helper should produce a physical Escape callback"
        );
        monitor.dispose();
        let _ = std::fs::remove_dir_all(root);
    }
}
