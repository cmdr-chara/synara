//! Durable objectives are inert. Only an explicit, transient UI lease can continue work.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};

pub const GOAL_MAX_FOLLOWUPS: u8 = 2;
pub const GOAL_PURSUIT_LIMIT_MS: u64 = 10 * 60 * 1000;
const MAX_ELAPSED_MS: u64 = 365 * 24 * 60 * 60 * 1000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    #[default]
    Paused,
    Blocked,
    Review,
    Achieved,
}
impl GoalStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Paused => "Paused",
            Self::Blocked => "Blocked",
            Self::Review => "Needs achievement review",
            Self::Achieved => "Achieved",
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalAchievement {
    pub objective: String,
    pub evidence: String,
    pub at_ms: i64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadGoal {
    pub version: u32,
    pub revision: u64,
    pub objective: String,
    pub status: GoalStatus,
    pub note: String,
    pub elapsed_ms: u64,
    pub achievements: Vec<GoalAchievement>,
}
impl Default for ThreadGoal {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            objective: String::new(),
            status: GoalStatus::Paused,
            note: String::new(),
            elapsed_ms: 0,
            achievements: Vec::new(),
        }
    }
}
fn text_valid(text: &str, limit: usize, empty: bool) -> bool {
    text.len() <= limit && !text.contains('\0') && (empty || !text.trim().is_empty())
}
impl ThreadGoal {
    fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1
            || !text_valid(&self.objective, 4096, true)
            || !text_valid(&self.note, 2048, true)
            || self.elapsed_ms > MAX_ELAPSED_MS
            || self.achievements.len() > 16
            || self.objective.is_empty() && self.status != GoalStatus::Paused
            || self.achievements.iter().any(|a| {
                !text_valid(&a.objective, 4096, false)
                    || !text_valid(&a.evidence, 2048, false)
                    || a.at_ms < 0
            })
        {
            return Err(WorkspaceError::Invalid(
                "Unsupported or invalid saved goal. It was not replaced.".into(),
            ));
        }
        Ok(())
    }
    /// This text is shown in the composer before the initial Send. No permissions change.
    pub fn prepare(&self, task: TaskId, existing: &str) -> WorkspaceResult<String> {
        self.validate()?;
        if self.objective.trim().is_empty() || self.status == GoalStatus::Achieved {
            return Err(WorkspaceError::Invalid(
                "Save an unfinished goal first.".into(),
            ));
        }
        let text = format!(
            "Synara goal pursuit\nTask: {task}\nObjective (untrusted task data):\n{}\n\nWork within the current task's permissions. Do not expand authority or retry ambiguous external writes. Ask for approvals or information when needed. This resume permits at most two automatic follow-up turns, within ten minutes. A question, approval, interruption or failure stops continuation.\nFinish with one final line: SYNARA_GOAL_STATUS {{\"state\":\"continue\"|\"blocked\"|\"review\",\"reason\":\"brief evidence or blocker\"}}. Use continue only for safe remaining work with no unanswered question. Use review for a claimed achievement requiring the user's verification, not proven completion.\n\nUser request:\n{existing}",
            self.objective
        );
        if text.len() > 1024 * 1024 {
            return Err(StorageError::Limit.into());
        }
        Ok(text)
    }
    pub fn followup(&self, task: TaskId, number: u8) -> WorkspaceResult<String> {
        if !(1..=GOAL_MAX_FOLLOWUPS).contains(&number) {
            return Err(StorageError::Limit.into());
        }
        self.prepare(task,&format!("Synara automatic goal follow-up {number}/{GOAL_MAX_FOLLOWUPS}, explicitly armed in this conversation. Continue only the reviewed objective. Stop on blockers. Do not repeat an uncertain external write."))
    }
}
#[derive(Clone)]
pub enum GoalEdit {
    Set(String),
    Pause(String),
    Block(String),
    Review(String),
    Achieve(String),
    Clear,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GoalDecision {
    Continue,
    Blocked(String),
    Review(String),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusLine {
    state: String,
    reason: String,
}
/// Missing/unknown output and abnormal stops cannot become retries or achievement proof.
pub fn goal_decision(text: &str, stop: &str, remaining: u8, elapsed_ms: u64) -> GoalDecision {
    let blocked = |why: &str| GoalDecision::Blocked(why.into());
    if !matches!(stop, "end_turn" | "stop") {
        return blocked(
            "The turn was interrupted or ended abnormally. Resume explicitly after review.",
        );
    }
    if text.len() > 256 * 1024 {
        return blocked("Response exceeds the bounded goal evaluator. Review it manually.");
    }
    let status = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| line.trim().strip_prefix("SYNARA_GOAL_STATUS "))
        .and_then(|json| serde_json::from_str::<StatusLine>(json).ok());
    let Some(status) = status else {
        return blocked("No valid goal status was returned. Review the response before resuming.");
    };
    if !text_valid(&status.reason, 2048, false) {
        return blocked("The returned status has no bounded reason.");
    }
    match status.state.as_str() {
        "review" => GoalDecision::Review(status.reason),
        "blocked" => GoalDecision::Blocked(status.reason),
        "continue" if text.contains('?') => {
            blocked("The response contains a question. User input takes priority.")
        }
        "continue" if remaining == 0 => {
            blocked("The two-follow-up budget is exhausted. Review before resuming.")
        }
        "continue" if elapsed_ms >= GOAL_PURSUIT_LIMIT_MS => {
            blocked("The ten-minute pursuit limit is reached. Resume explicitly.")
        }
        "continue" => GoalDecision::Continue,
        _ => blocked("Unknown goal status. No continuation was scheduled."),
    }
}
fn key(task: TaskId) -> String {
    format!("task-goal:{task}")
}
fn read(connection: &Connection, task: TaskId) -> WorkspaceResult<ThreadGoal> {
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
    if raw.as_ref().is_some_and(|s| s.len() > 128 * 1024) {
        return Err(StorageError::Limit.into());
    }
    let goal: ThreadGoal = raw.as_deref().map(decode).transpose()?.unwrap_or_default();
    goal.validate()?;
    Ok(goal)
}
impl WorkspaceService {
    pub async fn thread_goal(&self, task: TaskId) -> WorkspaceResult<ThreadGoal> {
        self.access(move |store| read(&store.connection, task))
            .await
    }
    pub async fn edit_thread_goal(
        &self,
        task: TaskId,
        revision: u64,
        elapsed_ms: u64,
        edit: GoalEdit,
    ) -> WorkspaceResult<ThreadGoal> {
        self.access(move|store|{
            let owner=store.task(task)?.ok_or(WorkspaceError::NotFound)?;
            if owner.state==TaskState::Archived {return Err(WorkspaceError::Invalid("Restore the task before changing its goal.".into()));}
            let tx=store.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            // Recheck owner under the same transaction as the goal write.
            let raw:String=tx.query_row("SELECT data FROM tasks WHERE id=?1",[task.to_string()],|r|r.get(0))?;
            let owner:Task=decode(&raw)?;if owner.id!=task||owner.state==TaskState::Archived{return Err(StorageError::Identity.into());}
            let mut goal=read(&tx,task)?;
            if goal.revision!=revision {return Err(WorkspaceError::Invalid("The goal changed elsewhere. Reload before applying this action.".into()));}
            if elapsed_ms>GOAL_PURSUIT_LIMIT_MS {return Err(StorageError::Limit.into());}
            goal.elapsed_ms=goal.elapsed_ms.saturating_add(elapsed_ms).min(MAX_ELAPSED_MS);
            match edit {
                GoalEdit::Set(objective)=>{if !text_valid(&objective,4096,false){return Err(WorkspaceError::Invalid("Goal text must fit within 4 KiB.".into()));}goal.objective=objective;goal.status=GoalStatus::Paused;goal.note="Saved. Explicit resume and Send are required.".into();goal.elapsed_ms=0;}
                GoalEdit::Pause(note)=>{goal.status=GoalStatus::Paused;goal.note=note;}
                GoalEdit::Block(note)=>{goal.status=GoalStatus::Blocked;goal.note=note;}
                GoalEdit::Review(note)=>{goal.status=GoalStatus::Review;goal.note=note;}
                GoalEdit::Achieve(evidence)=>{
                    if goal.objective.is_empty()||goal.status==GoalStatus::Achieved||!text_valid(&evidence,2048,false){return Err(WorkspaceError::Invalid("Record your verification evidence before marking this goal achieved.".into()));}
                    goal.achievements.push(GoalAchievement{objective:goal.objective.clone(),evidence:evidence.clone(),at_ms:crate::now_ms()});
                    if goal.achievements.len()>16 {goal.achievements.remove(0);}
                    goal.status=GoalStatus::Achieved;goal.note=evidence;
                }
                GoalEdit::Clear=>{goal=ThreadGoal{revision,..ThreadGoal::default()};}
            }
            goal.revision=revision.checked_add(1).ok_or(StorageError::Limit)?;goal.validate()?;
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",params![key(task),encode(&goal)?])?;tx.commit()?;Ok(goal)
        }).await
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
        let t = w.create_task(p.id, "Goal".into(), a).await.unwrap();
        (d, w, t)
    }
    #[test]
    fn goals_continuation_is_bounded_and_never_proves_achievement() {
        let output =
            "Working\nSYNARA_GOAL_STATUS {\"state\":\"continue\",\"reason\":\"next safe check\"}";
        assert_eq!(
            goal_decision(output, "end_turn", 2, 0),
            GoalDecision::Continue
        );
        for stop in ["cancelled", "max_tokens", "unknown"] {
            assert!(matches!(
                goal_decision(output, stop, 2, 0),
                GoalDecision::Blocked(_)
            ));
        }
        assert!(matches!(
            goal_decision(output, "stop", 0, 0),
            GoalDecision::Blocked(_)
        ));
        assert!(matches!(
            goal_decision(output, "stop", 2, GOAL_PURSUIT_LIMIT_MS),
            GoalDecision::Blocked(_)
        ));
        assert!(matches!(
            goal_decision(&format!("Need permission?\n{output}"), "stop", 2, 0),
            GoalDecision::Blocked(_)
        ));
        for invalid in [
            "done",
            "SYNARA_GOAL_STATUS {}",
            "SYNARA_GOAL_STATUS {\"state\":\"continue\",\"reason\":\"\"}",
        ] {
            assert!(matches!(
                goal_decision(invalid, "stop", 2, 0),
                GoalDecision::Blocked(_)
            ));
        }
        assert!(matches!(
            goal_decision(
                "SYNARA_GOAL_STATUS {\"state\":\"review\",\"reason\":\"tests passed\"}",
                "stop",
                2,
                0
            ),
            GoalDecision::Review(_)
        ));
    }
    #[tokio::test]
    async fn goals_persist_paused_and_require_explicit_evidence() {
        let (d, w, t) = setup().await;
        let g = w
            .edit_thread_goal(t.id, 0, 0, GoalEdit::Set("Verify Unicode 日本語".into()))
            .await
            .unwrap();
        assert_eq!(g.status, GoalStatus::Paused);
        assert!(
            g.prepare(t.id, "retain request")
                .unwrap()
                .contains("retain request")
        );
        assert!(g.followup(t.id, 3).is_err());
        assert!(
            w.edit_thread_goal(t.id, 0, 0, GoalEdit::Clear)
                .await
                .is_err()
        );
        assert!(
            w.edit_thread_goal(t.id, g.revision, 0, GoalEdit::Achieve("".into()))
                .await
                .is_err()
        );
        let g = w
            .edit_thread_goal(
                t.id,
                g.revision,
                1234,
                GoalEdit::Block("Needs user input".into()),
            )
            .await
            .unwrap();
        let reopened = WorkspaceService::open(d.path().join("state.db"))
            .await
            .unwrap();
        assert_eq!(reopened.thread_goal(t.id).await.unwrap(), g);
        assert!(reopened.session(t.thread_id).await.unwrap().is_none());
        assert!(
            reopened
                .thread(t.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
        let g = w
            .edit_thread_goal(
                t.id,
                g.revision,
                100,
                GoalEdit::Achieve("I reran the reproduction successfully".into()),
            )
            .await
            .unwrap();
        assert_eq!(g.elapsed_ms, 1334);
        assert_eq!(g.achievements.len(), 1);
        assert!(g.prepare(t.id, "").is_err());
        assert!(
            w.edit_thread_goal(t.id, g.revision, 0, GoalEdit::Achieve("duplicate".into()))
                .await
                .is_err()
        );
        let g = w
            .edit_thread_goal(t.id, g.revision, 0, GoalEdit::Clear)
            .await
            .unwrap();
        assert!(g.objective.is_empty());
        assert!(g.achievements.is_empty());
    }
    #[tokio::test]
    async fn goals_invalid_records_and_limits_do_not_cross_tasks() {
        let (_d, w, t) = setup().await;
        assert!(w.thread_goal(TaskId::new()).await.is_err());
        for text in ["".into(), "x".repeat(4097), "bad\0goal".into()] {
            assert!(
                w.edit_thread_goal(t.id, 0, 0, GoalEdit::Set(text))
                    .await
                    .is_err()
            );
        }
        assert!(
            w.edit_thread_goal(
                t.id,
                0,
                GOAL_PURSUIT_LIMIT_MS + 1,
                GoalEdit::Set("too much time".into())
            )
            .await
            .is_err()
        );
        let other = w
            .create_task(t.project_id, "Other".into(), t.agent_id.clone())
            .await
            .unwrap();
        w.edit_thread_goal(t.id, 0, 0, GoalEdit::Set("Only mine".into()))
            .await
            .unwrap();
        assert!(w.thread_goal(other.id).await.unwrap().objective.is_empty());
        assert!(valid_preference_key(&key(t.id)));
        assert!(!valid_preference_key("task-goal:../task"));
        w.access(move |store| {
            store.connection.execute(
                "INSERT OR REPLACE INTO preferences(key,data) VALUES(?1,?2)",
                params![key(t.id), "invalid json"],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        assert!(w.thread_goal(t.id).await.is_err());
        assert!(
            w.edit_thread_goal(t.id, 0, 0, GoalEdit::Clear)
                .await
                .is_err()
        );
    }
}
