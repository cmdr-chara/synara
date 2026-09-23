#!/usr/bin/env python3
"""Exercise the real GPUI theme editor with an isolated profile and X11 input.

Reference catalog values are data, never desktop commands. This checks the
native controls, imported/shared payloads, paint samples and durable settings.
It does not claim whole-screen visual or full product equivalence.
"""
from __future__ import annotations

import argparse
import copy
import ctypes as C
import json
import os
from pathlib import Path
import subprocess
import time

from native_smoke import Scenario, wait_until
from native_model_draft_smoke import preference
from native_electron_parity_smoke import PAGES, open_page, section_slots, resize, read_log

ROOT = Path(__file__).resolve().parents[1]
CATALOG = json.loads((ROOT / 'crates/synara-workspace/src/settings/theme/catalog.json').read_text())
PREFIX = 'codex-theme-v1:'


def clipboard_environment(ui):
    return {**{key: os.environ[key] for key in ('PATH', 'LD_LIBRARY_PATH') if key in os.environ}, 'DISPLAY': ui.name}


def paste(ui, text):
    subprocess.run(['xclip', '-selection', 'clipboard', '-in'], input=text.encode('utf-8'),
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                   env=clipboard_environment(ui), check=True, timeout=3)
    ui.focus()
    ui.key('a', ('Control_L',))
    ui.key('v', ('Control_L',))


def clipboard(ui):
    result = subprocess.run(['xclip', '-selection', 'clipboard', '-out'],
                            env=clipboard_environment(ui), stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, check=True, timeout=3)
    assert len(result.stdout) <= 65536
    return result.stdout.decode('utf-8')


def move(ui, x, y):
    root_x, root_y, child = C.c_int(), C.c_int(), C.c_ulong()
    assert ui.x.XTranslateCoordinates(ui.display, ui.window, ui.root, 0, 0,
                                      C.byref(root_x), C.byref(root_y), C.byref(child))
    ui.xt.XTestFakeMotionEvent(ui.display, -1, root_x.value + int(x), root_y.value + int(y), 0)
    ui.x.XFlush(ui.display)


