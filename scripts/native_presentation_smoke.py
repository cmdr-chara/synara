#!/usr/bin/env python3
"""Check native home geometry and motion on an isolated, owned X11 display."""
import argparse
import ctypes as C
import json
from pathlib import Path
import re
import sqlite3
import time
import uuid

from native_smoke import Scenario, wait_until


def log_text(scenario):
    return re.sub(r'\x1b\[[0-9;]*[A-Za-z]', '',
                  Path(scenario.log.name).read_text(errors='replace'))


def widths(text):
    return [float(value) for value in re.findall(
        r'control="sidebar-drawer"[^\n]*?\bwidth=([0-9.]+)', text)]


def resize(ui, width, height, scale=1):
    width, height = round(width * scale), round(height * scale)
    operation = ui.x.XMoveResizeWindow
    operation.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint]
    operation.restype = C.c_int
    operation(ui.display, ui.window, 0, 0, width, height)
    ui.x.XFlush(ui.display)
    wait_until(lambda: ui.geometry() == (0, 0, width, height), 'owned window geometry')
    time.sleep(0.35)


def seed_visual_catalog(scenario):
    """Shape-matched content, only in this owned fixture while its app is closed.

    These rows exercise truncation, five-row expansion and section spacing. They
    are not a transcript fixture and are never shipped as application content.
    """
    scenario.desktop.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'close before visual fixture')
    assert scenario.process.returncode == 0
    scenario.log.close()
    scenario.log = None
    with sqlite3.connect(scenario.data / 'native-workspace.sqlite3') as db:
        project = json.loads(db.execute('SELECT data FROM projects LIMIT 1').fetchone()[0])
        project['name'] = 'Xiaomi'
        db.execute('UPDATE projects SET data=? WHERE id=?', (json.dumps(project), project['id']))
        original = json.loads(db.execute('SELECT data FROM tasks LIMIT 1').fetchone()[0])
        titles = ['Check Device Synara Source'] * 4 + ['can you summarize', 'Earlier conversation']
        for index, title in enumerate(titles):
            task = dict(original, id=str(uuid.uuid4()), thread_id=str(uuid.uuid4()),
                        title=title, state='completed', scope='chat',
                        updated_at_ms=original['updated_at_ms'] - (index + 1) * 1000)
            db.execute('INSERT INTO tasks (id,project_id,thread_id,updated_ms,data) VALUES (?,?,?,?,?)',
                       (task['id'], task['project_id'], task['thread_id'], task['updated_at_ms'], json.dumps(task)))
    scenario.launch()


