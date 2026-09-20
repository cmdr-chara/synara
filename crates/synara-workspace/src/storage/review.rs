//! Non-secret, project/worktree-scoped review choices and commit drafts.
//! Diff contents are derived from Git and are never persisted here.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::path::{Component, PathBuf};

const MAX_RECORD_BYTES: usize = 512 * 1024;
pub const MAX_COMMIT_DRAFT_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewScope {
    pub project: ProjectId,
    pub root: PathBuf,
}
impl ReviewScope {
    fn key(&self) -> WorkspaceResult<String> {
        if !self.root.has_root() || self.root.as_os_str().len() > 8192 {
            return Err(WorkspaceError::Invalid("Invalid Git review scope".into()));
        }
        Ok(format!("git-review:{}", encode(self)?))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewPreferences {
    pub version: u32,
    pub revision: u64,
    pub path: Option<PathBuf>,
    pub staged: bool,
    pub raw: bool,
    pub commit_message: String,
}
impl Default for ReviewPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            path: None,
            staged: false,
            raw: false,
            commit_message: String::new(),
        }
    }
}
impl ReviewPreferences {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1
            || self.commit_message.len() > MAX_COMMIT_DRAFT_BYTES
            || self.commit_message.contains('\0')
            || self.path.as_ref().is_some_and(|path| {
                path.as_os_str().is_empty()
                    || path.as_os_str().len() > 8192
                    || path
                        .components()
                        .any(|part| !matches!(part, Component::Normal(_)))
            })
        {
            return Err(WorkspaceError::Invalid(
                "Invalid review choices or commit draft exceeds 64 KiB. Your text has not been discarded.".into(),
            ));
        }
        Ok(())
    }
}
fn read(connection: &Connection, scope: &ReviewScope) -> WorkspaceResult<ReviewPreferences> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
            [scope.project.to_string()],
            |row| row.get(0),
        )
        .map_err(StorageError::from)?;
    if !exists {
        return Err(WorkspaceError::NotFound);
    }
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [scope.key()?],
            |row| row.get(0),
        )
        .optional()
        .map_err(StorageError::from)?;
    let value = match raw {
        Some(raw) if raw.len() > MAX_RECORD_BYTES => return Err(StorageError::Limit.into()),
        Some(raw) => decode::<ReviewPreferences>(&raw)?,
        None => ReviewPreferences::default(),
    };
    value.validate()?;
    Ok(value)
}
impl WorkspaceService {
    pub async fn review_preferences(
        &self,
        scope: ReviewScope,
    ) -> WorkspaceResult<ReviewPreferences> {
        self.access(move |store| read(&store.connection, &scope))
            .await
    }

    /// A serialized transaction and revision check protect a newer window's draft.
    /// Unknown/future values are read before writing, never replaced with defaults.
    pub async fn save_review_preferences(
        &self,
        scope: ReviewScope,
        expected_revision: u64,
        mut value: ReviewPreferences,
    ) -> WorkspaceResult<ReviewPreferences> {
        value.validate()?;
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(StorageError::from)?;
            let current = read(&tx, &scope)?;
            if current.revision != expected_revision || value.revision != expected_revision {
                return Err(WorkspaceError::Invalid(
                    "Git review changed in another window. Copy your local draft, then explicitly reload the saved version.".into(),
                ));
            }
            if current == value {
                tx.commit().map_err(StorageError::from)?;
                return Ok(current);
            }
            value.revision = expected_revision.checked_add(1).ok_or(StorageError::Limit)?;
            let data = encode(&value)?;
            if data.len() > MAX_RECORD_BYTES {
                return Err(StorageError::Limit.into());
            }
            tx.execute(
                "INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",
                params![scope.key()?, data],
            ).map_err(StorageError::from)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(value)
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn scope(service: &WorkspaceService, path: &Path) -> ReviewScope {
        let project = service.add_local_workspace(path.into()).await.unwrap();
        ReviewScope {
            project: project.id,
            root: path.into(),
        }
    }

    #[tokio::test]
    async fn review_round_trip_keeps_worktrees_and_projects_independent() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("review.db");
        let service = WorkspaceService::open(db.clone()).await.unwrap();
        let first = scope(&service, dir.path()).await;
        let other_root = dir.path().join("other");
        std::fs::create_dir(&other_root).unwrap();
        let other = scope(&service, &other_root).await;
        let worktree = ReviewScope {
            root: dir.path().join("worktree"),
            ..first.clone()
        };
        let value = ReviewPreferences {
            path: Some("src/caffè.rs".into()),
            staged: true,
            raw: true,
            commit_message: "Review 日本語\n\nKeep exact spacing  ".into(),
            ..Default::default()
        };
        let saved = service
            .save_review_preferences(first.clone(), 0, value)
            .await
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(
            service.review_preferences(other).await.unwrap(),
            ReviewPreferences::default()
        );
        assert_eq!(
            service.review_preferences(worktree).await.unwrap(),
            ReviewPreferences::default()
        );
        drop(service);
        let reopened = WorkspaceService::open(db).await.unwrap();
        assert_eq!(reopened.review_preferences(first).await.unwrap(), saved);
    }

    #[tokio::test]
    async fn stale_save_and_invalid_paths_do_not_overwrite_the_draft() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let scope = scope(&service, dir.path()).await;
        let saved = service
            .save_review_preferences(
                scope.clone(),
                0,
                ReviewPreferences {
                    commit_message: "Keep me".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(
            service
                .save_review_preferences(scope.clone(), 0, ReviewPreferences::default())
                .await
                .is_err()
        );
        for path in ["../outside", "/absolute", ""] {
            assert!(
                service
                    .save_review_preferences(
                        scope.clone(),
                        1,
                        ReviewPreferences {
                            path: Some(path.into()),
                            ..saved.clone()
                        }
                    )
                    .await
                    .is_err()
            );
        }
        assert!(
            service
                .save_review_preferences(
                    scope.clone(),
                    1,
                    ReviewPreferences {
                        commit_message: "x".repeat(MAX_COMMIT_DRAFT_BYTES + 1),
                        ..saved.clone()
                    }
                )
                .await
                .is_err()
        );
        assert_eq!(service.review_preferences(scope).await.unwrap(), saved);
    }

    #[tokio::test]
    async fn future_or_malformed_review_data_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let scope = scope(&service, dir.path()).await;
        for raw in ["{broken", "{\"version\":99}"] {
            let key = scope.key().unwrap();
            service
                .access(move |store| {
                    store
                        .connection
                        .execute(
                            "INSERT OR REPLACE INTO preferences(key,data) VALUES(?1,?2)",
                            params![key, raw],
                        )
                        .map_err(StorageError::from)?;
                    Ok(())
                })
                .await
                .unwrap();
            assert!(service.review_preferences(scope.clone()).await.is_err());
            assert!(
                service
                    .save_review_preferences(scope.clone(), 0, ReviewPreferences::default())
                    .await
                    .is_err()
            );
            let key = scope.key().unwrap();
            service
                .access(move |store| {
                    assert_eq!(store.preference_raw(&key)?.unwrap(), raw);
                    Ok(())
                })
                .await
                .unwrap();
        }
    }
}
