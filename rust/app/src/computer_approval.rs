//! Synara-owned Computer consent gate, port of upstream
//! computer/ComputerApprovalGate.ts. Consent is scoped to one live turn,
//! never a provider-wide grant: acceptForSession is deliberately declined.
//! Routine and foreground (visible-use) consent are separate — background
//! input authorization never lets a task take the user's screen. A desktop
//! interruption revokes standing grants but declines and pending prompts
//! survive: an answer that arrives after the interruption IS the re-auth.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const COMPUTER_APPROVAL_QUEUE_FULL_CODE: &str = "approval_queue_full";
pub const COMPUTER_APPROVAL_QUEUE_GLOBAL_LIMIT: usize = 128;
pub const COMPUTER_APPROVAL_QUEUE_THREAD_LIMIT: usize = 8;
const PROMPT_EXPIRY: std::time::Duration = std::time::Duration::from_secs(5 * 60);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Accept,
    Decline,
    /// The prompt died before anyone answered — nothing was decided and a
    /// fresh prompt may be shown. Never report this as a user refusal.
    Cancel,
    Expired,
}

struct PendingApproval {
    thread_id: String,
    turn_id: Option<String>,
    settle: tokio::sync::oneshot::Sender<Outcome>,
}

#[derive(Clone)]
struct TaskApproval {
    turn_id: String,
    granted: Option<bool>,
    /// The in-flight prompt is shared by every concurrent caller of the same
    /// turn — the first publishes, followers await the same decision.
    pending: Option<tokio::sync::watch::Receiver<Option<Outcome>>>,
}

pub struct ComputerApprovalGate {
    pending: Mutex<HashMap<String, PendingApproval>>,
    tasks: Mutex<HashMap<String, TaskApproval>>,
    foreground_tasks: Mutex<HashMap<String, TaskApproval>>,
}

pub fn gate() -> &'static ComputerApprovalGate {
    static GATE: OnceLock<ComputerApprovalGate> = OnceLock::new();
    GATE.get_or_init(|| ComputerApprovalGate {
        pending: Mutex::new(HashMap::new()),
        tasks: Mutex::new(HashMap::new()),
        foreground_tasks: Mutex::new(HashMap::new()),
    })
}

fn cancel_map_entries(
    tasks: &Mutex<HashMap<String, TaskApproval>>,
    thread_id: &str,
    turn_id: Option<&str>,
) {
    let mut guard = tasks.lock().unwrap();
    let remove = match (guard.get(thread_id), turn_id) {
        (Some(task), Some(turn)) => task.turn_id != turn,
        (Some(_), None) => true,
        (None, _) => false,
    };
    if remove {
        guard.remove(thread_id);
    }
}

