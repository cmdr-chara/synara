//! One transient, selected-conversation pursuit. Persistence never restores a lease.
use super::*;
use crate::ui::{self, palette};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

struct Lease {
    epoch: u64,
    task: TaskId,
    thread: ThreadId,
    last_sequence: u64,
    remaining: u8,
    started: Instant,
    accounted: Instant,
    prepared: String,
    begun: bool,
    running: bool,
    done: bool,
    finish: Option<String>,
    response: String,
    due: Option<Instant>,
    token: CancellationToken,
}
impl Lease {
    fn elapsed(&self) -> u64 {
        if self.begun {
            self.started
                .elapsed()
                .as_millis()
                .min(GOAL_PURSUIT_LIMIT_MS as u128) as u64
        } else {
            0
        }
    }
}
pub(super) enum Reply {
    Loaded(TaskId, u64, Result<ThreadGoal, String>),
    Saved(TaskId, String, Result<ThreadGoal, String>),
    Preflight {
        task: TaskId,
        epoch: u64,
        sequence: u64,
        result: Result<(), String>,
    },
    Done {
        task: TaskId,
        epoch: u64,
        error: Option<String>,
    },
}
pub(super) struct GoalsState {
    task: Option<TaskId>,
    value: Option<ThreadGoal>,
    input: Entity<TextEntry>,
    evidence: Entity<TextEntry>,
    pub(super) open: bool,
    busy: bool,
    loading: bool,
    command_save_pending: bool,
    error: Option<String>,
    lease: Option<Lease>,
    epoch: u64,
    checking: bool,
    quitting: bool,
    deferred: Option<(u64, GoalEdit)>,
    _subscriptions: Vec<Subscription>,
}
impl GoalsState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let input = cx
            .new(|cx| TextEntry::new("Durable objective, up to 4 KiB", EntryMode::Editor, 60., cx));
        let evidence = cx.new(|cx| {
            TextEntry::new(
                "Your achievement verification evidence",
                EntryMode::SingleLine,
                30.,
                cx,
            )
        });
        let subscriptions = vec![
            cx.subscribe(&input, |this, _, event, cx| {
                if matches!(event, EntryEvent::Changed) && this.goals.lease.is_some() {
                    this.pause_goals("Goal editing pauses continuation.", false, cx);
                }
                cx.notify();
            }),
            cx.subscribe(&evidence, |_, _, _, cx| cx.notify()),
        ];
        Self {
            task: None,
            value: None,
            input,
            evidence,
            open: false,
            busy: false,
            loading: false,
            command_save_pending: false,
            error: None,
            lease: None,
            epoch: 0,
            checking: false,
            quitting: false,
            deferred: None,
            _subscriptions: subscriptions,
        }
    }
    fn pending_write(&self) -> bool {
        self.busy && !self.loading
    }
    fn dirty(&self, cx: &App) -> bool {
        self.open
            && (self.input.read(cx).is_composing()
                || self.evidence.read(cx).is_composing()
                || self
                    .value
                    .as_ref()
                    .is_some_and(|v| v.objective != self.input.read(cx).text())
                || !self.evidence.read(cx).text().is_empty())
    }
}

fn pause_command_error(
    selected_task: Option<TaskId>,
    current_task: Option<TaskId>,
    goal_task: Option<TaskId>,
    goal_loaded: bool,
    lease_task: Option<TaskId>,
    pending_edits_or_writes: bool,
) -> Option<&'static str> {
    let Some(selected_task) = selected_task else {
        return Some("Select a task before pausing a goal. The command was kept.");
    };
    if current_task != Some(selected_task) || goal_task != Some(selected_task) || !goal_loaded {
        return Some(
            "The goal for this task is still loading or unavailable. The command was kept.",
        );
    }
    if pending_edits_or_writes {
        return Some("Finish saving or discard goal edits before pausing. The command was kept.");
    }
    if lease_task != Some(selected_task) {
        return Some(
            "No active goal is running for this task. Review or resume it from the Goal panel; the command was kept.",
        );
    }
    None
}

