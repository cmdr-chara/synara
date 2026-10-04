//! Hub task rows share the existing Kanban controller and task lifecycle.
use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Filter {
    #[default]
    All,
    Draft,
    Active,
    Attention,
    Finished,
}
#[derive(Default)]
pub(super) struct HubBoardState {
    filter: Filter,
    query: Option<Entity<TextEntry>>,
    subscription: Option<Subscription>,
    limit: usize,
    /// Presentation-only selection. The task remains owned by the catalog.
    inspected_task: Option<TaskId>,
}
fn accepts(filter: Filter, task: &Task, starting: bool) -> bool {
    match filter {
        Filter::All => true,
        Filter::Draft => execution_column(task, starting) == Some(0),
        Filter::Active => execution_column(task, starting) == Some(1),
        Filter::Attention => matches!(task.state, TaskState::Waiting | TaskState::Failed),
        Filter::Finished => execution_column(task, starting) == Some(2),
    }
}
fn is_hub_board_task(task: &Task, project: Option<ProjectId>) -> bool {
    task.scope == TaskScope::Studio
        && Some(task.project_id) == project
        && task.state != TaskState::Archived
}
fn selected_hub_task(
    tasks: &[Task],
    selected: Option<TaskId>,
    project: Option<ProjectId>,
) -> Option<&Task> {
    selected.and_then(|id| {
        tasks
            .iter()
            .find(|task| task.id == id && is_hub_board_task(task, project))
    })
}
fn task_status(task: &Task, starting: bool, stopping: bool) -> &'static str {
    if stopping {
        "Stopping"
    } else if starting && task.state == TaskState::Ready {
        "Starting"
    } else {
        match task.state {
            TaskState::Waiting => "Needs input",
            TaskState::Failed => "Failed",
            TaskState::Running => "Running",
            TaskState::Ready => "Draft",
            _ => "Finished",
        }
    }
}
impl Shell {
    pub(in crate::shell) fn open_hub_tasks(&mut self, project: ProjectId, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx)
            || !self.hubs.rows.iter().any(|h| h.profile.project == project)
        {
            return;
        }
        self.hubs.selected = Some(project);
        self.navigation.studio = true;
        self.kanban.project = Some(project);
        self.kanban.hub_view.filter = Filter::All;
        self.kanban.hub_view.limit = 60;
        self.kanban.hub_view.inspected_task = None;
        if let Some(query) = &self.kanban.hub_view.query {
            query.update(cx, |entry, cx| entry.clear(cx));
        }
        self.kanban.poll_failed = false;
        self.set_panel(Panel::Kanban, cx);
        self.focus_composer = false;
    }
    pub(super) fn in_hub_board(&self, task: &Task) -> bool {
        is_hub_board_task(task, self.hubs.selected)
    }
    pub(super) fn clear_hub_task_inspection(&mut self) {
        self.kanban.hub_view.inspected_task = None;
    }
    pub(super) fn reconcile_hub_task_inspection(&mut self) {
        if selected_hub_task(
            &self.catalog.tasks,
            self.kanban.hub_view.inspected_task,
            self.hubs.selected,
        )
        .is_none()
        {
            self.clear_hub_task_inspection();
        }
    }
    fn inspect_hub_task(&mut self, id: TaskId) {
        if self
            .catalog
            .tasks
            .iter()
            .any(|task| task.id == id && self.in_hub_board(task))
        {
            self.kanban.hub_view.inspected_task = Some(id);
        }
    }
    fn inspected_hub_task(&self) -> Option<&Task> {
        selected_hub_task(
            &self.catalog.tasks,
            self.kanban.hub_view.inspected_task,
            self.hubs.selected,
        )
    }
    pub(super) fn kanban_task_can_run(&self, task: &Task) -> bool {
        task.scope != TaskScope::Studio
            || self
                .hubs
                .rows
                .iter()
                .any(|hub| hub.profile.project == task.project_id)
    }
    fn find_hub_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.kanban.hub_view.query.is_none() {
            let input = cx.new(|cx| {
                TextEntry::new("Search task title or agent", EntryMode::SingleLine, 32., cx)
            });
            self.kanban.hub_view.subscription = Some(cx.subscribe(&input, |this, _, _, cx| {
                this.kanban.hub_view.limit = 60;
                cx.notify();
            }));
            self.kanban.hub_view.query = Some(input);
        }
        if let Some(query) = &self.kanban.hub_view.query {
            window.focus(&query.read(cx).focus_handle(cx), cx);
        }
        cx.notify();
    }
    fn hub_task_field(label: &'static str, value: String) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette().muted))
                    .child(label),
            )
            .child(div().text_size(px(13.)).text_ellipsis().child(value))
            .into_any_element()
    }
    fn hub_task_inspector(&self, task: &Task, cx: &mut Context<Self>) -> gpui::AnyElement {
        let id = task.id;
        let starting = self.busy.contains(&id)
            || self.connecting.contains(&id)
            || self.kanban.launching.contains_key(&id);
        let stopping = self.kanban.stopping.contains(&id);
        let group = execution_column(task, starting).unwrap_or(2);
        let agent = self
            .profiles
            .iter()
            .find(|profile| profile.id == task.agent_id)
            .map_or(task.agent_id.as_str(), |profile| profile.name.as_str())
            .to_owned();
        let hub = self
            .hubs
            .rows
            .iter()
            .find(|hub| Some(hub.profile.project) == self.hubs.selected)
            .map(|hub| hub.profile.name.clone())
            .unwrap_or_else(|| "Hub".into());
        let status = task_status(task, starting, stopping);
        let status_color = match task.state {
            TaskState::Waiting => palette().awaiting,
            TaskState::Failed => palette().error,
            TaskState::Running => palette().focus,
            _ => palette().text,
        };
        let open_id = SharedString::from(format!("hub-inspector-open-{id}"));
        let command_id = SharedString::from(format!("hub-inspector-command-{id}"));
        div()
            .id("hub-task-inspector")
            // Keep the inspector as a supplementary, edge-attached surface. It
            // follows the Kobra Sheet geometry without becoming modal: the task
            // list remains visible and selectable behind it.
            .role(gpui::Role::Dialog)
            .aria_label(format!("Task details for {}", task.title))
            .aria_description("Supplementary task details and actions")
            .absolute()
            .top(px(12.))
            .bottom(px(12.))
            .right(px(12.))
            .w(px(320.))
            .max_w_full()
            .min_h_0()
            .overflow_hidden()
            .rounded_xl()
            .border_1()
            .border_color(rgb(palette().border))
            .bg(rgb(palette().overlay))
            .shadow_lg()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.clear_hub_task_inspection();
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(
                div()
                    .relative()
                    .px_4()
                    .py_4()
                    .pr_12()
                    .border_b_1()
                    .border_color(rgb(palette().border))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_ellipsis()
                            .child(task.title.clone()),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child("Task details"),
                    )
                    .child(
                        ui::chrome_button(
                            "hub-task-inspector-close",
                            "Close task details",
                            Glyph::Close,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.clear_hub_task_inspection();
                                cx.notify();
                            }),
                        )
                        .absolute()
                        .top(px(12.))
                        .right(px(12.)),
                    ),
            )
            .child(
                div()
                    .id("hub-task-inspector-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(status_color))
                                    .child(status),
                            )
                            .child(div().flex_1())
                            .child(ui::icon(self.agent_glyph(&task.agent_id)).size(px(14.)))
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette().muted))
                                    .text_ellipsis()
                                    .child(agent),
                            ),
                    )
                    .child(Self::hub_task_field("Hub", hub))
                    .child(Self::hub_task_field(
                        "Working directory",
                        task.working_directory.display().to_string(),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(ui::action(
                                open_id,
                                "Open conversation",
                                Some(Glyph::Chat),
                                false,
                                cx.listener(move |this, _: &(), _, cx| {
                                    if this.select_task(id, cx) {
                                        this.show_conversation(cx);
                                    }
                                }),
                            ))
                            .children((group < 2).then(|| {
                                ui::action(
                            command_id,
                            if group == 0 {
                                "Run saved draft"
                            } else {
                                "Stop task"
                            },
                            Some(if group == 0 {
                                Glyph::Send
                            } else {
                                Glyph::Stop
                            }),
                            stopping,
                            cx.listener(move |this, _: &(), _, cx| {
                                if group == 0 {
                                    this.run_kanban_draft(id, cx);
                                } else {
                                    this.stop_kanban_task(id, cx);
                                }
                            }),
                        )
                            })),
                    ),
            )
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(rgb(palette().border))
                    .text_size(px(11.))
                    .text_color(rgb(palette().muted))
                    .child("Status follows the task lifecycle; actions use the existing conversation controls."),
            )
            .into_any_element()
    }
    pub(super) fn hub_task_board(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self
            .hubs
            .rows
            .iter()
            .any(|hub| Some(hub.profile.project) == self.hubs.selected)
        {
            return div()
                .p_4()
                .child("Select a Hub to see its tasks.")
                .into_any_element();
        }
        let state = &self.kanban.hub_view;
        let query = state
            .query
            .as_ref()
            .map_or("", |entry| entry.read(cx).text())
            .trim()
            .to_lowercase();
        let starting = |id: TaskId| {
            self.busy.contains(&id)
                || self.connecting.contains(&id)
                || self.kanban.launching.contains_key(&id)
        };
        let mut tasks: Vec<_> = self
            .catalog
            .tasks
            .iter()
            .filter(|task| self.in_hub_board(task))
            .collect();
        // Decisions before running work, then editable drafts and finished history.
        tasks.sort_by_key(|task| {
            (
                if matches!(task.state, TaskState::Waiting | TaskState::Failed) {
                    0
                } else if execution_column(task, starting(task.id)) == Some(1) {
                    1
                } else if task.state == TaskState::Ready {
                    2
                } else {
                    3
                },
                std::cmp::Reverse(task.updated_at_ms),
                task.id,
            )
        });
        let filtered: Vec<_> = tasks
            .iter()
            .copied()
            .filter(|task| {
                let agent = self
                    .profiles
                    .iter()
                    .find(|p| p.id == task.agent_id)
                    .map_or(task.agent_id.as_str(), |p| p.name.as_str());
                accepts(state.filter, task, starting(task.id))
                    && format!("{} {agent}", task.title)
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();
        let cap = state.limit.max(60);
        let inspected = self.inspected_hub_task();
        div()
            .id("hub-task-board")
            .relative()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(div().px_3().py_2().flex().items_center().flex_wrap().gap_2()
                .child(ui::action("hub-board-home", "Overview", Some(Glyph::Back), false,
                    cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Hubs, cx))))
                .child(ui::action("hub-board-new", "New task", Some(Glyph::Plus), false,
                    cx.listener(|this, _: &(), _, cx| this.open_task_dialog(true, cx))))
                .child(ui::chrome_button("hub-board-find", "Find tasks", Glyph::Search, false,
                    cx.listener(|this, _: &(), window, cx| this.find_hub_tasks(window, cx))))
                .child(ui::chrome_button("hub-board-refresh", "Refresh tasks", Glyph::Restore, self.kanban.polling,
                    cx.listener(|this, _: &(), _, cx| {
                        this.kanban.poll_failed = false;
                        this.load_hubs();
                        this.poll_kanban();
                        cx.notify();
                    }))))
            .children(state.query.clone().map(|entry| div().px_3().pb_2().child(entry)))
            .child(div().px_3().flex().items_center().flex_wrap().gap_1().border_b_1().border_color(rgb(palette().border))
                .children([(Filter::All, "All"), (Filter::Draft, "Drafts"), (Filter::Active, "Active"),
                    (Filter::Attention, "Needs attention"), (Filter::Finished, "Finished")]
                    .into_iter().enumerate().map(|(index, (filter, label))| {
                        let count = tasks.iter().filter(|task| accepts(filter, task, starting(task.id))).count();
                        ui::action(("hub-task-filter", index), format!("{label} {count}"), None, false,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.kanban.hub_view.filter = filter;
                                this.kanban.hub_view.limit = 60;
                                cx.notify();
                            })).text_size(px(12.)).rounded_none().bg(gpui::rgba(0))
                            .border_b_2().border_color(if filter == state.filter { rgb(palette().focus) } else { gpui::rgba(0) })
                    })))
             .child(
                 div()
                     .id("hub-task-content")
                     .flex_1()
                     .min_h_0()
                     .min_w_0()
                     .flex()
                     .gap_3()
                     .child(
                         div()
                             .id("hub-task-rows")
                             .flex_1()
                             .min_w_0()
                             .min_h_0()
                             .overflow_y_scroll()
                             .px_3()
                             .children(filtered.iter().take(cap).map(|task| {
                                 let id = task.id;
                                 let group = execution_column(task, starting(id)).unwrap_or(2);
                                 let status = task_status(
                                     task,
                                     starting(id),
                                     self.kanban.stopping.contains(&id),
                                 );
                                 let agent = self
                                     .profiles
                                     .iter()
                                     .find(|p| p.id == task.agent_id)
                                     .map_or(task.agent_id.as_str(), |p| p.name.as_str())
                                     .to_owned();
                                 div()
                                     .id(SharedString::from(format!("hub-task-row-{id}")))
                                     .py_2()
                                     .flex()
                                     .items_center()
                                     .gap_2()
                                     .border_b_1()
                                     .border_color(rgb(palette().border))
                                     .child(
                                         ui::action(
                                             SharedString::from(format!("hub-open-task-{id}")),
                                             task.title.clone(),
                                             Some(self.agent_glyph(&task.agent_id)),
                                             self.kanban.hub_view.inspected_task == Some(id),
                                             cx.listener(move |this, _: &(), _, cx| {
                                                 this.inspect_hub_task(id);
                                                 cx.notify();
                                             }),
                                         )
                                         .flex_1()
                                         .min_w_0()
                                         .h_auto()
                                         .flex_wrap()
                                         .rounded_none()
                                         .bg(gpui::rgba(0))
                                         .aria_label(format!("Inspect task: {}", task.title))
                                         .aria_selected(
                                             self.kanban.hub_view.inspected_task == Some(id),
                                         )
                                         .child(
                                             div()
                                                 .w_full()
                                                 .pl_6()
                                                 .text_size(px(11.))
                                                 .text_color(rgb(palette().muted))
                                                 .child(format!("{agent} · {status}")),
                                         ),
                                     )
                                     .child(self.thread_pin_button(task, cx))
                                     .children((group < 2).then(|| {
                                         let stopping = self.kanban.stopping.contains(&id);
                                         let label = if group == 0 {
                                             "Run saved draft"
                                         } else {
                                             "Stop task"
                                         };
                                         let glyph = if group == 0 {
                                             Glyph::Send
                                         } else {
                                             Glyph::Stop
                                         };
                                         ui::button_shell(
                                             SharedString::from(format!("hub-task-action-{id}")),
                                             label,
                                             stopping,
                                         )
                                         .size(px(30.))
                                         .p_0()
                                         .bg(gpui::rgba(0))
                                         .flex()
                                         .items_center()
                                         .justify_center()
                                         .when(stopping, |el| {
                                             el.opacity(0.4).cursor_default()
                                         })
                                         .child(ui::icon(glyph).size(px(14.)))
                                         .on_click(cx.listener(move |this, _, _, cx| {
                                             if stopping {
                                                 return;
                                             }
                                             if group == 0 {
                                                 this.run_kanban_draft(id, cx);
                                             } else {
                                                 this.stop_kanban_task(id, cx);
                                             }
                                             cx.stop_propagation();
                                         }))
                                     }))
                             }))
                             .children(
                                 filtered
                                     .is_empty()
                                     .then(|| {
                                         div()
                                             .p_4()
                                             .text_color(rgb(palette().muted))
                                             .child("No matching tasks. Create a draft or adjust the filters.")
                                     }),
                             )
                             .children((filtered.len() > cap).then(|| {
                                 ui::action(
                                     "hub-more-tasks",
                                     "Show more tasks",
                                     None,
                                     false,
                                     cx.listener(move |this, _: &(), _, cx| {
                                         this.kanban.hub_view.limit = cap + 60;
                                         cx.notify();
                                     }),
                                 )
                             })),
                     )
                      ,
              )
              .children(inspected.map(|task| self.hub_task_inspector(task, cx)))
            .child(div().px_4().py_2().border_t_1().border_color(rgb(palette().border)).text_size(px(11.))
                .text_color(rgb(palette().muted)).child("Task state comes from the agent lifecycle. Run is explicit, and opening a task never approves its requests."))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task(scope: TaskScope, state: TaskState) -> Task {
        Task {
            id: TaskId::new(),
            project_id: ProjectId::new(),
            title: "Task".into(),
            state,
            thread_id: ThreadId::new(),
            agent_id: "fixture".into(),
            working_directory: PathBuf::from("/tmp"),
            updated_at_ms: 0,
            scope,
        }
    }
    #[test]
    fn hub_tasks_use_real_lifecycle_without_leaking_into_the_normal_board() {
        let draft = task(TaskScope::Studio, TaskState::Ready);
        assert_eq!(column(&draft, false), None);
        assert_eq!(execution_column(&draft, false), Some(0));
        assert!(accepts(Filter::Active, &draft, true));
        assert!(!accepts(Filter::Draft, &draft, true));
        assert!(accepts(
            Filter::Attention,
            &task(TaskScope::Studio, TaskState::Waiting),
            false
        ));
        assert!(accepts(
            Filter::Attention,
            &task(TaskScope::Studio, TaskState::Failed),
            false
        ));
        assert_eq!(
            execution_column(&task(TaskScope::Studio, TaskState::Archived), true),
            None
        );
        assert_eq!(
            column(&task(TaskScope::Chat, TaskState::Ready), false),
            Some(0)
        );
    }
    #[test]
    fn inspector_selection_uses_a_catalog_task_id_and_rejects_stale_rows() {
        let project = ProjectId::new();
        let mut selected = task(TaskScope::Studio, TaskState::Ready);
        selected.project_id = project;
        let selected_id = selected.id;
        let mut archived = selected.clone();
        archived.state = TaskState::Archived;
        let mut other_project = selected.clone();
        other_project.project_id = ProjectId::new();
        assert_eq!(
            selected_hub_task(&[selected.clone()], Some(selected_id), Some(project))
                .map(|task| task.id),
            Some(selected_id)
        );
        assert!(selected_hub_task(&[archived], Some(selected_id), Some(project)).is_none());
        assert!(selected_hub_task(&[other_project], Some(selected_id), Some(project)).is_none());
        assert!(selected_hub_task(&[selected], None, Some(project)).is_none());
    }
    #[test]
    fn inspector_status_reuses_runtime_flags_without_inventing_task_state() {
        assert_eq!(
            task_status(&task(TaskScope::Studio, TaskState::Ready), true, false),
            "Starting"
        );
        assert_eq!(
            task_status(&task(TaskScope::Studio, TaskState::Running), true, true),
            "Stopping"
        );
        assert_eq!(
            task_status(&task(TaskScope::Studio, TaskState::Waiting), false, false),
            "Needs input"
        );
        assert_eq!(
            task_status(&task(TaskScope::Studio, TaskState::Completed), false, false),
            "Finished"
        );
    }
}
