#!/usr/bin/env python3
"""Native project/task drafting, release-safe creation, run/stop and restart.

Uses only Scenario's owned Xvfb, SQLite, folders and ACP fixture processes.
"""
import argparse
import json
from pathlib import Path
import sqlite3
import time
import uuid
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, event_cursor, task_events, prompt_finished
from native_studio_settings_smoke import tasks
from native_presentation_smoke import resize


def draft(s, task):
    with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
        row = db.execute('SELECT data FROM preferences WHERE key=?', ('task-draft:' + task,)).fetchone()
        return json.loads(row[0])['text'] if row else ''


def fill(s, text):
    s.click_control('task-prompt')
    s.desktop.key('a', ('Control_L',))
    s.desktop.text(text)
    assert s.desktop.copy_input() == text
    s.desktop.focus()


def close(s):
    s.desktop.request_close()
    wait_until(lambda: s.process.poll() is not None, 'draft-safe shutdown')
    assert s.process.returncode == 0
    s.log.close()
    s.log = None


def assistant_has_text(events, expected):
    # ACP streams messages in chunks. Do not require the entire answer in one
    # delta or accidentally assemble text across different messages/roles.
    messages = {}
    for event in events:
        if event.get('type') == 'text_delta' and event.get('role') == 'assistant':
            key = event['message_id']
            messages[key] = messages.get(key, '') + event.get('text', '')
    return any(expected in value for value in messages.values())


def open_project(s, project_id):
    with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
        # The native overview uses the catalog's workspace/project ordering.
        projects = [row[0] for row in db.execute('SELECT id FROM projects ORDER BY workspace_id,id')
                    if any(t['project_id'] == row[0] and t['state'] != 'archived' and t['scope'] != 'studio'
                           for t in tasks(s).values())]
    s.click_control('kanban-open-project', slot=projects.index(project_id))


