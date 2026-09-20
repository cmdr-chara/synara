#!/usr/bin/env python3
"""Native Environment tabs, split controls, saved layout and no-autostart checks.

All input, SQLite state, files and child processes belong to Scenario's private
Xvfb profile. No user workspace, external service or authenticated agent is used.
"""
import argparse
import json
import os
import shlex
from pathlib import Path
import sqlite3
import time

from native_smoke import Scenario, wait_until
from native_presentation_smoke import resize
from native_navigation_smoke import selection, event_cursor


def preference(s, key='environment-layout'):
    with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
        row = db.execute('SELECT data FROM preferences WHERE key=?', (key,)).fetchone()
    return json.loads(row[0]) if row else None


def layout_saved(s, **values):
    value = preference(s)
    return value if value and all(value.get(key) == expected for key, expected in values.items()) else None


def close(s):
    s.desktop.request_close()
    wait_until(lambda: s.process.poll() is not None, 'layout and draft-safe shutdown')
    assert s.process.returncode == 0
    s.log.close()
    s.log = None


def choose_tool(s, label):
    s.click_control('environment-add')
    s.desktop.text(label)
    s.desktop.key('Return')
    assert s.process.poll() is None, 'Opening an Environment menu crashed the application'


def reveal_setting(s, control):
    ui = s.desktop
    for _ in range(24):
        bounds = s.control_bounds(control)
        left, top, width, height = ui.geometry()
        viewport_height = height / s.scale
        if bounds and 48 <= bounds[1] and bounds[1] + bounds[3] < viewport_height:
            return
        content = wait_until(lambda: s.control_bounds('settings-content'), 'Settings content geometry')
        x = round(left + (content[0] + content[2] / 2) * s.scale)
        y = round(top + height / 2)
        ui.xt.XTestFakeMotionEvent(ui.display, -1, x, y, 0)
        direction = 4 if bounds and bounds[1] < 48 else 5
        for _ in range(3):
            ui.xt.XTestFakeButtonEvent(ui.display, direction, True, 0)
            ui.xt.XTestFakeButtonEvent(ui.display, direction, False, 0)
        ui.x.XFlush(ui.display)
        time.sleep(0.15)
    raise AssertionError(f'{control} did not become visible in the native Settings scroll area')


def drag_divider(s, delta):
    ui = s.desktop
    x, y, width, height = s.control_bounds('environment-divider')
    origin_x, origin_y, _, _ = ui.geometry()
    x = round(origin_x + (x + width / 2) * s.scale)
    y = round(origin_y + (y + min(height / 2, 200)) * s.scale)
    ui.xt.XTestFakeMotionEvent(ui.display, -1, x, y, 0)
    ui.xt.XTestFakeButtonEvent(ui.display, 1, True, 0)
    ui.x.XFlush(ui.display)
    time.sleep(0.15)
    for step in range(1, 7):
        ui.xt.XTestFakeMotionEvent(ui.display, -1, round(x + delta * s.scale * step / 6), y, 0)
        ui.x.XFlush(ui.display)
        time.sleep(0.04)
    ui.xt.XTestFakeButtonEvent(ui.display, 1, False, 0)
    ui.x.XFlush(ui.display)
    time.sleep(0.3)


