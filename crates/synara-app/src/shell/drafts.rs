//! Debounced per-chat drafts. Serialized writes cannot overtake newer edits.
use super::*;
use std::time::{Duration, Instant};
#[derive(Default)]
pub(super) struct DraftState {
    dirty: HashMap<TaskId, Instant>,
    saving: HashSet<TaskId>,
    failed: HashSet<TaskId>,
    pub loading: HashSet<TaskId>,
    versions: HashMap<TaskId, u64>,
    sent: HashMap<TaskId, (u64, String)>,
    display: HashMap<TaskId, String>,
    pub quitting: bool,
}
impl DraftState {
    pub fn pending_for(&self, id: TaskId) -> bool {
        self.dirty.contains_key(&id) || self.saving.contains(&id) || self.loading.contains(&id)
    }
    pub fn forget_task(&mut self, id: TaskId) {
        self.dirty.remove(&id); self.saving.remove(&id); self.failed.remove(&id);
        self.loading.remove(&id); self.versions.remove(&id); self.sent.remove(&id); self.display.remove(&id);
    }

    pub fn version(&self, id: TaskId) -> u64 { self.versions.get(&id).copied().unwrap_or(0) }
    fn changed(&mut self, id: TaskId) {
        self.dirty.insert(id, Instant::now());
        self.failed.remove(&id);
        let version = self.versions.entry(id).or_default();
        *version = version.wrapping_add(1);
    }
    pub fn submitted(&mut self, id: TaskId, text: String) {
        self.display.remove(&id);
        self.sent
            .insert(id, (self.versions.get(&id).copied().unwrap_or(0), text));
    }
    pub fn submitted_with_display(&mut self, id: TaskId, text: String, display: String) {
        self.submitted(id, text);
        self.display.insert(id, display);
    }
    fn accepted(&mut self, id: TaskId, text: &str) -> bool {
        let Some((version, sent)) = self.sent.get(&id) else {
            return false;
        };
        if self.display.get(&id).unwrap_or(sent) != text {
            return false;
        }
        let unchanged = *version == self.versions.get(&id).copied().unwrap_or(0);
        self.sent.remove(&id);
        self.display.remove(&id);
        unchanged
    }
    fn due(&mut self, force: bool) -> Vec<TaskId> {
        let ids: Vec<_> = self
            .dirty
            .iter()
            .filter_map(|(id, at)| {
                (!self.saving.contains(id)
                    && (force
                        || (!self.failed.contains(id)
                            && at.elapsed() >= Duration::from_millis(500))))
                .then_some(*id)
            })
            .collect();
        for id in &ids {
            self.dirty.remove(id);
            self.saving.insert(*id);
        }
        ids
    }
    fn completed(&mut self, id: TaskId, failed: bool) {
        self.saving.remove(&id);
        if failed {
            self.failed.insert(id);
            self.dirty.entry(id).or_insert_with(Instant::now);
        }
    }
    fn pending(&self) -> bool {
        !self.dirty.is_empty() || !self.saving.is_empty()
    }
}
impl Shell {
    pub(super) fn draft_close_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let cancel = cx.listener(|this: &mut Self, _: &(), _, cx| {
            this.draft_state.quitting = false;
            this.environment.quitting = false;
            this.close.cancel();
            this.focus_composer = this.panel == Panel::Conversation;
            cx.notify();
        });
        div()
            .id("saving-chat-drafts")
            .role(gpui::Role::Dialog)
            .aria_label(if self.environment.quitting {
                "Saving Environment layout before closing"
            } else {
                "Saving chat drafts before closing"
            })
            .track_focus(&self.close_focus)
            .tab_group()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .bg(rgb(crate::ui::palette().canvas))
            .text_color(rgb(crate::ui::palette().text))
            .child(if self.environment.quitting {
                "Saving Environment layout..."
            } else {
                "Saving chat drafts..."
            })
            .child(crate::ui::action(
                "cancel-draft-close",
                "Keep Synara open",
                None,
                false,
                cancel,
            ))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.draft_state.quitting = false;
                    this.environment.quitting = false;
                    this.close.cancel();
                    this.focus_composer = this.panel == Panel::Conversation;
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .into_any_element()
    }
    pub(super) fn remember_draft(&mut self, cx: &mut Context<Self>) {
        self.goal_input_changed(cx);
        if let Some(id) = self.selected {
            self.store_draft(id, self.composer.read(cx).text().to_owned());
        }
        cx.notify();
    }

    pub(super) fn remember_task_draft(
        &mut self,
        id: TaskId,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.store_draft(id, text);
        cx.notify();
    }
    pub(super) fn snapshot_draft(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.selected
            && (self.drafts.contains_key(&id) || !self.composer.read(cx).text().is_empty())
        {
            self.remember_draft(cx);
        }
    }
    fn store_draft(&mut self, id: TaskId, text: String) {
        if self.drafts.get(&id) != Some(&text) {
            self.drafts.insert(id, text);
            self.draft_state.changed(id);
        }
    }
    pub(super) fn load_draft(&mut self, id: TaskId) {
        if self.drafts.contains_key(&id) || !self.draft_state.loading.insert(id) {
            return;
        }
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::DraftLoaded(
                id,
                workspace.task_draft(id).await.map_err(|e| e.to_string()),
            ))
        });
    }
    pub(super) fn restore_draft(
        &mut self,
        id: TaskId,
        result: Result<String, String>,
        cx: &mut Context<Self>,
    ) {
        self.draft_state.loading.remove(&id);
        // An intentional clear is also newer local data. A placeholder is not.
        if self.drafts.contains_key(&id) {
            return;
        }
        match result {
            Ok(text) => {
                if self.selected == Some(id) {
                    self.composer
                        .update(cx, |e, cx| e.set_text(text.clone(), cx));
                }
                self.drafts.insert(id, text);
            }
            Err(error) if self.selected == Some(id) => {
                self.error = Some(format!("Could not restore chat draft: {error}"))
            }
            _ => {}
        }
        cx.notify();
    }
    pub(super) fn acknowledge_draft(&mut self, envelope: &EventEnvelope, cx: &mut Context<Self>) {
        let ThreadEvent::TextDelta {
            role: Role::User,
            text,
            ..
        } = &envelope.event
        else {
            return;
        };
        let Some(id) = self
            .catalog
            .tasks
            .iter()
            .find(|t| t.thread_id == envelope.thread_id)
            .map(|t| t.id)
        else {
            return;
        };
        let original = self.draft_state.sent.get(&id).map(|(_, text)| text.clone());
        if self.draft_state.accepted(id, text)
            && self.drafts.get(&id).is_some_and(|draft| Some(draft) == original.as_ref())
        {
            self.store_draft(id, String::new());
            if self.selected == Some(id) {
                self.composer
                    .update(cx, |e, cx| e.set_text(String::new(), cx));
            }
            if self.side_chats.selected == Some(id) {
                self.side_chats
                    .composer
                    .update(cx, |e, cx| e.set_text(String::new(), cx));
            }
            self.flush_drafts(true);
        }
    }
    pub(super) fn flush_drafts(&mut self, force: bool) {
        self.draft_state
            .dirty
            .retain(|id, _| self.catalog.tasks.iter().any(|t| t.id == *id));
        for id in self.draft_state.due(force) {
            let text = self.drafts.get(&id).cloned().unwrap_or_default();
            let workspace = self.controller.workspace.clone();
            self.job(async move {
                Ok(Update::DraftSaved(
                    id,
                    workspace
                        .save_task_draft(id, text)
                        .await
                        .err()
                        .map(|e| e.to_string()),
                ))
            });
        }
    }
    pub(super) fn draft_saved(
        &mut self,
        id: TaskId,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let exists = self.catalog.tasks.iter().any(|t| t.id == id);
        self.draft_state.completed(id, error.is_some() && exists);
        if let Some(error) = error.filter(|_| exists) {
            self.error = Some(format!(
                "Draft save failed. Your text is still here: {error}"
            ));
            self.draft_state.quitting = false;
            self.close.cancel();
        } else if self.draft_state.quitting {
            self.flush_drafts(true);
            if !self.draft_state.pending() {
                self.draft_state.quitting = false;
                self.begin_quit(cx);
            }
        }
        cx.notify();
    }
    pub(super) fn save_drafts_before_quit(&mut self, cx: &mut Context<Self>) -> bool {
        self.snapshot_draft(cx);
        if !self.draft_state.pending() {
            return false;
        }
        self.draft_state.quitting = true;
        self.flush_drafts(true);
        if !self.draft_state.pending() {
            self.draft_state.quitting = false;
            return false;
        }
        cx.notify();
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rich_prompt_echo_clears_only_the_original_unedited_text_draft() {
        let id = TaskId::new();
        let mut state = DraftState::default();
        state.changed(id);
        state.submitted_with_display(id, "Review".into(), "Review\nAttached file: a.png\n[Image]".into());
        assert!(!state.accepted(id, "Review"));
        assert!(state.accepted(id, "Review\nAttached file: a.png\n[Image]"));
        state.submitted_with_display(id, "Review".into(), "display".into());
        state.changed(id);
        assert!(!state.accepted(id, "display"));
    }
    #[test]
    fn edits_during_save_are_coalesced_without_reordering() {
        let id = TaskId::new();
        let mut s = DraftState::default();
        s.changed(id);
        assert_eq!(s.due(true), vec![id]);
        s.changed(id);
        s.changed(id);
        assert!(s.due(true).is_empty());
        s.completed(id, false);
        assert_eq!(s.due(true), vec![id]);
        s.completed(id, false);
        assert!(!s.pending());
    }
    #[test]
    fn save_errors_keep_text_pending_without_a_retry_storm() {
        let id = TaskId::new();
        let mut s = DraftState::default();
        s.changed(id);
        s.due(true);
        s.completed(id, true);
        assert!(s.pending());
        assert!(s.due(false).is_empty());
        assert_eq!(s.due(true), vec![id]);
    }
    #[test]
    fn late_echo_cannot_clear_new_edits_even_when_text_was_retyped_identically() {
        let id = TaskId::new();
        let mut s = DraftState::default();
        s.changed(id);
        s.submitted(id, "hello".into());
        s.changed(id);
        s.changed(id);
        assert!(!s.accepted(id, "hello"));
        s.submitted(id, "hello".into());
        assert!(s.accepted(id, "hello"));
        assert!(!s.accepted(id, "hello"));
    }
    #[test]
    fn another_chat_or_text_cannot_acknowledge_a_pending_draft() {
        let id = TaskId::new();
        let mut s = DraftState::default();
        s.submitted(id, "hello".into());
        assert!(!s.accepted(TaskId::new(), "hello"));
        assert!(!s.accepted(id, "other"));
        assert!(s.accepted(id, "hello"));
    }
}
