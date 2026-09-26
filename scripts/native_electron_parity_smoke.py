#!/usr/bin/env python3
"""Capture every primary native Settings page against the synced Electron taxonomy.

Uses actual GPUI input/rendering on a private Xvfb display, an owned Git fixture,
and a fresh profile. It does not execute an agent or mutate the application DB.
Rendered navigation coverage is not a claim of feature or pixel equivalence.
"""
from __future__ import annotations

import argparse
import ctypes as C
import json
from pathlib import Path
import re
import subprocess
import time

from native_smoke import Scenario, wait_until
from native_model_draft_smoke import preference

PAGES = [
    ('General', 'General', 'general'),
    ('Profile', 'Profile', 'profile'),
    ('Appearance', 'Appearance', 'appearance'),
    ('Notifications', 'Notifications', 'notifications'),
    ('Behavior', 'Chat behavior', 'behavior'),
    ('Keybindings', 'Keybindings', 'shortcuts'),
    ('Usage', 'Usage', 'usage'),
    ('AppSnap', 'AppSnap', 'appsnap'),
    ('Computer', 'Computer use', 'computer'),
    ('Mcp', 'MCP connections', 'integrations'),
    ('Providers', 'Agent providers', 'providers'),
    ('Models', 'custom model slugs', 'models'),
    ('Skills', 'Agent skills', 'skills'),
    ('Worktrees', 'Managed worktrees', 'worktrees'),
    ('System', 'System tools', 'advanced'),
    ('Archived', 'Archived threads', 'archived'),
]
EXTENSIONS = [
    ('ProjectImport', 'Project import', 'project-import'),
    ('DirectModels', 'Direct models', 'direct-models'),
    ('Device', 'Device / capture', 'device'),
    ('Plugins', 'Plugins', 'plugins'),
    ('Privacy', 'Privacy', 'privacy'),
]


def read_log(scenario: Scenario) -> str:
    return re.sub(r'\x1b\[[0-9;]*[A-Za-z]', '', Path(scenario.log.name).read_text(errors='replace'))


