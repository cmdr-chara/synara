#!/usr/bin/env python3
"""Exercise native chat creation, Studio, and settings on owned fixture data."""
import argparse
import json
import sqlite3
import time
from pathlib import Path

from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count, event_cursor, task_events, prompt_finished
from native_presentation_smoke import resize


def tasks(scenario):
    path = scenario.data / 'native-workspace.sqlite3'
    with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True) as db:
        return {row[0]: json.loads(row[1]) for row in db.execute('SELECT id,data FROM tasks')}


def settings(scenario):
    path = scenario.data / 'native-workspace.sqlite3'
    with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True) as db:
        row = db.execute("SELECT data FROM preferences WHERE key='settings'").fetchone()
        return json.loads(row[0]) if row else {}


def mode(scenario, studio):
    scenario.click_control('workspace-tools')
    scenario.click_control('mode-choice', slot=int(studio))


def prompt(scenario, text):
    task = selection(scenario)
    cursor = event_cursor(scenario, task)
    scenario.prompt(text)
    wait_until(lambda: prompt_finished(scenario, task, cursor), 'selected chat completed')
    time.sleep(0.35)
    return task


def reveal(scenario, control):
    ui = scenario.desktop
    for _ in range(18):
        bounds = scenario.control_bounds(control)
        if bounds and 48 <= bounds[1] * scenario.scale and (bounds[1] + bounds[3]) * scenario.scale <= ui.geometry()[3] - 12:
            return
        left, top, width, height = ui.geometry()
        ui.xt.XTestFakeMotionEvent(ui.display, -1, left + int(width * 0.75), top + int(height * 0.6), 0)
        button = 4 if bounds and bounds[1] < 48 else 5
        for _ in range(2):
            ui.xt.XTestFakeButtonEvent(ui.display, button, 1, 0)
            ui.xt.XTestFakeButtonEvent(ui.display, button, 0, 0)
        ui.x.XFlush(ui.display)
        time.sleep(0.18)
    raise AssertionError(f'Could not reveal {control}')


def fill(scenario, control, value):
    reveal(scenario, control)
    scenario.click_control(control)
    scenario.desktop.key('a', ('Control_L',))
    scenario.desktop.text(value)
    wait_until(lambda: scenario.desktop.copy_input() == value, f'exact native input for {control}')
    scenario.desktop.focus()


