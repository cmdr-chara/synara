#!/usr/bin/env python3
"""One-time exact-context integration, confined to the isolated CI checkout.

No GitHub access or process launch. Removed after checked source publication.
"""
from pathlib import Path
import hashlib


def replace(text, old, new, name):
    assert text.count(old) == 1, (name, 'ambiguous/missing anchor', old[:100])
    return text.replace(old, new, 1)


def edit(name, expected, edits):
    path = Path(name)
    raw = path.read_bytes()
    digest = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    assert digest == expected, (name, 'base blob changed', digest)
    text = raw.decode('utf-8')
    for old, new in edits:
        text = replace(text, old, new, name)
    path.write_text(text, encoding='utf-8', newline='')


def main():
    edit('crates/synara-app/src/shell.rs', '296384226575fb1d10640c77c45328af9ef61b4a', [
        ('mod overview;', 'mod overview;\nmod kanban;'),
        ('enum Update {', 'enum Update {\n    Kanban(Box<kanban::KanbanReply>),'),
        ('pub struct Shell {', 'pub struct Shell {\n    kanban: kanban::KanbanState,'),
        ('let mut this = Self {', 'let mut this = Self {\n            kanban: kanban::KanbanState::default(),'),
        ('match update {\n            Update::DraftLoaded', 'match update {\n            Update::Kanban(reply) => self.kanban_reply(*reply, cx),\n            Update::DraftLoaded'),
        ('Update::Tick => {\n                self.flush_drafts(false);', 'Update::Tick => {\n                self.poll_kanban();\n                self.flush_drafts(false);'),
        ('pub fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {', '''pub fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.kanban.creating {
            self.notice = Some("Finishing task creation before closing. Your prompt is being saved.".into());
            cx.notify();
            return false;
        }
        if let Some(dialog) = &self.kanban.dialog {
            if dialog.read(cx).has_text(cx) {
                dialog.update(cx, |dialog, cx| dialog.failed("Create the task or explicitly discard this unfinished prompt before closing.".into(), cx));
                window.focus(&dialog.read(cx).focus_handle(cx), cx);
                return false;
            }
            self.kanban.dialog = None;
        }'''),
    ])
    edit('crates/synara-app/src/shell/chrome.rs', '260bf5d2a56f695f501bf1f1be3c71dfaabf3cb8', [
        ('if self.focus_composer && !self.navigation.menu_open && !self.controls.is_open() {', 'if self.focus_composer && !self.navigation.menu_open && !self.controls.is_open() && self.kanban.dialog.is_none() {'),
        ('let key = event.keystroke.key.as_str();\n                if this.navigation.menu_open {', '''let key = event.keystroke.key.as_str();
                if this.kanban.dialog.is_some() { return; }
                if this.panel == Panel::Kanban && key == "t" && modifiers.alt
                    && (modifiers.control || modifiers.platform) && !modifiers.shift && !event.is_held {
                    this.open_task_dialog(false, cx);
                    cx.stop_propagation();
                    return;
                }
                if this.navigation.menu_open {'''),
        ('.children(self.navigation.menu_open.then(|| self.tools_overlay(cx)))', '.children(self.kanban.dialog.clone())\n            .children(self.navigation.menu_open.then(|| self.tools_overlay(cx)))'),
    ])
    path = Path('crates/synara-app/src/ui.rs')
    text = path.read_text()
    assert 'pub mod task_dialog;' not in text
    path.write_text(replace(text, 'pub mod menu;', 'pub mod menu;\npub mod task_dialog;', str(path)), encoding='utf-8')
    path = Path('crates/synara-app/src/shell/overview.rs')
    text = path.read_text()
    start = text.index('    pub(super) fn kanban_panel')
    end = text.index('    pub(super) fn help_panel')
    assert start < end
    text = text[:start] + text[end:]
    text = replace(text, 'use crate::ui::{self, palette};', 'use crate::ui;', str(path))
    path.write_text(text, encoding='utf-8')

    path = Path('crates/synara-app/src/ui/task_dialog.rs')
    text = path.read_text()
    old = '''    pub fn new(
        projects: Vec<(ProjectId, String)>,
        agents: Vec<(String, String, Glyph)>,
        initial_project: Option<ProjectId>,
        default_agent: Option<&str>,
        draft: bool,
        send_on_enter: bool,
        cx: &mut Context<Self>,
    ) -> Self {'''
    config = '''pub struct TaskDialogConfig {
    pub projects: Vec<(ProjectId, String)>,
    pub agents: Vec<(String, String, Glyph)>,
    pub initial_project: Option<ProjectId>,
    pub default_agent: Option<String>,
    pub draft: bool,
    pub send_on_enter: bool,
}
'''
    text = replace(text, 'pub struct TaskDialog {', config + 'pub struct TaskDialog {', str(path))
    text = replace(text, old, '''    pub fn new(config: TaskDialogConfig, cx: &mut Context<Self>) -> Self {
        let TaskDialogConfig { projects, agents, initial_project, default_agent, draft, send_on_enter } = config;''', str(path))
    text = replace(text, 'Some(id.as_str()) == default_agent)', 'Some(id.as_str()) == default_agent.as_deref())', str(path))
    text = replace(text, 'Entity, EventEmitter, FocusHandle, Focusable, Subscription', 'Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable, Subscription', str(path))
    text = replace(text, '''.on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {''', '''.on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.prefer_character_input || this.prompt.update(cx, |entry, cx| entry.marked_text_range(window, cx).is_some()) {
                    return;
                }
                if event.keystroke.key == "escape" {''', str(path))
    path.write_text(text, encoding='utf-8')

    path = Path('crates/synara-app/src/shell/kanban.rs')
    text = path.read_text()
    text = replace(text, 'launching: HashSet<TaskId>,', 'launching: HashMap<TaskId, u64>,\n    next_launch: u64,', str(path))
    text = replace(text, 'DraftReady(TaskId, Result<String, String>),', 'DraftReady(TaskId, u64, Result<String, String>),', str(path))
    text = text.replace('launching.contains(', 'launching.contains_key(')
    text = replace(text, 'self.kanban.launching.insert(id);', 'let generation = self.kanban.reserve_launch(id);', str(path))
    text = replace(text, 'KanbanReply::DraftReady(id, result)', 'KanbanReply::DraftReady(id, generation, result)', str(path))
    text = replace(text, '''            KanbanReply::DraftReady(id, result) => match result {
                Ok(text) => self.submit_kanban_text(id, text, cx),
                Err(error) => { self.kanban.launching.remove(&id); self.error = Some(error); }
            },''', '''            KanbanReply::DraftReady(id, generation, result) => {
                if !self.kanban.take_launch(id, generation) { return; }
                match result {
                    Ok(text) => self.submit_kanban_text(id, text, cx),
                    Err(error) => self.error = Some(error),
                }
            }''', str(path))
    text = replace(text, '''        if self.kanban.stopping.contains(&id) || self.kanban.launching.contains_key(&id) { return; }''', '''        if self.kanban.launching.remove(&id).is_some() {
            cx.notify();
            return;
        }
        if self.kanban.stopping.contains(&id) { return; }''', str(path))
    text = replace(text, '.is_none_or(|existing| existing.updated_at_ms <= task.updated_at_ms)', '.is_some_and(|existing| existing.updated_at_ms <= task.updated_at_ms)', str(path))
    text = replace(text, '''                        // A queued snapshot cannot erase a task created later or
                        // regress a task after a newer durable event was received.''', '''                        // Update known tasks only. A queued snapshot cannot revive
                        // a deleted task, erase a later creation, or replace a newer event.''', str(path))
    text = replace(text, 'fn column(task: &Task, starting: bool)', '''impl KanbanState {
    fn reserve_launch(&mut self, task: TaskId) -> u64 {
        self.next_launch = self.next_launch.wrapping_add(1);
        self.launching.insert(task, self.next_launch);
        self.next_launch
    }
    fn take_launch(&mut self, task: TaskId, generation: u64) -> bool {
        if self.launching.get(&task) != Some(&generation) { return false; }
        self.launching.remove(&task);
        true
    }
}
fn column(task: &Task, starting: bool)''', str(path))
    text = replace(text, '''    #[test]
    fn task_title_is_whitespace_normalized_and_unicode_bounded()''', '''    #[test]
    fn cancelled_load_cannot_consume_a_later_launch_or_start_twice() {
        let mut state = KanbanState::default();
        let id = TaskId::new();
        let first = state.reserve_launch(id);
        state.launching.remove(&id);
        let second = state.reserve_launch(id);
        assert!(!state.take_launch(id, first));
        assert!(state.take_launch(id, second));
        assert!(!state.take_launch(id, second));
    }
    #[test]
    fn task_title_is_whitespace_normalized_and_unicode_bounded()''', str(path))
    path.write_text(text, encoding='utf-8')

    edit('crates/synara-workspace/src/service.rs', '4c000c6fb531a787ace28979966f17b7294e56d0', [
        ('''    pub async fn create_scoped_task(
        &self,
        project: ProjectId,
        title: String,
        agent_id: String,
        scope: TaskScope,
    ) -> WorkspaceResult<Task> {''', '''    pub async fn create_scoped_task(
        &self, project: ProjectId, title: String, agent_id: String, scope: TaskScope,
    ) -> WorkspaceResult<Task> {
        self.create_scoped_task_with_draft(project, title, agent_id, scope, String::new()).await
    }

    /// Save an unsent prompt with its task atomically. This never connects or runs an agent.
    pub async fn create_scoped_task_with_draft(
        &self, project: ProjectId, title: String, agent_id: String, scope: TaskScope, draft: String,
    ) -> WorkspaceResult<Task> {
        if draft.len() > 1024 * 1024 { return Err(WorkspaceError::Invalid("Task draft exceeds 1 MiB".into())); }'''),
        ('''            store.save_task(&task)?;
            Ok(task)
        })
        .await
    }
    pub async fn task''', '''            store.insert_task_with_draft(&task, draft)?;
            Ok(task)
        })
        .await
    }
    pub async fn task'''),
    ])
    path = Path('crates/synara-workspace/src/storage.rs')
    text = path.read_text()
    assert 'mod task_creation;' not in text
    path.write_text(replace(text, 'mod chat_preferences;', 'mod chat_preferences;\nmod task_creation;', str(path)), encoding='utf-8')
    path = Path('crates/synara-workspace/src/storage/task_creation.rs')
    text = path.read_text()
    text = replace(text, '''END;")?;''', '''END;").map_err(StorageError::from)?;''', str(path))
    path.write_text(text, encoding='utf-8')
    print('Integrated the reviewed task creation, launch-generation, board and modal boundaries.')

if __name__ == '__main__':
    main()
