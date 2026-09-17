use crate::{TaskState, ThreadEvent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Small replayable metadata projection shared by transcripts and the durable task catalog.
/// It retains interaction IDs, never message, tool, or terminal payloads.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadActivity {
    pub title: String,
    pub state: TaskState,
    active: bool,
    permissions: BTreeSet<String>,
    inputs: BTreeSet<String>,
    history_title: Option<String>,
}
impl ThreadActivity {
    pub fn new(title: String) -> Self {
        Self {
            title,
            state: TaskState::Ready,
            active: false,
            permissions: BTreeSet::new(),
            inputs: BTreeSet::new(),
            history_title: None,
        }
    }
    pub fn apply(&mut self, event: &ThreadEvent) {
        match event {
            ThreadEvent::HistoryStarted => {
                if self.history_title.is_none() {
                    self.history_title = Some(self.title.clone());
                }
                self.active = false;
                self.clear_interactions();
                self.state = TaskState::Ready;
            }
            ThreadEvent::HistoryCompleted => {
                self.history_title = None;
                self.state = TaskState::Ready;
            }
            ThreadEvent::PromptStarted { .. } => {
                self.active = true;
                self.state = TaskState::Running;
            }
            ThreadEvent::PermissionRequested { request } => {
                self.permissions.insert(request.id.clone());
                self.state = TaskState::Waiting;
            }
            ThreadEvent::UserInputRequested { request } => {
                self.inputs.insert(request.id.clone());
                self.state = TaskState::Waiting;
            }
            ThreadEvent::PermissionResolved { id, .. } => {
                self.permissions.remove(id);
                self.refresh_waiting();
            }
            ThreadEvent::UserInputResolved { id } => {
                self.inputs.remove(id);
                self.refresh_waiting();
            }
            ThreadEvent::CancellationRequested => self.clear_interactions(),
            ThreadEvent::PromptFinished { .. } => {
                self.active = false;
                self.clear_interactions();
                self.state = TaskState::Completed;
            }
            ThreadEvent::TitleChanged { title } => self.title.clone_from(title),
            ThreadEvent::Error {
                recoverable: false, ..
            } => {
                if let Some(previous_title) = self.history_title.take() {
                    self.title = previous_title;
                }
                self.active = false;
                self.clear_interactions();
                self.state = TaskState::Failed;
            }
            _ => {}
        }
    }
    fn clear_interactions(&mut self) {
        self.permissions.clear();
        self.inputs.clear();
    }
    fn refresh_waiting(&mut self) {
        if self.active && self.permissions.is_empty() && self.inputs.is_empty() {
            self.state = TaskState::Running;
        }
    }
}
