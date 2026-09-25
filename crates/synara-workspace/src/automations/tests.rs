use super::*;
use crate::WorkspaceService;
use std::sync::Arc;
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
        context: AutomationContextPolicy::Project,
        max_runs: None,
        stop_after_consecutive_failures: None,
        failure_streak: 0,
        run_count: 0,
        max_runtime_seconds: DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS,
    };
    service
        .save_automation(definition.clone(), None)
        .await
        .unwrap();
    let definition = service.automations().await.unwrap().definitions.remove(0);
    (service, definition, root)
}
#[test]
fn schedules_validate_fixed_offsets_and_iana_zones() {
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
    assert!(parse_timezone("Europe/Rome").is_ok());
    for zone in [
        "Mars/Olympus",
        "../UTC",
        "America//New_York",
        "/etc/localtime",
    ] {
        assert!(parse_timezone(zone).is_err());
    }
}

fn timestamp_ms(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value)
        .unwrap()
        .timestamp_millis()
}

#[test]
fn daily_iana_schedule_skips_gaps_and_uses_first_fold_occurrence() {
    let zone = "America/New_York";
    let gap = AutomationSchedule::parse("daily 02:30").unwrap();
    assert_eq!(
        gap.next_after(timestamp_ms("2026-03-08T05:00:00Z"), zone)
            .unwrap(),
        timestamp_ms("2026-03-09T06:30:00Z")
    );

    let fold = AutomationSchedule::parse("daily 01:30").unwrap();
    let first = timestamp_ms("2026-11-01T05:30:00Z");
    assert_eq!(
        fold.next_after(timestamp_ms("2026-11-01T04:00:00Z"), zone)
            .unwrap(),
        first
    );
    assert_eq!(
        fold.next_after(first, zone).unwrap(),
        timestamp_ms("2026-11-02T06:30:00Z")
    );
}

#[test]
fn cron_uses_upstream_five_field_grammar_and_dom_dow_semantics() {
    let schedule = AutomationSchedule::parse("cron 5,35 9-17/2 1,15 */2 mon-fri").unwrap();
    assert_eq!(schedule.label(), "cron 5,35 9-17/2 1,15 */2 mon-fri");
    assert_eq!(
        AutomationSchedule::parse("cron 0 9 * * *")
            .unwrap()
            .next_after(0, "+02:00")
            .unwrap(),
        7 * 60 * 60 * 1000
    );

    // With both calendar day fields constrained, upstream semantics match either one.
    // May 4 is Monday, so it matches the weekday even though the DOM is not 1.
    assert_eq!(
        AutomationSchedule::parse("cron 0 9 1 * mon")
            .unwrap()
            .next_after(timestamp_ms("2026-05-01T10:00:00Z"), "UTC")
            .unwrap(),
        timestamp_ms("2026-05-04T09:00:00Z")
    );
    assert_eq!(
        AutomationSchedule::parse("cron 0 9 * * MON-FRI")
            .unwrap()
            .next_after(timestamp_ms("2026-05-01T10:00:00Z"), "UTC")
            .unwrap(),
        timestamp_ms("2026-05-04T09:00:00Z")
    );
    // Numeric weekday 7 aliases Sunday.
    assert_eq!(
        AutomationSchedule::parse("cron 0 9 * * 7")
            .unwrap()
            .next_after(timestamp_ms("2026-05-04T10:00:00Z"), "UTC")
            .unwrap(),
        timestamp_ms("2026-05-10T09:00:00Z")
    );
    // Sparse leap-day schedules remain valid across a non-leap year and a
    // century exception instead of being rejected after a one-year search.
    assert_eq!(
        AutomationSchedule::parse("cron 0 9 29 2 *")
            .unwrap()
            .next_after(timestamp_ms("2097-03-01T00:00:00Z"), "UTC")
            .unwrap(),
        timestamp_ms("2104-02-29T09:00:00Z")
    );

    for value in [
        "cron",
        "cron * * * *",
        "cron 60 * * * *",
        "cron * 24 * * *",
        "cron * * 0 * *",
        "cron * * * 13 *",
        "cron * * * * 8",
        "cron */0 * * * *",
        "cron 1,,2 * * * *",
        "cron 5-2 * * * *",
        "cron 1-2-3 * * * *",
        "cron * * * * monday",
    ] {
        assert!(
            AutomationSchedule::parse(value).is_err(),
            "accepted {value}"
        );
    }
    assert!(AutomationSchedule::parse(&format!("cron {} * * * *", "1".repeat(121))).is_err());
    // Impossible dates are rejected after a bounded calendar search.
    assert!(
        AutomationSchedule::parse("cron 0 0 31 2 *")
            .unwrap()
            .next_after(timestamp_ms("2026-01-01T00:00:00Z"), "UTC")
            .is_err()
    );
}

