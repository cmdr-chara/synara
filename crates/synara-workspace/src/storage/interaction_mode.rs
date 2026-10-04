//! Per-task provider interaction mode. Upstream parity: `OrchestrationThread.interactionMode`
//! persists on the thread and `withProviderDebugModePrompt`/`withProviderPlanModePrompt`
//! inject the mode instructions at provider dispatch. Provider-owned session modes
//! advertised over ACP stay on the session and are separate from these.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};

/// Verbatim upstream `PROVIDER_DEBUG_MODE_PROMPT_PREFIX`
/// (apps/server/src/provider/debugMode.ts).
pub const DEBUG_MODE_PROMPT_PREFIX: &str = "<synara_debug_mode>\nYou are operating in Synara Debug mode. Diagnose the reported defect using this evidence-first loop: observe -> reproduce -> investigate -> fix -> verify.\n\n- Inspect the real current state before editing. Reproduce locally when possible and collect relevant logs, errors, and stack traces.\n- Form testable hypotheses and use evidence to narrow them. Fix the smallest root cause rather than masking symptoms.\n- Add or update a regression test when practical. Run an appropriate verification and confirm the original symptom before declaring the bug resolved. Never claim success without verification.\n- Preserve the current runtime permission mode. Debug does not grant extra access and is not Plan mode.\n- If reproduction requires the user, give exact steps and say what must remain open. When a structured user-input tool is available, ask one reproduction question with the choices \"Reproduced\", \"Could not reproduce\", and \"Cancel\". If the provider cannot pause for structured input, send the same instructions as normal text, end the turn, and continue only after the user's next message.\n- Do not imply Synara can observe external actions. If browser state, terminal output, logs, or another required signal is inaccessible, ask the user for that evidence.\n- If blocked, report what was inspected, the evidence obtained, the remaining uncertainty, and the next concrete step.\n</synara_debug_mode>";

