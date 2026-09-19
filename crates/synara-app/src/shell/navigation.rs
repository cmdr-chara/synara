use super::*;
use crate::ui::{self, DARK, Glyph};
use gpui::FocusHandle;

const PAGE_SIZE: usize = 64;

/// Presentation-only disclosure, paging and focus. Catalog/task state is never duplicated here.
pub(super) struct NavigationState {
    pub visible: bool,
    pub projects_open: bool,
    pub chats_open: bool,
    pub path_open: bool,
    pub title_open: bool,
    pub project_page: usize,
    pub task_page: usize,
    pub menu_open: bool,
    pub menu_index: usize,
    pub root_focus: FocusHandle,
    pub tools_focus: FocusHandle,
    pub menu_focus: [FocusHandle; 3],
    pub initialized: bool,
}
impl NavigationState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        Self {
            visible: true,
            projects_open: true,
            chats_open: true,
            path_open: false,
            title_open: false,
            project_page: 0,
            task_page: 0,
            menu_open: false,
            menu_index: 0,
            root_focus: cx.focus_handle(),
            tools_focus: cx.focus_handle(),
            menu_focus: std::array::from_fn(|_| cx.focus_handle()),
            initialized: false,
        }
    }
}

fn page_start(page: usize, count: usize) -> usize {
    page.min(count.saturating_sub(1) / PAGE_SIZE) * PAGE_SIZE
}

