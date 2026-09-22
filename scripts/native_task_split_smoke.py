#!/usr/bin/env python3
"""Two independent tasks through real native composers and the existing ACP owner."""
import argparse
import json
import re
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count, event_cursor, task_events, prompt_finished
from native_model_draft_smoke import preference, close
from native_integrations_smoke import fresh_probe
from native_presentation_smoke import resize


def text(s, control, value):
    s.click_control(control)
    s.desktop.key('a', ('Control_L',))
    s.desktop.text(value)


def draft(s, task):
    return (preference(s, 'task-draft:' + task) or {}).get('text', '')


def run(s):
    s.launch()
    secondary = selection(s)
    text(s, 'primary-composer-input', 'draft from project one')
    wait_until(lambda: draft(s, secondary) == 'draft from project one', 'first durable draft')
    close(s)
    # Opening another ordinary local workspace via the application's public CLI
    # proves cross-project selection without editing SQLite or internal state.
    s.project = s.output / 'another-project'
    s.project.mkdir()
    s.launch()
    primary = wait_until(lambda: selection(s) != secondary and selection(s), 'second project selected')
    assert task_count(s) == 2
    text(s, 'primary-composer-input', 'draft from project two')
    wait_until(lambda: draft(s, primary) == 'draft from project two', 'second durable draft')
    before = s.events()
    s.click_control('task-split-open')
    s.click_control('task-split-choice', slot=0)
    s.click_control('task-split-composer')
    assert s.desktop.copy_input() == 'draft from project one'
    s.click_control('primary-composer-input')
    assert s.desktop.copy_input() == 'draft from project two', 'Primary composer must retain the second project draft'
    assert selection(s) == primary and s.events() == before
    left = s.control_bounds('task-split-primary')
    right = s.control_bounds('task-split-secondary')
    assert left[2] > 300 and right[2] > 300 and right[0] >= left[0] + left[2] - 1
    s.desktop.screenshot('two-projects-independent-drafts', window_only=True)
    s.checks.append('two-existing-cross-project-tasks-visible-with-independent-restored-drafts-and-no-send')

    secondary_cursor = event_cursor(s, secondary)
    primary_before = task_events(s, primary, 0)
    text(s, 'task-split-composer', 'hold')
    s.desktop.key('Return')
    wait_until(lambda: any(e.get('text') == 'Started waiting' for e in task_events(s, secondary, secondary_cursor)), 'secondary stream')
    assert task_events(s, primary, 0) == primary_before and draft(s, primary) == 'draft from project two'
    primary_cursor = event_cursor(s, primary)
    text(s, 'primary-composer-input', 'hello')
    s.desktop.key('Return')
    wait_until(lambda: prompt_finished(s, primary, primary_cursor), 'primary completion during independent secondary stream')
    assert not prompt_finished(s, secondary, secondary_cursor)
    s.click_control('task-split-send')
    wait_until(lambda: prompt_finished(s, secondary, secondary_cursor), 'explicit secondary Stop')
    s.checks.append('focused-enter-and-stop-route-to-the-correct-task-while-both-sessions-are-active')

    text(s, 'task-split-composer', 'keep second draft')
    text(s, 'primary-composer-input', 'keep first draft')
    wait_until(lambda: draft(s, secondary) == 'keep second draft' and draft(s, primary) == 'keep first draft', 'both final drafts')
    before = s.events()
    fresh_probe(s, 'task-split-switch', lambda: resize(s.desktop, 980, 930))
    s.desktop.screenshot('narrow-second-task', window_only=True)
    fresh_probe(s, 'task-split-primary', lambda: s.click_control('task-split-switch'))
    s.click_control('primary-composer-input')
    assert s.desktop.copy_input() == 'keep first draft'
    fresh_probe(s, 'task-split-secondary', lambda: s.click_control('task-split-switch'))
    s.click_control('task-split-composer')
    assert s.desktop.copy_input() == 'keep second draft'
    assert s.events() == before
    s.checks.append('narrow-window-shows-one-task-at-a-time-with-explicit-switch-and-no-draft-or-session-transfer')

    fresh_probe(s, 'task-split-primary', lambda: resize(s.desktop, 1420, 930))
    s.click_control('task-split-close')
    s.click_control('task-split-open')
    s.click_control('task-split-choice', slot=0)
    s.click_control('task-split-composer')
    assert s.desktop.copy_input() == 'keep second draft' and s.events() == before
    s.checks.append('close-and-reselect-only-retire-presentation-without-cancelling-or-restarting-tasks')
    close(s)
    s.launch(preserve_selection=True)
    assert s.events() == before and selection(s) == primary
    assert draft(s, primary) == 'keep first draft' and draft(s, secondary) == 'keep second draft'
    log = re.sub(r'\x1b\[[0-9;]*[A-Za-z]', '', Path(s.log.name).read_text())
    assert 'control="task-split-secondary"' not in log
    s.checks.append('restart-keeps-durable-drafts-but-closes-the-transient-split-and-never-autostarts')


def main():
    p = argparse.ArgumentParser()
    for flag in ('binary', 'fixture', 'output'):
        p.add_argument('--' + flag, type=Path, required=True)
    s = Scenario(p.parse_args())
    result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb / owned ACP fixture'}
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
