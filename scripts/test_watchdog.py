"""Behavioral checks for bounded diagnostic runs and descendant cleanup."""
import json
import os
import signal
from types import SimpleNamespace
from unittest import mock

import watchdog
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
        self.assertEqual(result.returncode, 7, result.stderr)
        self.assertEqual(metadata["status"], "completed")
        self.assertEqual(stdout, b"x" * 100000 + b"\n")
        self.assertEqual(stderr, b"y" * 100000 + b"\n")

    def test_cleanup_drain_failure_still_kills_reaps_and_closes(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "run"
            source = """
import sys
from pathlib import Path
from types import SimpleNamespace
from unittest import mock
import watchdog
real_stream = watchdog.stream
real_popen = watchdog.subprocess.Popen
children = []
def launch(*args, **kwargs):
    child = real_popen(*args, **kwargs)
    children.append(child)
    return child
def stream(*args, **kwargs):
    if kwargs.get('shutdown'):
        raise OSError('injected drain failure')
    return real_stream(*args, **kwargs)
args = SimpleNamespace(output=Path(sys.argv[1]), timeout=0.3, grace=0.1,
    command=[sys.executable, '-c', 'import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); print("ready",flush=True); time.sleep(30)'])
with mock.patch.object(watchdog.subprocess, 'Popen', launch), mock.patch.object(watchdog, 'stream', stream):
    try:
        watchdog.run(args)
    except OSError as error:
        assert str(error) == 'injected drain failure'
    else:
        raise AssertionError('expected drain failure')
assert children[0].returncode == -9
assert children[0].stdout.closed and children[0].stderr.closed
"""
            process = subprocess.Popen([sys.executable, "-c", source, str(output)],
                                       cwd=WATCHDOG.parent, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                _, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 0, stderr)
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

    def test_exited_leader_is_reaped_while_inherited_pipe_stays_open(self):
        # Model Darwin's zombie-only killpg EPERM with real inherited pipe
        # holders. Reaping must occur during both normal and shutdown drains.
        for shutdown in (False, True):
            with self.subTest(shutdown=shutdown), tempfile.TemporaryFile() as log:
                readers, writers = [], []
                process = SimpleNamespace(pid=12345, reaped=False)

                def poll():
                    process.reaped = True
                    return 0

                def killpg(group, signum):
                    if not process.reaped:
                        raise PermissionError("Darwin zombie-only process group")
                    raise ProcessLookupError()

                process.poll = poll
                try:
                    for name in ("stdout", "stderr"):
                        read, write = os.pipe()
                        readers.append(os.fdopen(read, "rb"))
                        writers.append(write)
                        setattr(process, name, readers[-1])
                    status = watchdog.stream(process, {"stdout": log, "stderr": log},
                                             watchdog.time.monotonic() + 0.02,
                                             shutdown=shutdown)
                    self.assertEqual(status, "timeout")
                    self.assertTrue(process.reaped)
                    with mock.patch.object(watchdog.os, "killpg", killpg):
                        watchdog.terminate(process, 0, lambda deadline: None)
                finally:
                    for reader in readers:
                        reader.close()
                    for writer in writers:
                        os.close(writer)

    def test_cleanup_reaps_before_signals_even_when_drain_fails(self):
        process = SimpleNamespace(pid=12345, reaped=False)
        signals = []

        def poll():
            process.reaped = True
            return 0

        def killpg(group, signum):
            if not process.reaped:
                raise PermissionError("Darwin zombie-only process group")
            signals.append(signum)

        def drain(deadline):
            # The leader may exit during the grace period, including when
            # reading its output fails before stream can reap it.
            process.reaped = False
            raise OSError("injected drain failure")

        process.poll = poll
        with mock.patch.object(watchdog.os, "killpg", killpg):
            with self.assertRaisesRegex(OSError, "injected drain failure"):
                watchdog.terminate(process, 0, drain)
        self.assertEqual(signals, [signal.SIGTERM, signal.SIGKILL])

    def test_final_drain_finishes_at_eof_without_sleep(self):
        process = SimpleNamespace(stdout=None, stderr=None, poll=lambda: 0)
        with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
            # Pipes at EOF exercise the selector path portably on Linux/macOS.
            readers = []
            try:
                for name in ("stdout", "stderr"):
                    read, write = os.pipe()
                    os.close(write)
                    reader = os.fdopen(read, "rb")
                    readers.append(reader)
                    setattr(process, name, reader)
                with mock.patch.object(watchdog.time, "sleep", side_effect=AssertionError("EOF must not sleep")):
                    self.assertEqual(watchdog.stream(process, {"stdout": stdout, "stderr": stderr},
                                                    watchdog.time.monotonic() + 1), "completed")
            finally:
                for reader in readers:
                    reader.close()

    def test_interrupt_inside_final_record_is_republished(self):
        with tempfile.TemporaryDirectory() as directory:
            args = SimpleNamespace(output=Path(directory) / "run", timeout=3, grace=0,
                                   command=[sys.executable, "-c", "pass"])
            original_write = Path.write_text
            original_handler = signal.getsignal(signal.SIGINT)
            injected = False

            def write(path, content, *args, **kwargs):
                nonlocal injected
                result = original_write(path, content, *args, **kwargs)
                if path.name == "run.json" and json.loads(content)["status"] == "completed" and not injected:
                    injected = True
                    os.kill(os.getpid(), signal.SIGINT)
                return result

            with mock.patch.object(Path, "write_text", write):
                self.assertEqual(watchdog.run(args), 130)
            self.assertTrue(injected)
            self.assertEqual(signal.getsignal(signal.SIGINT), original_handler)
            self.assertEqual(json.loads((args.output / "run.json").read_text())["status"], "interrupted")

    def test_timeout_retains_term_handler_output(self):
        result, metadata, stdout, stderr = self.invoke(
            'import os,signal,time; '
            'signal.signal(signal.SIGTERM, lambda *_: '
            '(os.write(1,b"out\\x00\\xff"), os.write(2,b"err\\x00\\xff"))); '
            'print("ready",flush=True); time.sleep(30)', timeout=0.4)
        self.assertEqual(result.returncode, 124, result.stderr)
        self.assertEqual(metadata["status"], "timeout")
        self.assertEqual(stdout, b"ready\n" + b"out\x00\xff")
        self.assertEqual(stderr, b"err\x00\xff")

    def test_nonfinite_budgets_rejected(self):
        for script in (WATCHDOG, WATCHDOG.with_name("measure_ci.py")):
            for option in ("--timeout", "--grace"):
                for value in ("nan", "inf", "-inf"):
                    with self.subTest(script=script, option=option, value=value):
                        command = [sys.executable, str(script), "--timeout", "1",
                                   "--output", "/unused", f"{option}={value}", "--", "true"]
                        if script.name == "measure_ci.py":
                            command[2:2] = ["--phase", "validation"]
                        result = subprocess.run(command, capture_output=True, timeout=5)
                        self.assertEqual(result.returncode, 2, result.stderr)

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
            self.assertEqual(result.returncode, 124, result.stderr)
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
        self.assertEqual(result.returncode, 124, result.stderr)
        self.assertEqual(metadata["status"], "timeout")


if __name__ == "__main__":
    unittest.main()
