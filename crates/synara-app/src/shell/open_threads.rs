//! Native horizontal tabs for the selected conversations.
//!
//! The persisted value is an ordered list of task identities. Everything shown
//! in the strip is resolved from the live catalog so renamed tasks and changed
//! provider profiles never leave stale presentation data behind.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::{FocusHandle, MouseButton, SharedString};

#[derive(Default)]
pub(super) struct SaveState {
    dirty: bool,
    saving: bool,
    failed: bool,
}

impl SaveState {
    fn changed(&mut self) {
        self.dirty = true;
        self.failed = false;
    }

    fn start(&mut self, force: bool) -> bool {
        if !self.dirty || self.saving || (self.failed && !force) {
            return false;
        }
        self.dirty = false;
        self.saving = true;
        true
    }

    fn finish(&mut self, failed: bool) {
        self.saving = false;
        self.failed = failed;
        self.dirty |= failed;
    }

    fn pending(&self) -> bool {
        self.dirty || self.saving
    }
}

#[derive(Clone, Copy)]
pub(super) struct PendingClose {
    pub task: TaskId,
    pub revision: u64,
}

pub(super) fn pending_close_matches(
    pending: Option<PendingClose>,
    selected: Option<TaskId>,
    revision: u64,
) -> bool {
    pending.is_some_and(|pending| selected == Some(pending.task) && revision == pending.revision)
}

pub(super) struct OpenThreadState {
    pub value: OpenThreadTabs,
    pub recovery: Option<String>,
    pub save: SaveState,
    pub quitting: bool,
    pub pending_close: Option<PendingClose>,
    pub focus: HashMap<TaskId, FocusHandle>,
}

impl OpenThreadState {
    pub(super) fn new(loaded: LoadedOpenThreadTabs, cx: &mut Context<Shell>) -> Self {
        let value = loaded.tabs;
        let focus = value
            .task_ids
            .iter()
            .copied()
            .map(|task| (task, cx.focus_handle()))
            .collect();
        Self {
            value,
            recovery: loaded.recovery,
            save: SaveState::default(),
            quitting: false,
            pending_close: None,
            focus,
        }
    }

    fn ensure_focus(&mut self, task: TaskId, cx: &mut Context<Shell>) {
        self.focus.entry(task).or_insert_with(|| cx.focus_handle());
    }
}

impl Shell {
    fn live_open_thread_ids(&self) -> Vec<TaskId> {
        self.open_threads
            .value
            .task_ids
            .iter()
            .copied()
            .filter(|id| {
                self.catalog
                    .tasks
                    .iter()
                    .any(|task| task.id == *id && task.state != TaskState::Archived)
            })
            .collect()
    }

    pub(super) fn open_thread_tab_count(&self) -> usize {
        self.live_open_thread_ids().len()
    }

    /// Catalog replacement is an explicit state transition. Render only reads
    /// this filtered list and never repairs persisted navigation state.
    pub(super) fn reconcile_open_thread_tabs(&mut self, cx: &mut Context<Self>) {
        let changed = self.open_threads.value.prune(|id| {
            self.catalog
                .tasks
                .iter()
                .any(|task| task.id == id && task.state != TaskState::Archived)
        });
        if changed {
            self.open_threads.save.changed();
            self.flush_open_thread_tabs(false);
        }
        if let Some(selected) = self.selected {
            self.open_threads.ensure_focus(selected, cx);
        }
    }

    pub(super) fn record_open_thread_tab(&mut self, task: TaskId, cx: &mut Context<Self>) {
        let protected = self.selected;
        let before = self.open_threads.value.clone();
        let evicted = self.open_threads.value.open(task, protected);
        if before == self.open_threads.value {
            self.open_threads.ensure_focus(task, cx);
            return;
        }
        self.open_threads.ensure_focus(task, cx);
        if let Some(evicted) = evicted {
            // Retiring the focus handle is safe only after navigation has left
            // the evicted task. Keep it in the map until the next explicit
            // focus move, avoiding a render-time focus side effect.
            if self.selected != Some(evicted) {
                self.open_threads.focus.remove(&evicted);
            }
        }
        self.open_threads.save.changed();
        self.flush_open_thread_tabs(false);
    }

    pub(super) fn close_open_thread_tab_identity(
        &mut self,
        task: TaskId,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.open_threads.value.close(task) {
            return false;
        }
        self.open_threads.focus.remove(&task);
        self.open_threads.save.changed();
        self.flush_open_thread_tabs(false);
        cx.notify();
        true
    }

