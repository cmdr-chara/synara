//! First-run setup reuses the existing Settings, agent registry, theme, and
//! workspace controls. Detection only checks a configured command on disk;
//! authentication remains owned by the connected agent.
mod auth;
use super::*;
use crate::ui::{self, palette};
pub(super) use auth::Reply;
use std::path::{Component, Path, PathBuf};

const STEPS: [&str; 6] = [
    "Welcome",
    "What you can do",
    "Agents",
    "Appearance",
    "Project",
    "Ready",
];

fn command_found(command: &Path) -> bool {
    if command.is_absolute() || command.components().count() > 1 {
        return command.is_file();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|directory| directory.join(command).is_file())
}

fn validate_new_project_folder(path: &Path) -> Result<(), &'static str> {
    if !path.is_absolute() {
        return Err("Enter an absolute path for the new project folder.");
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("Use a path without `..` components.");
    }
    if path.file_name().is_none() {
        return Err("Choose a new folder path, not a filesystem root.");
    }
    if path.exists() {
        return Err("That path already exists. Choose another path or select an existing folder.");
    }
    let Some(parent) = path.parent() else {
        return Err("Choose a new folder inside an existing parent directory.");
    };
    if !parent.is_dir() {
        return Err("The parent directory must already exist.");
    }
    Ok(())
}

fn is_registered_local_project(path: &Path, catalog: &Catalog) -> bool {
    let Ok(root) = path.canonicalize() else {
        return false;
    };
    catalog.projects.iter().any(|project| {
        project.relative_directory.as_os_str().is_empty()
            && catalog.workspaces.iter().any(|workspace| {
                workspace.id == project.workspace_id
                    && matches!(
                        &workspace.location,
                        WorkspaceLocation::Local { root: registered_root }
                            if registered_root == &root
                    )
            })
    })
}

impl Shell {
    pub(super) fn replay_onboarding(&mut self, cx: &mut Context<Self>) {
        if !self.onboarding_navigation_ready(cx) {
            return;
        }
        self.settings.onboarding_step = 0;
        self.settings.onboarding_import_open = false;
        self.set_panel(Panel::Settings, cx);
        self.open_settings_section(settings::Section::Onboarding, cx);
    }

    pub(super) fn finish_onboarding(&mut self, cx: &mut Context<Self>) {
        if self.settings.saving || !self.onboarding_navigation_ready(cx) {
            return;
        }
        self.settings.onboarding_step = STEPS.len() - 1;
        self.settings.onboarding_finishing = true;
        self.save_setting(
            |settings| {
                settings.onboarding.started = true;
                settings.onboarding.completed = true;
            },
            cx,
        );
    }

