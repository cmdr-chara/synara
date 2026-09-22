#!/usr/bin/env python3
"""Session-only checked publisher, removed before final integration."""
import os
from pathlib import Path
import runpy
import subprocess

BRANCH = 'refs/heads/astra/max-feature-sprint'
BASE = '33fa0a42dd5ac1098a7ba5f1e7bd5b240abbf577'
EXACT = {
    'crates/synara-app/src/shell.rs',
    'crates/synara-app/src/shell/chat_tools.rs',
    'crates/synara-app/src/shell/chrome.rs',
    'crates/synara-app/src/shell/conversation.rs',
    'crates/synara-app/src/shell/handoff.rs',
    'crates/synara-app/src/shell/revisions.rs',
    'crates/synara-workspace/src/controller.rs',
    'crates/synara-workspace/src/controller/handoff.rs',
    'crates/synara-workspace/src/storage.rs',
    'crates/synara-workspace/src/storage/conversation_tools.rs',
    'crates/synara-workspace/src/storage/conversation_tools/related.rs',
    'crates/synara-workspace/src/storage/conversation_tools/related/handoff.rs',
    'crates/synara-workspace/src/storage/conversation_tools/related/handoff/tests.rs',
    'scripts/native_handoff_smoke.py',
}

def run(*args):
    subprocess.run(args, check=True)

def main():
    if os.environ.get('GITHUB_REPOSITORY') != 'cmdr-chara/synara' or os.environ.get('GITHUB_REF') != BRANCH:
        raise SystemExit('Wrong repository or session ref')
    run('git', 'merge-base', '--is-ancestor', BASE, 'HEAD')
    common = runpy.run_path(str(Path(__file__).with_name('apply_source.py')))
    scope = common['main'].__globals__
    source_path, apply_diff = scope['source_path'], scope['apply_diff']
    def checked_path(name):
        result = source_path(name)
        if name not in EXACT:
            raise SystemExit(f'Outside reviewed handoff slice: {name}')
        return result
    def validated_diff(data):
        apply_diff(data)
        changed = set(subprocess.check_output(['git', 'diff', 'HEAD', '--name-only'], text=True).splitlines())
        if changed != EXACT:
            raise SystemExit('Incomplete or unexpected handoff slice')
        new_rust = [p for p in sorted(EXACT) if p.endswith('.rs') and subprocess.run(['git', 'cat-file', '-e', f'HEAD:{p}'], stderr=subprocess.DEVNULL).returncode]
        run('rustfmt', '+1.98.1', '--check', '--edition', '2024', '--config', 'skip_children=true', *new_rust)
        run('cargo', '+1.98.1', 'test', '--locked', '-p', 'synara-workspace', 'handoff')
        run('cargo', '+1.98.1', 'build', '--locked', '-p', 'synara-app', '--bin', 'synara-app', '-p', 'synara-acp', '--bin', 'synara-acp-fixture')
        for journey, output in [('native_handoff_smoke.py', 'handoff'), ('native_model_draft_smoke.py', 'model-draft-regression')]:
            run('/usr/bin/python3', f'scripts/{journey}', '--binary', 'target/debug/synara-app', '--fixture', 'target/debug/synara-acp-fixture', '--output', f'/tmp/max-feature-evidence/{output}')
        run('git', 'diff', '--check')
    scope.update(BRANCH=BRANCH, PAYLOAD=Path.cwd() / '.synara-max-feature.json', source_path=checked_path, apply_diff=validated_diff)
    scope['main']()

if __name__ == '__main__':
    main()
