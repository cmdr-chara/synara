use std::{sync::Arc, time::Duration};
use synara_acp::AcpBackend;
use synara_agent::{DenyInteractions, InteractionBroker, UiInteraction};
use synara_core::*;
use synara_workspace::{AgentProfile, Controller, WorkspaceService};

async fn workspace(profile: &str) -> (WorkspaceService, Task, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let workspace = WorkspaceService::open(root.path().join("synara.sqlite3"))
        .await
        .unwrap();
    let project = workspace
        .add_local_workspace(root.path().into())
        .await
        .unwrap();
    workspace
        .save_profiles(vec![AgentProfile {
            id: profile.into(),
            name: profile.into(),
            command: env!("CARGO_BIN_EXE_synara-acp-fixture").into(),
            args: vec!["--integration-fixture".into(), profile.into()],
            inherit_env: vec![],
        }])
        .await
        .unwrap();
    let task = workspace
        .create_task(project.id, "Controller test".into(), profile.into())
        .await
        .unwrap();
    (workspace, task, root)
}
#[tokio::test]
async fn two_agent_profiles_execute_through_the_controller_and_persist_history() {
    for profile in ["alpha", "beta"] {
        let (workspace, task, root) = workspace(profile).await;
        let controller = Controller::new(
            workspace.clone(),
            Arc::new(AcpBackend::default()),
            Arc::new(DenyInteractions),
        );
        controller.connect(task.id).await.unwrap();
        controller.submit(task.id, "hello".into()).await.unwrap();
        let first = controller.details(task.id).await.unwrap().unwrap();
        controller.submit(task.id, "again".into()).await.unwrap();
        let second = controller.details(task.id).await.unwrap().unwrap();
        assert_eq!(first.connection.id, second.connection.id);
        assert_eq!(first.session_id, second.session_id);
        controller.shutdown().await.unwrap();
        let reloaded = WorkspaceService::open(root.path().join("synara.sqlite3"))
            .await
            .unwrap();
        let thread = reloaded.thread(task.thread_id).await.unwrap();
        assert_eq!(thread.state, TaskState::Completed);
        assert_eq!(
            thread
                .messages
                .iter()
                .filter(|m| m.role == Role::User)
                .count(),
            2
        );
        assert!(
            thread
                .messages
                .iter()
                .any(|m| m.role == Role::Assistant && m.text == format!("Hello from {profile}"))
        );
    }
}
#[tokio::test]
async fn permissions_are_visible_and_wait_for_explicit_ui_response() {
    let (workspace, task, _root) = workspace("alpha").await;
    let (broker, mut receiver) = InteractionBroker::new();
    let controller = Arc::new(Controller::new(
        workspace.clone(),
        Arc::new(AcpBackend::default()),
        Arc::new(broker),
    ));
    let operation = {
        let controller = controller.clone();
        tokio::spawn(async move { controller.submit(task.id, "permission".into()).await })
    };
    let interaction = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    let thread = workspace.thread(task.thread_id).await.unwrap();
    assert_eq!(thread.state, TaskState::Waiting);
    assert_eq!(thread.permissions.len(), 1);
    if let UiInteraction::Permission {
        request, response, ..
    } = interaction
    {
        response.send(Some(request.choices[0].id.clone())).unwrap();
    } else {
        panic!("expected permission");
    }
    operation.await.unwrap().unwrap();
    assert!(
        workspace
            .thread(task.thread_id)
            .await
            .unwrap()
            .permissions
            .is_empty()
    );
    controller.shutdown().await.unwrap();
}
#[tokio::test]
async fn controller_cancellation_remains_available_during_a_prompt() {
    let (workspace, task, _root) = workspace("alpha").await;
    let controller = Arc::new(Controller::new(
        workspace.clone(),
        Arc::new(AcpBackend::default()),
        Arc::new(DenyInteractions),
    ));
    let mut events = workspace.subscribe();
    let operation = {
        let controller = controller.clone();
        tokio::spawn(async move { controller.submit(task.id, "hold".into()).await })
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(
                events.recv().await.unwrap().event,
                ThreadEvent::TextDelta {
                    role: Role::Assistant,
                    ..
                }
            ) {
                break;
            }
        }
    })
    .await
    .unwrap();
    controller.cancel(task.id).await.unwrap();
    assert_eq!(operation.await.unwrap().unwrap(), "cancelled");
    controller.shutdown().await.unwrap();
}
#[tokio::test]
async fn auth_required_connection_can_be_authenticated_before_creating_a_session() {
    let (workspace, task, _root) = workspace("auth").await;
    let controller = Controller::new(
        workspace,
        Arc::new(AcpBackend::default()),
        Arc::new(DenyInteractions),
    );
    assert!(controller.connect(task.id).await.is_err());
    assert!(
        !controller
            .details(task.id)
            .await
            .unwrap()
            .unwrap()
            .connection
            .authentication
            .is_empty()
    );
    controller
        .authenticate(task.id, "test-login".into())
        .await
        .unwrap();
    controller.submit(task.id, "hello".into()).await.unwrap();
    controller.shutdown().await.unwrap();
}
