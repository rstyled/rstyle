#!/usr/bin/env python3
"""Original JSON + Zstandard size control, without the shared-table transform."""

import argparse
import hashlib
import json
from pathlib import Path

import zstandard


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def control(directory, level):
    prepare = json.loads((directory / "prepare.json").read_text())
    out = directory / "original-json-zstd"
    out.mkdir(exist_ok=True)
    rows = []
    for item in prepare["inputs"]:
        path = Path(item["path"])
        if digest(path) != item["sha256"]:
            raise RuntimeError(f"Input changed: {path}")
        target = out / (path.name + ".zst")
        with path.open("rb") as source, target.open("wb") as destination:
            zstandard.ZstdCompressor(level=level, threads=0).copy_stream(source, destination)
        with target.open("rb") as source:
            with zstandard.ZstdDecompressor().stream_reader(source) as stream:
                reconstructed = hashlib.file_digest(stream, "sha256").hexdigest()
        if reconstructed != item["sha256"]:
            raise RuntimeError(f"Compressed control did not round-trip: {path}")
        rows.append({**item, "compressed_bytes": target.stat().st_size,
                     "compressed_sha256": digest(target), "lossless": True})
    result = {"zstd_level": level, "zstandard_python": zstandard.__version__,
              "zstd_version": zstandard.ZSTD_VERSION,
              "original_bytes": sum(row["bytes"] for row in rows),
              "compressed_bytes": sum(row["compressed_bytes"] for row in rows),
              "scope": "One independent frame per original document; size control only, excluded from timed codec ranking",
              "inputs": rows}
    (directory / "original-json-zstd.json").write_text(json.dumps(result, indent=2) + "\n")
    print(directory.name, result["original_bytes"], result["compressed_bytes"], flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--level", type=int, default=3)
    parser.add_argument("directories", type=Path, nargs="+")
    args = parser.parse_args()
    for directory in args.directories:
        control(directory, args.level)


if __name__ == "__main__":
    main()
