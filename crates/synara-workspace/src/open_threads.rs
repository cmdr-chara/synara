//! Persisted identity-only state for the native conversation tab strip.
//!
//! A native draft belongs to a task, so its `TaskId` is the stable identity for
//! both an existing conversation and its unsent draft.  Titles, provider glyphs,
//! transcript state, and draft text deliberately stay out of this preference;
//! the app derives those from the current catalog and its existing draft owner.
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use synara_core::{TaskId, TaskState};

const KEY: &str = "open-thread-tabs";
pub const OPEN_THREAD_TABS_VERSION: u32 = 1;
pub const MAX_OPEN_THREAD_TABS: usize = 32;
const MAX_OPEN_THREAD_TABS_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenThreadTabs {
    pub version: u32,
    /// Stable task identities in the user's tab order. A task also owns its
    /// persisted draft, so no title, glyph, or draft text is stored here.
    pub task_ids: Vec<TaskId>,
}

impl Default for OpenThreadTabs {
    fn default() -> Self {
        Self {
            version: OPEN_THREAD_TABS_VERSION,
            task_ids: Vec::new(),
        }
    }
}

impl OpenThreadTabs {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.version != OPEN_THREAD_TABS_VERSION
            || self.task_ids.len() > MAX_OPEN_THREAD_TABS
            || self.task_ids.iter().collect::<HashSet<_>>().len() != self.task_ids.len()
        {
            return Err(WorkspaceError::Invalid(
                "Invalid open conversation tabs".into(),
            ));
        }
        Ok(())
    }

    /// Open a task once, retaining its existing position. When the bounded
    /// list is full, evict the oldest task other than `protected`; the caller
    /// passes the currently visible task so switching cannot strand it.
    pub fn open(&mut self, task: TaskId, protected: Option<TaskId>) -> Option<TaskId> {
        if self.task_ids.contains(&task) {
            return None;
        }
        let evicted = if self.task_ids.len() >= MAX_OPEN_THREAD_TABS {
            let index = self
                .task_ids
                .iter()
                .position(|candidate| Some(*candidate) != protected)
                .unwrap_or(0);
            Some(self.task_ids.remove(index))
        } else {
            None
        };
        self.task_ids.push(task);
        evicted
    }

    pub fn close(&mut self, task: TaskId) -> bool {
        let Some(index) = self
            .task_ids
            .iter()
            .position(|candidate| *candidate == task)
        else {
            return false;
        };
        self.task_ids.remove(index);
        true
    }

    pub fn contains(&self, task: TaskId) -> bool {
        self.task_ids.contains(&task)
    }

    /// Remove identities that no longer have a live task. This is called on
    /// load and catalog replacement, never from render, so stale persisted
    /// tabs cannot become navigation targets.
    pub fn prune(&mut self, is_live: impl Fn(TaskId) -> bool) -> bool {
        let before = self.task_ids.len();
        self.task_ids.retain(|task| is_live(*task));
        before != self.task_ids.len()
    }

    /// Select the tab that slides into the closed tab's slot: the next tab,
    /// or the previous tab when the closed tab was last.
    pub fn successor_after_close(&self, task: TaskId) -> Option<TaskId> {
        let index = self
            .task_ids
            .iter()
            .position(|candidate| *candidate == task)?;
        let mut remaining = self.task_ids.iter().copied();
        remaining.nth(index + 1).or_else(|| {
            (index > 0)
                .then(|| self.task_ids.get(index - 1).copied())
                .flatten()
        })
    }
}

pub struct LoadedOpenThreadTabs {
    pub tabs: OpenThreadTabs,
    /// Invalid/future data remains untouched until an explicit future schema
    /// migration or reset. Normal stale task ids are pruned on read.
    pub recovery: Option<String>,
}

impl WorkspaceService {
    pub async fn open_thread_tabs(&self) -> WorkspaceResult<LoadedOpenThreadTabs> {
        self.access(|store| {
            let Some(raw) = store.preference_raw(KEY)? else {
                return Ok(LoadedOpenThreadTabs {
                    tabs: OpenThreadTabs::default(),
                    recovery: None,
                });
            };
            let parsed = (raw.len() <= MAX_OPEN_THREAD_TABS_BYTES)
                .then(|| serde_json::from_str::<OpenThreadTabs>(&raw).ok())
                .flatten()
                .filter(|tabs| tabs.validate().is_ok());
            let Some(mut tabs) = parsed else {
                return Ok(LoadedOpenThreadTabs {
                    tabs: OpenThreadTabs::default(),
                    recovery: Some(
                        "The saved conversation tabs are invalid or from a newer version. The original value was preserved.".into(),
                    ),
                });
            };
            let catalog = crate::service::catalog(store)?;
            tabs.prune(|task_id| {
                catalog.tasks.iter().any(|task| {
                    task.id == task_id && task.state != TaskState::Archived
                })
            });
            Ok(LoadedOpenThreadTabs {
                tabs,
                recovery: None,
            })
        })
        .await
    }