/// Verbatim upstream `PROVIDER_PLAN_MODE_PROMPT_PREFIX`
/// (apps/server/src/provider/planMode.ts).
pub const PLAN_MODE_PROMPT_PREFIX: &str = "Synara plan mode is active.\nDo not implement or mutate files in this turn. You may inspect or ask targeted questions as needed.\nWhen you are ready to present the final plan, wrap only the final plan markdown in these exact tags:\n<proposed_plan>\nplan content\n</proposed_plan>\nUse at most one proposed_plan block. Keep the tags in English exactly as shown.";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    #[default]
    Default,
    Plan,
    Debug,
}
impl InteractionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Plan => "Plan",
            Self::Debug => "Debug",
        }
    }
}
/// Mirrors upstream `withProviderDebugModePrompt`: debug-only and idempotent.
pub fn with_debug_prompt(mode: InteractionMode, text: &str) -> String {
    if mode != InteractionMode::Debug || text.starts_with(DEBUG_MODE_PROMPT_PREFIX) {
        return text.to_owned();
    }
    if text.is_empty() {
        DEBUG_MODE_PROMPT_PREFIX.to_owned()
    } else {
        format!("{DEBUG_MODE_PROMPT_PREFIX}\n\n{text}")
    }
}
/// Mirrors upstream `withProviderPlanModePrompt`: plan-only.
pub fn with_plan_prompt(mode: InteractionMode, text: &str) -> String {
    if mode != InteractionMode::Plan {
        return text.to_owned();
    }
    let text = text.trim();
    if text.is_empty() {
        PLAN_MODE_PROMPT_PREFIX.to_owned()
    } else {
        format!("{PLAN_MODE_PROMPT_PREFIX}\n\nUser request:\n{text}")
    }
}
/// Applies the persisted per-task interaction-mode prompt shim.
pub fn with_interaction_prompt(mode: InteractionMode, text: &str) -> String {
    match mode {
        InteractionMode::Plan => with_plan_prompt(mode, text),
        InteractionMode::Debug => with_debug_prompt(mode, text),
        InteractionMode::Default => text.to_owned(),
    }
}
fn key(task: TaskId) -> String {
    format!("task-interaction-mode:{task}")
}
fn read(connection: &Connection, task: TaskId) -> WorkspaceResult<InteractionMode> {
    let owner: Option<String> = connection
        .query_row(
            "SELECT data FROM tasks WHERE id=?1",
            [task.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    let owner: Task = decode(&owner.ok_or(WorkspaceError::NotFound)?)?;
    if owner.id != task {
        return Err(StorageError::Identity.into());
    }
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [key(task)],
            |r| r.get(0),
        )
        .optional()?;
    // An unreadable or oversized record must not fail the task; the mode falls
    // back to Default like an absent one.
    match raw.as_deref() {
        Some(raw) if raw.len() <= 128 * 1024 => decode(raw).or(Ok(InteractionMode::Default)),
        _ => Ok(InteractionMode::Default),
    }
}
impl WorkspaceService {
    pub async fn interaction_mode(&self, task: TaskId) -> WorkspaceResult<InteractionMode> {
        self.access(move |store| read(&store.connection, task))
            .await
    }
    pub async fn set_interaction_mode(
        &self,
        task: TaskId,
        mode: InteractionMode,
    ) -> WorkspaceResult<InteractionMode> {
        self.access(move |store| {
            let owner = store.task(task)?.ok_or(WorkspaceError::NotFound)?;
            if owner.state == TaskState::Archived {
                return Err(WorkspaceError::Invalid(
                    "Restore the task before changing its interaction mode.".into(),
                ));
            }
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let raw: String = tx.query_row(
                "SELECT data FROM tasks WHERE id=?1",
                [task.to_string()],
                |r| r.get(0),
            )?;
            let owner: Task = decode(&raw)?;
            if owner.id != task || owner.state == TaskState::Archived {
                return Err(StorageError::Identity.into());
            }
            tx.execute(
                "INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",
                params![key(task), encode(&mode)?],
            )?;
            tx.commit()?;
            Ok(mode)
        })
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    async fn setup() -> (tempfile::TempDir, WorkspaceService, Task) {
        let d = tempfile::tempdir().unwrap();
        let w = WorkspaceService::open(d.path().join("state.db"))
            .await
            .unwrap();
        let p = w.add_local_workspace(d.path().into()).await.unwrap();
        let a = w.profiles().await.unwrap()[0].id.clone();
        let t = w.create_task(p.id, "Mode".into(), a).await.unwrap();
        (d, w, t)
    }
    #[tokio::test]
    async fn mode_defaults_and_persists_per_task() {
        let (d, w, t) = setup().await;
        let t2 = w
            .create_task(t.project_id, "Other".into(), t.agent_id.clone())
            .await
            .unwrap();
        assert_eq!(
            w.interaction_mode(t.id).await.unwrap(),
            InteractionMode::Default
        );
        w.set_interaction_mode(t.id, InteractionMode::Debug)
            .await
            .unwrap();
        assert_eq!(
            w.interaction_mode(t.id).await.unwrap(),
            InteractionMode::Debug
        );
        assert_eq!(
            w.interaction_mode(t2.id).await.unwrap(),
            InteractionMode::Default
        );
        w.set_interaction_mode(t.id, InteractionMode::Default)
            .await
            .unwrap();
        assert_eq!(
            w.interaction_mode(t.id).await.unwrap(),
            InteractionMode::Default
        );
        // Survives a reopen of the same database.
        drop(w);
        let w2 = WorkspaceService::open(d.path().join("state.db"))
            .await
            .unwrap();
        w2.set_interaction_mode(t.id, InteractionMode::Debug)
            .await
            .unwrap();
        drop(w2);
        let w3 = WorkspaceService::open(d.path().join("state.db"))
            .await
            .unwrap();
        assert_eq!(
            w3.interaction_mode(t.id).await.unwrap(),
            InteractionMode::Debug
        );
    }
    #[tokio::test]
    async fn mode_rejects_unknown_and_archived_tasks() {
        let (_d, w, t) = setup().await;
        assert!(w.interaction_mode(TaskId::new()).await.is_err());
        assert!(
            w.set_interaction_mode(TaskId::new(), InteractionMode::Debug)
                .await
                .is_err()
        );
        w.archive_task(t.id).await.unwrap();
        assert_eq!(
            w.interaction_mode(t.id).await.unwrap(),
            InteractionMode::Default
        );
        assert!(
            w.set_interaction_mode(t.id, InteractionMode::Debug)
                .await
                .is_err()
        );
    }
    #[test]
    fn debug_prompt_prefix_matches_upstream_contract() {
        // Default mode and repeated application are no-ops.
        assert_eq!(
            with_debug_prompt(InteractionMode::Default, "fix it"),
            "fix it"
        );
        let once = with_debug_prompt(InteractionMode::Debug, "fix it");
        assert!(once.starts_with(DEBUG_MODE_PROMPT_PREFIX));
        assert!(once.ends_with("fix it"));
        assert_eq!(with_debug_prompt(InteractionMode::Debug, &once), once);
        assert_eq!(
            with_debug_prompt(InteractionMode::Debug, ""),
            DEBUG_MODE_PROMPT_PREFIX
        );
    }
    #[test]
    fn plan_prompt_prefix_matches_upstream_contract() {
        assert_eq!(
            with_interaction_prompt(InteractionMode::Default, "fix it"),
            "fix it"
        );
        let once = with_plan_prompt(InteractionMode::Plan, "fix it");
        assert!(once.starts_with(PLAN_MODE_PROMPT_PREFIX));
        assert!(once.ends_with("User request:\nfix it"));
        assert_eq!(
            with_plan_prompt(InteractionMode::Plan, ""),
            PLAN_MODE_PROMPT_PREFIX
        );
        assert_eq!(
            with_interaction_prompt(InteractionMode::Plan, "  fix it  "),
            format!("{PLAN_MODE_PROMPT_PREFIX}\n\nUser request:\nfix it")
        );
        assert_eq!(with_plan_prompt(InteractionMode::Debug, "fix it"), "fix it");
    }
    #[tokio::test]
    async fn plan_mode_round_trips_per_task() {
        let (_d, w, t) = setup().await;
        w.set_interaction_mode(t.id, InteractionMode::Plan)
            .await
            .unwrap();
        assert_eq!(
            w.interaction_mode(t.id).await.unwrap(),
            InteractionMode::Plan
        );
        w.set_interaction_mode(t.id, InteractionMode::Default)
            .await
            .unwrap();
        assert_eq!(
            w.interaction_mode(t.id).await.unwrap(),
            InteractionMode::Default
        );
    }
}
