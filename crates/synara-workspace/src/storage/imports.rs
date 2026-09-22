//! Atomic installation of an already reviewed, read-only history snapshot.
use super::*;
use crate::imports::*;
use crate::{AgentProfile, WorkspaceError, WorkspaceResult, default_profiles, now_ms};
use tokio_util::sync::CancellationToken;

fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
fn ledger(db: &Connection) -> WorkspaceResult<HistoryImportLedger> {
    let raw: Option<String> = db
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [IMPORT_LEDGER],
            |row| row.get(0),
        )
        .optional()?;
    if raw.as_ref().is_some_and(|raw| raw.len() > 2 * 1024 * 1024) {
        return Err(StorageError::Limit.into());
    }
    let value: HistoryImportLedger = raw.as_deref().map(decode).transpose()?.unwrap_or_default();
    value.validate()?;
    Ok(value)
}
fn destination(
    db: &Connection,
    project: ProjectId,
    agent_id: &str,
) -> WorkspaceResult<(Project, WorkspaceLocation, std::path::PathBuf, String)> {
    let raw: Option<String> = db
        .query_row(
            "SELECT data FROM projects WHERE id=?1",
            [project.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    let record: Project = decode(&raw.ok_or(WorkspaceError::NotFound)?)?;
    if record.id != project {
        return Err(StorageError::Identity.into());
    }
    let raw: String = db.query_row(
        "SELECT data FROM workspaces WHERE id=?1",
        [record.workspace_id.to_string()],
        |row| row.get(0),
    )?;
    let workspace: Workspace = decode(&raw)?;
    if workspace.id != record.workspace_id {
        return Err(StorageError::Identity.into());
    }
    if !matches!(workspace.location, WorkspaceLocation::Local { .. }) {
        return Err(invalid(
            "History import requires a reviewed local destination. It never maps source paths into an SSH workspace.",
        ));
    }
    let directory = crate::service::project_directory(&workspace, &record)?;
    let raw: Option<String> = db
        .query_row(
            "SELECT data FROM preferences WHERE key='agent_profiles'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let profiles: Vec<AgentProfile> = raw
        .as_deref()
        .map(decode)
        .transpose()?
        .unwrap_or_else(default_profiles);
    let agent = profiles
        .iter()
        .find(|profile| profile.id == agent_id)
        .ok_or_else(|| invalid("The selected future agent no longer exists."))?;
    Ok((record, workspace.location, directory, agent.name.clone()))
}
fn matching_task(db: &Connection, receipt: &HistoryImportReceipt) -> WorkspaceResult<Option<Task>> {
    let raw: Option<String> = db
        .query_row(
            "SELECT data FROM tasks WHERE id=?1",
            [receipt.task.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    let task: Option<Task> = raw.as_deref().map(decode).transpose()?;
    if task
        .as_ref()
        .is_some_and(|task| task.id != receipt.task || task.project_id != receipt.project)
    {
        return Err(StorageError::Identity.into());
    }
    Ok(task)
}
impl Store {
    pub(crate) fn review_history_import(
        &mut self,
        preview: HistoryPreview,
        project: ProjectId,
        agent_id: String,
    ) -> WorkspaceResult<HistoryImportReview> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)?;
        let (project, location, working_directory, agent_name) =
            destination(&tx, project, &agent_id)?;
        let prior = ledger(&tx)?
            .receipts
            .into_iter()
            .find(|receipt| receipt.identity == preview.identity());
        Ok(HistoryImportReview {
            preview,
            project,
            location,
            working_directory,
            agent_id,
            agent_name,
            prior,
        })
    }
    pub(crate) fn import_history(
        &mut self,
        review: HistoryImportReview,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<HistoryImportOutcome> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if cancel.is_cancelled() {
            return Err(invalid("History import cancelled before commit."));
        }
        let mut ledger = ledger(&tx)?;
        let identity = review.preview.identity();
        if let Some(receipt) = ledger
            .receipts
            .iter()
            .find(|receipt| receipt.identity == identity)
        {
            return Ok(HistoryImportOutcome::AlreadyImported {
                task: matching_task(&tx, receipt)?,
                source_changed: receipt.sha256 != review.preview.digest
                    || receipt.leaf != review.preview.selected_leaf,
            });
        }
        if ledger.receipts.len() == 4096 {
            return Err(invalid(
                "The local import ledger is full. No history was imported.",
            ));
        }
        let (project, location, directory, agent_name) =
            destination(&tx, review.project.id, &review.agent_id)?;
        if project.workspace_id != review.project.workspace_id
            || project.relative_directory != review.project.relative_directory
            || project.name != review.project.name
            || location != review.location
            || directory != review.working_directory
            || agent_name != review.agent_name
        {
            return Err(invalid(
                "The destination or future agent changed after review. Review it again. Nothing was imported.",
            ));
        }
        let imported_at = now_ms();
        let task = Task {
            id: TaskId::new(),
            project_id: project.id,
            thread_id: ThreadId::new(),
            title: format!("[Imported] {}", review.preview.title),
            state: TaskState::Ready,
            agent_id: review.agent_id,
            working_directory: directory,
            updated_at_ms: imported_at,
            scope: TaskScope::Chat,
        };
        let mut events = vec![
            (
                imported_at,
                ThreadEvent::TitleChanged {
                    title: task.title.clone(),
                },
            ),
            (
                imported_at,
                ThreadEvent::Notice {
                    message: format!(
                        "Imported {} local text snapshot ({}). Source history is unchanged. No provider session, hidden reasoning, tool state, approvals, attachment or filesystem state was imported. Nothing was sent. An ACP agent starts a fresh session only on explicit Send. Direct model use requires its own reviewed model selection.",
                        review.preview.provider().label(),
                        review.preview.session
                    ),
                },
            ),
        ];
        for (index, message) in review.preview.messages.iter().enumerate() {
            events.push((
                message.timestamp_ms.unwrap_or(imported_at),
                ThreadEvent::TextDelta {
                    message_id: Some(format!("import:{}:{index}", task.id)),
                    role: message.role,
                    text: message.text.clone(),
                },
            ));
        }
        let mut thread = Thread::new(task.thread_id);
        let mut activity = ThreadActivity::new(task.title.clone());
        let mut encoded = Vec::with_capacity(events.len());
        let mut bytes = 0i64;
        for (index, (timestamp_ms, event)) in events.into_iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(invalid("History import cancelled before commit."));
            }
            let envelope = EventEnvelope {
                id: EventId::new(),
                thread_id: task.thread_id,
                sequence: index as u64 + 1,
                timestamp_ms,
                event,
            };
            thread.apply(&envelope).map_err(StorageError::from)?;
            activity.apply(&envelope.event);
            let data = encode(&envelope.event)?;
            bytes = bytes
                .checked_add(data.len() as i64)
                .ok_or(StorageError::Limit)?;
            if bytes > 16 * 1024 * 1024 {
                return Err(StorageError::Limit.into());
            }
            encoded.push((envelope, data));
        }
        if thread.state != TaskState::Ready
            || !thread.permissions.is_empty()
            || !thread.inputs.is_empty()
            || !thread.tools.is_empty()
        {
            return Err(StorageError::Identity.into());
        }
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
        for (envelope, data) in &encoded {
            if cancel.is_cancelled() {
                return Err(invalid("History import cancelled before commit."));
            }
            tx.execute("INSERT INTO events(thread_id,sequence,id,timestamp_ms,data) VALUES(?1,?2,?3,?4,?5)",params![task.thread_id.to_string(),envelope.sequence as i64,envelope.id.to_string(),envelope.timestamp_ms,data])?;
        }
        tx.execute(
            "INSERT INTO event_heads(thread_id,sequence,bytes) VALUES(?1,?2,?3)",
            params![task.thread_id.to_string(), encoded.len() as i64, bytes],
        )?;
        update_activity(
            &tx,
            task.clone(),
            encoded.len() as i64,
            imported_at,
            &activity,
        )?;
        tx.execute(
            "INSERT INTO preferences(key,data) VALUES(?1,?2)",
            params![
                format!("task-draft:{}", task.id),
                encode(&serde_json::json!({"version":1,"text":""}))?
            ],
        )?;
        ledger.receipts.push(HistoryImportReceipt {
            identity,
            sha256: review.preview.digest,
            provider: review.preview.source.provider,
            session_id: review.preview.session,
            leaf: review.preview.selected_leaf,
            task: task.id,
            project: task.project_id,
            imported_at_ms: imported_at,
            message_count: review.preview.messages.len(),
        });
        ledger.validate()?;
        tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",params![IMPORT_LEDGER,encode(&ledger)?])?;
        if cancel.is_cancelled() {
            return Err(invalid("History import cancelled before commit."));
        }
        tx.commit()?;
        Ok(HistoryImportOutcome::Imported(task))
    }
}

#[cfg(test)]
impl Store {
    pub(crate) fn history_import_fault(&mut self, enabled: bool) -> StorageResult<()> {
        self.connection.execute_batch(if enabled {
            "CREATE TRIGGER fail_history_import BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT,'fixture failure'); END;"
        } else { "DROP TRIGGER fail_history_import" })?;
        Ok(())
    }
}
