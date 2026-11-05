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


def solid_control(inputs, out, level):
    """One frame over a JSON array of the exact input documents, size only."""
    target = out / "solid-array.json.zst"
    expected = hashlib.sha256()
    raw_bytes = 0
    with target.open("wb") as destination:
        with zstandard.ZstdCompressor(level=level, threads=0).stream_writer(destination) as writer:
            def write(data):
                nonlocal raw_bytes
                writer.write(data)
                expected.update(data)
                raw_bytes += len(data)
            write(b"[")
            for index, item in enumerate(inputs):
                if index:
                    write(b",")
                with Path(item["path"]).open("rb") as source:
                    while chunk := source.read(1024 * 1024):
                        write(chunk)
            write(b"]")
    with target.open("rb") as source:
        with zstandard.ZstdDecompressor().stream_reader(source) as stream:
            reconstructed = hashlib.file_digest(stream, "sha256").hexdigest()
    if reconstructed != expected.hexdigest():
        raise RuntimeError("Solid JSON-array control did not round-trip")
    return {"compressed_bytes": target.stat().st_size, "uncompressed_bytes": raw_bytes,
            "compressed_sha256": digest(target), "reconstructed_sha256": reconstructed,
            "lossless": True, "scope": "One Zstandard frame over an array of the unchanged original JSON documents; size only, no timed reader"}


def control(directory, level):
    prepare = json.loads((directory / "prepare.json").read_text())
    out = directory / "original-json-zstd"
    out.mkdir(exist_ok=True)
    rows = []
    for index, item in enumerate(prepare["inputs"]):
        path = Path(item["path"])
        if digest(path) != item["sha256"]:
            raise RuntimeError(f"Input changed: {path}")
        # Captured units reuse document-000000.cpg.json and other basenames.
        target = out / f"document-{index:06}.json.zst"
        with path.open("rb") as source, target.open("wb") as destination:
            zstandard.ZstdCompressor(level=level, threads=0).copy_stream(source, destination)
        with target.open("rb") as source:
            with zstandard.ZstdDecompressor().stream_reader(source) as stream:
                reconstructed = hashlib.file_digest(stream, "sha256").hexdigest()
        if reconstructed != item["sha256"]:
            raise RuntimeError(f"Compressed control did not round-trip: {path}")
        rows.append({**item, "compressed_file": str(target), "compressed_bytes": target.stat().st_size,
                     "compressed_sha256": digest(target), "lossless": True})
    result = {"zstd_level": level, "zstandard_python": zstandard.__version__,
              "zstd_version": zstandard.ZSTD_VERSION,
              "original_bytes": sum(row["bytes"] for row in rows),
              "compressed_bytes": sum(row["compressed_bytes"] for row in rows),
              "solid_json_array": solid_control(prepare["inputs"], out, level),
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