def run(s):
    # A canary launcher proves both no-autostart and the explicit start path.
    shell_starts = s.output / 'shell-starts.txt'
    shell = s.output / 'owned-shell'
    shell.write_text('#!/bin/sh\nprintf "%s\\n" "$$" >> ' + shlex.quote(str(shell_starts)) + '\nexec /bin/sh\n')
    shell.chmod(0o700)
    os.environ['SHELL'] = str(shell)
    s.launch()
    ui = s.desktop
    original = selection(s)
    baseline = preference(s, 'settings')
    resize(ui, 1280, 803, s.scale)
    s.click_control('composer-input')
    ui.text('Keep this unsent chat while arranging tools.')
    wait_until(lambda: preference(s, 'task-draft:' + original), 'durable chat draft')
    assert event_cursor(s, original) == 0
    s.click_control('Files', settle=0.45)
    wait_until(lambda: layout_saved(s, open_by_default=True), 'explicit open preference')
    assert abs(s.control_bounds('workspace-pane')[2] - 512) < 2
    ui.screenshot('environment-launcher', window_only=True)

    choose_tool(s, 'Browser')
    assert (preference(s)['tabs'], preference(s)['active']) == ([], None)
    assert event_cursor(s, original) == 0
    ui.screenshot('environment-unsupported-browser', window_only=True)
    ui.key('Escape')  # Clear the menu search, then dismiss.
    ui.key('Escape')
    choose_tool(s, 'Explorer')
    wait_until(lambda: layout_saved(s, active='explorer'), 'Explorer selection')
    wait_until(lambda: s.control_bounds('file-tree'), 'real file explorer')
    choose_tool(s, 'Terminal')
    wait_until(lambda: layout_saved(s, active='terminal'), 'Terminal selection')
    assert not any(e.get('type') == 'prompt_started' for e in s.events())
    # The real terminal view exists, but opening/restoring it must not start a shell.
    assert not shell_starts.exists(), 'Selecting a Terminal tab started a shell'
    s.click_control('start-shell')
    wait_until(shell_starts.exists, 'explicit shell start canary')
    pid = int(shell_starts.read_text().strip())
    assert Path(f'/proc/{pid}').exists()
    choose_tool(s, 'Changes')
    wait_until(lambda: layout_saved(s, active='changes'), 'Changes selection')
    assert preference(s)['tabs'] == ['explorer', 'terminal', 'changes']
    s.click_control('environment-tab', slot=1)
    ui.key('Right')
    wait_until(lambda: layout_saved(s, active='terminal'), 'keyboard tab selection')
    ui.key('Left')
    wait_until(lambda: layout_saved(s, active='explorer'), 'keyboard previous tab')
    s.click_control('composer-input')
    assert ui.copy_input() == 'Keep this unsent chat while arranging tools.'
    ui.focus()
    assert selection(s) == original and event_cursor(s, original) == 0
    ui.screenshot('environment-tabs-explorer', window_only=True)
    s.checks.append('native-tools-tab-selection-keyboard-navigation-and-no-implicit-agent-launch')

    drag_divider(s, -100)
    value = wait_until(lambda: preference(s) if preference(s)['width_ratio'] > 0.56 else None, 'dragged split saved')
    assert 0.56 < value['width_ratio'] < 0.65
    assert s.control_bounds('workspace-pane')[2] > 560
    s.click_control('environment-divider')
    ui.key('Home')
    wait_until(lambda: layout_saved(s, width_ratio=0.5), 'keyboard equal split')
    ui.key('Left')
    value = wait_until(lambda: preference(s) if preference(s)['width_ratio'] > 0.52 else None, 'keyboard resize saved')
    assert 0.52 < value['width_ratio'] < 0.54
    for width, height in [(1100, 760), (960, 700)]:
        resize(ui, width, height, s.scale)
        x, _, dock_width, _ = s.control_bounds('workspace-pane')
        left, _, chat_width, _ = s.control_bounds('chat-pane')
        assert dock_width >= 319 and chat_width >= 319
        assert abs(x + dock_width - width) < 2 and abs(left + chat_width - x) < 2
        assert preference(s)['width_ratio'] == value['width_ratio'], 'Window resize overwrote the desired split'
        ui.screenshot(f'environment-split-{width}', window_only=True)
    resize(ui, 1280, 803, s.scale)
    s.click_control('environment-maximize')
    wait_until(lambda: s.control_bounds('workspace-pane')[2] > 1000, 'maximized workspace')
    ui.screenshot('environment-maximized', window_only=True)
    s.click_control('environment-maximize')
    wait_until(lambda: abs(s.control_bounds('workspace-pane')[2] - 1024 * value['width_ratio']) < 2, 'restored split')
    s.checks.append('pointer-keyboard-resize-minimum-widths-and-maximize-restore-preserve-layout')

    s.click_control('environment-close')
    wait_until(lambda: layout_saved(s, open_by_default=False), 'explicit hide preference')
    s.click_control('composer-input')
    assert ui.copy_input() == 'Keep this unsent chat while arranging tools.'
    ui.focus()
    assert preference(s)['tabs'] == ['explorer', 'terminal', 'changes']
    assert Path(f'/proc/{pid}').exists(), 'Hiding Environment stopped the owned shell'
    s.click_control('Files', settle=0.45)
    wait_until(lambda: layout_saved(s, open_by_default=True, active='explorer'), 'reopen selected tool')
    ui.key('6', ('Control_L',))
    reveal_setting(s, 'environment-default')
    s.click_control('environment-default')
    wait_until(lambda: layout_saved(s, open_by_default=False), 'General setting off')
    reveal_setting(s, 'environment-default')
    s.click_control('environment-default')
    wait_until(lambda: layout_saved(s, open_by_default=True), 'General setting on')
    ui.screenshot('environment-general-preference', window_only=True)
    assert preference(s, 'settings') == baseline, 'Environment setting overwrote unrelated preferences'
    close(s)
    wait_until(lambda: not Path(f'/proc/{pid}').exists(), 'owned shell cleanup on application close')
    s.launch(preserve_selection=True)
    assert shell_starts.read_text().splitlines() == [str(pid)], 'Restart silently started a saved terminal'
    wait_until(lambda: s.control_bounds('file-tree'), 'restored active Explorer')
    assert preference(s)['width_ratio'] == value['width_ratio']
    assert preference(s)['tabs'] == ['explorer', 'terminal', 'changes']
    assert selection(s) == original and event_cursor(s, original) == 0
    s.click_control('composer-input')
    assert ui.copy_input() == 'Keep this unsent chat while arranging tools.'
    ui.focus()
    ui.screenshot('environment-restored', window_only=True)
    s.checks.append('explicit-hide-general-preference-and-restart-preserve-tabs-ratio-and-chat-draft')

    # A storage fault must retain the window and last good preference, not be
    # hidden by retrying endlessly or bypassed by synthetic success in the UI.
    previous = preference(s)
    with sqlite3.connect(s.data / 'native-workspace.sqlite3') as db:
        db.execute("CREATE TRIGGER reject_environment BEFORE UPDATE ON preferences WHEN NEW.key='environment-layout' BEGIN SELECT RAISE(FAIL, 'owned environment save fault'); END")
    s.click_control('environment-divider')
    ui.key('Home')
    time.sleep(0.8)
    ui.request_close()
    time.sleep(0.8)
    assert s.process.poll() is None, 'The app quit after a failed layout save'
    assert preference(s) == previous, 'Failed layout save replaced good data'
    ui.screenshot('environment-save-error', window_only=True)
    with sqlite3.connect(s.data / 'native-workspace.sqlite3') as db:
        db.execute('DROP TRIGGER reject_environment')
    close(s)
    assert preference(s)['width_ratio'] == 0.5
    s.checks.append('layout-save-failure-keeps-window-open-and-explicit-close-retries-without-data-loss')

    # Recovery is deliberate: a future layout is never overwritten merely by
    # startup or navigating a native tool. Reset is a separate user action.
    unknown = {'version': 999, 'retained': 'future-layout'}
    with sqlite3.connect(s.data / 'native-workspace.sqlite3') as db:
        db.execute('UPDATE preferences SET data=? WHERE key=?', (json.dumps(unknown), 'environment-layout'))
    s.launch(preserve_selection=True)
    s.click_control('Files', settle=0.45)
    time.sleep(0.7)
    assert preference(s) == unknown
    ui.key('6', ('Control_L',))
    reveal_setting(s, 'environment-reset')
    ui.screenshot('environment-layout-recovery', window_only=True)
    s.click_control('environment-reset')
    wait_until(lambda: layout_saved(s, version=1, width_ratio=0.5, tabs=[], active=None, open_by_default=False), 'explicit reset from future layout')
    assert event_cursor(s, original) == 0 and preference(s, 'settings') == baseline
    close(s)
    s.checks.append('unknown-layout-is-preserved-until-explicit-reset-with-no-tool-autostart')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = dict(status='failed', checks=s.checks, platform='Linux/X11/private Xvfb', agents='none started')
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