impl ComputerApprovalGate {
    /// Thread or turn terminal events discard pending prompts; restart and
    /// Stop take this path.
    pub fn cancel_thread(&self, thread_id: &str, turn_id: Option<&str>) {
        cancel_map_entries(&self.tasks, thread_id, turn_id);
        cancel_map_entries(&self.foreground_tasks, thread_id, turn_id);
        let mut pending = self.pending.lock().unwrap();
        let ids: Vec<String> = pending
            .iter()
            .filter(|(_, p)| {
                p.thread_id == thread_id && (turn_id.is_none() || p.turn_id.as_deref() == turn_id)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(p) = pending.remove(&id) {
                let _ = p.settle.send(Outcome::Cancel);
            }
        }
    }

    /// A desktop interruption (screen lock, sleep, session switch) revokes
    /// every standing grant: consent answered before the interruption must
    /// not silently authorize the post-interruption desktop.
    pub fn revoke_task_grants(&self) {
        for tasks in [&self.tasks, &self.foreground_tasks] {
            let mut guard = tasks.lock().unwrap();
            for task in guard.values_mut() {
                if task.granted == Some(true) {
                    task.granted = None;
                }
            }
        }
    }

    /// Whether the user approved visible use for this exact turn. Never prompts.
    pub fn has_foreground_grant(&self, thread_id: &str, turn_id: &str) -> bool {
        self.foreground_tasks
            .lock()
            .unwrap()
            .get(thread_id)
            .is_some_and(|task| task.turn_id == turn_id && task.granted == Some(true))
    }

    /// Settles a published prompt. acceptForSession is deliberately
    /// unavailable for this gate — it degrades to decline.
    pub fn respond(&self, thread_id: &str, request_id: &str, decision: &str) -> bool {
        let mut pending = self.pending.lock().unwrap();
        let Some(entry) = pending.get(request_id) else {
            return false;
        };
        if entry.thread_id != thread_id {
            return false;
        }
        let entry = pending.remove(request_id).unwrap();
        let outcome = match decision {
            "accept" => Outcome::Accept,
            _ => Outcome::Decline,
        };
        let _ = entry.settle.send(outcome);
        // Wake any shared waiters of this turn's task prompt.
        for tasks in [&self.tasks, &self.foreground_tasks] {
            let mut guard = tasks.lock().unwrap();
            if let Some(task) = guard.get_mut(thread_id) {
                if let Some(rx) = &task.pending {
                    let _ = rx.clone();
                }
                if task.turn_id == entry.turn_id.clone().unwrap_or_default()
                    || entry.turn_id.is_none()
                {
                    task.granted = Some(outcome == Outcome::Accept);
                    task.pending = None;
                }
            }
        }
        true
    }

    /// One consent for the exact active turn. Grants stick for the turn;
    /// a new turn invalidates them. `publish` shows the consent card with the
    /// request id and is called at most once per turn-prompt.
    pub async fn request_task(
        &self,
        thread_id: &str,
        turn_id: &str,
        foreground: bool,
        publish: impl Fn(String) -> futures_util::future::BoxFuture<'static, ()> + Send + Sync,
    ) -> Outcome {
        let tasks_map = if foreground {
            &self.foreground_tasks
        } else {
            &self.tasks
        };
        // Resolve or establish the turn-scoped task entry.
        enum Action {
            Decided(Outcome),
            Wait(tokio::sync::watch::Receiver<Option<Outcome>>),
            Publish,
        }
        let action = {
            let guard = tasks_map.lock().unwrap();
            match guard.get(thread_id) {
                Some(task) if task.turn_id == turn_id => {
                    if let Some(granted) = task.granted {
                        Action::Decided(if granted {
                            Outcome::Accept
                        } else {
                            Outcome::Decline
                        })
                    } else if let Some(rx) = &task.pending {
                        Action::Wait(rx.clone())
                    } else {
                        Action::Publish
                    }
                }
                _ => {
                    // A new turn ends every earlier turn's consent.
                    drop(guard);
                    self.cancel_stale_turns(thread_id, turn_id);
                    let mut guard = tasks_map.lock().unwrap();
                    guard.insert(
                        thread_id.to_string(),
                        TaskApproval {
                            turn_id: turn_id.to_string(),
                            granted: None,
                            pending: None,
                        },
                    );
                    Action::Publish
                }
            }
        };
        match action {
            Action::Decided(outcome) => outcome,
            Action::Wait(mut rx) => {
                // Followers await the shared prompt's decision.
                loop {
                    if let Some(outcome) = *rx.borrow() {
                        return outcome;
                    }
                    if rx.changed().await.is_err() {
                        return Outcome::Cancel;
                    }
                }
            }
            Action::Publish => {
                self.publish_prompt(tasks_map, thread_id, turn_id, publish)
                    .await
            }
        }
    }

    async fn publish_prompt(
        &self,
        tasks_map: &Mutex<HashMap<String, TaskApproval>>,
        thread_id: &str,
        turn_id: &str,
        publish: impl Fn(String) -> futures_util::future::BoxFuture<'static, ()> + Send + Sync,
    ) -> Outcome {
        // Queue limits: a stuck turn must not starve every other chat, and a
        // full queue is a retryable refusal — never implicit denial.
        {
            let pending = self.pending.lock().unwrap();
            let thread_pending = pending
                .values()
                .filter(|p| p.thread_id == thread_id)
                .count();
            if thread_pending >= COMPUTER_APPROVAL_QUEUE_THREAD_LIMIT
                || pending.len() >= COMPUTER_APPROVAL_QUEUE_GLOBAL_LIMIT
            {
                return Outcome::Cancel;
            }
        }
        let request_id = format!("computer:{}", crate::orchestration_engine::uuid_v4());
        let (settle_tx, settle_rx) = tokio::sync::oneshot::channel::<Outcome>();
        let (watch_tx, watch_rx) = tokio::sync::watch::channel::<Option<Outcome>>(None);
        {
            self.pending.lock().unwrap().insert(
                request_id.clone(),
                PendingApproval {
                    thread_id: thread_id.to_string(),
                    turn_id: Some(turn_id.to_string()),
                    settle: settle_tx,
                },
            );
            let mut guard = tasks_map.lock().unwrap();
            if let Some(task) = guard.get_mut(thread_id) {
                task.pending = Some(watch_rx);
            }
        }
        publish(request_id.clone()).await;
        let outcome = match tokio::time::timeout(PROMPT_EXPIRY, settle_rx).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Outcome::Cancel,
            Err(_) => Outcome::Expired,
        };
        {
            self.pending.lock().unwrap().remove(&request_id);
            let mut guard = tasks_map.lock().unwrap();
            if let Some(task) = guard.get_mut(thread_id) {
                if task.turn_id == turn_id && task.granted.is_none() {
                    if matches!(outcome, Outcome::Accept | Outcome::Decline) {
                        task.granted = Some(outcome == Outcome::Accept);
                    }
                    task.pending = None;
                }
            }
            let _ = watch_tx.send(Some(outcome));
        }
        outcome
    }

