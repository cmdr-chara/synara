#!/usr/bin/env python3
"""One owned native journey for Spaces, project assignment and saved chat context."""
import argparse
import json
import sqlite3
from pathlib import Path

from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection
from native_presentation_smoke import resize


def preference(s, key):
    with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
        row = db.execute('SELECT data FROM preferences WHERE key=?', (key,)).fetchone()
        return json.loads(row[0]) if row else None


def close(s):
    s.desktop.request_close()
    wait_until(lambda: s.process.poll() is not None, 'close after metadata saves')
    assert s.process.returncode == 0
    s.log.close()
    s.log = None


def run(s):
    s.launch()
    ui = s.desktop
    resize(ui, 1280, 900, s.scale)
    task = selection(s)
    events = s.events()
    s.click_control('composer-input')
    ui.text('Keep this unsent prompt.')
    wait_until(lambda: preference(s, 'task-draft:' + task), 'draft persistence')
    s.click_control('spaces-manage')
    wait_until(lambda: s.control_bounds('organization-dialog'), 'Space manager')
    s.click_control('space-create')
    s.click_control('space-name-input')
    ui.text('Work')
    s.click_control('space-save')
    organization = wait_until(lambda: preference(s, 'workspace-organization'), 'Space creation')
    work = organization['active']
    assert organization['spaces'][0]['name'] == 'Work'
    s.click_control('project-space', slot=0)
    ui.text('Work')
    ui.key('Return')
    wait_until(lambda: preference(s, 'workspace-organization')['project_spaces'], 'project assignment')
    organization = preference(s, 'workspace-organization')
    assert list(organization['project_spaces'].values()) == [work]
    ui.screenshot('spaces-project-manager', window_only=True)
    s.click_control('organization-close')
    s.click_control('space-tab', slot=0)
    wait_until(lambda: preference(s, 'workspace-organization')['active'] is None, 'Void selection')
    assert selection(s) == task
    s.click_control('space-tab', slot=1)
    wait_until(lambda: preference(s, 'workspace-organization')['active'] == work, 'named Space selection')
    s.click_control('composer-input')
    assert ui.copy_input() == 'Keep this unsent prompt.'
    ui.focus()
    assert s.events() == events
    ui.screenshot('spaces-sidebar', window_only=True)
    s.checks.append('Space-create-assign-and-sidebar-switch-preserve-chat-draft-and-events')

    s.click_control('chat-notes')
    wait_until(lambda: s.control_bounds('context-notes-input'), 'loaded notes editor')
    s.click_control('context-notes-input')
    ui.text('Private notes for this chat.\nKeep the original design.')
    s.click_control('context-item-input')
    ui.text('Review the sidebar')
    s.click_control('context-item-add')
    s.click_control('context-item-input')
    ui.text('Check saved drafts')
    s.click_control('context-item-add')
    s.click_control('context-check', slot=0)
    s.click_control('context-save')
    key = 'task-context:' + task
    saved = wait_until(lambda: preference(s, key), 'notes and checklist save')
    assert saved['notes'] == 'Private notes for this chat.\nKeep the original design.'
    assert [item['text'] for item in saved['checklist']] == ['Review the sidebar', 'Check saved drafts']
    assert [item['done'] for item in saved['checklist']] == [True, False]
    ui.screenshot('chat-notes-checklist', window_only=True)
    s.click_control('context-to-draft')
    wait_until(lambda: '## My notes' in preference(s, 'task-draft:' + task)['text'], 'explicit insertion into draft')
    draft = preference(s, 'task-draft:' + task)['text']
    assert draft.startswith('Keep this unsent prompt.\n\n')
    assert '- [x] Review the sidebar' in draft and '- [ ] Check saved drafts' in draft
    assert s.events() == events
    s.click_control('context-close')
    s.checks.append('saved-notes-checklist-and-explicit-draft-insertion-never-send-a-prompt')

    s.click_control('chat-notes')
    wait_until(lambda: s.control_bounds('context-notes-input'), 'reopened notes')
    s.click_control('context-notes-input')
    ui.key('a', ('Control_L',))
    ui.text('Unsaved note changes')
    s.click_control('context-close')
    s.click_control('context-keep')
    s.click_control('context-notes-input')
    assert ui.copy_input() == 'Unsaved note changes'
    ui.focus()
    assert preference(s, key) == saved
    s.click_control('context-close')
    s.click_control('context-discard')
    close(s)
    s.launch(preserve_selection=True)
    assert preference(s, 'workspace-organization')['active'] == work
    assert preference(s, key) == saved
    assert selection(s) == task and s.events() == events
    s.click_control('composer-input')
    assert ui.copy_input() == draft
    ui.focus()
    s.click_control('chat-notes')
    wait_until(lambda: s.control_bounds('context-notes-input'), 'notes restored after application restart')
    ui.screenshot('organization-and-notes-restored', window_only=True)
    s.click_control('context-close')
    s.checks.append('unsaved-note-close-guard-and-restart-preserve-organization-context-and-draft')
    close(s)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = dict(status='failed', checks=scenario.checks, platform='Linux/X11/private Xvfb')
    try:
        run(scenario)
        result['status'] = 'passed'
    except BaseException:
        import traceback
        result['error'] = traceback.format_exc()
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure')
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
