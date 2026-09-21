//! Hub task rows share the existing Kanban controller and task lifecycle.
use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Filter { #[default] All, Draft, Active, Attention, Finished }
#[derive(Default)]
pub(super) struct HubBoardState {
    filter: Filter,
    query: Option<Entity<TextEntry>>,
    subscription: Option<Subscription>,
    limit: usize,
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
impl Shell {
    pub(in crate::shell) fn open_hub_tasks(&mut self, project: ProjectId, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) || !self.hubs.rows.iter().any(|h| h.profile.project == project) { return; }
        self.hubs.selected = Some(project);
        self.navigation.studio = true;
        self.kanban.project = Some(project);
        self.kanban.hub_view.filter = Filter::All;
        self.kanban.hub_view.limit = 60;
        if let Some(query) = &self.kanban.hub_view.query { query.update(cx, |entry, cx| entry.clear(cx)); }
        self.kanban.poll_failed = false;
        self.set_panel(Panel::Kanban, cx);
        self.focus_composer = false;
    }
    pub(super) fn in_hub_board(&self, task: &Task) -> bool {
        task.scope == TaskScope::Studio && Some(task.project_id) == self.hubs.selected
            && task.state != TaskState::Archived
    }
    pub(super) fn kanban_task_can_run(&self, task: &Task) -> bool {
        task.scope != TaskScope::Studio || self.hubs.rows.iter().any(|hub|
            hub.profile.project == task.project_id && !hub.profile.archived)
    }
    fn find_hub_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.kanban.hub_view.query.is_none() {
            let input = cx.new(|cx| TextEntry::new("Search task title or agent", EntryMode::SingleLine, 32., cx));
            self.kanban.hub_view.subscription = Some(cx.subscribe(&input, |this, _, _, cx| {
                this.kanban.hub_view.limit = 60;
                cx.notify();
            }));
            self.kanban.hub_view.query = Some(input);
        }
        if let Some(query) = &self.kanban.hub_view.query { window.focus(&query.read(cx).focus_handle(cx), cx); }
        cx.notify();
    }
    pub(super) fn hub_task_board(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(profile) = self.hubs.rows.iter().find(|hub| Some(hub.profile.project) == self.hubs.selected)
            .map(|hub| &hub.profile) else {
            return div().p_4().child("Select a Hub to see its tasks.").into_any_element();
        };
        let state = &self.kanban.hub_view;
        let query = state.query.as_ref().map_or("", |entry| entry.read(cx).text()).trim().to_lowercase();
        let starting = |id: TaskId| self.busy.contains(&id) || self.connecting.contains(&id) || self.kanban.launching.contains_key(&id);
        let mut tasks: Vec<_> = self.catalog.tasks.iter().filter(|task| self.in_hub_board(task)).collect();
        // Decisions before running work, then editable drafts and finished history.
        tasks.sort_by_key(|task| (
            if matches!(task.state, TaskState::Waiting | TaskState::Failed) { 0 }
            else if execution_column(task, starting(task.id)) == Some(1) { 1 }
            else if task.state == TaskState::Ready { 2 } else { 3 },
            std::cmp::Reverse(task.updated_at_ms), task.id,
        ));
        let filtered: Vec<_> = tasks.iter().copied().filter(|task| {
            let agent = self.profiles.iter().find(|p| p.id == task.agent_id).map_or(task.agent_id.as_str(), |p| p.name.as_str());
            accepts(state.filter, task, starting(task.id))
                && format!("{} {agent}", task.title).to_lowercase().contains(&query)
        }).collect();
        let cap = state.limit.max(60);
        div().id("hub-task-board").flex_1().min_h_0().min_w_0().flex().flex_col()
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
            .children(profile.archived.then(|| div().px_4().py_2().text_size(px(12.))
                .child("Archived Hub. Existing work remains visible. Restore it to create or run drafts.")))
            .child(div().id("hub-task-rows").flex_1().min_h_0().overflow_y_scroll().px_3()
                .children(filtered.iter().take(cap).map(|task| {
                    let id = task.id;
                    let group = execution_column(task, starting(id)).unwrap_or(2);
                    let status = if self.kanban.stopping.contains(&id) { "Stopping" }
                        else if self.kanban.launching.contains_key(&id) || starting(id) && task.state == TaskState::Ready { "Starting" }
                        else { match task.state {
                            TaskState::Waiting => "Needs input", TaskState::Failed => "Failed",
                            TaskState::Running => "Running", TaskState::Ready => "Draft", _ => "Finished",
                        }};
                    let agent = self.profiles.iter().find(|p| p.id == task.agent_id)
                        .map_or(task.agent_id.as_str(), |p| p.name.as_str()).to_owned();
                    div().id(SharedString::from(format!("hub-task-row-{id}"))).py_2().flex().items_center().gap_2()
                        .border_b_1().border_color(rgb(palette().border))
                        .child(ui::action(SharedString::from(format!("hub-open-task-{id}")), task.title.clone(),
                            Some(self.agent_glyph(&task.agent_id)), false,
                            cx.listener(move |this, _: &(), _, cx| { if this.select_task(id, cx) { this.show_conversation(cx); } }))
                            .flex_1().min_w_0().h_auto().flex_wrap().rounded_none().bg(gpui::rgba(0))
                            .child(div().w_full().pl_6().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{agent} · {status}"))))
                        .child(self.thread_pin_button(task, cx))
                        .children((group < 2).then(|| ui::chrome_button(
                            "hub-task-action",
                            if group == 0 { "Run saved draft" } else { "Stop task" },
                            if group == 0 { Glyph::Send } else { Glyph::Stop },
                            group == 0 && profile.archived || self.kanban.stopping.contains(&id),
                            cx.listener(move |this, _: &(), _, cx| {
                                if group == 0 { this.run_kanban_draft(id, cx); } else { this.stop_kanban_task(id, cx); }
                            }))))
                }))
                .children(filtered.is_empty().then(|| div().p_4().text_color(rgb(palette().muted))
                    .child("No matching tasks. Create a draft or adjust the filters.")))
                .children((filtered.len() > cap).then(|| ui::action("hub-more-tasks", "Show more tasks", None, false,
                    cx.listener(move |this, _: &(), _, cx| { this.kanban.hub_view.limit = cap + 60; cx.notify(); })))) )
            .child(div().px_4().py_2().border_t_1().border_color(rgb(palette().border)).text_size(px(11.))
                .text_color(rgb(palette().muted)).child("Task state comes from the agent lifecycle. Run is explicit, and opening a task never approves its requests."))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task(scope: TaskScope, state: TaskState) -> Task {
        Task { id: TaskId::new(), project_id: ProjectId::new(), title: "Task".into(), state,
            thread_id: ThreadId::new(), agent_id: "fixture".into(), working_directory: PathBuf::from("/tmp"), updated_at_ms: 0, scope }
    }
    #[test]
    fn hub_tasks_use_real_lifecycle_without_leaking_into_the_normal_board() {
        let draft = task(TaskScope::Studio, TaskState::Ready);
        assert_eq!(column(&draft, false), None);
        assert_eq!(execution_column(&draft, false), Some(0));
        assert!(accepts(Filter::Active, &draft, true));
        assert!(!accepts(Filter::Draft, &draft, true));
        assert!(accepts(Filter::Attention, &task(TaskScope::Studio, TaskState::Waiting), false));
        assert!(accepts(Filter::Attention, &task(TaskScope::Studio, TaskState::Failed), false));
        assert_eq!(execution_column(&task(TaskScope::Studio, TaskState::Archived), true), None);
        assert_eq!(column(&task(TaskScope::Chat, TaskState::Ready), false), Some(0));
    }
}
