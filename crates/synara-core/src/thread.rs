use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub text: String,
}

/// Presentation metadata reconstructed from the same durable events as the text.
#[derive(Clone, Debug, PartialEq)]
pub struct TurnSummary {
    pub id: String,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub first_timeline_index: usize,
    pub end_timeline_index: usize,
    pub failed: bool,
}

/// Metadata of the latest durable replacement of a tool's output, not authorship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolOutputOrigin {
    pub turn_index: Option<usize>,
    pub timestamp_ms: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub title: String,
    pub status: ToolStatus,
    pub kind: Option<String>,
    pub output: Vec<ToolOutput>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TranscriptItem {
    Message { index: usize },
    Tool { id: String },
    Permission { id: String },
    Input { id: String },
    Notice { text: String, is_error: bool },
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ReplayError {
    #[error("event belongs to another thread")]
    WrongThread,
    #[error("expected event sequence {expected}, received {actual}")]
    Sequence { expected: u64, actual: u64 },
    #[error("event identifier was reused at a different sequence")]
    DuplicateId,
    #[error("transcript exceeds configured limit")]
    Limit,
}

/// The reducer consumes only durable, ordered events. UI scroll state is deliberately separate.
#[derive(Clone, Debug)]
pub struct Thread {
    pub id: ThreadId,
    pub title: String,
    pub state: TaskState,
    pub messages: Vec<Message>,
    pub images: Vec<MessageImage>,
    pub message_timestamps: BTreeMap<String, i64>,
    pub turns: Vec<TurnSummary>,
    pub tools: BTreeMap<String, Tool>,
    pub tool_output_origins: BTreeMap<String, ToolOutputOrigin>,
    pub terminals: BTreeMap<String, TerminalRecord>,
    pub permissions: BTreeMap<String, PermissionRequest>,
    pub inputs: BTreeMap<String, UserInputRequest>,
    pub timeline: Vec<TranscriptItem>,
    pub plan: Vec<PlanEntry>,
    pub usage: Usage,
    pub configuration: SessionConfiguration,
    pub commands: Vec<SlashCommand>,
    pub last_sequence: u64,
    seen: HashMap<EventId, u64>,
    text_bytes: usize,
    max_text_bytes: usize,
    max_events: usize,
    activity: ThreadActivity,
    replay_backup: Option<Box<Thread>>,
}

impl Thread {
    pub fn new(id: ThreadId) -> Self {
        Self {
            id,
            title: "New task".into(),
            state: TaskState::Ready,
            messages: vec![],
            images: vec![],
            message_timestamps: BTreeMap::new(),
            turns: vec![],
            tools: BTreeMap::new(),
            tool_output_origins: BTreeMap::new(),
            terminals: BTreeMap::new(),
            permissions: BTreeMap::new(),
            inputs: BTreeMap::new(),
            timeline: vec![],
            plan: vec![],
            usage: Usage::default(),
            configuration: SessionConfiguration::default(),
            commands: vec![],
            last_sequence: 0,
            seen: HashMap::new(),
            text_bytes: 0,
            max_text_bytes: 64 * 1024 * 1024,
            max_events: 200_000,
            activity: ThreadActivity::new("New task".into()),
            replay_backup: None,
        }
    }

    pub fn apply(&mut self, envelope: &EventEnvelope) -> Result<bool, ReplayError> {
        if envelope.thread_id != self.id {
            return Err(ReplayError::WrongThread);
        }
        if let Some(sequence) = self.seen.get(&envelope.id) {
            return if *sequence == envelope.sequence {
                Ok(false)
            } else {
                Err(ReplayError::DuplicateId)
            };
        }
        if envelope.sequence != self.last_sequence + 1 {
            return Err(ReplayError::Sequence {
                expected: self.last_sequence + 1,
                actual: envelope.sequence,
            });
        }
        let added_bytes = match &envelope.event {
            ThreadEvent::TextDelta { text, .. } => text.len(),
            ThreadEvent::ImageMessage { image, .. } => {
                if !image.bounded() || self.images.len() >= 256 {
                    return Err(ReplayError::Limit);
                }
                image.base64.len()
            }
            _ => 0,
        };
        if self.seen.len() >= self.max_events
            || self.text_bytes.saturating_add(added_bytes) > self.max_text_bytes
        {
            return Err(ReplayError::Limit);
        }
        match &envelope.event {
            ThreadEvent::HistoryStarted => {
                if self.replay_backup.is_none() {
                    self.replay_backup = Some(Box::new(self.clone()));
                }
                self.messages.clear();
                self.images.clear();
                self.message_timestamps.clear();
                self.turns.clear();
                self.tools.clear();
                self.tool_output_origins.clear();
                self.terminals.clear();
                self.timeline.clear();
                self.permissions.clear();
                self.inputs.clear();
                self.plan.clear();
                self.commands.clear();
                self.text_bytes = 0;
            }
            ThreadEvent::HistoryCompleted => {
                self.replay_backup = None;
            }
            ThreadEvent::CancellationRequested => {
                for tool in self.tools.values_mut() {
                    if matches!(tool.status, ToolStatus::Pending | ToolStatus::Running) {
                        tool.status = ToolStatus::Cancelled;
                    }
                }
                self.permissions.clear();
                self.inputs.clear();
            }
            ThreadEvent::TerminalOutput {
                id,
                text,
                truncated,
                exit_code,
            } => {
                self.terminals.insert(
                    id.clone(),
                    TerminalRecord {
                        text: text.to_owned(),
                        truncated: *truncated,
                        exit_code: *exit_code,
                    },
                );
            }
            ThreadEvent::PromptStarted { turn } => {
                self.turns.push(TurnSummary {
                    id: turn.clone(),
                    started_at_ms: envelope.timestamp_ms,
                    finished_at_ms: None,
                    first_timeline_index: self.timeline.len(),
                    end_timeline_index: self.timeline.len(),
                    failed: false,
                });
            }
            ThreadEvent::TextDelta {
                message_id, role, ..
            }
            | ThreadEvent::ImageMessage {
                message_id, role, ..
            } => {
                let text = match &envelope.event {
                    ThreadEvent::TextDelta { text, .. } => text.as_str(),
                    _ => "",
                };
                let existing = message_id.as_ref().and_then(|id| {
                    self.messages
                        .iter()
                        .position(|m| &m.id == id && m.role == *role)
                });
                let tail = match self.timeline.last() {
                    Some(TranscriptItem::Message { index })
                        if self.messages[*index].role == *role =>
                    {
                        Some(*index)
                    }
                    _ => None,
                };
                let index = existing.or_else(|| if message_id.is_none() { tail } else { None });
                let index = if let Some(index) = index {
                    self.messages[index].text.push_str(text);
                    index
                } else {
                    let index = self.messages.len();
                    self.messages.push(Message {
                        id: message_id
                            .clone()
                            .unwrap_or_else(|| format!("event-{}", envelope.id)),
                        role: *role,
                        text: text.to_owned(),
                    });
                    self.timeline.push(TranscriptItem::Message { index });
                    self.message_timestamps
                        .insert(self.messages[index].id.clone(), envelope.timestamp_ms);
                    index
                };
                if let ThreadEvent::ImageMessage { image, .. } = &envelope.event {
                    self.images.push(MessageImage {
                        id: envelope.id,
                        message_id: self.messages[index].id.clone(),
                        role: *role,
                        image: image.clone(),
                    });
                }
            }
            ThreadEvent::ToolChanged { patch } => {
                if !self.tools.contains_key(&patch.id) {
                    self.timeline.push(TranscriptItem::Tool {
                        id: patch.id.clone(),
                    });
                }
                let tool = self.tools.entry(patch.id.clone()).or_insert_with(|| Tool {
                    id: patch.id.clone(),
                    ..Tool::default()
                });
                if let Some(title) = &patch.title {
                    tool.title.clone_from(title);
                }
                if let Some(status) = patch.status {
                    tool.status = status;
                }
                if let Some(kind) = &patch.kind {
                    tool.kind = Some(kind.clone());
                }
                if let Some(output) = &patch.output {
                    tool.output.clone_from(output);
                    self.tool_output_origins.insert(
                        patch.id.clone(),
                        ToolOutputOrigin {
                            turn_index: self
                                .turns
                                .iter()
                                .enumerate()
                                .next_back()
                                .filter(|(_, turn)| turn.finished_at_ms.is_none())
                                .map(|(index, _)| index),
                            timestamp_ms: envelope.timestamp_ms,
                        },
                    );
                }
            }
            ThreadEvent::PermissionRequested { request } => {
                if !self.permissions.contains_key(&request.id) {
                    self.timeline.push(TranscriptItem::Permission {
                        id: request.id.clone(),
                    });
                }
                self.permissions.insert(request.id.clone(), request.clone());
            }
            ThreadEvent::PermissionResolved { id, .. } => {
                self.permissions.remove(id);
            }
            ThreadEvent::UserInputRequested { request } => {
                if !self.inputs.contains_key(&request.id) {
                    self.timeline.push(TranscriptItem::Input {
                        id: request.id.clone(),
                    });
                }
                self.inputs.insert(request.id.clone(), request.clone());
            }
            ThreadEvent::UserInputResolved { id } => {
                self.inputs.remove(id);
            }
            ThreadEvent::PlanChanged { entries } => self.plan.clone_from(entries),
            ThreadEvent::UsageChanged { usage } => self.usage = usage.clone(),
            ThreadEvent::ConfigurationChanged { configuration } => {
                self.configuration = configuration.clone()
            }
            ThreadEvent::CommandsChanged { commands } => self.commands.clone_from(commands),
            ThreadEvent::TitleChanged { .. } => {}
            ThreadEvent::PromptFinished { .. } => {
                if let Some(turn) = self.turns.last_mut() {
                    turn.finished_at_ms = Some(envelope.timestamp_ms.max(turn.started_at_ms));
                }
                self.permissions.clear();
                self.inputs.clear();
            }
            ThreadEvent::Error {
                message,
                recoverable,
            } => {
                if !recoverable && let Some(previous) = self.replay_backup.take() {
                    let sequence = self.last_sequence;
                    let seen = std::mem::take(&mut self.seen);
                    *self = *previous;
                    self.last_sequence = sequence;
                    self.seen = seen;
                }
                self.timeline.push(TranscriptItem::Notice {
                    text: message.clone(),
                    is_error: true,
                });
                if !recoverable {
                    if let Some(turn) = self
                        .turns
                        .last_mut()
                        .filter(|turn| turn.finished_at_ms.is_none())
                    {
                        turn.finished_at_ms = Some(envelope.timestamp_ms.max(turn.started_at_ms));
                        turn.failed = true;
                    }
                    self.permissions.clear();
                    self.inputs.clear();
                }
            }
            ThreadEvent::SessionStatus { status } => self.timeline.push(TranscriptItem::Notice {
                text: status.clone(),
                is_error: false,
            }),
            ThreadEvent::ContextCompaction { message } | ThreadEvent::Notice { message } => {
                self.timeline.push(TranscriptItem::Notice {
                    text: message.clone(),
                    is_error: false,
                })
            }
        }
        if let Some(turn) = self.turns.last_mut() {
            turn.end_timeline_index = self.timeline.len();
        }
        self.activity.apply(&envelope.event);
        self.state = self.activity.state;
        self.title.clone_from(&self.activity.title);
        self.text_bytes += added_bytes;
        self.seen.insert(envelope.id, envelope.sequence);
        self.last_sequence = envelope.sequence;
        Ok(true)
    }

    pub fn history_in_progress(&self) -> bool {
        self.replay_backup.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollOwnership {
    #[default]
    Following,
    User,
}
impl ScrollOwnership {
    pub fn user_scrolled(&mut self, at_bottom: bool) {
        *self = if at_bottom {
            Self::Following
        } else {
            Self::User
        };
    }
    pub fn jump_to_latest(&mut self) {
        *self = Self::Following;
    }
    pub fn should_follow(&self, event: &ThreadEvent) -> bool {
        *self == Self::Following
            && matches!(
                event,
                ThreadEvent::TextDelta { .. } | ThreadEvent::ImageMessage { .. }
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn apply(thread: &mut Thread, event: ThreadEvent) {
        let envelope = EventEnvelope {
            id: EventId::new(),
            thread_id: thread.id,
            sequence: thread.last_sequence + 1,
            timestamp_ms: 0,
            event,
        };
        assert_eq!(thread.apply(&envelope), Ok(true));
    }
    #[test]
    fn turn_metadata_replays_timestamps_without_stream_or_duplicate_drift() {
        let id = ThreadId::new();
        let events = [
            (1000, ThreadEvent::PromptStarted { turn: "one".into() }),
            (
                1010,
                ThreadEvent::TextDelta {
                    message_id: Some("u".into()),
                    role: Role::User,
                    text: "hello".into(),
                },
            ),
            (
                2000,
                ThreadEvent::TextDelta {
                    message_id: Some("a".into()),
                    role: Role::Assistant,
                    text: "First".into(),
                },
            ),
            (
                3000,
                ThreadEvent::TextDelta {
                    message_id: Some("a".into()),
                    role: Role::Assistant,
                    text: " answer".into(),
                },
            ),
            (
                89000,
                ThreadEvent::PromptFinished {
                    reason: "end_turn".into(),
                },
            ),
            (90000, ThreadEvent::PromptStarted { turn: "two".into() }),
            (
                90010,
                ThreadEvent::TextDelta {
                    message_id: Some("u2".into()),
                    role: Role::User,
                    text: "again".into(),
                },
            ),
            (
                89999,
                ThreadEvent::Error {
                    message: "Turn failed".into(),
                    recoverable: false,
                },
            ),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (timestamp_ms, event))| EventEnvelope {
            id: EventId::new(),
            thread_id: id,
            sequence: i as u64 + 1,
            timestamp_ms,
            event,
        })
        .collect::<Vec<_>>();
        let mut live = Thread::new(id);
        for event in &events {
            assert_eq!(live.apply(event), Ok(true));
            assert_eq!(live.apply(event), Ok(false));
        }
        assert_eq!(live.message_timestamps["a"], 2000);
        assert_eq!(live.turns[0].finished_at_ms, Some(89000));
        assert_eq!(
            (
                live.turns[0].first_timeline_index,
                live.turns[0].end_timeline_index
            ),
            (0, 2)
        );
        assert_eq!(
            (
                live.turns[1].first_timeline_index,
                live.turns[1].end_timeline_index
            ),
            (2, 4)
        );
        assert_eq!(live.turns[1].finished_at_ms, Some(90000)); // Clock skew cannot create a negative duration.
        assert!(live.turns[1].failed);
        let mut restored = Thread::new(id);
        for event in &events {
            restored.apply(event).unwrap();
        }
        assert_eq!(restored.turns, live.turns);
        assert_eq!(restored.message_timestamps, live.message_timestamps);
    }

    #[test]
    fn failed_history_refresh_restores_completed_turn_metadata() {
        let mut thread = Thread::new(ThreadId::new());
        apply(
            &mut thread,
            ThreadEvent::PromptStarted {
                turn: "kept".into(),
            },
        );
        apply(
            &mut thread,
            ThreadEvent::TextDelta {
                message_id: Some("a".into()),
                role: Role::Assistant,
                text: "Kept answer".into(),
            },
        );
        apply(
            &mut thread,
            ThreadEvent::PromptFinished {
                reason: "end_turn".into(),
            },
        );
        let stamps = thread.message_timestamps.clone();
        apply(&mut thread, ThreadEvent::HistoryStarted);
        assert!(thread.turns.is_empty());
        apply(
            &mut thread,
            ThreadEvent::PromptStarted {
                turn: "incomplete-import".into(),
            },
        );
        apply(
            &mut thread,
            ThreadEvent::Error {
                message: "Import failed".into(),
                recoverable: false,
            },
        );
        assert_eq!(thread.turns.len(), 1);
        assert_eq!(thread.turns[0].id, "kept");
        assert!(!thread.turns[0].failed);
        assert_eq!(thread.message_timestamps, stamps);
        assert_eq!(thread.messages[0].text, "Kept answer");
    }

    #[test]
    fn streaming_text_coalesces_without_merging_across_tools() {
        let mut t = Thread::new(ThreadId::new());
        let text = |s: &str| ThreadEvent::TextDelta {
            message_id: None,
            role: Role::Assistant,
            text: s.into(),
        };
        apply(&mut t, text("one"));
        apply(&mut t, text(" two"));
        apply(
            &mut t,
            ThreadEvent::ToolChanged {
                patch: ToolPatch {
                    id: "tool-1".into(),
                    title: Some("Read".into()),
                    ..ToolPatch::default()
                },
            },
        );
        apply(&mut t, text("three"));
        assert_eq!(t.messages.len(), 2);
        assert_eq!(t.messages[0].text, "one two");
        assert_eq!(t.timeline.len(), 3);
    }
    #[test]
    fn partial_tool_update_keeps_prior_content() {
        let mut t = Thread::new(ThreadId::new());
        apply(
            &mut t,
            ThreadEvent::ToolChanged {
                patch: ToolPatch {
                    id: "a".into(),
                    title: Some("Read".into()),
                    output: Some(vec![ToolOutput::Text { text: "ok".into() }]),
                    ..ToolPatch::default()
                },
            },
        );
        apply(
            &mut t,
            ThreadEvent::ToolChanged {
                patch: ToolPatch {
                    id: "a".into(),
                    status: Some(ToolStatus::Completed),
                    ..ToolPatch::default()
                },
            },
        );
        assert_eq!(t.tools["a"].title, "Read");
        assert_eq!(t.tools["a"].output.len(), 1);
        assert_eq!(t.timeline.len(), 1);
    }
    #[test]
    fn replay_is_ordered_and_idempotent() {
        let mut t = Thread::new(ThreadId::new());
        let mut e = EventEnvelope {
            id: EventId::new(),
            thread_id: t.id,
            sequence: 1,
            timestamp_ms: 0,
            event: ThreadEvent::Notice {
                message: "hello".into(),
            },
        };
        assert_eq!(t.apply(&e), Ok(true));
        assert_eq!(t.apply(&e), Ok(false));
        e.id = EventId::new();
        e.sequence = 3;
        assert_eq!(
            t.apply(&e),
            Err(ReplayError::Sequence {
                expected: 2,
                actual: 3
            })
        );
        assert_eq!(t.last_sequence, 1);
    }
    #[test]
    fn multiple_pending_permissions_do_not_resume_early() {
        let mut t = Thread::new(ThreadId::new());
        apply(&mut t, ThreadEvent::PromptStarted { turn: "x".into() });
        for id in ["1", "2"] {
            apply(
                &mut t,
                ThreadEvent::PermissionRequested {
                    request: PermissionRequest {
                        id: id.into(),
                        tool_id: None,
                        title: "Run".into(),
                        choices: vec![],
                    },
                },
            );
        }
        apply(
            &mut t,
            ThreadEvent::PermissionResolved {
                id: "1".into(),
                selected: None,
            },
        );
        assert_eq!(t.state, TaskState::Waiting);
        apply(
            &mut t,
            ThreadEvent::PermissionResolved {
                id: "2".into(),
                selected: None,
            },
        );
        assert_eq!(t.state, TaskState::Running);
    }
    #[test]
    fn generic_activity_never_takes_scroll_ownership() {
        let mut s = ScrollOwnership::default();
        assert!(!s.should_follow(&ThreadEvent::Notice {
            message: "busy".into()
        }));
        s.user_scrolled(false);
        assert!(!s.should_follow(&ThreadEvent::TextDelta {
            role: Role::Assistant,
            message_id: None,
            text: "text".into()
        }));
    }
    #[test]
    fn fatal_error_clears_pending_interactions() {
        let mut t = Thread::new(ThreadId::new());
        apply(&mut t, ThreadEvent::PromptStarted { turn: "1".into() });
        apply(
            &mut t,
            ThreadEvent::Error {
                message: "agent exited".into(),
                recoverable: false,
            },
        );
        assert_eq!(t.state, TaskState::Failed);
        apply(
            &mut t,
            ThreadEvent::PermissionResolved {
                id: "late".into(),
                selected: None,
            },
        );
        assert_eq!(t.state, TaskState::Failed);
    }

    #[test]
    fn output_origins_follow_output_replacement_not_later_tool_status_or_chat_activity() {
        let mut thread = Thread::new(ThreadId::new());
        let output = || ThreadEvent::ToolChanged {
            patch: ToolPatch {
                id: "write".into(),
                status: Some(ToolStatus::Completed),
                output: Some(vec![ToolOutput::Text {
                    text: "reported".into(),
                }]),
                ..ToolPatch::default()
            },
        };
        apply(
            &mut thread,
            ThreadEvent::PromptStarted { turn: "one".into() },
        );
        apply(&mut thread, output());
        assert_eq!(thread.tool_output_origins["write"].turn_index, Some(0));
        apply(
            &mut thread,
            ThreadEvent::PromptFinished {
                reason: "end_turn".into(),
            },
        );
        apply(
            &mut thread,
            ThreadEvent::PromptStarted { turn: "two".into() },
        );
        apply(
            &mut thread,
            ThreadEvent::ToolChanged {
                patch: ToolPatch {
                    id: "write".into(),
                    title: Some("New title, same output".into()),
                    status: Some(ToolStatus::Completed),
                    ..ToolPatch::default()
                },
            },
        );
        assert_eq!(thread.tool_output_origins["write"].turn_index, Some(0));
        apply(&mut thread, output());
        assert_eq!(thread.tool_output_origins["write"].turn_index, Some(1));
        apply(
            &mut thread,
            ThreadEvent::PromptFinished {
                reason: "end_turn".into(),
            },
        );
        apply(&mut thread, output());
        assert_eq!(thread.tool_output_origins["write"].turn_index, None);
    }
    #[test]
    fn output_origins_replay_exactly_and_failed_history_replacement_restores_them() {
        let id = ThreadId::new();
        let events: Vec<_> = [
            ThreadEvent::PromptStarted { turn: "one".into() },
            ThreadEvent::ToolChanged {
                patch: ToolPatch {
                    id: "write".into(),
                    output: Some(vec![]),
                    ..ToolPatch::default()
                },
            },
            ThreadEvent::PromptFinished {
                reason: "end_turn".into(),
            },
        ]
        .into_iter()
        .enumerate()
        .map(|(index, event)| EventEnvelope {
            id: EventId::new(),
            thread_id: id,
            sequence: index as u64 + 1,
            timestamp_ms: 1000 + index as i64 * 50,
            event,
        })
        .collect();
        let mut original = Thread::new(id);
        let mut reopened = Thread::new(id);
        for event in &events {
            original.apply(event).unwrap();
            reopened.apply(event).unwrap();
            assert!(!reopened.apply(event).unwrap());
        }
        assert_eq!(original.tool_output_origins, reopened.tool_output_origins);
        assert_eq!(reopened.tool_output_origins["write"].timestamp_ms, 1050);
        apply(&mut reopened, ThreadEvent::HistoryStarted);
        assert!(reopened.tool_output_origins.is_empty());
        apply(
            &mut reopened,
            ThreadEvent::Error {
                message: "history failed".into(),
                recoverable: false,
            },
        );
        assert_eq!(original.tool_output_origins, reopened.tool_output_origins);
        apply(&mut reopened, ThreadEvent::HistoryStarted);
        apply(&mut reopened, ThreadEvent::HistoryCompleted);
        assert!(reopened.tool_output_origins.is_empty());
    }
}