impl Shell {
    fn navigate_project(&mut self, id: ProjectId, cx: &mut Context<Self>) {
        if self.dirty(cx) || self.saving {
            self.error =
                Some("Save or discard the open document before switching projects.".into());
            cx.notify();
            return;
        }
        if let Some(task) = self
            .catalog
            .tasks
            .iter()
            .find(|task| task.project_id == id && task.state != TaskState::Archived)
        {
            let task = task.id;
            self.select_task(task, cx);
        } else {
            if let Some(previous) = self.selected {
                self.drafts
                    .insert(previous, self.composer.read(cx).text().to_owned());
            }
            self.project = Some(id);
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
        self.set_panel(Panel::Conversation, cx);
    }

    pub(super) fn sidebar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let project_start = page_start(self.navigation.project_page, self.catalog.projects.len());
        let tasks: Vec<_> = self
            .catalog
            .tasks
            .iter()
            .filter(|task| {
                Some(task.project_id) == self.project && task.state != TaskState::Archived
            })
            .collect();
        let task_start = page_start(self.navigation.task_page, tasks.len());
        div()
            .id("workspace-navigation")
            .role(gpui::Role::Navigation)
            .aria_label("Projects and chats")
            .tab_group()
            .w(px(ui::SIDEBAR_WIDTH))
            .flex_shrink_0()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(rgb(DARK.sidebar))
            .border_r_1()
            .border_color(rgb(DARK.border))
            .child(
                div()
                    .h(px(42.0))
                    .flex_shrink_0()
                    .px_4()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Synara"),
                    ),
            )
            .child(
                div()
                    .px_2()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::action(
                        "new-thread",
                        "New thread",
                        Some(Glyph::Compose),
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            this.navigation.task_page = 0;
                            if this.project.is_some() {
                                this.task_title.update(cx, |entry, cx| entry.clear(cx));
                                this.create_task(cx);
                                this.set_panel(Panel::Conversation, cx);
                            } else {
                                this.browse_workspace(cx);
                            }
                        }),
                    ))
                    .child(ui::action(
                        "add-project",
                        "Add project",
                        Some(Glyph::Folder),
                        false,
                        cx.listener(|this, _: &(), _, cx| this.browse_workspace(cx)),
                    ))
                    .child(ui::action(
                        "workspace-path-toggle",
                        "Open by path",
                        None,
                        self.navigation.path_open,
                        cx.listener(|this, _: &(), window, cx| {
                            this.navigation.path_open = !this.navigation.path_open;
                            if this.navigation.path_open {
                                window.focus(&this.workspace_path.read(cx).focus_handle(cx), cx);
                            }
                            cx.notify();
                        }),
                    ))
                    .children(self.navigation.path_open.then(|| {
                        div()
                            .px_1()
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
                    })),
            )
            .child(
                div()
                    .mt_3()
                    .px_2()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .child(
                        ui::action(
                            "projects-disclosure",
                            "Projects",
                            Some(if self.navigation.projects_open {
                                Glyph::Chevron
                            } else {
                                Glyph::ChevronRight
                            }),
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.navigation.projects_open = !this.navigation.projects_open;
                                cx.notify();
                            }),
                        )
                        .flex_1(),
                    )
                    .child(
                        ui::action(
                            "refresh-catalog",
                            "Refresh",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, _| {
                                let workspace = this.controller.workspace.clone();
                                this.job(
                                    async move { Ok(Update::Catalog(workspace.catalog().await?)) },
                                );
                            }),
                        )
                        .text_size(px(11.0)),
                    ),
            )
            .children(self.navigation.projects_open.then(|| {
                div()
                    .id("project-list")
                    .max_h(px(200.0))
                    .min_h_0()
                    .px_2()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(
                        self.catalog
                            .projects
                            .iter()
                            .skip(project_start)
                            .take(PAGE_SIZE)
                            .map(|project| {
                                let id = project.id;
                                ui::action(
                                    SharedString::from(format!("project-{id}")),
                                    project.name.clone(),
                                    Some(Glyph::Folder),
                                    self.project == Some(id),
                                    cx.listener(move |this, _: &(), _, cx| {
                                        this.navigate_project(id, cx)
                                    }),
                                )
                            }),
                    )
                    .children(
                        self.catalog
                            .projects
                            .is_empty()
                            .then(|| ui::section_label("No projects yet")),
                    )
                    .children((project_start > 0).then(|| {
                        ui::action(
                            "previous-projects",
                            "Previous projects",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.navigation.project_page =
                                    this.navigation.project_page.saturating_sub(1);
                                cx.notify();
                            }),
                        )
                    }))
                    .children(
                        (project_start + PAGE_SIZE < self.catalog.projects.len()).then(|| {
                            ui::action(
                                "next-projects",
                                "More projects",
                                None,
                                false,
                                cx.listener(|this, _: &(), _, cx| {
                                    this.navigation.project_page =
                                        this.navigation.project_page.saturating_add(1);
                                    cx.notify();
                                }),
                            )
                        }),
                    )
            }))
            .child(
                div()
                    .mt_3()
                    .px_2()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .child(
                        ui::action(
                            "chats-disclosure",
                            "Chats",
                            Some(if self.navigation.chats_open {
                                Glyph::Chevron
                            } else {
                                Glyph::ChevronRight
                            }),
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.navigation.chats_open = !this.navigation.chats_open;
                                cx.notify();
                            }),
                        )
                        .flex_1(),
                    )
                    .child(
                        ui::action(
                            "thread-title-toggle",
                            "New...",
                            None,
                            self.navigation.title_open,
                            cx.listener(|this, _: &(), window, cx| {
                                this.navigation.title_open = !this.navigation.title_open;
                                if this.navigation.title_open {
                                    window.focus(&this.task_title.read(cx).focus_handle(cx), cx);
                                }
                                cx.notify();
                            }),
                        )
                        .text_size(px(11.0)),
                    ),
            )
            .children(self.navigation.title_open.then(|| {
                div()
                    .px_3()
                    .py_2()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(self.task_title.clone())
                    .child(ui::action(
                        "create-task",
                        "Create thread",
                        Some(Glyph::Compose),
                        false,
                        cx.listener(|this, _: &(), _, cx| this.create_task(cx)),
                    ))
            }))
            .child(
                div()
                    .id("task-list")
                    .flex_1()
                    .min_h_0()
                    .px_2()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .when(self.navigation.chats_open, |list| {
                        list.children(tasks.iter().skip(task_start).take(PAGE_SIZE).map(|task| {
                            let id = task.id;
                            let state = if self.busy.contains(&id) {
                                "Working"
                            } else if self.connecting.contains(&id) {
                                "Connecting"
                            } else {
                                match task.state {
                                    TaskState::Failed => "Failed",
                                    TaskState::Completed => "Complete",
                                    TaskState::Waiting => "Waiting",
                                    _ => "Ready",
                                }
                            };
                            let label = SharedString::from(format!("{} · {state}", task.title));
                            ui::action(
                                SharedString::from(format!("task-{id}")),
                                task.title.clone(),
                                None,
                                self.selected == Some(id),
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.select_task(id, cx);
                                    this.set_panel(Panel::Conversation, cx);
                                }),
                            )
                            .aria_label(label)
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(px(10.0))
                                    .text_color(rgb(DARK.muted))
                                    .child(state),
                            )
                        }))
                        .children(
                            tasks
                                .is_empty()
                                .then(|| ui::section_label("No chats in this project")),
                        )
                        .children((task_start > 0).then(|| {
                            ui::action(
                                "previous-chats",
                                "Previous chats",
                                None,
                                false,
                                cx.listener(|this, _: &(), _, cx| {
                                    this.navigation.task_page =
                                        this.navigation.task_page.saturating_sub(1);
                                    cx.notify();
                                }),
                            )
                        }))
                        .children(
                            (task_start + PAGE_SIZE < tasks.len()).then(|| {
                                ui::action(
                                    "next-chats",
                                    "More chats",
                                    None,
                                    false,
                                    cx.listener(|this, _: &(), _, cx| {
                                        this.navigation.task_page =
                                            this.navigation.task_page.saturating_add(1);
                                        cx.notify();
                                    }),
                                )
                            }),
                        )
                    }),
            )
            .child(
                div()
                    .px_2()
                    .py_2()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(rgb(DARK.border))
                    .child(ui::action(
                        "settings-navigation",
                        "Settings",
                        Some(Glyph::Settings),
                        self.panel == Panel::Settings,
                        cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Settings, cx)),
                    )),
            )
            .into_any_element()
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
