#!/usr/bin/env python3
"""Drive the actual GPUI Remote/Files/Terminal UI against ssh_smoke.py's private host."""
import argparse
import json
import os
import re
from pathlib import Path
import sqlite3
import shutil
import subprocess
import time

from native_smoke import Desktop, stop, wait_until


def type_text(ui, value):
    names = {
        ' ': 'space',
        '-': 'minus',
        '.': 'period',
        '/': 'slash',
        '\n': 'Return',
    }
    for char in value:
        if char == '_':
            ui.key('minus', ('Shift_L',))
        elif char.isascii() and char.isupper():
            ui.key(char.lower(), ('Shift_L',))
        else:
            ui.key(names.get(char, char))


def set_field(ui, x, y, value):
    ui.click(x, y)
    ui.key('a', ('Control_L',))
    type_text(ui, value)


def control_bounds(log_path, control, slot=None, enabled=None):
    text = re.sub(r'\x1b\[[0-9;]*[A-Za-z]', '', Path(log_path).read_text(errors='replace'))
    rows = [
        line for line in text.splitlines()
        if 'control-layout' in line
        and ('control="' + control + '"') in line
        and (slot is None or re.search(r'\bslot=' + str(slot) + r'\b', line))
    ]
    if not rows:
        return None
    if enabled is not None and ('enabled=' + str(enabled).lower()) not in rows[-1]:
        return None
    values = dict(re.findall(r'\b(x|y|width|height)=(-?[0-9]+(?:\.[0-9]+)?)', rows[-1]))
    if len(values) != 4:
        return None
    return tuple(float(values[key]) for key in ('x', 'y', 'width', 'height'))


def click_control(ui, log_path, control, slot=None, enabled=None, timeout=20):
    x, y, width, height = wait_until(
        lambda: control_bounds(log_path, control, slot, enabled),
        control + ' remote native geometry/state',
        timeout,
    )
    ui.click_client(round(x + width / 2), round(y + height / 2))


def replace_focused_text(ui, value):
    ui.key('a', ('Control_L',))
    type_text(ui, value)


