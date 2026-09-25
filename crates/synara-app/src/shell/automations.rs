//! Native automation controls. Loading never arms the process-session scheduler.
mod view;
use super::*;
use crate::ui::{self, palette};
use std::time::{Duration, Instant};

pub(super) struct AutomationsView {
    ledger: AutomationLedger,
    scheduler: Arc<AutomationScheduler>,
    loaded: bool,
    loading: bool,
    changing: bool,
    save_snapshot: Option<(AutomationId, u64)>,
    executing: bool,
    exporting_history: bool,
    generation: u64,
    refreshed: Instant,
    polled: Instant,
    editor: Option<Editor>,
    pending: Option<Pending>,
    title: Entity<TextEntry>,
    instructions: Entity<TextEntry>,
    schedule: Entity<TextEntry>,
    timezone: Entity<TextEntry>,
    max_runs: Entity<TextEntry>,
    failure_limit: Entity<TextEntry>,
    max_runtime: Entity<TextEntry>,
    selected_run: Option<AutomationId>,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}
#[derive(Clone)]
struct Editor {
    id: AutomationId,
    revision: Option<u64>,
    edit_revision: u64,
    agent: Option<String>,
    project: Option<ProjectId>,
    missed: MissedRunPolicy,
    context: AutomationContextPolicy,
}
#[derive(Clone)]
enum Pending {
    Arm,
    Run(AutomationDefinition),
    Enable(AutomationDefinition, bool),
    Delete(AutomationDefinition),
    Recover(AutomationRun),
    PruneHistory(usize),
    PruneDefinitionHistory(AutomationDefinition, usize),
}
impl AutomationsView {
    pub fn new(controller: Arc<Controller>, cx: &mut Context<Shell>) -> Self {
        let mut make = |label, mode, height| cx.new(|cx| TextEntry::new(label, mode, height, cx));
        let title = make("Automation title", EntryMode::SingleLine, 34.);
        let instructions = make(
            "Exact instructions to send to the selected agent",
            EntryMode::Editor,
            150.,
        );
        let schedule = make(
            "every 60m, daily 09:00, weekly mon 09:00, cron 0 9 * * *",
            EntryMode::SingleLine,
            34.,
        );
        let timezone = make(
            "UTC, fixed offset, or IANA zone, e.g. Europe/Rome",
            EntryMode::SingleLine,
            34.,
        );
        let max_runs = make(
            "Maximum runs (empty = unlimited)",
            EntryMode::SingleLine,
            34.,
        );
        let failure_limit = make(
            "Stop after consecutive failures (empty = unlimited)",
            EntryMode::SingleLine,
            34.,
        );
        let max_runtime = make(
            "Maximum runtime in seconds (1–3600; default 900)",
            EntryMode::SingleLine,
            34.,
        );
        let subscriptions = [
            &title,
            &instructions,
            &schedule,
            &timezone,
            &max_runs,
            &failure_limit,
            &max_runtime,
        ]
        .into_iter()
        .map(|input| {
            cx.subscribe(input, |this, _, event, cx| {
                if matches!(event, EntryEvent::Changed)
                    && let Some(editor) = &mut this.automations.editor
                {
                    editor.edit_revision = editor.edit_revision.wrapping_add(1);
                }
                cx.notify();
            })
        })
        .collect();
        Self {
            ledger: AutomationLedger::default(),
            scheduler: Arc::new(AutomationScheduler::new(controller)),
            loaded: false,
            loading: false,
            changing: false,
            save_snapshot: None,
            executing: false,
            exporting_history: false,
            generation: 0,
            refreshed: Instant::now(),
            polled: Instant::now(),
            editor: None,
            pending: None,
            title,
            instructions,
            schedule,
            timezone,
            max_runs,
            failure_limit,
            max_runtime,
            selected_run: None,
            error: None,
            _subscriptions: subscriptions,
        }
    }
    pub(super) fn retire(&self) {
        self.scheduler.arm(false);
        self.scheduler.stop();
    }
}
impl Drop for AutomationsView {
    fn drop(&mut self) {
        self.retire();
    }
}
pub(super) enum Reply {
    Loaded {
        generation: u64,
        result: Result<(AutomationLedger, Catalog, Vec<AgentProfile>), String>,
    },
    Changed(Result<(), String>),
    Finished(Result<(), String>),
    HistoryExported(Result<usize, String>),
}
impl Shell {
    pub(super) fn automation_before_quit(&mut self, cx: &mut Context<Self>) -> bool {
        if self.automations.exporting_history {
            self.panel = Panel::Automations;
            self.automations.error =
                Some("Finish or cancel the automation history export before quitting.".into());
            cx.notify();
            return true;
        }
        if self.automations.editor.is_some() || self.automations.changing {
            self.panel = Panel::Automations;
            self.automations.error =
                Some("Save or explicitly discard the automation form before quitting.".into());
            cx.notify();
            return true;
        }
        false
    }
    pub(super) fn refresh_automations(&mut self, cx: &mut Context<Self>) {
        if self.automations.loading || self.automations.changing {
            return;
        }
        self.automations.loading = true;
        self.automations.generation = self.automations.generation.wrapping_add(1);
        self.automations.refreshed = Instant::now();
        let generation = self.automations.generation;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                Ok::<_, WorkspaceError>((
                    workspace.automations().await?,
                    workspace.catalog().await?,
                    workspace.profiles().await?,
                ))
            }
            .await
            .map_err(|e| e.to_string());
            Ok(Update::Automations(Box::new(Reply::Loaded {
                generation,
                result,
            })))
        });
        cx.notify();
    }
    pub(super) fn tick_automations(&mut self, cx: &mut Context<Self>) {
        if self.close != CloseState::Open {
            self.automations.retire();
            return;
        }
        if (self.panel == Panel::Automations || self.automations.scheduler.busy())
            && self.automations.refreshed.elapsed() >= Duration::from_secs(2)
        {
            self.refresh_automations(cx);
        }
        if self.automations.scheduler.armed()
            && !self.automations.executing
            && !self.automations.changing
            && self.automations.polled.elapsed() >= Duration::from_secs(1)
        {
            self.automations.polled = Instant::now();
            self.automations.executing = true;
            // Capture the cancellation epoch before handing the future to Tokio.
            let future = self.automations.scheduler.tick();
            self.job(async move {
                Ok(Update::Automations(Box::new(Reply::Finished(
                    future.await.map_err(|e| e.to_string()),
                ))))
            });
        }
    }
    pub(super) fn automation_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Loaded { generation, result } => {
                self.automations.loading = false;
                if generation != self.automations.generation {
                    self.refresh_automations(cx);
                    return;
                }
                match result {
                    Ok((ledger, catalog, profiles)) => {
                        self.automations.ledger = ledger;
                        self.automations.loaded = true;
                        self.catalog = catalog;
                        self.profiles = profiles;
                    }
                    Err(error) => {
                        self.automations.error = Some(error);
                        self.automations.scheduler.arm(false);
                    }
                }
            }
            Reply::Changed(result) => {
                self.automations.changing = false;
                let snapshot = self.automations.save_snapshot.take();
                match result {
                    Ok(()) => {
                        self.automations.error = None;
                        if let Some((id, saved_revision)) = snapshot
                            && let Some(editor) = &mut self.automations.editor
                            && editor.id == id
                        {
                            editor.revision = Some(editor.revision.unwrap_or(0).saturating_add(1));
                            if editor.edit_revision == saved_revision {
                                self.automations.editor = None;
                            } else {
                                self.automations.error = Some(
                                    "Saved paused. Newer form edits are still unsaved.".into(),
                                );
                            }
                        }
                    }
                    Err(error) => self.automations.error = Some(error),
                }
                self.refresh_automations(cx);
            }
            Reply::Finished(result) => {
                self.automations.executing = false;
                if let Err(error) = result {
                    self.automations.error = Some(error);
                    self.automations.scheduler.arm(false);
                }
                self.refresh_automations(cx);
            }
            Reply::HistoryExported(result) => {
                self.automations.exporting_history = false;
                match result {
                    Ok(count) => {
                        self.automations.error = None;
                        self.notice = Some(format!(
                            "Exported {count} retained automation run record{} to the selected new file.",
                            if count == 1 { "" } else { "s" }
                        ));
                    }
                    Err(error) => {
                        self.automations.error = Some(format!(
                            "Automation history export failed: {error}. Existing files are never overwritten."
                        ));
                    }
                }
            }
        }
        cx.notify();
    }
    fn edit_automation(
        &mut self,
        definition: Option<AutomationDefinition>,
        cx: &mut Context<Self>,
    ) {
        if self.automations.changing || self.automations.editor.is_some() {
            return;
        }
        let (editor, title, instructions, schedule, timezone, max_runs, failure_limit, max_runtime) =
            match definition {
                Some(d) => (
                    Editor {
                        id: d.id,
                        revision: Some(d.revision),
                        edit_revision: 0,
                        agent: Some(d.agent_id),
                        project: Some(d.project_id),
                        missed: d.missed,
                        context: d.context,
                    },
                    d.title,
                    d.instructions,
                    d.schedule.label(),
                    d.timezone,
                    d.max_runs.map(|n| n.to_string()).unwrap_or_default(),
                    d.stop_after_consecutive_failures
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                    d.max_runtime_seconds.to_string(),
                ),
                None => (
                    Editor {
                        id: AutomationId::new_v4(),
                        revision: None,
                        edit_revision: 0,
                        agent: None,
                        project: self.project,
                        missed: MissedRunPolicy::Skip,
                        context: AutomationContextPolicy::Project,
                    },
                    String::new(),
                    String::new(),
                    "every 60m".into(),
                    "UTC".into(),
                    String::new(),
                    "3".into(),
                    DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS.to_string(),
                ),
            };
        self.automations
            .title
            .update(cx, |entry, cx| entry.set_text(title, cx));
        self.automations
            .instructions
            .update(cx, |entry, cx| entry.set_text(instructions, cx));
        self.automations
            .schedule
            .update(cx, |entry, cx| entry.set_text(schedule, cx));
        self.automations
            .timezone
            .update(cx, |entry, cx| entry.set_text(timezone, cx));
        self.automations
            .max_runs
            .update(cx, |entry, cx| entry.set_text(max_runs, cx));
        self.automations
            .failure_limit
            .update(cx, |entry, cx| entry.set_text(failure_limit, cx));
        self.automations
            .max_runtime
            .update(cx, |entry, cx| entry.set_text(max_runtime, cx));
        self.automations.editor = Some(editor);
        self.automations.pending = None;
        self.automations.error = None;
        cx.notify();
    }
    pub(super) fn open_new_automation(&mut self, cx: &mut Context<Self>) -> bool {
        if self.automations.changing || self.automations.editor.is_some() {
            return false;
        }
        self.set_panel(Panel::Automations, cx);
        self.edit_automation(None, cx);
        self.automations.editor.is_some()
    }
    pub(super) fn open_automation_for_review(
        &mut self,
        id: AutomationId,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.automations.changing {
            self.error = Some("An automation update is in progress. The command was kept.".into());
            return false;
        }
        if self.automations.editor.is_some() {
            self.error = Some(
                "Save or discard the open automation form before opening another. The command was kept."
                    .into(),
            );
            return false;
        }
        if !self.automations.loaded || self.automations.loading {
            self.set_panel(Panel::Automations, cx);
            if self.panel == Panel::Automations {
                self.error = Some(
                    "Automation list is loading. The command was kept; retry after it finishes."
                        .into(),
                );
            }
            return false;
        }
        let Some(definition) = self
            .automations
            .ledger
            .definitions
            .iter()
            .find(|definition| definition.id == id)
            .cloned()
        else {
            self.error = Some(
                "No saved automation with that ID was found. Refresh the list and retry; the command was kept."
                    .into(),
            );
            return false;
        };
        self.set_panel(Panel::Automations, cx);
        if self.panel != Panel::Automations {
            return false;
        }
        self.edit_automation(Some(definition), cx);
        self.automations
            .editor
            .as_ref()
            .is_some_and(|editor| editor.id == id)
    }
    fn save_automation_form(&mut self, cx: &mut Context<Self>) {
        if self.automations.changing {
            return;
        }
        let Some(editor) = self.automations.editor.clone() else {
            return;
        };
        let result = (|| -> WorkspaceResult<_> {
            let definition = AutomationDefinition {
                id: editor.id,
                revision: editor.revision.unwrap_or(0),
                title: self.automations.title.read(cx).text().trim().into(),
                instructions: self.automations.instructions.read(cx).text().into(),
                agent_id: editor
                    .agent
                    .ok_or_else(|| WorkspaceError::Invalid("Choose an agent explicitly.".into()))?,
                project_id: editor
                    .project
                    .ok_or_else(|| WorkspaceError::Invalid("Choose a project.".into()))?,
                schedule: AutomationSchedule::parse(self.automations.schedule.read(cx).text())?,
                timezone: self.automations.timezone.read(cx).text().trim().into(),
                enabled: false,
                next_run_ms: now_ms(),
                missed: editor.missed,
                context: editor.context,
                max_runs: parse_positive_limit(self.automations.max_runs.read(cx).text())?,
                stop_after_consecutive_failures: parse_positive_limit(
                    self.automations.failure_limit.read(cx).text(),
                )?,
                failure_streak: 0,
                run_count: 0,
                max_runtime_seconds: parse_runtime_seconds(
                    self.automations.max_runtime.read(cx).text(),
                )?,
            };
            definition.validate()?;
            Ok(definition)
        })();
        match result {
            Err(error) => self.automations.error = Some(error.to_string()),
            Ok(definition) => {
                self.automations.save_snapshot = Some((editor.id, editor.edit_revision));
                self.automations.changing = true;
                self.automations.generation = self.automations.generation.wrapping_add(1);
                let workspace = self.controller.workspace.clone();
                self.job(async move {
                    Ok(Update::Automations(Box::new(Reply::Changed(
                        workspace
                            .save_automation(definition, editor.revision)
                            .await
                            .map_err(|e| e.to_string()),
                    ))))
                });
            }
        }
        cx.notify();
    }
    fn export_automation_history(&mut self, cx: &mut Context<Self>) {
        if self.automations.exporting_history || self.close != CloseState::Open {
            return;
        }
        self.automations.exporting_history = true;
        self.automations.error = None;
        let picker =
            cx.prompt_for_new_path(&self.scratch_directory, Some("automation-history.json"));
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                if this.close != CloseState::Open {
                    this.automations.exporting_history = false;
                    cx.notify();
                    return;
                }
                match result {
                    Ok(Ok(Some(destination))) => {
                        let workspace = this.controller.workspace.clone();
                        this.job(async move {
                            Ok(Update::Automations(Box::new(Reply::HistoryExported(
                                workspace
                                    .export_automation_history(destination)
                                    .await
                                    .map_err(|error| error.to_string()),
                            ))))
                        });
                    }
                    Ok(Ok(None)) => this.automations.exporting_history = false,
                    _ => {
                        this.automations.exporting_history = false;
                        this.automations.error = Some(
                            "The system save dialog is unavailable. No history was exported."
                                .into(),
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn confirm_automation(&mut self, cx: &mut Context<Self>) {
        if self.automations.changing {
            return;
        }
        let Some(pending) = self.automations.pending.take() else {
            return;
        };
        self.automations.error = None;
        match pending {
            Pending::Arm => {
                self.automations.scheduler.arm(true);
            }
            Pending::Run(definition) => {
                if self.automations.executing {
                    self.automations.error = Some("Another scheduler operation is active. Review and try again when it finishes.".into());
                } else {
                    self.automations.executing = true;
                    let future = self
                        .automations
                        .scheduler
                        .run_now(definition.id, definition.revision);
                    self.job(async move {
                        Ok(Update::Automations(Box::new(Reply::Finished(
                            future.await.map_err(|e| e.to_string()),
                        ))))
                    });
                }
            }
            pending => {
                self.automations.changing = true;
                self.automations.generation = self.automations.generation.wrapping_add(1);
                let workspace = self.controller.workspace.clone();
                let owner = self.automations.scheduler.owner();
                self.job(async move {
                    let result = match pending {
                        Pending::Enable(d, enabled) => {
                            workspace.enable_automation(d.id, d.revision, enabled).await
                        }
                        Pending::Delete(d) => {
                            workspace.delete_automation(d.id, d.revision, true).await
                        }
                        Pending::Recover(run) => {
                            workspace
                                .resolve_interrupted_automation(run.id, owner, true)
                                .await
                        }
                        Pending::PruneHistory(_) => workspace
                            .prune_deleted_automation_history(true)
                            .await
                            .map(|_| ()),
                        Pending::PruneDefinitionHistory(definition, _) => workspace
                            .prune_automation_history(definition.id, definition.revision, true)
                            .await
                            .map(|_| ()),
                        _ => unreachable!("arm/run handled on the UI thread"),
                    }
                    .map_err(|e| e.to_string());
                    Ok(Update::Automations(Box::new(Reply::Changed(result))))
                });
            }
        }
        cx.notify();
    }
    fn open_automation_task(&mut self, id: TaskId, cx: &mut Context<Self>) {
        if self.select_task(id, cx) {
            self.set_panel(Panel::Conversation, cx);
        }
    }
}
fn parse_positive_limit(text: &str) -> WorkspaceResult<Option<u32>> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let value: u32 = text.parse().map_err(|_| {
        WorkspaceError::Invalid("Enter a positive whole-number limit, or leave it empty.".into())
    })?;
    if value == 0 {
        return Err(WorkspaceError::Invalid("Limits must be positive.".into()));
    }
    Ok(Some(value))
}
fn parse_runtime_seconds(text: &str) -> WorkspaceResult<u32> {
    let seconds = parse_positive_limit(text)?.unwrap_or(DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS);
    if seconds > MAX_AUTOMATION_MAX_RUNTIME_SECONDS {
        return Err(WorkspaceError::Invalid(format!(
            "Maximum runtime is {MAX_AUTOMATION_MAX_RUNTIME_SECONDS} seconds."
        )));
    }
    Ok(seconds)
}
fn time_label(value: i64) -> String {
    chrono::DateTime::from_timestamp_millis(value)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Time unavailable".into())
}
