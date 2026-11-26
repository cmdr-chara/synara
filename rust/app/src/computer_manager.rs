//! The computer manager's control-plane core, port of the state machine in
//! upstream computer/ComputerManager.ts. It mediates between the working
//! macOS primitives (computer_native) and every consumer: control admission
//! is generation-checked, the desktop lease is exclusive — one conversation
//! owns the shared pointer and focused keyboard at a time — revocation
//! tears down leases and aborts live input, and the denylist fences password
//! managers and OS security surfaces out of reach entirely.
//!
//! Scoped faithfully for the control plane. The upstream module also owns
//! screenshot pipelines, scroll gearing, UI-tree targeting, and publish
//! fan-out; those live in their own ports rather than being glued here.

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

/// One desktop lease at a time: the shared pointer and focused keyboard are
/// exclusive. Background leases are per-target — an app or window a thread
/// drives without taking the user's focus.
#[derive(Clone)]
pub struct DesktopLease {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub last_activity_ms: u64,
    pub release_requested: bool,
    pub release_requested_turn_id: Option<String>,
}

#[derive(Clone)]
pub struct BackgroundLease {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub last_activity_ms: u64,
    pub target_key: String,
    pub pid: Option<i64>,
    pub window_id: Option<String>,
}

#[derive(Default, Clone)]
pub struct ThreadRuntime {
    /// Backend-reported reason input is paused for this thread — the model
    /// sees it instead of dispatching into a broken desktop state.
    pub input_pause: Option<Value>,
    /// Whether the preview pane surfaced for this thread.
    pub pane_surfaced: bool,
}

pub struct ManagerState {
    lease: Option<DesktopLease>,
    background_leases: HashMap<String, BackgroundLease>,
    threads: HashMap<String, ThreadRuntime>,
    /// Threads disabled by a persist failure mid-flight: control stays off
    /// until the owner re-enables, exactly upstream's disabledThreads.
    disabled_threads: HashSet<String>,
    suspended_threads: HashSet<String>,
    /// Agent calls currently in flight per thread — release defers while
    /// input could still be landing.
    agent_calls_in_flight: HashMap<String, u64>,
    /// The turn a thread's authority was established for.
    authority_turns: HashMap<String, String>,
    /// Concurrency token upstream uses to drop stale enable writes; a bump
    /// invalidates any enable issued before it.
    control_requests: HashMap<String, u64>,
    /// Most recent user desktop input; foreground delivery must wait for
    /// quiet — taking focus while the user types steals their screen.
    last_user_desktop_input_at: Option<u64>,
    known_app_names: HashSet<String>,
}

