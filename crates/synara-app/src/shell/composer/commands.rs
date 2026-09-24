//! Explicit native workflow commands. ACP names cannot contain '/', so the
//! /synara/ namespace never shadows a provider-advertised command.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Plan,
    ModelNext,
    ModelPrevious,
    Debug,
    Goal,
    GoalPause,
    GoalResume,
    GoalClear,
    GoalEdit,
    Fork,
    Subagents,
    Export,
    ExportZip,
    Automation,
    AutomationList,
    AutomationNew,
    Computer,
    Settings,
    Recap,
    Status,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedCommand {
    Native(Command),
    SetGoal(String),
    EditGoal(String),
    AutomationEdit(AutomationId),
    SettingsSection(&'static str),
}
const GOAL_MAX_BYTES: usize = 4096;
const AUTOMATION_USAGE: &str =
    "Automation usage: /synara/automation [list | new | edit <id>]. Nothing was sent.";
const AUTOMATION_EDIT_USAGE: &str =
    "Automation usage: /synara/automation edit <id>. Nothing was sent.";
const SETTINGS_USAGE: &str = "Settings usage: /synara/settings <section>. Choose one section from the command menu. Nothing was sent.";
const SETTINGS_SECTIONS: &[(&str, &str)] = &[
    ("onboarding", "Getting started"),
    ("device", "Device and capture"),
    ("privacy", "Privacy and security"),
    ("general", "General preferences"),
    ("profile", "Local activity and profile"),
    ("appearance", "Theme and typography"),
    ("notifications", "Notification preferences"),
    ("behavior", "Chat behavior"),
    ("keybindings", "Keyboard shortcuts"),
    ("usage", "Reported usage and limits"),
    ("appsnap", "AppSnap capture"),
    ("computer", "Computer use setup"),
    ("plugins", "Plugins and integrations"),
    ("mcp", "MCP connections"),
    ("providers", "Agent providers"),
    ("models", "Models and writing"),
    ("direct-models", "Direct model providers"),
    ("project-import", "Project import"),
    ("skills", "Agent skills"),
    ("worktrees", "Managed worktrees"),
    ("system", "System tools"),
    ("archived", "Archived threads"),
    ("workflows", "Subagents and workflows"),
];
const COMMANDS: &[(&str, &str, Command)] = &[
    ("plan", "Select the advertised ACP Plan mode", Command::Plan),
    (
        "model next",
        "Select the next advertised ACP model (Alt+])",
        Command::ModelNext,
    ),
    (
        "model previous",
        "Select the previous advertised ACP model (Alt+[)",
        Command::ModelPrevious,
    ),
    (
        "debug",
        "Open the evidence-first Debug workflow",
        Command::Debug,
    ),
    (
        "goal",
        "Review this task's goal and explicit set/edit/pause/resume/clear actions",
        Command::Goal,
    ),
    (
        "goal pause",
        "Pause active goal continuation without sending a provider prompt",
        Command::GoalPause,
    ),
    (
        "goal resume",
        "Prepare the saved goal with its bounded pursuit; Send stays explicit",
        Command::GoalResume,
    ),
    (
        "goal clear",
        "Clear this task's saved goal and history, only while paused",
        Command::GoalClear,
    ),
    (
        "goal edit",
        "Open the goal editor; optional text remains unsaved until Save",
        Command::GoalEdit,
    ),
    (
        "fork",
        "Create an unsent branch through the last assistant turn, same checkout",
        Command::Fork,
    ),
    (
        "subagents",
        "Open workflow review, without starting agents",
        Command::Subagents,
    ),
    (
        "export",
        "Save the text conversation as Markdown",
        Command::Export,
    ),
    (
        "export-zip",
        "Save a completed conversation as Markdown and JSON in a ZIP",
        Command::ExportZip,
    ),
    (
        "automation",
        "Review automations, without arming the scheduler",
        Command::Automation,
    ),
    (
        "automation list",
        "Open saved automations, without arming the scheduler",
        Command::AutomationList,
    ),
    (
        "automation new",
        "Open a new unsaved automation form for this project",
        Command::AutomationNew,
    ),
    (
        "computer-use",
        "Open Computer Use setup, without granting control",
        Command::Computer,
    ),
    (
        "settings",
        "Open General settings or choose a section",
        Command::Settings,
    ),
    ("recap", "Review the conversation recap", Command::Recap),
    (
        "status",
        "Show reported usage, without inventing provider telemetry",
        Command::Status,
    ),
];
fn parse(text: &str) -> Option<Result<ParsedCommand, &'static str>> {
    let text = text.trim();
    let name = text.strip_prefix("/synara/")?;
    let parsed = if let Some(rest) = name.strip_prefix("goal") {
        if rest.is_empty() {
            COMMANDS
                .iter()
                .find(|(candidate, _, _)| *candidate == "goal")
                .map(|(_, _, command)| ParsedCommand::Native(*command))
                .ok_or("Unknown native command. Nothing was sent.")
        } else if rest.chars().next().is_some_and(char::is_whitespace) {
            let arguments = rest.trim_start();
            if arguments == "pause" {
                return Some(Ok(ParsedCommand::Native(Command::GoalPause)));
            }
            for (name, command) in [
                ("resume", Command::GoalResume),
                ("clear", Command::GoalClear),
                ("edit", Command::GoalEdit),
            ] {
                if arguments == name {
                    return Some(Ok(ParsedCommand::Native(command)));
                }
            }
            let editing = arguments.starts_with("edit");
            let Some(objective) = arguments.strip_prefix(if editing { "edit" } else { "set" })
            else {
                return Some(Err(
                    "Goal usage: /synara/goal [pause | resume | clear | edit [objective] | set <objective>]. Nothing was sent.",
                ));
            };
            if objective.is_empty() {
                return Some(Err(
                    "Goal usage: /synara/goal set <objective>. Nothing was sent.",
                ));
            }
            if !objective.chars().next().is_some_and(char::is_whitespace) {
                return Some(Err(
                    "Goal usage: /synara/goal [pause | resume | clear | edit [objective] | set <objective>]. Nothing was sent.",
                ));
            }
            let objective = objective.trim();
            if objective.is_empty() || objective.len() > GOAL_MAX_BYTES || objective.contains('\0')
            {
                return Some(Err(
                    "Goal text must be non-empty, contain no NUL, and fit within 4 KiB. Nothing was sent.",
                ));
            }
            Ok(if editing {
                ParsedCommand::EditGoal(objective.to_owned())
            } else {
                ParsedCommand::SetGoal(objective.to_owned())
            })
        } else {
            Err(
                "Unknown native command or extra arguments. Use an exact /synara/ command from the menu. Nothing was sent.",
            )
        }
    } else if let Some(rest) = name.strip_prefix("automation") {
        if rest.is_empty() {
            COMMANDS
                .iter()
                .find(|(candidate, _, _)| *candidate == "automation")
                .map(|(_, _, command)| ParsedCommand::Native(*command))
                .ok_or("Unknown native command. Nothing was sent.")
        } else if !rest.chars().next().is_some_and(char::is_whitespace) {
            Err(
                "Unknown native command or extra arguments. Use an exact /synara/ command from the menu. Nothing was sent.",
            )
        } else {
            let arguments = rest.trim_start();
            if arguments == "list" {
                return Some(Ok(ParsedCommand::Native(Command::AutomationList)));
            }
            if arguments == "new" {
                return Some(Ok(ParsedCommand::Native(Command::AutomationNew)));
            }
            let Some(id) = arguments.strip_prefix("edit") else {
                return Some(Err(AUTOMATION_USAGE));
            };
            if !id.chars().next().is_some_and(char::is_whitespace) {
                return Some(Err(AUTOMATION_EDIT_USAGE));
            }
            let id = id.trim();
            if id.is_empty() || id.chars().any(char::is_whitespace) {
                return Some(Err(AUTOMATION_EDIT_USAGE));
            }
            match id.parse::<AutomationId>() {
                Ok(id) => Ok(ParsedCommand::AutomationEdit(id)),
                Err(_) => Err(AUTOMATION_EDIT_USAGE),
            }
        }
    } else if let Some(rest) = name.strip_prefix("settings") {
        if rest.is_empty() {
            COMMANDS
                .iter()
                .find(|(candidate, _, _)| *candidate == "settings")
                .map(|(_, _, command)| ParsedCommand::Native(*command))
                .ok_or("Unknown native command. Nothing was sent.")
        } else if !rest.chars().next().is_some_and(char::is_whitespace) {
            Err(
                "Unknown native command or extra arguments. Use an exact /synara/ command from the menu. Nothing was sent.",
            )
        } else {
            let section = rest.trim();
            if section.is_empty() || section.chars().any(char::is_whitespace) {
                return Some(Err(SETTINGS_USAGE));
            }
            SETTINGS_SECTIONS
                .iter()
                .find(|(name, _)| *name == section)
                .map(|(name, _)| ParsedCommand::SettingsSection(name))
                .ok_or(SETTINGS_USAGE)
        }
    } else {
        COMMANDS
            .iter()
            .find(|(candidate, _, _)| *candidate == name)
            .map(|(_, _, command)| ParsedCommand::Native(*command))
            .ok_or("Unknown native command or extra arguments. Use an exact /synara/ command from the menu. Nothing was sent.")
    };
    Some(parsed)
}
fn settings_section(name: &str) -> Option<settings::Section> {
    Some(match name {
        "onboarding" => settings::Section::Onboarding,
        "device" => settings::Section::Device,
        "privacy" => settings::Section::Privacy,
        "general" => settings::Section::General,
        "profile" => settings::Section::Profile,
        "appearance" => settings::Section::Appearance,
        "notifications" => settings::Section::Notifications,
        "behavior" => settings::Section::Behavior,
        "keybindings" => settings::Section::Keybindings,
        "usage" => settings::Section::Usage,
        "appsnap" => settings::Section::AppSnap,
        "computer" => settings::Section::Computer,
        "plugins" => settings::Section::Plugins,
        "mcp" => settings::Section::Mcp,
        "providers" => settings::Section::Providers,
        "models" => settings::Section::Models,
        "direct-models" => settings::Section::DirectModels,
        "project-import" => settings::Section::ProjectImport,
        "skills" => settings::Section::Skills,
        "worktrees" => settings::Section::Worktrees,
        "system" => settings::Section::System,
        "archived" => settings::Section::Archived,
        "workflows" => settings::Section::Workflows,
        _ => return None,
    })
}
impl Shell {
    pub(in crate::shell) fn native_command_draft(&self, cx: &App) -> bool {
        parse(self.composer.read(cx).text()).is_some()
    }
    pub(in crate::shell) fn consume_native_command(&mut self, cx: &mut Context<Self>) -> bool {
        let text = self.composer.read(cx).text().to_owned();
        let Some(parsed) = parse(&text) else {
            return false;
        };
        let command = match parsed {
            Ok(command) => command,
            Err(error) => {
                self.error = Some(error.into());
                cx.notify();
                return true;
            }
        };
        // The caller already applies the ordinary task/loading/IME/busy guards.
        // In particular, this route cannot turn the Stop button into an action.
        let Some(task) = self.selected else {
            return true;
        };
        if self.creating_task {
            self.error = Some("Wait for task creation to finish. The command was kept.".into());
            cx.notify();
            return true;
        }
        self.error = None;
        let accepted = match command {
            ParsedCommand::SetGoal(objective) => self.set_goal_from_command(objective, cx),
            ParsedCommand::EditGoal(objective) => self.edit_goal_from_command(Some(objective), cx),
            ParsedCommand::AutomationEdit(id) => self.open_automation_for_review(id, cx),
            ParsedCommand::SettingsSection(section) => match settings_section(section) {
                Some(section) => {
                    self.set_panel(Panel::Settings, cx);
                    if self.panel == Panel::Settings {
                        self.open_settings_section(section, cx);
                        true
                    } else {
                        false
                    }
                }
                None => {
                    self.error = Some(SETTINGS_USAGE.into());
                    false
                }
            },
            ParsedCommand::Native(command) => match command {
                Command::Plan => self.native_plan_mode(cx),
                Command::ModelNext => self.cycle_session_model(true, cx),
                Command::ModelPrevious => self.cycle_session_model(false, cx),
                Command::Debug => {
                    self.open_debug(cx);
                    self.debug_workflow.open
                }
                Command::Goal => {
                    self.open_goals(cx);
                    self.goals.open
                }
                Command::GoalPause => self.pause_goal_from_command(cx),
                Command::GoalResume => self.resume_goal_from_command(cx),
                Command::GoalClear => self.clear_goal_from_command(cx),
                Command::GoalEdit => self.edit_goal_from_command(None, cx),
                Command::Recap => {
                    self.open_recap(cx);
                    self.recap.open
                }
                Command::Fork => {
                    let anchor = self
                        .thread
                        .as_ref()
                        .filter(|thread| {
                            self.task().is_some_and(|task| task.thread_id == thread.id)
                        })
                        .and_then(|thread| {
                            thread
                                .messages
                                .iter()
                                .rev()
                                .find(|message| message.role == Role::Assistant)
                        })
                        .map(MessageAnchor::from);
                    if let Some(anchor) = anchor {
                        self.branch_message(task, anchor, cx);
                        self.creating_task
                    } else {
                        self.error = Some("A saved assistant turn is required for a context-derived branch. Nothing was created.".into());
                        false
                    }
                }
                Command::Subagents => {
                    self.open_settings_section(settings::Section::Workflows, cx);
                    self.panel == Panel::Settings
                }
                Command::Computer => {
                    self.open_settings_section(settings::Section::Computer, cx);
                    self.panel == Panel::Settings
                }
                Command::Settings => {
                    self.set_panel(Panel::Settings, cx);
                    if self.panel == Panel::Settings {
                        self.open_settings_section(settings::Section::General, cx);
                        true
                    } else {
                        false
                    }
                }
                Command::Status => {
                    self.open_settings_section(settings::Section::Usage, cx);
                    self.panel == Panel::Settings
                }
                Command::Automation => {
                    self.set_panel(Panel::Automations, cx);
                    self.panel == Panel::Automations
                }
                Command::AutomationList => {
                    self.set_panel(Panel::Automations, cx);
                    self.panel == Panel::Automations
                }
                Command::AutomationNew => self.open_new_automation(cx),
                Command::ExportZip => self.export_zip_conversation(cx),
                Command::Export => {
                    self.export_conversation(cx);
                    true
                }
            },
        };
        if accepted && self.selected == Some(task) && self.composer.read(cx).text() == text {
            // Consume only this exact command. Attachments and other task drafts
            // remain untouched. No prompt, approval or scheduler arm is synthesized.
            self.composer.update(cx, |entry, cx| entry.clear(cx));
            self.snapshot_draft(cx);
        } else if !accepted && self.error.is_none() {
            self.error = Some(
                "This workflow is still loading or blocked. The command was kept for retry.".into(),
            );
        }
        cx.notify();
        true
    }
    fn pause_goal_from_command(&mut self, cx: &mut Context<Self>) -> bool {
        if let Some(error) = self.goal_pause_command_error(cx) {
            self.error = Some(error.into());
            return false;
        }
        self.open_goals(cx);
        self.pause_goals(
            "Paused by you with /synara/goal pause. No future continuation is armed.",
            false,
            cx,
        );
        true
    }
    pub(in crate::shell) fn native_commands_view(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let text = self.composer.read(cx).text().to_owned();
        let trimmed = text.trim();
        let mut view = div().id("native-command-menu");
        let prefix = if trimmed == "/" {
            ""
        } else if let Some(prefix) = trimmed.strip_prefix("/synara/") {
            prefix
        } else {
            return view.into_any_element();
        };
        view = view
            .max_h(px(180.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .child(div().text_sm().text_color(rgb(palette().muted)).child(
                "Synara commands · provider commands remain unchanged. Goal, automation, settings and model argument forms reuse their native owners. No command is sent to the provider as a prompt.",
            ));
        if prefix.starts_with("goal ") {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(rgb(palette().muted))
                    .child("Usage: /synara/goal pause, resume, clear, set <objective>, or edit [objective]. Edit requires Save. Resume prepares a bounded pursuit and never sends its first prompt."),
            );
        }
        if prefix.starts_with("automation ") {
            view = view.child(
                    div()
                        .text_sm()
                        .text_color(rgb(palette().muted))
                        .child("Usage: /synara/automation list, /synara/automation new, or /synara/automation edit <id>. Edit opens a saved automation for review; Save stays explicit. New opens an unsaved form; nothing is saved or scheduled."),
                );
        }
        if prefix == "settings" || prefix.starts_with("settings ") {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(rgb(palette().muted))
                    .child(SETTINGS_USAGE),
            );
        }
        let task = self.selected;
        for (index, (name, detail, _)) in COMMANDS.iter().enumerate() {
            if !name.starts_with(prefix) {
                continue;
            }
            let command = format!("/synara/{name}");
            let expected = text.clone();
            view = view.child(
                ui::action(
                    ("native-command", index),
                    format!("{command} · {detail}"),
                    None,
                    false,
                    cx.listener(move |this, _, _, cx| {
                        if this.selected != task
                            || this.composer.read(cx).text() != expected
                            || this.composer.read(cx).is_composing()
                            || task.is_some_and(|id| {
                                this.busy.contains(&id)
                                    || this.connecting.contains(&id)
                                    || this.controls.is_pending(id)
                            })
                        {
                            return;
                        }
                        this.composer
                            .update(cx, |entry, cx| entry.set_text(command.clone(), cx));
                        this.send_prompt(cx);
                    }),
                )
                .relative()
                .child(ui::layout_probe_slot("native-command-row", index)),
            );
        }
        for (index, (section, detail)) in SETTINGS_SECTIONS.iter().enumerate() {
            let name = format!("settings {section}");
            if !name.starts_with(prefix) {
                continue;
            }
            let command = format!("/synara/{name}");
            let expected = text.clone();
            let task = self.selected;
            view = view.child(
                ui::action(
                    ("native-settings-command", index),
                    format!("{command} · Open {detail}"),
                    None,
                    false,
                    cx.listener(move |this, _, _, cx| {
                        if this.selected != task
                            || this.composer.read(cx).text() != expected
                            || this.composer.read(cx).is_composing()
                            || task.is_some_and(|id| {
                                this.busy.contains(&id)
                                    || this.connecting.contains(&id)
                                    || this.controls.is_pending(id)
                            })
                        {
                            return;
                        }
                        this.composer
                            .update(cx, |entry, cx| entry.set_text(command.clone(), cx));
                        this.send_prompt(cx);
                    }),
                )
                .relative()
                .child(ui::layout_probe_slot("native-settings-command-row", index)),
            );
        }
        view.into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_cycle_commands_are_exact_and_never_steal_provider_commands() {
        assert_eq!(
            parse("/synara/model next"),
            Some(Ok(ParsedCommand::Native(Command::ModelNext)))
        );
        assert_eq!(
            parse("/synara/model previous"),
            Some(Ok(ParsedCommand::Native(Command::ModelPrevious)))
        );
        assert!(parse("/model next").is_none());
        for command in [
            "/synara/model",
            "/synara/model next now",
            "/synara/model previous all",
            "/synara/models next",
        ] {
            assert!(parse(command).unwrap().is_err());
        }
    }
    #[test]
    fn native_namespace_never_shadows_provider_commands_or_accepts_extra_prompt_text() {
        for text in [
            "/plan",
            "/debug",
            "/synara:debug",
            "ordinary /synara/debug text",
        ] {
            assert!(parse(text).is_none());
        }
        for (name, _, command) in COMMANDS {
            assert_eq!(
                parse(&format!(" /synara/{name}\n")),
                Some(Ok(ParsedCommand::Native(*command)))
            );
        }
        for text in [
            "/synara/",
            "/synara/unknown",
            "/synara/debug run this",
            "/synara/goal\nsecret",
        ] {
            assert!(parse(text).unwrap().is_err());
        }
    }

    #[test]
    fn goal_set_preserves_literal_unicode_objective_and_rejects_unsafe_input() {
        assert_eq!(
            parse("/synara/goal set  Preserve 日本語; $HOME literally  "),
            Some(Ok(ParsedCommand::SetGoal(
                "Preserve 日本語; $HOME literally".into()
            )))
        );
        for text in [
            "/synara/goal set",
            "/synara/goal archive",
            "/synara/goal set \0hidden",
        ] {
            assert!(parse(text).unwrap().is_err());
        }
        let too_long = format!("/synara/goal set {}", "x".repeat(GOAL_MAX_BYTES + 1));
        assert!(parse(&too_long).unwrap().is_err());
    }

    #[test]
    fn goal_pause_is_a_qualified_exact_form_and_never_shadows_provider_goal() {
        assert_eq!(
            parse("/synara/goal pause"),
            Some(Ok(ParsedCommand::Native(Command::GoalPause)))
        );
        for text in [
            "/synara/goal pause now",
            "/synara/goal pause\n/synara/status",
            "/goal pause",
            "/synara:goal pause",
        ] {
            assert!(
                parse(text).is_none_or(|result| result.is_err()),
                "accepted {text}"
            );
        }
    }

    #[test]
    fn automation_list_and_new_accept_only_exact_safe_forms() {
        assert_eq!(
            parse("/synara/automation list"),
            Some(Ok(ParsedCommand::Native(Command::AutomationList)))
        );
        assert_eq!(
            parse("/synara/automation new"),
            Some(Ok(ParsedCommand::Native(Command::AutomationNew)))
        );
        assert_eq!(
            parse("/synara/automation"),
            Some(Ok(ParsedCommand::Native(Command::Automation)))
        );
        for text in [
            "/synara/automation list all",
            "/synara/automation new now",
            "/synara/automation list\n/synara/status",
            "/automation list",
            "/automation new",
        ] {
            assert!(
                parse(text).is_none_or(|result| result.is_err()),
                "accepted {text}"
            );
        }
    }

    #[test]
    fn automation_edit_accepts_one_exact_uuid_argument_only() {
        let id: AutomationId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        assert_eq!(
            parse(&format!("/synara/automation edit {id}")),
            Some(Ok(ParsedCommand::AutomationEdit(id)))
        );
        assert_eq!(
            parse(&format!("/synara/automation   edit\t{id}  ")),
            Some(Ok(ParsedCommand::AutomationEdit(id)))
        );
        for text in [
            "/automation edit 550e8400-e29b-41d4-a716-446655440000",
            "/synara/automation edit",
            "/synara/automation edit not-a-uuid",
            "/synara/automation edit 550e8400-e29b-41d4-a716-446655440000 extra",
            "/synara/automation editx 550e8400-e29b-41d4-a716-446655440000",
            "/synara/automation delete 550e8400-e29b-41d4-a716-446655440000",
        ] {
            assert!(
                parse(text).is_none_or(|result| result.is_err()),
                "accepted {text}"
            );
        }
    }
    #[test]
    fn settings_arguments_select_only_known_native_sections() {
        assert_eq!(
            parse("/synara/settings"),
            Some(Ok(ParsedCommand::Native(Command::Settings)))
        );
        for (section, _) in SETTINGS_SECTIONS {
            assert!(settings_section(section).is_some());
            assert_eq!(
                parse(&format!("/synara/settings {section}")),
                Some(Ok(ParsedCommand::SettingsSection(section)))
            );
        }
        assert_eq!(
            parse("/synara/settings   direct-models  "),
            Some(Ok(ParsedCommand::SettingsSection("direct-models")))
        );
        for text in [
            "/synara/settings unknown",
            "/synara/settings usage extra",
            "/synara/settingsx usage",
            "/synara/settings\nusage\nextra",
        ] {
            assert!(
                parse(text).is_none_or(|result| result.is_err()),
                "accepted {text}"
            );
        }
        assert!(parse("/settings usage").is_none());
    }
    #[test]
    fn goal_resume_clear_and_edit_are_qualified_and_literal() {
        assert_eq!(
            parse("/synara/goal edit Literal 日本語 $HOME"),
            Some(Ok(ParsedCommand::EditGoal("Literal 日本語 $HOME".into())))
        );
        for text in [
            "/synara/goal resume now",
            "/synara/goal clear all",
            "/synara/goal editx",
            "/synara/goal edit \0hidden",
            "/goal resume",
            "/goal clear",
        ] {
            assert!(
                parse(text).is_none_or(|result| result.is_err()),
                "accepted {text}"
            );
        }
        assert!(
            parse(&format!(
                "/synara/goal edit {}",
                "x".repeat(GOAL_MAX_BYTES + 1)
            ))
            .unwrap()
            .is_err()
        );
    }
}
