#!/usr/bin/env python3
"""Real native recap review, explicit fixture generation, caching and restart."""
import argparse
import json
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, event_cursor, prompt_finished, task_count
from native_model_draft_smoke import preference, close
from native_project_import_smoke import task_events
from native_integrations_smoke import fill, click


def run(s):
    s.launch()
    source = selection(s)
    before = s.prompt('hello')
    s.finished(before)
    fill(s, 'composer-input', 'Keep my original draft')
    wait_until(lambda: (preference(s, 'task-draft:' + source) or {}).get('text') == 'Keep my original draft', 'source draft saved')
    original = task_events(s, source)
    count = task_count(s)
    click(s, 'recap-open')
    click(s, 'recap-review')
    wait_until(lambda: s.control_bounds('recap-editor'), 'visible recap context editor')
    s.click_control('recap-editor')
    text = s.desktop.copy_input()
    assert 'Hello from alpha' in text and 'Create a concise thread recap' in text
    assert s.control_bounds('recap-editor')[3] >= 180
    assert task_count(s) == count and task_events(s, source) == original
    click(s, 'recap-cancel')
    assert task_count(s) == count
    s.checks.append('bounded-visible-recap-review-and-cancel-never-send-or-mutate-source')
    click(s, 'recap-open')
    click(s, 'recap-review')
    wait_until(lambda: s.control_bounds('recap-editor'), 'fresh recap review')
    s.desktop.screenshot('recap-reviewed-context', window_only=True)
    click(s, 'recap-create')
    wait_until(lambda: selection(s) != source, 'independent unsent recap task')
    child = selection(s)
    assert task_events(s, child)[1:] == ([], 0)
    assert preference(s, 'thread-origin:' + child)['kind'] == 'recap'
    assert preference(s, 'task-draft:' + child)['text'] == text
    assert task_events(s, source) == original
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == child and task_events(s, child)[1:] == ([], 0)
    assert preference(s, 'task-draft:' + child)['text'] == text
    s.checks.append('recap-request-has-independent-identity-and-restores-without-autostart')
    before = event_cursor(s, child)
    click(s, 'composer-submit')
    wait_until(lambda: prompt_finished(s, child, before), 'explicit recap generation on existing agent runtime')
    click(s, 'recap-open')
    click(s, 'recap-cache')
    wait_until(lambda: preference(s, 'task-recap:' + source), 'generated recap cached')
    wait_until(lambda: selection(s) == source, 'source reopened after caching')
    saved = preference(s, 'task-recap:' + source)
    assert saved['source'] == source and saved['generated_task'] == child
    assert 'Hello from alpha' in saved['text']
    assert task_events(s, source) == original
    s.click_control('composer-input')
    assert s.desktop.copy_input() == 'Keep my original draft'
    s.desktop.screenshot('recap-cached-on-source', window_only=True)
    s.checks.append('explicit-generated-recap-cache-preserves-original-history-session-and-draft')
    close(s)
    s.launch(preserve_selection=True)
    click(s, 'recap-open')
    assert preference(s, 'task-recap:' + source) == saved
    assert task_events(s, source) == original
    click(s, 'recap-review')
    wait_until(lambda: s.control_bounds('recap-editor'), 'refresh reviews a new bounded request')
    assert task_count(s) == count + 1
    click(s, 'recap-cancel')
    assert preference(s, 'task-recap:' + source) == saved
    s.checks.append('cached-recap-survives-restart-and-refresh-remains-review-first')
    close(s)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
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
