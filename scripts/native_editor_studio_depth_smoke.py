#!/usr/bin/env python3
"""Native opt-in auto-save/conflict handling and read-only still-WebP previews."""
import argparse
import json
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection
from native_studio_settings_smoke import tasks
from native_studio_explorer_smoke import name_action, find_studio_file, close
from native_integrations_smoke import fill, click
from native_environment_smoke import choose_tool
from native_presentation_smoke import resize


def run(s):
    from PIL import Image
    s.launch()
    resize(s.desktop, 1440, 940, s.scale)
    baseline = s.events()
    s.click_control('Files')
    choose_tool(s, 'Explorer')
    name_action(s, 'file-new', 'idle.txt')
    target = s.project / 'idle.txt'
    wait_until(target.is_file, 'new buffer')
    fill(s, 'editor-input', 'Explicitly opted-in edits')
    time.sleep(1.3)
    assert target.read_text() == '', 'auto-save must be off initially'
    click(s, 'editor-autosave')
    wait_until(lambda: target.read_text() == 'Explicitly opted-in edits', 'opt-in idle save')
    fill(s, 'editor-input', 'A newer buffer revision')
    wait_until(lambda: target.read_text() == 'A newer buffer revision', 'later idle save')
    # This is an external filesystem edit, not a synthetic application mutation.
    target.write_text('An external writer owns this revision')
    fill(s, 'editor-input', 'Preserve my conflicting buffer')
    wait_until(lambda: s.control_bounds('editor-conflict-reload'), 'auto-save conflict controls')
    assert target.read_text() == 'An external writer owns this revision'
    s.click_control('editor-input')
    assert s.desktop.copy_input() == 'Preserve my conflicting buffer'
    fill(s, 'editor-input', 'Still unsaved after auto-save stopped')
    time.sleep(1.3)
    assert target.read_text() == 'An external writer owns this revision'
    s.desktop.screenshot('auto-save-conflict-keeps-both-versions', window_only=True)
    # Explicit recovery uses the already shipped version-checked overwrite owner.
    click(s, 'editor-conflict-overwrite')
    wait_until(lambda: target.read_text() == 'Still unsaved after auto-save stopped', 'explicit conflict resolution')
    fill(s, 'editor-input', 'Auto-save remains disabled after recovery')
    time.sleep(1.3)
    assert target.read_text() == 'Still unsaved after auto-save stopped'
    click(s, 'save-document')
    wait_until(lambda: target.read_text() == 'Auto-save remains disabled after recovery', 'explicit save after paused auto-save')
    s.checks.append('auto-save-default-off-idle-save-disk-conflict-preserves-buffer-and-disables-retry')
    # Hubs are explicit workspaces now. Switching navigation modes does not
    # create a thread. Exercise the actual New Hub owner through its palette.
    s.click_control('command-palette')
    s.desktop.text('New Hub')
    s.desktop.key('Return')
    wait_until(lambda: s.control_bounds('hub-save'), 'New Hub form')
    s.desktop.text('Media review')
    click(s, 'hub-save')
    wait_until(lambda: tasks(s)[selection(s)]['scope'] == 'studio', 'Hub thread')
    click(s, 'hub-home-thread', slot=0)
    studio = selection(s)
    root = Path(tasks(s)[studio]['working_directory'])
    image = root / 'still.webp'
    Image.new('RGB', (160, 100), (30, 100, 150)).save(image, 'WEBP', lossless=True)
    original = image.read_bytes()
    s.click_control('studio-outputs')
    find_studio_file(s, 'still.webp')
    wait_until(lambda: s.control_bounds('studio-image-preview'), 'still-WebP native preview')
    click(s, 'studio-images')
    find_studio_file(s, 'still.webp')
    click(s, 'studio-image-in')
    wait_until(lambda: s.control_bounds('studio-zoomed-image'), 'converted WebP zoom')
    assert abs(s.control_bounds('studio-zoomed-image')[2] - 200) < 2
    assert image.read_bytes() == original
    assert s.events() == baseline
    s.desktop.screenshot('studio-webp-preview', window_only=True)
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == studio
    s.click_control('studio-outputs')
    find_studio_file(s, 'still.webp')
    wait_until(lambda: s.control_bounds('studio-image-preview'), 'WebP after restart')
    assert image.read_bytes() == original and s.events() == baseline
    s.checks.append('WebP-native-preview-filter-zoom-and-reopen-never-rewrites-source-or-starts-agent')
    close(s)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for key in ('binary', 'fixture', 'output'):
        p.add_argument('--' + key, type=Path, required=True)
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
