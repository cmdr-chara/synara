//! Browser use enrolls one exact connected task/profile after native confirmation.
//! Never infer transport capability from the provider name, or auto-launch an agent.
use super::*;
impl Controller {
    pub fn browser_use_enabled(&self, id: TaskId) -> bool {
        self.browser_endpoints
            .lock()
            .is_ok_and(|e| e.get(&id).and_then(|e| e.context()).is_some())
    }
    pub fn revoke_browser_use(&self, id: TaskId) {
        if let Ok(mut endpoints) = self.browser_endpoints.lock() {
            endpoints.remove(&id);
        }
        self.browser.revoke(id.0.as_u128());
    }
    pub async fn configure_browser_use(&self, id: TaskId, enable: bool) -> WorkspaceResult<()> {
        let _integrations = self.integrations_gate.write().await;
        if !enable {
            self.revoke_browser_use(id);
            return Ok(());
        }
        let task = self.workspace.task(id).await?;
        if !matches!(
            self.workspace.workspace_for_task(&task).await?.location,
            WorkspaceLocation::Local { .. }
        ) {
            return Err(WorkspaceError::Invalid("Browser use requires an explicitly connected local agent. SSH access is unsupported.".into()));
        }
        if !self
            .browser
            .with(|s, _| Ok(s.capabilities().navigation))
            .map_err(|e| WorkspaceError::Invalid(e.to_string()))?
        {
            return Err(WorkspaceError::Invalid("The native browser adapter is not installed. No browser-use capability was granted.".into()));
        }
        let slot = self.slot(id).await?;
        if slot.active.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        let _creation = slot.creation.lock().await;
        let connection = slot.connection()?.ok_or_else(|| {
            WorkspaceError::Invalid("Connect the selected agent explicitly first.".into())
        })?;
        if connection.info().state != ConnectionState::Connected
            || !connection.info().capabilities.mcp_http
        {
            return Err(WorkspaceError::Invalid(
                "This connection has not negotiated HTTP MCP support.".into(),
            ));
        }
        let profile = self.profile(&task.agent_id).await?;
        {
            let live = slot.live.lock().map_err(|_| WorkspaceError::Worker)?;
            if live.as_ref().is_none_or(|live| {
                live.profile != profile || !Arc::ptr_eq(&live.connection, &connection)
            }) {
                return Err(WorkspaceError::Invalid(
                    "Reconnect the current profile before granting browser use.".into(),
                ));
            }
        }
        if let Some(session) = slot.session()? {
            tokio::time::timeout(std::time::Duration::from_secs(8), session.close())
                .await
                .map_err(|_| AgentError::Timeout)??;
            slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
        }
        self.workspace.forget_session(task.thread_id).await?;
        self.revoke_browser_use(id);
        let endpoint = crate::browser::mcp::start(
            self.browser.clone(),
            self.workspace.clone(),
            id,
            profile,
            connection,
        )
        .await
        .map_err(WorkspaceError::Invalid)?;
        self.browser_endpoints
            .lock()
            .map_err(|_| WorkspaceError::Worker)?
            .insert(id, endpoint);
        Ok(())
    }
    pub(super) fn browser_context(
        &self,
        id: TaskId,
        profile: &AgentProfile,
        connection: &dyn AgentConnection,
    ) -> WorkspaceResult<Option<ContextServer>> {
        let endpoints = self
            .browser_endpoints
            .lock()
            .map_err(|_| WorkspaceError::Worker)?;
        let Some(endpoint) = endpoints.get(&id) else {
            return Ok(None);
        };
        if &endpoint.profile != profile || !connection.info().capabilities.mcp_http {
            return Err(WorkspaceError::Invalid(
                "Browser-use identity changed. Revoke and re-enable explicitly.".into(),
            ));
        }
        Ok(endpoint.context())
    }
}
