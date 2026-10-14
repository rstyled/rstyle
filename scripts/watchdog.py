#!/usr/bin/env python3
"""Bound a POSIX diagnostic command and retain live output and run metadata."""
import argparse
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time


def terminate(group, grace):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(group, sig)
        except ProcessLookupError:
            return
        if sig == signal.SIGTERM:
            time.sleep(grace)


def stream(process, logs, deadline):
    with selectors.DefaultSelector() as selector:
        for name, pipe in (("stdout", process.stdout), ("stderr", process.stderr)):
            selector.register(pipe, selectors.EVENT_READ, name)
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return "timeout"
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
        while process.poll() is None:
            if time.monotonic() >= deadline:
                return "timeout"
            time.sleep(0.05)
    return "completed"


def run(args):
    args.output.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    metadata = {"command": args.command, "cwd": os.getcwd(),
                "timeout_seconds": args.timeout, "status": "starting"}
    record = args.output / "run.json"
    record.write_text(json.dumps(metadata, indent=2) + "\n")
    with (args.output / "stdout.log").open("wb") as stdout, \
            (args.output / "stderr.log").open("wb") as stderr:
        process = subprocess.Popen(args.command, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, start_new_session=True)
        metadata.update(pid=process.pid, status="running")
        record.write_text(json.dumps(metadata, indent=2) + "\n")
        try:
            status = stream(process, {"stdout": stdout, "stderr": stderr},
                            started + args.timeout)
        except KeyboardInterrupt:
            status = "interrupted"
        finally:
            # Also clean up descendants when the root exits before its children.
            terminate(process.pid, args.grace)
            process.wait()
            process.stdout.close()
            process.stderr.close()
    metadata.update(status=status, child_returncode=process.returncode,
                    elapsed_seconds=time.monotonic() - started)
    record.write_text(json.dumps(metadata, indent=2) + "\n")
    print(f"watchdog: {status}; retained {args.output}", file=sys.stderr)
    return {"timeout": 124, "interrupted": 130}.get(
        status, process.returncode if process.returncode >= 0 else 128 - process.returncode)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timeout", type=float, required=True)
    parser.add_argument("--grace", type=float, default=2)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if not args.command or args.timeout <= 0 or args.grace < 0:
        parser.error("provide a command, positive timeout and nonnegative grace")
    if os.name != "posix":
        parser.error("process-group cleanup requires POSIX")
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
