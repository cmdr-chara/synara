#!/usr/bin/env python3
"""Exercise explicit goal commands in native GPUI with an owned local ACP fixture."""
import argparse
import json
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection
from native_model_draft_smoke import preference, close
from native_integrations_smoke import fill, click


def run(s):
    s.launch()
    before = len(s.events())
    fill(s, 'composer-input', 'hello')
    s.click_control('composer-submit', enabled=True)
    s.finished(before)
    task = selection(s)
    key = 'task-goal:' + task
    initial = sum(e['type'] == 'prompt_started' for e in s.events())

    def command(text):
        fill(s, 'composer-input', text)
        s.click_control('composer-submit', enabled=True)

    command('/synara/goal set Original reviewed objective')
    wait_until(lambda: (preference(s, key) or {}).get('objective') == 'Original reviewed objective', 'goal set from command')
    command('/synara/goal edit New unsaved objective')
    s.click_control('goal-input')
    assert s.desktop.copy_input() == 'New unsaved objective'
    assert preference(s, key)['objective'] == 'Original reviewed objective'
    click(s, 'goal-save')
    wait_until(lambda: preference(s, key)['objective'] == 'New unsaved objective', 'explicit Save after command edit')
    command('/synara/goal resume')
    s.click_control('composer-input')
    text = s.desktop.copy_input()
    assert 'New unsaved objective' in text and task in text
    assert '/synara/goal resume' not in text
    assert sum(e['type'] == 'prompt_started' for e in s.events()) == initial
    # Editing the prepared draft disarms its lease before the destructive command.
    fill(s, 'composer-input', '')
    wait_until(lambda: 'User input' in preference(s, key)['note'], 'input disarms prepared pursuit')
    command('/synara/goal clear')
    wait_until(lambda: not preference(s, key)['objective'], 'explicit command clear')
    assert preference(s, key)['achievements'] == []
    command('/synara/goal resume')
    s.click_control('composer-input')
    assert s.desktop.copy_input() == '/synara/goal resume', 'failed resume preserves command'
    time.sleep(0.3)
    assert sum(e['type'] == 'prompt_started' for e in s.events()) == initial
    s.desktop.screenshot('goal-commands', window_only=True)
    s.checks.append('native-set-edit-save-resume-draft-clear-and-refusal-with-no-provider-send')
    close(s)


def main():
    parser = argparse.ArgumentParser()
    for name in ['binary', 'fixture', 'output']:
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
