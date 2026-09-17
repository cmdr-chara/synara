use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub text: String,
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
    pub tools: BTreeMap<String, Tool>,
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
    active_turn: Option<String>,
    replay_backup: Option<Box<Thread>>,
}

impl Thread {
    pub fn new(id: ThreadId) -> Self {
        Self {
            id,
            title: "New task".into(),
            state: TaskState::Ready,
            messages: vec![],
            tools: BTreeMap::new(),
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
            active_turn: None,
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
                self.tools.clear();
                self.terminals.clear();
                self.timeline.clear();
                self.permissions.clear();
                self.inputs.clear();
                self.plan.clear();
                self.commands.clear();
                self.text_bytes = 0;
                self.active_turn = None;
                self.state = TaskState::Ready;
            }
            ThreadEvent::HistoryCompleted => {
                self.replay_backup = None;
                self.state = TaskState::Ready;
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
                        text: text.clone(),
                        truncated: *truncated,
                        exit_code: *exit_code,
                    },
                );
            }
            ThreadEvent::PromptStarted { turn } => {
                self.active_turn = Some(turn.clone());
                self.state = TaskState::Running;
            }
            ThreadEvent::TextDelta {
                message_id,
                role,
                text,
            } => {
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
                if let Some(index) = index {
                    self.messages[index].text.push_str(text);
                } else {
                    let index = self.messages.len();
                    self.messages.push(Message {
                        id: message_id
                            .clone()
                            .unwrap_or_else(|| format!("event-{}", envelope.id)),
                        role: *role,
                        text: text.clone(),
                    });
                    self.timeline.push(TranscriptItem::Message { index });
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
                }
            }
            ThreadEvent::PermissionRequested { request } => {
                if !self.permissions.contains_key(&request.id) {
                    self.timeline.push(TranscriptItem::Permission {
                        id: request.id.clone(),
                    });
                }
                self.permissions.insert(request.id.clone(), request.clone());
                self.state = TaskState::Waiting;
            }
            ThreadEvent::PermissionResolved { id, .. } => {
                self.permissions.remove(id);
                self.refresh_waiting();
            }
            ThreadEvent::UserInputRequested { request } => {
                if !self.inputs.contains_key(&request.id) {
                    self.timeline.push(TranscriptItem::Input {
                        id: request.id.clone(),
                    });
                }
                self.inputs.insert(request.id.clone(), request.clone());
                self.state = TaskState::Waiting;
            }
            ThreadEvent::UserInputResolved { id } => {
                self.inputs.remove(id);
                self.refresh_waiting();
            }
            ThreadEvent::PlanChanged { entries } => self.plan.clone_from(entries),
            ThreadEvent::UsageChanged { usage } => self.usage = usage.clone(),
            ThreadEvent::ConfigurationChanged { configuration } => {
                self.configuration = configuration.clone()
            }
            ThreadEvent::CommandsChanged { commands } => self.commands.clone_from(commands),
            ThreadEvent::TitleChanged { title } => self.title.clone_from(title),
            ThreadEvent::PromptFinished { .. } => {
                self.active_turn = None;
                self.state = TaskState::Completed;
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
                    self.state = TaskState::Failed;
                    self.active_turn = None;
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
        self.text_bytes += added_bytes;
        self.seen.insert(envelope.id, envelope.sequence);
        self.last_sequence = envelope.sequence;
        Ok(true)
    }

    fn refresh_waiting(&mut self) {
        if self.permissions.is_empty() && self.inputs.is_empty() && self.active_turn.is_some() {
            self.state = TaskState::Running;
        }
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
        *self == Self::Following && matches!(event, ThreadEvent::TextDelta { .. })
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
}
