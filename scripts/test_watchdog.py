"""Behavioral checks for bounded diagnostic runs and descendant cleanup."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

WATCHDOG = Path(__file__).with_name("watchdog.py")


class WatchdogTests(unittest.TestCase):
    def invoke(self, source, timeout=3):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "run"
            result = subprocess.run(
                [sys.executable, str(WATCHDOG), "--timeout", str(timeout),
                 "--grace", "0.1", "--output", str(output), "--",
                 sys.executable, "-u", "-c", source],
                capture_output=True, timeout=10)
            return (result, json.loads((output / "run.json").read_text()),
                    (output / "stdout.log").read_bytes(),
                    (output / "stderr.log").read_bytes())

    def test_preserves_exit_and_both_large_streams(self):
        result, metadata, stdout, stderr = self.invoke(
            'import sys; print("x" * 100000); '
            'print("y" * 100000, file=sys.stderr); sys.exit(7)')
        self.assertEqual(result.returncode, 7)
        self.assertEqual(metadata["status"], "completed")
        self.assertEqual(stdout, b"x" * 100000 + b"\n")
        self.assertEqual(stderr, b"y" * 100000 + b"\n")

    def test_timeout_kills_term_ignoring_descendant(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "escaped"
            child = ('import signal,time; from pathlib import Path; '
                     'signal.signal(signal.SIGTERM, signal.SIG_IGN); '
                     f'time.sleep(1); Path({str(marker)!r}).touch()')
            source = ('import subprocess,sys,time; '
                      f'subprocess.Popen([sys.executable,"-c",{child!r}]); '
                      'print("started", flush=True); time.sleep(30)')
            result, metadata, stdout, _ = self.invoke(source, timeout=0.4)
            self.assertEqual(result.returncode, 124)
            self.assertEqual(metadata["status"], "timeout")
            self.assertIn(b"started", stdout)
            # If cleanup failed, this child would create the marker after timeout.
            import time
            time.sleep(1.1)
            self.assertFalse(marker.exists())

    def test_inherited_pipe_is_bounded_after_root_exit(self):
        result, metadata, _, _ = self.invoke(
            'import subprocess,sys; subprocess.Popen('
            '[sys.executable,"-c","import time; time.sleep(30)"])', timeout=0.4)
        self.assertEqual(result.returncode, 124)
        self.assertEqual(metadata["status"], "timeout")


if __name__ == "__main__":
    unittest.main()
