//! Native evidence-first Debug workflow. Preparing a step only edits the draft.
use super::*;
use crate::ui::{self, Glyph, palette};
pub(super) struct DebugState {
    task: Option<TaskId>,
    value: Option<DebugWorkflow>,
    input: Entity<TextEntry>,
    busy: bool,
    pub(super) open: bool,
    error: Option<String>,
    detached_edit: bool,
    _subscription: Subscription,
}
pub(super) struct Reply {
    task: TaskId,
    before: Option<String>,
    result: Result<DebugWorkflow, String>,
}
impl DebugState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let input = cx.new(|cx| {
            TextEntry::new(
                "Observed evidence for this phase, up to 16 KiB",
                EntryMode::Editor,
                80.,
                cx,
            )
        });
        let subscription = cx.subscribe(&input, |_, _, _, cx| cx.notify());
        Self {
            task: None,
            value: None,
            input,
            busy: false,
            open: false,
            error: None,
            detached_edit: false,
            _subscription: subscription,
        }
    }
    pub fn pending(&self, cx: &App) -> bool {
        (self.busy && self.value.is_some())
            || self.detached_edit
            || self.open
                && (self.input.read(cx).is_composing()
                    || self.value.as_ref().is_some_and(|value| {
                        self.input.read(cx).text() != value.current_evidence()
                    }))
    }
}
impl Shell {
    pub(super) fn load_debug(&mut self, task: TaskId, cx: &mut Context<Self>) {
        self.debug_workflow.task = Some(task);
        self.debug_workflow.value = None;
        self.debug_workflow.busy = true;
        self.debug_workflow.open = false;
        self.debug_workflow.error = None;
        self.debug_workflow.detached_edit = false;
        self.debug_workflow
            .input
            .update(cx, |entry, cx| entry.clear(cx));
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::DebugWorkflow(Box::new(Reply {
                task,
                before: None,
                result: workspace
                    .debug_workflow(task)
                    .await
                    .map_err(|e| e.to_string()),
            })))
        });
    }
    pub(super) fn open_debug(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_none() || self.loading_task.is_some() || self.debug_workflow.busy {
            return;
        }
        self.show_conversation(cx);
        // Accordion: one workflow expanded at a time (see open_goals).
        self.goals.open = false;
        self.recap.open = false;
        self.debug_workflow.open = true;
        cx.notify();
    }
    fn change_debug(&mut self, edit: DebugEdit, cx: &mut Context<Self>) {
        let view = &mut self.debug_workflow;
        let Some(task) = self.selected.filter(|id| Some(*id) == view.task) else {
            return;
        };
        if view.busy
            || view.detached_edit
            || self.close != CloseState::Open
            || view.input.read(cx).is_composing()
        {
            return;
        }
        let Some(value) = &view.value else {
            return;
        };
        let before = view.input.read(cx).text().to_owned();
        if !matches!(edit, DebugEdit::Evidence(_)) && before != value.current_evidence() {
            view.error = Some(
                "Save or discard the current evidence edit before changing phase or mode.".into(),
            );
            cx.notify();
            return;
        }
        let revision = value.revision;
        view.busy = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::DebugWorkflow(Box::new(Reply {
                task,
                before: Some(before),
                result: workspace
                    .edit_debug_workflow(task, revision, edit)
                    .await
                    .map_err(|e| e.to_string()),
            })))
        });
        cx.notify();
    }
    pub(super) fn debug_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        let view = &mut self.debug_workflow;
        if view.task != Some(reply.task) {
            return;
        }
        view.busy = false;
        match reply.result {
            Ok(value) => {
                let changed_during_write = reply
                    .before
                    .as_ref()
                    .is_some_and(|before| view.input.read(cx).text() != before);
                let changed_phase = view
                    .value
                    .as_ref()
                    .is_some_and(|old| old.phase != value.phase);
                if !changed_during_write {
                    view.input.update(cx, |entry, cx| {
                        entry.set_text(value.current_evidence().to_owned(), cx)
                    });
                }
                view.value = Some(value);
                view.detached_edit = changed_during_write && changed_phase;
                view.error = view.detached_edit.then(|| "This unsaved edit belongs to the previous phase. Copy it before discarding the edit. It will not be saved to a different phase.".into());
            }
            Err(error) => view.error = Some(error),
        }
        cx.notify();
    }
    fn prepare_debug(&mut self, cx: &mut Context<Self>) {
        if self.debug_workflow.pending(cx)
            || self.loading_task.is_some()
            || self.composer.read(cx).is_composing()
        {
            return;
        }
        let Some(task) = self
            .selected
            .filter(|id| Some(*id) == self.debug_workflow.task)
        else {
            return;
        };
        let Some(value) = &self.debug_workflow.value else {
            return;
        };
        match value.prepare_prompt(task, self.composer.read(cx).text()) {
            Ok(text) => {
                self.composer
                    .update(cx, |entry, cx| entry.set_text(text, cx));
                self.remember_draft(cx);
                self.focus_composer = true;
                self.notice = Some("Debug step prepared in the composer. Review and Send explicitly. No permissions changed.".into());
            }
            Err(error) => self.debug_workflow.error = Some(error.to_string()),
        }
        cx.notify();
    }
    /// Collapsed idle debug switch for the shared one-row workflow strip.
    pub(super) fn debug_compact(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let view = &self.debug_workflow;
        if self.selected.is_none() || self.selected != view.task {
            return None;
        }
        if self
            .thread
            .as_ref()
            .is_some_and(|t| t.timeline.is_empty() && t.plan.is_empty())
        {
            return None;
        }
        if view.open {
            return None;
        }
        let label = view.value.as_ref().map_or("Debug".to_owned(), |value| {
            if value.enabled {
                format!(
                    "Debug: {}{}",
                    value.phase.label(),
                    if value.completed { " (verified)" } else { "" }
                )
            } else {
                "Debug: off".into()
            }
        });
        Some(
            ui::header_action(
                "debug-open",
                label,
                Some(Glyph::Debug),
                false,
                cx.listener(|this, _: &(), _, cx| this.open_debug(cx)),
            )
            .child(ui::layout_probe_enabled(
                "debug-open",
                !view.busy && self.loading_task.is_none(),
            ))
            .into_any_element(),
        )
    }
    pub(super) fn debug_bar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let view = &self.debug_workflow;
        if self.selected.is_none() || self.selected != view.task {
            return div().into_any_element();
        }
        // Collapsed idle state lives in the shared workflow strip.
        if !view.open {
            return div().into_any_element();
        }
        let label = view.value.as_ref().map_or("Debug".to_owned(), |value| {
            if value.enabled {
                format!(
                    "Debug: {}{}",
                    value.phase.label(),
                    if value.completed { " (verified)" } else { "" }
                )
            } else {
                "Debug: off".into()
            }
        });
        let mut root = div().px_4().py_1().flex().flex_col().gap_1().child(
            ui::action(
                "debug-open",
                label,
                Some(Glyph::Debug),
                view.open,
                cx.listener(|this, _: &(), _, cx| this.open_debug(cx)),
            )
            .child(ui::layout_probe_enabled(
                "debug-open",
                !view.busy && self.loading_task.is_none(),
            )),
        );
        if let Some(value) = &view.value {
            let enabled = value.enabled;
            let pending = view.pending(cx);
            root = root
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette().muted))
                        .child(value.phase.instruction()),
                )
                .child(
                    div()
                        .id("debug-evidence-input")
                        .h(px(80.))
                        .flex_shrink_0()
                        .relative()
                        .child(ui::layout_probe("debug-evidence-input"))
                        .child(view.input.clone()),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            ui::icon_button(
                                "debug-enabled",
                                if enabled {
                                    "Pause Debug mode"
                                } else {
                                    "Enable Debug mode"
                                },
                                Glyph::Debug,
                                view.busy,
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.change_debug(DebugEdit::Enable(!enabled), cx)
                                }),
                            )
                            .child(ui::layout_probe("debug-enabled")),
                        )
                        .child(
                            ui::action(
                                "debug-save-evidence",
                                "Save evidence",
                                None,
                                view.busy,
                                cx.listener(|this, _: &(), _, cx| {
                                    let text = this.debug_workflow.input.read(cx).text().to_owned();
                                    this.change_debug(DebugEdit::Evidence(text), cx);
                                }),
                            )
                            .child(ui::layout_probe("debug-save-evidence")),
                        )
                        .child(ui::action(
                            "debug-back",
                            "Back",
                            None,
                            view.busy,
                            cx.listener(|this, _: &(), _, cx| {
                                this.change_debug(DebugEdit::Back, cx)
                            }),
                        ))
                        .child(
                            ui::action(
                                "debug-next",
                                if value.phase == DebugPhase::Verify {
                                    "Mark verified"
                                } else {
                                    "Next phase"
                                },
                                None,
                                view.busy,
                                cx.listener(|this, _: &(), _, cx| {
                                    this.change_debug(DebugEdit::Advance, cx)
                                }),
                            )
                            .child(ui::layout_probe("debug-next")),
                        )
                        .child(
                            ui::icon_button(
                                "debug-prepare",
                                "Prepare current Debug step in composer",
                                Glyph::Compose,
                                pending || !enabled || value.completed,
                                cx.listener(|this, _: &(), _, cx| this.prepare_debug(cx)),
                            )
                            .child(ui::layout_probe("debug-prepare")),
                        )
                        .child(ui::action(
                            "debug-discard",
                            "Discard edit",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                if !this.debug_workflow.busy {
                                    let text = this
                                        .debug_workflow
                                        .value
                                        .as_ref()
                                        .map_or("", DebugWorkflow::current_evidence)
                                        .to_owned();
                                    this.debug_workflow
                                        .input
                                        .update(cx, |entry, cx| entry.set_text(text, cx));
                                    this.debug_workflow.detached_edit = false;
                                    this.debug_workflow.error = None;
                                    cx.notify();
                                }
                            }),
                        ))
                        .child(
                            ui::action(
                                "debug-close",
                                "Close",
                                None,
                                false,
                                cx.listener(|this, _: &(), _, cx| {
                                    if !this.debug_workflow.pending(cx) {
                                        this.debug_workflow.open = false;
                                        cx.notify();
                                    }
                                }),
                            )
                            .child(ui::layout_probe("debug-close")),
                        ),
                );
        } else {
            root = root.child(div().text_xs().child(if view.busy {
                "Loading saved Debug workflow..."
            } else {
                "Debug workflow could not be loaded."
            }));
            if !view.busy {
                root = root.child(ui::action(
                    "debug-reload",
                    "Reload",
                    None,
                    false,
                    cx.listener(|this, _: &(), _, cx| {
                        if let Some(task) = this.selected {
                            this.load_debug(task, cx);
                            this.debug_workflow.open = true;
                        }
                    }),
                ));
            }
        }
        if let Some(error) = &view.error {
            root = root.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette().error))
                    .child(error.clone()),
            );
        }
        root.into_any_element()
    }
}
