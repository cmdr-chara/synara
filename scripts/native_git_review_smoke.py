#!/usr/bin/env python3
"""Real temporary Git repository, native review, draft recovery and restart.

No user repository, credential, network or provider is used. Status, diffs,
index changes and commits are checked with Git independently of the UI.
"""
import argparse
import json
import sqlite3
import subprocess
import time
from pathlib import Path

from native_smoke import Scenario, wait_until
from native_environment_smoke import choose_tool
from native_navigation_smoke import selection
from native_presentation_smoke import resize
from native_rich_text_smoke import clipboard


def git(s, *args):
    return subprocess.check_output(['git', '-C', str(s.project), *args], timeout=15).decode()


def messages(s):
    return Path(s.log.name).read_text(errors='replace')


def review(s):
    with sqlite3.connect(s.data / 'native-workspace.sqlite3') as db:
        row = db.execute("SELECT data FROM preferences WHERE key LIKE 'git-review:%'").fetchone()
    return json.loads(row[0]) if row else None


def saved(s, **expected):
    value = review(s)
    return value if value and all(value.get(k) == v for k, v in expected.items()) else None


def draft(s, task):
    with sqlite3.connect(s.data / 'native-workspace.sqlite3') as db:
        row = db.execute('SELECT data FROM preferences WHERE key=?', ('task-draft:' + task,)).fetchone()
    return json.loads(row[0])['text'] if row else ''


def entries(s, staged):
    fields = iter(git(s, 'status', '--porcelain=v1', '-z').split('\0'))
    result = []
    for item in fields:
        if not item:
            continue
        state, name = item[:2], item[3:]
        if 'R' in state or 'C' in state:
            next(fields)
        changed = (state[0] not in ' ?') if staged else (state[1] != ' ')
        if changed:
            result.append(name)
    return result


def action(s, control, slot=None):
    before = messages(s).count('review-ready')
    s.click_control(control, slot=slot)
    wait_until(lambda: messages(s).count('review-ready') > before, control + ' acknowledged Git read')


def select_file(s, name, staged=False):
    action(s, 'review-file', entries(s, staged).index(name))
    wait_until(lambda: saved(s, path=name, staged=staged), 'selected file persisted')


def copy_diff(s, expected):
    s.click_control('review-copy')
    assert clipboard(s.desktop) == expected, 'Copied diff differs from the actual Git response'
    s.desktop.focus()


def close(s):
    def request():
        if s.process.poll() is None:
            s.desktop.request_close()
            return False
        return True
    wait_until(request, 'review/draft-safe application close')
    assert s.process.returncode == 0
    s.log.close()
    s.log = None


