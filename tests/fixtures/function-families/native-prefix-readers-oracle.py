"""Run independent expectations against fixed development and original bodies.

Usage: python native-prefix-readers-oracle.py /path/to/regex-automata-0.4.18/src/util/wire.rs
This runtime-only harness does not assess extraction or omit Cargo neighbors.
"""
import argparse
import hashlib
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
NATURAL = (
    (29612, 29761, "852fd0e757528a8591eb6c9f3ddf1eb51665e7ad8aff97735f6d950e750d4d4c"),
    (30069, 30218, "302aaf96df8e3b521df5bcc491decff3b23f403f71457870b03a9f54ce6eb865"),
    (30359, 30513, "cd2740158fc9036a45584ac18e0dc3fa3d3fc0c2c45401d9d05915ad820a5797"),
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wire_source", type=Path)
    args = parser.parse_args()
    source = args.wire_source.read_bytes()
    original = []
    for start, end, digest in NATURAL:
        body = source[start:end]
        if hashlib.sha256(body).hexdigest() != digest:
            raise ValueError(f"Original source identity changed at {start}..{end}")
        original.append(body.decode())
    compiler = subprocess.check_output(["rustc", "-vV"], cwd=ROOT, text=True)
    if "commit-hash: 5c543b0b8c73c7b72bc8284ced4fb22ead15734d\n" not in compiler:
        raise ValueError("Use the repository's pinned rustc for this oracle")
    corpus = ROOT / "tests/fixtures/function-families/native-prefix-readers.rs"
    oracle = ROOT / "tests/fixtures/function-families/native-prefix-readers-oracle.rs"
    harness = (corpus.read_text() + "\nmod native_prefix_natural {\n"
               "use core::mem::size_of;\n" + "\n".join(original) + "\n}\n"
               + oracle.read_text())
    with tempfile.TemporaryDirectory(prefix="rstyle-native-prefix-oracle-") as temp:
        directory = Path(temp)
        path = directory / "oracle.rs"
        binary = directory / "oracle"
        path.write_text(harness)
        subprocess.run(["rustc", "--edition=2021", "--test", "-C",
                        "overflow-checks=on", str(path), "-o", str(binary)],
                       cwd=ROOT, check=True)
        subprocess.run([str(binary)], check=True)


if __name__ == "__main__":
    main()