    pub(super) fn onboarding_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let step = self.settings.onboarding_step.min(STEPS.len() - 1);
        let content = match step {
            0 => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(onboarding_card("Your workspace stays local", "Projects, tasks and transcripts live in your selected data directory. You can bring the coding agents already installed on this machine."))
                .child(onboarding_card("Review before delivery", "Inspect changes, use a terminal and browser, and decide when work is ready to commit or share."))
                .into_any_element(),
            1 => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(onboarding_card("Tasks and projects", "Keep each conversation with its project, files and review history. Start a local chat or add a project folder."))
                .child(onboarding_card("Tools in one window", "Use the file editor, diff review, terminal, browser and task history while your agent works."))
                .child(onboarding_card("Stay in control", "Agent requests and external actions still require the approvals set by their owners. Setup starts no agent until you explicitly use Connect. Sign-in uses only the selected agent's advertised methods."))
                .into_any_element(),
            2 => self.onboarding_agents(cx),
            3 => self.onboarding_appearance(cx),
            4 => self.onboarding_project(cx),
            _ => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(onboarding_card("Ready to work", "Your setup choices are saved locally. You can replay this guide from Settings → Getting started at any time."))
                .into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette().muted))
                    .child(format!(
                        "Step {} of {} · {}",
                        step + 1,
                        STEPS.len(),
                        STEPS[step]
                    )),
            )
            .child(content)
            .child(
                div()
                    .pt_3()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children((step > 0).then(|| {
                        ui::button("onboarding-back", "Back", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                if !this.onboarding_navigation_ready(cx) {
                                    return;
                                }
                                this.settings.onboarding_step =
                                    this.settings.onboarding_step.saturating_sub(1);
                                cx.notify();
                            },
                        ))
                    }))
                    .child(if step < STEPS.len() - 1 {
                        ui::button("onboarding-next", "Continue", false)
                            .relative()
                            .child(ui::layout_probe("onboarding-next"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if !this.onboarding_navigation_ready(cx) {
                                    return;
                                }
                                this.settings.onboarding_step += 1;
                                cx.notify();
                            }))
                    } else {
                        ui::button(
                            "onboarding-finish",
                            if self.settings.saving {
                                "Saving..."
                            } else {
                                "Finish setup"
                            },
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.finish_onboarding(cx)))
                    })
                    .children((step < STEPS.len() - 1).then(|| {
                        ui::button("onboarding-skip", "Skip setup", false)
                            .on_click(cx.listener(|this, _, _, cx| this.finish_onboarding(cx)))
                    })),
            )
            .into_any_element()
    }

    fn onboarding_agents(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let found = self
            .profiles
            .iter()
            .filter(|profile| command_found(&profile.command))
            .count();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(onboarding_card(
                "Configured coding agents",
                "A command found on disk may still need its own login. Connect starts the selected agent and exposes only its advertised ACP sign-in methods. If it has no ACP sign-in method, use its own CLI login in the setup terminal. Synara never collects provider credentials.",
            ))
            .child(
                div()
                    .text_color(rgb(palette().muted))
                    .child(format!("{} of {} configured commands found", found, self.profiles.len())),
            )
            .children(self.profiles.iter().enumerate().map(|(index, profile)| {
                let available = command_found(&profile.command);
                let status = if available {
                    "Command found"
                } else {
                    "Command not found"
                };
                let provider_id = profile.id.clone();
                let selected = self.settings.value.general.default_provider.as_ref()
                    == Some(&provider_id);
                div()
                    .id(("onboarding-agent", index))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(palette().border))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(format!("{} · {status}", profile.name))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child(profile.command.display().to_string()),
                    )
                    .child(self.onboarding_provider_guide(index, profile, cx))
                    .children(available.then(|| self.onboarding_agent_access(index, &profile.id, cx)))
                    .children(available.then(|| {
                        ui::button(
                            ("onboarding-agent-default", index),
                            if selected { "Default agent" } else { "Use as default" },
                            selected,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.save_setting(
                                |settings| {
                                    settings.general.default_provider = Some(provider_id.clone())
                                },
                                cx,
                            );
                        }))
                    }))
            }))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(ui::button("onboarding-provider-settings", "Agent settings", false)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_settings_section(settings::Section::Providers, cx);
                        })))
                    .child(ui::button("onboarding-agent-registry", "Manage agents", false)
                        .on_click(cx.listener(|this, _, _, cx| this.set_panel(Panel::Registry, cx)))),
            )
            .into_any_element()
    }

    fn onboarding_appearance(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(onboarding_card(
                "Choose a theme",
                "Theme changes use the same saved Appearance setting as the rest of Synara.",
            ))
            .child(
                div().flex().flex_wrap().gap_2().children(
                    [
                        (ThemePreference::System, "System", "onboarding-system"),
                        (ThemePreference::Light, "Light", "onboarding-light"),
                        (ThemePreference::Dark, "Dark", "onboarding-dark"),
                    ]
                    .into_iter()
                    .map(|(theme, label, id)| {
                        ui::button(id, label, self.settings.value.appearance.theme == theme)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.save_setting(|settings| settings.appearance.theme = theme, cx)
                            }))
                    }),
                ),
            )
            .child(
                ui::button(
                    "onboarding-more-appearance",
                    "More appearance settings",
                    false,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.open_settings_section(settings::Section::Appearance, cx);
                })),
            )
            .into_any_element()
    }

    fn onboarding_project(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let entered_path = self.workspace_path.read(cx).text().trim().to_owned();
        let entered_path_buf = PathBuf::from(&entered_path);
        let existing_directory = entered_path_buf.is_dir();
        let registered =
            existing_directory && is_registered_local_project(&entered_path_buf, &self.catalog);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(onboarding_card("Add your first project", "A project is a local folder. Open one that already exists, create a new folder under an existing parent, or continue without a project."))
            .child(ui::button("onboarding-project-browse", "Choose project folder", false)
                .on_click(cx.listener(|this, _, _, cx| this.pick_onboarding_project_folder(cx))))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child("Or enter an absolute folder path")
                    .child(self.workspace_path.clone())
                    .children((!entered_path.is_empty()).then(|| {
                        if existing_directory {
                            ui::button(
                                "onboarding-project-add-existing",
                                if registered {
                                    "Open project now"
                                } else {
                                    "Add existing folder"
                                },
                                false,
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if registered {
                                    this.open_workspace(cx);
                                } else {
                                    this.add_onboarding_project(
                                        entered_path_buf.clone(),
                                        false,
                                        cx,
                                    );
                                }
                            }))
                        } else {
                            ui::button("onboarding-project-create", "Create project folder", false)
                                .on_click(cx.listener(|this, _, _, cx| this.create_onboarding_project(cx)))
                        }
                    })),
            )
            .children(registered.then(|| div()
                .text_color(rgb(palette().muted))
                .child("Project added to this workspace. Continue setup or open it now to start a task.")))
            .children(self.error.as_ref().map(|error| div()
                .id("onboarding-project-error")
                .role(gpui::Role::Alert)
                .text_color(rgb(palette().error))
                .child(error.clone())))
            .child(ui::button("onboarding-project-import", if self.settings.onboarding_import_open { "Hide history import" } else { "Import conversation history here" }, self.settings.onboarding_import_open)
                .relative().child(ui::layout_probe("onboarding-project-import"))
                .on_click(cx.listener(|this, _, _, cx| {
                    if this.project_import.pending() {
                        this.notice = Some("Finish or cancel the history read/review before hiding it.".into());
                    } else {
                        this.settings.onboarding_import_open = !this.settings.onboarding_import_open;
                    }
                    cx.notify();
                })))
            .when(self.settings.onboarding_import_open, |el| el.child(
                div().id("onboarding-history-import").relative().child(ui::layout_probe("onboarding-history-import"))
                    .flex().flex_col().gap_2().border_t_1().border_color(rgb(palette().border)).pt_3()
                    .child(self.project_import_settings(cx))))
            .into_any_element()
    }

    fn onboarding_navigation_ready(&mut self, cx: &mut Context<Self>) -> bool {
        if self.settings.onboarding_import_open && self.project_import.pending() {
            self.notice = Some("Finish the history import or cancel/back out of its review before continuing setup.".into());
            cx.notify();
            return false;
        }
        true
    }

    fn create_onboarding_project(&mut self, cx: &mut Context<Self>) {
        let entered_path = self.workspace_path.read(cx).text().trim().to_owned();
        let path = PathBuf::from(entered_path);
        if let Err(error) = validate_new_project_folder(&path) {
            self.error = Some(error.into());
            cx.notify();
            return;
        }
        if let Err(error) = std::fs::create_dir(&path) {
            self.error = Some(if error.kind() == std::io::ErrorKind::AlreadyExists {
                "That path already exists. Choose another path or select an existing folder.".into()
            } else {
                format!("Could not create the project folder: {error}")
            });
            cx.notify();
            return;
        }

        self.error = None;
        self.add_onboarding_project(path, true, cx);
    }

    fn add_onboarding_project(
        &mut self,
        path: PathBuf,
        created_here: bool,
        cx: &mut Context<Self>,
    ) {
        self.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            workspace
                .add_local_workspace(path)
                .await
                .map_err(|error| {
                    WorkspaceError::Invalid(if created_here {
                        format!("The folder was created, but Synara could not add it: {error}. It remains on disk; select it again as an existing folder to retry.")
                    } else {
                        format!("Could not add the selected folder: {error}")
                    })
                })?;
            let catalog = workspace.catalog().await.map_err(|error| {
                WorkspaceError::Invalid(if created_here {
                    format!("The folder was created and registered, but Synara could not refresh the project list: {error}")
                } else {
                    format!("The folder was added, but Synara could not refresh the project list: {error}")
                })
            })?;
            Ok(Update::Catalog(catalog))
        });
        cx.notify();
    }

    fn pick_onboarding_project_folder(&self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose a project folder".into()),
        });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        this.workspace_path.update(cx, |entry, cx| {
                            entry.set_text(path.to_string_lossy().into_owned(), cx)
                        });
                        this.error = None;
                    }
                    cx.notify();
                }
                Ok(Ok(None)) => {}
                _ => {
                    this.error = Some(
                        "The folder picker could not open. Enter the absolute path instead.".into(),
                    );
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

fn onboarding_card(title: &'static str, detail: &'static str) -> gpui::Div {
    div()
        .rounded_lg()
        .border_1()
        .border_color(rgb(palette().border))
        .p_4()
        .flex()
        .flex_col()
        .gap_2()
        .child(title)
        .child(
            div()
                .text_color(rgb(palette().muted))
                .line_height(px(22.))
                .child(detail),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_nonexistent_child(parent: &Path) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        parent.join(format!("synara-onboarding-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn new_project_folder_requires_an_absolute_normalized_path_and_existing_parent() {
        assert!(validate_new_project_folder(Path::new("relative/project")).is_err());

        let parent = std::env::temp_dir();
        let new_folder = unique_nonexistent_child(&parent);
        assert_eq!(validate_new_project_folder(&new_folder), Ok(()));
        assert!(validate_new_project_folder(&new_folder.join("missing-parent/project")).is_err());
        assert!(validate_new_project_folder(&parent).is_err());

        let with_parent = new_folder.join("..").join("another-project");
        assert!(validate_new_project_folder(&with_parent).is_err());
    }
}
