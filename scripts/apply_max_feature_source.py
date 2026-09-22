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
IMPORT_PATHS = {
    'Cargo.lock', 'crates/synara-app/Cargo.toml',
    'crates/synara-app/src/shell.rs',
    'crates/synara-app/src/shell/settings.rs',
    'crates/synara-app/src/shell/project_import.rs',
    'crates/synara-app/src/shell/project_import/view.rs',
    'crates/synara-runtime/src/filesystem.rs',
    'crates/synara-workspace/Cargo.toml', 'crates/synara-workspace/src/lib.rs',
    'crates/synara-workspace/src/imports.rs',
    'crates/synara-workspace/src/imports/tests.rs',
    'crates/synara-workspace/src/storage/imports.rs',
    'crates/synara-workspace/src/storage.rs',
    'crates/synara-workspace/src/storage/recovery.rs',
    'scripts/native_project_import_smoke.py',
}
EXACT = IMPORT_PATHS | {
    'crates/synara-app/src/shell/direct_models/view.rs',
    'crates/synara-workspace/src/controller.rs',
    'crates/synara-workspace/src/controller/direct_models.rs',
    'scripts/native_direct_models_smoke.py',
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
    if not isinstance(parts, list) or not 1 <= len(parts) <= 16 or len(set(parts)) != len(parts) or any(not isinstance(p, str) or not re.fullmatch(r'\.synara-transfer-[0-9]{4}', p) for p in parts):
        raise SystemExit('Invalid bounded transfer parts')
    common = runpy.run_path(str(Path(__file__).with_name('apply_source.py')))
    scope = common['main'].__globals__
    source_path, apply_diff = scope['source_path'], scope['apply_diff']

    def checked_path(name):
        result = source_path(name)
        if name not in EXACT and not name.startswith('crates/synara-model/'):
            raise SystemExit(f'Path outside reviewed sprint ownership: {name}')
        return result

    def changed_paths():
        names = subprocess.check_output(['git', 'diff', 'HEAD', '--name-only', '-z']).decode().split('\0')
        return [n for n in names if n and n not in parts]

    def validated_diff(data):
        apply_diff(data)
        rust = []
        for name in changed_paths():
            checked_path(name)
            if name.endswith('.rs') and subprocess.run(['git', 'cat-file', '-e', f'{BASE}:{name}'], stderr=subprocess.DEVNULL).returncode:
                rust.append(name)
        if rust:
            run('rustfmt', '+1.98.1', '--check', '--edition', '2024', '--config', 'skip_children=true', *rust)
        run('cargo', '+1.98.1', 'test', '--locked', '-p', 'synara-model', '-p', 'synara-runtime', '-p', 'synara-workspace')
        run('cargo', '+1.98.1', 'build', '--locked', '-p', 'synara-app', '--bin', 'synara-app', '-p', 'synara-acp', '--bin', 'synara-acp-fixture')
        for journey, output in [('native_project_import_smoke.py', 'import'), ('native_direct_models_smoke.py', 'direct'), ('native_model_draft_smoke.py', 'model-draft-regression')]:
            run('/usr/bin/python3', f'scripts/{journey}', '--binary', 'target/debug/synara-app', '--fixture', 'target/debug/synara-acp-fixture', '--output', f'/tmp/max-feature-evidence/{output}')
        run('git', 'diff', '--check')
        changed = changed_paths()
        for name in changed:
            checked_path(name)
        if not IMPORT_PATHS.issubset(set(changed)):
            raise SystemExit('Import commit is missing required vertical-slice files')
        # Both feature groups have passed validation. Keep their ownership distinct.
        run('git', 'reset', '--mixed', 'HEAD')
        run('git', 'add', '--', *sorted(IMPORT_PATHS))
        run('git', '-c', 'user.name=github-actions[bot]', '-c', 'user.email=41898282+github-actions[bot]@users.noreply.github.com', 'commit', '-m', 'feat(import): review and atomically import local Codex and Claude histories')
        # The common publisher commits model corrections and removes all transfer files.
        # Its final non-force push publishes both commits only to the session ref.

    scope.update(BRANCH=BRANCH, PAYLOAD=Path.cwd() / PAYLOAD, source_path=checked_path, apply_diff=validated_diff)
    scope['main']()


if __name__ == '__main__':
    main()
