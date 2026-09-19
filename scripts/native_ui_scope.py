#!/usr/bin/env python3
"""Select focused native verification only for a closed set of UI-only changes.

Unknown paths, unavailable history, or malformed commit identifiers fail closed
into the full native lane. A documentation-only push needs no native rebuild.
"""
import re
import subprocess
import sys

UI_PATHS = frozenset({
    'crates/synara-app/src/ui/menu.rs',
    'crates/synara-app/src/ui/markdown.rs',
    'scripts/native_picker_search_smoke.py',
    'scripts/native_rich_text_smoke.py',
    'scripts/native_ui_scope.py',
    'scripts/test_native_ui_scope.py',
    '.github/workflows/ui-presentation.yml',
    '.github/workflows/native.yml',
})


def documentation(path):
    return path in {'ROADMAP.md', 'README.md'} or path.startswith('docs/ui/')


def scope_for_paths(paths):
    paths = set(paths)
    if not paths:
        return 'full'
    if all(documentation(path) for path in paths):
        return 'docs'
    if all(path in UI_PATHS or documentation(path) for path in paths):
        return 'presentation'
    return 'full'


def git_scope(base, revision):
    if not all(re.fullmatch(r'[0-9a-f]{40}', value) and value != '0' * 40
               for value in (base, revision)):
        return 'full'
    try:
        # NUL delimiters keep unusual filenames from being treated as new paths.
        result = subprocess.run(
            ['git', 'diff', '--name-only', '--no-renames', '-z', base, revision, '--'],
            check=True, capture_output=True, timeout=30,
        )
        paths = result.stdout.decode('utf-8').rstrip('\0').split('\0')
    except (OSError, UnicodeError, subprocess.SubprocessError):
        return 'full'
    return scope_for_paths(paths)


def main():
    scope = git_scope(*sys.argv[1:]) if len(sys.argv) == 3 else 'full'
    print(f'scope={scope}')


if __name__ == '__main__':
    main()
