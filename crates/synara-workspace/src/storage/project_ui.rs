//! Per-project renderer preference: a local display name plus the appearance
//! chosen in Edit project. Upstream parity: `projectNamesByCwd` /
//! `projectAppearanceByCwd` are renderer-local maps keyed by the project's
//! directory, validated on read, and a reset writes nothing. This store keeps
//! the same two values in one `project-ui:<id>` record keyed by project id.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUi {
    /// Local display-name override; `None` shows the project's own name.
    /// Upstream stores an empty string as "no override".
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub appearance: Option<ProjectAppearance>,
}

fn key(project: ProjectId) -> String {
    format!("project-ui:{project}")
}

fn normalize_ui(mut ui: ProjectUi) -> ProjectUi {
    ui.name = ui
        .name
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    ui.appearance = parse_project_appearance(ui.appearance);
    ui
}

/// A missing or malformed record reads as the default project look, the same
/// fallback upstream's validated-on-read maps take.
fn decode_ui(raw: Option<String>) -> ProjectUi {
    raw.and_then(|raw| {
        if raw.len() <= 128 * 1024 {
            decode(&raw).ok()
        } else {
            None
        }
    })
    .map(normalize_ui)
    .unwrap_or_default()
}

fn read(connection: &Connection, project: ProjectId) -> WorkspaceResult<ProjectUi> {
    let owner: Option<String> = connection
        .query_row(
            "SELECT data FROM projects WHERE id=?1",
            [project.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    let owner: Project = decode(&owner.ok_or(WorkspaceError::NotFound)?)?;
    if owner.id != project {
        return Err(StorageError::Identity.into());
    }
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [key(project)],
            |r| r.get(0),
        )
        .optional()?;
    Ok(decode_ui(raw))
}

fn read_all(connection: &Connection) -> WorkspaceResult<HashMap<ProjectId, ProjectUi>> {
    let mut statement =
        connection.prepare("SELECT key, data FROM preferences WHERE key LIKE 'project-ui:%'")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut result = HashMap::new();
    for (key, data) in rows {
        let Some(id) = key.strip_prefix("project-ui:") else {
            continue;
        };
        // Unknown or malformed keys stay unreadable, not fatal — upstream
        // prunes invalid persisted appearances the same way.
        let Ok(project) = serde_json::from_value::<ProjectId>(serde_json::Value::String(id.into()))
        else {
            continue;
        };
        let ui = decode_ui(Some(data));
        if ui != ProjectUi::default() {
            result.insert(project, ui);
        }
    }
    Ok(result)
}

impl WorkspaceService {
    pub async fn project_ui(&self, project: ProjectId) -> WorkspaceResult<ProjectUi> {
        self.access(move |store| read(&store.connection, project))
            .await
    }
    /// Every project's local name/appearance in one read for the sidebar.
    pub async fn project_uis(&self) -> WorkspaceResult<HashMap<ProjectId, ProjectUi>> {
        self.access(move |store| read_all(&store.connection)).await
    }
    /// Upstream `setProjectNameLocally` + `setProjectAppearanceLocally` in one
    /// write: `None`/`None` clears the record entirely (reset leaves nothing
    /// behind, so the next read falls back to the default folder).
    pub async fn set_project_ui(
        &self,
        project: ProjectId,
        name: Option<String>,
        appearance: Option<ProjectAppearance>,
    ) -> WorkspaceResult<()> {
        self.access(move |store| {
            if store
                .connection
                .query_row(
                    "SELECT 1 FROM projects WHERE id=?1",
                    [project.to_string()],
                    |_| Ok(()),
                )
                .optional()?
                .is_none()
            {
                return Err(WorkspaceError::NotFound);
            }
            let ui = normalize_ui(ProjectUi { name, appearance });
            if ui == ProjectUi::default() {
                store.delete_preference(&key(project))
            } else {
                store.set_preference(&key(project), &ui)
            }
            .map_err(Into::into)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Store;

    fn store_with_project() -> (Store, Workspace, Project) {
        let mut storage = Store::memory().unwrap();
        let workspace = Workspace {
            id: WorkspaceId::new(),
            name: "w".into(),
            location: WorkspaceLocation::Local {
                root: "/tmp/w".into(),
            },
        };
        let project = Project {
            id: ProjectId::new(),
            workspace_id: workspace.id,
            name: "app".into(),
            relative_directory: "app".into(),
        };
        storage
            .create_workspace_project(&workspace, &project)
            .unwrap();
        (storage, workspace, project)
    }

    #[test]
    fn absent_or_malformed_reads_as_default_and_reset_deletes() {
        let (storage, _workspace, project) = store_with_project();
        assert_eq!(
            read(&storage.connection, project.id).unwrap(),
            ProjectUi::default()
        );
        storage
            .set_preference(&key(project.id), &serde_json::json!({"not": "a ui"}))
            .unwrap();
        assert_eq!(
            read(&storage.connection, project.id).unwrap(),
            ProjectUi::default()
        );
        storage
            .set_preference(
                &key(project.id),
                &ProjectUi {
                    name: Some("  Mine  ".into()),
                    appearance: Some(ProjectAppearance::Icon {
                        icon: "rocket".into(),
                        color: Some(ProjectColorName("blue".into())),
                    }),
                },
            )
            .unwrap();
        let ui = read(&storage.connection, project.id).unwrap();
        assert_eq!(ui.name.as_deref(), Some("Mine"));
        assert_eq!(
            ui.appearance,
            Some(ProjectAppearance::Icon {
                icon: "rocket".into(),
                color: Some(ProjectColorName("blue".into()))
            })
        );
        storage.delete_preference(&key(project.id)).unwrap();
        assert_eq!(
            read(&storage.connection, project.id).unwrap(),
            ProjectUi::default()
        );
    }

    #[test]
    fn read_all_skips_malformed_keys_and_empty_records() {
        let (storage, _workspace, project) = store_with_project();
        storage
            .set_preference(&key(project.id), &ProjectUi::default())
            .unwrap();
        storage
            .connection
            .execute(
                "INSERT INTO preferences(key,data) VALUES('project-ui:not-a-uuid','{}')",
                [],
            )
            .unwrap();
        let all = read_all(&storage.connection).unwrap();
        assert!(all.is_empty());
        storage
            .set_preference(
                &key(project.id),
                &ProjectUi {
                    name: Some("Mine".into()),
                    appearance: None,
                },
            )
            .unwrap();
        assert_eq!(
            read_all(&storage.connection).unwrap()[&project.id]
                .name
                .as_deref(),
            Some("Mine")
        );
    }
}
