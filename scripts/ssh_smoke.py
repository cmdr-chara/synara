#!/usr/bin/env python3
"""Exercise native SSH transport and ACP using a disposable loopback-only server.

No user SSH configuration, known-hosts database or authorized_keys file is edited.
Keys are temporary, generated only for this fixture and never uploaded as artifacts.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import pwd
import shutil
import socket
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def run(args: list[str], **kwargs) -> None:
    subprocess.run(args, check=True, timeout=600, **kwargs)


def config_path(path: Path) -> str:
    value = str(path)
    if any(ord(char) < 32 or char in '\\"%$' for char in value):
        raise ValueError('Fixture directory cannot contain SSH configuration escapes')
    return f'"{value}"'


def write_private(path: Path, value: str) -> None:
    path.write_text(value, encoding='utf-8')
    path.chmod(0o600)


def await_server(server: subprocess.Popen, port: int) -> None:
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if server.poll() is not None:
            raise RuntimeError('Disposable SSH server exited before becoming ready')
        try:
            with socket.create_connection(('127.0.0.1', port), timeout=0.25) as connection:
                if connection.recv(256).startswith(b'SSH-'):
                    return
        except (OSError, TimeoutError):
            pass
        time.sleep(0.05)
    raise RuntimeError('Disposable SSH server did not become ready')


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--native-only', action='store_true')
    parser.add_argument('--skip-acp-tests', action='store_true')
    parser.add_argument('--native-binary', type=Path)
    parser.add_argument('--fixture', type=Path)
    parser.add_argument('--native-output', type=Path)
    options = parser.parse_args()
    native_values = [options.native_binary, options.fixture, options.native_output]
    if any(value is not None for value in native_values) and not all(
        value is not None for value in native_values
    ):
        raise SystemExit('Native SSH smoke requires --native-binary, --fixture and --native-output')
    if sys.platform != 'linux' or os.geteuid() == 0:
        raise SystemExit('Run this Linux fixture as an ordinary user, not as root')
    tools = {name: shutil.which(name) for name in ['ssh', 'ssh-keygen', 'sshd', 'cargo', 'git']}
    if not tools['sshd'] and Path('/usr/sbin/sshd').is_file():
        tools['sshd'] = '/usr/sbin/sshd'
    missing = [name for name, path in tools.items() if path is None]
    if missing:
        raise SystemExit(f'Missing fixture requirements: {", ".join(missing)}')
    user = pwd.getpwuid(os.geteuid()).pw_name
    if not user or any(not (c.isalnum() or c in '._-') for c in user):
        raise SystemExit('Unsupported local fixture account name')
    # Stay below the user's home so sshd StrictModes can validate every ancestor.
    with tempfile.TemporaryDirectory(prefix='.synara-ssh-smoke-', dir=Path.home()) as directory:
        root = Path(directory).resolve()
        root.chmod(0o700)
        write_private(root / 'fixture.marker', 'Synara isolated SSH fixture v1\n')
        (root / "project with ' quote").mkdir(mode=0o700)
        for name in ['host', 'identity', 'wrong identity', 'changed host']:
            run([tools['ssh-keygen'], '-q', '-t', 'ed25519', '-N', '', '-f', str(root / name)])
        write_private(root / 'authorized_keys', (root / 'identity.pub').read_text())
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            port = listener.getsockname()[1]
        for name, key in [('known hosts', 'host.pub'), ('changed hosts', 'changed host.pub')]:
            public = (root / key).read_text().split()
            write_private(root / name, f'[127.0.0.1]:{port} {public[0]} {public[1]}\n')
        write_private(root / 'unknown hosts', '')
        write_private(root / 'ambient config', '\n'.join([
            'Host *',
            '    ForwardAgent yes',
            '    ForwardX11 yes',
            '    ClearAllForwardings no',
            '    PermitLocalCommand yes',
            '    ControlMaster auto',
            f'    ControlPath {config_path(root / "ambient-socket")}',
            '    ControlPersist yes',
            '    RequestTTY force',
            '    ForkAfterAuthentication yes',
            '    LocalForward 127.0.0.1:54321 127.0.0.1:54322',
            '    RemoteForward 127.0.0.1:54323 127.0.0.1:54324',
            '',
        ]))
        configuration = '\n'.join([
            'AddressFamily inet',
            'ListenAddress 127.0.0.1',
            f'Port {port}',
            f'HostKey {config_path(root / "host")}',
            f'PidFile {config_path(root / "server.pid")}',
            f'AuthorizedKeysFile {config_path(root / "authorized_keys")}',
            f'AllowUsers {user}',
            'StrictModes yes',
            'PubkeyAuthentication yes',
            'AuthenticationMethods publickey',
            'PasswordAuthentication no',
            'KbdInteractiveAuthentication no',
            'PermitEmptyPasswords no',
            'UsePAM no',
            'PermitRootLogin no',
            'AllowTcpForwarding local',
            'PermitOpen 127.0.0.1:*',
            'AllowAgentForwarding no',
            'X11Forwarding no',
            'PermitTunnel no',
            'PrintMotd no',
            'PrintLastLog no',
            'LogLevel VERBOSE',
            '',
        ])
        config = root / 'sshd_config'
        write_private(config, configuration)
        env = dict(os.environ)
        helper = (ROOT / 'target' / 'debug' / 'synara-remote-fs').resolve()
        run([
            tools['cargo'], 'build', '--locked', '-p', 'synara-runtime',
            '--bin', 'synara-remote-fs',
        ], cwd=ROOT)
        env.update({
            'SYNARA_SSH_SMOKE_ROOT': str(root),
            'SYNARA_SSH_SMOKE_PORT': str(port),
            'SYNARA_SSH_SMOKE_USER': user,
            'SYNARA_REMOTE_FS_HELPER': str(helper),
            'LC_ALL': 'C',
        })
        log = root / 'sshd.log'
        server = None
        try:
            run([tools['sshd'], '-t', '-f', str(config)])
            with log.open('wb') as diagnostic:
                server = subprocess.Popen(
                    [tools['sshd'], '-D', '-e', '-f', str(config)],
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL,
                    stderr=diagnostic,
                    start_new_session=True,
                )
                await_server(server, port)
                if not options.native_only:
                    packages = ['synara-runtime', 'synara-workspace']
                    if not options.skip_acp_tests:
                        packages.append('synara-acp')
                    for package in packages:
                        print(f'Running isolated SSH acceptance tests for {package}...', flush=True)
                        run([
                            tools['cargo'], 'test', '--locked', '-p', package,
                            '--test', 'ssh_live', '--', '--ignored', '--test-threads=1',
                        ], cwd=ROOT, env=env)
                if options.native_binary is not None:
                    run([
                        sys.executable,
                        str(ROOT / 'scripts' / 'remote_native_smoke.py'),
                        '--binary', str(options.native_binary),
                        '--fixture', str(options.fixture),
                        '--output', str(options.native_output),
                    ], cwd=ROOT, env=env)
            print('PASS: isolated SSH transport, remote FS/PTY/Git and ACP integration')
        except Exception:
            if log.exists():
                # sshd logs authentication metadata, never private key contents.
                print(log.read_text(errors='replace')[-16000:], file=sys.stderr)
            raise
        finally:
            if server is not None:
                server.terminate()
                try:
                    server.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait(timeout=5)


if __name__ == '__main__':
    main()
