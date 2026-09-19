#!/usr/bin/env python3
"""Regression checks for fail-closed native UI test selection."""
import subprocess
import unittest
from unittest.mock import patch

from native_ui_scope import git_scope, scope_for_paths


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
            'crates/synara-app/src/input.rs', 'crates/synara-app/src/shell.rs',
            'rust-toolchain.toml', 'scripts/native_smoke.py',
            '.github/workflows/security.yml', 'unrecognized.rs', '',
        ]:
            with self.subTest(path=path):
                self.assertEqual(scope_for_paths([
                    'crates/synara-app/src/ui/menu.rs', path,
                ]), 'full')

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
