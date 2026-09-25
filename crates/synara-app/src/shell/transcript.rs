//! Virtualized presentation rows, grouped without changing the durable transcript.
mod search;
use super::*;
use gpui::{FollowMode, ListAlignment, ListOffset, ListState};
use std::time::Instant;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum RowKey {
    Message(String, u8),
    Tool(String),
    Permission(String),
    Input(String),
    Notice(usize),
    Activity(String),
    Plan,
    Empty,
}
#[derive(Clone, Debug)]
enum RenderRow {
    Timeline(usize),
    Activity(usize),
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
fn projected_rows(thread: &Thread) -> Vec<(RowKey, RenderRow)> {
    // Build turn ownership once. Hidden tool/reasoning rows never enter ListState,
    // so even a long collapsed work log costs one measured row per turn.
    let mut ownership = vec![None; thread.timeline.len()];
    let answers: Vec<_> = thread
        .turns
        .iter()
        .map(|turn| super::activity::answer_index(thread, turn))
        .collect();
    for (turn_index, turn) in thread.turns.iter().enumerate() {
        if let Some(anchor) = super::activity::activity_anchor(thread, turn) {
            for owner in ownership
                .iter_mut()
                .take(turn.end_timeline_index)
                .skip(turn.first_timeline_index)
            {
                *owner = Some((turn_index, anchor));
            }
        }
    }
    let mut rows = vec![];
    for (index, item) in thread.timeline.iter().enumerate() {
        if let Some((turn, anchor)) = ownership[index]
            && anchor == index
        {
            rows.push((
                RowKey::Activity(thread.turns[turn].id.clone()),
                RenderRow::Activity(turn),
            ));
        }
        let visible = match item {
            TranscriptItem::Tool { .. } => ownership[index].is_none(),
            TranscriptItem::Message { index: message } => {
                ownership[index].is_none_or(|(turn, _)| match thread.messages[*message].role {
                    Role::User => true,
                    Role::Assistant => answers[turn] == Some(index),
                    Role::Reasoning => false,
                })
            }
            TranscriptItem::Permission { id } => thread.permissions.contains_key(id),
            TranscriptItem::Input { id } => thread.inputs.contains_key(id),
            _ => true,
        };
        if visible {
            rows.push((row_key(thread, index), RenderRow::Timeline(index)));
        }
    }
    if !thread.plan.is_empty() {
        rows.push((RowKey::Plan, RenderRow::Plan));
    }
    if rows.is_empty() {
        rows.push((RowKey::Empty, RenderRow::Empty));
    }
    rows
}

pub(super) struct TranscriptState {
    pub list: ListState,
    thread: Option<ThreadId>,
    rows: Vec<RowKey>,
    render_rows: Vec<RenderRow>,
    indices: HashMap<RowKey, usize>,
    timeline_len: usize,
    message_entries: HashMap<String, Instant>,
}
impl TranscriptState {
    pub fn new() -> Self {
        let list = ListState::new(0, ListAlignment::Top, px(160.));
        list.set_follow_mode(FollowMode::Tail);
        Self {
            list,
            thread: None,
            rows: vec![],
            render_rows: vec![],
            indices: HashMap::new(),
            timeline_len: 0,
            message_entries: HashMap::new(),
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
        if !same_thread
            || matches!(
                event,
                Some(ThreadEvent::HistoryStarted | ThreadEvent::HistoryCompleted)
            )
        {
            self.message_entries.clear();
        }
        if same_thread && event.is_some() && thread.timeline.len() > self.timeline_len {
            for index in self.timeline_len..thread.timeline.len() {
                if let RowKey::Message(id, 0) = row_key(thread, index) {
                    self.message_entries.insert(id, Instant::now());
                }
            }
        }
        let topology_changed = !same_thread
            || event.is_none()
            || thread.timeline.len() != self.timeline_len
            || matches!(
                event,
                Some(
                    ThreadEvent::PlanChanged { .. }
                        | ThreadEvent::PermissionResolved { .. }
                        | ThreadEvent::UserInputResolved { .. }
                        | ThreadEvent::PromptFinished { .. }
                        | ThreadEvent::HistoryStarted
                        | ThreadEvent::HistoryCompleted
                        | ThreadEvent::Error { .. }
                )
            );
        if topology_changed {
            let anchor = self.list.logical_scroll_top();
            let anchor_key = self.rows.get(anchor.item_ix).cloned();
            let following = !same_thread || self.is_following();
            let (rows, render_rows): (Vec<_>, Vec<_>) = projected_rows(thread).into_iter().unzip();
            let prefix = self
                .rows
                .iter()
                .zip(&rows)
                .take_while(|(a, b)| a == b)
                .count();
            let suffix = self.rows[prefix..]
                .iter()
                .rev()
                .zip(rows[prefix..].iter().rev())
                .take_while(|(a, b)| a == b)
                .count();
            if self.rows != rows {
                self.list.splice(
                    prefix..self.rows.len() - suffix,
                    rows.len() - prefix - suffix,
                );
            }
            self.indices = rows
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, key)| (key, i))
                .collect();
            self.rows = rows;
            self.render_rows = render_rows;
            if event.is_none() || !same_thread {
                self.list.remeasure_items(0..self.rows.len());
            }
            if following {
                self.follow();
            } else {
                let index = anchor_key
                    .and_then(|key| self.indices.get(&key).copied())
                    .unwrap_or_else(|| anchor.item_ix.min(self.rows.len().saturating_sub(1)));
                self.list.scroll_to(ListOffset {
                    item_ix: index,
                    offset_in_item: anchor.offset_in_item,
                });
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
                if let Some(index) = thread.timeline.len().checked_sub(1) {
                    self.invalidate(&row_key(thread, index));
                }
            }
            Some(ThreadEvent::ImageMessage { .. }) => self.invalidate_media(),
            Some(ThreadEvent::ToolChanged { patch }) => {
                self.invalidate(&RowKey::Tool(patch.id.clone()))
            }
            Some(ThreadEvent::PermissionRequested { request }) => {
                self.invalidate(&RowKey::Permission(request.id.clone()))
            }
            Some(ThreadEvent::UserInputRequested { request }) => {
                self.invalidate(&RowKey::Input(request.id.clone()))
            }
            Some(ThreadEvent::PlanChanged { .. }) => self.invalidate(&RowKey::Plan),
            Some(
                ThreadEvent::PromptFinished { .. }
                | ThreadEvent::CancellationRequested
                | ThreadEvent::TerminalOutput { .. },
            ) => self.list.remeasure_items(0..self.rows.len()),
            _ => {}
        }
        if let Some(turn) = thread.turns.last() {
            self.invalidate_activity(&turn.id);
        }
        debug_assert_eq!(self.list.item_count(), self.rows.len());
    }
    pub fn interaction_changed(&self, key: &InteractionKey) {
        if self.thread == Some(key.0) {
            self.invalidate(&RowKey::Permission(key.1.clone()));
            self.invalidate(&RowKey::Input(key.1.clone()));
        }
    }
    pub fn invalidate_media(&self) {
        self.list.remeasure_items(0..self.rows.len());
    }
    pub fn invalidate_activity(&self, turn: &str) {
        self.invalidate(&RowKey::Activity(turn.into()));
    }
    pub fn message_progress(&self, id: &str, now: Instant) -> f32 {
        self.message_entries.get(id).map_or(1., |started| {
            crate::ui::motion::ease_out(
                now.saturating_duration_since(*started).as_secs_f32()
                    / crate::ui::motion::message_duration().as_secs_f32(),
            )
        })
    }
    pub fn advance_animations(&mut self, now: Instant) -> bool {
        for id in self.message_entries.keys() {
            self.invalidate(&RowKey::Message(id.clone(), 0));
        }
        self.message_entries.retain(|_, started| {
            now.saturating_duration_since(*started) < crate::ui::motion::message_duration()
        });
        !self.message_entries.is_empty()
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
                let Some(description) = this.transcript.render_rows.get(index) else {
                    return div().into_any_element();
                };
                let spacing = match *description {
                    RenderRow::Activity(_) => 0.,
                    RenderRow::Timeline(row) => match &thread.timeline[row] {
                        TranscriptItem::Message { index }
                            if thread.messages[*index].role == Role::User =>
                        {
                            40.
                        }
                        TranscriptItem::Message { .. } => 8.,
                        _ => 24.,
                    },
                    _ => 24.,
                };
                let row = match *description {
                    RenderRow::Timeline(index) => this.transcript_item(thread, index, cx),
                    RenderRow::Activity(turn) => this.activity_summary(thread, turn, cx),
                    RenderRow::Plan => div()
                        .p_3()
                        .rounded_md()
                        .bg(rgb(crate::ui::palette().overlay))
                        .child("Plan")
                        .children(thread.plan.iter().map(|entry| {
                            div()
                                .mt_1()
                                .child(format!("{} · {}", entry.status, entry.text))
                        }))
                        .into_any_element(),
                    RenderRow::Empty => div().into_any_element(),
                };
                div()
                    .id(("timeline-row", index))
                    .pl_6()
                    .pr(px(34.))
                    .pb(px(spacing))
                    .child(row)
                    .into_any_element()
            })
            .unwrap_or_else(|_| div().into_any_element())
        })
        .flex_1()
        .min_h_0()
        .w_full()
        .max_w(px(crate::ui::chat_width() + 48.))
        .mx_auto()
        .py_6()
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_groups_intermediate_assistant_updates_with_work_and_keeps_one_final_answer() {
        let id = ThreadId::new();
        let events = vec![
            ThreadEvent::PromptStarted { turn: "one".into() },
            ThreadEvent::TextDelta {
                message_id: Some("user".into()),
                role: Role::User,
                text: "hello".into(),
            },
            ThreadEvent::TextDelta {
                message_id: Some("checking".into()),
                role: Role::Assistant,
                text: "Checking the workspace.".into(),
            },
            ThreadEvent::TextDelta {
                message_id: Some("progress".into()),
                role: Role::Assistant,
                text: "Reading the project.".into(),
            },
            ThreadEvent::TextDelta {
                message_id: Some("reasoning".into()),
                role: Role::Reasoning,
                text: "A separate work detail.".into(),
            },
            ThreadEvent::TextDelta {
                message_id: Some("final".into()),
                role: Role::Assistant,
                text: "Hello from Synara.".into(),
            },
            ThreadEvent::PromptFinished {
                reason: "end_turn".into(),
            },
        ];
        let envelopes: Vec<_> = events
            .into_iter()
            .enumerate()
            .map(|(index, event)| EventEnvelope {
                id: EventId::new(),
                thread_id: id,
                sequence: index as u64 + 1,
                timestamp_ms: index as i64 * 1000,
                event,
            })
            .collect();
        let mut live = Thread::new(id);
        let mut state = TranscriptState::new();
        for envelope in &envelopes {
            live.apply(envelope).unwrap();
            state.sync(&live, Some(&envelope.event));
        }
        assert_eq!(
            state.rows,
            vec![
                RowKey::Message("user".into(), 0),
                RowKey::Activity("one".into()),
                RowKey::Message("final".into(), 1)
            ]
        );
        assert_eq!(
            live.messages.len(),
            5,
            "Collapsing must never discard the actual messages"
        );
        let mut replay = Thread::new(id);
        for envelope in &envelopes {
            replay.apply(envelope).unwrap();
        }
        let restored = projected_rows(&replay)
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();
        assert_eq!(restored, state.rows);
        assert_eq!(
            super::super::activity::answer_index(&replay, &replay.turns[0]),
            Some(4)
        );
    }

    #[test]
    fn collapsed_tools_do_not_allocate_hidden_rows_or_hide_pending_permissions() {
        let mut thread = Thread::new(ThreadId::new());
        thread.timeline = (0..10_000)
            .map(|i| TranscriptItem::Tool {
                id: format!("tool-{i}"),
            })
            .collect();
        thread.timeline.push(TranscriptItem::Permission {
            id: "approval".into(),
        });
        thread.permissions.insert(
            "approval".into(),
            PermissionRequest {
                id: "approval".into(),
                tool_id: None,
                title: "Run".into(),
                choices: vec![],
            },
        );
        thread.turns.push(TurnSummary {
            id: "one".into(),
            started_at_ms: 0,
            finished_at_ms: None,
            first_timeline_index: 0,
            end_timeline_index: thread.timeline.len(),
            failed: false,
            direct_provider_id: None,
            direct_model_id: None,
            usage: None,
        });
        let rows = projected_rows(&thread);
        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0].1, RenderRow::Activity(0)));
        assert_eq!(rows[1].0, RowKey::Permission("approval".into()));
        thread.permissions.clear();
        assert_eq!(projected_rows(&thread).len(), 1);
        thread.messages.push(Message {
            id: "answer".into(),
            role: Role::Assistant,
            text: "Complete".into(),
        });
        thread.timeline.push(TranscriptItem::Message { index: 0 });
        thread.turns[0].end_timeline_index = thread.timeline.len();
        let rows = projected_rows(&thread);
        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0].1, RenderRow::Activity(0)));
        assert_eq!(rows[1].0, RowKey::Message("answer".into(), 1));
    }

    #[test]
    fn live_send_animation_survives_same_thread_refresh_but_history_does_not_animate() {
        let mut thread = Thread::new(ThreadId::new());
        let mut state = TranscriptState::new();
        state.sync(&thread, None);
        thread.messages.push(Message {
            id: "sent".into(),
            role: Role::User,
            text: "Hello".into(),
        });
        thread.timeline.push(TranscriptItem::Message { index: 0 });
        state.sync(
            &thread,
            Some(&ThreadEvent::TextDelta {
                message_id: Some("sent".into()),
                role: Role::User,
                text: "Hello".into(),
            }),
        );
        let frame = state.message_entries["sent"] + std::time::Duration::from_millis(50);
        let progress = state.message_progress("sent", frame);
        assert!(progress > 0. && progress < 1.);
        state.sync(&thread, None); // Concurrent durable hydration of the same live row.
        assert_eq!(state.message_progress("sent", frame), progress);
        let mut restored = TranscriptState::new();
        restored.sync(&thread, None);
        assert_eq!(restored.message_progress("sent", frame), 1.);
    }
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
