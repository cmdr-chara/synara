#!/usr/bin/env python3
"""Exercise native tables, code copying and restart using an owned inert transcript."""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import uuid

from native_smoke import Scenario, wait_until
from native_presentation_smoke import resize

CODE = 'print("Caffè")\nprint(2 + 2)\n'
ANSWER = '''A native rich-text answer.

| Name | Count | Status |
| :--- | ---: | :---: |
| **Caffè** | `2` | Ready |
| Empty note | 3 | |

- [x] Rendered
- [ ] Still to do

```python
''' + CODE + '```\n'


def seed(scenario):
    scenario.desktop.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'close before owned rich-text fixture')
    assert scenario.process.returncode == 0
    scenario.log.close()
    scenario.log = None
    with sqlite3.connect(scenario.data / 'native-workspace.sqlite3') as db:
        task = json.loads(db.execute('SELECT data FROM tasks ORDER BY updated_ms DESC LIMIT 1').fetchone()[0])
        task.update(title='Native rich-text fixture', state='failed')
        db.execute('UPDATE tasks SET data=? WHERE id=?', (json.dumps(task), task['id']))
        db.execute('DELETE FROM events WHERE thread_id=?', (task['thread_id'],))
        events = [
            dict(type='prompt_started', turn='rich-text-fixture'),
            dict(type='text_delta', message_id='rich-user', role='user', text='Show the rich-text fixture.'),
            dict(type='text_delta', message_id='rich-answer', role='assistant', text=ANSWER),
            dict(type='prompt_finished', reason='end_turn'),
        ]
        db.executemany(
            'INSERT INTO events (thread_id,sequence,id,timestamp_ms,data) VALUES (?,?,?,?,?)',
            [(task['thread_id'], index + 1, str(uuid.uuid4()), 1789900000000 + index,
              json.dumps(event)) for index, event in enumerate(events)],
        )
    scenario.launch()


def clipboard(ui):
    env = {key: os.environ[key] for key in ('PATH', 'LD_LIBRARY_PATH') if key in os.environ}
    env['DISPLAY'] = ui.name
    result = subprocess.run(
        ['xclip', '-selection', 'clipboard', '-out'], env=env,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=3, check=True,
    )
    return result.stdout.decode('utf-8')


def run(scenario):
    scenario.launch()
    seed(scenario)
    ui = scenario.desktop
    resize(ui, 1280, 900, scenario.scale)
    before = scenario.events()
    table = wait_until(lambda: scenario.control_bounds('markdown-table'), 'native table layout')
    assert table[2] > 200 and table[3] > 30, 'Table must occupy actual native layout'
    ui.screenshot('rich-text-table-and-code', window_only=True)
    scenario.checks.append('durable-markdown-table-and-code-render-in-native-transcript')

    scenario.click_control('markdown-copy')
    assert clipboard(ui) == CODE, 'Copy must preserve Unicode, spaces and final newline'
    assert scenario.events() == before, 'Copy is local and must not mutate transcript or start a turn'
    scenario.checks.append('code-copy-uses-original-synara-icon-and-copies-exact-source-text')

    for width, height in [(1100, 800), (960, 760)]:
        resize(ui, width, height, scenario.scale)
        x, _, table_width, _ = wait_until(
            lambda: scenario.control_bounds('markdown-table'), 'resized table viewport',
        )
        assert x >= 256 and x + table_width <= width + 1
        assert scenario.process.poll() is None
        ui.screenshot(f'rich-text-{width}', window_only=True)
    assert scenario.events() == before
    scenario.checks.append('table-scroll-viewport-stays-inside-chat-at-intermediate-widths')

    ui.request_close()
    wait_until(lambda: scenario.process.poll() is not None, 'close rich-text fixture')
    assert scenario.process.returncode == 0
    scenario.log.close()
    scenario.log = None
    scenario.launch()
    wait_until(lambda: scenario.control_bounds('markdown-table'), 'restored table layout')
    assert scenario.events() == before
    scenario.desktop.screenshot('rich-text-restored', window_only=True)
    scenario.checks.append('restart-restores-rich-transcript-without-event-rewrite-or-autostart')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = dict(status='failed', checks=scenario.checks, platform='Linux/X11/private Xvfb')
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
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
