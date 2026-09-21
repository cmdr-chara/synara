#!/usr/bin/env python3
"""Apply only the reviewed Device + Settings source on its authorized session branch."""
import base64
import lzma
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess

BRANCH = 'refs/heads/astra/device-settings'
BASE = 'be611337195eb826def7d3d57688cb5b8f491239'
INTEGRATION_BASE = '980d86b59a1f06636a55aa0b75b71eef41fd3841'
PARTS = [f'scripts/device-settings-part-{i}' for i in range(1, 5)]
TRANSPORT = ['scripts/device-settings-source.json', 'scripts/apply_device_settings.py', '.github/workflows/device-settings-focus.yml', *PARTS]
ROOT = Path.cwd().resolve()

def git(*args, **kwargs):
    return subprocess.check_output(['git', *args], **kwargs).decode().strip()

def commit(message):
    if not isinstance(message, str) or not 1 <= len(message) <= 240 or '\n' in message or '[skip ci]' not in message:
        raise SystemExit('Invalid reviewed commit message')
    subprocess.run(['git', '-c', 'user.name=github-actions[bot]', '-c', 'user.email=41898282+github-actions[bot]@users.noreply.github.com', 'commit', '-m', message], check=True)

def source_path(name):
    if not isinstance(name, str) or not re.fullmatch(r'[A-Za-z0-9_./-]+', name):
        raise SystemExit('Invalid source path')
    path = PurePosixPath(name)
    if path.is_absolute() or str(path) != name or any(p in ('..', '.git') for p in path.parts):
        raise SystemExit('Unsafe source path')
    if name != 'ROADMAP.md' and not name.startswith(('crates/synara-runtime/src/', 'crates/synara-workspace/src/', 'crates/synara-app/src/', 'docs/')):
        raise SystemExit('Source path outside the reviewed implementation')
    target = ROOT / path
    for item in (target, *target.parents):
        if item == ROOT:
            break
        if item.is_symlink():
            raise SystemExit('Symlink in source path')
    return name

