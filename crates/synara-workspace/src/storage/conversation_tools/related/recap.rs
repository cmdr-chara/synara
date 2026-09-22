//! Immutable generated text cached for its source sequence. No source events are written.
use super::*;

const MAX_RECAP: usize = 64 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadRecap {
    pub version: u32,
    pub source: TaskId,
    pub source_sequence: u64,
    pub generated_task: TaskId,
    pub generated_sequence: u64,
    pub message: MessageAnchor,
    pub text: String,
    pub saved_at_ms: i64,
}
impl ThreadRecap {
    fn validate(&self, task: TaskId) -> WorkspaceResult<()> {
        if self.version != 1
            || self.source != task
            || self.source == self.generated_task
            || self.message.role != Role::Assistant
            || !self.message.valid()
            || self.text.trim().is_empty()
            || self.text.len() > MAX_RECAP
            || self.text.contains('\0')
        {
            return Err(invalid(
                "Stored recap is invalid or unsupported. It was not replaced.",
            ));
        }
        Ok(())
    }
}
fn key(task: TaskId) -> String {
    format!("task-recap:{task}")
}
impl WorkspaceService {
    pub async fn thread_recap(&self, task: TaskId) -> WorkspaceResult<Option<ThreadRecap>> {
        self.access(move |store| {
            task_record(&store.connection, task)?;
            let value: Option<ThreadRecap> = store.preference(&key(task))?;
            if let Some(value) = &value {
                value.validate(task)?;
            }
            Ok(value)
        })
        .await
    }
    /// Called only after explicit review of a completed generated conversation.
    /// Pin both the result and source sequence and fail closed on stale/failed runs.
    pub async fn save_thread_recap(
        &self,
        child: TaskId,
        reviewed_sequence: u64,
    ) -> WorkspaceResult<ThreadRecap> {
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
            let (generated, thread) = read_conversation(&tx, child)?;
            let origin = read_origin(&tx, child)?.filter(|o| o.kind == RelatedThreadKind::Recap)
                .ok_or_else(|| invalid("This task is not an owned recap request."))?;
            let (source, original) = read_conversation(&tx, origin.parent)?;
            if generated.state != TaskState::Completed || thread.state != TaskState::Completed
                || thread.last_sequence != reviewed_sequence || original.last_sequence != origin.sequence
                || source.state == TaskState::Archived || source.project_id != generated.project_id
                || source.working_directory != generated.working_directory
            { return Err(invalid("The result is unfinished, interrupted, stale, or its source changed. Review a new recap request. The previous cache was preserved.")); }
            let turn = thread.turns.last().filter(|t| t.finished_at_ms.is_some() && !t.failed)
                .ok_or_else(|| invalid("No successfully completed recap turn exists."))?;
            let message = thread.timeline.get(turn.first_timeline_index..turn.end_timeline_index)
                .unwrap_or_default().iter().rev().find_map(|item| match item {
                    TranscriptItem::Message { index } => thread.messages.get(*index)
                        .filter(|message| message.role == Role::Assistant && !message.text.trim().is_empty()),
                    _ => None,
                }).ok_or_else(|| invalid("The completed turn has no visible assistant recap."))?;
            let value = ThreadRecap { version: 1, source: source.id, source_sequence: origin.sequence,
                generated_task: child, generated_sequence: reviewed_sequence, message: message.into(),
                text: message.text.clone(), saved_at_ms: now_ms() };
            value.validate(source.id)?;
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",
                params![key(source.id), encode(&value)?]).map_err(sql)?;
            tx.commit().map_err(sql)?;
            Ok(value)
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn seed(service: &WorkspaceService, root: PathBuf) -> Task {
        let project = service.add_local_workspace(root).await.unwrap();
        let task = service
            .create_task(
                project.id,
                "Source".into(),
                default_profiles()[0].id.clone(),
            )
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                ThreadEvent::TextDelta {
                    message_id: Some("u".into()),
                    role: Role::User,
                    text: "Investigate the observed issue".into(),
                },
            )
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                ThreadEvent::TextDelta {
                    message_id: Some("r".into()),
                    role: Role::Reasoning,
                    text: "hidden reasoning".into(),
                },
            )
            .await
            .unwrap();
        task
    }
    async fn generate(service: &WorkspaceService, child: &Task, text: &str) -> u64 {
        for event in [
            ThreadEvent::PromptStarted {
                turn: "recap-turn".into(),
            },
            ThreadEvent::TextDelta {
                message_id: Some("recap".into()),
                role: Role::Assistant,
                text: text.into(),
            },
            ThreadEvent::PromptFinished {
                reason: "end_turn".into(),
            },
        ] {
            service.record(child.thread_id, event).await.unwrap();
        }
        service.thread(child.thread_id).await.unwrap().last_sequence
    }
    #[tokio::test]
    async fn recap_generated_cache_is_source_owned_and_restart_inert() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.db");
        let service = WorkspaceService::open(db.clone()).await.unwrap();
        let source = seed(&service, root.path().into()).await;
        let before = service.thread(source.thread_id).await.unwrap();
        let review = service.review_recap(source.id).await.unwrap();
        assert!(review.context().contains("Create a concise thread recap"));
        assert!(!review.context().contains("hidden reasoning"));
        let child = service
            .create_handoff(review.clone(), review.context().into())
            .await
            .unwrap();
        assert!(
            service
                .thread(child.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
        assert!(service.session(child.thread_id).await.unwrap().is_none());
        assert!(service.save_thread_recap(child.id, 0).await.is_err());
        let seq = generate(
            &service,
            &child,
            "Objective, known evidence and next steps.",
        )
        .await;
        let recap = service.save_thread_recap(child.id, seq).await.unwrap();
        assert_eq!(recap.source, source.id);
        assert_eq!(recap.source_sequence, before.last_sequence);
        assert_eq!(
            service.thread(source.thread_id).await.unwrap().messages,
            before.messages
        );
        drop(service);
        let service = WorkspaceService::open(db).await.unwrap();
        assert_eq!(service.thread_recap(source.id).await.unwrap(), Some(recap));
        assert!(service.thread_recap(child.id).await.unwrap().is_none());
        assert!(service.session(child.thread_id).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn recap_rejects_stale_source_result_and_wrong_task_without_replacing_cache() {
        let root = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let source = seed(&service, root.path().into()).await;
        let review = service.review_recap(source.id).await.unwrap();
        let child = service
            .create_handoff(review.clone(), review.context().into())
            .await
            .unwrap();
        let seq = generate(&service, &child, "First valid summary").await;
        let original = service.save_thread_recap(child.id, seq).await.unwrap();
        assert!(service.save_thread_recap(child.id, seq - 1).await.is_err());
        assert!(service.save_thread_recap(source.id, 1).await.is_err());
        service
            .record(
                source.thread_id,
                ThreadEvent::Notice {
                    message: "Source changed".into(),
                },
            )
            .await
            .unwrap();
        assert!(service.save_thread_recap(child.id, seq).await.is_err());
        assert_eq!(
            service.thread_recap(source.id).await.unwrap(),
            Some(original)
        );
    }
    #[tokio::test]
    async fn recap_refuses_huge_output_and_empty_input() {
        let root = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let source = seed(&service, root.path().into()).await;
        let review = service.review_recap(source.id).await.unwrap();
        let child = service
            .create_handoff(review.clone(), review.context().into())
            .await
            .unwrap();
        assert!(service.review_recap(child.id).await.is_err());
        let seq = generate(&service, &child, &"x".repeat(MAX_RECAP + 1)).await;
        assert!(service.save_thread_recap(child.id, seq).await.is_err());
        assert!(service.thread_recap(source.id).await.unwrap().is_none());
    }
}
