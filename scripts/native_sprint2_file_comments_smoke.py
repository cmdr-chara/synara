#!/usr/bin/env python3
"""Exercise selected-line review through native input and read-only observations."""
import argparse
import hashlib
import json
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_model_draft_smoke import preference, close
from native_integrations_smoke import click, fill, fresh_probe
from native_environment_smoke import choose_tool
from native_project_import_smoke import task_events


def draft(s, task):
    return (preference(s, 'task-draft:' + task) or {}).get('text', '')


def run(s):
    original = b'first line\nsecond line\nthird line\n'
    s.document.write_bytes(original)
    sha = hashlib.sha256(original).hexdigest()
    s.launch()
    task = selection(s)
    events = task_events(s, task)
    fill(s, 'composer-input', 'Keep my unsent instructions')
    wait_until(lambda: draft(s, task) == 'Keep my unsent instructions', 'original saved draft')
    s.click_control('Files')
    choose_tool(s, 'Explorer')
    click(s, 'file-row', slot=0)
    click(s, 'editor-input')
    s.desktop.key('Home', ('Control_L',))
    s.desktop.key('Down')
    s.desktop.key('Home')
    s.desktop.key('End', ('Shift_L',))
    click(s, 'editor-comment')
    fill(s, 'file-comment-input', 'Check this exact second line')
    assert draft(s, task) == 'Keep my unsent instructions'
    assert task_events(s, task) == events
    s.desktop.screenshot('inline-comment-review', window_only=True)
    s.checks.append('selected-line-review-is-editable-and-does-not-send-or-change-the-existing-draft')

    # Use the normal bounded clipboard wait before clearing. An empty selection
    # does not replace X11 clipboard contents, and a one-shot clipboard read can
    # race the native selection owner even after the sentinel is painted.
    fill(s, 'file-comment-input', 'empty-field-sentinel')
    s.desktop.key('BackSpace')
    fresh_probe(s, 'file-comment-error', lambda: click(s, 'file-comment-attach'))
    assert draft(s, task) == 'Keep my unsent instructions'
    fill(s, 'file-comment-input', 'Keep my corrected comment')
    s.document.write_text('Changed outside the app\n')
    fresh_probe(s, 'file-comment-error', lambda: click(s, 'file-comment-attach'))
    click(s, 'file-comment-input')
    assert s.desktop.copy_input() == 'Keep my corrected comment'
    assert draft(s, task) == 'Keep my unsent instructions'
    assert s.document.read_text() == 'Changed outside the app\n'
    s.checks.append('empty-comment-and-stale-file-are-rejected-without-losing-edits-or-writing-files')

    # A deleted/replaced path must fail through the existing filesystem owner.
    s.document.unlink()
    fresh_probe(s, 'file-comment-error', lambda: click(s, 'file-comment-attach'))
    assert not s.document.exists() and draft(s, task) == 'Keep my unsent instructions'
    s.document.write_bytes(original)
    s.checks.append('deleted-file-is-not-recreated-or-guessed-during-review-verification')

    click(s, 'file-comment-discard')
    assert draft(s, task) == 'Keep my unsent instructions'
    click(s, 'file-comment-discard-confirm')
    click(s, 'editor-input')
    s.desktop.key('Home', ('Control_L',))
    s.desktop.key('Down')
    s.desktop.key('Home')
    s.desktop.key('End', ('Shift_L',))
    click(s, 'editor-comment')
    fill(s, 'file-comment-input', 'Review this saved line only')
    s.checks.append('explicit-discard-preserves-the-normal-draft-and-allows-recapture')

    click(s, 'file-comment-attach')
    wait_until(lambda: 'synara-file-review-v1' in draft(s, task), 'explicit reviewed attachment')
    combined = draft(s, task)
    assert combined.startswith('Keep my unsent instructions\n\nFile review comment')
    data = json.loads(combined[combined.index('{'):])
    assert data['path'] == 'document.txt' and data['sha256'] == sha
    assert data['start_line'] == 2 and data['end_line'] == 2
    assert data['excerpt'] == 'second line' and data['comment'] == 'Review this saved line only'
    assert combined.count('synara-file-review-v1') == 1
    assert selection(s) == task and task_count(s) == 1 and task_events(s, task) == events
    assert s.document.read_bytes() == original
    s.desktop.screenshot('inline-comment-unsent', window_only=True)
    s.checks.append('fresh-file-range-hash-and-edited-comment-attach-once-to-the-correct-unsent-draft')

    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == task and draft(s, task) == combined
    assert task_events(s, task) == events and task_count(s) == 1
    assert s.document.read_bytes() == original
    s.checks.append('restart-restores-the-frozen-annotation-in-the-draft-without-autostart-or-file-mutation')


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