def section_slots() -> dict[str, int]:
    source = (Path(__file__).resolve().parents[1] / 'crates/synara-app/src/shell/settings.rs').read_text()
    body = source.split('enum Section {', 1)[1].split('}', 1)[0]
    variants = re.findall(r'^\s*(\w+),\s*


def open_page(scenario: Scenario, page, slots):
    variant, query, _ = page
    ui = scenario.desktop
    scenario.click_control('settings-search')
    ui.key('a', ('Control_L',))
    # slash has a named X11 keysym, unlike ordinary alphanumeric characters.
    for part_index, part in enumerate(query.split('/')):
        if part_index:
            ui.key('slash')
        ui.text(part)
    cursor = len(read_log(scenario))
    ui.key('Return')
    expected = slots[variant]
    def observed():
        lines = [line for line in read_log(scenario)[cursor:].splitlines()
                 if 'control="settings-page"' in line
                 and re.search(r'\bslot=' + str(expected) + r'\b', line)]
        if not lines:
            return None
        values = dict(re.findall(r'\b(x|y|width|height)=(-?[0-9]+(?:\.[0-9]+)?)', lines[-1]))
        return values if len(values) == 4 else None
    return wait_until(observed, variant + ' fresh native Settings render')


def resize(ui, width: int, height: int):
    ui.x.XResizeWindow.argtypes = [C.c_void_p, C.c_ulong, C.c_uint, C.c_uint]
    ui.x.XResizeWindow.restype = C.c_int
    ui.x.XResizeWindow(ui.display, ui.window, width, height)
    ui.x.XFlush(ui.display)
    wait_until(lambda: ui.geometry()[2:] == (width, height), 'private native resize')
    time.sleep(0.2)


def init_repository(scenario: Scenario):
    def git(*args):
        subprocess.run(['git', '-C', str(scenario.project), *args], check=True,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=15)
    git('init', '--initial-branch=main')
    git('config', 'user.name', 'Synara parity fixture')
    git('config', 'user.email', 'native-parity@example.invalid')
    git('add', 'document.txt')
    git('-c', 'core.hooksPath=/dev/null', 'commit', '-m', 'Fixture')
    worktree = scenario.output / 'worktree-fixture'
    git('-c', 'core.hooksPath=/dev/null', 'worktree', 'add', '-b', 'parity-fixture', str(worktree))
    return worktree


def run(scenario: Scenario, captures: list[dict]):
    from PIL import Image
    worktree = init_repository(scenario)
    scenario.launch()
    ui = scenario.desktop
    slots = section_slots()
    scenario.click_control('composer-input')
    ui.text('parity unsent draft')
    before = scenario.events()
    task = scenario.task()['id']
    ui.key('6', ('Control_L',))
    for width, height in [(1420, 930), (960, 700)]:
        resize(ui, width, height)
        for theme in ('light', 'dark'):
            open_page(scenario, PAGES[2], slots)
            scenario.click_control('theme-' + theme)
            wait_until(lambda: (preference(scenario, 'settings') or {}).get('appearance', {}).get('theme') == theme,
                       'persisted theme ' + theme)
            for page in PAGES:
                bounds = open_page(scenario, page, slots)
                assert scenario.process.poll() is None
                assert scenario.events() == before, 'Settings navigation started or changed an agent turn'
                assert scenario.task()['id'] == task
                assert float(bounds['width']) > 0 and float(bounds['height']) > 0
                name = f'settings-{page[2]}-{theme}-{width}'
                ui.screenshot(name, window_only=True)
                with Image.open(scenario.output / (name + '.png')) as image:
                    assert image.size == (width, height)
                captures.append({'file': name + '.png', 'section': page[2], 'theme': theme,
                                 'viewport': [width, height], 'scale': 1, 'bounds': bounds})
    scenario.checks.append('all-sixteen-primary-settings-pages-render-at-two-widths-in-both-themes')

    resize(ui, 1420, 930)
    for page in EXTENSIONS:
        open_page(scenario, page, slots)
        name = 'native-extension-' + page[2]
        ui.screenshot(name, window_only=True)
        captures.append({'file': name + '.png', 'section': page[2], 'theme': 'dark',
                         'viewport': [1420, 930], 'scale': 1})
    scenario.checks.append('all-five-native-extensions-remain-searchable-without-cluttering-primary-navigation')

    open_page(scenario, PAGES[13], slots)
    wait_until(lambda: scenario.control_bounds('repo-remove-worktree', slot=1), 'real worktree from the owned Git repository')
    assert worktree.is_dir(), 'Browsing worktrees removed a worktree'
    scenario.checks.append('worktree-settings-use-the-existing-git-owner-and-never-delete-on-open')
    open_page(scenario, PAGES[7], slots)
    scenario.click_control('settings-appsnap-open', enabled=True)
    wait_until(lambda: scenario.control_bounds('appsnap-panel'), 'existing task-owned AppSnap surface')
    assert scenario.events() == before
    ui.screenshot('settings-appsnap-opens-existing-capture-owner', window_only=True)
    captures.append({'file': 'settings-appsnap-opens-existing-capture-owner.png', 'section': 'appsnap',
                     'theme': 'dark', 'viewport': [1420, 930], 'scale': 1})
    scenario.checks.append('appsnap-settings-open-the-real-capture-owner-without-capturing-or-sending')
    scenario.click_control('appsnap-close')
    scenario.click_control('composer-input')
    assert ui.copy_input() == 'parity unsent draft'
    assert scenario.events() == before
    scenario.checks.append('settings-browsing-preserves-the-unsent-draft-and-agent-event-history')
    settings = preference(scenario, 'settings')
    assert settings['appearance']['fonts']['ui_size'] == 13.0
    assert settings['appearance']['fonts']['code_size'] == 12.0
    assert settings['appearance']['personalization']['terminal_font_size'] == 12
    scenario.checks.append('new-profile-typography-defaults-match-the-pinned-electron-reference')
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'owned parity session shutdown')
    assert scenario.process.returncode == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'captures': [],
              'reference_revision': 'eaa61eded31b6755d4f30ba8eabc5d905cf817cb',
              'scope': 'Rendered navigation, settings persistence, ownership and geometry metadata. Not pixel-equivalence or full feature parity.'}
    try:
        run(scenario, result['captures'])
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure', window_only=True)
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
, body, re.M)
    required = {page[0] for page in PAGES + EXTENSIONS}
    assert len(variants) == len(set(variants)), variants
    assert required <= set(variants), (sorted(required - set(variants)), variants)
    return {name: index for index, name in enumerate(variants)}


def open_page(scenario: Scenario, page, slots):
    variant, query, _ = page
    ui = scenario.desktop
    scenario.click_control('settings-search')
    ui.key('a', ('Control_L',))
    # slash has a named X11 keysym, unlike ordinary alphanumeric characters.
    for part_index, part in enumerate(query.split('/')):
        if part_index:
            ui.key('slash')
        ui.text(part)
    cursor = len(read_log(scenario))
    ui.key('Return')
    expected = slots[variant]
    def observed():
        lines = [line for line in read_log(scenario)[cursor:].splitlines()
                 if 'control="settings-page"' in line
                 and re.search(r'\bslot=' + str(expected) + r'\b', line)]
        if not lines:
            return None
        values = dict(re.findall(r'\b(x|y|width|height)=(-?[0-9]+(?:\.[0-9]+)?)', lines[-1]))
        return values if len(values) == 4 else None
    return wait_until(observed, variant + ' fresh native Settings render')


