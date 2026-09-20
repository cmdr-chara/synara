//! Search and pin navigation operates on presentation rows, not message indices.
use super::*;

impl TranscriptState {
    pub fn jump_to_message(&self, thread: &Thread, anchor: &MessageAnchor) -> bool {
        if self.thread != Some(thread.id) {
            return false;
        }
        let Some(timeline) = thread.timeline.iter().position(|item| {
            matches!(item, TranscriptItem::Message { index } if thread.messages.get(*index).is_some_and(|message| anchor.matches(message)))
        }) else { return false; };
        let direct = RowKey::Message(anchor.id.clone(), role_key(anchor.role));
        let index = self.indices.get(&direct).copied().or_else(|| {
            thread
                .turns
                .iter()
                .find(|turn| {
                    turn.first_timeline_index <= timeline && timeline < turn.end_timeline_index
                })
                .and_then(|turn| {
                    self.indices
                        .get(&RowKey::Activity(turn.id.clone()))
                        .copied()
                })
        });
        let Some(index) = index else {
            return false;
        };
        // Search is deliberate navigation. Streaming must not immediately snap
        // back to the tail. The existing follow button explicitly restores it.
        self.list.set_follow_mode(FollowMode::Normal);
        self.list.scroll_to(ListOffset {
            item_ix: index,
            offset_in_item: px(0.),
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_targets_visible_or_grouped_rows_with_role_aware_identity() {
        let mut thread = Thread::new(ThreadId::new());
        let mut state = TranscriptState::new();
        let events = [
            ThreadEvent::PromptStarted { turn: "one".into() },
            ThreadEvent::TextDelta {
                message_id: Some("shared".into()),
                role: Role::User,
                text: "Question".into(),
            },
            ThreadEvent::TextDelta {
                message_id: Some("shared".into()),
                role: Role::Reasoning,
                text: "Thinking".into(),
            },
            ThreadEvent::TextDelta {
                message_id: Some("answer".into()),
                role: Role::Assistant,
                text: "Answer".into(),
            },
            ThreadEvent::PromptFinished {
                reason: "end_turn".into(),
            },
        ];
        for (index, event) in events.into_iter().enumerate() {
            let envelope = EventEnvelope {
                id: EventId::new(),
                thread_id: thread.id,
                sequence: index as u64 + 1,
                timestamp_ms: index as i64,
                event,
            };
            thread.apply(&envelope).unwrap();
            state.sync(&thread, Some(&envelope.event));
        }
        assert!(state.jump_to_message(
            &thread,
            &MessageAnchor {
                id: "shared".into(),
                role: Role::Reasoning
            }
        ));
        assert_eq!(
            state.rows[state.list.logical_scroll_top().item_ix],
            RowKey::Activity("one".into())
        );
        assert!(!state.is_following());
        assert!(state.jump_to_message(
            &thread,
            &MessageAnchor {
                id: "shared".into(),
                role: Role::User
            }
        ));
        assert_eq!(
            state.rows[state.list.logical_scroll_top().item_ix],
            RowKey::Message("shared".into(), 0)
        );
        assert!(!state.jump_to_message(
            &thread,
            &MessageAnchor {
                id: "shared".into(),
                role: Role::Assistant
            }
        ));
        assert!(!state.jump_to_message(
            &Thread::new(ThreadId::new()),
            &MessageAnchor {
                id: "shared".into(),
                role: Role::User
            }
        ));
        state.follow();
        assert!(state.is_following());
    }
}
