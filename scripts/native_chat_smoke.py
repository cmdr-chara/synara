#!/usr/bin/env python3
"""Compare native transcript, dock and Add menu with the supplied Electron states.

Reference words are inert display data in an isolated, owned SQLite fixture.
No commands or links in the transcript are executed. Live backend operations are
covered separately by native_smoke and native_controls_smoke.
"""
import argparse
import json
from pathlib import Path
import sqlite3
import time
import re
import uuid

from native_smoke import Scenario, wait_until
from native_presentation_smoke import seed_visual_catalog, resize, log_text

ANSWER = '''Sì, si vede eccome quanto è diverso — quello è il nostro lavoro sul web dev, e hai ragione a farlo notare:

- Presets in cima (“Save a model + effort with the star below”) — non esiste nella stabile;
- Current configuration con stellina — salva modello+effort insieme;
- bottone unico “GPT-6 Astra · Ultra” invece dei due pulsanti separati modello/effort della stabile;
- slider nuovo: traccia sottile, fill con gradient, pallino con sheen, step dots — niente glow (tolto come chiesto).

Ho anche killato l’istanza desktop spuria che girava col binario stabile e creava confusione — la stabile vera non l’ho toccata. Scusa per il giro a vuoto sull’AppImage: la verifica che conta l’hai appena fatta tu sullo schermo.'''


def seed_transcript(scenario):
    ui = scenario.desktop
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'close before visual transcript')
    assert scenario.process.returncode == 0
    scenario.log.close()
    scenario.log = None
    (scenario.project / 'outputs').mkdir()
    (scenario.project / 'work').mkdir()
    scenario.document.unlink()  # Owned test fixture only; show the reference's two folders.
    # Exercise the actual provider-to-glyph mapping with explicit provider IDs.
    # These are visual fixture profiles; their commands still point only at the
    # owned ACP fixture, and this scenario never starts a provider session.
    profiles = json.loads(scenario.profiles.read_text())
    profiles.extend(dict(profiles[0], id=provider, name=f'{label} (visual fixture)')
                    for provider, label in [('opencode', 'OpenCode'), ('codex', 'Codex')])
    scenario.profiles.write_text(json.dumps(profiles))
    with sqlite3.connect(scenario.data / 'native-workspace.sqlite3') as db:
        task = json.loads(db.execute('SELECT data FROM tasks ORDER BY updated_ms DESC LIMIT 1').fetchone()[0])
        task.update(title='Check Device Synara Source Build', state='failed', agent_id='opencode')
        duplicate = db.execute('SELECT id FROM tasks WHERE id != ? ORDER BY updated_ms DESC LIMIT 1', (task['id'],)).fetchone()[0]
        db.execute('DELETE FROM tasks WHERE id=?', (duplicate,))
        db.execute('UPDATE tasks SET data=? WHERE id=?', (json.dumps(task), task['id']))
        for row_id, value in db.execute('SELECT id,data FROM tasks WHERE id != ?', (task['id'],)).fetchall():
            row = json.loads(value)
            row['agent_id'] = 'codex' if row['title'] == 'can you summarize' else 'opencode'
            db.execute('UPDATE tasks SET data=? WHERE id=?', (json.dumps(row), row_id))
        events = []
        start = 1789494812000
        def emit(event, offset):
            events.append((task['thread_id'], len(events) + 1, str(uuid.uuid4()), start + offset, json.dumps(event)))
        emit(dict(type='title_changed', title=task['title']), 0)
        emit(dict(type='prompt_started', turn='visual-first'), 0)
        emit(dict(type='text_delta', message_id='visual-user-1', role='user', text='questo è dal localhost web, vedi quanto è diverso?'), 0)
        emit(dict(type='tool_changed', patch=dict(id='visual-read', title='Read source', status='completed', kind='read', output=[dict(kind='text', text='Isolated reference fixture')])), 1000)
        emit(dict(type='text_delta', message_id='visual-answer', role='assistant', text=ANSWER), 88000)
        emit(dict(type='prompt_finished', reason='end_turn'), 88000)
        emit(dict(type='prompt_started', turn='visual-second'), 100000)
        emit(dict(type='text_delta', message_id='visual-user-2', role='user', text='ora potresti aprire la dev build con la pr e slider'), 100000)
        for i in range(4):
            emit(dict(type='tool_changed', patch=dict(id=f'visual-command-{i}', title=f'Command {i + 1}', status='completed', kind='execute', output=[dict(kind='text', text=f'Fixture output {i + 1}')])), 101000 + i * 1000)
        emit(dict(type='error', message='Turn failed', recoverable=False), 105000)
        db.executemany('INSERT INTO events (thread_id,sequence,id,timestamp_ms,data) VALUES (?,?,?,?,?)', events)
    scenario.launch()


