//! Explicit native workflow commands. ACP names cannot contain '/', so the
//! /synara/ namespace never shadows a provider-advertised command.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Plan,
    Debug,
    Goal,
    Fork,
    Subagents,
    Export,
    ExportZip,
    Automation,
    Computer,
    Recap,
    Status,
}
const COMMANDS: &[(&str, &str, Command)] = &[
    ("plan", "Select the advertised ACP Plan mode", Command::Plan),
    (
        "debug",
        "Open the evidence-first Debug workflow",
        Command::Debug,
    ),
    (
        "goal",
        "Review the persistent goal, without arming it",
        Command::Goal,
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
fn parse(text: &str) -> Option<Result<Command, &'static str>> {
    let text = text.trim();
    let name = text.strip_prefix("/synara/")?;
    Some(COMMANDS.iter().find(|(candidate, _, _)| *candidate == name)
        .map(|(_, _, command)| *command)
        .ok_or("Unknown native command or extra arguments. Use an exact /synara/ command from the menu. Nothing was sent."))
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
            Command::Plan => self.native_plan_mode(cx),
            Command::Debug => {
                self.open_debug(cx);
                self.debug_workflow.open
            }
            Command::Goal => {
                self.open_goals(cx);
                self.goals.open
            }
            Command::Recap => {
                self.open_recap(cx);
                self.recap.open
            }
            Command::Fork => {
                let anchor = self
                    .thread
                    .as_ref()
                    .filter(|thread| self.task().is_some_and(|task| task.thread_id == thread.id))
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
            Command::ExportZip => self.export_zip_conversation(cx),
            Command::Export => {
                self.export_conversation(cx);
                true
            }
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
                "Synara commands · provider commands remain unchanged. Bare commands only.",
            ));
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
            assert_eq!(parse(&format!(" /synara/{name}\n")), Some(Ok(*command)));
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
}
