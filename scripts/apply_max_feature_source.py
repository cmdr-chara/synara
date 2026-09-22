#!/usr/bin/env python3
"""Validate bounded source deltas before publishing only the authorized session branch."""
import json
import os
from pathlib import Path
import re
import runpy
import subprocess

BRANCH = 'refs/heads/astra/max-feature-sprint'
BASE = '33fa0a42dd5ac1098a7ba5f1e7bd5b240abbf577'
PAYLOAD = '.synara-max-feature.json'
EXACT = {
    'Cargo.toml', 'Cargo.lock', 'ROADMAP.md',
    'crates/synara-app/Cargo.toml', 'crates/synara-app/src/main.rs',
    'crates/synara-app/src/shell.rs',
    'crates/synara-app/src/shell/composer.rs',
    'crates/synara-app/src/shell/attachments.rs',
    'crates/synara-app/src/shell/settings.rs',
    'crates/synara-app/src/shell/direct_models.rs',
    'crates/synara-app/src/shell/direct_models/view.rs',
    'crates/synara-runtime/Cargo.toml', 'crates/synara-runtime/src/lib.rs',
    'crates/synara-runtime/src/native_secrets.rs',
    'crates/synara-workspace/Cargo.toml', 'crates/synara-workspace/src/lib.rs',
    'crates/synara-workspace/src/controller.rs',
    'crates/synara-workspace/src/direct_models.rs',
    'crates/synara-workspace/src/controller/direct_models.rs',
    'crates/synara-workspace/src/storage/direct_models.rs',
    'crates/synara-workspace/src/storage.rs',
    'crates/synara-workspace/src/storage/recovery.rs',
    'scripts/native_direct_models_smoke.py',
    'docs/ui/direct-models.md', 'docs/verification/max-feature-sprint.md',
    'docs/ui/electron-vs-gpui-feature-gap.md',
}


def run(*args):
    subprocess.run(args, check=True)


def main():
    if os.environ.get('GITHUB_REPOSITORY') != 'cmdr-chara/synara' or os.environ.get('GITHUB_REF') != BRANCH:
        raise SystemExit('Sprint publisher refuses every other repository and ref')
    run('git', 'merge-base', '--is-ancestor', BASE, 'HEAD')
    path = Path(PAYLOAD)
    if not path.is_file() or path.is_symlink() or path.stat().st_size > 8 * 1024 * 1024:
        raise SystemExit('Invalid sprint source package')
    package = json.loads(path.read_text())
    if not isinstance(package, dict) or package.get('mode') != 'diff' or set(package) - {'mode', 'base_commit', 'gzip_base64', 'parts', 'sha256', 'message'}:
        raise SystemExit('Only integrity-checked explicit deltas are accepted')
    parts = package.get('parts')
    if parts is not None and (package.get('gzip_base64') is not None or not isinstance(parts, list) or not 1 <= len(parts) <= 16 or any(not isinstance(p, str) or not re.fullmatch(r'\.synara-transfer-[0-9]{4}', p) for p in parts)):
        raise SystemExit('Invalid bounded transfer parts')
    common = runpy.run_path(str(Path(__file__).with_name('apply_source.py')))
    scope = common['main'].__globals__
    source_path, apply_diff = scope['source_path'], scope['apply_diff']

    def checked_path(name):
        result = source_path(name)
        if name not in EXACT and not name.startswith('crates/synara-model/'):
            raise SystemExit(f'Path outside reviewed sprint ownership: {name}')
        return result

    def validated_diff(data):
        apply_diff(data)
        changed = subprocess.check_output(['git', 'diff', 'HEAD', '--name-only', '-z']).decode().split('\0')
        rust = []
        for name in filter(None, changed):
            if name in (parts or []):
                continue
            checked_path(name)
            if name.endswith('.rs') and subprocess.run(['git', 'cat-file', '-e', f'{BASE}:{name}'], stderr=subprocess.DEVNULL).returncode:
                rust.append(name)
        if rust:
            run('rustfmt', '+1.98.1', '--edition', '2024', '--config', 'skip_children=true', *rust)
        run('cargo', '+1.98.1', 'test', '-p', 'synara-model')
        run('cargo', '+1.98.1', 'test', '--locked', '-p', 'synara-runtime', 'native_secrets')
        run('cargo', '+1.98.1', 'test', '--locked', '-p', 'synara-workspace', 'direct_models')
        run('cargo', '+1.98.1', 'build', '--locked', '-p', 'synara-app', '--bin', 'synara-app', '-p', 'synara-acp', '--bin', 'synara-acp-fixture')
        run('/usr/bin/python3', 'scripts/native_direct_models_smoke.py', '--output', '/tmp/max-feature-evidence/native')
        run('git', 'diff', '--check')
        for name in filter(None, subprocess.check_output(['git', 'diff', 'HEAD', '--name-only', '-z']).decode().split('\0')):
            if name in (parts or []):
                continue
            checked_path(name)
        run('git', 'add', '--all')

    scope.update(BRANCH=BRANCH, PAYLOAD=Path.cwd() / PAYLOAD, source_path=checked_path, apply_diff=validated_diff)
    scope['main']()


if __name__ == '__main__':
    main()
