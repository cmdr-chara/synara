#!/usr/bin/env python3
"""One-time, exact-context integration of the reviewed Kanban source checkpoint.

Run only in the isolated CI checkout. It never contacts GitHub, starts agents, or
writes outside the listed source paths. Removed after checked source publication.
"""
from pathlib import Path
import hashlib


def edit(name, expected, edits):
    path = Path(name)
    raw = path.read_bytes()
    digest = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    assert digest == expected, (name, 'base blob changed', digest)
    text = raw.decode('utf-8')
    for old, new in edits:
        assert text.count(old) == 1, (name, 'ambiguous/missing anchor', old[:100])
        text = text.replace(old, new, 1)
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
    # These substitutions do not replace unrelated event, input, or close logic.
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
    assert 'pub mod task_dialog;' not in text and text.count('pub mod menu;') == 1
    path.write_text(text.replace('pub mod menu;', 'pub mod menu;\npub mod task_dialog;'), encoding='utf-8')
    path = Path('crates/synara-app/src/shell/overview.rs')
    text = path.read_text()
    start = text.index('    pub(super) fn kanban_panel')
    end = text.index('    pub(super) fn help_panel')
    assert start < end
    text = text[:start] + text[end:]
    text = text.replace('use crate::ui::{self, palette};', 'use crate::ui;')
    path.write_text(text, encoding='utf-8')
    # Group form inputs into a single configuration value, not an eight-argument API.
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
    assert text.count(old) == 1
    config = '''pub struct TaskDialogConfig {
    pub projects: Vec<(ProjectId, String)>,
    pub agents: Vec<(String, String, Glyph)>,
    pub initial_project: Option<ProjectId>,
    pub default_agent: Option<String>,
    pub draft: bool,
    pub send_on_enter: bool,
}
'''
    text = text.replace('pub struct TaskDialog {', config + 'pub struct TaskDialog {', 1)
    text = text.replace(old, '''    pub fn new(config: TaskDialogConfig, cx: &mut Context<Self>) -> Self {
        let TaskDialogConfig { projects, agents, initial_project, default_agent, draft, send_on_enter } = config;''', 1)
    text = text.replace('Some(id.as_str()) == default_agent)', 'Some(id.as_str()) == default_agent.as_deref())')
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
    assert text.count('mod chat_preferences;') == 1 and 'mod task_creation;' not in text
    path.write_text(text.replace('mod chat_preferences;', 'mod chat_preferences;\nmod task_creation;'), encoding='utf-8')
    print('Integrated only the reviewed task creation, board, and modal boundaries.')

if __name__ == '__main__':
    main()