    fn cancel_stale_turns(&self, thread_id: &str, turn_id: &str) {
        for tasks in [&self.tasks, &self.foreground_tasks] {
            let mut guard = tasks.lock().unwrap();
            if guard.get(thread_id).is_some_and(|t| t.turn_id != turn_id) {
                guard.remove(thread_id);
            }
        }
        let mut pending = self.pending.lock().unwrap();
        let ids: Vec<String> = pending
            .iter()
            .filter(|(_, p)| p.thread_id == thread_id && p.turn_id.as_deref() != Some(turn_id))
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(p) = pending.remove(&id) {
                let _ = p.settle.send(Outcome::Cancel);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_gate() -> ComputerApprovalGate {
        ComputerApprovalGate {
            pending: Mutex::new(HashMap::new()),
            tasks: Mutex::new(HashMap::new()),
            foreground_tasks: Mutex::new(HashMap::new()),
        }
    }

    #[tokio::test]
    async fn accept_sticks_for_the_turn() {
        let gate = fresh_gate();
        let publish =
            |_id: String| Box::pin(async move {}) as futures_util::future::BoxFuture<'static, ()>;
        let request = gate.request_task("t1", "turn1", false, publish);
        tokio::pin!(request);
        // Let the prompt publish, then answer it.
        tokio::select! {
            _ = &mut request => panic!("prompt resolved before any answer"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(60)) => {}
        }
        let request_id = gate.pending.lock().unwrap().keys().next().cloned().unwrap();
        assert!(gate.respond("t1", &request_id, "accept"));
        let outcome = request.await;
        assert_eq!(outcome, Outcome::Accept);
        // Same turn, decided without prompting again.
        let outcome2 = gate
            .request_task("t1", "turn1", false, |_id| {
                Box::pin(async move {}) as futures_util::future::BoxFuture<'static, ()>
            })
            .await;
        assert_eq!(outcome2, Outcome::Accept);
    }

    #[tokio::test]
    async fn session_wide_acceptance_degrades_to_decline() {
        let gate = fresh_gate();
        let publish =
            |_id: String| Box::pin(async move {}) as futures_util::future::BoxFuture<'static, ()>;
        let request = gate.request_task("t2", "turn1", false, publish);
        tokio::pin!(request);
        tokio::select! {
            _ = &mut request => panic!("prompt resolved before any answer"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(60)) => {}
        }
        let request_id = gate.pending.lock().unwrap().keys().next().cloned().unwrap();
        assert!(gate.respond("t2", &request_id, "acceptForSession"));
        assert_eq!(request.await, Outcome::Decline);
    }

    #[test]
    fn revoke_clears_grants_but_not_declines() {
        let gate = fresh_gate();
        gate.tasks.lock().unwrap().insert(
            "t3".to_string(),
            TaskApproval {
                turn_id: "turn1".to_string(),
                granted: Some(true),
                pending: None,
            },
        );
        gate.tasks.lock().unwrap().insert(
            "t4".to_string(),
            TaskApproval {
                turn_id: "turn1".to_string(),
                granted: Some(false),
                pending: None,
            },
        );
        gate.revoke_task_grants();
        assert_eq!(gate.tasks.lock().unwrap().get("t3").unwrap().granted, None);
        assert_eq!(
            gate.tasks.lock().unwrap().get("t4").unwrap().granted,
            Some(false)
        );
    }
}
