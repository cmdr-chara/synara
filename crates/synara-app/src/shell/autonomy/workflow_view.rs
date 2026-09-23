use super::*;
impl Shell {
    fn workflow_example(&mut self, cx: &mut Context<Self>) {
        if self.autonomy.busy
            || self.autonomy.value.is_some()
            || !self.autonomy.editor.read(cx).text().is_empty()
        {
            return;
        }
        let Some(agent) = self.task().map(|t| t.agent_id.clone()) else {
            return;
        };
        if let Ok(text) = serde_json::to_string_pretty(&WorkflowSpec::example(&agent)) {
            self.autonomy
                .editor
                .update(cx, |e, cx| e.set_text(text, cx));
            self.autonomy.steer = None;
        }
    }
    fn save_workflow_draft(&mut self, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.editor.read(cx).is_composing() {
            return;
        }
        let Some(parent) = self.selected else { return };
        let text = self.autonomy.editor.read(cx).text().to_owned();
        let controller = self.controller.clone();
        if let Some(index) = self.autonomy.steer {
            let Some(value) = self.autonomy.value.clone() else {
                return;
            };
            self.autonomy_job(
                async move {
                    controller
                        .workspace
                        .steer_workflow_step(parent, value.id, value.revision, index, text)
                        .await?;
                    Ok(Outcome::Changed)
                },
                cx,
            );
        } else {
            let spec: WorkflowSpec = match serde_json::from_str(&text) {
                Ok(value) => value,
                Err(error) => {
                    self.autonomy.error = Some(format!("Invalid workflow JSON: {error}"));
                    cx.notify();
                    return;
                }
            };
            if let Err(error) = spec.validate() {
                self.autonomy.error = Some(error.to_string());
                cx.notify();
                return;
            }
            self.autonomy_job(
                async move {
                    controller.workspace.create_workflow(parent, spec).await?;
                    Ok(Outcome::Changed)
                },
                cx,
            );
        }
        // Do not discard the unsaved editor until the exact successful response.
        self.autonomy.submitted = Some(text_snapshot(
            self.autonomy.editor.read(cx).text(),
            self.autonomy.steer,
        ));
    }
    fn workflow_command(&mut self, command: &'static str, index: usize, cx: &mut Context<Self>) {
        let Some(parent) = self.selected else { return };
        if matches!(command, "pause" | "stop")
            && self
                .controller
                .autonomy
                .interrupt(parent, command == "stop")
        {
            self.autonomy.notice = Some("Cancelling active children. Already completed effects are retained. Interrupted steps require explicit retry.".into());
            cx.notify();
            return;
        }
        if self.autonomy.busy {
            return;
        }
        let Some(value) = self.autonomy.value.clone() else {
            return;
        };
        let controller = self.controller.clone();
        self.autonomy_job(
            async move {
                match command {
                    "run" => {
                        controller
                            .run_workflow(parent, value.id, value.revision)
                            .await?;
                    }
                    "recover" => {
                        controller
                            .recover_workflow(parent, value.id, value.revision)
                            .await?;
                    }
                    "retry" => {
                        controller
                            .retry_workflow_step(parent, value.id, value.revision, index)
                            .await?;
                    }
                    "detach" => {
                        controller
                            .workspace
                            .detach_workflow(parent, value.id, value.revision)
                            .await?;
                    }
                    "pause" | "stop" => {
                        controller.pause_workflow(parent, command == "stop").await?;
                    }
                    _ => {
                        return Err(WorkspaceError::Invalid(
                            "Unknown native workflow operation".into(),
                        ));
                    }
                }
                Ok(Outcome::Changed)
            },
            cx,
        );
    }
    pub(in crate::shell) fn autonomy_workflow_settings(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(root) = self.selected else {
            return div()
                .child("Open a chat to create its child-agent workflow.")
                .into_any_element();
        };
        let mut body = self.autonomy_preamble().gap_3()
            .child(div().text_color(rgb(palette().muted)).child("Child agents are ordinary native tasks with independent sessions, usage and permissions. They share this task's working folder. Parallel steps can edit the same files. Review agents and instructions before Run. Nothing runs after restart until explicitly recovered and resumed."))
            .child(ui::action("workflow-refresh", "Refresh workflow", None, false, cx.listener(|this, _, _, cx| this.refresh_autonomy(cx))).relative().child(ui::layout_probe("workflow-refresh")));
        if let Some(parent) = self.autonomy.parent {
            return body
                .child("This is a delegated child. Its parent controls scheduling and retries.")
                .child(ui::action(
                    "workflow-parent",
                    "Open workflow parent",
                    Some(Glyph::Back),
                    false,
                    cx.listener(move |this, _, _, cx| this.open_workflow_child(parent, cx)),
                ))
                .into_any_element();
        }
        if let Some(value) = &self.autonomy.value {
            body = body.child(
                div()
                    .relative()
                    .child(ui::layout_probe("workflow-state"))
                    .child(format!(
                        "{} · {:?} · revision {} · concurrency {}",
                        value.spec.title, value.phase, value.revision, value.spec.concurrency
                    )),
            );
            let mut controls = div().flex().flex_wrap().gap_2();
            for (id, title, command) in [
                ("workflow-run", "Run / resume pending steps", "run"),
                ("workflow-pause", "Pause active children", "pause"),
                ("workflow-stop", "Stop workflow", "stop"),
                ("workflow-recover", "Recover without replay", "recover"),
                (
                    "workflow-detach",
                    "Detach idle children, keep their data",
                    "detach",
                ),
            ] {
                controls = controls.child(
                    ui::action(
                        id,
                        title,
                        None,
                        false,
                        cx.listener(move |this, _, _, cx| this.workflow_command(command, 0, cx)),
                    )
                    .relative()
                    .child(ui::layout_probe(id)),
                );
            }
            body = body.child(controls);
            for (index, step) in value.steps.iter().enumerate() {
                let task = step.task;
                let spec = &value.spec.steps[index];
                let usage = self.autonomy.status["workflow"]["steps"][index]["usage"].clone();
                let instruction = spec.instruction.clone();
                body = body.child(
                    div()
                        .py_2()
                        .border_b_1()
                        .border_color(rgb(palette().border))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(format!(
                            "{}. {} · {} · {:?} · attempts {} · dependencies {:?}",
                            index + 1,
                            spec.title,
                            spec.agent_id,
                            step.state,
                            step.attempts,
                            spec.depends_on
                        ))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(palette().muted))
                                .child(format!(
                                    "Reported usage: {}",
                                    if usage.is_null() {
                                        "Not reported".into()
                                    } else {
                                        usage.to_string()
                                    }
                                )),
                        )
                        .children(
                            step.error
                                .as_ref()
                                .map(|e| div().text_color(rgb(palette().error)).child(e.clone())),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .child(
                                    ui::action(
                                        ("workflow-child", index),
                                        "Open child / permissions / transcript",
                                        None,
                                        false,
                                        cx.listener(move |this, _, _, cx| {
                                            this.open_workflow_child(task, cx)
                                        }),
                                    )
                                    .relative()
                                    .child(ui::layout_probe_slot("workflow-child", index)),
                                )
                                .child(
                                    ui::action(
                                        ("workflow-retry", index),
                                        "Review effects, then prepare retry",
                                        None,
                                        false,
                                        cx.listener(move |this, _, _, cx| {
                                            this.workflow_command("retry", index, cx)
                                        }),
                                    )
                                    .relative()
                                    .child(ui::layout_probe_slot("workflow-retry", index)),
                                )
                                .child(
                                    ui::action(
                                        ("workflow-steer", index),
                                        "Edit pending instruction",
                                        None,
                                        false,
                                        cx.listener(move |this, _, _, cx| {
                                            if !this.autonomy.busy
                                                && this.autonomy.editor.read(cx).text().is_empty()
                                            {
                                                this.autonomy.steer = Some(index);
                                                this.autonomy.editor.update(cx, |e, cx| {
                                                    e.set_text(instruction.clone(), cx)
                                                });
                                            }
                                        }),
                                    )
                                    .relative()
                                    .child(ui::layout_probe_slot("workflow-steer", index)),
                                ),
                        ),
                );
            }
        } else {
            body = body.child(
                ui::action(
                    "workflow-example",
                    "New two-step workflow",
                    Some(Glyph::Blocks),
                    false,
                    cx.listener(|this, _, _, cx| this.workflow_example(cx)),
                )
                .relative()
                .child(ui::layout_probe("workflow-example")),
            );
        }
        if !self.autonomy.editor.read(cx).text().is_empty() || self.autonomy.value.is_none() {
            body = body
                .child(
                    div()
                        .relative()
                        .h(px(240.))
                        .flex_shrink_0()
                        .flex()
                        .flex_col()
                        .child(self.autonomy.editor.clone())
                        .child(ui::layout_probe("workflow-editor")),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            ui::action(
                                "workflow-save",
                                if self.autonomy.steer.is_some() {
                                    "Save reviewed pending instruction"
                                } else {
                                    "Create reviewed unsent children"
                                },
                                None,
                                false,
                                cx.listener(|this, _, _, cx| this.save_workflow_draft(cx)),
                            )
                            .relative()
                            .child(ui::layout_probe("workflow-save")),
                        )
                        .child(
                            ui::action(
                                "workflow-discard",
                                "Discard unsaved draft",
                                None,
                                false,
                                cx.listener(|this, _, _, cx| {
                                    if !this.autonomy.busy {
                                        this.autonomy.editor.update(cx, |e, cx| e.clear(cx));
                                        this.autonomy.steer = None;
                                        this.autonomy.submitted = None;
                                    }
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe("workflow-discard")),
                        ),
                );
        }
        body.child(self.autonomy_gateway_settings(cx))
            .child(div().text_sm().child(format!("Ownership root: {root}")))
            .into_any_element()
    }
}
fn text_snapshot(text: &str, step: Option<usize>) -> (String, Option<usize>) {
    (text.to_owned(), step)
}
