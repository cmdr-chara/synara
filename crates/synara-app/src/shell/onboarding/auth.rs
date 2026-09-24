//! A setup chat supplies the task/cwd owner required by generic ACP authentication.
//! Preparing it never starts an agent. Connect and advertised sign-in stay explicit.
use super::*;

#[derive(Clone, Copy)]
struct ProviderLoginGuide {
    provider: &'static str,
    description: &'static str,
    command_args: &'static str,
    terminal_label: &'static str,
    docs_url: &'static str,
    docs_label: &'static str,
}

fn provider_login_guide(profile: &AgentProfile) -> Option<ProviderLoginGuide> {
    let command = profile
        .command
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let identity = format!("{} {} {command}", profile.id, profile.name).to_ascii_lowercase();
    if identity.contains("opencode") {
        Some(ProviderLoginGuide {
            provider: "OpenCode",
            description: "OpenCode manages provider sign-in in its own CLI. Choose the provider you plan to use, follow its prompt, then reconnect here.",
            command_args: "auth login",
            terminal_label: "Open terminal · run OpenCode sign-in",
            docs_url: "https://opencode.ai/docs/cli/#auth",
            docs_label: "OpenCode sign-in guide",
        })
    } else if identity.contains("gemini") {
        Some(ProviderLoginGuide {
            provider: "Gemini CLI",
            description: "Gemini CLI can sign in with Google from its interactive terminal. Launch it, choose Sign in with Google, and follow the browser prompt. Most personal accounts do not need a Google Cloud project; organization and some Gemini Code Assist accounts may require one.",
            command_args: "",
            terminal_label: "Open terminal · start Gemini CLI",
            docs_url: "https://geminicli.com/docs/get-started/authentication/",
            docs_label: "Gemini CLI sign-in guide",
        })
    } else {
        None
    }
}

fn provider_login_command(profile: &AgentProfile, guide: ProviderLoginGuide) -> Option<String> {
    let executable = profile
        .command
        .file_name()?
        .to_string_lossy()
        .to_ascii_lowercase();
    let known_executable = match guide.provider {
        "OpenCode" => executable == "opencode" || executable == "opencode.exe",
        "Gemini CLI" => matches!(
            executable.as_str(),
            "gemini" | "gemini.exe" | "gemini-cli" | "gemini-cli.exe" | "gemini.js"
        ),
        _ => false,
    };
    if !known_executable
        || (!profile.command.is_absolute() && profile.command.components().count() > 1)
    {
        return None;
    }

    let path = profile.command.to_str()?;
    #[cfg(windows)]
    let quoted = {
        // cmd.exe expands these even inside quotes, so do not put them in a
        // command copied for the user to run.
        if path
            .chars()
            .any(|character| matches!(character, '%' | '!' | '"'))
        {
            return None;
        }
        format!("\"{path}\"")
    };
    #[cfg(not(windows))]
    let quoted = format!("'{}'", path.replace('\'', "'\\''"));

    Some(if guide.command_args.is_empty() {
        quoted
    } else {
        format!("{quoted} {}", guide.command_args)
    })
}

pub(in crate::shell) enum Reply {
    Prepared {
        revision: u64,
        result: Result<(Task, Catalog), String>,
    },
}

