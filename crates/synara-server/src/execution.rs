//! Explicit local web runs reuse the native Controller and task-scoped interaction
//! broker. Connection-scoped authentication remains unsupported by the web UI.
use super::*;
use std::collections::HashMap;
use synara_core::{TaskId, TaskState};
use synara_workspace::WorkspaceError;
use tokio::sync::Mutex;

const MAX_ACTIVE_RUNS: usize = 8;
const MAX_RUN_RECORDS: usize = 128;
const RUN_TIMEOUT: Duration = Duration::from_secs(3600);

#[derive(Clone, Copy, Serialize)]
struct RunView {
    state: &'static str,
    error: Option<&'static str>,
}
impl RunView {
    fn new(state: &'static str) -> Self {
        Self { state, error: None }
    }
    fn response(self, status: u16) -> Response {
        Response::json(
            status,
            serde_json::to_vec(&self).expect("static run status"),
        )
    }
}
struct Run {
    view: Arc<StdMutex<RunView>>,
    cancellation: CancellationToken,
    worker: JoinHandle<()>,
}
impl Run {
    fn view(&self) -> RunView {
        let mut view = *self
            .view
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.worker.is_finished() && matches!(view.state, "running" | "stopping") {
            view.state = "failed";
            view.error = Some("worker_stopped");
        }
        view
    }
}
#[derive(Default)]
pub(super) struct ExecutionOwner {
    runs: Mutex<HashMap<TaskId, Run>>,
    // Draft writes and admission share this gate, so a stale tab cannot overwrite
    // a newer draft between comparison and explicit submission.
    pub(super) mutation: Mutex<()>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartRequest {
    text: String,
    expected_draft: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StopRequest {}

fn task_id(path: &str, suffix: &str) -> Option<TaskId> {
    let raw = path.strip_prefix("/api/tasks/")?.strip_suffix(suffix)?;
    serde_json::from_value(serde_json::Value::String(raw.to_owned())).ok()
}
fn error(status: u16, message: &'static str) -> Response {
    Response::json(
        status,
        serde_json::to_vec(&serde_json::json!({"error": message})).unwrap(),
    )
}

pub(super) async fn dispatch_get(path: &str, state: &AppState) -> Response {
    let Some(id) = task_id(path, "/run") else {
        return bad_request();
    };
    let runtime = state.runtime.read().await;
    let Some(runtime) = runtime.as_ref() else {
        return health_response(Lifecycle::Starting);
    };
    if runtime.workspace.task(id).await.is_err() {
        return error(404, "not_found");
    }
    let runs = state.execution.runs.lock().await;
    runs.get(&id)
        .map_or(RunView::new("idle"), Run::view)
        .response(200)
}

pub(super) async fn dispatch_post(
    path: &str,
    request: &Request,
    state: &AppState,
    runtime: &RuntimeServices,
) -> Response {
    if path.ends_with("/stop") {
        let Some(id) = task_id(path, "/stop") else {
            return bad_request();
        };
        if serde_json::from_slice::<StopRequest>(&request.body).is_err() {
            return bad_request();
        }
        let runs = state.execution.runs.lock().await;
        let Some(run) = runs.get(&id) else {
            return error(409, "no_web_run");
        };
        {
            let mut view = run
                .view
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if view.state == "running" && !run.worker.is_finished() {
                run.cancellation.cancel();
                *view = RunView::new("stopping");
            }
        }
        return run.view().response(200);
    }
    let Some(id) = task_id(path, "/run") else {
        return bad_request();
    };
    let Ok(payload) = serde_json::from_slice::<StartRequest>(&request.body) else {
        return bad_request();
    };
    if payload.text.trim().is_empty()
        || payload.text.contains('\0')
        || payload.text.len() > MAX_WEB_DRAFT_BYTES
        || payload.expected_draft.len() > MAX_WEB_DRAFT_BYTES
    {
        return bad_request();
    }
    // Serialize admission, including the preflight awaits. Nothing is detached until
    // ownership has been installed. Dropping an HTTP request can only save a draft.
    let _mutation = state.execution.mutation.lock().await;
    let mut runs = state.execution.runs.lock().await;
    if runs.get(&id).is_some_and(|run| !run.worker.is_finished()) {
        return error(409, "run_active");
    }
    if runs
        .values()
        .filter(|run| !run.worker.is_finished())
        .count()
        >= MAX_ACTIVE_RUNS
    {
        return error(429, "run_limit");
    }
    let Ok(task) = runtime.workspace.task(id).await else {
        return error(404, "not_found");
    };
    if matches!(
        task.state,
        TaskState::Archived | TaskState::Running | TaskState::Waiting
    ) {
        return error(409, "task_unavailable");
    }
    let Ok(workspace) = runtime.workspace.workspace_for_task(&task).await else {
        return error(503, "workspace_unavailable");
    };
    if !matches!(workspace.location, WorkspaceLocation::Local { .. }) {
        return error(409, "local_tasks_only");
    }
    match runtime.workspace.direct_model_binding(id).await {
        Ok(None) => {}
        Ok(Some(_)) => return error(409, "acp_tasks_only"),
        Err(_) => return error(503, "route_unavailable"),
    }
    match runtime.workspace.task_draft(id).await {
        Ok(draft) if draft == payload.expected_draft => {}
        Ok(_) => return error(409, "draft_changed"),
        Err(_) => return error(503, "draft_unavailable"),
    }
    // Retain the exact submitted text for explicit retry even if setup fails. Do
    // not clear a later draft or invent a successful user turn before the agent.
    if runtime
        .workspace
        .save_task_draft(id, payload.text.clone())
        .await
        .is_err()
    {
        return error(503, "draft_unavailable");
    }
    if runs.len() >= MAX_RUN_RECORDS && !runs.contains_key(&id) {
        // Keep recent terminal status available across navigation and refresh.
        let retired = runs
            .iter()
            .find(|(_, run)| run.worker.is_finished())
            .map(|(id, _)| *id);
        if let Some(retired) = retired {
            runs.remove(&retired);
        } else {
            return error(429, "run_limit");
        }
    }
    let cancellation = CancellationToken::new();
    let view = Arc::new(StdMutex::new(RunView::new("running")));
    let worker = tokio::spawn(run_task(
        runtime.controller.clone(),
        id,
        payload.text,
        cancellation.clone(),
        view.clone(),
        RUN_TIMEOUT,
    ));
    runs.insert(
        id,
        Run {
            view,
            cancellation,
            worker,
        },
    );
    RunView::new("running").response(202)
}

async fn run_task(
    controller: Arc<Controller>,
    id: TaskId,
    text: String,
    cancellation: CancellationToken,
    view: Arc<StdMutex<RunView>>,
    run_timeout: Duration,
) {
    let submit = controller.submit_interruptible(id, text, cancellation.clone());
    tokio::pin!(submit);
    let outcome = tokio::select! {
        biased;
        () = cancellation.cancelled() => "cancelled",
        _ = tokio::time::sleep(run_timeout) => "timed_out",
        result = &mut submit => {
            let (state, error) = match result {
                Ok(_) => ("completed", None),
                Err(WorkspaceError::Agent(synara_agent::AgentError::AuthenticationRequired)) => ("failed", Some("authentication_required")),
                Err(_) => ("failed", Some("agent_failed")),
            };
            *view.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = RunView { state, error };
            return;
        }
    };
    cancellation.cancel();
    // Keep the submission owner alive while cancellation is delivered. Bound the
    // drain before releasing the worker; Controller shutdown remains authoritative.
    let _ = timeout(Duration::from_secs(8), controller.cancel(id)).await;
    let _ = timeout(Duration::from_secs(8), &mut submit).await;
    *view
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = RunView {
        state: outcome,
        error: (outcome == "timed_out").then_some("run_timeout"),
    };
}

impl ExecutionOwner {
    pub(super) async fn shutdown(&self, controller: &Controller) -> Result<()> {
        let mut runs = self.runs.lock().await;
        for run in runs.values() {
            run.cancellation.cancel();
        }
        // Let workers deliver scoped cancellation before marking Controller closed.
        // The final disconnect reaps every shared process while the DB lock is held.
        for (_, mut run) in runs.drain() {
            if timeout(Duration::from_secs(20), &mut run.worker)
                .await
                .is_err()
            {
                run.worker.abort();
                let _ = run.worker.await;
            }
        }
        controller
            .shutdown()
            .await
            .context("controller shutdown failed")
    }
}

#[cfg(test)]
mod tests;
