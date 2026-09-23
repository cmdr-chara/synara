//! App-owned orchestration and incoming-client boundaries. Nothing starts on load.
pub(crate) mod workflow;
/// Native workflow, client receipt and observation identifiers.
pub use uuid::Uuid as AutonomyId;
pub use workflow::*;
mod gateway;
pub use gateway::{
    GatewayClientKind, GatewayInfo, GatewayOperation, GatewayRequest, GatewayRequestState,
    GatewayService,
};
pub(crate) mod computer;
pub(crate) mod transport;
pub use computer::{ComputerFrame, ComputerService};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use synara_core::TaskId;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct RunControl {
    pub id: Uuid,
    pub cancel: CancellationToken,
    pub stop: Arc<AtomicBool>,
}
#[derive(Default)]
pub struct Autonomy {
    runs: Arc<Mutex<HashMap<TaskId, RunControl>>>,
    pub gateway: GatewayService,
    pub computer: ComputerService,
}
pub(crate) struct RunGuard {
    root: TaskId,
    owner: Arc<Mutex<HashMap<TaskId, RunControl>>>,
    pub control: RunControl,
}
impl Drop for RunGuard {
    fn drop(&mut self) {
        self.control.cancel.cancel();
        if let Ok(mut runs) = self.owner.lock()
            && runs
                .get(&self.root)
                .is_some_and(|v| v.id == self.control.id)
        {
            runs.remove(&self.root);
        }
    }
}
impl Autonomy {
    pub fn workflow_running(&self, root: TaskId) -> bool {
        self.runs.lock().map_or(true, |v| v.contains_key(&root))
    }
    pub(crate) fn begin(&self, root: TaskId) -> crate::WorkspaceResult<RunGuard> {
        self.begin_interruptible(root, CancellationToken::new())
    }
    pub(crate) fn begin_interruptible(
        &self,
        root: TaskId,
        cancellation: CancellationToken,
    ) -> crate::WorkspaceResult<RunGuard> {
        if cancellation.is_cancelled() {
            return Err(workflow::invalid(
                "Workflow cancelled before acquiring ownership",
            ));
        }
        let mut runs = self
            .runs
            .lock()
            .map_err(|_| crate::WorkspaceError::Worker)?;
        if runs.contains_key(&root) || runs.len() >= 32 {
            return Err(workflow::invalid(
                "A workflow already owns this task, or the live workflow limit was reached",
            ));
        }
        let control = RunControl {
            id: Uuid::new_v4(),
            cancel: cancellation.child_token(),
            stop: Arc::new(AtomicBool::new(false)),
        };
        runs.insert(root, control.clone());
        Ok(RunGuard {
            root,
            owner: self.runs.clone(),
            control,
        })
    }
    /// Synchronous interruption remains available while any async operation awaits.
    pub fn interrupt(&self, root: TaskId, stop: bool) -> bool {
        if let Ok(runs) = self.runs.lock()
            && let Some(run) = runs.get(&root)
        {
            if stop {
                run.stop.store(true, Ordering::Release);
            }
            run.cancel.cancel();
            return true;
        }
        false
    }
    pub fn revoke(&self, root: TaskId) {
        self.interrupt(root, true);
        self.gateway.revoke(root, None);
        self.computer.revoke(root);
    }
    pub fn shutdown(&self) {
        if let Ok(runs) = self.runs.lock() {
            for run in runs.values() {
                run.stop.store(true, Ordering::Release);
                run.cancel.cancel();
            }
        }
        self.gateway.shutdown();
        self.computer.shutdown();
    }
}
impl Drop for Autonomy {
    fn drop(&mut self) {
        self.shutdown();
    }
}
