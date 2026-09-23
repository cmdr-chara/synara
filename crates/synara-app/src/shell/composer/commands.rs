//! Explicit native workflow commands. ACP names cannot contain '/', so the
//! /synara/ namespace never shadows a provider-advertised command.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Plan,
    Debug,
    Goal,
    GoalPause,
    Fork,
    Subagents,
    Export,
    ExportZip,
    Automation,
    AutomationList,
    AutomationNew,
    Computer,
    Recap,
    Status,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedCommand {
    Native(Command),
    SetGoal(String),
    AutomationEdit(AutomationId),
}
const GOAL_MAX_BYTES: usize = 4096;
const AUTOMATION_USAGE: &str =
    "Automation usage: /synara/automation [list | new | edit <id>]. Nothing was sent.";
const AUTOMATION_EDIT_USAGE: &str =
    "Automation usage: /synara/automation edit <id>. Nothing was sent.";
const COMMANDS: &[(&str, &str, Command)] = &[
    ("plan", "Select the advertised ACP Plan mode", Command::Plan),
    (
        "debug",
        "Open the evidence-first Debug workflow",
        Command::Debug,
    ),
    (
        "goal",
        "Review this task's goal; pause an active goal or set <objective>",
        Command::Goal,
    ),
    (
        "goal pause",
        "Pause active goal continuation; resume from the goal panel",
        Command::GoalPause,
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
            let Some(objective) = arguments.strip_prefix("set") else {
                return Some(Err(
                    "Goal usage: /synara/goal [pause | set <objective>]. Nothing was sent.",
                ));
            };
            if objective.is_empty() {
                return Some(Err(
                    "Goal usage: /synara/goal set <objective>. Nothing was sent.",
                ));
            }
            if !objective.chars().next().is_some_and(char::is_whitespace) {
                return Some(Err(
                    "Goal usage: /synara/goal [pause | set <objective>]. Nothing was sent.",
                ));
            }
            let objective = objective.trim();
            if objective.is_empty() || objective.len() > GOAL_MAX_BYTES || objective.contains('\0')
            {
                return Some(Err(
                    "Goal text must be non-empty, contain no NUL, and fit within 4 KiB. Nothing was sent.",
                ));
            }
            Ok(ParsedCommand::SetGoal(objective.to_owned()))
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
    } else {
        COMMANDS
            .iter()
            .find(|(candidate, _, _)| *candidate == name)
            .map(|(_, _, command)| ParsedCommand::Native(*command))
            .ok_or("Unknown native command or extra arguments. Use an exact /synara/ command from the menu. Nothing was sent.")
    };
    Some(parsed)
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
            ParsedCommand::AutomationEdit(id) => self.open_automation_for_review(id, cx),
            ParsedCommand::Native(command) => match command {
                Command::Plan => self.native_plan_mode(cx),
                Command::Debug => {
                    self.open_debug(cx);
                    self.debug_workflow.open
                }
                Command::Goal => {
                    self.open_goals(cx);
                    self.goals.open
                }
                Command::GoalPause => self.pause_goal_from_command(cx),
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
                "Synara commands · provider commands remain unchanged. Goal pause/set and automation list/new/edit are native argument forms; goal resume stays in the goal panel.",
            ));
        if prefix.starts_with("goal ") {
            view = view.child(
                div()
                    .text_sm()
                    .text_color(rgb(palette().muted))
                    .child("Usage: /synara/goal pause or /synara/goal set <objective> · resume is available in the goal panel; Send remains explicit."),
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
        view.into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
}
