use super::*;
use crate::WorkspaceService;
async fn setup(
    path: Option<std::path::PathBuf>,
) -> (WorkspaceService, AutomationDefinition, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let service = match path {
        Some(path) => WorkspaceService::open(path).await.unwrap(),
        None => WorkspaceService::memory().unwrap(),
    };
    let project = service
        .add_local_workspace(root.path().into())
        .await
        .unwrap();
    let definition = AutomationDefinition {
        id: AutomationId::new_v4(),
        revision: 0,
        title: "Inspect the project".into(),
        instructions: "List files without editing them.".into(),
        agent_id: service.profiles().await.unwrap()[0].id.clone(),
        project_id: project.id,
        schedule: AutomationSchedule::Interval { minutes: 60 },
        timezone: "UTC".into(),
        enabled: true,
        next_run_ms: now_ms(),
        missed: MissedRunPolicy::CatchUpOnce,
        max_runs: None,
        stop_after_consecutive_failures: None,
        failure_streak: 0,
    };
    service
        .save_automation(definition.clone(), None)
        .await
        .unwrap();
    let definition = service.automations().await.unwrap().definitions.remove(0);
    (service, definition, root)
}
#[test]
fn schedules_handle_fixed_offsets_and_weekdays_and_reject_unimplemented_dst_zones() {
    let daily = AutomationSchedule::parse("daily 09:00").unwrap();
    assert_eq!(daily.next_after(0, "+02:00").unwrap(), 7 * 60 * 60 * 1000);
    assert_eq!(
        daily.next_after(7 * 60 * 60 * 1000, "+02:00").unwrap(),
        (24 + 7) * 60 * 60 * 1000
    );
    let friday = 24 * 60 * 60 * 1000;
    let monday = 4 * 24 * 60 * 60 * 1000;
    assert_eq!(
        AutomationSchedule::parse("weekdays 09:00")
            .unwrap()
            .next_after(friday + 10 * 60 * 60 * 1000, "UTC")
            .unwrap(),
        monday + 9 * 60 * 60 * 1000
    );
    assert_eq!(
        AutomationSchedule::parse("weekly mon 09:00")
            .unwrap()
            .next_after(0, "+02:00")
            .unwrap(),
        monday + 7 * 60 * 60 * 1000
    );
    for value in [
        "every 0m",
        "daily 24:00",
        "weekly fuu 09:00",
        "* * * * *",
        "every 999999m",
    ] {
        assert!(AutomationSchedule::parse(value).is_err());
    }
    for zone in ["Europe/Rome", "+14:30", "-99:00", "+02:99", "UTCfoo"] {
        assert!(timezone_offset(zone).is_err());
    }
}
#[tokio::test]
async fn saving_pauses_and_conflicting_edits_are_rejected() {
    let (service, mut definition, _root) = setup(None).await;
    let mut legacy = serde_json::to_value(&definition).unwrap();
    let old_fields = legacy.as_object_mut().unwrap();
    old_fields.remove("max_runs");
    old_fields.remove("stop_after_consecutive_failures");
    old_fields.remove("failure_streak");
    let decoded: AutomationDefinition = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded.max_runs, None);
    assert_eq!(decoded.stop_after_consecutive_failures, None);
    assert_eq!(decoded.failure_streak, 0);
    assert!(!definition.enabled);
    assert_eq!(definition.revision, 1);
    assert!(service.automations().await.unwrap().runs.is_empty());
    definition.title = "Changed".into();
    service
        .save_automation(definition.clone(), Some(1))
        .await
        .unwrap();
    assert!(service.save_automation(definition, Some(1)).await.is_err());
    assert_eq!(
        service.automations().await.unwrap().definitions[0].revision,
        2
    );
}
#[tokio::test]
async fn claims_are_durable_atomic_and_never_replay_after_restart() {
    let db = tempfile::tempdir().unwrap();
    let path = db.path().join("workspace.sqlite");
    let (service, definition, _root) = setup(Some(path.clone())).await;
    service
        .enable_automation(definition.id, 1, true)
        .await
        .unwrap();
    let due = service.automations().await.unwrap().definitions[0].next_run_ms;
    let owner = AutomationId::new_v4();
    let run = service
        .claim_automation(definition.id, owner, true, due)
        .await
        .unwrap()
        .unwrap();
    let task = service.task(run.task_id.unwrap()).await.unwrap();
    assert_eq!(task.project_id, definition.project_id);
    assert_eq!(task.agent_id, definition.agent_id);
    assert_eq!(
        service.task_draft(task.id).await.unwrap(),
        definition.instructions
    );
    assert!(
        service
            .thread(task.thread_id)
            .await
            .unwrap()
            .messages
            .is_empty()
    );
    assert!(
        service
            .claim_automation(definition.id, owner, true, due)
            .await
            .unwrap()
            .is_none()
    );
    drop(service);
    let service = WorkspaceService::open(path).await.unwrap();
    let state = service.automations().await.unwrap();
    assert_eq!(state.runs.len(), 1);
    assert_eq!(state.runs[0].status, AutomationRunStatus::Running);
    assert!(
        service
            .claim_automation(definition.id, AutomationId::new_v4(), false, due + 1000)
            .await
            .unwrap()
            .is_none()
    );
    service
        .resolve_interrupted_automation(run.id, AutomationId::new_v4(), true)
        .await
        .unwrap();
    assert!(!service.automations().await.unwrap().definitions[0].enabled);
    assert!(
        service
            .finish_automation(run.id, owner, AutomationRunStatus::Succeeded, "late".into())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn independent_sqlite_connections_cannot_claim_the_same_slot() {
    let db = tempfile::tempdir().unwrap();
    let path = db.path().join("workspace.sqlite");
    let (a, definition, _root) = setup(Some(path.clone())).await;
    a.enable_automation(definition.id, 1, true).await.unwrap();
    let due = a.automations().await.unwrap().definitions[0].next_run_ms;
    let b = WorkspaceService::open(path).await.unwrap();
    let (one, two) = tokio::join!(
        a.claim_automation(definition.id, AutomationId::new_v4(), true, due),
        b.claim_automation(definition.id, AutomationId::new_v4(), true, due)
    );
    assert_eq!(
        usize::from(one.unwrap().is_some()) + usize::from(two.unwrap().is_some()),
        1
    );
    assert_eq!(a.automations().await.unwrap().runs.len(), 1);
}
#[tokio::test]
async fn skip_and_catch_up_once_have_explicit_bounded_semantics() {
    for policy in [MissedRunPolicy::Skip, MissedRunPolicy::CatchUpOnce] {
        let (service, mut definition, _root) = setup(None).await;
        definition.missed = policy;
        service
            .save_automation(definition.clone(), Some(1))
            .await
            .unwrap();
        service
            .enable_automation(definition.id, 2, true)
            .await
            .unwrap();
        let due = service.automations().await.unwrap().definitions[0].next_run_ms;
        let now = due + 20 * 60 * 60 * 1000;
        let run = service
            .claim_automation(definition.id, AutomationId::new_v4(), true, now)
            .await
            .unwrap();
        let ledger = service.automations().await.unwrap();
        assert_eq!(ledger.runs.len(), 1);
        assert!(ledger.definitions[0].next_run_ms > now);
        assert_eq!(run.is_some(), policy == MissedRunPolicy::CatchUpOnce);
        if policy == MissedRunPolicy::Skip {
            assert!(ledger.runs[0].task_id.is_none());
            assert_eq!(ledger.runs[0].status, AutomationRunStatus::Skipped);
        }
    }
}
#[tokio::test]
async fn run_limit_and_failure_limit_pause_durably_without_replaying() {
    let db = tempfile::tempdir().unwrap();
    let path = db.path().join("workspace.sqlite");
    let (service, mut definition, _root) = setup(Some(path.clone())).await;
    definition.max_runs = Some(2);
    definition.stop_after_consecutive_failures = Some(2);
    service
        .save_automation(definition.clone(), Some(1))
        .await
        .unwrap();
    service
        .enable_automation(definition.id, 2, true)
        .await
        .unwrap();
    let owner = AutomationId::new_v4();
    let first = service
        .claim_automation(definition.id, owner, false, now_ms())
        .await
        .unwrap()
        .unwrap();
    service
        .finish_automation(first.id, owner, AutomationRunStatus::Failed, "first".into())
        .await
        .unwrap();
    assert!(service.automations().await.unwrap().definitions[0].enabled);
    let second = service
        .claim_automation(definition.id, owner, false, now_ms())
        .await
        .unwrap()
        .unwrap();
    service
        .finish_automation(
            second.id,
            owner,
            AutomationRunStatus::Failed,
            "second".into(),
        )
        .await
        .unwrap();
    drop(service);
    let reopened = WorkspaceService::open(path).await.unwrap();
    let ledger = reopened.automations().await.unwrap();
    assert!(!ledger.definitions[0].enabled);
    assert_eq!(ledger.runs.len(), 2);
    assert!(
        reopened
            .enable_automation(definition.id, ledger.definitions[0].revision, true)
            .await
            .is_err()
    );
    assert!(
        reopened
            .claim_automation(definition.id, owner, false, now_ms())
            .await
            .is_err()
    );
    assert_eq!(reopened.automations().await.unwrap().runs.len(), 2);
}
#[tokio::test]
async fn deletion_needs_confirmation_and_retains_history_and_conversation() {
    let (service, definition, _root) = setup(None).await;
    let owner = AutomationId::new_v4();
    let run = service
        .claim_automation(definition.id, owner, false, now_ms())
        .await
        .unwrap()
        .unwrap();
    assert!(
        service
            .delete_automation(definition.id, 1, true)
            .await
            .is_err()
    );
    service
        .finish_automation(
            run.id,
            owner,
            AutomationRunStatus::Failed,
            "Explicit failure".into(),
        )
        .await
        .unwrap();
    assert!(
        service
            .delete_automation(definition.id, 1, false)
            .await
            .is_err()
    );
    service
        .delete_automation(definition.id, 1, true)
        .await
        .unwrap();
    let ledger = service.automations().await.unwrap();
    assert!(ledger.definitions.is_empty());
    assert_eq!(ledger.runs[0].output, "Explicit failure");
    assert!(service.task(run.task_id.unwrap()).await.is_ok());
}
#[tokio::test]
async fn missing_provider_does_not_fall_back_or_create_a_task() {
    let (service, mut definition, _root) = setup(None).await;
    definition.agent_id = "not-installed".into();
    assert!(
        service
            .save_automation(definition.clone(), Some(1))
            .await
            .is_err()
    );
    assert!(service.automations().await.unwrap().runs.is_empty());
    assert!(service.catalog().await.unwrap().tasks.is_empty());
}

struct NeverLaunch;
#[async_trait::async_trait]
impl synara_agent::AgentBackend for NeverLaunch {
    async fn connect(
        &self,
        _: &synara_agent::AgentSpec,
        _: synara_agent::ConnectionContext,
    ) -> synara_agent::AgentResult<std::sync::Arc<dyn synara_agent::AgentConnection>> {
        panic!("No agent may be launched by disarmed, stale or cancelled work")
    }
}
#[tokio::test]
async fn stopped_scheduler_and_cancelled_queued_future_never_claim_or_launch() {
    let (workspace, definition, _root) = setup(None).await;
    let controller = std::sync::Arc::new(crate::Controller::new(
        workspace.clone(),
        std::sync::Arc::new(NeverLaunch),
        std::sync::Arc::new(synara_agent::DenyInteractions),
    ));
    let scheduler = std::sync::Arc::new(AutomationScheduler::new(controller));
    assert!(!scheduler.armed());
    scheduler.tick().await.unwrap();
    let future = scheduler.run_now(definition.id, definition.revision);
    scheduler.stop();
    assert!(future.await.is_err());
    assert!(!scheduler.busy());
    assert!(workspace.automations().await.unwrap().runs.is_empty());
    assert!(workspace.catalog().await.unwrap().tasks.is_empty());
}
#[tokio::test]
async fn changed_instructions_invalidate_run_confirmation_without_creating_tasks() {
    let (workspace, mut definition, _root) = setup(None).await;
    let revision = definition.revision;
    definition.instructions = "New instructions requiring a new confirmation".into();
    workspace
        .save_automation(definition.clone(), Some(revision))
        .await
        .unwrap();
    assert!(
        workspace
            .claim_automation_revision(
                definition.id,
                AutomationId::new_v4(),
                false,
                now_ms(),
                Some(revision)
            )
            .await
            .is_err()
    );
    assert!(workspace.automations().await.unwrap().runs.is_empty());
    assert!(workspace.catalog().await.unwrap().tasks.is_empty());
}

#[tokio::test]
async fn backup_restores_ledger_and_task_identity_without_replaying_a_claim() {
    let (workspace, definition, _root) = setup(None).await;
    let owner = AutomationId::new_v4();
    let run = workspace
        .claim_automation(definition.id, owner, false, now_ms())
        .await
        .unwrap()
        .unwrap();
    workspace
        .finish_automation(
            run.id,
            owner,
            AutomationRunStatus::Succeeded,
            "Retained output".into(),
        )
        .await
        .unwrap();
    let files = tempfile::tempdir().unwrap();
    let backup = files.path().join("snapshot.sqlite");
    workspace
        .backup_to(backup.clone(), crate::RecoveryOptions::default())
        .await
        .unwrap();
    let restored = files.path().join("restored.sqlite");
    WorkspaceService::restore_to(backup, restored.clone(), crate::RecoveryOptions::default())
        .await
        .unwrap();
    let restored = WorkspaceService::open(restored).await.unwrap();
    let ledger = restored.automations().await.unwrap();
    assert_eq!(ledger.definitions[0].id, definition.id);
    assert_eq!(ledger.runs[0].id, run.id);
    assert_eq!(ledger.runs[0].output, "Retained output");
    assert_eq!(
        restored.task(run.task_id.unwrap()).await.unwrap().agent_id,
        definition.agent_id
    );
    assert_eq!(restored.automations().await.unwrap().runs.len(), 1);
}
