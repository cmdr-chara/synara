#!/usr/bin/env python3
"""Session-only transport adapter; never publishes to the integration branches.

The existing checked publisher owns ancestry, no-clobber ref updates, path safety,
size bounds and commit creation. This adapter narrows its write scope and accepts
reviewable text edits when the editing environment cannot reach Git directly.
No transported source code is executed in the credentialed publishing job.
"""
import base64
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

BRANCH = 'refs/heads/astra/session-fgh'
BASE = '1cd24dd6f5ac9571c1ea2b7329bcb5fc1a4ad121'
PUBLISHER_BLOB = '2adfb7c0ee95f475ba76b18694298733f7057b06'
PACKAGE = '.synara-fgh-source.json'
SHARED = {
    'Cargo.lock', 'ROADMAP.md', 'README.md',
    'crates/synara-app/src/main.rs',
    'crates/synara-app/src/shell.rs',
    'crates/synara-app/src/shell/panels.rs',
    'crates/synara-app/src/input.rs',
    'crates/synara-app/src/close.rs',
}
PREFIXES = (
    'crates/synara-workspace/',
    'crates/synara-app/src/fgh/',
    'docs/fgh-', 'scripts/test_fgh_', 'scripts/fgh_smoke',
)


def permitted(name):
    return isinstance(name, str) and (name in SHARED or name.startswith(PREFIXES))


def normalize_text_package(package, source_path):
    """Expand exact text replacements, not executable patches or regular expressions."""
    if 'files' not in package and 'edits' not in package:
        return package
    if package.get('mode', 'patch') != 'patch':
        raise SystemExit('Text packages must use patch mode')
    if 'gzip_base64' in package or 'parts' in package:
        raise SystemExit('Ambiguous source package representation')
    files = package.get('files', {})
    edits = package.get('edits', {})
    if not isinstance(files, dict) or not isinstance(edits, dict):
        raise SystemExit('Invalid text package maps')
    files = dict(files)
    for name, replacements in edits.items():
        if name in files or not isinstance(replacements, list) or not replacements:
            raise SystemExit('Invalid or overlapping text edits')
        target = source_path(name)
        if not target.is_file() or target.stat().st_size > 2 * 1024 * 1024:
            raise SystemExit('Missing or oversized text edit target')
        text = target.read_text(encoding='utf-8')
        for pair in replacements:
            if not isinstance(pair, list) or len(pair) != 2 or not all(isinstance(s, str) for s in pair):
                raise SystemExit('Invalid replacement pair')
            old, new = pair
            if not old or text.count(old) != 1:
                raise SystemExit(f'Text edit is not unique in {name}')
            text = text.replace(old, new, 1)
        files[name] = text
    for name in files:
        source_path(name)
    data = json.dumps(files, ensure_ascii=False, separators=(',', ':')).encode()
    if len(data) > 20 * 1024 * 1024:
        raise SystemExit('Source package exceeds limit')
    normalized = {key: value for key, value in package.items() if key not in ('files', 'edits')}
    normalized['mode'] = 'patch'
    normalized['sha256'] = hashlib.sha256(data).hexdigest()
    normalized['gzip_base64'] = base64.b64encode(gzip.compress(data, mtime=0)).decode('ascii')
    return normalized


def main():
    if os.environ.get('GITHUB_REPOSITORY') != 'cmdr-chara/synara' or os.environ.get('GITHUB_REF') != BRANCH:
        raise SystemExit('FGH publishing is restricted to astra/session-fgh')
    subprocess.run(['git', 'merge-base', '--is-ancestor', BASE, 'HEAD'], check=True)
    publisher_path = Path('scripts/apply_source.py')
    blob = subprocess.check_output(['git', 'hash-object', str(publisher_path)], text=True).strip()
    if blob != PUBLISHER_BLOB:
        raise SystemExit('The reviewed source publisher changed')
    spec = importlib.util.spec_from_file_location('synara_checked_publisher', publisher_path)
    publisher = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(publisher)
    original_source_path = publisher.source_path

    def restricted_source_path(name):
        if not permitted(name):
            raise SystemExit(f'Path is outside FGH ownership: {name}')
        return original_source_path(name)

    publisher.source_path = restricted_source_path
    publisher.BRANCH = BRANCH
    publisher.PAYLOAD = publisher.ROOT / PACKAGE
    if publisher.PAYLOAD.is_symlink() or not publisher.PAYLOAD.is_file() or publisher.PAYLOAD.stat().st_size > 8 * 1024 * 1024:
        raise SystemExit('Missing or invalid FGH source package')
    package = json.loads(publisher.PAYLOAD.read_text(encoding='utf-8'))
    if not isinstance(package, dict) or package.get('mode', 'patch') not in ('patch', 'diff'):
        raise SystemExit('FGH cannot replace a repository snapshot')
    package = normalize_text_package(package, restricted_source_path)
    publisher.PAYLOAD.write_text(json.dumps(package), encoding='utf-8')
    publisher.main()


if __name__ == '__main__':
    main()
