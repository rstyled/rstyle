"""Behavioral checks for the one-command measurement wrapper (no Rust builds)."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

SCRIPT = Path(__file__).with_name("measure_ci.py")


class MeasurementTests(unittest.TestCase):
    def command(self, output, command, timeout="3"):
        return [sys.executable, str(SCRIPT), "--phase", "unit-test",
                "--timeout", timeout, "--grace", "0.05", "--output", str(output),
                "--", *command]

    def test_literal_argv_exact_bytes_exit_and_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "run"
            literal = '$(touch should-not-exist); * $HOME'
            command = [sys.executable, "-c", 'import sys; '
                       'sys.stdout.buffer.write(sys.argv[1].encode()+b"\\x00\\xff"); '
                       'sys.stderr.buffer.write(b"err\\x00\\xff"); sys.exit(7)', literal]
            result = subprocess.run(self.command(output, command), capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 7)
            self.assertEqual((output / "stdout.log").read_bytes(), literal.encode()+b"\x00\xff")
            self.assertEqual((output / "stderr.log").read_bytes(), b"err\x00\xff")
            run = json.loads((output / "run.json").read_text())
            data = json.loads((output / "measurement.json").read_text())
            self.assertEqual(run["status"], "completed")
            self.assertEqual(data["argv"], command)
            self.assertEqual(data["cwd"], str(Path.cwd()))
            self.assertGreaterEqual(run["elapsed_seconds"], 0)
            self.assertGreaterEqual(data["user_cpu_seconds"], 0)
            self.assertIsNone(data["peak_rss_bytes"])
            self.assertNotIn("environment", data)
            original = (output / "run.json").read_bytes()
            result = subprocess.run(self.command(output, command), capture_output=True, timeout=10)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual((output / "run.json").read_bytes(), original)

    def test_timeout_and_interruption(self):
        for interrupt in (False, True):
            with self.subTest(interrupt=interrupt), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "run"
                process = subprocess.Popen(self.command(output, [sys.executable, "-u", "-c",
                                           'import time; print("ready"); time.sleep(30)'],
                                           timeout="3" if interrupt else "0.2"),
                                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                if interrupt:
                    deadline = time.monotonic() + 5
                    while not (output / "stdout.log").exists() or not (output / "stdout.log").read_bytes():
                        if time.monotonic() >= deadline:
                            process.kill()
                            self.fail("command did not start")
                        time.sleep(0.01)
                    process.send_signal(signal.SIGINT)
                process.communicate(timeout=10)
                self.assertEqual(process.returncode, 130 if interrupt else 124)
                data = json.loads((output / "run.json").read_text())
                self.assertEqual(data["status"], "interrupted" if interrupt else "timeout")
                self.assertTrue((output / "measurement.json").exists())

    def test_interrupt_during_grace_retains_shutdown_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "run"
            marker = Path(directory) / "terminated"
            source = ("import os,signal,time; from pathlib import Path; "
                      "signal.signal(signal.SIGTERM, lambda *_: "
                      "(os.write(1,b'out\\x00\\xff'), os.write(2,b'err\\x00\\xff'), "
                      f"Path({str(marker)!r}).touch())); "
                      "print('ready',flush=True); time.sleep(30)")
            command = self.command(output, [sys.executable, "-c", source], timeout="0.5")
            command[command.index("--grace") + 1] = "0.5"
            process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                deadline = time.monotonic() + 5
                while not marker.exists():
                    if time.monotonic() >= deadline:
                        self.fail("TERM handler did not run")
                    time.sleep(0.01)
                process.send_signal(signal.SIGINT)
                process.communicate(timeout=5)
                self.assertEqual(process.returncode, 130)
                run = json.loads((output / "run.json").read_text())
                self.assertEqual(run["status"], "interrupted")
                self.assertEqual(run["child_returncode"], -signal.SIGKILL)
                self.assertTrue((output / "measurement.json").exists())
                self.assertEqual((output / "stdout.log").read_bytes(), b"ready\n" + b"out\x00\xff")
                self.assertEqual((output / "stderr.log").read_bytes(), b"err\x00\xff")
            finally:
                record = output / "run.json"
                if record.exists():
                    pid = json.loads(record.read_text()).get("pid")
                    if pid:
                        try:
                            os.killpg(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                if process.poll() is None:
                    process.kill()
                process.communicate(timeout=5)

    def test_launch_error(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "run"
            result = subprocess.run(self.command(output, [str(Path(directory) / "missing")]),
                                    capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 127)
            data = json.loads((output / "run.json").read_text())
            self.assertEqual(data["status"], "launch_error")
            self.assertGreaterEqual(data["elapsed_seconds"], 0)


if __name__ == "__main__":
    unittest.main()
