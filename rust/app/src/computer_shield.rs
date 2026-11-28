//! Persistent AppSnap `--shield` host, port of the lifecycle guarantees in
//! apps/desktop/src/computerShield.ts. The helper owns the actual panels; this
//! host owns command writes, engage confirmation, attribution, and cleanup when
//! a task, the helper, or the computer host ends.

use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const ENGAGE_TIMEOUT: Duration = Duration::from_secs(5);
const ENDED_TASK_CAP: usize = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct ShieldFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShieldEngagement {
    pub shield_id: String,
    pub frame: ShieldFrame,
    pub window_id: u32,
    pub pid: u32,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShieldTask {
    pub thread_id: String,
    pub turn_id: Option<String>,
}

fn task_key(task: &ShieldTask) -> String {
    serde_json::json!([task.thread_id, task.turn_id]).to_string()
}

#[derive(Clone)]
struct LiveShield {
    task_key: Option<String>,
    thread_id: Option<String>,
}

struct ShieldState {
    child: Option<Child>,
    live: HashMap<String, LiveShield>,
    pending: HashMap<String, Result<(), String>>,
    ended_tasks: HashSet<String>,
    ended_order: VecDeque<String>,
    closed: bool,
}

struct Shared {
    state: Mutex<ShieldState>,
    changed: Condvar,
}

pub struct ComputerShield {
    helper_path: PathBuf,
    shared: Arc<Shared>,
}

fn terminate_child_gracefully(child: &mut Child) {
    #[cfg(unix)]
    {
        let _ = Command::new("/bin/kill")
            .arg("-TERM")
            .arg(child.id().to_string())
            .status();
        for _ in 0..20 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let _ = child.kill();
}

impl ComputerShield {
    pub fn new(helper_path: PathBuf) -> Self {
        Self {
            helper_path,
            shared: Arc::new(Shared {
                state: Mutex::new(ShieldState {
                    child: None,
                    live: HashMap::new(),
                    pending: HashMap::new(),
                    ended_tasks: HashSet::new(),
                    ended_order: VecDeque::new(),
                    closed: false,
                }),
                changed: Condvar::new(),
            }),
        }
    }

    fn start_locked(&self, state: &mut ShieldState) -> Result<(), String> {
        let mut child = Command::new(&self.helper_path)
            .arg("--shield")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("The activation shield helper could not start: {error}"))?;
        if child.stdin.is_none() || child.stdout.is_none() {
            let _ = child.kill();
            return Err("The activation shield helper has no command channel.".into());
        }
        let stdout = child.stdout.take().unwrap();
        state.child = Some(child);
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || {
            let mut ended_child = None;
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if line.len() > 4096 {
                    continue;
                }
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let mut guard = match shared.state.lock() {
                    Ok(guard) => guard,
                    Err(_) => break,
                };
                if message.get("type").and_then(Value::as_str) == Some("shield") {
                    if let Some(id) = message.get("id").and_then(Value::as_str) {
                        let result =
                            if message.get("state").and_then(Value::as_str) == Some("engaged") {
                                Ok(())
                            } else {
                                Err(format!(
                                    "The activation shield was refused ({}{}).",
                                    message
                                        .get("state")
                                        .and_then(Value::as_str)
                                        .unwrap_or("unknown"),
                                    message
                                        .get("code")
                                        .and_then(Value::as_str)
                                        .map(|code| format!(": {code}"))
                                        .unwrap_or_default(),
                                ))
                            };
                        if guard.pending.contains_key(id) {
                            guard.pending.insert(id.to_string(), result);
                            shared.changed.notify_all();
                        }
                    }
                } else if message.get("type").and_then(Value::as_str) == Some("error") {
                    if let Some(id) = message.get("id").and_then(Value::as_str) {
                        let text = message
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        if guard.pending.contains_key(id) {
                            guard.pending.insert(
                                id.to_string(),
                                Err(format!("The activation shield failed: {text}")),
                            );
                            shared.changed.notify_all();
                        }
                    }
                }
                drop(guard);
            }
            if let Ok(mut guard) = shared.state.lock() {
                guard.live.clear();
                for id in guard.pending.keys().cloned().collect::<Vec<_>>() {
                    guard
                        .pending
                        .insert(id, Err("The activation shield helper exited.".into()));
                }
                ended_child = guard.child.take();
                shared.changed.notify_all();
            }
            if let Some(mut child) = ended_child {
                let _ = child.wait();
            }
        });
        Ok(())
    }

    fn write_locked(state: &mut ShieldState, line: &str) -> Result<(), String> {
        let child = state
            .child
            .as_mut()
            .ok_or("The activation shield helper is not listening.")?;
        if matches!(child.try_wait(), Ok(Some(_))) {
            return Err("The activation shield helper exited.".into());
        }
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("The activation shield helper is not listening.")?;
        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|error| format!("The activation shield helper is not listening: {error}"))
    }

    fn remember_locked(state: &mut ShieldState, key: String) {
        if state.ended_tasks.insert(key.clone()) {
            state.ended_order.push_back(key);
            while state.ended_order.len() > ENDED_TASK_CAP {
                if let Some(old) = state.ended_order.pop_front() {
                    state.ended_tasks.remove(&old);
                }
            }
        }
    }

    /// Engage and wait for the helper's on-screen confirmation. The pending
    /// entry is installed before the command write, matching upstream's lost
    /// same-tick reply race.
    pub fn engage(
        &self,
        request: ShieldEngagement,
        task: Option<&ShieldTask>,
    ) -> Result<(), String> {
        let task_key = task.map(task_key);
        let started = Instant::now();
        let mut guard = self
            .shared
            .state
            .lock()
            .map_err(|_| "shield state lock".to_string())?;
        if guard.closed {
            return Err("The activation shield host is closed.".into());
        }
        if guard
            .child
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(Some(_))))
        {
            guard.child = None;
            guard.live.clear();
        }
        if guard.child.is_none() {
            self.start_locked(&mut guard)?;
        }
        guard
            .pending
            .insert(request.shield_id.clone(), Err("pending".into()));
        let label = request
            .label
            .as_deref()
            .map(|label| format!(" {label}"))
            .unwrap_or_default();
        let line = format!(
            "engage {} {} {} {} {}{}\n",
            request.shield_id,
            request.frame.x,
            request.frame.y,
            request.frame.width,
            request.frame.height,
            label,
        );
        if let Err(error) = Self::write_locked(&mut guard, &line) {
            guard.pending.remove(&request.shield_id);
            return Err(error);
        }
        loop {
            match guard.pending.get(&request.shield_id) {
                Some(Ok(())) => {
                    guard.pending.remove(&request.shield_id);
                    break;
                }
                Some(Err(error)) if error != "pending" => {
                    let error = error.clone();
                    guard.pending.remove(&request.shield_id);
                    if task_key
                        .as_ref()
                        .is_some_and(|key| guard.ended_tasks.contains(key))
                    {
                        let _ = Self::write_locked(
                            &mut guard,
                            &format!("release {}\n", request.shield_id),
                        );
                    }
                    return Err(error);
                }
                _ => {
                    let elapsed = started.elapsed();
                    if elapsed >= ENGAGE_TIMEOUT {
                        guard.pending.remove(&request.shield_id);
                        return Err("The activation shield did not confirm in time.".into());
                    }
                    let remaining = ENGAGE_TIMEOUT - elapsed;
                    let (next, timeout) = self
                        .shared
                        .changed
                        .wait_timeout(guard, remaining)
                        .map_err(|_| "shield state lock".to_string())?;
                    guard = next;
                    if timeout.timed_out() {
                        guard.pending.remove(&request.shield_id);
                        return Err("The activation shield did not confirm in time.".into());
                    }
                }
            }
        }
        guard.live.insert(
            request.shield_id.clone(),
            LiveShield {
                task_key: task_key.clone(),
                thread_id: task.map(|task| task.thread_id.clone()),
            },
        );
        if task_key
            .as_ref()
            .is_some_and(|key| guard.ended_tasks.contains(key))
        {
            let _ = Self::write_locked(&mut guard, &format!("release {}\n", request.shield_id));
        }
        Ok(())
    }

    pub fn release(&self, shield_id: &str) {
        if let Ok(mut guard) = self.shared.state.lock() {
            guard.live.remove(shield_id);
            if guard.child.is_some() {
                let _ = Self::write_locked(&mut guard, &format!("release {shield_id}\n"));
            }
        }
    }

    pub fn release_all(&self) -> usize {
        if let Ok(mut guard) = self.shared.state.lock() {
            let released = guard.live.len();
            guard.live.clear();
            if guard.child.is_some() {
                let _ = Self::write_locked(&mut guard, "release-all\n");
            }
            released
        } else {
            0
        }
    }

    pub fn end_task(&self, task: &ShieldTask) {
        if let Ok(mut guard) = self.shared.state.lock() {
            Self::remember_locked(&mut guard, task_key(task));
            let key = task_key(task);
            let releases = guard
                .live
                .iter()
                .filter(|(_, shield)| {
                    shield.thread_id.as_deref() == Some(task.thread_id.as_str())
                        && (task.turn_id.is_none()
                            || shield.task_key.as_deref() == Some(key.as_str()))
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in releases {
                guard.live.remove(&id);
                if guard.child.is_some() {
                    let _ = Self::write_locked(&mut guard, &format!("release {id}\n"));
                }
            }
        }
    }

    pub fn stop(&self) {
        let child = if let Ok(mut guard) = self.shared.state.lock() {
            guard.live.clear();
            for id in guard.pending.keys().cloned().collect::<Vec<_>>() {
                guard
                    .pending
                    .insert(id, Err("The activation shield host stopped.".into()));
            }
            self.shared.changed.notify_all();
            guard.child.take()
        } else {
            None
        };
        if let Some(mut child) = child {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(b"quit\n");
                let _ = stdin.flush();
            }
            for _ in 0..15 {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            terminate_child_gracefully(&mut child);
            let _ = child.wait();
        }
    }

    pub fn dispose(&self) {
        if let Ok(mut guard) = self.shared.state.lock() {
            guard.closed = true;
        }
        self.stop();
    }
}

impl Drop for ComputerShield {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Resolve the packaged or development AppSnap helper from the backend. In a
/// packaged macOS app the backend is Contents/MacOS/index and the helper is
/// Contents/Helpers/synara-appsnap-helper; in development the prebuilt helper
/// lives beside its Swift sources.
pub fn resolve_appsnap_helper() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SYNARA_APPSNAP_HELPER")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return Some(path);
    }
    if let Ok(exe) = std::env::current_exe() {
        let bundled = exe
            .parent()?
            .parent()?
            .join("Helpers")
            .join("synara-appsnap-helper");
        if bundled.is_file() {
            return Some(bundled);
        }
    }
    let development = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../native/appsnap/build/synara-appsnap-helper");
    development.is_file().then_some(development)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShieldArgs {
    Engage(ShieldEngagement),
    Release { shield_id: String },
    ReleaseAll,
}

fn valid_shield_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn coordinate(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .filter(|number| number.is_finite() && number.abs() <= 1_000_000.0)
}

fn extent(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .filter(|number| number.is_finite() && *number > 0.0 && *number <= 32_768.0)
}

fn sanitized_label(value: &Value) -> Option<String> {
    let label = value.as_str()?
        .chars()
        .filter(|character| !character.is_control() && !matches!(character, '\u{00ad}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{206f}' | '\u{feff}'))
        .collect::<String>()
        .trim()
        .chars()
        .take(160)
        .collect::<String>();
    (!label.is_empty()).then_some(label)
}

/// Validate the wire shape shared with upstream `parseCuaShieldArgs`. A
/// malformed request must be refused before any helper command is written.
pub fn parse_shield_args(value: &Value) -> Option<ShieldArgs> {
    if value.get("action").and_then(Value::as_str) == Some("release_all") {
        return Some(ShieldArgs::ReleaseAll);
    }
    let shield_id = value.get("shield_id").and_then(Value::as_str)?;
    if !valid_shield_id(shield_id) {
        return None;
    }
    match value.get("action").and_then(Value::as_str)? {
        "release" => Some(ShieldArgs::Release {
            shield_id: shield_id.to_string(),
        }),
        "engage" => {
            let frame = value.get("frame")?;
            Some(ShieldArgs::Engage(ShieldEngagement {
                shield_id: shield_id.to_string(),
                frame: ShieldFrame {
                    x: coordinate(frame.get("x")?)?,
                    y: coordinate(frame.get("y")?)?,
                    width: extent(frame.get("width")?)?,
                    height: extent(frame.get("height")?)?,
                },
                window_id: value
                    .get("window_id")?
                    .as_u64()
                    .filter(|id| *id > 0 && *id <= u32::MAX as u64)?
                    as u32,
                pid: value
                    .get("pid")?
                    .as_u64()
                    .filter(|id| *id > 0 && *id <= i32::MAX as u64)? as u32,
                label: sanitized_label(value.get("label")?),
            }))
        }
        _ => None,
    }
}

/// Process-wide shield host. The computer backend owns one helper generation;
/// explicit shutdown is required because a process-static value never runs Drop.
pub fn shared_host() -> Option<&'static ComputerShield> {
    static HOST: std::sync::OnceLock<Option<ComputerShield>> = std::sync::OnceLock::new();
    HOST.get_or_init(|| resolve_appsnap_helper().map(ComputerShield::new))
        .as_ref()
}

