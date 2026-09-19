#!/usr/bin/env python3
"""Assemble exact native source objects in an isolated candidate checkout.

No ref updates. Existing-file blob guards preserve concurrent user work.
"""
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
    '.github/workflows/ui-recovery-audit.yml': '185e9497cd0c923a2afee441b6d4af6d4db25840',
}
INPUTS = {
    'crates/synara-app/src/main.rs': '99f860d0b8d6cc3734cca2f9aaaa3fd4100e2dea',
    'crates/synara-app/src/shell.rs': '0d2bce5172ba0b133661fe88f350c14e6db3ec9f',
    'crates/synara-app/src/input.rs': '5e8a677123cddf3a72f3a122399a458f2c151a4e',
    'crates/synara-app/src/shell/conversation.rs': '9138b9d86c81adca913c7817a9b399a37352ebf2',
    'crates/synara-app/src/ui.rs': '7462cbaf2acc7edf08565ac96856d3ac8eb48b66',
    'crates/synara-app/src/shell/navigation.rs': '9c4c17efd1d3ecc33ff8aef5a29d52a0217e8bf1',
    'crates/synara-app/src/shell/chrome.rs': 'fe002623909c2d47a7b1f092f5c554085224688f',
    'scripts/native_navigation_smoke.py': '6ea0af9987cec3aa10bb914e52e4100c87294669',
    '.github/workflows/native.yml': '87ee65a4575f104d3feabcc9a0417cb924fe0600',
    'ROADMAP.md': '6afa4a5886e74170e012c54a237631ea61b9a22a',
    'docs/ui/native-navigation.md': '3db2250b966aa004bcc3babcd7d5efd22f0e1a93',
}


def git(*args):
    return subprocess.check_output(['git', *args], text=True).strip()


def blob_sha(data):
    return hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()


def replace(text, old, new):
    if text.count(old) != 1:
        raise RuntimeError('source anchor is not unique: ' + old[:80])
    return text.replace(old, new, 1)


def write(path, text):
    Path(path).write_text(text, encoding='utf-8', newline='')


def main():
    assert os.environ['GITHUB_REPOSITORY'] == 'cmdr-chara/synara'
    assert os.environ['GITHUB_REF'] == 'refs/heads/astra/gpui-clean-rewrite'
    assert git('rev-parse', 'HEAD') == os.environ['GITHUB_SHA']
    assert git('rev-list', '--max-parents=0', 'HEAD') == '43b1fb89bf19dadc388d18008f9ceb21b8215716'
    evidence = Path(os.environ['RUNNER_TEMP']) / 'ui-evidence'
    evidence.mkdir(exist_ok=True)
    write(evidence / 'inputs.json', json.dumps(INPUTS, indent=2) + '\n')
    for name, sha in EXPECTED.items():
        assert blob_sha(Path(name).read_bytes()) == sha, 'source changed: ' + name
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

    # Non-content debug metadata distinguishes missed hit targets from state loss.
    # No typed characters, key sequences, paths or credentials are logged.
    p = 'crates/synara-app/src/input.rs'
    text = Path(p).read_text()
    text = replace(text, '        self.bounds = bounds;',
        '        if self.bounds != bounds {\n            tracing::debug!(target: "synara_ui_layout", composer = self.mode == EntryMode::Composer, editor = self.mode == EntryMode::Editor, ?bounds, "input-layout");\n        }\n        self.bounds = bounds;')
    text = replace(text, '                    window.focus(&this.focus, cx);',
        '                    tracing::debug!(target: "synara_ui_layout", composer = this.mode == EntryMode::Composer, editor = this.mode == EntryMode::Editor, position = ?event.position, "input-mouse-focus");\n                    window.focus(&this.focus, cx);')
    text = replace(text, '        self.buffer = TextBuffer::new(text);',
        '        tracing::debug!(target: "synara_ui_layout", composer = self.mode == EntryMode::Composer, editor = self.mode == EntryMode::Editor, empty = text.is_empty(), "input-replaced");\n        self.buffer = TextBuffer::new(text);')
    write(p, text)
    p = 'crates/synara-app/src/shell.rs'
    text = Path(p).read_text()
    text = replace(text, '        let dirty = self.dirty(cx);\n        if self.close.request(dirty, self.saving) {',
        '        let dirty = self.dirty(cx);\n        tracing::debug!(target: "synara_ui_layout", dirty, saving = self.saving, document = self.document.is_some(), "close-request");\n        if self.close.request(dirty, self.saving) {')
    write(p, text)

    p = 'scripts/native_smoke.py'
    text = Path(p).read_text()
    method = '''    def copy_input(self):
        self.key('a', ('Control_L',))
        self.key('c', ('Control_L',))
        env = {key: os.environ[key] for key in ('PATH', 'LD_LIBRARY_PATH') if key in os.environ}
        env['DISPLAY'] = self.name
        result = subprocess.run(['xclip', '-selection', 'clipboard', '-out'], env=env,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=3)
        if result.returncode:
            raise AssertionError('Owned native input did not publish a clipboard selection')
        assert len(result.stdout) <= 1024 * 1024
        return result.stdout.decode('utf-8')

'''
    text = replace(text, '    def text(self, value):\n', method + '    def text(self, value):\n')
    text = replace(text, "RUST_LOG='synara=info,gpui=warn'", "RUST_LOG='synara=info,synara_ui_layout=debug,gpui=warn'")
    old = "        ui.click(850, 190)\n        ui.key('a', ('Control_L',))\n        ui.text('edited text\\n')"
    new = "        ui.click(850, 190)\n        loaded = ui.copy_input()\n        assert loaded == 'original text\\n', f'Editor was not loaded before edit: {loaded!r}'\n        ui.text('edited text\\n')\n        edited = ui.copy_input()\n        assert edited == 'edited text\\n', f'Editor did not receive synthetic edit: {edited!r}'\n        ui.screenshot('editor-before-close')"
    text = replace(text, old, new)
    # Failure-only diagnostics are restricted to the non-content metadata target.
    text = replace(text, "        result['error'] = str(error)\n", "        result['error'] = str(error)\n        if scenario.log:\n            print('\\n'.join(line for line in Path(scenario.log.name).read_text(errors='replace').splitlines() if 'synara_ui_layout' in line)[-12000:])\n")
    write(p, text)
    p = 'scripts/native_navigation_smoke.py'
    text = Path(p).read_text()
    text = replace(text, "    ui.text('saved draft')\n", "    ui.text('saved draft')\n    initial_draft = ui.copy_input()\n    assert initial_draft == 'saved draft', f'Composer did not receive the initial draft: {initial_draft!r}'\n")
    text = replace(text, "    before = len(scenario.events())\n    ui.key('Return')", "    restored = ui.copy_input()\n    assert restored == 'saved draft', f'Synthetic draft not restored: {restored!r}'\n    before = len(scenario.events())\n    ui.key('Return')")
    text = replace(text, "        result['error'] = str(error)\n", "        result['error'] = str(error)\n        if scenario.log:\n            print('\\n'.join(line for line in Path(scenario.log.name).read_text(errors='replace').splitlines() if 'synara_ui_layout' in line)[-12000:])\n")
    write(p, text)
    p = '.github/workflows/native.yml'
    text = Path(p).read_text()
    text = replace(text, 'python3-pil', 'python3-pil xclip')
    write(p, text)
    Path('.github/workflows/ui-checkpoint.yml').unlink()
    Path('.github/workflows/ui-recovery-audit.yml').unlink()
    Path(__file__).unlink()


if __name__ == '__main__':
    main()
