#!/usr/bin/env python3
"""Exercise debounced Explorer search and narrow-pane pointer hit targets in GPUI."""
import argparse
import json
import time
import traceback
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, fresh_probe
from native_environment_smoke import choose_tool
from native_presentation_smoke import resize
from native_studio_explorer_smoke import close


def run(s):
    (s.project / 'other.txt').write_text('A unique second needle\n')
    s.launch()
    resize(s.desktop, 1440, 940, s.scale)
    events = s.events()
    s.click_control('Files')
    choose_tool(s, 'Explorer')
    s.click_control('files-content-search')
    fresh_probe(s, 'file-content-match', lambda: fill(s, 'file-content-query', 'original text'))
    row = s.control_bounds('file-content-match', slot=0)
    tree = s.control_bounds('file-tree')
    assert row[1] >= tree[1] and row[1] + row[3] < tree[1] + tree[3] - 24, (tree, row)
    s.desktop.screenshot('live-search-visible-pointer-target', window_only=True)
    s.click_control('file-content-match', slot=0)
    wait_until(lambda: s.control_bounds('editor-input'), 'result opened without Search or Enter')
    s.click_control('editor-input')
    assert s.desktop.copy_input() == 'original text\n'
    fresh_probe(s, 'file-content-match', lambda: fill(s, 'file-content-query', 'unique second needle'))
    s.click_control('file-content-match', slot=0)
    s.click_control('editor-input')
    assert s.desktop.copy_input() == 'A unique second needle\n'
    s.click_control('file-content-query')
    s.desktop.key('Escape')
    wait_until(lambda: abs(s.control_bounds('file-tree')[3] - 185) < 2, 'search collapsed')
    # Ctrl+P changes both the search kind and tool layout; retire old hit bounds.
    fresh_probe(s, 'file-content-query', lambda: s.desktop.key('p', ('Control_L',)))
    fresh_probe(s, 'file-name-match', lambda: fill(s, 'file-content-query', 'other.txt'))
    s.click_control('file-name-match', slot=0)
    s.click_control('editor-input')
    assert s.desktop.copy_input() == 'A unique second needle\n'
    fill(s, 'file-content-query', 'never-matching-value')
    s.desktop.key('Escape')
    time.sleep(0.6)
    assert abs(s.control_bounds('file-tree')[3] - 185) < 2
    assert s.events() == events
    assert s.document.read_text() == 'original text\n'
    s.checks.append('local-live-name-content-search-visible-mouse-targets-query-replacement-escape-and-no-writes')
    close(s)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--fixture', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.set_defaults(scale=1.0, display_startup_timeout=30)
    s = Scenario(p.parse_args())
    result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb'}
    try:
        run(s)
        result['status'] = 'passed'
    except BaseException:
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
