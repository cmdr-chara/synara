#!/usr/bin/env python3
"""Apply an integrity-checked source checkpoint to the explicitly authorized branch."""
import base64
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess

ROOT = Path.cwd().resolve()
BRANCH = 'refs/heads/astra/gpui-clean-rewrite'
ROOT_COMMIT = '43b1fb89bf19dadc388d18008f9ceb21b8215716'
PAYLOAD = ROOT / '.synara-source.json'

def git(*args):
    return subprocess.check_output(['git', *args], text=True).strip()

def source_path(name):
    if not isinstance(name, str) or not re.fullmatch(r'[A-Za-z0-9_./-]+', name):
        raise SystemExit('Invalid source path')
    path = PurePosixPath(name)
    if path.is_absolute() or str(path) != name or any(p in ('..', '.git') for p in path.parts):
        raise SystemExit('Unsafe source path')
    if name.startswith(('.github/workflows/', '.synara-source', '.synara-transfer-')):
        raise SystemExit('Reserved source path')
    target = ROOT / path
    if not target.resolve().is_relative_to(ROOT):
        raise SystemExit('Source path escapes checkout')
    for parent in (target, *target.parents):
        if parent == ROOT:
            break
        if parent.is_symlink():
            raise SystemExit('Symlink in source path')
    return target

def main():
    if os.environ.get('GITHUB_REPOSITORY') != 'cmdr-chara/synara' or os.environ.get('GITHUB_REF') != BRANCH:
        raise SystemExit('This publisher only writes the authorized rewrite branch')
    revision = git('rev-parse', 'HEAD')
    if git('rev-list', '--max-parents=0', 'HEAD').splitlines() != [ROOT_COMMIT]:
        raise SystemExit('Unexpected Git lineage')
    if PAYLOAD.exists():
        if PAYLOAD.is_symlink() or PAYLOAD.stat().st_size > 8 * 1024 * 1024:
            raise SystemExit('Invalid source package')
        package = json.loads(PAYLOAD.read_text())
        if git('show', '-s', '--format=%P', 'HEAD').split() != [package['base_commit']]:
            raise SystemExit('Source package base does not match its parent')
        if git('ls-remote', 'origin', BRANCH).split()[0] != revision:
            raise SystemExit('Target branch changed while this package was queued')
        encoded = package.get('gzip_base64')
        parts = package.get('parts', [])
        if encoded is None:
            if not isinstance(parts, list) or not 1 <= len(parts) <= 64 or len(set(parts)) != len(parts):
                raise SystemExit('Invalid source package parts')
            texts = []
            total = 0
            for name in parts:
                if not re.fullmatch(r'\.synara-transfer-[0-9]{4}', name):
                    raise SystemExit('Invalid transfer part path')
                path = ROOT / name
                if path.is_symlink() or not path.is_file():
                    raise SystemExit('Invalid transfer part')
                total += path.stat().st_size
                if total > 8 * 1024 * 1024:
                    raise SystemExit('Source transfer exceeds limit')
                texts.append(path.read_text(encoding='ascii').strip())
            encoded = ''.join(texts)
        compressed = base64.b64decode(encoded, validate=True)
        with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
            data = stream.read(20 * 1024 * 1024 + 1)
        if len(data) > 20 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != package['sha256']:
            raise SystemExit('Source package checksum or size mismatch')
        files = json.loads(data)
        if not isinstance(files, dict) or not 1 <= len(files) <= 2000:
            raise SystemExit('Invalid source file map')
        for name, content in files.items():
            source_path(name)
            if not isinstance(content, str) or '\x00' in content or len(content.encode()) > 2 * 1024 * 1024:
                raise SystemExit('Invalid source file contents')
        mode = package.get('mode', 'snapshot')
        if mode not in ('snapshot', 'patch'):
            raise SystemExit('Invalid source application mode')
        deleted = package.get('delete', [])
        if not isinstance(deleted, list) or len(deleted) > 2000:
            raise SystemExit('Invalid deletion list')
        for name in deleted:
            source_path(name)
            if name in files:
                raise SystemExit('Source path is both written and deleted')
        if mode == 'snapshot':
            tracked = subprocess.check_output(['git', 'ls-files', '-z']).decode().split('\0')
            for name in filter(None, tracked):
                if name not in files and not name.startswith(('.github/workflows/', '.synara-source', '.synara-transfer-')):
                    source_path(name).unlink(missing_ok=True)
        for name in deleted:
            source_path(name).unlink(missing_ok=True)
        for name, content in files.items():
            target = source_path(name)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content, encoding='utf-8', newline='')
        PAYLOAD.unlink()
        for name in parts:
            (ROOT / name).unlink()
        message = package.get('message', 'feat: integrate native source checkpoint')
        if not isinstance(message, str) or not 1 <= len(message) <= 200 or '\n' in message:
            raise SystemExit('Invalid commit message')
        subprocess.run(['git', 'add', '--all'], check=True)
        subprocess.run(['git', '-c', 'user.name=github-actions[bot]', '-c', 'user.email=41898282+github-actions[bot]@users.noreply.github.com', 'commit', '-m', message], check=True)
        subprocess.run(['git', 'push', 'origin', f'HEAD:{BRANCH}'], check=True)
        revision = git('rev-parse', 'HEAD')
    with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
        output.write(f'revision={revision}\n')

if __name__ == '__main__':
    main()
