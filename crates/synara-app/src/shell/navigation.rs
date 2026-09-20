use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::FocusHandle;

const PAGE_SIZE: usize = 64;

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

impl Shell {
    pub(super) fn is_chat_workspace(&self, project: &Project) -> bool {
        self.catalog.workspaces.iter().find(|workspace| workspace.id == project.workspace_id)
            .is_some_and(|workspace| matches!(&workspace.location, WorkspaceLocation::Local { root } if root.starts_with(&self.scratch_directory)))
    }

    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
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
            self.details = None;
            self.trace.clear();
            self.document = None;
            self.files.clear();
            self.directory.clear();
            self.git = GitStatus::default();
            self.diff.clear();
            self.create_task(cx);
        }
        self.navigation.task_page = 0;
        self.show_conversation(cx);
    }

    pub(super) fn switch_mode(&mut self, studio: bool, cx: &mut Context<Self>) {
        self.navigation.menu_open = false;
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

    fn thread_row(&self, task: &Task, nested: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
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
        .child(ui::layout_probe_slot("thread-row", index))
        .children(
            (self.busy.contains(&id) || self.connecting.contains(&id))
                .then(|| div().size(px(5.)).rounded_full().bg(rgb(palette().focus))),
        )
        .into_any_element()
    }

    pub(super) fn sidebar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
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
            .filter(|project| !self.is_chat_workspace(project))
            .collect();
        if self.settings.value.general.alphabetical_projects {
            projects.sort_by_key(|project| project.name.to_lowercase());
        }
        let project_start = page_start(self.navigation.project_page, projects.len());
        let shown = self
            .navigation
            .task_page
            .saturating_add(1)
            .saturating_mul(5);
        let chats: Vec<_> = tasks
            .iter()
            .filter(|task| task.scope != TaskScope::Project)
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
            .bg(rgb(palette().sidebar))
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
                                        this.navigation.search_open = true;
                                        window.focus(
                                            &this.navigation.search.read(cx).focus_handle(cx),
                                            cx,
                                        );
                                        cx.notify();
                                    }),
                                )
                                .size(px(26.))
                            })),
                    ),
            )
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
                    .when(!studio, |nav| {
                        nav.child(
                            ui::action(
                                "kanban-navigation",
                                "Kanban",
                                Some(Glyph::Kanban),
                                self.panel == Panel::Kanban,
                                cx.listener(|this, _: &(), _, cx| {
                                    this.set_panel(Panel::Kanban, cx)
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe("kanban-navigation")),
                        )
                        .child(ui::unavailable_action(
                            "pull-requests-navigation",
                            "Pull requests",
                            Glyph::PullRequest,
                            "Pull requests are not available in this native build yet.",
                        ))
                        .child(ui::unavailable_action(
                            "automations-navigation",
                            "Automations",
                            Glyph::Clock,
                            "Automations are not available in this native build yet.",
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
                                        let name = project.name.clone();
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
                                                        ui::action(
                                                            SharedString::from(format!(
                                                                "project-{id}"
                                                            )),
                                                            project.name.clone(),
                                                            Some(Glyph::Folder),
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
                                                                name: name.clone(),
                                                                path: path.clone(),
                                                                count,
                                                            })
                                                            .into()
                                                        }),
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

    fn new_project_chat(&mut self, project: ProjectId, cx: &mut Context<Self>) {
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
        self.project = Some(project);
        self.snapshot_draft(cx);
        self.selected = None;
        self.thread = None;
        self.document = None;
        self.files.clear();
        self.directory.clear();
        self.git = GitStatus::default();
        self.diff.clear();
        self.composer.update(cx, |entry, cx| entry.clear(cx));
        self.navigation.collapsed_projects.remove(&project);
        self.create_task(cx);
    }
}

struct ProjectTip {
    name: String,
    path: String,
    count: usize,
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
                    .child(ui::icon(Glyph::Folder))
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_pages_are_bounded_and_clamped_after_catalog_changes() {
        assert_eq!(page_start(0, 0), 0);
        assert_eq!(page_start(1, PAGE_SIZE), 0);
        assert_eq!(page_start(1, PAGE_SIZE + 1), PAGE_SIZE);
        assert_eq!(page_start(usize::MAX, 3), 0);
        assert_eq!(page_start(1000, 129), 128);
    }
}
