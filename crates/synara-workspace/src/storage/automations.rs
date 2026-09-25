//! One versioned scheduler ledger in the existing SQLite store. Every mutation
//! takes the database writer lock before reading. Transcript events never decide
//! whether a scheduled slot has already been claimed.
use super::organization::automation_hub_context;
use super::*;
use crate::automations::*;
use crate::{
    AgentProfile, DirectModelBinding, ProviderSettings, WorkspaceError, WorkspaceResult,
    default_profiles,
};
const KEY: &str = "automation-ledger-v1";
fn read(connection: &Connection) -> WorkspaceResult<AutomationLedger> {
    let data: Option<String> = connection
        .query_row("SELECT data FROM preferences WHERE key=?1", [KEY], |row| {
            row.get(0)
        })
        .optional()?;
    let ledger: AutomationLedger = data.map(|s| decode(&s)).transpose()?.unwrap_or_default();
    ledger.validate()?;
    Ok(ledger)
}
fn write(connection: &Connection, ledger: &AutomationLedger) -> WorkspaceResult<()> {
    ledger.validate()?;
    connection.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![KEY, encode(ledger)?])?;
    Ok(())
}
fn context(
    connection: &Connection,
    definition: &AutomationDefinition,
) -> WorkspaceResult<(Project, Workspace)> {
    let data: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key='agent_profiles'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let profiles: Vec<AgentProfile> = data
        .map(|s| decode(&s))
        .transpose()?
        .unwrap_or_else(default_profiles);
    if !profiles.iter().any(|p| p.id == definition.agent_id) {
        return Err(invalid(
            "Automation's explicit agent profile is unavailable. Edit and select an installed profile.",
        ));
    }
    let project: String = connection
        .query_row(
            "SELECT data FROM projects WHERE id=?1",
            [definition.project_id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| invalid("Automation project no longer exists."))?;
    let project: Project = decode(&project)?;
    let workspace: String = connection.query_row(
        "SELECT data FROM workspaces WHERE id=?1",
        [project.workspace_id.to_string()],
        |r| r.get(0),
    )?;
    if definition.context == AutomationContextPolicy::Hub {
        let _ = automation_hub_context(connection, project.id)?;
    }
    Ok((project, decode(&workspace)?))
}
fn validate_completion_policy(
    connection: &Connection,
    definition: &AutomationDefinition,
) -> WorkspaceResult<()> {
    let AutomationCompletionPolicy::AiEvaluated { evaluator, .. } = &definition.completion_policy
    else {
        return Ok(());
    };
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key='direct-model-providers-v1'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let settings: ProviderSettings = raw.as_deref().map(decode).transpose()?.unwrap_or_default();
    settings
        .validate()
        .map_err(|error| invalid(error.to_string()))?;
    let profile = evaluator.profile(&settings)?;
    let mut selection = evaluator.selection.clone();
    selection.output = synara_model::OutputFormat::JsonSchema {
        name: "automation_completion".into(),
        schema: serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":["stopMatched","confidence","reason"],
            "properties":{
                "stopMatched":{"type":"boolean"},
                "confidence":{"type":"number","minimum":0,"maximum":1},
                "reason":{"type":"string","maxLength":2000}
            }
        }),
    };
    selection.max_output_tokens = selection.max_output_tokens.min(512);
    synara_model::validate_request(
        profile,
        &selection.request(vec![synara_model::Message::text(
            synara_model::MessageRole::User,
            "Validate automation completion evaluator".into(),
        )]),
    )
    .map_err(|error| invalid(error.to_string()))
}

fn continuation_target_identity(
    connection: &Connection,
    definition: &AutomationDefinition,
    id: TaskId,
) -> WorkspaceResult<Task> {
    let raw: String = connection
        .query_row(
            "SELECT data FROM tasks WHERE id=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| invalid("Automation continuation target no longer exists."))?;
    let task: Task = decode(&raw)?;
    if task.id != id
        || task.project_id != definition.project_id
        || task.agent_id != definition.agent_id
        || task.state == TaskState::Archived
    {
        return Err(invalid(
            "Automation continuation target must be unarchived, in the selected project, and use the selected ACP agent.",
        ));
    }
    let raw_binding: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [format!("task-direct-model:{id}")],
            |row| row.get(0),
        )
        .optional()?;
    let binding = raw_binding
        .as_deref()
        .map(decode::<Option<DirectModelBinding>>)
        .transpose()?
        .flatten();
    if binding.is_some() {
        return Err(invalid(
            "Automation continuation currently supports ACP-owned target conversations only.",
        ));
    }
    Ok(task)
}

