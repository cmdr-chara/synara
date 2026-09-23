//! A bounded DAG runner using existing Controller task/session slots.
//! Cancellation reaches only an owned child prompt, including during setup.
use super::*;
use crate::autonomy::workflow::{fingerprint, invalid};
use crate::autonomy::{MAX_WORKFLOW_OUTPUT, Workflow, WorkflowPhase, WorkflowStepState};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

impl Controller {
    pub async fn run_workflow(
        self: &Arc<Self>,
        parent: TaskId,
        workflow: Uuid,
        revision: u64,
    ) -> WorkspaceResult<Workflow> {
        self.run_workflow_interruptible(parent, workflow, revision, CancellationToken::new())
            .await
    }
    pub async fn run_workflow_interruptible(
        self: &Arc<Self>,
        parent: TaskId,
        workflow: Uuid,
        revision: u64,
        cancellation: CancellationToken,
    ) -> WorkspaceResult<Workflow> {
        if self.closing.load(Ordering::Acquire) || cancellation.is_cancelled() {
            return Err(AgentError::Cancelled.into());
        }
        let guard = self.autonomy.begin_interruptible(parent, cancellation)?;
        let run = guard.control.id;
        let mut value = self.workspace.change_workflow(parent, Some(revision), None, move |w| {
            if w.id != workflow || w.phase == WorkflowPhase::Completed || w.steps.iter().any(|s| s.state == WorkflowStepState::Running) {
                return Err(invalid("Stale graph, completed workflow or interrupted run. Review and recover explicitly"));
            }
            w.run = Some(run);
            w.phase = WorkflowPhase::Running;
            Ok(())
        }).await?;
        let mut failed = false;
        loop {
            if guard.control.cancel.is_cancelled() || self.closing.load(Ordering::Acquire) {
                break;
            }
            let ready = value.ready_indices();
            if ready.is_empty() {
                break;
            }
            let batch = guard.control.cancel.child_token();
            let mut workers = Vec::new();
            for index in ready.into_iter().take(value.spec.concurrency) {
                if batch.is_cancelled() {
                    break;
                }
                match self.workspace.claim_workflow_step(parent, run, index).await {
                    Ok(claimed) => {
                        let controller = self.clone();
                        let batch = batch.clone();
                        workers.push(tokio::spawn(async move {
                            let result = controller
                                .workflow_child(parent, run, index, claimed, batch.clone())
                                .await;
                            if result.is_err() {
                                batch.cancel();
                            }
                            result
                        }));
                    }
                    Err(_) => {
                        failed = true;
                        batch.cancel();
                        self.workspace.change_workflow(parent, None, Some(run), move |w| {
                            w.steps[index].state = WorkflowStepState::Failed;
                            w.steps[index].error = Some("Child configuration, draft or transcript changed. Review before retrying.".into());
                            Ok(())
                        }).await?;
                        break;
                    }
                }
            }
            // JoinHandles intentionally do not abort their cleanup supervisors
            // on Drop. RunGuard cancels their tokens and the existing session
            // owner completes cancellation before releasing task ownership.
            for worker in workers {
                if !matches!(worker.await, Ok(Ok(()))) {
                    failed = true;
                    batch.cancel();
                }
            }
            value = self
                .workspace
                .workflow(parent)
                .await?
                .ok_or(WorkspaceError::NotFound)?;
            if failed
                || value
                    .steps
                    .iter()
                    .any(|s| s.state == WorkflowStepState::Failed)
            {
                failed = true;
                break;
            }
        }
        let cancelled = guard.control.cancel.is_cancelled() || self.closing.load(Ordering::Acquire);
        let stopped = guard.control.stop.load(Ordering::Acquire);
        let result = self
            .workspace
            .change_workflow(parent, None, Some(run), move |w| {
                for step in &mut w.steps {
                    if step.state == WorkflowStepState::Running {
                        step.state = WorkflowStepState::Interrupted;
                        step.error = Some(
                            "Interrupted. Inspect child effects before explicitly retrying.".into(),
                        );
                    }
                }
                w.run = None;
                w.phase = if cancelled {
                    if stopped {
                        WorkflowPhase::Stopped
                    } else {
                        WorkflowPhase::Paused
                    }
                } else if !failed
                    && w.steps
                        .iter()
                        .all(|s| s.state == WorkflowStepState::Completed)
                {
                    WorkflowPhase::Completed
                } else {
                    WorkflowPhase::Failed
                };
                Ok(())
            })
            .await;
        drop(guard);
        result
    }
    async fn workflow_child(
        self: Arc<Self>,
        parent: TaskId,
        run: Uuid,
        index: usize,
        value: Workflow,
        batch: CancellationToken,
    ) -> WorkspaceResult<()> {
        let step = &value.steps[index];
        let id = step.task;
        let attempt = step.attempts;
        let prompt = value.prompt(index)?;
        let cancel = batch.child_token();
        let result = self
            .workflow_prompt(id, &value, index, prompt, cancel.clone())
            .await;
        let child = self.workspace.task(id).await?;
        let thread = self.workspace.thread(child.thread_id).await?;
        let sequence = thread.last_sequence;
        let usage = thread.usage.clone();
        let mut output = String::new();
        let mut error = None;
        let state = if cancel.is_cancelled() {
            error =
                Some("Interrupted. Side effects may already exist. Review before retrying.".into());
            WorkflowStepState::Interrupted
        } else if result.is_err() {
            error =
                Some("Agent step failed. Inspect its preserved transcript and permissions.".into());
            WorkflowStepState::Failed
        } else {
            // Only the just-completed turn may supply dependency output. An
            // earlier response must not masquerade as a successful retry result.
            output = thread
                .turns
                .last()
                .filter(|t| t.finished_at_ms.is_some() && !t.failed)
                .and_then(|turn| {
                    thread
                        .timeline
                        .get(turn.first_timeline_index..turn.end_timeline_index)
                })
                .and_then(|items| {
                    items.iter().rev().find_map(|item| match item {
                        TranscriptItem::Message { index } => thread
                            .messages
                            .get(*index)
                            .filter(|m| m.role == Role::Assistant)
                            .map(|m| m.text.clone()),
                        _ => None,
                    })
                })
                .unwrap_or_default();
            if output.len() > MAX_WORKFLOW_OUTPUT {
                output.clear();
                error = Some(
                    "Dependency output exceeds 32 KiB. No truncated result was forwarded.".into(),
                );
                WorkflowStepState::Failed
            } else {
                WorkflowStepState::Completed
            }
        };
        let failed = state == WorkflowStepState::Failed;
        self.workspace
            .change_workflow(parent, None, Some(run), move |w| {
                let step = &mut w.steps[index];
                if step.task != id
                    || step.attempts != attempt
                    || step.state != WorkflowStepState::Running
                {
                    return Err(invalid("Late child completion lost its workflow ownership"));
                }
                step.state = state;
                step.sequence = sequence;
                step.output = output;
                step.error = error;
                step.usage = Some(usage);
                Ok(())
            })
            .await?;
        if failed {
            batch.cancel();
        }
        Ok(())
    }
    async fn workflow_prompt(
        &self,
        id: TaskId,
        value: &Workflow,
        index: usize,
        text: String,
        cancellation: CancellationToken,
    ) -> WorkspaceResult<String> {
        if cancellation.is_cancelled() {
            return Err(AgentError::Cancelled.into());
        }
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _guard = PromptOwnership(slot.clone());
        *slot
            .setup_cancel
            .lock()
            .map_err(|_| WorkspaceError::Worker)? = Some(cancellation.clone());
        self.require_agent_route(id).await?;
        let task = self.workspace.task(id).await?;
        let spec = &value.spec.steps[index];
        let step = &value.steps[index];
        if cancellation.is_cancelled() || self.closing.load(Ordering::Acquire) {
            return Err(AgentError::Cancelled.into());
        }
        if task.agent_id != spec.agent_id
            || matches!(
                task.state,
                TaskState::Running | TaskState::Waiting | TaskState::Archived
            )
            || fingerprint(&self.profile(&task.agent_id).await?)? != step.profile_fingerprint
            || self.workspace.thread(task.thread_id).await?.last_sequence != step.sequence
            || self.workspace.task_draft(id).await? != spec.instruction
            || self.workspace.task(value.parent).await?.state == TaskState::Archived
        {
            return Err(invalid("Child changed before dispatch. Nothing was sent"));
        }
        let session = tokio::select! { biased;
            _ = cancellation.cancelled() => return Err(AgentError::Cancelled.into()),
            result = self.session_for(id) => result?,
        };
        if cancellation.is_cancelled() {
            return Err(AgentError::Cancelled.into());
        }
        let mut events = self.workspace.subscribe();
        let prompt = session.prompt(Prompt::text(text));
        tokio::pin!(prompt);
        loop {
            tokio::select! { biased;
                _ = cancellation.cancelled() => {
                    // Only this reserved session is cancelled. In particular,
                    // cancellation before acquiring a task cannot stop another
                    // user's independently running prompt in the same child.
                    let _ = session.cancel().await;
                    let _ = prompt.await;
                    return Err(AgentError::Cancelled.into());
                }
                result = &mut prompt => return result.map_err(Into::into),
                event = events.recv() => {
                    if let Ok(EventEnvelope { thread_id, event: ThreadEvent::UsageChanged { usage }, .. }) = event
                        && thread_id == task.thread_id
                    {
                        let owner = value.parent;
                        let run = value.run;
                        let attempt = step.attempts;
                        let saved = self.workspace.change_workflow(owner, None, run, move |w| {
                            let current = &mut w.steps[index];
                            if current.task != id || current.attempts != attempt || current.state != WorkflowStepState::Running { return Err(invalid("Usage lost its child owner")); }
                            current.usage = Some(usage);
                            Ok(())
                        }).await;
                        if let Err(error) = saved {
                            let _ = session.cancel().await;
                            let _ = prompt.await;
                            return Err(error);
                        }
                    }
                }
            }
        }
    }
    pub async fn pause_workflow(&self, parent: TaskId, stop: bool) -> WorkspaceResult<()> {
        if self.autonomy.interrupt(parent, stop) {
            return Ok(());
        }
        let value = self
            .workspace
            .workflow(parent)
            .await?
            .ok_or(WorkspaceError::NotFound)?;
        let expected = value.id;
        self.workspace
            .change_workflow(parent, Some(value.revision), None, move |w| {
                if w.id != expected {
                    return Err(invalid("Workflow changed"));
                }
                if w.phase != WorkflowPhase::Completed {
                    w.phase = if stop {
                        WorkflowPhase::Stopped
                    } else {
                        WorkflowPhase::Paused
                    };
                }
                Ok(())
            })
            .await?;
        Ok(())
    }
    pub async fn recover_workflow(
        &self,
        parent: TaskId,
        workflow: Uuid,
        revision: u64,
    ) -> WorkspaceResult<Workflow> {
        let guard = self.autonomy.begin(parent)?;
        let value = self
            .workspace
            .workflow(parent)
            .await?
            .ok_or(WorkspaceError::NotFound)?;
        let mut sequences = Vec::new();
        for step in &value.steps {
            if self.slot(step.task).await?.active.load(Ordering::Acquire) {
                return Err(AgentError::Busy.into());
            }
            let task = self.workspace.task(step.task).await?;
            let thread = self.workspace.thread(task.thread_id).await?;
            if matches!(thread.state, TaskState::Running | TaskState::Waiting) {
                return Err(invalid(
                    "An active child must stop before workflow recovery",
                ));
            }
            sequences.push(thread.last_sequence);
        }
        let result = self
            .workspace
            .change_workflow(parent, Some(revision), value.run, move |w| {
                if w.id != workflow {
                    return Err(invalid("Workflow identity changed"));
                }
                w.run = None;
                w.phase = WorkflowPhase::Paused;
                for (step, sequence) in w.steps.iter_mut().zip(sequences) {
                    if step.state == WorkflowStepState::Running {
                        step.state = WorkflowStepState::Interrupted;
                        step.sequence = sequence;
                        step.error = Some(
                            "Recovered without replay. Review side effects before retrying.".into(),
                        );
                    }
                }
                Ok(())
            })
            .await;
        drop(guard);
        result
    }
    pub async fn retry_workflow_step(
        &self,
        parent: TaskId,
        workflow: Uuid,
        revision: u64,
        index: usize,
    ) -> WorkspaceResult<Workflow> {
        if self.autonomy.workflow_running(parent) {
            return Err(AgentError::Busy.into());
        }
        let value = self
            .workspace
            .workflow(parent)
            .await?
            .ok_or(WorkspaceError::NotFound)?;
        let step = value
            .steps
            .get(index)
            .ok_or_else(|| invalid("Unknown workflow step"))?;
        if self.slot(step.task).await?.active.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        let task = self.workspace.task(step.task).await?;
        let sequence = self.workspace.thread(task.thread_id).await?.last_sequence;
        self.workspace.change_workflow(parent, Some(revision), None, move |w| {
            if w.id != workflow { return Err(invalid("Workflow identity changed")); }
            let step = &mut w.steps[index];
            if !matches!(step.state, WorkflowStepState::Failed | WorkflowStepState::Interrupted) || step.attempts >= 3 { return Err(invalid("Only a failed/interrupted step with fewer than three attempts can be retried")); }
            step.state = WorkflowStepState::Pending;
            step.sequence = sequence;
            step.output.clear();
            step.error = None;
            w.phase = WorkflowPhase::Paused;
            Ok(())
        }).await
    }
}
