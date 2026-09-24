//! User notes and checklists, separate from the agent transcript and composer.
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
    Insert(String),
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
                    this.job(async move { Ok(Update::SavedContext(Box::new(ContextReply::Saved {
                        task: task.id, generation,
                        result: workspace.save_task_context(task.id, value.revision, value).await.map_err(|error| error.to_string()),
                    }))) });
                }
                ContextEvent::Insert(text) => {
                    if this.selected != Some(task.id) || this.loading_task.is_some() || this.close != CloseState::Open { return; }
                    let draft = this.composer.read(cx).text().to_owned();
                    let separator = if draft.is_empty() { "" } else { "\n\n" };
                    if draft.len().saturating_add(separator.len()).saturating_add(text.len()) > 1024 * 1024 {
                        if let Some(dialog) = &this.saved_context.dialog {
                            dialog.update(cx, |dialog, cx| { dialog.error = Some("Combined draft exceeds 1 MiB. Nothing was inserted.".into()); cx.notify(); });
                        }
                    } else {
                        this.composer.update(cx, |entry, cx| entry.set_text(format!("{draft}{separator}{text}"), cx));
                        this.remember_draft(cx);
                        if let Some(dialog) = &this.saved_context.dialog {
                            dialog.update(cx, |dialog, cx| { dialog.status = Some("Added to the chat draft without sending. Save notes separately.".into()); cx.notify(); });
                        }
                    }
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
            "Notes and checklist",
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
    item: Entity<TextEntry>,
    base: Option<TaskContext>,
    checklist: Vec<ChecklistItem>,
    folder_references: Vec<String>,
    picking_folders: bool,
    edit_item: Option<String>,
    loading: bool,
    saving: bool,
    error: Option<String>,
    status: Option<String>,
    confirm_discard: bool,
    confirm_reload: bool,
    hide_done: bool,
    focus: FocusHandle,
    needs_focus: bool,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<ContextEvent> for ContextDialog {}
impl ContextDialog {
    fn new(title: String, cx: &mut Context<Self>) -> Self {
        let notes = cx.new(|cx| {
            TextEntry::new(
                "Private notes for this chat...",
                EntryMode::Editor,
                180.,
                cx,
            )
        });
        let item =
            cx.new(|cx| TextEntry::new("Add a checklist item...", EntryMode::SingleLine, 34., cx));
        let subscriptions = vec![
            cx.subscribe(&notes, |this, _, event, cx| {
                if matches!(event, EntryEvent::Save) {
                    this.save(cx);
                }
                cx.notify();
            }),
            cx.subscribe(&item, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.apply_item(cx);
                }
                cx.notify();
            }),
        ];
        Self {
            title,
            notes,
            item,
            base: None,
            checklist: Vec::new(),
            folder_references: Vec::new(),
            picking_folders: false,
            edit_item: None,
            loading: true,
            saving: false,
            error: None,
            status: None,
            confirm_discard: false,
            confirm_reload: false,
            hide_done: false,
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
                    self.checklist = value.checklist.clone();
                    self.folder_references = value.folder_references.clone();
                    self.item.update(cx, |entry, cx| entry.clear(cx));
                    self.edit_item = None;
                }
                // Saving updates the acknowledged baseline, never a newer edit
                // typed while the worker was writing the submitted snapshot.
                self.base = Some(value);
                self.error = None;
                let edits_remain = saved && self.dirty(cx);
                self.status = saved.then(|| {
                    if edits_remain {
                        "Saved the submitted context. Newer edits remain unsaved.".into()
                    } else {
                        "Notes, checklist, and folder references saved.".into()
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
            checklist: self.checklist.clone(),
            folder_references: self.folder_references.clone(),
        })
    }
    fn dirty(&self, cx: &Context<Self>) -> bool {
        !self.item.read(cx).text().is_empty()
            || self.base.as_ref().is_some_and(|base| {
                self.notes.read(cx).text() != base.notes
                    || self.checklist != base.checklist
                    || self.folder_references != base.folder_references
            })
    }
    fn choose_folders(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.saving || self.picking_folders || self.base.is_none() {
            return;
        }
        let revision = self.base.as_ref().map(|value| value.revision);
        self.picking_folders = true;
        self.error = None;
        self.status = None;
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some(
                "Choose folders to save path references only; folder contents will not be read"
                    .into(),
            ),
        });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                this.picking_folders = false;
                if this.base.as_ref().map(|value| value.revision) != revision {
                    this.status = Some(
                        "The saved context changed while the folder picker was open. Reload before selecting folders again.".into(),
                    );
                    cx.notify();
                    return;
                }
                match result {
                    Ok(Ok(Some(paths))) => {
                        let mut selected = Vec::new();
                        for path in paths {
                            let Some(path) = path.to_str() else {
                                this.error = Some(
                                    "A selected folder path cannot be represented as UTF-8. Nothing was added.".into(),
                                );
                                cx.notify();
                                return;
                            };
                            if let Err(error) = TaskContext::validate_folder_reference(path) {
                                this.error = Some(format!(
                                    "A selected folder path is not supported: {error}. Nothing was added."
                                ));
                                cx.notify();
                                return;
                            }
                            if !selected.iter().any(|existing: &String| existing.as_str() == path)
                                && !this
                                    .folder_references
                                    .iter()
                                    .any(|existing| existing.as_str() == path)
                            {
                                selected.push(path.to_owned());
                            }
                        }
                        if this.folder_references.len() + selected.len()
                            > TaskContext::MAX_FOLDER_REFERENCES
                        {
                            this.error = Some(format!(
                                "A chat can save at most {} folder references. Nothing was added.",
                                TaskContext::MAX_FOLDER_REFERENCES
                            ));
                        } else if selected.is_empty() {
                            this.status = Some("Those folder references are already listed.".into());
                        } else {
                            let added = selected.len();
                            this.folder_references.extend(selected);
                            this.status = Some(format!(
                                "Added {added} folder path reference(s). Save to keep them with this chat."
                            ));
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => {
                        this.error = Some(
                            "The folder picker could not open. No folder references were changed.".into(),
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn apply_item(&mut self, cx: &mut Context<Self>) -> bool {
        if self.loading || self.base.is_none() {
            return false;
        }
        let text = self.item.read(cx).text().trim().to_owned();
        if text.is_empty() {
            if self.edit_item.is_some() {
                self.error = Some("Checklist items cannot be empty.".into());
                cx.notify();
                return false;
            }
            return true;
        }
        if text.len() > MAX_CHECKLIST_TEXT || text.chars().any(char::is_control) {
            self.error = Some("Keep a checklist item on one line, within 512 bytes.".into());
            cx.notify();
            return false;
        }
        if let Some(id) = &self.edit_item {
            if let Some(item) = self.checklist.iter_mut().find(|item| &item.id == id) {
                item.text = text;
            } else {
                self.error = Some(
                    "The edited item was removed. Cancel its edit before adding a new item.".into(),
                );
                cx.notify();
                return false;
            }
        } else {
            if self.checklist.len() >= MAX_CHECKLIST_ITEMS {
                self.error = Some("This checklist already has 128 items.".into());
                cx.notify();
                return false;
            }
            self.checklist.push(ChecklistItem::new(text));
        }
        self.edit_item = None;
        self.item.update(cx, |entry, cx| entry.clear(cx));
        self.error = None;
        self.status = None;
        cx.notify();
        true
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        if self.saving || self.loading || self.picking_folders || !self.apply_item(cx) {
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
        if self.saving || self.picking_folders {
            self.status = Some("Finish or cancel the current save or folder picker first.".into());
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
        if self.saving || self.loading || self.picking_folders {
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
    fn move_item(&mut self, id: &str, earlier: bool, cx: &mut Context<Self>) {
        let Some(index) = self.checklist.iter().position(|item| item.id == id) else {
            return;
        };
        let next = if earlier {
            index.checked_sub(1)
        } else {
            index
                .checked_add(1)
                .filter(|index| *index < self.checklist.len())
        };
        if let Some(next) = next {
            self.checklist.swap(index, next);
            self.status = None;
            cx.notify();
        }
    }
    fn context_text(&mut self, cx: &mut Context<Self>) -> Option<String> {
        if !self.apply_item(cx) {
            return None;
        }
        let value = self.snapshot(cx)?;
        if let Err(error) = value.validate() {
            self.error = Some(error.to_string());
            cx.notify();
            return None;
        }
        let text = value.as_prompt_context();
        if text.is_empty() {
            self.status = Some("Add notes, checklist items, or folder references first.".into());
            cx.notify();
            None
        } else {
            Some(text)
        }
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
        let complete = self.checklist.iter().filter(|item| item.done).count();
        let rows = self
            .checklist
            .iter()
            .enumerate()
            .filter(|(_, item)| !self.hide_done || !item.done)
            .map(|(index, item)| {
                let toggle = item.id.clone();
                let edit = item.id.clone();
                let remove = item.id.clone();
                let earlier = item.id.clone();
                let later = item.id.clone();
                div()
                    .id(("context-item", index))
                    .flex()
                    .items_center()
                    .gap_1()
                    .min_w_0()
                    .py_1()
                    .child(
                        ui::button_shell(
                            SharedString::from(format!("context-check-{index}")),
                            "Checklist item",
                            item.done,
                        )
                        .role(gpui::Role::CheckBox)
                        .aria_label(item.text.clone())
                        .aria_toggled(if item.done {
                            gpui::Toggled::True
                        } else {
                            gpui::Toggled::False
                        })
                        .size(px(24.))
                        .p_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .relative()
                        .child(ui::layout_probe_slot("context-check", index))
                        .children(item.done.then(|| ui::icon(Glyph::Check).size(px(14.))))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(item) =
                                this.checklist.iter_mut().find(|item| item.id == toggle)
                            {
                                item.done = !item.done;
                                this.status = None;
                                cx.notify();
                            }
                        })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(13.))
                            .when(item.done, |el| el.text_color(rgb(palette().muted)))
                            .child(item.text.clone()),
                    )
                    .child(
                        ui::chrome_button(
                            "context-item-up",
                            "Move item earlier",
                            Glyph::Back,
                            index == 0,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.move_item(&earlier, true, cx)
                            }),
                        )
                        .size(px(22.)),
                    )
                    .child(
                        ui::chrome_button(
                            "context-item-down",
                            "Move item later",
                            Glyph::Forward,
                            index + 1 == self.checklist.len(),
                            cx.listener(move |this, _: &(), _, cx| {
                                this.move_item(&later, false, cx)
                            }),
                        )
                        .size(px(22.)),
                    )
                    .child(
                        ui::chrome_button(
                            "context-item-edit",
                            "Edit checklist item",
                            Glyph::Compose,
                            false,
                            cx.listener(move |this, _: &(), window, cx| {
                                if !this.item.read(cx).text().is_empty() {
                                    this.error = Some(
                                        "Add, save or cancel the current item text first.".into(),
                                    );
                                    cx.notify();
                                    return;
                                }
                                if let Some(item) =
                                    this.checklist.iter().find(|item| item.id == edit)
                                {
                                    this.item.update(cx, |entry, cx| {
                                        entry.set_text(item.text.clone(), cx)
                                    });
                                    this.edit_item = Some(edit.clone());
                                    window.focus(&this.item.read(cx).focus_handle(cx), cx);
                                    cx.notify();
                                }
                            }),
                        )
                        .size(px(22.)),
                    )
                    .child(
                        ui::chrome_button(
                            "context-item-remove",
                            "Remove checklist item",
                            Glyph::Close,
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.checklist.retain(|item| item.id != remove);
                                this.status = None;
                                cx.notify();
                            }),
                        )
                        .size(px(22.)),
                    )
            })
            .collect::<Vec<_>>();
        let folder_rows = self
            .folder_references
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let remove = path.clone();
                div()
                    .id(("context-folder", index))
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .py_1()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .text_ellipsis()
                            .child(path.clone()),
                    )
                    .child(
                        ui::chrome_button(
                            "context-folder-remove",
                            "Remove saved folder reference",
                            Glyph::Close,
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.folder_references.retain(|saved| saved != &remove);
                                this.error = None;
                                this.status = None;
                                cx.notify();
                            }),
                        )
                        .id(("context-folder-remove", index))
                        .size(px(22.)),
                    )
            })
            .collect::<Vec<_>>();
        let modal = div().id("saved-context-dialog").role(gpui::Role::Dialog).aria_label("Saved task context")
            .track_focus(&self.focus).tab_group().tab_stop(true).relative().occlude().w_full().max_w(px(760.))
            .max_h(window.viewport_size().height - px(40.)).min_h_0().flex().flex_col().rounded(px(16.))
            .border_1().border_color(rgb(palette().border)).bg(rgb(palette().overlay)).shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.prefer_character_input || this.notes.read(cx).is_composing() || this.item.read(cx).is_composing() { return; }
                let modifiers = event.keystroke.modifiers;
                if event.keystroke.key == "escape" {
                    if this.confirm_discard || this.confirm_reload { this.confirm_discard = false; this.confirm_reload = false; cx.notify(); }
                    else { this.dismiss(cx); }
                    cx.stop_propagation();
                } else if event.keystroke.key == "s" && (modifiers.control || modifiers.platform) && !modifiers.alt {
                    this.save(cx); cx.stop_propagation();
                } else if event.keystroke.key == "tab" && !modifiers.control && !modifiers.platform && !modifiers.alt {
                    if modifiers.shift { window.focus_prev(cx); } else { window.focus_next(cx); }
                    cx.stop_propagation();
                }
            }))
            .child(ui::layout_probe("saved-context-dialog"))
            .child(div().p_4().flex().items_center().gap_2()
                .child(ui::icon(Glyph::Notebook))
                .child(div().flex_1().min_w_0().flex().flex_col().child("Notes, checklist & folders").child(div().text_size(px(12.)).text_ellipsis().text_color(rgb(palette().muted)).child(self.title.clone())))
                .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(if self.saving { "Saving..." } else if dirty { "Unsaved changes" } else { "Saved" }))
                .child(ui::chrome_button("context-close", "Close saved context", Glyph::Close, self.saving || self.picking_folders, cx.listener(|this, _: &(), _, cx| this.dismiss(cx)))))
            .children(self.error.clone().map(|error| div().px_4().py_2().text_color(rgb(palette().error)).child(error)))
            .children(self.status.clone().map(|status| div().px_4().py_1().text_size(px(12.)).text_color(rgb(palette().muted)).child(status)))
            .child(div().id("saved-context-scroll").px_4().min_h_0().overflow_y_scroll().flex().flex_col().gap_3()
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Private to this chat. Notes and saved folder paths are not sent to the agent unless you add them to the draft."))
                .when(self.loading, |el| el.child("Loading saved context..."))
                .when(available, |el| el
                    .child(div().relative().child(ui::layout_probe("context-notes-input")).child(self.notes.clone()))
                    .child(div().flex().items_center().gap_2()
                        .child(div().flex_1().child(format!("Checklist · {complete}/{} done", self.checklist.len())))
                        .child(ui::button("context-hide-done", if self.hide_done { "Show completed" } else { "Hide completed" }, self.hide_done)
                            .on_click(cx.listener(|this, _, _, cx| { this.hide_done = !this.hide_done; cx.notify(); }))))
                    .children(rows)
                    .child(div().flex().items_center().gap_2()
                        .child(div().flex_1().min_w_0().relative().child(ui::layout_probe("context-item-input")).child(self.item.clone()))
                        .child(ui::button("context-item-add", if self.edit_item.is_some() { "Update item" } else { "Add item" }, false)
                            .relative().child(ui::layout_probe("context-item-add"))
                            .on_click(cx.listener(|this, _, _, cx| { this.apply_item(cx); })))
                        .when(self.edit_item.is_some(), |el| el.child(ui::button("context-item-cancel", "Cancel", false).on_click(cx.listener(|this, _, _, cx| {
                            this.edit_item = None; this.item.update(cx, |entry, cx| entry.clear(cx)); cx.notify();
                        }))))))
                    .child(div().pt_2().flex().items_center().gap_2()
                        .child(div().flex_1().child("Saved folder references"))
                        .child(ui::button(
                            "context-choose-folders",
                            if self.picking_folders { "Choosing…" } else { "Choose folders" },
                            false,
                        )
                            .when(self.picking_folders || self.saving, |el| el.opacity(0.4))
                            .on_click(cx.listener(|this, _, _, cx| this.choose_folders(cx)))))
                    .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(
                        "Only selected path strings are saved. Folder contents are not read and these references do not grant filesystem access. Use Add to draft to include them in a prompt."
                    ))
                    .children(folder_rows))
            .child(div().p_4().flex().items_center().gap_2().border_t_1().border_color(rgb(palette().border))
                .child(ui::button("context-reload", "Reload saved", false)
                    .when(self.saving || self.loading || self.picking_folders, |el| el.opacity(0.4))
                    .on_click(cx.listener(|this, _, _, cx| this.reload(false, cx))))
                .child(div().flex_1())
                .child(ui::button("context-copy", "Copy", false).when(!available, |el| el.opacity(0.4)).on_click(cx.listener(|this, _, _, cx| {
                    if let Some(text) = this.context_text(cx) { cx.write_to_clipboard(gpui::ClipboardItem::new_string(text)); }
                })))
                .child(ui::button("context-to-draft", "Add to draft", false).relative().child(ui::layout_probe("context-to-draft"))
                    .when(!available, |el| el.opacity(0.4)).on_click(cx.listener(|this, _, _, cx| {
                        if let Some(text) = this.context_text(cx) { cx.emit(ContextEvent::Insert(text)); }
                    })))
                .child(ui::button("context-save", "Save context", true).relative().child(ui::layout_probe("context-save"))
                    .when(!available || self.saving || self.picking_folders, |el| el.opacity(0.4)).on_click(cx.listener(|this, _, _, cx| this.save(cx)))))
            .when(self.confirm_discard || self.confirm_reload, |el| el.child(div().p_3().bg(rgb(palette().notice_surface)).flex().items_center().gap_2()
                .child(div().flex_1().child("Discard the unsaved notes, checklist, and folder reference edits?"))
                .child(ui::button("context-keep", "Keep editing", false).relative().child(ui::layout_probe("context-keep")).on_click(cx.listener(|this, _, _, cx| { this.confirm_discard = false; this.confirm_reload = false; cx.notify(); })))
                .child(ui::button("context-discard", "Discard edits", false).relative().child(ui::layout_probe("context-discard")).on_click(cx.listener(|this, _, _, cx| {
                    if this.confirm_reload { this.reload(true, cx); } else { cx.emit(ContextEvent::Dismiss); }
                })))));
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
