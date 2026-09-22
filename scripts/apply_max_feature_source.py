#!/usr/bin/env python3
"""Last session-only metadata publisher. Deletes itself before committing."""
import json
import os
from pathlib import Path
import re
import runpy
import subprocess

BRANCH = 'refs/heads/astra/max-feature-sprint'
ACCEPTED = '16789f4b290268bd54f315855cccae479aab8b2f'
BASE = '33fa0a42dd5ac1098a7ba5f1e7bd5b240abbf577'
DOCS = {
    'README.md', 'ROADMAP.md', 'docs/database-recovery.md', 'docs/integrations.md',
    'docs/ui/electron-vs-gpui-feature-gap.md', 'docs/ui/direct-models.md',
    'docs/ui/project-import.md', 'docs/ui/provider-handoff.md',
    'docs/verification/max-feature-sprint.md',
}
REMOVE = ['.github/workflows/max-feature-sprint.yml', 'scripts/apply_max_feature_source.py']

def run(*args):
    print('+', ' '.join(args), flush=True)
    subprocess.run(args, check=True)

def git(*args):
    return subprocess.check_output(['git', *args], text=True).strip()

def validate_metadata():
    run('git', 'diff', '--exit-code', ACCEPTED, '--', 'Cargo.toml', 'Cargo.lock', 'crates', ':(glob)scripts/native*.py')
    run('python3', 'scripts/check_roadmap.py')
    run('python3', 'scripts/check_roadmap.py', '--self-test')
    for test in ['test_audit_workspace.py', 'test_native_ui_scope.py', 'test_ci_router.py', 'test_apply_source.py']:
        run('python3', 'scripts/' + test)
    report = runpy.run_path('scripts/audit_workspace.py')['audit'](Path.cwd())
    print('Legacy structural audit, including declared baseline failure:', json.dumps(report), flush=True)
    baseline_path = 'crates/synara-browser/src/native/actions.js'
    assert report['errors'] == [baseline_path + ': non-Rust core source']
    assert git('rev-parse', BASE + ':' + baseline_path) == git('hash-object', baseline_path)
    def bodies(text):
        return re.findall(r'(?m)^- \[[ x]\] [A-Q][0-9]+ .*(?:\n[ \t]+[^\n]*)*', text)
    old = bodies(git('show', ACCEPTED + ':ROADMAP.md'))
    new = bodies(Path('ROADMAP.md').read_text())
    assert len(old) == 120 and old == new, 'Historical task bodies changed'
    for name in DOCS:
        for href in re.findall(r'\]\(([^)]+)\)', Path(name).read_text()):
            href = href.split('#', 1)[0]
            if href and ':' not in href and not href.startswith('/'):
                assert (Path(name).parent / href).exists(), (name, href)
    workflow = '.github/workflows/direct-conversations.yml'
    assert git('hash-object', workflow) == 'e4f04be2f729d8efe8323b4db2fdf1880cdbf3e4'
    text = Path(workflow).read_text()
    assert 'contents: read' in text and 'persist-credentials: false' in text
    assert 'workflow_call:' in text and 'workflow_dispatch:' in text and '\n  push:' not in text
    run('git', 'diff', '--check')
    print('Metadata verified. Executable inputs exactly match accepted source. 120 original task bodies retained.', flush=True)

def main():
    if os.environ.get('GITHUB_REPOSITORY') != 'cmdr-chara/synara' or os.environ.get('GITHUB_REF') != BRANCH:
        raise SystemExit('Wrong repository or session branch')
    run('git', 'merge-base', '--is-ancestor', BASE, 'HEAD')
    run('git', 'merge-base', '--is-ancestor', ACCEPTED, 'HEAD')
    scope = runpy.run_path('scripts/apply_source.py')['main'].__globals__
    source_path, apply_diff = scope['source_path'], scope['apply_diff']
    def checked_path(name):
        if name not in DOCS:
            raise SystemExit('Outside reviewed documentation scope: ' + str(name))
        return source_path(name)
    def validated_diff(data):
        apply_diff(data)
        changed = set(git('diff', 'HEAD', '--name-only').splitlines())
        assert changed == DOCS, 'Incomplete or unexpected documentation diff'
        validate_metadata()
        for name in REMOVE:
            path = Path(name)
            assert path.is_file() and not path.is_symlink()
            path.unlink()
        run('git', 'add', '--', *REMOVE)
    scope.update(BRANCH=BRANCH, PAYLOAD=Path.cwd() / '.synara-max-feature.json', source_path=checked_path, apply_diff=validated_diff)
    scope['main']()
    assert not git('status', '--porcelain'), 'Final checkout is dirty'
    assert not any(Path('.').glob('.synara-transfer-*'))
    assert not Path('.synara-max-feature.json').exists()
    assert all(not Path(name).exists() for name in REMOVE)

if __name__ == '__main__':
    main()
