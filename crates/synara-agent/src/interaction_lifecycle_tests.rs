use super::*;
use synara_core::{PermissionChoice, PermissionKind};

fn scope() -> InteractionContext {
    InteractionContext {
        thread_id: ThreadId::new(),
        session_id: "session".into(),
        cancelled: CancellationToken::new(),
    }
}

fn permission() -> PermissionRequest {
    PermissionRequest {
        id: "permission".into(),
        tool_id: None,
        title: "Write one file".into(),
        choices: vec![PermissionChoice {
            id: "once".into(),
            label: "Allow once".into(),
            kind: PermissionKind::AllowOnce,
        }],
    }
}

fn question() -> UserInputRequest {
    UserInputRequest {
        id: "question".into(),
        message: "Continue in the browser?".into(),
        url: Some("https://example.com/login".into()),
        fields: vec![],
    }
}

#[tokio::test]
async fn cancelled_interactions_are_not_enqueued() {
    let (broker, mut ui) = InteractionBroker::new();
    let context = scope();
    context.cancelled.cancel();
    assert_eq!(
        broker
            .permission(context.clone(), permission())
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        broker.input(context, question()).await.unwrap(),
        UserInputResponse::Cancel
    );
    assert!(ui.try_recv().is_err());
}

#[tokio::test]
async fn cancellation_dominates_a_ready_permission_response() {
    let (broker, mut ui) = InteractionBroker::new();
    let context = scope();
    let cancelled = context.cancelled.clone();
    let responder = async {
        let Some(UiInteraction::Permission { response, .. }) = ui.recv().await else {
            panic!("permission expected");
        };
        response.send(Some("once".into())).unwrap();
        cancelled.cancel();
    };
    let (result, ()) = tokio::join!(broker.permission(context, permission()), responder);
    assert_eq!(result.unwrap(), None);
}

#[tokio::test]
async fn cancellation_dominates_a_ready_form_response() {
    let (broker, mut ui) = InteractionBroker::new();
    let context = scope();
    let cancelled = context.cancelled.clone();
    let responder = async {
        let Some(UiInteraction::Input { response, .. }) = ui.recv().await else {
            panic!("question expected");
        };
        response
            .send(UserInputResponse::Accept {
                values: BTreeMap::new(),
            })
            .unwrap();
        cancelled.cancel();
    };
    let (result, ()) = tokio::join!(broker.input(context, question()), responder);
    assert_eq!(result.unwrap(), UserInputResponse::Cancel);
}

#[tokio::test]
async fn overlapping_permissions_have_independent_lifetimes() {
    let (broker, mut ui) = InteractionBroker::new();
    let one = scope();
    let two = scope();
    let cancelled = one.cancelled.clone();
    let first_thread = one.thread_id;
    let responder = async {
        let mut stale = None;
        for _ in 0..2 {
            let Some(UiInteraction::Permission {
                context, response, ..
            }) = ui.recv().await
            else {
                panic!("permission expected");
            };
            if context.thread_id == first_thread {
                stale = Some(response);
            } else {
                response.send(Some("once".into())).unwrap();
            }
        }
        cancelled.cancel();
        stale.unwrap()
    };
    let (one, two, stale) = tokio::join!(
        broker.permission(one, permission()),
        broker.permission(two, permission()),
        responder
    );
    assert_eq!(one.unwrap(), None);
    assert_eq!(two.unwrap().as_deref(), Some("once"));
    assert!(stale.send(Some("once".into())).is_err());
}

#[tokio::test]
async fn expired_permission_and_question_close_the_response_channels() {
    let (mut broker, mut ui) = InteractionBroker::new();
    broker.timeout = Duration::from_millis(10);
    assert_eq!(
        broker.permission(scope(), permission()).await.unwrap(),
        None
    );
    let Some(UiInteraction::Permission { response, .. }) = ui.recv().await else {
        panic!("permission expected");
    };
    assert!(response.send(Some("once".into())).is_err());
    assert_eq!(
        broker.input(scope(), question()).await.unwrap(),
        UserInputResponse::Cancel
    );
    let Some(UiInteraction::Input { response, .. }) = ui.recv().await else {
        panic!("question expected");
    };
    assert!(
        response
            .send(UserInputResponse::Accept {
                values: BTreeMap::new()
            })
            .is_err()
    );
}