def choose_environment_tool(ui, log_path, label):
    click_control(ui, log_path, 'environment-add')
    type_text(ui, label)
    ui.key('Return')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    options = parser.parse_args()
    output = options.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    data = output / 'data'
    data.mkdir()
    home = output / 'home'
    home.mkdir()
    runtime = output / 'runtime'
    runtime.mkdir(mode=0o700)
    local_project = output / 'bootstrap-local'
    local_project.mkdir()
    remote_fixture = Path(os.environ['SYNARA_SSH_SMOKE_ROOT']).resolve()
    remote_project = remote_fixture / 'remote-ui-project'
    remote_project.mkdir()
    remote_document = remote_project / 'remote-ui-doc.txt'
    remote_document.write_text('remote original\n', encoding='utf-8')
    terminal_marker = remote_project / 'remote-ui-terminal-marker'
    restart_marker = remote_project / 'remote-ui-terminal-restarted'
    profiles = output / 'profiles.json'
    profiles.write_text(json.dumps([{
        'id': 'remote-smoke',
        'name': 'Remote Smoke Fixture',
        'command': str(options.fixture.resolve()),
        'args': ['--integration-fixture', 'remote-smoke'],
    }]), encoding='utf-8')

    desktop = Desktop(output)
    process = None
    log = (output / 'app.log').open('w')
    checks = []
    try:
        env = {key: os.environ[key] for key in ('PATH', 'LD_LIBRARY_PATH') if key in os.environ}
        env['PATH'] = os.pathsep.join(
            part for part in ('/usr/bin', '/bin', env.get('PATH', '')) if part
        )
        trust = remote_fixture / 'known hosts'
        identity = remote_fixture / 'identity'
        helper = Path(os.environ['SYNARA_REMOTE_FS_HELPER']).resolve()
        for required in (trust, identity, helper):
            assert required.is_file(), f'missing remote smoke input: {required}'
        assert shutil.which('ssh', path=env['PATH']), 'OpenSSH client is absent from native smoke PATH'
        env.update(
            DISPLAY=desktop.name,
            XDG_RUNTIME_DIR=str(runtime),
            HOME=str(home),
            GPUI_PLATFORM='x11',
            LIBGL_ALWAYS_SOFTWARE='1',
            RUST_LOG='synara=info,synara_ui_layout=debug,gpui=warn',
        )
        process = subprocess.Popen([
            str(options.binary.resolve()),
            '--workspace', str(local_project),
            '--data-dir', str(data),
            '--agents', str(profiles),
        ], env=env, cwd=local_project, stdout=log, stderr=subprocess.STDOUT)

        def ready():
            if process.poll() is not None:
                raise RuntimeError('Synara exited before the remote smoke window opened')
            return desktop.find_window()

        wait_until(ready, 'remote native window', 30)
        desktop.focus()
        assert desktop.geometry()[2:] == (1420, 930)

        desktop.key('8', ('Control_L',))
        time.sleep(0.7)
        # Fixed 1420x930 AJM layout. Keep a screenshot before submission for diagnostics.
        set_field(desktop, 520, 200, '127.0.0.1')
        set_field(desktop, 1070, 200, os.environ['SYNARA_SSH_SMOKE_PORT'])
        set_field(desktop, 800, 242, os.environ['SYNARA_SSH_SMOKE_USER'])
        set_field(desktop, 800, 285, str(remote_project))
        set_field(desktop, 800, 328, str(trust))
        set_field(desktop, 800, 370, str(identity))
        set_field(desktop, 800, 413, str(helper))
        desktop.screenshot('remote-enrollment')
        desktop.click(1218, 476)

        database = data / 'native-workspace.sqlite3'
        def enrolled():
            if not database.exists():
                return False
            with sqlite3.connect(database.as_uri() + '?mode=ro', uri=True) as db:
                rows = [row[0] for row in db.execute('SELECT data FROM workspaces')]
            return any(
                json.loads(row).get('location', {}).get('kind') == 'ssh'
                and str(remote_project) in row
                for row in rows
            )

        wait_until(enrolled, 'persisted pinned remote workspace', 20)
        checks.append('remote-panel-pinned-enrollment')

        click_control(desktop, log.name, 'Files', timeout=20)
        choose_environment_tool(desktop, log.name, 'Explorer')
        wait_until(
            lambda: control_bounds(log.name, 'file-tree'),
            'remote file explorer geometry',
            20,
        )
        desktop.screenshot('remote-files')
        click_control(desktop, log.name, 'file-row', slot=0)
        click_control(desktop, log.name, 'editor-input')
        desktop.key('a', ('Control_L',))
        type_text(desktop, 'remote edited\n')
        desktop.key('s', ('Control_L',))
        wait_until(
            lambda: remote_document.read_text(encoding='utf-8') == 'remote edited\n',
            'guarded remote file save',
            20,
        )
        checks.append('remote-files-guarded-save')

        search_document = remote_project / 'remote-search-target.txt'
        search_document.write_text('unique remote ssh search needle\\n', encoding='utf-8')
        desktop.key('f', ('Control_L', 'Shift_L'))
        replace_focused_text(desktop, 'unique remote ssh search needle')
        click_control(desktop, log.name, 'file-content-match', slot=0)
        click_control(desktop, log.name, 'editor-input')
        assert desktop.copy_input() == 'unique remote ssh search needle\\n'
        desktop.key('p', ('Control_L',))
        replace_focused_text(desktop, 'remote-search-target.txt')
        click_control(desktop, log.name, 'file-name-match', slot=0)
        click_control(desktop, log.name, 'editor-input')
        assert desktop.copy_input() == 'unique remote ssh search needle\\n'
        click_control(desktop, log.name, 'file-content-query')
        desktop.key('Escape')
        assert search_document.read_text(encoding='utf-8') == 'unique remote ssh search needle\\n'
        desktop.screenshot('remote-search', window_only=True)
        checks.append('remote-content-and-name-search-over-pinned-ssh')

        choose_environment_tool(desktop, log.name, 'Terminal')
        wait_until(
            lambda: control_bounds(log.name, 'terminal-workspace'),
            'remote terminal workspace geometry',
            20,
        )
        click_control(desktop, log.name, 'start-shell', timeout=20)
        click_control(desktop, log.name, 'terminal-screen', timeout=20)
        type_text(desktop, 'touch remote-ui-terminal-marker\n')
        wait_until(terminal_marker.exists, 'direct remote terminal input', 20)
        checks.append('remote-terminal-direct-input')
        desktop.screenshot('remote-terminal')

        # Restart through the same native terminal surface and prove stale-session isolation.
        # A running terminal requires explicit confirmation before Synara stops its PTY.
        click_control(desktop, log.name, 'start-shell', timeout=20)
        click_control(desktop, log.name, 'terminal-confirm', timeout=20)
        wait_until(
            lambda: not control_bounds(log.name, 'terminal-confirm'),
            'remote terminal restart confirmation to clear',
            20,
        )
        click_control(desktop, log.name, 'terminal-screen', timeout=20)
        type_text(desktop, 'touch remote-ui-terminal-restarted\n')
        wait_until(restart_marker.exists, 'remote terminal restart', 20)
        checks.append('remote-terminal-restart')

        # Close while a foreground command is active. Synara must stop its owned SSH PTY first.
        click_control(desktop, log.name, 'terminal-screen', timeout=20)
        type_text(desktop, 'sleep 30\n')
        time.sleep(0.35)
        desktop.request_close()
        wait_until(lambda: process.poll() is not None, 'terminal-owned native close', 12)
        assert process.returncode == 0
        checks.append('terminal-shutdown-before-window-close')
        (output / 'result.json').write_text(
            json.dumps({'status': 'passed', 'checks': checks}, indent=2) + '\n',
            encoding='utf-8',
        )
        print('PASS: remote native UI smoke: ' + ', '.join(checks))
    except BaseException:
        try:
            if desktop.window:
                desktop.screenshot('failure')
        except Exception:
            pass
        raise
    finally:
        if process is not None:
            stop(process)
        log.close()
        desktop.close()


if __name__ == '__main__':
    main()
