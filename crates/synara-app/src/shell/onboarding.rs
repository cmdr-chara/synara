//! First-run setup reuses the existing Settings, agent registry, theme, and
//! workspace controls. Detection only checks a configured command on disk;
//! authentication remains owned by the connected agent.
use super::*;
use crate::ui::{self, palette};
use std::path::Path;

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

impl Shell {
    pub(super) fn replay_onboarding(&mut self, cx: &mut Context<Self>) {
        self.settings.onboarding_step = 0;
        self.set_panel(Panel::Settings, cx);
        self.open_settings_section(settings::Section::Onboarding, cx);
    }

    pub(super) fn finish_onboarding(&mut self, cx: &mut Context<Self>) {
        if self.settings.saving {
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
                .child(onboarding_card("Stay in control", "Agent requests and external actions still require the approvals set by their owners. Setup never starts an agent or signs you in."))
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
                                this.settings.onboarding_step =
                                    this.settings.onboarding_step.saturating_sub(1);
                                cx.notify();
                            },
                        ))
                    }))
                    .child(if step < STEPS.len() - 1 {
                        ui::button("onboarding-next", "Continue", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.settings.onboarding_step += 1;
                                cx.notify();
                            },
                        ))
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
                "A command found on disk may still need its own login. Synara checks authentication only after connecting to that agent; no account state is assumed here.",
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
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(onboarding_card("Open a project folder", "Choose an existing folder. For a new project, create its folder on disk first, then select it. Synara adds the folder as a local project and opens a task in it. You can also continue without a project."))
            .child(ui::button("onboarding-project-browse", "Choose project folder", false)
                .on_click(cx.listener(|this, _, _, cx| this.browse_workspace(cx))))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child("Or enter an existing absolute directory")
                    .child(self.workspace_path.clone())
                    .child(ui::button("onboarding-project-path", "Open project", false)
                        .on_click(cx.listener(|this, _, _, cx| this.open_workspace(cx)))),
            )
            .child(ui::button("onboarding-project-import", "Import conversation history", false)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.open_settings_section(settings::Section::ProjectImport, cx);
                })))
            .into_any_element()
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
