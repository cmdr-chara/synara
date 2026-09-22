#!/usr/bin/env python3
"""Read actual compiled version, dismiss and restore; no fabricated release feed."""
import argparse
import json
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, click
from native_navigation_smoke import selection
from native_model_draft_smoke import preference, close
from native_project_import_smoke import task_events
from native_presentation_smoke import resize

KEY = 'native-version-history'

def open_releases(s):
    s.click_control('command-palette')
    s.desktop.text('Releases')
    s.desktop.key('Return')
    wait_until(lambda: s.control_bounds('native-build-version'), 'real compiled version surface')

def run(s):
    s.launch()
    task = selection(s)
    baseline = task_events(s, task)
    fill(s, 'composer-input', 'Retain this independent draft')
    wait_until(lambda: preference(s, KEY), 'compiled version observed')
    history = preference(s, KEY)
    assert len(history['visits']) == 1 and not history['visits'][0]['read']
    # Expected version comes from the authoritative manifest used to compile this binary.
    import tomllib
    compiled = tomllib.loads((Path(__file__).resolve().parents[1]/'Cargo.toml').read_text())['workspace']['package']['version']
    assert history['visits'][0]['version'] == compiled
    click(s, 'native-build-open')
    wait_until(lambda: s.control_bounds('native-update-unconfigured'), 'honest unavailable production updater')
    assert task_events(s, task) == baseline and selection(s) == task
    s.desktop.screenshot('native-release-state', window_only=True)
    s.checks.append('compiled-version-and-unconfigured-updater-visible-without-network-or-task-mutation')
    click(s, 'native-build-read')
    wait_until(lambda: preference(s, KEY)['visits'][-1]['read'], 'read state persisted')
    saved = preference(s, KEY)
    s.desktop.key('1', ('Control_L',))
    close(s)
    s.launch(preserve_selection=True)
    assert preference(s, KEY) == saved and task_events(s, task) == baseline
    s.click_control('composer-input')
    assert s.desktop.copy_input() == 'Retain this independent draft'
    open_releases(s)
    assert preference(s, KEY)['visits'][-1]['read']
    s.checks.append('dismissal-restart-and-explicit-reopen-preserve-read-state-task-and-draft')
    resize(s.desktop, 800, 640)
    open_releases(s)
    wait_until(lambda: s.control_bounds('native-build-version'), 'narrow release surface')
    s.desktop.screenshot('native-release-narrow', window_only=True)
    assert task_events(s, task) == baseline
    s.checks.append('narrow-native-release-surface-remains-accessible')
    close(s)

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--fixture', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    s = Scenario(p.parse_args())
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
        (s.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result, indent=2))

if __name__ == '__main__':
    main()