    pub(super) fn flush_open_thread_tabs(&mut self, force: bool) {
        if self.open_threads.recovery.is_some() || !self.open_threads.save.start(force) {
            return;
        }
        let value = self.open_threads.value.clone();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::OpenThreadTabsSaved(
                workspace
                    .save_open_thread_tabs(value)
                    .await
                    .err()
                    .map(|error| error.to_string()),
            ))
        });
    }

    pub(super) fn open_thread_tabs_saved(&mut self, error: Option<String>, cx: &mut Context<Self>) {
        self.open_threads.save.finish(error.is_some());
        if let Some(error) = error {
            self.error = Some(format!(
                "Open conversation tabs could not be saved: {error}. Your conversations are still open."
            ));
            self.open_threads.quitting = false;
            self.close.cancel();
        } else if self.open_threads.quitting {
            self.flush_open_thread_tabs(true);
            if !self.open_threads.save.pending() {
                self.open_threads.quitting = false;
                self.begin_quit(cx);
            }
        }
        cx.notify();
    }

    pub(super) fn save_open_thread_tabs_before_quit(&mut self) -> bool {
        if self.open_threads.recovery.is_some() || !self.open_threads.save.pending() {
            return false;
        }
        self.open_threads.quitting = true;
        self.flush_open_thread_tabs(true);
        true
    }

    pub(super) fn open_thread_tabs_strip(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let tabs = self
            .live_open_thread_ids()
            .into_iter()
            .filter_map(|id| {
                let task = self.catalog.tasks.iter().find(|task| task.id == id)?;
                Some((id, task.title.clone(), self.agent_glyph(&task.agent_id)))
            })
            .collect::<Vec<_>>();
        let active = self.selected;
        div()
            .id("open-thread-tabs")
            .role(gpui::Role::TabList)
            .aria_label("Open conversations")
            .aria_description(
                "Left and Right select conversations. Delete closes the selected conversation tab.",
            )
            .flex()
            .items_center()
            .gap_1()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(
                div()
                    .id("open-thread-tabs-scroll")
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_x_scroll()
                    .children(tabs.into_iter().map(|(id, title, glyph)| {
                        let selected = active == Some(id);
                        let tab_id = SharedString::from(format!("open-thread-tab-{id}"));
                        let close_id = SharedString::from(format!("open-thread-close-{id}"));
                        let close_label = SharedString::from(format!("Close {title}"));
                        let focus = self
                            .open_threads
                            .focus
                            .get(&id)
                            .expect("open conversation tabs have focus handles");
                        div()
                            .id(SharedString::from(format!("open-thread-tab-group-{id}")))
                            .flex()
                            .items_center()
                            .flex_shrink_0()
                            .h(px(32.))
                            .rounded_md()
                            .bg(if selected {
                                rgb(palette().selected)
                            } else {
                                rgb(palette().canvas)
                            })
                            .child(
                                ui::button_shell(tab_id, title.clone(), selected)
                                    .role(gpui::Role::Tab)
                                    .aria_label(title.clone())
                                    .aria_selected(selected)
                                    .aria_description(
                                        "Left and Right select. Delete closes this conversation.",
                                    )
                                    .track_focus(focus)
                                    .tab_stop(selected)
                                    .h(px(30.))
                                    .min_w(px(112.))
                                    .max_w(px(260.))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .when(!selected, |el| el.bg(gpui::rgba(0)))
                                    .child(ui::icon(glyph).size(px(14.)))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_ellipsis()
                                            .text_size(px(12.))
                                            .child(title.clone()),
                                    )
                                    .on_key_down(cx.listener(move |this, event, window, cx| {
                                        this.open_thread_tab_key(id, event, window, cx);
                                    }))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.select_open_thread_tab(id, window, cx);
                                    }))
                                    .on_mouse_down(
                                        MouseButton::Middle,
                                        cx.listener(move |this, _, window, cx| {
                                            this.close_open_thread_tab(id, window, cx);
                                            cx.stop_propagation();
                                        }),
                                    ),
                            )
                            .child(
                                ui::button_shell(close_id, close_label.clone(), false)
                                    .aria_label(close_label)
                                    .aria_description("Close conversation")
                                    .tab_stop(selected)
                                    .size(px(22.))
                                    .p_0()
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(ui::icon(Glyph::Close).size(px(13.)))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.close_open_thread_tab(id, window, cx);
                                        cx.stop_propagation();
                                    })),
                            )
                    })),
            )
            .child(
                ui::chrome_button(
                    "open-thread-new",
                    "New conversation",
                    Glyph::Plus,
                    self.creating_task,
                    cx.listener(|this, _, _, cx| this.start_new_chat(cx)),
                )
                .size(px(28.)),
            )
            .into_any_element()
    }

    fn select_open_thread_tab(
        &mut self,
        task: TaskId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.close != CloseState::Open
            || !self.open_threads.value.contains(task)
            || !self.select_task(task, cx)
        {
            return;
        }
        self.show_conversation(cx);
        self.focus_composer = false;
        if let Some(focus) = self.open_threads.focus.get(&task) {
            window.focus(focus, cx);
        }
    }

    fn close_open_thread_tab(&mut self, task: TaskId, window: &mut Window, cx: &mut Context<Self>) {
        if self.close != CloseState::Open || self.open_threads.quitting {
            return;
        }
        if !self.open_threads.value.contains(task) {
            self.reconcile_open_thread_tabs(cx);
            return;
        }
        if self.selected != Some(task) {
            self.close_open_thread_tab_identity(task, cx);
            if let Some(active) = self.selected
                && let Some(focus) = self.open_threads.focus.get(&active)
            {
                window.focus(focus, cx);
            }
            return;
        }

        let successor = self.open_threads.value.successor_after_close(task);
        if let Some(successor) = successor {
            let revision = self.selection_revision;
            if self.select_task(successor, cx) && self.selection_revision != revision {
                self.show_conversation(cx);
                self.focus_composer = false;
                self.close_open_thread_tab_identity(task, cx);
                if let Some(focus) = self.open_threads.focus.get(&successor) {
                    window.focus(focus, cx);
                }
            }
            return;
        }

        let Some(current) = self.task().cloned() else {
            return;
        };
        let revision = self.selection_revision;
        let was_creating = self.creating_task;
        // The final-tab replacement is navigation too: preserve the current
        // composer before the new task is created, just like select_task.
        self.snapshot_draft(cx);
        if current.scope == TaskScope::Studio {
            // Hubs use their own guarded creation/reload pipeline; the
            // pending-close token is consumed when that pipeline selects the
            // new task, not when the background create completes.
            self.new_hub_thread(cx);
        } else {
            self.create_chat(current.scope, cx);
        }
        if !was_creating
            && self.creating_task
            && self.selected == Some(task)
            && self.selection_revision == revision
        {
            self.open_threads.pending_close = Some(PendingClose { task, revision });
        } else if current.state == TaskState::Ready
            && self
                .thread
                .as_ref()
                .is_some_and(|thread| thread.turns.is_empty() && thread.messages.is_empty())
            && self.composer.read(cx).text().is_empty()
        {
            // A lone untouched draft is already the fresh draft requested by
            // the close action. Retain it rather than creating a duplicate.
            self.focus_composer = true;
            cx.notify();
        }
    }

    fn open_thread_tab_key(
        &mut self,
        task: TaskId,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control
            || modifiers.platform
            || modifiers.alt
            || modifiers.shift
            || event.prefer_character_input
        {
            return;
        }
        if event.keystroke.key == "delete" {
            if !event.is_held {
                self.close_open_thread_tab(task, window, cx);
            }
            cx.stop_propagation();
            return;
        }
        let tabs = self.live_open_thread_ids();
        let Some(index) = tabs.iter().position(|candidate| *candidate == task) else {
            return;
        };
        let next = match event.keystroke.key.as_str() {
            "left" => (index + tabs.len() - 1) % tabs.len(),
            "right" => (index + 1) % tabs.len(),
            "home" => 0,
            "end" => tabs.len() - 1,
            _ => return,
        };
        self.select_open_thread_tab(tabs[next], window, cx);
        cx.stop_propagation();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_close_target_is_guarded_by_task_and_selection_revision() {
        let task = TaskId::new();
        let pending = Some(PendingClose { task, revision: 7 });
        assert!(pending_close_matches(pending, Some(task), 7));
        assert!(!pending_close_matches(pending, Some(task), 8));
        assert!(!pending_close_matches(pending, Some(TaskId::new()), 7));
        assert!(!pending_close_matches(None, Some(task), 7));
    }
}
