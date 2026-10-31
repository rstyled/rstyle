#!/usr/bin/env python3
"""Bound a POSIX diagnostic command and retain live output and run metadata."""
import argparse
import json
import math
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time


def terminate(group, grace, drain):
    try:
        os.killpg(group, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        drain(time.monotonic() + grace)
    finally:
        try:
            os.killpg(group, signal.SIGKILL)
        except ProcessLookupError:
            pass


def stream(process, logs, deadline, cancelled=lambda: False, shutdown=False):
    with selectors.DefaultSelector() as selector:
        for name, pipe in (("stdout", process.stdout), ("stderr", process.stderr)):
            selector.register(pipe, selectors.EVENT_READ, name)
        while True:
            if cancelled():
                return "interrupted"
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return "timeout"
            if not selector.get_map():
                if not shutdown and process.poll() is not None:
                    return "completed"
                time.sleep(min(remaining, 0.05))
                continue
            for key, _ in selector.select(min(remaining, 0.1)):
                chunk = os.read(key.fileobj.fileno(), 65536)
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                logs[key.data].write(chunk)
                logs[key.data].flush()
                try:
                    target = getattr(sys, key.data).buffer
                    target.write(chunk)
                    target.flush()
                except BrokenPipeError:
                    pass


def run(args):
    args.output.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    metadata = {"command": args.command, "cwd": os.getcwd(),
                "timeout_seconds": args.timeout, "status": "starting"}
    record = args.output / "run.json"
    record.write_text(json.dumps(metadata, indent=2) + "\n")
    cancelled = False

    def interrupt(signum, frame):
        nonlocal cancelled
        cancelled = True

    previous = signal.signal(signal.SIGINT, interrupt)
    try:
        with (args.output / "stdout.log").open("wb") as stdout, \
                (args.output / "stderr.log").open("wb") as stderr:
            try:
                process = subprocess.Popen(args.command, stdout=subprocess.PIPE,
                                           stderr=subprocess.PIPE, start_new_session=True)
            except OSError as error:
                metadata.update(status="launch_error", error=str(error), child_returncode=None,
                                elapsed_seconds=time.monotonic() - started)
                record.write_text(json.dumps(metadata, indent=2) + "\n")
                return 127
            logs = {"stdout": stdout, "stderr": stderr}
            try:
                metadata.update(pid=process.pid, status="running")
                record.write_text(json.dumps(metadata, indent=2) + "\n")
                status = stream(process, logs, started + args.timeout, lambda: cancelled)
            finally:
                # SIGINT records cancellation throughout launch and cleanup, so it
                # cannot interrupt TERM/grace before KILL and wait have run.
                try:
                    try:
                        terminate(process.pid, args.grace,
                                  lambda deadline: stream(process, logs, deadline, shutdown=True))
                    finally:
                        process.wait()
                    # Bound inherited pipes, but finish immediately at EOF.
                    stream(process, logs, time.monotonic() + 0.2)
                finally:
                    process.stdout.close()
                    process.stderr.close()
        if cancelled:
            status = "interrupted"
        metadata.update(status=status, child_returncode=process.returncode,
                        elapsed_seconds=time.monotonic() - started)
        record.write_text(json.dumps(metadata, indent=2) + "\n")
        # A signal may arrive during the record write itself.
        if cancelled and status != "interrupted":
            status = "interrupted"
            metadata["status"] = status
            record.write_text(json.dumps(metadata, indent=2) + "\n")
        print(f"watchdog: {status}; retained {args.output}", file=sys.stderr)
        return {"timeout": 124, "interrupted": 130}.get(
            status, process.returncode if process.returncode >= 0 else 128 - process.returncode)
    finally:
        signal.signal(signal.SIGINT, previous)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timeout", type=float, required=True)
    parser.add_argument("--grace", type=float, default=2)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if (not args.command or not math.isfinite(args.timeout) or args.timeout <= 0
            or not math.isfinite(args.grace) or args.grace < 0):
        parser.error("provide a command, positive timeout and nonnegative grace")
    if os.name != "posix":
        parser.error("process-group cleanup requires POSIX")
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
