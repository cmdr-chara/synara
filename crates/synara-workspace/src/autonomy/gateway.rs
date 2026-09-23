//! Revocable incoming-client leases and one-shot native approval receipts.
//! Remote clients can propose operations, never approve or change their scope.
use super::{
    WorkflowSpec,
    workflow::{bounded, invalid},
};
use crate::{WorkspaceError, WorkspaceResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use synara_agent::ContextServer;
use synara_core::TaskId;
use synara_runtime::ComputerAction;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayClientKind {
    Agent,
    External,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum GatewayOperation {
    CreateWorkflow {
        spec: WorkflowSpec,
    },
    RunWorkflow {
        workflow: Uuid,
        revision: u64,
    },
    PauseWorkflow {
        stop: bool,
    },
    SteerWorkflow {
        workflow: Uuid,
        revision: u64,
        step: usize,
        instruction: String,
    },
    RetryWorkflow {
        workflow: Uuid,
        revision: u64,
        step: usize,
    },
    ObserveWindow {
        target: Uuid,
    },
    InputWindow {
        frame: Uuid,
        action: ComputerAction,
    },
}
impl GatewayOperation {
    pub fn validate(&self) -> WorkspaceResult<()> {
        match self {
            Self::CreateWorkflow { spec } => spec.validate(),
            Self::SteerWorkflow {
                step, instruction, ..
            } => {
                if *step >= 8 {
                    return Err(invalid("Unknown workflow step"));
                }
                bounded(instruction, 16 * 1024, true)
            }
            Self::RetryWorkflow { step, .. } if *step >= 8 => Err(invalid("Unknown workflow step")),
            Self::InputWindow { action, .. } => action.validate(8192, 8192).map_err(Into::into),
            _ => Ok(()),
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::CreateWorkflow { .. } => "Create unsent child-agent workflow",
            Self::RunWorkflow { .. } => "Run reviewed workflow and share its bounded child reports",
            Self::PauseWorkflow { stop: true } => "Stop workflow",
            Self::PauseWorkflow { stop: false } => "Pause workflow",
            Self::SteerWorkflow { .. } => "Replace reviewed pending-step instruction",
            Self::RetryWorkflow { .. } => "Prepare interrupted child for explicit retry",
            Self::ObserveWindow { .. } => "Share selected-window screenshot with this client",
            Self::InputWindow { .. } => "Deliver reviewed input to the observed window",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayRequestState {
    Pending,
    Running,
    Completed,
    Denied,
    Cancelled,
    Failed,
    Expired,
}
#[derive(Clone, Serialize)]
pub struct GatewayRequest {
    pub id: Uuid,
    pub parent: TaskId,
    pub client: Uuid,
    pub client_name: String,
    pub kind: GatewayClientKind,
    pub operation: GatewayOperation,
    pub state: GatewayRequestState,
    pub result: Option<Value>,
    pub error: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct GatewayInfo {
    pub id: Uuid,
    pub parent: TaskId,
    pub name: String,
    pub kind: GatewayClientKind,
    pub url: String,
    pub remaining_seconds: u64,
}
/// Deliberately no Debug/Serialize: contains a short-lived bearer credential.
#[derive(Clone)]
pub(crate) struct Lease {
    pub id: Uuid,
    pub parent: TaskId,
    pub name: String,
    pub kind: GatewayClientKind,
    pub url: String,
    pub token: String,
    pub profile: Option<String>,
    pub cancel: CancellationToken,
    pub expires: Instant,
}
impl Lease {
    pub fn live(&self) -> bool {
        !self.cancel.is_cancelled() && Instant::now() < self.expires
    }
}
struct Receipt {
    value: GatewayRequest,
    nonce: String,
    expires: Instant,
    cancel: CancellationToken,
    result_bytes: usize,
}
#[derive(Default)]
struct State {
    leases: HashMap<Uuid, Lease>,
    receipts: BTreeMap<Uuid, Receipt>,
    output_bytes: usize,
}
#[derive(Clone, Default)]
pub struct GatewayService {
    state: Arc<Mutex<State>>,
}
fn reap(state: &mut State) {
    for lease in state.leases.values() {
        if !lease.live() {
            lease.cancel.cancel();
        }
    }
    for receipt in state.receipts.values_mut() {
        if matches!(
            receipt.value.state,
            GatewayRequestState::Pending | GatewayRequestState::Running
        ) {
            if state
                .leases
                .get(&receipt.value.client)
                .is_none_or(|l| !l.live())
                || receipt.cancel.is_cancelled()
            {
                receipt.cancel.cancel();
                receipt.value.state = GatewayRequestState::Cancelled;
            } else if receipt.value.state == GatewayRequestState::Pending
                && Instant::now() >= receipt.expires
            {
                receipt.cancel.cancel();
                receipt.value.state = GatewayRequestState::Expired;
            }
        }
    }
}
impl GatewayService {
    pub(crate) fn register(
        &self,
        parent: TaskId,
        kind: GatewayClientKind,
        name: String,
        profile: Option<String>,
        url: String,
    ) -> WorkspaceResult<Lease> {
        bounded(&name, 80, false)?;
        let mut state = self.state.lock().map_err(|_| WorkspaceError::Worker)?;
        reap(&mut state);
        if state.leases.len() >= 8
            || state
                .leases
                .values()
                .any(|v| v.parent == parent && v.kind == kind)
        {
            return Err(invalid(
                "Revoke the previous client lease before enabling another. At most eight clients are retained",
            ));
        }
        let value = Lease {
            id: Uuid::new_v4(),
            parent,
            kind,
            name,
            profile,
            url,
            token: format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
            cancel: CancellationToken::new(),
            expires: Instant::now() + Duration::from_secs(15 * 60),
        };
        state.leases.insert(value.id, value.clone());
        Ok(value)
    }
    pub(crate) fn lease(&self, id: Uuid) -> WorkspaceResult<Lease> {
        let mut state = self.state.lock().map_err(|_| WorkspaceError::Worker)?;
        reap(&mut state);
        state
            .leases
            .get(&id)
            .filter(|v| v.live())
            .cloned()
            .ok_or_else(|| invalid("Client lease expired or was revoked"))
    }
    pub fn clients(&self, parent: TaskId) -> Vec<GatewayInfo> {
        let Ok(mut state) = self.state.lock() else {
            return vec![];
        };
        reap(&mut state);
        let mut info: Vec<_> = state
            .leases
            .values()
            .filter(|v| v.parent == parent)
            .map(|v| GatewayInfo {
                id: v.id,
                parent,
                name: v.name.clone(),
                kind: v.kind,
                url: v.url.clone(),
                remaining_seconds: if v.live() {
                    v.expires
                        .saturating_duration_since(Instant::now())
                        .as_secs()
                } else {
                    0
                },
            })
            .collect();
        info.sort_by_key(|v| v.id);
        info
    }
    /// Called only by an explicit native clipboard action. No automatic file export.
    pub fn configuration(&self, parent: TaskId, id: Uuid) -> WorkspaceResult<Value> {
        let value = self.lease(id)?;
        if value.parent != parent {
            return Err(invalid("Client belongs to a different task"));
        }
        Ok(
            serde_json::json!({"mcpServers":{"synara":{"type":"http","url":value.url,"headers":{"Authorization":format!("Bearer {}",value.token)}}}}),
        )
    }
    pub(crate) fn agent_context(&self, parent: TaskId, profile: &str) -> Option<ContextServer> {
        let mut state = self.state.lock().ok()?;
        reap(&mut state);
        let lease = state.leases.values().find(|v| {
            v.live()
                && v.parent == parent
                && v.kind == GatewayClientKind::Agent
                && v.profile.as_deref() == Some(profile)
        })?;
        Some(ContextServer::Http {
            name: "synara-agent-gateway".into(),
            url: lease.url.clone(),
            headers: BTreeMap::from([("Authorization".into(), format!("Bearer {}", lease.token))]),
        })
    }
    pub(crate) fn enqueue(
        &self,
        client: Uuid,
        nonce: String,
        operation: GatewayOperation,
    ) -> WorkspaceResult<Uuid> {
        operation.validate()?;
        if nonce.is_empty()
            || nonce.len() > 128
            || !nonce
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        {
            return Err(invalid("Invalid request nonce"));
        }
        let bytes = serde_json::to_vec(&operation).map_err(|_| invalid("Invalid operation"))?;
        if bytes.len() > 192 * 1024 {
            return Err(invalid("Operation exceeds request budget"));
        }
        let mut state = self.state.lock().map_err(|_| WorkspaceError::Worker)?;
        reap(&mut state);
        let lease = state
            .leases
            .get(&client)
            .filter(|v| v.live())
            .cloned()
            .ok_or_else(|| invalid("Client is no longer enrolled"))?;
        if let Some(old) = state
            .receipts
            .values()
            .find(|r| r.value.client == client && r.nonce == nonce)
        {
            return if old.value.operation == operation {
                Ok(old.value.id)
            } else {
                Err(invalid("Nonce already identifies a different operation"))
            };
        }
        if state.receipts.len() >= 128
            || state
                .receipts
                .values()
                .filter(|r| r.value.client == client)
                .count()
                >= 16
        {
            return Err(invalid(
                "Receipt limit reached. Revoke and explicitly reconnect this client",
            ));
        }
        let id = Uuid::new_v4();
        state.receipts.insert(
            id,
            Receipt {
                value: GatewayRequest {
                    id,
                    parent: lease.parent,
                    client,
                    client_name: lease.name,
                    kind: lease.kind,
                    operation,
                    state: GatewayRequestState::Pending,
                    result: None,
                    error: None,
                },
                nonce,
                expires: Instant::now() + Duration::from_secs(120),
                cancel: lease.cancel.child_token(),
                result_bytes: 0,
            },
        );
        Ok(id)
    }
    pub fn pending_count(&self, parent: TaskId) -> usize {
        let Ok(mut state) = self.state.lock() else {
            return 0;
        };
        reap(&mut state);
        state
            .receipts
            .values()
            .filter(|r| r.value.parent == parent && r.value.state == GatewayRequestState::Pending)
            .count()
    }
    pub fn requests(&self, parent: TaskId) -> Vec<GatewayRequest> {
        let Ok(mut state) = self.state.lock() else {
            return vec![];
        };
        reap(&mut state);
        state
            .receipts
            .values()
            .filter(|r| r.value.parent == parent)
            .map(|r| GatewayRequest {
                id: r.value.id,
                parent: r.value.parent,
                client: r.value.client,
                client_name: r.value.client_name.clone(),
                kind: r.value.kind,
                operation: r.value.operation.clone(),
                state: r.value.state,
                result: None,
                error: r.value.error.clone(),
            })
            .collect()
    }
    pub(crate) fn result(&self, client: Uuid, id: Uuid) -> WorkspaceResult<GatewayRequest> {
        self.lease(client)?;
        let mut state = self.state.lock().map_err(|_| WorkspaceError::Worker)?;
        reap(&mut state);
        state
            .receipts
            .get(&id)
            .filter(|r| r.value.client == client)
            .map(|r| r.value.clone())
            .ok_or_else(|| invalid("Receipt does not belong to this client"))
    }
    pub fn deny(&self, parent: TaskId, id: Uuid) -> WorkspaceResult<()> {
        let mut state = self.state.lock().map_err(|_| WorkspaceError::Worker)?;
        reap(&mut state);
        let r = state
            .receipts
            .get_mut(&id)
            .filter(|r| r.value.parent == parent)
            .ok_or_else(|| invalid("Unknown approval receipt"))?;
        if r.value.state != GatewayRequestState::Pending {
            return Err(invalid("Receipt is no longer awaiting approval"));
        }
        r.value.state = GatewayRequestState::Denied;
        r.cancel.cancel();
        Ok(())
    }
    pub(crate) fn cancel_request(&self, client: Uuid, id: Uuid) -> WorkspaceResult<()> {
        self.lease(client)?;
        let mut state = self.state.lock().map_err(|_| WorkspaceError::Worker)?;
        let r = state
            .receipts
            .get_mut(&id)
            .filter(|r| r.value.client == client)
            .ok_or_else(|| invalid("Receipt does not belong to this client"))?;
        if matches!(
            r.value.state,
            GatewayRequestState::Pending | GatewayRequestState::Running
        ) {
            r.cancel.cancel();
            r.value.state = GatewayRequestState::Cancelled;
        }
        Ok(())
    }
    /// This method is not exposed through MCP. Only the native UI calls it.
    pub(crate) fn claim(&self, parent: TaskId, id: Uuid) -> WorkspaceResult<GatewayClaim> {
        let mut state = self.state.lock().map_err(|_| WorkspaceError::Worker)?;
        reap(&mut state);
        let r = state
            .receipts
            .get_mut(&id)
            .filter(|r| r.value.parent == parent)
            .ok_or_else(|| invalid("Unknown native approval receipt"))?;
        if r.value.state != GatewayRequestState::Pending {
            return Err(invalid("Approval is stale, consumed, expired or revoked"));
        }
        r.value.state = GatewayRequestState::Running;
        Ok(GatewayClaim {
            owner: self.clone(),
            request: r.value.clone(),
            cancel: r.cancel.clone(),
        })
    }
    pub fn revoke(&self, parent: TaskId, client: Option<Uuid>) {
        if let Ok(mut state) = self.state.lock() {
            let ids: Vec<_> = state
                .leases
                .values()
                .filter(|v| v.parent == parent && client.is_none_or(|id| id == v.id))
                .map(|v| v.id)
                .collect();
            for id in &ids {
                if let Some(lease) = state.leases.remove(id) {
                    lease.cancel.cancel();
                }
            }
            state.receipts.retain(|_, r| !ids.contains(&r.value.client));
            state.output_bytes = state.receipts.values().map(|r| r.result_bytes).sum();
        }
    }
    pub fn shutdown(&self) {
        if let Ok(mut state) = self.state.lock() {
            for lease in state.leases.values() {
                lease.cancel.cancel();
            }
            state.leases.clear();
            state.receipts.clear();
            state.output_bytes = 0;
        }
    }
}
pub(crate) struct GatewayClaim {
    owner: GatewayService,
    pub request: GatewayRequest,
    pub cancel: CancellationToken,
}
impl GatewayClaim {
    pub fn finish(&self, result: WorkspaceResult<Value>) {
        if let Ok(mut state) = self.owner.state.lock() {
            reap(&mut state);
            let mut added = 0;
            let budget = 16 * 1024 * 1024 - state.output_bytes.min(16 * 1024 * 1024);
            if let Some(r) = state.receipts.get_mut(&self.request.id) {
                if r.value.state != GatewayRequestState::Running || r.cancel.is_cancelled() {
                    return;
                }
                match result {
                    Ok(value) => {
                        let bytes = value.to_string().len();
                        if bytes > 3 * 1024 * 1024 || bytes > budget {
                            r.value.state = GatewayRequestState::Failed;
                            r.value.error = Some("Operation ended, but its response exceeds the retained result budget. Inspect native state before retrying.".into());
                        } else {
                            r.value.state = GatewayRequestState::Completed;
                            r.value.result = Some(value);
                            r.result_bytes = bytes;
                            added = bytes;
                        }
                    }
                    Err(_) => {
                        r.value.state = GatewayRequestState::Failed;
                        r.value.error = Some("Operation failed or was interrupted. Inspect native state. Effects may already exist, and no automatic retry was attempted.".into());
                    }
                }
            }
            state.output_bytes += added;
        }
    }
}
impl Drop for GatewayClaim {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Ok(mut state) = self.owner.state.lock()
            && let Some(r) = state.receipts.get_mut(&self.request.id)
            && r.value.state == GatewayRequestState::Running
        {
            r.value.state = GatewayRequestState::Cancelled;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn client(owner: &GatewayService, parent: TaskId) -> Lease {
        owner
            .register(
                parent,
                GatewayClientKind::External,
                "Fixture".into(),
                None,
                "http://127.0.0.1:123/mcp".into(),
            )
            .unwrap()
    }
    #[test]
    fn nonce_receipts_are_scoped_idempotent_and_native_approval_is_one_shot() {
        let owner = GatewayService::default();
        let parent = TaskId::new();
        let a = client(&owner, parent);
        let id = owner
            .enqueue(
                a.id,
                "n".into(),
                GatewayOperation::ObserveWindow {
                    target: Uuid::nil(),
                },
            )
            .unwrap();
        assert_eq!(
            owner
                .enqueue(
                    a.id,
                    "n".into(),
                    GatewayOperation::ObserveWindow {
                        target: Uuid::nil()
                    }
                )
                .unwrap(),
            id
        );
        assert!(
            owner
                .enqueue(
                    a.id,
                    "n".into(),
                    GatewayOperation::PauseWorkflow { stop: true }
                )
                .is_err()
        );
        assert!(owner.claim(TaskId::new(), id).is_err());
        assert_eq!(
            owner.result(a.id, id).unwrap().state,
            GatewayRequestState::Pending
        );
        let claim = owner.claim(parent, id).unwrap();
        assert!(owner.claim(parent, id).is_err());
        claim.finish(Ok(serde_json::json!({"done":true})));
        drop(claim);
        assert_eq!(
            owner.result(a.id, id).unwrap().state,
            GatewayRequestState::Completed
        );
        let b = client(&owner, TaskId::new());
        assert!(owner.result(b.id, id).is_err());
    }
    #[test]
    fn revoke_denial_expiry_and_dropped_approval_never_become_success() {
        let owner = GatewayService::default();
        let parent = TaskId::new();
        let a = client(&owner, parent);
        let one = owner
            .enqueue(
                a.id,
                "one".into(),
                GatewayOperation::ObserveWindow {
                    target: Uuid::nil(),
                },
            )
            .unwrap();
        owner.deny(parent, one).unwrap();
        assert!(owner.claim(parent, one).is_err());
        let two = owner
            .enqueue(
                a.id,
                "two".into(),
                GatewayOperation::ObserveWindow {
                    target: Uuid::nil(),
                },
            )
            .unwrap();
        drop(owner.claim(parent, two).unwrap());
        assert_eq!(
            owner.result(a.id, two).unwrap().state,
            GatewayRequestState::Cancelled
        );
        let three = owner
            .enqueue(
                a.id,
                "three".into(),
                GatewayOperation::ObserveWindow {
                    target: Uuid::nil(),
                },
            )
            .unwrap();
        owner
            .state
            .lock()
            .unwrap()
            .receipts
            .get_mut(&three)
            .unwrap()
            .expires = Instant::now();
        assert!(owner.claim(parent, three).is_err());
        let four = owner
            .enqueue(
                a.id,
                "four".into(),
                GatewayOperation::ObserveWindow {
                    target: Uuid::nil(),
                },
            )
            .unwrap();
        let live = owner.claim(parent, four).unwrap();
        owner.revoke(parent, None);
        assert!(live.cancel.is_cancelled());
        assert!(owner.lease(a.id).is_err());
        assert!(owner.result(a.id, four).is_err());
        assert!(owner.clients(parent).is_empty());
    }
    #[test]
    fn request_scope_and_input_shapes_cannot_be_forged() {
        assert!(
            serde_json::from_str::<GatewayOperation>(
                r#"{"operation":"observe_window","parent":"forged"}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<GatewayOperation>(r#"{"operation":"approve","id":"forged"}"#)
                .is_err()
        );
        let owner = GatewayService::default();
        let a = client(&owner, TaskId::new());
        for i in 0..16 {
            owner
                .enqueue(
                    a.id,
                    format!("n{i}"),
                    GatewayOperation::ObserveWindow {
                        target: Uuid::nil(),
                    },
                )
                .unwrap();
        }
        assert!(
            owner
                .enqueue(
                    a.id,
                    "extra".into(),
                    GatewayOperation::ObserveWindow {
                        target: Uuid::nil()
                    }
                )
                .is_err()
        );
        assert!(owner.configuration(TaskId::new(), a.id).is_err());
        assert!(owner.agent_context(a.parent, "profile").is_none());
    }
}
