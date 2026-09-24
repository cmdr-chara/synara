use super::*;
use synara_agent::{InteractionContext, InteractionHandler};
use synara_core::{
    ConnectionId, InputField, InputFieldKind, InputValue, PermissionChoice, PermissionRequest,
    UserInputRequest,
};
use tokio::sync::oneshot;

fn context(thread_id: ThreadId) -> InteractionContext {
    InteractionContext {
        scope: InteractionScope::Session,
        thread_id,
        session_id: "not-a-public-session-id".into(),
        cancelled: CancellationToken::new(),
    }
}
fn permission(context: InteractionContext) -> (UiInteraction, oneshot::Receiver<Option<String>>) {
    let (response, rx) = oneshot::channel();
    (
        UiInteraction::Permission {
            context,
            request: PermissionRequest {
                id: "provider-reused-id".into(),
                tool_id: None,
                title: "Run the reviewed tool".into(),
                choices: vec![
                    PermissionChoice {
                        id: "once".into(),
                        label: "Once".into(),
                        kind: PermissionKind::AllowOnce,
                    },
                    PermissionChoice {
                        id: "always".into(),
                        label: "Always".into(),
                        kind: PermissionKind::AllowAlways,
                    },
                ],
            },
            response,
        },
        rx,
    )
}
fn new_inbox() -> (Inbox, mpsc::Sender<UiInteraction>) {
    let (sender, receiver) = mpsc::channel(32);
    (
        Inbox {
            receiver,
            pending: VecDeque::new(),
        },
        sender,
    )
}
#[tokio::test]
async fn approvals_are_one_shot_thread_scoped_and_never_persistent() {
    let thread = ThreadId::new();
    let (mut inbox, sender) = new_inbox();
    let (request, rx) = permission(context(thread));
    sender.send(request).await.ok().unwrap();
    inbox.refresh();
    let id = inbox.pending[0].0.clone();
    let view = public_view(&id, &inbox.pending[0].1, None, None).unwrap();
    assert_eq!(view["choices"].as_array().unwrap().len(), 1);
    assert!(!view.to_string().contains("session-id"));
    assert_eq!(
        answer(
            &mut inbox,
            ThreadId::new(),
            Reply::Permission {
                id: id.clone(),
                choice: Some("once".into())
            }
        )
        .status,
        409
    );
    assert_eq!(
        answer(
            &mut inbox,
            thread,
            Reply::Permission {
                id: id.clone(),
                choice: Some("always".into())
            }
        )
        .status,
        400
    );
    assert_eq!(
        answer(
            &mut inbox,
            thread,
            Reply::Permission {
                id: id.clone(),
                choice: Some("once".into())
            }
        )
        .status,
        200
    );
    assert_eq!(rx.await.unwrap().as_deref(), Some("once"));
    assert_eq!(
        answer(
            &mut inbox,
            thread,
            Reply::Permission {
                id,
                choice: Some("once".into())
            }
        )
        .status,
        409
    );
}
#[tokio::test]
async fn cancelled_closed_connection_and_oversized_requests_never_gain_authority() {
    let thread = ThreadId::new();
    let (mut inbox, sender) = new_inbox();
    let ctx = context(thread);
    let (request, rx) = permission(ctx.clone());
    sender.send(request).await.ok().unwrap();
    inbox.refresh();
    let id = inbox.pending[0].0.clone();
    ctx.cancelled.cancel();
    assert_eq!(
        answer(
            &mut inbox,
            thread,
            Reply::Permission {
                id,
                choice: Some("once".into())
            }
        )
        .status,
        409
    );
    assert!(rx.await.is_err());
    let mut ctx = context(thread);
    ctx.scope = InteractionScope::Connection(ConnectionId::new());
    let (request, rx) = permission(ctx);
    sender.send(request).await.ok().unwrap();
    inbox.refresh();
    assert!(rx.await.is_err());
    let (mut request, rx) = permission(context(thread));
    if let UiInteraction::Permission { request, .. } = &mut request {
        request.title = "界".repeat(MAX_ITEM_BYTES);
    }
    sender.send(request).await.ok().unwrap();
    inbox.refresh();
    assert!(rx.await.is_err());
    let (request, rx) = permission(context(thread));
    drop(rx);
    sender.send(request).await.ok().unwrap();
    inbox.refresh();
    let (response, rx) = oneshot::channel();
    sender
        .send(UiInteraction::Input {
            context: context(thread),
            request: UserInputRequest {
                id: "url".into(),
                message: "Open login".into(),
                url: Some("https://example.invalid/login".into()),
                fields: vec![],
            },
            response,
        })
        .await
        .ok()
        .unwrap();
    inbox.refresh();
    assert!(rx.await.is_err());
    assert!(inbox.pending.is_empty());
}
#[tokio::test]
async fn questions_keep_invalid_answers_pending_and_validate_original_schema() {
    let (mut inbox, sender) = new_inbox();
    let thread = ThreadId::new();
    let request = UserInputRequest {
        id: "question".into(),
        message: "Choose a count".into(),
        url: None,
        fields: vec![InputField {
            id: "count".into(),
            label: "Count".into(),
            required: true,
            kind: InputFieldKind::Number {
                integer: true,
                minimum: Some(1.),
                maximum: Some(3.),
            },
        }],
    };
    let (response, rx) = oneshot::channel();
    sender
        .send(UiInteraction::Input {
            context: context(thread),
            request,
            response,
        })
        .await
        .ok()
        .unwrap();
    inbox.refresh();
    let id = inbox.pending[0].0.clone();
    assert_eq!(
        answer(
            &mut inbox,
            thread,
            Reply::Input {
                id: id.clone(),
                response: UserInputResponse::Accept {
                    values: Default::default()
                }
            }
        )
        .status,
        400
    );
    let values = std::collections::BTreeMap::from([("count".into(), InputValue::Number(2.))]);
    assert_eq!(
        answer(
            &mut inbox,
            thread,
            Reply::Input {
                id,
                response: UserInputResponse::Accept {
                    values: values.clone()
                }
            }
        )
        .status,
        200
    );
    assert_eq!(rx.await.unwrap(), UserInputResponse::Accept { values });
}
#[tokio::test]
async fn broker_receipts_survive_polls_but_never_restart_or_shutdown() {
    let owner = Owner::default();
    let broker = owner.broker.clone();
    let thread = ThreadId::new();
    let (UiInteraction::Permission { request, .. }, _) = permission(context(thread)) else {
        unreachable!()
    };
    let pending =
        tokio::spawn(async move { broker.permission(context(thread), request).await.unwrap() });
    let id = timeout(Duration::from_secs(1), async {
        loop {
            let mut inbox = owner.inbox.lock().await;
            inbox.refresh();
            if let Some((id, _)) = inbox.pending.front() {
                break id.clone();
            }
            drop(inbox);
            tokio::task::yield_now().await;
        }
    })
    .await
    .ok()
    .unwrap();
    let mut inbox = owner.inbox.lock().await;
    inbox.refresh();
    assert_eq!(inbox.pending[0].0, id);
    drop(inbox);
    owner.close().await;
    assert_eq!(pending.await.unwrap(), None);
    let (mut next, sender) = new_inbox();
    let (request, _rx) = permission(context(thread));
    sender.send(request).await.ok().unwrap();
    next.refresh();
    assert_ne!(next.pending[0].0, id);
}
#[tokio::test]
async fn queue_and_serialized_views_are_bounded() {
    let (mut inbox, sender) = new_inbox();
    let thread = ThreadId::new();
    let mut receivers = Vec::new();
    for _ in 0..32 {
        let (request, rx) = permission(context(thread));
        receivers.push(rx);
        sender.send(request).await.ok().unwrap();
    }
    inbox.refresh();
    assert_eq!(inbox.pending.len(), MAX_PENDING);
    let (request, rx) = permission(context(thread));
    sender.send(request).await.ok().unwrap();
    inbox.refresh();
    assert!(rx.await.is_err());
    let views: Vec<_> = inbox
        .pending
        .iter()
        .take(MAX_VISIBLE)
        .filter_map(|(id, item)| public_view(id, item, None, None))
        .collect();
    assert!(serde_json::to_vec(&views).unwrap().len() < MAX_RESPONSE_BYTES);
    drop(receivers);
    inbox.refresh();
    assert!(inbox.pending.is_empty());
}
#[tokio::test]
async fn api_keeps_auth_origin_task_and_receipt_boundaries() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = WorkspaceService::memory().unwrap();
    let project = workspace
        .add_local_workspace(directory.path().into())
        .await
        .unwrap();
    let agent = workspace.profiles().await.unwrap()[0].id.clone();
    let task = workspace
        .create_task(project.id, "Approval fixture".into(), agent)
        .await
        .unwrap();
    let state = AppState::new("only-a-local-fixture-token-at-least-32-bytes").unwrap();
    state.install_runtime(workspace, None).await;
    let path = format!("/api/tasks/{}/interactions", task.id);
    let mut request = Request {
        method: "GET".into(),
        target: path.clone(),
        version: "HTTP/1.1".into(),
        headers: vec![("host".into(), "localhost:18100".into())],
        body: vec![],
    };
    assert_eq!(dispatch(&request, &state, 18100).await.status, 401);
    request.headers.push((
        "authorization".into(),
        "Bearer only-a-local-fixture-token-at-least-32-bytes".into(),
    ));
    request
        .headers
        .push(("origin".into(), "https://foreign.invalid".into()));
    assert_eq!(dispatch(&request, &state, 18100).await.status, 421);
    request.headers.pop();
    let broker = state.interactions.broker.clone();
    let thread = task.thread_id;
    let (
        UiInteraction::Permission {
            request: permission_request,
            ..
        },
        _,
    ) = permission(context(thread))
    else {
        unreachable!()
    };
    let pending = tokio::spawn(async move {
        broker
            .permission(context(thread), permission_request)
            .await
            .unwrap()
    });
    let id = timeout(Duration::from_secs(1), async {
        loop {
            let response = dispatch(&request, &state, 18100).await;
            let data: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
            if let Some(id) = data["items"][0]["id"].as_str() {
                break id.to_owned();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    request.method = "POST".into();
    request.body =
        serde_json::to_vec(&serde_json::json!({"action":"permission","id":id,"choice":"once"}))
            .unwrap();
    request
        .headers
        .push(("content-length".into(), request.body.len().to_string()));
    assert_eq!(dispatch(&request, &state, 18100).await.status, 415);
    request
        .headers
        .push(("content-type".into(), "application/json".into()));
    assert_eq!(dispatch(&request, &state, 18100).await.status, 200);
    assert_eq!(pending.await.unwrap().as_deref(), Some("once"));
    assert_eq!(dispatch(&request, &state, 18100).await.status, 409);
    state.interactions.close().await;
}
