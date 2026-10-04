//! Queued follow-up turns — upstream `composerDraftStore.queuedTurns` +
//! `ComposerQueuedHeader`/`QueuedComposerActions`/`queuedComposerDrain`.
//!
//! Submitting while a turn is in flight enqueues the draft instead of
//! sending; when the turn settles and the gates clear, the queue head is
//! auto-dispatched. Each row offers Steer (interrupt the running turn and
//! dispatch now), Edit (restore the text into the composer draft), and Delete.
use super::*;
use crate::ui::{self, Glyph, palette};

pub(super) enum Reply {
    Loaded(TaskId, Result<FollowupQueue, String>),
    Saved(TaskId, Option<(u64, String)>, Result<FollowupQueue, String>),
}

pub(super) struct FollowupState {
    pub task: Option<TaskId>,
    value: Option<FollowupQueue>,
    pub loading: bool,
    pub saving: bool,
    pub error: Option<String>,
}

impl FollowupState {
    pub fn new(_cx: &mut Context<Shell>) -> Self {
        Self {
            task: None,
            value: None,
            loading: false,
            saving: false,
            error: None,
        }
    }
    pub fn items(&self) -> &[FollowupDraft] {
        self.value
            .as_ref()
            .map_or(&[][..], |queue| queue.items.as_slice())
    }
    /// A queue write is in flight — used by the workspace/close guards so a
    /// navigation does not race a pending save.
    pub fn pending(&self, _cx: &App) -> bool {
        self.saving
    }
}

/// Upstream `compactQueuedComposerPreviewMarkdown`: first non-empty trimmed
/// line with heading/quote/checkbox/list/numbering prefixes stripped; fenced
/// code previews collapse to "Code block", empty input to "Queued follow-up".
fn compact_queued_preview(value: &str) -> String {
    let first = value
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    if first.is_empty() {
        return "Queued follow-up".into();
    }
    if first.starts_with("```") || first.starts_with("~~~") {
        return "Code block".into();
    }
    let mut line = first;
    if let Some(rest) = line
        .strip_prefix('#')
        .or_else(|| line.strip_prefix("##"))
        .or_else(|| line.strip_prefix("###"))
    {
        line = rest.trim_start();
    }
    if let Some(rest) = line.strip_prefix('>') {
        line = rest.trim_start();
    }
    for marker in ["- [ ] ", "- [x] ", "- [X] ", "- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            line = rest.trim_start();
            break;
        }
    }
    if let Some(rest) = line.strip_prefix(char::is_numeric)
        && let Some(rest) = rest.strip_prefix(['.', ')'])
    {
        line = rest.trim_start();
    }
    if line.trim().is_empty() {
        "Queued follow-up".into()
    } else {
        line.trim().chars().take(120).collect()
    }
}

