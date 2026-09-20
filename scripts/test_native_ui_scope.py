#!/usr/bin/env python3
"""Regression checks for fail-closed native UI test selection."""
import subprocess
import unittest
from unittest.mock import patch

from native_ui_scope import git_scope, scope_for_paths, kanban_only, environment_only, terminal_probes_only


class NativeScopeTests(unittest.TestCase):
    def test_git_review_uses_environment_with_its_own_regressions(self):
        paths = [
            'crates/synara-app/src/shell/review.rs',
            'crates/synara-app/src/shell/review/diff.rs',
            'crates/synara-app/src/shell.rs',
            'crates/synara-app/src/shell/panels.rs',
            'crates/synara-workspace/src/storage/review.rs',
            'crates/synara-workspace/src/storage.rs',
            'scripts/native_git_review_smoke.py',
            '.github/workflows/ui-environment.yml',
            'ROADMAP.md',
        ]
        self.assertEqual(scope_for_paths(paths), 'environment')
        for unrelated in ['crates/synara-workspace/src/tools.rs', 'Cargo.lock',
                          'crates/synara-runtime/src/lib.rs']:
            self.assertEqual(scope_for_paths(paths + [unrelated]), 'full')
        self.assertEqual(scope_for_paths(['crates/synara-app/src/shell/panels.rs']), 'full')

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

    def test_terminal_probe_additions_have_a_closed_instruction_set(self):
        diff = '--- a/panels.rs\n+++ b/panels.rs\n@@ -1,0 +2,2 @@\n+ .relative()\n+ .child(crate::ui::layout_probe("start-shell"))\n'
        self.assertTrue(terminal_probes_only(diff))
        for extra in ['- .on_click(callback)', '+ .on_click(callback)', '+ this.start_terminal(cx)',
                      '+ .child(crate::ui::layout_probe("unknown"))']:
            self.assertFalse(terminal_probes_only(diff + extra))
        self.assertFalse(terminal_probes_only(''))

    def test_inert_terminal_probes_allow_the_existing_environment_journey(self):
        result = subprocess.CompletedProcess('git', 0, stdout=(
            b'crates/synara-app/src/shell/panels.rs\0scripts/native_environment_smoke.py\0'))
        diff = b'+ .relative()\n+ .child(crate::ui::layout_probe("start-shell"))\n'
        with patch('native_ui_scope.subprocess.run', return_value=result), \
             patch('native_ui_scope.subprocess.check_output', return_value=diff):
            self.assertEqual(git_scope('a' * 40, 'b' * 40), 'environment')

    def test_panel_behavior_changes_cannot_hide_behind_an_environment_test(self):
        result = subprocess.CompletedProcess('git', 0, stdout=(
            b'crates/synara-app/src/shell/panels.rs\0scripts/native_environment_smoke.py\0'))
        with patch('native_ui_scope.subprocess.run', return_value=result), \
             patch('native_ui_scope.subprocess.check_output', return_value=b'+ this.start_terminal(cx)\n'):
            self.assertEqual(git_scope('a' * 40, 'b' * 40), 'full')

    def test_missing_panel_diff_cannot_reduce_coverage(self):
        result = subprocess.CompletedProcess('git', 0, stdout=b'crates/synara-app/src/shell/panels.rs\0')
        with patch('native_ui_scope.subprocess.run', return_value=result), \
             patch('native_ui_scope.subprocess.check_output', side_effect=subprocess.CalledProcessError(1, 'git')):
            self.assertEqual(git_scope('a' * 40, 'b' * 40), 'full')


class ChatUtilityScopeTests(unittest.TestCase):
    def test_batched_chat_tools_select_one_coherent_lane(self):
        self.assertEqual(scope_for_paths([
            'crates/synara-app/src/shell/chat_tools.rs',
            'crates/synara-workspace/src/storage/conversation_tools.rs',
            'crates/synara-app/src/shell/transcript/search.rs',
            'crates/synara-app/src/shell/environment.rs',
            '.github/workflows/ui-chat-tools.yml',
        ]), 'chat-tools')

    def test_unrelated_backend_or_dependency_changes_still_require_full_lane(self):
        for path in ['Cargo.lock', 'crates/synara-agent/src/lib.rs', 'unknown.rs']:
            self.assertEqual(scope_for_paths(['scripts/native_chat_tools_smoke.py', path]), 'full')


class MarkdownScopeTests(unittest.TestCase):
    def test_markdown_slice_has_an_explicit_anchor_and_fail_closed_boundary(self):
        from native_ui_scope import MARKDOWN_PATHS, markdown_only
        self.assertTrue(markdown_only(MARKDOWN_PATHS | {'docs/ui/parity-alerts.md'}))
        self.assertEqual(scope_for_paths(MARKDOWN_PATHS), 'presentation')
        self.assertFalse(markdown_only({'.github/workflows/ui-presentation.yml'}))
        for extra in ['crates/synara-app/src/ui/menu.rs', 'crates/synara-app/src/shell/review.rs',
                      'crates/synara-workspace/src/storage.rs', 'Cargo.lock', 'unknown.rs']:
            self.assertFalse(markdown_only({'crates/synara-app/src/ui/markdown.rs', extra}))


if __name__ == '__main__':
    unittest.main()