pub fn stop_shared_host() {
    if let Some(host) = shared_host() {
        host.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;
    static TEST_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn fake_helper(script: &str) -> (PathBuf, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "synara-shield-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let helper = directory.join("helper.sh");
        std::fs::write(&helper, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        (directory, helper)
    }

    fn engagement() -> ShieldEngagement {
        ShieldEngagement {
            shield_id: "shield-1".into(),
            frame: ShieldFrame {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0,
            },
            window_id: 10,
            pid: 20,
            label: None,
        }
    }

    #[test]
    fn engage_waits_for_confirmation_and_release_all_counts_live() {
        let (directory, helper) = fake_helper(
            "#!/bin/sh\nwhile IFS= read -r line; do case \"$line\" in engage*) printf '%s\\n' '{\"type\":\"shield\",\"id\":\"shield-1\",\"state\":\"engaged\"}' ;; quit*) exit 0 ;; esac; done\n",
        );
        let shield = ComputerShield::new(helper);
        shield.engage(engagement(), None).unwrap();
        assert_eq!(shield.release_all(), 1);
        shield.stop();
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn refused_engagement_surfaces_helper_reason() {
        let (directory, helper) = fake_helper(
            "#!/bin/sh\nwhile IFS= read -r line; do case \"$line\" in engage*) printf '%s\\n' '{\"type\":\"shield\",\"id\":\"shield-1\",\"state\":\"refused\",\"code\":\"screen-share\"}' ;; quit*) exit 0 ;; esac; done\n",
        );
        let shield = ComputerShield::new(helper);
        let error = shield.engage(engagement(), None).unwrap_err();
        assert!(
            error.contains("refused") && error.contains("screen-share"),
            "{error}"
        );
        shield.stop();
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn helper_exit_fails_pending_engagement() {
        let (directory, helper) = fake_helper("#!/bin/sh\nexit 0\n");
        let shield = ComputerShield::new(helper);
        let error = shield.engage(engagement(), None).unwrap_err();
        assert!(
            error.contains("exited") || error.contains("confirm"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(directory);
    }
    #[test]
    fn parses_wire_shield_arguments() {
        let args = parse_shield_args(&serde_json::json!({
            "action":"engage","shield_id":"ok_1","frame":{"x":1.0,"y":2.0,"width":3.0,"height":4.0},
            "window_id":10,"pid":20,"label":" agent\u{202e} label "
        }))
        .unwrap();
        assert_eq!(
            args,
            ShieldArgs::Engage(ShieldEngagement {
                shield_id: "ok_1".into(),
                frame: ShieldFrame {
                    x: 1.0,
                    y: 2.0,
                    width: 3.0,
                    height: 4.0
                },
                window_id: 10,
                pid: 20,
                label: Some("agent label".into()),
            })
        );
        assert!(
            parse_shield_args(&serde_json::json!({"action":"engage","shield_id":"bad id"}))
                .is_none()
        );
        assert_eq!(
            parse_shield_args(&serde_json::json!({"action":"release_all"})),
            Some(ShieldArgs::ReleaseAll)
        );
    }
}