fn continuation_target_for_run(
    connection: &Connection,
    ledger: &AutomationLedger,
    definition: &AutomationDefinition,
    id: TaskId,
    scheduled: bool,
    now: i64,
) -> WorkspaceResult<Option<Task>> {
    let task = continuation_target_identity(connection, definition, id)?;
    let defer = |reason: &str| {
        if scheduled {
            Ok(None)
        } else {
            Err(invalid(reason))
        }
    };
    if matches!(task.state, TaskState::Running | TaskState::Waiting) {
        return defer(
            "Automation continuation target is currently active. Wait for it to become idle.",
        );
    }
    if ledger
        .runs
        .iter()
        .any(|run| run.status == AutomationRunStatus::Running && run.task_id == Some(id))
    {
        return defer("Another automation run is already using the continuation target.");
    }
    let draft = crate::storage::chat_preferences::task_draft_text(connection, id)?
        .ok_or(WorkspaceError::NotFound)?;
    if !draft.is_empty() {
        return defer(
            "Automation continuation target has an unsent draft. Send or clear it first.",
        );
    }
    if crate::storage::attachments::has_pending_attachments(connection, id)? {
        return defer(
            "Automation continuation target has pending attachments. Send or remove them first.",
        );
    }
    if definition.heartbeat_cooldown_seconds > 0 {
        let latest_own_finish = ledger
            .runs
            .iter()
            .filter(|run| run.definition.id == definition.id && run.task_id == Some(id))
            .filter_map(|run| run.finished_ms)
            .max();
        let external_activity =
            latest_own_finish.is_none_or(|finished| task.updated_at_ms > finished);
        let cooldown_ms = i64::from(definition.heartbeat_cooldown_seconds) * 1000;
        if external_activity && now.saturating_sub(task.updated_at_ms) < cooldown_ms {
            return defer("Automation continuation target is inside its activity cooldown.");
        }
    }
    Ok(Some(task))
}

