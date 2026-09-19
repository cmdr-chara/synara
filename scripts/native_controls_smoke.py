#!/usr/bin/env python3
"""Real native session pickers, release activation and composer/backend regression."""
import argparse
import ctypes as C
import json
from pathlib import Path
import time

from native_smoke import Scenario, wait_until
from native_navigation_smoke import key_edge


def configuration(scenario):
    events = [event['configuration'] for event in scenario.events()
              if event['type'] == 'configuration_changed']
    return events[-1] if events else None


def option(scenario, key):
    config = configuration(scenario)
    if not config:
        return None
    return next((entry['current']['value'] for entry in config['options'] if entry['id'] == key), None)


def config_count(scenario):
    return sum(event['type'] == 'configuration_changed' for event in scenario.events())


def run(scenario):
    scenario.launch()
    ui = scenario.desktop
    ui.screenshot('composer-welcome')
    initial = scenario.events()
    scenario.click_control('agent-picker')
    ui.screenshot('agent-picker')
    ui.key('Escape')
    # Escape returns to the original trigger. Native Return reopens the menu.
    ui.key('Return')
    ui.key('End')
    key_edge(ui, 'Return', True)
    assert scenario.task()['agent_id'] == 'alpha', 'Provider changed before key release'
    key_edge(ui, 'Return', False)
    wait_until(lambda: scenario.task()['agent_id'] == 'beta', 'persisted agent selection')
    assert scenario.events() == initial, 'Selecting a profile must not start its process/session'
    scenario.checks.append('agent-menu-release-activation-and-escape-focus-without-autostart')

    before = scenario.prompt('hello')
    scenario.finished(before)
    assert scenario.has_text('Hello from beta', before)
    wait_until(lambda: option(scenario, 'model') == 'beta', 'advertised configuration')
    scenario.checks.append('new-composer-submits-through-real-selected-agent')
    ui.screenshot('conversation-controls')

    scenario.click_control('model-picker')
    ui.screenshot('model-picker')
    ui.key('End')
    count = config_count(scenario)
    key_edge(ui, 'Return', True)
    assert config_count(scenario) == count and option(scenario, 'model') == 'beta'
    key_edge(ui, 'Return', False)
    wait_until(lambda: option(scenario, 'model') == 'alternate', 'durable acknowledged model')
    assert config_count(scenario) == count + 1, 'One activation must dispatch once'
    scenario.click_control('model-picker')
    ui.key('Return')
    time.sleep(0.4)
    assert config_count(scenario) == count + 1, 'Selecting the applied value should be a no-op'
    scenario.checks.append('model-choices-are-explicit-acknowledged-once-and-noop-when-selected')

    scenario.click_control('mode-picker')
    ui.key('End')
    ui.key('Return')
    wait_until(lambda: configuration(scenario)['current_mode'] == 'plan', 'durable session mode')
    scenario.checks.append('mode-picker-uses-the-controller-and-durable-events')

    scenario.click_control('options-picker')
    ui.screenshot('boolean-options')
    ui.key('End')
    ui.key('Return')
    wait_until(lambda: option(scenario, 'review') is False, 'explicit boolean false')
    scenario.checks.append('boolean-picker-sends-an-explicit-advertised-value')

    _, _, width, height = ui.geometry()
    ui.click(width // 2, height - 150)
    ui.text('draft stays intact')
    assert ui.copy_input() == 'draft stays intact'
    before = scenario.events()
    scenario.click_control('model-picker')
    ui.key('Escape')
    ui.key('Return')
    ui.click_client(width // 2, 150)
    ui.click(width // 2, height - 150)
    assert ui.copy_input() == 'draft stays intact'
    assert scenario.events() == before, 'Dismissal must not submit or mutate session configuration'
    scenario.checks.append('escape-and-clickaway-preserve-draft-without-submission')
    ui.key('BackSpace')

    before = scenario.prompt('hold')
    wait_until(lambda: scenario.has_text('Started waiting', before), 'in-flight prompt')
    count = config_count(scenario)
    scenario.click_control('model-picker')
    time.sleep(0.2)
    assert config_count(scenario) == count
    ui.screenshot('working-stop-state')
    scenario.click_control('composer-submit')
    scenario.finished(before)
    assert any(event['type'] == 'cancellation_requested' for event in scenario.events()[before:])
    assert config_count(scenario) == count
    scenario.checks.append('busy-selectors-do-not-mutate-and-stop-cancels-the-real-request')

    resize = ui.x.XResizeWindow
    resize.argtypes = [C.c_void_p, C.c_ulong, C.c_uint, C.c_uint]
    resize.restype = C.c_int
    for width, height in [(1280, 803), (1100, 760), (960, 700), (1420, 930)]:
        resize(ui.display, ui.window, width, height)
        ui.x.XFlush(ui.display)
        wait_until(lambda: ui.geometry()[2:] == (width, height), 'native resize')
        time.sleep(0.3)
        scenario.click_control('agent-picker')
        assert scenario.process.poll() is None
        ui.screenshot(f'controls-{width}')
        ui.key('Escape')
    scenario.checks.append('responsive-composer-and-anchored-menus-at-four-native-window-sizes')
    old_events = scenario.events()
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'normal native shutdown')
    assert scenario.process.returncode == 0
    scenario.log.close()
    scenario.log = None
    scenario.launch()
    assert scenario.task()['agent_id'] == 'beta'
    assert scenario.events() == old_events
    ui.screenshot('restored-controls')
    scenario.checks.append('restart-restores-agent-and-transcript-without-autostart')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'agents': ['fixture-alpha', 'fixture-beta']}
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
