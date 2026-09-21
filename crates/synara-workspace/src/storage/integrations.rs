//! Atomic user-owned integration catalog, separate from general Settings.
use super::*;
use crate::{integrations::*, WorkspaceError, WorkspaceResult, WorkspaceService};

fn read(connection: &Connection) -> WorkspaceResult<IntegrationSettings> {
    let raw: Option<String> = connection.query_row(
        "SELECT data FROM preferences WHERE key=?1", [INTEGRATIONS_KEY], |r|r.get(0)
    ).optional().map_err(StorageError::from)?;
    let value = match raw {
        Some(raw) if raw.len() > 4 * 1024 * 1024 => return Err(StorageError::Limit.into()),
        Some(raw) => decode::<IntegrationSettings>(&raw)?,
        None => IntegrationSettings::default(),
    };
    value.validate()?;
    Ok(value)
}
fn change(store: &mut Store, revision: u64, edit: impl FnOnce(&Connection, &mut IntegrationSettings) -> WorkspaceResult<()>) -> WorkspaceResult<IntegrationSettings> {
    let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(StorageError::from)?;
    let mut value = read(&tx)?;
    if value.revision != revision { return Err(invalid("Integrations changed. Reload and review the current settings before retrying.")); }
    edit(&tx, &mut value)?;
    value.validate()?;
    value.revision = revision.checked_add(1).ok_or(StorageError::Limit)?;
    let data = encode(&value)?;
    if data.len() > 4 * 1024 * 1024 { return Err(StorageError::Limit.into()); }
    tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![INTEGRATIONS_KEY,data]).map_err(StorageError::from)?;
    tx.commit().map_err(StorageError::from)?;
    Ok(value)
}
impl WorkspaceService {
    pub async fn integrations(&self) -> WorkspaceResult<IntegrationSettings> {
        self.access(|store|read(&store.connection)).await
    }
    pub async fn review_skill(&self, path: std::path::PathBuf) -> WorkspaceResult<SkillReview> {
        tokio::task::spawn_blocking(move || SkillReview::inspect(path)).await.map_err(|_| WorkspaceError::Worker)?
    }
    /// Explicit approval of a previously displayed document snapshot.
    pub async fn install_skill(&self, revision: u64, review: SkillReview, replace: Option<String>) -> WorkspaceResult<IntegrationSettings> {
        let mut skill = tokio::task::spawn_blocking(move || review.recheck()).await.map_err(|_| WorkspaceError::Worker)??;
        self.access(move |store|change(store,revision,move |_,value| {
            if let Some(id) = replace {
                let old = value.skills.iter_mut().find(|old| old.id == id).ok_or(WorkspaceError::NotFound)?;
                skill.id = old.id.clone();
                skill.previous_origins = old.previous_origins.clone();
                skill.previous_origins.push(old.origin.clone());
                if skill.previous_origins.len() > 8 { skill.previous_origins.remove(0); }
                *old = skill;
            } else {
                if value.skills.iter().any(|old| old.origin.sha256 == skill.origin.sha256) {
                    return Err(invalid("This exact skill document is already installed."));
                }
                value.skills.push(skill);
            }
            Ok(())
        })).await
    }
    pub async fn edit_skill(&self, revision: u64, edit: SkillEdit) -> WorkspaceResult<IntegrationSettings> {
        self.access(move |store|change(store,revision,move |_,value| {
            match edit {
                SkillEdit::SetEnabled {id,enabled} => value.skills.iter_mut().find(|s|s.id==id).ok_or(WorkspaceError::NotFound)?.enabled = enabled,
                SkillEdit::Remove(id) => {
                    let index = value.skills.iter().position(|s|s.id==id).ok_or(WorkspaceError::NotFound)?;
                    value.skills.remove(index);
                },
            }
            Ok(())
        })).await
    }
    /// Called only by Controller after it has excluded prompts and retired the old
    /// session. Raw repository files and provider-owned configs never reach here.
    pub(crate) async fn edit_mcp(&self, revision: u64, edit: McpEdit) -> WorkspaceResult<IntegrationSettings> {
        self.access(move |store|change(store,revision,move |tx,value| {
            let task = edit.target(value)?;
            let raw: Option<String> = tx.query_row("SELECT data FROM tasks WHERE id=?1",[task.to_string()],|r|r.get(0)).optional().map_err(StorageError::from)?;
            let task: Option<Task> = raw.as_deref().map(decode).transpose()?;
            if task.is_none() && !matches!(&edit,McpEdit::Remove(_) | McpEdit::SetEnabled{enabled:false,..}) {
                return Err(WorkspaceError::NotFound);
            }
            edit.apply(value)?;
            let Some(task)=task else {return Ok(());};
            // Inactive entries may refer to an earlier selected agent. Enabling
            // and additions must target the current exact profile.
            if value.mcp.iter().any(|item|item.task==task.id && item.enabled && item.agent_id!=task.agent_id) {
                return Err(invalid("Select this connection's agent before enabling it."));
            }
            // Never restore a remote session carrying a superseded configuration.
            tx.execute("DELETE FROM sessions WHERE thread_id=?1",[task.thread_id.to_string()]).map_err(StorageError::from)?;
            Ok(())
        })).await
    }
}

#[cfg(test)]
mod tests;
