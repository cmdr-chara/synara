//! User-owned project organization. Spaces never move directories or change
//! task/agent ownership. Each explicit edit is applied to the latest transaction.
mod hubs;
pub(crate) use hubs::automation_hub_context;
#[cfg(test)]
mod hubs_tests;
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

const KEY: &str = "workspace-organization";
const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_SPACES: usize = 64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceSymbol {
    #[default]
    Folder,
    Star,
    Code,
    Globe,
}
impl SpaceSymbol {
    pub const ALL: [Self; 4] = [Self::Folder, Self::Star, Self::Code, Self::Globe];
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSpace {
    pub id: String,
    pub name: String,
    pub symbol: SpaceSymbol,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceOrganization {
    pub version: u32,
    pub revision: u64,
    pub active: Option<String>,
    pub spaces: Vec<NativeSpace>,
    pub project_spaces: BTreeMap<ProjectId, String>,
    pub pinned_projects: Vec<ProjectId>,
    pub pinned_threads: Vec<TaskId>,
}
impl Default for WorkspaceOrganization {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            active: None,
            spaces: Vec::new(),
            project_spaces: BTreeMap::new(),
            pinned_projects: Vec::new(),
            pinned_threads: Vec::new(),
        }
    }
}
#[derive(Clone, Debug)]
pub enum OrganizationEdit {
    Create {
        name: String,
        symbol: SpaceSymbol,
    },
    Rename {
        id: String,
        name: String,
        symbol: SpaceSymbol,
    },
    Delete {
        id: String,
    },
    Select(Option<String>),
    Move {
        id: String,
        backwards: bool,
    },
    Assign {
        project: ProjectId,
        space: Option<String>,
    },
    PinProject {
        project: ProjectId,
        pinned: bool,
    },
    PinThread {
        task: TaskId,
        pinned: bool,
    },
}
fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name == name.trim()
        && name.chars().count() <= 40
        && !name.chars().any(char::is_control)
        && !name.eq_ignore_ascii_case("Void")
}
fn unique<T: Eq + std::hash::Hash>(values: &[T]) -> bool {
    values.iter().collect::<HashSet<_>>().len() == values.len()
}
impl WorkspaceOrganization {
    pub fn space_for(&self, project: ProjectId) -> Option<&str> {
        self.project_spaces.get(&project).map(String::as_str)
    }
    pub fn contains_project(&self, project: ProjectId) -> bool {
        self.space_for(project) == self.active.as_deref()
    }
    pub fn active_name(&self) -> &str {
        self.active
            .as_ref()
            .and_then(|id| self.spaces.iter().find(|space| &space.id == id))
            .map_or("Void", |space| space.name.as_str())
    }
    fn validate(&self) -> StorageResult<()> {
        let ids: HashSet<_> = self.spaces.iter().map(|space| space.id.as_str()).collect();
        let names: HashSet<_> = self
            .spaces
            .iter()
            .map(|space| space.name.to_lowercase())
            .collect();
        if self.version != 1
            || self.spaces.len() > MAX_SPACES
            || self.project_spaces.len() > 10_000
            || self.pinned_projects.len() > 256
            || self.pinned_threads.len() > 512
            || ids.len() != self.spaces.len()
            || names.len() != self.spaces.len()
            || self
                .spaces
                .iter()
                .any(|space| uuid::Uuid::parse_str(&space.id).is_err() || !valid_name(&space.name))
            || self
                .active
                .as_ref()
                .is_some_and(|id| !ids.contains(id.as_str()))
            || self
                .project_spaces
                .values()
                .any(|id| !ids.contains(id.as_str()))
            || !unique(&self.pinned_projects)
            || !unique(&self.pinned_threads)
        {
            return Err(StorageError::Identity);
        }
        Ok(())
    }
    fn require_space(&self, id: &str) -> WorkspaceResult<usize> {
        self.spaces
            .iter()
            .position(|space| space.id == id)
            .ok_or(WorkspaceError::NotFound)
    }
    fn name_available(&self, name: &str, except: Option<&str>) -> WorkspaceResult<()> {
        if !valid_name(name) {
            return Err(WorkspaceError::Invalid("Use a nonempty Space name of up to 40 characters. Void is reserved for unassigned projects.".into()));
        }
        if self.spaces.iter().any(|space| {
            Some(space.id.as_str()) != except && space.name.to_lowercase() == name.to_lowercase()
        }) {
            return Err(WorkspaceError::Invalid(
                "A Space with that name already exists.".into(),
            ));
        }
        Ok(())
    }
    fn apply(&mut self, edit: OrganizationEdit) -> WorkspaceResult<()> {
        match edit {
            OrganizationEdit::Create { name, symbol } => {
                let name = name.trim().to_owned();
                self.name_available(&name, None)?;
                if self.spaces.len() >= MAX_SPACES {
                    return Err(StorageError::Limit.into());
                }
                let id = uuid::Uuid::new_v4().to_string();
                self.active = Some(id.clone());
                self.spaces.push(NativeSpace { id, name, symbol });
            }
            OrganizationEdit::Rename { id, name, symbol } => {
                let index = self.require_space(&id)?;
                let name = name.trim().to_owned();
                self.name_available(&name, Some(&id))?;
                self.spaces[index].name = name;
                self.spaces[index].symbol = symbol;
            }
            OrganizationEdit::Delete { id } => {
                let index = self.require_space(&id)?;
                self.spaces.remove(index);
                self.project_spaces.retain(|_, space| space != &id);
                if self.active.as_ref() == Some(&id) {
                    self.active = None;
                }
            }
            OrganizationEdit::Select(id) => {
                if let Some(id) = &id {
                    self.require_space(id)?;
                }
                self.active = id;
            }
            OrganizationEdit::Move { id, backwards } => {
                let index = self.require_space(&id)?;
                let next = if backwards {
                    index.checked_sub(1)
                } else {
                    index
                        .checked_add(1)
                        .filter(|next| *next < self.spaces.len())
                };
                if let Some(next) = next {
                    self.spaces.swap(index, next);
                }
            }
            OrganizationEdit::Assign { project, space } => {
                if let Some(space) = space {
                    self.require_space(&space)?;
                    self.project_spaces.insert(project, space);
                } else {
                    self.project_spaces.remove(&project);
                }
            }
            OrganizationEdit::PinProject { project, pinned } => {
                if pinned && !self.pinned_projects.contains(&project) {
                    self.pinned_projects.push(project);
                }
                if !pinned {
                    self.pinned_projects.retain(|value| *value != project);
                }
            }
            OrganizationEdit::PinThread { task, pinned } => {
                if pinned && !self.pinned_threads.contains(&task) {
                    self.pinned_threads.push(task);
                }
                if !pinned {
                    self.pinned_threads.retain(|value| *value != task);
                }
            }
        }
        self.validate()?;
        Ok(())
    }
}
fn read_organization(connection: &Connection) -> StorageResult<WorkspaceOrganization> {
    let raw: Option<String> = connection
        .query_row("SELECT data FROM preferences WHERE key=?1", [KEY], |row| {
            row.get(0)
        })
        .optional()?;
    let organization = match raw {
        Some(raw) if raw.len() > MAX_BYTES => return Err(StorageError::Limit),
        Some(raw) => decode::<WorkspaceOrganization>(&raw)?,
        None => WorkspaceOrganization::default(),
    };
    organization.validate()?;
    Ok(organization)
}
impl WorkspaceService {
    pub async fn organization(&self) -> WorkspaceResult<WorkspaceOrganization> {
        self.access(|store| Ok(read_organization(&store.connection)?))
            .await
    }
    pub async fn edit_organization(
        &self,
        edit: OrganizationEdit,
    ) -> WorkspaceResult<WorkspaceOrganization> {
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(StorageError::from)?;
            let mut organization = read_organization(&tx)?;
            match &edit {
                OrganizationEdit::Assign { project, .. } | OrganizationEdit::PinProject { project, .. } => {
                    let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)", [project.to_string()], |row| row.get(0)).map_err(StorageError::from)?;
                    if !exists { return Err(WorkspaceError::NotFound); }
                }
                OrganizationEdit::PinThread { task, .. } => {
                    let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)", [task.to_string()], |row| row.get(0)).map_err(StorageError::from)?;
                    if !exists { return Err(WorkspaceError::NotFound); }
                }
                _ => {}
            }
            let before = organization.clone();
            organization.apply(edit)?;
            if before != organization {
                organization.revision = organization.revision.checked_add(1).ok_or(StorageError::Limit)?;
                let data = encode(&organization)?;
                if data.len() > MAX_BYTES { return Err(StorageError::Limit.into()); }
                tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![KEY, data]).map_err(StorageError::from)?;
            }
            tx.commit().map_err(StorageError::from)?;
            Ok(organization)
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn spaces_assign_reorder_rename_and_delete_without_deleting_projects_or_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_task(project.id, "Keep this task".into(), agent)
            .await
            .unwrap();
        let first = service
            .edit_organization(OrganizationEdit::Create {
                name: "Work".into(),
                symbol: SpaceSymbol::Code,
            })
            .await
            .unwrap()
            .active
            .unwrap();
        let second = service
            .edit_organization(OrganizationEdit::Create {
                name: "Personal".into(),
                symbol: SpaceSymbol::Star,
            })
            .await
            .unwrap()
            .active
            .unwrap();
        service
            .edit_organization(OrganizationEdit::Assign {
                project: project.id,
                space: Some(first.clone()),
            })
            .await
            .unwrap();
        let moved = service
            .edit_organization(OrganizationEdit::Move {
                id: second.clone(),
                backwards: true,
            })
            .await
            .unwrap();
        assert_eq!(moved.spaces[0].id, second);
        service
            .edit_organization(OrganizationEdit::Rename {
                id: first.clone(),
                name: "制作".into(),
                symbol: SpaceSymbol::Globe,
            })
            .await
            .unwrap();
        let selected = service
            .edit_organization(OrganizationEdit::Select(Some(first.clone())))
            .await
            .unwrap();
        assert!(selected.contains_project(project.id));
        assert_eq!(selected.active_name(), "制作");
        let removed = service
            .edit_organization(OrganizationEdit::Delete { id: first })
            .await
            .unwrap();
        assert_eq!(removed.active, None);
        assert_eq!(removed.space_for(project.id), None);
        assert!(service.task(task.id).await.is_ok());
        assert_eq!(service.catalog().await.unwrap().projects.len(), 1);
        assert!(
            service
                .thread(task.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
        assert!(service.session(task.thread_id).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn organization_reopens_and_concurrent_explicit_edits_do_not_lose_updates() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("state.db");
        let a = WorkspaceService::open(database.clone()).await.unwrap();
        let b = WorkspaceService::open(database.clone()).await.unwrap();
        let (x, y) = tokio::join!(
            a.edit_organization(OrganizationEdit::Create {
                name: "A".into(),
                symbol: SpaceSymbol::Folder
            }),
            b.edit_organization(OrganizationEdit::Create {
                name: "B".into(),
                symbol: SpaceSymbol::Star
            })
        );
        x.unwrap();
        y.unwrap();
        let expected = a.organization().await.unwrap();
        assert_eq!(expected.spaces.len(), 2);
        drop(a);
        drop(b);
        assert_eq!(
            WorkspaceService::open(database)
                .await
                .unwrap()
                .organization()
                .await
                .unwrap(),
            expected
        );
    }
    #[tokio::test]
    async fn malformed_or_future_organization_is_never_silently_replaced() {
        let service = WorkspaceService::memory().unwrap();
        service
            .access(|store| {
                store
                    .connection
                    .execute(
                        "INSERT INTO preferences(key,data) VALUES(?1,?2)",
                        params![KEY, "{\"version\":99}"],
                    )
                    .map_err(StorageError::from)?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(service.organization().await.is_err());
        assert!(
            service
                .edit_organization(OrganizationEdit::Create {
                    name: "New".into(),
                    symbol: SpaceSymbol::Folder
                })
                .await
                .is_err()
        );
        service
            .access(|store| {
                assert_eq!(store.preference_raw(KEY)?.unwrap(), "{\"version\":99}");
                Ok(())
            })
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn explicit_pins_are_idempotent_bounded_and_unknown_destinations_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_task(project.id, "Pin".into(), agent)
            .await
            .unwrap();
        service
            .edit_organization(OrganizationEdit::PinProject {
                project: project.id,
                pinned: true,
            })
            .await
            .unwrap();
        let a = service
            .edit_organization(OrganizationEdit::PinThread {
                task: task.id,
                pinned: true,
            })
            .await
            .unwrap();
        let b = service
            .edit_organization(OrganizationEdit::PinThread {
                task: task.id,
                pinned: true,
            })
            .await
            .unwrap();
        assert_eq!(a, b);
        assert!(
            service
                .edit_organization(OrganizationEdit::Assign {
                    project: project.id,
                    space: Some(uuid::Uuid::new_v4().to_string())
                })
                .await
                .is_err()
        );
        assert!(
            service
                .edit_organization(OrganizationEdit::PinThread {
                    task: TaskId::new(),
                    pinned: true
                })
                .await
                .is_err()
        );
        for name in ["", "Void", "bad\nname", &"x".repeat(41)] {
            assert!(
                service
                    .edit_organization(OrganizationEdit::Create {
                        name: name.into(),
                        symbol: SpaceSymbol::Folder
                    })
                    .await
                    .is_err()
            );
        }
        assert_eq!(service.organization().await.unwrap(), b);
    }
}
