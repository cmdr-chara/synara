#!/usr/bin/env python3
"""Assemble exact native source in an isolated checkout without updating refs."""
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import urllib.request

EXPECTED = {
    'crates/synara-app/src/main.rs': 'ae7618ef5ca4e1c0eca7b14594cd2171dd9bf973',
    'crates/synara-app/src/shell.rs': '7e0e217b73491823732199a54374a286f13a027f',
    'crates/synara-app/src/input.rs': '6b85843db1cda6372048795400cfbcfd9cc7a77f',
    'crates/synara-app/src/shell/conversation.rs': 'db645ae0bd7a67326df576c1fc902fcafb809113',
    '.github/workflows/native.yml': '6b034788e1273ff60242af93ff2b446d5ea733a5',
    'scripts/native_smoke.py': '49c3ebfb0a206b84e82dacf64bd76a0d57c72dd2',
    'ROADMAP.md': '1d8bd9b6fd30b27099ff3986e0614bc76c4b1fa1',
    '.github/workflows/ui-recovery-audit.yml': 'ef4dd7645f3adce008f993955c36bf3779916627',
}
INPUTS = {
    'crates/synara-app/src/main.rs': '99f860d0b8d6cc3734cca2f9aaaa3fd4100e2dea',
    'crates/synara-app/src/shell.rs': '10dec9591609d0a58e2af9aa8f0821d06dcd164f',
    'crates/synara-app/src/input.rs': '59be67637dfef8fd67ed8efd3d023c05605e66e8',
    'crates/synara-app/src/shell/conversation.rs': '9138b9d86c81adca913c7817a9b399a37352ebf2',
    'crates/synara-app/src/ui.rs': '7462cbaf2acc7edf08565ac96856d3ac8eb48b66',
    'crates/synara-app/src/shell/navigation.rs': '9c4c17efd1d3ecc33ff8aef5a29d52a0217e8bf1',
    'crates/synara-app/src/shell/chrome.rs': '968dc8f36a7f1f49ba00d833467bdadc23a3cb74',
    'scripts/native_navigation_smoke.py': 'e8b50905becaaa01fd4f28f26e24f872a9dfa87f',
    'scripts/native_smoke.py': '9093a935597886d08dc407c828981b9f97e65538',
    '.github/workflows/native.yml': 'f452d27659fd45be9085290714bf3212b5679f96',
    'ROADMAP.md': '6afa4a5886e74170e012c54a237631ea61b9a22a',
    'docs/ui/native-navigation.md': '3db2250b966aa004bcc3babcd7d5efd22f0e1a93',
}


def git(*args):
    return subprocess.check_output(['git', *args], text=True).strip()


def blob_sha(data):
    return hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()


def main():
    assert os.environ['GITHUB_REPOSITORY'] == 'cmdr-chara/synara'
    assert os.environ['GITHUB_REF'] == 'refs/heads/astra/gpui-clean-rewrite'
    assert git('rev-parse', 'HEAD') == os.environ['GITHUB_SHA']
    assert git('rev-list', '--max-parents=0', 'HEAD') == '43b1fb89bf19dadc388d18008f9ceb21b8215716'
    evidence = Path(os.environ['RUNNER_TEMP']) / 'ui-evidence'
    evidence.mkdir(exist_ok=True)
    (evidence / 'inputs.json').write_text(json.dumps(INPUTS, indent=2) + '\n')
    for name, sha in EXPECTED.items():
        assert blob_sha(Path(name).read_bytes()) == sha, 'concurrent source change: ' + name
    for name, sha in INPUTS.items():
        path = Path(name)
        assert name in EXPECTED or not path.exists(), 'new path already exists: ' + name
        request = urllib.request.Request(
            'https://api.github.com/repos/cmdr-chara/synara/git/blobs/' + sha,
            headers={'Accept': 'application/vnd.github+json', 'User-Agent': 'Synara-native-verification'})
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.read(1024 * 1024 + 1)
        assert len(raw) <= 1024 * 1024
        payload = json.loads(raw)
        assert payload['encoding'] == 'base64' and payload['size'] < 128 * 1024
        data = base64.b64decode(payload['content'])
        assert blob_sha(data) == sha
        data.decode('utf-8')
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    Path('.github/workflows/ui-checkpoint.yml').unlink()
    Path('.github/workflows/ui-recovery-audit.yml').unlink()
    Path(__file__).unlink()


if __name__ == '__main__':
    main()