impl Shell {
    pub(super) fn onboarding_provider_guide(
        &self,
        index: usize,
        profile: &AgentProfile,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(guide) = provider_login_guide(profile) else {
            return div().into_any_element();
        };
        let command_available = command_found(&profile.command);
        let login_command = command_available
            .then(|| provider_login_command(profile, guide))
            .flatten();
        let docs_url = guide.docs_url.to_owned();
        let mut view = div()
            .mt_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(crate::ui::palette().border))
            .flex()
            .flex_col()
            .gap_2()
            .child(format!("{} sign-in", guide.provider))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(crate::ui::palette().muted))
                    .child(guide.description),
            )
            .children(login_command.as_ref().map(|command| {
                div()
                    .font_family(crate::ui::code_font())
                    .text_size(px(12.))
                    .child(command.clone())
            }));
        if let Some(command) = login_command {
            view = view.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        ui::button(
                            ("onboarding-provider-copy-login", index),
                            "Copy CLI command",
                            false,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                command.clone(),
                            ));
                            this.notice = Some("CLI sign-in command copied. Enter provider credentials only in the provider's own flow.".into());
                            cx.notify();
                        })),
                    ),
            );
        } else {
            view = view.child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(crate::ui::palette().muted))
                    .child(if command_available {
                        "This configured command is a wrapper or custom path. Use an advertised ACP sign-in method here, or follow the provider guide for that executable."
                    } else {
                        "The configured executable was not found. Install or configure this agent before signing in."
                    }),
            );
        }
        view.child(
            ui::button(("onboarding-provider-docs", index), guide.docs_label, false)
                .on_click(move |_, _, cx| cx.open_url(&docs_url)),
        )
        .into_any_element()
    }

    pub(in crate::shell) fn onboarding_auth_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        let Reply::Prepared { revision, result } = reply;
        self.creating_task = false;
        match result {
            Err(error) => self.error = Some(format!("Setup chat could not be prepared: {error}")),
            Ok((task, catalog)) => {
                self.settings
                    .onboarding_tasks
                    .insert(task.agent_id.clone(), task.id);
                self.catalog = catalog;
                if self.selection_revision == revision && self.select_task(task.id, cx) {
                    self.set_panel(Panel::Settings, cx);
                    self.open_settings_section(settings::Section::Onboarding, cx);
                    self.settings.onboarding_step = 2;
                    self.focus_composer = false;
                    self.notice = Some("Setup chat saved. Use Connect to start the agent, then choose its advertised sign-in method. No prompt has been sent.".into());
                } else {
                    self.notice = Some("Setup chat saved. Reopen it from Getting started or thread search when ready. No agent was started.".into());
                }
            }
        }
        cx.notify();
    }

    fn prepare_onboarding_agent(&mut self, agent: String, cx: &mut Context<Self>) {
        if self.creating_task
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.dirty(cx)
            || self.saving
            || self.hubs.pending(cx)
            || self.followups.pending(cx)
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing()
        {
            self.error = Some(
                "Finish the current load or editor changes before preparing a setup chat.".into(),
            );
            cx.notify();
            return;
        }
        let Some(profile) = self.profiles.iter().find(|p| p.id == agent) else {
            return;
        };
        if !command_found(&profile.command) {
            self.error = Some(
                "The configured command was not found. Install or configure the agent first."
                    .into(),
            );
            cx.notify();
            return;
        }
        if let Some(task) = self.settings.onboarding_tasks.get(&agent).copied()
            && self
                .catalog
                .tasks
                .iter()
                .any(|t| t.id == task && t.agent_id == agent && t.state != TaskState::Archived)
        {
            if self.select_task(task, cx) {
                self.set_panel(Panel::Settings, cx);
                self.open_settings_section(settings::Section::Onboarding, cx);
                self.settings.onboarding_step = 2;
                self.focus_composer = false;
            }
            return;
        }
        let title = format!(
            "Setup: {}",
            profile.name.chars().take(80).collect::<String>()
        );
        let workspace = self.controller.workspace.clone();
        let directory = self.scratch_directory.join(ThreadId::new().to_string());
        let revision = self.selection_revision;
        self.snapshot_draft(cx);
        self.creating_task = true;
        self.error = None;
        self.job(async move {
            let result = async {
                std::fs::create_dir_all(&directory).map_err(synara_runtime::RuntimeError::Io)?;
                let project = workspace.add_local_workspace(directory).await?;
                // Read before the atomic task/draft insert, avoiding retryable duplicates
                // after successful creation if a later catalog read were to fail.
                let mut catalog = workspace.catalog().await?;
                let task = workspace
                    .create_scoped_task(project.id, title, agent, TaskScope::Chat)
                    .await?;
                catalog.tasks.insert(0, task.clone());
                Ok::<_, WorkspaceError>((task, catalog))
            }
            .await
            .map_err(|e| e.to_string());
            Ok(Update::Onboarding(Box::new(Reply::Prepared {
                revision,
                result,
            })))
        });
        cx.notify();
    }

    pub(super) fn onboarding_agent_access(
        &self,
        index: usize,
        agent: &str,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let current = self.task().filter(|task| {
            task.agent_id == agent && task.state != TaskState::Archived && !self.uses_direct_model()
        });
        let Some(task) = current else {
            let agent = agent.to_owned();
            return ui::button(
                ("onboarding-agent-prepare", index),
                if self.creating_task {
                    "Preparing..."
                } else {
                    "Prepare sign-in chat"
                },
                false,
            )
            .relative()
            .child(ui::layout_probe_slot("onboarding-agent-prepare", index))
            .on_click(
                cx.listener(move |this, _, _, cx| this.prepare_onboarding_agent(agent.clone(), cx)),
            )
            .into_any_element();
        };
        let task_id = task.id;
        let terminal_label = self
            .profiles
            .iter()
            .find(|profile| profile.id == agent)
            .and_then(provider_login_guide)
            .map_or("Open task terminal", |guide| guide.terminal_label);
        let blocked = self.controls_blocked() || self.loading_task.is_some() || self.creating_task;
        let mut view = div().flex().flex_col().gap_2()
            .child(format!("Connection owner: {}", task.title))
            .child("Preparation is not sign-in. Connect starts the configured ACP command. Authentication and questions stay with that agent; no prompt is submitted.")
            .child(ui::button(("onboarding-agent-connect", index), if blocked { "Connection busy..." } else { "Connect / check sign-in" }, false)
                .relative().child(ui::layout_probe_slot("onboarding-agent-connect", index))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.selected == Some(task_id) && !this.controls_blocked() && this.loading_task.is_none()
                        && this.close == CloseState::Open {
                        this.connect("connect", cx);
                    }
                })));
        if let Some(details) = &self.details {
            view = view.child(format!(
                "Agent-reported connection: {:?}",
                details.connection.state
            ));
            let state_guidance = match details.connection.state {
                ConnectionState::AuthenticationRequired => "Authentication is required.",
                ConnectionState::Authenticating => {
                    "Sign-in is in progress. Finish the provider prompt and wait for the connection result."
                }
                ConnectionState::Connected => {
                    "Connected. You can set this agent as default and start a task. This does not verify subscription, quota, or model access."
                }
                ConnectionState::Failed | ConnectionState::Exited => {
                    "The agent stopped or failed to connect. Review its terminal or provider output, then try Connect again."
                }
                ConnectionState::Disconnected => {
                    "No active session. Use Connect after the provider is installed and signed in."
                }
                ConnectionState::Starting
                | ConnectionState::Initializing
                | ConnectionState::Restarting => {
                    "The agent is starting. Wait for its reported connection state."
                }
            };
            view = view.child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(crate::ui::palette().muted))
                    .child(state_guidance),
            );
            if details.connection.state == ConnectionState::AuthenticationRequired {
                if details.connection.authentication.is_empty() {
                    view = view.child("No ACP sign-in method was advertised. Use the provider CLI guide and reconnect after sign-in.");
                } else {
                    view =
                        view.child("Choose one of the sign-in methods advertised by this agent:");
                }
                for (method_index, method) in details.connection.authentication.iter().enumerate() {
                    let id = method.id.clone();
                    view = view.child(
                        ui::button(
                            ("onboarding-auth-method", method_index),
                            method.name.clone(),
                            false,
                        )
                        .relative()
                        .child(ui::layout_probe_slot(
                            "onboarding-auth-method",
                            method_index,
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if this.selected == Some(task_id)
                                && !this.controls_blocked()
                                && this.loading_task.is_none()
                                && this.close == CloseState::Open
                            {
                                this.authenticate(id.clone(), cx);
                            }
                        })),
                    );
                }
            }
        }
        view.child(self.connection_questions(cx))
            .child(ui::button(("onboarding-agent-terminal", index), terminal_label, false)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.selected == Some(task_id) { this.set_panel(Panel::Terminal, cx); }
                })))
            .child("Replay Getting started from Settings to return. A connected session is not proof of subscription, quota or successful provider execution.")
            .into_any_element()
    }
}
