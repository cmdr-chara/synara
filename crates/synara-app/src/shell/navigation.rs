use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::FocusHandle;

const PAGE_SIZE: usize = 64;
const TASK_PAGE_SIZE: usize = 5;

/// Presentation-only disclosure, paging and focus. Catalog/task state is never duplicated here.
pub(super) struct NavigationState {
    pub studio: bool,
    pub visible: bool,
    pub drawer: ui::motion::Drawer,
    pub search_open: bool,
    pub search: Entity<TextEntry>,
    pub history: Vec<TaskId>,
    pub history_index: usize,
    pub projects_open: bool,
    pub chats_open: bool,
    pub path_open: bool,
    pub title_open: bool,
    pub project_page: usize,
    pub task_page: usize,
    pub menu_open: bool,
    pub menu_index: usize,
    pub root_focus: FocusHandle,
    pub brand_focus: FocusHandle,
    pub menu_focus: [FocusHandle; 2],
    pub collapsed_projects: HashSet<ProjectId>,
    pub last_synara: Option<TaskId>,
    pub last_studio: Option<TaskId>,
    pub initialized: bool,
    _search_subscription: Subscription,
}
impl NavigationState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let search = cx.new(|cx| TextEntry::new("Search threads", EntryMode::SingleLine, 34., cx));
        let subscription = cx.subscribe(&search, |_, _, _, cx| cx.notify());
        Self {
            studio: false,
            collapsed_projects: HashSet::new(),
            last_synara: None,
            last_studio: None,
            visible: true,
            drawer: ui::motion::Drawer::new(true),
            search_open: false,
            search,
            history: vec![],
            history_index: 0,
            projects_open: true,
            chats_open: true,
            path_open: false,
            title_open: false,
            project_page: 0,
            task_page: 0,
            menu_open: false,
            menu_index: 0,
            root_focus: cx.focus_handle(),
            brand_focus: cx.focus_handle(),
            menu_focus: std::array::from_fn(|_| cx.focus_handle()),
            initialized: false,
            _search_subscription: subscription,
        }
    }

    pub fn record_task(&mut self, id: TaskId) {
        if self.history.get(self.history_index) == Some(&id) {
            return;
        }
        self.history.truncate(self.history_index.saturating_add(1));
        self.history.push(id);
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.history_index = self.history.len() - 1;
    }
}