def run(scenario):
    scenario.launch()
    seed_visual_catalog(scenario)
    ui = scenario.desktop
    reference_height = 1032 / 1.25 if scenario.scale == 1.25 else 826
    initial_events = scenario.events()
    assert widths(log_text(scenario)) and all(
        abs(width - 256) < 0.1 for width in widths(log_text(scenario))), 'Drawer animated on mount'
    for width, height in [(1536, reference_height), (1280, 803), (1100, 760), (960, 700)]:
        resize(ui, width, height, scenario.scale)
        x, y, composer_width, composer_height = scenario.control_bounds('composer-surface')
        # The probe measures the inside of the 1 px border.
        assert abs(composer_width + 2 - min(736, width - 256 - 40)) < 1
        assert abs(x + composer_width / 2 - (width + 256) / 2) < 1
        assert height - 20 <= y + composer_height <= height - 10
        scenario.click_control('project-picker', enabled=True)
        ui.key('Escape')
        # Clear pointer/focus treatment before the visual comparison. Keyboard
        # return-to-trigger focus is separately verified by the controls suite.
        scenario.click_control('welcome-heading')
        ui.screenshot(f'home-{width}', window_only=True)
    scenario.checks.append('home-composer-cap-centering-bottom-anchor-at-four-native-sizes')
    menu_frames = [float(value) for value in re.findall(
        r'surface="session-menu" progress=([0-9.]+)', log_text(scenario))]
    assert any(0 < value < 1 for value in menu_frames) and menu_frames[-1] == 1
    scenario.checks.append('popup-opening-renders-intermediate-fade-and-scale-frames')
    scenario.click_control('project-picker', enabled=True)
    ui.key('Return')  # Selecting the current persisted project is a no-op.
    assert scenario.events() == initial_events
    scenario.checks.append('project-picker-dismissal-and-current-project-selection-do-not-start-an-agent')

    scenario.click_control('search-threads')
    ui.text('no matching thread')
    ui.screenshot('search-empty', window_only=True)
    scenario.click_control('search-threads')
    assert scenario.events() == initial_events
    scenario.checks.append('search-can-open-type-and-close-without-changing-agent-state')

    scenario.click_control('kanban-navigation')
    wait_until(lambda: scenario.control_bounds('kanban-board'), 'native board')
    scenario.click_control('kanban-ready-task')
    scenario.click_control('help-navigation')
    wait_until(lambda: scenario.control_bounds('help-panel'), 'native help')
    ui.key('1', ('Control_L',))
    assert scenario.events() == initial_events
    scenario.checks.append('kanban-opens-a-real-catalog-task-and-help-returns-without-an-agent-launch')

    resize(ui, 1536, reference_height, scenario.scale)
    expanded_center = scenario.control_bounds('composer-surface')[0]
    marker = len(log_text(scenario))
    scenario.click_control('sidebar-toggle', settle=0.4)
    closing = widths(log_text(scenario)[marker:])
    assert any(0 < width < 256 for width in closing), 'Closing drawer skipped its animation'
    assert all(b <= a + 1 for a, b in zip(closing, closing[1:])), 'Closing drawer reversed direction'
    collapsed_center = scenario.control_bounds('composer-surface')[0]
    assert abs(expanded_center - collapsed_center - 128) < 1
    marker = len(log_text(scenario))
    scenario.click_control('sidebar-toggle', settle=0.4)
    opening = widths(log_text(scenario)[marker:])
    assert any(0 < width < 256 for width in opening), 'Opening drawer skipped its animation'
    assert all(b + 1 >= a for a, b in zip(opening, opening[1:])), 'Opening drawer reversed direction'
    assert abs(opening[-1] - 256) < 0.1
    scenario.checks.append('drawer-renders-intermediate-frames-and-settles-without-mount-animation')

    scenario.click_control('sidebar-toggle', settle=0.06)
    scenario.click_control('sidebar-toggle', settle=0.4)
    assert abs(scenario.control_bounds('sidebar-drawer')[2] - 256) < 0.1
    assert abs(scenario.control_bounds('composer-surface')[0] - expanded_center) < 1
    scenario.checks.append('rapid-drawer-reversal-settles-in-the-requested-state')

    marker = len(log_text(scenario))
    before = scenario.prompt('hello')
    scenario.finished(before)
    progress = [float(value) for value in re.findall(
        r'surface="conversation" progress=([0-9.]+)', log_text(scenario)[marker:])]
    assert progress and all(value == 1 for value in progress), 'Streaming restarted the entry fade'
    scenario.checks.append('streaming-updates-do-not-restart-conversation-entry-animation')
    message_progress = [float(value) for value in re.findall(
        r'surface="user-message" progress=([0-9.]+)', log_text(scenario)[marker:])]
    assert any(0 < value < 1 for value in message_progress) and message_progress[-1] == 1
    assert all(b >= a for a, b in zip(message_progress, message_progress[1:])), 'User entry replayed during streaming'
    scenario.checks.append('new-user-message-animates-once-and-settles-during-streaming')

    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'native close before preference fixture')
    assert scenario.process.returncode == 0
    scenario.log.close()
    scenario.log = None
    # Only initial preference setup for the owned fixture. This is not a claim
    # that a native appearance editor exists, and no task/event data is changed.
    with sqlite3.connect(scenario.data / 'native-workspace.sqlite3') as db:
        settings = {'version': 1, 'appearance': {'reduced_motion': True}, 'keybindings': []}
        db.execute('INSERT OR REPLACE INTO preferences (key,data) VALUES (?,?)',
                   ('settings', json.dumps(settings)))
    scenario.launch()
    resize(ui, 1536, reference_height, scenario.scale)
    marker = len(log_text(scenario))
    scenario.click_control('sidebar-toggle', settle=0.4)
    assert abs(scenario.control_bounds('composer-surface')[0] - collapsed_center) < 1
    scenario.click_control('sidebar-toggle', settle=0.4)
    observed = widths(log_text(scenario)[marker:])
    assert observed and all(abs(width - 256) < 0.1 for width in observed)
    progress = [float(value) for value in re.findall(
        r'surface="conversation" progress=([0-9.]+)', log_text(scenario))]
    assert progress and all(value == 1 for value in progress)
    scenario.click_control('composer-extras', enabled=True)
    ui.key('Escape')
    for surface in ('session-menu', 'user-message'):
        frames = [float(value) for value in re.findall(
            r'surface="' + surface + r'" progress=([0-9.]+)', log_text(scenario))]
        assert frames and all(value == 1 for value in frames), surface + ' ignored reduced motion/history restoration'
    scenario.checks.append('persisted-reduced-motion-disables-drawer-and-pane-transitions')
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'native presentation shutdown')
    assert scenario.process.returncode == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--scale', type=float, choices=(1, 1.25), default=1,
                        help='Native X11 display scale; 1.25 captures the 1920 x 1032 reference size')
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks,
              'platform': 'Linux/X11/private Xvfb', 'reference_scale': scenario.scale,
              'content': 'Isolated visual catalog, matching the supplied sidebar row lengths and count'}
    try:
        run(scenario)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure')
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
