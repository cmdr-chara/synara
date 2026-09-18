//! Layout-independent, virtualized conversation presentation state.
//! Only explicit navigation or the list's user-controlled tail mode changes ownership.
use super::*;
use gpui::{FollowMode, ListAlignment, ListOffset, ListState};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum RowKey {
    Message(String, u8),
    Tool(String),
    Permission(String),
    Input(String),
    Notice(usize),
    Plan,
    Empty,
}
fn role_key(role: Role) -> u8 {
    match role {
        Role::User => 0,
        Role::Assistant => 1,
        Role::Reasoning => 2,
    }
}
fn row_key(thread: &Thread, index: usize) -> RowKey {
    match &thread.timeline[index] {
        TranscriptItem::Message { index } => {
            let message = &thread.messages[*index];
            RowKey::Message(message.id.clone(), role_key(message.role))
        }
        TranscriptItem::Tool { id } => RowKey::Tool(id.clone()),
        TranscriptItem::Permission { id } => RowKey::Permission(id.clone()),
        TranscriptItem::Input { id } => RowKey::Input(id.clone()),
        TranscriptItem::Notice { .. } => RowKey::Notice(index),
    }
}

pub(super) struct TranscriptState {
    pub list: ListState,
    thread: Option<ThreadId>,
    rows: Vec<RowKey>,
    indices: HashMap<RowKey, usize>,
    timeline_len: usize,
}
impl TranscriptState {
    pub fn new() -> Self {
        let list = ListState::new(0, ListAlignment::Top, px(160.));
        list.set_follow_mode(FollowMode::Tail);
        Self {
            list,
            thread: None,
            rows: vec![],
            indices: HashMap::new(),
            timeline_len: 0,
        }
    }
    pub fn follow(&self) {
        self.list.set_follow_mode(FollowMode::Tail);
    }
    pub fn is_following(&self) -> bool {
        self.list.is_following_tail()
    }
    pub fn sync(&mut self, thread: &Thread, event: Option<&ThreadEvent>) {
        let same_thread = self.thread == Some(thread.id);
        let rebuilding = !same_thread
            || event.is_none()
            || thread.timeline.len() < self.timeline_len
            || matches!(
                event,
                Some(
                    ThreadEvent::HistoryStarted
                        | ThreadEvent::HistoryCompleted
                        | ThreadEvent::Error {
                            recoverable: false,
                            ..
                        }
                )
            );
        if rebuilding {
            let anchor = self.list.logical_scroll_top();
            let anchor_key = self.rows.get(anchor.item_ix).cloned();
            let following = !same_thread || self.is_following();
            let mut rows: Vec<_> = (0..thread.timeline.len())
                .map(|i| row_key(thread, i))
                .collect();
            if !thread.plan.is_empty() {
                rows.push(RowKey::Plan);
            }
            if rows.is_empty() {
                rows.push(RowKey::Empty);
            }
            if self.rows == rows {
                self.list.remeasure_items(0..rows.len());
            } else {
                self.list.splice(0..self.rows.len(), rows.len());
            }
            self.indices = rows
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, key)| (key, i))
                .collect();
            self.rows = rows;
            if following {
                self.follow();
            } else if let Some(index) = anchor_key.and_then(|key| self.indices.get(&key).copied()) {
                self.list.scroll_to(ListOffset {
                    item_ix: index,
                    offset_in_item: anchor.offset_in_item,
                });
            } else {
                self.list.scroll_to(ListOffset {
                    item_ix: anchor.item_ix.min(self.rows.len().saturating_sub(1)),
                    offset_in_item: px(0.),
                });
            }
        } else {
            // Append only the new rows, retaining cached heights and the user's anchor.
            // A plan is a stable trailing row, so insert new transcript rows before it.
            let added = thread.timeline.len() - self.timeline_len;
            if added > 0 {
                if self.rows == [RowKey::Empty] {
                    self.list.splice(0..1, 0);
                    self.rows.clear();
                    self.indices.clear();
                }
                self.list
                    .splice(self.timeline_len..self.timeline_len, added);
                let new_rows: Vec<_> = (self.timeline_len..thread.timeline.len())
                    .map(|i| row_key(thread, i))
                    .collect();
                for (offset, key) in new_rows.iter().enumerate() {
                    self.indices.insert(key.clone(), self.timeline_len + offset);
                }
                self.rows
                    .splice(self.timeline_len..self.timeline_len, new_rows);
            }
            if !thread.plan.is_empty() && self.rows == [RowKey::Empty] {
                self.list.splice(0..1, 0);
                self.rows.clear();
                self.indices.clear();
            }
            let had_plan = self.indices.contains_key(&RowKey::Plan);
            let has_plan = !thread.plan.is_empty();
            match (had_plan, has_plan) {
                (false, true) => {
                    self.list.splice(self.rows.len()..self.rows.len(), 1);
                    self.rows.push(RowKey::Plan);
                }
                (true, false) => {
                    let index = self.rows.len() - 1;
                    self.list.splice(index..index + 1, 0);
                    self.rows.pop();
                    self.indices.remove(&RowKey::Plan);
                }
                _ => {}
            }
            if has_plan {
                self.indices.insert(RowKey::Plan, self.rows.len() - 1);
            }
            if self.rows.is_empty() {
                self.rows.push(RowKey::Empty);
                self.indices.insert(RowKey::Empty, 0);
                self.list.splice(0..0, 1);
            }
        }
        self.thread = Some(thread.id);
        self.timeline_len = thread.timeline.len();
        match event {
            Some(ThreadEvent::TextDelta {
                message_id: Some(id),
                role,
                ..
            }) => self.invalidate(&RowKey::Message(id.clone(), role_key(*role))),
            Some(ThreadEvent::TextDelta {
                message_id: None, ..
            }) => {
                if let Some(key) = thread
                    .timeline
                    .len()
                    .checked_sub(1)
                    .map(|i| row_key(thread, i))
                {
                    self.invalidate(&key);
                }
            }
            Some(ThreadEvent::ToolChanged { patch }) => {
                self.invalidate(&RowKey::Tool(patch.id.clone()))
            }
            Some(ThreadEvent::PermissionRequested { request }) => {
                self.invalidate(&RowKey::Permission(request.id.clone()))
            }
            Some(ThreadEvent::PermissionResolved { id, .. }) => {
                self.invalidate(&RowKey::Permission(id.clone()))
            }
            Some(ThreadEvent::UserInputRequested { request }) => {
                self.invalidate(&RowKey::Input(request.id.clone()))
            }
            Some(ThreadEvent::UserInputResolved { id }) => {
                self.invalidate(&RowKey::Input(id.clone()))
            }
            Some(ThreadEvent::PlanChanged { .. }) => self.invalidate(&RowKey::Plan),
            Some(
                ThreadEvent::TerminalOutput { .. }
                | ThreadEvent::CancellationRequested
                | ThreadEvent::PromptFinished { .. },
            ) => self.list.remeasure_items(0..self.rows.len()),
            _ => {}
        }
        debug_assert_eq!(self.list.item_count(), self.rows.len());
    }
    pub fn interaction_changed(&self, key: &InteractionKey) {
        if self.thread == Some(key.0) {
            self.invalidate(&RowKey::Permission(key.1.clone()));
            self.invalidate(&RowKey::Input(key.1.clone()));
        }
    }
    fn invalidate(&self, key: &RowKey) {
        if let Some(index) = self.indices.get(key) {
            self.list.remeasure_items(*index..*index + 1);
        }
    }
}
impl Shell {
    pub(super) fn virtual_transcript(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let view = cx.entity().downgrade();
        let notify = view.clone();
        self.transcript.list.set_scroll_handler(move |_, _, cx| {
            let _ = notify.update(cx, |_, cx| cx.notify());
        });
        gpui::list(self.transcript.list.clone(), move |index, _, cx| {
            view.update(cx, |this, cx| {
                let Some(thread) = &this.thread else {
                    return div().into_any_element();
                };
                let row = if index < thread.timeline.len() {
                    this.transcript_item(thread, index, cx)
                } else if !thread.plan.is_empty() {
                    div()
                        .p_3()
                        .rounded_md()
                        .bg(rgb(0x1a2633))
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Plan"))
                        .children(thread.plan.iter().map(|entry| {
                            div()
                                .mt_1()
                                .child(format!("{} · {}", entry.status, entry.text))
                        }))
                        .into_any_element()
                } else {
                    div()
                        .p_6()
                        .rounded_lg()
                        .bg(rgb(0x17202c))
                        .child(
                            "Ready when you are. Your conversation will be saved on this computer.",
                        )
                        .into_any_element()
                };
                div()
                    .id(("timeline-row", index))
                    .pb_4()
                    .child(row)
                    .into_any_element()
            })
            .unwrap_or_else(|_| div().into_any_element())
        })
        .flex_1()
        .min_h_0()
        .px_5()
        .py_4()
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_transcript_growth_does_not_move_a_user_owned_anchor() {
        let mut thread = Thread::new(ThreadId::new());
        thread.timeline = (0..10_000)
            .map(|i| TranscriptItem::Notice {
                text: format!("Row {i}"),
                is_error: false,
            })
            .collect();
        let mut state = TranscriptState::new();
        state.sync(&thread, None);
        assert_eq!(state.list.item_count(), 10_000);
        state.list.scroll_to(ListOffset {
            item_ix: 45,
            offset_in_item: px(12.),
        });
        thread.timeline.push(TranscriptItem::Notice {
            text: "New output".into(),
            is_error: false,
        });
        state.sync(
            &thread,
            Some(&ThreadEvent::Notice {
                message: "New output".into(),
            }),
        );
        assert!(!state.is_following());
        let offset = state.list.logical_scroll_top();
        assert_eq!(offset.item_ix, 45);
        assert_eq!(offset.offset_in_item, px(12.));
        assert_eq!(state.list.item_count(), 10_001);
        state.sync(&thread, None);
        let offset = state.list.logical_scroll_top();
        assert_eq!(offset.item_ix, 45);
        assert_eq!(offset.offset_in_item, px(12.));
        state.follow();
        assert!(state.is_following());
        assert_eq!(state.list.logical_scroll_top().item_ix, 10_001);
    }
    #[test]
    fn stream_remeasurement_preserves_offset_and_duplicate_hydration_does_not_add_rows() {
        let mut thread = Thread::new(ThreadId::new());
        thread.messages.push(Message {
            id: "answer".into(),
            role: Role::Assistant,
            text: "before".into(),
        });
        thread.timeline.push(TranscriptItem::Message { index: 0 });
        let mut state = TranscriptState::new();
        state.sync(&thread, None);
        state.list.scroll_to(ListOffset {
            item_ix: 0,
            offset_in_item: px(8.),
        });
        thread.messages[0].text.push_str(" after");
        state.sync(
            &thread,
            Some(&ThreadEvent::TextDelta {
                message_id: Some("answer".into()),
                role: Role::Assistant,
                text: " after".into(),
            }),
        );
        assert_eq!(state.list.logical_scroll_top().offset_in_item, px(8.));
        assert!(!state.is_following());
        state.sync(&thread, None);
        assert_eq!(state.list.item_count(), 1);
        let replacement = Thread::new(ThreadId::new());
        state.sync(&replacement, None);
        assert!(state.is_following());
    }
}
