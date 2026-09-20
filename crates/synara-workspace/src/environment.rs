//! Non-secret Environment chrome. Restoring this data never starts a tool.
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const KEY: &str = "environment-layout";
const VERSION: u32 = 1;
const MAX_LAYOUT_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentTab {
    Terminal,
    Explorer,
    Changes,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentLayout {
    pub version: u32,
    pub open_by_default: bool,
    /// Desired fraction of the available workspace width, excluding the sidebar.
    /// Narrow-window constraints are applied by the UI without changing this value.
    pub width_ratio: f32,
    pub tabs: Vec<EnvironmentTab>,
    pub active: Option<EnvironmentTab>,
}
impl Default for EnvironmentLayout {
    fn default() -> Self {
        Self {
            version: VERSION,
            open_by_default: false,
            width_ratio: 0.5,
            tabs: Vec::new(),
            active: None,
        }
    }
}
impl EnvironmentLayout {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.version != VERSION
            || !self.width_ratio.is_finite()
            || !(0.2..=0.8).contains(&self.width_ratio)
            || self.tabs.len() > 3
            || self.tabs.iter().collect::<HashSet<_>>().len() != self.tabs.len()
            || self.active.is_some_and(|tab| !self.tabs.contains(&tab))
            || (self.active.is_none() && !self.tabs.is_empty())
        {
            return Err(WorkspaceError::Invalid("Invalid Environment layout".into()));
        }
        Ok(())
    }
    pub fn select(&mut self, tab: EnvironmentTab) {
        if !self.tabs.contains(&tab) {
            self.tabs.push(tab);
        }
        self.active = Some(tab);
    }
}

pub struct LoadedEnvironmentLayout {
    pub layout: EnvironmentLayout,
    /// Invalid data remains untouched until the user explicitly resets it.
    pub recovery: Option<String>,
}
impl WorkspaceService {
    pub async fn environment_layout(&self) -> WorkspaceResult<LoadedEnvironmentLayout> {
        self.access(|store| {
            let Some(raw) = store.preference_raw(KEY)? else {
                return Ok(LoadedEnvironmentLayout {
                    layout: EnvironmentLayout::default(),
                    recovery: None,
                });
            };
            let parsed = (raw.len() <= MAX_LAYOUT_BYTES)
                .then(|| serde_json::from_str::<EnvironmentLayout>(&raw).ok())
                .flatten()
                .filter(|layout| layout.validate().is_ok());
            Ok(match parsed {
                Some(layout) => LoadedEnvironmentLayout { layout, recovery: None },
                None => LoadedEnvironmentLayout {
                    layout: EnvironmentLayout::default(),
                    recovery: Some("The saved Environment layout is invalid or from a newer version. Reset it in General settings to save a new layout. The original value has been preserved.".into()),
                },
            })
        })
        .await
    }
    pub async fn save_environment_layout(&self, layout: EnvironmentLayout) -> WorkspaceResult<()> {
        layout.validate()?;
        self.access(move |store| {
            store.set_preference(KEY, &layout)?;
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_selection_is_stable_and_idempotent() {
        let mut layout = EnvironmentLayout::default();
        layout.select(EnvironmentTab::Explorer);
        layout.select(EnvironmentTab::Terminal);
        layout.select(EnvironmentTab::Explorer);
        assert_eq!(
            layout.tabs,
            vec![EnvironmentTab::Explorer, EnvironmentTab::Terminal]
        );
        assert_eq!(layout.active, Some(EnvironmentTab::Explorer));
        assert!(layout.validate().is_ok());
    }

    #[test]
    fn invalid_layouts_are_rejected_before_storage() {
        for ratio in [f32::NAN, f32::INFINITY, -1., 0., 0.19, 0.81, 1.] {
            let layout = EnvironmentLayout {
                width_ratio: ratio,
                ..Default::default()
            };
            assert!(layout.validate().is_err());
        }
        for layout in [
            EnvironmentLayout {
                version: 2,
                ..Default::default()
            },
            EnvironmentLayout {
                active: Some(EnvironmentTab::Explorer),
                ..Default::default()
            },
            EnvironmentLayout {
                tabs: vec![EnvironmentTab::Terminal],
                ..Default::default()
            },
            EnvironmentLayout {
                tabs: vec![EnvironmentTab::Terminal; 2],
                active: Some(EnvironmentTab::Terminal),
                ..Default::default()
            },
        ] {
            assert!(layout.validate().is_err());
        }
    }

    #[tokio::test]
    async fn layout_survives_reopen_without_tasks_sessions_or_settings_mutations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.sqlite3");
        let service = WorkspaceService::open(path.clone()).await.unwrap();
        let mut layout = EnvironmentLayout::default();
        layout.select(EnvironmentTab::Changes);
        layout.select(EnvironmentTab::Explorer);
        layout.width_ratio = 0.6;
        layout.open_by_default = true;
        service
            .save_environment_layout(layout.clone())
            .await
            .unwrap();
        let settings = service.settings().await.unwrap().settings;
        service.save_settings(settings.clone()).await.unwrap();
        drop(service);
        let service = WorkspaceService::open(path).await.unwrap();
        assert_eq!(service.environment_layout().await.unwrap().layout, layout);
        assert_eq!(service.settings().await.unwrap().settings, settings);
        assert!(service.catalog().await.unwrap().tasks.is_empty());
        let bad = EnvironmentLayout {
            width_ratio: 5.,
            ..layout.clone()
        };
        assert!(service.save_environment_layout(bad).await.is_err());
        assert_eq!(service.environment_layout().await.unwrap().layout, layout);
    }

    #[tokio::test]
    async fn malformed_future_and_oversized_values_are_preserved_on_read() {
        let service = WorkspaceService::memory().unwrap();
        for value in [
            serde_json::json!({"version": 2}),
            serde_json::json!({"version": 1, "width_ratio": "bad"}),
            serde_json::json!({"unknown": "x".repeat(MAX_LAYOUT_BYTES + 1)}),
        ] {
            let before = value.clone();
            service
                .access(move |store| {
                    store.set_preference(KEY, &value)?;
                    Ok(())
                })
                .await
                .unwrap();
            let loaded = service.environment_layout().await.unwrap();
            assert!(loaded.recovery.is_some());
            assert_eq!(loaded.layout, EnvironmentLayout::default());
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