impl Shell {
    pub(super) fn load_goals(&mut self, task: TaskId, cx: &mut Context<Self>) {
        self.goals.task = Some(task);
        self.goals.value = None;
        self.goals.busy = true;
        self.goals.loading = true;
        self.goals.command_save_pending = false;
        self.goals.epoch = self.goals.epoch.wrapping_add(1);
        let epoch = self.goals.epoch;
        self.goals.open = false;
        self.goals.error = None;
        self.goals.input.update(cx, |e, cx| e.clear(cx));
        self.goals.evidence.update(cx, |e, cx| e.clear(cx));
        let w = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Goals(Box::new(Reply::Loaded(
                task,
                epoch,
                w.thread_goal(task).await.map_err(|e| e.to_string()),
            ))))
        });
    }
    pub(super) fn open_goals(&mut self, cx: &mut Context<Self>) {
        if self.panel != Panel::Conversation {
            self.show_conversation(cx);
        }
        // Accordion: one workflow expanded at a time. Inputs persist in
        // their editors, so collapsing never discards edits.
        self.recap.open = false;
        self.goals.open = true;
        cx.notify();
    }
    pub(super) fn set_goal_from_command(
        &mut self,
        objective: String,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.goal_edit_command_ready(cx) {
            return false;
        }

        self.open_goals(cx);
        self.goals.error = None;
        self.goals.command_save_pending = true;
        self.notice = None;
        self.goals
            .input
            .update(cx, |entry, cx| entry.set_text(objective.clone(), cx));
        self.write_goal(0, GoalEdit::Set(objective), cx);
        true
    }
    fn goal_edit_command_ready(&mut self, cx: &mut Context<Self>) -> bool {
        let blocked = if self.goals.task != self.selected || self.goals.value.is_none() {
            Some("The goal for this task is still loading or unavailable. The command was kept.")
        } else if self.goals.loading || self.goals.busy || self.goals.deferred.is_some() {
            Some("Wait for the current goal operation to finish. The command was kept.")
        } else if self.goals.lease.is_some() {
            Some("Pause the active goal before replacing its objective. The command was kept.")
        } else if self.goals.dirty(cx) {
            Some(
                "Save or discard the open goal edits before replacing its objective. The command was kept.",
            )
        } else {
            None
        };
        if let Some(error) = blocked {
            self.error = Some(error.into());
            cx.notify();
            return false;
        }

        true
    }
    pub(super) fn edit_goal_from_command(
        &mut self,
        objective: Option<String>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.goal_edit_command_ready(cx) {
            return false;
        }
        self.open_goals(cx);
        if let Some(objective) = objective {
            self.goals
                .input
                .update(cx, |entry, cx| entry.set_text(objective, cx));
        }
        self.notice = Some(
            "Goal editor opened. Save is required to change the saved objective. Nothing was sent."
                .into(),
        );
        cx.notify();
        true
    }
    pub(super) fn clear_goal_from_command(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.goal_edit_command_ready(cx) {
            return false;
        }
        self.open_goals(cx);
        self.goals.error = None;
        self.notice = None;
        self.write_goal(0, GoalEdit::Clear, cx);
        true
    }
    pub(super) fn resume_goal_from_command(&mut self, cx: &mut Context<Self>) -> bool {
        self.open_goals(cx);
        // The exact command is replaced only on successful preparation. Never
        // quote the command itself into the provider prompt or discard it on error.
        self.resume_goals_with_draft(Some(""), cx)
    }
    fn write_goal(&mut self, delta: u64, edit: GoalEdit, cx: &mut Context<Self>) {
        if self.goals.busy {
            self.goals.deferred = Some((delta, edit));
            return;
        }
        let (Some(task), Some(value)) = (self.goals.task, self.goals.value.as_ref()) else {
            return;
        };
        let revision = value.revision;
        let before = self.goals.input.read(cx).text().to_owned();
        self.goals.busy = true;
        let w = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Goals(Box::new(Reply::Saved(
                task,
                before,
                w.edit_thread_goal(task, revision, delta, edit)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    pub(super) fn pause_goals(&mut self, why: &str, blocked: bool, cx: &mut Context<Self>) {
        let Some(lease) = self.goals.lease.take() else {
            return;
        };
        lease.token.cancel();
        self.goals.checking = false;
        let delta = if lease.begun {
            lease
                .accounted
                .elapsed()
                .as_millis()
                .min(GOAL_PURSUIT_LIMIT_MS as u128) as u64
        } else {
            0
        };
        self.write_goal(
            delta,
            if blocked {
                GoalEdit::Block(why.into())
            } else {
                GoalEdit::Pause(why.into())
            },
            cx,
        );
    }
    pub(super) fn goal_pause_command_error(&self, cx: &App) -> Option<&'static str> {
        pause_command_error(
            self.selected,
            self.task().map(|task| task.id),
            self.goals.task,
            !self.goals.loading && self.goals.value.is_some(),
            self.goals.lease.as_ref().map(|lease| lease.task),
            self.goals.busy || self.goals.deferred.is_some() || self.goals.dirty(cx),
        )
    }
    pub(super) fn goal_send_pending(&self, cx: &App) -> bool {
        self.goals.busy || self.goals.deferred.is_some() || self.goals.dirty(cx)
    }
    pub(super) fn goal_send_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.goal_send_pending(cx) {
            self.notice = Some(
                "Save or discard goal edits and let the pending save finish before sending.".into(),
            );
            cx.notify();
            true
        } else {
            false
        }
    }
    pub(super) fn goal_navigation_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.goals.lease.is_some() {
            self.pause_goals(
                "Navigation paused this pursuit. Resume explicitly in its conversation.",
                false,
                cx,
            );
        }
        // Read-only metadata restoration must not strand the panel transition
        // that selected this task. Real writes and unsaved edits still fence it.
        if self.goals.pending_write() || self.goals.deferred.is_some() || self.goals.dirty(cx) {
            self.notice=Some("The goal is paused. Save/discard goal edits and let its pending save finish before leaving.".into());
            cx.notify();
            true
        } else {
            false
        }
    }
    pub(super) fn goal_close_edits_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.goals.dirty(cx) {
            self.notice = Some("Save or discard the goal editor before closing.".into());
            cx.notify();
            true
        } else {
            false
        }
    }
    pub(super) fn goal_before_quit(&mut self, cx: &mut Context<Self>) -> bool {
        self.pause_goals(
            "Closed by you. Restart restores this goal without execution.",
            false,
            cx,
        );
        if self.goals.pending_write() || self.goals.deferred.is_some() {
            self.goals.quitting = true;
            cx.notify();
            true
        } else {
            false
        }
    }
    fn goal_resume_ready(&self, cx: &App) -> bool {
        if self.goals.busy
            || self.goals.deferred.is_some()
            || self.goals.loading
            || self.goals.checking
            || self.goals.lease.is_some()
            || self.goals.dirty(cx)
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.composer.read(cx).is_composing()
        {
            return false;
        }
        let (Some(task), Some(thread), Some(_)) =
            (self.task(), self.thread.as_ref(), self.goals.value.as_ref())
        else {
            return false;
        };
        Some(task.id) == self.goals.task
            && task.thread_id == thread.id
            && !self.busy.contains(&task.id)
            && !self.connecting.contains(&task.id)
            && !self.controls.is_pending(task.id)
            && !self.attachments_have_pending()
            && !self.attachment_send_blocked()
            && !self.pending.keys().any(|(t, _)| *t == thread.id)
            && !matches!(
                task.state,
                TaskState::Archived | TaskState::Running | TaskState::Waiting
            )
    }
    fn resume_goals(&mut self, cx: &mut Context<Self>) {
        self.resume_goals_with_draft(None, cx);
    }
    fn resume_goals_with_draft(&mut self, draft: Option<&str>, cx: &mut Context<Self>) -> bool {
        if !self.goal_resume_ready(cx) {
            self.goals.error = Some(
                "Finish the current turn, approvals and attachment edits before resuming a goal."
                    .into(),
            );
            cx.notify();
            return false;
        }
        let (Some(task), Some(thread), Some(value)) =
            (self.task(), self.thread.as_ref(), self.goals.value.as_ref())
        else {
            return false;
        };
        let accepted = match value.prepare(
            task.id,
            draft.unwrap_or_else(|| self.composer.read(cx).text()),
        ) {
            Ok(text) => {
                let task = task.id;
                let thread = thread.id;
                let sequence = self.thread.as_ref().unwrap().last_sequence;
                self.composer
                    .update(cx, |e, cx| e.set_text(text.clone(), cx));
                self.remember_draft(cx);
                self.goals.epoch = self.goals.epoch.wrapping_add(1);
                self.goals.lease = Some(Lease {
                    epoch: self.goals.epoch,
                    task,
                    thread,
                    last_sequence: sequence,
                    remaining: GOAL_MAX_FOLLOWUPS,
                    started: Instant::now(),
                    accounted: Instant::now(),
                    prepared: text,
                    begun: false,
                    running: false,
                    done: false,
                    finish: None,
                    response: String::new(),
                    due: None,
                    token: CancellationToken::new(),
                });
                self.goals.error = None;
                self.notice=Some("Review the goal request and Send. This resume allows at most two visible, delayed follow-ups. Editing, Pause or navigation disarms it.".into());
                self.focus_composer = true;
                true
            }
            Err(e) => {
                self.goals.error = Some(e.to_string());
                false
            }
        };
        cx.notify();
        accepted
    }
    pub(super) fn goal_input_changed(&mut self, cx: &mut Context<Self>) {
        if self.goals.lease.as_ref().is_some_and(|l| {
            if !l.running && l.due.is_none() && !self.goals.checking {
                self.composer.read(cx).text() != l.prepared
            } else {
                !self.composer.read(cx).text().is_empty()
            }
        }) {
            self.pause_goals(
                "User input takes priority. Resume explicitly after reviewing the new message.",
                false,
                cx,
            );
        }
    }
    pub(super) fn goal_manual_send(&mut self, task: TaskId, text: &str, cx: &mut Context<Self>) {
        if let Some(l) = self.goals.lease.as_mut() {
            if l.task == task
                && !l.running
                && l.due.is_none()
                && !self.goals.checking
                && text == l.prepared
            {
                l.begun = true;
                l.running = true;
                l.done = false;
                l.started = Instant::now();
                l.accounted = l.started;
                l.finish = None;
                l.response.clear();
                l.last_sequence = self.thread.as_ref().map_or(0, |t| t.last_sequence);
            } else {
                self.pause_goals(
                    "A new manual message takes priority over continuation.",
                    false,
                    cx,
                );
            }
        }
    }
    pub(super) fn goal_event(&mut self, e: &EventEnvelope, cx: &mut Context<Self>) {
        let Some(l) = self
            .goals
            .lease
            .as_mut()
            .filter(|l| l.thread == e.thread_id)
        else {
            return;
        };
        if e.sequence <= l.last_sequence {
            return;
        }
        if e.sequence != l.last_sequence + 1 {
            self.pause_goals("A conversation event gap requires manual review.", true, cx);
            return;
        }
        l.last_sequence = e.sequence;
        match &e.event {
            ThreadEvent::HistoryStarted|ThreadEvent::PermissionRequested {..}|ThreadEvent::UserInputRequested {..}|ThreadEvent::CancellationRequested|ThreadEvent::Error {..}=>self.pause_goals("Approval, question, cancellation or failure stopped continuation. Resolve it and resume explicitly.",true,cx),
            ThreadEvent::ToolChanged {patch} if matches!(patch.status,Some(ToolStatus::Failed|ToolStatus::Cancelled))=>self.pause_goals("A tool failed or was cancelled. No implicit retry is allowed.",true,cx),
            ThreadEvent::TextDelta {role:Role::Assistant,text,..} if l.running=>{if l.response.len().saturating_add(text.len())>256*1024 {self.pause_goals("The response exceeded the goal evaluator bound.",true,cx);}else{l.response.push_str(text);}},
            ThreadEvent::PromptFinished {reason} if l.running=>l.finish=Some(reason.clone()),
            ThreadEvent::PromptStarted {..} if !l.running=>self.pause_goals("Another prompt started. Goal continuation was disarmed.",true,cx),
            _=>{}
        }
    }
    pub(super) fn goal_prompt_done(
        &mut self,
        task: TaskId,
        error: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        if self
            .goals
            .lease
            .as_ref()
            .is_none_or(|l| l.task != task || !l.running)
        {
            return;
        }
        if error.is_some() {
            self.pause_goals(
                "The prompt failed. It will not be retried automatically.",
                true,
                cx,
            );
        } else if let Some(l) = self.goals.lease.as_mut() {
            l.done = true;
        }
    }
    pub(super) fn tick_goals(&mut self, cx: &mut Context<Self>) {
        let Some(l) = self.goals.lease.as_ref() else {
            return;
        };
        if self.selected != Some(l.task)
            || self.panel != Panel::Conversation
            || self.close != CloseState::Open
            || self.task().is_none_or(|t| t.state == TaskState::Archived)
        {
            self.pause_goals("The task is no longer available for pursuit.", true, cx);
            return;
        }
        if l.elapsed() >= GOAL_PURSUIT_LIMIT_MS {
            self.pause_goals(
                "The ten-minute pursuit limit was reached. No further prompt will start.",
                true,
                cx,
            );
            return;
        }
        if self.attachments_have_pending() || self.pending.keys().any(|(t, _)| *t == l.thread) {
            self.pause_goals(
                "Pending input or attachments require explicit user review.",
                true,
                cx,
            );
            return;
        }
        if self.goals.busy || self.goals.deferred.is_some() {
            return;
        }
        let l = self.goals.lease.as_mut().unwrap();
        if l.running && l.done && l.finish.is_some() {
            let decision = goal_decision(
                &l.response,
                l.finish.as_deref().unwrap(),
                l.remaining,
                l.elapsed(),
            );
            match decision {
                GoalDecision::Continue => {
                    let delta = l
                        .accounted
                        .elapsed()
                        .as_millis()
                        .min(GOAL_PURSUIT_LIMIT_MS as u128) as u64;
                    l.accounted = Instant::now();
                    l.running = false;
                    l.due = Some(Instant::now() + Duration::from_secs(5));
                    self.write_goal(
                        delta,
                        GoalEdit::Pause(
                            "Follow-up preview is visible. Restart remains paused.".into(),
                        ),
                        cx,
                    );
                }
                GoalDecision::Blocked(note) => self.pause_goals(&note, true, cx),
                GoalDecision::Review(note) => {
                    let l = self.goals.lease.take().unwrap();
                    let delta = l
                        .accounted
                        .elapsed()
                        .as_millis()
                        .min(GOAL_PURSUIT_LIMIT_MS as u128) as u64;
                    l.token.cancel();
                    self.write_goal(delta, GoalEdit::Review(note), cx);
                }
            }
            cx.notify();
            return;
        }
        if l.due.is_some_and(|due| Instant::now() >= due) && !self.goals.checking {
            let task = l.task;
            let epoch = l.epoch;
            let sequence = l.last_sequence;
            let revision = self.goals.value.as_ref().unwrap().revision;
            let thread = l.thread;
            if !self.composer.read(cx).text().is_empty() || self.composer.read(cx).is_composing() {
                self.pause_goals("New user input takes priority.", false, cx);
                return;
            }
            if self.draft_state.pending_for(task)
                || self.busy.contains(&task)
                || self.connecting.contains(&task)
                || self.controls.is_pending(task)
                || self.attachment_send_blocked()
            {
                return;
            }
            self.goals.checking = true;
            let w = self.controller.workspace.clone();
            self.job(async move {
                let result = async {
                    let owner = w.task(task).await?;
                    if owner.thread_id != thread
                        || matches!(
                            owner.state,
                            TaskState::Archived | TaskState::Running | TaskState::Waiting
                        )
                        || w.thread_goal(task).await?.revision != revision
                        || w.thread(thread).await?.last_sequence != sequence
                        || !w.task_draft(task).await?.is_empty()
                        || !w.attachment_draft(task).await?.pending.is_empty()
                    {
                        return Err(WorkspaceError::Invalid(
                            "Goal, task, draft or transcript changed during continuation review."
                                .into(),
                        ));
                    }
                    Ok(())
                }
                .await;
                Ok(Update::Goals(Box::new(Reply::Preflight {
                    task,
                    epoch,
                    sequence,
                    result: result.map_err(|e: WorkspaceError| e.to_string()),
                })))
            });
        }
        cx.notify();
    }
    pub(super) fn goals_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Loaded(task, epoch, result) => {
                if self.goals.task != Some(task) || self.goals.epoch != epoch {
                    return;
                }
                self.goals.busy = false;
                self.goals.loading = false;
                match result {
                    Ok(value) => {
                        self.goals
                            .input
                            .update(cx, |e, cx| e.set_text(value.objective.clone(), cx));
                        self.goals.value = Some(value);
                    }
                    Err(e) => self.goals.error = Some(e),
                }
            }
            Reply::Saved(task, before, result) => {
                if self.goals.task != Some(task) {
                    return;
                }
                self.goals.busy = false;
                let command_save = std::mem::take(&mut self.goals.command_save_pending);
                match result {
                    Ok(value) => {
                        if self.goals.input.read(cx).text() == before {
                            self.goals
                                .input
                                .update(cx, |e, cx| e.set_text(value.objective.clone(), cx));
                        }
                        self.goals.value = Some(value);
                        self.goals.error = None;
                        if command_save {
                            self.notice = Some(
                                "Goal saved paused. Explicit resume and Send are required to begin.".into(),
                            );
                        }
                    }
                    Err(e) => {
                        if let Some(l) = self.goals.lease.take() {
                            l.token.cancel();
                        }
                        self.goals.deferred = None;
                        self.goals.quitting = false;
                        self.close.cancel();
                        self.goals.error = Some(e);
                    }
                }
                if let Some((delta, edit)) = self.goals.deferred.take() {
                    self.write_goal(delta, edit, cx);
                }
                if self.goals.quitting && !self.goals.busy {
                    self.goals.quitting = false;
                    self.begin_quit(cx);
                }
            }
            Reply::Preflight {
                task,
                epoch,
                sequence,
                result,
            } => {
                if self
                    .goals
                    .lease
                    .as_ref()
                    .is_none_or(|l| l.task != task || l.epoch != epoch)
                {
                    return;
                }
                self.goals.checking = false;
                if let Err(e) = result {
                    self.pause_goals(&e, true, cx);
                    return;
                }
                let l = self.goals.lease.as_ref().unwrap();
                if self.selected != Some(task)
                    || l.last_sequence != sequence
                    || l.remaining == 0
                    || l.elapsed() >= GOAL_PURSUIT_LIMIT_MS
                    || self.goals.busy
                    || self.close != CloseState::Open
                    || !self.composer.read(cx).text().is_empty()
                    || self.composer.read(cx).is_composing()
                    || self.attachments_have_pending()
                    || self.attachment_send_blocked()
                    || self.busy.contains(&task)
                    || self.pending.keys().any(|(t, _)| *t == l.thread)
                {
                    self.pause_goals(
                        "Continuation review became stale. Nothing was sent.",
                        true,
                        cx,
                    );
                    return;
                }
                let prompt = match self
                    .goals
                    .value
                    .as_ref()
                    .unwrap()
                    .followup(task, GOAL_MAX_FOLLOWUPS - l.remaining + 1)
                {
                    Ok(p) => p,
                    Err(e) => {
                        self.pause_goals(&e.to_string(), true, cx);
                        return;
                    }
                };
                let l = self.goals.lease.as_mut().unwrap();
                l.remaining -= 1;
                l.running = true;
                l.done = false;
                l.finish = None;
                l.response.clear();
                l.due = None;
                l.token = CancellationToken::new();
                let token = l.token.clone();
                self.busy.insert(task);
                let c = self.controller.clone();
                self.job(async move {
                    let error = c
                        .submit_interruptible(task, prompt, token)
                        .await
                        .err()
                        .map(|e| e.to_string());
                    Ok(Update::Goals(Box::new(Reply::Done { task, epoch, error })))
                });
            }
            Reply::Done { task, epoch, error } => {
                self.busy.remove(&task);
                if self
                    .goals
                    .lease
                    .as_ref()
                    .is_some_and(|l| l.epoch == epoch && l.task == task)
                {
                    self.goal_prompt_done(task, error.as_deref(), cx);
                }
                if let Some(e) = error {
                    self.notice = Some(format!("Goal follow-up ended without retry: {e}"));
                }
                self.hydrate();
            }
        }
        cx.notify();
    }
    /// Collapsed idle goal for the shared one-row workflow strip.
    /// Probe-compatible with the full bar header it replaces.
    pub(super) fn goal_compact(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let g = &self.goals;
        if self.selected.is_none() || self.selected != g.task {
            return None;
        }
        if self
            .thread
            .as_ref()
            .is_some_and(|t| t.timeline.is_empty() && t.plan.is_empty())
        {
            return None;
        }
        if g.open || g.lease.is_some() {
            return None;
        }
        let state: String = g.value.as_ref().map_or("Loading".into(), |v| {
            if v.objective.is_empty() {
                "No goal".into()
            } else {
                v.status.label().into()
            }
        });
        Some(
            ui::header_action(
                "goal-open",
                format!("Goal: {state}"),
                None,
                false,
                cx.listener(|this, _: &(), _, cx| this.open_goals(cx)),
            )
            .relative()
            .child(ui::layout_probe("goal-open"))
            .into_any_element(),
        )
    }
    pub(super) fn goal_bar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let g = &self.goals;
        if self.selected.is_none() || self.selected != g.task {
            return div().into_any_element();
        }
        // Collapsed idle goals live in the shared workflow strip
        // (goal_compact); the palette offers them on empty threads.
        if !g.open && g.lease.is_none() {
            return div().into_any_element();
        }
        let state = if let Some(l) = &g.lease {
            if l.running {
                format!("Pursuing, {} follow-ups left", l.remaining)
            } else if let Some(due) = l.due {
                format!(
                    "Follow-up in {}s, Pause to cancel",
                    due.saturating_duration_since(Instant::now()).as_secs()
                )
            } else {
                "Armed for your explicit Send".into()
            }
        } else {
            g.value.as_ref().map_or("Loading".into(), |v| {
                if v.objective.is_empty() {
                    "No goal".into()
                } else {
                    v.status.label().into()
                }
            })
        };
        let mut root = div()
            .id("goal-workflow")
            .max_h(px(340.))
            .overflow_y_scroll()
            .px_4()
            .py_1()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                ui::action(
                    "goal-open",
                    format!("Goal: {state}"),
                    None,
                    g.open,
                    cx.listener(|this, _: &(), _, cx| this.open_goals(cx)),
                )
                .relative()
                .child(ui::layout_probe("goal-open")),
            );
        // Countdown and Pause remain visible even when the editor is collapsed.
        if g.lease.is_some() {
            root = root.child(
                ui::action(
                    "goal-pause",
                    "Pause goal",
                    None,
                    false,
                    cx.listener(|this, _: &(), _, cx| {
                        this.pause_goals(
                            "Paused by you. No future continuation is armed.",
                            false,
                            cx,
                        )
                    }),
                )
                .relative()
                .child(ui::layout_probe("goal-pause")),
            );
        }
        if g.pending_write() {
            root = root.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette().muted))
                    .child("Saving goal…"),
            );
        }
        if !g.open {
            return root.into_any_element();
        }
        root = root.child(
            div()
                .h(px(60.))
                .flex_shrink_0()
                .relative()
                .child(ui::layout_probe("goal-input"))
                .child(g.input.clone()),
        );
        let disabled = g.busy || g.lease.is_some();
        root = root.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    ui::icon_button(
                        "goal-save",
                        "Save goal",
                        ui::Glyph::Notebook,
                        disabled,
                        cx.listener(|this, _: &(), _, cx| {
                            let text = this.goals.input.read(cx).text().to_owned();
                            if !this.goals.input.read(cx).is_composing() {
                                this.write_goal(0, GoalEdit::Set(text), cx);
                            }
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("goal-save")),
                )
                .child(
                    ui::icon_button(
                        "goal-resume",
                        "Prepare and resume (2 follow-ups)",
                        ui::Glyph::Send,
                        !self.goal_resume_ready(cx),
                        cx.listener(|this, _: &(), _, cx| this.resume_goals(cx)),
                    )
                    .relative()
                    .child(ui::layout_probe_enabled(
                        "goal-resume",
                        self.goal_resume_ready(cx),
                    )),
                )
                .child(ui::action(
                    "goal-discard",
                    "Discard edits / reload",
                    None,
                    g.busy,
                    cx.listener(|this, _: &(), _, cx| {
                        if !this.goals.busy
                            && this.goals.lease.is_none()
                            && let Some(t) = this.selected
                        {
                            this.load_goals(t, cx);
                            this.goals.open = true;
                        }
                    }),
                ))
                .child(
                    ui::action(
                        "goal-clear",
                        "Clear goal and history",
                        None,
                        disabled,
                        cx.listener(|this, _: &(), _, cx| {
                            if !this.goals.busy && this.goals.lease.is_none() {
                                this.goals.evidence.update(cx, |e, cx| e.clear(cx));
                                this.write_goal(0, GoalEdit::Clear, cx);
                            }
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("goal-clear")),
                ),
        );
        if let Some(v) = &g.value {
            let elapsed = v.elapsed_ms
                + g.lease.as_ref().filter(|l| l.begun).map_or(0, |l| {
                    l.accounted
                        .elapsed()
                        .as_millis()
                        .min(GOAL_PURSUIT_LIMIT_MS as u128) as u64
                });
            root = root.child(div().text_xs().child(format!(
                "Pursuit time: {}s. {}",
                elapsed / 1000,
                v.note
            )));
            if g.lease
                .as_ref()
                .is_some_and(|l| l.due.is_some() || g.checking)
            {
                root=root.child(div().text_xs().text_color(rgb(palette().muted)).child(format!("Next prompt preview: continue only this objective: {}. Do not repeat uncertain external writes. No approval is inherited.",v.objective)));
            }
            root = root
                .child(
                    div()
                        .h(px(30.))
                        .flex_shrink_0()
                        .relative()
                        .child(ui::layout_probe("goal-evidence"))
                        .child(g.evidence.clone()),
                )
                .child(
                    ui::icon_button(
                        "goal-achieve",
                        "Mark achieved with my evidence",
                        ui::Glyph::Check,
                        disabled,
                        cx.listener(|this, _: &(), _, cx| {
                            let evidence = this.goals.evidence.read(cx).text().to_owned();
                            if !evidence.trim().is_empty()
                                && !this.goals.evidence.read(cx).is_composing()
                            {
                                this.write_goal(0, GoalEdit::Achieve(evidence), cx);
                                this.goals.evidence.update(cx, |e, cx| e.clear(cx));
                            }
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("goal-achieve")),
                );
            let mut history = div()
                .id("goal-achievement-history")
                .max_h(px(80.))
                .overflow_y_scroll()
                .flex()
                .flex_col();
            for a in v.achievements.iter().rev() {
                history = history.child(
                    div()
                        .text_xs()
                        .child(format!("Achieved: {} | {}", a.objective, a.evidence)),
                );
            }
            root = root.child(history);
        }
        if let Some(e) = &g.error {
            root = root.child(div().text_xs().child(e.clone()));
        }
        root.child(
            ui::action(
                "goal-close",
                "Hide goal details",
                None,
                false,
                cx.listener(|this, _: &(), _, cx| {
                    if !this.goals.dirty(cx) {
                        this.goals.open = false;
                    }
                    cx.notify();
                }),
            )
            .relative()
            .child(ui::layout_probe("goal-close")),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod pause_command_tests {
    use super::*;

    #[test]
    fn pause_command_requires_loaded_goal_and_matching_active_lease() {
        let task = TaskId::new();
        let other = TaskId::new();
        assert!(pause_command_error(None, None, None, false, None, false).is_some());
        assert!(
            pause_command_error(Some(task), Some(other), Some(task), true, Some(task), false)
                .is_some()
        );
        assert!(
            pause_command_error(Some(task), Some(task), Some(other), true, Some(task), false)
                .is_some()
        );
        assert!(
            pause_command_error(Some(task), Some(task), Some(task), false, Some(task), false)
                .is_some()
        );
        assert!(
            pause_command_error(Some(task), Some(task), Some(task), true, Some(task), true)
                .is_some()
        );
        assert_eq!(
            pause_command_error(Some(task), Some(task), Some(task), true, None, false),
            Some(
                "No active goal is running for this task. Review or resume it from the Goal panel; the command was kept."
            )
        );
        assert_eq!(
            pause_command_error(Some(task), Some(task), Some(task), true, Some(task), false),
            None
        );
    }
}