def main():
    if os.environ.get('GITHUB_REPOSITORY') != 'cmdr-chara/synara' or os.environ.get('GITHUB_REF') != BRANCH:
        raise SystemExit('Only the authorized session branch may be published')
    if git('status', '--porcelain'):
        raise SystemExit('Checkout must be clean')
    revision = git('rev-parse', 'HEAD')
    if git('show', '-s', '--format=%P', 'HEAD') != BASE:
        raise SystemExit('Unexpected transport parent')
    if sorted(git('diff', '--name-only', BASE, 'HEAD').splitlines()) != sorted(TRANSPORT):
        raise SystemExit('Unexpected transport changes')
    if git('ls-remote', 'origin', BRANCH).split()[0] != revision:
        raise SystemExit('Session branch changed while queued')
    package_path = Path(TRANSPORT[0])
    if package_path.is_symlink() or package_path.stat().st_size > 1024 * 1024:
        raise SystemExit('Invalid source package')
    package = json.loads(package_path.read_text())
    if package['base'] != BASE or package['integration_base'] != INTEGRATION_BASE:
        raise SystemExit('Unexpected source base')
    if package['parts'] != PARTS:
        raise SystemExit('Unexpected transfer part names')
    parts = []
    for name in PARTS:
        path = Path(name)
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 16000:
            raise SystemExit('Invalid transfer part')
        parts.append(path.read_text(encoding='ascii'))
    compressed = base64.b64decode(''.join(parts), validate=True)
    with lzma.LZMAFile(io.BytesIO(compressed)) as stream:
        raw = stream.read(2 * 1024 * 1024 + 1)
    if len(raw) > 2 * 1024 * 1024 or hashlib.sha256(raw).hexdigest() != package['sha256']:
        raise SystemExit('Invalid source digest or size')
    source = json.loads(raw)
    if not 1 <= len(source['groups']) <= 6:
        raise SystemExit('Invalid source group count')
    for group in source['groups']:
        patch = group['diff'].encode('utf-8')
        if b'GIT binary patch' in patch or b'Binary files ' in patch or b'rename from ' in patch:
            raise SystemExit('Only text changes are authorized')
        for mode in re.findall(rb'(?m)^(?:new file mode|new mode) ([0-9]+)$', patch):
            if mode != b'100644':
                raise SystemExit('Links, executable additions and submodules are not authorized')
        rows = subprocess.check_output(['git', 'apply', '--numstat', '-z', '-'], input=patch).split(b'\0')
        paths = []
        for row in filter(None, rows):
            added, removed, name = row.split(b'\t', 2)
            if not added.isdigit() or not removed.isdigit():
                raise SystemExit('Binary source change')
            paths.append(source_path(name.decode('utf-8')))
        if sorted(paths) != sorted(group['paths']) or len(paths) != len(set(paths)):
            raise SystemExit('Reviewed path manifest mismatch')
        subprocess.run(['git', 'apply', '--index', '--check', '--whitespace=error-all', '-'], input=patch, check=True)
        subprocess.run(['git', 'apply', '--index', '--whitespace=error-all', '-'], input=patch, check=True)
        commit(group['message'])
    # Restore the exact historical workflow and remove all temporary transport.
    original = subprocess.check_output(['git', 'show', INTEGRATION_BASE + ':.github/workflows/source-checkpoint.yml'])
    Path('.github/workflows/source-checkpoint.yml').write_bytes(original)
    subprocess.run(['git', 'rm', '--', *TRANSPORT], check=True)
    subprocess.run(['git', 'add', '.github/workflows/source-checkpoint.yml'], check=True)
    if git('write-tree') != source['expected_clean_tree']:
        raise SystemExit('Published source differs from the reviewed clean tree')
    commit('chore(device): restore checkpoint trigger and remove session transport [skip ci]')
    for path in source['new_rust']:
        source_path(path)
        if not path.endswith('.rs'):
            raise SystemExit('Invalid format target')
    subprocess.run(['rustfmt', '+1.98.1', '--edition', '2024', '--config', 'skip_children=true', *source['new_rust']], check=True)
    changed = git('diff', '--name-only').splitlines()
    if not set(changed).issubset(source['new_rust']):
        raise SystemExit('Formatter touched historical source')
    if changed:
        subprocess.run(['git', 'add', '--', *changed], check=True)
        commit('style(device): format new runtime and native Settings modules [skip ci]')
    subprocess.run(['git', 'diff', '--check', INTEGRATION_BASE, 'HEAD'], check=True)
    subprocess.run(['python3', 'scripts/check_roadmap.py'], check=True)
    if git('status', '--porcelain'):
        raise SystemExit('Publication did not leave a clean tree')
    # Credentials are supplied only to this push through ephemeral environment.
    env = dict(os.environ)
    token = env.pop('PUBLISH_TOKEN')
    env['GIT_CONFIG_COUNT'] = '1'
    env['GIT_CONFIG_KEY_0'] = 'http.https://github.com/.extraheader'
    env['GIT_CONFIG_VALUE_0'] = 'AUTHORIZATION: basic ' + base64.b64encode(('x-access-token:' + token).encode()).decode()
    subprocess.run(['git', 'push', 'origin', 'HEAD:' + BRANCH], check=True, env=env)
    final = git('rev-parse', 'HEAD')
    if git('ls-remote', 'origin', BRANCH).split()[0] != final:
        raise SystemExit('Published session ref was not verified')
    output = Path('/tmp/device-settings-result')
    output.mkdir(exist_ok=True)
    (output / 'revision.txt').write_text(final + '\n')
    (output / 'source-manifest.json').write_text(json.dumps({'source_sha256':package['sha256'], 'reviewed_clean_tree':source['expected_clean_tree'], 'final_tree':git('rev-parse', 'HEAD^{tree}'), 'formatted_paths':changed}, indent=2))
    subprocess.run(['git', 'bundle', 'create', str(output / 'source.bundle'), 'HEAD'], check=True)
    with open(os.environ['GITHUB_OUTPUT'], 'a') as out:
        out.write('revision=' + final + '\n')

if __name__ == '__main__':
    main()
