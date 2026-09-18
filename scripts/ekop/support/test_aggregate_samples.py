import json
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from aggregate_samples import load, summarize, write_new

SHA = 'a' * 40

def samples():
    return dict(case='browser-policy', build='release', synthetic=True,
                operations_per_sample=100, samples_ms=[1, 2, 3, 4, 5])


class MeasureTests(unittest.TestCase):
    def test_statistics_and_scope_are_explicit(self):
        report = summarize(samples(), SHA, 'Darwin', 'arm64')
        self.assertEqual((report['median'], report['p95_nearest_rank']), (3, 5))
        self.assertEqual(report['platform'], 'macos')
        self.assertTrue(report['synthetic'])
        self.assertEqual(report['candidate'], SHA)

    def test_private_fields_are_rejected_not_silently_exported(self):
        for key in ['hostname', 'username', 'path', 'url', 'prompt', 'stderr', 'environment', 'screenshot']:
            raw = samples()
            raw[key] = 'secret-canary'
            with self.assertRaises(ValueError):
                summarize(raw, SHA)

    def test_platform_strings_are_mapped_not_copied(self):
        report = json.dumps(summarize(samples(), SHA, 'secret-canary', 'private-machine'))
        self.assertNotIn('canary', report)
        self.assertNotIn('private-machine', report)

    def test_non_numeric_nonfinite_and_excessive_samples_are_refused(self):
        for invalid in [[1, 2, True], [1, 2, float('nan')], [1, 2, float('inf')],
                        [1, 2, -1], [1, 2, 3_600_001], [1, 2, 'secret'], [1, 2, 10**400], [1, 2], [1]*1001]:
            raw = samples()
            raw['samples_ms'] = invalid
            with self.assertRaises(ValueError):
                summarize(raw, SHA)

    def test_metadata_is_strict(self):
        for key, value in [('case', []), ('case', 'user-defined-secret'), ('build', []),
                           ('synthetic', 1), ('operations_per_sample', True), ('operations_per_sample', 0)]:
            raw = samples()
            raw[key] = value
            with self.assertRaises(ValueError):
                summarize(raw, SHA)
        with self.assertRaises(ValueError):
            summarize(samples(), 'user/path')

    def test_duplicate_or_oversized_input_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'input'
            path.write_text('{"case":"a","case":"b"}')
            with self.assertRaises(ValueError): load(path)
            path.write_bytes(b' ' * 65537)
            with self.assertRaises(ValueError): load(path)

    def test_existing_output_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'report.json'
            report = summarize(samples(), SHA)
            write_new(path, report)
            before = path.read_bytes()
            with self.assertRaises(FileExistsError): write_new(path, report)
            self.assertEqual(path.read_bytes(), before)


class CommandLineTests(unittest.TestCase):
    def invoke(self, *args):
        return subprocess.run([sys.executable, str(Path(__file__).with_name('aggregate_samples.py')), *args],
                              capture_output=True, text=True, timeout=15)

    def test_command_line_rejects_personal_input_without_copying_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = samples()
            data['path'] = 'personal-canary'
            source = root / 'input.json'
            source.write_text(json.dumps(data), encoding='utf-8')
            target = root / 'output.json'
            result = self.invoke('--input', str(source), '--output', str(target), '--candidate', SHA)
            self.assertEqual(result.returncode, 2)
            self.assertFalse(target.exists())
            self.assertNotIn('personal-canary', result.stdout + result.stderr)
            self.assertNotIn(str(root), result.stdout + result.stderr)

    def test_cli_round_trip_and_no_clobber(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'input.json'
            source.write_text(json.dumps(samples()), encoding='utf-8')
            target = root / 'output.json'
            args = ['--input', str(source), '--output', str(target), '--candidate', SHA]
            self.assertEqual(self.invoke(*args).returncode, 0)
            self.assertEqual(json.loads(target.read_text(encoding='utf-8'))['sample_count'], 5)
            before = target.read_bytes()
            self.assertEqual(self.invoke(*args).returncode, 2)
            self.assertEqual(target.read_bytes(), before)

    def test_result_does_not_alias_mutable_input_samples(self):
        raw = samples()
        result = summarize(raw, SHA)
        raw['samples_ms'][0] = 'personal-canary'
        self.assertEqual(result['samples'], [1, 2, 3, 4, 5])


if __name__ == '__main__':
    unittest.main()
