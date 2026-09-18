#!/usr/bin/env python3
"""Reuse the checked source publisher with an exclusive B/C/D write allowlist."""
import json
import os
import re
from pathlib import Path, PurePosixPath
import runpy
import subprocess

BRANCH = 'refs/heads/astra/session-bcd'
BASE = '1cd24dd6f5ac9571c1ea2b7329bcb5fc1a4ad121'
PAYLOAD = '.synara-bcd-source.json'
EXACT = {
    'ROADMAP.md', 'docs/agent-compatibility.md', 'docs/acp-coverage.md',
    'docs/bcd-session-handoff.md', 'scripts/bcd_smoke.py',
    'crates/synara-app/src/shell.rs',
    'crates/synara-app/src/shell/conversation.rs',
    'crates/synara-app/src/shell/transcript.rs',
    'crates/synara-app/src/shell/questions.rs',
    'crates/synara-app/src/shell/composer.rs',
    'crates/synara-app/src/shell/connection.rs',
}
PREFIXES = (
    'crates/synara-agent/', 'crates/synara-acp/',
    'crates/synara-core/', 'crates/synara-workspace/',
)


def allowed(name):
    if not isinstance(name, str):
        return False
    path = PurePosixPath(name)
    if path.is_absolute() or str(path) != name or any(p in {'.', '..', '.git'} for p in path.parts):
        return False
    if 'ssh' in name.lower() or 'terminal' in name.lower():
        return False
    return name in EXACT or name.startswith(PREFIXES)


def validate_package(package):
    if not isinstance(package, dict) or package.get('mode') != 'diff':
        raise SystemExit('BCD publisher only accepts explicit source deltas')
    if set(package) - {'mode', 'base_commit', 'gzip_base64', 'parts', 'sha256', 'message'}:
        raise SystemExit('Unsupported BCD package field')
    encoded = package.get('gzip_base64')
    parts = package.get('parts')
    if isinstance(encoded, str) and parts is None:
        return
    if encoded is not None or not isinstance(parts, list) or not 1 <= len(parts) <= 64:
        raise SystemExit('Choose one bounded BCD transfer representation')
    if any(not isinstance(name, str) or not re.fullmatch(r'\.synara-transfer-[0-9]{4}', name)
           for name in parts) or len(set(parts)) != len(parts):
        raise SystemExit('Invalid BCD transfer parts')


def main():
    if os.environ.get('GITHUB_REPOSITORY') != 'cmdr-chara/synara' or os.environ.get('GITHUB_REF') != BRANCH:
        raise SystemExit('BCD publisher refuses every other repository or ref')
    subprocess.run(['git', 'merge-base', '--is-ancestor', BASE, 'HEAD'], check=True)
    path = Path(PAYLOAD)
    if path.exists():
        if path.is_symlink() or path.stat().st_size > 8 * 1024 * 1024:
            raise SystemExit('Invalid BCD source package')
        package = json.loads(path.read_text())
        validate_package(package)
    loaded = runpy.run_path(str(Path(__file__).with_name('apply_source.py')))
    scope = loaded['main'].__globals__
    validate_path = scope['source_path']
    apply_diff = scope['apply_diff']

    def bcd_path(name):
        if not allowed(name):
            raise SystemExit(f'Path outside exclusive BCD ownership: {name}')
        return validate_path(name)

    def formatted_diff(data):
        apply_diff(data)
        subprocess.run(['cargo', '+1.98.1', 'fmt', '--all'], check=True)
        changed = subprocess.check_output(['git', 'diff', 'HEAD', '--name-only', '-z']).decode().split('\0')
        for name in filter(None, changed):
            bcd_path(name)
        subprocess.run(['git', 'add', '--all'], check=True)

    scope.update(BRANCH=BRANCH, PAYLOAD=Path.cwd() / PAYLOAD,
                 source_path=bcd_path, apply_diff=formatted_diff)
    scope['main']()


if __name__ == '__main__':
    main()
