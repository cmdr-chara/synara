//! Ephemeral, task-scoped web presentation of the existing agent interaction broker.
//! Login requests retain the exact active web authentication connection owner.
use super::*;
use std::collections::VecDeque;
use synara_agent::{
    InteractionBroker, InteractionScope, UiInteraction, validate_input, validate_input_request,
};
use synara_core::{
    ConnectionId, EventId, PermissionKind, TaskId, Thread, ThreadId, Tool, ToolOutput,
    UserInputResponse,
};
use tokio::sync::{Mutex, mpsc};

const MAX_PENDING: usize = 32;
const MAX_VISIBLE: usize = 2;
const MAX_ITEM_BYTES: usize = 24 * 1024;

pub(super) struct Owner {
    pub broker: Arc<InteractionBroker>,
    inbox: Mutex<Inbox>,
}
struct Inbox {
    receiver: mpsc::Receiver<UiInteraction>,
    pending: VecDeque<(String, UiInteraction)>,
}
impl Default for Owner {
    fn default() -> Self {
        let (broker, receiver) = InteractionBroker::new();
        Self {
            broker: Arc::new(broker),
            inbox: Mutex::new(Inbox {
                receiver,
                pending: VecDeque::new(),
            }),
        }
    }
}
impl Owner {
    pub async fn close(&self) {
        let mut inbox = self.inbox.lock().await;
        inbox.receiver.close();
        inbox.pending.clear();
        while inbox.receiver.try_recv().is_ok() {}
    }
}
impl Inbox {
    fn refresh(&mut self) {
        self.pending
            .retain(|(_, interaction)| interaction.is_active());
        // Broker admission and this queue are independently bounded. Overflow drops
        // the response sender and therefore cancels rather than implicitly approving.
        for _ in 0..MAX_PENDING {
            let Ok(interaction) = self.receiver.try_recv() else {
                break;
            };
            if self.pending.len() < MAX_PENDING && interaction.is_active() {
                // Fresh UUID receipts never reuse provider IDs, even after restart.
                let id = EventId::new().to_string();
                if public_view(&id, &interaction, None, None).is_some() {
                    self.pending.push_back((id, interaction));
                }
            }
        }
    }
}
fn tool_context(tool: &Tool, thread: &Thread) -> serde_json::Value {
    let diffs: Vec<_> = tool
        .output
        .iter()
        .filter_map(|output| match output {
            ToolOutput::Diff { path, before, after } => Some(serde_json::json!({
                "path": path.chars().take(500).collect::<String>(),
                "before": before.as_deref().map(|value| value.chars().take(2000).collect::<String>()),
                "after": after.as_deref().map(|value| value.chars().take(2000).collect::<String>()),
                "truncated": path.chars().count() > 500
                    || before.as_deref().is_some_and(|value| value.chars().count() > 2000)
                    || after.as_deref().is_some_and(|value| value.chars().count() > 2000),
            })),
            _ => None,
        })
        .take(2)
        .collect();
    let details: Vec<_> = tool
        .output
        .iter()
        .filter_map(|output| match output {
            ToolOutput::Text { text } => Some(serde_json::json!({
                "kind": "text", "text": text.chars().take(3000).collect::<String>(),
                "truncated": text.chars().count() > 3000,
            })),
            ToolOutput::Terminal { id } => thread.terminals.get(id).map(|record| {
                serde_json::json!({
                    "kind": "terminal", "text": record.text.chars().take(3000).collect::<String>(),
                    "truncated": record.truncated || record.text.chars().count() > 3000,
                    "exit_code": record.exit_code,
                })
            }),
            _ => None,
        })
        .take(2)
        .collect();
    serde_json::json!({
        "title": tool.title.chars().take(2000).collect::<String>(),
        "kind": tool.kind.as_deref().map(|value| value.chars().take(100).collect::<String>()),
        "status": tool.status,
        "input": tool.input.as_ref().map(|input| serde_json::json!({
            "text":input.text.chars().take(3000).collect::<String>(),
            "truncated":input.truncated || input.text.chars().count() > 3000,
        })),
        "diffs": diffs,
        "diffs_omitted": tool.output.iter().filter(|output| matches!(output, ToolOutput::Diff { .. })).count().saturating_sub(2),
        "details": details,
    })
}
fn public_view(
    id: &str,
    interaction: &UiInteraction,
    tool: Option<&Tool>,
    thread: Option<&Thread>,
) -> Option<serde_json::Value> {
    let value = match interaction {
        UiInteraction::Permission { request, .. } => {
            if interaction.context().scope != InteractionScope::Session {
                return None;
            }
            if request.choices.len() > 64 || request.title.is_empty() || request.title.len() > 8192
            {
                return None;
            }
            if request
                .choices
                .iter()
                .any(|choice| choice.id.len() > 128 || choice.label.len() > 1024)
            {
                return None;
            }
            let choices: Vec<_> = request
                .choices
                .iter()
                .filter(|choice| {
                    matches!(
                        choice.kind,
                        PermissionKind::AllowOnce | PermissionKind::DenyOnce
                    )
                })
                .collect();
            if choices.is_empty()
                || choices.iter().any(|choice| {
                    choice.id.is_empty() || choice.id.len() > 128 || choice.label.len() > 1024
                })
            {
                return None;
            }
            let unique: std::collections::HashSet<_> =
                request.choices.iter().map(|choice| &choice.id).collect();
            if unique.len() != request.choices.len() {
                return None;
            }
            let context = tool
                .zip(thread)
                .map(|(tool, thread)| tool_context(tool, thread));
            let exact = tool.map(|tool| serde_json::json!({"tool":tool,"shown":context}));
            let review = format!("{:x}", Sha256::digest(serde_json::to_vec(&exact).ok()?));
            serde_json::json!({"id":id,"kind":"permission","title":request.title,"choices":choices,
                "tool":context,"review":review,"tool_id":request.tool_id.as_deref().map(|value| value.chars().take(128).collect::<String>())})
        }
        UiInteraction::Input { request, .. } => {
            if validate_input_request(request).is_err() {
                return None;
            }
            let connection = matches!(interaction.context().scope, InteractionScope::Connection(_));
            serde_json::json!({"id":id,"kind":if request.url.is_some() {"url"} else {"input"},
                "title":request.message,"fields":request.fields,"url":request.url,
                "connection_scoped":connection,"persist_draft":!connection && request.url.is_none()})
        }
    };
    (serde_json::to_vec(&value).ok()?.len() <= MAX_ITEM_BYTES).then_some(value)
}
fn belongs(
    interaction: &UiInteraction,
    thread: ThreadId,
    connection: Option<ConnectionId>,
) -> bool {
    match interaction.context().scope {
        InteractionScope::Session => interaction.context().thread_id == thread,
        InteractionScope::Connection(id) => {
            connection == Some(id) && matches!(interaction, UiInteraction::Input { .. })
        }
    }
}
async fn active_connection(
    state: &AppState,
    runtime: &RuntimeServices,
    task: TaskId,
) -> Option<ConnectionId> {
    let current = runtime.controller.details(task).await.ok()??;
    state
        .providers
        .connection(task, current.connection.id)
        .await
}
fn task_id(path: &str) -> Option<TaskId> {
    let id = path
        .strip_prefix("/api/tasks/")?
        .strip_suffix("/interactions")?;
    serde_json::from_value(serde_json::Value::String(id.to_owned())).ok()
}
fn error(status: u16, error: &'static str) -> Response {
    Response::json(
        status,
        serde_json::to_vec(&serde_json::json!({"error":error})).unwrap(),
    )
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
    let Ok(thread) = runtime.workspace.thread(task.thread_id).await else {
        return error(503, "interactions_unavailable");
    };
    let connection = active_connection(state, runtime, task.id).await;
    let mut inbox = state.interactions.inbox.lock().await;
    inbox.refresh();
    let pending: Vec<_> = inbox
        .pending
        .iter()
        .filter(|(_, interaction)| belongs(interaction, task.thread_id, connection))
        .collect();
    let items: Vec<_> = pending
        .iter()
        .take(MAX_VISIBLE)
        .filter_map(|(id, interaction)| {
            let tool = match interaction {
                UiInteraction::Permission { request, .. } => {
                    request.tool_id.as_ref().and_then(|id| thread.tools.get(id))
                }
                _ => None,
            };
            public_view(id, interaction, tool, Some(&thread))
        })
        .collect();
    let body =
        serde_json::to_vec(&serde_json::json!({"items":items,"more":pending.len() > MAX_VISIBLE}))
            .unwrap();
    if body.len() > MAX_RESPONSE_BYTES {
        return error(500, "interactions_unavailable");
    }
    Response::json(200, body)
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Reply {
    Permission {
        id: String,
        choice: Option<String>,
        #[serde(default)]
        review: Option<String>,
    },
    Input {
        id: String,
        response: UserInputResponse,
    },
}
impl Reply {
    fn id(&self) -> &str {
        match self {
            Self::Permission { id, .. } | Self::Input { id, .. } => id,
        }
    }
}
#[cfg(test)]
fn answer(inbox: &mut Inbox, thread: ThreadId, reply: Reply) -> Response {
    answer_owned(inbox, thread, None, reply)
}
fn answer_owned(
    inbox: &mut Inbox,
    thread: ThreadId,
    connection: Option<ConnectionId>,
    reply: Reply,
) -> Response {
    inbox.refresh();
    let Some(index) = inbox.pending.iter().position(|(id, interaction)| {
        id == reply.id() && belongs(interaction, thread, connection) && interaction.is_active()
    }) else {
        return error(409, "interaction_expired");
    };
    // Validate against the original broker request before consuming its one-shot sender.
    match (&inbox.pending[index].1, &reply) {
        (UiInteraction::Permission { request, .. }, Reply::Permission { choice, .. }) => {
            if choice.as_ref().is_some_and(|id| {
                !request.choices.iter().any(|option| {
                    option.id == *id
                        && matches!(
                            option.kind,
                            PermissionKind::AllowOnce | PermissionKind::DenyOnce
                        )
                })
            }) {
                return error(400, "invalid_interaction_reply");
            }
        }
        (UiInteraction::Input { request, .. }, Reply::Input { response, .. }) => {
            if let UserInputResponse::Accept { values } = response
                && validate_input(request, values).is_err()
            {
                return error(400, "invalid_interaction_reply");
            }
        }
        _ => return error(400, "invalid_interaction_reply"),
    }
    let (_, interaction) = inbox.pending.remove(index).unwrap();
    let delivered = match (interaction, reply) {
        (UiInteraction::Permission { response, .. }, Reply::Permission { choice, .. }) => {
            response.send(choice).is_ok()
        }
        (
            UiInteraction::Input { response, .. },
            Reply::Input {
                response: value, ..
            },
        ) => response.send(value).is_ok(),
        _ => unreachable!("response kind checked before removal"),
    };
    if delivered {
        Response::json(200, br#"{"submitted":true}"#.to_vec())
    } else {
        error(409, "interaction_expired")
    }
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
    let Ok(reply) = serde_json::from_slice::<Reply>(&request.body) else {
        return bad_request();
    };
    if reply.id().len() > 64 {
        return bad_request();
    }
    let Ok(task) = runtime.workspace.task(id).await else {
        return error(404, "not_found");
    };
    let connection = active_connection(state, runtime, task.id).await;
    let thread = match runtime.workspace.thread(task.thread_id).await {
        Ok(thread) => thread,
        Err(_) => return error(503, "interactions_unavailable"),
    };
    let mut inbox = state.interactions.inbox.lock().await;
    inbox.refresh();
    if let Reply::Permission {
        id,
        choice: Some(choice),
        review,
    } = &reply
        && let Some((_, interaction @ UiInteraction::Permission { request, .. })) = inbox
            .pending
            .iter()
            .find(|(receipt, item)| receipt == id && belongs(item, task.thread_id, connection))
        && request
            .choices
            .iter()
            .any(|option| option.id == *choice && option.kind == PermissionKind::AllowOnce)
    {
        let tool = request.tool_id.as_ref().and_then(|id| thread.tools.get(id));
        let view = public_view(id, interaction, tool, Some(&thread));
        if view.as_ref().and_then(|value| value["review"].as_str()) != review.as_deref() {
            return error(409, "tool_context_changed");
        }
    }
    answer_owned(&mut inbox, task.thread_id, connection, reply)
}
#[cfg(test)]
mod tests;
