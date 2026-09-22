//! Persisted direct-model choices. Configuration is inert, revision checked and
//! separate from agent profiles, agent sessions and ACP configuration.
use crate::{StorageError, WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use synara_core::TaskId;
pub use synara_model::{ModelSelection, ProviderProfile, ProviderSettings};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectModelBinding {
    pub selection: ModelSelection,
    pub reviewed_profile_sha256: String,
}
impl DirectModelBinding {
    pub fn profile<'a>(
        &self,
        settings: &'a ProviderSettings,
    ) -> WorkspaceResult<&'a ProviderProfile> {
        let profile = settings
            .providers
            .iter()
            .find(|p| p.id == self.selection.provider_id)
            .ok_or_else(|| {
                WorkspaceError::Invalid(
                    "The direct provider was removed. Review another model in Settings.".into(),
                )
            })?;
        if profile_digest(profile)? != self.reviewed_profile_sha256 {
            return Err(WorkspaceError::Invalid("The direct provider configuration changed. Review and select it again before sending.".into()));
        }
        Ok(profile)
    }
}
pub(crate) fn profile_digest(profile: &ProviderProfile) -> WorkspaceResult<String> {
    let bytes = serde_json::to_vec(profile).map_err(StorageError::Encoding)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
impl WorkspaceService {
    pub async fn direct_model_settings(&self) -> WorkspaceResult<ProviderSettings> {
        self.access(|store| {
            let settings: ProviderSettings = store
                .preference("direct-model-providers-v1")?
                .unwrap_or_default();
            settings
                .validate()
                .map_err(|e| WorkspaceError::Invalid(e.to_string()))?;
            Ok(settings)
        })
        .await
    }
    pub async fn direct_model_binding(
        &self,
        task: TaskId,
    ) -> WorkspaceResult<Option<DirectModelBinding>> {
        self.access(move |store| {
            store.task(task)?.ok_or(WorkspaceError::NotFound)?;
            Ok(store
                .preference::<Option<DirectModelBinding>>(&format!("task-direct-model:{task}"))?
                .flatten())
        })
        .await
    }
    pub(crate) async fn save_direct_model_settings(
        &self,
        settings: ProviderSettings,
    ) -> WorkspaceResult<ProviderSettings> {
        settings
            .validate()
            .map_err(|e| WorkspaceError::Invalid(e.to_string()))?;
        self.access(move |store| store.save_direct_model_settings(settings))
            .await
    }
    pub(crate) async fn bind_direct_model(
        &self,
        task: TaskId,
        selection: Option<ModelSelection>,
        settings_revision: u64,
        reviewed_sequence: u64,
    ) -> WorkspaceResult<Option<DirectModelBinding>> {
        self.access(move |store| {
            store.bind_direct_model(task, selection, settings_revision, reviewed_sequence)
        })
        .await
    }
}
