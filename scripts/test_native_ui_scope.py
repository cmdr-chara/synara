#!/usr/bin/env python3
"""Regression checks for fail-closed native UI test selection."""
import subprocess
import unittest
from unittest.mock import patch

from native_ui_scope import git_scope, scope_for_paths, kanban_only, environment_only


class NativeScopeTests(unittest.TestCase):
    def test_known_presentation_delta_uses_focused_lane(self):
        self.assertEqual(scope_for_paths([
            'crates/synara-app/src/ui/menu.rs',
            'crates/synara-app/src/ui/markdown.rs',
            'scripts/native_picker_search_smoke.py',
            'ROADMAP.md',
        ]), 'presentation')

    def test_documentation_does_not_rebuild_the_application(self):
        self.assertEqual(scope_for_paths(['ROADMAP.md', 'docs/ui/parity-9488759.md']), 'docs')

    def test_backend_dependency_input_or_unknown_changes_use_full_lane(self):
        for path in [
            'crates/synara-agent/src/lib.rs', 'Cargo.lock', 'Cargo.toml',
            'rust-toolchain.toml', 'crates/synara-runtime/src/lib.rs',
            '.github/workflows/security.yml', 'unrecognized.rs', '',
        ]:
            with self.subTest(path=path):
                self.assertEqual(scope_for_paths([
                    'crates/synara-app/src/ui/menu.rs', path,
                ]), 'full')

    def test_chat_preferences_and_input_have_expanded_focused_coverage(self):
        for path in ['crates/synara-app/src/input.rs', 'crates/synara-app/src/shell.rs',
                     'crates/synara-workspace/src/storage.rs', 'crates/synara-workspace/src/settings/chat.rs',
                     'scripts/native_smoke.py', 'scripts/native_chat_behavior_smoke.py']:
            with self.subTest(path=path):
                self.assertEqual(scope_for_paths([path]), 'presentation')

    def test_complete_kanban_slice_uses_creation_and_modal_journeys(self):
        paths = ['crates/synara-app/src/shell/kanban.rs',
                 'crates/synara-workspace/src/service.rs',
                 'crates/synara-workspace/src/storage/task_creation.rs',
                 'scripts/test_native_kanban_smoke.py', 'ROADMAP.md']
        self.assertEqual(scope_for_paths(paths), 'presentation')
        self.assertTrue(kanban_only(paths))

    def test_mixed_kanban_changes_retain_broader_ui_acceptance(self):
        for extra in ['crates/synara-app/src/ui/menu.rs',
                      'crates/synara-app/src/ui/markdown.rs',
                      'crates/synara-app/src/input.rs',
                      'crates/synara-workspace/src/settings/chat.rs']:
            self.assertFalse(kanban_only(['crates/synara-app/src/shell/kanban.rs', extra]))

    def test_no_semantic_task_change_cannot_select_narrow_journeys(self):
        for paths in [[], ['ROADMAP.md'], ['.github/workflows/ui-presentation.yml'],
                      ['crates/synara-workspace/src/service.rs'], ['unknown.rs'],
                      ['crates/synara-app/src/shell/kanban.rs', 'unknown.rs']]:
            self.assertFalse(kanban_only(paths))
        self.assertEqual(scope_for_paths(['crates/synara-workspace/src/service.rs']), 'full')

    def test_environment_slice_has_its_own_bounded_lane(self):
        paths = ['crates/synara-app/src/shell/environment.rs',
                 'crates/synara-workspace/src/environment.rs',
                 'crates/synara-app/src/main.rs',
                 'crates/synara-workspace/src/lib.rs',
                 'scripts/native_environment_smoke.py', 'ROADMAP.md']
        self.assertTrue(environment_only(paths))
        self.assertEqual(scope_for_paths(paths), 'environment')

    def test_mixed_environment_changes_cannot_skip_other_ui_checks(self):
        paths = ['crates/synara-app/src/shell/environment.rs', 'crates/synara-app/src/ui/menu.rs']
        self.assertFalse(environment_only(paths))
        self.assertEqual(scope_for_paths(paths), 'presentation')
        for unknown in ['Cargo.lock', 'crates/synara-workspace/src/controller.rs', 'unknown.rs']:
            self.assertFalse(environment_only(paths + [unknown]))
            self.assertEqual(scope_for_paths(paths + [unknown]), 'full')

    def test_bootstrap_or_export_changes_alone_are_not_environment_only(self):
        for path in ['crates/synara-app/src/main.rs', 'crates/synara-workspace/src/lib.rs',
                     '.github/workflows/ui-environment.yml']:
            self.assertFalse(environment_only([path]))
            self.assertEqual(scope_for_paths([path]), 'full')

    def test_empty_diff_is_not_assumed_verified(self):
        self.assertEqual(scope_for_paths([]), 'full')

    def test_invalid_or_initial_commit_is_not_sent_to_git(self):
        with patch('native_ui_scope.subprocess.run') as run:
            for value in ['--help', '', '0' * 40, 'a' * 39, 'not-a-commit']:
                self.assertEqual(git_scope(value, 'b' * 40), 'full')
            run.assert_not_called()

    def test_missing_history_falls_back_to_full(self):
        with patch('native_ui_scope.subprocess.run', side_effect=subprocess.CalledProcessError(1, 'git')):
            self.assertEqual(git_scope('a' * 40, 'b' * 40), 'full')

    def test_nul_delimiters_do_not_split_newline_filenames(self):
        result = subprocess.CompletedProcess('git', 0, stdout=b'unknown\nROADMAP.md\0')
        with patch('native_ui_scope.subprocess.run', return_value=result):
            self.assertEqual(git_scope('a' * 40, 'b' * 40), 'full')


if __name__ == '__main__':
    unittest.main()
