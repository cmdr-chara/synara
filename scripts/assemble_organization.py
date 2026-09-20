#!/usr/bin/env python3
"""One-shot, exact-base wiring for the authorized organization batch."""
from pathlib import Path
import subprocess

EXPECTED = {
    'crates/synara-app/src/shell.rs': 'c4b28c1aa93fe9749a1d87f7eef9ae986523c548',
    'crates/synara-app/src/shell/navigation.rs': '91370fa60c58cecb2637b49024813e038e42bc31',
    'crates/synara-app/src/shell/chrome.rs': '85c10c07b031f8e1bda58d6a67e68588dd01f87f',
    'crates/synara-workspace/src/storage.rs': '6a636f5d7503f00a4dc6493845d047eadbd9cdd0',
}
for path, expected in EXPECTED.items():
    assert subprocess.check_output(['git', 'hash-object', '--', path]).decode().strip() == expected, path

def replace(path, old, new):
    file = Path(path)
    content = file.read_text()
    assert content.count(old) == 1, (path, old[:100], content.count(old))
    file.write_text(content.replace(old, new), encoding='utf-8', newline='')

storage = 'crates/synara-workspace/src/storage.rs'
replace(storage, 'mod chat_preferences;\n', 'mod chat_preferences;\nmod organization;\npub use organization::{NativeSpace, OrganizationEdit, SpaceSymbol, WorkspaceOrganization};\n')
replace(storage, '"model-favorites" | "environment-layout"', '"model-favorites" | "environment-layout" | "workspace-organization"')
shell = 'crates/synara-app/src/shell.rs'
replace(shell, 'mod navigation;\n', 'mod navigation;\nmod organization;\n')
replace(shell, 'enum Update {\n', 'enum Update {\n    Organization(Box<organization::OrganizationReply>),\n')
replace(shell, 'pub struct Shell {\n', 'pub struct Shell {\n    organization: organization::OrganizationState,\n')
replace(shell, 'let mut this = Self {\n', 'let mut this = Self {\n            organization: organization::OrganizationState::new(cx),\n')
replace(shell, '        this.composer.update(cx, |entry, _| {', '        this.load_organization();\n        this.composer.update(cx, |entry, _| {')
replace(shell, '        match update {\n', '        match update {\n            Update::Organization(reply) => self.organization_reply(*reply, cx),\n')
replace(shell, '        if self.chat_tools.pending_write() {', '        if self.organization.saving || self.organization.dialog.is_some() {\n            self.notice = Some("Finish the Space save and close its manager before closing Synara.".into());\n            cx.notify();\n            return false;\n        }\n        if self.chat_tools.pending_write() {')
nav = 'crates/synara-app/src/shell/navigation.rs'
replace(nav, '.filter(|project| !self.is_chat_workspace(project))', '.filter(|project| !self.is_chat_workspace(project) && self.project_in_active_space(project.id))')
replace(nav, '        if self.settings.value.general.oldest_threads_first {\n            tasks.reverse();\n        }', '        if self.settings.value.general.oldest_threads_first {\n            tasks.reverse();\n        }\n        tasks.sort_by_key(|task| !self.pinned_thread(task.id));')
replace(nav, '        if self.settings.value.general.alphabetical_projects {\n            projects.sort_by_key(|project| project.name.to_lowercase());\n        }', '        if self.settings.value.general.alphabetical_projects {\n            projects.sort_by_key(|project| project.name.to_lowercase());\n        }\n        projects.sort_by_key(|project| !self.pinned_project(project.id));')
replace(nav, '.child(ui::layout_probe_slot("thread-row", index))', '.child(ui::layout_probe_slot("thread-row", index))\n        .child(self.thread_pin_button(task, cx))')
replace(nav, '            .children(\n                self.navigation\n                    .search_open', '            .children((!studio).then(|| self.space_strip(cx)))\n            .children(\n                self.navigation\n                    .search_open')
chrome = 'crates/synara-app/src/shell/chrome.rs'
replace(chrome, '            && self.kanban.dialog.is_none()', '            && self.kanban.dialog.is_none()\n            && self.organization.dialog.is_none()')
replace(chrome, '        self.restore_environment_focus(window, cx);', '        self.restore_environment_focus(window, cx);\n        self.restore_organization_focus(window, cx);')
replace(chrome, '                if this.kanban.dialog.is_some() {', '                if this.kanban.dialog.is_some() || this.organization.dialog.is_some() {')
replace(chrome, '                if this.chat_tools_shortcut(event, window, cx) {', '                if this.organization_shortcut(event, window, cx) {\n                    cx.stop_propagation();\n                    return;\n                }\n                if this.chat_tools_shortcut(event, window, cx) {')
replace(chrome, '            .children(self.kanban.dialog.clone())', '            .children(self.kanban.dialog.clone())\n            .children(self.organization.dialog.clone())')
replace('crates/synara-app/src/shell/organization/dialog.rs', 'Glyph::Trash', 'Glyph::Close')
replace('ROADMAP.md', '## Current checkpoint\n', '''## Current checkpoint

### September 20: Spaces and organization batch

Space tabs, create/rename/icon editing, ordering, project assignment and guarded
Space deletion are implemented as a new candidate. Deleting a Space returns its
projects to Void without deleting files or chats. Project and thread pins are
persisted and ordered ahead of unpinned entries. Switching Spaces filters the
sidebar without stopping tools or changing the active conversation. Keyboard
switching/reordering and activity indicators use the existing native controls.
The [organization receipt](docs/ui/parity-organization-context.md) separates source
implementation from targeted and native acceptance. No F7/F8/I10 gate is closed
until its evidence exists. Original theme defaults remain unchanged.
''')
Path('.github/workflows/native.yml').write_bytes(subprocess.check_output(['git', 'show', '7dbbb2f8fb20fe3306973d54b83bd3265d894cca:.github/workflows/native.yml']))
Path(__file__).unlink()
