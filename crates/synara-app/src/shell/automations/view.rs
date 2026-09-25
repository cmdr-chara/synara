use super::*;
impl Shell {
    pub(in crate::shell) fn automations_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let state = &self.automations;
        let prunable_history = state
            .ledger
            .runs
            .iter()
            .filter(|run| {
                run.status != AutomationRunStatus::Running
                    && !state
                        .ledger
                        .definitions
                        .iter()
                        .any(|definition| definition.id == run.definition.id)
            })
            .count();
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
                Pending::Run(d) => format!("Run '{}' now using {} in project {}? Execution mode: {}. {} Hub context, when selected, is snapshotted from the current saved Hub at claim time. Maximum runtime: {} seconds. Exact automation instructions:\n{}", d.title, d.agent_id, d.project_id, d.mode.label(), match d.mode { AutomationMode::Standalone => "A fresh owned conversation will be created.", AutomationMode::Heartbeat => "The reviewed target conversation will be continued.", AutomationMode::Dedicated => if d.target_task_id.is_some() { "The automation-owned conversation will be continued." } else { "The first run will create an automation-owned conversation." } }, d.context.label(), d.max_runtime_seconds, d.instructions),
                Pending::Enable(d, enabled) => format!("{} '{}'? {}", if *enabled { "Resume" } else { "Pause" }, d.title, if *enabled { "The next run is recalculated from now. It will run only when scheduling is armed." } else { "This stops future scheduled runs, not an active run." }),
                Pending::Delete(d) => format!("Delete '{}'? Run history and generated conversations will be retained. This cannot be undone here.", d.title),
                Pending::Recover(r) => format!("Resolve the previous process's run of '{}'? Confirm only after verifying that the other process has stopped. External effects are unknown. This marks Interrupted, pauses the definition, and does not retry.", r.definition.title),
                Pending::PruneHistory(count) => format!("Permanently remove {count} retained run-history record{} whose automation definitions were already deleted? Generated conversations remain intact. Active runs and history belonging to current definitions are not eligible.", if *count == 1 { "" } else { "s" }),
                Pending::PruneDefinitionHistory(d, count) => format!("Permanently remove {count} retained terminal run-history record{} for '{}'? Generated conversations remain intact. Its cumulative run count stays at {}, so pruning cannot reset the maximum-run limit. Failure streak and schedule state are unchanged.", if *count == 1 { "" } else { "s" }, d.title, d.run_count.max(state.ledger.runs.iter().filter(|run| run.definition.id == d.id && run.task_id.is_some()).count() as u32)),
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
                            if editor.project != Some(id)
                                && editor.mode == AutomationMode::Heartbeat
                            {
                                editor.target_task = None;
                            }
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
                        if editor.agent.as_ref() != Some(&id)
                            && editor.mode == AutomationMode::Heartbeat
                        {
                            editor.target_task = None;
                        }
                        editor.agent = Some(id.clone());
                        editor.edit_revision = editor.edit_revision.wrapping_add(1);
                    }
                    cx.notify();
                }))
            });
            let completion_models = state
                .direct_models
                .providers
                .iter()
                .flat_map(|provider| {
                    provider.models.iter().filter_map(move |model| {
                        if model.capabilities.structured_output != synara_model::Support::Supported
                        {
                            return None;
                        }
                        let selection = ModelSelection {
                            history_turns: Some(0),
                            provider_id: provider.id.clone(),
                            model_id: model.id.clone(),
                            max_output_tokens: model
                                .capabilities
                                .max_output_tokens
                                .unwrap_or(512)
                                .min(512)
                                .max(1) as u32,
                            reasoning_effort: None,
                            output: synara_model::OutputFormat::Text,
                        };
                        Some((format!("{} / {}", provider.name, model.name), selection))
                    })
                })
                .enumerate()
                .map(|(i, (label, selection))| {
                    let selected = editor.completion_selection.as_ref() == Some(&selection);
                    ui::button(("auto-completion-model", i), label, selected).on_click(cx.listener(
                        move |this, _, _, cx| {
                            if let Some(editor) = &mut this.automations.editor {
                                editor.completion_selection = Some(selection.clone());
                                editor.edit_revision = editor.edit_revision.wrapping_add(1);
                            }
                            cx.notify();
                        },
                    ))
                });
            let heartbeat_targets = self
                .catalog
                .tasks
                .iter()
                .enumerate()
                .filter(|(_, task)| {
                    Some(task.project_id) == editor.project && task.state != TaskState::Archived
                })
                .map(|(i, task)| {
                    let id = task.id;
                    ui::button(
                        ("auto-heartbeat-target", i),
                        format!("{} · {}", task.title, task.id),
                        editor.target_task == Some(id),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(editor) = &mut this.automations.editor {
                            editor.target_task = Some(id);
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
                .child(div().text_sm().child("Execution mode"))
                .child(div().flex().flex_wrap().gap_1()
                    .child(ui::button("auto-mode-standalone", AutomationMode::Standalone.label(), editor.mode == AutomationMode::Standalone)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(editor)=&mut this.automations.editor {
                                editor.mode=AutomationMode::Standalone;
                                editor.target_task=None;
                                editor.edit_revision=editor.edit_revision.wrapping_add(1);
                            }
                            cx.notify();
                        })))
                    .child(ui::button("auto-mode-heartbeat", AutomationMode::Heartbeat.label(), editor.mode == AutomationMode::Heartbeat)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(editor)=&mut this.automations.editor {
                                if editor.mode != AutomationMode::Heartbeat {
                                    editor.target_task=None;
                                }
                                editor.mode=AutomationMode::Heartbeat;
                                editor.edit_revision=editor.edit_revision.wrapping_add(1);
                            }
                            cx.notify();
                        })))
                    .child(ui::button("auto-mode-dedicated", AutomationMode::Dedicated.label(), editor.mode == AutomationMode::Dedicated)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(editor)=&mut this.automations.editor {
                                if editor.mode != AutomationMode::Dedicated {
                                    editor.target_task=None;
                                }
                                editor.mode=AutomationMode::Dedicated;
                                editor.edit_revision=editor.edit_revision.wrapping_add(1);
                            }
                            cx.notify();
                        }))))
                .children((editor.mode == AutomationMode::Heartbeat).then(|| div().flex().flex_col().gap_1()
                    .child(div().text_sm().child("Heartbeat target conversation"))
                    .child(div().flex().flex_wrap().gap_1().children(heartbeat_targets))
                    .child(div().text_sm().text_color(rgb(palette().muted)).child("The target must stay idle, unarchived, in this project, use the selected ACP agent, have no direct-model route, no unsent draft or pending attachments, and not already be owned by another automation run."))))
                .child(div().text_sm().text_color(rgb(palette().muted)).child("Standalone creates a fresh owned conversation for every run. Heartbeat continues the selected existing conversation. Dedicated creates one automation-owned conversation on its first run and reuses it thereafter; its target cannot be imported from another task."))
                .child(state.heartbeat_cooldown.clone())
                .child(div().text_sm().text_color(rgb(palette().muted)).child("Continuation cooldown applies to Heartbeat and Dedicated targets after recent external activity. The automation's own previous completed run does not throttle its next scheduled wake. Use 0 to disable the cooldown."))
                .child(div().text_sm().child("Run context"))
                .child(div().flex().flex_wrap().gap_1()
                    .child(ui::button("auto-context-project", AutomationContextPolicy::Project.label(), editor.context == AutomationContextPolicy::Project)
                        .on_click(cx.listener(|this, _, _, cx| { if let Some(editor)=&mut this.automations.editor { editor.context=AutomationContextPolicy::Project; editor.edit_revision=editor.edit_revision.wrapping_add(1); } cx.notify(); })))
                    .child(ui::button("auto-context-hub", AutomationContextPolicy::Hub.label(), editor.context == AutomationContextPolicy::Hub)
                        .on_click(cx.listener(|this, _, _, cx| { if let Some(editor)=&mut this.automations.editor { editor.context=AutomationContextPolicy::Hub; editor.edit_revision=editor.edit_revision.wrapping_add(1); } cx.notify(); }))))
                .child(div().text_sm().text_color(rgb(palette().muted)).child("Project context submits only the saved automation instructions. Hub context requires the selected project to have an active Hub and snapshots its user-maintained shared instructions/knowledge into the visible owned conversation before each run. Transcripts and files are never harvested automatically."))
                .child(div().text_sm().child("Completion policy"))
                .child(div().flex().flex_wrap().gap_1()
                    .child(ui::button("auto-completion-none", "No AI stop check", editor.completion_selection.is_none())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(editor)=&mut this.automations.editor {
                                editor.completion_selection=None;
                                editor.edit_revision=editor.edit_revision.wrapping_add(1);
                            }
                            cx.notify();
                        })))
                    .children(completion_models))
                .children(editor.completion_selection.is_some().then(|| div().flex().flex_col().gap_1()
                    .child(state.completion_stop_when.clone())
                    .child(state.completion_threshold.clone())
                    .child(div().text_sm().text_color(rgb(palette().muted)).child("The stop evaluator is a separately reviewed direct-model request. It receives only the saved stop condition, automation instructions, exact run prompt and this run's assistant output. No tools, ACP session state, approvals, files or hidden reasoning are provided. A failed or timed-out check never disables the automation; a matching result disables only if this exact policy revision is still current."))))
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
                let edit = d.clone(); let run = d.clone(); let enable = d.clone(); let delete = d.clone(); let prune = d.clone();
                let retained_claimed = state.ledger.runs.iter().filter(|history| history.definition.id == d.id && history.task_id.is_some()).count();
                let run_count = d.run_count.max(u32::try_from(retained_claimed).unwrap_or(u32::MAX));
                let prunable = state.ledger.runs.iter().filter(|history| history.definition.id == d.id && history.status != AutomationRunStatus::Running).count();
                let project = self.catalog.projects.iter().find(|p| p.id == d.project_id).map(|p| p.name.as_str()).unwrap_or("Unavailable project");
                div().border_b_1().border_color(rgb(palette().border)).py_3().flex().flex_col().gap_1()
                    .child(div().text_base().child(d.title.clone()))
                    .child(div().text_xs().text_color(rgb(palette().muted)).child(format!("ID: {}", d.id)))
                    .child(div().text_sm().child(format!("{} / {} / {} / {} / {:?} / {} / {} / {}", if d.enabled { "Enabled" } else { "Paused" }, project, d.agent_id, d.schedule.label(), d.missed, d.mode.label(), d.context.label(), match &d.completion_policy { AutomationCompletionPolicy::None => "No stop check", AutomationCompletionPolicy::AiEvaluated { .. } => "AI stop check" })))
                    .child(div().text_sm().text_color(rgb(palette().muted)).child(format!("Runs claimed: {run_count} / Total run limit: {} / Consecutive failures: {} / Failure limit: {}", d.max_runs.map(|n| n.to_string()).unwrap_or_else(|| "none".into()), d.failure_streak, d.stop_after_consecutive_failures.map(|n| n.to_string()).unwrap_or_else(|| "none".into()))))
                    .child(div().text_sm().text_color(rgb(palette().muted)).child(format!("Maximum runtime: {} seconds / Continuation cooldown: {} seconds", d.max_runtime_seconds, d.heartbeat_cooldown_seconds)))
                    .child(div().text_sm().text_color(rgb(palette().muted)).child(format!("Timezone: {} / Next: {}{}", d.timezone, time_label(d.next_run_ms), if !d.enabled { " (paused)" } else { "" })))
                    .child(div().text_sm().child(d.instructions.chars().take(280).collect::<String>()))
                    .child(div().flex().flex_wrap().gap_1()
                        .child(ui::button(("auto-run", i), "Run now...", false).on_click(cx.listener(move |this, _, _, cx| { this.automations.pending = Some(Pending::Run(run.clone())); cx.notify(); })))
                        .child(ui::button(("auto-enable", i), if d.enabled { "Pause..." } else { "Resume..." }, false).on_click(cx.listener(move |this, _, _, cx| { this.automations.pending = Some(Pending::Enable(enable.clone(), !enable.enabled)); cx.notify(); })))
                        .child(ui::button(("auto-edit", i), "Edit", false).on_click(cx.listener(move |this, _, _, cx| this.edit_automation(Some(edit.clone()), cx))))
                        .children((prunable > 0).then(|| ui::button(("auto-prune-definition-history", i), format!("Prune {prunable} history..."), false).on_click(cx.listener(move |this, _, _, cx| { this.automations.pending = Some(Pending::PruneDefinitionHistory(prune.clone(), prunable)); cx.notify(); }))))
                        .child(ui::button(("auto-delete", i), "Delete...", false).on_click(cx.listener(move |this, _, _, cx| { this.automations.pending = Some(Pending::Delete(delete.clone())); cx.notify(); }))))
            }))
            .child(div().pt_3().flex().items_center().gap_2()
                .child(div().flex_1().text_lg().child(format!("Run history ({}/256)", state.ledger.runs.len())))
                .child(ui::button(
                    "auto-export-history",
                    if state.exporting_history { "Exporting..." } else { "Export history..." },
                    false,
                ).on_click(cx.listener(|this, _, _, cx| this.export_automation_history(cx)))))
            .child(div().text_sm().text_color(rgb(palette().muted)).child("History is retained by default. Export writes a versioned JSON snapshot of retained run records and may contain private instructions or output. Terminal history can be explicitly pruned for a live or deleted definition; generated conversations remain. Live-definition pruning preserves its cumulative run count, failure streak, schedule and maximum-run enforcement."))
            .children((prunable_history > 0).then(|| {
                ui::button(
                    "auto-prune-history",
                    format!("Prune {prunable_history} deleted-definition history record{}...", if prunable_history == 1 { "" } else { "s" }),
                    false,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.automations.pending = Some(Pending::PruneHistory(prunable_history));
                    cx.notify();
                }))
            }))
            .children(state.ledger.runs.iter().rev().enumerate().map(|(i, run)| {
                let id = run.id; let recover = run.clone();
                let expanded = state.selected_run == Some(id);
                let mut row = div().py_2().border_b_1().border_color(rgb(palette().border)).flex().flex_col().gap_1()
                    .child(ui::button(("auto-history", i), format!("{} / {:?} / {}", run.definition.title, run.status, time_label(run.started_ms)), expanded).on_click(cx.listener(move |this, _, _, cx| { this.automations.selected_run = if this.automations.selected_run == Some(id) { None } else { Some(id) }; cx.notify(); })));
                if expanded {
                    row = row.child(div().text_sm().child(format!("Agent: {} / Project: {} / Mode: {}{} / Context: {}{} / Scheduled: {} / Maximum runtime: {} seconds / Continuation cooldown: {} seconds\nAutomation instructions:\n{}\nExact submitted prompt:\n{}\nOutput / error:\n{}", run.definition.agent_id, run.definition.project_id, run.definition.mode.label(), run.task_id.map(|id| format!(" · task {id}")).unwrap_or_default(), run.definition.context.label(), run.hub_revision.map(|revision| format!(" (Hub revision {revision})")).unwrap_or_default(), run.scheduled_ms.map(time_label).unwrap_or_else(|| "Manual run".into()), run.definition.max_runtime_seconds, run.definition.heartbeat_cooldown_seconds, run.definition.instructions, if run.prompt.is_empty() { "(not retained for this legacy/skipped run)" } else { run.prompt.as_str() }, run.output)))
                        .children(run.completion_evaluation.as_ref().map(|evaluation| {
                            div().text_sm().text_color(rgb(if evaluation.failed { palette().error } else { palette().muted })).child(format!(
                                "Stop check: {} · confidence {:.2} · {}{}",
                                if evaluation.stop_matched { "matched" } else { "not matched" },
                                evaluation.confidence,
                                evaluation.reason,
                                if evaluation.policy_applied { " · automation paused" } else { "" }
                            ))
                        }))
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
