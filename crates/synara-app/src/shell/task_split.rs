//! Presentation-only two-task split. The existing secondary view, shared draft
//! store and Controller remain the sole owners. Layout is intentionally transient.
use super::*;
use crate::ui::{self, palette};

fn wide_split(width: f32) -> bool {
    width.is_finite() && width >= 840.
}
fn selectable(primary: Option<TaskId>, task: &Task) -> bool {
    primary != Some(task.id) && task.state != TaskState::Archived
}
impl Shell {
    pub(super) fn task_split_action(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_shrink_0()
            .justify_end()
            .px_3()
            .py_1()
            .child(
                ui::action(
                    "task-split-open",
                    if self.side_chats.split {
                        "Choose second task"
                    } else {
                        "View two tasks"
                    },
                    None,
                    self.selected.is_none() || self.loading_task.is_some(),
                    cx.listener(|this, _: &(), _, cx| this.open_task_split(cx)),
                )
                .relative()
                .child(ui::layout_probe("task-split-open")),
            )
            .into_any_element()
    }
    fn open_task_split(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_none()
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.side_chats.pending(cx)
            || self.revision_navigation_blocked(cx)
            || self.hub_navigation_blocked(cx)
        {
            return;
        }
        self.remember_side_draft(cx);
        self.set_panel(Panel::Conversation, cx);
        if self.panel != Panel::Conversation {
            return;
        }
        let state = &mut self.side_chats;
        state.generation = state.generation.wrapping_add(1);
        state.loading = false;
        state.split = true;
        state.split_picker = true;
        state.split_secondary = true;
        state.parent = self.selected;
        if state.selected == self.selected {
            state.selected = None;
            state.thread = None;
        }
        state.composer.update(cx, |entry, _| {
            entry.set_send_on_enter(self.settings.value.chat.send_on_enter)
        });
        self.focus_composer = false;
        cx.notify();
    }
    fn select_split_task(&mut self, child: TaskId, cx: &mut Context<Self>) {
        let Some(parent) = self.selected else { return };
        if !self.side_chats.split
            || self.side_chats.loading
            || self.side_chats.pending(cx)
            || !self
                .catalog
                .tasks
                .iter()
                .any(|task| task.id == child && selectable(Some(parent), task))
        {
            return;
        }
        self.remember_side_draft(cx);
        self.side_chats.parent = Some(parent);
        self.side_chats.generation = self.side_chats.generation.wrapping_add(1);
        self.side_chats.error = None;
        self.load_side_thread(parent, child, self.side_chats.generation, false, cx);
        self.focus_composer = false;
    }
    fn close_task_split(&mut self, cx: &mut Context<Self>) {
        if self.side_chats.pending(cx) {
            return;
        }
        self.remember_side_draft(cx);
        self.side_chats.split = false;
        self.side_chats.split_picker = false;
        self.side_chats.parent = None;
        if let Some(parent) = self.selected {
            self.load_side_chats(parent, cx);
        }
        self.focus_composer = true;
        cx.notify();
    }
    pub(super) fn task_split_surface(
        &self,
        window: &Window,
        width: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let wide = wide_split(width);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(palette().border))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(11.))
                            .text_color(rgb(palette().muted))
                            .child(
                                "Two tasks · drafts saved independently · layout closes on restart",
                            ),
                    )
                    .when(!wide, |row| {
                        row.child(
                            ui::action(
                                "task-split-switch",
                                if self.side_chats.split_secondary {
                                    "Show first task"
                                } else {
                                    "Show second task"
                                },
                                None,
                                false,
                                cx.listener(|this, _: &(), _, cx| {
                                    this.side_chats.split_secondary =
                                        !this.side_chats.split_secondary;
                                    this.focus_composer = !this.side_chats.split_secondary;
                                    cx.notify();
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe("task-split-switch")),
                        )
                    })
                    .child(
                        ui::action(
                            "task-split-close",
                            "Close split",
                            None,
                            self.side_chats.pending(cx),
                            cx.listener(|this, _: &(), _, cx| this.close_task_split(cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("task-split-close")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .when(wide || !self.side_chats.split_secondary, |row| {
                        row.child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .overflow_hidden()
                                .relative()
                                .child(ui::layout_probe("task-split-primary"))
                                .child(self.conversation(window, cx)),
                        )
                    })
                    .when(wide || self.side_chats.split_secondary, |row| {
                        row.child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .overflow_hidden()
                                .border_l_1()
                                .border_color(rgb(palette().border))
                                .relative()
                                .child(ui::layout_probe("task-split-secondary"))
                                .child(self.split_task_panel(cx)),
                        )
                    }),
            )
            .into_any_element()
    }
    fn split_task_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let state = &self.side_chats;
        let row = state
            .selected
            .and_then(|id| self.catalog.tasks.iter().find(|t| t.id == id));
        let busy = state
            .selected
            .is_some_and(|id| self.busy.contains(&id) || self.connecting.contains(&id));
        let mut pane = div().flex().flex_col().flex_1().min_w_0().min_h_0().child(
            div()
                .flex()
                .items_center()
                .flex_shrink_0()
                .gap_2()
                .px_3()
                .py_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.))
                        .child(row.map_or_else(
                            || "Choose a second task".into(),
                            |task| {
                                let project = self
                                    .catalog
                                    .projects
                                    .iter()
                                    .find(|p| p.id == task.project_id)
                                    .map_or("Unknown project", |p| p.name.as_str());
                                format!("{} · {} · {:?}", project, task.title, task.state)
                            },
                        )),
                )
                .child(
                    ui::action(
                        "task-split-replace",
                        "Choose task",
                        None,
                        state.loading,
                        cx.listener(|this, _: &(), _, cx| {
                            if !this.side_chats.pending(cx) {
                                this.side_chats.split_picker = !this.side_chats.split_picker;
                                cx.notify();
                            }
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("task-split-replace")),
                ),
        );
        if state.split_picker {
            let mut tasks: Vec<_> = self
                .catalog
                .tasks
                .iter()
                .filter(|t| selectable(self.selected, t))
                .collect();
            tasks.sort_by_key(|t| (std::cmp::Reverse(t.updated_at_ms), t.id));
            pane = pane.child(
                div()
                    .id("task-split-picker")
                    .max_h(px(220.))
                    .overflow_y_scroll()
                    .flex_shrink_0()
                    .px_3()
                    .child(div().text_size(px(11.)).child(
                        "Existing tasks from any project. Selecting never starts a session.",
                    ))
                    .children(tasks.iter().enumerate().map(|(index, task)| {
                        let id = task.id;
                        ui::action(
                            ("task-split-choice", index),
                            format!("{} · {}", task.title, task.project_id),
                            None,
                            state.loading,
                            cx.listener(move |this, _: &(), _, cx| this.select_split_task(id, cx)),
                        )
                        .relative()
                        .child(ui::layout_probe_slot("task-split-choice", index))
                    })),
            );
        }
        if let Some(error) = &state.error {
            pane = pane.child(
                div()
                    .px_3()
                    .text_size(px(12.))
                    .text_color(rgb(palette().error))
                    .child(error.clone()),
            );
        }
        if let Some(thread) = &state.thread {
            let start = thread.timeline.len().saturating_sub(200);
            pane = pane.child(div().id("split-task-history").flex_1().min_h_0().min_w_0().overflow_y_scroll().px_3().py_2().flex().flex_col().gap_3()
                .when(start > 0, |v| v.child(div().text_size(px(11.)).child("Latest 200 items. Open this task as the first conversation for complete history.")))
                .children((start..thread.timeline.len()).map(|index| self.side_chat_item(thread, index, cx))));
        } else {
            pane = pane.child(div().flex_1().min_h_0().p_3().child(if state.loading {
                "Loading task..."
            } else {
                "No second task selected."
            }));
        }
        let target = state.selected;
        let editable = !state.loading && row.is_some_and(|t| selectable(self.selected, t));
        let can_send = !state.loading
            && row.is_some_and(|t| selectable(self.selected, t))
            && !state.composer.read(cx).text().trim().is_empty();
        pane.child(
            div()
                .flex_shrink_0()
                .px_3()
                .py_2()
                .border_t_1()
                .border_color(rgb(palette().border))
                .when(editable, |v| {
                    v.child(
                        div()
                            .relative()
                            .child(state.composer.clone())
                            .child(ui::layout_probe("task-split-composer")),
                    )
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(11.))
                                .text_color(rgb(palette().muted))
                                .child("Text composer · attachments and full tools in first view"),
                        )
                        .child(
                            ui::action(
                                "task-split-send",
                                if busy {
                                    "Stop second task"
                                } else {
                                    "Send to second task"
                                },
                                None,
                                !busy && !can_send,
                                cx.listener(move |this, _: &(), _, cx| {
                                    if !this.side_chats.split || this.side_chats.selected != target
                                    {
                                        return;
                                    }
                                    if busy {
                                        this.cancel_side_prompt(cx);
                                    } else {
                                        this.send_side_prompt(cx);
                                    }
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe("task-split-send")),
                        ),
                ),
        )
        .into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_split_width_boundary_is_deterministic_and_invalid_sizes_fall_back() {
        assert!(wide_split(840.));
        assert!(wide_split(1600.));
        for width in [839., 0., -1., f32::NAN, f32::INFINITY] {
            assert!(!wide_split(width));
        }
    }
}
