#!/usr/bin/env python3
"""Native local build notes, acknowledgement and restart; no update endpoint."""
import argparse
import json
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_model_draft_smoke import preference, close
from native_integrations_smoke import click, fill
from native_studio_settings_smoke import reveal
from native_project_import_smoke import task_events

def journal(s):
    return preference(s, 'native-release-experience-v1')

def page(s):
    s.desktop.key('6', ('Control_L',))
    fill(s, 'settings-search', 'build notes')
    s.click_control('releases')

def run(s):
    s.launch()
    task = selection(s)
    events = task_events(s, task)
    fill(s, 'composer-input', 'My unchanged draft')
    wait_until(lambda: (preference(s, 'task-draft:' + task) or {}).get('text') == 'My unchanged draft', 'saved normal draft')
    first = wait_until(lambda: journal(s), 'observed compiled build')
    assert len(first['history']) == 1 and first['acknowledged'] is None
    assert first['history'][0]['version'] and len(first['history'][0]['notes_sha256']) == 64
    page(s)
    reveal(s, 'release-current-version')
    reveal(s, 'release-update-unavailable')
    s.desktop.screenshot('release-development-status', window_only=True)
    s.checks.append('native-current-build-and-unconfigured-update-status-with-no-invented-release')

    reveal(s, 'release-bundled-notes')
    click(s, 'release-ack')
    seen = wait_until(lambda: journal(s) if journal(s)['acknowledged'] is not None else None, 'durable explicit acknowledgement')
    assert seen['history'] == first['history']
    assert seen['acknowledged']['notes_sha256'] == first['history'][0]['notes_sha256']
    assert seen['revision'] == first['revision'] + 1
    s.checks.append('bundled-notes-have-explicit-durable-mark-read-without-changing-build-history')

    click(s, 'release-reload')
    wait_until(lambda: journal(s) == seen, 'same local history')
    assert task_count(s) == 1 and task_events(s, task) == events
    assert preference(s, 'task-draft:' + task)['text'] == 'My unchanged draft'
    s.checks.append('reload-does-not-create-tasks-send-prompts-or-change-the-unsent-draft')

    close(s)
    s.launch(preserve_selection=True)
    assert journal(s) == seen
    page(s)
    reveal(s, 'release-read')
    s.desktop.screenshot('release-acknowledged-restart', window_only=True)
    assert selection(s) == task and task_count(s) == 1 and task_events(s, task) == events
    assert preference(s, 'task-draft:' + task)['text'] == 'My unchanged draft'
    s.checks.append('restart-keeps-acknowledgement-history-and-existing-conversation-inert')

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
        print(json.dumps(result, indent=2), flush=True)

if __name__ == '__main__':
    main()
