//! Explicit task-owned connection and authentication jobs. These never submit a prompt.
use super::*;
use std::collections::HashMap;
use synara_core::{ConnectionId, ConnectionState, EventId, TaskId, TaskState};
use synara_workspace::WorkspaceError;
use tokio::sync::Mutex;

struct Operation {
    id: EventId,
    connection: Option<ConnectionId>,
    follows_connection: bool,
    result: Arc<StdMutex<Option<&'static str>>>,
    worker: JoinHandle<()>,
}
#[derive(Default)]
pub(super) struct Owner {
    operations: Mutex<HashMap<TaskId, Operation>>,
}
impl Owner {
    pub async fn busy(&self, task: TaskId) -> bool {
        self.operations
            .lock()
            .await
            .get(&task)
            .is_some_and(|op| !op.worker.is_finished())
    }
    pub async fn connection(&self, task: TaskId, current: ConnectionId) -> Option<ConnectionId> {
        let mut operations = self.operations.lock().await;
        if operations.iter().any(|(owner, op)| {
            *owner != task && op.connection == Some(current) && !op.worker.is_finished()
        }) {
            return None;
        }
        let operation = operations
            .get_mut(&task)
            .filter(|op| !op.worker.is_finished())?;
        if !operation.follows_connection && operation.connection != Some(current) {
            return None;
        }
        operation.connection = Some(current);
        Some(current)
    }
    pub async fn shutdown(&self) {
        let mut operations = self.operations.lock().await;
        for (_, operation) in operations.drain() {
            operation.worker.abort();
            let _ = operation.worker.await;
        }
    }
}

