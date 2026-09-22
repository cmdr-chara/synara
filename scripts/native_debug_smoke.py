#!/usr/bin/env python3
"""Evidence-first Debug controls on real GPUI/X11, never fixture SQL writes."""
import argparse
import json
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection
from native_model_draft_smoke import preference, close
from native_project_import_smoke import task_events
from native_integrations_smoke import fill, click


def run(s):
    s.launch()
    task = selection(s)
    original = task_events(s, task)
    key = 'task-debug:' + task
    s.click_control('debug-open', enabled=True)
    click(s, 'debug-enabled')
    wait_until(lambda: (preference(s, key) or {}).get('enabled'), 'Debug enabled without agent start')
    assert task_events(s, task) == original
    click(s, 'debug-next')
    time.sleep(.25)
    assert preference(s, key)['phase'] == 'observe' and not preference(s, key)['completed']
    s.checks.append('Debug-is-app-owned-and-cannot-advance-without-evidence')
    fill(s, 'composer-input', 'Preserve this request')
    click(s, 'debug-prepare')
    s.click_control('composer-input')
    draft = s.desktop.copy_input()
    assert 'Current phase: Observe' in draft and task in draft and 'Preserve this request' in draft
    assert 'Do not modify files yet' in draft and 'existing task' in draft
    wait_until(lambda: (preference(s, 'task-draft:' + task) or {}).get('text') == draft, 'prepared draft persisted')
    assert task_events(s, task) == original
    s.checks.append('prepare-step-is-visible-editable-unsent-and-preserves-request-and-authority')
    for index, phase in enumerate(['observe', 'reproduce', 'investigate', 'fix', 'verify']):
        evidence = phase + ' evidence recorded through native input'
        fill(s, 'debug-evidence-input', evidence)
        click(s, 'debug-save-evidence')
        wait_until(lambda: (preference(s, key) or {}).get('evidence', {}).get(phase) == evidence, phase + ' evidence saved')
        click(s, 'debug-next')
        if phase == 'verify':
            wait_until(lambda: preference(s, key)['completed'], 'explicit evidence-backed verification')
        else:
            following = ['observe', 'reproduce', 'investigate', 'fix', 'verify'][index + 1]
            wait_until(lambda: preference(s, key)['phase'] == following, 'phase advanced to ' + following)
    assert len(preference(s, key)['evidence']) == 5
    assert task_events(s, task) == original
    s.desktop.screenshot('debug-verified-evidence', window_only=True)
    s.checks.append('all-five-evidence-phases-and-explicit-verification-use-real-controls')
    click(s, 'debug-enabled')
    wait_until(lambda: not preference(s, key)['enabled'], 'Debug paused')
    saved = preference(s, key)
    close(s)
    s.launch(preserve_selection=True)
    assert preference(s, key) == saved and task_events(s, task) == original
    s.click_control('composer-input')
    assert s.desktop.copy_input() == draft
    s.click_control('debug-open', enabled=True)
    s.desktop.screenshot('debug-restart-paused', window_only=True)
    s.checks.append('mode-evidence-completion-and-draft-survive-restart-without-execution')
    click(s, 'debug-close')
    click(s, 'new-thread')
    wait_until(lambda: selection(s) != task, 'independent task selected')
    assert preference(s, 'task-debug:' + selection(s)) is None
    assert task_events(s, selection(s))[1:] == ([], 0)
    assert preference(s, key) == saved
    s.checks.append('Debug-state-is-task-isolated-and-never-creates-a-provider-session')
    close(s)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'platform': 'Linux/X11/private Xvfb'}
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
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
