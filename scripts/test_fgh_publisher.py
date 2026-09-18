#!/usr/bin/env python3
"""Regression checks for the session-only source transport adapter."""
import base64
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('fgh_publisher', Path(__file__).with_name('apply_fgh_source.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PublisherTests(unittest.TestCase):
    def test_scope_excludes_peer_subsystems_and_protected_configuration(self):
        for name in ('crates/synara-agent/src/lib.rs', 'crates/synara-acp/src/lib.rs',
                     'crates/synara-runtime/src/filesystem.rs', '.github/workflows/native.yml',
                     'scripts/apply_source.py', 'scripts/apply_fgh_source.py',
                     'crates/synara-app/src/shell/conversation.rs', '.git/config'):
            self.assertFalse(module.permitted(name), name)
        for name in ('crates/synara-workspace/src/storage.rs', 'crates/synara-app/src/fgh/files.rs',
                     'docs/fgh-handoff.md', 'ROADMAP.md'):
            self.assertTrue(module.permitted(name), name)

    def test_wrong_branch_is_rejected_before_running_git(self):
        for branch in ('refs/heads/main', 'refs/heads/astra/gpui-clean-rewrite',
                       'refs/heads/astra/session-bcd', 'refs/heads/astra/session-ajm'):
            with patch.dict(os.environ, {'GITHUB_REPOSITORY': 'cmdr-chara/synara', 'GITHUB_REF': branch}), patch.object(module.subprocess, 'run') as run:
                with self.assertRaises(SystemExit):
                    module.main()
                run.assert_not_called()

    def test_wrong_repository_is_rejected(self):
        with patch.dict(os.environ, {'GITHUB_REPOSITORY': 'unrelated/repository', 'GITHUB_REF': module.BRANCH}), patch.object(module.subprocess, 'run') as run:
            with self.assertRaises(SystemExit):
                module.main()
            run.assert_not_called()

    def test_exact_edits_are_integrity_checked_data(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / 'code.rs'
            target.write_text('before\nUnicode 😀\n', encoding='utf-8')
            package = {'base_commit': 'a' * 40, 'mode': 'patch', 'message': 'test change',
                       'files': {'new.rs': 'new\n'}, 'edits': {'code.rs': [['before', 'after']]}}
            normalized = module.normalize_text_package(package, lambda name: root / name)
            decoded = gzip.decompress(base64.b64decode(normalized['gzip_base64'], validate=True))
            self.assertEqual(hashlib.sha256(decoded).hexdigest(), normalized['sha256'])
            self.assertEqual(json.loads(decoded), {'new.rs': 'new\n', 'code.rs': 'after\nUnicode 😀\n'})
            self.assertEqual(target.read_text(encoding='utf-8'), 'before\nUnicode 😀\n')
            self.assertEqual(normalized['base_commit'], package['base_commit'])

    def test_nonunique_and_missing_edits_fail_without_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / 'code.rs'
            target.write_text('twice twice', encoding='utf-8')
            for old in ('twice', 'absent', ''):
                with self.assertRaises(SystemExit):
                    module.normalize_text_package({'edits': {'code.rs': [[old, 'replacement']]}}, lambda name: root / name)
                self.assertEqual(target.read_text(), 'twice twice')
            with self.assertRaises(SystemExit):
                module.normalize_text_package({'edits': {'missing.rs': [['old', 'new']]}}, lambda name: root / name)

    def test_snapshot_ambiguous_and_overlapping_packages_fail(self):
        packages = [
            {'mode': 'snapshot', 'files': {'a': 'b'}},
            {'files': {}, 'gzip_base64': ''},
            {'files': {}, 'parts': []},
            {'files': [], 'edits': {}},
            {'files': {'a': 'x'}, 'edits': {'a': [['x', 'y']]}},
        ]
        for package in packages:
            with self.assertRaises(SystemExit):
                module.normalize_text_package(package, Path)


if __name__ == '__main__':
    unittest.main()