def run(s):
    git(s, 'init', '-q')
    git(s, 'config', 'user.name', 'Synara Review Fixture')
    git(s, 'config', 'user.email', 'review-fixture@example.invalid')
    (s.project / 'a.rs').write_text('fn main() {\n    original();\n}\n')
    (s.project / 'old.md').write_text('# Original\n')
    (s.project / 'binary.bin').write_bytes(b'\x00before')
    (s.project / 'deleted.txt').write_text('Delete this fixture only\n')
    git(s, 'add', '--', '.')
    git(s, 'commit', '-qm', 'Initial fixture')
    (s.project / 'a.rs').write_text('fn main() {\n    staged();\n}\n')
    git(s, 'add', '--', 'a.rs')
    (s.project / 'a.rs').write_text('fn main() {\n    working();\n}\n')
    git(s, 'mv', 'old.md', 'renamed.md')
    (s.project / 'binary.bin').write_bytes(b'\x00after')
    (s.project / 'deleted.txt').unlink()
    (s.project / 'new.txt').write_text('Untracked fixture\n')
    hook = s.project / '.git/hooks/pre-commit'
    hook.write_text('#!/bin/sh\ntouch hook-must-not-run\nexit 1\n')
    hook.chmod(0o700)
    git(s, 'config', 'commit.gpgsign', 'true')

    s.launch()
    ui = s.desktop
    resize(ui, 1440, 940, s.scale)
    task = selection(s)
    baseline = s.events()
    s.click_control('composer-input')
    ui.text('Keep my unsent chat.')
    s.click_control('Files')
    choose_tool(s, 'Changes')
    wait_until(lambda: 'review-ready' in messages(s), 'initial Git review')
    s.click_control('environment-maximize')
    select_file(s, 'a.rs')
    copy_diff(s, git(s, 'diff', '--no-color', '--', 'a.rs'))
    ui.screenshot('review-unified-wide', window_only=True)
    s.checks.append('per-file-working-diff-matches-Git-without-index-mutation')

    action(s, 'review-staged')
    wait_until(lambda: saved(s, staged=True, path='a.rs'), 'staged selection')
    copy_diff(s, git(s, 'diff', '--no-color', '--cached', '--', 'a.rs'))
    select_file(s, 'renamed.md', staged=True)
    copy_diff(s, git(s, 'diff', '--no-color', '--cached', '--', 'renamed.md'))
    ui.screenshot('review-staged-rename', window_only=True)
    s.click_control('review-raw')
    wait_until(lambda: saved(s, raw=True), 'raw review preference')
    ui.screenshot('review-raw', window_only=True)
    s.checks.append('staged-working-separation-rename-metadata-and-exact-raw-copy')

    action(s, 'review-worktree')
    select_file(s, 'a.rs')
    before = messages(s).count('review-ready')
    ui.key('Down')
    wait_until(lambda: messages(s).count('review-ready') > before, 'keyboard file selection')
    wait_until(lambda: saved(s, path='binary.bin'), 'keyboard-selected binary persisted')
    ui.screenshot('review-keyboard-binary', window_only=True)
    copy_diff(s, git(s, 'diff', '--no-color', '--', 'binary.bin'))
    select_file(s, 'new.txt')
    ui.screenshot('review-untracked', window_only=True)
    action(s, 'review-index', entries(s, False).index('new.txt'))
    assert 'new.txt' in entries(s, True)
    assert (s.project / 'new.txt').read_text() == 'Untracked fixture\n'
    action(s, 'review-staged')
    select_file(s, 'new.txt', staged=True)
    action(s, 'review-index', entries(s, True).index('new.txt'))
    assert 'new.txt' not in entries(s, True)
    assert (s.project / 'new.txt').read_text() == 'Untracked fixture\n'
    s.checks.append('keyboard-navigation-and-scoped-stage-unstage-preserve-working-files')

    action(s, 'review-worktree')
    select_file(s, 'a.rs')
    s.click_control('review-to-chat')
    wait_until(lambda: 'Git review:' in draft(s, task), 'diff appended to durable unsent draft')
    assert draft(s, task).startswith('Keep my unsent chat.\n\n')
    s.click_control('review-commit-input')
    ui.text('Keep this commit draft')
    wait_until(lambda: saved(s, commit_message='Keep this commit draft'), 'commit draft saved')
    s.click_control('environment-maximize')
    for width in (1100, 960):
        resize(ui, width, 820, s.scale)
        for control in ('git-review', 'review-file-list', 'review-commit-input'):
            x, _, w, _ = s.control_bounds(control)
            assert x >= 0 and x + w <= width + 1, (control, x, w, width)
        ui.screenshot(f'review-split-{width}', window_only=True)
    assert s.events() == baseline
    close(s)
    s.launch(preserve_selection=True)
    wait_until(lambda: 'review-ready' in messages(s), 'restored review')
    assert saved(s, path='a.rs', raw=True, commit_message='Keep this commit draft')
    s.click_control('review-copy-draft')
    assert clipboard(ui) == 'Keep this commit draft'
    ui.focus()
    assert draft(s, task).startswith('Keep my unsent chat.\n\n')
    ui.screenshot('review-restored', window_only=True)
    s.checks.append('review-choices-commit-draft-and-chat-context-survive-restart-without-agent-execution')

    # A second window writes a newer value. The running UI must retain local text
    # and refuse its stale save rather than replacing the other window's draft.
    with sqlite3.connect(s.data / 'native-workspace.sqlite3') as db:
        key, raw = db.execute("SELECT key,data FROM preferences WHERE key LIKE 'git-review:%'").fetchone()
        remote = json.loads(raw)
        remote['revision'] += 1
        remote['commit_message'] = 'Saved by another window'
        db.execute('UPDATE preferences SET data=? WHERE key=?', (json.dumps(remote), key))
    s.click_control('review-commit-input')
    ui.key('a', ('Control_L',))
    ui.text('My local draft is not disposable')
    wait_until(lambda: s.control_bounds('review-reload'), 'stale-write recovery controls')
    assert saved(s, commit_message='Saved by another window')
    s.click_control('review-copy-draft')
    assert clipboard(ui) == 'My local draft is not disposable'
    ui.focus()
    ui.request_close()
    time.sleep(0.3)
    assert s.process.poll() is None, 'Unsaved review conflict must block close'
    ui.screenshot('review-stale-draft-recovery', window_only=True)
    before = messages(s).count('review-ready')
    s.click_control('review-reload')
    s.click_control('review-reload')
    wait_until(lambda: saved(s, commit_message='Saved by another window'), 'saved version retained')
    wait_until(lambda: messages(s).count('review-ready') > before, 'reloaded review')
    s.click_control('review-copy-draft')
    assert clipboard(ui) == 'Saved by another window'
    ui.focus()
    s.checks.append('stale-save-refusal-copy-recovery-and-explicit-two-step-reload')

    before_head = git(s, 'rev-parse', 'HEAD')
    action(s, 'review-commit')
    assert git(s, 'rev-parse', 'HEAD') != before_head
    assert git(s, 'log', '-1', '--format=%s').strip() == 'Saved by another window'
    assert not (s.project / 'hook-must-not-run').exists()
    assert (s.project / 'a.rs').read_text() == 'fn main() {\n    working();\n}\n'
    wait_until(lambda: saved(s, commit_message=''), 'acknowledged commit draft cleared')
    s.checks.append('explicit-local-commit-disabled-hooks-signing-and-post-acknowledgement-draft-clear')

    # A failed repository read must not be presented as a clean repository.
    metadata = s.project / '.git'
    hidden = s.project / '.fixture-git'
    metadata.rename(hidden)
    before = messages(s).count('review-failed')
    s.click_control('review-refresh')
    wait_until(lambda: messages(s).count('review-failed') > before, 'visible failed Git read')
    ui.screenshot('review-git-error', window_only=True)
    hidden.rename(metadata)
    action(s, 'review-refresh')
    assert s.events() == baseline
    s.checks.append('failed-Git-read-distinguished-from-clean-and-refresh-recovers')
    close(s)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = dict(status='failed', checks=s.checks, platform='Linux/X11/private Xvfb')
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
