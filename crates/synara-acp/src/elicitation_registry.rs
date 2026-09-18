//! Bounded URL-flow registrations. A completion expires only that question, never its parent RPC.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use synara_agent::{AgentError, AgentResult};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub(crate) struct Registry(Mutex<HashMap<String, Entry>>);
struct Entry {
    request_id: String,
    owner: CancellationToken,
    question: CancellationToken,
}
pub(crate) struct Registration {
    registry: Arc<Registry>,
    id: String,
    request_id: String,
    question: CancellationToken,
    accepted: bool,
}
impl Registry {
    pub fn register(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        owner: CancellationToken,
        question: CancellationToken,
    ) -> AgentResult<Registration> {
        let mut entries = self
            .0
            .lock()
            .map_err(|_| AgentError::Disconnected("question registry unavailable".into()))?;
        entries.retain(|_, entry| !entry.owner.is_cancelled());
        if entries.len() >= 64 || entries.contains_key(id) {
            return Err(AgentError::Limit);
        }
        entries.insert(
            id.into(),
            Entry {
                request_id: request_id.into(),
                owner,
                question: question.clone(),
            },
        );
        Ok(Registration {
            registry: self.clone(),
            id: id.into(),
            request_id: request_id.into(),
            question,
            accepted: false,
        })
    }
    pub fn complete(&self, id: &str) {
        if let Ok(mut entries) = self.0.lock()
            && let Some(entry) = entries.remove(id)
        {
            entry.question.cancel();
        }
    }
}
impl Registration {
    pub fn accepted(&mut self) {
        self.accepted = true;
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        self.question.cancel();
        if !self.accepted
            && let Ok(mut entries) = self.registry.0.lock()
            && entries
                .get(&self.id)
                .is_some_and(|entry| entry.request_id == self.request_id)
        {
            entries.remove(&self.id);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abandonment_decline_and_cancellation_do_not_leak_slots() {
        let registry = Arc::new(Registry::default());
        let owner = CancellationToken::new();
        for _ in 0..200 {
            let child = owner.child_token();
            drop(
                registry
                    .register("same-id", "question", owner.clone(), child.clone())
                    .unwrap(),
            );
            assert!(child.is_cancelled());
        }
        assert!(!owner.is_cancelled());
        assert!(registry.0.lock().unwrap().is_empty());
    }
    #[test]
    fn completion_and_reused_ids_cannot_cancel_a_new_question_or_parent_request() {
        let registry = Arc::new(Registry::default());
        let owner = CancellationToken::new();
        let old = registry
            .register("url", "old", owner.clone(), owner.child_token())
            .unwrap();
        registry.complete("url");
        let child = owner.child_token();
        let new = registry
            .register("url", "new", owner.clone(), child.clone())
            .unwrap();
        drop(old);
        assert!(!child.is_cancelled());
        assert!(registry.0.lock().unwrap().contains_key("url"));
        registry.complete("url");
        assert!(child.is_cancelled());
        assert!(!owner.is_cancelled());
        drop(new);
    }
    #[test]
    fn accepted_url_flows_are_bounded_and_expire_with_their_owner() {
        let registry = Arc::new(Registry::default());
        let owner = CancellationToken::new();
        for i in 0..64 {
            let mut registration = registry
                .register(
                    &i.to_string(),
                    "question",
                    owner.clone(),
                    owner.child_token(),
                )
                .unwrap();
            registration.accepted();
        }
        assert!(matches!(
            registry.register("overflow", "question", owner.clone(), owner.child_token()),
            Err(AgentError::Limit)
        ));
        owner.cancel();
        let next = CancellationToken::new();
        assert!(
            registry
                .register("next", "question", next.clone(), next.child_token())
                .is_ok()
        );
    }
}
