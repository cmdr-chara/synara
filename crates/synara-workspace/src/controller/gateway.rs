//! Incoming clients can propose operations. Native UI enrollment and approval
//! are the only callers of the mutating controller entry points in this module.
use super::*;
use crate::autonomy::workflow::{fingerprint, invalid};
use crate::autonomy::{GatewayClientKind, GatewayInfo, GatewayOperation};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

impl Controller {
    pub async fn enable_gateway(
        self: &Arc<Self>,
        parent: TaskId,
        kind: GatewayClientKind,
        name: String,
    ) -> WorkspaceResult<GatewayInfo> {
        let _integrations = self.integrations_gate.write().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(AgentError::Cancelled.into());
        }
        let task = self.workspace.task(parent).await?;
        if task.state == TaskState::Archived {
            return Err(invalid("Archived tasks cannot enroll incoming clients"));
        }
        if !matches!(
            self.workspace.workspace_for_task(&task).await?.location,
            WorkspaceLocation::Local { .. }
        ) {
            return Err(invalid(
                "Incoming MCP clients are local-only. SSH gateway forwarding is not implemented",
            ));
        }
        if self.workspace.workflow_parent(parent).await?.is_some() {
            return Err(invalid(
                "Delegated children cannot enroll gateways or recursively delegate",
            ));
        }
        if !self
            .autonomy
            .gateway
            .clients(parent)
            .iter()
            .all(|v| v.kind != kind)
        {
            return Err(invalid(
                "Revoke the existing client of this kind before enrolling again",
            ));
        }
        let profile = if kind == GatewayClientKind::Agent {
            let slot = self.slot(parent).await?;
            if slot.active.swap(true, Ordering::AcqRel) {
                return Err(AgentError::Busy.into());
            }
            let _ownership = PromptOwnership(slot.clone());
            let _creation = slot.creation.lock().await;
            self.require_agent_route(parent).await?;
            let connection = slot.connection()?.ok_or_else(|| {
                invalid("Connect the selected local agent explicitly before enabling Agent Gateway")
            })?;
            if connection.info().state != ConnectionState::Connected
                || !connection.info().capabilities.mcp_http
            {
                return Err(invalid(
                    "Selected connection has not negotiated HTTP MCP support",
                ));
            }
            let profile = self.profile(&task.agent_id).await?;
            {
                let live = slot.live.lock().map_err(|_| WorkspaceError::Worker)?;
                if live.as_ref().is_none_or(|v| {
                    v.profile != profile || !Arc::ptr_eq(&v.connection, &connection)
                }) {
                    return Err(invalid(
                        "Reconnect this exact agent profile before enrollment",
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
            Some(fingerprint(&profile)?)
        } else {
            None
        };
        let lease = crate::autonomy::transport::start(self, parent, kind, name, profile)?;
        self.autonomy
            .gateway
            .clients(parent)
            .into_iter()
            .find(|v| v.id == lease.id)
            .ok_or(WorkspaceError::Worker)
    }
    pub(crate) async fn gateway_agent_connected(&self, parent: TaskId) -> bool {
        self.slot(parent)
            .await
            .ok()
            .and_then(|slot| slot.connection().ok().flatten())
            .is_some_and(|connection| {
                connection.info().state == ConnectionState::Connected
                    && connection.info().capabilities.mcp_http
            })
    }
    pub(super) fn gateway_context(
        &self,
        parent: TaskId,
        profile: &AgentProfile,
        connection: &dyn AgentConnection,
    ) -> WorkspaceResult<Option<ContextServer>> {
        let context = self
            .autonomy
            .gateway
            .agent_context(parent, &fingerprint(profile)?);
        if context.is_some() && !connection.info().capabilities.mcp_http {
            return Err(invalid(
                "This connection no longer advertises HTTP MCP support. Revoke Agent Gateway",
            ));
        }
        Ok(context)
    }
    /// Metadata only. Parent messages, files, secrets and dependency output are
    /// not exposed merely because a local client was enrolled.
    pub async fn gateway_status(&self, parent: TaskId) -> WorkspaceResult<Value> {
        let root = self.workspace.task(parent).await?;
        if root.state == TaskState::Archived {
            return Err(invalid("Task is archived"));
        }
        let profiles = self.workspace.profiles().await?;
        let agents: Vec<_> = profiles
            .iter()
            .map(|p| json!({"id":p.id,"name":p.name}))
            .collect();
        let workflow = match self.workspace.workflow(parent).await? {
            None => Value::Null,
            Some(value) => {
                let mut steps = Vec::new();
                for (index, step) in value.steps.iter().enumerate() {
                    let task = self.workspace.task(step.task).await?;
                    steps.push(json!({"index":index,"task":task.id,"title":task.title,"agent_id":task.agent_id,"state":step.state,"task_state":task.state,"attempts":step.attempts,"depends_on":value.spec.steps[index].depends_on,"usage":step.usage,"error":step.error}));
                }
                json!({"id":value.id,"revision":value.revision,"title":value.spec.title,"phase":value.phase,"concurrency":value.spec.concurrency,"live_owner":self.autonomy.workflow_running(parent),"steps":steps})
            }
        };
        let computer = self.autonomy.computer.selected(parent).map(|window| json!({"target":self.autonomy.computer.selection_id(parent),"title":window.title,"width":window.width,"height":window.height,"input_requires_fresh_frame_and_native_approval":true}));
        Ok(
            json!({"task":parent,"title":root.title,"agents":agents,"workflow":workflow,"computer":computer,"scope":"Only this task and its workflow. No parent conversation, files, secrets or arbitrary child access."}),
        )
    }
    pub async fn approve_gateway(
        self: &Arc<Self>,
        parent: TaskId,
        receipt: Uuid,
    ) -> WorkspaceResult<()> {
        if self.closing.load(Ordering::Acquire) {
            return Err(AgentError::Cancelled.into());
        }
        let claim = self.autonomy.gateway.claim(parent, receipt)?;
        let task = self.workspace.task(parent).await?;
        if task.state == TaskState::Archived || claim.cancel.is_cancelled() {
            return Err(invalid("Approval scope is no longer active"));
        }
        self.workspace
            .record(
                task.thread_id,
                ThreadEvent::Notice {
                    message: format!(
                        "Native approval {receipt}: {}. Scoped to this task. No automatic retry.",
                        claim.request.operation.label()
                    ),
                },
            )
            .await?;
        let result = self
            .execute_gateway_operation(
                parent,
                claim.request.operation.clone(),
                claim.cancel.clone(),
            )
            .await;
        let succeeded = result.is_ok() && !claim.cancel.is_cancelled();
        claim.finish(result);
        self.workspace.record(task.thread_id, ThreadEvent::Notice { message: format!("Gateway receipt {receipt}: {}. Inspect workflow or target state before another action.", if succeeded { "operation returned" } else { "interrupted or failed, effects may already exist" }) }).await?;
        Ok(())
    }
    async fn execute_gateway_operation(
        self: &Arc<Self>,
        parent: TaskId,
        operation: GatewayOperation,
        cancel: CancellationToken,
    ) -> WorkspaceResult<Value> {
        if cancel.is_cancelled() {
            return Err(AgentError::Cancelled.into());
        }
        match operation {
            GatewayOperation::CreateWorkflow { spec } => {
                let value = self.workspace.create_workflow(parent, spec).await?;
                Ok(
                    json!({"workflow":value.id,"revision":value.revision,"phase":value.phase,"created_unsent":true}),
                )
            }
            GatewayOperation::RunWorkflow { workflow, revision } => {
                let value = self
                    .run_workflow_interruptible(parent, workflow, revision, cancel)
                    .await?;
                let reports: Vec<_> = value
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(index, step)| {
                        json!({
                            "index":index,"task":step.task,"state":step.state,"output":step.output,
                            "usage":step.usage,"error":step.error
                        })
                    })
                    .collect();
                Ok(
                    json!({"workflow":value.id,"revision":value.revision,"phase":value.phase,
                    "reports":reports,"outputs_are_untrusted":true}),
                )
            }
            GatewayOperation::PauseWorkflow { stop } => {
                self.pause_workflow(parent, stop).await?;
                Ok(json!({"interruption_requested":true,"stop":stop}))
            }
            GatewayOperation::SteerWorkflow {
                workflow,
                revision,
                step,
                instruction,
            } => {
                let value = self
                    .workspace
                    .steer_workflow_step(parent, workflow, revision, step, instruction)
                    .await?;
                Ok(json!({"workflow":value.id,"revision":value.revision,"sent":false}))
            }
            GatewayOperation::RetryWorkflow {
                workflow,
                revision,
                step,
            } => {
                let value = self
                    .retry_workflow_step(parent, workflow, revision, step)
                    .await?;
                Ok(json!({"workflow":value.id,"revision":value.revision,"sent":false}))
            }
            GatewayOperation::ObserveWindow { target } => {
                let frame = self
                    .autonomy
                    .computer
                    .observe(parent, target, cancel)
                    .await?;
                Ok(
                    json!({"frame":frame.id,"width":frame.window.width,"height":frame.window.height,"title":frame.window.title,"png_base64":crate::autonomy::computer::base64(&frame.png),"content_is_untrusted":true,"input_lease_seconds":60}),
                )
            }
            GatewayOperation::InputWindow { frame, action } => {
                self.autonomy
                    .computer
                    .act(parent, frame, action, cancel)
                    .await?;
                Ok(
                    json!({"input_delivered":true,"application_success_verified":false,"next_step":"Explicitly observe the target again. The frame has been consumed."}),
                )
            }
        }
    }
}
