"""Check independent gates and preservation of graph-rich report before replay."""
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('ci_style', Path(__file__).with_name('ci-style.py'))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class SharedCaptureTests(unittest.TestCase):
    def check_gates(self, numeric, patterns, expected=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            capture = root / 'exact-current-run'
            capture.mkdir()
            archive = capture / 'report.json'
            archive.write_text('original graph-rich archive')
            calls = []

            def invoke(command, output):
                calls.append(command)
                if 'patterns' in command:
                    output.write_text(json.dumps({'run_directory': str(capture)}))
                    return patterns
                self.assertEqual(command[-2:], ['--replay-run', str(capture)])
                archive.write_text('numeric replay overwrote shared presentation')
                output.write_text('{"summary": "numeric"}')
                return numeric

            output = root / 'results'
            with mock.patch.object(MODULE, 'invoke', invoke), mock.patch.object(MODULE.sys, 'stdout', io.StringIO()) as console:
                self.assertEqual(MODULE.collect(Path('/cli'), output), numeric if expected is None else expected)
            self.assertIn('ci-style: numeric:', console.getvalue())
            self.assertIn(str(output), console.getvalue())
            self.assertLess(len(console.getvalue()), 512)
            self.assertNotIn('run_directory', console.getvalue())
            self.assertEqual(len(calls), 2)
            self.assertEqual((output / 'patterns-capture-report.json').read_text(),
                             'original graph-rich archive')
            status = json.loads((output / 'status.json').read_text())
            self.assertEqual(status['numeric'], numeric if expected is None else expected)
            self.assertEqual(status['patterns'], patterns)
            if expected is None:
                self.assertIsNone(status['error'])
            self.assertTrue((output / 'rstyle-report.json').exists())

    def test_pattern_candidates_do_not_skip_numeric_gate(self):
        self.check_gates(numeric=0, patterns=1)

    def test_numeric_failure_preserves_pattern_gate(self):
        self.check_gates(numeric=1, patterns=0)

    def test_replay_tool_error_stays_tool_error(self):
        self.check_gates(numeric=2, patterns=0)

    def test_pattern_tool_error_cannot_certify_numeric_even_with_report(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'results'
            calls = []
            def invoke(command, report):
                calls.append(command)
                report.write_text('{"run_directory": "/valid-looking"}')
                return 2
            with mock.patch.object(MODULE, 'invoke', invoke):
                self.assertEqual(MODULE.collect(Path('/cli'), output), 2)
            self.assertEqual(len(calls), 1)
            self.assertEqual(json.loads((output / 'status.json').read_text())['numeric'], 2)

    def test_missing_or_malformed_saved_status_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            argv = ['ci-style', '--output', str(output), '--patterns-status']
            with mock.patch('sys.argv', argv):
                self.assertEqual(MODULE.main(), 2)
                (output / 'status.json').write_text('{"patterns": "0"}')
                self.assertEqual(MODULE.main(), 2)
                (output / 'status.json').write_text('{"patterns": 0}')
                self.assertEqual(MODULE.main(), 2)

    def test_launch_failure_retains_tool_error(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'results'
            with mock.patch.object(MODULE.subprocess, 'run', side_effect=OSError('launch failed')):
                self.assertEqual(MODULE.collect(Path('/missing'), output), 2)
            self.assertEqual(json.loads((output / 'status.json').read_text())['numeric'], 2)

    def test_invalid_saved_status_values_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            for content in ('null', '[]', '{}', '{"numeric":0,"patterns":null}',
                            '{"numeric":true,"patterns":0}', '{"numeric":0,"patterns":3}', '{'):
                with self.subTest(content=content):
                    (output / 'status.json').write_text(content)
                    self.assertEqual(MODULE.saved_patterns(output), 2)

    def test_saved_pattern_summary_and_each_required_presentation(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            files = ('rstyle-report.json', 'rstyle-patterns.json', 'patterns-capture-report.json')
            for code in (0, 1, 2):
                (output / 'status.json').write_text(json.dumps({'numeric': 0, 'patterns': code, 'error': 'specific protocol failure' if code == 2 else None}))
                for name in files:
                    (output / name).write_bytes(b'pattern\x00\xff')
                buffer = io.StringIO()
                with mock.patch.object(MODULE.sys, 'stdout', buffer), mock.patch.object(MODULE.sys, 'stderr', io.StringIO()) as errors:
                    self.assertEqual(MODULE.saved_patterns(output), code)
                if code == 2:
                    self.assertIn('specific protocol failure', errors.getvalue())
                self.assertIn(('passed', 'findings', 'tool error')[code], buffer.getvalue())
                self.assertIn(str(output), buffer.getvalue())
                self.assertLess(len(buffer.getvalue()), 512)
                self.assertNotIn('pattern\x00', buffer.getvalue())
            (output / 'status.json').write_text('{"numeric":0,"patterns":0}')
            for missing in files:
                for name in files:
                    (output / name).write_bytes(b'presentation')
                (output / missing).unlink()
                with self.subTest(missing=missing):
                    self.assertEqual(MODULE.saved_patterns(output), 2)
                (output / missing).mkdir()
                with self.subTest(non_regular=missing):
                    self.assertEqual(MODULE.saved_patterns(output), 2)
                (output / missing).rmdir()

    def test_successful_capture_malformed_or_missing_run_fails_closed(self):
        for content in ('not json', '{}', 'null', '{"run_directory":null}'):
            with self.subTest(content=content), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / 'results'
                calls = []
                def invoke(command, report):
                    calls.append(command)
                    report.write_text(content)
                    return 0
                with mock.patch.object(MODULE, 'invoke', invoke):
                    with mock.patch.object(MODULE.sys, 'stderr', io.StringIO()) as errors:
                        self.assertEqual(MODULE.collect(Path('/cli'), output), 2)
                    self.assertTrue(errors.getvalue().startswith('ci-style: '))
                self.assertEqual(len(calls), 1)
                self.assertEqual(json.loads((output / 'status.json').read_text())['numeric'], 2)

    def test_capture_tool_failure_cannot_reuse_stale_report(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'results'
            def invoke(command, report):
                report.write_text('capture failed')
                return 2
            with mock.patch.object(MODULE, 'invoke', invoke):
                self.assertEqual(MODULE.collect(Path('/cli'), output), 2)
            status = json.loads((output / 'status.json').read_text())
            self.assertEqual((status['numeric'], status['patterns']), (2, 2))
            self.assertIsNotNone(status['error'])


if __name__ == '__main__':
    unittest.main()