fn page_start(page: usize, count: usize) -> usize {
    page.min(count.saturating_sub(1) / PAGE_SIZE) * PAGE_SIZE
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SidebarRevealState {
    projects_open: bool,
    chats_open: bool,
    project_page: usize,
    task_page: usize,
    collapsed_projects: HashSet<ProjectId>,
}

/// Return the first page that can contain an item without moving backwards.
/// Projects use fixed pages; task lists use the existing progressive "show more"
/// size, so both can share the same boundary calculation.
fn reveal_page(current_page: usize, item_index: usize, page_size: usize) -> usize {
    current_page.max(item_index / page_size.max(1))
}

/// Pure navigation-only reveal. The caller supplies the already-filtered sidebar
/// order so stable ids, rather than render positions or titles, decide what opens.
fn reveal_sidebar_task(
    mut state: SidebarRevealState,
    task_id: TaskId,
    project: Option<(ProjectId, &[ProjectId], &[TaskId])>,
    chat_task_ids: &[TaskId],
) -> SidebarRevealState {
    if let Some((project_id, project_ids, project_task_ids)) = project {
        state.projects_open = true;
        state.collapsed_projects.remove(&project_id);
        if let Some(project_index) = project_ids.iter().position(|id| *id == project_id) {
            state.project_page = reveal_page(state.project_page, project_index, PAGE_SIZE);
        }
        if let Some(task_index) = project_task_ids.iter().position(|id| *id == task_id) {
            state.task_page = reveal_page(state.task_page, task_index, TASK_PAGE_SIZE);
        }
        return state;
    }

    state.chats_open = true;
    if let Some(task_index) = chat_task_ids.iter().position(|id| *id == task_id) {
        state.task_page = reveal_page(state.task_page, task_index, TASK_PAGE_SIZE);
    }
    state
}

impl Shell {
    pub(super) fn is_chat_workspace(&self, project: &Project) -> bool {
        self.catalog.workspaces.iter().find(|workspace| workspace.id == project.workspace_id)
            .is_some_and(|workspace| matches!(&workspace.location, WorkspaceLocation::Local { root } if root.starts_with(&self.scratch_directory)))
    }

    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        // Upstream `handleSidebarOpenChange`: under the rail the toggle routes
        // through `set_sidebar_open` so a hidden panel re-selects its item.
        if self.rail_enabled() {
            let open = !self.navigation.visible;
            self.set_sidebar_open(open, cx);
            return;
        }
        self.navigation.visible = !self.navigation.visible;
        self.navigation.drawer.set_open(
            self.navigation.visible,
            std::time::Instant::now(),
            cx.reduce_motion(),
        );
        cx.notify();
    }

    pub(super) fn history_back(&mut self, back: bool, cx: &mut Context<Self>) {
        let old = self.navigation.history_index;
        let next = if back {
            old.checked_sub(1)
        } else {
            old.checked_add(1)
        };
        let Some(next) = next else {
            return;
        };
        let Some(id) = self.navigation.history.get(next).copied() else {
            return;
        };
        self.navigation.history_index = next;
        if self.select_task(id, cx) {
            self.show_conversation(cx);
        } else {
            self.navigation.history_index = old;
        }
    }

    pub(super) fn navigate_project(&mut self, id: ProjectId, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) {
            return;
        }
        if self.dirty(cx) || self.saving {
            self.error =
                Some("Save or discard the open document before switching projects.".into());
            cx.notify();
            return;
        }
        if let Some(task) = self.catalog.tasks.iter().find(|task| {
            task.project_id == id
                && task.scope == TaskScope::Project
                && task.state != TaskState::Archived
        }) {
            let task = task.id;
            self.select_task(task, cx);
        } else {
            if let Some(previous) = self.selected {
                self.drafts
                    .insert(previous, self.composer.read(cx).text().to_owned());
            }
            self.project = Some(id);
            self.snapshot_draft(cx);
            self.selected = None;
            self.thread = None;
            self.refresh_prompt_history(cx);
            self.details = None;
            self.trace.clear();
            self.reset_editor_tabs();
            self.document = None;
            self.files.clear();
            self.directory.clear();
            self.create_task(cx);
        }
        self.navigation.task_page = 0;
        self.show_conversation(cx);
    }

    pub(super) fn switch_mode(&mut self, studio: bool, cx: &mut Context<Self>) {
        self.navigation.menu_open = false;
        if self.hub_navigation_blocked(cx) {
            return;
        }
        if studio {
            self.show_hubs(cx);
            return;
        }
        if self.navigation.studio == studio {
            cx.notify();
            return;
        }
        if self.creating_task || self.loading_task.is_some() {
            cx.notify();
            return;
        }
        if self.dirty(cx) || self.saving {
            self.error = Some("Save or discard the open document before switching modes.".into());
            cx.notify();
            return;
        }
        self.selection_revision = self.selection_revision.wrapping_add(1);
        self.device.retire();
        self.navigation.studio = studio;
        self.navigation.task_page = 0;
        self.navigation
            .search
            .update(cx, |entry, cx| entry.clear(cx));
        let previous = if studio {
            self.navigation.last_studio
        } else {
            self.navigation.last_synara
        };
        let next = previous.or_else(|| {
            self.catalog
                .tasks
                .iter()
                .find(|task| {
                    task.state != TaskState::Archived && (task.scope == TaskScope::Studio) == studio
                })
                .map(|task| task.id)
        });
        if let Some(next) = next {
            self.select_task(next, cx);
        } else {
            self.start_new_chat(cx);
        }
        self.show_conversation(cx);
    }

    pub(super) fn start_new_chat(&mut self, cx: &mut Context<Self>) {
        self.navigation.task_page = 0;
        self.task_title.update(cx, |entry, cx| entry.clear(cx));
        self.create_chat(
            if self.navigation.studio {
                TaskScope::Studio
            } else {
                TaskScope::Chat
            },
            cx,
        );
    }

    pub(super) fn thread_row(
        &self,
        task: &Task,
        nested: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = task.id;
        let index = self
            .catalog
            .tasks
            .iter()
            .position(|item| item.id == id)
            .unwrap_or(0);
        ui::action(
            SharedString::from(format!("task-{id}")),
            task.title.clone(),
            Some(self.agent_glyph(&task.agent_id)),
            self.selected == Some(id) && (self.panel == Panel::Conversation || self.dock_open()),
            cx.listener(move |this, _: &(), _, cx| {
                if this.select_task(id, cx) {
                    this.show_conversation(cx);
                }
            }),
        )
        .h(px(32.))
        .when(nested, |row| row.pl(px(24.)))
        .aria_label(format!("{} · {:?}", task.title, task.state))
        .relative()
        .group("thread-row")
        .child(ui::layout_probe_slot("thread-row", index))
        .child(self.thread_pin_button(task, cx))
        .children(
            self.pending
                .iter()
                .any(|(key, interaction)| {
                    key.0 == task.thread_id
                        && matches!(interaction, UiInteraction::Permission { .. })
                })
                .then(|| {
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(palette().pending))
                        .child("Pending")
                }),
        )
        .children(self.thread_status_dot(task))
        .into_any_element()
    }

    /// Reveal the selected task in the same ordered lists rendered by the
    /// sidebar. This is an explicit navigation transition, not render-time
    /// reconciliation, so collapsed sections and pagination remain stable until
    /// a user selects a task.
    pub(super) fn reveal_selected_task(
        &mut self,
        task_id: TaskId,
        task: &Task,
        cx: &mut Context<Self>,
    ) {
        // Hubs own their own sidebar disclosure and paging; do not route their
        // selection through the classic Projects/Chats navigation state.
        if task.scope == TaskScope::Studio {
            return;
        }
        let query = if self.navigation.search_open {
            self.navigation.search.read(cx).text().trim().to_lowercase()
        } else {
            String::new()
        };
        let mut tasks: Vec<_> = self
            .catalog
            .tasks
            .iter()
            .filter(|candidate| {
                candidate.state != TaskState::Archived
                    && (candidate.scope == TaskScope::Studio) == self.navigation.studio
                    // Keep the selected row discoverable while its draft is being
                    // restored; the regular sidebar still hides untouched drafts
                    // when they are not the active task.
                    && (candidate.id == task_id
                        || !(candidate.state == TaskState::Ready
                            && matches!(
                                candidate.title.as_str(),
                                "New task" | "New thread" | "New studio chat"
                            )
                            && self.drafts.get(&candidate.id).is_none_or(String::is_empty)
                            && (self.selected != Some(candidate.id)
                                || self.composer.read(cx).text().is_empty())))
                    && (query.is_empty() || candidate.title.to_lowercase().contains(&query))
            })
            .collect();
        tasks.sort_by_key(|candidate| std::cmp::Reverse(candidate.updated_at_ms));
        if self.settings.value.general.oldest_threads_first {
            tasks.reverse();
        }

        let mut projects: Vec<_> = self
            .catalog
            .projects
            .iter()
            .filter(|project| {
                !self.is_chat_workspace(project) && self.project_in_active_space(project.id)
            })
            .collect();
        if self.settings.value.general.alphabetical_projects {
            projects.sort_by_key(|project| self.project_name(project).to_lowercase());
        }
        projects.sort_by_key(|project| !self.pinned_project(project.id));

        let project_ids: Vec<_> = projects.iter().map(|project| project.id).collect();
        let project_task_ids: Vec<_> = tasks
            .iter()
            .filter(|candidate| {
                candidate.scope == TaskScope::Project
                    && candidate.project_id == task.project_id
                    && !self.pinned_thread(candidate.id)
            })
            .map(|candidate| candidate.id)
            .collect();
        let chat_task_ids: Vec<_> = tasks
            .iter()
            .filter(|candidate| {
                candidate.scope != TaskScope::Project && !self.pinned_thread(candidate.id)
            })
            .map(|candidate| candidate.id)
            .collect();
        let project = (task.scope == TaskScope::Project).then_some((
            task.project_id,
            project_ids.as_slice(),
            project_task_ids.as_slice(),
        ));
        let state = reveal_sidebar_task(
            SidebarRevealState {
                projects_open: self.navigation.projects_open,
                chats_open: self.navigation.chats_open,
                project_page: self.navigation.project_page,
                task_page: self.navigation.task_page,
                collapsed_projects: self.navigation.collapsed_projects.clone(),
            },
            task_id,
            project,
            &chat_task_ids,
        );
        self.navigation.projects_open = state.projects_open;
        self.navigation.chats_open = state.chats_open;
        self.navigation.project_page = state.project_page;
        self.navigation.task_page = state.task_page;
        self.navigation.collapsed_projects = state.collapsed_projects;
    }

    /// Upstream trailing status, in priority order: pending approval (amber)
    /// → awaiting input (indigo) → working/connecting (focus dot).
    fn thread_status_dot(&self, task: &Task) -> Option<gpui::AnyElement> {
        let mut has_input = false;
        for (key, interaction) in &self.pending {
            if key.0 != task.thread_id {
                continue;
            }
            if matches!(interaction, UiInteraction::Permission { .. }) {
                return Some(
                    div()
                        .size(px(5.))
                        .rounded_full()
                        .bg(rgb(palette().pending))
                        .into_any_element(),
                );
            }
            has_input = true;
        }
        let color = if has_input {
            palette().awaiting
        } else if self.busy.contains(&task.id) || self.connecting.contains(&task.id) {
            palette().focus
        } else if task.state == TaskState::Completed
            && self.selected != Some(task.id)
            && task.updated_at_ms > self.catalog.visited.get(&task.id).copied().unwrap_or(0)
        {
            // Upstream "unread completion" — the accent dot shows until the
            // thread is visited again.
            palette().focus
        } else {
            return None;
        };
        Some(
            div()
                .size(px(5.))
                .rounded_full()
                .bg(rgb(color))
                .into_any_element(),
        )
    }

    pub(super) fn sidebar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.navigation.studio {
            return self.hub_sidebar(cx);
        }
        let studio = self.navigation.studio;
        let query = if self.navigation.search_open {
            self.navigation.search.read(cx).text().trim().to_lowercase()
        } else {
            String::new()
        };
        let mut tasks: Vec<_> = self
            .catalog
            .tasks
            .iter()
            .filter(|task| {
                task.state != TaskState::Archived
                    && (task.scope == TaskScope::Studio) == studio
                    // An untouched draft is the welcome screen, not a second
                    // conversation row. Keep typed drafts reachable on return.
                    && !(task.state == TaskState::Ready
                        && matches!(task.title.as_str(), "New task" | "New thread" | "New studio chat")
                        && self.drafts.get(&task.id).is_none_or(String::is_empty)
                        && (self.selected != Some(task.id) || self.composer.read(cx).text().is_empty()))
                    && (query.is_empty() || task.title.to_lowercase().contains(&query))
            })
            .collect();
        tasks.sort_by_key(|task| std::cmp::Reverse(task.updated_at_ms));
        if self.settings.value.general.oldest_threads_first {
            tasks.reverse();
        }
        let mut projects: Vec<_> = self
            .catalog
            .projects
            .iter()
            .filter(|project| {
                !self.is_chat_workspace(project) && self.project_in_active_space(project.id)
            })
            .collect();
        if self.settings.value.general.alphabetical_projects {
            projects.sort_by_key(|project| self.project_name(project).to_lowercase());
        }
        projects.sort_by_key(|project| !self.pinned_project(project.id));
        let project_start = page_start(self.navigation.project_page, projects.len());
        let shown = self
            .navigation
            .task_page
            .saturating_add(1)
            .saturating_mul(TASK_PAGE_SIZE);
        let pinned: Vec<_> = tasks
            .iter()
            .filter(|task| self.pinned_thread(task.id))
            .copied()
            .collect();
        let chats: Vec<_> = tasks
            .iter()
            .filter(|task| task.scope != TaskScope::Project && !self.pinned_thread(task.id))
            .copied()
            .collect();
        div()
            .id("workspace-navigation")
            .role(gpui::Role::Navigation)
            .aria_label(if studio {
                "Studio chats"
            } else {
                "Projects and chats"
            })
            .tab_group()
            .w(px(ui::SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(ui::surface(palette().sidebar))
            .child(
                div()
                    .h(px(38.))
                    .flex_shrink_0()
                    .pl_4()
                    .pr_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        ui::button(
                            "synara-menu",
                            if studio { "Studio" } else { "Synara" },
                            false,
                        )
                        .px_0()
                        .py_0()
                        .border_0()
                        .bg(gpui::rgba(0))
                        .font_family("Cal Sans")
                        .font_weight(gpui::FontWeight::NORMAL)
                        .text_size(px(17.))
                        .flex()
                        .items_center()
                        .gap_2()
                        .track_focus(&self.navigation.brand_focus)
                        .relative()
                        .top(px(-2.))
                        .child(ui::layout_probe("workspace-tools"))
                        .child(ui::icon(Glyph::Chevron).size(px(11.)))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.controls.retire();
                            this.navigation.menu_open = !this.navigation.menu_open;
                            this.navigation.menu_index = usize::from(this.navigation.studio);
                            window.focus(
                                if this.navigation.menu_open {
                                    &this.navigation.menu_focus[this.navigation.menu_index]
                                } else {
                                    &this.navigation.brand_focus
                                },
                                cx,
                            );
                            cx.notify();
                        })),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                ui::chrome_button(
                                    "search-threads",
                                    "Search threads",
                                    Glyph::Search,
                                    false,
                                    cx.listener(|this, _: &(), window, cx| {
                                        this.navigation.search_open = !this.navigation.search_open;
                                        if this.navigation.search_open {
                                            window.focus(
                                                &this.navigation.search.read(cx).focus_handle(cx),
                                                cx,
                                            );
                                        }
                                        cx.notify();
                                    }),
                                )
                                .size(px(26.)),
                            )
                            .children((!studio).then(|| {
                                ui::chrome_button(
                                    "all-threads",
                                    "Search all threads",
                                    Glyph::Notebook,
                                    false,
                                    cx.listener(|this, _: &(), window, cx| {
                                        this.open_thread_finder(window, cx)
                                    }),
                                )
                                .size(px(26.))
                            })),
                    ),
            )
            .children((!studio).then(|| self.space_strip(cx)))
            .children(
                self.navigation
                    .search_open
                    .then(|| div().px_3().py_1().child(self.navigation.search.clone())),
            )
            .child(
                div()
                    .px_2()
                    .pt(px(3.))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .child(
                        ui::action(
                            "new-thread",
                            if studio {
                                "New studio chat"
                            } else {
                                "New thread"
                            },
                            Some(Glyph::Compose),
                            false,
                            cx.listener(|this, _: &(), _, cx| this.start_new_chat(cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("new-thread")),
                    )
                    // Upstream `panelSidebarNavIds`: under the rail the panel
                    // sidebar keeps only the new-thread action — the rail owns
                    // the destination rows.
                    .when(!studio && !self.rail_enabled(), |nav| {
                        nav.child(
                            ui::action(
                                "kanban-navigation",
                                "Tasks",
                                Some(Glyph::Kanban),
                                self.panel == Panel::Kanban,
                                cx.listener(|this, _: &(), _, cx| {
                                    this.set_panel(Panel::Kanban, cx)
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe("kanban-navigation")),
                        )
                        .child(
                            ui::action(
                                "pull-requests-navigation",
                                "Pull requests",
                                Some(Glyph::PullRequest),
                                self.panel == Panel::PullRequests,
                                cx.listener(|this, _: &(), _, cx| {
                                    this.set_panel(Panel::PullRequests, cx)
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe("pull-requests-navigation")),
                        )
                        .child(ui::action(
                            "automations-navigation",
                            "Automations",
                            Some(Glyph::Clock),
                            self.panel == Panel::Automations,
                            cx.listener(|this, _: &(), _, cx| {
                                this.set_panel(Panel::Automations, cx)
                            }),
                        ))
                    }),
            )
            .child(
                div()
                    .id("sidebar-lists")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pb_4()
                    .children((!pinned.is_empty()).then(|| {
                        div()
                            .id("pinned-threads")
                            .mt_4()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px_2()
                                    .pb_1()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette().muted))
                                    .child("Pinned"),
                            )
                            .child(div().px_2().flex().flex_col().children(
                                pinned.iter().map(|task| self.thread_row(task, false, cx)),
                            ))
                    }))
                    .when(!studio, |list| {
                        list.child(
                            div()
                                .mt_4()
                                .px_2()
                                .flex()
                                .items_center()
                                .group("project-actions")
                                .child(
                                    ui::action(
                                        "projects-disclosure",
                                        "Projects",
                                        None,
                                        false,
                                        cx.listener(|this, _: &(), _, cx| {
                                            this.navigation.projects_open =
                                                !this.navigation.projects_open;
                                            cx.notify();
                                        }),
                                    )
                                    .text_color(rgb(palette().muted))
                                    .flex_1(),
                                )
                                .child(
                                    ui::chrome_button(
                                        "add-project",
                                        "Add project",
                                        Glyph::Plus,
                                        false,
                                        cx.listener(|this, _: &(), _, cx| {
                                            this.browse_workspace(cx)
                                        }),
                                    )
                                    .size(px(24.))
                                    .opacity(0.)
                                    .group_hover("project-actions", |style| style.opacity(1.))
                                    .focus_visible(|style| style.opacity(1.)),
                                )
                                .child(
                                    ui::chrome_button(
                                        "workspace-path-toggle",
                                        "Open project by path",
                                        Glyph::More,
                                        false,
                                        cx.listener(|this, _: &(), window, cx| {
                                            this.navigation.path_open = !this.navigation.path_open;
                                            if this.navigation.path_open {
                                                window.focus(
                                                    &this.workspace_path.read(cx).focus_handle(cx),
                                                    cx,
                                                );
                                            }
                                            cx.notify();
                                        }),
                                    )
                                    .size(px(24.))
                                    .opacity(0.)
                                    .group_hover("project-actions", |style| style.opacity(1.))
                                    .focus_visible(|style| style.opacity(1.)),
                                ),
                        )
                        .children(self.navigation.path_open.then(|| {
                            div()
                                .px_3()
                                .py_2()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(self.workspace_path.clone())
                                .child(ui::action(
                                    "open-workspace",
                                    "Open project",
                                    None,
                                    false,
                                    cx.listener(|this, _: &(), _, cx| this.open_workspace(cx)),
                                ))
                                .child(ui::action(
                                    "refresh-catalog",
                                    "Refresh projects and chats",
                                    None,
                                    false,
                                    cx.listener(|this, _: &(), _, _| {
                                        let workspace = this.controller.workspace.clone();
                                        this.job(async move {
                                            Ok(Update::Catalog(workspace.catalog().await?))
                                        });
                                    }),
                                ))
                        }))
                        .children(self.navigation.projects_open.then(|| {
                            div()
                                .id("project-list")
                                .px_2()
                                .flex()
                                .flex_col()
                                .children(projects.iter().skip(project_start).take(PAGE_SIZE).map(
                                    |project| {
                                        let id = project.id;
                                        let children: Vec<_> = tasks
                                            .iter()
                                            .filter(|task| {
                                                task.scope == TaskScope::Project
                                                    && task.project_id == id
                                                    && !self.pinned_thread(task.id)
                                            })
                                            .copied()
                                            .collect();
                                        let count = children.len();
                                        let open =
                                            !self.navigation.collapsed_projects.contains(&id)
                                                || !query.is_empty();
                                        let path = self
                                            .catalog
                                            .workspaces
                                            .iter()
                                            .find(|workspace| workspace.id == project.workspace_id)
                                            .map(|workspace| match &workspace.location {
                                                WorkspaceLocation::Local { root } => root
                                                    .join(&project.relative_directory)
                                                    .display()
                                                    .to_string(),
                                                WorkspaceLocation::Ssh { root, .. } => root.clone(),
                                            })
                                            .unwrap_or_default();
                                        let name = self.project_name(project);
                                        let icon = self.project_icon(
                                            project,
                                            open,
                                            project_ui::ProjectGlyphPresentation::Badge,
                                        );
                                        let appearance = self
                                            .project_ui
                                            .uis
                                            .get(&project.id)
                                            .and_then(|ui| ui.appearance.clone());
                                        let favicon =
                                            self.project_ui.favicons.get(&project.id).cloned();
                                        div()
                                            .id(SharedString::from(format!("project-group-{id}")))
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .group("project-row")
                                                    .child(
                                                        ui::action_icon(
                                                            SharedString::from(format!(
                                                                "project-{id}"
                                                            )),
                                                            name.clone(),
                                                            Some(icon),
                                                            false,
                                                            cx.listener(
                                                                move |this, _: &(), _, cx| {
                                                                    if !this
                                                                        .navigation
                                                                        .collapsed_projects
                                                                        .remove(&id)
                                                                    {
                                                                        this.navigation
                                                                            .collapsed_projects
                                                                            .insert(id);
                                                                    }
                                                                    cx.notify();
                                                                },
                                                            ),
                                                        )
                                                        .h(px(32.))
                                                        .flex_1()
                                                        .tooltip(move |_, cx| {
                                                            cx.new(|_| ProjectTip {
                                                                name: name.to_string(),
                                                                path: path.clone(),
                                                                count,
                                                                appearance: appearance.clone(),
                                                                favicon: favicon.clone(),
                                                            })
                                                            .into()
                                                        }),
                                                    )
                                                    .child(
                                                        ui::chrome_button(
                                                            "project-edit",
                                                            "Edit project",
                                                            Glyph::Pencil,
                                                            false,
                                                            cx.listener(
                                                                move |this, _: &(), window, cx| {
                                                                    this.open_project_edit(
                                                                        id, window, cx,
                                                                    );
                                                                },
                                                            ),
                                                        )
                                                        .size(px(24.))
                                                        .opacity(0.)
                                                        .group_hover("project-row", |style| {
                                                            style.opacity(1.)
                                                        })
                                                        .focus_visible(|style| style.opacity(1.)),
                                                    )
                                                    .child(
                                                        ui::chrome_button(
                                                            "project-new-chat",
                                                            "New project thread",
                                                            Glyph::Compose,
                                                            false,
                                                            cx.listener(
                                                                move |this, _: &(), _, cx| {
                                                                    this.new_project_chat(id, cx);
                                                                },
                                                            ),
                                                        )
                                                        .size(px(24.))
                                                        .opacity(0.)
                                                        .group_hover("project-row", |style| {
                                                            style.opacity(1.)
                                                        })
                                                        .focus_visible(|style| style.opacity(1.)),
                                                    ),
                                            )
                                            .children(open.then(|| {
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .children(children.iter().take(shown).map(
                                                        |task| self.thread_row(task, true, cx),
                                                    ))
                                                    .children(
                                                        (count > shown).then(|| self.show_more(cx)),
                                                    )
                                            }))
                                    },
                                ))
                                .children((project_start + PAGE_SIZE < projects.len()).then(|| {
                                    ui::action(
                                        "next-projects",
                                        "More projects",
                                        None,
                                        false,
                                        cx.listener(|this, _: &(), _, cx| {
                                            this.navigation.project_page += 1;
                                            cx.notify();
                                        }),
                                    )
                                }))
                        }))
                    })
                    .when(studio || self.settings.value.general.show_chats, |list| {
                        list.child(
                            div()
                                .mt_3()
                                .px_2()
                                .flex()
                                .items_center()
                                .child(
                                    ui::action(
                                        "chats-disclosure",
                                        if studio { "Studio" } else { "Chats" },
                                        None,
                                        false,
                                        cx.listener(|this, _: &(), _, cx| {
                                            this.navigation.chats_open =
                                                !this.navigation.chats_open;
                                            cx.notify();
                                        }),
                                    )
                                    .text_color(rgb(palette().muted))
                                    .flex_1()
                                    .child(
                                        ui::icon(if self.navigation.chats_open {
                                            Glyph::Chevron
                                        } else {
                                            Glyph::ChevronRight
                                        })
                                        .size(px(11.)),
                                    ),
                                )
                                .child(
                                    ui::chrome_button(
                                        "thread-title-toggle",
                                        "Name a new thread",
                                        Glyph::Compose,
                                        false,
                                        cx.listener(|this, _: &(), window, cx| {
                                            this.navigation.title_open =
                                                !this.navigation.title_open;
                                            if this.navigation.title_open {
                                                window.focus(
                                                    &this.task_title.read(cx).focus_handle(cx),
                                                    cx,
                                                );
                                            }
                                            cx.notify();
                                        }),
                                    )
                                    .size(px(24.)),
                                ),
                        )
                        .children(self.navigation.title_open.then(|| {
                            div()
                                .px_3()
                                .py_2()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(self.task_title.clone())
                                .child(ui::action(
                                    "create-task",
                                    "Create thread",
                                    Some(Glyph::Compose),
                                    false,
                                    cx.listener(|this, _: &(), _, cx| {
                                        this.create_chat(
                                            if this.navigation.studio {
                                                TaskScope::Studio
                                            } else {
                                                TaskScope::Chat
                                            },
                                            cx,
                                        )
                                    }),
                                ))
                        }))
                        .children(self.navigation.chats_open.then(|| {
                            div()
                                .id("task-list")
                                .px_2()
                                .pt(px(2.))
                                .flex()
                                .flex_col()
                                .children(
                                    chats
                                        .iter()
                                        .take(shown)
                                        .map(|task| self.thread_row(task, false, cx)),
                                )
                                .children((chats.len() > shown).then(|| self.show_more(cx)))
                                .children(chats.is_empty().then(|| {
                                    div()
                                        .py_5()
                                        .text_center()
                                        .text_color(rgb(palette().muted))
                                        .child(if studio {
                                            "No studio chats yet"
                                        } else if query.is_empty() {
                                            "No chats yet"
                                        } else {
                                            "No matching chats"
                                        })
                                }))
                        }))
                    }),
            )
            .child(
                div()
                    .px_2()
                    .h(px(43.))
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(rgb(palette().border))
                    .flex()
                    .items_center()
                    .child(
                        ui::action(
                            "settings-navigation",
                            "Settings",
                            Some(Glyph::Settings),
                            false,
                            cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Settings, cx)),
                        )
                        .flex_1()
                        .relative()
                        .child(ui::layout_probe("settings-navigation")),
                    )
                    .child(
                        ui::chrome_button(
                            "help-navigation",
                            "Help and licenses",
                            Glyph::Help,
                            false,
                            cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Help, cx)),
                        )
                        .opacity(0.5),
                    ),
            )
            .into_any_element()
    }

    fn show_more(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        ui::action(
            "next-chats",
            "Show more",
            None,
            false,
            cx.listener(|this, _: &(), _, cx| {
                this.navigation.task_page += 1;
                cx.notify();
            }),
        )
        .pl(px(30.))
        .h(px(32.))
        .text_color(rgb(palette().muted))
        .into_any_element()
    }

    pub(super) fn new_project_chat(&mut self, project: ProjectId, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) {
            return;
        }
        if self.creating_task || self.loading_task.is_some() {
            return;
        }
        if Some(project) != self.project && (self.dirty(cx) || self.saving) {
            self.error =
                Some("Save or discard the open document before switching projects.".into());
            cx.notify();
            return;
        }
        if Some(project) == self.project
            && self
                .task()
                .is_some_and(|task| task.scope == TaskScope::Project)
        {
            self.create_task(cx);
            return;
        }
        if let Some(previous) = self.selected {
            self.drafts
                .insert(previous, self.composer.read(cx).text().to_owned());
        }
        self.selection_revision = self.selection_revision.wrapping_add(1);
        self.device.retire();
        self.project = Some(project);
        self.snapshot_draft(cx);
        self.selected = None;
        self.thread = None;
        self.refresh_prompt_history(cx);
        self.reset_editor_tabs();
        self.document = None;
        self.files.clear();
        self.directory.clear();
        self.composer.update(cx, |entry, cx| entry.clear(cx));
        self.navigation.collapsed_projects.remove(&project);
        self.create_task(cx);
    }
}

