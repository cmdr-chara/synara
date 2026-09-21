//! Hub metadata shares the storage worker, not the Space record. Keeping each Hub
//! in its own bounded record avoids rewriting unrelated context during edits.
use super::super::*;
use crate::{HubProfile, HubSummary, WorkspaceService, WorkspaceResult, WorkspaceError, AgentProfile, default_profiles, now_ms};
use std::path::PathBuf;

const MAX_HUB_BYTES: usize = 128 * 1024;
fn key(project: ProjectId) -> String { format!("hub:{project}") }
fn sql(error: rusqlite::Error) -> WorkspaceError { StorageError::from(error).into() }
fn read(connection: &Connection, project: ProjectId) -> WorkspaceResult<Option<HubProfile>> {
    let raw: Option<String> = connection.query_row("SELECT data FROM preferences WHERE key=?1", [key(project)], |row| row.get(0)).optional().map_err(sql)?;
    let Some(raw) = raw else { return Ok(None) };
    if raw.len() > MAX_HUB_BYTES { return Err(StorageError::Limit.into()); }
    let profile: HubProfile = decode(&raw)?;
    profile.validate()?;
    if profile.project != project { return Err(StorageError::Identity.into()); }
    Ok(Some(profile))
}
fn tasks(connection: &Connection, project: ProjectId) -> WorkspaceResult<Vec<Task>> {
    let mut query = connection.prepare("SELECT data FROM tasks WHERE project_id=?1 ORDER BY id").map_err(sql)?;
    let mut rows = query.query([project.to_string()]).map_err(sql)?;
    let mut tasks = Vec::new();
    let mut visited = 0;
    while let Some(row) = rows.next().map_err(sql)? {
        visited += 1;
        if visited > 10_000 { return Err(StorageError::Limit.into()); }
        let task: Task = decode(&row.get::<_, String>(0).map_err(sql)?)?;
        if task.project_id != project { return Err(StorageError::Identity.into()); }
        if task.scope == TaskScope::Studio { tasks.push(task); }
    }
    Ok(tasks)
}
fn resolved(connection: &Connection, project: &Project) -> WorkspaceResult<Option<HubSummary>> {
    let tasks = tasks(connection, project.id)?;
    let saved = read(connection, project.id)?;
    let imported = saved.is_none();
    let profile = if let Some(profile) = saved { profile } else {
        let Some(first) = tasks.first() else { return Ok(None) };
        let name: String = if uuid::Uuid::parse_str(&project.name).is_ok() {
            if first.title.starts_with("New ") { "Imported work".into() } else { first.title.chars().take(40).collect() }
        } else { project.name.chars().take(40).collect() };
        let name: String = name.chars().filter(|ch| !ch.is_control()).collect();
        HubProfile::new(project.id, first.id, if name.trim().is_empty() { "Imported work".into() } else { name })
    };
    Ok(Some(HubSummary { profile, imported,
        threads: tasks.iter().filter(|task| task.state != TaskState::Archived).count() }))
}
fn project(connection: &Connection, id: ProjectId) -> WorkspaceResult<Project> {
    let raw: String = connection.query_row("SELECT data FROM projects WHERE id=?1", [id.to_string()], |row| row.get(0)).optional().map_err(sql)?.ok_or(WorkspaceError::NotFound)?;
    let value: Project = decode(&raw)?;
    if value.id != id { return Err(StorageError::Identity.into()); }
    Ok(value)
}
impl WorkspaceService {
    /// Legacy Studio threads appear immediately, with no schema/task rewrite.
    /// Invalid or future metadata is surfaced instead of replaced with defaults.
    pub async fn hubs(&self) -> WorkspaceResult<Vec<HubSummary>> {
        self.access(|store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Deferred).map_err(sql)?;
            let projects: Vec<Project> = {
                let mut query = tx.prepare("SELECT data FROM projects ORDER BY id").map_err(sql)?;
                let rows = query.query_map([], |row| row.get::<_, String>(0)).map_err(sql)?;
                let mut result = Vec::new();
                for row in rows {
                    if result.len() >= 10_000 { return Err(StorageError::Limit.into()); }
                    result.push(decode(&row.map_err(sql)?)?);
                }
                result
            };
            let mut result = Vec::new();
            for project in projects {
                if let Some(hub) = resolved(&tx, &project)? { result.push(hub); }
            }
            tx.commit().map_err(sql)?;
            result.sort_by(|a,b| a.profile.name.to_lowercase().cmp(&b.profile.name.to_lowercase()).then_with(|| a.profile.project.cmp(&b.profile.project)));
            Ok(result)
        }).await
    }
    pub async fn save_hub(&self, expected: u64, mut value: HubProfile) -> WorkspaceResult<HubProfile> {
        value.validate()?;
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
            let project = project(&tx, value.project)?;
            let current = resolved(&tx, &project)?.ok_or(WorkspaceError::NotFound)?.profile;
            if current.revision != expected || value.revision != expected || value.main_task != current.main_task {
                return Err(WorkspaceError::Invalid("Hub context changed elsewhere. Copy your edits, then reload before saving.".into()));
            }
            value.revision = expected.checked_add(1).ok_or(StorageError::Limit)?;
            let data = encode(&value)?;
            if data.len() > MAX_HUB_BYTES { return Err(StorageError::Limit.into()); }
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![key(value.project),data]).map_err(sql)?;
            tx.commit().map_err(sql)?;
            Ok(value)
        }).await
    }
    /// Folder registration is idempotent. Hub + Main thread + empty draft commit
    /// together, and no provider starts as a side effect of creation.
    pub async fn create_hub(&self, root: PathBuf, name: String, agent: String) -> WorkspaceResult<(HubProfile, Task)> {
        let registered = self.add_local_workspace(root).await?;
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
            let project = project(&tx, registered.id)?;
            if resolved(&tx, &project)?.is_some() {
                return Err(WorkspaceError::Invalid("This folder already has a Hub. Open the existing Hub instead.".into()));
            }
            let profiles: Option<String> = tx.query_row("SELECT data FROM preferences WHERE key='agent_profiles'", [], |row| row.get(0)).optional().map_err(sql)?;
            let profiles: Vec<AgentProfile> = profiles.map(|text| decode(&text)).transpose()?.unwrap_or_else(default_profiles);
            if !profiles.iter().any(|profile| profile.id == agent) { return Err(WorkspaceError::Invalid("Unknown agent profile.".into())); }
            let raw: String = tx.query_row("SELECT data FROM workspaces WHERE id=?1", [project.workspace_id.to_string()], |row| row.get(0)).map_err(sql)?;
            let workspace: Workspace = decode(&raw)?;
            let WorkspaceLocation::Local { root } = workspace.location else { return Err(WorkspaceError::Invalid("Choose a local folder for a new Hub.".into())) };
            if workspace.id != project.workspace_id || !project.relative_directory.as_os_str().is_empty() { return Err(StorageError::Identity.into()); }
            let task = Task { id: TaskId::new(), project_id: project.id, title: "Main".into(), state: TaskState::Ready,
                thread_id: ThreadId::new(), agent_id: agent, working_directory: root, updated_at_ms: now_ms(), scope: TaskScope::Studio };
            let profile = HubProfile::new(project.id, task.id, name.trim().into()); profile.validate()?;
            tx.execute("INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5)",
                params![task.id.to_string(),task.project_id.to_string(),task.thread_id.to_string(),task.updated_at_ms,encode(&task)?]).map_err(sql)?;
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2)", params![format!("task-draft:{}",task.id),encode(&serde_json::json!({"version":1,"text":""}))?]).map_err(sql)?;
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2)", params![key(project.id),encode(&profile)?]).map_err(sql)?;
            tx.commit().map_err(sql)?;
            Ok((profile,task))
        }).await
    }
    pub async fn create_hub_thread(&self, project: ProjectId, title: String, agent: String) -> WorkspaceResult<Task> {
        self.insert_hub_task(project,title,agent,None).await
    }
    /// Dedicated task composers already contain the user's reviewed prompt.
    /// Do not prepend hidden Hub context to Create-and-run requests.
    pub async fn create_hub_task(&self, project: ProjectId, title: String, agent: String, draft: String) -> WorkspaceResult<Task> {
        self.insert_hub_task(project,title,agent,Some(draft)).await
    }
    async fn insert_hub_task(&self, id: ProjectId, title: String, agent: String, draft: Option<String>) -> WorkspaceResult<Task> {
        if title.trim().is_empty() || title.len() > 400 || title.contains('\0')
            || draft.as_ref().is_some_and(|draft|draft.len() > 1024 * 1024) {
            return Err(WorkspaceError::Invalid("Invalid Hub task title or draft size.".into()));
        }
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
            let project = project(&tx,id)?;
            let hub = resolved(&tx,&project)?.ok_or(WorkspaceError::NotFound)?.profile;
            if hub.archived { return Err(WorkspaceError::Invalid("Restore this Hub before starting a thread.".into())); }
            let profiles: Option<String> = tx.query_row("SELECT data FROM preferences WHERE key='agent_profiles'",[],|row|row.get(0)).optional().map_err(sql)?;
            let profiles: Vec<AgentProfile> = profiles.map(|text|decode(&text)).transpose()?.unwrap_or_else(default_profiles);
            if !profiles.iter().any(|profile|profile.id == agent) { return Err(WorkspaceError::Invalid("Unknown agent profile.".into())); }
            let raw: String = tx.query_row("SELECT data FROM workspaces WHERE id=?1",[project.workspace_id.to_string()],|row|row.get(0)).map_err(sql)?;
            let workspace: Workspace = decode(&raw)?;
            if workspace.id != project.workspace_id || project.relative_directory.components().any(|part|
                !matches!(part,std::path::Component::Normal(_) | std::path::Component::CurDir)) {
                return Err(StorageError::Identity.into());
            }
            let working_directory = match workspace.location {
                WorkspaceLocation::Local { root } => {
                    let fs = synara_runtime::WorkspaceFs::open(&root)?;
                    if !project.relative_directory.as_os_str().is_empty() { fs.entries(&project.relative_directory)?; }
                    fs.root().join(&project.relative_directory)
                }
                WorkspaceLocation::Ssh { root, .. } => {
                    if !root.starts_with('/') || root.contains('\0') { return Err(StorageError::Identity.into()); }
                    PathBuf::from(root).join(&project.relative_directory)
                }
            };
            let text = draft.unwrap_or_else(|| if hub.include_in_new_threads { hub.context_draft() } else { String::new() });
            let task = Task { id: TaskId::new(), project_id: id, title: title.trim().into(), state: TaskState::Ready,
                thread_id: ThreadId::new(), agent_id: agent, working_directory, updated_at_ms: now_ms(), scope: TaskScope::Studio };
            tx.execute("INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5)",
                params![task.id.to_string(),id.to_string(),task.thread_id.to_string(),task.updated_at_ms,encode(&task)?]).map_err(sql)?;
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2)",
                params![format!("task-draft:{}",task.id),encode(&serde_json::json!({"version":1,"text":text}))?]).map_err(sql)?;
            tx.commit().map_err(sql)?;
            Ok(task)
        }).await
    }
}
