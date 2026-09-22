//! Bounded thread-recap snapshots and a compare-and-swap cache, separate from
//! transcript events. A cached model summary never becomes source history.
use crate::{Store, WorkspaceError, WorkspaceResult, WorkspaceService, ProviderSettings, now_ms};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use synara_core::{TaskId, ProjectId, ThreadId, Role, TaskState};
use tokio_util::sync::CancellationToken;

const MAX_MESSAGES: usize = 64;
const MAX_SOURCE_BYTES: usize = 48 * 1024;
pub(crate) const MAX_RECAP_BYTES: usize = 32 * 1024;
const MAX_CACHE_BYTES: usize = 64 * 1024;

fn invalid(message: &str) -> WorkspaceError { WorkspaceError::Invalid(message.into()) }
fn digest(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }
fn valid_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max &&
        !s.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}
pub(crate) fn recap_key(id: TaskId) -> String { format!("task-recap:{id}") }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecapSource {
    pub task: TaskId,
    pub project: ProjectId,
    pub thread: ThreadId,
    pub root: PathBuf,
    pub sequence: u64,
    pub sha256: String,
    pub included_messages: usize,
    pub omitted_messages: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecapSnapshot {
    pub source: RecapSource,
    /// Exact bounded visible text, reviewed before any separate model request.
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadRecap {
    pub version: u32,
    pub revision: u64,
    pub source: RecapSource,
    pub text: String,
    pub provider_id: String,
    pub model_id: String,
    pub endpoint: String,
    pub profile_sha256: String,
    pub generated_at_ms: i64,
}
impl ThreadRecap {
    pub(crate) fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1 || self.revision == 0 || !bounded(&self.text, MAX_RECAP_BYTES)
            || !valid_hash(&self.source.sha256) || !valid_hash(&self.profile_sha256)
            || self.source.included_messages == 0 || self.source.included_messages > MAX_MESSAGES
            || self.source.omitted_messages > 200_000 || self.generated_at_ms < 0
            || !bounded(&self.provider_id, 256) || !bounded(&self.model_id, 512)
            || !bounded(&self.endpoint, 2048)
            || serde_json::to_vec(self).map_err(|_| invalid("Invalid recap."))?.len() > MAX_CACHE_BYTES
        { return Err(invalid("Unsupported, invalid or oversized recap cache. It was not overwritten.")); }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct RecapReview {
    pub snapshot: RecapSnapshot,
    pub cached: Option<ThreadRecap>,
    pub settings: ProviderSettings,
}
impl RecapReview {
    pub fn expected_revision(&self) -> u64 { self.cached.as_ref().map_or(0, |c| c.revision) }
}
pub(crate) fn snapshot(store: &Store, id: TaskId) -> WorkspaceResult<RecapSnapshot> {
    let task = store.task(id)?.ok_or(WorkspaceError::NotFound)?;
    let thread = store.replay(task.thread_id)?;
    let visible: Vec<_> = thread.messages.iter().filter(|m|
        matches!(m.role, Role::User | Role::Assistant) && !m.text.trim().is_empty()).collect();
    let mut chosen = Vec::new();
    let mut bytes = 0;
    // Whole messages only. Overlarge messages are explicitly counted as omitted,
    // not silently cut in half or replayed as original binary context.
    for message in visible.iter().rev() {
        if chosen.len() >= MAX_MESSAGES { break; }
        if message.text.len() > MAX_SOURCE_BYTES || message.id.len() > 512 { continue; }
        let role = if message.role == Role::User { "User" } else { "Assistant" };
        let entry = format!("[{role} / message {}]\n{}\n\n", message.id, message.text);
        if chosen.len() < MAX_MESSAGES && bytes + entry.len() <= MAX_SOURCE_BYTES {
            bytes += entry.len();
            chosen.push(entry);
        }
    }
    chosen.reverse();
    let text = chosen.concat();
    let identity = serde_json::to_vec(&(task.id, task.project_id, task.thread_id,
        &task.working_directory, thread.last_sequence, &text))
        .map_err(|_| invalid("Cannot fingerprint source conversation."))?;
    Ok(RecapSnapshot { source: RecapSource {
        task: task.id, project: task.project_id, thread: task.thread_id,
        root: task.working_directory, sequence: thread.last_sequence,
        sha256: digest(&identity), included_messages: chosen.len(),
        omitted_messages: visible.len().saturating_sub(chosen.len()),
    }, text })
}
pub(crate) fn cached(store: &Store, id: TaskId) -> WorkspaceResult<Option<ThreadRecap>> {
    let Some(raw) = store.preference_raw(&recap_key(id))? else { return Ok(None); };
    if raw.len() > MAX_CACHE_BYTES { return Err(invalid("Oversized recap cache. It was not overwritten.")); }
    let result: ThreadRecap = serde_json::from_str(&raw)
        .map_err(|_| invalid("Unreadable recap cache. It was not overwritten."))?;
    result.validate()?;
    if result.source.task != id { return Err(invalid("Recap cache belongs to another task.")); }
    Ok(Some(result))
}
pub(crate) fn assert_current(store: &Store, reviewed: &RecapSnapshot, expected: u64) -> WorkspaceResult<()> {
    let id = reviewed.source.task;
    let task = store.task(id)?.ok_or(WorkspaceError::NotFound)?;
    if matches!(task.state, TaskState::Archived | TaskState::Running | TaskState::Waiting) {
        return Err(invalid("Finish the active turn or pending interaction before generating a recap."));
    }
    if reviewed.text.is_empty() || snapshot(store, id)? != *reviewed {
        return Err(invalid("Conversation or task ownership changed. Reload and review the source again."));
    }
    if cached(store, id)?.as_ref().map_or(0, |c| c.revision) != expected {
        return Err(invalid("Another recap was saved. Reload before regenerating."));
    }
    Ok(())
}
impl WorkspaceService {
    pub async fn review_thread_recap(&self, id: TaskId) -> WorkspaceResult<RecapReview> {
        let settings = self.direct_model_settings().await?;
        self.access(move |store| Ok(RecapReview {
            snapshot: snapshot(store, id)?, cached: cached(store, id)?, settings,
        })).await
    }
    pub(crate) async fn check_recap_source(&self, reviewed: RecapSnapshot, expected: u64) -> WorkspaceResult<()> {
        self.access(move |store| assert_current(store, &reviewed, expected)).await
    }
    pub(crate) async fn finish_thread_recap(
        &self, reviewed: RecapSnapshot, expected: u64, mut result: ThreadRecap, cancel: CancellationToken,
    ) -> WorkspaceResult<ThreadRecap> {
        self.access(move |store| {
            if cancel.is_cancelled() { return Err(invalid("Recap cancelled. The previous cache is preserved.")); }
            assert_current(store, &reviewed, expected)?;
            result.source = reviewed.source;
            result.revision = expected.checked_add(1).ok_or_else(|| invalid("Recap revision exhausted."))?;
            result.generated_at_ms = now_ms().max(0);
            result.validate()?;
            store.set_preference(&recap_key(result.source.task), &result)?;
            Ok(result)
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synara_core::ThreadEvent;
    async fn seed(s: &WorkspaceService, root: &std::path::Path) -> TaskId {
        let p = s.add_local_workspace(root.to_owned()).await.unwrap();
        let agent = s.profiles().await.unwrap()[0].id.clone();
        let t = s.create_task(p.id, "Recap test".into(), agent).await.unwrap();
        s.record(t.thread_id, ThreadEvent::TextDelta { message_id: Some("user-1".into()),
            role: Role::User, text: "Investigate the timeout".into() }).await.unwrap();
        t.id
    }
    fn result(source: RecapSource) -> ThreadRecap {
        ThreadRecap { version: 1, revision: 1, source, text: "Context: investigate the timeout.".into(),
            provider_id: "fixture".into(), model_id: "fixture".into(),
            endpoint: "https://example.invalid/v1".into(), profile_sha256: "a".repeat(64), generated_at_ms: 0 }
    }
    #[tokio::test]
    async fn source_review_and_save_leave_transcript_inert_and_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.sqlite3");
        let s = WorkspaceService::open(path.clone()).await.unwrap();
        let id = seed(&s, dir.path()).await;
        let r = s.review_thread_recap(id).await.unwrap();
        assert!(r.cached.is_none());
        let saved = s.finish_thread_recap(r.snapshot.clone(), 0, result(r.snapshot.source.clone()), CancellationToken::new()).await.unwrap();
        assert_eq!(s.review_thread_recap(id).await.unwrap().snapshot, r.snapshot);
        drop(s);
        let s = WorkspaceService::open(path).await.unwrap();
        let reopened = s.review_thread_recap(id).await.unwrap();
        assert_eq!(reopened.cached, Some(saved));
        assert_eq!(reopened.snapshot, r.snapshot);
    }
    #[tokio::test]
    async fn cancellation_and_duplicate_save_preserve_previous_cache() {
        let dir = tempfile::tempdir().unwrap();
        let s = WorkspaceService::memory().unwrap();
        let id = seed(&s, dir.path()).await;
        let r = s.review_thread_recap(id).await.unwrap();
        let saved = s.finish_thread_recap(r.snapshot.clone(), 0, result(r.snapshot.source.clone()), CancellationToken::new()).await.unwrap();
        assert!(s.finish_thread_recap(r.snapshot.clone(), 0, result(r.snapshot.source.clone()), CancellationToken::new()).await.is_err());
        let cancel = CancellationToken::new(); cancel.cancel();
        assert!(s.finish_thread_recap(r.snapshot.clone(), 1, result(r.snapshot.source.clone()), cancel).await.is_err());
        assert_eq!(s.review_thread_recap(id).await.unwrap().cached, Some(saved));
    }
    #[tokio::test]
    async fn changed_source_and_forged_task_identity_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let s = WorkspaceService::memory().unwrap();
        let id = seed(&s, dir.path()).await;
        let r = s.review_thread_recap(id).await.unwrap();
        let mut forged = r.snapshot.clone();
        forged.source.task = TaskId::new();
        assert!(s.check_recap_source(forged, 0).await.is_err());
        s.record(r.snapshot.source.thread, ThreadEvent::TextDelta { message_id: Some("user-1".into()),
            role: Role::User, text: " with changed requirements".into() }).await.unwrap();
        assert!(s.finish_thread_recap(r.snapshot.clone(), 0, result(r.snapshot.source.clone()), CancellationToken::new()).await.is_err());
        assert!(s.review_thread_recap(id).await.unwrap().cached.is_none());
    }
    #[tokio::test]
    async fn bounds_exclude_hidden_roles_and_report_omitted_whole_messages() {
        let dir = tempfile::tempdir().unwrap();
        let s = WorkspaceService::memory().unwrap();
        let id = seed(&s, dir.path()).await;
        let thread = s.task(id).await.unwrap().thread_id;
        for i in 0..80 {
            s.record(thread, ThreadEvent::TextDelta { message_id: Some(format!("message-{i}")),
                role: Role::Assistant, text: format!("Visible message {i}") }).await.unwrap();
        }
        s.record(thread, ThreadEvent::TextDelta { message_id: Some("overlarge".into()),
            role: Role::Assistant, text: "x".repeat(MAX_SOURCE_BYTES + 1) }).await.unwrap();
        s.record(thread, ThreadEvent::TextDelta { message_id: Some("private-reasoning".into()),
            role: Role::Reasoning, text: "Hidden reasoning must not be shared".into() }).await.unwrap();
        let r = s.review_thread_recap(id).await.unwrap();
        assert!(!r.snapshot.text.contains("Hidden reasoning"));
        assert_eq!(r.snapshot.source.included_messages, 64);
        assert_eq!(r.snapshot.source.omitted_messages, 18);
        assert!(r.snapshot.text.len() <= MAX_SOURCE_BYTES);
        assert!(r.snapshot.text.contains("Visible message 79"));
        assert!(!r.snapshot.text.contains("[Assistant / message overlarge]"));
    }
    #[test]
    fn malformed_cached_output_and_provenance_fail_closed() {
        let source = RecapSource { task: TaskId::new(), project: ProjectId::new(), thread: ThreadId::new(),
            root: "/tmp".into(), sequence: 1, sha256: "b".repeat(64), included_messages: 1, omitted_messages: 0 };
        let mut r = result(source);
        assert!(r.validate().is_ok());
        r.text = " ".into(); assert!(r.validate().is_err());
        r.text = "x".repeat(MAX_RECAP_BYTES + 1); assert!(r.validate().is_err());
        r.text = "safe".into(); r.profile_sha256 = "fake".into(); assert!(r.validate().is_err());
    }
    #[tokio::test]
    async fn forged_preview_and_oversized_output_do_not_create_cache() {
        let dir = tempfile::tempdir().unwrap();
        let s = WorkspaceService::memory().unwrap();
        let id = seed(&s, dir.path()).await;
        let r = s.review_thread_recap(id).await.unwrap();
        let mut forged = r.snapshot.clone(); forged.text = "Unreviewed text".into();
        assert!(s.check_recap_source(forged, 0).await.is_err());
        let mut big = result(r.snapshot.source.clone()); big.text = "x".repeat(MAX_RECAP_BYTES + 1);
        assert!(s.finish_thread_recap(r.snapshot, 0, big, CancellationToken::new()).await.is_err());
        assert!(s.review_thread_recap(id).await.unwrap().cached.is_none());
    }
}
