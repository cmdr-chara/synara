#!/usr/bin/env python3
"""Native searchable choices against the owned ACP fixture and durable store."""
import argparse
import json
from pathlib import Path
import re
import time

from native_smoke import Scenario, wait_until
from native_navigation_smoke import key_edge
from native_presentation_smoke import log_text
from native_controls_smoke import config_count, option, wait_applied_render, settled_control_count


def visible_count(scenario):
    counts = re.findall(r'surface="session-menu"[^\n]*?\bvisible=(\d+)', log_text(scenario))
    return int(counts[-1]) if counts else None


def search(scenario, query, expected):
    scenario.desktop.key('a', ('Control_L',))
    scenario.desktop.text(query)
    wait_until(lambda: visible_count(scenario) == expected, f'{expected} filtered choices')


def run(scenario):
    scenario.launch()
    ui = scenario.desktop
    initial_events = scenario.events()
    scenario.click_control('agent-picker')
    search(scenario, 'no-such-provider', 0)
    ui.key('Return')
    assert scenario.task()['agent_id'] == 'alpha'
    assert scenario.events() == initial_events
    ui.screenshot('picker-no-results')
    ui.key('Escape')
    wait_until(lambda: (visible_count(scenario) or 0) > 0, 'Escape clears query')
    scenario.checks.append('empty-results-enter-is-inert-and-escape-clears-search')

    search(scenario, 'BETA coding', 1)
    ui.screenshot('picker-filtered-provider')
    key_edge(ui, 'Return', True)
    try:
        assert scenario.task()['agent_id'] == 'alpha', 'Provider switched before release'
    finally:
        key_edge(ui, 'Return', False)
    wait_until(lambda: scenario.task()['agent_id'] == 'beta', 'filtered provider persisted')
    assert scenario.events() == initial_events, 'Filtering/selecting must not start an agent'
    scenario.checks.append('filtered-provider-keeps-original-action-index-and-release-semantics')

    before = scenario.prompt('hello')
    scenario.finished(before)
    wait_until(lambda: option(scenario, 'model') == 'beta', 'live model catalog')
    scenario.click_control('model-picker')
    search(scenario, 'alternate', 1)
    count = config_count(scenario)
    ui.key('space')
    assert config_count(scenario) == count, 'A search space must not activate a model'
    key_edge(ui, 'Return', True)
    try:
        search(scenario, 'no-match-after-press', 0)
    finally:
        key_edge(ui, 'Return', False)
    time.sleep(0.2)
    assert config_count(scenario) == count
    assert option(scenario, 'model') == 'beta'
    scenario.checks.append('search-space-and-query-changes-cannot-commit-an-armed-model')

    search(scenario, 'alternate', 1)
    applied = settled_control_count(scenario)
    ui.screenshot('picker-filtered-model')
    ui.key('Return')
    wait_until(lambda: option(scenario, 'model') == 'alternate', 'filtered model acknowledged')
    wait_applied_render(scenario, applied)
    assert config_count(scenario) == count + 1
    scenario.checks.append('filtered-model-applies-once-through-real-controller-and-events')

    scenario.click_control('composer-input')
    ui.text('Unsent draft survives model search')
    scenario.click_control('model-picker')
    search(scenario, 'unavailable-result', 0)
    ui.key('Escape')
    ui.key('Escape')
    scenario.click_control('composer-input')
    assert ui.copy_input() == 'Unsent draft survives model search'
    assert config_count(scenario) == count + 1
    scenario.checks.append('search-dismissal-preserves-composer-draft-and-session-state')
    ui.screenshot('picker-dismissed-draft')


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
