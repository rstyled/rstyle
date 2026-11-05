#!/usr/bin/env python3
"""Matched codec trials. Each timed operation runs in a fresh Rust process."""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import time


CODECS = ["json", "flatbuffers", "flexbuffers", "protobuf"]
BINARY_CODECS = CODECS[1:]


def read_json(path):
    return json.loads(Path(path).read_text())


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def sha256(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def cgroup():
    result = {}
    for name in ["cpu.max", "cpu.stat", "memory.max", "memory.current", "memory.events", "memory.pressure"]:
        path = Path("/sys/fs/cgroup") / name
        if path.exists():
            result[name] = path.read_text()
    return result


def machine(binary):
    def command(*args):
        result = subprocess.run(args, capture_output=True, text=True, check=False)
        return (result.stdout + result.stderr).strip()
    return {
        "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "cpu_count": os.cpu_count(),
        "affinity": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
        "lscpu": command("lscpu"),
        "rustc": command(str(Path.home() / ".cargo/bin/rustc"), "-vV"),
        "binary": str(binary),
        "binary_sha256": sha256(binary),
        "cgroup": cgroup(),
    }


def execute(command, cpu=None):
    if cpu is not None:
        command = ["taskset", "--cpu-list", str(cpu), *command]
    start = time.monotonic()
    subprocess.run(command, check=True)
    return time.monotonic() - start


def trial(binary, directory, codec, operation, phase, number, level, expected_digest, cpu, inputs):
    label = codec + ("-zstd" if level is not None else "")
    result_path = directory / "trials" / f"{phase}-{number:02d}-{label}-{operation}.json"
    wire = directory / "wire" / label
    if codec == "original-json":
        command = [str(binary), "original-trial", "--operation", operation,
                   "--wire", str(wire), "--result", str(result_path)]
    else:
        command = [str(binary), "trial", "--codec", codec, "--operation", operation,
                   "--model", str(directory / "model.json"), "--wire", str(wire),
                   "--result", str(result_path)]
    if level is not None:
        command.extend(["--zstd-level", str(level)])
    if codec == "original-json":
        command.extend(map(str,inputs))
    elapsed = execute(command, cpu)
    result = read_json(result_path)
    if result["archive_digest"] != expected_digest:
        raise RuntimeError(f"{label} {operation}: reconstructed archive digest changed")
    result.update(label=label, phase=phase, repetition=number, process_wall_seconds=elapsed)
    write_json(result_path, result)
    print(f"{directory.name} {phase} {number} {label} {operation}: "
          f"{result['seconds']:.6f}s, {result['wire_bytes']:,} bytes", flush=True)
    return result


def summarize(trials):
    grouped = {}
    for trial in trials:
        if trial["phase"] == "measured":
            grouped.setdefault((trial["label"], trial["operation"]), []).append(trial)
    rows = {}
    for (label, operation), values in grouped.items():
        row = rows.setdefault(label, {"wire_bytes": values[0]["wire_bytes"], "codec": values[0]["codec"],
                                     "representation": values[0].get("representation", "shared-arena"),
                                     "zstd_level": values[0]["zstd_level"]})
        hashes = {value["wire_sha256"] for value in values}
        if len(hashes) != 1:
            raise RuntimeError(f"Non-deterministic wire output: {label}")
        row["wire_sha256"] = next(iter(hashes))
        row[operation] = {}
        for field in ["seconds", "codec_seconds", "compression_seconds", "io_seconds",
                      "baseline_rss_bytes", "peak_rss_bytes", "process_wall_seconds"]:
            data = [value[field] for value in values if value[field] is not None]
            if data:
                median = statistics.median(data)
                row[operation][field] = {"median": median, "min": min(data), "max": max(data),
                                        "mad": statistics.median(abs(x - median) for x in data), "samples": data}
        row[operation]["peak_rss_reset"] = all(v["peak_rss_reset"] for v in values)
    for row in rows.values():
        row["encode_plus_decode_seconds"] = row["encode"]["seconds"]["median"] + row["decode"]["seconds"]["median"]
    for rank, (label, row) in enumerate(sorted(rows.items(), key=lambda item: item[1]["encode_plus_decode_seconds"]), 1):
        row["time_rank"] = rank
    for operation in ["encode", "decode"]:
        for rank, (_, row) in enumerate(sorted(rows.items(), key=lambda item: item[1][operation]["seconds"]["median"]), 1):
            row[operation + "_time_rank"] = rank
    return rows


def benchmark(args):
    binary = args.binary.resolve()
    directory = args.out.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "trials").mkdir(exist_ok=True)
    (directory / "wire").mkdir(exist_ok=True)
    inputs = sorted(path.resolve() for path in args.inputs)
    if not inputs:
        raise RuntimeError("No input documents")
    prepare_path = directory / "prepare.json"
    if args.reuse_prepared:
        prepare = read_json(prepare_path)
        actual = [(str(path), sha256(path)) for path in inputs]
        expected = [(item["path"], item["sha256"]) for item in prepare["inputs"]]
        if actual != expected:
            raise RuntimeError("Prepared input inventory no longer matches")
    else:
        if prepare_path.exists():
            raise RuntimeError("Output already prepared; use a new directory or --reuse-prepared")
        execute([str(binary), "prepare", "--out", str(directory), *map(str, inputs)], args.cpu)
        prepare = read_json(prepare_path)
    environment = machine(binary)
    environment.update(cpu_pin=args.cpu, seed=args.seed, repetitions=args.repetitions,
                       warmups=args.warmups, zstd_level=args.zstd_level,
                       selection_rule="lowest sum of median encode and fully materialized decode seconds among uncompressed binary codecs")
    write_json(directory / "environment.json", environment)
    execute([str(binary), "original-inventory", "--out", str(directory / "original-inventory.json"),
             *map(str,inputs)], args.cpu)
    original_digest = read_json(directory / "original-inventory.json")["original_digest"]
    rng = random.Random(args.seed)
    trials = []
    baseline_variants = ([(codec, None) for codec in CODECS] + [("json", args.zstd_level)]
                         + [("original-json", None), ("original-json", args.zstd_level)])

    def run(variants):
        for phase, count in [("warmup", args.warmups), ("measured", args.repetitions)]:
            for number in range(count):
                order = variants.copy()
                rng.shuffle(order)
                for codec, level in order:
                    for operation in ["encode", "decode"]:
                        trials.append(trial(binary, directory, codec, operation, phase, number,
                                            level, original_digest if codec == "original-json" else prepare["archive_digest"],
                                            args.cpu, inputs))

    run(baseline_variants)
    baseline_rows = summarize(trials)
    fastest = min(BINARY_CODECS, key=lambda codec: baseline_rows[codec]["encode_plus_decode_seconds"])
    write_json(directory / "selection.json", {"fastest_binary": fastest, "candidates": {
        codec: baseline_rows[codec]["encode_plus_decode_seconds"] for codec in BINARY_CODECS},
        "rule": environment["selection_rule"]})
    print(f"Selected {fastest} for the additional Zstandard trials", flush=True)
    # A new set of trials, rather than inferring compression performance.
    run([(fastest, args.zstd_level)])
    results = {"schema": "cpg-storage-benchmark/1", "prepare": prepare,
               "environment": environment, "end_cgroup": cgroup(),
               "fastest_binary": fastest, "results": summarize(trials)}
    write_json(directory / "results.json", results)
    print(f"Results: {directory / 'results.json'}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=7)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--zstd-level", type=int, default=3)
    parser.add_argument("--seed", type=int, default=20261105)
    parser.add_argument("--cpu", type=int)
    parser.add_argument("--reuse-prepared", action="store_true")
    parser.add_argument("inputs", type=Path, nargs="+")
    args = parser.parse_args()
    if args.repetitions < 3 or args.warmups < 1:
        parser.error("Use at least three measured trials and one warmup")
    benchmark(args)


if __name__ == "__main__":
    main()
