//! Per-task notes — a freeform scratchpad (upstream `ThreadNotes`), separate
//! from the agent transcript and composer.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::{EventEmitter, FocusHandle};

pub(super) enum ContextReply {
    Loaded {
        task: TaskId,
        generation: u64,
        result: Result<TaskContext, String>,
    },
    Saved {
        task: TaskId,
        generation: u64,
        result: Result<TaskContext, String>,
    },
}
enum ContextEvent {
    Save(TaskContext),
    Reload,
    Dismiss,
}
#[derive(Default)]
pub(super) struct SavedContextState {
    pub dialog: Option<Entity<ContextDialog>>,
    task: Option<TaskId>,
    generation: u64,
    subscription: Option<Subscription>,
    previous_focus: Option<FocusHandle>,
    restore_focus: bool,
}
impl Shell {
    pub(super) fn open_saved_context(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.task().cloned() else {
            return;
        };
        if self.checkpoints.writing()
            || self.saved_context.dialog.is_some()
            || self.organization.dialog.is_some()
            || self.kanban.dialog.is_some()
            || self.loading_task.is_some()
            || self.close != CloseState::Open
        {
            return;
        }
        self.controls.retire();
        self.chat_tools.retire();
        self.environment.retire_popup();
        self.navigation.menu_open = false;
        self.settings.popup = None;
        self.focus_composer = false;
        self.saved_context.generation = self.saved_context.generation.wrapping_add(1);
        self.saved_context.task = Some(task.id);
        let dialog = cx.new(|cx| ContextDialog::new(task.title, cx));
        self.saved_context.subscription = Some(cx.subscribe(&dialog, move |this, _, event, cx| {
            match event {
                ContextEvent::Dismiss => {
                    this.saved_context.dialog = None;
                    this.saved_context.task = None;
                    this.saved_context.restore_focus = true;
                }
                ContextEvent::Reload => this.load_saved_context(),
                ContextEvent::Save(value) => {
                    let workspace = this.controller.workspace.clone();
                    let value = value.clone();
                    let generation = this.saved_context.generation;
                    this.job(async move {
                        Ok(Update::SavedContext(Box::new(ContextReply::Saved {
                            task: task.id,
                            generation,
                            result: workspace
                                .save_task_context(task.id, value.revision, value)
                                .await
                                .map_err(|error| error.to_string()),
                        })))
                    });
                }
            }
            cx.notify();
        }));
        self.saved_context.previous_focus = window.focused(cx);
        self.saved_context.dialog = Some(dialog);
        self.load_saved_context();
        cx.notify();
    }
    fn load_saved_context(&mut self) {
        let Some(task) = self.saved_context.task else {
            return;
        };
        let generation = self.saved_context.generation;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::SavedContext(Box::new(ContextReply::Loaded {
                task,
                generation,
                result: workspace
                    .task_context(task)
                    .await
                    .map_err(|error| error.to_string()),
            })))
        });
    }
    pub(super) fn saved_context_reply(&mut self, reply: ContextReply, cx: &mut Context<Self>) {
        let (task, generation, saved, result) = match reply {
            ContextReply::Loaded {
                task,
                generation,
                result,
            } => (task, generation, false, result),
            ContextReply::Saved {
                task,
                generation,
                result,
            } => (task, generation, true, result),
        };
        if self.saved_context.task != Some(task) || self.saved_context.generation != generation {
            return;
        }
        if let Some(dialog) = &self.saved_context.dialog {
            dialog.update(cx, |dialog, cx| dialog.receive(saved, result, cx));
        }
    }
    pub(super) fn restore_saved_context_focus(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saved_context.restore_focus {
            self.saved_context.restore_focus = false;
            if let Some(focus) = self.saved_context.previous_focus.take() {
                window.focus(&focus, cx);
            }
        }
    }
    pub(super) fn saved_context_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        ui::chrome_button(
            "chat-notes",
            "Notes",
            Glyph::Notebook,
            self.selected.is_none(),
            cx.listener(|this, _: &(), window, cx| this.open_saved_context(window, cx)),
        )
        .size(px(26.))
        .into_any_element()
    }
}