struct ProjectTip {
    name: String,
    path: String,
    count: usize,
    appearance: Option<ProjectAppearance>,
    favicon: Option<Arc<gpui::Image>>,
}
impl Render for ProjectTip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(256.))
            .rounded_lg()
            .border_1()
            .border_color(rgb(palette().border))
            .bg(rgb(palette().overlay))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(px(13.))
            .text_color(rgb(palette().text))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(project_ui::project_glyph(
                        self.appearance.as_ref(),
                        self.favicon.as_ref(),
                        false,
                        project_ui::ProjectGlyphPresentation::Badge,
                    ))
                    .child(self.name.clone()),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .text_color(rgb(palette().muted))
                    .child(ui::icon(Glyph::Chat))
                    .child(format!("{} chats", self.count)),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(rgb(palette().border))
                    .pt_2()
                    .text_color(rgb(palette().muted))
                    .child(self.path.clone()),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reveal_state() -> SidebarRevealState {
        SidebarRevealState {
            projects_open: false,
            chats_open: false,
            project_page: 0,
            task_page: 0,
            collapsed_projects: HashSet::new(),
        }
    }

    #[test]
    fn navigation_pages_are_bounded_and_clamped_after_catalog_changes() {
        assert_eq!(page_start(0, 0), 0);
        assert_eq!(page_start(1, PAGE_SIZE), 0);
        assert_eq!(page_start(1, PAGE_SIZE + 1), PAGE_SIZE);
        assert_eq!(page_start(usize::MAX, 3), 0);
        assert_eq!(page_start(1000, 129), 128);
    }

    #[test]
    fn revealing_chats_advances_at_show_more_boundaries_without_moving_back() {
        let chat_ids: Vec<_> = (0..11).map(|_| TaskId::new()).collect();

        let before_boundary = reveal_sidebar_task(reveal_state(), chat_ids[4], None, &chat_ids);
        assert!(before_boundary.chats_open);
        assert_eq!(before_boundary.task_page, 0);

        let at_boundary = reveal_sidebar_task(reveal_state(), chat_ids[5], None, &chat_ids);
        assert_eq!(at_boundary.task_page, 1);

        let already_revealed = reveal_sidebar_task(
            SidebarRevealState {
                task_page: 3,
                ..reveal_state()
            },
            chat_ids[0],
            None,
            &chat_ids,
        );
        assert_eq!(already_revealed.task_page, 3);
    }

    #[test]
    fn revealing_project_tasks_advances_project_and_task_page_boundaries() {
        let project_ids: Vec<_> = (0..65).map(|_| ProjectId::new()).collect();
        let task_ids: Vec<_> = (0..6).map(|_| TaskId::new()).collect();

        let before_project_boundary = reveal_sidebar_task(
            reveal_state(),
            task_ids[4],
            Some((project_ids[63], &project_ids, &task_ids)),
            &[],
        );
        assert_eq!(before_project_boundary.project_page, 0);
        assert_eq!(before_project_boundary.task_page, 0);

        let at_both_boundaries = reveal_sidebar_task(
            reveal_state(),
            task_ids[5],
            Some((project_ids[64], &project_ids, &task_ids)),
            &[],
        );
        assert_eq!(at_both_boundaries.project_page, 1);
        assert_eq!(at_both_boundaries.task_page, 1);
    }

    #[test]
    fn revealing_a_collapsed_project_reopens_projects_and_expands_that_project() {
        let project_id = ProjectId::new();
        let task_id = TaskId::new();
        let mut state = reveal_state();
        state.collapsed_projects.insert(project_id);

        let revealed = reveal_sidebar_task(
            state,
            task_id,
            Some((project_id, &[project_id], &[task_id])),
            &[],
        );

        assert!(revealed.projects_open);
        assert!(!revealed.chats_open);
        assert!(!revealed.collapsed_projects.contains(&project_id));
    }
}
