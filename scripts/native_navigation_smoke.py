#!/usr/bin/env python3
"""Native navigation regression on a private display and isolated fixture data."""
import argparse
import ctypes as C
import json
from pathlib import Path
import sqlite3
import time

from native_smoke import Scenario, wait_until


def database(scenario):
    path = scenario.data / 'native-workspace.sqlite3'
    return sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)


def selection(scenario):
    with database(scenario) as db:
        row = db.execute("SELECT data FROM preferences WHERE key='selection'").fetchone()
        return json.loads(row[0]).get('task') if row else None


def task_count(scenario):
    with database(scenario) as db:
        return db.execute('SELECT COUNT(*) FROM tasks').fetchone()[0]


def event_cursor(scenario, task_id):
    with database(scenario) as db:
        return db.execute(
            'SELECT COALESCE(MAX(sequence),0) FROM events WHERE thread_id='
            '(SELECT thread_id FROM tasks WHERE id=?)', (task_id,)).fetchone()[0]


def task_events(scenario, task_id, after):
    # Sequence belongs to one thread, not the whole database. Slicing globally
    # sorted events by a previous row count can select a sibling's old finish
    # while hiding the new user message at a lower per-thread sequence.
    with database(scenario) as db:
        return [json.loads(row[0]) for row in db.execute(
            'SELECT data FROM events WHERE thread_id='
            '(SELECT thread_id FROM tasks WHERE id=?) AND sequence>? ORDER BY sequence',
            (task_id, after))]


def prompt_finished(scenario, task_id, cursor):
    return any(event['type'] == 'prompt_finished'
               for event in task_events(scenario, task_id, cursor))


def key_edge(ui, name, pressed):
    code = ui.x.XKeysymToKeycode(ui.display, ui.x.XStringToKeysym(name.encode()))
    assert code, 'unmapped test key'
    ui.xt.XTestFakeKeyEvent(ui.display, code, int(pressed), 0)
    ui.x.XFlush(ui.display)
    time.sleep(0.1)


def focus_without_activation(ui, x, y, outside_x):
    left, top, width, height = ui.geometry()
    assert 0 <= x < width and 0 <= outside_x < width and 0 <= y < height
    ui.xt.XTestFakeMotionEvent(ui.display, -1, left + x, top + y, 0)
    ui.xt.XTestFakeButtonEvent(ui.display, 1, 1, 0)
    ui.x.XFlush(ui.display)
    time.sleep(0.1)
    ui.xt.XTestFakeMotionEvent(ui.display, -1, left + outside_x, top + y, 0)
    ui.xt.XTestFakeButtonEvent(ui.display, 1, 0, 0)
    ui.x.XFlush(ui.display)
    time.sleep(0.2)


