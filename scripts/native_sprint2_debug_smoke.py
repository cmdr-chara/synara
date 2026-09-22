#!/usr/bin/env python3
"""Real GPUI Debug lifecycle on an owned Xvfb display and isolated SQLite data.

Controls are driven through X11 input. SQL is read-only observation, never setup
or mutation of the feature under test. No live provider account is used.
"""
import argparse
import json
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_model_draft_smoke import preference, close
from native_integrations_smoke import click, fill
from native_project_import_smoke import task_events


def state(s, task):
    return preference(s, 'task-debug:' + task)


def phase(s, task):
    value = state(s, task)
    return ((value or {}).get('current') or {}).get('phase')


def open_debug(s):
    s.desktop.key('p', ('Control_L', 'Shift_L'))
    time.sleep(0.2)
    s.desktop.text('Debug workflow')
    time.sleep(0.2)
    s.desktop.key('Return')


def evidence(s, task, text):
    before = state(s, task)['revision']
    fill(s, 'debug-evidence', text)
    click(s, 'debug-add-evidence')
    wait_until(lambda: state(s, task)['revision'] > before, 'saved phase evidence')


def run(s):
    s.launch()
    task = selection(s)
    original = task_events(s, task)
    fill(s, 'composer-input', 'Keep my unsent request')
    open_debug(s)
    fill(s, 'debug-problem', 'A failing fixture needs a reproducible explanation')
    click(s, 'debug-start')
    wait_until(lambda: phase(s, task) == 'observation', 'persisted observation phase')
    assert task_events(s, task) == original and task_count(s) == 1
    assert preference(s, 'task-draft:' + task)['text'] == 'Keep my unsent request'
    s.checks.append('command-surface-starts-persisted-debug-without-execution-or-draft-mutation')
    s.desktop.screenshot('debug-observation', window_only=True)

    evidence(s, task, 'Actual output differs from the expected result')
    click(s, 'debug-advance')
    wait_until(lambda: phase(s, task) == 'reproduction', 'evidence-gated reproduction phase')
    assert len(state(s, task)['current']['evidence']) == 1
    assert task_events(s, task) == original
    s.checks.append('native-evidence-save-and-explicit-phase-transition-are-inert')

    click(s, 'debug-insert')
    wait_until(lambda: 'Phase: Reproduction' in preference(s, 'task-draft:' + task)['text'], 'visible instructions persisted in draft')
    draft = preference(s, 'task-draft:' + task)['text']
    assert draft.startswith('Keep my unsent request\n\n')
    assert 'Do not invent a successful reproduction' in draft
    assert 'do not grant additional permissions' in draft
    assert task_events(s, task) == original
    click(s, 'debug-close')
    s.click_control('composer-input')
    assert s.desktop.copy_input() == draft
    s.checks.append('stage-instructions-append-to-editable-draft-without-sending-or-permission-change')

    saved = state(s, task)
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == task and state(s, task) == saved
    assert task_events(s, task) == original
    assert preference(s, 'task-draft:' + task)['text'] == draft
    click(s, 'debug-status')
    s.desktop.screenshot('debug-restarted', window_only=True)
    s.checks.append('restart-restores-phase-evidence-and-draft-without-provider-autostart')

    click(s, 'debug-pause')
    wait_until(lambda: state(s, task)['current']['paused'], 'paused debugging')
    assert task_events(s, task) == original
    click(s, 'debug-pause')
    wait_until(lambda: not state(s, task)['current']['paused'], 'resumed debugging')
    s.checks.append('pause-and-resume-change-only-local-workflow-state')

    for expected, next_phase in [('reproduction', 'investigation'), ('investigation', 'fix'), ('fix', 'verification'), ('verification', 'complete')]:
        assert phase(s, task) == expected
        evidence(s, task, 'Recorded ' + expected + ' evidence from a controlled check')
        click(s, 'debug-advance')
        wait_until(lambda: phase(s, task) == next_phase, 'advanced to ' + next_phase)
    assert len(state(s, task)['current']['evidence']) == 5
    assert task_events(s, task) == original
    s.desktop.screenshot('debug-completed', window_only=True)
    s.checks.append('full-native-observe-reproduce-investigate-fix-verify-lifecycle')

    click(s, 'debug-reopen')
    wait_until(lambda: phase(s, task) == 'investigation', 'reopened investigation')
    assert state(s, task)['current']['visit'] == 6
    assert not any(e['visit'] == 6 for e in state(s, task)['current']['evidence'])
    s.checks.append('reopened-investigation-needs-fresh-evidence-not-an-old-verification')
    click(s, 'debug-clear')
    assert state(s, task)['current'] is not None
    click(s, 'debug-confirm-clear')
    wait_until(lambda: state(s, task)['current'] is None, 'explicitly closed run')
    assert len(state(s, task)['history']) == 1
    assert state(s, task)['history'][0]['evidence_count'] == 5
    assert task_events(s, task) == original
    click(s, 'debug-close')
    s.checks.append('closing-a-run-requires-confirmation-and-retains-an-honest-summary')

    click(s, 'new-thread')
    wait_until(lambda: task_count(s) == 2 and selection(s) != task, 'independent task')
    other = selection(s)
    assert state(s, other) is None
    assert state(s, task)['history'][0]['evidence_count'] == 5
    assert task_events(s, task) == original
    s.checks.append('debug-state-is-task-scoped-with-no-ownership-or-session-duplication')


def main():
    parser = argparse.ArgumentParser()
    for name in ('binary', 'fixture', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb'}
    try:
        run(s)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if s.process and s.process.poll() is None:
            s.desktop.screenshot('failure')
        raise
    finally:
        s.close()
        (s.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
