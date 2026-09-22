#!/usr/bin/env python3
"""Native draft/notes checkpoint rollback, with owned fixture data only.

All mutations use native controls. SQLite reads are test observations, not a
substitute implementation. No repository files, real agent or remote account.
"""
import argparse
import json
import re
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count, event_cursor, task_events, prompt_finished
from native_model_draft_smoke import preference, close
from native_integrations_smoke import fresh_probe


def text(s, control, value):
    s.click_control(control)
    s.desktop.key('a', ('Control_L',))
    s.desktop.text(value)


def draft(s, task):
    return (preference(s, 'task-draft:' + task) or {}).get('text', '')


def history(s, task):
    return preference(s, 'task-checkpoints:' + task) or {'items': [], 'revision': 0}


def notes(s, task):
    return preference(s, 'task-context:' + task) or {'notes': '', 'checklist': [], 'revision': 0}


def edit_notes(s, task, value, add=False):
    fresh_probe(s, 'context-notes-input', lambda: s.click_control('chat-notes'))
    text(s, 'context-notes-input', value)
    if add:
        text(s, 'context-item-input', 'Keep this checklist item')
        s.click_control('context-item-add')
        s.click_control('context-check', slot=0)
    s.click_control('context-save')
    wait_until(lambda: notes(s, task)['notes'] == value, 'saved notes')
    s.click_control('context-close')


def run(s):
    s.launch()
    task = selection(s)
    file = s.project / 'unchanged-by-checkpoints.txt'
    file.write_bytes(b'Workspace files are not in this checkpoint.\r\n')
    text(s, 'primary-composer-input', 'Original unsent draft')
    wait_until(lambda: draft(s, task) == 'Original unsent draft', 'original durable draft')
    edit_notes(s, task, 'Original private notes', add=True)
    original = notes(s, task)
    before = s.events()
    fresh_probe(s, 'checkpoint-history', lambda: s.click_control('checkpoint-open'))
    assert history(s, task)['items'] == [] and s.events() == before
    s.click_control('checkpoint-capture', enabled=True)
    saved = wait_until(lambda: history(s, task)['items'] and history(s, task), 'explicit saved checkpoint')
    assert saved['items'][0]['draft'] == 'Original unsent draft'
    assert saved['items'][0]['context'] == original
    assert s.events() == before
    s.checks.append('explicit-capture-preserves-unsent-draft-and-saved-notes-without-agent-execution')

    text(s, 'primary-composer-input', 'Later unsent draft')
    wait_until(lambda: draft(s, task) == 'Later unsent draft', 'later draft')
    edit_notes(s, task, 'Later private notes')
    later = notes(s, task)
    fresh_probe(s, 'checkpoint-review', lambda: s.click_control('checkpoint-item', slot=0))
    assert draft(s, task) == 'Later unsent draft' and notes(s, task) == later
    s.desktop.screenshot('checkpoint-review-boundary', window_only=True)
    fresh_probe(s, 'checkpoint-history', lambda: s.click_control('checkpoint-cancel'))
    assert history(s, task) == saved and s.events() == before
    s.checks.append('review-and-cancel-never-restore-or-send-and-display-the-metadata-only-boundary')

    fresh_probe(s, 'checkpoint-review', lambda: s.click_control('checkpoint-item', slot=0))
    fresh_probe(s, 'checkpoint-history', lambda: s.click_control('checkpoint-restore'))
    restored = history(s, task)
    assert len(restored['items']) == 2 and restored['items'][-1]['label'] == 'Before revert (recovery)'
    assert restored['items'][-1]['draft'] == 'Later unsent draft'
    assert restored['items'][-1]['context'] == later
    assert draft(s, task) == 'Original unsent draft'
    assert notes(s, task)['notes'] == original['notes'] and notes(s, task)['checklist'] == original['checklist']
    assert notes(s, task)['revision'] == later['revision'] + 1
    s.click_control('primary-composer-input')
    assert s.desktop.copy_input() == 'Original unsent draft'
    assert file.read_bytes() == b'Workspace files are not in this checkpoint.\r\n' and s.events() == before
    s.desktop.screenshot('checkpoint-restored-with-recovery', window_only=True)
    s.checks.append('confirmed-atomic-rollback-refreshes-composer-and-notes-with-recovery-but-leaves-files-and-transcript-unchanged')

    # Newest item is the pre-revert recovery, not a fabricated provider rewind.
    fresh_probe(s, 'checkpoint-review', lambda: s.click_control('checkpoint-item', slot=0))
    fresh_probe(s, 'checkpoint-history', lambda: s.click_control('checkpoint-restore'))
    assert draft(s, task) == 'Later unsent draft' and notes(s, task)['notes'] == 'Later private notes'
    fresh_probe(s, 'checkpoint-review', lambda: s.click_control('checkpoint-item', slot=2))
    current = history(s, task)
    text(s, 'primary-composer-input', 'Intervening local edit')
    wait_until(lambda: draft(s, task) == 'Intervening local edit', 'intervening draft edit')
    fresh_probe(s, 'checkpoint-error', lambda: s.click_control('checkpoint-restore'))
    assert draft(s, task) == 'Intervening local edit' and history(s, task) == current and s.events() == before
    s.checks.append('recovery-is-usable-and-stale-native-review-refuses-overwriting-newer-draft')

    cursor = event_cursor(s, task)
    text(s, 'primary-composer-input', 'hold')
    s.desktop.key('Return')
    wait_until(lambda: any(e.get('text') == 'Started waiting' for e in task_events(s, task, cursor)), 'active fixture turn')
    current = history(s, task)
    s.click_control('checkpoint-capture', enabled=False)
    assert history(s, task) == current and not prompt_finished(s, task, cursor)
    s.click_control('composer-submit')
    wait_until(lambda: prompt_finished(s, task, cursor), 'explicit Stop')
    text(s, 'primary-composer-input', 'Retain after restart')
    wait_until(lambda: draft(s, task) == 'Retain after restart', 'restart draft saved')
    before = s.events()
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == task and history(s, task) == current and draft(s, task) == 'Retain after restart'
    assert s.events() == before
    log = re.sub(r'\x1b\[[0-9;]*[A-Za-z]', '', Path(s.log.name).read_text())
    assert 'control="checkpoint-review"' not in log
    fresh_probe(s, 'checkpoint-history', lambda: s.click_control('checkpoint-open'))
    s.desktop.screenshot('checkpoint-restart-history-not-consent', window_only=True)
    s.click_control('checkpoint-close')
    s.click_control('new-thread')
    other = wait_until(lambda: selection(s) != task and selection(s), 'different task selected')
    fresh_probe(s, 'checkpoint-history', lambda: s.click_control('checkpoint-open'))
    assert task_count(s) == 2 and history(s, other)['items'] == []
    assert history(s, task) == current and draft(s, task) == 'Retain after restart'
    assert file.read_bytes() == b'Workspace files are not in this checkpoint.\r\n'
    s.checks.append('active-task-refusal-restart-without-consent-or-autostart-and-independent-task-history')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for flag in ('binary', 'fixture', 'output'):
        p.add_argument('--' + flag, type=Path, required=True)
    s = Scenario(p.parse_args())
    result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb / owned ACP fixture'}
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
        (s.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