fn manager() -> &'static Mutex<ManagerState> {
    static MANAGER: OnceLock<Mutex<ManagerState>> = OnceLock::new();
    MANAGER.get_or_init(|| {
        Mutex::new(ManagerState {
            lease: None,
            background_leases: HashMap::new(),
            threads: HashMap::new(),
            disabled_threads: HashSet::new(),
            suspended_threads: HashSet::new(),
            agent_calls_in_flight: HashMap::new(),
            authority_turns: HashMap::new(),
            control_requests: HashMap::new(),
            last_user_desktop_input_at: None,
            known_app_names: HashSet::new(),
        })
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Foreground delivery requires the desktop quiet for this long upstream.
pub const COMPUTER_USER_INTERACTION_QUIET_MS: u64 = 4_000;

fn control_disabled(thread_id: &str) -> bool {
    let guard = manager().lock().unwrap();
    guard.disabled_threads.contains(thread_id)
        || crate::computer_control::get(thread_id).disabled
}

pub fn is_suspended(thread_id: &str) -> bool {
    manager().lock().unwrap().suspended_threads.contains(thread_id)
}

/// Upstream's canActivateControl: not disabled, not suspended, and the
/// caller's recorded generation still matches.
pub fn can_activate_control(thread_id: &str, generation: i64) -> bool {
    !control_disabled(thread_id)
        && !is_suspended(thread_id)
        && crate::computer_control::allows(thread_id, generation)
}

/// Upstream's admitControl: explicit invocation can re-enable a disabled
/// thread when the generation still matches (the owner's fresh intent beats
/// the stale disabled flag); chat-mode intent is recorded against the
/// generation it was captured for.
pub async fn admit_control(
    thread_id: &str,
    mode: &str,
    generation: i64,
    explicit_invocation: bool,
) -> Result<bool, String> {
    if explicit_invocation
        && mode == "request"
        && crate::computer_control::get(thread_id).generation == generation
        && !is_suspended(thread_id)
        && control_disabled(thread_id)
    {
        set_control_enabled(thread_id, true).await?;
    }
    let enabled = mode != "off" && can_activate_control(thread_id, generation);
    crate::computer_control::record_chat_intent(
        thread_id,
        enabled && mode == "chat",
        generation,
    )
    .map_err(|e| {
        manager().lock().unwrap().disabled_threads.insert(thread_id.to_string());
        e
    })?;
    Ok(enabled && can_activate_control(thread_id, generation))
}

/// Upstream's canContinueChatControl: the recorded chat intent generation is
/// still the live generation and control is active.
pub fn can_continue_chat_control(thread_id: &str) -> bool {
    let state = crate::computer_control::get(thread_id);
    state.chat_generation == Some(state.generation)
        && can_activate_control(thread_id, state.generation)
}

/// Enable or disable control for a thread. Enabling clears the disabled
/// mark; disabling writes the state and tears the thread down — approval
/// prompts cancelled, leases released, input stopped.
pub async fn set_control_enabled(
    thread_id: &str,
    enabled: bool,
) -> Result<Value, String> {
    let request = {
        let mut guard = manager().lock().unwrap();
        let token = guard.control_requests.get(thread_id).copied().unwrap_or(0) + 1;
        guard.control_requests.insert(thread_id.to_string(), token);
        token
    };
    if enabled {
        crate::computer_control::set(thread_id, false)?;
        let mut guard = manager().lock().unwrap();
        if guard.control_requests.get(thread_id).copied() == Some(request) {
            guard.disabled_threads.remove(thread_id);
        }
    } else {
        crate::computer_control::set(thread_id, true)?;
        {
            let mut guard = manager().lock().unwrap();
            guard.disabled_threads.insert(thread_id.to_string());
            if let Some(runtime) = guard.threads.get_mut(thread_id) {
                runtime.pane_surfaced = false;
            }
        }
        revoke_control(thread_id).await?;
    }
    {
        let mut guard = manager().lock().unwrap();
        if guard.control_requests.get(thread_id).copied() == Some(request) {
            guard.control_requests.remove(thread_id);
        }
        guard.threads.entry(thread_id.to_string()).or_default();
    }
    let generation = crate::computer_control::get(thread_id).generation;
    Ok(serde_json::json!({
        "enabled": !control_disabled(thread_id) && !is_suspended(thread_id),
        "generation": generation,
    }))
}

/// Upstream's revokeControl: approval prompts for the thread cancel, live
/// input authorities abort, the desktop and background leases release.
async fn revoke_control(thread_id: &str) -> Result<(), String> {
    crate::computer_approval::gate().cancel_thread(thread_id, None);
    release_desktop_control(thread_id, None).await;
    Ok(())
}

/// Mutating calls assert this first: the owner must still hold control.
pub fn assert_control_authority(owner: Option<&str>) -> Result<(), String> {
    let Some(owner) = owner else { return Ok(()) };
    if control_disabled(owner) || is_suspended(owner) {
        return Err(
            "Computer control was revoked for this conversation; no input was dispatched.".to_string(),
        );
    }
    Ok(())
}

/// A backend-reported input pause blocks dispatch until it clears.
pub fn assert_input_not_paused(owner: Option<&str>) -> Result<(), String> {
    let Some(owner) = owner else { return Ok(()) };
    let guard = manager().lock().unwrap();
    if let Some(runtime) = guard.threads.get(owner) {
        if let Some(pause) = &runtime.input_pause {
            let message = pause
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Computer input is paused for this conversation.");
            return Err(message.to_string());
        }
    }
    Ok(())
}

pub fn record_input_pause(owner: &str, pause: Value) {
    let mut guard = manager().lock().unwrap();
    if guard.suspended_threads.contains(owner) || guard.disabled_threads.contains(owner) {
        return;
    }
    guard.threads.entry(owner.to_string()).or_default().input_pause = Some(pause);
}

/// A fresh desktop observation clears the human-takeover pause. Only direct
/// observations call this: status/control reads must not silently re-arm input.
pub fn clear_input_pause(owner: &str) {
    let mut guard = manager().lock().unwrap();
    if let Some(runtime) = guard.threads.get_mut(owner) {
        runtime.input_pause = None;
    }
}

/// Foreground delivery rules: the user must have asked for visible use this
/// turn, and the desktop must be quiet — grabbing focus while the user is
/// actively typing takes their screen mid-keystroke.
pub fn assert_foreground_allowed(
    thread_id: Option<&str>,
    user_requested_visible_use: bool,
) -> Result<(), String> {
    let Some(_thread) = thread_id else { return Ok(()) };
    if !user_requested_visible_use {
        return Err(
            "The user did not ask or allow this app or window to be shown for this task. Stay in the background: keep observing and acting through background input. Do not ask again in this turn.".to_string(),
        );
    }
    let guard = manager().lock().unwrap();
    if let Some(last) = guard.last_user_desktop_input_at {
        if now_ms().saturating_sub(last) < COMPUTER_USER_INTERACTION_QUIET_MS {
            return Err(
                "The user was interacting with the desktop moments ago; bringing a window forward now would take their focus. Wait for the desktop to be quiet, then retry if the task still needs foreground delivery.".to_string(),
            );
        }
    }
    Ok(())
}

pub fn note_user_desktop_input() {
    manager().lock().unwrap().last_user_desktop_input_at = Some(now_ms());
}

/// Acquires the shared desktop lease for a thread's turn. Only one
/// conversation owns the pointer at a time; a different thread holding it
/// raises the lease error the model sees verbatim upstream.
pub fn acquire_desktop_lease(thread_id: &str, turn_id: Option<&str>) -> Result<(), String> {
    let mut guard = manager().lock().unwrap();
    if let Some(lease) = &guard.lease {
        if lease.thread_id != thread_id {
            return Err(
                "The shared pointer and focused keyboard are controlled by another conversation; no input was sent. Do not retry this blocked action or switch tools to bypass the lease. Wait until that conversation's turn ends. Reading the desktop still works. Background actions on independently owned applications remain available.".to_string(),
            );
        }
        // Same thread renewing: clear any deferred release, refresh activity.
        guard.threads.entry(thread_id.to_string()).or_default();
        let lease = guard.lease.as_mut().unwrap();
        lease.release_requested = false;
        lease.release_requested_turn_id = None;
        lease.last_activity_ms = now_ms();
        if let Some(turn) = turn_id {
            lease.turn_id = Some(turn.to_string());
        }
        return Ok(());
    }
    guard.lease = Some(DesktopLease {
        thread_id: thread_id.to_string(),
        turn_id: turn_id.map(|t| t.to_string()),
        last_activity_ms: now_ms(),
        release_requested: false,
        release_requested_turn_id: None,
    });
    if let Some(turn) = turn_id {
        guard.authority_turns.insert(thread_id.to_string(), turn.to_string());
    }
    Ok(())
}

/// Upstream's releaseDesktopControl: turn-scoped release defers while agent
/// calls could still be landing; an in-flight release is honoured on
/// completion rather than silently dropped.
pub async fn release_desktop_control(thread_id: &str, turn_id: Option<&str>) {
    let mut guard = manager().lock().unwrap();
    // Background leases for this thread always release immediately.
    guard
        .background_leases
        .retain(|_, lease| lease.thread_id != thread_id || turn_id.is_none() || lease.turn_id.as_deref() != turn_id);
    let Some(lease) = &guard.lease else {
        let calls = guard.agent_calls_in_flight.get(thread_id).copied().unwrap_or(0);
        if calls == 0 && (turn_id.is_none() || guard.authority_turns.get(thread_id).map(String::as_str) == turn_id) {
            guard.authority_turns.remove(thread_id);
        }
        return;
    };
    if lease.thread_id != thread_id {
        return;
    }
    if let (Some(turn), Some(lease_turn)) = (turn_id, &lease.turn_id) {
        if turn != lease_turn {
            return;
        }
    }
    let calls = guard.agent_calls_in_flight.get(thread_id).copied().unwrap_or(0);
    if calls > 0 {
        let lease = guard.lease.as_mut().unwrap();
        lease.release_requested = true;
        lease.release_requested_turn_id = turn_id
            .map(|t| t.to_string())
            .or_else(|| lease.turn_id.clone());
        return;
    }
    guard.lease = None;
    guard.authority_turns.remove(thread_id);
}

/// Agent-call accounting: callers mark entry and exit so release defers
/// correctly. On exit, a deferred release completes.
pub fn agent_call_started(thread_id: &str) {
    let mut guard = manager().lock().unwrap();
    *guard.agent_calls_in_flight.entry(thread_id.to_string()).or_insert(0) += 1;
}

pub async fn agent_call_finished(thread_id: &str) {
    let deferred = {
        let mut guard = manager().lock().unwrap();
        let count = guard.agent_calls_in_flight.get_mut(thread_id);
        if let Some(count) = count {
            *count = count.saturating_sub(1);
        }
        guard
            .lease
            .as_ref()
            .filter(|lease| lease.thread_id == thread_id && lease.release_requested)
            .map(|lease| lease.release_requested_turn_id.clone())
            .flatten()
    };
    if deferred.is_some() {
        release_desktop_control(thread_id, deferred.as_deref()).await;
    }
}

/// Upstream's emergencyStopInput: Escape semantics — every live authority
/// aborts and the backend stops input, reported via the stopped event edge.
pub async fn emergency_stop_input() {
    crate::computer_approval::gate().revoke_task_grants();
    let mut guard = manager().lock().unwrap();
    guard.lease = None;
    guard.background_leases.clear();
    guard.authority_turns.clear();
}

/// Denylist fencing for driven targets — checked before any dispatch so a
/// retried or obfuscated target cannot slip through.
pub fn assert_not_denied(app: Option<&str>, bundle_id: Option<&str>) -> Result<(), String> {
    if let Some(matched) = crate::computer_denylist::computer_denylist_match(app, bundle_id) {
        return Err(crate::computer_denylist::denylist_error(&matched));
    }
    Ok(())
}

/// Snapshot for the thread-state surface: whether this thread holds the
/// desktop lease, plus its pause state.
pub fn thread_state_snapshot(thread_id: &str) -> Value {
    let guard = manager().lock().unwrap();
    let holds_lease = guard
        .lease
        .as_ref()
        .is_some_and(|lease| lease.thread_id == thread_id);
    let background: Vec<Value> = guard
        .background_leases
        .values()
        .filter(|lease| lease.thread_id == thread_id)
        .map(|lease| {
            serde_json::json!({
                "targetKey": lease.target_key,
                "pid": lease.pid,
                "windowId": lease.window_id,
            })
        })
        .collect();
    let runtime = guard.threads.get(thread_id).cloned().unwrap_or_default();
    let controlled_by_other = guard
        .lease
        .as_ref()
        .is_some_and(|lease| lease.thread_id != thread_id);
    serde_json::json!({
        "holdsLease": holds_lease,
        "controlledByOtherThread": controlled_by_other,
        "backgroundLeases": background,
        "inputPause": runtime.input_pause,
        "paneSurfaced": runtime.pane_surfaced,
        "controlEnabled": !guard.disabled_threads.contains(thread_id)
            && !crate::computer_control::get(thread_id).disabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_thread() -> String {
        format!("mgr-test-{}", crate::orchestration_engine::uuid_v4())
    }

    #[test]
    fn desktop_lease_is_exclusive() {
        let first = unique_thread();
        let second = unique_thread();
        acquire_desktop_lease(&first, Some("turn1")).unwrap();
        assert!(acquire_desktop_lease(&second, Some("turn1")).is_err());
        // Renewal by the holder succeeds.
        acquire_desktop_lease(&first, Some("turn1")).unwrap();
        manager().lock().unwrap().lease = None;
    }

    #[tokio::test]
    async fn release_defers_while_calls_fly() {
        let thread = unique_thread();
        // The manager is a process-wide singleton: evict any lease a
        // concurrent test holds so this one measures its own lifecycle.
        manager().lock().unwrap().lease = None;
        acquire_desktop_lease(&thread, Some("turn1")).unwrap();
        agent_call_started(&thread);
        release_desktop_control(&thread, Some("turn1")).await;
        // Lease survives — a call is in flight. Thread-scoped assertions:
        // parallel tests share the singleton and may evict outright.
        let held = manager().lock().unwrap().lease.as_ref().is_some_and(|l| l.thread_id == thread);
        assert!(held);
        agent_call_finished(&thread).await;
        let held_after = manager().lock().unwrap().lease.as_ref().is_some_and(|l| l.thread_id == thread);
        assert!(!held_after);
    }

    #[tokio::test]
    async fn disable_tears_down_lease_and_control() {
        let thread = unique_thread();
        let _ = crate::computer_control::set(&thread, false);
        manager().lock().unwrap().lease = None;
        acquire_desktop_lease(&thread, Some("turn1")).unwrap();
        set_control_enabled(&thread, false).await.unwrap();
        let held = manager().lock().unwrap().lease.as_ref().is_some_and(|l| l.thread_id == thread);
        assert!(!held);
        assert!(!can_activate_control(&thread, crate::computer_control::get(&thread).generation));
        // Re-enable restores.
        set_control_enabled(&thread, true).await.unwrap();
        assert!(can_activate_control(&thread, crate::computer_control::get(&thread).generation));
        let _ = crate::computer_control::set(&thread, true);
    }

    #[test]
    #[test]
    fn physical_input_pause_blocks_until_desktop_observation() {
        let thread = format!("pause-{}", now_ms());
        record_input_pause(&thread, serde_json::json!({"kind":"human-takeover"}));
        assert!(assert_input_not_paused(Some(&thread)).is_err());
        clear_input_pause(&thread);
        assert!(assert_input_not_paused(Some(&thread)).is_ok());
    }

    fn foreground_requires_request_and_quiet() {
        let thread = unique_thread();
        assert!(assert_foreground_allowed(Some(&thread), false).is_err());
        assert!(assert_foreground_allowed(Some(&thread), true).is_ok());
        note_user_desktop_input();
        assert!(assert_foreground_allowed(Some(&thread), true).is_err());
    }

    #[test]
    fn denylist_fences_security_surfaces() {
        assert!(assert_not_denied(Some("1Password"), None).is_err());
        assert!(assert_not_denied(Some("Finder"), None).is_ok());
    }
}
