#!/usr/bin/env python3
"""Hermetic tests for bounded source transport. No network or remote writes."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('publisher', Path(__file__).with_name('apply_source.py'))
publisher = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(publisher)

class DeltaTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.previous = Path.cwd()
        import os
        os.chdir(self.root)
        publisher.ROOT = self.root.resolve()
        self.git('init', '-q')
        self.git('config', 'user.name', 'Source test')
        self.git('config', 'user.email', 'source-test@example.invalid')
        (self.root / 'file.txt').write_text('before\n')
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture')
    def tearDown(self):
        import os
        os.chdir(self.previous)
        self.temp.cleanup()
    def git(self, *args):
        return subprocess.check_output(['git', *args], stderr=subprocess.DEVNULL)
    def patch(self):
        self.git('add', '--all')
        data = self.git('diff', '--cached', '--binary', '--no-ext-diff')
        self.git('reset', '--hard', 'HEAD')
        return data
    def test_regular_delta_updates_the_index(self):
        (self.root / 'file.txt').write_text('after\n')
        publisher.apply_diff(self.patch())
        self.assertEqual((self.root / 'file.txt').read_text(), 'after\n')
        self.assertEqual(self.git('show', ':file.txt'), b'after\n')
    def test_workflow_edits_are_not_a_source_delta(self):
        target = self.root / '.github/workflows/no.yml'
        target.parent.mkdir(parents=True)
        target.write_text('not allowed\n')
        with self.assertRaises(SystemExit): publisher.apply_diff(self.patch())
        self.assertFalse(target.exists())
    def test_links_cannot_be_created(self):
        (self.root / 'outside-link').symlink_to('/tmp/not-owned')
        with self.assertRaises(SystemExit): publisher.apply_diff(self.patch())
        self.assertFalse((self.root / 'outside-link').is_symlink())
    def test_binary_payload_is_refused(self):
        (self.root / 'file.txt').write_bytes(b'\0\1\2')
        with self.assertRaises(SystemExit): publisher.apply_diff(self.patch())
    def test_empty_payload_is_refused(self):
        with self.assertRaises((SystemExit, subprocess.CalledProcessError)): publisher.apply_diff(b'')
    def test_changed_base_does_not_get_overwritten(self):
        (self.root / 'file.txt').write_text('proposed\n')
        delta = self.patch()
        (self.root / 'file.txt').write_text('concurrent work\n')
        with self.assertRaises(subprocess.CalledProcessError): publisher.apply_diff(delta)
        self.assertEqual((self.root / 'file.txt').read_text(), 'concurrent work\n')

if __name__ == '__main__':
    unittest.main()
