#!/usr/bin/env python3
"""Session-only checked publisher, removed before final integration."""
import os
from pathlib import Path
import runpy
import subprocess

BRANCH = 'refs/heads/astra/max-feature-sprint'
BASE = '33fa0a42dd5ac1098a7ba5f1e7bd5b240abbf577'
EXACT = {
    'crates/synara-app/src/shell/direct_models.rs',
    'crates/synara-app/src/shell/direct_models/view.rs',
    'crates/synara-model/src/catalog.rs',
    'crates/synara-model/src/config.rs',
    'crates/synara-model/src/google.rs',
    'crates/synara-model/src/google/tests.rs',
    'crates/synara-model/src/lib.rs',
    'crates/synara-model/src/protocol.rs',
    'crates/synara-model/src/schema.rs',
    'crates/synara-model/src/stream.rs',
    'crates/synara-model/src/tests.rs',
    'crates/synara-model/src/transport.rs',
    'scripts/native_google_models_smoke.py',
}

def run(*args):
    print('+', ' '.join(args), flush=True)
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
            raise SystemExit(f'Outside reviewed model slice: {name}')
        return result
    def validated_diff(data):
        apply_diff(data)
        changed = set(subprocess.check_output(['git', 'diff', 'HEAD', '--name-only'], text=True).splitlines())
        if changed != EXACT:
            raise SystemExit('Incomplete or unexpected model slice')
        new_rust = [p for p in sorted(EXACT) if p.endswith('.rs') and subprocess.run(['git', 'cat-file', '-e', f'HEAD:{p}'], stderr=subprocess.DEVNULL).returncode]
        run('rustfmt', '+1.98.1', '--check', '--edition', '2024', '--config', 'skip_children=true', *new_rust)
        run('cargo', '+1.98.1', 'test', '--locked', '--workspace', '--exclude', 'synara-app')
        run('cargo', '+1.98.1', 'build', '--locked', '-p', 'synara-app', '--bin', 'synara-app', '-p', 'synara-acp', '--bin', 'synara-acp-fixture')
        journeys = [
            ('native_google_models_smoke.py', 'google-models'),
            ('native_direct_models_smoke.py', 'direct-models'),
            ('native_project_import_smoke.py', 'project-import'),
            ('native_handoff_smoke.py', 'handoff'),
            ('native_model_draft_smoke.py', 'model-draft-regression'),
            ('native_browser_webview_smoke.py', 'browser-regression'),
            ('native_integrations_smoke.py', 'integrations-regression'),
        ]
        for journey, output in journeys:
            run('/usr/bin/python3', f'scripts/{journey}', '--binary', 'target/debug/synara-app', '--fixture', 'target/debug/synara-acp-fixture', '--output', f'/tmp/max-feature-evidence/{output}')
        run('git', 'diff', '--check')
    scope.update(BRANCH=BRANCH, PAYLOAD=Path.cwd() / '.synara-max-feature.json', source_path=checked_path, apply_diff=validated_diff)
    scope['main']()

if __name__ == '__main__':
    main()
