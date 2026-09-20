#!/usr/bin/env python3
"""Select bounded native verification from actual changed paths, failing closed."""
import re
import subprocess
import sys

UI_PATHS = frozenset({
    'crates/synara-app/src/input/policy.rs',
    'crates/synara-app/src/shell/chrome.rs',
    'crates/synara-app/src/shell/messages.rs',
    'crates/synara-app/src/shell/settings.rs',
    'crates/synara-app/src/shell/settings/chat.rs',
    'crates/synara-workspace/src/settings.rs',
    'crates/synara-workspace/src/settings/chat.rs',
    'scripts/native_chat_behavior_smoke.py',
    'scripts/native_smoke.py',
    'crates/synara-app/assets/icons/model-picker-manifest.json',
    'crates/synara-app/assets/icons/tabler/star-filled.svg',
    'crates/synara-app/assets/icons/tabler/star.svg',
    'crates/synara-app/src/input.rs',
    'crates/synara-app/src/shell.rs',
    'crates/synara-app/src/shell/controls.rs',
    'crates/synara-app/src/shell/drafts.rs',
    'crates/synara-app/src/shell/navigation.rs',
    'crates/synara-app/src/ui/icons.rs',
    'crates/synara-app/src/ui/menu/models.rs',
    'crates/synara-workspace/src/storage.rs',
    'crates/synara-workspace/src/storage/chat_preferences.rs',
    'scripts/native_controls_smoke.py',
    'scripts/native_model_draft_smoke.py',
    'crates/synara-app/src/ui/menu.rs',
    'crates/synara-app/src/ui/markdown.rs',
    'scripts/native_picker_search_smoke.py',
    'scripts/native_rich_text_smoke.py',
    'scripts/native_ui_scope.py',
    'scripts/test_native_ui_scope.py',
    '.github/workflows/ui-presentation.yml',
    '.github/workflows/native.yml',
    'crates/synara-app/src/ui.rs',
    'crates/synara-app/src/shell/overview.rs',
    'crates/synara-app/src/shell/kanban.rs',
    'crates/synara-app/src/ui/task_dialog.rs',
    'crates/synara-workspace/src/storage/task_creation.rs',
    'scripts/native_kanban_smoke.py',
    'scripts/prepare_kanban_checkpoint.py',
})
KANBAN_CREATION = frozenset({
    'crates/synara-app/src/shell/kanban.rs',
    'crates/synara-workspace/src/storage/task_creation.rs',
})


def documentation(path):
    return path in {'ROADMAP.md', 'README.md'} or path.startswith('docs/ui/')


def scope_for_paths(paths):
    paths = set(paths)
    if not paths:
        return 'full'
    if all(documentation(path) for path in paths):
        return 'docs'
    allowed = UI_PATHS
    # The task-creation caller is covered by this coherent slice. Unrelated
    # service-only work must still use the full lane, not UI smoke alone.
    if paths & KANBAN_CREATION:
        allowed = allowed | {'crates/synara-workspace/src/service.rs'}
    if all(path in allowed or documentation(path) for path in paths):
        return 'presentation'
    return 'full'


def git_scope(base, revision):
    if not all(re.fullmatch(r'[0-9a-f]{40}', value) and value != '0' * 40
               for value in (base, revision)):
        return 'full'
    try:
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