def run(s):
    s.launch()
    original = selection(s)
    original_project = tasks(s)[original]['project_id']
    close(s)
    second = str(uuid.uuid4())
    # Register only fixture data before startup, then select it through the UI.
    (s.project / 'second-project').mkdir()
    with sqlite3.connect(s.data / 'native-workspace.sqlite3') as db:
        p = json.loads(db.execute('SELECT data FROM projects WHERE id=?', (original_project,)).fetchone()[0])
        p.update(id=second, name='Second project', relative_directory='second-project')
        db.execute('INSERT INTO projects(id,workspace_id,data) VALUES(?,?,?)', (second, p['workspace_id'], json.dumps(p)))
        db.execute('INSERT OR REPLACE INTO preferences(key,data) VALUES(?,?)',
                   ('settings', json.dumps({'version': 1, 'appearance': {'theme': 'dark', 'dark_theme': 'dracula'}})))
    s.launch(preserve_selection=True)
    ui = s.desktop
    resize(ui, 1280, 803, s.scale)
    s.click_control('composer-input')
    ui.text('keep the original conversation draft')
    wait_until(lambda: draft(s, original) == 'keep the original conversation draft', 'original draft saved')
    ui.key('9', ('Control_L',))
    s.click_control('kanban-new-task')
    s.click_control('task-create')
    assert len(tasks(s)) == 1, 'Empty form created a task'
    fill(s, 'hello')
    ui.key('t', ('Control_L', 'Alt_L'))
    s.click_control('task-prompt')
    assert ui.copy_input() == 'hello', 'Repeated shortcut remounted the form'
    ui.focus()
    s.click_control('task-project')
    ui.key('Home')
    # Project ordering is catalog/UUID order, so use the native chooser search.
    ui.text('Second project')
    ui.key('Return')
    s.click_control('task-agent')
    ui.text('beta')
    ui.key('Return')
    s.click_control('task-as-draft')
    ui.screenshot('new-task-draft-ready', window_only=True)
    s.click_control('task-create')
    new = wait_until(lambda: next((t for t in tasks(s).values() if t['id'] != original), None), 'atomic task creation')
    task = new['id']
    assert new['project_id'] == second and new['agent_id'] == 'beta'
    assert new['scope'] == 'project' and new['state'] == 'ready'
    assert Path(new['working_directory']) == s.project / 'second-project'
    assert draft(s, task) == 'hello' and event_cursor(s, task) == 0
    assert selection(s) == original and draft(s, original) == 'keep the original conversation draft'
    ui.screenshot('kanban-project-overview', window_only=True)
    open_project(s, second)
    ui.screenshot('kanban-draft-task', window_only=True)
    s.checks.append('isolated-composer-project-agent-and-atomic-draft-without-autostart')

    s.click_control('kanban-open-draft', slot=0)
    wait_until(lambda: selection(s) == task, 'open persisted draft')
    s.click_control('composer-input')
    assert ui.copy_input() == 'hello'
    ui.focus()
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == task and draft(s, task) == 'hello' and event_cursor(s, task) == 0
    ui.key('9', ('Control_L',))
    open_project(s, second)
    ui.screenshot('kanban-restored', window_only=True)
    s.checks.append('project-overview-drilldown-and-draft-restore-without-agent-start')

    cursor = event_cursor(s, task)
    s.click_control('kanban-run-draft', slot=0)
    wait_until(lambda: prompt_finished(s, task, cursor), 'run the saved draft')
    events = task_events(s, task, cursor)
    assert sum(e['type'] == 'prompt_started' for e in events) == 1
    assert assistant_has_text(events, 'Hello from beta'), 'Expected the selected provider answer across its streamed message chunks'
    wait_until(lambda: draft(s, task) == '', 'acknowledged draft clear')
    assert event_cursor(s, original) == 0 and draft(s, original) == 'keep the original conversation draft'
    ui.screenshot('kanban-task-done', window_only=True)
    s.checks.append('explicit-run-uses-real-controller-selected-provider-and-preserves-other-drafts')

    before = set(tasks(s))
    s.click_control('kanban-new-task')
    fill(s, 'hold')
    ui.screenshot('new-task-run-ready', window_only=True)
    s.click_control('task-create')
    running = wait_until(lambda: next((t for key, t in tasks(s).items() if key not in before), None), 'create and run task')['id']
    wait_until(lambda: assistant_has_text(task_events(s, running, 0), 'Started waiting'), 'running backend')
    wait_until(lambda: s.control_bounds('kanban-stop-task', slot=0), 'running task control')
    ui.screenshot('kanban-running-task', window_only=True)
    s.click_control('kanban-stop-task', slot=0)
    wait_until(lambda: prompt_finished(s, running, 0), 'explicit cancellation')
    assert any(e['type'] == 'cancellation_requested' for e in task_events(s, running, 0))
    s.checks.append('create-and-run-and-board-stop-drive-real-agent-lifecycle')

    before = set(tasks(s))
    s.click_control('kanban-add-draft')
    fill(s, 'do not discard silently')
    ui.request_close()
    time.sleep(0.3)
    assert s.process.poll() is None, 'Unsubmitted form was lost on window close'
    s.click_control('task-close')
    ui.screenshot('new-task-discard-confirmation', window_only=True)
    s.click_control('task-discard')
    assert set(tasks(s)) == before
    s.checks.append('closing-window-and-dismissing-form-protect-unsaved-task-text')

    for width, height in [(1100, 760), (960, 700)]:
        resize(ui, width, height, s.scale)
        s.click_control('kanban-add-draft')
        x, y, w, h = s.control_bounds('new-task-dialog')
        assert 0 <= x and x + w <= width + 1 and 0 <= y and y + h <= height + 1
        ui.screenshot(f'new-task-{width}', window_only=True)
        ui.key('Escape')
    s.checks.append('native-task-modal-and-scrollable-board-at-intermediate-widths')
    close(s)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = dict(status='failed', checks=s.checks, platform='Linux/X11/private Xvfb', agents=['fixture-alpha', 'fixture-beta'])
    try:
        run(s)
        result['status'] = 'passed'
    except BaseException:
        import traceback
        result['error'] = traceback.format_exc()
        if s.process and s.process.poll() is None:
            s.desktop.screenshot('failure')
        raise
    finally:
        s.close()
        (s.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))

if __name__ == '__main__':
    main()
