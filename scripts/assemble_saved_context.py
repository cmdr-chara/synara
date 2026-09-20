#!/usr/bin/env python3
"""Exact-base integration of saved context into the completed Space batch."""
from pathlib import Path
import subprocess

EXPECTED = {
    'crates/synara-app/src/input.rs': '1126d383375ec02f7ea2ee3e8612686d3bc36e0f',
    'crates/synara-app/src/shell.rs': '53ffec8893b0618766472adb46c2bbb1c78b1246',
    'crates/synara-app/src/shell/chrome.rs': '0e40386b9435acb12abbea467111d14b66254c23',
    'crates/synara-workspace/src/storage.rs': 'd9aa92ac8082d768f36a8b636b6760fc3054af4d',
}
for path, expected in EXPECTED.items():
    assert subprocess.check_output(['git', 'hash-object', '--', path]).decode().strip() == expected, path

def replace(path, old, new):
    file = Path(path)
    content = file.read_text()
    assert content.count(old) == 1, (path, old[:100], content.count(old))
    file.write_text(content.replace(old, new), encoding='utf-8', newline='')

replace('crates/synara-app/src/input.rs', '    pub fn text(&self) -> &str {', '''    /// Whether a platform IME currently owns marked text. Presentation shortcuts
    /// must leave Enter/Escape and candidate selection to that composition.
    pub fn is_composing(&self) -> bool {
        self.buffer.marked().is_some()
    }

    pub fn text(&self) -> &str {''')
storage = 'crates/synara-workspace/src/storage.rs'
replace(storage, 'mod organization;\n', 'mod organization;\nmod task_context;\npub use task_context::{ChecklistItem, TaskContext, MAX_NOTE_BYTES, MAX_CHECKLIST_ITEMS, MAX_CHECKLIST_TEXT};\n')
replace(storage, '.or_else(|| key.strip_prefix("message-pins:"))', '.or_else(|| key.strip_prefix("message-pins:")).or_else(|| key.strip_prefix("task-context:"))')
replace(storage, '"DELETE FROM preferences WHERE key IN (?1,?2)",', '"DELETE FROM preferences WHERE key IN (?1,?2,?3)",')
replace(storage, 'params![format!("task-draft:{id}"), format!("message-pins:{id}")]', 'params![format!("task-draft:{id}"), format!("message-pins:{id}"), format!("task-context:{id}")]')
shell = 'crates/synara-app/src/shell.rs'
replace(shell, 'mod organization;\n', 'mod organization;\nmod saved_context;\n')
replace(shell, 'enum Update {\n', 'enum Update {\n    SavedContext(Box<saved_context::ContextReply>),\n')
replace(shell, 'pub struct Shell {\n', 'pub struct Shell {\n    saved_context: saved_context::SavedContextState,\n')
replace(shell, 'let mut this = Self {\n', 'let mut this = Self {\n            saved_context: saved_context::SavedContextState::default(),\n')
replace(shell, '        match update {\n', '        match update {\n            Update::SavedContext(reply) => self.saved_context_reply(*reply, cx),\n')
replace(shell, '        if self.organization.saving || self.organization.dialog.is_some() {', '        if self.organization.saving || self.organization.dialog.is_some() || self.saved_context.dialog.is_some() {')
replace(shell, 'Finish the Space save and close its manager before closing Synara.', 'Finish pending saves and close the Space or notes editor before closing Synara.')
chrome = 'crates/synara-app/src/shell/chrome.rs'
replace(chrome, '            && self.organization.dialog.is_none()', '            && self.organization.dialog.is_none()\n            && self.saved_context.dialog.is_none()')
replace(chrome, '        self.restore_organization_focus(window, cx);', '        self.restore_organization_focus(window, cx);\n        self.restore_saved_context_focus(window, cx);')
replace(chrome, 'if this.kanban.dialog.is_some() || this.organization.dialog.is_some() {', 'if this.kanban.dialog.is_some() || this.organization.dialog.is_some() || this.saved_context.dialog.is_some() {')
replace(chrome, '            .children(self.organization.dialog.clone())', '            .children(self.organization.dialog.clone())\n            .children(self.saved_context.dialog.clone())')
chat = Path('crates/synara-app/src/shell/chat_tools.rs')
text = chat.read_text()
start = text.index('    pub(super) fn chat_tools_bar(')
end = text.index('    pub(super) fn message_find_bar(', start)
section = text[start:end]
assert section.count('.gap_1()') == 1
section = section.replace('.gap_1()', '.gap_1()\n            .child(self.saved_context_button(cx))')
chat.write_text(text[:start] + section + text[end:], encoding='utf-8', newline='')
# The view is referenced by the sibling shell chrome, not just its parent module.
replace('crates/synara-app/src/shell/organization/dialog.rs', 'pub(super) struct OrganizationDialog {', 'pub(in crate::shell) struct OrganizationDialog {')
notes = 'crates/synara-app/src/shell/saved_context.rs'
for control, label in [('context-keep', 'Keep editing'), ('context-discard', 'Discard edits')]:
    replace(notes, f'ui::button("{control}", "{label}", false)', f'ui::button("{control}", "{label}", false).relative().child(ui::layout_probe("{control}"))')
replace('ROADMAP.md', '## Current checkpoint\n', '''## Current checkpoint

### September 20: saved chat notes and checklists

Each conversation now has user-owned notes and an ordered checklist with add,
edit, complete, hide-completed, reorder and remove controls. Explicit Save uses a
revision check to reject stale edits. Copy and Add to draft preserve current chat
text and never send automatically. Close/reload guards protect unfinished edits,
and permanent task deletion removes its context in the same transaction.
D10/F2/I10 remain partial. The [organization/context receipt](docs/ui/parity-organization-context.md)
records the focused combined checks separately from remaining visual/platform work.
''')
Path('.github/workflows/native.yml').write_bytes(subprocess.check_output(['git', 'show', '7dbbb2f8fb20fe3306973d54b83bd3265d894cca:.github/workflows/native.yml']))
Path(__file__).unlink()