impl Store {
    pub(crate) fn automation_ledger(&self) -> WorkspaceResult<AutomationLedger> {
        read(&self.connection)
    }
    pub(crate) fn validate_automation_context(
        &self,
        definition: &AutomationDefinition,
    ) -> WorkspaceResult<()> {
        context(&self.connection, definition)?;
        validate_completion_policy(&self.connection, definition)?;
        match (definition.mode, definition.target_task_id) {
            (AutomationMode::Heartbeat, Some(target)) => {
                continuation_target_identity(&self.connection, definition, target)?;
            }
            (AutomationMode::Heartbeat, None) => {
                return Err(invalid(
                    "Heartbeat automations require a target conversation.",
                ));
            }
            (AutomationMode::Standalone, Some(_)) => {
                return Err(invalid(
                    "Standalone automations cannot keep a target conversation.",
                ));
            }
            (AutomationMode::Dedicated, Some(target)) => {
                continuation_target_identity(&self.connection, definition, target)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub(crate) fn edit_automations<R>(
        &mut self,
        edit: impl FnOnce(&mut AutomationLedger) -> WorkspaceResult<R>,
    ) -> WorkspaceResult<R> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut ledger = read(&tx)?;
        let value = edit(&mut ledger)?;
        write(&tx, &ledger)?;
        tx.commit()?;
        Ok(value)
    }
    pub(crate) fn claim_automation(
        &mut self,
        id: AutomationId,
        owner: AutomationId,
        scheduled: bool,
        now: i64,
        revision: Option<u64>,
    ) -> WorkspaceResult<Option<AutomationRun>> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut ledger = read(&tx)?;
        let Some(index) = ledger.definitions.iter().position(|d| d.id == id) else {
            return Err(invalid("Automation no longer exists."));
        };
        let mut definition = ledger.definitions[index].clone();
        if revision.is_some_and(|expected| expected != definition.revision) {
            return Err(invalid(
                "Automation changed after confirmation. Reload and review its current instructions before running.",
            ));
        }
        if scheduled && (!definition.enabled || definition.next_run_ms > now) {
            return Ok(None);
        }
        if ledger
            .runs
            .iter()
            .any(|r| r.definition.id == id && r.status == AutomationRunStatus::Running)
        {
            return Ok(None);
        }
        let previous_runs = effective_run_count(&definition, &ledger.runs);
        if definition
            .max_runs
            .is_some_and(|limit| previous_runs >= limit)
        {
            return Err(invalid(
                "Run limit reached. Increase the limit before running again.",
            ));
        }
        if ledger.runs.len() >= 256 {
            return Err(invalid(
                "Automation history is full. No run was started. Retained history is not silently deleted.",
            ));
        }
        let scheduled_ms = scheduled.then_some(definition.next_run_ms);
        if let Some(slot) = scheduled_ms {
            if ledger
                .runs
                .iter()
                .any(|r| r.definition.id == id && r.scheduled_ms == Some(slot))
            {
                return Err(invalid("Scheduled slot was already claimed."));
            }
            ledger.definitions[index].next_run_ms =
                definition
                    .schedule
                    .advance(slot, now, &definition.timezone)?;
            if definition.missed == MissedRunPolicy::Skip && now.saturating_sub(slot) > 30_000 {
                ledger.runs.push(AutomationRun {
                    id: AutomationId::new_v4(),
                    definition,
                    owner,
                    scheduled_ms,
                    started_ms: now,
                    finished_ms: Some(now),
                    status: AutomationRunStatus::Skipped,
                    task_id: None,
                    prompt: String::new(),
                    hub_revision: None,
                    completion_evaluation: None,
                    output: "Missed-run policy skipped an overdue slot. No agent was launched."
                        .into(),
                });
                write(&tx, &ledger)?;
                tx.commit()?;
                return Ok(None);
            }
        }
        let (project, workspace) = context(&tx, &definition)?;
        let (new_scope, prompt, hub_revision) = match definition.context {
            AutomationContextPolicy::Project => {
                (TaskScope::Project, definition.instructions.clone(), None)
            }
            AutomationContextPolicy::Hub => {
                let (revision, shared) = automation_hub_context(&tx, project.id)?;
                let mut prompt = shared;
                prompt.push_str(&definition.instructions);
                if prompt.len() > MAX_AUTOMATION_PROMPT_BYTES || prompt.contains('\0') {
                    return Err(invalid(
                        "Combined Hub automation context exceeds the 128 KiB prompt limit.",
                    ));
                }
                (TaskScope::Studio, prompt, Some(revision))
            }
        };

        let mut created_task = false;
        let task = match definition.mode {
            AutomationMode::Standalone => {
                created_task = true;
                Task {
                    id: TaskId::new(),
                    project_id: project.id,
                    title: format!("Automation: {}", definition.title),
                    state: TaskState::Ready,
                    thread_id: ThreadId::new(),
                    agent_id: definition.agent_id.clone(),
                    working_directory: crate::service::project_directory(&workspace, &project)?,
                    updated_at_ms: now,
                    scope: new_scope,
                }
            }
            AutomationMode::Heartbeat => match continuation_target_for_run(
                &tx,
                &ledger,
                &definition,
                definition
                    .target_task_id
                    .ok_or_else(|| invalid("Heartbeat automation has no target conversation."))?,
                scheduled,
                now,
            )? {
                Some(task) => task,
                None => return Ok(None),
            },
            AutomationMode::Dedicated => match definition.target_task_id {
                Some(target) => match continuation_target_for_run(
                    &tx,
                    &ledger,
                    &definition,
                    target,
                    scheduled,
                    now,
                )? {
                    Some(task) => task,
                    None => return Ok(None),
                },
                None => {
                    created_task = true;
                    let task = Task {
                        id: TaskId::new(),
                        project_id: project.id,
                        title: format!("Automation: {}", definition.title),
                        state: TaskState::Ready,
                        thread_id: ThreadId::new(),
                        agent_id: definition.agent_id.clone(),
                        working_directory: crate::service::project_directory(&workspace, &project)?,
                        updated_at_ms: now,
                        scope: new_scope,
                    };
                    definition.target_task_id = Some(task.id);
                    ledger.definitions[index].target_task_id = Some(task.id);
                    task
                }
            },
        };

        let run = AutomationRun {
            id: AutomationId::new_v4(),
            definition,
            owner,
            scheduled_ms,
            started_ms: now,
            finished_ms: None,
            status: AutomationRunStatus::Running,
            task_id: Some(task.id),
            prompt,
            hub_revision,
            completion_evaluation: None,
            output: String::new(),
        };
        // Claimed slot, conversation identity (when new) and its visible unsent
        // prompt are one transaction. Continuation modes never replace a nonempty
        // user draft because continuation_target_for_run checked it above.
        if created_task {
            tx.execute(
                "INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5)",
                params![
                    task.id.to_string(),
                    task.project_id.to_string(),
                    task.thread_id.to_string(),
                    task.updated_at_ms,
                    encode(&task)?
                ],
            )?;
        }
        tx.execute(
            "INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",
            params![
                format!("task-draft:{}", task.id),
                encode(&serde_json::json!({"version":1,"text":run.prompt}))?
            ],
        )?;
        ledger.runs.push(run.clone());
        let next_run_count = previous_runs
            .checked_add(1)
            .ok_or_else(|| invalid("Automation run count overflow."))?;
        ledger.definitions[index].run_count = next_run_count;
        if run
            .definition
            .max_runs
            .is_some_and(|limit| next_run_count >= limit)
        {
            ledger.definitions[index].enabled = false;
            ledger.definitions[index].revision = ledger.definitions[index]
                .revision
                .checked_add(1)
                .ok_or_else(|| invalid("Revision overflow."))?;
        }
        write(&tx, &ledger)?;
        tx.commit()?;
        Ok(Some(run))
    }
}
