//! Kanban is a projection of tasks, not a second agent state machine.
use super::*;
mod hub;
use crate::ui::{
    self, Glyph, palette,
    task_dialog::{NewTaskRequest, TaskDialog, TaskDialogEvent},
};

#[derive(Default)]
pub(super) struct KanbanState {
    pub dialog: Option<Entity<TaskDialog>>,
    subscription: Option<Subscription>,
    pub project: Option<ProjectId>,
    pub creating: bool,
    dialog_hub: Option<ProjectId>,
    hub_view: hub::HubBoardState,
    polling: bool,
    poll_failed: bool,
    launching: HashMap<TaskId, u64>,
    next_launch: u64,
    stopping: HashSet<TaskId>,
    limits: [usize; 3],
}
pub(super) enum KanbanReply {
    Created(Result<Task, String>, String, bool),
    DraftReady(TaskId, u64, Result<String, String>),
    Finished(TaskId, Option<Task>, Option<String>),
    Stopped(TaskId, Option<String>),
    Catalog(Result<Catalog, String>),
}
impl KanbanState {
    pub(super) fn cancel_pending_launches(&mut self) {
        self.launching.clear();
    }
    fn reserve_launch(&mut self, task: TaskId) -> u64 {
        self.next_launch = self.next_launch.wrapping_add(1);
        self.launching.insert(task, self.next_launch);
        self.next_launch
    }
    fn take_launch(&mut self, task: TaskId, generation: u64) -> bool {
        if self.launching.get(&task) != Some(&generation) {
            return false;
        }
        self.launching.remove(&task);
        true
    }
}
fn column(task: &Task, starting: bool) -> Option<usize> {
    if task.scope == TaskScope::Studio { return None; }
    execution_column(task, starting)
}
fn execution_column(task: &Task, starting: bool) -> Option<usize> {
    if task.state == TaskState::Archived { return None; }
    Some(if starting || matches!(task.state, TaskState::Running | TaskState::Waiting) {
        1
    } else if task.state == TaskState::Ready { 0 } else { 2 })
}
fn title(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(64)
        .collect()
}
impl Shell {
    pub(super) fn open_task_dialog(&mut self, draft: bool, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) || self.close != CloseState::Open
            || self.kanban.dialog.is_some()
            || self.kanban.creating
            || self.creating_task
        {
            return;
        }
        // Capture the origin when the dialog opens. Later navigation must not
        // change whether a request creates a normal task or a Hub task.
        let hub = if self.navigation.studio {
            let Some(hub) = self.hubs.rows.iter().find(|hub|
                Some(hub.profile.project) == self.hubs.selected && !hub.profile.archived)
            else {
                self.error = Some("Select an active Hub before creating a task.".into());
                cx.notify();
                return;
            };
            Some(hub.profile.project)
        } else { None };
        let projects: Vec<_> = self.catalog.projects.iter()
            .filter(|p| hub.map_or_else(|| !self.is_chat_workspace(p), |id| p.id == id))
            .map(|p| (p.id, self.hubs.rows.iter().find(|h| Some(h.profile.project) == hub)
                .map_or_else(|| p.name.clone(), |h| h.profile.name.clone())))
            .collect();
        if projects.is_empty() {
            self.notice = Some("Open a project before creating a Kanban task.".into());
            self.browse_workspace(cx);
            return;
        }
        let agents = self
            .profiles
            .iter()
            .map(|p| (p.id.clone(), p.name.clone(), self.agent_glyph(&p.id)))
            .collect();
        let default = self
            .settings
            .value
            .general
            .default_provider
            .as_deref()
            .or_else(|| self.task().map(|t| t.agent_id.as_str()));
        let dialog = cx.new(|cx| {
            TaskDialog::new(
                ui::task_dialog::TaskDialogConfig {
                    projects,
                    agents,
                    initial_project: self.kanban.project,
                    default_agent: default.map(str::to_owned),
                    draft,
                    send_on_enter: self.settings.value.chat.send_on_enter,
                },
                cx,
            )
        });
        self.kanban.dialog_hub = hub;
        self.kanban.subscription = Some(cx.subscribe(&dialog, |this, _, event, cx| {
            match event {
                TaskDialogEvent::Dismissed => {
                    this.kanban.dialog = None;
                    this.kanban.dialog_hub = None;
                }
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
        if self.kanban.creating {
            return;
        }
        let hub = self.kanban.dialog_hub;
        if hub.is_some_and(|id| id != request.project) {
            if let Some(dialog) = &self.kanban.dialog {
                dialog.update(cx, |d, cx| d.failed("The task belongs to a different Hub. Reopen the task composer.".into(), cx));
            }
            return;
        }
        self.kanban.creating = true;
        let workspace = self.controller.workspace.clone();
        let (project, agent, text, send) = (
            request.project,
            request.agent.clone(),
            request.text.clone(),
            request.send,
        );
        self.job(async move {
            let result = if hub.is_some() {
                // The dialog contains the full reviewed prompt. Never insert
                // hidden shared context into a Create-and-run request.
                workspace.create_hub_task(project, title(&text), agent, text.clone()).await
            } else {
                workspace.create_scoped_task_with_draft(
                    project, title(&text), agent, TaskScope::Project, text.clone(),
                ).await
            }.map_err(|e| e.to_string());
            Ok(Update::Kanban(Box::new(KanbanReply::Created(
                result, text, send,
            ))))
        });
        cx.notify();
    }
    pub(super) fn poll_kanban(&mut self) {
        if self.panel != Panel::Kanban || self.kanban.polling || self.kanban.poll_failed {
            return;
        }
        self.kanban.polling = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Kanban(Box::new(KanbanReply::Catalog(
                workspace.catalog().await.map_err(|e| e.to_string()),
            ))))
        });
    }
    fn run_kanban_draft(&mut self, id: TaskId, cx: &mut Context<Self>) {
        if self.close != CloseState::Open
            || self.busy.contains(&id)
            || self.connecting.contains(&id)
            || self.controls.is_pending(id)
            || self.kanban.launching.contains_key(&id)
            || !self
                .catalog
                .tasks
                .iter()
                .any(|t| t.id == id && execution_column(t, false) == Some(0) && self.kanban_task_can_run(t))
        {
            return;
        }
        // Capture the current editor before taking a send snapshot. The dialog
        // never borrows or clears the selected conversation's composer.
        self.snapshot_draft(cx);
        let generation = self.kanban.reserve_launch(id);
        let draft = self.drafts.get(&id).cloned();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = match draft {
                Some(text) => Ok(text),
                None => workspace.task_draft(id).await.map_err(|e| e.to_string()),
            };
            Ok(Update::Kanban(Box::new(KanbanReply::DraftReady(
                id, generation, result,
            ))))
        });
        cx.notify();
    }
    fn submit_kanban_text(&mut self, id: TaskId, text: String, cx: &mut Context<Self>) {
        self.kanban.launching.remove(&id);
        let context_only = self.catalog.tasks.iter().any(|task| task.id == id && task.scope == TaskScope::Studio)
            && (text.starts_with("## Hub instructions\n") || text.starts_with("## Shared Hub knowledge\n"))
            && text.rsplit_once("\nTask:\n").is_some_and(|(_, task)| task.trim().is_empty());
        if text.trim().is_empty() || context_only {
            self.notice = Some("Open this task and write a prompt before running it.".into());
            return;
        }
        if self.close != CloseState::Open
            || self.busy.contains(&id)
            || self.connecting.contains(&id)
            || self.controls.is_pending(id)
            || !self
                .catalog
                .tasks
                .iter()
                .any(|t| t.id == id && execution_column(t, false) == Some(0) && self.kanban_task_can_run(t))
        {
            return;
        }
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
            let error = controller
                .submit(id, text)
                .await
                .err()
                .map(|e| e.to_string());
            let task = controller.workspace.task(id).await.ok();
            Ok(Update::Kanban(Box::new(KanbanReply::Finished(
                id, task, error,
            ))))
        });
        cx.notify();
    }
    fn stop_kanban_task(&mut self, id: TaskId, cx: &mut Context<Self>) {
        if self.kanban.launching.remove(&id).is_some() {
            cx.notify();
            return;
        }
        if self.kanban.stopping.contains(&id) {
            return;
        }
        if !self.busy.contains(&id)
            && !self
                .catalog
                .tasks
                .iter()
                .any(|t| t.id == id && execution_column(t, false) == Some(1))
        {
            return;
        }
        self.kanban.stopping.insert(id);
        let controller = self.controller.clone();
        self.job(async move {
            Ok(Update::Kanban(Box::new(KanbanReply::Stopped(
                id,
                controller.cancel(id).await.err().map(|e| e.to_string()),
            ))))
        });
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
                        self.kanban.dialog_hub = None;
                        self.kanban.poll_failed = false;
                        // Stay on the board, preserving the prior conversation's
                        // text/selection. Only the explicit Create request can send.
                        if send {
                            self.submit_kanban_text(id, text, cx);
                        }
                    }
                    Err(error) => {
                        if let Some(dialog) = &self.kanban.dialog {
                            dialog.update(cx, |d, cx| d.failed(error, cx));
                        } else {
                            self.error = Some(error);
                        }
                    }
                }
            }
            KanbanReply::DraftReady(id, generation, result) => {
                if !self.kanban.take_launch(id, generation) {
                    return;
                }
                match result {
                    Ok(text) => self.submit_kanban_text(id, text, cx),
                    Err(error) => self.error = Some(error),
                }
            }
            KanbanReply::Finished(id, task, error) => {
                self.busy.remove(&id);
                self.kanban.stopping.remove(&id);
                if let Some(task) = task {
                    self.replace_task(task);
                }
                if let Some(error) = error {
                    self.error = Some(format!(
                        "Task did not finish: {error}. Its draft or transcript remains available."
                    ));
                }
                if self.selected == Some(id) {
                    self.hydrate();
                }
            }
            KanbanReply::Stopped(id, error) => {
                self.kanban.stopping.remove(&id);
                if let Some(error) = error {
                    self.error = Some(error);
                }
            }
            KanbanReply::Catalog(result) => {
                self.kanban.polling = false;
                match result {
                    Ok(catalog) => {
                        // Update known tasks only. A queued snapshot cannot revive
                        // a deleted task, erase a later creation, or replace a newer event.
                        for task in catalog.tasks {
                            if self
                                .catalog
                                .tasks
                                .iter()
                                .find(|t| t.id == task.id)
                                .is_some_and(|existing| {
                                    existing.updated_at_ms <= task.updated_at_ms
                                })
                            {
                                self.replace_task(task);
                            }
                        }
                    }
                    Err(error) => {
                        self.kanban.poll_failed = true;
                        self.error = Some(format!("Could not refresh Kanban: {error}"));
                    }
                }
            }
        }
        cx.notify();
    }
    pub(super) fn kanban_heading(&self) -> String {
        if self.navigation.studio {
            return self.hubs.rows.iter().find(|hub| Some(hub.profile.project) == self.hubs.selected)
                .map_or_else(|| "Hub tasks".into(), |hub| format!("{} · Tasks", hub.profile.name));
        }
        self.kanban
            .project
            .and_then(|id| self.catalog.projects.iter().find(|p| p.id == id))
            .map_or_else(|| "Kanban".into(), |p| p.name.clone())
    }
    pub(super) fn kanban_count(&self) -> usize {
        if self.navigation.studio {
            return self.catalog.tasks.iter().filter(|task| self.in_hub_board(task)).count();
        }
        self.catalog
            .tasks
            .iter()
            .filter(|t| {
                column(t, false).is_some()
                    && self.kanban.project.is_none_or(|id| t.project_id == id)
            })
            .count()
    }
    pub(super) fn kanban_back(&mut self, cx: &mut Context<Self>) {
        if self.navigation.studio {
            self.set_panel(Panel::Hubs, cx);
            return;
        }
        self.kanban.project = None;
        self.kanban.limits = [0; 3];
        cx.notify();
    }
    fn kanban_open_project(&mut self, project: ProjectId, cx: &mut Context<Self>) {
        self.kanban.project = Some(project);
        self.kanban.limits = [0; 3];
        cx.notify();
    }
    fn kanban_card(
        &self,
        task: &Task,
        slot: usize,
        controls: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = task.id;
        let index = column(
            task,
            self.busy.contains(&id) || self.kanban.launching.contains_key(&id),
        )
        .unwrap_or(2);
        let state = if self.kanban.stopping.contains(&id) {
            "Stopping..."
        } else if self.kanban.launching.contains_key(&id)
            || (self.busy.contains(&id) && task.state == TaskState::Ready)
        {
            "Starting..."
        } else {
            match task.state {
                TaskState::Waiting => "Needs input",
                TaskState::Failed => "Failed",
                TaskState::Running => "Running",
                TaskState::Ready => "Draft",
                _ => "Done",
            }
        };
        let agent = self
            .profiles
            .iter()
            .find(|p| p.id == task.agent_id)
            .map_or(task.agent_id.as_str(), |p| p.name.as_str())
            .to_owned();
        div()
            .id(SharedString::from(format!("kanban-card-{id}")))
            .relative()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(palette().border))
            .child(ui::layout_probe_slot(
                match index {
                    0 => "kanban-draft-card",
                    1 => "kanban-running-card",
                    _ => "kanban-done-card",
                },
                slot,
            ))
            .child(
                ui::button_shell(
                    SharedString::from(format!("kanban-task-{id}")),
                    task.title.clone(),
                    false,
                )
                .w_full()
                .min_w_0()
                .p(px(12.))
                .bg(gpui::rgba(0))
                .rounded(px(12.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .relative()
                .child(ui::layout_probe_slot(
                    match index {
                        0 => "kanban-open-draft",
                        1 => "kanban-open-running",
                        _ => "kanban-open-done",
                    },
                    slot,
                ))
                .children((index == 0).then(|| ui::layout_probe("kanban-ready-task")))
                .child(
                    div()
                        .w_full()
                        .text_size(px(14.))
                        .text_ellipsis()
                        .child(task.title.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(11.))
                        .text_color(rgb(palette().muted))
                        .child(ui::icon(self.agent_glyph(&task.agent_id)).size(px(14.)))
                        .child(div().min_w_0().text_ellipsis().child(agent))
                        .child(div().flex_1())
                        .child(state),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.select_task(id, cx) {
                        this.show_conversation(cx);
                    }
                })),
            )
            .children((controls && index < 2).then(|| {
                ui::button(
                    SharedString::from(format!("kanban-command-{id}")),
                    if index == 0 { "Run draft" } else { "Stop" },
                    false,
                )
                .w_full()
                .bg(gpui::rgba(0))
                .border_t_1()
                .border_color(rgb(palette().border))
                .text_size(px(12.))
                .relative()
                .child(ui::layout_probe_slot(
                    if index == 0 {
                        "kanban-run-draft"
                    } else {
                        "kanban-stop-task"
                    },
                    slot,
                ))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if index == 0 {
                        this.run_kanban_draft(id, cx);
                    } else {
                        this.stop_kanban_task(id, cx);
                    }
                    cx.stop_propagation();
                }))
            }))
            .into_any_element()
    }
    pub(super) fn kanban_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.navigation.studio { return self.hub_task_board(cx); }
        let project = self
            .kanban
            .project
            .filter(|id| self.catalog.projects.iter().any(|p| p.id == *id));
        let mut tasks: Vec<_> = self
            .catalog
            .tasks
            .iter()
            .filter(|t| column(t, false).is_some() && project.is_none_or(|id| id == t.project_id))
            .collect();
        tasks.sort_by(|a, b| {
            b.updated_at_ms
                .cmp(&a.updated_at_ms)
                .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
        });
        let content = if project.is_some() {
            div()
                .id("kanban-columns")
                .size_full()
                .min_h_0()
                .flex()
                .gap_4()
                .overflow_x_scroll()
                .children(
                    ["Draft", "In Progress", "Done"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, label)| {
                            let rows: Vec<_> = tasks
                                .iter()
                                .copied()
                                .filter(|t| {
                                    column(
                                        t,
                                        self.busy.contains(&t.id)
                                            || self.kanban.launching.contains_key(&t.id),
                                    ) == Some(index)
                                })
                                .collect();
                            let cap = self.kanban.limits[index].max(50);
                            div()
                                .id(("kanban-column", index))
                                .min_w(px(260.))
                                .flex_1()
                                .h_full()
                                .min_h_0()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .h(px(32.))
                                        .px_2()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .text_size(px(13.))
                                        .child(label)
                                        .child(
                                            div()
                                                .text_color(rgb(palette().muted))
                                                .child(rows.len().to_string()),
                                        )
                                        .child(div().flex_1())
                                        .children((index == 0).then(|| {
                                            ui::chrome_button(
                                                "kanban-add-draft",
                                                "New draft task",
                                                Glyph::Plus,
                                                self.kanban.creating,
                                                cx.listener(|this, _: &(), _, cx| {
                                                    this.open_task_dialog(true, cx)
                                                }),
                                            )
                                        })),
                                )
                                .child(
                                    div()
                                        .id(("kanban-tasks", index))
                                        .flex_1()
                                        .min_h_0()
                                        .overflow_y_scroll()
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .p_1()
                                        .children(rows.is_empty().then(|| {
                                            div()
                                                .p_4()
                                                .text_size(px(12.))
                                                .text_color(rgb(palette().muted))
                                                .child(match index {
                                                    0 => "No draft tasks",
                                                    1 => "No tasks in progress",
                                                    _ => "No completed tasks",
                                                })
                                        }))
                                        .children(rows.iter().take(cap).enumerate().map(
                                            |(slot, task)| self.kanban_card(task, slot, true, cx),
                                        ))
                                        .children((rows.len() > cap).then(|| {
                                            ui::button(
                                                SharedString::from(format!("kanban-more-{index}")),
                                                format!(
                                                    "Show more ({} remaining)",
                                                    rows.len() - cap
                                                ),
                                                false,
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.kanban.limits[index] = cap + 50;
                                                    cx.notify();
                                                }),
                                            )
                                        })),
                                )
                        }),
                )
                .into_any_element()
        } else if tasks.is_empty() {
            div().size_full().flex().items_center().justify_center()
                .child(div().max_w(px(400.)).text_center().flex().flex_col().gap_1()
                    .child("Nothing on the board yet")
                    .child(div().text_size(px(13.)).text_color(rgb(palette().muted)).child("Drafted prompts, running turns, and completed chats will show up here automatically.")))
                .into_any_element()
        } else {
            // Project groups keep a stable catalog order. Empty projects are not
            // noise on the overview. Existing standalone chats retain their identity.
            let mut state_slots = [0usize; 3];
            let slots: HashMap<_, _> = tasks
                .iter()
                .map(|t| {
                    let index = column(
                        t,
                        self.busy.contains(&t.id) || self.kanban.launching.contains_key(&t.id),
                    )
                    .unwrap_or(2);
                    let slot = state_slots[index];
                    state_slots[index] += 1;
                    (t.id, slot)
                })
                .collect();
            div()
                .id("kanban-overview")
                .size_full()
                .min_h_0()
                .flex()
                .gap_4()
                .overflow_x_scroll()
                .children(
                    self.catalog
                        .projects
                        .iter()
                        .filter(|p| tasks.iter().any(|t| t.project_id == p.id))
                        .enumerate()
                        .map(|(project_slot, p)| {
                            let id = p.id;
                            let label = if self.is_chat_workspace(p) {
                                "Chats"
                            } else {
                                p.name.as_str()
                            };
                            let mut rows: Vec<_> = tasks
                                .iter()
                                .copied()
                                .filter(|t| t.project_id == id)
                                .collect();
                            rows.sort_by_key(|t| {
                                match column(
                                    t,
                                    self.busy.contains(&t.id)
                                        || self.kanban.launching.contains_key(&t.id),
                                ) {
                                    Some(1) => 0,
                                    Some(0) => 1,
                                    _ => 2,
                                }
                            });
                            let count = rows.len();
                            div()
                                .w(px(288.))
                                .flex_shrink_0()
                                .h_full()
                                .min_h_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .h(px(32.))
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            ui::button_shell(
                                                SharedString::from(format!("kanban-project-{id}")),
                                                format!("Open {label} board"),
                                                false,
                                            )
                                            .bg(gpui::rgba(0))
                                            .flex_1()
                                            .min_w_0()
                                            .px_2()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .relative()
                                            .child(ui::layout_probe_slot(
                                                "kanban-open-project",
                                                project_slot,
                                            ))
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .text_ellipsis()
                                                    .text_size(px(15.))
                                                    .child(label.to_owned()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(rgb(palette().muted))
                                                    .child(count.to_string()),
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.kanban_open_project(id, cx)
                                                }),
                                            ),
                                        )
                                        .child(
                                            ui::button_shell(
                                                SharedString::from(format!(
                                                    "kanban-project-add-{id}"
                                                )),
                                                format!("New task in {label}"),
                                                false,
                                            )
                                            .size(px(28.))
                                            .p_0()
                                            .bg(gpui::rgba(0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(ui::icon(Glyph::Plus).size(px(14.)))
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    let prior = this.kanban.project;
                                                    this.kanban.project = Some(id);
                                                    this.open_task_dialog(false, cx);
                                                    this.kanban.project = prior;
                                                }),
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "kanban-overview-tasks-{id}"
                                        )))
                                        .flex_1()
                                        .min_h_0()
                                        .overflow_y_scroll()
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .p_1()
                                        .children(
                                            rows.iter().take(12).map(|t| {
                                                self.kanban_card(t, slots[&t.id], false, cx)
                                            }),
                                        )
                                        .children((count > 12).then(|| {
                                            ui::button(
                                                SharedString::from(format!(
                                                    "kanban-overview-more-{id}"
                                                )),
                                                format!("Show {} more", count - 12),
                                                false,
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.kanban_open_project(id, cx)
                                                }),
                                            )
                                        })),
                                )
                        }),
                )
                .into_any_element()
        };
        div()
            .relative()
            .child(ui::layout_probe("kanban-board"))
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p_4()
            .flex()
            .flex_col()
            .children(self.kanban.poll_failed.then(|| {
                ui::button("kanban-refresh", "Retry refreshing board", false).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.kanban.poll_failed = false;
                        this.poll_kanban();
                        cx.notify();
                    },
                ))
            }))
            .child(content)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task(state: TaskState, scope: TaskScope) -> Task {
        Task {
            id: TaskId::new(),
            project_id: ProjectId::new(),
            thread_id: ThreadId::new(),
            title: "Example".into(),
            state,
            agent_id: "fixture".into(),
            working_directory: PathBuf::from("/owned"),
            updated_at_ms: 1,
            scope,
        }
    }
    #[test]
    fn columns_preserve_runtime_truth_and_exclude_studio_and_archived_tasks() {
        for (state, expected) in [
            (TaskState::Ready, 0),
            (TaskState::Running, 1),
            (TaskState::Waiting, 1),
            (TaskState::Completed, 2),
            (TaskState::Failed, 2),
        ] {
            assert_eq!(
                column(&task(state, TaskScope::Project), false),
                Some(expected)
            );
            assert_eq!(column(&task(state, TaskScope::Studio), false), None);
        }
        assert_eq!(
            column(&task(TaskState::Archived, TaskScope::Project), true),
            None
        );
        assert_eq!(
            column(&task(TaskState::Ready, TaskScope::Chat), true),
            Some(1)
        );
    }
    #[test]
    fn cancelled_load_cannot_consume_a_later_launch_or_start_twice() {
        let mut state = KanbanState::default();
        let id = TaskId::new();
        let first = state.reserve_launch(id);
        state.launching.remove(&id);
        let second = state.reserve_launch(id);
        assert!(!state.take_launch(id, first));
        assert!(state.take_launch(id, second));
        assert!(!state.take_launch(id, second));
    }
    #[test]
    fn shutdown_retires_all_pending_loads_without_consuming_a_future_request() {
        let mut state = KanbanState::default();
        let first = TaskId::new();
        let second = TaskId::new();
        let a = state.reserve_launch(first);
        let b = state.reserve_launch(second);
        state.cancel_pending_launches();
        assert!(!state.take_launch(first, a));
        assert!(!state.take_launch(second, b));
        let next = state.reserve_launch(first);
        assert!(state.take_launch(first, next));
    }
    #[test]
    fn task_title_is_whitespace_normalized_and_unicode_bounded() {
        assert_eq!(title("  Plan\n the\twork  "), "Plan the work");
        assert_eq!(title(&"日本語".repeat(100)).chars().count(), 64);
    }
}
