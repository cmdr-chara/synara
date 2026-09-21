//! Session changes and credential resolution remain inside the generic controller.
use super::*;
use crate::integrations::{
    invalid,
    probe::{credential, probe},
};
use crate::{IntegrationSettings, ManagedMcp, McpEdit, McpProbeReport};
use synara_runtime::SecretStoreState;

impl Controller {
    pub fn integration_secret_state(&self) -> SecretStoreState {
        self.secrets.state()
    }

    /// Explicit user action only. A probe does not enable a connection, assign an
    /// agent, execute a tool, or establish that the agent can reach this endpoint.
    pub async fn test_mcp(&self, revision: u64, id: String) -> WorkspaceResult<McpProbeReport> {
        let _gate = self.integrations_gate.read().await;
        let value = self.workspace.integrations().await?;
        if value.revision != revision {
            return Err(invalid("Integrations changed. Reload before testing."));
        }
        let config = value
            .mcp
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(WorkspaceError::NotFound)?;
        self.local_mcp_task(config.task).await?;
        let token = credential(&config, self.secrets.as_ref()).await?;
        tokio::task::spawn_blocking(move || probe(config, token))
            .await
            .map_err(|_| WorkspaceError::Worker)?
    }
    async fn local_mcp_task(&self, id: TaskId) -> WorkspaceResult<Task> {
        let task = self.workspace.task(id).await?;
        if !matches!(
            self.workspace.workspace_for_task(&task).await?.location,
            WorkspaceLocation::Local { .. }
        ) {
            return Err(invalid(
                "Synara-managed MCP is currently local-workspace only. A desktop test cannot establish reachability from an SSH agent.",
            ));
        }
        Ok(task)
    }
    /// Changes are serialized against session creation. Running prompts refuse
    /// reconfiguration. An agent that cannot close its session must be explicitly
    /// disconnected first, not silently restarted with the old credentials.
    pub async fn configure_mcp(
        &self,
        revision: u64,
        edit: McpEdit,
    ) -> WorkspaceResult<IntegrationSettings> {
        let _gate = self.integrations_gate.write().await;
        let value = self.workspace.integrations().await?;
        if value.revision != revision {
            return Err(invalid(
                "Integrations changed. Reload before changing this connection.",
            ));
        }
        let id = edit.target(&value)?;
        let mut prospective = value.clone();
        edit.clone().apply(&mut prospective)?;
        if matches!(
            &edit,
            McpEdit::Save(_) | McpEdit::SetEnabled { enabled: true, .. }
        ) {
            let task = self.local_mcp_task(id).await?;
            let agent = match &edit {
                McpEdit::Save(config) => &config.agent_id,
                McpEdit::SetEnabled { id, .. } => {
                    &prospective
                        .mcp
                        .iter()
                        .find(|item| &item.id == id)
                        .ok_or(WorkspaceError::NotFound)?
                        .agent_id
                }
                _ => unreachable!(),
            };
            if agent != &task.agent_id {
                return Err(invalid(
                    "This connection belongs to another agent. Select that agent before enabling or editing it.",
                ));
            }
            self.profile(agent).await?;
        }
        let slot = self.slot(id).await?;
        if slot.active.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        let _creation = slot.creation.lock().await;
        if let Some(session) = slot.session()? {
            if slot
                .connection()?
                .is_some_and(|c| c.info().state == ConnectionState::Connected)
            {
                session.close().await.map_err(|_|invalid("The agent did not confirm session closure. Settings were not changed. Stop its active tasks, then explicitly disconnect the shared agent process before retrying."))?;
            }
            slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
        }
        self.workspace.edit_mcp(revision, edit).await
    }
    /// Explicit consent in the UI discloses that the process may be shared.
    /// Refuse while any task on that process has an active prompt. Never relaunch.
    pub async fn disconnect_mcp_agent(&self, id: TaskId) -> WorkspaceResult<()> {
        let _gate = self.integrations_gate.write().await;
        let slot = self.slot(id).await?;
        let Some(connection) = slot.connection()? else {
            return Ok(());
        };
        let slots: Vec<_> = self.tasks.lock().await.values().cloned().collect();
        for slot in &slots {
            if slot
                .connection()?
                .is_some_and(|other| Arc::ptr_eq(&other, &connection))
                && slot.active.load(Ordering::Acquire)
            {
                return Err(AgentError::Busy.into());
            }
        }
        connection.disconnect().await?;
        Ok(())
    }
    pub(super) async fn managed_mcp_context(
        &self,
        task: &Task,
        connection: &dyn AgentConnection,
    ) -> WorkspaceResult<Vec<ContextServer>> {
        let value = self.workspace.integrations().await?;
        let selected: Vec<ManagedMcp> = value
            .mcp
            .into_iter()
            .filter(|item| item.applies_to(task.id, &task.agent_id))
            .collect();
        if selected.is_empty() {
            return Ok(vec![]);
        }
        self.local_mcp_task(task.id).await?;
        if !connection.info().capabilities.mcp_http {
            return Err(invalid(
                "The selected agent did not negotiate HTTP MCP configuration support. Disable this task's managed connections or select a compatible agent. Agent names are not compatibility evidence.",
            ));
        }
        let mut servers = Vec::new();
        for config in selected {
            let token = credential(&config, self.secrets.as_ref()).await?;
            let mut headers = std::collections::BTreeMap::new();
            if let Some(token) = token {
                let token = std::str::from_utf8(token.expose())
                    .map_err(|_| invalid("Invalid bearer token."))?;
                headers.insert("Authorization".into(), format!("Bearer {token}"));
            }
            servers.push(ContextServer::Http {
                name: config.name,
                url: config.endpoint,
                headers,
            });
        }
        Ok(servers)
    }
}

#[cfg(test)]
mod tests;
