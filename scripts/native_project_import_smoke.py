#!/usr/bin/env python3
"""Real native history discovery/review/import/retry/duplicate/branch/restart journey.

Source JSONL fixtures are ordinary user-selected files. SQLite access below is
read-only verification, never a shortcut for native input or import installation.
"""
import argparse
import json
import sqlite3
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, click
from native_model_draft_smoke import preference, close
from native_navigation_smoke import selection
from native_studio_settings_smoke import reveal

CODEX = '11111111-1111-4111-8111-111111111111'
CLAUDE = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
USER = '22222222-2222-4222-8222-222222222222'
EARLY = '33333333-3333-4333-8333-333333333333'
LATER = '44444444-4444-4444-8444-444444444444'


def ledger(s):
    return preference(s, 'history-imports-v1') or {'receipts': []}


def task_events(s, task):
    path = s.data / 'native-workspace.sqlite3'
    with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True) as db:
        row = db.execute('SELECT data,thread_id FROM tasks WHERE id=?', (task,)).fetchone()
        events = [json.loads(row[0]) for row in db.execute('SELECT data FROM events WHERE thread_id=? ORDER BY sequence', (row[1],))]
        sessions = db.execute('SELECT COUNT(*) FROM sessions WHERE thread_id=?', (row[1],)).fetchone()[0]
        return json.loads(row[0]), events, sessions


def settings(s):
    s.desktop.key('6', ('Control_L',))
    fill(s, 'settings-search', 'Project import')
    s.click_control('project-import')


def prepare(s, root):
    fill(s, 'import-root', str(root))
    click(s, 'import-scan')
    click(s, 'import-preview-file')
    click(s, 'import-destination')
    click(s, 'import-agent')
    click(s, 'import-review-destination')
    reveal(s, 'import-confirm')


def run(s):
    root = s.data.parent / 'history-source'
    root.mkdir()
    source = root / 'rollout.jsonl'
    rows = [
        {'type': 'session_meta', 'payload': {'id': CODEX, 'cwd': '/not-the-destination'}},
        {'type': 'event_msg', 'timestamp': '2025-01-02T03:04:05Z', 'payload': {'type': 'user_message', 'message': 'Source history question'}},
        {'type': 'response_item', 'payload': {'type': 'message', 'role': 'user', 'content': [{'type': 'input_text', 'text': 'Source history question'}]}},
        {'type': 'event_msg', 'timestamp': '2025-01-02T03:04:06Z', 'payload': {'type': 'agent_message', 'message': 'Imported native answer'}},
        {'type': 'response_item', 'payload': {'type': 'reasoning', 'encrypted_content': 'NEVER_IMPORT_THIS'}},
    ]
    original = ('\n'.join(map(json.dumps, rows)) + '\n').encode()
    source.write_bytes(original)
    (root / 'auth.json').write_text('NEVER_IMPORT_THIS')
    s.launch()
    original_task = selection(s)
    before = task_events(s, original_task)
    settings(s)
    prepare(s, root)
    assert ledger(s)['receipts'] == [] and task_events(s, original_task) == before
    s.desktop.screenshot('project-import-review', window_only=True)
    s.checks.append('native-source-preview-and-destination-review-are-inert')
    source.write_bytes(original + b'\n')
    click(s, 'import-confirm')
    reveal(s, 'import-error')
    assert ledger(s)['receipts'] == []
    s.checks.append('changed-source-refuses-unreviewed-native-import')
    source.write_bytes(original)
    click(s, 'import-back-preview')
    click(s, 'import-preview-file')
    click(s, 'import-review-destination')
    click(s, 'import-confirm')
    wait_until(lambda: len(ledger(s)['receipts']) == 1, 'atomic native history import')
    receipt = ledger(s)['receipts'][0]
    imported = receipt['task']
    task, events, sessions = task_events(s, imported)
    assert task['scope'] == 'chat' and task['state'] == 'ready' and sessions == 0
    assert task['working_directory'] == before[0]['working_directory']
    assert preference(s, 'task-draft:' + imported)['text'] == ''
    assert [e['text'] for e in events if e['type'] == 'text_delta'] == ['Source history question', 'Imported native answer']
    assert all(e['type'] in ('text_delta', 'notice', 'title_changed') for e in events)
    assert 'NEVER_IMPORT_THIS' not in json.dumps(events)
    assert source.read_bytes() == original and task_events(s, original_task) == before
    s.checks.append('native-import-is-text-only-atomic-scoped-unsent-and-preserves-source')
    click(s, 'import-review-destination')
    click(s, 'import-confirm')
    reveal(s, 'import-open-chat')
    assert len(ledger(s)['receipts']) == 1 and task_events(s, imported) == (task, events, 0)
    s.checks.append('native-repeated-import-returns-existing-chat-without-duplicates')
    click(s, 'import-open-chat')
    wait_until(lambda: selection(s) == imported, 'imported chat opened')
    s.desktop.screenshot('project-import-transcript', window_only=True)
    assert task_events(s, imported) == (task, events, 0)
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == imported and task_events(s, imported) == (task, events, 0)
    assert len(ledger(s)['receipts']) == 1
    s.checks.append('native-open-and-restart-restore-text-and-receipt-without-execution')
    claude_root = root.parent / 'claude-history'
    claude_root.mkdir()
    def message(id_, parent, role, text):
        return {'type': role, 'uuid': id_, 'parentUuid': parent, 'sessionId': CLAUDE, 'message': {'role': role, 'content': [{'type': 'text', 'text': text}]}}
    claude_bytes = ('\n'.join(map(json.dumps, [message(USER, None, 'user', 'Branch question'), message(EARLY, USER, 'assistant', 'Earlier branch answer'), message(LATER, USER, 'assistant', 'Later branch answer')])) + '\n').encode()
    (claude_root / 'conversation.jsonl').write_bytes(claude_bytes)
    settings(s)
    click(s, 'import-claude')
    fill(s, 'import-root', str(claude_root))
    click(s, 'import-scan')
    click(s, 'import-preview-file')
    click(s, 'import-first-leaf')
    click(s, 'import-destination')
    click(s, 'import-agent')
    click(s, 'import-review-destination')
    s.desktop.screenshot('project-import-claude-branch', window_only=True)
    click(s, 'import-confirm')
    wait_until(lambda: len(ledger(s)['receipts']) == 2, 'selected Claude branch imported')
    receipt = next(r for r in ledger(s)['receipts'] if r['provider'] == 'claude')
    assert receipt['leaf'] == EARLY
    task2, events2, sessions2 = task_events(s, receipt['task'])
    assert [e['text'] for e in events2 if e['type'] == 'text_delta'] == ['Branch question', 'Earlier branch answer']
    assert sessions2 == 0 and task2['scope'] == 'chat'
    assert (claude_root / 'conversation.jsonl').read_bytes() == claude_bytes
    s.checks.append('native-Claude-branch-choice-imports-one-ancestor-chain-not-siblings')
    s.click_control('settings-back')
    close(s)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'platform': 'Linux/X11/private Xvfb, owned JSONL fixtures'}
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


if __name__ == '__main__':
    main()