def scroll(ui, down, steps=4):
    _, _, width, height = ui.geometry()
    move(ui, width - 60, height // 2)
    for _ in range(steps):
        for pressed in (1, 0):
            ui.xt.XTestFakeButtonEvent(ui.display, 5 if down else 4, pressed, 0)
    ui.x.XFlush(ui.display)
    time.sleep(0.16)


def visible_control(scenario, name, slot=None):
    ui = scenario.desktop
    for _ in range(35):
        bounds = scenario.control_bounds(name, slot=slot)
        _, _, width, height = ui.geometry()
        if bounds:
            x, y, w, h = bounds
            if w > 0 and h > 0 and 60 <= y and y + h < height - 12 and 0 <= x and x + w <= width:
                return bounds
            scroll(ui, down=y >= height - 12, steps=3)
        else:
            scroll(ui, down=True, steps=3)
    raise AssertionError(f'Control did not become visible: {name} slot {slot}')


def click(scenario, name, slot=None, *, enabled=None):
    visible_control(scenario, name, slot)
    scenario.click_control(name, slot=slot, enabled=enabled)


def settings(scenario):
    return preference(scenario, 'settings')


def themes(scenario):
    value = settings(scenario)
    return value and value['appearance'].get('electron_theme')


def wait_pack(scenario, variant, predicate, message):
    return wait_until(lambda: themes(scenario) and predicate(themes(scenario)[variant]), message)


def capture(scenario, name, output, *, expected_accent=None, slot=None):
    from PIL import Image
    ui = scenario.desktop
    _, _, width, height = ui.geometry()
    move(ui, 30, 30)
    ui.screenshot(name, window_only=True)
    path = scenario.output / (name + '.png')
    with Image.open(path) as image:
        assert image.size == (width, height)
        if expected_accent is not None:
            x, y, w, h = scenario.control_bounds('theme-pack-accent', slot=slot)
            # Interior of the opaque color pill, away from border, glyph and text.
            point = (round(x + w - 24), round(y + 5))
            actual = image.convert('RGB').getpixel(point)
            expected = tuple(int(expected_accent[index:index + 2], 16) for index in (1, 3, 5))
            assert all(abs(a - b) <= 1 for a, b in zip(actual, expected)), (name, point, actual, expected)
    output.append({'file': path.name, 'viewport': [width, height], 'scale': 1,
                   'mode': settings(scenario)['appearance']['theme']})


def appearance(scenario):
    open_page(scenario, PAGES[2], section_slots())


def set_mode(scenario, mode):
    appearance(scenario)
    scenario.click_control('theme-' + mode)
    wait_until(lambda: settings(scenario) and settings(scenario)['appearance']['theme'] == mode,
               'persisted ' + mode + ' mode')
    time.sleep(0.15)


def select_preset(scenario, mode, identifier):
    slot = 0 if mode == 'light' else 1
    click(scenario, 'theme-pack-preset', slot)
    # The shared ChoiceMenu focuses its search field when opened.
    label = ' '.join(part.capitalize() for part in identifier.split('-'))
    # Search stable slugs that differ from labels only for these source entries.
    label = {'vscode-plus': 'VS Code Plus', 'codex': 'Codex'}.get(identifier, label)
    scenario.desktop.text(label)
    scenario.desktop.key('Return')
    wait_pack(scenario, mode, lambda pack: pack['codeThemeId'] == identifier,
              mode + ' preset ' + identifier)
    wait_pack(scenario, mode, lambda pack: pack['theme']['accent'] == CATALOG[identifier][mode]['accent'],
              'preset accent persisted')
    visible_control(scenario, 'theme-pack-accent', slot)


def run(scenario, captures):
    scenario.launch()
    ui = scenario.desktop
    scenario.click_control('composer-input')
    ui.text('theme changes must preserve this draft')
    events = scenario.events()
    task = scenario.task()['id']
    ui.key('6', ('Control_L',))
    set_mode(scenario, 'light')
    baseline = copy.deepcopy(settings(scenario))
    assert themes(scenario)['light']['codeThemeId'] == 'codex'
    assert themes(scenario)['dark']['codeThemeId'] == 'codex'
    catalog_variants = sum(len(variants) for variants in CATALOG.values())
    observed_variants = 0
    for mode in ('light', 'dark'):
        set_mode(scenario, mode)
        slot = 0 if mode == 'light' else 1
        for identifier, variants in CATALOG.items():
            if mode not in variants:
                continue
            select_preset(scenario, mode, identifier)
            capture(scenario, f'theme-{mode}-{identifier}', captures,
                    expected_accent=variants[mode]['accent'], slot=slot)
            assert scenario.task()['id'] == task and scenario.events() == events
            observed_variants += 1
        select_preset(scenario, mode, 'codex')
    assert observed_variants == catalog_variants
    scenario.checks.append(f'all-{catalog_variants}-supported-reference-preset-variants-select-persist-and-paint-their-real-accent')

    slot = 1
    click(scenario, 'theme-pack-accent', slot)
    wait_until(lambda: scenario.control_bounds('theme-color-hex'), 'native HSV editor')
    scenario.click_control('theme-color-hex')
    paste(ui, '#1A2B3C')
    wait_pack(scenario, 'dark', lambda pack: pack['theme']['accent'] == '#1a2b3c', 'debounced valid hexadecimal color')
    capture(scenario, 'theme-native-hsv-picker', captures)
    scenario.click_control('theme-color-hex')
    paste(ui, '#not-a-color')
    time.sleep(0.3)
    assert themes(scenario)['dark']['theme']['accent'] == '#1a2b3c'
    ui.key('Escape')
    scenario.checks.append('native-hex-editor-commits-valid-color-and-rejects-invalid-drafts')

    click(scenario, 'theme-pack-accent', slot)
    scenario.click_control('theme-color-saturation')
    ui.key('Right')
    ui.key('Up')
    ui.key('Escape')
    wait_pack(scenario, 'dark', lambda pack: pack['theme']['accent'] != '#1a2b3c', 'keyboard HSV edit flushed on dismissal')
    scenario.checks.append('native-hsv-keyboard-edit-flushes-on-popup-dismissal')

    click(scenario, 'theme-dark-contrast')
    ui.key('Home')
    for _ in range(7):
        ui.key('Right')
    wait_pack(scenario, 'dark', lambda pack: pack['theme']['contrast'] == 7, 'last rapid contrast edit persisted')
    capture(scenario, 'theme-contrast-keyboard', captures)
    scenario.checks.append('contrast-keyboard-updates-coalesce-to-the-last-durable-value')

    click(scenario, 'theme-pack-ui-font', slot)
    paste(ui, 'Liberation Sans')
    wait_pack(scenario, 'dark', lambda pack: pack['theme']['fonts']['ui'] == 'Liberation Sans', 'UI font draft persistence')
    click(scenario, 'theme-pack-code-font', slot)
    paste(ui, 'JetBrains Mono')
    wait_pack(scenario, 'dark', lambda pack: pack['theme']['fonts']['code'] == 'JetBrains Mono', 'code font persistence')
    scenario.checks.append('per-variant-font-fields-save-through-the-owned-settings-writer')

    appearance(scenario)
    click(scenario, 'theme-pack-copy', slot)
    shared = clipboard(ui)
    assert shared.startswith(PREFIX)
    payload = json.loads(shared[len(PREFIX):])
    assert payload['variant'] == 'dark'
    assert payload['theme'] == themes(scenario)['dark']['theme']
    click(scenario, 'theme-pack-import', slot)
    scenario.click_control('theme-import-text')
    wrong = {**payload, 'variant': 'light'}
    before = copy.deepcopy(settings(scenario))
    paste(ui, PREFIX + json.dumps(wrong))
    scenario.click_control('theme-import-confirm', enabled=True)
    time.sleep(0.3)
    assert settings(scenario) == before
    capture(scenario, 'theme-import-variant-error', captures)
    scenario.click_control('theme-import-cancel')
    scenario.checks.append('wrong-variant-import-never-mutates-any-persisted-settings')

    click(scenario, 'theme-pack-import', slot)
    scenario.click_control('theme-import-text')
    imported = copy.deepcopy(payload)
    imported['theme']['contrast'] = 23
    imported['theme']['accent'] = '#345678'
    paste(ui, PREFIX + json.dumps(imported))
    scenario.click_control('theme-import-confirm', enabled=True)
    wait_pack(scenario, 'dark', lambda pack: pack['theme']['contrast'] == 23 and pack['theme']['accent'] == '#345678', 'valid shared-theme import')
    assert themes(scenario)['light'] == before['appearance']['electron_theme']['light']
    scenario.checks.append('shared-theme-import-updates-only-the-matching-variant')
    visible_control(scenario, 'theme-pack-accent', slot)
    capture(scenario, 'theme-imported-dark', captures, expected_accent='#345678', slot=slot)

    click(scenario, 'theme-pack-reset', slot)
    wait_pack(scenario, 'dark', lambda pack: pack['codeThemeId'] == 'codex' and pack['theme'] == CATALOG['codex']['dark'], 'reset exact Codex pack')
    assert themes(scenario)['light'] == before['appearance']['electron_theme']['light']
    scenario.checks.append('reset-restores-exact-reference-default-without-changing-the-other-slot')

    resize(ui, 960, 700)
    set_mode(scenario, 'light')
    capture(scenario, 'theme-appearance-light-960', captures)
    set_mode(scenario, 'dark')
    click(scenario, 'theme-pack-preset', 1)
    capture(scenario, 'theme-preset-menu-960', captures)
    ui.key('Escape')
    click(scenario, 'theme-pack-import', 1)
    capture(scenario, 'theme-import-dialog-960', captures)
    ui.key('Tab')
    ui.key('Tab', ('Shift_L',))
    ui.key('Escape')
    scenario.checks.append('theme-popup-layout-and-keyboard-dismissal-at-intermediate-width')

    current = settings(scenario)
    for key in baseline:
        if key != 'appearance':
            assert current[key] == baseline[key], 'Theme editing changed unrelated settings: ' + key
    assert current['appearance']['personalization'] == baseline['appearance']['personalization']
    assert scenario.events() == events and scenario.task()['id'] == task
    persisted = copy.deepcopy(current)
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'theme session shutdown')
    assert scenario.process.returncode == 0
    scenario.launch()
    assert settings(scenario) == persisted
    scenario.click_control('composer-input')
    assert ui.copy_input() == 'theme changes must preserve this draft'
    assert scenario.events() == events
    scenario.checks.append('all-theme-edits-survive-relaunch-with-draft-task-and-permission-state-preserved')
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'final theme session shutdown')
    assert scenario.process.returncode == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'captures': [],
              'reference_revision': 'eaa61eded31b6755d4f30ba8eabc5d905cf817cb',
              'scope': 'Native theme functionality, actual accent paint, persistence and ownership. Not full visual or feature equivalence.'}
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
