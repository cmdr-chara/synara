#!/usr/bin/env python3
"""Assemble reviewed native source without updating any ref.

Temporary continuation tooling. Exact blob guards preserve concurrent work.
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
}
INPUTS = {
    'crates/synara-app/src/ui.rs': 'f2ba4390813f68e2374daf0ba4d3cc6e34329ca7',
    'crates/synara-app/src/shell/navigation.rs': 'b46d84e45f4916d332129c5fba90ff52b1a7de6a',
    'crates/synara-app/src/shell/chrome.rs': '95dd558ad8230a651c4a037927407001be74cf8b',
    'scripts/native_navigation_smoke.py': '1b1a60142b670778dfd99273211d6efe89cb059a',
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
        assert not path.exists(), 'new path already exists: ' + name
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

    p = 'crates/synara-app/src/shell/navigation.rs'
    text = Path(p).read_text()
    text = replace(text, 'use super::*;\nuse crate::ui::{self, DARK, Glyph};',
        'use super::*;\nuse crate::ui::{self, DARK, Glyph};\nuse gpui::FocusHandle;')
    text = replace(text, 'Some(Glyph::Chevron), self.navigation.projects_open,',
        'Some(if self.navigation.projects_open { Glyph::Chevron } else { Glyph::ChevronRight }), false,')
    text = replace(text, 'Some(Glyph::Chevron), self.navigation.chats_open,',
        'Some(if self.navigation.chats_open { Glyph::Chevron } else { Glyph::ChevronRight }), false,')
    text = replace(text, '                            if this.project.is_some() {',
        '                            this.navigation.task_page = 0;\n                            if this.project.is_some() {')
    write(p, text)

    p = 'crates/synara-app/src/main.rs'
    write(p, replace(Path(p).read_text(), 'mod shell;\n', 'mod shell;\nmod ui;\n'))
    p = 'crates/synara-app/src/input.rs'
    write(p, replace(Path(p).read_text(), '.track_focus(&self.focus)',
        '.track_focus(&self.focus)\n            .tab_index(0)'))
    p = 'crates/synara-app/src/shell.rs'
    text = replace(Path(p).read_text(), 'mod conversation;\n',
        'mod chrome;\nmod conversation;\nmod navigation;\n')
    text = replace(text, 'pub struct Shell {\n',
        'pub struct Shell {\n    navigation: navigation::NavigationState,\n')
    text = replace(text, '        let mut this = Self {\n',
        '        let mut this = Self {\n            navigation: navigation::NavigationState::new(cx),\n')
    assert text.count('impl Render for Shell {') == 1
    text = text[:text.index('impl Render for Shell {')].rstrip() + '\n'
    start, end = text.index('\nfn button('), text.index('\nasync fn remote_filesystem(')
    assert start < end
    text = text[:start] + '\nfn button(\n    id: impl Into<gpui::ElementId>,\n    text: impl Into<SharedString>,\n    active: bool,\n) -> gpui::Stateful<gpui::Div> {\n    crate::ui::button(id, text, active)\n}\n' + text[end:]
    write(p, text)
    p = 'crates/synara-app/src/shell/conversation.rs'
    text = Path(p).read_text()
    start, end = text.index('    pub(super) fn sidebar('), text.index('    pub(super) fn conversation(')
    assert start < end
    write(p, text[:start] + text[end:])

    p = 'scripts/native_navigation_smoke.py'
    text = Path(p).read_text()
    # Check the exact persisted user prompt, not arbitrary text in serialized JSON.
    # This fixture has no private user input. A mismatch prints only its synthetic prompt.
    text = replace(text, "    assert 'saved draft' in json.dumps(scenario.events()[before:])",
        "    submitted = [e.get('text') for e in scenario.events()[before:] if e.get('type') == 'text_delta' and e.get('role') == 'user']\n    assert submitted == ['saved draft'], f'Synthetic restored draft mismatch: {submitted!r}'")
    write(p, text)
    p = '.github/workflows/native.yml'
    extra = '\n'.join([
        '      - name: Native navigation and backend interaction smoke',
        '        timeout-minutes: 4',
        '        run: python3 scripts/native_navigation_smoke.py --binary target/debug/synara-app --fixture target/debug/synara-acp-fixture --output /tmp/synara-navigation-smoke',
        '      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02',
        '        if: always()',
        '        with:',
        '          name: native-navigation-smoke',
        '          path: |',
        '            /tmp/synara-navigation-smoke/*.png',
        '            /tmp/synara-navigation-smoke/result.json',
        '          retention-days: 7',
        '          if-no-files-found: warn',
    ]) + '\n'
    write(p, replace(Path(p).read_text(), '      - name: Export reproducible verification inputs\n',
        extra + '      - name: Export reproducible verification inputs\n'))
    p = 'ROADMAP.md'
    text = Path(p).read_text()
    note = '\n\nNative navigation continuation (September 19, 2026): the published-source shell has a modular navigation/design foundation, backend-owned project/chat actions and a native keyboard-menu regression in regular CI. This is a partial checkpoint, not recovery of the unpublished local UI or a visual-parity claim. See `docs/ui/native-navigation.md` and the exact-candidate evidence receipt. Existing unfinished roadmap items remain open.\n'
    assert 'Native navigation continuation (September 19, 2026)' not in text
    write(p, text.rstrip() + note)
    Path('.github/workflows/ui-checkpoint.yml').unlink()
    Path(__file__).unlink()


if __name__ == '__main__':
    main()
