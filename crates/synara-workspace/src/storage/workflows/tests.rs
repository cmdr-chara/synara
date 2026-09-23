use super::*;
use crate::*;
use synara_core::*;

async fn setup() -> (tempfile::TempDir, WorkspaceService, Task) {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::open(root.path().join("workspace.sqlite3"))
        .await
        .unwrap();
    let project = service
        .add_local_workspace(root.path().into())
        .await
        .unwrap();
    let task = service
        .create_task(
            project.id,
            "Parent".into(),
            default_profiles()[0].id.clone(),
        )
        .await
        .unwrap();
    (root, service, task)
}
#[test]
fn workflow_graph_is_bounded_acyclic_and_explicit() {
    let mut spec = WorkflowSpec::example("fixture");
    spec.validate().unwrap();
    spec.steps[0].depends_on = vec![1];
    assert!(spec.validate().is_err());
    spec.steps[0].depends_on.clear();
    spec.steps[1].depends_on = vec![0, 0];
    assert!(spec.validate().is_err());
    spec.steps[1].depends_on = vec![0];
    spec.concurrency = 0;
    assert!(spec.validate().is_err());
    spec.concurrency = 5;
    assert!(spec.validate().is_err());
    spec.concurrency = 4;
    spec.steps[0].instruction = "a".repeat(16 * 1024 + 1);
    assert!(spec.validate().is_err());
    assert!(
        serde_json::from_str::<WorkflowSpec>(r#"{"title":"bad","steps":[],"shell":"execute"}"#)
            .is_err()
    );
    assert!(
        serde_json::from_str::<WorkflowStepSpec>(
            r#"{"title":"bad","agent_id":"a","instruction":"i","depends_on":[-1]}"#
        )
        .is_err()
    );
}
#[tokio::test]
async fn creation_is_atomic_unsent_and_ownership_survives_reopen() {
    let (root, service, parent) = setup().await;
    service
        .save_task_draft(parent.id, "Original unsent parent draft".into())
        .await
        .unwrap();
    let value = service
        .create_workflow(parent.id, WorkflowSpec::example(&parent.agent_id))
        .await
        .unwrap();
    assert_eq!(value.steps.len(), 2);
    assert_eq!(value.phase, WorkflowPhase::Ready);
    assert!(value.run.is_none());
    for step in &value.steps {
        let child = service.task(step.task).await.unwrap();
        assert_eq!(child.working_directory, parent.working_directory);
        assert_eq!(child.project_id, parent.project_id);
        assert!(child.updated_at_ms >= parent.updated_at_ms);
        assert_ne!(child.thread_id, parent.thread_id);
        assert_eq!(
            service.workflow_parent(child.id).await.unwrap(),
            Some(parent.id)
        );
        assert!(
            service
                .thread(child.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
        assert!(service.session(child.thread_id).await.unwrap().is_none());
    }
    assert_eq!(
        service.task_draft(parent.id).await.unwrap(),
        "Original unsent parent draft"
    );
    assert!(
        service
            .create_workflow(value.steps[0].task, WorkflowSpec::example(&parent.agent_id))
            .await
            .is_err()
    );
    drop(service);
    let reopened = WorkspaceService::open(root.path().join("workspace.sqlite3"))
        .await
        .unwrap();
    let restored = reopened.workflow(parent.id).await.unwrap().unwrap();
    assert_eq!(restored.id, value.id);
    assert_eq!(restored.steps[0].task, value.steps[0].task);
    assert!(restored.run.is_none());
    assert_eq!(restored.ready_indices(), vec![0]);
}
#[tokio::test]
async fn invalid_agent_and_transaction_failure_leave_no_partial_children() {
    let (_root, service, parent) = setup().await;
    assert!(
        service
            .create_workflow(parent.id, WorkflowSpec::example("not-installed"))
            .await
            .is_err()
    );
    assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    service.access(|store| {
        store.connection.execute_batch("CREATE TEMP TRIGGER fail_workflow BEFORE INSERT ON preferences WHEN NEW.key LIKE 'task-workflow:%' BEGIN SELECT RAISE(ABORT,'fixture failure'); END;")?;
        Ok(())
    }).await.unwrap();
    assert!(
        service
            .create_workflow(parent.id, WorkflowSpec::example(&parent.agent_id))
            .await
            .is_err()
    );
    assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    assert!(service.workflow(parent.id).await.unwrap().is_none());
}
#[tokio::test]
async fn stale_graph_revision_and_edited_child_draft_never_get_overwritten() {
    let (_root, service, parent) = setup().await;
    let value = service
        .create_workflow(parent.id, WorkflowSpec::example(&parent.agent_id))
        .await
        .unwrap();
    assert!(
        service
            .steer_workflow_step(
                parent.id,
                Uuid::new_v4(),
                value.revision,
                0,
                "wrong graph".into()
            )
            .await
            .is_err()
    );
    assert!(
        service
            .steer_workflow_step(parent.id, value.id, 0, 0, "stale".into())
            .await
            .is_err()
    );
    let changed = service
        .steer_workflow_step(
            parent.id,
            value.id,
            value.revision,
            0,
            "reviewed replacement".into(),
        )
        .await
        .unwrap();
    assert_eq!(
        service.task_draft(changed.steps[0].task).await.unwrap(),
        "reviewed replacement"
    );
    service
        .save_task_draft(changed.steps[0].task, "independent human edit".into())
        .await
        .unwrap();
    assert!(
        service
            .steer_workflow_step(
                parent.id,
                changed.id,
                changed.revision,
                0,
                "must not replace".into()
            )
            .await
            .is_err()
    );
    assert_eq!(
        service.task_draft(changed.steps[0].task).await.unwrap(),
        "independent human edit"
    );
}
#[tokio::test]
async fn detach_preserves_child_data_and_linked_deletion_is_explicit() {
    let (_root, service, parent) = setup().await;
    let value = service
        .create_workflow(parent.id, WorkflowSpec::example(&parent.agent_id))
        .await
        .unwrap();
    let child = value.steps[0].task;
    service.archive_task(child).await.unwrap();
    assert!(service.delete_task(child).await.is_err());
    service
        .detach_workflow(parent.id, value.id, value.revision)
        .await
        .unwrap();
    assert!(service.workflow(parent.id).await.unwrap().is_none());
    assert!(service.workflow_parent(child).await.unwrap().is_none());
    assert!(service.task(child).await.is_ok());
    service.delete_task(child).await.unwrap();
    assert!(service.task(value.steps[1].task).await.is_ok());
}
#[test]
fn run_ownership_and_cancellation_are_independent_and_drop_revokes() {
    let owner = Autonomy::default();
    let a = TaskId::new();
    let b = TaskId::new();
    let run_a = owner.begin(a).unwrap();
    let run_b = owner.begin(b).unwrap();
    assert!(owner.begin(a).is_err());
    assert!(owner.interrupt(a, true));
    assert!(run_a.control.cancel.is_cancelled());
    assert!(!run_b.control.cancel.is_cancelled());
    drop(run_a);
    assert!(!owner.workflow_running(a));
    owner.shutdown();
    assert!(run_b.control.cancel.is_cancelled());
    drop(run_b);
    assert!(!owner.workflow_running(b));
}
