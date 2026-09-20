#!/usr/bin/env python3
"""One isolated journey for Studio previews and explicit Explorer file actions."""
import argparse
import json
import sqlite3
import time
from pathlib import Path

from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection
from native_studio_settings_smoke import mode, tasks
from native_presentation_smoke import resize
from native_rich_text_smoke import clipboard
from native_environment_smoke import choose_tool


def draft(s, task):
    with sqlite3.connect(s.data / 'native-workspace.sqlite3') as db:
        row = db.execute('SELECT data FROM preferences WHERE key=?', ('task-draft:' + task,)).fetchone()
    return json.loads(row[0])['text'] if row else ''


def close(s):
    s.desktop.request_close()
    wait_until(lambda: s.process.poll() is not None, 'guarded application close')
    assert s.process.returncode == 0
    s.log.close()
    s.log = None


def name_action(s, control, name):
    s.click_control(control)
    s.click_control('file-action-name')
    s.desktop.key('a', ('Control_L',))
    s.desktop.text(name)
    s.click_control('file-action-confirm')


def find_studio_file(s, name):
    s.click_control('studio-search')
    s.desktop.key('a', ('Control_L',))
    s.desktop.text(name)
    s.click_control('studio-file', slot=0)


def run(s):
    from PIL import Image
    s.launch()
    ui = s.desktop
    resize(ui, 1440, 940, s.scale)
    initial = selection(s)
    baseline = s.events()
    s.click_control('composer-input')
    ui.text('Keep my draft.')
    s.click_control('Files')
    choose_tool(s, 'Explorer')
    name_action(s, 'folder-new', 'assets')
    wait_until(lambda: (s.project / 'assets').is_dir(), 'create folder')
    name_action(s, 'file-new', 'example.txt')
    created = s.project / 'example.txt'
    wait_until(created.is_file, 'new empty file')
    s.click_control('editor-input')
    ui.text('Example content\nSecond line')
    s.click_control('save-document')
    wait_until(lambda: created.read_text() == 'Example content\nSecond line', 'save created file')
    # Selection-to-draft keeps editor content and the original unsent draft.
    s.click_control('editor-input')
    ui.key('a', ('Control_L',))
    s.click_control('selection-to-chat')
    wait_until(lambda: '> Example content' in draft(s, initial), 'selection appended to draft')
    assert draft(s, initial).startswith('Keep my draft.\n\n')
    assert created.read_text() == 'Example content\nSecond line'
    name_action(s, 'file-rename', 'renamed.txt')
    renamed = s.project / 'renamed.txt'
    wait_until(lambda: renamed.is_file() and not created.exists(), 'rename without content loss')
    s.click_control('file-copy-path')
    assert clipboard(ui) == 'renamed.txt'
    s.click_control('file-to-chat')
    wait_until(lambda: 'Workspace file: "renamed.txt"' in draft(s, initial), 'explicit file reference')
    ui.screenshot('explorer-created-renamed-selection', window_only=True)
    s.checks.append('create-folder-file-save-rename-and-selection-to-draft-preserve-content')

    s.click_control('files-content-search')
    s.click_control('file-content-query')
    ui.text('Example content')
    s.click_control('file-search-run')
    wait_until(lambda: s.control_bounds('file-content-match'), 'real content search result')
    s.click_control('file-content-match', slot=0)
    s.click_control('editor-input')
    assert ui.copy_input() == 'Example content\nSecond line'
    ui.focus()
    s.click_control('file-delete')
    s.click_control('file-action-cancel')
    assert renamed.read_text() == 'Example content\nSecond line'
    # The fixture changes the file after the editor snapshot was read.
    renamed.write_text('Changed outside Synara')
    s.click_control('file-delete')
    s.click_control('file-action-confirm')
    time.sleep(0.5)
    assert renamed.read_text() == 'Changed outside Synara'
    assert s.process.poll() is None
    s.click_control('file-action-cancel')
    s.click_control('file-content-match', slot=0)
    s.click_control('editor-input')
    wait_until(lambda: ui.copy_input() == 'Changed outside Synara', 'reload external modification')
    ui.focus()
    s.click_control('file-delete')
    s.click_control('file-action-confirm')
    wait_until(lambda: not renamed.exists(), 'explicit version-checked deletion')
    assert s.document.read_text() == 'original text\n'
    assert s.events() == baseline
    ui.screenshot('explorer-content-search', window_only=True)
    s.checks.append('literal-content-search-delete-cancel-and-external-change-refusal')

    mode(s, True)
    wait_until(lambda: tasks(s)[selection(s)]['scope'] == 'studio', 'independent Studio chat')
    studio = selection(s)
    root = Path(tasks(s)[studio]['working_directory'])
    (root / 'notes.md').write_text('# Studio output\n\nA saved document.\n')
    (root / 'context').mkdir(exist_ok=True)
    (root / 'context' / 'private.txt').write_text('Input infrastructure is not output')
    Image.new('RGB', (160, 100), (46, 104, 153)).save(root / 'picture.png')
    s.click_control('composer-input')
    ui.text('Unsent Studio text')
    s.click_control('studio-outputs')
    wait_until(lambda: s.control_bounds('studio-files-panel'), 'Studio files surface')
    find_studio_file(s, 'notes.md')
    wait_until(lambda: s.control_bounds('studio-text-preview'), 'native Markdown preview')
    s.click_control('studio-raw-toggle')
    wait_until(lambda: s.control_bounds('studio-raw-preview'), 'raw Markdown source view')
    ui.screenshot('studio-output-raw-text', window_only=True)
    assert (root / 'notes.md').read_text() == '# Studio output\n\nA saved document.\n'
    s.click_control('studio-raw-toggle')
    s.click_control('studio-reference')
    wait_until(lambda: 'Studio workspace file:' in draft(s, studio), 'Studio reference appended')
    assert draft(s, studio).startswith('Unsent Studio text')
    ui.screenshot('studio-output-markdown', window_only=True)
    find_studio_file(s, 'picture.png')
    wait_until(lambda: s.control_bounds('studio-image-preview'), 'contained image preview')
    ui.screenshot('studio-output-image', window_only=True)
    s.click_control('studio-image-in')
    wait_until(lambda: s.control_bounds('studio-zoomed-image'), 'zoomed image geometry')
    assert abs(s.control_bounds('studio-zoomed-image')[2] - 200) < 2
    assert abs(s.control_bounds('studio-zoomed-image')[3] - 125) < 2
    ui.screenshot('studio-output-zoomed', window_only=True)
    s.click_control('studio-image-fit')
    for width in (1100, 960):
        resize(ui, width, 820, s.scale)
        x, _, w, _ = s.control_bounds('studio-image-preview')
        assert x >= 0 and x + w <= width + 1
        ui.screenshot(f'studio-image-{width}', window_only=True)
    assert s.events() == baseline
    s.checks.append('Studio-Markdown-image-preview-and-draft-reference-without-agent-execution')
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == studio and 'Studio workspace file:' in draft(s, studio)
    s.click_control('studio-outputs')
    find_studio_file(s, 'picture.png')
    wait_until(lambda: s.control_bounds('studio-image-preview'), 'image reopened after restart')
    assert s.events() == baseline
    ui.screenshot('studio-files-restored', window_only=True)
    s.checks.append('Studio-files-and-drafts-reopen-after-restart-without-autostart')
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
