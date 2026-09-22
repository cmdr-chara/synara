#!/usr/bin/env python3
"""Temporary, exact-ref source transport for the authorized feature-closure branch.

The prepare/check/package job has read-only credentials. The separate publish job
never executes candidate code and writes only verified Git objects and one
non-force session ref. Remove this file and its workflow before integration.
"""
import base64
import gzip
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import urllib.request
from pathlib import Path, PurePosixPath

REPO = 'cmdr-chara/synara'
BRANCH = 'astra/feature-closure-sprint'
REQUEST = '.feature-closure-request.json'
INFRA = {REQUEST, 'scripts/feature_closure_transfer.py', '.github/workflows/feature-closure-sprint.yml'} | {f'.feature-closure-part-{n:04d}' for n in range(16)}
EVIDENCE = Path('/tmp/feature-closure-evidence')
BUNDLE = Path('/tmp/feature-closure-publication')
LIMIT = 16 * 1024 * 1024
CRATES = {'synara-workspace', 'synara-core', 'synara-runtime', 'synara-model', 'synara-agent', 'synara-acp', 'synara-browser'}
JOURNEYS = {'debug', 'goals', 'recap', 'pr_fix', 'inline_comments', 'releases', 'rich_media', 'task_split', 'stacked_prs', 'appsnap', 'handoff', 'project_import', 'direct_models', 'chat_behavior', 'browser_webview', 'integrations', 'model_draft'}


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def git(*args):
    return subprocess.check_output(['git', *args])


def require_context():
    assert os.environ.get('GITHUB_REPOSITORY') == REPO
    assert os.environ.get('GITHUB_REF') == 'refs/heads/' + BRANCH
    head = os.environ['GITHUB_SHA']
    assert re.fullmatch('[0-9a-f]{40}', head)
    assert git('rev-parse', 'HEAD').decode().strip() == head
    return head


def read_request():
    path = Path(REQUEST)
    assert not path.is_symlink() and 0 < path.stat().st_size <= LIMIT
    value = json.loads(path.read_text())
    assert set(value) == {'version', 'base', 'message', 'patch_sha256', 'patch_gzip_base64', 'tests', 'native', 'integrated'}
    assert value['version'] == 1 and re.fullmatch('[0-9a-f]{40}', value['base'])
    assert isinstance(value['message'], str) and 0 < len(value['message']) <= 180 and '\n' not in value['message'] and '\0' not in value['message']
    assert isinstance(value['tests'], list) and len(value['tests']) <= 24
    assert all(isinstance(pair, list) and len(pair) == 2 and pair[0] in CRATES and re.fullmatch('[A-Za-z0-9_:]{1,100}', pair[1]) for pair in value['tests'])
    assert isinstance(value['native'], list) and len(value['native']) <= len(JOURNEYS) and set(value['native']) <= JOURNEYS
    assert type(value['integrated']) is bool
    assert value['tests'] or value['native'] or value['integrated']
    return value


def source_path(path):
    pure = PurePosixPath(path)
    assert not pure.is_absolute() and '..' not in pure.parts and str(pure) == path
    assert not any(part.startswith('.') for part in pure.parts)
    return (path.startswith('crates/') and path.endswith('.rs')
            or path.startswith('docs/') and path.endswith('.md')
            or path == 'ROADMAP.md'
            or path in {'scripts/native_' + name + '_smoke.py' for name in JOURNEYS})


def changed_paths():
    paths = git('diff', '--cached', '--name-only', '-z').decode().strip('\0').split('\0')
    assert 0 < len(paths) <= 150 and len(set(paths)) == len(paths)
    for path in paths:
        assert source_path(path), path
        assert Path(path).is_file() and not Path(path).is_symlink(), path
        assert Path(path).stat().st_size <= 2 * 1024 * 1024, path
        mode = git('ls-files', '-s', '--', path).decode().split()[0]
        assert mode in {'100644', '100755'}, (path, mode)
        Path(path).read_text(encoding='utf-8')
    return paths


