#!/usr/bin/env python3
"""Native immutable PDF pages, Library export review and inline onboarding import.

Uses owned local files and Xvfb. The platform save dialog is deliberately not
counted as accepted by this fixture. Export bytes/failure paths have service tests.
"""
import argparse
import json
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, fresh_probe
from native_navigation_smoke import selection
from native_studio_settings_smoke import tasks
from native_studio_explorer_smoke import find_studio_file
from native_model_draft_smoke import close
from native_project_import_smoke import CODEX, ledger, task_events
from native_presentation_smoke import resize


def reveal(s, control):
    # Scroll over the settings page's right margin, not a single-line editor,
    # which deliberately consumes its own wheel events.
    ui = s.desktop
    for _ in range(24):
        bounds = s.control_bounds(control)
        if bounds and 48 <= bounds[1] * s.scale and (bounds[1] + bounds[3]) * s.scale <= ui.geometry()[3] - 12:
            return
        left, top, width, height = ui.geometry()
        ui.xt.XTestFakeMotionEvent(ui.display, -1, left + width - 32, top + height // 2, 0)
        button = 4 if bounds and bounds[1] < 48 else 5
        for _ in range(3):
            ui.xt.XTestFakeButtonEvent(ui.display, button, 1, 0)
            ui.xt.XTestFakeButtonEvent(ui.display, button, 0, 0)
        ui.x.XFlush(ui.display)
        time.sleep(0.18)
    raise AssertionError(f'Could not reveal {control} from the page margin')


def click(s, control, slot=None):
    reveal(s, control)
    s.click_control(control, slot=slot)


def pdf(s):
    s.launch()
    baseline = s.events()
    resize(s.desktop, 1440, 1040, s.scale)
    s.click_control('command-palette')
    s.desktop.text('New Hub')
    s.desktop.key('Return')
    wait_until(lambda: s.control_bounds('hub-save'), 'New Hub form')
    s.desktop.text('Document review')
    click(s, 'hub-save')
    wait_until(lambda: tasks(s)[selection(s)]['scope'] == 'studio', 'Hub thread')
    click(s, 'hub-home-thread', slot=0)
    task = selection(s)
    root = Path(tasks(s)[task]['working_directory'])
    source = root / 'report.pdf'
    original = (Path(__file__).parent / 'fixtures/parity-document.pdf').read_bytes()
    source.write_bytes(original)
    s.click_control('studio-outputs')
    find_studio_file(s, 'report.pdf')
    wait_until(lambda: s.control_bounds('studio-pdf-page', slot=1), 'first native PDF page')
    s.desktop.screenshot('pdf-first-page', window_only=True)
    click(s, 'studio-pdf-next')
    wait_until(lambda: s.control_bounds('studio-pdf-page', slot=2), 'second native PDF page')
    s.desktop.screenshot('pdf-second-page', window_only=True)
    click(s, 'studio-pdf-in')
    wait_until(lambda: s.control_bounds('studio-pdf-zoomed'), 'PDF zoom')
    assert source.read_bytes() == original and s.events() == baseline
    source.write_bytes(b'%PDF-1.4\nAn externally changed and broken document')
    fresh_probe(s, 'studio-pdf-page', lambda: click(s, 'studio-pdf-previous'))
    # Previous page can still render from the captured PDF despite disk mutation.
    time.sleep(0.4)
    click(s, 'studio-pdf-reload')
    wait_until(lambda: s.control_bounds('studio-error'), 'corrupt PDF reload refusal')
    source.write_bytes(original)
    fresh_probe(s, 'studio-pdf-preview', lambda: find_studio_file(s, 'report.pdf'))
    click(s, 'studio-export-review')
    wait_until(lambda: s.control_bounds('studio-export-confirm'), 'explicit original-file export review')
    assert source.read_bytes() == original and s.events() == baseline
    s.desktop.screenshot('pdf-export-review', window_only=True)
    # Leave the review through ordinary preview selection, without opening a save dialog.
    find_studio_file(s, 'report.pdf')
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == task and s.events() == baseline
    s.click_control('studio-outputs')
    find_studio_file(s, 'report.pdf')
    wait_until(lambda: s.control_bounds('studio-pdf-page', slot=1), 'PDF reopening')
    assert source.read_bytes() == original
    s.checks.append('PDF-native-pages-zoom-immutable-snapshot-corrupt-reload-export-review-and-inert-restart')
    close(s)


def onboarding(s):
    root = s.output / 'history'
    root.mkdir()
    source = root / 'rollout.jsonl'
    rows = [
        {'type': 'session_meta', 'payload': {'id': CODEX, 'cwd': '/metadata-only'}},
        {'type': 'event_msg', 'timestamp': '2025-01-02T03:04:05Z', 'payload': {'type': 'user_message', 'message': 'Imported during onboarding'}},
        {'type': 'event_msg', 'timestamp': '2025-01-02T03:04:06Z', 'payload': {'type': 'agent_message', 'message': 'Visible history answer'}},
    ]
    original = ('\n'.join(map(json.dumps, rows)) + '\n').encode()
    source.write_bytes(original)
    s.launch()
    resize(s.desktop, 1440, 1040, s.scale)
    s.desktop.key('6', ('Control_L',))
    fill(s, 'settings-search', 'Getting started')
    s.click_control('onboarding')
    for _ in range(4):
        click(s, 'onboarding-next')
    click(s, 'onboarding-project-import')
    wait_until(lambda: s.control_bounds('onboarding-history-import'), 'inline import in project step')
    fill(s, 'import-root', str(root))
    for control in ('import-scan', 'import-preview-file', 'import-destination', 'import-agent', 'import-review-destination'):
        click(s, control)
    reveal(s, 'import-confirm')
    assert not ledger(s)['receipts']
    # Continuing setup must not conceal a pending confirmation or commit it.
    click(s, 'onboarding-next')
    reveal(s, 'import-confirm')
    assert not ledger(s)['receipts']
    click(s, 'import-confirm')
    wait_until(lambda: len(ledger(s)['receipts']) == 1, 'inline native history import')
    imported = ledger(s)['receipts'][0]['task']
    _, events, sessions = task_events(s, imported)
    assert sessions == 0 and not any(e['type'] == 'prompt_started' for e in events)
    assert source.read_bytes() == original
    s.desktop.screenshot('onboarding-import-completed', window_only=True)
    click(s, 'onboarding-next')
    s.checks.append('onboarding-inline-import-retains-review-owner-blocks-navigation-and-does-not-start-agent')
    close(s)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('binary', 'fixture', 'output'):
        parser.add_argument('--' + key, required=True, type=Path)
    args = parser.parse_args()
    results = []
    for name, run in [('pdf', pdf), ('onboarding', onboarding)]:
        opts = argparse.Namespace(**vars(args))
        opts.output = args.output / name
        s = Scenario(opts)
        result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb'}
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
            results.append(result)
    (args.output / 'result.json').write_text(json.dumps(results, indent=2) + '\n')
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
