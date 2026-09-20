//! Kanban is a projection of tasks, not a second agent state machine.
use super::*;
use crate::ui::{self, Glyph, palette, task_dialog::{NewTaskRequest, TaskDialog, TaskDialogEvent}};

#[derive(Default)]
pub(super) struct KanbanState {
    pub dialog: Option<Entity<TaskDialog>>,
    subscription: Option<Subscription>,
    pub project: Option<ProjectId>,
    pub creating: bool,
    polling: bool,
    poll_failed: bool,
    launching: HashSet<TaskId>,
    stopping: HashSet<TaskId>,
    limits: [usize; 3],
}
pub(super) enum KanbanReply {
    Created(Result<Task, String>, String, bool),
    DraftReady(TaskId, Result<String, String>),
    Finished(TaskId, Option<Task>, Option<String>),
    Stopped(TaskId, Option<String>),
    Catalog(Result<Catalog, String>),
}
fn column(task: &Task, starting: bool) -> Option<usize> {
    if task.scope == TaskScope::Studio || task.state == TaskState::Archived { return None; }
    Some(if starting || matches!(task.state, TaskState::Running | TaskState::Waiting) { 1 }
        else if task.state == TaskState::Ready { 0 } else { 2 })
}
fn title(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(64).collect()
}
impl Shell {
    pub(super) fn open_task_dialog(&mut self, draft: bool, cx: &mut Context<Self>) {
        if self.kanban.dialog.is_some() || self.kanban.creating || self.creating_task { return; }
        let projects: Vec<_> = self.catalog.projects.iter().filter(|p| !self.is_chat_workspace(p))
            .map(|p| (p.id, p.name.clone())).collect();
        if projects.is_empty() {
            self.notice = Some("Open a project before creating a Kanban task.".into());
            self.browse_workspace(cx);
            return;
        }
        let agents = self.profiles.iter().map(|p| (p.id.clone(), p.name.clone(), self.agent_glyph(&p.id))).collect();
        let default = self.settings.value.general.default_provider.as_deref()
            .or_else(|| self.task().map(|t| t.agent_id.as_str()));
        let dialog = cx.new(|cx| TaskDialog::new(ui::task_dialog::TaskDialogConfig {
            projects, agents, initial_project: self.kanban.project,
            default_agent: default.map(str::to_owned), draft,
            send_on_enter: self.settings.value.chat.send_on_enter,
        }, cx));
        self.kanban.subscription = Some(cx.subscribe(&dialog, |this, _, event, cx| {
            match event {
                TaskDialogEvent::Dismissed => { this.kanban.dialog = None; }
                TaskDialogEvent::Create(request) => this.create_kanban_task(request, cx),
            }
            cx.notify();
        }));
        self.controls.retire();
        self.navigation.menu_open = false;
        self.focus_composer = false;
        self.kanban.dialog = Some(dialog);
        cx.notify();
    }
    fn create_kanban_task(&mut self, request: &NewTaskRequest, cx: &mut Context<Self>) {
        if self.kanban.creating { return; }
        self.kanban.creating = true;
        let workspace = self.controller.workspace.clone();
        let (project, agent, text, send) = (request.project, request.agent.clone(), request.text.clone(), request.send);
        self.job(async move {
            let result = workspace.create_scoped_task_with_draft(project, title(&text), agent, TaskScope::Project, text.clone())
                .await.map_err(|e| e.to_string());
            Ok(Update::Kanban(Box::new(KanbanReply::Created(result, text, send))))
        });
        cx.notify();
    }
    pub(super) fn poll_kanban(&mut self) {
        if self.panel != Panel::Kanban || self.kanban.polling || self.kanban.poll_failed { return; }
        self.kanban.polling = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move { Ok(Update::Kanban(Box::new(KanbanReply::Catalog(
            workspace.catalog().await.map_err(|e| e.to_string())
        )))) });
    }
    fn run_kanban_draft(&mut self, id: TaskId, cx: &mut Context<Self>) {
        if self.busy.contains(&id) || self.connecting.contains(&id) || self.controls.is_pending(id)
            || self.kanban.launching.contains(&id)
            || !self.catalog.tasks.iter().any(|t| t.id == id && column(t, false) == Some(0)) { return; }
        // Capture the current editor before taking a send snapshot. The dialog
        // never borrows or clears the selected conversation's composer.
        self.snapshot_draft(cx);
        self.kanban.launching.insert(id);
        let draft = self.drafts.get(&id).cloned();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = match draft { Some(text) => Ok(text), None => workspace.task_draft(id).await.map_err(|e| e.to_string()) };
            Ok(Update::Kanban(Box::new(KanbanReply::DraftReady(id, result))))
        });
        cx.notify();
    }
    fn submit_kanban_text(&mut self, id: TaskId, text: String, cx: &mut Context<Self>) {
        self.kanban.launching.remove(&id);
        if text.trim().is_empty() {
            self.notice = Some("Open this task and write a prompt before running it.".into());
            return;
        }
        if self.busy.contains(&id) || self.connecting.contains(&id) || self.controls.is_pending(id)
            || !self.catalog.tasks.iter().any(|t| t.id == id && column(t, false) == Some(0)) { return; }
        if self.drafts.get(&id).is_some_and(|latest| latest != &text) {
            self.notice = Some("The draft changed while loading. Review it before running.".into());
            return;
        }
        self.drafts.insert(id, text.clone());
        self.draft_state.submitted(id, text.clone());
        self.busy.insert(id);
        self.error = None;
        self.notice = None;
        let controller = self.controller.clone();
        self.job(async move {
            let error = controller.submit(id, text).await.err().map(|e| e.to_string());
            let task = controller.workspace.task(id).await.ok();
            Ok(Update::Kanban(Box::new(KanbanReply::Finished(id, task, error))))
        });
        cx.notify();
    }
    fn stop_kanban_task(&mut self, id: TaskId, cx: &mut Context<Self>) {
        if self.kanban.stopping.contains(&id) || self.kanban.launching.contains(&id) { return; }
        if !self.busy.contains(&id) && !self.catalog.tasks.iter().any(|t| t.id == id && column(t, false) == Some(1)) { return; }
        self.kanban.stopping.insert(id);
        let controller = self.controller.clone();
        self.job(async move { Ok(Update::Kanban(Box::new(KanbanReply::Stopped(id,
            controller.cancel(id).await.err().map(|e| e.to_string())
        )))) });
        cx.notify();
    }
    pub(super) fn kanban_reply(&mut self, reply: KanbanReply, cx: &mut Context<Self>) {
        match reply {
            KanbanReply::Created(result, text, send) => {
                self.kanban.creating = false;
                match result {
                    Ok(task) => {
                        let id = task.id;
                        self.replace_task(task);
                        self.drafts.insert(id, text.clone());
                        self.kanban.dialog = None;
                        self.kanban.poll_failed = false;
                        // Stay on the board, preserving the prior conversation's
                        // text/selection. Only the explicit Create request can send.
                        if send { self.submit_kanban_text(id, text, cx); }
                    }
                    Err(error) => {
                        if let Some(dialog) = &self.kanban.dialog { dialog.update(cx, |d, cx| d.failed(error, cx)); }
                        else { self.error = Some(error); }
                    }
                }
            }
            KanbanReply::DraftReady(id, result) => match result {
                Ok(text) => self.submit_kanban_text(id, text, cx),
                Err(error) => { self.kanban.launching.remove(&id); self.error = Some(error); }
            },
            KanbanReply::Finished(id, task, error) => {
                self.busy.remove(&id);
                self.kanban.stopping.remove(&id);
                if let Some(task) = task { self.replace_task(task); }
                if let Some(error) = error { self.error = Some(format!("Task did not finish: {error}. Its draft or transcript remains available.")); }
                if self.selected == Some(id) { self.hydrate(); }
            }
            KanbanReply::Stopped(id, error) => {
                self.kanban.stopping.remove(&id);
                if let Some(error) = error { self.error = Some(error); }
            }
            KanbanReply::Catalog(result) => {
                self.kanban.polling = false;
                match result {
                    Ok(catalog) => {
                        // A queued snapshot cannot erase a task created later or
                        // regress a task after a newer durable event was received.
                        for task in catalog.tasks {
                            if self.catalog.tasks.iter().find(|t| t.id == task.id)
                                .is_none_or(|existing| existing.updated_at_ms <= task.updated_at_ms) {
                                self.replace_task(task);
                            }
                        }
                    }
                    Err(error) => { self.kanban.poll_failed = true; self.error = Some(format!("Could not refresh Kanban: {error}")); }
                }
            }
        }
        cx.notify();
    }
    pub(super) fn kanban_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let project = self.kanban.project.filter(|id| self.catalog.projects.iter().any(|p| p.id == *id));
        let mut tasks: Vec<_> = self.catalog.tasks.iter().filter(|t| column(t, false).is_some()
            && project.is_none_or(|id| id == t.project_id)).collect();
        tasks.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms).then_with(|| a.id.to_string().cmp(&b.id.to_string())));
        let title = project.and_then(|id| self.catalog.projects.iter().find(|p| p.id == id))
            .map_or("Kanban", |p| p.name.as_str());
        div().relative().child(ui::layout_probe("kanban-board"))
            .flex_1().min_h_0().min_w_0().flex().flex_col().px_4().py_3().gap_3()
            .child(div().flex().items_center().gap_2()
                .child(ui::icon(Glyph::Kanban))
                .child(div().text_size(px(17.)).child(title.to_owned()))
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child(format!("{} tasks", tasks.len())))
                .child(div().flex_1())
                .child(ui::button("kanban-refresh", "Refresh", false).on_click(cx.listener(|this, _, _, cx| {
                    this.kanban.poll_failed = false; this.poll_kanban(); cx.notify();
                })))
                .child(ui::action("kanban-new-task", "New task", Some(Glyph::Plus), false,
                    cx.listener(|this, _: &(), _, cx| this.open_task_dialog(false, cx)))
                    .relative().child(ui::layout_probe("kanban-new-task"))))
            .child(div().id("kanban-projects").flex().gap_1().overflow_x_scroll().flex_shrink_0()
                .child(ui::button("kanban-all-projects", "All projects", project.is_none())
                    .on_click(cx.listener(|this, _, _, cx| { this.kanban.project = None; this.kanban.limits = [0; 3]; cx.notify(); })))
                .children(self.catalog.projects.iter().filter(|p| !self.is_chat_workspace(p)).enumerate().map(|(slot, p)| {
                    let id = p.id;
                    ui::button(SharedString::from(format!("kanban-project-{id}")), p.name.clone(), project == Some(id))
                        .flex_shrink_0().relative().child(ui::layout_probe_slot("kanban-project", slot))
                        .on_click(cx.listener(move |this, _, _, cx| { this.kanban.project = Some(id); this.kanban.limits = [0; 3]; cx.notify(); }))
                })))
            .child(div().id("kanban-columns").flex_1().min_h_0().flex().gap_3().overflow_x_scroll()
                .children(["Draft", "In Progress", "Done"].into_iter().enumerate().map(|(index, label)| {
                    let rows: Vec<_> = tasks.iter().copied().filter(|t| column(t, self.busy.contains(&t.id) || self.kanban.launching.contains(&t.id)) == Some(index)).collect();
                    let cap = self.kanban.limits[index].max(50);
                    div().id(("kanban-column", index)).min_w(px(260.)).flex_1().min_h_0().flex().flex_col().gap_2()
                        .child(div().flex().items_center().gap_2().text_size(px(12.)).text_color(rgb(palette().muted))
                            .child(label).child(rows.len().to_string()).child(div().flex_1())
                            .children((index == 0).then(|| ui::chrome_button("kanban-add-draft", "New draft task", Glyph::Plus, self.kanban.creating,
                                cx.listener(|this, _: &(), _, cx| this.open_task_dialog(true, cx))))))
                        .child(div().id(("kanban-tasks", index)).flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap_2()
                            .children(rows.is_empty().then(|| div().p_4().text_size(px(12.)).text_color(rgb(palette().muted))
                                .child(match index { 0 => "No draft tasks", 1 => "No tasks in progress", _ => "No completed tasks" })))
                            .children(rows.iter().take(cap).enumerate().map(|(slot, task)| {
                                let id = task.id;
                                let project = self.catalog.projects.iter().find(|p| p.id == task.project_id)
                                    .map_or("Project", |p| if self.is_chat_workspace(p) { "Chats" } else { p.name.as_str() });
                                let agent = self.profiles.iter().find(|p| p.id == task.agent_id).map_or(task.agent_id.as_str(), |p| p.name.as_str());
                                let state = if self.kanban.stopping.contains(&id) { "Stopping..." }
                                    else if self.kanban.launching.contains(&id) || (self.busy.contains(&id) && task.state == TaskState::Ready) { "Starting..." }
                                    else { match task.state { TaskState::Waiting => "Needs input", TaskState::Failed => "Failed", TaskState::Running => "Running", TaskState::Ready => "Draft", _ => "Done" } };
                                div().id(SharedString::from(format!("kanban-card-{id}"))).relative().p_2().rounded_lg()
                                    .border_1().border_color(rgb(palette().border)).bg(rgb(palette().overlay)).flex().flex_col().gap_2()
                                    .child(ui::layout_probe_slot(match index { 0 => "kanban-draft-card", 1 => "kanban-running-card", _ => "kanban-done-card" }, slot))
                                    .child(ui::button(SharedString::from(format!("kanban-task-{id}")), task.title.clone(), false)
                                        .text_size(px(13.)).w_full().min_w_0().text_ellipsis().relative()
                                        .children((index == 0).then(|| ui::layout_probe("kanban-ready-task")))
                                        .child(ui::layout_probe_slot(match index { 0 => "kanban-open-draft", 1 => "kanban-open-running", _ => "kanban-open-done" }, slot))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this.select_task(id, cx) { this.set_panel(Panel::Conversation, cx); }
                                        })))
                                    .child(div().px_2().text_size(px(11.)).text_color(rgb(palette().muted)).child(project.to_owned()))
                                    .child(div().px_2().flex().items_center().gap_2().text_size(px(11.))
                                        .child(ui::icon(self.agent_glyph(&task.agent_id)).size(px(14.)))
                                        .child(div().min_w_0().text_ellipsis().child(agent.to_owned()))
                                        .child(div().flex_1()).child(state))
                                    .children((index == 0).then(|| ui::button(SharedString::from(format!("kanban-run-{id}")), "Run draft", false)
                                        .relative().child(ui::layout_probe_slot("kanban-run-draft", slot)).text_size(px(12.))
                                        .on_click(cx.listener(move |this, _, _, cx| this.run_kanban_draft(id, cx)))))
                                    .children((index == 1).then(|| ui::button(SharedString::from(format!("kanban-stop-{id}")), "Stop", false)
                                        .relative().child(ui::layout_probe_slot("kanban-stop-task", slot)).text_size(px(12.))
                                        .on_click(cx.listener(move |this, _, _, cx| this.stop_kanban_task(id, cx)))))
                            }))
                            .children((rows.len() > cap).then(|| ui::button(SharedString::from(format!("kanban-more-{index}")), format!("Show more ({} remaining)", rows.len() - cap), false)
                                .on_click(cx.listener(move |this, _, _, cx| { this.kanban.limits[index] = cap + 50; cx.notify(); })))))
                })))
            .into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn task(state: TaskState, scope: TaskScope) -> Task {
        Task { id: TaskId::new(), project_id: ProjectId::new(), thread_id: ThreadId::new(),
            title: "Example".into(), state, agent_id: "fixture".into(), working_directory: PathBuf::from("/owned"), updated_at_ms: 1, scope }
    }
    #[test]
    fn columns_preserve_runtime_truth_and_exclude_studio_and_archived_tasks() {
        for (state, expected) in [(TaskState::Ready, 0), (TaskState::Running, 1), (TaskState::Waiting, 1), (TaskState::Completed, 2), (TaskState::Failed, 2)] {
            assert_eq!(column(&task(state, TaskScope::Project), false), Some(expected));
            assert_eq!(column(&task(state, TaskScope::Studio), false), None);
        }
        assert_eq!(column(&task(TaskState::Archived, TaskScope::Project), true), None);
        assert_eq!(column(&task(TaskState::Ready, TaskScope::Chat), true), Some(1));
    }
    #[test]
    fn task_title_is_whitespace_normalized_and_unicode_bounded() {
        assert_eq!(title("  Plan\n the\twork  "), "Plan the work");
        assert_eq!(title(&"日本語".repeat(100)).chars().count(), 64);
    }
}