def prepare():
    head = require_context()
    request = read_request()
    run('git', 'merge-base', '--is-ancestor', request['base'], head)
    intervening = set(git('diff', '--name-only', request['base'], head).decode().splitlines())
    assert intervening <= INFRA, ('Unexpected source changes after reviewed base', intervening)
    encoded = request['patch_gzip_base64']
    if isinstance(encoded, list):
        assert 0 < len(encoded) <= 16 and len(set(encoded)) == len(encoded)
        assert all(path in INFRA and path.startswith('.feature-closure-part-') for path in encoded)
        assert all(not Path(path).is_symlink() and 0 < Path(path).stat().st_size <= 1024 * 1024 for path in encoded)
        encoded = ''.join(Path(path).read_text().strip() for path in encoded)
    compressed = base64.b64decode(encoded, validate=True)
    with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
        patch = stream.read(LIMIT + 1)
    assert 0 < len(patch) <= LIMIT
    assert hashlib.sha256(patch).hexdigest() == request['patch_sha256']
    assert b'GIT binary patch' not in patch and b'rename from ' not in patch and b'copy from ' not in patch
    patch.decode('utf-8')
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / 'transport-revision.txt').write_text(head + '\n')
    (EVIDENCE / 'requested.patch').write_bytes(patch)
    run('git', 'apply', '--index', '--check', '--whitespace=error-all', str(EVIDENCE / 'requested.patch'))
    run('git', 'apply', '--index', '--whitespace=error-all', str(EVIDENCE / 'requested.patch'))
    paths = changed_paths()
    added = set(git('diff', '--cached', '--diff-filter=A', '--name-only').decode().splitlines())
    # Do not rewrite pre-existing files for cosmetic formatting.
    for path in paths:
        if path in added and path.endswith('.rs'):
            run('rustfmt', '+1.98.1', '--edition', '2024', '--config', 'skip_children=true', path)
            run('git', 'add', '--', path)
    (EVIDENCE / 'source-paths.json').write_text(json.dumps(paths))
    run('git', 'diff', '--cached', '--check')


def logged(name, args, allowed_errors=None):
    path = EVIDENCE / name
    with path.open('wb') as out:
        result = subprocess.run(args, stdout=out, stderr=subprocess.STDOUT)
    print(path.read_text(errors='replace'), flush=True)
    if result.returncode:
        if allowed_errors is not None:
            assert json.loads(path.read_text())['errors'] == allowed_errors
        else:
            raise subprocess.CalledProcessError(result.returncode, args)


def check():
    require_context()
    request = read_request()
    logged('roadmap.log', ['python3', 'scripts/check_roadmap.py'])
    logged('structural-audit.log', ['python3', 'scripts/audit_workspace.py'],
           ['crates/synara-browser/src/native/actions.js: non-Rust core source'])
    for index, (crate, test_filter) in enumerate(request['tests']):
        logged(f'focused-{index}.log', ['cargo', '+1.98.1', 'test', '--locked', '-p', crate, test_filter])
    if request['integrated']:
        logged('integrated.log', ['cargo', '+1.98.1', 'test', '--locked', '--workspace', '--exclude', 'synara-app'])
    paths = json.loads((EVIDENCE / 'source-paths.json').read_text())
    if request['native'] or any(path.startswith('crates/synara-app/') for path in paths):
        logged('native-build.log', ['cargo', '+1.98.1', 'build', '--locked', '-p', 'synara-app', '--bin', 'synara-app', '-p', 'synara-acp', '--bin', 'synara-acp-fixture'])
    for journey in request['native']:
        logged(f'native-{journey}.log', ['/usr/bin/python3', f'scripts/native_{journey}_smoke.py', '--binary', 'target/debug/synara-app', '--fixture', 'target/debug/synara-acp-fixture', '--output', str(EVIDENCE / journey)])
    run('git', 'diff', '--exit-code')
    assert set(changed_paths()) == set(paths)


