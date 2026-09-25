//! Explicit process-session scheduler. At-most-once claims, no automatic retries.
use super::*;
use crate::Controller;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct AutomationScheduler {
    controller: Arc<Controller>,
    owner: AutomationId,
    armed: AtomicBool,
    busy: AtomicBool,
    stop_epoch: AtomicU64,
    cancel: Mutex<Option<CancellationToken>>,
}
struct Busy<'a>(&'a AtomicBool);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl AutomationScheduler {
    pub fn new(controller: Arc<Controller>) -> Self {
        Self {
            controller,
            owner: AutomationId::new_v4(),
            armed: AtomicBool::new(false),
            busy: AtomicBool::new(false),
            stop_epoch: AtomicU64::new(0),
            cancel: Mutex::new(None),
        }
    }
    pub fn owner(&self) -> AutomationId {
        self.owner
    }
    pub fn armed(&self) -> bool {
        self.armed.load(Ordering::Acquire)
    }
    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }
    pub fn arm(&self, enabled: bool) {
        self.armed.store(enabled, Ordering::Release);
    }
    /// Also invalidates queued futures that have not reached the runtime yet.
    pub fn stop(&self) {
        self.stop_epoch.fetch_add(1, Ordering::AcqRel);
        if let Ok(cancel) = self.cancel.lock()
            && let Some(token) = cancel.as_ref()
        {
            token.cancel();
        }
    }
    pub fn tick(self: &Arc<Self>) -> impl Future<Output = WorkspaceResult<()>> + Send + 'static {
        let scheduler = self.clone();
        let epoch = self.stop_epoch.load(Ordering::Acquire);
        async move {
            if !scheduler.armed() || scheduler.busy() {
                return Ok(());
            }
            let ledger = scheduler.controller.workspace.automations().await?;
            let now = now_ms();
            let next = ledger
                .definitions
                .iter()
                .filter(|d| d.enabled && d.next_run_ms <= now)
                .filter(|d| {
                    !ledger.runs.iter().any(|r| {
                        r.definition.id == d.id && r.status == AutomationRunStatus::Running
                    })
                })
                .min_by_key(|d| d.next_run_ms)
                .map(|d| (d.id, d.revision));
            if let Some((id, revision)) = next
                && scheduler.armed()
            {
                scheduler.execute(id, revision, true, epoch).await?;
            }
            Ok(())
        }
    }
    /// Capture confirmation revision and cancellation epoch on the caller's thread.
    pub fn run_now(
        self: &Arc<Self>,
        id: AutomationId,
        revision: u64,
    ) -> impl Future<Output = WorkspaceResult<()>> + Send + 'static {
        let scheduler = self.clone();
        let epoch = self.stop_epoch.load(Ordering::Acquire);
        async move { scheduler.execute(id, revision, false, epoch).await }
    }
    async fn execute(
        &self,
        id: AutomationId,
        revision: u64,
        scheduled: bool,
        epoch: u64,
    ) -> WorkspaceResult<()> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(invalid(
                "Another automation is active. Stop or wait for it before running another.",
            ));
        }
        let _busy = Busy(&self.busy);
        let cancel = CancellationToken::new();
        *self
            .cancel
            .lock()
            .map_err(|_| invalid("Scheduler lock unavailable."))? = Some(cancel.clone());
        if epoch != self.stop_epoch.load(Ordering::Acquire) || cancel.is_cancelled() {
            return Err(invalid("Queued automation cancelled before claim."));
        }
        if scheduled && !self.armed() {
            return Ok(());
        }
        let Some(run) = self
            .controller
            .workspace
            .claim_automation_revision(id, self.owner, scheduled, now_ms(), Some(revision))
            .await?
        else {
            return Ok(());
        };
        let task_id = run
            .task_id
            .ok_or_else(|| invalid("Claim did not create its owned task."))?;
        let prompt = if run.prompt.is_empty() {
            run.definition.instructions.clone()
        } else {
            run.prompt.clone()
        };
        let before_messages = match self.controller.workspace.task(task_id).await {
            Ok(task) => self
                .controller
                .workspace
                .thread(task.thread_id)
                .await
                .ok()
                .map(|thread| thread.messages.len()),
            Err(_) => None,
        };
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(invalid("Automation cancelled before or during execution.")),
            result = tokio::time::timeout(Duration::from_secs(u64::from(run.definition.max_runtime_seconds)), self.controller.submit(task_id, prompt)) => result.unwrap_or_else(|_| Err(invalid(format!("Automation exceeded its {}-second execution limit.", run.definition.max_runtime_seconds)))),
        };
        let (status, output) = match result {
            Ok(_) => {
                // A display/read failure must not strand a completed reservation as Running.
                let output = async {
                    let task = self.controller.workspace.task(task_id).await?;
                    let thread = self.controller.workspace.thread(task.thread_id).await?;
                    let mut output = String::new();
                    if let Some(before_messages) = before_messages {
                        for message in thread
                            .messages
                            .iter()
                            .skip(before_messages)
                            .filter(|m| m.role == synara_core::Role::Assistant)
                        {
                            let remaining = 4096_usize.saturating_sub(output.chars().count());
                            if remaining == 0 { break; }
                            output.extend(message.text.chars().take(remaining));
                        }
                    }
                    if output.is_empty() { output = "Run completed. Open the owned conversation for tool output and details.".into(); }
                    Ok::<_, WorkspaceError>(output)
                }.await.unwrap_or_else(|e| format!("Provider completed, but output could not be loaded: {e}. Open the owned conversation."));
                (AutomationRunStatus::Succeeded, output)
            }
            Err(error) => {
                // Use the existing session cancellation path, preserving permission ownership.
                let _ =
                    tokio::time::timeout(Duration::from_secs(10), self.controller.cancel(task_id))
                        .await;
                (
                    if cancel.is_cancelled() {
                        AutomationRunStatus::Cancelled
                    } else {
                        AutomationRunStatus::Failed
                    },
                    error.to_string(),
                )
            }
        };
        // The exact submitted prompt remains in the immutable run snapshot. Do
        // not leave it in the composer where it could look safe to send twice.
        let _ = self
            .controller
            .workspace
            .save_task_draft(task_id, String::new())
            .await;
        let completion_output = output.clone();
        self.controller
            .workspace
            .finish_automation(run.id, self.owner, status, output)
            .await?;

        if status == AutomationRunStatus::Succeeded
            && matches!(
                &run.definition.completion_policy,
                AutomationCompletionPolicy::AiEvaluated { .. }
            )
        {
            let current = self.controller.workspace.automations().await?;
            let policy_current = current.definitions.iter().any(|definition| {
                definition.id == run.definition.id
                    && definition.revision == run.definition.revision
                    && definition.enabled
                    && definition.completion_policy == run.definition.completion_policy
            });
            if policy_current {
                let mut evaluated_run = run.clone();
                evaluated_run.output = completion_output;
                let evaluation = match self
                    .controller
                    .evaluate_automation_completion(&run.definition, &evaluated_run)
                    .await
                {
                    Ok(evaluation) => evaluation,
                    Err(error) => AutomationCompletionEvaluation {
                        stop_matched: false,
                        confidence: 0.0,
                        reason: format!("Stop check failed: {error}")
                            .chars()
                            .take(2000)
                            .collect(),
                        policy_applied: false,
                        failed: true,
                    },
                };
                // A stop-check failure is metadata, not a failed automation run.
                // The atomic recorder rechecks the exact policy revision before
                // it can disable anything.
                let _ = self
                    .controller
                    .workspace
                    .record_automation_completion_evaluation(run.id, evaluation)
                    .await;
            }
        }
        Ok(())
    }
}
impl Drop for AutomationScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}
