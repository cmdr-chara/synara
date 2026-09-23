use super::*;
impl Shell {
    pub(in crate::shell) fn automations_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let state = &self.automations;
        let mut pane = div().size_full().flex().flex_col().min_h_0().gap_2().text_color(rgb(palette().text))
            .child(div().flex().items_center().gap_2().p_3().border_b_1().border_color(rgb(palette().border))
                .child(div().flex_1().text_lg().child("Automations"))
                .child(ui::button("auto-refresh", if state.loading { "Loading..." } else { "Refresh" }, false).on_click(cx.listener(|this, _, _, cx| this.refresh_automations(cx))))
                .child(ui::button("auto-new", "New automation", false).on_click(cx.listener(|this, _, _, cx| this.edit_automation(None, cx))))
                .child(ui::button("auto-arm", if state.scheduler.armed() { "Stop scheduling" } else { "Start enabled schedules" }, state.scheduler.armed()).on_click(cx.listener(|this, _, _, cx| {
                    if this.automations.scheduler.armed() { this.automations.scheduler.arm(false); }
                    else { this.automations.pending = Some(Pending::Arm); }
                    cx.notify();
                })))
                .child(ui::button("auto-stop", "Stop active / queued run", false).on_click(cx.listener(|this, _, _, cx| { this.automations.scheduler.arm(false); this.automations.scheduler.stop(); cx.notify(); }))))
            .child(div().px_3().text_sm().text_color(rgb(palette().muted)).child(if state.scheduler.armed() {
                "Scheduling is armed for this app session. Runs use the selected profile and existing permission prompts. No automatic retry."
            } else { "Scheduling is stopped. Saved work never launches on startup. Run now is a separate explicit action." }))
            .children(state.error.as_ref().map(|error| div().px_3().text_sm().text_color(rgb(palette().error)).child(error.clone())));
        if let Some(pending) = &state.pending {
            let message = match pending {
                Pending::Arm => "Start currently enabled schedules in this app session? Their saved instructions will be sent at the scheduled time. Overdue slots use each definition's visible missed-run policy. Existing permission prompts still apply.".into(),
                Pending::Run(d) => format!("Run '{}' now using {} in project {}? A new owned conversation will be created. Maximum runtime: {} seconds. Exact instructions:\n{}", d.title, d.agent_id, d.project_id, d.max_runtime_seconds, d.instructions),
                Pending::Enable(d, enabled) => format!("{} '{}'? {}", if *enabled { "Resume" } else { "Pause" }, d.title, if *enabled { "The next run is recalculated from now. It will run only when scheduling is armed." } else { "This stops future scheduled runs, not an active run." }),
                Pending::Delete(d) => format!("Delete '{}'? Run history and generated conversations will be retained. This cannot be undone here.", d.title),
                Pending::Recover(r) => format!("Resolve the previous process's run of '{}'? Confirm only after verifying that the other process has stopped. External effects are unknown. This marks Interrupted, pauses the definition, and does not retry.", r.definition.title),
            };
            pane = pane.child(
                div()
                    .mx_3()
                    .p_3()
                    .border_1()
                    .border_color(rgb(palette().focus))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .id("auto-confirm-message")
                            .max_h(px(200.))
                            .overflow_y_scroll()
                            .text_sm()
                            .child(message),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                ui::button(
                                    "auto-confirm",
                                    if matches!(pending, Pending::Recover(_)) {
                                        "I verified the prior process stopped"
                                    } else {
                                        "Confirm"
                                    },
                                    true,
                                )
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.confirm_automation(cx)),
                                ),
                            )
                            .child(ui::button("auto-dismiss", "Cancel", false).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.automations.pending = None;
                                    cx.notify();
                                }),
                            )),
                    ),
            );
        }
        if let Some(editor) = &state.editor {
            let projects = self
                .catalog
                .projects
                .iter()
                .enumerate()
                .map(|(i, project)| {
                    let id = project.id;
                    ui::button(
                        ("auto-project", i),
                        project.name.clone(),
                        editor.project == Some(id),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(editor) = &mut this.automations.editor {
                            editor.project = Some(id);
                            editor.edit_revision = editor.edit_revision.wrapping_add(1);
                        }
                        cx.notify();
                    }))
                });
            let profiles = self.profiles.iter().enumerate().map(|(i, profile)| {
                let id = profile.id.clone();
                ui::button(
                    ("auto-agent", i),
                    format!("{} ({})", profile.name, profile.id),
                    editor.agent.as_ref() == Some(&id),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(editor) = &mut this.automations.editor {
                        editor.agent = Some(id.clone());
                        editor.edit_revision = editor.edit_revision.wrapping_add(1);
                    }
                    cx.notify();
                }))
            });
            pane = pane.child(div().flex_1().min_h_0().id("auto-editor").overflow_y_scroll().p_3().flex().flex_col().gap_2()
                .child(state.title.clone()).child(state.instructions.clone())
                .child(div().text_sm().child("Agent / provider (required, no fallback)"))
                .child(div().flex().flex_wrap().gap_1().children(profiles))
                .child(div().text_sm().child("Project / workspace"))
                .child(div().flex().flex_wrap().gap_1().children(projects))
                .child(state.schedule.clone()).child(state.timezone.clone())
                .child(div().text_sm().text_color(rgb(palette().muted)).child("Schedules: every 1m through every 10080m, daily HH:MM, weekdays HH:MM, weekly mon HH:MM, or cron followed by five fields: minute hour day-of-month month day-of-week. Cron supports lists, ranges, steps, and sun through sat names, with an eight-year search horizon. If both day-of-month and weekday are constrained, either match runs. Time uses UTC, a fixed offset, or an IANA zone. A spring-forward gap skips that wall-clock slot; a fall-back fold runs at the earlier occurrence once. Saved schedule, timezone, and next run appear in the automation row."))
                .child(state.max_runs.clone()).child(state.failure_limit.clone()).child(state.max_runtime.clone())
                .child(div().text_sm().text_color(rgb(palette().muted)).child("Run and consecutive-failure limits pause the automation automatically. Existing automations keep their saved limits; new automations default to 3 consecutive failures. Runtime is bounded from 1 to 3600 seconds; new definitions default to 900 seconds."))
                .child(ui::button("auto-missed", format!("Missed runs: {:?} (change)", editor.missed), false).on_click(cx.listener(|this, _, _, cx| {
                    if let Some(editor) = &mut this.automations.editor { editor.missed = match editor.missed { MissedRunPolicy::Skip => MissedRunPolicy::CatchUpOnce, MissedRunPolicy::CatchUpOnce => MissedRunPolicy::Skip }; editor.edit_revision = editor.edit_revision.wrapping_add(1); } cx.notify();
                })))
                .child(div().text_sm().text_color(rgb(palette().muted)).child("Skip: skip when over 30 seconds late. CatchUpOnce: one run, never replay every missed interval. Failures have no automatic retry. A runtime timeout cancels the owned task and records a failure."))
                .child(div().flex().gap_2()
                    .child(ui::button("auto-save", if state.changing { "Saving..." } else { "Save paused" }, true).on_click(cx.listener(|this, _, _, cx| this.save_automation_form(cx))))
                    .child(ui::button("auto-discard", "Discard form edits", false).on_click(cx.listener(|this, _, _, cx| { if !this.automations.changing { this.automations.editor = None; } cx.notify(); })))));
            return pane.into_any_element();
        }
        pane.child(div().flex_1().min_h_0().id("auto-content").overflow_y_scroll().p_3().flex().flex_col().gap_2()
            .children((state.loaded && state.ledger.definitions.is_empty()).then(|| div().text_sm().child("No automations. Create one, choose its profile and project, then save it paused.")))
            .children(state.ledger.definitions.iter().enumerate().map(|(i, d)| {
                let edit = d.clone(); let run = d.clone(); let enable = d.clone(); let delete = d.clone();
                let project = self.catalog.projects.iter().find(|p| p.id == d.project_id).map(|p| p.name.as_str()).unwrap_or("Unavailable project");
                div().border_b_1().border_color(rgb(palette().border)).py_3().flex().flex_col().gap_1()
                    .child(div().text_base().child(d.title.clone()))
                    .child(div().text_xs().text_color(rgb(palette().muted)).child(format!("ID: {}", d.id)))
                    .child(div().text_sm().child(format!("{} / {} / {} / {} / {:?}", if d.enabled { "Enabled" } else { "Paused" }, project, d.agent_id, d.schedule.label(), d.missed)))
                    .child(div().text_sm().text_color(rgb(palette().muted)).child(format!("Total run limit: {} / Consecutive failures: {} / Failure limit: {}", d.max_runs.map(|n| n.to_string()).unwrap_or_else(|| "none".into()), d.failure_streak, d.stop_after_consecutive_failures.map(|n| n.to_string()).unwrap_or_else(|| "none".into()))))
                    .child(div().text_sm().text_color(rgb(palette().muted)).child(format!("Maximum runtime: {} seconds", d.max_runtime_seconds)))
                    .child(div().text_sm().text_color(rgb(palette().muted)).child(format!("Timezone: {} / Next: {}{}", d.timezone, time_label(d.next_run_ms), if !d.enabled { " (paused)" } else { "" })))
                    .child(div().text_sm().child(d.instructions.chars().take(280).collect::<String>()))
                    .child(div().flex().flex_wrap().gap_1()
                        .child(ui::button(("auto-run", i), "Run now...", false).on_click(cx.listener(move |this, _, _, cx| { this.automations.pending = Some(Pending::Run(run.clone())); cx.notify(); })))
                        .child(ui::button(("auto-enable", i), if d.enabled { "Pause..." } else { "Resume..." }, false).on_click(cx.listener(move |this, _, _, cx| { this.automations.pending = Some(Pending::Enable(enable.clone(), !enable.enabled)); cx.notify(); })))
                        .child(ui::button(("auto-edit", i), "Edit", false).on_click(cx.listener(move |this, _, _, cx| this.edit_automation(Some(edit.clone()), cx))))
                        .child(ui::button(("auto-delete", i), "Delete...", false).on_click(cx.listener(move |this, _, _, cx| { this.automations.pending = Some(Pending::Delete(delete.clone())); cx.notify(); }))))
            }))
            .child(div().pt_3().text_lg().child(format!("Run history ({}/256)", state.ledger.runs.len())))
            .child(div().text_sm().text_color(rgb(palette().muted)).child("History is retained on deletion. At capacity, new runs stop rather than silently removing evidence. Open a row for its exact instructions and output."))
            .children(state.ledger.runs.iter().rev().enumerate().map(|(i, run)| {
                let id = run.id; let recover = run.clone();
                let expanded = state.selected_run == Some(id);
                let mut row = div().py_2().border_b_1().border_color(rgb(palette().border)).flex().flex_col().gap_1()
                    .child(ui::button(("auto-history", i), format!("{} / {:?} / {}", run.definition.title, run.status, time_label(run.started_ms)), expanded).on_click(cx.listener(move |this, _, _, cx| { this.automations.selected_run = if this.automations.selected_run == Some(id) { None } else { Some(id) }; cx.notify(); })));
                if expanded {
                    row = row.child(div().text_sm().child(format!("Agent: {} / Project: {} / Scheduled: {} / Maximum runtime: {} seconds\nInstructions:\n{}\nOutput / error:\n{}", run.definition.agent_id, run.definition.project_id, run.scheduled_ms.map(time_label).unwrap_or_else(|| "Manual run".into()), run.definition.max_runtime_seconds, run.definition.instructions, run.output)))
                        .children(run.task_id.map(|task| ui::button(("auto-open-task", i), "Open owned conversation", false).on_click(cx.listener(move |this, _, _, cx| this.open_automation_task(task, cx)))));
                    if run.owner != state.scheduler.owner() && run.status == AutomationRunStatus::Running {
                        row = row.child(div().text_sm().child("Previous process: outcome unknown. No automatic restart."))
                            .child(ui::button(("auto-recover", i), "Resolve after verifying previous process stopped...", false).on_click(cx.listener(move |this, _, _, cx| { this.automations.pending = Some(Pending::Recover(recover.clone())); cx.notify(); })));
                    }
                }
                row
            }))).into_any_element()
    }
}
