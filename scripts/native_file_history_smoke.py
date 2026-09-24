#!/usr/bin/env python3
"""Read committed file revisions in GPUI while preserving an edited buffer.

All Git writes below prepare an isolated local fixture before the app launches.
No provider, real repository history, or remote authentication is involved.
"""
import argparse
import json
import subprocess
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, fresh_probe
from native_environment_smoke import choose_tool
from native_presentation_smoke import resize
from native_rich_text_smoke import clipboard
from native_studio_explorer_smoke import close


def run(s):
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=s.project, stderr=subprocess.PIPE)
    file = s.project / 'document.txt'
    git('init', '-q')
    git('config', 'user.name', 'History Fixture')
    git('config', 'user.email', 'fixture@example.invalid')
    original = 'Original committed text\n'
    current = 'Second committed text\n'
    file.write_text(original)
    git('add', '--', 'document.txt')
    git('-c', 'commit.gpgSign=false', '-c', 'core.hooksPath=/dev/null', 'commit', '-qm', 'Original version')
    file.write_text(current)
    git('add', '--', 'document.txt')
    git('-c', 'commit.gpgSign=false', '-c', 'core.hooksPath=/dev/null', 'commit', '-qm', 'Second version')
    head = git('rev-parse', 'HEAD')
    s.launch()
    resize(s.desktop, 1440, 940, s.scale)
    events = s.events()
    s.click_control('Files')
    choose_tool(s, 'Explorer')
    # Exercise history from the real Explorer file owner, not clipped search rows.
    wait_until(lambda: s.control_bounds('file-row', slot=0), 'Explorer file row')
    s.click_control('file-row', slot=0)
    wait_until(lambda: s.control_bounds('editor-input'), 'opened editor buffer')
    # This text is only in the editor buffer, not committed or written to disk.
    unsaved = 'Preserve my unsaved buffer'
    fill(s, 'editor-input', unsaved)
    s.click_control('editor-history')
    wait_until(lambda: s.control_bounds('editor-history-commit', slot=1), 'two actual file commits')
    s.click_control('editor-history-commit', slot=1)
    wait_until(lambda: s.control_bounds('editor-history-preview'), 'old immutable revision')
    s.click_control('editor-history-copy')
    assert clipboard(s.desktop) == original
    assert file.read_text() == current and git('rev-parse', 'HEAD') == head
    assert s.events() == events
    # Typing while inspecting the read-only revision must not edit a hidden buffer.
    s.desktop.focus()
    s.desktop.key('x')
    s.desktop.screenshot('committed-revision-keeps-editor-buffer', window_only=True)
    s.click_control('editor-history-close')
    s.click_control('editor-input')
    assert s.desktop.copy_input() == unsaved
    # Reopening history gets another explicit snapshot and never writes the buffer.
    fresh_probe(s, 'editor-history-commit', lambda: s.click_control('editor-history'))
    fresh_probe(s, 'editor-history-preview', lambda: s.click_control('editor-history-commit', slot=0))
    s.click_control('editor-history-copy')
    assert clipboard(s.desktop) == current
    s.click_control('editor-history-close')
    s.click_control('editor-input')
    assert s.desktop.copy_input() == unsaved
    # Add one separate edit after returning, then prove the retained Undo owner.
    s.desktop.key('End')
    time.sleep(0.4)
    s.desktop.text('X')
    assert s.desktop.copy_input() == unsaved + 'X'
    s.desktop.key('z', ('Control_L',))
    assert s.desktop.copy_input() == unsaved
    assert file.read_text() == current and s.events() == events
    # Restore the unchanged disk text in the buffer before a normal clean close.
    fill(s, 'editor-input', current)
    close(s)
    s.launch(preserve_selection=True)
    assert file.read_text() == current and s.events() == events
    assert git('rev-parse', 'HEAD') == head
    s.checks.append('local-commit-list-revision-copy-preserves-unsaved-buffer-undo-disk-head-and-inert-restart')
    close(s)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--display-startup-timeout', type=int, default=30,
                   help='Private Xvfb startup deadline in seconds (1-60)')
    for key in ('binary', 'fixture', 'output'):
        p.add_argument('--' + key, required=True, type=Path)
    s = Scenario(p.parse_args())
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
