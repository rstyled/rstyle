#!/usr/bin/env python3
"""Measure one explicitly supplied CI phase using the existing POSIX watchdog."""
import argparse
import json
import math
import os
from pathlib import Path
import platform
import resource
import subprocess
import sys

import watchdog


def probe(command):
    """Best-effort local metadata only; never allow rustup to install tools."""
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=3,
                                env=dict(os.environ, RUSTUP_AUTO_INSTALL="0"))
        return result.stdout.strip() if result.returncode == 0 else None
    except (OSError, subprocess.SubprocessError):
        return None


def read_toolchain():
    try:
        return (Path(__file__).resolve().parent.parent / "rust-toolchain.toml").read_text()
    except OSError:
        return None


def available_memory():
    if sys.platform.startswith("linux"):
        try:
            for line in Path("/proc/meminfo").read_text().splitlines():
                if line.startswith("MemAvailable:"):
                    return {"bytes": int(line.split()[1]) * 1024,
                            "source": "/proc/meminfo MemAvailable (host)"}
        except (OSError, ValueError):
            pass
    # macOS has no directly equivalent stdlib available-memory observation.
    return {"bytes": None, "source": None}


def metadata(args):
    return {
        "schema_version": 1, "phase": args.phase,
        "argv": args.command, "cwd": os.getcwd(),
        "commit": probe(["git", "rev-parse", "HEAD"]),
        "toolchain_file": read_toolchain(),
        "toolchain_override": os.environ.get("RUSTUP_TOOLCHAIN"),
        "platform": {"system": platform.system(), "release": platform.release(),
                     "machine": platform.machine(), "cpu_count": os.cpu_count()},
        "available_memory": available_memory(),
        # These are explicit build/test controls, not an environment dump.
        "build_controls": {key: os.environ.get(key) for key in
                           ("CARGO_TARGET_DIR", "RSTYLE_TEST_DRIVER", "CARGO_INCREMENTAL")},
        "resource_scope": "CPU: delta of RUSAGE_CHILDREN for children reaped during watchdog.run; "
                          "descendants only as propagated by POSIX child accounting. "
                          "Excludes metadata probes and supervisor CPU; not process-tree sampling.",
        "peak_rss_bytes": None,
        "peak_rss_unavailable_reason": "RUSAGE_CHILDREN peak is cumulative across metadata probes "
                                       "and cannot be differenced; no tree memory sampling.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", required=True)
    parser.add_argument("--timeout", type=float, required=True)
    parser.add_argument("--grace", type=float, default=2)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if (not args.command or not math.isfinite(args.timeout) or args.timeout <= 0
            or not math.isfinite(args.grace) or args.grace < 0):
        parser.error("provide a command, finite positive timeout and finite nonnegative grace")
    if os.name != "posix":
        parser.error("the watchdog requires POSIX process groups")
    details = metadata(args)
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    try:
        result = watchdog.run(args)
    except FileExistsError:
        parser.error("output directory must be new")
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    details.update(user_cpu_seconds=after.ru_utime - before.ru_utime,
                   system_cpu_seconds=after.ru_stime - before.ru_stime)
    (args.output / "measurement.json").write_text(json.dumps(details, indent=2) + "\n")
    return result


if __name__ == "__main__":
    sys.exit(main())