def run(scenario):
    scenario.launch()
    seed_visual_catalog(scenario)
    seed_transcript(scenario)
    ui = scenario.desktop
    resize(ui, 1536, 1032 / 1.25, scenario.scale)
    before = scenario.events()
    ui.screenshot('chat-full', window_only=True)
    marker = len(log_text(scenario))
    scenario.click_control('Files', settle=0.45)
    x, _, width, _ = scenario.control_bounds('chat-pane')
    right, _, right_width, _ = scenario.control_bounds('workspace-pane')
    assert abs(x - 256) < 1 and abs(width - 640) < 1
    assert abs(right - 896) < 2 and abs(right_width - 640) < 2
    cx, _, cw, _ = scenario.control_bounds('composer-surface')
    assert abs(cx - 277) < 2 and abs(cw + 2 - 600) < 1
    ui.click_client(1500, 150)
    ui.screenshot('chat-workspace', window_only=True)
    scenario.checks.append('workspace-launcher-keeps-transcript-and-composer-in-equal-left-pane')
    frames = [float(value) for value in re.findall(r'control="workspace-pane"[^\n]*?width=([0-9.]+)', log_text(scenario)[marker:])]
    assert any(0 < width < 640 for width in frames) and abs(frames[-1] - 640) < 1
    scenario.checks.append('workspace-opening-renders-intermediate-drawer-widths')
    scenario.click_control('dock-files')
    tree = wait_until(lambda: scenario.control_bounds('file-tree'), 'native Explorer tree')
    assert abs(tree[2] + 1 - 240) < 1
    ui.screenshot('chat-explorer', window_only=True)
    scenario.checks.append('explorer-has-240px-tree-and-separate-empty-document-surface')
    scenario.click_control('composer-extras', enabled=True)
    _, menu_y, menu_width, menu_height = scenario.control_bounds('session-menu')
    _, composer_y, composer_width, _ = scenario.control_bounds('composer-surface')
    assert abs(menu_width - (composer_width - 10)) < 2
    assert menu_y + menu_height < composer_y
    assert 160 <= menu_height <= 180
    ui.xt.XTestFakeMotionEvent(ui.display, -1, 1500, 150, 0)
    ui.x.XFlush(ui.display)
    time.sleep(0.15)
    ui.screenshot('chat-add-menu', window_only=True)
    ui.key('Escape')
    assert scenario.events() == before
    scenario.checks.append('five-row-add-menu-spans-composer-and-dismisses-without-agent-events')
    scenario.click_control('work-summary', slot=1)
    ui.screenshot('chat-activity-expanded', window_only=True)
    scenario.click_control('work-summary', slot=1)
    assert scenario.events() == before
    scenario.checks.append('activity-expansion-is-local-and-preserves-durable-transcript')
    resize(ui, 1100, 760, scenario.scale)
    scenario.click_control('composer-extras', enabled=True)
    ui.screenshot('chat-add-menu-narrow', window_only=True)
    mx, _, mw, _ = scenario.control_bounds('session-menu')
    assert mx >= 256 and mx + mw <= 1100
    ui.key('Escape')
    scenario.checks.append('split-composer-and-add-menu-remain-inside-intermediate-window')
    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'chat verification shutdown')
    assert scenario.process.returncode == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--scale', type=float, choices=(1, 1.25), default=1.25)
    scenario = Scenario(parser.parse_args())
    result = dict(status='failed', checks=scenario.checks, platform='Linux/X11/private Xvfb', scale=scenario.scale, content='Inert transcript fixture shaped from user-provided reference')
    try:
        run(scenario)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure')
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
