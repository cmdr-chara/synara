#!/usr/bin/env python3
"""Drive the actual GPUI Remote/Files/Terminal UI against ssh_smoke.py's private host."""
import argparse
import json
import os
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
        ui.key(names.get(char, char))


def set_field(ui, x, y, value):
    ui.click(x, y)
    ui.key('a', ('Control_L',))
    type_text(ui, value)


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
            RUST_LOG='synara=info,gpui=warn',
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

        # Text fields consume some control-number combinations through IME/focus handling.
        # Click the actual navigation tab so this smoke exercises the intended panel.
        desktop.click(785, 5)
        time.sleep(1.1)
        desktop.screenshot('remote-files')
        desktop.click(310, 170)
        time.sleep(0.6)
        desktop.click(850, 190)
        desktop.key('a', ('Control_L',))
        type_text(desktop, 'remote edited\n')
        desktop.key('s', ('Control_L',))
        wait_until(
            lambda: remote_document.read_text(encoding='utf-8') == 'remote edited\n',
            'guarded remote file save',
            20,
        )
        checks.append('remote-files-guarded-save')

        desktop.click(954, 5)
        time.sleep(0.7)
        desktop.click(1040, 82)
        time.sleep(1.2)
        desktop.click(790, 300)
        type_text(desktop, 'touch remote-ui-terminal-marker\n')
        wait_until(terminal_marker.exists, 'direct remote terminal input', 20)
        checks.append('remote-terminal-direct-input')
        desktop.screenshot('remote-terminal')

        # Restart through the same native terminal surface and prove stale-session isolation.
        desktop.click(1040, 82)
        time.sleep(1.2)
        desktop.click(790, 300)
        type_text(desktop, 'touch remote-ui-terminal-restarted\n')
        wait_until(restart_marker.exists, 'remote terminal restart', 20)
        checks.append('remote-terminal-restart')

        # Close while a foreground command is active. Synara must stop its owned SSH PTY first.
        desktop.click(790, 300)
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
