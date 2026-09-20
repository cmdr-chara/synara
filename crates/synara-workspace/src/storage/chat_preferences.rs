//! User-owned chat preferences. Reads never start agents or alter events.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
const MAX_DRAFT: usize = 1024 * 1024;
const MAX_FAVORITES: usize = 256;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelFavorite {
    pub agent: String,
    /// None denotes the legacy model selector, not a config option.
    pub option: Option<String>,
    pub value: String,
}
impl ModelFavorite {
    fn valid(&self) -> bool {
        fn text(s: &str) -> bool {
            !s.is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control)
        }
        text(&self.agent)
            && self.agent.len() <= 128
            && text(&self.value)
            && self.option.as_deref().is_none_or(text)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Favorites {
    version: u32,
    entries: Vec<ModelFavorite>,
}
impl Default for Favorites {
    fn default() -> Self {
        Self {
            version: 1,
            entries: vec![],
        }
    }
}
impl Favorites {
    fn validate(&self) -> StorageResult<()> {
        if self.version != 1
            || self.entries.len() > MAX_FAVORITES
            || self.entries.iter().any(|e| !e.valid())
            || self
                .entries
                .iter()
                .enumerate()
                .any(|(i, e)| self.entries[..i].contains(e))
        {
            return Err(StorageError::Identity);
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    version: u32,
    text: String,
}
impl Store {
    fn model_favorites(&self) -> StorageResult<Vec<ModelFavorite>> {
        let stored = self
            .preference::<Favorites>("model-favorites")?
            .unwrap_or_default();
        stored.validate()?;
        Ok(stored.entries)
    }
    fn set_model_favorite(
        &mut self,
        value: ModelFavorite,
        enabled: bool,
    ) -> StorageResult<Vec<ModelFavorite>> {
        if !value.valid() {
            return Err(StorageError::Identity);
        }
        // Read/modify/write is one SQLite transaction, including across separate openers.
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let raw: Option<String> = tx
            .query_row(
                "SELECT data FROM preferences WHERE key='model-favorites'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let mut stored = raw
            .as_deref()
            .map(decode::<Favorites>)
            .transpose()?
            .unwrap_or_default();
        stored.validate()?;
        if enabled && !stored.entries.contains(&value) {
            stored.entries.push(value);
        } else if !enabled {
            stored.entries.retain(|e| e != &value);
        }
        stored.validate()?;
        tx.execute("INSERT INTO preferences(key,data) VALUES('model-favorites',?1) ON CONFLICT(key) DO UPDATE SET data=excluded.data", [encode(&stored)?])?;
        tx.commit()?;
        Ok(stored.entries)
    }
    fn task_draft(&self, id: TaskId) -> StorageResult<Option<String>> {
        if self.task(id)?.is_none() {
            return Ok(None);
        }
        let draft = self.preference::<Draft>(&format!("task-draft:{id}"))?;
        match draft {
            Some(draft) if draft.version == 1 && draft.text.len() <= MAX_DRAFT => {
                Ok(Some(draft.text))
            }
            Some(_) => Err(StorageError::Limit),
            None => Ok(Some(String::new())),
        }
    }
    fn save_task_draft(&mut self, id: TaskId, text: String) -> StorageResult<bool> {
        if text.len() > MAX_DRAFT {
            return Err(StorageError::Limit);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)",
            [id.to_string()],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(false);
        }
        let data = encode(&Draft { version: 1, text })?;
        tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![format!("task-draft:{id}"), data])?;
        tx.commit()?;
        Ok(true)
    }
}
impl WorkspaceService {
    pub async fn model_favorites(&self) -> WorkspaceResult<Vec<ModelFavorite>> {
        self.access(|store| Ok(store.model_favorites()?)).await
    }
    /// Sets an explicit state, rather than a retry-sensitive toggle.
    pub async fn set_model_favorite(
        &self,
        favorite: ModelFavorite,
        enabled: bool,
    ) -> WorkspaceResult<Vec<ModelFavorite>> {
        self.access(move |store| Ok(store.set_model_favorite(favorite, enabled)?))
            .await
    }
    pub async fn task_draft(&self, id: TaskId) -> WorkspaceResult<String> {
        self.access(move |store| store.task_draft(id)?.ok_or(WorkspaceError::NotFound))
            .await
    }
    pub async fn save_task_draft(&self, id: TaskId, text: String) -> WorkspaceResult<()> {
        self.access(move |store| {
            if store.save_task_draft(id, text)? {
                Ok(())
            } else {
                Err(WorkspaceError::NotFound)
            }
        })
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn favorite(agent: &str, value: &str) -> ModelFavorite {
        ModelFavorite {
            agent: agent.into(),
            option: Some("model".into()),
            value: value.into(),
        }
    }
    #[tokio::test]
    async fn favorite_identity_is_provider_and_option_scoped_and_sets_are_idempotent() {
        let s = WorkspaceService::memory().unwrap();
        let a = favorite("alpha", "same");
        let b = favorite("beta", "same");
        s.set_model_favorite(a.clone(), true).await.unwrap();
        assert_eq!(
            s.set_model_favorite(a.clone(), true).await.unwrap().len(),
            1
        );
        assert_eq!(
            s.set_model_favorite(b.clone(), true).await.unwrap().len(),
            2
        );
        let mut legacy = a.clone();
        legacy.option = None;
        assert_eq!(s.set_model_favorite(legacy, true).await.unwrap().len(), 3);
        assert_eq!(s.set_model_favorite(a, false).await.unwrap().len(), 2);
        assert!(s.model_favorites().await.unwrap().contains(&b));
    }
    #[tokio::test]
    async fn favorites_survive_reopen_and_unrelated_settings_saves() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let s = WorkspaceService::open(path.clone()).await.unwrap();
        let stale_settings = s.settings().await.unwrap().settings;
        let a = favorite("alpha", "a");
        s.set_model_favorite(a.clone(), true).await.unwrap();
        s.save_settings(stale_settings).await.unwrap();
        drop(s);
        assert_eq!(
            WorkspaceService::open(path)
                .await
                .unwrap()
                .model_favorites()
                .await
                .unwrap(),
            vec![a]
        );
    }
    #[tokio::test]
    async fn concurrent_openers_do_not_lose_favorites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let a = WorkspaceService::open(path.clone()).await.unwrap();
        let b = WorkspaceService::open(path).await.unwrap();
        let (x, y) = tokio::join!(
            a.set_model_favorite(favorite("a", "m"), true),
            b.set_model_favorite(favorite("b", "m"), true)
        );
        x.unwrap();
        y.unwrap();
        assert_eq!(a.model_favorites().await.unwrap().len(), 2);
    }
    #[tokio::test]
    async fn malformed_preferences_are_not_silently_replaced() {
        let s = WorkspaceService::memory().unwrap();
        s.access(|store| {
            store.set_preference(
                "model-favorites",
                &serde_json::json!({"version":99,"entries":[]}),
            )?;
            Ok(())
        })
        .await
        .unwrap();
        assert!(s.model_favorites().await.is_err());
        assert!(
            s.set_model_favorite(favorite("a", "m"), true)
                .await
                .is_err()
        );
        s.access(|store| {
            assert_eq!(
                store
                    .preference::<serde_json::Value>("model-favorites")?
                    .unwrap()["version"],
                99
            );
            Ok(())
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn favorites_are_bounded_and_hostile_values_preserve_last_good_state() {
        let s = WorkspaceService::memory().unwrap();
        s.set_model_favorite(favorite("a", "m"), true)
            .await
            .unwrap();
        for value in ["".to_owned(), "x\ny".into(), "x".repeat(1025)] {
            assert!(
                s.set_model_favorite(favorite("a", &value), true)
                    .await
                    .is_err()
            );
        }
        assert_eq!(s.model_favorites().await.unwrap().len(), 1);
        let stored = Favorites {
            version: 1,
            entries: (0..257).map(|i| favorite("a", &i.to_string())).collect(),
        };
        assert!(stored.validate().is_err());
    }
    #[tokio::test]
    async fn drafts_restore_unicode_without_events_sessions_or_autostart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let s = WorkspaceService::open(path.clone()).await.unwrap();
        let p = s.add_local_workspace(dir.path().into()).await.unwrap();
        let agent = s.profiles().await.unwrap()[0].id.clone();
        let task = s.create_task(p.id, "Draft".into(), agent).await.unwrap();
        let text = "Caffè 日本語\nUnsent @src/main.rs";
        s.save_task_draft(task.id, text.into()).await.unwrap();
        drop(s);
        let s = WorkspaceService::open(path).await.unwrap();
        assert_eq!(s.task_draft(task.id).await.unwrap(), text);
        assert!(s.thread(task.thread_id).await.unwrap().messages.is_empty());
        assert!(s.session(task.thread_id).await.unwrap().is_none());
        assert_eq!(s.task(task.id).await.unwrap().state, TaskState::Ready);
    }
    #[tokio::test]
    async fn oversized_or_deleted_task_drafts_cannot_leave_orphan_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let s = WorkspaceService::memory().unwrap();
        let p = s.add_local_workspace(dir.path().into()).await.unwrap();
        let agent = s.profiles().await.unwrap()[0].id.clone();
        let task = s.create_task(p.id, "Draft".into(), agent).await.unwrap();
        s.save_task_draft(task.id, "keep".into()).await.unwrap();
        assert!(
            s.save_task_draft(task.id, "x".repeat(MAX_DRAFT + 1))
                .await
                .is_err()
        );
        assert_eq!(s.task_draft(task.id).await.unwrap(), "keep");
        s.archive_task(task.id).await.unwrap();
        s.delete_task(task.id).await.unwrap();
        assert!(s.save_task_draft(task.id, "orphan".into()).await.is_err());
        s.access(move |store| {
            assert!(
                store
                    .preference::<Draft>(&format!("task-draft:{}", task.id))?
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
    }
    #[test]
    fn draft_keys_reject_paths_and_invalid_ids() {
        for key in [
            "task-draft:",
            "task-draft:../settings",
            "task-draft:not-a-uuid",
        ] {
            assert!(!valid_preference_key(key));
        }
        assert!(valid_preference_key(&format!(
            "task-draft:{}",
            TaskId::new()
        )));
    }
}