fn error(status: u16, code: &'static str) -> Response {
    Response::json(
        status,
        serde_json::to_vec(&serde_json::json!({"error":code})).unwrap(),
    )
}
fn task_id(path: &str) -> Option<TaskId> {
    serde_json::from_value(serde_json::Value::String(
        path.strip_prefix("/api/tasks/")?
            .strip_suffix("/provider")?
            .to_owned(),
    ))
    .ok()
}
async fn stamp(runtime: &RuntimeServices, task: &Task) -> Option<String> {
    let profile = runtime
        .workspace
        .profiles()
        .await
        .ok()?
        .into_iter()
        .find(|p| p.id == task.agent_id)?;
    let workspace = runtime.workspace.workspace_for_task(task).await.ok()?;
    let remote = runtime.workspace.ssh_profile(workspace.id).await.ok()?;
    let bytes = serde_json::to_vec(&(
        task.id,
        task.thread_id,
        &task.agent_id,
        &task.working_directory,
        profile,
        workspace,
        remote,
    ))
    .ok()?;
    Some(format!("{:x}", Sha256::digest(bytes)))
}
pub(super) async fn get(path: &str, state: &AppState) -> Response {
    let Some(id) = task_id(path) else {
        return bad_request();
    };
    let runtime = state.runtime.read().await;
    let Some(runtime) = runtime.as_ref() else {
        return health_response(Lifecycle::Starting);
    };
    let Ok(task) = runtime.workspace.task(id).await else {
        return error(404, "not_found");
    };
    match runtime.workspace.direct_model_binding(id).await {
        Ok(Some(_)) => return Response::json(200, br#"{"direct":true,"busy":false}"#.to_vec()),
        Ok(None) => {}
        Err(_) => return error(503, "provider_unavailable"),
    }
    let Some(stamp) = stamp(runtime, &task).await else {
        return error(503, "provider_unavailable");
    };
    let details = match runtime.controller.details(id).await {
        Ok(value) => value,
        Err(_) => return error(503, "provider_unavailable"),
    };
    let operations = state.providers.operations.lock().await;
    let operation = operations.get(&id);
    let busy = operation.is_some_and(|op| !op.worker.is_finished());
    let last_error = operation.and_then(|op| {
        *op.result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    });
    let methods: Vec<_> = details.as_ref().map(|details| details.connection.authentication.iter()
        .filter(|method| !method.id.is_empty() && method.id.len() <= 256)
        .take(32).map(|method| serde_json::json!({
            "id": method.id, "name": method.name.chars().take(200).collect::<String>(),
            "description": method.description.as_deref().map(|value| value.chars().take(500).collect::<String>())
        })).collect()).unwrap_or_default();
    Response::json(200, serde_json::to_vec(&serde_json::json!({
        "agent_id": task.agent_id, "stamp":stamp, "direct":false, "busy":busy, "error":last_error,
        "operation_id": operation.map(|op| op.id),
        "connection_id": details.as_ref().map(|d| d.connection.id),
        "state": details.as_ref().map_or(ConnectionState::Disconnected, |d| d.connection.state),
        "session_ready": details.as_ref().is_some_and(|d| d.session_id.is_some()),
        "methods": methods,
    })).unwrap())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    action: String,
    expected_stamp: String,
    expected_connection: Option<ConnectionId>,
    expected_operation: Option<EventId>,
    method: Option<String>,
}
pub(super) async fn post(
    path: &str,
    request: &Request,
    state: &AppState,
    runtime: &RuntimeServices,
) -> Response {
    let Some(id) = task_id(path) else {
        return bad_request();
    };
    let Ok(action) = serde_json::from_slice::<Action>(&request.body) else {
        return bad_request();
    };
    if !matches!(
        action.action.as_str(),
        "connect" | "authenticate" | "reconnect" | "cancel"
    ) || action.expected_stamp.len() != 64
        || action.method.as_ref().is_some_and(|v| v.len() > 256)
    {
        return bad_request();
    }
    let _mutation = state.execution.mutation.lock().await;
    let Ok(task) = runtime.workspace.task(id).await else {
        return error(404, "not_found");
    };
    if stamp(runtime, &task).await.as_deref() != Some(action.expected_stamp.as_str()) {
        return error(409, "provider_changed");
    }
    let mut operations = state.providers.operations.lock().await;
    if action.action == "cancel" {
        if operations.get(&id).map(|op| op.id) != action.expected_operation {
            return error(409, "provider_changed");
        }
        if let Some(operation) = operations.remove(&id) {
            operation.worker.abort();
            let _ = operation.worker.await;
        }
        return Response::json(200, br#"{"cancelled":true}"#.to_vec());
    }
    if operations
        .get(&id)
        .is_some_and(|op| !op.worker.is_finished())
    {
        return error(409, "provider_busy");
    }
    if matches!(
        task.state,
        TaskState::Running | TaskState::Waiting | TaskState::Archived
    ) || state.execution.active(id).await
    {
        return error(409, "task_unavailable");
    }
    match runtime.workspace.direct_model_binding(id).await {
        Ok(None) => {}
        _ => return error(409, "agent_route_required"),
    }
    let details = match runtime.controller.details(id).await {
        Ok(value) => value,
        Err(_) => return error(503, "provider_unavailable"),
    };
    let connection = details.as_ref().map(|d| d.connection.id);
    if action.expected_connection != connection {
        return error(409, "provider_changed");
    }
    if action.action == "authenticate" {
        let Some(details) = details.as_ref() else {
            return error(409, "connect_first");
        };
        if details.connection.state != ConnectionState::AuthenticationRequired
            || !details
                .connection
                .authentication
                .iter()
                .any(|method| Some(&method.id) == action.method.as_ref())
        {
            return error(400, "authentication_method_unavailable");
        }
        // A shared connection has one authentication owner across all tasks.
        if operations
            .values()
            .any(|op| op.connection == connection && !op.worker.is_finished())
        {
            return error(409, "provider_busy");
        }
    }
    operations.retain(|_, op| !op.worker.is_finished());
    if operations.len() >= 8 {
        return error(429, "provider_limit");
    }
    let result = Arc::new(StdMutex::new(None));
    // Connect/reconnect can acquire a new connection. Authentication is pinned
    // to the already reviewed one. Presentation still rechecks Controller's
    // exact current connection for this task before exposing any callback.
    let interaction_connection = (action.action == "authenticate")
        .then_some(connection)
        .flatten();
    let follows_connection = action.action != "authenticate";
    let worker_result = result.clone();
    let controller = runtime.controller.clone();
    let worker = tokio::spawn(async move {
        let operation = async {
            match action.action.as_str() {
                "authenticate" => {
                    controller
                        .authenticate(id, action.method.unwrap_or_default())
                        .await
                }
                "reconnect" => controller.restart(id).await,
                _ => controller.connect(id).await,
            }
        };
        let code = match timeout(Duration::from_secs(300), operation).await {
            Ok(Ok(_)) => None,
            Ok(Err(WorkspaceError::Agent(synara_agent::AgentError::AuthenticationRequired))) => {
                Some("authentication_required")
            }
            Ok(Err(_)) => Some("provider_failed"),
            Err(_) => Some("provider_timeout"),
        };
        *worker_result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = code;
    });
    operations.insert(
        id,
        Operation {
            id: EventId::new(),
            connection: interaction_connection,
            follows_connection,
            result,
            worker,
        },
    );
    Response::json(202, br#"{"started":true}"#.to_vec())
}