impl Shell {
    /// Navigation gate: a pending reviewed recap request still blocks leaving
    /// the conversation (the queued-turn surface itself holds no editor state).
    pub(super) fn followup_navigation_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.recap.pending() {
            self.error = Some(
                "Create or cancel the reviewed recap request before leaving this conversation."
                    .into(),
            );
            cx.notify();
            return true;
        }
        false
    }
    pub(super) fn load_followups(&mut self, task: TaskId) {
        self.followups.task = Some(task);
        self.followups.value = None;
        self.followups.loading = true;
        self.followups.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Followups(Box::new(Reply::Loaded(
                task,
                workspace
                    .followup_queue(task)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
    }
    fn change_followup(
        &mut self,
        edit: FollowupEdit,
        clear: Option<(u64, String)>,
        cx: &mut Context<Self>,
    ) {
        let Some(task) = self.selected.filter(|t| Some(*t) == self.followups.task) else {
            return;
        };
        if self.followups.saving || self.followups.loading || self.close != CloseState::Open {
            return;
        }
        let Some(queue) = &self.followups.value else {
            return;
        };
        let revision = queue.revision;
        self.followups.saving = true;
        self.followups.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Followups(Box::new(Reply::Saved(
                task,
                clear,
                workspace
                    .edit_followups(task, revision, edit)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    /// Upstream `enqueueQueuedTurn`: a submit while the turn is in flight
    /// stores the draft as a queued turn instead of sending it.
    pub(super) fn queue_current_draft(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.selected else { return };
        if self.composer.read(cx).is_composing() || self.loading_task.is_some() {
            return;
        }
        if self.attachment_send_blocked() || self.attachments_have_pending() {
            self.followups.error =
                Some("Queued follow-ups carry text only. Send or remove attachments first.".into());
            cx.notify();
            return;
        }
        let text = self.composer.read(cx).text().to_owned();
        if text.trim().is_empty() {
            return;
        }
        self.snapshot_draft(cx);
        self.change_followup(
            FollowupEdit::Add(text.clone()),
            Some((self.draft_state.version(task), text)),
            cx,
        );
    }
    /// Upstream `onSteerQueuedComposerTurn` on a non-natively-steerable
    /// provider: remove the item, interrupt the running turn, and dispatch the
    /// text once the settle gates open (`queuedComposerDrain`).
    fn steer_followup(&mut self, id: String, text: String, cx: &mut Context<Self>) {
        let Some(task) = self.selected else { return };
        self.change_followup(FollowupEdit::Remove(id), None, cx);
        if self.busy.contains(&task) || self.connecting.contains(&task) {
            self.pending_steer = Some((task, text));
            self.cancel(cx);
        } else {
            self.dispatch_text = Some(text);
            self.send_prompt(cx);
        }
    }
    /// Upstream `onEditQueuedComposerTurn`: remove the item and restore its
    /// text into the composer draft.
    fn edit_followup(&mut self, id: String, text: String, cx: &mut Context<Self>) {
        self.change_followup(FollowupEdit::Remove(id), None, cx);
        let value = text;
        self.composer
            .update(cx, |entry, cx| entry.set_text(value, cx));
        self.remember_draft(cx);
        self.focus_composer = true;
        cx.notify();
    }
    /// Upstream `shouldAutoDispatchQueuedComposerTurn` + drain: once the turn
    /// is fully settled (not busy/connecting, no pending inputs) dispatch the
    /// queue head — or the pending steer — as a fresh prompt.
    pub(super) fn maybe_drain_followups(&mut self, task: TaskId, cx: &mut Context<Self>) {
        if self.selected != Some(task)
            || self.busy.contains(&task)
            || self.connecting.contains(&task)
            || self.controls.is_pending(task)
            || self.followups.saving
            || self.followups.loading
            || self.followups.task != Some(task)
            || self.loading_task.is_some()
        {
            return;
        }
        if let Some((steered, text)) = self.pending_steer.take_if(|(steered, _)| *steered == task) {
            let _ = steered;
            self.dispatch_text = Some(text);
            self.send_prompt(cx);
            return;
        }
        let Some(queue) = &self.followups.value else {
            return;
        };
        let Some(head) = queue.items.first() else {
            return;
        };
        if self
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.inputs.is_empty())
        {
            return;
        }
        let head = head.clone();
        self.change_followup(FollowupEdit::Remove(head.id), None, cx);
        self.dispatch_text = Some(head.text);
        self.send_prompt(cx);
    }
    pub(super) fn followup_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        let (task, result, clear) = match reply {
            Reply::Loaded(task, result) => {
                if self.followups.task == Some(task) {
                    self.followups.loading = false;
                }
                (task, result, None)
            }
            Reply::Saved(task, clear, result) => {
                self.followups.saving = false;
                (task, result, clear)
            }
        };
        if self.followups.task != Some(task) {
            return;
        }
        match result {
            Ok(value) => {
                if self
                    .followups
                    .value
                    .as_ref()
                    .is_none_or(|old| old.revision <= value.revision)
                {
                    self.followups.value = Some(value);
                }
                if let Some((version, text)) = clear
                    && self.selected == Some(task)
                    && self.draft_state.version(task) == version
                    && self.composer.read(cx).text() == text
                {
                    self.composer.update(cx, |entry, cx| entry.clear(cx));
                    self.remember_draft(cx);
                }
                self.followups.error = None;
            }
            Err(error) => self.followups.error = Some(error),
        }
        cx.notify();
        self.maybe_drain_followups(task, cx);
    }
    /// Upstream `ComposerQueuedHeader`: queued rows in a stacked panel fused
    /// to the top of the composer — always rendered while items exist, each
    /// row showing the steer icon, a compact preview, and
    /// Steer/Delete/Edit actions.
    pub(super) fn followups_view(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let state = &self.followups;
        let items = state.items();
        if items.is_empty() && !state.loading && state.error.is_none() {
            return div().into_any_element();
        }
        let mut root = div().id("composer-followups").min_w_0().flex().flex_col();
        if let Some(error) = &state.error {
            root = root.child(
                div()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(rgb(palette().error))
                            .child(error.clone()),
                    )
                    .child(
                        ui::action(
                            "reload-followups",
                            "Reload",
                            Some(Glyph::Restore),
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                if let Some(task) = this.selected {
                                    this.load_followups(task);
                                    cx.notify();
                                }
                            }),
                        )
                        .text_size(px(12.)),
                    ),
            );
        }
        if state.loading && items.is_empty() {
            return root
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(12.))
                        .text_color(rgb(palette().muted))
                        .child("Loading queued follow-ups..."),
                )
                .into_any_element();
        }
        root.children(items.iter().enumerate().map(|(index, item)| {
            let steer_id = item.id.clone();
            let steer_text = item.text.clone();
            let edit_id = item.id.clone();
            let edit_text = item.text.clone();
            let remove_id = item.id.clone();
            div()
                .id(("queued-followup", index))
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .border_t_1()
                .border_color(rgb(palette().border))
                .child(
                    ui::icon(Glyph::Send)
                        .size(px(12.))
                        .text_color(rgb(palette().muted)),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_size(px(12.))
                        .text_color(rgb(palette().text))
                        .child(compact_queued_preview(&item.text)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .gap_1()
                        .child(
                            ui::action(
                                ("steer-followup", index),
                                "Steer",
                                Some(Glyph::Send),
                                state.saving,
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.steer_followup(steer_id.clone(), steer_text.clone(), cx)
                                }),
                            )
                            .text_size(px(12.)),
                        )
                        .child(
                            ui::chrome_button(
                                "remove-followup",
                                "Delete queued follow-up",
                                Glyph::Close,
                                state.saving,
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.change_followup(
                                        FollowupEdit::Remove(remove_id.clone()),
                                        None,
                                        cx,
                                    )
                                }),
                            )
                            .id(("followup-remove", index))
                            .size(px(20.)),
                        )
                        .child(
                            ui::chrome_button(
                                "edit-followup",
                                "Edit queued follow-up",
                                Glyph::Compose,
                                state.saving,
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.edit_followup(edit_id.clone(), edit_text.clone(), cx)
                                }),
                            )
                            .id(("followup-edit", index))
                            .size(px(20.)),
                        ),
                )
        }))
        .into_any_element()
    }
}