    pub async fn save_open_thread_tabs(&self, tabs: OpenThreadTabs) -> WorkspaceResult<()> {
        tabs.validate()?;
        self.access(move |store| {
            store.set_preference(KEY, &tabs)?;
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_ids_are_identity_only_and_close_selects_a_predictable_successor() {
        let first = TaskId::new();
        let second = TaskId::new();
        let third = TaskId::new();
        let mut tabs = OpenThreadTabs::default();
        tabs.open(first, None);
        tabs.open(second, Some(first));
        tabs.open(third, Some(second));
        assert_eq!(tabs.task_ids, vec![first, second, third]);
        assert_eq!(tabs.successor_after_close(second), Some(third));
        assert_eq!(tabs.successor_after_close(third), Some(second));
        assert_eq!(tabs.successor_after_close(first), Some(second));
        assert!(tabs.close(second));
        assert_eq!(tabs.task_ids, vec![first, third]);
        assert!(tabs.validate().is_ok());
    }

    #[test]
    fn opening_is_idempotent_and_bounded_without_evicting_the_visible_task() {
        let protected = TaskId::new();
        let mut tabs = OpenThreadTabs::default();
        tabs.open(protected, None);
        for _ in 1..MAX_OPEN_THREAD_TABS {
            tabs.open(TaskId::new(), Some(protected));
        }
        assert_eq!(tabs.task_ids.len(), MAX_OPEN_THREAD_TABS);
        assert_eq!(tabs.open(protected, Some(protected)), None);
        let replacement = TaskId::new();
        let evicted = tabs.open(replacement, Some(protected));
        assert!(evicted.is_some());
        assert!(tabs.contains(protected));
        assert!(tabs.contains(replacement));
        assert_eq!(tabs.task_ids.len(), MAX_OPEN_THREAD_TABS);
        assert!(tabs.validate().is_ok());
    }

    #[test]
    fn stale_identity_pruning_never_reorders_live_tabs() {
        let first = TaskId::new();
        let stale = TaskId::new();
        let last = TaskId::new();
        let mut tabs = OpenThreadTabs {
            version: OPEN_THREAD_TABS_VERSION,
            task_ids: vec![first, stale, last],
        };
        assert!(tabs.prune(|task| task != stale));
        assert_eq!(tabs.task_ids, vec![first, last]);
        assert!(!tabs.prune(|_| true));
    }

    #[tokio::test]
    async fn valid_tabs_round_trip_and_stale_tasks_are_dropped() {
        let service = WorkspaceService::memory().unwrap();
        let root = tempfile::tempdir().unwrap();
        let task = service
            .add_local_workspace(root.path().to_owned())
            .await
            .expect("test workspace");
        let agent = service.profiles().await.unwrap().remove(0).id;
        let live = service
            .create_task(task.id, "Live".into(), agent)
            .await
            .unwrap();
        let stale = TaskId::new();
        let tabs = OpenThreadTabs {
            version: OPEN_THREAD_TABS_VERSION,
            task_ids: vec![live.id, stale],
        };
        // Saving accepts only the bounded identity model; loading applies the
        // catalog liveness check without changing unrelated settings.
        service.save_open_thread_tabs(tabs).await.unwrap();
        let loaded = service.open_thread_tabs().await.unwrap();
        assert_eq!(loaded.tabs.task_ids, vec![live.id]);
        assert!(loaded.recovery.is_none());
    }

    #[tokio::test]
    async fn future_or_malformed_values_are_recovered_without_overwriting_the_preference() {
        let service = WorkspaceService::memory().unwrap();
        for value in [
            serde_json::json!({"version": 2}),
            serde_json::json!({"version": 1, "task_ids": "bad"}),
        ] {
            let before = value.clone();
            service
                .access(move |store| {
                    store.set_preference(KEY, &value)?;
                    Ok(())
                })
                .await
                .unwrap();
            let loaded = service.open_thread_tabs().await.unwrap();
            assert!(loaded.recovery.is_some());
            assert_eq!(loaded.tabs, OpenThreadTabs::default());
            service
                .access(move |store| {
                    assert_eq!(store.preference::<serde_json::Value>(KEY)?, Some(before));
                    Ok(())
                })
                .await
                .unwrap();
        }
    }
}
