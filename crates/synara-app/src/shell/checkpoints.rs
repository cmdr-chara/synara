//! Bounded, explicit rollback of app-owned draft and saved notes/checklist only.
//! The Controller reserves the existing task. A single SQLite transaction owns
//! recovery + restoration. The view never rewinds a provider or workspace files.
use super::*;
use crate::ui::{self, palette};

const BOUNDARY: &str = "Draft + saved notes/checklist only. Files, Git, transcript, attachments and provider sessions are NOT restored.";
#[derive(Default)]
pub(super) struct CheckpointState {
    task: Option<TaskId>,
    epoch: u64,
    open: bool,
    loading: bool,
    writing: bool,
    history: Option<TaskCheckpoints>,
    review: Option<CheckpointReview>,
    error: Option<String>,
}
impl CheckpointState {
    pub fn writing(&self) -> bool {
        self.writing
    }
}
pub(super) enum Outcome {
    Loaded(TaskCheckpoints),
    Reviewed(CheckpointReview),
    Restored(CheckpointRestored),
}
pub(super) struct Reply {
    task: TaskId,
    epoch: u64,
    result: Result<Outcome, String>,
}
impl Shell {
    pub(super) fn checkpoint_navigation_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.checkpoints.writing {
            return false;
        }
        self.notice = Some("Completing the atomic checkpoint write before changing tasks.".into());
        cx.notify();
        true
    }
    pub(super) fn checkpoint_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        ui::action(
            "checkpoint-open",
            "Checkpoints",
            None,
            self.selected.is_none() || self.loading_task.is_some() || self.checkpoints.writing,
            cx.listener(|this, _: &(), _, cx| this.open_checkpoints(cx)),
        )
        .relative()
        .child(ui::layout_probe("checkpoint-open"))
        .into_any_element()
    }
    fn open_checkpoints(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.selected else {
            return;
        };
        if self.loading_task.is_some() || self.checkpoints.writing || self.close != CloseState::Open
        {
            return;
        }
        // Opening is read-only. An epoch fences every asynchronous response.
        self.checkpoints.task = Some(task);
        self.checkpoints.epoch = self.checkpoints.epoch.wrapping_add(1);
        self.checkpoints.open = true;
        self.checkpoints.loading = true;
        self.checkpoints.history = None;
        self.checkpoints.review = None;
        self.checkpoints.error = None;
        let epoch = self.checkpoints.epoch;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Checkpoints(Box::new(Reply {
                task,
                epoch,
                result: workspace
                    .task_checkpoints(task)
                    .await
                    .map(Outcome::Loaded)
                    .map_err(|e| e.to_string()),
            })))
        });
        cx.notify();
    }
    fn checkpoint_ready(&self, cx: &App) -> bool {
        let Some(task) = self.task() else {
            return false;
        };
        self.checkpoints.task == Some(task.id)
            && self.checkpoints.open
            && !self.checkpoints.loading
            && !self.checkpoints.writing
            && self.checkpoints.history.is_some()
            && !self.busy.contains(&task.id)
            && !self.connecting.contains(&task.id)
            && !matches!(
                task.state,
                TaskState::Running | TaskState::Waiting | TaskState::Archived
            )
            && self.loading_task.is_none()
            && !self.creating_task
            && self.close == CloseState::Open
            && !self.draft_state.pending_for(task.id)
            && !self.composer.read(cx).is_composing()
            && self.saved_context.dialog.is_none()
            && self.organization.dialog.is_none()
            && self.kanban.dialog.is_none()
            && !self.revisions.open()
            && !self.handoff.open()
            && !self.recap.pending()
            && !self.followups.pending(cx)
            && !self.debug_workflow.pending(cx)
            && !self.hubs.pending(cx)
            && !self.controls.is_pending(task.id)
            && !self.goal_send_pending(cx)
            && !self
                .pending
                .keys()
                .any(|(thread, _)| *thread == task.thread_id)
    }
    fn checkpoint_start(
        &mut self,
        writing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(TaskId, u64)> {
        if !self.checkpoint_ready(cx) {
            return None;
        }
        // An armed goal is explicitly disarmed, never resumed by restoring text.
        if self.goal_navigation_blocked(cx) || self.inline_navigation_blocked(cx) {
            return None;
        }
        let task = self.selected?;
        self.checkpoints.error = None;
        self.checkpoints.loading = !writing;
        self.checkpoints.writing = writing;
        if writing {
            self.focus_composer = false;
            window.focus(&self.close_focus, cx);
        }
        cx.notify();
        Some((task, self.checkpoints.epoch))
    }
    fn capture_checkpoint(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.checkpoints.review.is_some() {
            return;
        }
        let Some((task, epoch)) = self.checkpoint_start(true, window, cx) else {
            return;
        };
        let draft = self.composer.read(cx).text().to_owned();
        let controller = self.controller.clone();
        self.job(async move {
            Ok(Update::Checkpoints(Box::new(Reply {
                task,
                epoch,
                result: controller
                    .capture_task_checkpoint(task, draft)
                    .await
                    .map(Outcome::Loaded)
                    .map_err(|e| e.to_string()),
            })))
        });
    }
    fn review_checkpoint(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some((task, epoch)) = self.checkpoint_start(false, window, cx) else {
            return;
        };
        self.checkpoints.review = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Checkpoints(Box::new(Reply {
                task,
                epoch,
                result: workspace
                    .review_task_checkpoint(task, id)
                    .await
                    .map(Outcome::Reviewed)
                    .map_err(|e| e.to_string()),
            })))
        });
    }
    fn restore_checkpoint(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = self.checkpoints.review.clone() else {
            return;
        };
        if self.selected != Some(review.task())
            || self.composer.read(cx).text() != review.current_draft()
        {
            self.checkpoints.review = None;
            self.checkpoints.error = Some(
                "Draft changed after review. Review the checkpoint again. Nothing was restored."
                    .into(),
            );
            cx.notify();
            return;
        }
        let Some((task, epoch)) = self.checkpoint_start(true, window, cx) else {
            return;
        };
        let controller = self.controller.clone();
        self.job(async move {
            Ok(Update::Checkpoints(Box::new(Reply {
                task,
                epoch,
                result: controller
                    .restore_task_checkpoint(review)
                    .await
                    .map(Outcome::Restored)
                    .map_err(|e| e.to_string()),
            })))
        });
    }
    pub(super) fn checkpoint_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if self.checkpoints.task != Some(reply.task) || self.checkpoints.epoch != reply.epoch {
            return;
        }
        self.checkpoints.loading = false;
        self.checkpoints.writing = false;
        if self.selected != Some(reply.task) {
            self.checkpoints.open = false;
            self.checkpoints.review = None;
            return;
        }
        match reply.result {
            Ok(Outcome::Loaded(history)) => {
                self.checkpoints.history = Some(history);
                self.checkpoints.review = None;
            }
            Ok(Outcome::Reviewed(review)) => self.checkpoints.review = Some(review),
            Ok(Outcome::Restored(value)) => {
                // The UI was quiescent during the atomic write. Do not enqueue a
                // second draft save or restore a prompt/session/approval lease.
                self.draft_state.forget_task(reply.task);
                self.drafts.insert(reply.task, value.draft.clone());
                self.composer
                    .update(cx, |e, cx| e.set_text(value.draft, cx));
                self.checkpoints.history = Some(value.history);
                self.checkpoints.review = None;
                self.notice = Some("Draft and saved notes/checklist restored. Pre-revert recovery saved. Nothing sent, no files or sessions changed.".into());
            }
            Err(error) => {
                self.checkpoints.error = Some(error);
                self.checkpoints.review = None;
            }
        }
        cx.notify();
    }
    pub(super) fn checkpoint_busy_panel(&self, _cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("checkpoint-writing")
            .role(gpui::Role::Dialog)
            .aria_label("Atomic checkpoint write in progress")
            .track_focus(&self.close_focus)
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .bg(rgb(palette().canvas))
            .text_color(rgb(palette().text))
            .child("Saving the checkpoint transaction...")
            .child(BOUNDARY)
            .on_key_down(|_: &gpui::KeyDownEvent, _, cx| cx.stop_propagation())
            .into_any_element()
    }
    pub(super) fn checkpoint_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self.checkpoints.open || self.checkpoints.task != self.selected {
            return div().into_any_element();
        }
        let state = &self.checkpoints;
        let disabled = !self.checkpoint_ready(cx);
        let mut panel = div().id("checkpoint-panel").flex_shrink_0().max_h(px(310.))
            .overflow_y_scroll().px_4().py_2().flex().flex_col().gap_2()
            .border_b_1().border_color(rgb(palette().border)).text_size(px(12.))
            .child(div().text_color(rgb(palette().muted)).child(BOUNDARY))
            .child(div().flex().gap_2().items_center()
                .child(ui::action("checkpoint-capture", "Save checkpoint", None, disabled || state.review.is_some(),
                    cx.listener(|this, _: &(), window, cx| this.capture_checkpoint(window, cx)))
                    .relative().child(ui::layout_probe_enabled("checkpoint-capture", !disabled && state.review.is_none())))
                .child(ui::action("checkpoint-refresh", "Reload", None, state.loading || state.writing,
                    cx.listener(|this, _: &(), _, cx| this.open_checkpoints(cx))))
                .child(ui::action("checkpoint-close", "Close", None, state.writing,
                    cx.listener(|this, _: &(), _, cx| { if !this.checkpoints.writing {
                        this.checkpoints.epoch = this.checkpoints.epoch.wrapping_add(1);
                        this.checkpoints.open = false; this.checkpoints.review = None; this.checkpoints.loading = false;
                        cx.notify();
                    }})).relative().child(ui::layout_probe("checkpoint-close"))))
            .child(div().text_color(rgb(palette().muted))
                .child("Up to 8 snapshots, 512 KiB per task, 128 KiB per snapshot. Oldest snapshots are pruned; oversized state is refused, never truncated."));
        if state.loading {
            panel = panel.child("Loading checkpoint state...");
        }
        if let Some(error) = &state.error {
            panel = panel.child(
                div()
                    .text_color(rgb(palette().error))
                    .child(error.clone())
                    .relative()
                    .child(ui::layout_probe("checkpoint-error")),
            );
        }
        if let Some(review) = &state.review {
            let saved = review.checkpoint();
            panel = panel.child(div().relative().flex().flex_col().gap_1()
                .child(format!("Restore {} ({})? Current draft: {} bytes; saved draft: {} bytes; saved notes: {} bytes; checklist: {} items.",
                    saved.label, time(saved.created_at_ms), review.current_draft().len(), saved.draft.len(), saved.context.notes.len(), saved.context.checklist.len()))
                .child(format!("Saved draft preview (first 600 characters): {}", excerpt(&saved.draft)))
                .child(format!("Saved notes preview (first 600 characters): {}", excerpt(&saved.context.notes)))
                .child("A recovery snapshot of the current draft and notes is saved atomically. This review expires after five minutes and refuses intervening changes.")
                .child(div().flex().gap_2()
                    .child(ui::action("checkpoint-restore", "Restore draft + notes", None, disabled,
                        cx.listener(|this, _: &(), window, cx| this.restore_checkpoint(window, cx)))
                        .relative().child(ui::layout_probe("checkpoint-restore")))
                    .child(ui::action("checkpoint-cancel", "Cancel review", None, state.writing,
                        cx.listener(|this, _: &(), _, cx| { this.checkpoints.review = None; cx.notify(); }))
                        .relative().child(ui::layout_probe("checkpoint-cancel"))))
                .child(ui::layout_probe("checkpoint-review")));
        } else if let Some(history) = &state.history {
            if history.items.is_empty() {
                panel = panel.child("No checkpoints for this task.");
            }
            for (index, checkpoint) in history.items.iter().rev().enumerate() {
                let id = checkpoint.id.clone();
                panel = panel.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().min_w_0().child(format!(
                            "{} · {} · {} draft bytes, {} checklist items",
                            time(checkpoint.created_at_ms),
                            checkpoint.label,
                            checkpoint.draft.len(),
                            checkpoint.context.checklist.len()
                        )))
                        .child(
                            ui::action(
                                ("checkpoint-item", index),
                                "Review",
                                None,
                                disabled,
                                cx.listener(move |this, _: &(), window, cx| {
                                    this.review_checkpoint(id.clone(), window, cx)
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe_slot("checkpoint-item", index)),
                        ),
                );
            }
            panel = panel.child(ui::layout_probe("checkpoint-history"));
        }
        panel.into_any_element()
    }
}
fn time(value: i64) -> String {
    chrono::DateTime::from_timestamp_millis(value)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Unknown time".into())
}
fn excerpt(value: &str) -> String {
    value.chars().take(600).collect()
}
