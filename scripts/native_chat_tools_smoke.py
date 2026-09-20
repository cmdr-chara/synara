#!/usr/bin/env python3
"""Batched native chat utility journey using only Scenario-owned data and agents."""
import argparse
import json
import re
import sqlite3
from pathlib import Path

from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection
from native_presentation_smoke import resize
from native_rich_text_smoke import clipboard


def preference(s, key):
    with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
        row = db.execute('SELECT data FROM preferences WHERE key=?', (key,)).fetchone()
        return json.loads(row[0]) if row else None


def close(s):
    s.desktop.request_close()
    wait_until(lambda: s.process.poll() is not None, 'chat utility shutdown')
    assert s.process.returncode == 0
    s.log.close()
    s.log = None


def action(s, label):
    s.click_control('chat-actions')
    s.desktop.text(label)
    s.desktop.key('Return')


def latest_search_count(s):
    text = re.sub(r'\x1b\[[0-9;]*[A-Za-z]', '', Path(s.log.name).read_text(errors='replace'))
    lines = [line for line in text.splitlines() if 'message-search-complete' in line]
    if not lines:
        return None
    match = re.search(r'\bhits=(\d+)', lines[-1])
    return int(match[1]) if match else None


def run(s):
    s.launch()
    ui = s.desktop
    task = selection(s)
    cursor = s.prompt('hello')
    s.finished(cursor)
    baseline = s.events()
    resize(ui, 1280, 900, s.scale)
    s.click_control('composer-input')
    ui.text('Do not send this draft.')
    wait_until(lambda: preference(s, 'task-draft:' + task), 'initial unsent draft')
    ui.key('f', ('Control_L',))
    s.click_control('message-find-input')
    ui.text('Hello from')
    wait_until(lambda: latest_search_count(s) == 1, 'find reconstructed assistant text')
    wait_until(lambda: s.control_bounds('message-match'), 'matched native message')
    ui.screenshot('chat-find-message', window_only=True)
    ui.key('a', ('Control_L',))
    ui.text('NO_SUCH_MESSAGE_UTILITY_TEST')
    wait_until(lambda: latest_search_count(s) == 0, 'no-match search')
    ui.key('Escape')
    s.click_control('composer-input')
    assert ui.copy_input() == 'Do not send this draft.'
    ui.focus()
    assert s.events() == baseline
    s.checks.append('conversation-search-and-empty-results-preserve-unsent-text-and-durable-events')

    s.click_control('message-pin')
    key = 'message-pins:' + task
    pins = wait_until(lambda: preference(s, key), 'durable message pin')
    assert len(pins['entries']) == 1
    s.click_control('chat-pins')
    ui.screenshot('chat-pinned-messages', window_only=True)
    ui.key('Return')
    assert s.events() == baseline
    s.checks.append('pin-message-and-jump-without-changing-history-or-submitting-a-prompt')

    action(s, 'Copy text conversation')
    copied = wait_until(lambda: clipboard(ui) if 'Synara text conversation' in clipboard(ui) else None, 'Markdown conversation clipboard')
    assert 'Hello from' in copied and '## User' in copied and '## Assistant' in copied
    assert 'Do not send this draft.' not in copied
    assert s.events() == baseline
    action(s, 'Reuse last prompt')
    s.click_control('composer-input')
    assert ui.copy_input() == 'Do not send this draft.\n\nhello'
    ui.focus()
    assert s.events() == baseline
    s.click_control('message-reuse')
    s.click_control('composer-input')
    quoted = ui.copy_input()
    assert quoted.startswith('Do not send this draft.\n\nhello\n\n')
    assert '> ' in quoted and 'Hello from' in quoted
    ui.focus()
    ui.screenshot('chat-quote-and-reuse', window_only=True)
    s.checks.append('copy-excludes-unsent-draft-and-reuse-quote-append-without-automatic-send')

    s.click_control('chat-commands')
    ui.screenshot('chat-advertised-commands', window_only=True)
    ui.key('Escape')
    ui.key('k', ('Control_L',))
    ui.screenshot('chat-global-thread-finder', window_only=True)
    ui.key('Return')
    assert selection(s) == task and s.events() == baseline
    s.click_control('composer-input')
    assert ui.copy_input() == quoted
    ui.focus()
    s.checks.append('command-discovery-and-global-thread-finder-do-not-send-or-discard-drafts')

    for width, height in [(1100, 800), (960, 760)]:
        resize(ui, width, height, s.scale)
        s.click_control('chat-find')
        x, _, w, _ = s.control_bounds('message-find-bar')
        assert x >= 0 and x + w <= width + 1
        ui.screenshot(f'chat-utilities-{width}', window_only=True)
        ui.key('Escape')
    close(s)
    s.launch(preserve_selection=True)
    assert preference(s, key) == pins
    assert selection(s) == task and s.events() == baseline
    s.click_control('composer-input')
    assert ui.copy_input() == quoted
    ui.focus()
    ui.screenshot('chat-utility-restoration', window_only=True)
    s.checks.append('pins-and-combined-draft-survive-restart-without-new-agent-events')
    close(s)


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
    except BaseException:
        import traceback
        result['error'] = traceback.format_exc()
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure')
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