pub(in crate::shell) struct ContextDialog {
    title: String,
    notes: Entity<TextEntry>,
    base: Option<TaskContext>,
    loading: bool,
    saving: bool,
    error: Option<String>,
    status: Option<String>,
    confirm_discard: bool,
    confirm_reload: bool,
    focus: FocusHandle,
    needs_focus: bool,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<ContextEvent> for ContextDialog {}
impl ContextDialog {
    fn new(title: String, cx: &mut Context<Self>) -> Self {
        // Upstream placeholder copy: "Type here".
        let notes = cx.new(|cx| TextEntry::new("Type here", EntryMode::Editor, 180., cx));
        let subscriptions = vec![cx.subscribe(&notes, |this, _, event, cx| {
            if matches!(event, EntryEvent::Save) {
                this.save(cx);
            }
            cx.notify();
        })];
        Self {
            title,
            notes,
            base: None,
            loading: true,
            saving: false,
            error: None,
            status: None,
            confirm_discard: false,
            confirm_reload: false,
            focus: cx.focus_handle(),
            needs_focus: true,
            _subscriptions: subscriptions,
        }
    }
    fn receive(
        &mut self,
        saved: bool,
        result: Result<TaskContext, String>,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        self.saving = false;
        match result {
            Ok(value) => {
                if !saved {
                    self.notes
                        .update(cx, |entry, cx| entry.set_text(value.notes.clone(), cx));
                }
                // Saving updates the acknowledged baseline, never a newer edit
                // typed while the worker was writing the submitted snapshot.
                self.base = Some(value);
                self.error = None;
                let edits_remain = saved && self.dirty(cx);
                self.status = saved.then(|| {
                    if edits_remain {
                        "Saved the submitted notes. Newer edits remain unsaved.".into()
                    } else {
                        "Notes saved.".into()
                    }
                });
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }
    fn snapshot(&self, cx: &Context<Self>) -> Option<TaskContext> {
        self.base.as_ref().map(|base| TaskContext {
            version: TaskContext::CURRENT_VERSION,
            revision: base.revision,
            notes: self.notes.read(cx).text().to_owned(),
        })
    }
    fn dirty(&self, cx: &Context<Self>) -> bool {
        self.base
            .as_ref()
            .is_some_and(|base| self.notes.read(cx).text() != base.notes)
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        if self.saving || self.loading {
            return;
        }
        let Some(value) = self.snapshot(cx) else {
            return;
        };
        if let Err(error) = value.validate() {
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        self.saving = true;
        self.error = None;
        cx.emit(ContextEvent::Save(value));
        cx.notify();
    }
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            self.status = Some("Finish or cancel the current save first.".into());
            cx.notify();
            return;
        }
        if self.dirty(cx) {
            self.confirm_discard = true;
            cx.notify();
        } else {
            cx.emit(ContextEvent::Dismiss);
        }
    }
    fn reload(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        if self.saving || self.loading {
            return;
        }
        if self.dirty(cx) && !confirmed {
            self.confirm_reload = true;
            cx.notify();
            return;
        }
        self.confirm_reload = false;
        self.loading = true;
        self.error = None;
        cx.emit(ContextEvent::Reload);
        cx.notify();
    }
}
impl gpui::Render for ContextDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_focus {
            window.focus(&self.focus, cx);
            self.needs_focus = false;
        }
        let dirty = self.dirty(cx);
        let available = self.base.is_some() && !self.loading;
        let modal = div()
            .id("saved-context-dialog")
            .role(gpui::Role::Dialog)
            .aria_label("Thread notes")
            .track_focus(&self.focus)
            .tab_group()
            .tab_stop(true)
            .relative()
            .occlude()
            .w_full()
            .max_w(px(760.))
            .max_h(window.viewport_size().height - px(40.))
            .min_h_0()
            .flex()
            .flex_col()
            .rounded(px(16.))
            .border_1()
            .border_color(rgb(palette().border))
            .bg(rgb(palette().overlay))
            .shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.prefer_character_input || this.notes.read(cx).is_composing() {
                    return;
                }
                let modifiers = event.keystroke.modifiers;
                if event.keystroke.key == "escape" {
                    if this.confirm_discard || this.confirm_reload {
                        this.confirm_discard = false;
                        this.confirm_reload = false;
                        cx.notify();
                    } else {
                        this.dismiss(cx);
                    }
                    cx.stop_propagation();
                } else if event.keystroke.key == "s"
                    && (modifiers.control || modifiers.platform)
                    && !modifiers.alt
                {
                    this.save(cx);
                    cx.stop_propagation();
                } else if event.keystroke.key == "tab"
                    && !modifiers.control
                    && !modifiers.platform
                    && !modifiers.alt
                {
                    if modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
            }))
            .child(ui::layout_probe("saved-context-dialog"))
            .child(
                div()
                    .p_4()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(ui::icon(Glyph::Notebook))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child("Notes")
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_ellipsis()
                                    .text_color(rgb(palette().muted))
                                    .child(self.title.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette().muted))
                            .child(if self.saving {
                                "Saving..."
                            } else if dirty {
                                "Unsaved changes"
                            } else {
                                "Saved"
                            }),
                    )
                    .child(ui::chrome_button(
                        "context-close",
                        "Close notes",
                        Glyph::Close,
                        self.saving,
                        cx.listener(|this, _: &(), _, cx| this.dismiss(cx)),
                    )),
            )
            .children(self.error.clone().map(|error| {
                div()
                    .px_4()
                    .py_2()
                    .text_color(rgb(palette().error))
                    .child(error)
            }))
            .children(self.status.clone().map(|status| {
                div()
                    .px_4()
                    .py_1()
                    .text_size(px(12.))
                    .text_color(rgb(palette().muted))
                    .child(status)
            }))
            .child(
                div()
                    .id("saved-context-scroll")
                    .px_4()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child("Private to this chat. Notes are not sent to the agent."),
                    )
                    .when(self.loading, |el| el.child("Loading notes..."))
                    .when(available, |el| {
                        el.child(
                            div()
                                .relative()
                                .child(ui::layout_probe("context-notes-input"))
                                .child(self.notes.clone()),
                        )
                    }),
            )
            .child(
                div()
                    .p_4()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_t_1()
                    .border_color(rgb(palette().border))
                    .child(
                        ui::button("context-reload", "Reload saved", false)
                            .when(self.saving || self.loading, |el| el.opacity(0.4))
                            .on_click(cx.listener(|this, _, _, cx| this.reload(false, cx))),
                    )
                    .child(div().flex_1())
                    .child(
                        ui::button("context-save", "Save notes", true)
                            .relative()
                            .child(ui::layout_probe("context-save"))
                            .when(!available || self.saving, |el| el.opacity(0.4))
                            .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                    ),
            )
            .when(self.confirm_discard || self.confirm_reload, |el| {
                el.child(
                    div()
                        .p_3()
                        .bg(rgb(palette().notice_surface))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().child("Discard the unsaved notes edits?"))
                        .child(
                            ui::button("context-keep", "Keep editing", false)
                                .relative()
                                .child(ui::layout_probe("context-keep"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.confirm_discard = false;
                                    this.confirm_reload = false;
                                    cx.notify();
                                })),
                        )
                        .child(
                            ui::button("context-discard", "Discard edits", false)
                                .relative()
                                .child(ui::layout_probe("context-discard"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if this.confirm_reload {
                                        this.reload(true, cx);
                                    } else {
                                        cx.emit(ContextEvent::Dismiss);
                                    }
                                })),
                        ),
                )
            });
        div()
            .absolute()
            .inset_0()
            .size_full()
            .p_4()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x00000088))
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.dismiss(cx);
                    cx.stop_propagation();
                }),
            )
            .child(modal)
    }
}