def resize(ui, width: int, height: int):
    ui.x.XResizeWindow.argtypes = [C.c_void_p, C.c_ulong, C.c_uint, C.c_uint]
    ui.x.XResizeWindow.restype = C.c_int
    ui.x.XResizeWindow(ui.display, ui.window, width, height)
    ui.x.XFlush(ui.display)
    wait_until(lambda: ui.geometry()[2:] == (width, height), 'private native resize')
    time.sleep(0.2)


def init_repository(scenario: Scenario):
    def git(*args):
        subprocess.run(['git', '-C', str(scenario.project), *args], check=True,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=15)
    git('init', '--initial-branch=main')
    git('config', 'user.name', 'Synara parity fixture')
    git('config', 'user.email', 'native-parity@example.invalid')
    git('add', 'document.txt')
    git('-c', 'core.hooksPath=/dev/null', 'commit', '-m', 'Fixture')
    worktree = scenario.output / 'worktree-fixture'
    git('-c', 'core.hooksPath=/dev/null', 'worktree', 'add', '-b', 'parity-fixture', str(worktree))
    return worktree


def run(scenario: Scenario, captures: list[dict]):
    from PIL import Image
    worktree = init_repository(scenario)
    scenario.launch()
    ui = scenario.desktop
    slots = section_slots()
    scenario.click_control('composer-input')
    ui.text('parity unsent draft')
    before = scenario.events()
    task = scenario.task()['id']
    ui.key('6', ('Control_L',))
    for width, height in [(1420, 930), (960, 700)]:
        resize(ui, width, height)
        for theme in ('light', 'dark'):
            open_page(scenario, PAGES[2], slots)
            scenario.click_control('theme-' + theme)
            wait_until(lambda: (preference(scenario, 'settings') or {}).get('appearance', {}).get('theme') == theme,
                       'persisted theme ' + theme)
            for page in PAGES:
                bounds = open_page(scenario, page, slots)
                assert scenario.process.poll() is None
                assert scenario.events() == before, 'Settings navigation started or changed an agent turn'
                assert scenario.task()['id'] == task
                assert float(bounds['width']) > 0 and float(bounds['height']) > 0
                name = f'settings-{page[2]}-{theme}-{width}'
                ui.screenshot(name, window_only=True)
                with Image.open(scenario.output / (name + '.png')) as image:
                    assert image.size == (width, height)
                captures.append({'file': name + '.png', 'section': page[2], 'theme': theme,
                                 'viewport': [width, height], 'scale': 1, 'bounds': bounds})
    scenario.checks.append('all-sixteen-primary-settings-pages-render-at-two-widths-in-both-themes')

    resize(ui, 1420, 930)
    for page in EXTENSIONS:
        open_page(scenario, page, slots)
        name = 'native-extension-' + page[2]
        ui.screenshot(name, window_only=True)
        captures.append({'file': name + '.png', 'section': page[2], 'theme': 'dark',
                         'viewport': [1420, 930], 'scale': 1})
    scenario.checks.append('all-five-native-extensions-remain-searchable-without-cluttering-primary-navigation')

    open_page(scenario, PAGES[13], slots)
    wait_until(lambda: scenario.control_bounds('repo-remove-worktree', slot=1), 'real worktree from the owned Git repository')
    assert worktree.is_dir(), 'Browsing worktrees removed a worktree'
    scenario.checks.append('worktree-settings-use-the-existing-git-owner-and-never-delete-on-open')
    open_page(scenario, PAGES[7], slots)
    scenario.click_control('settings-appsnap-open', enabled=True)
    wait_until(lambda: scenario.control_bounds('appsnap-panel'), 'existing task-owned AppSnap surface')
    assert scenario.events() == before
    ui.screenshot('settings-appsnap-opens-existing-capture-owner', window_only=True)
    captures.append({'file': 'settings-appsnap-opens-existing-capture-owner.png', 'section': 'appsnap',
                     'theme': 'dark', 'viewport': [1420, 930], 'scale': 1})
    scenario.checks.append('appsnap-settings-open-the-real-capture-owner-without-capturing-or-sending')
    scenario.click_control('appsnap-close')
    scenario.click_control('composer-input')
    assert ui.copy_input() == 'parity unsent draft'
    assert scenario.events() == before
    scenario.checks.append('settings-browsing-preserves-the-unsent-draft-and-agent-event-history')
    settings = preference(scenario, 'settings')
    assert settings['appearance']['fonts']['ui_size'] == 13.0
    assert settings['appearance']['fonts']['code_size'] == 12.0
    assert settings['appearance']['personalization']['terminal_font_size'] == 12
    scenario.checks.append('new-profile-typography-defaults-match-the-pinned-electron-reference')
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'owned parity session shutdown')
    assert scenario.process.returncode == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'captures': [],
              'reference_revision': 'eaa61eded31b6755d4f30ba8eabc5d905cf817cb',
              'scope': 'Rendered navigation, settings persistence, ownership and geometry metadata. Not pixel-equivalence or full feature parity.'}
    try:
        run(scenario, result['captures'])
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure', window_only=True)
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