def package():
    head = require_context()
    request = read_request()
    paths = changed_paths()
    entries = []
    for path in paths:
        mode, sha, _ = git('ls-files', '-s', '--', path).decode().split()[:3]
        content = Path(path).read_bytes()
        assert hashlib.sha1(b'blob ' + str(len(content)).encode() + b'\0' + content).hexdigest() == sha
        entries.append({'path': path, 'mode': mode, 'sha': sha, 'content': base64.b64encode(content).decode()})
    value = {'parent': head, 'tree': git('write-tree').decode().strip(), 'message': request['message'], 'entries': entries}
    data = json.dumps(value, separators=(',', ':')).encode()
    assert len(data) <= LIMIT
    BUNDLE.mkdir(parents=True, exist_ok=True)
    (BUNDLE / 'publication.json').write_bytes(data)
    digest = hashlib.sha256(data).hexdigest()
    with open(os.environ['GITHUB_OUTPUT'], 'a') as out:
        out.write('publication_sha256=' + digest + '\n')
    # Preserve the actual checked source for local continuation, even before publication.
    run('git', '-c', 'user.name=github-actions[bot]', '-c', 'user.email=41898282+github-actions[bot]@users.noreply.github.com', '-c', 'core.hooksPath=/dev/null', 'commit', '--no-gpg-sign', '-m', request['message'])
    (EVIDENCE / 'tested-tree.txt').write_text(value['tree'] + '\n')
    (EVIDENCE / 'candidate-revision.txt').write_text(git('rev-parse', 'HEAD').decode())
    run('git', 'bundle', 'create', str(EVIDENCE / 'source.bundle'), 'HEAD', '^' + request['base'])


def api(method, path, body=None):
    assert path.startswith('/git/'), path
    request = urllib.request.Request('https://api.github.com/repos/' + REPO + path,
        data=None if body is None else json.dumps(body).encode(), method=method,
        headers={'Authorization': 'Bearer ' + os.environ['GH_TOKEN'], 'Accept': 'application/vnd.github+json', 'Content-Type': 'application/json', 'X-GitHub-Api-Version': '2022-11-28'})
    with urllib.request.urlopen(request, timeout=45) as response:
        return json.load(response)


def publish():
    head = require_context()
    path = BUNDLE / 'publication.json'
    assert not path.is_symlink() and 0 < path.stat().st_size <= LIMIT
    data = path.read_bytes()
    assert hashlib.sha256(data).hexdigest() == os.environ['PUBLICATION_SHA256']
    value = json.loads(data)
    assert set(value) == {'parent', 'tree', 'message', 'entries'} and value['parent'] == head
    assert value['message'] == read_request()['message']
    assert re.fullmatch('[0-9a-f]{40}', value['tree'])
    entries = value['entries']
    assert 0 < len(entries) <= 150 and len({entry['path'] for entry in entries}) == len(entries)
    ref = '/git/refs/heads/' + BRANCH
    read_ref = '/git/ref/heads/' + BRANCH
    assert api('GET', read_ref)['object']['sha'] == head, 'Session branch advanced. No write performed.'
    tree_entries = []
    for entry in entries:
        assert set(entry) == {'path', 'mode', 'sha', 'content'} and source_path(entry['path'])
        assert entry['mode'] in {'100644', '100755'}
        content = base64.b64decode(entry['content'], validate=True)
        assert len(content) <= 2 * 1024 * 1024
        content.decode('utf-8')
        assert hashlib.sha1(b'blob ' + str(len(content)).encode() + b'\0' + content).hexdigest() == entry['sha']
        blob = api('POST', '/git/blobs', {'content': entry['content'], 'encoding': 'base64'})
        assert blob['sha'] == entry['sha']
        tree_entries.append({'path': entry['path'], 'mode': entry['mode'], 'type': 'blob', 'sha': entry['sha']})
    base_tree = api('GET', '/git/commits/' + head)['tree']['sha']
    tree = api('POST', '/git/trees', {'base_tree': base_tree, 'tree': tree_entries})
    assert tree['sha'] == value['tree'], 'Published tree does not match checked tree'
    commit = api('POST', '/git/commits', {'message': value['message'], 'tree': tree['sha'], 'parents': [head]})
    assert api('GET', read_ref)['object']['sha'] == head, 'Session advanced during object creation. Ref not moved.'
    api('PATCH', ref, {'sha': commit['sha'], 'force': False})
    assert api('GET', read_ref)['object']['sha'] == commit['sha']
    print(json.dumps({'published': commit['sha'], 'tested_tree': tree['sha'], 'parent': head, 'branch': BRANCH}, indent=2))
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / 'published.json').write_text(json.dumps({'commit': commit['sha'], 'tree': tree['sha'], 'parent': head}, indent=2))


if __name__ == '__main__':
    assert len(sys.argv) == 2 and sys.argv[1] in {'prepare', 'check', 'package', 'publish'}
    globals()[sys.argv[1]]()
