#!/usr/bin/env python3
"""Regression checks for the isolated source publisher's ownership boundary."""
import os
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch
import apply_bcd_source as publisher


class BcdPublisherTests(unittest.TestCase):
    def test_checked_single_and_partitioned_diff_packages(self):
        publisher.validate_package({'mode': 'diff', 'gzip_base64': 'bounded-data'})
        publisher.validate_package({'mode': 'diff', 'parts': ['.synara-transfer-0001']})
        for package in (
            {'mode': 'snapshot', 'gzip_base64': 'data'},
            {'mode': 'diff'},
            {'mode': 'diff', 'parts': []},
            {'mode': 'diff', 'parts': ['.synara-transfer-0001'] * 2},
            {'mode': 'diff', 'parts': ['../escape']},
            {'mode': 'diff', 'parts': [None]},
            {'mode': 'diff', 'parts': ['.synara-transfer-0001'], 'gzip_base64': 'data'},
            {'mode': 'diff', 'gzip_base64': 'data', 'delete': ['ROADMAP.md']},
        ):
            with self.subTest(package=package), self.assertRaises(SystemExit):
                publisher.validate_package(package)


    def test_owned_and_narrow_shared_paths(self):
        for name in ('crates/synara-agent/src/interaction.rs',
                     'crates/synara-acp/src/callbacks.rs',
                     'crates/synara-core/src/thread.rs',
                     'crates/synara-workspace/src/controller.rs',
                     'crates/synara-app/src/shell/conversation.rs',
                     'docs/bcd-session-handoff.md', 'ROADMAP.md'):
            with self.subTest(name=name):
                self.assertTrue(publisher.allowed(name))

    def test_other_lane_and_repository_paths_are_rejected(self):
        for name in ('crates/synara-runtime/src/lib.rs',
                     'crates/synara-acp/tests/ssh_live.rs',
                     'crates/synara-app/src/shell/panels.rs',
                     '.github/workflows/ssh.yml', '.github/workflows/bcd.yml',
                     'scripts/apply_source.py', 'Cargo.lock',
                     '.synara-bcd-source.json', 'crates/synara-acp/../synara-runtime/a.rs',
                     '/crates/synara-acp/a.rs', 'crates//synara-acp/a.rs',
                     'crates/synara-acp/.git/config', None):
            with self.subTest(name=name):
                self.assertFalse(publisher.allowed(name))

    def test_non_authorized_refs_stop_before_git_or_payload_access(self):
        for ref in ('refs/heads/main', 'refs/heads/astra/gpui-clean-rewrite',
                    'refs/heads/archive/pre-rewrite-main-2026-09-17',
                    'refs/heads/astra/session-ajm'):
            with self.subTest(ref=ref), patch.dict(os.environ, {
                'GITHUB_REPOSITORY': 'cmdr-chara/synara', 'GITHUB_REF': ref,
            }), patch.object(subprocess, 'run') as run, patch.object(Path, 'exists') as exists:
                with self.assertRaises(SystemExit):
                    publisher.main()
                run.assert_not_called()
                exists.assert_not_called()


if __name__ == '__main__':
    unittest.main()