def run(scenario):
    scenario.launch()
    ui = scenario.desktop
    _, _, width, height = ui.geometry()
    original = selection(scenario)
    assert original and task_count(scenario) == 1
    ui.screenshot('navigation-initial')

    scenario.click_control('workspace-tools')
    ui.screenshot('mode-switcher-open')
    ui.key('Escape')
    ui.key('Return')  # Escape returns focus to the brand, reopening its mode menu.
    ui.key('Return')  # Current Synara mode; this must not create a task.
    assert task_count(scenario) == 1
    ui.key('7', ('Control_L',))
    time.sleep(0.6)
    before = scenario.events()
    scenario.click_control('registry-query')
    ui.text('smoke')
    scenario.click_control('registry-review', slot=0)
    assert not list(scenario.agents.glob('agent-*/receipt.json'))
    scenario.click_control('registry-confirm')
    wait_until(lambda: list(scenario.agents.glob('agent-*/receipt.json')), 'menu-selected agent approval')
    assert scenario.events() == before, 'Browsing and approving an agent must not launch it'
    scenario.checks.append('mode-menu-keyboard-dismissal-and-agent-settings-approval')

    # Approval removes its focused control. The application must restore focus
    # so this shortcut reaches the shell without an extra recovery mouse click.
    ui.key('1', ('Control_L',))
    time.sleep(0.3)
    scenario.click_control('composer-input')
    ui.text('saved draft')
    initial_draft = ui.copy_input()
    assert initial_draft == 'saved draft', f'Composer did not receive synthetic draft: {initial_draft!r}'
    scenario.checks.append('retired-approval-control-restores-shell-keyboard-navigation')
    x, y, w, h = scenario.control_bounds('new-thread')
    focus_without_activation(ui, round(x + w / 2), round(y + h / 2), 290)
    assert task_count(scenario) == 1, 'Aborted pointer activation created a thread'
    key_edge(ui, 'Return', True)
    assert task_count(scenario) == 1, 'A held key dispatched before release'
    key_edge(ui, 'Return', False)
    wait_until(lambda: task_count(scenario) == 2 and selection(scenario) != original,
               'one keyboard-created thread and durable selection')
    time.sleep(0.2)
    assert task_count(scenario) == 2, 'One key press dispatched more than once'
    new_task = selection(scenario)
    before = event_cursor(scenario, new_task)
    scenario.prompt('hello')
    wait_until(lambda: prompt_finished(scenario, new_task, before), 'new-thread backend completion')
    events = task_events(scenario, new_task, before)
    assistant_text = ''.join(event.get('text', '') for event in events
                             if event['type'] == 'text_delta' and event.get('role') == 'assistant')
    assert 'Hello from alpha' in assistant_text
    scenario.checks.append('native-key-release-creates-exactly-one-backend-thread')
    scenario.checks.append('sidebar-new-thread-uses-controller-and-durable-acp-streaming')

    scenario.click_control('thread-row', slot=1)
    wait_until(lambda: selection(scenario) == original, 'original thread selected')
    assert original != new_task
    time.sleep(0.3)
    restored = ui.copy_input()
    assert restored == 'saved draft', f'Synthetic draft not restored: {restored!r}'
    scenario.click_control('history-back')
    wait_until(lambda: selection(scenario) == new_task, 'back navigation')
    scenario.click_control('history-forward')
    wait_until(lambda: selection(scenario) == original, 'forward navigation')
    scenario.click_control('composer-input')
    assert ui.copy_input() == 'saved draft', 'History traversal lost the thread draft'
    scenario.checks.append('history-traversal-restores-selection-and-thread-drafts')
    before = event_cursor(scenario, original)
    sibling_before = event_cursor(scenario, new_task)
    assert not prompt_finished(scenario, original, before), 'Sibling finish leaked into scoped oracle'
    ui.focus()
    ui.key('Return')
    wait_until(lambda: prompt_finished(scenario, original, before), 'restored draft submitted')
    submitted = [event.get('text') for event in task_events(scenario, original, before)
                 if event['type'] == 'text_delta' and event.get('role') == 'user']
    assert submitted == ['saved draft'], f'Synthetic restored draft mismatch: {submitted!r}'
    assert event_cursor(scenario, new_task) == sibling_before, 'Submitting a draft changed its sibling'
    scenario.checks.append('compact-thread-row-preserves-draft-and-submits-to-own-backend-thread')
    ui.screenshot('navigation-conversation')

    old_events = scenario.events()
    scenario.click_control('sidebar-toggle')
    time.sleep(0.35)
    ui.screenshot('sidebar-collapsed')
    scenario.click_control('sidebar-toggle')
    time.sleep(0.35)
    assert scenario.events() == old_events, 'Disclosure must not create agent events'
    scenario.checks.append('sidebar-collapse-is-presentation-only')

    resize = ui.x.XResizeWindow
    resize.argtypes = [C.c_void_p, C.c_ulong, C.c_uint, C.c_uint]
    resize.restype = C.c_int
    for width, height in [(1280, 800), (960, 700), (1420, 930)]:
        resize(ui.display, ui.window, width, height)
        ui.x.XFlush(ui.display)
        wait_until(lambda: ui.geometry()[2:] == (width, height), 'private native resize')
        time.sleep(0.3)
        assert scenario.process.poll() is None
        assert selection(scenario) == original
        ui.screenshot(f'navigation-{width}')
    scenario.checks.append('native-resize-retains-process-and-selection')

    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'native navigation shutdown')
    assert scenario.process.returncode == 0
    scenario.checks.append('native-window-closes-through-owned-controller')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks,
              'platform': 'Linux/X11/private Xvfb', 'initial_window': [1420, 930]}
    try:
        run(scenario)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.log:
            print('\n'.join(line for line in Path(scenario.log.name).read_text(errors='replace').splitlines()
                            if 'synara_ui_layout' in line)[-12000:])
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure')
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