#[test]
fn cron_iana_schedule_skips_gaps_and_does_not_repeat_folded_wall_time() {
    let zone = "America/New_York";
    let gap = AutomationSchedule::parse("cron 30 2 * * *").unwrap();
    assert_eq!(
        gap.next_after(timestamp_ms("2026-03-08T05:00:00Z"), zone)
            .unwrap(),
        timestamp_ms("2026-03-09T06:30:00Z")
    );

    let fold = AutomationSchedule::parse("cron 30 1 * * *").unwrap();
    let first = timestamp_ms("2026-11-01T05:30:00Z");
    let next_day = timestamp_ms("2026-11-02T06:30:00Z");
    assert_eq!(
        fold.next_after(timestamp_ms("2026-11-01T04:00:00Z"), zone)
            .unwrap(),
        first
    );
    // The second 01:30 on Nov 1 is a DST fold duplicate of the claimed slot.
    assert_eq!(fold.next_after(first, zone).unwrap(), next_day);
    assert_eq!(
        fold.next_after(timestamp_ms("2026-11-01T05:45:00Z"), zone)
            .unwrap(),
        next_day
    );
}
#[tokio::test]
async fn saving_pauses_and_conflicting_edits_are_rejected() {
    let (service, mut definition, _root) = setup(None).await;
    let mut legacy = serde_json::to_value(&definition).unwrap();
    let old_fields = legacy.as_object_mut().unwrap();
    old_fields.remove("context");
    old_fields.remove("max_runs");
    old_fields.remove("stop_after_consecutive_failures");
    old_fields.remove("failure_streak");
    old_fields.remove("run_count");
    old_fields.remove("max_runtime_seconds");
    let decoded: AutomationDefinition = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded.context, AutomationContextPolicy::Project);
    assert_eq!(decoded.max_runs, None);
    assert_eq!(decoded.stop_after_consecutive_failures, None);
    assert_eq!(decoded.failure_streak, 0);
    assert_eq!(decoded.run_count, 0);
    assert_eq!(
        decoded.max_runtime_seconds,
        DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS
    );
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
async fn max_runtime_is_bounded_and_persists_across_reopen() {
    let db = tempfile::tempdir().unwrap();
    let path = db.path().join("workspace.sqlite");
    let (service, mut definition, _root) = setup(Some(path.clone())).await;
    assert_eq!(
        definition.max_runtime_seconds,
        DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS
    );

    definition.max_runtime_seconds = 0;
    assert!(definition.validate().is_err());
    definition.max_runtime_seconds = MAX_AUTOMATION_MAX_RUNTIME_SECONDS + 1;
    assert!(definition.validate().is_err());
    definition.max_runtime_seconds = 37;
    service
        .save_automation(definition.clone(), Some(1))
        .await
        .unwrap();
    drop(service);

    let reopened = WorkspaceService::open(path).await.unwrap();
    assert_eq!(
        reopened.automations().await.unwrap().definitions[0].max_runtime_seconds,
        37
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
async fn a_cron_fold_claim_advances_atomically_and_does_not_replay_after_restart() {
    let db = tempfile::tempdir().unwrap();
    let path = db.path().join("workspace.sqlite");
    let (service, mut definition, _root) = setup(Some(path.clone())).await;
    definition.schedule = AutomationSchedule::parse("cron 30 1 * * *").unwrap();
    definition.timezone = "America/New_York".into();
    service
        .save_automation(definition.clone(), Some(1))
        .await
        .unwrap();

    let id = definition.id;
    let due = timestamp_ms("2026-11-01T05:30:00Z");
    service
        .access(move |store| {
            store.edit_automations(move |ledger| {
                let definition = ledger
                    .definitions
                    .iter_mut()
                    .find(|definition| definition.id == id)
                    .ok_or_else(|| invalid("Test automation missing."))?;
                definition.enabled = true;
                definition.next_run_ms = due;
                Ok(())
            })
        })
        .await
        .unwrap();

    let owner = AutomationId::new_v4();
    let run = service
        .claim_automation(id, owner, true, due)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.scheduled_ms, Some(due));
    drop(service);

    let reopened = WorkspaceService::open(path).await.unwrap();
    let ledger = reopened.automations().await.unwrap();
    assert_eq!(
        ledger.definitions[0].next_run_ms,
        timestamp_ms("2026-11-02T06:30:00Z")
    );
    assert!(
        reopened
            .claim_automation(
                id,
                AutomationId::new_v4(),
                true,
                timestamp_ms("2026-11-01T06:30:00Z")
            )
            .await
            .unwrap()
            .is_none()
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
    let task_id = run.task_id.unwrap();
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
    assert!(service.task(task_id).await.is_ok());
    assert!(
        service
            .prune_deleted_automation_history(false)
            .await
            .is_err()
    );
    assert_eq!(
        service
            .prune_deleted_automation_history(true)
            .await
            .unwrap(),
        1
    );
    assert!(service.automations().await.unwrap().runs.is_empty());
    assert!(service.task(task_id).await.is_ok());
}

#[tokio::test]
async fn history_pruning_never_removes_live_definition_runs() {
    let (service, definition, _root) = setup(None).await;
    let owner = AutomationId::new_v4();
    let run = service
        .claim_automation(definition.id, owner, false, now_ms())
        .await
        .unwrap()
        .unwrap();
    service
        .finish_automation(run.id, owner, AutomationRunStatus::Succeeded, "kept".into())
        .await
        .unwrap();

    assert_eq!(
        service
            .prune_deleted_automation_history(true)
            .await
            .unwrap(),
        0
    );
    let ledger = service.automations().await.unwrap();
    assert_eq!(ledger.definitions.len(), 1);
    assert_eq!(ledger.runs.len(), 1);
    assert_eq!(ledger.runs[0].output, "kept");
    assert!(service.task(run.task_id.unwrap()).await.is_ok());
}

#[tokio::test]
async fn hub_context_is_opt_in_snapshotted_and_studio_scoped() {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    let (hub, _) = service
        .create_hub(
            root.path().to_path_buf(),
            "Automation Hub".into(),
            agent.clone(),
        )
        .await
        .unwrap();
    let mut hub = hub;
    hub.instructions = "Use the reviewed Hub method.".into();
    hub.memory = "Shared fact: release train 7.".into();
    let hub = service.save_hub(hub.revision, hub).await.unwrap();

    let definition = AutomationDefinition {
        id: AutomationId::new_v4(),
        revision: 0,
        title: "Hub run".into(),
        instructions: "Inspect the current release status.".into(),
        agent_id: agent,
        project_id: hub.project,
        schedule: AutomationSchedule::Interval { minutes: 60 },
        timezone: "UTC".into(),
        enabled: false,
        next_run_ms: now_ms(),
        missed: MissedRunPolicy::CatchUpOnce,
        context: AutomationContextPolicy::Hub,
        max_runs: None,
        stop_after_consecutive_failures: None,
        failure_streak: 0,
        run_count: 0,
        max_runtime_seconds: DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS,
    };
    service.save_automation(definition, None).await.unwrap();
    let current = service.automations().await.unwrap().definitions.remove(0);
    let run = service
        .claim_automation(current.id, AutomationId::new_v4(), false, now_ms())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.hub_revision, Some(hub.revision));
    assert!(run.prompt.contains("## Hub instructions"));
    assert!(run.prompt.contains("Use the reviewed Hub method."));
    assert!(run.prompt.contains("Shared fact: release train 7."));
    assert!(run.prompt.ends_with("Inspect the current release status."));
    let task = service.task(run.task_id.unwrap()).await.unwrap();
    assert_eq!(task.scope, synara_core::TaskScope::Studio);
    assert_eq!(service.task_draft(task.id).await.unwrap(), run.prompt);

    let mut changed = hub.clone();
    changed.memory = "Newer shared fact".into();
    service.save_hub(changed.revision, changed).await.unwrap();
    let retained = service.automations().await.unwrap().runs.remove(0);
    assert!(retained.prompt.contains("release train 7"));
    assert!(!retained.prompt.contains("Newer shared fact"));
}

#[tokio::test]
async fn project_context_never_harvests_existing_hub_context() {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    let (mut hub, _) = service
        .create_hub(
            root.path().to_path_buf(),
            "Project Hub".into(),
            agent.clone(),
        )
        .await
        .unwrap();
    hub.memory = "Must not be injected".into();
    let hub = service.save_hub(hub.revision, hub).await.unwrap();
    let definition = AutomationDefinition {
        id: AutomationId::new_v4(),
        revision: 0,
        title: "Project run".into(),
        instructions: "Only this instruction.".into(),
        agent_id: agent,
        project_id: hub.project,
        schedule: AutomationSchedule::Interval { minutes: 60 },
        timezone: "UTC".into(),
        enabled: false,
        next_run_ms: now_ms(),
        missed: MissedRunPolicy::CatchUpOnce,
        context: AutomationContextPolicy::Project,
        max_runs: None,
        stop_after_consecutive_failures: None,
        failure_streak: 0,
        run_count: 0,
        max_runtime_seconds: DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS,
    };
    service.save_automation(definition, None).await.unwrap();
    let current = service.automations().await.unwrap().definitions.remove(0);
    let run = service
        .claim_automation(current.id, AutomationId::new_v4(), false, now_ms())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.hub_revision, None);
    assert_eq!(run.prompt, "Only this instruction.");
    let task = service.task(run.task_id.unwrap()).await.unwrap();
    assert_eq!(task.scope, synara_core::TaskScope::Project);
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

#[derive(Default)]
struct BlockingRunEvidence {
    started: tokio::sync::Notify,
    cancelled: std::sync::atomic::AtomicBool,
}
struct BlockingRunBackend(Arc<BlockingRunEvidence>);
struct BlockingRunConnection {
    info: tokio::sync::watch::Sender<synara_agent::ConnectionInfo>,
    evidence: Arc<BlockingRunEvidence>,
}
struct BlockingRunSession {
    id: String,
    thread: synara_core::ThreadId,
    evidence: Arc<BlockingRunEvidence>,
}
#[async_trait::async_trait]
impl synara_agent::AgentBackend for BlockingRunBackend {
    async fn connect(
        &self,
        _: &synara_agent::AgentSpec,
        _: synara_agent::ConnectionContext,
    ) -> synara_agent::AgentResult<Arc<dyn synara_agent::AgentConnection>> {
        let (info, _) = tokio::sync::watch::channel(synara_agent::ConnectionInfo {
            id: synara_core::ConnectionId::new(),
            state: synara_core::ConnectionState::Connected,
            identity: None,
            capabilities: synara_core::AgentCapabilities::default(),
            authentication: vec![],
            host: "Local".into(),
            error: None,
        });
        Ok(Arc::new(BlockingRunConnection {
            info,
            evidence: self.0.clone(),
        }))
    }
}
#[async_trait::async_trait]
impl synara_agent::AgentConnection for BlockingRunConnection {
    fn info(&self) -> synara_agent::ConnectionInfo {
        self.info.borrow().clone()
    }
    fn observe(&self) -> tokio::sync::watch::Receiver<synara_agent::ConnectionInfo> {
        self.info.subscribe()
    }
    async fn new_session(
        &self,
        options: synara_agent::SessionOptions,
    ) -> synara_agent::AgentResult<Arc<dyn synara_agent::AgentSession>> {
        Ok(Arc::new(BlockingRunSession {
            id: "automation-timeout".into(),
            thread: options.thread_id,
            evidence: self.evidence.clone(),
        }))
    }
    async fn disconnect(&self) -> synara_agent::AgentResult<()> {
        Ok(())
    }
}
#[async_trait::async_trait]
impl synara_agent::AgentSession for BlockingRunSession {
    fn id(&self) -> &str {
        &self.id
    }
    fn thread_id(&self) -> synara_core::ThreadId {
        self.thread
    }
    fn configuration(&self) -> synara_core::SessionConfiguration {
        synara_core::SessionConfiguration::default()
    }
    async fn prompt(&self, _: synara_agent::Prompt) -> synara_agent::AgentResult<String> {
        self.evidence.started.notify_one();
        std::future::pending().await
    }
    async fn cancel(&self) -> synara_agent::AgentResult<()> {
        self.evidence
            .cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }
    async fn close(&self) -> synara_agent::AgentResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn configured_runtime_timeout_cancels_and_records_a_failure() {
    let (workspace, mut definition, _root) = setup(None).await;
    definition.max_runtime_seconds = 1;
    workspace
        .save_automation(definition.clone(), Some(1))
        .await
        .unwrap();
    let current = workspace.automations().await.unwrap().definitions.remove(0);
    let evidence = Arc::new(BlockingRunEvidence::default());
    let controller = Arc::new(crate::Controller::new(
        workspace.clone(),
        Arc::new(BlockingRunBackend(evidence.clone())),
        Arc::new(synara_agent::DenyInteractions),
    ));
    let scheduler = Arc::new(AutomationScheduler::new(controller));

    let future = scheduler.run_now(current.id, current.revision);
    let executing = tokio::spawn(future);
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        evidence.started.notified(),
    )
    .await
    .expect("agent prompt should start before the runtime limit");
    executing.await.unwrap().unwrap();

    assert!(
        evidence
            .cancelled
            .load(std::sync::atomic::Ordering::Acquire)
    );
    let ledger = workspace.automations().await.unwrap();
    assert_eq!(ledger.runs.len(), 1);
    assert_eq!(ledger.runs[0].status, AutomationRunStatus::Failed);
    assert!(ledger.runs[0].output.contains("1-second execution limit"));
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

#[tokio::test]
async fn live_history_pruning_preserves_cumulative_run_limit_and_conversations() {
    let (workspace, mut definition, _root) = setup(None).await;
    definition.max_runs = Some(2);
    workspace
        .save_automation(definition.clone(), Some(definition.revision))
        .await
        .unwrap();
    let mut current = workspace.automations().await.unwrap().definitions.remove(0);
    workspace
        .enable_automation(current.id, current.revision, true)
        .await
        .unwrap();
    current = workspace.automations().await.unwrap().definitions.remove(0);

    let owner = AutomationId::new_v4();
    let first = workspace
        .claim_automation(current.id, owner, false, now_ms())
        .await
        .unwrap()
        .unwrap();
    let first_task = first.task_id.unwrap();
    workspace
        .finish_automation(
            first.id,
            owner,
            AutomationRunStatus::Succeeded,
            "first".into(),
        )
        .await
        .unwrap();

    let before_prune = workspace.automations().await.unwrap();
    let live = before_prune.definitions[0].clone();
    assert_eq!(effective_run_count(&live, &before_prune.runs), 1);
    assert!(
        workspace
            .prune_automation_history(live.id, live.revision, false)
            .await
            .is_err()
    );
    assert_eq!(
        workspace
            .prune_automation_history(live.id, live.revision, true)
            .await
            .unwrap(),
        1
    );
    assert!(workspace.automations().await.unwrap().runs.is_empty());
    assert!(workspace.task(first_task).await.is_ok());

    let live = workspace.automations().await.unwrap().definitions.remove(0);
    assert_eq!(live.run_count, 1);
    let second = workspace
        .claim_automation(live.id, owner, false, now_ms())
        .await
        .unwrap()
        .unwrap();
    workspace
        .finish_automation(
            second.id,
            owner,
            AutomationRunStatus::Succeeded,
            "second".into(),
        )
        .await
        .unwrap();
    let exhausted = workspace.automations().await.unwrap().definitions.remove(0);
    assert_eq!(exhausted.run_count, 2);
    assert!(!exhausted.enabled);

    assert_eq!(
        workspace
            .prune_automation_history(exhausted.id, exhausted.revision, true)
            .await
            .unwrap(),
        1
    );
    let exhausted = workspace.automations().await.unwrap().definitions.remove(0);
    assert_eq!(exhausted.run_count, 2);
    assert!(
        workspace
            .enable_automation(exhausted.id, exhausted.revision, true)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn history_export_is_versioned_exact_and_never_overwrites() {
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
            "exported evidence".into(),
        )
        .await
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("automation-history.json");
    assert_eq!(
        workspace
            .export_automation_history(destination.clone())
            .await
            .unwrap(),
        1
    );
    let bytes = std::fs::read(&destination).unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["format"], "synara-automation-history-v1");
    assert!(payload["exportedAtMs"].as_i64().is_some());
    assert_eq!(payload["runs"].as_array().unwrap().len(), 1);
    assert_eq!(payload["runs"][0]["id"], run.id.to_string());
    assert_eq!(
        payload["runs"][0]["definition"]["id"],
        definition.id.to_string()
    );
    assert_eq!(payload["runs"][0]["output"], "exported evidence");

    std::fs::write(&destination, "user file").unwrap();
    assert!(
        workspace
            .export_automation_history(destination.clone())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(destination).unwrap(), "user file");
}
