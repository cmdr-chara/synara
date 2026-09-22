use super::*;
use crate::{
    DirectModelBinding, ModelSelection, ProviderSettings, WorkspaceError, WorkspaceResult,
    direct_models::profile_digest,
};

impl Store {
    pub(crate) fn save_direct_model_settings(
        &mut self,
        mut settings: ProviderSettings,
    ) -> WorkspaceResult<ProviderSettings> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let data: Option<String> = tx
            .query_row(
                "SELECT data FROM preferences WHERE key='direct-model-providers-v1'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let current: ProviderSettings = data.map(|d| decode(&d)).transpose()?.unwrap_or_default();
        if current.revision != settings.revision {
            return Err(WorkspaceError::Invalid(
                "Direct provider settings changed. Reload before saving.".into(),
            ));
        }
        settings.revision = settings
            .revision
            .checked_add(1)
            .ok_or(StorageError::Limit)?;
        tx.execute("INSERT INTO preferences(key,data) VALUES('direct-model-providers-v1',?1) ON CONFLICT(key) DO UPDATE SET data=excluded.data", [encode(&settings)?])?;
        tx.commit()?;
        Ok(settings)
    }
    pub(crate) fn bind_direct_model(
        &mut self,
        id: TaskId,
        selection: Option<ModelSelection>,
        settings_revision: u64,
        reviewed_sequence: u64,
    ) -> WorkspaceResult<Option<DirectModelBinding>> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let task: Option<String> = tx
            .query_row(
                "SELECT data FROM tasks WHERE id=?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        let task: Task = decode(&task.ok_or(WorkspaceError::NotFound)?)?;
        if matches!(
            task.state,
            TaskState::Running | TaskState::Waiting | TaskState::Archived
        ) {
            return Err(WorkspaceError::Invalid(
                "Choose an idle, unarchived conversation.".into(),
            ));
        }
        let sequence: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence),0) FROM events WHERE thread_id=?1",
            [task.thread_id.to_string()],
            |r| r.get(0),
        )?;
        if u64::try_from(sequence).ok() != Some(reviewed_sequence) {
            return Err(WorkspaceError::Invalid(
                "Conversation changed after model review. Review it again.".into(),
            ));
        }
        let raw: Option<String> = tx
            .query_row(
                "SELECT data FROM preferences WHERE key='direct-model-providers-v1'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let settings: ProviderSettings = raw.map(|d| decode(&d)).transpose()?.unwrap_or_default();
        if settings.revision != settings_revision {
            return Err(WorkspaceError::Invalid(
                "Provider settings changed after model review.".into(),
            ));
        }
        let binding = selection
            .map(|selection| -> WorkspaceResult<DirectModelBinding> {
                let profile = settings
                    .providers
                    .iter()
                    .find(|p| p.id == selection.provider_id)
                    .ok_or(WorkspaceError::NotFound)?;
                let request = selection.request(vec![synara_model::Message::text(
                    synara_model::MessageRole::User,
                    "Validate model options".into(),
                )]);
                synara_model::validate_request(profile, &request)
                    .map_err(|e| WorkspaceError::Invalid(e.to_string()))?;
                Ok(DirectModelBinding {
                    selection,
                    reviewed_profile_sha256: profile_digest(profile)?,
                })
            })
            .transpose()?;
        tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![format!("task-direct-model:{id}"),encode(&binding)?])?;
        // Explicit route changes never clone/resume an old provider session. The
        // transcript and all files remain intact. No prompt executes here.
        tx.execute(
            "DELETE FROM sessions WHERE thread_id=?1",
            [task.thread_id.to_string()],
        )?;
        tx.commit()?;
        Ok(binding)
    }
}
