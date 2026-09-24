//! A setup chat supplies the task/cwd owner required by generic ACP authentication.
//! Preparing it never starts an agent. Connect and advertised sign-in stay explicit.
use super::*;

pub(in crate::shell) enum Reply {
    Prepared {
        revision: u64,
        result: Result<(Task, Catalog), String>,
    },
}

impl Shell {
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
            if details.connection.state == ConnectionState::AuthenticationRequired {
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
                if details.connection.authentication.is_empty() {
                    view = view.child("This agent did not advertise a supported sign-in method. Use its normal external login, then Connect again. No authenticated state is assumed.");
                }
            }
        }
        view.child(self.connection_questions(cx))
            .child(ui::button(("onboarding-agent-terminal", index), "Open task terminal", false)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.selected == Some(task_id) { this.set_panel(Panel::Terminal, cx); }
                })))
            .child("Replay Getting started from Settings to return. A connected session is not proof of subscription, quota or successful provider execution.")
            .into_any_element()
    }
}