def run(scenario):
    scenario.launch()
    ui = scenario.desktop
    resize(ui, 1536, 826, scenario.scale)
    original = selection(scenario)
    ui.key('6', ('Control_L',))
    scenario.click_control('appearance')
    scenario.click_control('theme-dark')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('theme') == 'dark', 'reference dark theme')
    scenario.click_control('dark-theme')
    ui.key('End')
    ui.key('Return')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('dark_theme') == 'dracula', 'reference Dracula palette')
    scenario.click_control('settings-back')
    scenario.click_control('new-thread')
    wait_until(lambda: task_count(scenario) == 2 and selection(scenario) != original, 'standalone draft')
    standalone = selection(scenario)
    for _ in range(3):
        scenario.click_control('new-thread', settle=0.1)
    assert task_count(scenario) == 2 and selection(scenario) == standalone, 'Repeated New thread created empty duplicates'
    assert tasks(scenario)[standalone]['scope'] == 'chat'
    ui.screenshot('new-thread-single-draft', window_only=True)
    scenario.checks.append('repeated-new-thread-keeps-one-selected-empty-draft')
    prompt(scenario, 'hello')
    assert task_events(scenario, standalone, 0)[0] == {'type': 'title_changed', 'title': 'hello'}
    assert tasks(scenario)[standalone]['title'] == 'Fixture task', 'Agent-provided title should remain authoritative'
    assert event_cursor(scenario, original) == 0
    scenario.checks.append('new-chat-sends-to-independent-thread-and-gets-prompt-title')
    scenario.click_control('new-thread')
    wait_until(lambda: task_count(scenario) == 3, 'second standalone draft')
    draft = selection(scenario)
    scenario.click_control('composer-input')
    ui.text('unsent chat draft')
    mode(scenario, True)
    wait_until(lambda: task_count(scenario) == 4 and tasks(scenario)[selection(scenario)]['scope'] == 'studio', 'Studio draft')
    studio = selection(scenario)
    for _ in range(3):
        scenario.click_control('new-thread', settle=0.1)
    assert task_count(scenario) == 4 and selection(scenario) == studio
    all_tasks = tasks(scenario)
    assert all_tasks[studio]['working_directory'] != all_tasks[standalone]['working_directory']
    assert Path(all_tasks[studio]['working_directory']).is_relative_to(scenario.data / 'chats')
    ui.screenshot('studio', window_only=True)
    scenario.click_control('workspace-tools')
    ui.screenshot('studio-switcher', window_only=True)
    ui.key('Escape')
    standalone_cursor = event_cursor(scenario, standalone)
    prompt(scenario, 'hello')
    assert event_cursor(scenario, standalone) == standalone_cursor
    mode(scenario, False)
    wait_until(lambda: selection(scenario) == draft, 'restore Synara draft')
    scenario.click_control('composer-input')
    assert ui.copy_input() == 'unsent chat draft'
    mode(scenario, True)
    wait_until(lambda: selection(scenario) == studio, 'restore Studio conversation')
    assert task_count(scenario) == 4
    scenario.checks.append('Studio-has-independent-working-directory-conversation-and-restored-drafts')

    ui.focus()
    ui.key('6', ('Control_L',))
    scenario.click_control('general')
    wait_until(lambda: scenario.control_bounds('settings-content'), 'settings page')
    x, y, width, height = scenario.control_bounds('settings-content')
    assert abs(width - 624) < 1 and abs(x + width / 2 - 896) < 1
    ui.screenshot('settings-general', window_only=True)
    scenario.click_control('project-order')
    ui.key('Down')
    ui.key('Return')
    wait_until(lambda: settings(scenario).get('general', {}).get('alphabetical_projects'), 'saved project order')
    scenario.click_control('appearance')
    scenario.click_control('theme-dark')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('theme') == 'dark', 'saved dark theme')
    scenario.click_control('dark-theme')
    ui.key('End')
    ui.key('Return')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('dark_theme') == 'dracula', 'saved Dracula theme')
    ui.screenshot('settings-appearance-dracula', window_only=True)
    scenario.click_control('theme-light')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('theme') == 'light', 'saved light theme')
    ui.screenshot('settings-appearance-light', window_only=True)
    scenario.click_control('theme-dark')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('theme') == 'dark', 'restore dark theme')
    fill(scenario, 'ui-font', 'Liberation Sans')
    fill(scenario, 'code-font', 'DejaVu Sans Mono')
    reveal(scenario, 'save-fonts')
    scenario.click_control('save-fonts')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('fonts', {}).get('ui_family') == 'Liberation Sans', 'saved fonts')
    reveal(scenario, 'reduce-motion')
    scenario.click_control('reduce-motion')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('reduced_motion'), 'saved reduced motion')
    scenario.click_control('appearance')
    scenario.click_control('settings-restore')
    wait_until(lambda: settings(scenario).get('appearance', {}).get('theme') == 'system', 'restore appearance defaults')
    reveal(scenario, 'ui-font')
    scenario.click_control('ui-font')
    # Copying an empty selection keeps the old system clipboard. Append a
    # sentinel without selecting first so this checks the actual editor value.
    ui.text('x')
    assert ui.copy_input() == 'x', 'Restore defaults retained the old font editor value'
    ui.focus()
    ui.key('BackSpace')
    assert settings(scenario)['appearance']['fonts']['ui_family'] is None
    assert settings(scenario)['appearance']['reduced_motion'] is False
    scenario.click_control('appearance')
    scenario.click_control('theme-dark')
    wait_until(lambda: settings(scenario)['appearance']['theme'] == 'dark', 'restore reference dark theme')
    scenario.click_control('dark-theme')
    ui.key('End')
    ui.key('Return')
    wait_until(lambda: settings(scenario)['appearance']['dark_theme'] == 'dracula', 'restore reference palette')
    scenario.checks.append('restore-defaults-updates-saved-preferences-and-font-editors')
    scenario.click_control('profile')
    fill(scenario, 'profile-name', 'Native tester')
    fill(scenario, 'profile-username', 'native-test')
    reveal(scenario, 'save-profile')
    scenario.click_control('save-profile')
    wait_until(lambda: settings(scenario).get('profile', {}).get('name') == 'Native tester', 'saved local profile')
    scenario.click_control('general')
    reveal(scenario, 'show-studio')
    scenario.click_control('show-studio')
    wait_until(lambda: settings(scenario).get('general', {}).get('show_studio') is False, 'hide Studio')
    assert tasks(scenario)[studio]['scope'] == 'studio', 'Hiding Studio must not delete its chats'
    scenario.click_control('show-studio')
    wait_until(lambda: settings(scenario).get('general', {}).get('show_studio'), 'show Studio again')
    scenario.click_control('settings-back')
    mode(scenario, True)
    wait_until(lambda: selection(scenario) == studio, 'hidden Studio chat remains selectable')
    ui.key('6', ('Control_L',))
    scenario.click_control('profile')
    ui.screenshot('settings-profile', window_only=True)
    scenario.checks.append('font-motion-profile-and-Studio-visibility-settings-save-and-preserve-chats')
    scenario.click_control('providers')
    ui.screenshot('settings-agents', window_only=True)
    scenario.click_control('settings-back')
    assert selection(scenario) == studio and task_count(scenario) == 4
    ui.screenshot('studio-chat', window_only=True)
    scenario.checks.append('settings-navigation-real-theme-and-order-controls-persist-without-changing-chat')
    saved = settings(scenario)
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'native close')
    assert scenario.process.returncode == 0
    scenario.log.close()
    scenario.log = None
    scenario.launch(preserve_selection=True)
    assert selection(scenario) == studio and settings(scenario) == saved
    assert tasks(scenario)[studio]['scope'] == 'studio'
    ui.screenshot('studio-restored', window_only=True)
    scenario.checks.append('Studio-selection-and-settings-survive-restart')
    # Studio section's compose control: stable sidebar geometry, verified in
    # studio-restored.png immediately above (logical coordinates).
    ui.click_client(round(235 * scenario.scale), round(142 * scenario.scale))
    ui.text('Named studio')
    ui.key('Return')
    wait_until(lambda: task_count(scenario) == 5 and selection(scenario) != studio, 'named Studio draft')
    named = tasks(scenario)[selection(scenario)]
    assert named['title'] == 'Named studio' and named['scope'] == 'studio'
    scenario.checks.append('named-draft-Enter-uses-the-visible-Studio-scope')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--scale', type=float, default=1)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'platform': 'Linux/X11/private Xvfb'}
    try:
        run(scenario)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure', window_only=True)
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
